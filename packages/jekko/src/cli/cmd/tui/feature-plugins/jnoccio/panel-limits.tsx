import { createMemo, For } from "solid-js"
import { EmptyNotice, GREEN, ORANGE, RED, type PanelProps } from "./panel-common"
import { fmtMs, fmtN, fmtPct, maxConfigured, statusDot, textBar, truncate } from "./utils"

export function LimitsPanel(props: PanelProps) {
  const theme = () => props.api.theme.current
  const cap = () => props.snapshot.capacity
  const ctx = () => props.snapshot.context

  const knownModels = createMemo(() => cap().models)
  const unknownActive = createMemo(() =>
    cap().unknown_models.filter((m) => m.used > 0).sort((a, b) => b.successes - a.successes),
  )
  const errorModels = createMemo(() =>
    props.snapshot.models
      .filter((m) => m.last_error_kind)
      .sort((a, b) => b.failure_count - a.failure_count)
      .slice(0, 15),
  )
  const learnedCount = createMemo(() =>
    ctx().estimates.filter((e) => e.learned_context_window || e.learned_request_token_limit).length,
  )
  const overrunCount = createMemo(() =>
    ctx().recent_events.filter((e) => e.error_kind === "ContextOverflow").length,
  )

  return (
    <box flexDirection="column" width="100%" gap={1}>
      <text fg={theme().text}>
        <b>🔒 Rate Limit Observatory</b>
      </text>

      <box flexDirection="row" gap={2}>
        <text fg={theme().textMuted}>Configured {fmtN(maxConfigured(props.snapshot.models))}</text>
        <text fg={theme().textMuted}>Learned {fmtN(learnedCount())}</text>
        <text fg={theme().textMuted}>Overruns {fmtN(overrunCount())}</text>
      </box>

      <text fg={theme().text}>
        <b>Known — {fmtN(cap().known_remaining)}/{fmtN(cap().known_limit_per_hour)} remaining/hr</b>
      </text>
      {knownModels().length > 0 ? (
        <For each={knownModels()}>
          {(model) => {
            const pct = model.limit_per_hour > 0 ? (model.used / model.limit_per_hour) * 100 : 0
            const barColor = pct < 50 ? GREEN : pct < 80 ? ORANGE : RED
            const bar = textBar(model.used, model.limit_per_hour, 20)
            return (
              <box flexDirection="row" width="100%" gap={1} paddingLeft={1}>
                <text fg={theme().text} flexShrink={0}>
                  {statusDot(model.status)}
                </text>
                <box flexGrow={1} flexDirection="column">
                  <text fg={theme().text}>{truncate(model.display_name, 24)}</text>
                  <text fg={theme().textMuted}>
                    {model.provider} · {model.limit_kind}
                    {model.credit_tier ? " (credit)" : ""}
                  </text>
                </box>
                <text fg={barColor} flexShrink={0}>
                  {bar}
                </text>
                <text fg={theme().text} flexShrink={0}>
                  {fmtN(model.used)}/{fmtN(model.limit_per_hour)}
                </text>
              </box>
            )
          }}
        </For>
      ) : (
        <EmptyNotice api={props.api} message="No models with known limits" />
      )}

      {unknownActive().length > 0 ? (
        <>
          <text fg={theme().text}>
            <b>Inferred — Models Without Published Rates</b>
          </text>
          <For each={unknownActive()}>
            {(model) => {
              const maxInferred = Math.max(1, ...unknownActive().map((x) => x.successes))
              const bar = textBar(model.successes, maxInferred, 20)
              const failRate = model.used > 0 ? model.failures / model.used : 0
              return (
                <box flexDirection="row" width="100%" gap={1} paddingLeft={1}>
                  <box flexGrow={1} flexDirection="column">
                    <text fg={theme().text}>{model.display_name}</text>
                    <text fg={theme().textMuted}>
                      {model.provider} · {fmtN(model.used)} attempts · {fmtPct(failRate)} fail ·{" "}
                      {fmtMs(model.average_latency_ms)} avg
                    </text>
                  </box>
                  <text fg={GREEN} flexShrink={0}>
                    {bar}
                  </text>
                  <text fg={theme().text} flexShrink={0}>
                    ~{fmtN(model.successes)}/hr
                  </text>
                </box>
              )
            }}
          </For>
        </>
      ) : null}

      {errorModels().length > 0 ? (
        <>
          <text fg={theme().text}>
            <b>Error Patterns</b>
          </text>
          <For each={errorModels()}>
            {(model) => {
              const maxFail = Math.max(1, ...errorModels().map((x) => x.failure_count))
              const bar = textBar(model.failure_count, maxFail, 20)
              return (
                <box flexDirection="row" width="100%" gap={1} paddingLeft={1}>
                  <box flexGrow={1} flexDirection="column">
                    <text fg={theme().text}>{truncate(model.display_name, 24)}</text>
                    <text fg={theme().textMuted}>
                      {model.last_error_kind} — {truncate(model.last_error_message ?? "", 50)}
                    </text>
                  </box>
                  <text fg={RED} flexShrink={0}>
                    {bar}
                  </text>
                  <text fg={theme().text} flexShrink={0}>
                    {fmtN(model.failure_count)}
                  </text>
                </box>
              )
            }}
          </For>
        </>
      ) : null}
    </box>
  )
}
