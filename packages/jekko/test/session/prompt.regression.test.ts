import { NodeFileSystem } from "@effect/platform-node"
import { FetchHttpClient } from "effect/unstable/http"
import { expect, test } from "bun:test"
import { Cause, Effect, Exit, Fiber, Layer } from "effect"
import path from "path"
import { fileURLToPath } from "url"
import { NamedError } from "@jekko-ai/core/util/error"
import { Agent as AgentSvc } from "../../src/agent/agent"
import { Bus } from "../../src/bus"
import { Command } from "../../src/command"
import { Config } from "@/config/config"
import { LSP } from "@/lsp/lsp"
import { MCP } from "../../src/mcp"
import { Permission } from "../../src/permission"
import { Plugin } from "../../src/plugin"
import { Provider as ProviderSvc } from "@/provider/provider"
import { Env } from "../../src/env"
import { ModelID, ProviderID } from "../../src/provider/schema"
import { Question } from "../../src/question"
import { Pending } from "../../src/session/pending"
import { Session } from "@/session/session"
import { SessionMessageTable } from "../../src/session/session.sql"
import { LLM } from "../../src/session/llm"
import { MessageV2 } from "../../src/session/message"
import { AppFileSystem } from "@jekko-ai/core/filesystem"
import { PatchVmModule } from "../../src/patch/vm"
import { Memory } from "../../src/memory"
import { SessionCompaction } from "../../src/session/compaction"
import { SessionSummary } from "../../src/session/summary"
import { Instruction } from "../../src/session/instruction"
import { SessionProcessor } from "../../src/session/processor"
import { SessionPrompt } from "../../src/session/prompt"
import { SessionRevert } from "../../src/session/revert"
import { SessionRunState } from "../../src/session/run-state"
import { MessageID, PartID, SessionID } from "../../src/session/schema"
import { SessionStatus } from "../../src/session/status"
import { SessionV2 } from "../../src/v2/session"
import { Skill } from "../../src/skill"
import { SystemPrompt } from "../../src/session/system"
import { Shell } from "../../src/shell/shell"
import { Snapshot } from "../../src/snapshot"
import { ToolRegistry } from "@/tool/registry"
import { DaemonStore } from "@/session/daemon-store"
import { Truncate } from "@/tool/truncate"
import * as Log from "@jekko-ai/core/util/log"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import * as Database from "../../src/storage/db"
import { Ripgrep } from "../../src/file/ripgrep"
import { Format } from "../../src/format"
import { provideTmpdirInstance, provideTmpdirServer } from "../fixture/fixture"
import { testEffect } from "../lib/effect"
import { reply, TestLLMServer } from "../lib/llm-server"
import { addSubtask, boot, cfg, completedTool, defer, errorTool, infra, it, lsp, makeHttp, mcp, providerCfg, ref, run, seed, status, summary, toolPart, unix, user, withSh } from "./prompt.shared"
import type { CompletedToolPart, ErrorToolPart } from "./prompt.shared"


it.live("does not loop empty assistant turns for a simple reply", () =>
  provideTmpdirServer(
    Effect.fnUntraced(function* ({ llm }) {
      const prompt = yield* SessionPrompt.Service
      const sessions = yield* Session.Service
      const session = yield* sessions.create({ title: "Prompt regression" })

      yield* llm.text("packages/jekko/src/session/processor.ts")

      const result = yield* prompt.prompt({
        sessionID: session.id,
        agent: "build",
        parts: [{ type: "text", text: "Where is SessionProcessor?" }],
      })

      expect(result.info.role).toBe("assistant")
      expect(result.parts.some((part) => part.type === "text" && part.text.includes("processor.ts"))).toBe(true)

      const msgs = yield* sessions.messages({ sessionID: session.id })
      expect(msgs.filter((msg) => msg.info.role === "assistant")).toHaveLength(1)
      expect(yield* llm.calls).toBe(1)
    }),
    { git: true, config: providerCfg },
  ),
)

it.live(
  "records aborted errors when prompt is cancelled mid-stream",
  () =>
    provideTmpdirServer(
      Effect.fnUntraced(function* ({ llm }) {
        const prompt = yield* SessionPrompt.Service
        const sessions = yield* Session.Service
        const session = yield* sessions.create({ title: "Prompt cancel regression" })

        yield* llm.hang

        const fiber = yield* prompt
          .prompt({
            sessionID: session.id,
            agent: "build",
            parts: [{ type: "text", text: "Cancel me" }],
          })
          .pipe(Effect.forkChild)

        yield* llm.wait(1)
        yield* prompt.cancel(session.id)

        const exit = yield* Fiber.await(fiber)
        expect(Exit.isSuccess(exit)).toBe(true)
        if (Exit.isSuccess(exit)) {
          expect(exit.value.info.role).toBe("assistant")
          if (exit.value.info.role === "assistant") {
            expect(exit.value.info.error?.name).toBe("MessageAbortedError")
          }
        }

        const msgs = yield* sessions.messages({ sessionID: session.id })
        const last = msgs.findLast((msg) => msg.info.role === "assistant")
        expect(last?.info.role).toBe("assistant")
        if (last?.info.role === "assistant") {
          expect(last.info.error?.name).toBe("MessageAbortedError")
        }
      }),
      { git: true, config: providerCfg },
    ),
  3_000,
)

// Agent variant

