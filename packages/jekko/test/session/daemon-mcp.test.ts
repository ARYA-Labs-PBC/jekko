import { describe, expect, test } from "bun:test"
import { DaemonMcp } from "../../src/session/daemon-mcp"

describe("daemon mcp gate", () => {
  test("builds a strict allow map for the active pass profile", () => {
    const allow = DaemonMcp.buildMcpToolAllowMap({
      mcp: { profiles: { default: { tools: ["read", "list"] } } } as any,
      pass: { mcp_profile: "default" } as any,
    })
    expect(allow["mcp:*"]).toBe(false)
    expect(allow["mcp:read"]).toBe(true)
    expect(allow["mcp:list"]).toBe(true)
  })

  test("blocks failed required servers", () => {
    const result = DaemonMcp.checkRequiredProfiles({
      mcp: { profiles: { default: { servers: ["filesystem"], tools: ["read"] } } } as any,
      profile: "default",
      status: { filesystem: { status: "failed", error: "boom" } } as any,
    })
    expect(result.ok).toBe(false)
    expect(result.blocked[0]?.server).toBe("filesystem")
  })
})
