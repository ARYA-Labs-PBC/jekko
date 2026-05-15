import { describe, expect } from "bun:test"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { Duration, Effect, Layer } from "effect"
import { mkdir, mkdtemp, readFile, rm, writeFile } from "fs/promises"
import os from "os"
import path from "path"
import { Daemon } from "../../src/session/daemon"
import { DaemonChecks } from "../../src/session/daemon-checks"
import { DaemonCheckpoint } from "../../src/session/daemon-checkpoint"
import { DaemonJankurai } from "../../src/session/daemon-jankurai"
import { DaemonStore } from "../../src/session/daemon-store"
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

async function runShell(cwd: string, command: string) {
  const process = Bun.spawn(["/bin/sh", "-lc", command], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  })
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
    process.exited,
  ])
  return { exitCode, stdout, stderr }
}

async function initGitRepo(cwd: string) {
  const result = await runShell(
    cwd,
    [
      "git init",
      "git config core.fsmonitor false",
      "git config commit.gpgsign false",
      "git config user.email \"test@jekko.test\"",
      "git config user.name \"Test\"",
      "git commit --allow-empty -m root",
    ].join(" && "),
  )
  if (result.exitCode !== 0) throw new Error(`failed to initialize git repo in ${cwd}: ${result.stderr || result.stdout}`)
  const ignore = await runShell(cwd, `printf 'target/\\n' > .gitignore && git add .gitignore && git commit -m "ignore target"`)
  if (ignore.exitCode !== 0) throw new Error(`failed to add worker ignore rules in ${cwd}: ${ignore.stderr || ignore.stdout}`)
}

async function ignoreJekkoArtifacts(cwd: string) {
  await writeFile(path.join(cwd, ".gitignore"), ".jekko/\ntarget/\n")
  const result = await runShell(cwd, "git add .gitignore && git commit -m \"ignore jekko\"")
  if (result.exitCode !== 0) throw new Error(`failed to ignore jekko artifacts in ${cwd}: ${result.stderr || result.stdout}`)
}

