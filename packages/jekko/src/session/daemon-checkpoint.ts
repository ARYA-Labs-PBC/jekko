import { Effect, Layer, Context } from "effect"
import { Git } from "@/git"
import { DaemonChecks } from "./daemon-checks"
import type { ZyalCheckpoint, ZyalScript } from "@/agent-script/schema"

export type CheckpointResult = {
  readonly ok: boolean
  readonly sha?: string
  readonly reason?: string
}

export interface Interface {
  readonly runCheckpoint: (input: {
    cwd: string
    spec: ZyalScript
    checkpoint?: ZyalCheckpoint
  }) => Effect.Effect<CheckpointResult, any, any>
}

export class Service extends Context.Service<Service, Interface>()("@jekko/DaemonCheckpoint") {}

export const layer = Layer.effect(
  Service,
  Effect.gen(function* () {
    const git = yield* Git.Service
    const checks = yield* DaemonChecks.Service

    const runCheckpoint = Effect.fn("DaemonCheckpoint.runCheckpoint")(function* (input: {
      cwd: string
      spec: ZyalScript
      checkpoint?: ZyalCheckpoint
    }) {
      if (!input.checkpoint) return { ok: true }
      const verify = input.checkpoint.verify ?? []
      for (const check of verify) {
        const result = yield* checks.runShellCheck({
          cwd: check.cwd ?? input.cwd,
          command: check.command,
          timeout: check.timeout,
          assert: check.assert
            ? {
                exit_code: check.assert.exit_code,
                stdout_contains: check.assert.stdout_contains ? [...check.assert.stdout_contains] : undefined,
                stdout_regex: check.assert.stdout_regex ? [...check.assert.stdout_regex] : undefined,
                json: check.assert.json ? { ...check.assert.json } : undefined,
              }
            : undefined,
        })
        if (!result.matched) return { ok: false, reason: result.error ?? "verification failed" }
      }

      const status = yield* git.status(input.cwd)
      if (!status.length && input.checkpoint.noop_if_clean) return { ok: true, reason: "clean" }
      if (status.length === 0) return { ok: true, reason: "clean" }

      if (input.checkpoint.git?.add?.length) {
        for (const target of input.checkpoint.git.add) {
          yield* git.run(["add", target], { cwd: input.cwd })
        }
      }
      const commitMessage = input.checkpoint.git?.commit_message ?? `${input.spec.job.name}: verified change`
      const commit = yield* git.run(["commit", "-m", commitMessage], { cwd: input.cwd })
      if (commit.exitCode !== 0) return { ok: false, reason: commit.stderr.toString() || commit.text() || "commit failed" }
      const sha = yield* git.run(["rev-parse", "HEAD"], { cwd: input.cwd }).pipe(
        Effect.map((result) => (result.exitCode === 0 ? result.text().trim() : undefined)),
      )
      if (input.spec.jankurai?.enabled && input.spec.jankurai.verification?.require_clean_after_checkpoint !== false) {
        const afterStatus = yield* git.status(input.cwd)
        if (afterStatus.length > 0) {
          return {
            ok: false,
            reason: `jankurai requires a clean checkout after checkpoint; dirty paths: ${afterStatus.map((item) => item.file).join(", ")}`,
            sha,
          }
        }
      }
      if (input.checkpoint.git?.push === "allow") {
        const push = yield* git.run(["push"], { cwd: input.cwd })
        if (push.exitCode !== 0) return { ok: false, reason: push.stderr.toString() || push.text() || "push failed" }
      }
      if (input.checkpoint.git?.push === "ask") return { ok: false, reason: "push requires approval", sha }
      return { ok: true, sha }
    })

    return Service.of({ runCheckpoint })
  }),
)

export const defaultLayer = layer.pipe(
  Layer.provide(DaemonChecks.defaultLayer),
  Layer.provide(Git.defaultLayer),
)

export * as DaemonCheckpoint from "./daemon-checkpoint"
