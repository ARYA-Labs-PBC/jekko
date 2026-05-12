import { Effect, Context, Layer } from "effect"
import { AppFileSystem } from "@jekko-ai/core/filesystem"
import { hashBytes, hashString } from "./hash"
import * as path from "path"

export type FileChange =
  | { type: "add"; path: string; content: string; expectedHash?: string }
  | { type: "delete"; path: string; expectedHash: string }
  | {
      type: "update"
      path: string
      content: string
      expectedHash: string
      movePath?: string
    }

export type PatchRisk = "low" | "high"

export interface PatchPreview {
  patchId: string
  previewId: string
  risk: PatchRisk
  touchedPaths: string[]
  rollbackTokens: RollbackToken[]
}

export type PatchApplyProof = { type: "hashes_only" } | { type: "preview_id"; previewId: string }

export interface RollbackToken {
  path: string
  preImageHash?: string
  preImageBytes?: Uint8Array
  wasAdded: boolean
}

export interface PatchApplyReport {
  patchId: string
  outcome: "applied" | "rolled_back" | "rejected"
  touchedPaths: string[]
  rollbackTokens: RollbackToken[]
}

export interface PatchVmPolicy {
  maxFiles?: number
  allowFullFileRewrite?: boolean
}

const DEFAULT_POLICY: PatchVmPolicy = {
  maxFiles: 10,
  allowFullFileRewrite: false,
}

export interface Interface {
  readonly preview: (
    patchId: string,
    changes: FileChange[],
    policy?: PatchVmPolicy,
  ) => Effect.Effect<PatchPreview, Error>
  readonly apply: (
    patchId: string,
    changes: FileChange[],
    proof: PatchApplyProof,
    policy?: PatchVmPolicy,
  ) => Effect.Effect<PatchApplyReport, Error>
  readonly rollback: (tokens: RollbackToken[]) => Effect.Effect<void, Error>
}

export class Service extends Context.Service<Service, Interface>()("@jekko/PatchVm") {}

