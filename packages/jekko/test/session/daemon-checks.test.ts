import { describe, expect } from "bun:test"
import { Effect, Layer, Stream } from "effect"
import { ChildProcessSpawner } from "effect/unstable/process"
import { Config } from "../../src/config/config"
import { Git } from "../../src/git"
import { testEffect } from "../lib/effect"
import { TestInstance } from "../fixture/fixture"
import { DaemonChecks } from "../../src/session/daemon-checks"

const it = testEffect(DaemonChecks.defaultLayer)
const mockIt = testEffect(
  DaemonChecks.layer.pipe(
    Layer.provide(Layer.mergeAll(Config.defaultLayer, Git.defaultLayer, mockSpawnerLayer({ exitCode: 0, stdout: "ok" }))),
  ),
)

const encoder = new TextEncoder()

function mockSpawnerLayer(input: { exitCode: number; stdout?: string; stderr?: string }) {
  const spawner = ChildProcessSpawner.make((command) =>
    Effect.succeed(
      ChildProcessSpawner.makeHandle({
        pid: ChildProcessSpawner.ProcessId(0),
        exitCode: Effect.succeed(ChildProcessSpawner.ExitCode(input.exitCode)),
        isRunning: Effect.succeed(false),
        kill: () => Effect.void,
        stdin: { [Symbol.for("effect/Sink/TypeId")]: Symbol.for("effect/Sink/TypeId") } as any,
        stdout: input.stdout ? Stream.make(encoder.encode(input.stdout)) : Stream.empty,
        stderr: input.stderr ? Stream.make(encoder.encode(input.stderr)) : Stream.empty,
        all: Stream.empty,
        getInputFd: () => ({ [Symbol.for("effect/Sink/TypeId")]: Symbol.for("effect/Sink/TypeId") }) as any,
        getOutputFd: () => Stream.empty,
        unref: Effect.succeed(Effect.void),
      }),
    ),
  )
  return Layer.succeed(ChildProcessSpawner.ChildProcessSpawner, spawner)
}

describe("daemon checks", () => {
  mockIt.instance("defaults shell checks to exit code 0", () =>
    Effect.gen(function* () {
      const test = yield* TestInstance
      const checks = yield* DaemonChecks.Service
      const result = yield* checks.runShellCheck({
        cwd: test.directory,
        command: "ignored by mock spawner",
      })

      expect(result.exitCode).toBe(0)
      expect(result.matched).toBe(true)
      expect(result.stdout).toBe("ok")
    }),
  )

  it.instance("treats missing files as failed stop checks", () =>
    Effect.gen(function* () {
      const test = yield* TestInstance
      const checks = yield* DaemonChecks.Service
      const result = yield* checks.runShellCheck({
        cwd: test.directory,
        command: "test -f missing-file",
      })

      expect(result.exitCode).not.toBe(0)
      expect(result.matched).toBe(false)
      expect(result.error).toBe("shell assertion failed")
    }),
  )
})
