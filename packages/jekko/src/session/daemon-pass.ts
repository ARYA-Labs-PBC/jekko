import type { ZyalIncubatorPass } from "@/agent-script/schema"

export type PassReceipt = {
  summary: string
  claims?: { claim: string; confidence?: number; evidence_refs?: string[] }[]
  evidence?: { kind: string; ref?: string; result?: string }[]
  uncertainty?: { question: string; impact?: string; proposed_probe?: string }[]
  blockers?: string[]
  recommended_next?: string
  readiness_delta?: number
  risk_delta?: number
  kind?: string
  title?: string
}

export function passInstruction(pass: ZyalIncubatorPass) {
  return [
    "<zyal-incubator-pass-instructions>",
    `Pass ID: ${pass.id}`,
    `Pass type: ${pass.type}`,
    `Context: ${pass.context}`,
    `Writes: ${pass.writes}`,
    pass.writes === "isolated_worktree"
      ? "Prototype work must stay in the isolated worktree. Do not merge or write the main worktree."
      : "Write only structured scratch artifacts through daemon tools.",
    "",
    "Return a concise pass receipt with summary, claims, evidence, uncertainty, blockers, and recommended_next.",
    "Do not include private chain-of-thought.",
    "</zyal-incubator-pass-instructions>",
  ].join("\n")
}

export function memoryKindForPass(passType: string, receipt?: PassReceipt) {
  if (receipt?.kind) return receipt.kind
  switch (passType) {
    case "scout":
      return "problem_statement"
    case "idea":
      return "idea"
    case "strengthen":
      return "current_best_plan"
    case "critic":
      return "critic"
    case "synthesize":
      return "synthesis"
    case "prototype":
      return "prototype_evidence"
    case "promotion_review":
      return "risk_review"
    case "compress":
      return "compressed_summary"
    default:
      return "pass_receipt"
  }
}

export function normalizeReceipt(
  value: unknown,
  defaultReceipt: { passType: string; title: string },
): PassReceipt {
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>
    return {
      summary: typeof record.summary === "string" ? record.summary : JSON.stringify(value),
      claims: Array.isArray(record.claims) ? (record.claims as PassReceipt["claims"]) : undefined,
      evidence: Array.isArray(record.evidence) ? (record.evidence as PassReceipt["evidence"]) : undefined,
      uncertainty: Array.isArray(record.uncertainty) ? (record.uncertainty as PassReceipt["uncertainty"]) : undefined,
      blockers: Array.isArray(record.blockers) ? (record.blockers as string[]) : undefined,
      recommended_next: typeof record.recommended_next === "string" ? record.recommended_next : undefined,
      readiness_delta: typeof record.readiness_delta === "number" ? record.readiness_delta : undefined,
      risk_delta: typeof record.risk_delta === "number" ? record.risk_delta : undefined,
      kind: typeof record.kind === "string" ? record.kind : undefined,
      title: typeof record.title === "string" ? record.title : undefined,
    }
  }
  return {
    summary: typeof value === "string" && value.trim() ? value : `${defaultReceipt.passType} pass completed.`,
    title: defaultReceipt.title,
  }
}

export * as DaemonPass from "./daemon-pass"
