import { createMemo, For } from "solid-js"
import { EmptyNotice, GOLD, type PanelProps } from "./panel-common"
import {
  fmtN,
  fmtPct,
  flashesForModel,
  statusDot,
  successRate,
  textBar,
  truncate,
} from "./utils"
import { leaderboardModels, sortLabel } from "./panel-model"

export function LeaderboardPanel(props: PanelProps) {
  const theme = () => props.api.theme.current

  const sorted = createMemo(() =>
    leaderboardModels(
      props.snapshot.models,
      props.state.sortMode(),
      props.state.flashMap(),
      props.state.searchQuery(),
    ),
  )

  const maxVal = createMemo(() => {
    const mode = props.state.sortMode()
    const models = sorted()
    return Math.max(
      1,
      ...models.map((m) => {
        if (mode === "latest" || mode === "wins") return m.win_count
        if (mode === "win_rate") return m.win_rate
        return successRate(m)
      }),
    )
  })

  return (
    <box flexDirection="column" width="100%">
      <box flexDirection="row" justifyContent="space-between" paddingBottom={1}>
        <text fg={theme().text}>
          <b>🏆 Leaderboard — Who's Winning?</b>
        </text>
        <text fg={theme().textMuted}>sort: {sortLabel(props.state.sortMode())} (s to cycle)</text>
      </box>
      {sorted().length > 0 ? (
        <For each={sorted()}>
          {(model, idx) => {
            const isSelected = () => idx() === props.state.selectedIndex()
            const now = Date.now()
            const flashes = () => flashesForModel(props.state.flashMap(), model.id, now)
            const isFlashing = () => flashes().length > 0
            const mode = props.state.sortMode()
            const val =
              mode === "latest" || mode === "wins"
                ? model.win_count
                : mode === "win_rate"
                  ? model.win_rate
                  : successRate(model)
            const display = mode === "latest" || mode === "wins" ? fmtN(model.win_count) : fmtPct(val)
            const bar = textBar(val, maxVal(), 20)

            return (
              <box
                flexDirection="row"
                width="100%"
                gap={1}
                backgroundColor={isSelected() ? theme().backgroundElement : undefined}
                paddingLeft={1}
                paddingRight={1}
              >
                <text fg={isFlashing() ? GOLD : theme().text} flexShrink={0}>
                  {statusDot(model.status)}
                </text>
                <box flexGrow={1} flexDirection="column">
                  <text fg={isSelected() ? theme().selectedListItemText : theme().text}>
                    {isFlashing() ? "◌ " : ""}
                    {truncate(model.display_name, 24)}
                  </text>
                  <text fg={theme().textMuted}>
                    {model.provider} · {fmtN(model.call_count)} calls · {fmtPct(model.win_rate)} win
                  </text>
                </box>
                <text fg={GOLD} flexShrink={0}>
                  {bar}
                </text>
                <text fg={theme().text} flexShrink={0}>
                  {display}
                </text>
              </box>
            )
          }}
        </For>
      ) : (
        <EmptyNotice api={props.api} message="No models with calls yet" />
      )}
    </box>
  )
}
