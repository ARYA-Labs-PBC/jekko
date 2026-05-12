import fs from "fs"

// Audit policy is a TOML file. We avoid pulling a TOML dep for PR1 and parse the
// narrow surface we care about with line-based scanning. This is intentionally
// shallow: it covers the three sections jankurai requires (`decision`,
// `severity`, optional `caps`) and reports what is missing or malformed. Deep
// validation is the auditor binary's job.

export type PolicyAudit = {
  /** Source path. */
  path: string
  /** True if `min_score` exists in any [decision] / [thresholds] table. */
  hasMinScore: boolean
  /** Severities listed in `fail_on = [...]` (lowercased). */
  failOn: string[]
  /** Severities listed in `advisory_on = [...]` (lowercased). */
  advisoryOn: string[]
  /** Required severities (`critical`, `high`) missing from `fail_on`. */
  missingFailOn: string[]
  /** Optional severities (`medium`, `low`) missing from `advisory_on`. */
  missingAdvisoryOn: string[]
  /** True iff policy is acceptable as-is. */
  ok: boolean
}

const REQUIRED_FAIL_ON = ["critical", "high"] as const
const REQUIRED_ADVISORY_ON = ["medium", "low"] as const

export function auditPolicy(filePath: string): PolicyAudit {
  if (!fs.existsSync(filePath)) {
    return {
      path: filePath,
      hasMinScore: false,
      failOn: [],
      advisoryOn: [],
      missingFailOn: [...REQUIRED_FAIL_ON],
      missingAdvisoryOn: [...REQUIRED_ADVISORY_ON],
      ok: false,
    }
  }
  const text = fs.readFileSync(filePath, "utf8")
  const hasMinScore = /^[\s]*min_score[\s]*=/m.test(text)
  const failOn = extractArray(text, "fail_on").map((s) => s.toLowerCase())
  const advisoryOn = extractArray(text, "advisory_on").map((s) => s.toLowerCase())
  const missingFailOn = REQUIRED_FAIL_ON.filter((s) => !failOn.includes(s))
  const missingAdvisoryOn = REQUIRED_ADVISORY_ON.filter((s) => !advisoryOn.includes(s))
  return {
    path: filePath,
    hasMinScore,
    failOn,
    advisoryOn,
    missingFailOn,
    missingAdvisoryOn,
    ok: hasMinScore && missingFailOn.length === 0 && missingAdvisoryOn.length === 0,
  }
}

/**
 * Extracts string values from a TOML array literal on a single line:
 *   fail_on = ["critical", "high"]
 * Multi-line arrays are not supported (deliberately — keeps the parser tiny
 * and matches the format we write in the bundled template).
 */
function extractArray(text: string, key: string): string[] {
  const re = new RegExp(`^[\\s]*${key}[\\s]*=[\\s]*\\[(?<body>[^\\]]*)\\]`, "m")
  const match = text.match(re)
  if (!match?.groups?.body) return []
  return match.groups.body
    .split(",")
    .map((piece) => piece.trim().replace(/^['"]|['"]$/g, ""))
    .filter(Boolean)
}
