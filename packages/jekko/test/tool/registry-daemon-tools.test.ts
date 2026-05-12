import { describe, expect, test } from "bun:test"
import { DaemonTools } from "../../src/tool/daemon"

describe("daemon tool registry contract", () => {
  test("declares daemon tools as explicit opt-in tools", () => {
    expect(DaemonTools.map((tool) => tool.id)).toEqual([
      "daemon_report_confidence",
      "daemon_emit_idea",
      "daemon_report_pass_result",
      "daemon_memory_write",
      "daemon_memory_read",
      "daemon_request_incubation",
      "daemon_request_promotion",
      "daemon_context_snapshot",
    ])
  })
})

