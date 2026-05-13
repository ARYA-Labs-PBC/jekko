#!/usr/bin/env node
// Reference classifier for example 17. Reads agent/repo-score.json (and the
// optional next-finding hint at $JANKURAI_NEXT_FINDING_FILE), emits a single
// `{lane, reason}` JSON line on stdout AND writes it to the file declared by
// `dispatch.classifier.write_to`. The daemon-side `daemon-dispatch.ts`
// resolver only reads the file — stdout is a convenience for humans.
//
// Lane rules (mirror the planning table for example 17):
//   • cap finding              -> cap_incubator
//   • critical / high severity -> cap_incubator
//   • paths overlap research-required heuristics (web-content rule_ids,
//     research_ prefix, etc.)                       -> research_required
//   • touches **/auth/** or **/migrations/**        -> human_required
//   • single path, low severity, no cap             -> simple_parallel_batch
//   • multiple-path low severity                     -> cross_file_dag
//   • otherwise                                       -> simple_parallel_batch

import fs from "node:fs"
import path from "node:path"

const REPO = process.env.JANKURAI_REPO_ROOT ?? process.cwd()
const SCORE_PATH = path.join(REPO, "agent", "repo-score.json")
const WRITE_TO =
  process.env.JANKURAI_DISPATCH_WRITE_TO ??
  path.join(REPO, "target", "jankurai", "dispatch", "route.json")

function readScore() {
  try {
    return JSON.parse(fs.readFileSync(SCORE_PATH, "utf-8"))
  } catch {
    return null
  }
}

function classifyFinding(finding) {
  if (finding?.cap || (typeof finding?.rule_id === "string" && finding.rule_id.startsWith("cap:"))) {
    return { lane: "cap_incubator", reason: "cap" }
  }
  const severity = String(finding?.severity ?? "").toLowerCase()
  if (severity === "critical" || severity === "high") {
    return { lane: "cap_incubator", reason: `severity:${severity}` }
  }
  const paths = collectPaths(finding)
  if (paths.some((p) => /(^|\/)(auth|migrations)\//i.test(p))) {
    return { lane: "human_required", reason: "touches:auth_or_migrations" }
  }
  const ruleID = String(finding?.rule_id ?? "")
  if (/^research_/i.test(ruleID) || /web_content/i.test(ruleID)) {
    return { lane: "research_required", reason: "rule:research" }
  }
  if (paths.length > 1) {
    return { lane: "cross_file_dag", reason: `paths:${paths.length}` }
  }
  return { lane: "simple_parallel_batch", reason: "single_path_low" }
}

function collectPaths(finding) {
  const out = []
  if (typeof finding?.path === "string") out.push(finding.path)
  if (typeof finding?.file === "string") out.push(finding.file)
  if (Array.isArray(finding?.paths)) for (const p of finding.paths) if (typeof p === "string") out.push(p)
  if (Array.isArray(finding?.affected_files))
    for (const p of finding.affected_files) if (typeof p === "string") out.push(p)
  return Array.from(new Set(out))
}

function pickFinding(score) {
  if (!score) return undefined
  const caps = Array.isArray(score.caps_applied) ? score.caps_applied : []
  if (caps.length > 0) return { rule_id: `cap:${caps[0]?.id ?? "unknown"}`, cap: caps[0]?.id }
  const findings = Array.isArray(score.findings) ? score.findings : []
  if (findings.length === 0) return undefined
  // pick the worst severity first
  const order = { critical: 4, high: 3, medium: 2, low: 1, info: 0 }
  return findings
    .slice()
    .sort((a, b) => (order[String(b?.severity).toLowerCase()] ?? 0) - (order[String(a?.severity).toLowerCase()] ?? 0))[0]
}

const score = readScore()
const finding = pickFinding(score)
const decision = finding ? classifyFinding(finding) : { lane: "simple_parallel_batch", reason: "no_finding" }

fs.mkdirSync(path.dirname(WRITE_TO), { recursive: true })
fs.writeFileSync(WRITE_TO, JSON.stringify(decision) + "\n", "utf-8")
process.stdout.write(JSON.stringify(decision) + "\n")
