import { describe, expect, test } from "bun:test"
import {
  daemonSignalCountsAsError,
  effectiveDaemonSignal,
  suppressDaemonSignal,
} from "@/session/daemon-signals"
import { resolveDaemonAutomaticAction, shouldRestartDaemonFiber } from "@/session/daemon-lifetime"
import type { ZyalScript } from "@/agent-script/schema"

const unattended = { interaction: { user: "none" } } as Pick<ZyalScript, "interaction">
const implicitUnattended = {} as Pick<ZyalScript, "interaction">
const interactive = { interaction: { user: "present" } } as Pick<ZyalScript, "interaction">
const forever = { confirm: "RUN_FOREVER", loop: { policy: "forever" } } as Pick<ZyalScript, "confirm" | "loop">
const implicitForever = { confirm: "RUN_FOREVER" } as Pick<ZyalScript, "confirm" | "loop">
const bounded = { confirm: "RUN_FOREVER", loop: { policy: "bounded" } } as Pick<ZyalScript, "confirm" | "loop">

describe("daemon signals", () => {
  test("suppresses unattended permission_denied as a loop-control signal", () => {
    expect(suppressDaemonSignal({ spec: unattended, signal: "permission_denied" })).toBe(true)
    expect(effectiveDaemonSignal({ spec: unattended, signal: "permission_denied" })).toBeUndefined()
    expect(daemonSignalCountsAsError({ spec: unattended, signal: "permission_denied" })).toBe(false)
  })

  test("defaults daemon specs without interaction to no-human mode", () => {
    expect(suppressDaemonSignal({ spec: implicitUnattended, signal: "permission_denied" })).toBe(true)
  })

  test("keeps permission_denied visible for explicitly interactive specs", () => {
    expect(suppressDaemonSignal({ spec: interactive, signal: "permission_denied" })).toBe(false)
    expect(effectiveDaemonSignal({ spec: interactive, signal: "permission_denied" })).toBe("permission_denied")
    expect(daemonSignalCountsAsError({ spec: interactive, signal: "permission_denied" })).toBe(true)
  })

  test("suppresses automatic terminal actions for forever jobs", () => {
    expect(resolveDaemonAutomaticAction({ spec: forever, action: "pause" })).toEqual({
      action: "continue",
      suppressedByForever: true,
    })
    expect(resolveDaemonAutomaticAction({ spec: implicitForever, action: "abort" })).toEqual({
      action: "continue",
      suppressedByForever: true,
    })
    expect(resolveDaemonAutomaticAction({ spec: forever, action: "continue" })).toEqual({
      action: "continue",
      suppressedByForever: false,
    })
  })

  test("restarts forever fibers unless the operator paused or aborted", () => {
    expect(shouldRestartDaemonFiber({ spec: forever, status: "running" })).toBe(true)
    expect(shouldRestartDaemonFiber({ spec: forever, status: "failed" })).toBe(true)
    expect(shouldRestartDaemonFiber({ spec: forever, status: "satisfied" })).toBe(true)
    expect(shouldRestartDaemonFiber({ spec: forever, status: "paused" })).toBe(false)
    expect(shouldRestartDaemonFiber({ spec: forever, status: "aborted" })).toBe(false)
    expect(shouldRestartDaemonFiber({ spec: bounded, status: "running" })).toBe(false)
  })
})
