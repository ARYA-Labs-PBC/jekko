import fs from "fs"
import path from "path"

// Canonical files that `jankurai init` scaffolds. A repo is considered
// bootstrap-ready iff every required file exists (or is documented as optional).
export const CANONICAL_FILES = [
  { rel: "agent/JANKURAI_STANDARD.md", required: true },
  { rel: "agent/audit-policy.toml", required: true },
  { rel: "agent/owner-map.json", required: true },
  { rel: "agent/test-map.json", required: true },
  { rel: "agent/proof-lanes.toml", required: true },
  { rel: "agent/boundaries.toml", required: true },
  { rel: "agent/tool-adoption.toml", required: false },
  { rel: ".jekko/agent/generated-zones.toml", required: false },
] as const

export type CanonicalFile = (typeof CANONICAL_FILES)[number]

export type DetectResult = {
  /** Files that are required but missing. */
  missingRequired: string[]
  /** Files that are optional but missing. Informational. */
  missingOptional: string[]
  /** Files that exist. */
  present: string[]
  /** True iff every required file exists. */
  ok: boolean
}

/**
 * Pure file-existence check. No spawn, no parse. Caller (bootstrap CLI) decides
 * whether to repair missing files or pause for user input.
 */
export function detectCanonical(repoRoot: string): DetectResult {
  const missingRequired: string[] = []
  const missingOptional: string[] = []
  const present: string[] = []
  for (const file of CANONICAL_FILES) {
    const abs = path.join(repoRoot, file.rel)
    if (fs.existsSync(abs)) {
      present.push(file.rel)
    } else if (file.required) {
      missingRequired.push(file.rel)
    } else {
      missingOptional.push(file.rel)
    }
  }
  return {
    missingRequired,
    missingOptional,
    present,
    ok: missingRequired.length === 0,
  }
}

/**
 * Whether a repo has the minimal CI workflow scaffold.
 * Independent of canonical config files (some repos opt out of CI entirely).
 */
export function hasJankuraiCiWorkflow(repoRoot: string): boolean {
  return fs.existsSync(path.join(repoRoot, ".github", "workflows", "jankurai.yml"))
}
