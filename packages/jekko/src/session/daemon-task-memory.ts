import type { DaemonStore } from "./daemon-store"
import type { ConceptRecord } from "./daemon-concept-memory"

export type TaskCapsule = {
  taskID: string
  title: string
  lane: string
  status: string
  readinessScore: number
  riskScore: number
  currentBest?: string
  objections: string[]
  memories: string[]
}

export type ConceptScopeLine = string

/**
 * Render `<concepts-in-scope>...</concepts-in-scope>` from a concept set.
 * The block is injected only when the caller has actual concepts; an empty
 * input returns the empty string so context packets don't grow noise.
 */
export function renderConceptsInScope(concepts: readonly ConceptRecord[]): string {
  if (concepts.length === 0) return ""
  const lines: ConceptScopeLine[] = ["<concepts-in-scope>"]
  for (const concept of concepts) {
    if (concept.invalidatedAt !== undefined) continue
    const parents = concept.derivedFrom.length > 0 ? ` derived_from=[${concept.derivedFrom.join(",")}]` : ""
    const confidence = ` conf=${concept.confidence.toFixed(2)}`
    lines.push(`- ${concept.conceptID}${confidence}${parents}: ${concept.definition}`)
  }
  lines.push("</concepts-in-scope>")
  return lines.join("\n")
}

export function buildTaskCapsule(input: {
  task: DaemonStore.TaskInfo
  memories: DaemonStore.TaskMemoryInfo[]
  passes?: DaemonStore.TaskPassInfo[]
}): TaskCapsule {
  const currentBest = input.memories.find((item) => item.kind === "current_best_plan" || item.kind === "synthesis")
  const objections = input.memories.filter((item) => item.kind === "critic" || item.kind === "critical_objection")
  return {
    taskID: input.task.id,
    title: input.task.title,
    lane: input.task.lane,
    status: input.task.status,
    readinessScore: input.task.readiness_score,
    riskScore: input.task.risk_score,
    currentBest: currentBest ? `${currentBest.title}: ${currentBest.summary}` : undefined,
    objections: objections.map((item) => `${item.title}: ${item.summary}`),
    memories: input.memories.map((item) => `${item.kind} | ${item.title}: ${item.summary}`),
  }
}

export function buildContextPacket(input: {
  task: DaemonStore.TaskInfo
  memories: DaemonStore.TaskMemoryInfo[]
  passes: DaemonStore.TaskPassInfo[]
  mode: string
  passType?: string
  /** Concept records already loaded by the caller. Injected verbatim as a
   *  `<concepts-in-scope>` block when present. */
  concepts?: readonly ConceptRecord[]
}) {
  const capsule = buildTaskCapsule(input)
  const objective = taskObjective(input.task)
  const conceptsBlock = renderConceptsInScope(input.concepts ?? [])
  const header = [
    "<zyal-incubator-task>",
    `Task: ${input.task.id}`,
    `Title: ${input.task.title}`,
    `Lane: ${input.task.lane}`,
    `Status: ${input.task.status}`,
    `Pass: ${input.passType ?? "(next)"}`,
    `Context mode: ${input.mode}`,
    `Readiness: ${input.task.readiness_score.toFixed(3)}`,
    `Risk: ${input.task.risk_score.toFixed(3)}`,
    "</zyal-incubator-task>",
  ]
  const conceptTail = conceptsBlock ? ["", conceptsBlock] : []

  if (input.mode === "blind") {
    return [...header, ...conceptTail, "", "Objective:", objective, "", safetyFooter()].join("\n")
  }

  if (input.mode === "ledger_only") {
    return [
      ...header,
      ...conceptTail,
      "",
      "Compressed task memory:",
      capsule.memories.slice(0, 12).join("\n") || "(none)",
      "",
      safetyFooter(),
    ].join("\n")
  }

  if (input.mode === "promotion") {
    return [
      ...header,
      ...conceptTail,
      "",
      "Objective:",
      objective,
      "",
      "Current best:",
      capsule.currentBest ?? "(none)",
      "",
      "Objections:",
      capsule.objections.join("\n") || "(none)",
      "",
      "Required evidence: problem_statement, current_best_plan, verification_strategy, risk_review.",
      safetyFooter(),
    ].join("\n")
  }

  if (input.mode === "pool") {
    return [
      ...header,
      ...conceptTail,
      "",
      "Candidate and critique summaries:",
      capsule.memories.filter((item) => /idea|plan|critic|objection|synthesis/.test(item)).join("\n") || "(none)",
      "",
      "Pass receipts:",
      input.passes.map((pass) => `- ${pass.pass_number} ${pass.pass_type}: ${pass.status}`).join("\n") || "(none)",
      "",
      safetyFooter(),
    ].join("\n")
  }

  return [
    ...header,
    ...conceptTail,
    "",
    "Objective:",
    objective,
    "",
    "Current capsule:",
    capsule.memories.slice(0, 16).join("\n") || "(none)",
    "",
    "Current best:",
    capsule.currentBest ?? "(none)",
    "",
    "Open objections:",
    capsule.objections.join("\n") || "(none)",
    "",
    safetyFooter(),
  ].join("\n")
}

function taskObjective(task: DaemonStore.TaskInfo) {
  const body = task.body_json as Record<string, unknown>
  for (const key of ["objective", "goal", "summary", "description"]) {
    const value = body?.[key]
    if (typeof value === "string" && value.trim()) return value.trim()
  }
  return task.title
}

function safetyFooter() {
  return [
    "Do not include hidden chain-of-thought.",
    "Return concise structured evidence, claims, uncertainty, blockers, and recommended next action.",
  ].join("\n")
}

export * as DaemonTaskMemory from "./daemon-task-memory"
