export type CapabilityDecision = "pass" | "fail" | "advisory" | "unknown"

export type CapabilityState = {
  score: number
  conformanceClaimed: string
  conformanceObserved: string
  decision: CapabilityDecision
  hardFindings: number
  softFindings: number
  capsApplied: number
  blockers: string[]
  hardRules: { id: string; max_score: number }[]
  generatedAt: number | undefined
  standard: string
  standardVersion: string
  loaded: boolean
  error: string | undefined
}

export type CapabilityParseResult =
  | { ok: true; state: CapabilityState }
  | { ok: false; message: string; repairHint: string }

export const EMPTY: CapabilityState = {
  score: 0,
  conformanceClaimed: "",
  conformanceObserved: "",
  decision: "unknown",
  hardFindings: 0,
  softFindings: 0,
  capsApplied: 0,
  blockers: [],
  hardRules: [],
  generatedAt: undefined,
  standard: "",
  standardVersion: "",
  loaded: false,
  error: undefined,
}
