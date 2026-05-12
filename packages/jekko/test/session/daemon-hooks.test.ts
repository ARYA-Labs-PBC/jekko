import { describe, expect, test } from "bun:test"
import {
  getHookSteps,
  resolveHookFailureAction,
  countHooks,
  activeHookPhases,
} from "@/session/daemon-hooks"
import type { ZyalHooks } from "@/agent-script/schema"

describe("daemon hooks", () => {
  const hooks: ZyalHooks = {
    on_start: [
      { run: "git fetch origin main" },
      { run: "bun install" },
    ],
    before_iteration: [
      { run: "git rebase origin/main --autostash", on_fail: "pause" },
    ],
    after_checkpoint: [
      { run: "echo done", on_fail: "warn" },
    ],
  }

  test("getHookSteps returns steps for existing phase", () => {
    const steps = getHookSteps(hooks, "on_start")
    expect(steps).toHaveLength(2)
    expect(steps[0].run).toBe("git fetch origin main")
  })

  test("getHookSteps returns empty for missing phase", () => {
    const steps = getHookSteps(hooks, "on_stop")
    expect(steps).toHaveLength(0)
  })

  test("getHookSteps returns empty for undefined hooks", () => {
    expect(getHookSteps(undefined, "on_start")).toHaveLength(0)
  })

  test("resolveHookFailureAction returns configured action", () => {
    expect(resolveHookFailureAction({ run: "test", on_fail: "pause" })).toBe("pause")
    expect(resolveHookFailureAction({ run: "test", on_fail: "abort" })).toBe("abort")
    expect(resolveHookFailureAction({ run: "test", on_fail: "warn" })).toBe("warn")
  })

  test("resolveHookFailureAction defaults to continue", () => {
    expect(resolveHookFailureAction({ run: "test" })).toBe("continue")
  })

  test("countHooks counts total across all phases", () => {
    expect(countHooks(hooks)).toBe(4)
    expect(countHooks(undefined)).toBe(0)
  })

  test("activeHookPhases lists only phases with hooks", () => {
    const phases = activeHookPhases(hooks)
    expect(phases).toEqual(["on_start", "before_iteration", "after_checkpoint"])
  })

  test("activeHookPhases returns empty for undefined", () => {
    expect(activeHookPhases(undefined)).toEqual([])
  })
})
