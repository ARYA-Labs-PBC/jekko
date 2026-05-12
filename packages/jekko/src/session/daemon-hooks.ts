import type { ZyalHooks, ZyalHookStep } from "@/agent-script/schema"

/**
 * Lifecycle hook runner.
 * Extracts hooks for each lifecycle phase and provides execution helpers.
 */

export type HookPhase =
  | "on_start"
  | "before_iteration"
  | "after_iteration"
  | "before_checkpoint"
  | "after_checkpoint"
  | "on_promote"
  | "on_exhaust"
  | "on_stop"

export type HookResult =
  | { ok: true }
  | { ok: false; step: ZyalHookStep; error: string }

export type HookAction = "pause" | "abort" | "warn" | "block_promotion" | "continue"

/**
 * Get the hook steps for a specific lifecycle phase.
 */
export function getHookSteps(hooks: ZyalHooks | undefined, phase: HookPhase): readonly ZyalHookStep[] {
  if (!hooks) return []
  return hooks[phase] ?? []
}

/**
 * Determine the action to take when a hook step fails.
 */
export function resolveHookFailureAction(step: ZyalHookStep): HookAction {
  return step.on_fail ?? "continue"
}

/**
 * Count total hooks across all phases.
 */
export function countHooks(hooks: ZyalHooks | undefined): number {
  if (!hooks) return 0
  return Object.values(hooks).reduce((sum, arr) => sum + (arr?.length ?? 0), 0)
}

/**
 * List all phases that have at least one hook defined.
 */
export function activeHookPhases(hooks: ZyalHooks | undefined): HookPhase[] {
  if (!hooks) return []
  const phases: HookPhase[] = []
  const allPhases: HookPhase[] = [
    "on_start", "before_iteration", "after_iteration",
    "before_checkpoint", "after_checkpoint",
    "on_promote", "on_exhaust", "on_stop",
  ]
  for (const phase of allPhases) {
    if (hooks[phase] && hooks[phase]!.length > 0) {
      phases.push(phase)
    }
  }
  return phases
}
