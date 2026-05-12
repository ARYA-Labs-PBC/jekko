// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
// TS mirror of the Rust classifier in `crates/jankurai-runner/src/classifier.rs`.
// Same severity ladder, same caps-become-synthetic-criticals rule, same path
// collection logic. The daemon side reads `agent/repo-score.json` directly so
// it can plan parallel waves in-process even when the Rust runner is offline.
//
// Pure functions. No I/O beyond `fs.readFileSync`. Effect-free so this module
// can be called from synchronous code paths (e.g. preview).

import fs from "fs"
import path from "path"

export type Severity = "critical" | "high" | "medium" | "low" | "info"

export type Finding = {
  ruleID: string
  fingerprint: string
  severity: Severity
  paths: string[]
  /** `undefined` for rule findings; the cap id for synthetic cap findings. */
  cap?: string
}

export type ClassifyResult = {
  findings: Finding[]
  capsTotal: number
  hardTotal: number
  softTotal: number
  score?: number
}

const SEVERITY_RANK: Record<Severity, number> = {
  critical: 4,
  high: 3,
  medium: 2,
  low: 1,
  info: 0,
}

export function parseSeverity(raw: unknown): Severity {
  if (typeof raw !== "string") return "info"
  const lower = raw.toLowerCase()
  switch (lower) {
    case "critical":
      return "critical"
    case "high":
      return "high"
    case "medium":
    case "med":
      return "medium"
    case "low":
      return "low"
    default:
      return "info"
  }
}

export function isHard(severity: Severity): boolean {
  return severity === "critical" || severity === "high"
}

export function compareSeverity(a: Severity, b: Severity): number {
  return SEVERITY_RANK[b] - SEVERITY_RANK[a]
}

export function classify(repoRoot: string): ClassifyResult {
  const scorePath = path.join(repoRoot, "agent", "repo-score.json")
  const text = fs.readFileSync(scorePath, "utf-8")
  return classifyText(text)
}

export function classifyText(text: string): ClassifyResult {
  let parsed: unknown
  try {
    parsed = JSON.parse(text)
  } catch (err) {
    throw new Error(`parse agent/repo-score.json: ${(err as Error).message}`)
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("agent/repo-score.json is not an object")
  }
  const root = parsed as Record<string, unknown>

  const findings: Finding[] = []

  const rawFindings = Array.isArray(root.findings) ? root.findings : []
  for (const raw of rawFindings) {
    if (typeof raw !== "object" || raw === null) continue
    const f = raw as Record<string, unknown>
    findings.push({
      ruleID: stringField(f, "rule_id") ?? stringField(f, "id") ?? stringField(f, "rule") ?? "",
      fingerprint: stringField(f, "fingerprint") ?? "",
      severity: parseSeverity(f.severity),
      paths: collectPaths(f),
      cap: undefined,
    })
  }

  const rawCaps = Array.isArray(root.caps_applied) ? root.caps_applied : []
  for (const raw of rawCaps) {
    if (typeof raw !== "object" || raw === null) continue
    const c = raw as Record<string, unknown>
    const id = stringField(c, "id") ?? "unknown"
    const affects = collectStringArray(c, ["affects"])
    findings.push({
      ruleID: `cap:${id}`,
      fingerprint: `cap:${id}`,
      severity: "critical",
      paths: affects,
      cap: id,
    })
  }

  const capsTotal = findings.filter((f) => f.cap !== undefined).length
  const hardTotal = findings.filter((f) => isHard(f.severity) && f.cap === undefined).length
  const softTotal = Math.max(0, findings.length - capsTotal - hardTotal)

  return {
    findings,
    capsTotal,
    hardTotal,
    softTotal,
    score: typeof root.score === "number" ? root.score : undefined,
  }
}

function stringField(obj: Record<string, unknown>, key: string): string | undefined {
  const value = obj[key]
  return typeof value === "string" ? value : undefined
}

function collectPaths(raw: Record<string, unknown>): string[] {
  const out: string[] = []
  const single = stringField(raw, "path") ?? stringField(raw, "file")
  if (single) out.push(single)
  out.push(...collectStringArray(raw, ["paths", "affected_files"]))
  // de-dup + stable sort so two equivalent classify() calls return the same
  // order — the DAG step depends on this for deterministic wave packing.
  return Array.from(new Set(out)).sort()
}

function collectStringArray(raw: Record<string, unknown>, keys: string[]): string[] {
  const out: string[] = []
  for (const key of keys) {
    const value = raw[key]
    if (Array.isArray(value)) {
      for (const v of value) {
        if (typeof v === "string") out.push(v)
      }
    }
  }
  return out
}

export * as DaemonFindingClassifier from "./daemon-finding-classifier"
