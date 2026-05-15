import { describe, expect, spyOn, test } from "bun:test"
import { mkdtemp, writeFile } from "fs/promises"
import path from "path"
import { tmpdir } from "os"
import { collectDaemonProgress, formatDaemonEventLine } from "../../../src/session/daemon-progress"
import { DaemonStartCommand, DaemonStatusCommand, formatDaemonRunList, formatDaemonRunSummary, formatDaemonTaskList } from "../../../src/cli/cmd/daemon"
import * as Server from "@/server/server"

function jsonResponse(payload: unknown) {
  return new Response(JSON.stringify(payload), {
    headers: { "content-type": "application/json" },
  })
}

describe("daemon cli summaries", () => {
  test("render compact run and task summaries", () => {
    const run = {
      id: "run-1",
      status: "armed",
      phase: "evaluating_stop",
      iteration: 4,
      epoch: 2,
      last_error: null,
    }
    const tasks = [
      {
        id: "task-1",
        title: "Investigate latency",
        lane: "incubator",
        status: "incubating",
        readiness_score: 0.62,
        risk_score: 0.31,
        blocked_reason: null,
      },
    ]

    expect(formatDaemonRunSummary(run, tasks.length)).toContain("status armed")
    expect(formatDaemonRunSummary(run, tasks.length)).toContain("tasks 1")
    expect(formatDaemonTaskList(tasks)).toContain("ready 62%")
    expect(formatDaemonTaskList(tasks)).toContain("passes 0")
  })

  test("renders daemon status list output", () => {
    expect(formatDaemonRunList([])).toBe("No daemon runs.")
    expect(
      formatDaemonRunList([
        {
          id: "run-1",
          status: "running",
          phase: "iteration",
          iteration: 1,
          epoch: 1,
          last_error: null,
        },
      ]),
    ).toContain("run run-1")
  })

  test("builds a compact progress snapshot from stage events", () => {
    const progress = collectDaemonProgress([
      {
        event_type: "run.created",
        payload_json: {},
      },
      {
        event_type: "jankurai.seeded_artifacts.reused",
        payload_json: {
          reportPath: "target/jankurai/repo-score.json",
          repairPlanPath: "target/jankurai/repair-plan.json",
        },
      },
      {
        event_type: "jankurai.worker_wave.completed",
        payload_json: {
          started: 0,
          verified: 0,
          blocked: 0,
          reason: "no conflict-free task",
        },
      },
    ] as any)

    expect(progress.lastSuccessfulStage).toBe("worker_wave.completed")
    expect(progress.seededArtifacts).toBe("reused")
    expect(progress.blockedReasons).toContain("no conflict-free task")
    expect(formatDaemonEventLine({
      event_type: "jankurai.seeded_artifacts.reused",
      payload_json: {
        reportPath: "target/jankurai/repo-score.json",
        repairPlanPath: "target/jankurai/repair-plan.json",
      },
    } as any)?.text).toContain("seeded_artifacts.reused")
  })

  test("surfaces worker command failures in run status summaries", () => {
    const progress = collectDaemonProgress([
      {
        event_type: "jankurai.worker.blocked",
        payload_json: {
          reason: "command failed: just fast; exitCode=1; stdout=(empty); stderr=(empty)",
        },
      },
    ] as any)

    expect(progress.blockedReasons[0]).toContain("command failed: just fast")
    expect(
      formatDaemonRunSummary(
        {
          id: "run-1",
          status: "running",
          phase: "iteration",
          iteration: 1,
          epoch: 0,
          last_error: null,
        },
        0,
        progress,
      ),
    ).toContain("blocked command failed: just fast")
  })

  test("streams daemon start progress instead of dumping JSON", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "jekko-daemon-cli-"))
    const file = path.join(dir, "daemon.zyal")
    await writeFile(file, "dummy daemon text")

    const runID = "run-cli-progress"
    const sessionID = "session-cli-progress"
    const events: any[] = [
      { event_type: "run.created", payload_json: {}, time_created: 1 },
      { event_type: "run.previewed", payload_json: { preview: { spec: { job: { name: "CLI Progress" } } } }, time_created: 2 },
      { event_type: "jankurai.seeded_artifacts.reused", payload_json: { reportPath: "target/jankurai/repo-score.json", repairPlanPath: "target/jankurai/repair-plan.json" }, time_created: 3 },
      { event_type: "jankurai.worker_wave.started", payload_json: { workers: 1 }, time_created: 4 },
      { event_type: "jankurai.worker_wave.completed", payload_json: { workers: 1, started: 0, verified: 0, blocked: 0, reason: "no conflict-free task" }, time_created: 5 },
      { event_type: "jankurai.sleeping", payload_json: { sleep: "5 seconds" }, time_created: 6 },
    ]
    const statusSequence = ["running", "running", "satisfied"]
    let statusCalls = 0
    let eventCalls = 0
    const defaultSpy = spyOn(Server, "Default").mockReturnValue({
      app: {
        fetch: async (input: Request) => {
          const url = new URL(input.url)
          if (url.pathname === "/daemon/preview") {
            return jsonResponse({ spec: { job: { name: "CLI Progress" } } })
          }
          if (url.pathname === "/session") {
            return jsonResponse({ id: sessionID })
          }
          if (url.pathname === `/session/${sessionID}/daemon/start`) {
            return jsonResponse({ id: runID })
          }
          if (url.pathname === `/daemon/${runID}`) {
            const status = statusSequence[Math.min(statusCalls, statusSequence.length - 1)]
            statusCalls += 1
            return jsonResponse({
              id: runID,
              status,
              phase: status === "satisfied" ? "terminal" : "running_iteration",
              iteration: status === "satisfied" ? 1 : 0,
              epoch: 0,
              last_error: null,
            })
          }
          if (url.pathname === `/daemon/${runID}/events`) {
            eventCalls += 1
            return jsonResponse(events.slice(0, Math.min(events.length, eventCalls + 2)))
          }
          if (url.pathname === `/daemon/${runID}/tasks`) {
            return jsonResponse([])
          }
          throw new Error(`Unexpected request path: ${url.pathname}`)
        },
      },
    } as unknown as ReturnType<typeof Server.Default>)

    const writes: string[] = []
    const write = spyOn(process.stdout, "write").mockImplementation((chunk: string | Uint8Array) => {
      writes.push(String(chunk))
      return true
    })

    try {
      await DaemonStartCommand.handler({
        file,
        arm: "RUN_FOREVER",
        watch: true,
        session: undefined,
        attach: undefined,
        password: undefined,
        username: undefined,
      } as any)

      const output = writes.join("")
      expect(output).toContain("seeded_artifacts.reused")
      expect(output).toContain("worker_wave.completed")
      expect(output).toContain("\u001b[")
      expect(output).not.toContain("{\n")
    } finally {
      write.mockRestore()
      defaultSpy.mockRestore()
    }
  })

  test("streams daemon status progress when watch is enabled", async () => {
    const runID = "run-cli-status-watch"
    const events: any[] = [
      { event_type: "run.created", payload_json: {}, time_created: 1 },
      { event_type: "jankurai.seeded_artifacts.reused", payload_json: { reportPath: "target/jankurai/repo-score.json", repairPlanPath: "target/jankurai/repair-plan.json" }, time_created: 2 },
      { event_type: "jankurai.worker_wave.completed", payload_json: { workers: 1, started: 0, verified: 0, blocked: 0, reason: "no conflict-free task" }, time_created: 3 },
    ]
    const statusSequence = ["running", "running", "satisfied"]
    let statusCalls = 0
    let eventCalls = 0
    const defaultSpy = spyOn(Server, "Default").mockReturnValue({
      app: {
        fetch: async (input: Request) => {
          const url = new URL(input.url)
          if (url.pathname === `/daemon/${runID}`) {
            const status = statusSequence[Math.min(statusCalls, statusSequence.length - 1)]
            statusCalls += 1
            return jsonResponse({
              id: runID,
              status,
              phase: status === "satisfied" ? "terminal" : "running_iteration",
              iteration: status === "satisfied" ? 1 : 0,
              epoch: 0,
              last_error: null,
            })
          }
          if (url.pathname === `/daemon/${runID}/events`) {
            eventCalls += 1
            return jsonResponse(events.slice(0, Math.min(events.length, eventCalls + 1)))
          }
          if (url.pathname === `/daemon/${runID}/tasks`) {
            return jsonResponse([])
          }
          throw new Error(`Unexpected request path: ${url.pathname}`)
        },
      },
    } as unknown as ReturnType<typeof Server.Default>)

    const writes: string[] = []
    const write = spyOn(process.stdout, "write").mockImplementation((chunk: string | Uint8Array) => {
      writes.push(String(chunk))
      return true
    })

    try {
      await DaemonStatusCommand.handler({
        runID,
        watch: true,
        attach: undefined,
        password: undefined,
        username: undefined,
      } as any)

      const output = writes.join("")
      expect(output).toContain("seeded_artifacts.reused")
      expect(output).toContain("worker_wave.completed")
      expect(output).toContain("\u001b[")
      expect(output).toContain("status run")
    } finally {
      write.mockRestore()
      defaultSpy.mockRestore()
    }
  })
})
