import { describe, expect, test } from "bun:test"
import {
  emptySnapshot,
  normalizeSnapshot,
  totalsFromModels,
} from "../../../src/cli/cmd/tui/context/jnoccio-snapshot"
import type { DashboardModel, MetricEvent } from "../../../src/cli/cmd/tui/context/jnoccio-types"

function model(input: Partial<DashboardModel> & Pick<DashboardModel, "id">): DashboardModel {
  return {
    id: input.id,
    provider: input.provider ?? "test",
    display_name: input.display_name ?? input.id,
    upstream_model: input.upstream_model ?? input.id,
    roles: input.roles ?? [],
    enabled: input.enabled ?? true,
    status: input.status ?? "ok",
    cooldown_until: input.cooldown_until ?? null,
    capacity_known: input.capacity_known ?? false,
    hourly_capacity: input.hourly_capacity ?? null,
    hourly_used: input.hourly_used ?? 0,
    configured_context_window: input.configured_context_window ?? 0,
    safe_context_window: input.safe_context_window ?? 0,
    learned_context_window: input.learned_context_window ?? null,
    learned_request_token_limit: input.learned_request_token_limit ?? null,
    context_overrun_count: input.context_overrun_count ?? 0,
    smallest_overrun_requested_tokens: input.smallest_overrun_requested_tokens ?? null,
    call_count: input.call_count ?? 0,
    success_count: input.success_count ?? 0,
    failure_count: input.failure_count ?? 0,
    win_count: input.win_count ?? 0,
    win_rate: input.win_rate ?? 0,
    prompt_tokens: input.prompt_tokens ?? 0,
    completion_tokens: input.completion_tokens ?? 0,
    total_tokens: input.total_tokens ?? 0,
    avg_latency_ms: input.avg_latency_ms ?? null,
    last_latency_ms: input.last_latency_ms ?? null,
    min_latency_ms: input.min_latency_ms ?? null,
    max_latency_ms: input.max_latency_ms ?? null,
    last_error_kind: input.last_error_kind ?? null,
    last_error_message: input.last_error_message ?? null,
    updated_at: input.updated_at ?? 0,
  }
}

function event(input: Partial<MetricEvent> & Pick<MetricEvent, "id">): MetricEvent {
  return {
    id: input.id,
    request_id: input.request_id ?? `req-${input.id}`,
    phase: input.phase ?? "test",
    model_id: input.model_id ?? "model",
    provider: input.provider ?? "provider",
    status: input.status ?? "success",
    error_kind: input.error_kind ?? null,
    latency_ms: input.latency_ms ?? null,
    prompt_tokens: input.prompt_tokens ?? 0,
    completion_tokens: input.completion_tokens ?? 0,
    total_tokens: input.total_tokens ?? 0,
    route_mode: input.route_mode ?? null,
    backup_rank: input.backup_rank ?? null,
    complexity_tier: input.complexity_tier ?? null,
    sampled: input.sampled ?? null,
    winner_model_id: input.winner_model_id ?? null,
    capacity_known: input.capacity_known ?? null,
    agent_id: input.agent_id ?? null,
    agent_client: input.agent_client ?? null,
    agent_session_id: input.agent_session_id ?? null,
    created_at: input.created_at ?? input.id,
  }
}

describe("jnoccio snapshot normalization", () => {
  test("normalizes non-finite totals to bounded defaults", () => {
    const snapshot = emptySnapshot()
    snapshot.totals.calls = Number.NaN
    snapshot.totals.successes = Number.POSITIVE_INFINITY
    snapshot.totals.failures = Number.NEGATIVE_INFINITY
    snapshot.totals.prompt_tokens = 123
    snapshot.capacity.percent_used = Number.NaN
    snapshot.token_rate.median_m_tokens_per_24h = Number.POSITIVE_INFINITY

    const normalized = normalizeSnapshot(snapshot)

    expect(normalized.totals.calls).toBe(0)
    expect(normalized.totals.successes).toBe(0)
    expect(normalized.totals.failures).toBe(0)
    expect(normalized.totals.prompt_tokens).toBe(123)
    expect(normalized.capacity.percent_used).toBe(0)
    expect(normalized.token_rate.median_m_tokens_per_24h).toBe(0)
  })

  test("normalizes non-finite average latency to null", () => {
    const snapshot = emptySnapshot()
    snapshot.totals.average_latency_ms = Number.NaN

    expect(normalizeSnapshot(snapshot).totals.average_latency_ms).toBeNull()
  })

  test("deduplicates, sorts, and limits recent events", () => {
    const snapshot = emptySnapshot()
    snapshot.recent_events = Array.from({ length: 305 }, (_, index) =>
      event({ id: index + 1, created_at: index + 1 }),
    )
    snapshot.recent_events.push(event({ id: 12, created_at: 9999, request_id: "duplicate" }))

    const normalized = normalizeSnapshot(snapshot)

    expect(normalized.recent_events).toHaveLength(300)
    expect(normalized.recent_events.filter((item) => item.id === 12)).toHaveLength(1)
    expect(normalized.recent_events[0]?.id).toBe(305)
    expect(normalized.recent_events.at(-1)?.id).toBe(6)
  })

  test("computes weighted latency totals from models", () => {
    const totals = totalsFromModels([
      model({
        id: "a",
        call_count: 3,
        success_count: 2,
        failure_count: 1,
        prompt_tokens: 30,
        completion_tokens: 15,
        total_tokens: 45,
        avg_latency_ms: 100,
      }),
      model({
        id: "b",
        enabled: false,
        call_count: 1,
        win_count: 1,
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        avg_latency_ms: 200,
      }),
    ])

    expect(totals.total_models).toBe(2)
    expect(totals.enabled_models).toBe(1)
    expect(totals.calls).toBe(4)
    expect(totals.successes).toBe(2)
    expect(totals.failures).toBe(1)
    expect(totals.wins).toBe(1)
    expect(totals.prompt_tokens).toBe(40)
    expect(totals.completion_tokens).toBe(20)
    expect(totals.total_tokens).toBe(60)
    expect(totals.average_latency_ms).toBe(125)
  })
})
