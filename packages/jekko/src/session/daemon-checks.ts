// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import { ChildProcess, ChildProcessSpawner } from "effect/unstable/process"
import { Cause, Effect, Exit, Fiber, Layer, Context, Schema } from "effect"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { Config } from "@/config/config"
import { Shell } from "@/shell/shell"
import { Git } from "@/git"
import { parseDuration } from "./daemon-retry"
import { collectStreamOutput } from "@/util/process-output"
import * as Log from "@jekko-ai/core/util/log"

const log = Log.create({ service: "daemon-checks" })

export const ShellCheckResult = Schema.Struct({
  exitCode: Schema.Number,
  stdout: Schema.String,
  stderr: Schema.String,
  truncated: Schema.Boolean,
  matched: Schema.Boolean,
  error: Schema.optional(Schema.String),
})

export type ShellCheckResult = Schema.Schema.Type<typeof ShellCheckResult>

export interface Interface {
  readonly runShellCheck: (input: {
    cwd: string
    command: string
    timeout?: string
    assert?: {
      exit_code?: number
      stdout_contains?: string[]
      stdout_regex?: string[]
      json?: Record<string, unknown>
    }
  }) => Effect.Effect<ShellCheckResult, any, any>
  readonly gitClean: (input: { cwd: string; allowUntracked?: boolean }) => Effect.Effect<{ clean: boolean; dirty: string[] }, any, any>
  readonly evaluateJsonPath: (input: { value: unknown; path: string }) => Effect.Effect<unknown, any, any>
}

export class Service extends Context.Service<Service, Interface>()("@jekko/DaemonChecks") {}

export const layer = Layer.effect(
  Service,
  Effect.gen(function* () {
    const config = yield* Config.Service
    const spawner = yield* ChildProcessSpawner.ChildProcessSpawner
    const git = yield* Git.Service

    const runShellCheck = Effect.fn("DaemonChecks.runShellCheck")(function* (input: {
      cwd: string
      command: string
      timeout?: string
      assert?: {
        exit_code?: number
        stdout_contains?: string[]
        stdout_regex?: string[]
        json?: Record<string, unknown>
      }
    }) {
      return yield* Effect.scoped(
        Effect.gen(function* () {
          const shell = Shell.preferred((yield* config.get()).shell)
          const args = Shell.args(shell, input.command, input.cwd)
          log.info("shell check started", {
            cwd: input.cwd,
            shell,
            timeout: input.timeout ?? "30 seconds",
          })
          const proc = ChildProcess.make(shell, args, {
            cwd: input.cwd,
            extendEnv: true,
            env: { TERM: "dumb" },
            stdin: "ignore",
            stdout: "pipe",
            stderr: "pipe",
          })
          const handle = yield* spawner.spawn(proc)
          const stdoutFiber = yield* Effect.forkScoped(collectStreamOutput(handle.stdout, 64_000))
          const stderrFiber = yield* Effect.forkScoped(collectStreamOutput(handle.stderr, 64_000))
          const exit = yield* Effect.raceAll([
            handle.exitCode.pipe(Effect.map((code) => ({ kind: "exit" as const, code }))),
            Effect.sleep(parseDuration(input.timeout ?? "30 seconds")).pipe(
              Effect.map(() => ({ kind: "timeout" as const, code: 124 })),
            ),
          ])
          if (exit.kind === "timeout") {
            log.warn("shell check timed out; killing child", {
              cwd: input.cwd,
              shell,
              command: input.command,
              timeout: input.timeout ?? "30 seconds",
            })
            yield* handle.kill({ forceKillAfter: "3 seconds" }).pipe(Effect.orDie)
          }
          const stdout = yield* Fiber.await(stdoutFiber).pipe(
            Effect.flatMap((result) =>
              Exit.isSuccess(result)
                ? Effect.succeed(result.value)
                : Effect.failCause(result.cause),
            ),
          )
          const stderr = yield* Fiber.await(stderrFiber).pipe(
            Effect.flatMap((result) =>
              Exit.isSuccess(result)
                ? Effect.succeed(result.value)
                : Effect.failCause(result.cause),
            ),
          )
          const expectedExitCode = input.assert?.exit_code ?? 0
          const jsonPath = input.assert?.json ? checkJsonAssertions(stdout.text, input.assert.json) : true
          const matched =
            expectedExitCode === exit.code &&
            (input.assert?.stdout_contains?.every((needle) => stdout.text.includes(needle)) ?? true) &&
            (input.assert?.stdout_regex?.every((pattern) => new RegExp(pattern, "m").test(stdout.text)) ?? true) &&
            jsonPath
          log.info("shell check completed", {
            cwd: input.cwd,
            command: input.command,
            exitCode: exit.code,
            matched,
            truncated: stdout.truncated || stderr.truncated,
          })
          const stdoutText = stdout.buffer.toString("utf8")
          const stderrText = stderr.buffer.toString("utf8")
          const truncated = stdout.truncated || stderr.truncated
          return {
            exitCode: exit.code,
            stdout: stdoutText,
            stderr: stderrText,
            truncated,
            matched,
            error: matched
              ? undefined
              : summarizeShellFailure({
                  command: input.command,
                  exitCode: exit.code,
                  stdout: stdoutText,
                  stderr: stderrText,
                  truncated,
                }),
          }
        }),
      )
    })

    const gitClean = Effect.fn("DaemonChecks.gitClean")(function* (input: { cwd: string; allowUntracked?: boolean }) {
      const status = yield* git.status(input.cwd)
      const dirty = status.filter((item) => {
        if (input.allowUntracked && item.code === "??") return false
        return true
      })
      return { clean: dirty.length === 0, dirty: dirty.map((item) => item.file) }
    })

    const evaluateJsonPath = Effect.fn("DaemonChecks.evaluateJsonPath")(function* (input: { value: unknown; path: string }) {
      return walkJsonPath(input.value, input.path)
    })

    return Service.of({ runShellCheck, gitClean, evaluateJsonPath })
  }),
)

