import { describe, expect, test } from "bun:test"
import fs from "fs"
import os from "os"
import path from "path"
import { DaemonRunnerEvents } from "../../src/session/daemon-runner-events"

function tempRepo(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "daemon-runner-events-"))
}

function writeEvents(repo: string, lines: string[]) {
  const file = DaemonRunnerEvents.eventFilePath(repo)
  fs.mkdirSync(path.dirname(file), { recursive: true })
  fs.writeFileSync(file, lines.join("\n") + (lines.length > 0 ? "\n" : ""))
}

function appendEvents(repo: string, lines: string[]) {
  const file = DaemonRunnerEvents.eventFilePath(repo)
  fs.mkdirSync(path.dirname(file), { recursive: true })
  fs.appendFileSync(file, lines.join("\n") + (lines.length > 0 ? "\n" : ""))
}

describe("DaemonRunnerEvents.tail", () => {
  test("missing file returns empty result with same offset", () => {
    const repo = tempRepo()
    const out = DaemonRunnerEvents.tail(repo, 0)
    expect(out.events).toEqual([])
    expect(out.offset).toBe(0)
  })

  test("reads all events from offset 0 on first call", () => {
    const repo = tempRepo()
    writeEvents(repo, [
      JSON.stringify({ ts: 1, kind: "run_started", run_id: "r1", data: { pool_size: 2 } }),
      JSON.stringify({ ts: 2, kind: "worker_started", run_id: "r1", data: { worker: "w-01" } }),
    ])
    const out = DaemonRunnerEvents.tail(repo, 0)
    expect(out.events.length).toBe(2)
    expect(out.events[0].kind).toBe("run_started")
    expect(out.events[1].kind).toBe("worker_started")
    expect(out.offset).toBeGreaterThan(0)
  })

  test("subsequent call from previous offset returns only new events", () => {
    const repo = tempRepo()
    writeEvents(repo, [JSON.stringify({ ts: 1, kind: "run_started", run_id: "r1", data: {} })])
    const first = DaemonRunnerEvents.tail(repo, 0)
    appendEvents(repo, [JSON.stringify({ ts: 2, kind: "commit_landed", run_id: "r1", data: { sha: "abc" } })])
    const second = DaemonRunnerEvents.tail(repo, first.offset)
    expect(second.events.length).toBe(1)
    expect(second.events[0].kind).toBe("commit_landed")
    expect(second.events[0].data.sha).toBe("abc")
  })

  test("malformed JSON lines surface in `malformed` without throwing", () => {
    const repo = tempRepo()
    writeEvents(repo, ["{ not json"])
    const out = DaemonRunnerEvents.tail(repo, 0)
    expect(out.events.length).toBe(0)
    expect(out.malformed.length).toBe(1)
  })

  test("unknown event kind is rejected as malformed", () => {
    const repo = tempRepo()
    writeEvents(repo, [JSON.stringify({ ts: 1, kind: "unknown", run_id: "r1" })])
    const out = DaemonRunnerEvents.tail(repo, 0)
    expect(out.events.length).toBe(0)
    expect(out.malformed.length).toBe(1)
  })

  test("partial trailing line is not consumed", () => {
    const repo = tempRepo()
    const file = DaemonRunnerEvents.eventFilePath(repo)
    fs.mkdirSync(path.dirname(file), { recursive: true })
    // write a complete line + a partial (no trailing newline)
    fs.writeFileSync(
      file,
      JSON.stringify({ ts: 1, kind: "run_started", run_id: "r1", data: {} }) + "\n{\"partial\"",
    )
    const out = DaemonRunnerEvents.tail(repo, 0)
    expect(out.events.length).toBe(1)
    // offset should be past the first line only
    fs.appendFileSync(file, ":true}\n")
    const next = DaemonRunnerEvents.tail(repo, out.offset)
    // the second tail attempt reads the now-complete partial; it's missing
    // required fields so it shows up as malformed (acceptable)
    expect(next.events.length + next.malformed.length).toBeGreaterThan(0)
  })

  test("event file path resolves under agent/zyal/", () => {
    const repo = "/tmp/example"
    const file = DaemonRunnerEvents.eventFilePath(repo)
    expect(file).toBe(path.join(repo, "agent", "zyal", "runner-events.jsonl"))
  })
})
