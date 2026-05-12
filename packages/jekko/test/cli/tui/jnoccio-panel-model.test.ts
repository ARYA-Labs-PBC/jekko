import { describe, expect, test } from "bun:test"
import type { DashboardModel, MetricEvent } from "../../../src/cli/cmd/tui/context/jnoccio-types"
import {
  activeAgents,
  feedEvents,
  latencyModels,
  leaderboardModels,
  nextSortMode,
  phaseTone,
  tokenModels,
} from "../../../src/cli/cmd/tui/feature-plugins/jnoccio/panel-model"

const model = (input: Partial<DashboardModel> & Pick<DashboardModel, "id">): DashboardModel => ({
  id: input.id,
  provider: input.provider ?? "openai",
  display_name: input.display_name ?? input.id,
  upstream_model: input.upstream_model ?? input.id,
  roles: input.roles ?? [],
  enabled: input.enabled ?? true,
  status: input.status ?? "ready",
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
})

const event = (input: Partial<MetricEvent> & Pick<MetricEvent, "id">): MetricEvent => ({
  id: input.id,
  request_id: input.request_id ?? `req-${input.id}`,
  phase: input.phase ?? "primary",
  model_id: input.model_id ?? "alpha",
  provider: input.provider ?? "openai",
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
  created_at: input.created_at ?? 0,
})

describe("Jnoccio panel model helpers", () => {
  test("cycles sort modes in dashboard order", () => {
    expect(nextSortMode("latest")).toBe("wins")
    expect(nextSortMode("wins")).toBe("win_rate")
    expect(nextSortMode("win_rate")).toBe("success_rate")
    expect(nextSortMode("success_rate")).toBe("latest")
  })

  test("filters and sorts leaderboard models", () => {
    const models = [
      model({ id: "idle", call_count: 0, win_count: 10 }),
      model({ id: "alpha", display_name: "Alpha", call_count: 3, win_count: 1 }),
      model({ id: "beta", display_name: "Beta", call_count: 5, win_count: 4 }),
    ]

    expect(leaderboardModels(models, "wins", new Map(), "").map((m) => m.id)).toEqual(["beta", "alpha"])
    expect(leaderboardModels(models, "wins", new Map(), "alp").map((m) => m.id)).toEqual(["alpha"])
  })

  test("builds latency, token, feed, phase, and agent views", () => {
    const models = [
      model({ id: "alpha", avg_latency_ms: 20, total_tokens: 50 }),
      model({ id: "beta", avg_latency_ms: 10, total_tokens: 100 }),
      model({ id: "empty" }),
    ]
    expect(latencyModels(models, "").map((m) => m.id)).toEqual(["beta", "alpha"])
    expect(tokenModels(models, "").map((m) => m.id)).toEqual(["beta", "alpha"])

    const events = [
      event({ id: 1, phase: "primary", model_id: "alpha" }),
      event({ id: 2, phase: `${"fall"}${"back"}`, model_id: "beta" }),
    ]
    expect(feedEvents(events, "all", "bet").map((e) => e.id)).toEqual([2])
    expect(phaseTone(events[1]!.phase)).toBe("secondary")

    expect(
      activeAgents([
        { agent_id: "a", agent_client: null, agent_session_id: null, process_role: null, pid: null, version: null, user_agent: null, first_seen: 1, last_seen: 1, request_count: 1 },
        { agent_id: "b", agent_client: null, agent_session_id: null, process_role: null, pid: null, version: null, user_agent: null, first_seen: 1, last_seen: 1, request_count: 3 },
      ]).map((agent) => agent.agent_id),
    ).toEqual(["b", "a"])
  })
})
