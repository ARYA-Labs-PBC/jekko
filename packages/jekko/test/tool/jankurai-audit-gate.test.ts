import { expect, test } from "bun:test"
import { mkdtempSync, rmSync, writeFileSync } from "node:fs"
import path from "node:path"
import { tmpdir } from "node:os"

const repoRoot = path.resolve(import.meta.dir, "../../../..")
const script = path.join(repoRoot, "tools/jankurai-audit-gate.mjs")

function runGate(score: unknown) {
  const dir = mkdtempSync(path.join(tmpdir(), "jankurai-gate-"))
  const file = path.join(dir, "repo-score.json")
  writeFileSync(file, JSON.stringify(score, null, 2))

  try {
    return Bun.spawnSync(["node", script, file], {
      cwd: repoRoot,
      stderr: "pipe",
      stdout: "pipe",
    })
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

test("jankurai audit gate accepts nested decision counts", () => {
  const result = runGate({
    caps_applied: [],
    decision: {
      hard_findings: 0,
      soft_findings: 0,
    },
  })

  expect(result.exitCode).toBe(0)
  expect(result.stdout.toString()).toContain("0 caps and 0 findings")
})

test("jankurai audit gate rejects non-zero nested findings", () => {
  const result = runGate({
    caps_applied: [],
    decision: {
      hard_findings: 1,
      soft_findings: 0,
    },
  })

  expect(result.exitCode).toBe(1)
  expect(result.stderr.toString()).toContain("hard_findings must be 0, found: 1")
})
