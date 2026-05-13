// Reads `agent/baselines/main.repo-score.json` and watches for changes. The
// audit-live panel uses `useJankuraiBaseline()` to compute the Δ column
// against the current score. When the baseline file is missing, deltas
// render as `—`.

import { createSignal } from "solid-js"
import fs from "fs"
import path from "path"

export type JankuraiBaseline = {
  score: number
  hardFindings: number
  softFindings: number
  capsApplied: number
  conformanceLevel: string
  standardVersion: string
}

const [baseline, setBaseline] = createSignal<JankuraiBaseline | null>(null)

export function useJankuraiBaseline() {
  return baseline
}

let watcher: fs.FSWatcher | null = null
let creationPollTimer: ReturnType<typeof setInterval> | undefined
let debounceTimer: ReturnType<typeof setTimeout> | undefined
let activePath: string | null = null

function readAndUpdate(baselinePath: string) {
  try {
    const text = fs.readFileSync(baselinePath, "utf-8")
    const parsed = parseBaselineJson(text)
    if (parsed) setBaseline(parsed)
  } catch {
    // mid-write or rotation: leave the last good value in place
  }
}

export function parseBaselineJson(raw: string): JankuraiBaseline | null {
  try {
    const obj = JSON.parse(raw)
    if (typeof obj !== "object" || obj === null || Array.isArray(obj)) return null
    const r = obj as Record<string, unknown>
    if (typeof r.score !== "number") return null
    const decision = (r.decision && typeof r.decision === "object" ? (r.decision as Record<string, unknown>) : {}) as Record<string, unknown>
    const capsApplied = Array.isArray(r.caps_applied) ? (r.caps_applied as unknown[]).length : 0
    return {
      score: r.score,
      hardFindings: typeof decision.hard_findings === "number" ? (decision.hard_findings as number) : 0,
      softFindings: typeof decision.soft_findings === "number" ? (decision.soft_findings as number) : 0,
      capsApplied,
      conformanceLevel:
        typeof r.observed_conformance_level === "string"
          ? (r.observed_conformance_level as string)
          : typeof r.claimed_conformance_level === "string"
          ? (r.claimed_conformance_level as string)
          : "—",
      standardVersion: typeof r.standard_version === "string" ? (r.standard_version as string) : "—",
    }
  } catch {
    return null
  }
}

function debouncedRead(p: string) {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => readAndUpdate(p), 250)
}

function startWatcher(p: string) {
  if (watcher) {
    try {
      watcher.close()
    } catch {}
    watcher = null
  }
  try {
    watcher = fs.watch(p, { persistent: false }, () => debouncedRead(p))
    watcher.on("error", () => {
      try {
        watcher?.close()
      } catch {}
      watcher = null
      startCreationPoll(p)
    })
  } catch {
    startCreationPoll(p)
  }
}

function startCreationPoll(p: string) {
  if (creationPollTimer) return
  creationPollTimer = setInterval(() => {
    if (fs.existsSync(p)) {
      clearInterval(creationPollTimer!)
      creationPollTimer = undefined
      readAndUpdate(p)
      startWatcher(p)
    }
  }, 10_000)
  if (typeof creationPollTimer === "object" && creationPollTimer && "unref" in creationPollTimer) {
    ;(creationPollTimer as { unref?: () => void }).unref?.()
  }
}

export function startJankuraiBaselineWatch(directory: string) {
  const baselinePath = path.join(directory, "agent", "baselines", "main.repo-score.json")
  if (activePath === baselinePath) return
  stopJankuraiBaselineWatch()
  activePath = baselinePath
  if (fs.existsSync(baselinePath)) {
    readAndUpdate(baselinePath)
    startWatcher(baselinePath)
  } else {
    startCreationPoll(baselinePath)
  }
}

export function stopJankuraiBaselineWatch() {
  if (watcher) {
    try {
      watcher.close()
    } catch {}
    watcher = null
  }
  if (creationPollTimer) {
    clearInterval(creationPollTimer)
    creationPollTimer = undefined
  }
  if (debounceTimer) {
    clearTimeout(debounceTimer)
    debounceTimer = undefined
  }
  activePath = null
}

export function __setBaselineForTests(b: JankuraiBaseline | null) {
  setBaseline(b)
}
