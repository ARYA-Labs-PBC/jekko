import { describe, expect, test } from "bun:test"
import { formatDaemonBanner } from "../../../src/cli/cmd/tui/routes/session/daemon-banner"

describe("daemon banner", () => {
  test("renders task and readiness summary text", () => {
    const text = formatDaemonBanner({
      id: "run_1",
      iteration: 4,
      phase: "incubator_pass",
      spec_json: { job: { name: "Hard Task" } },
      active_task: { title: "Fix daemon", readiness_score: 0.83 },
      active_pass: { pass_type: "synthesize" },
    })
    expect(text).toContain("∞ FOREVER")
    expect(text).toContain("Fix daemon")
    expect(text).toContain("ready 0.83")
  })
})
