import type { ZyalMcp, ZyalIncubatorPass } from "@/agent-script/schema"

export type McpGateResult = {
  ok: boolean
  allowedTools: Record<string, boolean>
  blocked: { server: string; status: string }[]
}

export function buildMcpToolAllowMap(input: { mcp?: ZyalMcp; pass?: ZyalIncubatorPass }) {
  const profileName = input.pass?.mcp_profile
  if (!profileName) return {}
  const profile = input.mcp?.profiles?.[profileName]
  if (!profile) return {}
  return Object.fromEntries([["mcp:*", false], ...(profile.tools ?? []).map((tool) => [`mcp:${tool}`, true])])
}

export function checkRequiredProfiles(input: {
  mcp?: ZyalMcp
  profile?: string
  status: Record<string, { status: string }>
}): McpGateResult {
  const profile = input.profile ? input.mcp?.profiles?.[input.profile] : undefined
  if (!profile) return { ok: true, allowedTools: {}, blocked: [] }
  const blocked = (profile.servers ?? []).flatMap((server) => {
    const status = input.status[server]
    if (!status || status.status === "connected" || status.status === "disabled") return []
    return [{ server, status: status.status }]
  })
  return {
    ok: blocked.length === 0,
    allowedTools: Object.fromEntries([["mcp:*", false], ...(profile.tools ?? []).map((tool) => [`mcp:${tool}`, true])]),
    blocked,
  }
}

export * as DaemonMcp from "./daemon-mcp"
