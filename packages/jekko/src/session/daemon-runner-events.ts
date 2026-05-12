// Tails the NDJSON event stream the Rust runner writes at
// `agent/zyal/runner-events.jsonl` (PR3) and parses each line into a typed
// event the daemon can act on. Used by `daemon-jankurai.ts` to bridge runner
// progress into daemon-store and into HTTP/TUI surfaces.
//
// We deliberately keep this layer thin: no fs.watch yet (that lands when the
// daemon loop opts in). The tailer is stateless — the caller passes the last
// byte offset it consumed so a subsequent call only reads new bytes.

import fs from "fs"
import path from "path"

export type RunnerEventKind =
  | "run_started"
  | "worker_started"
  | "worker_pass"
  | "worker_fail"
  | "commit_landed"
  | "rebase_conflict"
  | "worker_rollback"
  | "gc_pruned"
  | "run_finished"
  | "bootstrap_required"

export type RunnerEvent = {
  ts: number
  kind: RunnerEventKind
  runID: string
  data: Record<string, unknown>
}

export type TailResult = {
  events: RunnerEvent[]
  /** New byte offset the caller should pass back next time. */
  offset: number
  /** Lines that failed to parse — useful for surfacing corrupt rows. */
  malformed: string[]
}

export const EVENT_FILE_REL = "agent/zyal/runner-events.jsonl"

export function eventFilePath(repoRoot: string): string {
  return path.join(repoRoot, EVENT_FILE_REL)
}

/**
 * Read every byte after `offset` from the event file. If the file does not
 * exist yet (the runner hasn't written anything), returns an empty result
 * with the original offset.
 */
export function tail(repoRoot: string, offset: number): TailResult {
  const file = eventFilePath(repoRoot)
  let stat: fs.Stats
  try {
    stat = fs.statSync(file)
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === "ENOENT") {
      return { events: [], offset, malformed: [] }
    }
    throw err
  }
  if (stat.size <= offset) {
    return { events: [], offset, malformed: [] }
  }
  const fd = fs.openSync(file, "r")
  try {
    const length = stat.size - offset
    const buf = Buffer.alloc(length)
    fs.readSync(fd, buf, 0, length, offset)
    const events: RunnerEvent[] = []
    const malformed: string[] = []
    // Walk the buffer one line at a time. A line ends at the next `\n`; a
    // trailing chunk without a `\n` is considered partial and not consumed.
    let consumed = offset
    let cursor = 0
    while (cursor < buf.length) {
      const nl = buf.indexOf(0x0a, cursor)
      if (nl === -1) {
        // Partial trailing line — leave for the next call.
        break
      }
      const lineBuf = buf.slice(cursor, nl)
      const line = lineBuf.toString("utf-8")
      consumed += lineBuf.length + 1 // +1 for the newline itself
      cursor = nl + 1
      if (line.length === 0) continue
      const parsed = parseLine(line)
      if (parsed) events.push(parsed)
      else malformed.push(line)
    }
    return { events, offset: consumed, malformed }
  } finally {
    fs.closeSync(fd)
  }
}

function parseLine(raw: string): RunnerEvent | undefined {
  try {
    const obj = JSON.parse(raw)
    if (typeof obj !== "object" || obj === null) return undefined
    const r = obj as Record<string, unknown>
    if (typeof r.ts !== "number" || typeof r.kind !== "string" || typeof r.run_id !== "string") return undefined
    if (!isRunnerEventKind(r.kind)) return undefined
    return {
      ts: r.ts,
      kind: r.kind,
      runID: r.run_id,
      data: (r.data && typeof r.data === "object" && !Array.isArray(r.data) ? (r.data as Record<string, unknown>) : {}),
    }
  } catch {
    return undefined
  }
}

const KNOWN_KINDS: ReadonlySet<RunnerEventKind> = new Set<RunnerEventKind>([
  "run_started",
  "worker_started",
  "worker_pass",
  "worker_fail",
  "commit_landed",
  "rebase_conflict",
  "worker_rollback",
  "gc_pruned",
  "run_finished",
  "bootstrap_required",
])

function isRunnerEventKind(value: unknown): value is RunnerEventKind {
  return typeof value === "string" && KNOWN_KINDS.has(value as RunnerEventKind)
}

export * as DaemonRunnerEvents from "./daemon-runner-events"
