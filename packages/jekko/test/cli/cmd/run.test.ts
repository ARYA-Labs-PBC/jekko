import { describe, expect, spyOn, test } from "bun:test"
import { resolveRunDirectory } from "../../../src/cli/cmd/run"

describe("resolveRunDirectory", () => {
  test("keeps attach mode on the provided directory without changing cwd", () => {
    const chdir = spyOn(process, "chdir").mockImplementation(() => {})

    try {
      expect(resolveRunDirectory({ dir: "/tmp/worktree", attach: "http://localhost:4096" })).toEqual({
        kind: "attached",
        directory: "/tmp/worktree",
      })
      expect(chdir).not.toHaveBeenCalled()
    } finally {
      chdir.mockRestore()
    }
  })

  test("reports a typed error when changing directory fails", () => {
    const chdir = spyOn(process, "chdir").mockImplementation(() => {
      throw new Error("boom")
    })

    try {
      expect(resolveRunDirectory({ dir: "/missing/worktree" })).toEqual({
        kind: "error",
        message: "Failed to change directory to /missing/worktree",
      })
      expect(chdir).toHaveBeenCalledTimes(1)
    } finally {
      chdir.mockRestore()
    }
  })
})
