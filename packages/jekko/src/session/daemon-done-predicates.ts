// Registry for `done.require` predicate strings. The schema (`schema-power.ts`
// `ZyalDone`) accepts an arbitrary list of names; the runtime needs to know
// what each name means. PR5 ships the registry plus four canonical predicates
// referenced by example 17:
//
//   • `jankurai_caps_zero`             — `repo-score.json.caps_applied` is empty
//   • `no_new_hard_findings_N_runs`    — last N score-history entries have
//                                        non-increasing hard finding counts
//   • `regression_free_cycles_gte_N`   — last N daemon_regression_cycle rows
//                                        have status="pass"
//   • `score_history_slope_flat_24h`   — the slope of the last 24h of score
//                                        points is within ±1.0 (i.e. stable)

import type { JankuraiHistoryPoint } from "../cli/cmd/tui/context/jankurai-history"

export type DonePredicateInput = {
  /** Current `agent/repo-score.json` snapshot. May be null when missing. */
  score: {
    capsApplied: number
    hardFindings: number
    softFindings: number
    score: number
  } | null
  /** Recent score history points, ordered oldest -> newest. */
  history: readonly JankuraiHistoryPoint[]
  /** Status flags from the last N regression cycles, newest first. */
  regressionStatuses: readonly ("pass" | "regression" | "halted")[]
  /** Threshold tunables. */
  thresholds?: {
    /** N for `no_new_hard_findings_N_runs`. Default 3. */
    noNewHardN?: number
    /** N for `regression_free_cycles_gte_N`. Default 5. */
    regressionFreeN?: number
    /** Max absolute slope in `score_history_slope_flat_24h` (score units per hour). Default 1.0. */
    slopeFlatPerHour?: number
    /** Window for slope check (seconds). Default 24h. */
    slopeWindowSecs?: number
  }
}

export type DonePredicateOutcome = {
  /** Whether the predicate is currently satisfied. */
  satisfied: boolean
  /** Short string explaining the outcome — surfaces in the run card. */
  reason: string
}

export type DonePredicate = (input: DonePredicateInput) => DonePredicateOutcome

const REGISTRY = new Map<string, DonePredicate>()

export function registerDonePredicate(name: string, predicate: DonePredicate): void {
  REGISTRY.set(name, predicate)
}

export function resolveDonePredicate(name: string): DonePredicate | undefined {
  if (REGISTRY.has(name)) return REGISTRY.get(name)
  // Pattern predicates: `regression_free_cycles_gte_<N>` and
  // `no_new_hard_findings_<N>_runs`. Pull the N out and return a closure.
  const regressionMatch = /^regression_free_cycles_gte_(\d+)$/.exec(name)
  if (regressionMatch) {
    const n = parseInt(regressionMatch[1] ?? "5", 10)
    return regressionFreeCycles(n)
  }
  const noNewHardMatch = /^no_new_hard_findings_(\d+)_runs$/.exec(name)
  if (noNewHardMatch) {
    const n = parseInt(noNewHardMatch[1] ?? "3", 10)
    return noNewHardFindings(n)
  }
  return undefined
}

export function evaluateDoneRequire(require: readonly string[], input: DonePredicateInput): {
  satisfied: boolean
  perPredicate: Record<string, DonePredicateOutcome>
  unresolved: string[]
} {
  const perPredicate: Record<string, DonePredicateOutcome> = {}
  const unresolved: string[] = []
  let satisfied = true
  for (const name of require) {
    const predicate = resolveDonePredicate(name)
    if (!predicate) {
      unresolved.push(name)
      satisfied = false
      continue
    }
    const outcome = predicate(input)
    perPredicate[name] = outcome
    if (!outcome.satisfied) satisfied = false
  }
  return { satisfied, perPredicate, unresolved }
}

// ─── built-in predicates ──────────────────────────────────────────────────

export const jankuraiCapsZero: DonePredicate = (input) => {
  if (!input.score) return { satisfied: false, reason: "no audit yet" }
  if (input.score.capsApplied === 0) return { satisfied: true, reason: "caps=0" }
  return { satisfied: false, reason: `caps=${input.score.capsApplied}` }
}

export function noNewHardFindings(n: number): DonePredicate {
  return (input) => {
    if (n <= 0) return { satisfied: true, reason: "n<=0" }
    if (input.history.length < n) {
      return {
        satisfied: false,
        reason: `history=${input.history.length}<${n}`,
      }
    }
    const tail = input.history.slice(input.history.length - n)
    let prev = tail[0]!.hardFindings
    if (prev === undefined) {
      return { satisfied: false, reason: "missing hard count in history" }
    }
    for (let i = 1; i < tail.length; i++) {
      const cur = tail[i]!.hardFindings
      if (cur === undefined) return { satisfied: false, reason: `missing hard count at idx ${i}` }
      if (cur > prev) {
        return { satisfied: false, reason: `hard grew ${prev}->${cur} at idx ${i}` }
      }
      prev = cur
    }
    return { satisfied: true, reason: `${n} runs non-increasing` }
  }
}

export function regressionFreeCycles(n: number): DonePredicate {
  return (input) => {
    if (n <= 0) return { satisfied: true, reason: "n<=0" }
    if (input.regressionStatuses.length < n) {
      return {
        satisfied: false,
        reason: `regression cycles=${input.regressionStatuses.length}<${n}`,
      }
    }
    const tail = input.regressionStatuses.slice(0, n)
    for (let i = 0; i < tail.length; i++) {
      if (tail[i] !== "pass") return { satisfied: false, reason: `cycle ${i}:${tail[i]}` }
    }
    return { satisfied: true, reason: `${n} cycles pass` }
  }
}

export const scoreHistorySlopeFlat24h: DonePredicate = (input) => {
  const slopeFlat = input.thresholds?.slopeFlatPerHour ?? 1.0
  const windowSecs = input.thresholds?.slopeWindowSecs ?? 24 * 60 * 60
  if (input.history.length < 2) {
    return { satisfied: false, reason: `history=${input.history.length}<2` }
  }
  const newest = input.history[input.history.length - 1]!
  const cutoff = newest.ts - windowSecs
  const window = input.history.filter((p) => p.ts >= cutoff)
  if (window.length < 2) {
    return { satisfied: false, reason: `window=${window.length}<2` }
  }
  // Slope = Δscore / Δhours
  const first = window[0]!
  const last = window[window.length - 1]!
  const deltaSecs = last.ts - first.ts
  if (deltaSecs <= 0) {
    return { satisfied: false, reason: "zero-second window" }
  }
  const slopePerHour = ((last.score - first.score) / deltaSecs) * 3600
  if (Math.abs(slopePerHour) <= slopeFlat) {
    return { satisfied: true, reason: `slope=${slopePerHour.toFixed(2)}` }
  }
  return { satisfied: false, reason: `slope=${slopePerHour.toFixed(2)} > ±${slopeFlat}` }
}

// Register the named-only predicates on module load. Pattern predicates (with
// N suffix) resolve on demand via `resolveDonePredicate`.
registerDonePredicate("jankurai_caps_zero", jankuraiCapsZero)
registerDonePredicate("score_history_slope_flat_24h", scoreHistorySlopeFlat24h)

export * as DaemonDonePredicates from "./daemon-done-predicates"
