import { describe, expect, test } from "bun:test"
import path from "path"
import { pathToFileURL } from "url"

const helperUrl = pathToFileURL(
  path.resolve(import.meta.dir, "../../../../src/cli/cmd/tui/command-guards.ts"),
).href

describe("runTuiCommandWithGuards", () => {
  test("rejects fork without continue or session", () => {
    const proc = Bun.spawnSync(
      [process.execPath, "-e", `
        import { runTuiCommandWithGuards } from ${JSON.stringify(helperUrl)}
        let ran = false
        await runTuiCommandWithGuards({ fork: true }, async () => {
          ran = true
        })
        console.log(JSON.stringify({ ran, exitCode: process.exitCode ?? null }))
      `],
      {
        stdout: "pipe",
        stderr: "pipe",
        env: process.env,
      },
    )

    expect(proc.exitCode).toBe(1)
    const stdout = new TextDecoder().decode(proc.stdout).trim()
    expect(stdout).toContain('"ran":false')
    expect(stdout).toContain('"exitCode":1')
  })
})

describe("runValidatedTuiCommandWithConfig", () => {
  test("loads config before invoking the body", async () => {
    const proc = Bun.spawnSync(
      [process.execPath, "-e", `
        import { runValidatedTuiCommandWithConfig } from ${JSON.stringify(helperUrl)}
        const events = []
        await runValidatedTuiCommandWithConfig(
          { fork: false },
          { url: "http://example.com" },
          async () => ({ name: "config" }),
          async (config) => {
            events.push(config.name)
          },
        )
        console.log(JSON.stringify(events))
      `],
      {
        stdout: "pipe",
        stderr: "pipe",
        env: process.env,
      },
    )

    expect(proc.exitCode).toBe(0)
    const stdout = new TextDecoder().decode(proc.stdout).trim()
    expect(stdout).toBe('["config"]')
  })
})
