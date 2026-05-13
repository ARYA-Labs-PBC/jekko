#!/usr/bin/env node
// Reference regression sentinel for example 17. Compares the current
// repo-score.json to a pre-jankurai baseline and halts the daemon run if the
// hard finding count grew. Idempotent: writes a single `regression-streak`
// counter file the stop-condition shell can read.
//
// Usage: regression-sentinel.mjs <pre-baseline.json> <current.json> <streak-file>

import fs from "node:fs"
import path from "node:path"

const [baselinePath, currentPath, streakPath] = process.argv.slice(2)

function readScore(file) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf-8"))
  } catch {
    return null
  }
}

function hard(score) {
  if (!score) return undefined
  if (typeof score?.decision?.hard_findings === "number") return score.decision.hard_findings
  if (Array.isArray(score?.findings)) {
    return score.findings.filter((f) => {
      const s = String(f?.severity ?? "").toLowerCase()
      return s === "critical" || s === "high"
    }).length
  }
  return undefined
}

if (!baselinePath || !currentPath || !streakPath) {
  process.stderr.write("usage: regression-sentinel.mjs <baseline.json> <current.json> <streak-file>\n")
  process.exit(64)
}

const baseline = readScore(baselinePath)
const current = readScore(currentPath)

const baselineHard = hard(baseline) ?? 0
const currentHard = hard(current) ?? 0

if (currentHard > baselineHard) {
  process.stderr.write(`regression: hard findings ${baselineHard} -> ${currentHard}\n`)
  fs.mkdirSync(path.dirname(streakPath), { recursive: true })
  fs.writeFileSync(streakPath, "", "utf-8")
  process.exit(1)
}

// On pass, append one tick to the streak file so the stop condition can count
// consecutive regression-free runs.
fs.mkdirSync(path.dirname(streakPath), { recursive: true })
fs.appendFileSync(streakPath, `${Math.floor(Date.now() / 1000)}\n`)
