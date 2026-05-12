import { describe, expect, test } from "bun:test"
import { buildDaemonLibraryOptions } from "../../../src/cli/cmd/tui/routes/session/dialog-daemon"

describe("daemon dialog", () => {
  test("exposes incubator examples in the daemon library", () => {
    const options = buildDaemonLibraryOptions()
    expect(options.some((option) => option.value === "hard-task-incubator")).toBe(true)
    expect(options.some((option) => option.value === "normal-user-incubator")).toBe(true)
  })
})
