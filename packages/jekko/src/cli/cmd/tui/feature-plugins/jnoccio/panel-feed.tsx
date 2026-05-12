import { createMemo, For } from "solid-js"
import { EmptyNotice, RED, phaseColor, type PanelProps } from "./panel-common"
import { feedEvents } from "./panel-model"
import { fmtMs, fmtTime, statusDot, truncate } from "./utils"

export function FeedPanel(props: PanelProps) {
  const theme = () => props.api.theme.current

  const filtered = createMemo(() =>
    feedEvents(props.snapshot.recent_events, props.state.phaseFilter(), props.state.searchQuery()),
  )

  const statusColor = (status: string) => {
    if (status === "success" || status === "winner") return theme().success
    if (status === "failure") return RED
    return theme().textMuted
  }

  return (
    <box flexDirection="column" width="100%">
      <box flexDirection="row" justifyContent="space-between" paddingBottom={1}>
        <text fg={theme().text}>
          <b>📡 Live Feed — {props.snapshot.recent_events.length} events</b>
        </text>
        <text fg={theme().textMuted}>
          phase: {props.state.phaseFilter()} · [ ] to cycle
          {props.state.paused() ? " · ⏸ PAUSED" : ""}
        </text>
      </box>
      {filtered().length > 0 ? (
        <For each={filtered()}>
          {(event, idx) => {
            const isSelected = () => idx() === props.state.selectedIndex()
            return (
              <box
                flexDirection="row"
                width="100%"
                gap={1}
                backgroundColor={isSelected() ? theme().backgroundElement : undefined}
                paddingLeft={1}
                paddingRight={1}
              >
                <text fg={statusColor(event.status)} flexShrink={0}>
                  {statusDot(
                    event.status === "success" || event.status === "winner"
                      ? "ready"
                      : event.status === "failure"
                        ? "error"
                        : "cooldown",
                  )}
                </text>
                <text fg={phaseColor(event.phase)} flexShrink={0}>
                  {truncate(event.phase, 8)}
                </text>
                <text fg={theme().text} flexGrow={1}>
                  {truncate(event.model_id, 24)}
                </text>
                <text fg={statusColor(event.status)} flexShrink={0}>
                  {event.status}
                </text>
                <text fg={theme().textMuted} flexShrink={0}>
                  {event.error_kind ?? fmtMs(event.latency_ms)}
                </text>
                <text fg={theme().textMuted} flexShrink={0}>
                  {fmtTime(event.created_at)}
                </text>
              </box>
            )
          }}
        </For>
      ) : (
        <EmptyNotice api={props.api} message="No events match" />
      )}
    </box>
  )
}
