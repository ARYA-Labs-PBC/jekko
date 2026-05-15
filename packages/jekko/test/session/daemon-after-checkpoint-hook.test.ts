import { describe, expect } from "bun:test"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { Effect, Layer, Duration } from "effect"
import { Daemon } from "../../src/session/daemon"
import { DaemonStore } from "../../src/session/daemon-store"
import { DaemonChecks } from "../../src/session/daemon-checks"
import { DaemonCheckpoint } from "../../src/session/daemon-checkpoint"
import { MCP } from "../../src/mcp"
import { ProjectTable } from "../../src/project/project.sql"
import { ProjectID } from "../../src/project/schema"
import { SessionTable } from "../../src/session/session.sql"
import { Session } from "../../src/session/session"
import { SessionID } from "../../src/session/schema"
import { SessionPrompt } from "../../src/session/prompt"
import { Worktree } from "../../src/worktree"
import { Database } from "../../src/storage/db"
import { testEffect } from "../lib/effect"
import { provideTmpdirInstance } from "../fixture/fixture"

describe("daemon after_checkpoint PR-push hook lifecycle", () => {
  const it = testEffect(Layer.mergeAll(Daemon.defaultLayer, DaemonStore.defaultLayer, CrossSpawnSpawner.defaultLayer))

  function seedProjectAndSession(input: {
    projectID: string
    sessionID: string
    directory: string
  }) {
    return Effect.sync(() =>
      Database.use((db) => {
        const now = Date.now()
        db.insert(ProjectTable)
          .values({
            id: input.projectID,
            worktree: "/",
            vcs: "git",
            name: "Daemon hook lifecycle test",
            sandboxes: [],
            time_created: now,
            time_updated: now,
          })
          .run()
        db.insert(SessionTable)
          .values({
            id: input.sessionID,
            project_id: input.projectID,
            slug: "daemon-hook-test",
            directory: input.directory,
            title: "daemon hook test",
            version: "1.0.0",
            time_created: now,
            time_updated: now,
          })
          .run()
      }),
    )
  }

  function sessionService(input: { sessionID: string; directory: string }) {
    const session = {
      id: input.sessionID,
      directory: input.directory,
      permission: [],
      agent: "build",
      model: undefined,
    } as any
    return Session.Service.of({
      get: () => Effect.succeed(session),
      setPermission: () => Effect.void,
    })
  }

  function promptService(hookCommand: string, calls: string[]) {
    let loopCalls = 0
    return SessionPrompt.Service.of({
      cancel: () => Effect.void,
      prompt: () => Effect.succeed({} as any),
      loop: () => Effect.never,
      loopResult: () => {
        loopCalls += 1
        calls.push(`loop-${loopCalls}`)
        return Effect.succeed({
          message: {
            info: {
              id: `assistant-${loopCalls}`,
              role: "assistant",
              finish: "stop",
              tokens: { input: 11, output: 13, total: 24 },
              cost: 0.02,
            },
          },
          terminal: "assistant_stop",
        })
      },
      shell: () => Effect.never,
      command: () => Effect.never,
      resolvePromptParts: () => Effect.succeed([] as any),
    })
  }

  function checksService(shellCommands: string[]) {
    return DaemonChecks.Service.of({
      runShellCheck: (input: { command: string }) =>
        Effect.sync(() => {
          shellCommands.push(input.command)
          return {
            exitCode: 0,
            stdout: "",
            stderr: "",
            truncated: false,
            matched: true,
          }
        }),
      gitClean: () => Effect.succeed({ clean: true, dirty: [] }),
      evaluateJsonPath: () => Effect.succeed(undefined),
    })
  }

  function checkpointService(checkpointCalls: string[]) {
    return {
      runCheckpoint: () => {
        checkpointCalls.push("runCheckpoint")
        return Effect.succeed({ ok: true })
      },
    } as unknown as any
  }

  function mcpService() {
    return MCP.Service.of({
      status: () => Effect.succeed({}),
      clients: () => Effect.succeed({}),
      tools: () => Effect.succeed({}),
      prompts: () => Effect.succeed({}),
      resources: () => Effect.succeed({}),
      add: () => Effect.succeed({ status: { status: "disabled" } }),
      connect: () => Effect.void,
      disconnect: () => Effect.void,
      getPrompt: () => Effect.succeed(undefined),
      readResource: () => Effect.succeed(undefined),
      startAuth: () => Effect.die("unexpected mcp auth"),
      authenticate: () => Effect.die("unexpected mcp auth"),
      finishAuth: () => Effect.die("unexpected mcp auth"),
      removeAuth: () => Effect.void,
      supportsOAuth: () => Effect.succeed(false),
      hasStoredTokens: () => Effect.succeed(false),
      getAuthStatus: () => Effect.succeed("not_authenticated" as const),
    })
  }

  function worktreeService() {
    return Worktree.Service.of({
      makeWorktreeInfo: () => Effect.die("unexpected worktree"),
      createFromInfo: () => Effect.void,
      create: () => Effect.die("unexpected worktree"),
      remove: () => Effect.succeed(true),
      reset: () => Effect.succeed(true),
    })
  }

  async function waitForTerminal(daemon: Daemon.Service, runID: string) {
    for (let attempt = 0; attempt < 200; attempt += 1) {
      const current = await Effect.runPromise(
        daemon.get(runID).pipe(
          Effect.map((item) => item ?? undefined),
        ),
      )
      if (current?.status === "satisfied" || current?.status === "failed" || current?.status === "paused") {
        return current
      }
      await Effect.runPromise(Effect.sleep(Duration.millis(25)))
    }
    return undefined
  }

  const zyalWithAfterCheckpointHook = `<<<ZYAL v1:daemon id=daemon-after-checkpoint-hook>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: "After-checkpoint hook lifecycle"
  objective: "Verify ZYAL after_checkpoint hooks execute before daemon stop"
loop:
  policy: once
stop:
  all:
    - shell:
        command: "true"
hooks:
  after_checkpoint:
    - run: "gh pr create --draft --fill --title 'daemon bug fix (hook)'"
<<<END_ZYAL id=daemon-after-checkpoint-hook>>>
ZYAL_ARM RUN_FOREVER id=daemon-after-checkpoint-hook
`

  it.effect(
    "runs after_checkpoint hook in daemon lifecycle after a successful checkpoint",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const projectID = ProjectID.make("proj_daemon_after_checkpoint")
          const sessionID = SessionID.make("ses_daemon_after_checkpoint")
          yield* seedProjectAndSession({
            projectID,
            sessionID,
            directory,
          })

          const hookCommand = "gh pr create --draft --fill --title 'daemon bug fix (hook)'"
          const shellCommands: string[] = []
          const checkpointCalls: string[] = []
          const promptCalls: string[] = []
          const daemon = yield* Daemon.Service

          const run = yield* daemon.start({
            sessionID,
            prompt: {
              parts: [{ type: "text", text: zyalWithAfterCheckpointHook }],
              agent: "build",
              noReply: true,
            } as any,
          }).pipe(
            Effect.provideService(Session.Service, sessionService({ sessionID, directory })),
            Effect.provideService(SessionPrompt.Service, promptService(hookCommand, promptCalls)),
            Effect.provideService(DaemonChecks.Service, checksService(shellCommands)),
            Effect.provideService(MCP.Service, mcpService()),
            Effect.provideService(Worktree.Service, worktreeService()),
            Effect.provideService(DaemonCheckpoint.Service, checkpointService(checkpointCalls)),
          )

          const final = yield* Effect.promise(() => waitForTerminal(daemon, run.id))
          expect(final?.status).toBe("satisfied")
          expect(checkpointCalls).toEqual(["runCheckpoint"])
          expect(shellCommands[0]).toBe(hookCommand)
          expect(shellCommands).toContain("true")
          expect(promptCalls).toEqual(["loop-1"])

          const events = yield* daemon.events(run.id)
          const afterHook = events.find((event) => event.event_type === "hook.after_checkpoint")
          expect(afterHook?.payload_json).toEqual({ command: hookCommand })
          expect(afterHook).toBeDefined()

          const hookOnlyCalls = shellCommands.filter((command) => command === hookCommand)
          expect(hookOnlyCalls).toHaveLength(1)
        }),
      { git: true },
    ),
  )
})
