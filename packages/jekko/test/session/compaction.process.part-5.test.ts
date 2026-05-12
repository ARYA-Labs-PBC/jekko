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
  test("summarizes only the head while keeping recent tail out of summary input", async () => {
    const sample = llm()
    let captured = ""
    sample.push(
      reply("summary", (input) => {
        captured = JSON.stringify(input.messages)
      }),
    )

    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "older context")
        await user(session.id, "keep this turn")
        await user(session.id, "and this one too")
        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = liveRuntime(sample.layer, wide())
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

          expect(captured).toContain("older context")
          expect(captured).not.toContain("keep this turn")
          expect(captured).not.toContain("and this one too")
          expect(captured).not.toContain("What did we do so far?")
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("anchors repeated compactions with the previous summary", async () => {
    const sample = llm()
    let captured = ""
    sample.push(reply("summary one"))
    sample.push(
      reply("summary two", (input) => {
        captured = JSON.stringify(input.messages)
      }),
    )

    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "older context")
        await user(session.id, "keep this turn")
        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = liveRuntime(sample.layer, wide())
        try {
          let msgs = await svc.messages({ sessionID: session.id })
          let parent = msgs.at(-1)?.info.id
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

          await user(session.id, "latest turn")
          await SessionCompaction.create({
            sessionID: session.id,
            agent: "build",
            model: ref,
            auto: false,
          })

          msgs = MessageV2.filterCompacted(MessageV2.stream(session.id))
          parent = msgs.at(-1)?.info.id
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

          expect(captured).toContain("<previous-summary>")
          expect(captured).toContain("summary one")
          expect(captured.match(/summary one/g)?.length).toBe(1)
          expect(captured).toContain("## Constraints & Preferences")
          expect(captured).toContain("## Progress")
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("keeps recent pre-compaction turns across repeated compactions", async () => {
    const sample = llm()
    sample.push(reply("summary one"))
    sample.push(reply("summary two"))
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        const u1 = await user(session.id, "one")
        const u2 = await user(session.id, "two")
        const u3 = await user(session.id, "three")
        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = liveRuntime(sample.layer, wide(), cfg({ tail_turns: 2, preserve_recent_tokens: 10_000 }))
        try {
          let msgs = await svc.messages({ sessionID: session.id })
          let parent = msgs.at(-1)?.info.id
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

          const u4 = await user(session.id, "four")
          await SessionCompaction.create({
            sessionID: session.id,
            agent: "build",
            model: ref,
            auto: false,
          })

          msgs = MessageV2.filterCompacted(MessageV2.stream(session.id))
          parent = msgs.at(-1)?.info.id
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

          const filtered = MessageV2.filterCompacted(MessageV2.stream(session.id))
          const ids = filtered.map((msg) => msg.info.id)

          expect(ids).not.toContain(u1.id)
          expect(ids).not.toContain(u2.id)
          expect(ids).toContain(u3.id)
          expect(ids).toContain(u4.id)
          expect(filtered.some((msg) => msg.info.role === "assistant" && msg.info.summary)).toBe(true)
          expect(
            filtered.some((msg) => msg.info.role === "user" && msg.parts.some((part) => part.type === "compaction")),
          ).toBe(true)
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("ignores previous summaries when sizing the retained tail", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "older")
        const keep = await user(session.id, "keep this turn")
        const keepReply = await assistant(session.id, keep.id, tmp.path)
        await svc.updatePart({
          id: PartID.ascending(),
          messageID: keepReply.id,
          sessionID: session.id,
          type: "text",
          text: "keep reply",
        })

        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })
        const firstCompaction = (await svc.messages({ sessionID: session.id })).at(-1)?.info.id
        expect(firstCompaction).toBeTruthy()
        await summaryAssistant(session.id, firstCompaction!, tmp.path, "summary ".repeat(800))

        const recent = await user(session.id, "recent turn")
        const recentReply = await assistant(session.id, recent.id, tmp.path)
        await svc.updatePart({
          id: PartID.ascending(),
          messageID: recentReply.id,
          sessionID: session.id,
          type: "text",
          text: "recent reply",
        })

        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = runtime("continue", Plugin.defaultLayer, wide(), cfg({ tail_turns: 2, preserve_recent_tokens: 500 }))
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
        } finally {
          await rt.dispose()
        }
      },
    })
  })
