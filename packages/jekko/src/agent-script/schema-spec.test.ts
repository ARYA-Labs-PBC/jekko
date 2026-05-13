import { describe, expect, test } from "bun:test"
import { spawnSync } from "child_process"
import { readFileSync } from "fs"
import path from "path"
import { assertKnownZyalKeys, renderZyalSpecMarkdown, ZYAL_SCHEMA_SPEC, ZYAL_TOP_LEVEL_KEYS } from "./schema-spec"
import { ZYAL_CONTRACT_VERSION, ZYAL_RESEARCH_BLOCK_VERSION, ZYAL_RUNTIME_SENTINEL_VERSION } from "./version"

const repoRoot = path.resolve(import.meta.dir, "../../../../")
const specPath = path.resolve(import.meta.dir, "../../../../docs/ZYAL/SPEC.md")
const examplesReadmePath = path.resolve(import.meta.dir, "../../../../docs/ZYAL/examples/README.md")

describe("ZYAL schema spec", () => {
  test("generated spec is current", () => {
    expect(readFileSync(specPath, "utf8")).toBe(`${renderZyalSpecMarkdown()}\n`)
  })

  test("generator writes the canonical spec", () => {
    const result = spawnSync("bun", ["--cwd", "packages/jekko", "./script/generate-zyal-spec.ts", "--write"], {
      cwd: repoRoot,
      encoding: "utf8",
    })
    expect(result.status).toBe(0)
    expect(result.stdout).toContain("wrote docs/ZYAL/SPEC.md")
    expect(readFileSync(specPath, "utf8")).toBe(`${renderZyalSpecMarkdown()}\n`)
  })

  test("examples README stays on the documented count and includes the new bug-finder rows", () => {
    const readme = readFileSync(examplesReadmePath, "utf8")
    expect(readme).toContain("Twenty-seven flagship runbooks")
    expect(readme.match(/^\| \[`/gm)?.length).toBe(27)
    expect(readme).toContain("[`18-semantic-bug-finder-basic.zyal`](18-semantic-bug-finder-basic.zyal)")
    expect(readme).toContain("[`19-semantic-bug-finder-advanced.zyal`](19-semantic-bug-finder-advanced.zyal)")
    expect(readme).toContain("[`20-semantic-bug-finder-ultra.zyal`](20-semantic-bug-finder-ultra.zyal)")
    expect(readme).toContain("[`21-semantic-improvement-finder-simple.zyal`](21-semantic-improvement-finder-simple.zyal)")
    expect(readme).toContain("[`22-semantic-improvement-finder-advanced.zyal`](22-semantic-improvement-finder-advanced.zyal)")
    expect(readme).toContain("[`23-semantic-improvement-finder-insane.zyal`](23-semantic-improvement-finder-insane.zyal)")
  })

  test("version metadata matches version.ts", () => {
    expect(ZYAL_SCHEMA_SPEC.contractVersion).toBe(ZYAL_CONTRACT_VERSION)
    expect(ZYAL_SCHEMA_SPEC.runtimeSentinelVersion).toBe(ZYAL_RUNTIME_SENTINEL_VERSION)
    expect(ZYAL_SCHEMA_SPEC.researchBlockVersion).toBe(ZYAL_RESEARCH_BLOCK_VERSION)
  })

  test("every registry node has description and status", () => {
    visitNode(ZYAL_SCHEMA_SPEC.root, (node) => {
      expect(node.description).toBeTruthy()
      expect(node.status).toMatch(/^(runtime|preview|generated|compat)$/)
    })
  })

  test("top-level keys match the registry root", () => {
    expect(ZYAL_TOP_LEVEL_KEYS).toEqual(Object.keys(ZYAL_SCHEMA_SPEC.root.children).sort())
  })

  test("known keys validator rejects unknown keys", () => {
    expect(() =>
      assertKnownZyalKeys({
        version: "v1",
        intent: "daemon",
        confirm: "RUN_FOREVER",
        id: "bad",
        job: { name: "x", objective: "y" },
        stop: { all: [{ git_clean: {} }] },
        surprise: true,
      }),
    ).toThrow("Unknown ZYAL top-level key: surprise")
  })
})

function visitNode(node: typeof ZYAL_SCHEMA_SPEC.root, visit: (node: typeof ZYAL_SCHEMA_SPEC.root) => void) {
  visit(node)
  if (node.kind === "object") {
    for (const child of Object.values(node.children)) visitNode(child as typeof ZYAL_SCHEMA_SPEC.root, visit)
    return
  }
  if (node.kind === "record") {
    visitNode(node.value as typeof ZYAL_SCHEMA_SPEC.root, visit)
    return
  }
  if (node.kind === "array") {
    visitNode(node.item as typeof ZYAL_SCHEMA_SPEC.root, visit)
  }
}