export const layer = Layer.effect(
  Service,
  Effect.gen(function* () {
    const fs = yield* AppFileSystem.Service

    const preview = Effect.fn("PatchVm.preview")(function* (
      patchId: string,
      changes: FileChange[],
      policy: PatchVmPolicy = DEFAULT_POLICY,
    ) {
      if (changes.length === 0) {
        return yield* Effect.fail(new Error("no file changes provided"))
      }

      if (policy.maxFiles && changes.length > policy.maxFiles) {
        return yield* Effect.fail(new Error(`patch touches ${changes.length} files, which is above max of ${policy.maxFiles}`))
      }

      const touchedPaths: string[] = []
      const rollbackTokens: RollbackToken[] = []
      let risk: PatchRisk = "low"

      if (changes.length > 1) {
        risk = "high"
      }

      for (const change of changes) {
        touchedPaths.push(change.path)
        if (change.type === "update" && change.movePath) {
          touchedPaths.push(change.movePath)
        }

        if (change.type === "delete" || (change.type === "update" && change.movePath)) {
          risk = "high"
        }

        const exists = yield* fs.existsSafe(change.path)

        if (change.type === "add") {
          if (exists) {
            // Addition of an existing file is handled at apply time, but we don't need a rollback token for the pre-image
            // since we shouldn't overwrite it.
          }
          rollbackTokens.push({ path: change.path, wasAdded: true })
          continue
        }

        if (!exists) {
          return yield* Effect.fail(new Error(`cannot patch missing file: ${change.path}`))
        }

        const currentBytes = yield* fs.readFile(change.path)
        const currentHash = hashBytes(currentBytes)

        if (currentHash !== change.expectedHash) {
          return yield* Effect.fail(
            new Error(`file hash mismatch for ${change.path}. Expected ${change.expectedHash}, got ${currentHash}`),
          )
        }

        if (change.type === "update" && !policy.allowFullFileRewrite) {
          // Heuristic: If we are replacing a file larger than 100KB with content
          // that doesn't look like a standard targeted patch, reject it as a full-file rewrite.
          // Note: In a real implementation this would check if the patch is using a diff format.
          // Since our changes provide the full content, we just check file size.
          if (currentBytes.length > 100 * 1024) {
            return yield* Effect.fail(
              new Error(`full-file rewrite of large file ${change.path} rejected by policy. Use chunked patching.`),
            )
          }
        }

        rollbackTokens.push({
          path: change.path,
          preImageHash: currentHash,
          preImageBytes: currentBytes,
          wasAdded: false,
        })
      }

      // Generate deterministic preview ID based on changes and hash signatures
      const signatureData =
        patchId +
        changes
          .map((c) =>
            c.type === "add" ? `add:${c.path}` : `${c.type}:${c.path}:${c.expectedHash}`,
          )
          .join(";")
      const previewId = "prv_" + hashString(signatureData).substring(0, 16)

      return {
        patchId,
        previewId,
        risk,
        touchedPaths,
        rollbackTokens,
      } as PatchPreview
    })

    const applyChange = Effect.fn("PatchVm.applyChange")(function* (change: FileChange) {
      if (change.type === "delete") {
        yield* fs.remove(change.path, { recursive: true }).pipe(Effect.catch(() => Effect.void))
        return
      }

      if (change.type === "update" && change.movePath) {
        yield* fs.writeWithDirs(change.movePath, change.content)
        yield* fs.remove(change.path, { recursive: true }).pipe(Effect.catch(() => Effect.void))
        return
      }

      yield* fs.writeWithDirs(change.path, change.content)
    })

    const rollback = Effect.fn("PatchVm.rollback")(function* (tokens: RollbackToken[]) {
      for (const token of [...tokens].reverse()) {
        if (token.wasAdded) {
          yield* fs.remove(token.path, { recursive: true }).pipe(Effect.catch(() => Effect.void))
        } else if (token.preImageBytes) {
          yield* fs.writeWithDirs(token.path, token.preImageBytes)
        }
      }
    })

    const apply = Effect.fn("PatchVm.apply")(function* (
      patchId: string,
      changes: FileChange[],
      proof: PatchApplyProof,
      policy: PatchVmPolicy = DEFAULT_POLICY,
    ) {
      const previewResult = yield* preview(patchId, changes, policy)

      if (previewResult.risk === "high") {
        if (proof.type !== "preview_id") {
          return yield* Effect.fail(
            new Error("high-risk patch requires a matching preview id proof (two-phase commit)"),
          )
        }
        if (proof.previewId !== previewResult.previewId) {
          return yield* Effect.fail(
            new Error(`preview id does not match. Expected ${previewResult.previewId}, got ${proof.previewId}`),
          )
        }
      }

      // Verify no 'add' files currently exist (prevent accidental overwrite)
      for (const change of changes) {
        if (change.type === "add") {
          if (yield* fs.existsSafe(change.path)) {
            return yield* Effect.fail(new Error(`refusing to add existing file: ${change.path}`))
          }
        }
      }

      const appliedTokens: RollbackToken[] = []

      // Try applying all changes
      const applyAll = Effect.gen(function* () {
        for (let i = 0; i < changes.length; i++) {
          const change = changes[i]!
          yield* applyChange(change)
          // Store token for successful apply so we can roll back if a later file fails
          const token = previewResult.rollbackTokens.find((t) => t.path === change.path)
          if (token) appliedTokens.push(token)
        }
      })

      // Run and catch errors
      const success = yield* applyAll.pipe(
        Effect.map(() => true),
        Effect.catch(() => Effect.succeed(false))
      )

      if (!success) {
        // Rollback on failure
        yield* rollback(appliedTokens)
        return {
          patchId,
          outcome: "rolled_back",
          touchedPaths: previewResult.touchedPaths,
          rollbackTokens: previewResult.rollbackTokens,
        } as PatchApplyReport
      }

      return {
        patchId,
        outcome: "applied",
        touchedPaths: previewResult.touchedPaths,
        rollbackTokens: previewResult.rollbackTokens,
      } as PatchApplyReport
    })

    return Service.of({ preview, apply, rollback })
  }),
)

export * as PatchVmModule from "./vm"
