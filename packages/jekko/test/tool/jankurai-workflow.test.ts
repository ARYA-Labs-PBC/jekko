import { expect, test } from "bun:test"
import { readFileSync } from "node:fs"
import path from "node:path"

const repoRoot = path.resolve(import.meta.dir, "../../../..")
const workflowPath = path.join(repoRoot, ".github/workflows/jankurai.yml")

test("jankurai workflow keeps the zero-caps gate in CI", () => {
  const workflow = readFileSync(workflowPath, "utf8")

  expect(workflow).toContain("pull_request:")
  expect(workflow).toContain("push:")
  expect(workflow).toContain("jankurai audit gate")
  expect(workflow).toContain("node tools/jankurai-audit-gate.mjs target/jankurai/repo-score.json")
})
