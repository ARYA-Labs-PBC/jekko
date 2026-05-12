// Critical-reviewer pass implementation. Slots between `prototype` and
// `promotion_review` in the incubator chain when the new pass type from PR2
// (`critical_reviewer`) is used. Evaluates the structured checklist declared
// at `jankurai.reviewer.checklist` and returns a verdict the host can use to
// block or allow promotion.
//
// PR4 ships the evaluator + verdict shape only. Daemon plumbing into the
// incubator loop (so the verdict materialises as a daemon-task-memory row
// with kind "critical_reviewer") is added separately.

import type { ZyalJankuraiReviewer, ZyalJankuraiReviewerChecklistItem } from "@/agent-script/schema"

export type ReviewerSeverity = "info" | "warning" | "blocker"

export type ReviewerCheckOutcome = "pass" | "warn" | "block" | "skip"

/**
 * Per-item evaluation result. The caller (an agent pass) supplies one of
 * these per checklist item via the structured-output assertion contract.
 */
export type ReviewerCheckResult = {
  id: string
  outcome: ReviewerCheckOutcome
  notes?: string
  /** Optional severity override; otherwise the checklist item's severity wins. */
  severityOverride?: ReviewerSeverity
}

export type ReviewerGap = {
  id: string
  severity: ReviewerSeverity
  notes?: string
}

export type ReviewerVerdict = {
  /** True when promotion should be blocked. */
  block: boolean
  /** Per-item gaps that the reviewer surfaced. */
  gaps: ReviewerGap[]
  /** Aggregate severity for telemetry. */
  severity: ReviewerSeverity
  /** Human-readable summary suitable for memory.summary. */
  summary: string
}

export const DEFAULT_CHECKLIST: ZyalJankuraiReviewerChecklistItem[] = [
  { id: "untested_edges", severity: "warning" },
  { id: "parallel_side_effects", severity: "warning" },
  { id: "regression_risk", severity: "blocker" },
  { id: "doc_owner_map_drift", severity: "info" },
  { id: "tool_adoption_drift", severity: "info" },
  { id: "proof_lane_coverage", severity: "blocker" },
]

export function effectiveChecklist(spec: ZyalJankuraiReviewer | undefined): ZyalJankuraiReviewerChecklistItem[] {
  const declared = spec?.checklist
  if (!declared || declared.length === 0) return DEFAULT_CHECKLIST
  return Array.from(declared)
}

export function blocksPromotion(spec: ZyalJankuraiReviewer | undefined): boolean {
  if (spec?.block_promotion === false) return false
  return true
}

/**
 * Evaluate the reviewer pass. Missing items are treated as `skip` — neither
 * pass nor block — so a partial response cannot accidentally unblock a
 * promotion. The verdict's `block` flag is true if any outcome resolves to a
 * blocker severity (after override), and `block_promotion` is enabled.
 */
export function evaluate(input: {
  spec: ZyalJankuraiReviewer | undefined
  results: readonly ReviewerCheckResult[]
}): ReviewerVerdict {
  const checklist = effectiveChecklist(input.spec)
  const blockOnSpec = blocksPromotion(input.spec)
  const byId = new Map(input.results.map((r) => [r.id, r]))

  const gaps: ReviewerGap[] = []
  let aggregateSeverity: ReviewerSeverity = "info"
  let blockFromResults = false

  for (const item of checklist) {
    const result = byId.get(item.id)
    const severity = result?.severityOverride ?? item.severity ?? "info"
    if (!result) {
      gaps.push({ id: item.id, severity, notes: "no result" })
      aggregateSeverity = mergeSeverity(aggregateSeverity, severity)
      continue
    }
    switch (result.outcome) {
      case "pass":
        // pass cleanly, no gap
        break
      case "warn":
        gaps.push({ id: item.id, severity, notes: result.notes })
        aggregateSeverity = mergeSeverity(aggregateSeverity, severity)
        break
      case "block":
        gaps.push({ id: item.id, severity: "blocker", notes: result.notes })
        aggregateSeverity = mergeSeverity(aggregateSeverity, "blocker")
        blockFromResults = true
        break
      case "skip":
        // explicit skip; record but do not aggravate severity
        gaps.push({ id: item.id, severity, notes: result.notes ?? "skipped" })
        break
    }
  }

  const block = blockOnSpec && (blockFromResults || gaps.some((g) => g.severity === "blocker"))
  const summary = renderSummary({ block, aggregateSeverity, gaps })
  return { block, gaps, severity: aggregateSeverity, summary }
}

const SEVERITY_ORDER: Record<ReviewerSeverity, number> = {
  info: 0,
  warning: 1,
  blocker: 2,
}

function mergeSeverity(a: ReviewerSeverity, b: ReviewerSeverity): ReviewerSeverity {
  return SEVERITY_ORDER[b] > SEVERITY_ORDER[a] ? b : a
}

function renderSummary(input: { block: boolean; aggregateSeverity: ReviewerSeverity; gaps: ReviewerGap[] }): string {
  if (input.gaps.length === 0) return "reviewer:clean"
  const head = input.block ? "reviewer:block" : `reviewer:${input.aggregateSeverity}`
  const tail = input.gaps
    .slice(0, 6)
    .map((g) => `${g.id}:${g.severity}${g.notes ? `(${g.notes})` : ""}`)
    .join(", ")
  return `${head} ${tail}`
}

export * as DaemonReviewerPass from "./daemon-reviewer-pass"
