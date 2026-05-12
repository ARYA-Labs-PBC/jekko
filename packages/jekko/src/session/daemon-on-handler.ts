import type { ZyalOnHandler, ZyalScript, ZyalSignal } from "@/agent-script/schema"

/**
 * Evaluates `on` handlers against a signal event.
 * Returns the list of actions to execute, respecting count_gte thresholds
 * and optional shell guards.
 */

export type SignalCounters = Record<string, number>

export type OnAction =
  | { type: "switch_agent"; agent: string }
  | { type: "run"; command: string }
  | { type: "incubate_current_task" }
  | { type: "checkpoint" }
  | { type: "pause" }
  | { type: "abort" }
  | { type: "notify"; message: string }
  | { type: "set_context"; values: Record<string, unknown> }

export function incrementSignalCounter(counters: SignalCounters, signal: string): SignalCounters {
  return { ...counters, [signal]: (counters[signal] ?? 0) + 1 }
}

export function evaluateOnHandlers(input: {
  handlers: readonly ZyalOnHandler[]
  signal: ZyalSignal
  counters: SignalCounters
  message?: string
  shellGuardFn?: (check: ZyalOnHandler["if"]) => boolean
}): OnAction[] {
  const { handlers, signal, counters, message, shellGuardFn } = input
  const count = counters[signal] ?? 0
  const actions: OnAction[] = []

  for (const handler of handlers) {
    if (handler.signal !== signal) continue
    if (handler.count_gte !== undefined && count < handler.count_gte) continue
    if (handler.message_contains && (!message || !message.includes(handler.message_contains))) continue
    if (handler.if && shellGuardFn && !shellGuardFn(handler.if)) continue

    for (const action of handler.do) {
      if ("switch_agent" in action) actions.push({ type: "switch_agent", agent: action.switch_agent })
      else if ("run" in action) actions.push({ type: "run", command: action.run })
      else if ("incubate_current_task" in action) actions.push({ type: "incubate_current_task" })
      else if ("checkpoint" in action) actions.push({ type: "checkpoint" })
      else if ("pause" in action) actions.push({ type: "pause" })
      else if ("abort" in action) actions.push({ type: "abort" })
      else if ("notify" in action) actions.push({ type: "notify", message: action.notify })
      else if ("set_context" in action) actions.push({ type: "set_context", values: action.set_context })
    }
  }

  return actions
}

/** Extract all on handlers from a ZYAL spec, returns empty array if none */
export function getOnHandlers(spec: ZyalScript): readonly ZyalOnHandler[] {
  return spec.on ?? []
}
