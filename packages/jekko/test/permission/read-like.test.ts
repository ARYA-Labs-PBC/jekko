import { describe, expect, test } from "bun:test"
import {
  isReadLikeRequest,
  resolveAskFallback,
  rulesAutoAllowReads,
  rulesNoHumanPrompts,
} from "../../src/permission/read-like"

describe("read-like permissions", () => {
  test("classifies read tool permissions as read-like", () => {
    for (const permission of ["read", "list", "glob", "grep"]) {
      expect(isReadLikeRequest({ permission })).toBe(true)
    }
  })

  test("only treats external_directory as read-like when metadata says read", () => {
    expect(isReadLikeRequest({ permission: "external_directory", metadata: { access: "read" } })).toBe(true)
    expect(isReadLikeRequest({ permission: "external_directory", metadata: { access: "write" } })).toBe(false)
    expect(isReadLikeRequest({ permission: "external_directory", metadata: { access: "unknown" } })).toBe(false)
    expect(isReadLikeRequest({ permission: "external_directory", metadata: {} })).toBe(false)
  })

  test("recognizes unattended marker rules", () => {
    const rules = [
      { permission: "zyal_auto_allow_reads", pattern: "*", action: "allow" as const },
      { permission: "zyal_unattended", pattern: "no_human_prompts", action: "allow" as const },
    ]

    expect(rulesAutoAllowReads(rules)).toBe(true)
    expect(rulesNoHumanPrompts(rules)).toBe(true)
  })

  test("resolves unattended ask resolution without prompting reads", () => {
    expect(
      resolveAskFallback({
        request: { permission: "external_directory", metadata: { access: "read" } },
        autoAllowReads: true,
        noHumanPrompts: true,
      }),
    ).toBe("allow")

    expect(
      resolveAskFallback({
        request: { permission: "external_directory", metadata: { access: "write" } },
        autoAllowReads: true,
        noHumanPrompts: true,
      }),
    ).toBe("deny")

    expect(
      resolveAskFallback({
        request: { permission: "bash", metadata: {} },
        autoAllowReads: false,
        noHumanPrompts: false,
      }),
    ).toBe("ask")
  })
})
