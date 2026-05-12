import * as path from "path"
import { Effect, Schema } from "effect"
import * as Tool from "./tool"
import { Bus } from "../bus"
import { FileWatcher } from "../file/watcher"
import { InstanceState } from "@/effect/instance-state"
import { Patch } from "../patch"
import { diffLines } from "diff"
import { assertExternalDirectoryEffect } from "./external-directory"
import { trimDiff } from "./edit"
import { LSP } from "@/lsp/lsp"
import { AppFileSystem } from "@jekko-ai/core/filesystem"
import DESCRIPTION from "./apply_patch.txt"
import { File } from "../file"
import { Format } from "../format"
import * as Bom from "@/util/bom"
import { PatchVmModule } from "../patch/vm"
import { hashBytes } from "../patch/hash"

export const Parameters = Schema.Struct({
  patchText: Schema.String.annotate({ description: "The full patch text that describes all changes to be made" }),
})

export const ApplyPatchTool = Tool.define(
  "apply_patch",
  Effect.gen(function* () {
    const lsp = yield* LSP.Service
    const afs = yield* AppFileSystem.Service
    const format = yield* Format.Service
    const bus = yield* Bus.Service
    const patchVm = yield* PatchVmModule.Service

    const run = Effect.fn("ApplyPatchTool.execute")(function* (
      params: Schema.Schema.Type<typeof Parameters>,
      ctx: Tool.Context,
    ) {
      if (!params.patchText) {
        return yield* Effect.fail(new Error("patchText is required"))
      }

      let hunks: Patch.Hunk[]
      try {
        const parseResult = Patch.parsePatch(params.patchText)
        hunks = parseResult.hunks
      } catch (error) {
        return yield* Effect.fail(new Error(`apply_patch verification failed: ${error}`))
      }

      if (hunks.length === 0) {
        const normalized = params.patchText.replace(/\r\n/g, "\n").replace(/\r/g, "\n").trim()
        if (normalized === "*** Begin Patch\n*** End Patch") {
          return yield* Effect.fail(new Error("patch rejected: empty patch"))
        }
        return yield* Effect.fail(new Error("apply_patch verification failed: no hunks found"))
      }

      const instance = yield* InstanceState.context

      const fileChanges: Array<{
        filePath: string
        oldContent: string
        newContent: string
        type: "add" | "update" | "delete" | "move"
        movePath?: string
        diff: string
        additions: number
        deletions: number
        bom: boolean
        expectedHash: string
      }> = []

      let totalDiff = ""
      const vmChanges: PatchVmModule.FileChange[] = []

      for (const hunk of hunks) {
        const filePath = path.resolve(instance.directory, hunk.path)
        yield* assertExternalDirectoryEffect(ctx, filePath, { access: "write" })

        switch (hunk.type) {
          case "add": {
            const oldContent = ""
            const newContent =
              hunk.contents.length === 0 || hunk.contents.endsWith("\n") ? hunk.contents : `${hunk.contents}\n`
            const next = Bom.split(newContent)
            
            let additions = 0
            for (const change of diffLines(oldContent, next.text)) {
              if (change.added) additions += change.count || 0
            }

            fileChanges.push({
              filePath,
              oldContent,
              newContent: next.text,
              type: "add",
              diff: next.text,
              additions,
              deletions: 0,
              bom: next.bom,
              expectedHash: "",
            })
            vmChanges.push({
              type: "add",
              path: filePath,
              content: Bom.join(next.text, next.bom)
            })

            totalDiff += `+ ${filePath}\n`
            break
          }

          case "update": {
            const stats = yield* afs.stat(filePath).pipe(Effect.catch(() => Effect.succeed(undefined)))
            if (!stats || stats.type === "Directory") {
              return yield* Effect.fail(
                new Error(`apply_patch verification failed: Failed to read file to update: ${filePath}`),
              )
            }

            const currentBytes = yield* afs.readFile(filePath)
            const expectedHash = hashBytes(currentBytes)
            const source = Bom.split(new TextDecoder().decode(currentBytes))
            
            const oldContent = source.text
            let newContent = oldContent
            let bom = source.bom

            try {
              const fileUpdate = Patch.deriveNewContentsFromChunks(filePath, hunk.chunks, oldContent)
              newContent = fileUpdate.content
              // Preserve original BOM: `deriveNewContentsFromChunks` only sees the
              // stripped content we pass in, so its returned `bom` flag can't tell
              // us about the original file. Fall back to the BOM we already
              // detected from `currentBytes` and OR with any new BOM introduced
              // by the patch payload itself.
              bom = source.bom || fileUpdate.bom
            } catch (error) {
              return yield* Effect.fail(new Error(`apply_patch verification failed: ${error}`))
            }

            let additions = 0
            let deletions = 0
            for (const change of diffLines(oldContent, newContent)) {
              if (change.added) additions += change.count || 0
              if (change.removed) deletions += change.count || 0
            }

            const movePath = hunk.move_path ? path.resolve(instance.directory, hunk.move_path) : undefined
            yield* assertExternalDirectoryEffect(ctx, movePath, { access: "write" })

            const diffStr = trimDiff(Patch.deriveNewContentsFromChunks(filePath, hunk.chunks, oldContent).unified_diff)

            fileChanges.push({
              filePath,
              oldContent,
              newContent,
              type: hunk.move_path ? "move" : "update",
              movePath,
              diff: diffStr,
              additions,
              deletions,
              bom,
              expectedHash
            })
            
            vmChanges.push({
              type: "update",
              path: filePath,
              content: Bom.join(newContent, bom),
              expectedHash,
              movePath
            })

            totalDiff += diffStr + "\n"
            break
          }

          case "delete": {
            const currentBytes = yield* afs.readFile(filePath).pipe(
              Effect.catch((error) =>
                Effect.fail(
                  new Error(`apply_patch verification failed: ${error instanceof Error ? error.message : String(error)}`),
                ),
              ),
            )
            const expectedHash = hashBytes(currentBytes)
            const source = Bom.split(new TextDecoder().decode(currentBytes))
            
            const contentToDelete = source.text
            const deletions = contentToDelete.split("\n").length

            fileChanges.push({
              filePath,
              oldContent: contentToDelete,
              newContent: "",
              type: "delete",
              diff: `- ${filePath}`,
              additions: 0,
              deletions,
              bom: source.bom,
              expectedHash
            })
            
            vmChanges.push({
              type: "delete",
              path: filePath,
              expectedHash
            })

            totalDiff += `- ${filePath}\n`
            break
          }
        }
      }

      const files = fileChanges.map((change) => ({
        filePath: change.filePath,
        relativePath: path.relative(instance.worktree, change.movePath ?? change.filePath).replaceAll("\\", "/"),
        type: change.type,
        patch: change.diff,
        additions: change.additions,
        deletions: change.deletions,
        movePath: change.movePath,
      }))

      const relativePaths = fileChanges.map((c) => path.relative(instance.worktree, c.filePath).replaceAll("\\", "/"))
      yield* ctx.ask({
        permission: "edit",
        patterns: relativePaths,
        always: ["*"],
        metadata: {
          filepath: relativePaths.join(", "),
          diff: totalDiff,
          files,
        },
      })

      // PatchVM Atomic Commit
      const patchId = ctx.messageID
      const preview = yield* patchVm.preview(patchId, vmChanges)
      const proof: PatchVmModule.PatchApplyProof = preview.risk === "high" 
        ? { type: "preview_id", previewId: preview.previewId } 
        : { type: "hashes_only" }
      
      const report = yield* patchVm.apply(patchId, vmChanges, proof)
      
      if (report.outcome === "rolled_back" || report.outcome === "rejected") {
        return yield* Effect.fail(new Error(`PatchVM rejected or rolled back the transaction: outcome=${report.outcome}`))
      }

      for (const change of fileChanges) {
        const edited = change.type === "delete" ? undefined : (change.movePath ?? change.filePath)
        if (edited) {
          if (yield* format.file(edited)) {
            yield* Bom.syncFile(afs, edited, change.bom)
          }
          yield* bus.publish(File.Event.Edited, { file: edited })
        }
      }

      // Publish file change events
      for (const change of fileChanges) {
        if (change.type === "add") yield* bus.publish(FileWatcher.Event.Updated, { file: change.filePath, event: "add" })
        if (change.type === "update") yield* bus.publish(FileWatcher.Event.Updated, { file: change.filePath, event: "change" })
        if (change.type === "move") {
          yield* bus.publish(FileWatcher.Event.Updated, { file: change.filePath, event: "unlink" })
          if (change.movePath) yield* bus.publish(FileWatcher.Event.Updated, { file: change.movePath, event: "add" })
        }
        if (change.type === "delete") yield* bus.publish(FileWatcher.Event.Updated, { file: change.filePath, event: "unlink" })
      }

      for (const change of fileChanges) {
        if (change.type === "delete") continue
        const target = change.movePath ?? change.filePath
        yield* lsp.touchFile(target, "document")
      }
      const diagnostics = yield* lsp.diagnostics()

      const summaryLines = fileChanges.map((change) => {
        if (change.type === "add") {
          return `A ${path.relative(instance.worktree, change.filePath).replaceAll("\\", "/")}`
        }
        if (change.type === "delete") {
          return `D ${path.relative(instance.worktree, change.filePath).replaceAll("\\", "/")}`
        }
        const target = change.movePath ?? change.filePath
        return `M ${path.relative(instance.worktree, target).replaceAll("\\", "/")}`
      })
      let output = `Success. Updated the following files:\n${summaryLines.join("\n")}`

      for (const change of fileChanges) {
        if (change.type === "delete") continue
        const target = change.movePath ?? change.filePath
        const block = LSP.Diagnostic.report(target, diagnostics[AppFileSystem.normalizePath(target)] ?? [])
        if (!block) continue
        const rel = path.relative(instance.worktree, target).replaceAll("\\", "/")
        output += `\n\nLSP errors detected in ${rel}, please fix:\n${block}`
      }

      return {
        title: output,
        metadata: {
          diff: totalDiff,
          files,
          diagnostics,
        },
        output,
      }
    })

    return {
      description: DESCRIPTION,
      parameters: Parameters,
      execute: (params: Schema.Schema.Type<typeof Parameters>, ctx: Tool.Context) =>
        run(params, ctx).pipe(Effect.orDie),
    }
  }),
)
