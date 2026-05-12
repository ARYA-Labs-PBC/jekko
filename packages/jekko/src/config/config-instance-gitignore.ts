import path from "path"
import { Effect } from "effect"
import { AppFileSystem } from "@jekko-ai/core/filesystem"

export function ensureGitignore(fs: typeof AppFileSystem.Type, dir: string) {
  const gitignore = path.join(dir, ".gitignore")
  return Effect.gen(function* () {
    const hasIgnore = yield* fs.existsSafe(gitignore)
    if (!hasIgnore) {
      yield* fs
        .writeFileString(
          gitignore,
          ["node_modules", "package.json", "package-lock.json", "bun.lock", ".gitignore"].join("\n"),
        )
        .pipe(
          Effect.catchIf(
            (e) => e.reason._tag === "PermissionDenied",
            () => Effect.void,
          ),
        )
    }
  })
}
