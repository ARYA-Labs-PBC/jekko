// Tails `agent/score-history.jsonl` and exposes the rolling tail as a Solid
// signal. The audit-live panel reads this for the sparkline above the score.
// Same fs.watch + creation-poll recovery pattern as `jankurai-score.ts` so the
// watcher keeps working when the file is missing or rotated.

import { createSignal } from "solid-js"
import fs from "fs"
import path from "path"

export type JankuraiHistoryPoint = {
  ts: number
  score: number
  hardFindings?: number
  softFindings?: number
  capsApplied?: number
}

const RING_CAP = 200

const [history, setHistory] = createSignal<JankuraiHistoryPoint[]>([])

export function useJankuraiHistory() {
  return history
}

let watcher: fs.FSWatcher | null = null
let creationPollTimer: ReturnType<typeof setInterval> | undefined
let debounceTimer: ReturnType<typeof setTimeout> | undefined
let activeHistoryPath: string | null = null

function readAndUpdate(historyPath: string) {
  try {
    const stat = fs.statSync(historyPath)
    const size = stat.size
    // Read the tail of the file — at most ~64 KB covers the rolling-200 cap
    // even with verbose records. Smaller files read fully.
    const tailBytes = Math.min(size, 64 * 1024)
    const startOffset = size - tailBytes
    const fd = fs.openSync(historyPath, "r")
    try {
      const buf = Buffer.alloc(tailBytes)
      fs.readSync(fd, buf, 0, tailBytes, startOffset)
      const text = buf.toString("utf-8")
      const lines = text.split("\n")
      // If we started mid-line (startOffset > 0), the first piece is partial
      // and must be dropped.
      const usable = startOffset > 0 ? lines.slice(1) : lines
      const out: JankuraiHistoryPoint[] = []
      for (const line of usable) {
        const trimmed = line.trim()
        if (!trimmed) continue
        const parsed = parseLine(trimmed)
        if (parsed) out.push(parsed)
      }
      // Cap the ring buffer at the most-recent RING_CAP entries.
      setHistory(out.length > RING_CAP ? out.slice(out.length - RING_CAP) : out)
    } finally {
      fs.closeSync(fd)
    }
  } catch {
    // mid-rotation; keep last good signal value
  }
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function parseLine(line: string): JankuraiHistoryPoint | undefined {
  try {
    return buildHistoryPoint(line)
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=parse-boundary-catches-malformed-jsonl-lines expires=2027-01-01
    return undefined
  }
}

function buildHistoryPoint(line: string): JankuraiHistoryPoint {
  const obj = JSON.parse(line)
  if (!isPlainObject(obj)) throw new Error("not a plain object")
  const ts = typeof obj.ts === "number" ? obj.ts : typeof obj.generated_at === "number" ? obj.generated_at : undefined
  const score = typeof obj.score === "number" ? obj.score : undefined
  if (ts === undefined || score === undefined) throw new Error("missing ts or score")
  const decision = isPlainObject(obj.decision) ? obj.decision : {}
  return {
    ts,
    score,
    hardFindings: typeof obj.hardFindings === "number"
      ? obj.hardFindings
      : typeof decision.hard_findings === "number"
      ? decision.hard_findings
      : undefined,
    softFindings: typeof obj.softFindings === "number"
      ? obj.softFindings
      : typeof decision.soft_findings === "number"
      ? decision.soft_findings
      : undefined,
    capsApplied: typeof obj.capsApplied === "number"
      ? obj.capsApplied
      : Array.isArray(obj.caps_applied)
      ? obj.caps_applied.length
      : undefined,
  }
}

function debouncedRead(historyPath: string) {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => readAndUpdate(historyPath), 250)
}

function startWatcher(historyPath: string) {
  if (watcher) {
    try {
      watcher.close()
    } catch {}
    watcher = null
  }
  try {
    watcher = fs.watch(historyPath, { persistent: false }, () => debouncedRead(historyPath))
    watcher.on("error", () => {
      try {
        watcher?.close()
      } catch {}
      watcher = null
      startCreationPoll(historyPath)
    })
  } catch {
    startCreationPoll(historyPath)
  }
}

function startCreationPoll(historyPath: string) {
  if (creationPollTimer) return
  creationPollTimer = setInterval(() => {
    if (fs.existsSync(historyPath)) {
      clearInterval(creationPollTimer!)
      creationPollTimer = undefined
      readAndUpdate(historyPath)
      startWatcher(historyPath)
    }
  }, 10_000)
  if (typeof creationPollTimer === "object" && creationPollTimer && "unref" in creationPollTimer) {
    ;(creationPollTimer as { unref?: () => void }).unref?.()
  }
}

export function startJankuraiHistoryWatch(directory: string) {
  const historyPath = path.join(directory, "agent", "score-history.jsonl")
  if (activeHistoryPath === historyPath) return
  stopJankuraiHistoryWatch()
  activeHistoryPath = historyPath
  if (fs.existsSync(historyPath)) {
    readAndUpdate(historyPath)
    startWatcher(historyPath)
  } else {
    startCreationPoll(historyPath)
  }
}

export function stopJankuraiHistoryWatch() {
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
  activeHistoryPath = null
}

/** Test-only hook: replace the signal value without spinning up fs.watch. */
export function __setHistoryForTests(points: JankuraiHistoryPoint[]) {
  setHistory(points)
}

/** Test-only hook: read a path directly without registering watchers. */
export function __readHistoryFileForTests(historyPath: string): JankuraiHistoryPoint[] {
  readAndUpdate(historyPath)
  return history()
}
