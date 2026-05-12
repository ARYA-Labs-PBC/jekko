import type { ZyalSandbox, ZyalSandboxPathRule } from "@/agent-script/schema"
import { resolve, relative, isAbsolute } from "path"

/**
 * Sandbox enforcement engine for ZYAL v2.
 *
 * Validates file paths, network requests, and resource usage against
 * the sandbox policy defined in the ZYAL script.
 */

export type PathCheckResult = {
  readonly allowed: boolean
  readonly access: "read" | "write" | "deny" | "unrestricted"
  readonly reason: string
  readonly matchedRule?: ZyalSandboxPathRule
}

export type NetworkCheckResult = {
  readonly allowed: boolean
  readonly reason: string
}

export type ResourceState = {
  readonly fileCount: number
  readonly totalDiskBytes: number
  readonly processCount: number
}

/**
 * Check if a file path is allowed under the sandbox policy.
 */
export function checkPathAccess(
  sandbox: ZyalSandbox | undefined,
  filePath: string,
  operation: "read" | "write",
  workdir?: string,
): PathCheckResult {
  if (!sandbox?.paths?.length) {
    return { allowed: true, access: "unrestricted", reason: "no sandbox path rules" }
  }

  const normalizedPath = normalizePath(filePath, workdir)

  for (const rule of sandbox.paths) {
    if (pathMatchesRule(normalizedPath, rule.path, workdir)) {
      if (rule.access === "deny") {
        return { allowed: false, access: "deny", reason: `path denied by rule: ${rule.path}`, matchedRule: rule }
      }
      if (operation === "write" && rule.access === "read") {
        return { allowed: false, access: "read", reason: `path is read-only: ${rule.path}`, matchedRule: rule }
      }
      return { allowed: true, access: rule.access, reason: `allowed by rule: ${rule.path}`, matchedRule: rule }
    }
  }

  // Default: deny if sandbox is active but no rule matched
  return { allowed: false, access: "deny", reason: "no matching sandbox rule" }
}

/**
 * Check if an outbound network request is allowed.
 */
export function checkNetworkAccess(
  sandbox: ZyalSandbox | undefined,
  destination: string,
): NetworkCheckResult {
  if (!sandbox?.network) {
    return { allowed: true, reason: "no network policy" }
  }

  const policy = sandbox.network
  if (policy.outbound === "allow") {
    return { allowed: true, reason: "outbound: allow" }
  }
  if (policy.outbound === "deny") {
    return { allowed: false, reason: "outbound: deny" }
  }
  if (policy.outbound === "allowlist") {
    if (!policy.allowlist?.length) {
      return { allowed: false, reason: "allowlist is empty" }
    }
    const matched = policy.allowlist.some((pattern) => destinationMatchesPattern(destination, pattern))
    return matched
      ? { allowed: true, reason: `matched allowlist entry` }
      : { allowed: false, reason: `destination '${destination}' not in allowlist` }
  }

  return { allowed: true, reason: "no outbound restriction" }
}

/**
 * Check if an environment variable is allowed.
 */
export function checkEnvAccess(
  sandbox: ZyalSandbox | undefined,
  envVar: string,
): { allowed: boolean; reason: string } {
  if (!sandbox) return { allowed: true, reason: "no sandbox" }

  if (sandbox.env_deny?.includes(envVar)) {
    return { allowed: false, reason: `env var '${envVar}' is denied` }
  }

  if (sandbox.env_inherit) {
    if (sandbox.env_inherit.includes(envVar) || sandbox.env_inherit.includes("*")) {
      return { allowed: true, reason: "in env_inherit list" }
    }
    return { allowed: false, reason: `env var '${envVar}' not in env_inherit` }
  }

  return { allowed: true, reason: "no env restrictions" }
}

/**
 * Parse a resource limit string (e.g., "100MB", "1GB") to bytes.
 */
export function parseResourceLimit(limit: string): number {
  const match = limit.match(/^(\d+(?:\.\d+)?)\s*(B|KB|MB|GB|TB)$/i)
  if (!match) return 0
  const value = parseFloat(match[1])
  const unit = match[2].toUpperCase()
  const multipliers: Record<string, number> = {
    B: 1,
    KB: 1024,
    MB: 1024 ** 2,
    GB: 1024 ** 3,
    TB: 1024 ** 4,
  }
  return value * (multipliers[unit] ?? 1)
}

/**
 * Check resource usage against limits.
 */
export function checkResourceLimits(
  sandbox: ZyalSandbox | undefined,
  state: ResourceState,
): { violations: string[] } {
  if (!sandbox?.resources) return { violations: [] }
  const violations: string[] = []

  if (sandbox.resources.max_total_disk) {
    const limit = parseResourceLimit(sandbox.resources.max_total_disk)
    if (limit > 0 && state.totalDiskBytes > limit) {
      violations.push(`disk usage ${state.totalDiskBytes} exceeds limit ${sandbox.resources.max_total_disk}`)
    }
  }

  if (sandbox.resources.max_processes !== undefined) {
    if (state.processCount > sandbox.resources.max_processes) {
      violations.push(`process count ${state.processCount} exceeds limit ${sandbox.resources.max_processes}`)
    }
  }

  return { violations }
}

/**
 * Get summary of sandbox configuration.
 */
export function describeSandbox(sandbox: ZyalSandbox | undefined): string {
  if (!sandbox) return "no sandbox"
  const parts: string[] = []
  if (sandbox.paths?.length) parts.push(`${sandbox.paths.length} path rules`)
  if (sandbox.network) parts.push(`network:${sandbox.network.outbound ?? "default"}`)
  if (sandbox.resources) parts.push("resource limits")
  if (sandbox.env_deny?.length) parts.push(`${sandbox.env_deny.length} denied env vars`)
  return parts.join(", ") || "configured"
}

function normalizePath(filePath: string, workdir?: string): string {
  if (isAbsolute(filePath)) return filePath
  return resolve(workdir ?? process.cwd(), filePath)
}

function pathMatchesRule(normalizedPath: string, rulePattern: string, workdir?: string): boolean {
  const ruleAbsolute = isAbsolute(rulePattern)
    ? rulePattern
    : resolve(workdir ?? process.cwd(), rulePattern)

  // Check if path is under the rule directory, or matches exactly
  if (normalizedPath === ruleAbsolute) return true
  if (normalizedPath.startsWith(ruleAbsolute + "/")) return true

  // Glob-like suffix matching (e.g., "*.ts")
  if (rulePattern.startsWith("*")) {
    const suffix = rulePattern.slice(1)
    return normalizedPath.endsWith(suffix)
  }

  return false
}

function destinationMatchesPattern(destination: string, pattern: string): boolean {
  // Exact match
  if (destination === pattern) return true
  // Wildcard subdomain (e.g., "*.github.com")
  if (pattern.startsWith("*.")) {
    const domain = pattern.slice(2)
    return destination === domain || destination.endsWith("." + domain)
  }
  return false
}
