import type { ZyalIncubatorPromotion } from "@/agent-script/schema"
import type { DaemonStore } from "./daemon-store"

export type PromotionDecision = {
  promote: boolean
  exhausted: boolean
  score: number
  missing: string[]
  blockers: string[]
}

const MEMORY_REQUIREMENTS: Record<string, string[]> = {
  problem_statement: ["problem_statement", "scout"],
  current_best_plan: ["current_best_plan", "synthesis", "strengthen"],
  verification_strategy: ["verification_strategy", "promotion_review"],
  risk_review: ["risk_review", "critic", "critical_objection"],
}

export function evaluatePromotion(input: {
  task: DaemonStore.TaskInfo
  memories: DaemonStore.TaskMemoryInfo[]
  passes: DaemonStore.TaskPassInfo[]
  promotion: ZyalIncubatorPromotion
  maxPasses: number
}) {
  const missing = missingRequirements(input.promotion.require ?? [], input.memories)
  const critical = unresolvedCriticalObjections(input.memories)
  const blockers: string[] = []
  const blockLimit = input.promotion.block_on?.unresolved_critical_objections_gte
  if (blockLimit !== undefined && critical >= blockLimit) blockers.push("unresolved_critical_objections")
  if (missing.length > 0) blockers.push("missing_required_evidence")
  const score = input.task.readiness_score
  const exhausted = input.passes.length >= input.maxPasses && !input.passes.some((pass) => pass.status === "running")
  return {
    promote: score >= input.promotion.promote_at && blockers.length === 0,
    exhausted,
    score,
    missing,
    blockers,
  } satisfies PromotionDecision
}

export function missingRequirements(requirements: readonly string[], memories: DaemonStore.TaskMemoryInfo[]) {
  return requirements.filter((requirement) => {
    const accepted = MEMORY_REQUIREMENTS[requirement] ?? [requirement]
    return !memories.some((item) => accepted.includes(item.kind))
  })
}

export function unresolvedCriticalObjections(memories: DaemonStore.TaskMemoryInfo[]) {
  return memories.filter((item) => {
    if (!["critical_objection", "critic"].includes(item.kind)) return false
    const payload = item.payload_json as Record<string, unknown> | null
    if (payload?.resolved === true) return false
    if (payload?.severity === "low") return false
    return true
  }).length
}

export * as DaemonTaskPromote from "./daemon-task-promote"
