import type { ZyalMcp, ZyalIncubatorPass } from "@/agent-script/schema"

export type McpGateResult = {
  ok: boolean
  allowedTools: Record<string, boolean>
  blocked: { server: string; status: string }[]
}

/**
 * Tools that escalate worker concurrency. Only the supervisor pass (and a
 * future explicit `spawn` pass type) may invoke them; allowing builders /
 * critics to fan-out themselves would break the worker-pool's slot budget.
 */
const SUPERVISOR_ONLY_TOOLS: ReadonlySet<string> = new Set([
  "jnoccio_spawn_parallel",
  "jnoccio_spawn_instance",
])

function isSupervisorPass(pass: ZyalIncubatorPass | undefined): boolean {
  if (!pass) return false
  return pass.context === "promotion" || pass.type === "promotion_review"
}

export function buildMcpToolAllowMap(input: { mcp?: ZyalMcp; pass?: ZyalIncubatorPass }) {
  const profileName = input.pass?.mcp_profile
  if (!profileName) return {}
  const profile = input.mcp?.profiles?.[profileName]
  if (!profile) return {}
  const allowSpawn = isSupervisorPass(input.pass)
  const entries: Array<[string, boolean]> = [["mcp:*", false]]
  for (const tool of profile.tools ?? []) {
    if (SUPERVISOR_ONLY_TOOLS.has(tool) && !allowSpawn) {
      entries.push([`mcp:${tool}`, false])
      continue
    }
    entries.push([`mcp:${tool}`, true])
  }
  return Object.fromEntries(entries)
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
