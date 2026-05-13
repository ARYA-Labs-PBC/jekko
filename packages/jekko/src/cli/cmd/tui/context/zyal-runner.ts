// Tails `agent/zyal/runner-events.jsonl` (written by the Rust runner from
// PR3) and exposes the most-recent N events as Solid signals. The audit-live
// panel renders the Workers section by deriving from this stream.

import { createMemo, createSignal } from "solid-js"
import fs from "fs"
import path from "path"

import { DaemonRunnerEvents, type RunnerEvent } from "../../../../session/daemon-runner-events"

const RING_CAP = 32

const [events, setEvents] = createSignal<RunnerEvent[]>([])
const [offset, setOffset] = createSignal(0)

export function useZyalRunnerEvents() {
  return events
}

export const useZyalWorkers = createMemo(() => {
  const map = new Map<string, { workerID: string; kind: RunnerEvent["kind"]; ts: number; data: Record<string, unknown> }>()
  for (const event of events()) {
    const workerID = typeof event.data.worker === "string" ? (event.data.worker as string) : undefined
    if (!workerID) continue
    const existing = map.get(workerID)
    if (!existing || event.ts >= existing.ts) {
      map.set(workerID, { workerID, kind: event.kind, ts: event.ts, data: event.data })
    }
  }
  return Array.from(map.values())
    .sort((a, b) => b.ts - a.ts)
    .slice(0, 6)
})

let watcher: fs.FSWatcher | null = null
let creationPollTimer: ReturnType<typeof setInterval> | undefined
let debounceTimer: ReturnType<typeof setTimeout> | undefined
let activeRoot: string | null = null

function readAndUpdate(repoRoot: string) {
  const { events: parsed, offset: nextOffset } = DaemonRunnerEvents.tail(repoRoot, offset())
  if (parsed.length === 0 && nextOffset === offset()) return
  setOffset(nextOffset)
  if (parsed.length === 0) return
  const merged = [...events(), ...parsed]
  setEvents(merged.length > RING_CAP ? merged.slice(merged.length - RING_CAP) : merged)
}

function debouncedRead(repoRoot: string) {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => readAndUpdate(repoRoot), 250)
}

function startWatcher(repoRoot: string) {
  const file = DaemonRunnerEvents.eventFilePath(repoRoot)
  if (watcher) {
    try {
      watcher.close()
    } catch {}
    watcher = null
  }
  try {
    watcher = fs.watch(file, { persistent: false }, () => debouncedRead(repoRoot))
    watcher.on("error", () => {
      try {
        watcher?.close()
      } catch {}
      watcher = null
      startCreationPoll(repoRoot)
    })
  } catch {
    startCreationPoll(repoRoot)
  }
}

function startCreationPoll(repoRoot: string) {
  if (creationPollTimer) return
  const file = DaemonRunnerEvents.eventFilePath(repoRoot)
  creationPollTimer = setInterval(() => {
    if (fs.existsSync(file)) {
      clearInterval(creationPollTimer!)
      creationPollTimer = undefined
      readAndUpdate(repoRoot)
      startWatcher(repoRoot)
    }
  }, 10_000)
  if (typeof creationPollTimer === "object" && creationPollTimer && "unref" in creationPollTimer) {
    ;(creationPollTimer as { unref?: () => void }).unref?.()
  }
}

export function startZyalRunnerWatch(directory: string) {
  if (activeRoot === directory) return
  stopZyalRunnerWatch()
  activeRoot = directory
  const file = DaemonRunnerEvents.eventFilePath(directory)
  if (fs.existsSync(file)) {
    readAndUpdate(directory)
    startWatcher(directory)
  } else {
    startCreationPoll(directory)
  }
}

export function stopZyalRunnerWatch() {
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
  activeRoot = null
  setOffset(0)
  setEvents([])
}

export function __setEventsForTests(input: RunnerEvent[]) {
  setEvents(input)
}
