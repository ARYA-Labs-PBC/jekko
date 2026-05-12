import type { ZyalRetryPolicy, ZyalRetry } from "@/agent-script/schema"

/**
 * Retry policy engine.
 * Computes delay for a given attempt number using the configured backoff strategy.
 */

export type RetryCategory = "shell_checks" | "checkpoint" | "worker_spawn" | "stop_evaluation"

/**
 * Resolve the retry policy for a given category.
 * Override takes precedence, then default, then hardcoded base policy.
 */
export function resolveRetryPolicy(
  retry: ZyalRetry | undefined,
  category: RetryCategory,
): Required<ZyalRetryPolicy> {
  const basePolicy: Required<ZyalRetryPolicy> = {
    max_attempts: 1,
    backoff: "none",
    initial_delay: "0s",
    max_delay: "60s",
    jitter: false,
  }

  if (!retry) return basePolicy

  const override = retry.overrides?.[category]
  const base = retry.default

  return {
    max_attempts: override?.max_attempts ?? base?.max_attempts ?? basePolicy.max_attempts,
    backoff: override?.backoff ?? base?.backoff ?? basePolicy.backoff,
    initial_delay: override?.initial_delay ?? base?.initial_delay ?? basePolicy.initial_delay,
    max_delay: override?.max_delay ?? base?.max_delay ?? basePolicy.max_delay,
    jitter: override?.jitter ?? base?.jitter ?? basePolicy.jitter,
  }
}

/**
 * Compute the delay in milliseconds for a given attempt number.
 * attempt is 0-indexed (0 = first retry after initial failure).
 */
export function computeRetryDelay(policy: Required<ZyalRetryPolicy>, attempt: number): number {
  const initialMs = parseDuration(policy.initial_delay)
  const maxMs = parseDuration(policy.max_delay)

  let delay: number
  switch (policy.backoff) {
    case "none":
      delay = initialMs
      break
    case "linear":
      delay = initialMs * (attempt + 1)
      break
    case "exponential":
      delay = initialMs * Math.pow(2, attempt)
      break
    default:
      delay = initialMs
  }

  delay = Math.min(delay, maxMs)

  if (policy.jitter) {
    delay = delay * (0.5 + Math.random() * 0.5)
  }

  return Math.round(delay)
}

/**
 * Check if another retry attempt is allowed.
 */
export function canRetry(policy: Required<ZyalRetryPolicy>, currentAttempt: number): boolean {
  return currentAttempt < policy.max_attempts
}

/**
 * Parse a duration string like "2s", "500ms", "1m" into milliseconds.
 */
export function parseDuration(duration: string): number {
  const match = duration.match(/^(\d+(?:\.\d+)?)\s*(ms|s|m|h)?$/)
  if (!match) return 0
  const value = parseFloat(match[1])
  switch (match[2]) {
    case "ms":
      return value
    case "s":
    case undefined:
      return value * 1000
    case "m":
      return value * 60_000
    case "h":
      return value * 3_600_000
    default:
      return value * 1000
  }
}
