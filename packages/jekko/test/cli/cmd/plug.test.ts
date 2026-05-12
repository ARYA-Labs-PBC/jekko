import { describe, expect, test } from "bun:test"

import { createPlugTask, type PlugDeps } from "../../../src/cli/cmd/plug"
import { Process } from "../../../src/util/process"

function createDeps(logs: { error: string[]; info: string[]; success: string[] }): PlugDeps {
  return {
    spinner: () => ({
      start() {},
      stop() {},
    }),
    log: {
      error: (msg) => logs.error.push(msg),
      info: (msg) => logs.info.push(msg),
      success: (msg) => logs.success.push(msg),
    },
    resolve: () =>
      Promise.reject(
        new Process.RunFailedError(
          ["pnpm", "add", "demo"],
          1,
          Buffer.alloc(0),
          Buffer.from("error: No version matching demo@1.0.0"),
        ),
      ),
    readText: async () => "",
    write: async () => {},
    exists: async () => false,
    files: () => [],
    global: "/tmp",
  }
}

describe("createPlugTask", () => {
  test("reports registry mismatch with explicit guidance", async () => {
    const logs = { error: [] as string[], info: [] as string[], success: [] as string[] }
    const task = createPlugTask({ mod: "demo" }, createDeps(logs))

    const ok = await task({
      worktree: "/tmp/worktree",
      directory: "/tmp/worktree",
    })

    expect(ok).toBe(false)
    expect(logs.error).toContain('Could not install "demo"')
    expect(logs.error).toContain("No version matching demo@1.0.0")
    expect(logs.info).toEqual([
      "The npm registry does not publish the requested version for this package.",
      "Verify registry and auth settings before running another install.",
    ])
  })
})
