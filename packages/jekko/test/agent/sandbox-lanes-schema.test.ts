import { describe, expect, it } from "bun:test"
import { Schema } from "effect"
import { readFileSync } from "node:fs"
import { join } from "node:path"
import { parse as parseToml } from "smol-toml"
import { SandboxLanes } from "@/config/sandbox-lanes"

const REPO_ROOT = join(import.meta.dir, "..", "..", "..", "..")
const LANES_PATH = join(REPO_ROOT, "agent", "sandbox-lanes.toml")
const FIXTURE_PATH = join(REPO_ROOT, "crates", "sandboxctl", "tests", "fixtures", "sample-lanes.toml")

function loadToml(path: string): unknown {
  return parseToml(readFileSync(path, "utf8"))
}

describe("agent/sandbox-lanes.toml schema", () => {
  it("parses the canonical file against the Effect.Schema mirror", () => {
    const raw = loadToml(LANES_PATH)
    const doc = Schema.decodeSync(SandboxLanes.Doc)(raw)
    expect(doc.lane.length).toBeGreaterThan(0)
    const errors = SandboxLanes.semanticValidate(doc)
    expect(errors).toEqual([])
  })

  it("parses the shared fixture (Rust crate fixture)", () => {
    const raw = loadToml(FIXTURE_PATH)
    const doc = Schema.decodeSync(SandboxLanes.Doc)(raw)
    expect(doc.lane.length).toBeGreaterThan(0)
    const errors = SandboxLanes.semanticValidate(doc)
    expect(errors).toEqual([])
  })

  it("rejects an empty allowed_patterns list", () => {
    const raw = loadToml(FIXTURE_PATH) as { lane: any[] }
    raw.lane[0].commands.allowed_patterns = []
    expect(() => Schema.decodeSync(SandboxLanes.Doc)(raw)).toThrow()
  })

  it("flags a bare '*' pattern via semantic validation", () => {
    const raw = loadToml(FIXTURE_PATH) as { lane: any[] }
    raw.lane[0].commands.allowed_patterns = ["*"]
    const doc = Schema.decodeSync(SandboxLanes.Doc)(raw)
    const errors = SandboxLanes.semanticValidate(doc)
    expect(errors.some((e) => e.includes("bare '*'"))).toBe(true)
  })

  it("flags docker backend missing runtime.image", () => {
    const raw = loadToml(FIXTURE_PATH) as { lane: any[] }
    raw.lane[1].runtime.image = undefined
    delete raw.lane[1].runtime.image
    const doc = Schema.decodeSync(SandboxLanes.Doc)(raw)
    const errors = SandboxLanes.semanticValidate(doc)
    expect(errors.some((e) => e.includes("runtime.image is required"))).toBe(true)
  })

  it("flags env paths missing {run_id} placeholder", () => {
    const raw = loadToml(FIXTURE_PATH) as { lane: any[] }
    raw.lane[0].environment.home = "relative/path"
    const doc = Schema.decodeSync(SandboxLanes.Doc)(raw)
    const errors = SandboxLanes.semanticValidate(doc)
    expect(errors.some((e) => e.includes("environment.home"))).toBe(true)
  })

  it("flags duplicate lane names", () => {
    const raw = loadToml(FIXTURE_PATH) as { lane: any[] }
    raw.lane.push({ ...raw.lane[0] })
    const doc = Schema.decodeSync(SandboxLanes.Doc)(raw)
    const errors = SandboxLanes.semanticValidate(doc)
    expect(errors.some((e) => e.includes("duplicate lane name"))).toBe(true)
  })
})
