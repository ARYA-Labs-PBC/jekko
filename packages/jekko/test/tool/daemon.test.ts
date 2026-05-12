import { describe, expect, test } from "bun:test"
import { DaemonTools } from "../../src/tool/daemon"

describe("daemon tools", () => {
  test("stay in the daemon namespace", () => {
    expect(DaemonTools.every((tool) => tool.id.startsWith("daemon_"))).toBe(true)
  })
})
