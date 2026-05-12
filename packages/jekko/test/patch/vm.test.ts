import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { PatchVmModule } from "../../src/patch/vm"
import type { FileChange, PatchApplyProof, PatchVmPolicy } from "../../src/patch/vm"
import { hashString } from "../../src/patch/hash"
import * as fs from "fs/promises"
import * as path from "path"
import { tmpdir } from "os"
import { Effect, Layer } from "effect"
import { AppFileSystem } from "@jekko-ai/core/filesystem"

describe("PatchVm", () => {
  let tempDir: string
  let testLayer: Layer.Layer<PatchVmModule.Service, never, never>

  beforeEach(async () => {
    tempDir = await fs.mkdtemp(path.join(tmpdir(), "patchvm-test-"))
    testLayer = PatchVmModule.layer.pipe(Layer.provide(AppFileSystem.defaultLayer))
  })

  afterEach(async () => {
    await fs.rm(tempDir, { recursive: true, force: true })
  })

  const runTest = <A>(eff: Effect.Effect<A, any, PatchVmModule.Service>) =>
    Effect.runPromise(eff.pipe(Effect.provide(testLayer)))

  describe("preview", () => {
    test("single-file add is low risk", async () => {
      const filePath = path.join(tempDir, "new.txt")
      const changes: FileChange[] = [
        { type: "add", path: filePath, content: "hello world" },
      ]

      const preview = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-1", changes)
      }))

      expect(preview.risk).toBe("low")
      expect(preview.touchedPaths).toEqual([filePath])
      expect(preview.rollbackTokens).toHaveLength(1)
      expect(preview.previewId).toBeTruthy()
    })

    test("single-file update with valid hash succeeds", async () => {
      const filePath = path.join(tempDir, "existing.txt")
      const originalContent = "original content"
      await fs.writeFile(filePath, originalContent)
      const expectedHash = hashString(originalContent)

      const changes: FileChange[] = [
        { type: "update", path: filePath, content: "updated content", expectedHash },
      ]

      const preview = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-2", changes)
      }))

      expect(preview.risk).toBe("low")
      expect(preview.rollbackTokens).toHaveLength(1)
      expect(preview.rollbackTokens[0].preImageHash).toBe(expectedHash)
    })

    test("outdated hash rejection", async () => {
      const filePath = path.join(tempDir, "existing.txt")
      await fs.writeFile(filePath, "original content")

      const changes: FileChange[] = [
        {
          type: "update",
          path: filePath,
          content: "updated content",
          expectedHash: "outdated-hash-that-does-not-match",
        },
      ]

      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-3", changes)
      }))).rejects.toThrow("file hash mismatch")
    })

    test("multi-file is high risk", async () => {
      const file1 = path.join(tempDir, "a.txt")
      const file2 = path.join(tempDir, "b.txt")
      await fs.writeFile(file1, "a")
      await fs.writeFile(file2, "b")
      const hash1 = hashString("a")
      const hash2 = hashString("b")

      const changes: FileChange[] = [
        { type: "update", path: file1, content: "aa", expectedHash: hash1 },
        { type: "update", path: file2, content: "bb", expectedHash: hash2 },
      ]

      const preview = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-4", changes)
      }))
      expect(preview.risk).toBe("high")
    })

    test("delete is high risk", async () => {
      const filePath = path.join(tempDir, "to-delete.txt")
      await fs.writeFile(filePath, "content")
      const hash = hashString("content")

      const changes: FileChange[] = [
        { type: "delete", path: filePath, expectedHash: hash },
      ]

      const preview = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-5", changes)
      }))
      expect(preview.risk).toBe("high")
    })

    test("empty changes rejected", async () => {
      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-6", [])
      }))).rejects.toThrow("no file changes")
    })

    test("too many files rejected", async () => {
      const changes: FileChange[] = Array.from({ length: 20 }, (_, i) => ({
        type: "add" as const,
        path: path.join(tempDir, `file-${i}.txt`),
        content: `content-${i}`,
      }))

      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-7", changes)
      }))).rejects.toThrow("above max")
    })

    test("missing file for update rejected", async () => {
      const changes: FileChange[] = [
        {
          type: "update",
          path: path.join(tempDir, "does-not-exist.txt"),
          content: "new",
          expectedHash: "abc",
        },
      ]

      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-8", changes)
      }))).rejects.toThrow("cannot patch missing file")
    })

    test("large file rewrite rejected by default", async () => {
      const filePath = path.join(tempDir, "large.txt")
      const largeContent = "x".repeat(300 * 1024)
      await fs.writeFile(filePath, largeContent)
      const hash = hashString(largeContent)

      const changes: FileChange[] = [
        { type: "update", path: filePath, content: "y".repeat(300 * 1024), expectedHash: hash },
      ]

      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-9", changes)
      }))).rejects.toThrow("full-file rewrite")
    })

    test("large file rewrite allowed with policy", async () => {
      const filePath = path.join(tempDir, "large.txt")
      const largeContent = "x".repeat(300 * 1024)
      await fs.writeFile(filePath, largeContent)
      const hash = hashString(largeContent)

      const changes: FileChange[] = [
        { type: "update", path: filePath, content: "y".repeat(300 * 1024), expectedHash: hash },
      ]

      const policy: PatchVmPolicy = { allowFullFileRewrite: true, maxFiles: 16 }
      const preview = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.preview("patch-10", changes, policy)
      }))
      expect(preview.risk).toBe("low")
    })
  })

  describe("apply", () => {
    test("single-file add succeeds with hashes_only proof", async () => {
      const filePath = path.join(tempDir, "new.txt")
      const changes: FileChange[] = [
        { type: "add", path: filePath, content: "hello world" },
      ]
      const proof: PatchApplyProof = { type: "hashes_only" }

      const report = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.apply("patch-11", changes, proof)
      }))
      expect(report.outcome).toBe("applied")
      expect(report.touchedPaths).toEqual([filePath])

      const content = await fs.readFile(filePath, "utf-8")
      expect(content).toBe("hello world")
    })

    test("high-risk without preview_id rejected", async () => {
      const file1 = path.join(tempDir, "a.txt")
      const file2 = path.join(tempDir, "b.txt")
      await fs.writeFile(file1, "a")
      await fs.writeFile(file2, "b")
      const hash1 = hashString("a")
      const hash2 = hashString("b")

      const changes: FileChange[] = [
        { type: "update", path: file1, content: "aa", expectedHash: hash1 },
        { type: "update", path: file2, content: "bb", expectedHash: hash2 },
      ]
      const proof: PatchApplyProof = { type: "hashes_only" }

      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.apply("patch-12", changes, proof)
      }))).rejects.toThrow("high-risk patch requires a matching preview id")
    })

    test("high-risk with wrong preview_id rejected", async () => {
      const file1 = path.join(tempDir, "a.txt")
      const file2 = path.join(tempDir, "b.txt")
      await fs.writeFile(file1, "a")
      await fs.writeFile(file2, "b")
      const hash1 = hashString("a")
      const hash2 = hashString("b")

      const changes: FileChange[] = [
        { type: "update", path: file1, content: "aa", expectedHash: hash1 },
        { type: "update", path: file2, content: "bb", expectedHash: hash2 },
      ]
      const proof: PatchApplyProof = { type: "preview_id", previewId: "wrong-id" }

      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.apply("patch-13", changes, proof)
      }))).rejects.toThrow("preview id does not match")
    })

    test("high-risk with correct preview_id succeeds (two-phase commit)", async () => {
      const file1 = path.join(tempDir, "a.txt")
      const file2 = path.join(tempDir, "b.txt")
      await fs.writeFile(file1, "a")
      await fs.writeFile(file2, "b")
      const hash1 = hashString("a")
      const hash2 = hashString("b")

      const changes: FileChange[] = [
        { type: "update", path: file1, content: "aa", expectedHash: hash1 },
        { type: "update", path: file2, content: "bb", expectedHash: hash2 },
      ]

      const { preview, report } = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        const preview = yield* vm.preview("patch-14", changes)
        const proof: PatchApplyProof = { type: "preview_id", previewId: preview.previewId }
        const report = yield* vm.apply("patch-14", changes, proof)
        return { preview, report }
      }))

      expect(preview.risk).toBe("high")
      expect(report.outcome).toBe("applied")

      const content1 = await fs.readFile(file1, "utf-8")
      const content2 = await fs.readFile(file2, "utf-8")
      expect(content1).toBe("aa")
      expect(content2).toBe("bb")
    })

    test("delete with correct preview_id succeeds", async () => {
      const filePath = path.join(tempDir, "to-delete.txt")
      await fs.writeFile(filePath, "content")
      const hash = hashString("content")

      const changes: FileChange[] = [
        { type: "delete", path: filePath, expectedHash: hash },
      ]

      const report = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        const preview = yield* vm.preview("patch-15", changes)
        const proof: PatchApplyProof = { type: "preview_id", previewId: preview.previewId }
        return yield* vm.apply("patch-15", changes, proof)
      }))

      expect(report.outcome).toBe("applied")
      const exists = await fs.access(filePath).then(() => true).catch(() => false)
      expect(exists).toBe(false)
    })

    test("move file with correct preview_id succeeds", async () => {
      const oldPath = path.join(tempDir, "prior.txt")
      const newPath = path.join(tempDir, "new.txt")
      await fs.writeFile(oldPath, "content")
      const hash = hashString("content")

      const changes: FileChange[] = [
        { type: "update", path: oldPath, content: "moved content", expectedHash: hash, movePath: newPath },
      ]

      const report = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        const preview = yield* vm.preview("patch-16", changes)
        const proof: PatchApplyProof = { type: "preview_id", previewId: preview.previewId }
        return yield* vm.apply("patch-16", changes, proof)
      }))

      expect(report.outcome).toBe("applied")
      const oldExists = await fs.access(oldPath).then(() => true).catch(() => false)
      expect(oldExists).toBe(false)
      const newContent = await fs.readFile(newPath, "utf-8")
      expect(newContent).toBe("moved content")
    })

    test("add fails on existing file", async () => {
      const filePath = path.join(tempDir, "exists.txt")
      await fs.writeFile(filePath, "already here")

      const changes: FileChange[] = [
        { type: "add", path: filePath, content: "new content" },
      ]
      const proof: PatchApplyProof = { type: "hashes_only" }

      await expect(runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.apply("patch-17", changes, proof)
      }))).rejects.toThrow("refusing to add existing file")
    })
  })

  describe("rollback", () => {
    test("emergency rollback restores files", async () => {
      const filePath = path.join(tempDir, "rollback.txt")
      const original = "original content"
      await fs.writeFile(filePath, original)
      const hash = hashString(original)

      const changes: FileChange[] = [
        { type: "update", path: filePath, content: "modified", expectedHash: hash },
      ]
      
      const report = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.apply("patch-18", changes, { type: "hashes_only" })
      }))

      expect(await fs.readFile(filePath, "utf-8")).toBe("modified")

      await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        return yield* vm.rollback(report.rollbackTokens)
      }))

      const content = await fs.readFile(filePath, "utf-8")
      expect(content).toBe(original)
    })

    test("preview IDs are deterministic", async () => {
      const filePath = path.join(tempDir, "deterministic.txt")
      await fs.writeFile(filePath, "content")
      const hash = hashString("content")

      const changes: FileChange[] = [
        { type: "update", path: filePath, content: "new", expectedHash: hash },
      ]

      const { preview1, preview2 } = await runTest(Effect.gen(function* () {
        const vm = yield* PatchVmModule.Service
        const p1 = yield* vm.preview("same-id", changes)
        const p2 = yield* vm.preview("same-id", changes)
        return { preview1: p1, preview2: p2 }
      }))
      
      expect(preview1.previewId).toBe(preview2.previewId)
    })
  })
})
