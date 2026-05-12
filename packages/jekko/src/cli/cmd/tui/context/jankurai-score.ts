// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
/**
 * Jankurai conformance score context for TUI sidebar.
 *
 * Watches `agent/repo-score.json` in the workspace directory using `fs.watch()`
 * for near-instant change detection with zero CPU cost at idle. Falls back to
 * periodic stat-polling when the file doesn't exist yet.
 *
 * Exposes SolidJS signals consumed by the sidebar plugin:
 *  - jankuraiInstalled()  — whether the CLI is on PATH
 *  - jankuraiScore()      — parsed score data (null if unavailable)
 *  - jankuraiFlash()      — true for 1.5s after score value changes
 *  - jankuraiLastUpdated()— epoch seconds from generated_at
 */
import { createSignal } from "solid-js"
import fs from "fs"
import path from "path"

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type JankuraiScoreData = {
  score: number
  rawScore: number
  hardFindings: number
  softFindings: number
  totalFindings: number
  capsApplied: number
  decision: "pass" | "fail"
  conformanceLevel: string
  standardVersion: string
  generatedAt: number // epoch seconds
}

// ---------------------------------------------------------------------------
// Signals
// ---------------------------------------------------------------------------

const [installed, setInstalled] = createSignal<boolean | null>(null) // null = not checked
const [scoreData, setScoreData] = createSignal<JankuraiScoreData | null>(null)
const [flash, setFlash] = createSignal(false)
const [lastUpdated, setLastUpdated] = createSignal<number | null>(null)

export function useJankuraiInstalled() {
  return installed
}

export function useJankuraiScore() {
  return scoreData
}

export function useJankuraiFlash() {
  return flash
}

export function useJankuraiLastUpdated() {
  return lastUpdated
}

// ---------------------------------------------------------------------------
// Installation detection (cached, no child process)
// ---------------------------------------------------------------------------

let checkedInstall = false

/**
 * Scan PATH directories for a `jankurai` binary without spawning a child
 * process. Pure fs.existsSync — ~0.1ms, no event-loop blocking, no resource
 * cost even under rapid iteration (loop of 1000+).
 */
function detectInstalled(): boolean {
  if (checkedInstall) return installed() ?? false
  checkedInstall = true

  const pathEnv = process.env.PATH ?? ""
  const dirs = pathEnv.split(path.delimiter).filter(Boolean)
  const names = process.platform === "win32" ? ["jankurai.exe", "jankurai.cmd", "jankurai"] : ["jankurai"]

  for (const dir of dirs) {
    for (const name of names) {
      try {
        if (fs.existsSync(path.join(dir, name))) {
          setInstalled(true)
          return true
        }
      } catch {
        // Permission denied on a PATH dir — skip it
      }
    }
  }

  setInstalled(false)
  return false
}

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

function parseScoreJson(raw: string): JankuraiScoreData | null {
  try {
    const d = JSON.parse(raw)
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (typeof d !== "object" || d === null) return null
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (typeof d.score !== "number") return null

    const decision = d.decision ?? {}
    return {
      score: d.score,
      rawScore: d.raw_score ?? d.score,
      hardFindings: decision.hard_findings ?? 0,
      softFindings: decision.soft_findings ?? 0,
      totalFindings: Array.isArray(d.findings) ? d.findings.length : 0,
      capsApplied: Array.isArray(d.caps_applied) ? d.caps_applied.length : 0,
      decision: decision.passed === false ? "fail" : "pass",
      conformanceLevel: d.observed_conformance_level ?? d.claimed_conformance_level ?? "—",
      standardVersion: d.standard_version ?? "—",
      generatedAt: Number(d.generated_at) || 0,
    }
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    return null
  }
}

// ---------------------------------------------------------------------------
// Flash animation trigger
// ---------------------------------------------------------------------------

let flashTimer: ReturnType<typeof setTimeout> | undefined

