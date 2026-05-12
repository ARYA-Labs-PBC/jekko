import { describe, expect, test } from "bun:test"
import {
  resolveRetryPolicy,
  computeRetryDelay,
  canRetry,
  parseDuration,
} from "@/session/daemon-retry"
import type { ZyalRetry } from "@/agent-script/schema"

describe("daemon retry", () => {
  test("parseDuration handles seconds", () => {
    expect(parseDuration("2s")).toBe(2000)
    expect(parseDuration("0.5s")).toBe(500)
  })

  test("parseDuration handles milliseconds", () => {
    expect(parseDuration("500ms")).toBe(500)
  })

  test("parseDuration handles minutes", () => {
    expect(parseDuration("1m")).toBe(60_000)
  })

  test("parseDuration handles bare numbers as seconds", () => {
    expect(parseDuration("5")).toBe(5000)
  })

  test("resolveRetryPolicy uses override over default", () => {
    const retry: ZyalRetry = {
      default: { max_attempts: 3, backoff: "exponential", initial_delay: "2s" },
      overrides: {
        shell_checks: { max_attempts: 5, backoff: "linear" },
      },
    }
    const policy = resolveRetryPolicy(retry, "shell_checks")
    expect(policy.max_attempts).toBe(5)
    expect(policy.backoff).toBe("linear")
    // Falls through to default for initial_delay
    expect(policy.initial_delay).toBe("2s")
  })

  test("resolveRetryPolicy uses default when no override", () => {
    const retry: ZyalRetry = {
      default: { max_attempts: 3, backoff: "exponential" },
    }
    const policy = resolveRetryPolicy(retry, "checkpoint")
    expect(policy.max_attempts).toBe(3)
    expect(policy.backoff).toBe("exponential")
  })

  test("resolveRetryPolicy uses built-in policy when retry is undefined", () => {
    const policy = resolveRetryPolicy(undefined, "shell_checks")
    expect(policy.max_attempts).toBe(1)
    expect(policy.backoff).toBe("none")
  })

  test("computeRetryDelay with exponential backoff", () => {
    const policy = resolveRetryPolicy(
      { default: { backoff: "exponential", initial_delay: "1s", max_delay: "60s", jitter: false } },
      "shell_checks",
    )
    expect(computeRetryDelay(policy, 0)).toBe(1000)
    expect(computeRetryDelay(policy, 1)).toBe(2000)
    expect(computeRetryDelay(policy, 2)).toBe(4000)
    expect(computeRetryDelay(policy, 3)).toBe(8000)
  })

  test("computeRetryDelay respects max_delay", () => {
    const policy = resolveRetryPolicy(
      { default: { backoff: "exponential", initial_delay: "10s", max_delay: "15s", jitter: false } },
      "shell_checks",
    )
    expect(computeRetryDelay(policy, 0)).toBe(10_000)
    // 2^1 * 10s = 20s > 15s, capped
    expect(computeRetryDelay(policy, 1)).toBe(15_000)
  })

  test("computeRetryDelay with linear backoff", () => {
    const policy = resolveRetryPolicy(
      { default: { backoff: "linear", initial_delay: "1s", max_delay: "60s", jitter: false } },
      "shell_checks",
    )
    expect(computeRetryDelay(policy, 0)).toBe(1000)
    expect(computeRetryDelay(policy, 1)).toBe(2000)
    expect(computeRetryDelay(policy, 2)).toBe(3000)
  })

  test("canRetry tracks attempts correctly", () => {
    const policy = resolveRetryPolicy(
      { default: { max_attempts: 3 } },
      "shell_checks",
    )
    expect(canRetry(policy, 0)).toBe(true)
    expect(canRetry(policy, 1)).toBe(true)
    expect(canRetry(policy, 2)).toBe(true)
    expect(canRetry(policy, 3)).toBe(false)
  })
})