export const defaultLayer = layer.pipe(
  Layer.provide(Config.defaultLayer),
  Layer.provide(Git.defaultLayer),
  Layer.provide(CrossSpawnSpawner.defaultLayer),
)

export function walkJsonPath(value: unknown, pointer: string): unknown {
  if (!pointer.startsWith("$")) throw new Error(`JSON path must start with $: ${pointer}`)
  const tokens: (string | number)[] = []
  for (const segment of pointer.slice(1).split(".")) {
    if (!segment) continue
    let index = 0
    while (index < segment.length) {
      if (segment[index] === "[") {
        const end = segment.indexOf("]", index + 1)
        // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
        if (end === -1) return undefined
        const rawIndex = segment.slice(index + 1, end)
        // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
        if (!rawIndex || !isDigits(rawIndex)) return undefined
        tokens.push(Number(rawIndex))
        index = end + 1
        continue
      }
      const start = index
      while (index < segment.length && segment[index] !== "[") index += 1
      const key = segment.slice(start, index)
      if (key) tokens.push(key)
    }
  }
  let current: unknown = value
  for (const token of tokens) {
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (current == null) return undefined
    if (typeof token === "number") {
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      if (!Array.isArray(current)) return undefined
      current = current[token]
    } else {
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      if (typeof current !== "object" || Array.isArray(current)) return undefined
      current = (current as Record<string, unknown>)[token]
    }
  }
  return current
}

function isDigits(value: string) {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index)
    if (code < 48 || code > 57) return false
  }
  return value.length > 0
}

function checkJsonAssertions(stdout: string, expected: Record<string, unknown>) {
  try {
    const parsed: unknown = JSON.parse(stdout)
    for (const [path, value] of Object.entries(expected)) {
      const found = walkJsonPath(parsed, path)
      if (!Object.is(found, value)) return false
    }
    return true
  } catch {
    return false
  }
}

function summarizeShellFailure(input: {
  command: string
  exitCode: number
  stdout: string
  stderr: string
  truncated: boolean
}) {
  return [
    `command failed: ${input.command}`,
    `exitCode=${input.exitCode}`,
    `stdout=${excerpt(input.stdout)}`,
    `stderr=${excerpt(input.stderr)}`,
    input.truncated ? "truncated=true" : undefined,
  ]
    .filter(Boolean)
    .join("; ")
}

function excerpt(value: string, limit = 240) {
  const normalized = value.replace(/\s+/g, " ").trim()
  if (!normalized) return "(empty)"
  return normalized.length > limit ? `${normalized.slice(0, limit - 1)}…` : normalized
}

export * as DaemonChecks from "./daemon-checks"
