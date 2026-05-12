import {
  GPT5_FAMILY_RE,
  OPENAI_NONE_EFFORT_RELEASE_DATE,
  OPENAI_XHIGH_EFFORT_RELEASE_DATE,
  WIDELY_SUPPORTED_EFFORTS,
} from "./transform-shared"

export function openaiReasoningEfforts(apiId: string, releaseDate: string): string[] {
  const id = apiId.toLowerCase()
  if (id === "gpt-5-pro" || id === "openai/gpt-5-pro") return []
  if (id.includes("codex")) {
    if (id.includes("5.2") || id.includes("5.3")) return [...WIDELY_SUPPORTED_EFFORTS, "xhigh"]
    return [...WIDELY_SUPPORTED_EFFORTS]
  }
  const efforts = [...WIDELY_SUPPORTED_EFFORTS]
  if (GPT5_FAMILY_RE.test(id)) efforts.unshift("minimal")
  if (releaseDate >= OPENAI_NONE_EFFORT_RELEASE_DATE) efforts.unshift("none")
  if (releaseDate >= OPENAI_XHIGH_EFFORT_RELEASE_DATE) efforts.push("xhigh")
  return efforts
}

export function anthropicAdaptiveEfforts(apiId: string): string[] {
  if (["opus-4-7", "opus-4.7"].some((v) => apiId.includes(v))) {
    return ["low", "medium", "high", "xhigh", "max"]
  }
  if (["opus-4-6", "opus-4.6", "sonnet-4-6", "sonnet-4.6"].some((v) => apiId.includes(v))) {
    return ["low", "medium", "high", "max"]
  }
  return []
}

export function oaiEncryptedEfforts(efforts: string[]) {
  return Object.fromEntries(
    efforts.map((effort) => [effort, { reasoningEffort: effort, reasoningSummary: "auto", include: ["reasoning.encrypted_content"] }]),
  )
}

export function googleThinkingConfig() {
  return {
    high: { thinkingConfig: { includeThoughts: true, thinkingBudget: 16000 } },
    max: { thinkingConfig: { includeThoughts: true, thinkingBudget: 24576 } },
  }
}

export function anthropicSimpleAdaptive(efforts: string[]) {
  return Object.fromEntries(efforts.map((effort) => [effort, { thinking: { type: "adaptive" }, effort }]))
}

export function anthropicStaticBudgets() {
  return {
    high: { thinking: { type: "enabled", budgetTokens: 16000 } },
    max: { thinking: { type: "enabled", budgetTokens: 31999 } },
  }
}
