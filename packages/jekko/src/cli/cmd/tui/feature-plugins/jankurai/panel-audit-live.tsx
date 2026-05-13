import { createMemo, For, Show } from "solid-js"
import { RGBA } from "@opentui/core"
import type { TuiPluginApi } from "@jekko-ai/plugin/tui"

import {
  useJankuraiScore,
  useJankuraiLastUpdated,
  formatJankuraiAge,
} from "../../context/jankurai-score"
import { useJankuraiHistory } from "../../context/jankurai-history"
import { useJankuraiBaseline } from "../../context/jankurai-baseline"
import { useZyalWorkers } from "../../context/zyal-runner"
import { sparkline } from "./sparkline"
import { delta, formatDelta } from "./delta"

const GOLD = RGBA.fromHex("#F5A623")
const GREEN = RGBA.fromHex("#22C55E")
const RED = RGBA.fromHex("#FF4757")
const BLUE = RGBA.fromHex("#3B82F6")

const SPARK_WIDTH = 24

export function JankuraiAuditLivePanel(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const score = useJankuraiScore()
  const history = useJankuraiHistory()
  const baseline = useJankuraiBaseline()
  const workers = useZyalWorkers
  const lastUpdated = useJankuraiLastUpdated()

  const sparkText = createMemo(() => {
    const points = history()
    return sparkline(points.map((p) => p.score), SPARK_WIDTH)
  })

  const scoreDelta = createMemo(() => delta(score()?.score, baseline()?.score, "score"))
  const capsDelta = createMemo(() => delta(score()?.capsApplied, baseline()?.capsApplied, "caps"))
  const hardDelta = createMemo(() => delta(score()?.hardFindings, baseline()?.hardFindings, "hard"))
  const softDelta = createMemo(() => delta(score()?.softFindings, baseline()?.softFindings, "soft"))

  const ageText = createMemo(() => {
    const ts = lastUpdated()
    return ts ? formatJankuraiAge(ts, Date.now()) : "no audit yet"
  })

  return (
    <box flexDirection="column" width="100%" gap={1}>
      {score() === null ? (
        <box>
          <text fg={theme().textMuted}>Jankurai not configured. Run `jekko jankurai bootstrap --yes` to scaffold.</text>
        </box>
      ) : null}
      <Show when={score() !== null}>
        <box flexDirection="row" gap={2}>
          <text fg={theme().text}>
            <b>Score {score()?.score.toFixed(1)}</b>
          </text>
          <text fg={GOLD}>{sparkText()}</text>
          <text fg={theme().textMuted}>vs main: {formatDelta(scoreDelta())}</text>
        </box>
        <text fg={theme().textMuted}>
          Audit · {ageText()} · {score()?.decision} · v{score()?.standardVersion}
        </text>
        <box flexDirection="row" gap={2}>
          <box flexDirection="column">
            <text fg={theme().text}>Caps {score()?.capsApplied}</text>
            <text fg={theme().text}>Hard {score()?.hardFindings}</text>
            <text fg={theme().text}>Soft {score()?.softFindings}</text>
            <text fg={theme().text}>Level {score()?.conformanceLevel}</text>
          </box>
          <box flexDirection="column">
            <text fg={pickColor(capsDelta().direction)}>Δ caps {formatDelta(capsDelta())}</text>
            <text fg={pickColor(hardDelta().direction)}>Δ hard {formatDelta(hardDelta())}</text>
            <text fg={pickColor(softDelta().direction)}>Δ soft {formatDelta(softDelta())}</text>
            <text fg={pickColor(scoreDelta().direction)}>Δ score {formatDelta(scoreDelta())}</text>
          </box>
        </box>
        <box flexDirection="column">
          <text fg={theme().text}>Workers</text>
          {workers().length === 0 ? (
            <text fg={theme().textMuted}>(no live workers)</text>
          ) : null}
          <Show when={workers().length > 0}>
            <For each={workers()}>
              {(worker) => (
                <text fg={theme().textMuted}>
                  ▣ {worker.workerID} · {worker.kind}
                </text>
              )}
            </For>
          </Show>
        </box>
      </Show>
    </box>
  )
}

function pickColor(direction: "improving" | "worsening" | "neutral" | "unknown") {
  if (direction === "improving") return GREEN
  if (direction === "worsening") return RED
  if (direction === "neutral") return BLUE
  return GOLD
}
