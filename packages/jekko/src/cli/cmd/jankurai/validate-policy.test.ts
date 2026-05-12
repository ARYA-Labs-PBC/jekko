import { describe, expect, test } from "bun:test"
import fs from "fs"
import os from "os"
import path from "path"
import { auditPolicy } from "./validate-policy"

function writePolicy(contents: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "jankurai-policy-"))
  const file = path.join(dir, "audit-policy.toml")
  fs.writeFileSync(file, contents, "utf8")
  return file
}

describe("auditPolicy", () => {
  test("missing file: ok=false, every required severity missing", () => {
    const result = auditPolicy(path.join(os.tmpdir(), "nope-" + Date.now() + ".toml"))
    expect(result.ok).toBe(false)
    expect(result.hasMinScore).toBe(false)
    expect(result.missingFailOn).toEqual(["critical", "high"])
    expect(result.missingAdvisoryOn).toEqual(["medium", "low"])
  })

  test("complete policy: ok=true", () => {
    const file = writePolicy(`
[decision]
min_score = 85
fail_on = ["critical", "high"]
advisory_on = ["medium", "low"]
`)
    const result = auditPolicy(file)
    expect(result.ok).toBe(true)
    expect(result.hasMinScore).toBe(true)
    expect(result.missingFailOn).toEqual([])
    expect(result.missingAdvisoryOn).toEqual([])
  })

  test("missing fail_on severity is reported", () => {
    const file = writePolicy(`
[decision]
min_score = 85
fail_on = ["critical"]
advisory_on = ["medium", "low"]
`)
    const result = auditPolicy(file)
    expect(result.ok).toBe(false)
    expect(result.missingFailOn).toEqual(["high"])
  })

  test("missing min_score gates ok=false even when severities are complete", () => {
    const file = writePolicy(`
[decision]
fail_on = ["critical", "high"]
advisory_on = ["medium", "low"]
`)
    const result = auditPolicy(file)
    expect(result.ok).toBe(false)
    expect(result.hasMinScore).toBe(false)
  })

  test("case-insensitive matching on severity names", () => {
    const file = writePolicy(`
[decision]
min_score = 90
fail_on = ["Critical", "HIGH"]
advisory_on = ["Medium", "low"]
`)
    const result = auditPolicy(file)
    expect(result.ok).toBe(true)
  })
})
