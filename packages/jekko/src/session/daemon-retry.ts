import type { ZyalRetryPolicy, ZyalRetry } from "@/agent-script/schema"

/**
 * Retry policy engine.
 * Computes delay for a given attempt number using the configured backoff strategy.
 */

export type RetryCategory = "shell_checks" | "agent_calls" | "checkpoint" | "worker_spawn" | "stop_evaluation"

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
    retries: 0,
    backoff: "none",
    initial_delay: "0s",
    max_delay: "60s",
    jitter: false,
    retry_on: [],
    do_not_retry: [],
  }

  if (!retry) return basePolicy

  const override = retry.overrides?.[category]
  const base = retry.default

  const configuredMaxAttempts =
    override?.max_attempts ?? (override?.retries !== undefined ? override.retries + 1 : undefined)
      ?? base?.max_attempts ?? (base?.retries !== undefined ? base.retries + 1 : undefined)
      ?? basePolicy.max_attempts
  return {
    max_attempts: configuredMaxAttempts,
    retries: Math.max(0, configuredMaxAttempts - 1),
    backoff: override?.backoff ?? base?.backoff ?? basePolicy.backoff,
    initial_delay: override?.initial_delay ?? base?.initial_delay ?? basePolicy.initial_delay,
    max_delay: override?.max_delay ?? base?.max_delay ?? basePolicy.max_delay,
    jitter: override?.jitter ?? base?.jitter ?? basePolicy.jitter,
    retry_on: override?.retry_on ?? base?.retry_on ?? basePolicy.retry_on,
    do_not_retry: override?.do_not_retry ?? base?.do_not_retry ?? basePolicy.do_not_retry,
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

export interface RetryEffectEvent {
  type: "retry.attempt" | "retry.sleep" | "retry.exhausted" | "retry.skipped_non_retryable"
  payload: Record<string, unknown>
}

export async function retryAsync<A>(input: {
  policy: Required<ZyalRetryPolicy>
  category: RetryCategory
  run: () => Promise<A>
  classify: (error: unknown) => string
  emit?: (event: RetryEffectEvent) => Promise<void>
}): Promise<A> {
  let lastError: unknown
  for (let attempt = 1; attempt <= input.policy.max_attempts; attempt++) {
    await input.emit?.({
      type: "retry.attempt",
      payload: { category: input.category, attempt, max_attempts: input.policy.max_attempts },
    })
    try {
      return await input.run()
    } catch (error) {
      lastError = error
      const reason = input.classify(error)
      if (!isRetryableReason(input.policy, reason)) {
        await input.emit?.({
          type: "retry.skipped_non_retryable",
          payload: { category: input.category, attempt, reason },
        })
        throw error
      }
      if (attempt >= input.policy.max_attempts) {
        await input.emit?.({
          type: "retry.exhausted",
          payload: { category: input.category, attempt, reason },
        })
        throw error
      }
      const delayMs = computeRetryDelay(input.policy, attempt - 1)
      await input.emit?.({
        type: "retry.sleep",
        payload: { category: input.category, attempt, next_attempt: attempt + 1, reason, delay_ms: delayMs },
      })
      await new Promise((resolve) => setTimeout(resolve, delayMs))
    }
  }
  throw lastError
}

export function isRetryableReason(policy: Required<ZyalRetryPolicy>, reason: string): boolean {
  if (policy.do_not_retry.includes(reason)) return false
  if (policy.retry_on.length === 0) return true
  return policy.retry_on.includes(reason)
}

/**
 * Parse a duration string like "2s", "500ms", "1m" into milliseconds.
 */
export function parseDuration(duration: string): number {
  const match = duration.trim().match(/^(\d+(?:\.\d+)?)\s*(ms|millisecond(?:s)?|s|sec(?:ond)?s?|m|min(?:ute)?s?|h|hour(?:s)?)?$/i)
  if (!match) {
    throw new Error(`Invalid duration: ${duration}`)
  }
  const value = parseFloat(match[1])
  switch (match[2]?.toLowerCase()) {
    case "ms":
    case "millisecond":
    case "milliseconds":
      return value
    case "s":
    case "sec":
    case "secs":
    case "second":
    case "seconds":
    case undefined:
      return value * 1000
    case "m":
    case "min":
    case "mins":
    case "minute":
    case "minutes":
      return value * 60_000
    case "h":
    case "hour":
    case "hours":
      return value * 3_600_000
    default:
      return value * 1000
  }
}
