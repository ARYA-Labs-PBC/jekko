import { RGBA } from "@opentui/core"
import type { TuiPluginApi } from "@jekko-ai/plugin/tui"
import type { DashboardSnapshot } from "../../context/jnoccio-types"
import type { DashboardState } from "./state"
import { phaseTone } from "./panel-model"

export const GOLD = RGBA.fromHex("#F5A623")
export const TEAL = RGBA.fromHex("#36D7B7")
export const RED = RGBA.fromHex("#FF4757")
export const PURPLE = RGBA.fromHex("#A855F7")
export const BLUE = RGBA.fromHex("#3B82F6")
export const GREEN = RGBA.fromHex("#22C55E")
export const ORANGE = RGBA.fromHex("#F97316")
export const CYAN = RGBA.fromHex("#06B6D4")

export type PanelProps = {
  api: TuiPluginApi
  snapshot: DashboardSnapshot
  state: DashboardState
}

export function EmptyNotice(props: { api: TuiPluginApi; message: string }) {
  const theme = () => props.api.theme.current
  return <text fg={theme().textMuted}>{props.message}</text>
}

export function phaseColor(phase: string) {
  switch (phaseTone(phase)) {
    case "primary":
      return TEAL
    case "secondary":
      return ORANGE
    case "retry":
      return PURPLE
    case "default":
      return BLUE
  }
}
