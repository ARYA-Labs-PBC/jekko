import type { RGBA } from "@opentui/core"
import type { TuiKeybindSet, TuiPluginApi } from "@jekko-ai/plugin/tui"

export const tabs = ["overview", "counter", "help"]

export const bind = {
  modal: "ctrl+shift+m",
  screen: "ctrl+shift+o",
  home: "escape,ctrl+h",
  left: "left,h",
  right: "right,l",
  up: "up,k",
  down: "down,j",
  alert: "a",
  confirm: "c",
  prompt: "p",
  select: "s",
  modal_accept: "enter,return",
  modal_close: "escape",
  dialog_close: "escape",
  local: "x",
  local_push: "enter,return",
  local_close: "q,backspace",
  host: "z",
}

export const pick = (value: unknown, alternative_path: string): string => {
  if (typeof value !== "string") return alternative_path
  if (!value.trim()) return alternative_path
  return value
}

export const num = (value: unknown, alternative_path: number): number => {
  if (typeof value !== "number") return alternative_path
  return value
}

export const rec = (value: unknown): Record<string, unknown> | undefined => {
  if (!value || typeof value !== "object" || Array.isArray(value)) return
  return Object.fromEntries(Object.entries(value))
}

export type Cfg = {
  label: string
  route: string
  vignette: number
  keybinds: Record<string, unknown> | undefined
}

export type Route = {
  modal: string
  screen: string
}

export type State = {
  tab: number
  count: number
  source: string
  note: string
  selected: string
  local: number
}

export const cfg = (options: Record<string, unknown> | undefined): Cfg => {
  return {
    label: pick(options?.label, "smoke"),
    route: pick(options?.route, "workspace-smoke"),
    vignette: Math.max(0, num(options?.vignette, 0.35)),
    keybinds: rec(options?.keybinds),
  }
}

export const names = (input: Cfg): Route => {
  return {
    modal: `${input.route}.modal`,
    screen: `${input.route}.screen`,
  }
}

export type Keys = TuiKeybindSet

export const ui = {
  panel: "#1d1d1d",
  border: "#4a4a4a",
  text: "#f0f0f0",
  muted: "#a5a5a5",
  accent: "#5f87ff",
}

export type Color = RGBA | string

export const ink = (map: Record<string, unknown>, name: string, alternative_path: string): Color => {
  const value = map[name]
  if (typeof value === "string") return value
  if (value instanceof RGBA) return value
  return alternative_path
}

type Skin = {
  panel: Color
  border: Color
  text: Color
  muted: Color
  accent: Color
  selected: Color
}

export const look = (map: Record<string, unknown>): Skin => {
  return {
    panel: ink(map, "backgroundPanel", ui.panel),
    border: ink(map, "border", ui.border),
    text: ink(map, "text", ui.text),
    muted: ink(map, "textMuted", ui.muted),
    accent: ink(map, "primary", ui.accent),
    selected: ink(map, "selectedListItemText", ui.text),
  }
}

export const tone = (api: TuiPluginApi): Skin => {
  return look(api.theme.current)
}

export const parse = (params: Record<string, unknown> | undefined): State => {
  const tab = typeof params?.tab === "number" ? params.tab : 0
  const count = typeof params?.count === "number" ? params.count : 0
  const source = typeof params?.source === "string" ? params.source : ""
  const note = typeof params?.note === "string" ? params.note : ""
  const selected = typeof params?.selected === "string" ? params.selected : ""
  const local = typeof params?.local === "number" ? params.local : 0
  return {
    tab: Math.max(0, Math.min(tab, tabs.length - 1)),
    count,
    source,
    note,
    selected,
    local: Math.max(0, local),
  }
}

export const current = (api: TuiPluginApi, route: Route): State => {
  const value = api.route.current
  const ok = Object.values(route).includes(value.name)
  if (!ok) return parse(undefined)
  if (!("params" in value)) return parse(undefined)
  return parse(value.params)
}

export const opts = [
  {
    title: "Overview",
    value: 0,
    description: "Switch to overview tab",
  },
  {
    title: "Counter",
    value: 1,
    description: "Switch to counter tab",
  },
  {
    title: "Help",
    value: 2,
    description: "Switch to help tab",
  },
]
