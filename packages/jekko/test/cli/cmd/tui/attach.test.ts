import { describe, expect, spyOn, test } from "bun:test"
import { resolveAttachDirectory } from "../../../../src/cli/cmd/tui/attach"

describe("resolveAttachDirectory", () => {
  test("returns the current cwd when no directory is provided", () => {
    const cwd = spyOn(process, "cwd").mockImplementation(() => "/tmp/local-worktree")

    try {
      expect(resolveAttachDirectory({})).toEqual({
        kind: "local",
        directory: "/tmp/local-worktree",
      })
    } finally {
      cwd.mockRestore()
    }
  })

  test("returns the local cwd when the directory exists", () => {
    const chdir = spyOn(process, "chdir").mockImplementation(() => {})
    const cwd = spyOn(process, "cwd").mockImplementation(() => "/tmp/local-worktree")

    try {
      expect(resolveAttachDirectory({ dir: "/tmp/worktree" })).toEqual({
        kind: "local",
        directory: "/tmp/local-worktree",
      })
    } finally {
      cwd.mockRestore()
      chdir.mockRestore()
    }
  })

  test("passes remote attach directories through when local chdir fails", () => {
    const chdir = spyOn(process, "chdir").mockImplementation(() => {
      throw new Error("missing")
    })

    try {
      expect(resolveAttachDirectory({ dir: "/remote/worktree" })).toEqual({
        kind: "remote",
        directory: "/remote/worktree",
      })
    } finally {
      chdir.mockRestore()
    }
  })
})
