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
  test("throws when parent is not a user message", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        const msg = await user(session.id, "hello")
        const reply = await assistant(session.id, msg.id, tmp.path)
        const rt = runtime("continue")
        try {
          const msgs = await svc.messages({ sessionID: session.id })
          await expect(
            rt.runPromise(
              SessionCompaction.Service.use((svc) =>
                svc.process({
                  parentID: reply.id,
                  messages: msgs,
                  sessionID: session.id,
                  auto: false,
                }),
              ),
            ),
          ).rejects.toThrow(`Compaction parent must be a user message: ${reply.id}`)
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("publishes compacted event on continue", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        const msg = await user(session.id, "hello")
        const msgs = await svc.messages({ sessionID: session.id })
        const done = defer()
        let seen = false
        const rt = runtime("continue", Plugin.defaultLayer, wide())
        let unsub: (() => void) | undefined
        try {
          unsub = await rt.runPromise(
            Bus.Service.use((svc) =>
              svc.subscribeCallback(SessionCompaction.Event.Compacted, (evt) => {
                if (evt.properties.sessionID !== session.id) return
                seen = true
                done.resolve()
              }),
            ),
          )

          const result = await rt.runPromise(
            SessionCompaction.Service.use((svc) =>
              svc.process({
                parentID: msg.id,
                messages: msgs,
                sessionID: session.id,
                auto: false,
              }),
            ),
          )

          await Promise.race([
            done.promise,
            wait(500).then(() => {
              throw new Error("timed out waiting for compacted event")
            }),
          ])
          expect(result).toBe("continue")
          expect(seen).toBe(true)
        } finally {
          unsub?.()
          await rt.dispose()
        }
      },
    })
  })

  test("marks summary message as errored on compact result", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        const msg = await user(session.id, "hello")
        const rt = runtime("compact", Plugin.defaultLayer, wide())
        try {
          const msgs = await svc.messages({ sessionID: session.id })
          const result = await rt.runPromise(
            SessionCompaction.Service.use((svc) =>
              svc.process({
                parentID: msg.id,
                messages: msgs,
                sessionID: session.id,
                auto: false,
              }),
            ),
          )

          const summary = (await svc.messages({ sessionID: session.id })).find(
            (msg) => msg.info.role === "assistant" && msg.info.summary,
          )

          expect(result).toBe("stop")
          expect(summary?.info.role).toBe("assistant")
          if (summary?.info.role === "assistant") {
            expect(summary.info.finish).toBe("error")
            expect(JSON.stringify(summary.info.error)).toContain("Session too large to compact")
          }
        } finally {
          await rt.dispose()
        }
      },
    })
  })

  test("adds synthetic continue prompt when auto is enabled", async () => {
    await using tmp = await tmpdir()
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        const session = await svc.create({})
        const msg = await user(session.id, "hello")
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
              }),
            ),
          )

          const all = await svc.messages({ sessionID: session.id })
          const last = all.at(-1)

          expect(result).toBe("continue")
          expect(last?.info.role).toBe("user")
          expect(last?.parts[0]).toMatchObject({
            type: "text",
            synthetic: true,
            metadata: { compaction_continue: true },
          })
          if (last?.parts[0]?.type === "text") {
            expect(last.parts[0].text).toContain("Continue if you have next steps")
          }
        } finally {
          await rt.dispose()
        }
      },
    })
  })

