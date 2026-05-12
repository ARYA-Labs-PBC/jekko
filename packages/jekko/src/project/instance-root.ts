import path from "path"
import type { InstanceContext } from "./instance-context"

export function resolveInstanceRoot(ctx: Pick<InstanceContext, "directory" | "worktree">): string {
  const worktree = typeof ctx.worktree === "string" ? ctx.worktree.trim() : ""
  if (worktree) {
    const resolved = path.resolve(worktree)
    if (resolved !== path.parse(resolved).root && worktree !== "/") {
      return worktree
    }
  }

  const directory = typeof ctx.directory === "string" ? ctx.directory.trim() : ""
  if (directory) return directory

  return worktree || directory || "/"
}

