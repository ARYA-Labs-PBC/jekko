import path from "path"

import { createPlugTask, type PlugCtx, type PlugDeps } from "../../src/cli/cmd/plug"
import { Filesystem } from "@/util/filesystem"

type Msg = {
  dir: string
  target: string
  mod: string
  global?: boolean
  force?: boolean
  globalDir?: string
  vcs?: string
  worktree?: string
  directory?: string
  holdMs?: number
}

function isMsg(parsed: unknown): parsed is Msg {
  if (typeof parsed !== "object" || parsed === null) return false
  const msg = parsed as Record<string, unknown>

  if (typeof msg.dir !== "string" || typeof msg.target !== "string" || typeof msg.mod !== "string") {
    return false
  }
  if (msg.global !== undefined && typeof msg.global !== "boolean") return false
  if (msg.force !== undefined && typeof msg.force !== "boolean") return false
  if (msg.globalDir !== undefined && typeof msg.globalDir !== "string") return false
  if (msg.vcs !== undefined && typeof msg.vcs !== "string") return false
  if (msg.worktree !== undefined && typeof msg.worktree !== "string") return false
  if (msg.directory !== undefined && typeof msg.directory !== "string") return false
  if (msg.holdMs !== undefined && (typeof msg.holdMs !== "number" || msg.holdMs < 0)) return false
  return true
}

function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, ms)
  })
}

function input(): Msg {
  const raw = process.argv[2]
  if (!raw) {
    throw new Error("Missing plug worker input")
  }

  const parsed = JSON.parse(raw)
  if (!isMsg(parsed)) {
    throw new Error("Input must be a JSON object")
  }

  return parsed
}

function deps(msg: Msg): PlugDeps {
  return {
    spinner: () => ({
      start() {},
      stop() {},
    }),
    log: {
      error() {},
      info() {},
      success() {},
    },
    resolve: async () => msg.target,
    readText: (file) => Filesystem.readText(file),
    write: async (file, text) => {
      if (msg.holdMs && msg.holdMs > 0) {
        await sleep(msg.holdMs)
      }
      await Filesystem.write(file, text)
    },
    exists: (file) => Filesystem.exists(file),
    files: (dir, name) => [path.join(dir, `${name}.jsonc`), path.join(dir, `${name}.json`)],
    global: msg.globalDir ?? path.join(msg.dir, ".global"),
  }
}

function ctx(msg: Msg): PlugCtx {
  return {
    vcs: msg.vcs ?? "git",
    worktree: msg.worktree ?? msg.dir,
    directory: msg.directory ?? msg.dir,
  }
}

async function main() {
  const msg = input()
  const run = createPlugTask(
    {
      mod: msg.mod,
      global: msg.global,
      force: msg.force,
    },
    deps(msg),
  )

  const ok = await run(ctx(msg))
  if (!ok) {
    throw new Error("Plug task failed")
  }
}

await main()
