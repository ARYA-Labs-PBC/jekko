// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import type { ZyalAssertions } from "@/agent-script/schema"

/**
 * Structured output assertion validator.
 * Validates model output against a JSON schema contract.
 */

export type AssertionVerdict =
  | { valid: true; parsed: Record<string, unknown> }
  | { valid: false; reason: string; action: string }

/**
 * Validate a model output string against the assertions spec.
 * If no assertions are configured, always passes.
 */
export function validateAssertions(
  assertions: ZyalAssertions | undefined,
  output: string,
): AssertionVerdict {
  if (!assertions?.require_structured_output) {
    return { valid: true, parsed: {} }
  }

  // Try to extract JSON from the output
  const extracted = extractJson(output)
  if (!extracted) {
    return {
      valid: false,
      reason: "Output does not contain valid JSON",
      action: assertions.on_invalid ?? "warn",
    }
  }

  // If a schema is defined, validate required fields
  if (assertions.schema) {
    const schemaValidation = validateAgainstSchema(extracted, assertions.schema)
    if (!schemaValidation.valid) {
      return {
        valid: false,
        reason: schemaValidation.reason,
        action: assertions.on_invalid ?? "warn",
      }
    }
  }

  return { valid: true, parsed: extracted }
}

/**
 * Check whether another assertion retry is allowed.
 */
export function canRetryAssertion(assertions: ZyalAssertions | undefined, currentAttempt: number): boolean {
  if (!assertions) return false
  const maxRetries = assertions.max_retries ?? 0
  return currentAttempt < maxRetries
}

/**
 * Try to extract a JSON object from a text string.
 * Handles JSON wrapped in code blocks or embedded in prose.
 */
function extractJson(text: string): Record<string, unknown> | null {
  // First try direct parse
  try {
    const parsed = JSON.parse(text.trim())
    if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
      return parsed
    }
  } catch {
    // continue
  }

  // Try extracting from code blocks
  const codeBlockMatch = text.match(/```(?:json)?\s*\n([\s\S]*?)\n\s*```/)
  if (codeBlockMatch) {
    try {
      const parsed = JSON.parse(codeBlockMatch[1].trim())
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        return parsed
      }
    } catch {
      // continue
    }
  }

  // Try finding first {...} in the text
  const braceMatch = text.match(/\{[\s\S]*\}/)
  if (braceMatch) {
    try {
      const parsed = JSON.parse(braceMatch[0])
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        return parsed
      }
    } catch {
      // continue
    }
  }

  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return null
}

/**
 * Basic schema validation: checks required fields and type constraints.
 * This is a lightweight validator — not a full JSON Schema implementation.
 */
function validateAgainstSchema(
  data: Record<string, unknown>,
  schema: Record<string, unknown>,
): { valid: true } | { valid: false; reason: string } {
  // Check required fields
  const required = schema.required as string[] | undefined
  if (required && Array.isArray(required)) {
    for (const field of required) {
      if (!(field in data) || data[field] === undefined || data[field] === null) {
        return { valid: false, reason: `Missing required field: ${field}` }
      }
    }
  }

  // Check property types
  const properties = schema.properties as Record<string, Record<string, unknown>> | undefined
  if (properties) {
    for (const [key, propSchema] of Object.entries(properties)) {
      if (!(key in data)) continue
      const value = data[key]
      const expectedType = propSchema.type as string | undefined

      if (expectedType) {
        const typeValid = checkType(value, expectedType)
        if (!typeValid) {
          return { valid: false, reason: `Field '${key}' expected type '${expectedType}', got '${typeof value}'` }
        }
      }

      // Check numeric bounds
      if (expectedType === "number" && typeof value === "number") {
        const min = propSchema.minimum as number | undefined
        const max = propSchema.maximum as number | undefined
        if (min !== undefined && value < min) {
          return { valid: false, reason: `Field '${key}' value ${value} below minimum ${min}` }
        }
        if (max !== undefined && value > max) {
          return { valid: false, reason: `Field '${key}' value ${value} above maximum ${max}` }
        }
      }
    }
  }

  return { valid: true }
}

function checkType(value: unknown, expectedType: string): boolean {
  switch (expectedType) {
    case "string":
      return typeof value === "string"
    case "number":
    case "integer":
      return typeof value === "number"
    case "boolean":
      return typeof value === "boolean"
    case "array":
      return Array.isArray(value)
    case "object":
      return typeof value === "object" && value !== null && !Array.isArray(value)
    default:
      return true
  }
}
