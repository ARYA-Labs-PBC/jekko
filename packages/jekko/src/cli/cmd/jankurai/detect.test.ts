import { describe, expect, test } from "bun:test"
import fs from "fs"
import os from "os"
import path from "path"
import { detectCanonical, hasJankuraiCiWorkflow, CANONICAL_FILES } from "./detect"

function makeTempRepo(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "jankurai-detect-"))
}

function touch(repoRoot: string, rel: string) {
  const abs = path.join(repoRoot, rel)
  fs.mkdirSync(path.dirname(abs), { recursive: true })
  fs.writeFileSync(abs, "", "utf8")
}

describe("detectCanonical", () => {
  test("empty repo: every required file is missing, ok=false", () => {
    const dir = makeTempRepo()
    const result = detectCanonical(dir)
    expect(result.ok).toBe(false)
    expect(result.present).toEqual([])
    const requiredCount = CANONICAL_FILES.filter((f) => f.required).length
    expect(result.missingRequired.length).toBe(requiredCount)
  })

  test("fully scaffolded repo: ok=true", () => {
    const dir = makeTempRepo()
    for (const file of CANONICAL_FILES) {
      touch(dir, file.rel)
    }
    const result = detectCanonical(dir)
    expect(result.ok).toBe(true)
    expect(result.missingRequired).toEqual([])
  })

  test("partial scaffold: still ok=false until required files present", () => {
    const dir = makeTempRepo()
    touch(dir, "agent/JANKURAI_STANDARD.md")
    touch(dir, "agent/audit-policy.toml")
    const result = detectCanonical(dir)
    expect(result.ok).toBe(false)
    expect(result.present).toContain("agent/JANKURAI_STANDARD.md")
    expect(result.present).toContain("agent/audit-policy.toml")
    expect(result.missingRequired).toContain("agent/owner-map.json")
  })

  test("optional files do not gate ok", () => {
    const dir = makeTempRepo()
    for (const file of CANONICAL_FILES) {
      if (file.required) touch(dir, file.rel)
    }
    const result = detectCanonical(dir)
    expect(result.ok).toBe(true)
    expect(result.missingOptional.length).toBeGreaterThan(0)
  })
})

describe("hasJankuraiCiWorkflow", () => {
  test("returns false when workflow file is absent", () => {
    expect(hasJankuraiCiWorkflow(makeTempRepo())).toBe(false)
  })

  test("returns true when workflow file is present", () => {
    const dir = makeTempRepo()
    touch(dir, ".github/workflows/jankurai.yml")
    expect(hasJankuraiCiWorkflow(dir)).toBe(true)
  })
})
