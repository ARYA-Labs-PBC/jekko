import { describe, expect, test } from "bun:test"

function evaluateSeed(env?: Record<string, string>) {
  const proc = Bun.spawnSync(
    ["just", "--justfile", "Justfile", "--evaluate", "memory_benchmark_seed"],
    {
      cwd: "/Users/bentaylor/Code/opencode",
      env: {
        ...process.env,
        ...env,
      },
      stdout: "pipe",
      stderr: "pipe",
    },
  )

  if (proc.exitCode !== 0) {
    throw new Error(
      `just --evaluate failed:\nstdout:\n${Buffer.from(proc.stdout).toString()}\nstderr:\n${Buffer.from(proc.stderr).toString()}`,
    )
  }

  return Buffer.from(proc.stdout).toString().trim()
}

describe("Justfile memory benchmark seed", () => {
  test("defaults to public-dev-0001", () => {
    expect(evaluateSeed()).toBe("public-dev-0001")
  })

  test("respects MEMORY_BENCHMARK_SEED overrides", () => {
    expect(evaluateSeed({ MEMORY_BENCHMARK_SEED: "public-dev-smoke" })).toBe("public-dev-smoke")
  })
})
