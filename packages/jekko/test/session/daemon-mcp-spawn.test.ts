import { describe, expect, test } from "bun:test"
import type { ZyalIncubatorPass, ZyalMcp } from "../../src/agent-script/schema"
import { DaemonMcp } from "../../src/session/daemon-mcp"

const mcp: ZyalMcp = {
  profiles: {
    builder: { tools: ["read", "edit", "jnoccio_spawn_parallel"] },
    supervisor: { tools: ["read", "jnoccio_spawn_parallel", "jnoccio_spawn_instance"] },
  },
}

function pass(profile: string, type: ZyalIncubatorPass["type"], context: ZyalIncubatorPass["context"]): ZyalIncubatorPass {
  return {
    id: "x",
    type,
    context,
    writes: "scratch_only",
    mcp_profile: profile,
  } as ZyalIncubatorPass
}

describe("DaemonMcp.buildMcpToolAllowMap", () => {
  test("builder pass cannot spawn parallel jnoccio instances", () => {
    const allow = DaemonMcp.buildMcpToolAllowMap({
      mcp,
      pass: pass("builder", "prototype", "inherit"),
    })
    expect(allow["mcp:jnoccio_spawn_parallel"]).toBe(false)
    expect(allow["mcp:read"]).toBe(true)
    expect(allow["mcp:edit"]).toBe(true)
  })

  test("supervisor / promotion_review pass may spawn", () => {
    const allow = DaemonMcp.buildMcpToolAllowMap({
      mcp,
      pass: pass("supervisor", "promotion_review", "promotion"),
    })
    expect(allow["mcp:jnoccio_spawn_parallel"]).toBe(true)
    expect(allow["mcp:jnoccio_spawn_instance"]).toBe(true)
  })

  test("empty profile returns empty allow map", () => {
    expect(DaemonMcp.buildMcpToolAllowMap({ mcp: { profiles: {} }, pass: pass("missing", "scout", "blind") })).toEqual({})
  })

  test("pass without mcp_profile returns empty allow map", () => {
    expect(DaemonMcp.buildMcpToolAllowMap({ mcp, pass: undefined })).toEqual({})
  })
})
