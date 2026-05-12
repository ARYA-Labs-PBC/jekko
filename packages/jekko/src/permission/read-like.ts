import { evaluate } from "./evaluate"

type Rule = {
  permission: string
  pattern: string
  action: "allow" | "deny" | "ask"
}

export type ExternalDirectoryAccess = "read" | "write" | "unknown"

export const READ_LIKE_PERMISSIONS = new Set(["read", "list", "glob", "grep"])
export const AUTO_ALLOW_READS_PERMISSION = "zyal_auto_allow_reads"
export const NO_HUMAN_PROMPTS_PERMISSION = "zyal_unattended"
export const NO_HUMAN_PROMPTS_PATTERN = "no_human_prompts"

export const UNATTENDED_READ_AUTO_ALLOW_RULES = [
  { permission: AUTO_ALLOW_READS_PERMISSION, pattern: "*", action: "allow" as const },
  { permission: NO_HUMAN_PROMPTS_PERMISSION, pattern: NO_HUMAN_PROMPTS_PATTERN, action: "allow" as const },
]

export function isReadLikeRequest(input: {
  permission: string
  metadata?: Record<string, unknown>
}) {
  if (READ_LIKE_PERMISSIONS.has(input.permission)) return true
  if (input.permission !== "external_directory") return false
  return input.metadata?.["access"] === "read"
}

export function envAutoAllowReads() {
  return process.env.JEKKO_AUTO_ALLOW_READS === "1" || process.env.ZYAL_RUN === "1"
}

export function envNoHumanPrompts() {
  return process.env.ZYAL_RUN === "1" || process.env.JEKKO_NO_HUMAN_PROMPTS === "1"
}

export function rulesAutoAllowReads(...rulesets: Rule[][]) {
  return evaluate(AUTO_ALLOW_READS_PERMISSION, "enabled", ...rulesets).action === "allow"
}

export function rulesNoHumanPrompts(...rulesets: Rule[][]) {
  return evaluate(NO_HUMAN_PROMPTS_PERMISSION, NO_HUMAN_PROMPTS_PATTERN, ...rulesets).action === "allow"
}

export type AskFallbackDecision = "allow" | "deny" | "ask"

export function resolveAskFallback(input: {
  readonly request: {
    readonly permission: string
    readonly metadata?: Record<string, unknown>
  }
  readonly autoAllowReads: boolean
  readonly noHumanPrompts: boolean
}): AskFallbackDecision {
  if (input.autoAllowReads && isReadLikeRequest(input.request)) return "allow"
  if (input.noHumanPrompts) return "deny"
  return "ask"
}
