import { describe, expect, test } from "bun:test"
import { readFileSync, readdirSync } from "fs"
import path from "path"
import { tokenizeYaml, type YamlToken } from "../../../src/cli/cmd/tui/util/yaml-tokenize"

function scopesIn(tokens: YamlToken[]): string[] {
  return tokens.map((t) => t.scope)
}

function findScope(tokens: YamlToken[], scope: string, text: string, start = 0) {
  return tokens.find((t) => t.scope === scope && text.slice(t.start, t.end).length > 0 && t.start >= start)
}

function example(name: string) {
  return readFileSync(path.join(import.meta.dir, "../../../../../docs/ZYAL/examples", name), "utf8")
}

function exampleNames() {
  return readdirSync(path.join(import.meta.dir, "../../../../../docs/ZYAL/examples"))
    .filter((name) => name.endsWith(".yml"))
    .sort()
}

function nonWhitespaceCoverage(text: string, tokens: YamlToken[]) {
  const covered = new Uint8Array(text.length)
  for (const token of tokens) {
    for (let index = token.start; index < token.end; index++) covered[index] = 1
  }
  let total = 0
  let colored = 0
  for (let index = 0; index < text.length; index++) {
    if (/\s/.test(text[index]!)) continue
    total++
    if (covered[index]) colored++
  }
  return total === 0 ? 1 : colored / total
}

