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
  test("persists tail_start_id for retained recent turns", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "first")
        const keep = await user(session.id, "second")
        await user(session.id, "third")
        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = runtime(
          "continue",
          Plugin.defaultLayer,
          wide(),
          cfg({ tail_turns: 2, preserve_recent_tokens: 10_000 }),
        )
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

  test("shrinks retained tail to fit preserve token budget", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        await user(session.id, "first")
        await user(session.id, "x".repeat(2_000))
        const keep = await user(session.id, "tiny")
        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = runtime("continue", Plugin.defaultLayer, wide(), cfg({ tail_turns: 2, preserve_recent_tokens: 100 }))
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

  test("falls back to full summary when even one recent turn exceeds preserve token budget", async () => {
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
        await user(session.id, "first")
        await user(session.id, "y".repeat(2_000))
        await SessionCompaction.create({
          sessionID: session.id,
          agent: "build",
          model: ref,
          auto: false,
        })

        const rt = liveRuntime(sample.layer, wide(), cfg({ tail_turns: 1, preserve_recent_tokens: 20 }))
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
          expect(part?.tail_start_id).toBeUndefined()
          expect(captured).toContain("yyyy")
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("falls back to full summary when retained tail media exceeds preserve token budget", async () => {
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
        const recent = await user(session.id, "recent image turn")
        await svc.updatePart({
          id: PartID.ascending(),
          messageID: recent.id,
          sessionID: session.id,
          type: "file",
          mime: "image/png",
          filename: "big.png",
          url: `data:image/png;base64,${"a".repeat(4_000)}`,
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
          expect(part?.tail_start_id).toBeUndefined()
          expect(captured).toContain("recent image turn")
          expect(captured).toContain("Attached image/png: big.png")
        } finally {
          await rt.dispose()
        }
      },
    })
  })

