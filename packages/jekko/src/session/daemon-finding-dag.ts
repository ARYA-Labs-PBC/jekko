// TS mirror of the Rust DAG scheduler in `crates/jankurai-runner/src/dag.rs`.
// Builds path-overlap waves so the daemon can plan parallel dispatch without
// blocking on the Rust runner's NDJSON stream. Both implementations agree on:
//
//   • caps land alone in wave 0
//   • higher severity packs first inside a wave
//   • overlapping paths push the loser to the next wave
//
// Pure functions; no I/O.

import { compareSeverity, type Finding } from "./daemon-finding-classifier"

export type Batch = {
  findings: Finding[]
}

export type Wave = {
  index: number
  batches: Batch[]
}

export function touchedPaths(batch: Batch): string[] {
  const set = new Set<string>()
  for (const f of batch.findings) {
    for (const p of f.paths) set.add(p)
  }
  return Array.from(set).sort()
}

/**
 * Schedules findings into a sequence of waves. Each wave's batches are
 * path-disjoint with respect to each other, so they can run in parallel
 * without lock contention. Later waves may overlap with earlier ones but
 * never start until the earlier wave drains.
 */
export function schedule(findings: readonly Finding[]): Wave[] {
  if (findings.length === 0) return []
  const waves: Wave[] = []

  // Wave 0 — every cap in its own batch.
  const caps = findings.filter((f) => f.cap !== undefined)
  if (caps.length > 0) {
    waves.push({ index: waves.length, batches: caps.map((f) => ({ findings: [f] })) })
  }

  let remaining: Finding[] = findings.filter((f) => f.cap === undefined).slice()
  // Sort by severity desc, then by rule id for stable output.
  remaining.sort((a, b) => compareSeverity(a.severity, b.severity) || a.ruleID.localeCompare(b.ruleID))

  while (remaining.length > 0) {
    const { wave, leftover } = packOneWave(remaining, waves.length)
    if (wave.batches.length === 0) {
      // Defensive: never an infinite loop. Promote everything left to a final
      // wave of singletons.
      waves.push({
        index: waves.length,
        batches: leftover.map((f) => ({ findings: [f] })),
      })
      break
    }
    waves.push(wave)
    remaining = leftover
  }

  return waves
}

function packOneWave(findings: readonly Finding[], waveIndex: number): { wave: Wave; leftover: Finding[] } {
  const claimed = new Set<string>()
  const batches: Batch[] = []
  const leftover: Finding[] = []
  for (const finding of findings) {
    const conflict = finding.paths.some((p) => claimed.has(p))
    if (conflict) {
      leftover.push(finding)
      continue
    }
    for (const p of finding.paths) claimed.add(p)
    batches.push({ findings: [finding] })
  }
  return { wave: { index: waveIndex, batches }, leftover }
}

export * as DaemonFindingDag from "./daemon-finding-dag"
