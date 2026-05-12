// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import type { ZyalSecurity, ZyalSecurityTrustZone } from "@/agent-script/schema"

/**
 * Security engine for ZYAL v2.
 *
 * Manages trust zones, injection scanning, secret scoping,
 * and provides path-level security policies.
 */

export type SecurityCheckResult = {
  readonly allowed: boolean
  readonly zone: string | null
  readonly reason: string
  readonly requiresApproval: boolean
}

export type InjectionScanResult = {
  readonly clean: boolean
  readonly detections: InjectionDetection[]
  readonly action: string
}

export type InjectionDetection = {
  readonly pattern: string
  readonly matched: string
  readonly position: number
}

/**
 * Check if a path falls within a trust zone.
 */
export function checkTrustZone(
  security: ZyalSecurity | undefined,
  filePath: string,
): SecurityCheckResult {
  if (!security?.trust_zones) {
    return { allowed: true, zone: null, reason: "no trust zones defined", requiresApproval: false }
  }

  for (const [zoneName, zone] of Object.entries(security.trust_zones)) {
    if (pathInZone(filePath, zone)) {
      return {
        allowed: !zone.require_approval,
        zone: zoneName,
        reason: zone.require_approval
          ? `path in zone '${zoneName}' requires approval`
          : `path in zone '${zoneName}'`,
        requiresApproval: zone.require_approval === true,
      }
    }
  }

  return { allowed: true, zone: null, reason: "path not in any trust zone", requiresApproval: false }
}

/**
 * Check if a file modification is allowed given the risk score.
 */
export function checkRiskThreshold(
  security: ZyalSecurity | undefined,
  filePath: string,
  riskScore: number,
): { allowed: boolean; reason: string } {
  if (!security?.trust_zones) {
    return { allowed: true, reason: "no trust zones" }
  }

  for (const [zoneName, zone] of Object.entries(security.trust_zones)) {
    if (pathInZone(filePath, zone) && zone.max_risk_score !== undefined) {
      if (riskScore > zone.max_risk_score) {
        return {
          allowed: false,
          reason: `risk score ${riskScore} exceeds zone '${zoneName}' max of ${zone.max_risk_score}`,
        }
      }
    }
  }

  return { allowed: true, reason: "within risk thresholds" }
}

/**
 * Scan text for injection patterns.
 */
export function scanForInjection(
  security: ZyalSecurity | undefined,
  text: string,
  direction: "input" | "output",
): InjectionScanResult {
  if (!security?.injection) {
    return { clean: true, detections: [], action: "none" }
  }

  if (direction === "input" && !security.injection.scan_inputs) {
    return { clean: true, detections: [], action: "none" }
  }
  if (direction === "output" && !security.injection.scan_outputs) {
    return { clean: true, detections: [], action: "none" }
  }

  const detections: InjectionDetection[] = []
  if (security.injection.deny_patterns) {
    for (const pattern of security.injection.deny_patterns) {
      const regex = patternToRegex(pattern)
      if (regex === null) continue
      for (const match of text.matchAll(regex)) {
        detections.push({
          pattern,
          matched: match[0],
          position: match.index,
        })
      }
    }
  }

  return {
    clean: detections.length === 0,
    detections,
    action: detections.length > 0 ? (security.injection.on_detect ?? "warn") : "none",
  }
}

/**
 * Apply injection stripping to text.
 */
export function stripInjections(
  security: ZyalSecurity | undefined,
  text: string,
): string {
  if (!security?.injection?.deny_patterns) return text
  let result = text
  for (const pattern of security.injection.deny_patterns) {
    const regex = patternToRegex(pattern)
    if (regex === null) continue
    result = result.replace(regex, "[STRIPPED]")
  }
  return result
}

/**
 * Check if an environment variable is allowed for secret access.
 */
export function checkSecretAccess(
  security: ZyalSecurity | undefined,
  envVar: string,
): { allowed: boolean; reason: string } {
  if (!security?.secrets) {
    return { allowed: true, reason: "no secret policy" }
  }
  if (!security.secrets.allowed_env) {
    return { allowed: true, reason: "no allowed_env restriction" }
  }
  if (security.secrets.allowed_env.includes(envVar)) {
    return { allowed: true, reason: "in allowed_env list" }
  }
  return { allowed: false, reason: `env var '${envVar}' not in allowed_env` }
}

/**
 * Check if a value should be redacted from logs.
 */
export function shouldRedactFromLogs(security: ZyalSecurity | undefined): boolean {
  return security?.secrets?.redact_from_logs === true
}

/**
 * Get all trust zone names.
 */
export function getTrustZoneNames(security: ZyalSecurity | undefined): string[] {
  if (!security?.trust_zones) return []
  return Object.keys(security.trust_zones)
}

/**
 * Get paths protected by a trust zone.
 */
export function getZonePaths(security: ZyalSecurity | undefined, zoneName: string): string[] {
  if (!security?.trust_zones) return []
  const zone = security.trust_zones[zoneName]
  return [...(zone?.paths ?? [])]
}

function pathInZone(filePath: string, zone: ZyalSecurityTrustZone): boolean {
  if (!zone.paths?.length) return false
  return zone.paths.some((zp) => {
    if (filePath === zp) return true
    if (filePath.startsWith(zp + "/")) return true
    // Glob suffix
    if (zp.startsWith("*")) return filePath.endsWith(zp.slice(1))
    return false
  })
}

function patternToRegex(pattern: string): RegExp | null {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!pattern.trim()) return null
  const escaped = pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\\\*/g, ".*")
  return new RegExp(escaped, "gi")
}