describe("daemon jankurai runtime", () => {
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
            name: "Daemon Jankurai Runtime Test",
            sandboxes: [],
            time_created: now,
            time_updated: now,
          })
          .run()
        db.insert(SessionTable)
          .values({
            id: input.sessionID,
            project_id: input.projectID,
            slug: "daemon-jankurai-runtime",
            directory: input.directory,
            title: "Daemon Jankurai Runtime Test",
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

  function promptService(prompts: string[]) {
    let loopCalls = 0
    return SessionPrompt.Service.of({
      cancel: () => Effect.void,
      prompt: (input: { parts?: { type: string; text?: string }[] }) =>
        Effect.sync(() => {
          const text = input.parts?.map((part) => (part.type === "text" ? part.text ?? "" : "")).join("\n") ?? ""
          prompts.push(text)
        }),
      loop: () => Effect.never,
      loopResult: () => {
        loopCalls += 1
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

  function checksService(directory: string, shellCommands: string[]) {
    return DaemonChecks.Service.of({
      runShellCheck: (input: { command: string; cwd: string }) =>
        Effect.promise(async () => {
          shellCommands.push(input.command)
          if (input.command.includes("jankurai audit")) {
            const reportPath = path.join(input.cwd, "target", "jankurai", "repo-score.json")
            await mkdir(path.dirname(reportPath), { recursive: true })
            await writeFile(
              reportPath,
              JSON.stringify({
                score: 80,
                findings: [],
              }),
            )
            return {
              exitCode: 0,
              stdout: "",
              stderr: "",
              truncated: false,
              matched: true,
            }
          }
          if (input.command.includes("repair-plan")) {
            const planPath = path.join(input.cwd, "target", "jankurai", "repair-plan.json")
            await mkdir(path.dirname(planPath), { recursive: true })
            await writeFile(
              planPath,
              JSON.stringify({
                packets: [],
              }),
            )
            return {
              exitCode: 0,
              stdout: "",
              stderr: "",
              truncated: false,
              matched: true,
            }
          }
          if (
            input.command.startsWith("git status --porcelain") ||
            input.command.startsWith("git diff --check") ||
            input.command.startsWith("git apply")
          ) {
            const result = await runShell(input.cwd, input.command)
            return {
              exitCode: result.exitCode,
              stdout: result.stdout,
              stderr: result.stderr,
              truncated: false,
              matched: result.exitCode === 0,
              error: result.exitCode === 0 ? undefined : `command failed: ${input.command}`,
            }
          }
          if (input.command.startsWith("git add -N . && git diff --binary HEAD >")) {
            const result = await runShell(input.cwd, input.command)
            return {
              exitCode: result.exitCode,
              stdout: result.stdout,
              stderr: result.stderr,
              truncated: false,
              matched: result.exitCode === 0,
              error: result.exitCode === 0 ? undefined : `command failed: ${input.command}`,
            }
          }
          if (input.command === "true") {
            return {
              exitCode: 0,
              stdout: "",
              stderr: "",
              truncated: false,
              matched: true,
            }
          }
          return {
            exitCode: 0,
            stdout: "",
            stderr: "",
            truncated: false,
            matched: true,
          }
        }),
      gitClean: (input: { cwd: string }) =>
        Effect.promise(async () => {
          const result = await runShell(input.cwd, "git status --porcelain")
          const dirty = result.stdout
            .split(/\r?\n/)
            .map((line) => line.trim())
            .filter(Boolean)
          return { clean: dirty.length === 0, dirty }
        }),
      evaluateJsonPath: () => Effect.succeed(undefined),
    })
  }

  function checkpointService() {
    return {
      runCheckpoint: () => Effect.succeed({ ok: true }),
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

  function makeWorkerStore() {
    return {
      listTaskMemory: () => Effect.succeed([] as any[]),
      upsertWorker: (input: unknown) => Effect.succeed(input as any),
      appendEvent: () => Effect.succeed({ id: "event" } as any),
      appendTaskMemory: (input: unknown) => Effect.succeed(input as any),
      blockTask: () => Effect.void,
      completeTask: () => Effect.void,
      upsertArtifact: (input: unknown) => Effect.succeed({ ...(input as any), id: "artifact" }),
      listWorkers: () => Effect.succeed([] as any[]),
      listTasks: () => Effect.succeed([] as any[]),
    } as any
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

  function makeZyal() {
    return `<<<ZYAL v1:daemon id=daemon-jankurai-runtime>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: "Daemon Jankurai Runtime"
  objective: "Exercise daemon jankurai fallbacks"
loop:
  policy: once
stop:
  all:
    - shell:
        command: "true"
jankurai:
  enabled: true
  selection:
    max_risk: low
    skip_human_review_required: true
  verification:
    require_clean_start: false
    commands: ["true"]
<<<END_ZYAL id=daemon-jankurai-runtime>>>
ZYAL_ARM RUN_FOREVER id=daemon-jankurai-runtime
`
  }

  function makeDaemonWorkerSpec() {
    return {
      version: "v1",
      intent: "daemon",
      confirm: "RUN_FOREVER",
      job: {
        name: "Daemon Jankurai Worker",
        objective: "Exercise worker patch capture.",
      },
      loop: {
        policy: "once",
      },
      stop: {
        all: [
          {
            shell: {
              command: "true",
            },
          },
        ],
      },
      jankurai: {
        enabled: true,
        selection: {
          max_risk: "low",
          skip_human_review_required: true,
        },
        verification: {
          require_clean_start: false,
          proof_from_test_map: false,
          commands: ["true"],
          audit_delta: "no_new_findings",
          rollback_unverified: true,
        },
      },
    } as any
  }

  it.effect(
    "reuses valid seeded artifacts on the first iteration",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const projectID = ProjectID.make("proj_daemon_jankurai_seed")
          const sessionID = SessionID.make("ses_daemon_jankurai_seed")
          yield* seedProjectAndSession({ projectID, sessionID, directory })

          yield* Effect.promise(() => mkdir(path.join(directory, "target", "jankurai"), { recursive: true }))
          yield* Effect.promise(() =>
            writeFile(path.join(directory, "target", "jankurai", "repo-score.json"), JSON.stringify({ score: 80, findings: [] })),
          )
          yield* Effect.promise(() =>
            writeFile(path.join(directory, "target", "jankurai", "repair-plan.json"), JSON.stringify({ packets: [] })),
          )

          const shellCommands: string[] = []
          const prompts: string[] = []
          const daemon = yield* Daemon.Service
          const run = yield* daemon.start({
            sessionID,
            prompt: {
              parts: [{ type: "text", text: makeZyal() }],
              agent: "build",
              noReply: true,
            } as any,
          }).pipe(
            Effect.provideService(Session.Service, sessionService({ sessionID, directory })),
            Effect.provideService(SessionPrompt.Service, promptService(prompts)),
            Effect.provideService(DaemonChecks.Service, checksService(directory, shellCommands)),
            Effect.provideService(MCP.Service, mcpService()),
            Effect.provideService(Worktree.Service, worktreeService()),
            Effect.provideService(DaemonCheckpoint.Service, checkpointService()),
          )

          const final = yield* Effect.promise(() => waitForTerminal(daemon, run.id))
          expect(final?.status).toBe("satisfied")

          const events = yield* daemon.events(run.id)
          expect(events.some((event) => event.event_type === "jankurai.seeded_artifacts.reused")).toBe(true)
          expect(events.some((event) => event.event_type === "jankurai.worker_wave.completed")).toBe(true)
          expect(events.find((event) => event.event_type === "jankurai.worker_wave.completed")?.payload_json).toMatchObject({
            started: 0,
            reason: "no conflict-free task",
          })
          expect(shellCommands.some((command) => command.includes("jankurai audit"))).toBe(true)
        }),
      { git: true },
    ),
  )

  it.effect(
    "blocks worker tasks that return no diff",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const sessionID = SessionID.make("ses_daemon_jankurai_worker_zero_diff")
          yield* Effect.promise(() => ignoreJekkoArtifacts(directory))
          yield* Effect.promise(() => mkdir(path.join(directory, "target", "jankurai"), { recursive: true }))
          yield* Effect.promise(() =>
            writeFile(path.join(directory, "target", "jankurai", "repo-score.json"), JSON.stringify({ score: 80, findings: [] })),
          )

          const worktreeDir = yield* Effect.promise(() => mkdtemp(path.join(os.tmpdir(), "jekko-worker-zero-")))
          yield* Effect.promise(() => initGitRepo(worktreeDir))

          const shellCommands: string[] = []
          const prompts: string[] = []
          const run = {
            id: "run-worker-zero",
            active_session_id: sessionID,
            iteration: 0,
          } as any
          const task = {
            id: "task-zero-diff",
            external_id: "sha256:task-zero-diff",
            title: "Zero diff task",
            body_json: {
              locked_paths: [],
              proof_commands: ["true"],
              risk: "low",
            },
            blocked_reason: null,
            locked_paths_json: [],
          } as any
          const store = makeWorkerStore()

          const result = yield* DaemonJankurai.runWorkerTask({
            cwd: directory,
            run,
            task,
            workerID: "worker-zero",
            config: DaemonJankurai.resolveJankuraiConfig(makeDaemonWorkerSpec())!,
            sessions: {
              create: () =>
                Effect.succeed({
                  id: sessionID,
                  slug: "worker-zero",
                  directory: worktreeDir,
                  title: "worker-zero",
                  version: "1.0.0",
                  time: { created: Date.now(), updated: Date.now() },
                } as any),
            } as any,
            prompt: promptService(prompts),
            store,
            checks: checksService(directory, shellCommands),
            worktree: {
              create: () => Effect.succeed({ name: "worker-zero", branch: "worker-zero", directory: worktreeDir }),
              remove: () => Effect.promise(() => rm(worktreeDir, { recursive: true, force: true })),
            } as any,
          })

          expect(result.ok).toBe(false)
          expect(result.reason).toBe("worker produced no diff")
          expect(result.patchPath).toBeNull()
          expect(shellCommands).toContain("git status --porcelain")
        }),
      { git: true },
    ),
  )

  it.effect(
    "captures untracked files into worker patches",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const sessionID = SessionID.make("ses_daemon_jankurai_worker_patch")
          yield* Effect.promise(() => ignoreJekkoArtifacts(directory))
          yield* Effect.promise(() => mkdir(path.join(directory, "target", "jankurai"), { recursive: true }))
          yield* Effect.promise(() =>
            writeFile(path.join(directory, "target", "jankurai", "repo-score.json"), JSON.stringify({ score: 80, findings: [] })),
          )

          const worktreeDir = yield* Effect.promise(() => mkdtemp(path.join(os.tmpdir(), "jekko-worker-patch-")))
          yield* Effect.promise(() => initGitRepo(worktreeDir))
          yield* Effect.promise(() => writeFile(path.join(worktreeDir, "new-file.txt"), "untracked change\n"))

          const shellCommands: string[] = []
          const prompts: string[] = []
          const run = {
            id: "run-worker-patch",
            active_session_id: sessionID,
            iteration: 0,
          } as any
          const task = {
            id: "task-worker-patch",
            external_id: "sha256:task-worker-patch",
            title: "Patch task",
            body_json: {
              locked_paths: [],
              proof_commands: ["true"],
              risk: "low",
            },
            blocked_reason: null,
            locked_paths_json: [],
          } as any
          const store = makeWorkerStore()

          const result = yield* DaemonJankurai.runWorkerTask({
            cwd: directory,
            run,
            task,
            workerID: "worker-patch",
            config: DaemonJankurai.resolveJankuraiConfig(makeDaemonWorkerSpec())!,
            sessions: {
              create: () =>
                Effect.succeed({
                  id: sessionID,
                  slug: "worker-patch",
                  directory: worktreeDir,
                  title: "worker-patch",
                  version: "1.0.0",
                  time: { created: Date.now(), updated: Date.now() },
                } as any),
            } as any,
            prompt: promptService(prompts),
            store,
            checks: checksService(directory, shellCommands),
            worktree: {
              create: () => Effect.succeed({ name: "worker-patch", branch: "worker-patch", directory: worktreeDir }),
              remove: () => Effect.promise(() => rm(worktreeDir, { recursive: true, force: true })),
            } as any,
          })

          expect(result.ok).toBe(true)
          expect(result.patchPath).toContain("worker.patch")
          const patch = yield* Effect.promise(() => readFile(result.patchPath, "utf8"))
          expect(shellCommands).toContain("git status --porcelain")
          expect(shellCommands.some((command) => command.startsWith("git add -N . && git diff --binary HEAD >"))).toBe(true)
          expect(patch).toContain("new-file.txt")
          expect(yield* Effect.promise(() => readFile(path.join(directory, "new-file.txt"), "utf8"))).toBe("untracked change\n")
        }),
      { git: true },
    ),
  )

  it.effect(
    "falls back cleanly when seeded artifacts are corrupt",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const projectID = ProjectID.make("proj_daemon_jankurai_fallback")
          const sessionID = SessionID.make("ses_daemon_jankurai_fallback")
          yield* seedProjectAndSession({ projectID, sessionID, directory })

          yield* Effect.promise(() => mkdir(path.join(directory, "target", "jankurai"), { recursive: true }))
          yield* Effect.promise(() => writeFile(path.join(directory, "target", "jankurai", "repo-score.json"), "{"))
          yield* Effect.promise(() => writeFile(path.join(directory, "target", "jankurai", "repair-plan.json"), "{"))

          const shellCommands: string[] = []
          const prompts: string[] = []
          const daemon = yield* Daemon.Service
          const run = yield* daemon.start({
            sessionID,
            prompt: {
              parts: [{ type: "text", text: makeZyal() }],
              agent: "build",
              noReply: true,
            } as any,
          }).pipe(
            Effect.provideService(Session.Service, sessionService({ sessionID, directory })),
            Effect.provideService(SessionPrompt.Service, promptService(prompts)),
            Effect.provideService(DaemonChecks.Service, checksService(directory, shellCommands)),
            Effect.provideService(MCP.Service, mcpService()),
            Effect.provideService(Worktree.Service, worktreeService()),
            Effect.provideService(DaemonCheckpoint.Service, checkpointService()),
          )

          const final = yield* Effect.promise(() => waitForTerminal(daemon, run.id))
          expect(final?.status).toBe("satisfied")

          const events = yield* daemon.events(run.id)
          expect(events.some((event) => event.event_type === "jankurai.seeded_artifacts.invalid")).toBe(true)
          expect(events.some((event) => event.event_type === "jankurai.audit.started")).toBe(true)
          expect(events.some((event) => event.event_type === "jankurai.repair_plan.completed")).toBe(true)
          expect(events.find((event) => event.event_type === "jankurai.worker_wave.completed")?.payload_json).toMatchObject({
            started: 0,
            reason: "no conflict-free task",
          })
          expect(shellCommands.some((command) => command.includes("jankurai audit"))).toBe(true)
        }),
      { git: true },
    ),
  )
})