it.live("applies agent variant only when using agent model", () =>
  provideTmpdirInstance(
    (_dir) =>
      Effect.gen(function* () {
        const prompt = yield* SessionPrompt.Service
        const sessions = yield* Session.Service
        const session = yield* sessions.create({})

        const other = yield* prompt.prompt({
          sessionID: session.id,
          agent: "build",
          model: { providerID: ProviderID.make("jekko"), modelID: ModelID.make("kimi-k2.5-free") },
          noReply: true,
          parts: [{ type: "text", text: "hello" }],
        })
        if (other.info.role !== "user") throw new Error("expected user message")
        expect(other.info.model.variant).toBeUndefined()

        const match = yield* prompt.prompt({
          sessionID: session.id,
          agent: "build",
          noReply: true,
          parts: [{ type: "text", text: "hello again" }],
        })
        if (match.info.role !== "user") throw new Error("expected user message")
        expect(match.info.model).toEqual({
          providerID: ProviderID.make("test"),
          modelID: ModelID.make("test-model"),
          variant: "xhigh",
        })
        expect(match.info.model.variant).toBe("xhigh")

        const override = yield* prompt.prompt({
          sessionID: session.id,
          agent: "build",
          noReply: true,
          variant: "high",
          parts: [{ type: "text", text: "hello third" }],
        })
        if (override.info.role !== "user") throw new Error("expected user message")
        expect(override.info.model.variant).toBe("high")

        yield* sessions.remove(session.id)
      }),
    {
      git: true,
      config: {
        ...cfg,
        provider: {
          ...cfg.provider,
          test: {
            ...cfg.provider.test,
            models: {
              "test-model": {
                ...cfg.provider.test.models["test-model"],
                variants: { xhigh: {}, high: {} },
              },
            },
          },
        },
        agent: {
          build: {
            model: "test/test-model",
            variant: "xhigh",
          },
        },
      },
    },
  ),
)

// Agent / command resolution errors

it.live(
  "unknown agent throws typed error",
  () =>
    provideTmpdirInstance(
      (_dir) =>
        Effect.gen(function* () {
          const prompt = yield* SessionPrompt.Service
          const sessions = yield* Session.Service
          const session = yield* sessions.create({})
          const exit = yield* prompt
            .prompt({
              sessionID: session.id,
              agent: "nonexistent-agent-xyz",
              noReply: true,
              parts: [{ type: "text", text: "hello" }],
            })
            .pipe(Effect.exit)

          expect(Exit.isFailure(exit)).toBe(true)
          if (Exit.isFailure(exit)) {
            const err = Cause.squash(exit.cause)
            expect(err).not.toBeInstanceOf(TypeError)
            expect(NamedError.Unknown.isInstance(err)).toBe(true)
            if (NamedError.Unknown.isInstance(err)) {
              expect(err.data.message).toContain('Agent not found: "nonexistent-agent-xyz"')
            }
          }
        }),
      { git: true },
    ),
  30_000,
)

it.live(
  "unknown agent error includes available agent names",
  () =>
    provideTmpdirInstance(
      (_dir) =>
        Effect.gen(function* () {
          const prompt = yield* SessionPrompt.Service
          const sessions = yield* Session.Service
          const session = yield* sessions.create({})
          const exit = yield* prompt
            .prompt({
              sessionID: session.id,
              agent: "nonexistent-agent-xyz",
              noReply: true,
              parts: [{ type: "text", text: "hello" }],
            })
            .pipe(Effect.exit)

          expect(Exit.isFailure(exit)).toBe(true)
          if (Exit.isFailure(exit)) {
            const err = Cause.squash(exit.cause)
            expect(NamedError.Unknown.isInstance(err)).toBe(true)
            if (NamedError.Unknown.isInstance(err)) {
              expect(err.data.message).toContain("build")
            }
          }
        }),
      { git: true },
    ),
  30_000,
)

it.live(
  "unknown command throws typed error with available names",
  () =>
    provideTmpdirInstance(
      (_dir) =>
        Effect.gen(function* () {
          const prompt = yield* SessionPrompt.Service
          const sessions = yield* Session.Service
          const session = yield* sessions.create({})
          const exit = yield* prompt
            .command({
              sessionID: session.id,
              command: "nonexistent-command-xyz",
              arguments: "",
            })
            .pipe(Effect.exit)

          expect(Exit.isFailure(exit)).toBe(true)
          if (Exit.isFailure(exit)) {
            const err = Cause.squash(exit.cause)
            expect(err).not.toBeInstanceOf(TypeError)
            expect(NamedError.Unknown.isInstance(err)).toBe(true)
            if (NamedError.Unknown.isInstance(err)) {
              expect(err.data.message).toContain('Command not found: "nonexistent-command-xyz"')
              expect(err.data.message).toContain("init")
            }
          }
        }),
      { git: true },
    ),
  30_000,
)

test("createStructuredOutputTool rejects array schemas", () => {
  expect(() =>
    SessionPrompt.createStructuredOutputTool({
      schema: [] as unknown as Record<string, any>,
      onSuccess: () => undefined,
    }),
  ).toThrow(TypeError)
})

test("createStructuredOutputTool accepts object schemas", () => {
  const tool = SessionPrompt.createStructuredOutputTool({
    schema: {
      type: "object",
      properties: {
        answer: { type: "string" },
      },
    },
    onSuccess: () => undefined,
  })

  expect(typeof tool.execute).toBe("function")
})

