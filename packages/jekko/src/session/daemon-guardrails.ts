// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import type { ZyalGuardrails, ZyalPatternGuardrail, ZyalShellGuardrail } from "@/agent-script/schema"

/**
 * Guardrail evaluation engine.
 * Input guardrails: pattern-match against tool inputs to block dangerous commands.
 * Output guardrails: run shell checks or pattern-match against outputs.
 * Iteration guardrails: shell checks between iterations.
 */

export type GuardrailVerdict =
  | { pass: true }
  | { pass: false; name: string; action: string; reason: string }

export function evaluateInputGuardrails(
  guardrails: ZyalGuardrails | undefined,
  text: string,
): GuardrailVerdict {
  if (!guardrails?.input) return { pass: true }

  for (const rail of guardrails.input) {
    const match = matchPatterns(rail.deny_patterns, text)
    if (match) {
      return {
        pass: false,
        name: rail.name,
        action: rail.action,
        reason: `Input matched denied pattern: ${match}`,
      }
    }
  }

  return { pass: true }
}

export function evaluateOutputPatternGuardrails(
  guardrails: ZyalGuardrails | undefined,
  text: string,
  scope: "tool_output" | "file_diff" | "commit_message" = "tool_output",
): GuardrailVerdict {
  if (!guardrails?.output) return { pass: true }

  for (const rail of guardrails.output) {
    if (!isPatternGuardrail(rail)) continue
    if (rail.scope && rail.scope !== scope) continue
    const match = matchPatterns(rail.deny_patterns, text)
    if (match) {
      return {
        pass: false,
        name: rail.name,
        action: rail.action,
        reason: `Output matched denied pattern: ${match}`,
      }
    }
  }

  return { pass: true }
}

/**
 * Get shell guardrails that need to run at iteration boundaries.
 */
export function getIterationShellGuardrails(
  guardrails: ZyalGuardrails | undefined,
): readonly ZyalShellGuardrail[] {
  return guardrails?.iteration ?? []
}

/**
 * Get output shell guardrails that need to run after model output.
 */
export function getOutputShellGuardrails(
  guardrails: ZyalGuardrails | undefined,
): readonly ZyalShellGuardrail[] {
  if (!guardrails?.output) return []
  return guardrails.output.filter((rail): rail is ZyalShellGuardrail => isShellGuardrail(rail))
}

function matchPatterns(patterns: readonly string[], text: string): string | null {
  for (const pattern of patterns) {
    try {
      const regex = new RegExp(pattern, "i")
      if (regex.test(text)) return pattern
    } catch {
      // Treat non-regex patterns as literal substring matches
      if (text.toLowerCase().includes(pattern.toLowerCase())) return pattern
    }
  }
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return null
}

function isPatternGuardrail(rail: ZyalPatternGuardrail | ZyalShellGuardrail): rail is ZyalPatternGuardrail {
  return "deny_patterns" in rail
}

function isShellGuardrail(rail: ZyalPatternGuardrail | ZyalShellGuardrail): rail is ZyalShellGuardrail {
  return "shell" in rail
}
