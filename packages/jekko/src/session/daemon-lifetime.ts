import type { ZyalScript } from "@/agent-script/schema"

export type DaemonAutomaticAction = "pause" | "abort" | "continue"
export type DaemonPolicy = "forever" | "bounded" | "once" | "finite"

export function resolveDaemonPolicy(spec: Pick<ZyalScript, "loop" | "confirm">): DaemonPolicy {
  if (spec.loop?.policy) return spec.loop.policy
  return spec.confirm === "RUN_FOREVER" ? "forever" : "finite"
}

export function isRunForeverPolicy(spec: Pick<ZyalScript, "loop" | "confirm">): boolean {
  return resolveDaemonPolicy(spec) === "forever"
}

export function resolveDaemonAutomaticAction(input: {
  spec: Pick<ZyalScript, "loop" | "confirm">
  action: DaemonAutomaticAction | undefined
}): { action: DaemonAutomaticAction; suppressedByForever: boolean } {
  const action = input.action ?? "pause"
  const policy = resolveDaemonPolicy(input.spec)
  if (policy === "forever" && action !== "continue") {
    return { action: "continue", suppressedByForever: true }
  }
  return { action, suppressedByForever: false }
}

export function shouldRestartDaemonFiber(input: {
  spec: Pick<ZyalScript, "loop" | "confirm">
  status: string | null | undefined
}): boolean {
  if (resolveDaemonPolicy(input.spec) !== "forever") return false
  if (!input.status) return false
  return input.status !== "paused" && input.status !== "aborted"
}
