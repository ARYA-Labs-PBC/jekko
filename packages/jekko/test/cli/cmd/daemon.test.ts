import { describe, expect, test } from "bun:test"
import { formatDaemonRunList, formatDaemonRunSummary, formatDaemonTaskList } from "../../../src/cli/cmd/daemon"

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
})
