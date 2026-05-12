import { describe, expect } from "bun:test"
import { Effect, Layer, Schema } from "effect"
import { DaemonCheckpoint } from "../../src/session/daemon-checkpoint"
import { DaemonChecks } from "../../src/session/daemon-checks"
import { Git } from "../../src/git"
import { ZyalScriptSchema } from "../../src/agent-script/schema"
import { testEffect } from "../lib/effect"

function spec() {
  return Schema.decodeUnknownSync(ZyalScriptSchema)({
    version: "v1",
    intent: "daemon",
    confirm: "RUN_FOREVER",
    id: "daemon-checkpoint-test",
    job: {
      name: "Checkpoint test",
      objective: "Exercise checkpoint behavior.",
    },
    stop: {
      all: [{ git_clean: { allow_untracked: false } }],
    },
  })
}

describe("session.daemon-checkpoint", () => {
  let gitStatusCalls = 0
  let gitRunCalls = 0
  const runtime = testEffect(
    DaemonCheckpoint.layer.pipe(
      Layer.provide(
        Layer.succeed(
          Git.Service,
          Git.Service.of({
            run: (_args: string[]) =>
              Effect.sync(() => {
                gitRunCalls += 1
                return {
                  exitCode: 0,
                  text: () => "",
                  stdout: Buffer.from(""),
                  stderr: Buffer.from(""),
                  truncated: false,
                }
              }),
            branch: () => Effect.succeed(undefined),
            prefix: () => Effect.succeed(""),
            defaultBranch: () => Effect.succeed(undefined),
            hasHead: () => Effect.succeed(false),
            mergeBase: () => Effect.succeed(undefined),
            show: () => Effect.succeed(""),
            status: () =>
              Effect.sync(() => {
                gitStatusCalls += 1
                return []
              }),
            diff: () => Effect.succeed([]),
            stats: () => Effect.succeed([]),
            patch: () => Effect.succeed({ text: "", truncated: false }),
            patchAll: () => Effect.succeed({ text: "", truncated: false }),
            patchUntracked: () => Effect.succeed({ text: "", truncated: false }),
            statUntracked: () => Effect.succeed(undefined),
          } as any),
        ),
      ),
      Layer.provide(
        Layer.succeed(
          DaemonChecks.Service,
          DaemonChecks.Service.of({
            runShellCheck: () =>
              Effect.succeed({
                exitCode: 1,
                stdout: "",
                stderr: "boom",
                truncated: false,
                matched: false,
                error: "verification failed",
              }),
            gitClean: () => Effect.succeed({ clean: true, dirty: [] }),
            evaluateJsonPath: () => Effect.succeed(undefined),
          }),
        ),
      ),
    ),
  )

  runtime.effect("halts on verification failure before git operations", Effect.gen(function* () {
    const checkpoint = yield* DaemonCheckpoint.Service
    const result = yield* checkpoint.runCheckpoint({
      cwd: "/tmp/checkpoint",
      spec: spec(),
      checkpoint: {
        verify: [{ command: "false" }],
        git: { add: ["."], push: "ask" },
      },
    })

    expect(result).toEqual({ ok: false, reason: "verification failed" })
    expect(gitStatusCalls).toBe(0)
    expect(gitRunCalls).toBe(0)
  }))

  const followup = testEffect(
    DaemonCheckpoint.layer.pipe(
      Layer.provide(
        Layer.succeed(
          Git.Service,
          Git.Service.of({
            run: (args: string[]) =>
              Effect.sync(() => {
                calls.push([...args])
                if (args[0] === "commit") {
                  return {
                    exitCode: 0,
                    text: () => "commit ok",
                    stdout: Buffer.from("commit ok"),
                    stderr: Buffer.from(""),
                    truncated: false,
                  }
                }
                if (args[0] === "rev-parse") {
                  return {
                    exitCode: 0,
                    text: () => "abc123\n",
                    stdout: Buffer.from("abc123\n"),
                    stderr: Buffer.from(""),
                    truncated: false,
                  }
                }
                return {
                  exitCode: 0,
                  text: () => "",
                  stdout: Buffer.from(""),
                  stderr: Buffer.from(""),
                  truncated: false,
                }
              }),
            branch: () => Effect.succeed(undefined),
            prefix: () => Effect.succeed(""),
            defaultBranch: () => Effect.succeed(undefined),
            hasHead: () => Effect.succeed(true),
            mergeBase: () => Effect.succeed(undefined),
            show: () => Effect.succeed(""),
            status: () => Effect.succeed([{ file: "one.txt", code: " M", status: "modified" as const }]),
            diff: () => Effect.succeed([]),
            stats: () => Effect.succeed([]),
            patch: () => Effect.succeed({ text: "", truncated: false }),
            patchAll: () => Effect.succeed({ text: "", truncated: false }),
            patchUntracked: () => Effect.succeed({ text: "", truncated: false }),
            statUntracked: () => Effect.succeed(undefined),
          } as any),
        ),
      ),
      Layer.provide(
        Layer.succeed(
          DaemonChecks.Service,
          DaemonChecks.Service.of({
            runShellCheck: () =>
              Effect.succeed({
                exitCode: 0,
                stdout: "",
                stderr: "",
                truncated: false,
                matched: true,
              }),
            gitClean: () => Effect.succeed({ clean: false, dirty: ["one.txt"] }),
            evaluateJsonPath: () => Effect.succeed(undefined),
          }),
        ),
      ),
    ),
  )

  const calls: string[][] = []
  followup.effect("returns sha and pauses for approval when push is ask", Effect.gen(function* () {
    const checkpoint = yield* DaemonCheckpoint.Service
    const result = yield* checkpoint.runCheckpoint({
      cwd: "/tmp/checkpoint",
      spec: spec(),
      checkpoint: {
        git: { add: ["."], push: "ask", commit_message: "checkpointed" },
      },
    })

    expect(result).toEqual({ ok: false, reason: "push requires approval", sha: "abc123" })
    expect(calls).toEqual([["add", "."], ["commit", "-m", "checkpointed"], ["rev-parse", "HEAD"]])
  }))
})
