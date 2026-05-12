import { describe, expect, test } from "bun:test"
import {
  evaluateOnHandlers,
  incrementSignalCounter,
  type SignalCounters,
} from "@/session/daemon-on-handler"
import type { ZyalOnHandler } from "@/agent-script/schema"

describe("daemon on-handler", () => {
  const handler = (overrides: Partial<ZyalOnHandler> = {}): ZyalOnHandler => ({
    signal: "no_progress",
    do: [{ pause: true as const }],
    ...overrides,
  })

  test("fires on matching signal", () => {
    const actions = evaluateOnHandlers({
      handlers: [handler()],
      signal: "no_progress",
      counters: { no_progress: 1 },
    })
    expect(actions).toHaveLength(1)
    expect(actions[0].type).toBe("pause")
  })

  test("does not fire on non-matching signal", () => {
    const actions = evaluateOnHandlers({
      handlers: [handler()],
      signal: "error",
      counters: { error: 1 },
    })
    expect(actions).toHaveLength(0)
  })

  test("respects count_gte threshold", () => {
    const h = handler({ count_gte: 3 })
    const below = evaluateOnHandlers({
      handlers: [h],
      signal: "no_progress",
      counters: { no_progress: 2 },
    })
    expect(below).toHaveLength(0)

    const at = evaluateOnHandlers({
      handlers: [h],
      signal: "no_progress",
      counters: { no_progress: 3 },
    })
    expect(at).toHaveLength(1)
  })

  test("respects message_contains filter", () => {
    const h = handler({
      signal: "error",
      message_contains: "ENOMEM",
      do: [{ abort: true as const }],
    })

    const noMatch = evaluateOnHandlers({
      handlers: [h],
      signal: "error",
      counters: { error: 1 },
      message: "some other error",
    })
    expect(noMatch).toHaveLength(0)

    const match = evaluateOnHandlers({
      handlers: [h],
      signal: "error",
      counters: { error: 1 },
      message: "fatal: ENOMEM in allocator",
    })
    expect(match).toHaveLength(1)
    expect(match[0].type).toBe("abort")
  })

  test("executes multiple actions from one handler", () => {
    const h = handler({
      do: [
        { switch_agent: "plan" },
        { notify: "switched to plan" },
      ],
    })
    const actions = evaluateOnHandlers({
      handlers: [h],
      signal: "no_progress",
      counters: { no_progress: 1 },
    })
    expect(actions).toHaveLength(2)
    expect(actions[0].type).toBe("switch_agent")
    expect(actions[1].type).toBe("notify")
  })

  test("incrementSignalCounter accumulates correctly", () => {
    let counters: SignalCounters = {}
    counters = incrementSignalCounter(counters, "error")
    counters = incrementSignalCounter(counters, "error")
    counters = incrementSignalCounter(counters, "no_progress")
    expect(counters.error).toBe(2)
    expect(counters.no_progress).toBe(1)
  })

  test("shell guard can prevent handler from firing", () => {
    const h = handler({
      if: { command: "git diff --stat --exit-code" },
    })
    const blocked = evaluateOnHandlers({
      handlers: [h],
      signal: "no_progress",
      counters: { no_progress: 1 },
      shellGuardFn: () => false,
    })
    expect(blocked).toHaveLength(0)

    const allowed = evaluateOnHandlers({
      handlers: [h],
      signal: "no_progress",
      counters: { no_progress: 1 },
      shellGuardFn: () => true,
    })
    expect(allowed).toHaveLength(1)
  })
})