function triggerFlash() {
  if (flashTimer) clearTimeout(flashTimer)
  setFlash(true)
  flashTimer = setTimeout(() => setFlash(false), 1500)
}

// ---------------------------------------------------------------------------
// Watch lifecycle
// ---------------------------------------------------------------------------

let watcher: fs.FSWatcher | null = null
let creationPollTimer: ReturnType<typeof setInterval> | undefined
let debounceTimer: ReturnType<typeof setTimeout> | undefined
let activeScorePath: string | null = null
let lastScoreValue: number | null = null

function readAndUpdate(scorePath: string) {
  try {
    const raw = fs.readFileSync(scorePath, "utf-8")
    const data = parseScoreJson(raw)
    if (!data) return

    // Detect change for flash
    if (lastScoreValue !== null && lastScoreValue !== data.score) {
      triggerFlash()
    }
    lastScoreValue = data.score

    setScoreData(data)
    setLastUpdated(data.generatedAt || null)
  } catch {
    // File may have been deleted or is mid-write; keep the last good data
  }
}

function debouncedRead(scorePath: string) {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => readAndUpdate(scorePath), 300)
}

function startWatcher(scorePath: string) {
  if (watcher) {
    try {
      watcher.close()
    } catch {}
    watcher = null
  }

  try {
    watcher = fs.watch(scorePath, { persistent: false }, (_eventType) => {
      debouncedRead(scorePath)
    })

    // Handle watcher errors (file deleted, renamed, etc.)
    watcher.on("error", () => {
      try {
        watcher?.close()
      } catch {}
      watcher = null
      // Fall back to creation polling
      startCreationPoll(scorePath)
    })
  } catch {
    // fs.watch can throw if the file doesn't exist
    startCreationPoll(scorePath)
  }
}

function startCreationPoll(scorePath: string) {
  if (creationPollTimer) return

  creationPollTimer = setInterval(() => {
    if (fs.existsSync(scorePath)) {
      clearInterval(creationPollTimer!)
      creationPollTimer = undefined
      readAndUpdate(scorePath)
      startWatcher(scorePath)
    }
  }, 10_000)

  // Don't keep process alive just for polling
  if (typeof creationPollTimer === "object" && creationPollTimer && "unref" in creationPollTimer) {
    ;(creationPollTimer as { unref?: () => void }).unref?.()
  }
}

/**
 * Start watching the jankurai score file in the given workspace directory.
 * Safe to call multiple times — resets the watch if directory changes.
 */
export function startJankuraiWatch(directory: string) {
  if (!detectInstalled()) return

  const scorePath = path.join(directory, "agent", "repo-score.json")

  // If already watching this path, skip
  if (activeScorePath === scorePath) return
  stopJankuraiWatch()

  activeScorePath = scorePath

  if (fs.existsSync(scorePath)) {
    readAndUpdate(scorePath)
    startWatcher(scorePath)
  } else {
    startCreationPoll(scorePath)
  }
}

/**
 * Stop watching and clean up all timers/watchers.
 */
export function stopJankuraiWatch() {
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
  if (flashTimer) {
    clearTimeout(flashTimer)
    flashTimer = undefined
  }
  activeScorePath = null
  // Don't reset lastScoreValue so flash works across re-watches
}

/**
 * Format a "time ago" string from an epoch-seconds timestamp.
 * Relative to `now` (epoch ms) for testability and ticker integration.
 */
export function formatJankuraiAge(generatedAtSec: number, nowMs: number): string {
  const ageSec = Math.max(0, Math.floor((nowMs / 1000) - generatedAtSec))
  if (ageSec < 60) return "just now"
  if (ageSec < 3600) return `${Math.floor(ageSec / 60)}m ago`
  if (ageSec < 86400) return `${Math.floor(ageSec / 3600)}h ago`
  return `${Math.floor(ageSec / 86400)}d ago`
}