describe("tokenizeYaml", () => {
  test("returns empty list for empty input", () => {
    expect(tokenizeYaml("")).toEqual([])
  })

  test("tags a bare property key", () => {
    const text = "version: v1"
    const tokens = tokenizeYaml(text)
    const prop = tokens.find((t) => t.scope === "property")
    expect(prop).toBeDefined()
    expect(text.slice(prop!.start, prop!.end)).toBe("version")
  })

  test("recognises a quoted string value", () => {
    const text = `name: "Jankurai dirty worktree hardening"`
    const tokens = tokenizeYaml(text)
    expect(scopesIn(tokens)).toContain("string")
    const stringToken = tokens.find((t) => t.scope === "string")!
    expect(text.slice(stringToken.start, stringToken.end)).toBe(`"Jankurai dirty worktree hardening"`)
  })

  test("recognises bare booleans and numbers", () => {
    const text = "enabled: true\nport: 8080\nratio: 1.5"
    const tokens = tokenizeYaml(text)
    expect(scopesIn(tokens)).toEqual(
      expect.arrayContaining(["property", "boolean", "property", "number", "property", "number"]),
    )
    const bool = tokens.find((t) => t.scope === "boolean")!
    expect(text.slice(bool.start, bool.end)).toBe("true")
    const num = tokens.find((t) => t.scope === "number")!
    expect(text.slice(num.start, num.end)).toBe("8080")
  })

  test("tags a leading-hash comment", () => {
    const text = "# top-level comment\nversion: v1"
    const tokens = tokenizeYaml(text)
    const comment = tokens.find((t) => t.scope === "comment")!
    expect(text.slice(comment.start, comment.end)).toBe("# top-level comment")
  })

  test("tags an end-of-line comment after a value", () => {
    const text = "sleep: 5s  # five seconds"
    const tokens = tokenizeYaml(text)
    const comment = tokens.find((t) => t.scope === "comment")!
    expect(text.slice(comment.start, comment.end)).toBe("# five seconds")
  })

  test("does not split on a hash inside a quoted string", () => {
    const text = `tag: "abc#xyz"`
    const tokens = tokenizeYaml(text)
    expect(tokens.find((t) => t.scope === "comment")).toBeUndefined()
    const stringToken = tokens.find((t) => t.scope === "string")!
    expect(text.slice(stringToken.start, stringToken.end)).toBe(`"abc#xyz"`)
  })

  test("tags ZYAL sentinels and ARM line as sentinel", () => {
    const text = "<<<ZYAL v1:daemon id=test>>>\nversion: v1\n<<<END_ZYAL id=test>>>\nZYAL_ARM RUN_FOREVER id=test"
    const tokens = tokenizeYaml(text)
    const sentinels = tokens.filter((t) => t.scope === "sentinel")
    expect(sentinels).toHaveLength(3)
    expect(text.slice(sentinels[0].start, sentinels[0].end)).toBe("<<<ZYAL v1:daemon id=test>>>")
    expect(text.slice(sentinels[1].start, sentinels[1].end)).toBe("<<<END_ZYAL id=test>>>")
    expect(text.slice(sentinels[2].start, sentinels[2].end)).toBe("ZYAL_ARM RUN_FOREVER id=test")
  })

  test("tags indented ZYAL sentinels from pasted terminal buffers", () => {
    const text = "  <<<ZYAL v1:daemon id=test>>>\nversion: v1\n  <<<END_ZYAL id=test>>>\n  ZYAL_ARM RUN_FOREVER id=test"
    const tokens = tokenizeYaml(text)
    const sentinels = tokens.filter((t) => t.scope === "sentinel")
    expect(sentinels).toHaveLength(3)
    expect(text.slice(sentinels[0].start, sentinels[0].end)).toBe("  <<<ZYAL v1:daemon id=test>>>")
    expect(text.slice(sentinels[1].start, sentinels[1].end)).toBe("  <<<END_ZYAL id=test>>>")
    expect(text.slice(sentinels[2].start, sentinels[2].end)).toBe("  ZYAL_ARM RUN_FOREVER id=test")
  })

  test("tolerates a partial paste with only the open sentinel", () => {
    const text = "<<<ZYAL v1:daemon id=test>>>\nversion:"
    expect(() => tokenizeYaml(text)).not.toThrow()
    const tokens = tokenizeYaml(text)
    expect(scopesIn(tokens)).toEqual(expect.arrayContaining(["sentinel", "property"]))
  })

  test("offsets are exact across newlines", () => {
    const text = "a: 1\nb: 2"
    const tokens = tokenizeYaml(text)
    for (const t of tokens) {
      expect(t.end).toBeGreaterThan(t.start)
      expect(t.start).toBeGreaterThanOrEqual(0)
      expect(t.end).toBeLessThanOrEqual(text.length)
    }
    const props = tokens.filter((t) => t.scope === "property")
    expect(props).toHaveLength(2)
    expect(text.slice(props[0].start, props[0].end)).toBe("a")
    expect(text.slice(props[1].start, props[1].end)).toBe("b")
  })

  test("preserves indentation in nested keys", () => {
    const text = "job:\n  name: foo\n  objective: bar"
    const tokens = tokenizeYaml(text)
    const props = tokens.filter((t) => t.scope === "property")
    expect(props.map((t) => text.slice(t.start, t.end))).toEqual(["job", "name", "objective"])
  })

  test("colors block scalar bodies, sequence keys, inline maps, and arrays", () => {
    const text = [
      "job:",
      "  objective: |",
      "    RANDOM SELECTION — REQUIRED. Use `shuf -n 1`.",
      "    - bash:  {\"command\":\"...\",\"description\":\"...\"}",
      "agents:",
      "  workers:",
      "    - id: worker",
      "      count: 10",
      "      agent: build",
      "context:",
      "  preserve: [objective, current_task_id]",
      "models:",
      "  profiles:",
      "    build: { provider: jnoccio, model: jnoccio-fusion }",
    ].join("\n")
    const tokens = tokenizeYaml(text)
    expect(scopesIn(tokens)).toEqual(
      expect.arrayContaining([
        "block",
        "operator",
        "sequence",
        "property",
        "punctuation",
        "string",
        "number",
        "literal",
      ]),
    )
    expect(nonWhitespaceCoverage(text, tokens)).toBeGreaterThan(0.85)
  })

  test("covers the real minimal Jankurai fleet example with rich scopes", () => {
    const text = example("12-jankurai-min-loop.zyal")
    const tokens = tokenizeYaml(text)
    const scopes = new Set(scopesIn(tokens))

    for (const scope of [
      "sentinel",
      "comment",
      "property",
      "punctuation",
      "literal",
      "string",
      "number",
      "boolean",
      "sequence",
      "block",
      "operator",
    ]) {
      expect(scopes.has(scope)).toBe(true)
    }
    expect(tokens.filter((t) => t.scope === "block").length).toBeGreaterThan(40)
    expect(nonWhitespaceCoverage(text, tokens)).toBeGreaterThan(0.78)
  })

  test("colors the full body of every checked-in ZYAL YAML example", () => {
    for (const name of exampleNames()) {
      const text = example(name)
      const tokens = tokenizeYaml(text)
      const scopes = new Set(scopesIn(tokens))

      expect(scopes.size, name).toBeGreaterThanOrEqual(9)
      expect(scopes.has("sentinel"), name).toBe(true)
      expect(scopes.has("property"), name).toBe(true)
      expect(scopes.has("punctuation"), name).toBe(true)
      expect(scopes.has("literal"), name).toBe(true)
      expect(scopes.has("string"), name).toBe(true)
      expect(scopes.has("sequence"), name).toBe(true)
      expect(nonWhitespaceCoverage(text, tokens), name).toBeGreaterThan(0.98)
    }
  })
})
