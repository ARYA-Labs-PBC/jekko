import { createMemo, For } from "solid-js"
import { BLUE, EmptyNotice, ORANGE, RED, TEAL, type PanelProps } from "./panel-common"
import { fmtMs, latencyIcon, latencyTier, textBar, truncate } from "./utils"
import { latencyModels } from "./panel-model"

export function LatencyPanel(props: PanelProps) {
  const theme = () => props.api.theme.current

  const sorted = createMemo(() => latencyModels(props.snapshot.models, props.state.searchQuery()))
  const maxLat = createMemo(() => Math.max(1, ...sorted().map((m) => m.avg_latency_ms ?? 0)))

  return (
    <box flexDirection="column" width="100%">
      <box paddingBottom={1}>
        <text fg={theme().text}>
          <b>⚡ Latency Arena — Who's Fastest?</b>
        </text>
      </box>
      {sorted().length > 0 ? (
        <For each={sorted()}>
          {(model, idx) => {
            const isSelected = () => idx() === props.state.selectedIndex()
            const lat = model.avg_latency_ms ?? 0
            const tier = latencyTier(lat)
            const icon = latencyIcon(tier)
            const barColor =
              tier === "fast" ? TEAL : tier === "normal" ? BLUE : tier === "slow" ? ORANGE : RED
            const bar = textBar(lat, maxLat(), 20)

            return (
              <box
                flexDirection="row"
                width="100%"
                gap={1}
                backgroundColor={isSelected() ? theme().backgroundElement : undefined}
                paddingLeft={1}
                paddingRight={1}
              >
                <text fg={barColor} flexShrink={0}>
                  {icon}
                </text>
                <box flexGrow={1} flexDirection="column">
                  <text fg={isSelected() ? theme().selectedListItemText : theme().text}>
                    {truncate(model.display_name, 24)}
                  </text>
                  <text fg={theme().textMuted}>
                    {model.provider} · min {fmtMs(model.min_latency_ms)} · max {fmtMs(model.max_latency_ms)}
                  </text>
                </box>
                <text fg={barColor} flexShrink={0}>
                  {bar}
                </text>
                <text fg={theme().text} flexShrink={0}>
                  {fmtMs(lat)}
                </text>
              </box>
            )
          }}
        </For>
      ) : (
        <EmptyNotice api={props.api} message="No latency data yet" />
      )}
    </box>
  )
}
