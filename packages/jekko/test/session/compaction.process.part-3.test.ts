import { describe, expect, test } from "bun:test"
import { APICallError } from "ai"
import { Effect, Layer, ManagedRuntime } from "effect"
import * as Stream from "effect/Stream"
import z from "zod"
import { Bus } from "../../src/bus"
import { Config } from "@/config/config"
import { Agent } from "../../src/agent/agent"
import { LLM } from "../../src/session/llm"
import { SessionCompaction } from "../../src/session/compaction"
import { Token } from "@/util/token"
import { Instance } from "../../src/project/instance"
import { WithInstance } from "../../src/project/with-instance"
import * as Log from "@jekko-ai/core/util/log"
import { Permission } from "../../src/permission"
import { Plugin } from "../../src/plugin"
import { provideTmpdirInstance, tmpdir } from "../fixture/fixture"
import { Session as SessionNs } from "@/session/session"
import { MessageV2 } from "../../src/session/message"
import { MessageID, PartID, SessionID } from "../../src/session/schema"
import { SessionStatus } from "../../src/session/status"
import { SessionSummary } from "../../src/session/summary"
import { SessionV2 } from "../../src/v2/session"
import { ModelID, ProviderID } from "../../src/provider/schema"
import { Memory } from "../../src/memory"
import type { Provider } from "@/provider/provider"
import * as SessionProcessorModule from "../../src/session/processor"
import { Snapshot } from "../../src/snapshot"
import { ProviderTest } from "../fake/provider"
import { testEffect } from "../lib/effect"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { TestConfig } from "../fixture/config"
import {
  svc,
  summary,
  ref,
  createModel,
  wide,
  user,
  assistant,
  summaryAssistant,
  lastCompactionPart,
  fake,
  layer,
  cfg,
  runtime,
  deps,
  env,
  it,
  llm,
  liveRuntime,
  reply,
  wait,
  defer,
  plugin,
  autocontinue,
} from "./compaction.fixture"

void Log.init({ print: false })
  test("retains a split turn suffix when a later message fits the preserve token budget", async () => {
    await using tmp = await tmpdir()
    const sample = llm()
    let captured = ""
    sample.push(
      reply("summary", (input) => {
        captured = JSON.stringify(input.messages)
      }),
    )
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "older")
        const recent = await user(session.id, "recent turn")
        const large = await assistant(session.id, recent.id, tmp.path)
        await svc.updatePart({
          id: PartID.ascending(),
          messageID: large.id,
          sessionID: session.id,
          type: "text",
          text: "z".repeat(2_000),
        })
        const keep = await assistant(session.id, recent.id, tmp.path)
        await svc.updatePart({
          id: PartID.ascending(),
          messageID: keep.id,
          sessionID: session.id,
          type: "text",
          text: "keep tail",
        })
        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = liveRuntime(sample.layer, wide(), cfg({ tail_turns: 1, preserve_recent_tokens: 100 }))
        try {
          const msgs = await svc.messages({ sessionID: session.id })
          const parent = msgs.at(-1)?.info.id
          expect(parent).toBeTruthy()
          await rt.runPromise(
            SessionCompaction.Service.use((svc) =>
              svc.process({
                parentID: parent!,
                messages: msgs,
                sessionID: session.id,
                auto: false,
              }),
            ),
          )

          const part = await lastCompactionPart(session.id)
          expect(part?.type).toBe("compaction")
          expect(part?.tail_start_id).toBe(keep.id)
          expect(captured).toContain("zzzz")
          expect(captured).not.toContain("keep tail")

          const filtered = MessageV2.filterCompacted(MessageV2.stream(session.id))
          expect(filtered.map((msg) => msg.info.id).slice(0, 3)).toEqual([parent!, expect.any(String), keep.id])
          expect(filtered[1]?.info.role).toBe("assistant")
          expect(filtered[1]?.info.role === "assistant" ? filtered[1].info.summary : false).toBe(true)
          expect(filtered.map((msg) => msg.info.id)).not.toContain(large.id)
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("allows plugins to disable synthetic continue prompt", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        const msg = await user(session.id, "hello")
        const rt = runtime("continue", autocontinue(false), wide())
        try {
          const msgs = await svc.messages({ sessionID: session.id })
          const result = await rt.runPromise(
            SessionCompaction.Service.use((svc) =>
              svc.process({
                parentID: msg.id,
                messages: msgs,
                sessionID: session.id,
                auto: true,
              }),
            ),
          )

          const all = await svc.messages({ sessionID: session.id })
          const last = all.at(-1)

          expect(result).toBe("continue")
          expect(last?.info.role).toBe("assistant")
          expect(
            all.some(
              (msg) =>
                msg.info.role === "user" &&
                msg.parts.some(
                  (part) =>
                    part.type === "text" && part.synthetic && part.text.includes("Continue if you have next steps"),
                ),
            ),
          ).toBe(false)
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("replays the prior user turn on overflow when earlier context exists", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "root")
        const replay = await user(session.id, "image")
        await svc.updatePart({
          id: PartID.ascending(),
          messageID: replay.id,
          sessionID: session.id,
          type: "file",
          mime: "image/png",
          filename: "cat.png",
          url: "https://example.com/cat.png",
        })
        const msg = await user(session.id, "current")
        const rt = runtime("continue", Plugin.defaultLayer, wide())
        try {
          const msgs = await svc.messages({ sessionID: session.id })
          const result = await rt.runPromise(
            SessionCompaction.Service.use((svc) =>
              svc.process({
                parentID: msg.id,
                messages: msgs,
                sessionID: session.id,
                auto: true,
                overflow: true,
              }),
            ),
          )

          const last = (await svc.messages({ sessionID: session.id })).at(-1)

          expect(result).toBe("continue")
          expect(last?.info.role).toBe("user")
          expect(last?.parts.some((part) => part.type === "file")).toBe(false)
          expect(
            last?.parts.some((part) => part.type === "text" && part.text.includes("Attached image/png: cat.png")),
          ).toBe(true)
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("falls back to overflow guidance when no replayable turn exists", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "earlier")
        const msg = await user(session.id, "current")

        const rt = runtime("continue", Plugin.defaultLayer, wide())
        try {
          const msgs = await svc.messages({ sessionID: session.id })
          const result = await rt.runPromise(
            SessionCompaction.Service.use((svc) =>
              svc.process({
                parentID: msg.id,
                messages: msgs,
                sessionID: session.id,
                auto: true,
                overflow: true,
              }),
            ),
          )

          const last = (await svc.messages({ sessionID: session.id })).at(-1)

          expect(result).toBe("continue")
          expect(last?.info.role).toBe("user")
          if (last?.parts[0]?.type === "text") {
            expect(last.parts[0].text).toContain("previous request exceeded the provider's size limit")
          }
        } finally {
          await rt.dispose()
        }
      },
    })
  })

