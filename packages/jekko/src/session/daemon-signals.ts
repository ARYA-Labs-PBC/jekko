import type { ZyalScript, ZyalSignal } from "@/agent-script/schema"

export function daemonNoHumanPrompts(spec: Pick<ZyalScript, "interaction">) {
  return spec.interaction?.user !== "present"
}

export function suppressDaemonSignal(input: {
  readonly spec: Pick<ZyalScript, "interaction">
  readonly signal: ZyalSignal | undefined
}) {
  return daemonNoHumanPrompts(input.spec) && input.signal === "permission_denied"
}

export function effectiveDaemonSignal(input: {
  readonly spec: Pick<ZyalScript, "interaction">
  readonly signal: ZyalSignal | undefined
}) {
  return suppressDaemonSignal(input) ? undefined : input.signal
}

export function daemonSignalCountsAsError(input: {
  readonly spec: Pick<ZyalScript, "interaction">
  readonly signal: ZyalSignal | undefined
}) {
  if (suppressDaemonSignal(input)) return false
  return input.signal === "permission_denied" || input.signal === "error" || input.signal === "cancelled"
}
