import { spawnSync } from "child_process"

export type UpdateResult = {
  /** Whether the `jankurai` binary was found on PATH. */
  installed: boolean
  /** Exit code from `jankurai update --client-start --quiet`, if spawned. */
  exitCode: number | null
  /** Stderr from the spawn, useful for surfacing to the user on non-zero exit. */
  stderr: string
  /** Captured stdout. Usually empty thanks to `--quiet`. */
  stdout: string
}

/**
 * Runs `jankurai update --client-start --quiet` if the binary is reachable.
 * Never throws — callers decide whether a non-zero exit should block bootstrap.
 *
 * The `--client-start` flag tells jankurai we're invoking from a long-running
 * client (jekko) so it self-updates idempotently in the background. `--quiet`
 * keeps stdout silent on a no-op.
 */
export function runJankuraiUpdate(opts: { dryRun?: boolean } = {}): UpdateResult {
  const probe = spawnSync("jankurai", ["--version"], { stdio: ["ignore", "pipe", "pipe"] })
  if (probe.error || probe.status !== 0) {
    return {
      installed: false,
      exitCode: null,
      stderr: probe.stderr?.toString() ?? "",
      stdout: probe.stdout?.toString() ?? "",
    }
  }
  if (opts.dryRun) {
    return { installed: true, exitCode: 0, stderr: "", stdout: "(dry-run: would run jankurai update --client-start --quiet)" }
  }
  const update = spawnSync("jankurai", ["update", "--client-start", "--quiet"], {
    stdio: ["ignore", "pipe", "pipe"],
  })
  return {
    installed: true,
    exitCode: typeof update.status === "number" ? update.status : null,
    stderr: update.stderr?.toString() ?? "",
    stdout: update.stdout?.toString() ?? "",
  }
}
