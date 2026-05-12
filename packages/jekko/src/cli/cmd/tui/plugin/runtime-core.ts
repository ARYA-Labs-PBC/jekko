// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import "@opentui/solid/runtime-plugin-support"
import {
  type TuiPluginApi,
  type TuiPlugin,
  type TuiPluginMeta,
  type TuiPluginModule,
  type TuiPluginStatus,
  type TuiTheme,
} from "@jekko-ai/plugin/tui"
import path from "path"
import { fileURLToPath } from "node:url"
import { TuiConfig } from "@/cli/cmd/tui/config/tui"
import { ConfigPlugin } from "@/config/plugin"
import * as Log from "@jekko-ai/core/util/log"
import { errorData, errorMessage } from "@/util/error"
import { isRecord } from "@/util/record"
import type { PluginSource } from "@/plugin/shared"
import { INTERNAL_TUI_PLUGINS, type InternalTuiPlugin } from "./internal"
import type { HostPluginApi, HostSlots } from "./slots"
export { createThemeInstaller, readThemeFiles, syncPluginThemes } from "./runtime-theme"

export type PluginLoad = {
  options: ConfigPlugin.Options | undefined
  spec: string
  target: string
  retry: boolean
  source: PluginSource | "internal"
  id: string
  module: TuiPluginModule
  origin: ConfigPlugin.Origin
  theme_root: string
  theme_files: string[]
}

export type Api = HostPluginApi

export type PluginScope = {
  lifecycle: TuiPluginApi["lifecycle"]
  track: (fn: (() => void) | undefined) => () => void
  dispose: () => Promise<void>
}

export type PluginEntry = {
  id: string
  load: PluginLoad
  meta: TuiPluginMeta
  themes: Record<string, PluginMeta.Theme>
  plugin: TuiPlugin
  enabled: boolean
  scope?: PluginScope
}

export type RuntimeState = {
  directory: string
  api: Api
  slots: HostSlots
  plugins: PluginEntry[]
  plugins_by_id: Map<string, PluginEntry>
  pending: Map<string, ConfigPlugin.Origin>
}

export type CleanupResult = { type: "ok" } | { type: "error"; error: unknown } | { type: "timeout" }

export const log = Log.create({ service: "tui.plugin" })
export const DISPOSE_TIMEOUT_MS = 5000
export const KV_KEY = "plugin_enabled"
export const EMPTY_TUI: TuiPluginModule = {
  tui: async () => {},
}

export function fail(message: string, data: Record<string, unknown>) {
  if (!("error" in data)) {
    log.error(message, data)
    console.error(`[tui.plugin] ${message}`, data)
    return
  }

  const text = `${message}: ${errorMessage(data.error)}`
  const next = { ...data, error: errorData(data.error) }
  log.error(text, next)
  console.error(`[tui.plugin] ${text}`, next)
}

export function warn(message: string, data: Record<string, unknown>) {
  log.warn(message, data)
  console.warn(`[tui.plugin] ${message}`, data)
}

export function runCleanup(fn: () => unknown, ms: number): Promise<CleanupResult> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      resolve({ type: "timeout" })
    }, ms)

    Promise.resolve()
      .then(fn)
      .then(
        () => {
          resolve({ type: "ok" })
        },
        (error) => {
          resolve({ type: "error", error })
        },
      )
      .finally(() => {
        clearTimeout(timer)
      })
  })
}

export function isTheme(value: unknown) {
  if (!isRecord(value)) return false
  if (!("theme" in value)) return false
  if (!isRecord(value.theme)) return false
  return true
}

export function resolveRoot(root: string) {
  if (root.startsWith("file://")) {
    const file = fileURLToPath(root)
    if (root.endsWith("/")) return file
    return path.dirname(file)
  }
  if (path.isAbsolute(root)) return root
  return path.resolve(process.cwd(), root)
}

export function createMeta(
  source: PluginLoad["source"],
  spec: string,
  target: string,
  meta: { state: PluginMeta.State; entry: PluginMeta.Entry } | undefined,
  id?: string,
): TuiPluginMeta {
  if (meta) {
    return {
      state: meta.state,
      ...meta.entry,
    }
  }

  const now = Date.now()
  return {
    state: source === "internal" ? "same" : "first",
    id: id ?? spec,
    source,
    spec,
    target,
    first_time: now,
    last_time: now,
    time_changed: now,
    load_count: 1,
    fingerprint: target,
  }
}

export function loadInternalPlugin(item: InternalTuiPlugin): PluginLoad {
  const spec = item.id
  const target = spec

  return {
    options: undefined,
    spec,
    target,
    retry: false,
    source: "internal",
    id: item.id,
    module: item,
    origin: {
      spec,
      scope: "global",
      source: target,
    },
    theme_root: process.cwd(),
    theme_files: [],
  }
}

export function createPluginScope(load: PluginLoad, id: string) {
  const ctrl = new AbortController()
  let list: { key: symbol; fn: () => void }[] = []
  let done = false

  const onDispose = (fn: () => void) => {
    if (done) return () => {}
    const key = Symbol()
    list.push({ key, fn })
    let drop = false
    return () => {
      if (drop) return
      drop = true
      list = list.filter((x) => x.key !== key)
    }
  }

  const track = (fn: (() => void) | undefined) => {
    if (!fn) return () => {}
    const off = onDispose(fn)
    let drop = false
    return () => {
      if (drop) return
      drop = true
      off()
      fn()
    }
  }

  const lifecycle: TuiPluginApi["lifecycle"] = {
    signal: ctrl.signal,
    onDispose,
  }

  const dispose = async () => {
    if (done) return
    done = true
    ctrl.abort()
    const queue = [...list].reverse()
    list = []
    const until = Date.now() + DISPOSE_TIMEOUT_MS
    for (const item of queue) {
      const left = until - Date.now()
      if (left <= 0) {
        fail("timed out cleaning up tui plugin", {
          path: load.spec,
          id,
          timeout: DISPOSE_TIMEOUT_MS,
        })
        break
      }

      const out = await runCleanup(item.fn, left)
      if (out.type === "ok") continue
      if (out.type === "timeout") {
        fail("timed out cleaning up tui plugin", {
          path: load.spec,
          id,
          timeout: DISPOSE_TIMEOUT_MS,
        })
        break
      }

      if (out.type === "error") {
        fail("failed to clean up tui plugin", {
          path: load.spec,
          id,
          error: out.error,
        })
      }
    }
  }

  return {
    lifecycle,
    track,
    dispose,
  }
}

export function readPluginEnabledMap(value: unknown) {
  if (!isRecord(value)) return {}
  return Object.fromEntries(
    Object.entries(value).filter((item): item is [string, boolean] => typeof item[1] === "boolean"),
  )
}

export function pluginEnabledState(state: RuntimeState, config: TuiConfig.Info) {
  return {
    ...readPluginEnabledMap(config.plugin_enabled),
    ...readPluginEnabledMap(state.api.kv.get(KV_KEY, {})),
  }
}

export function writePluginEnabledState(api: Api, id: string, enabled: boolean) {
  api.kv.set(KV_KEY, {
    ...readPluginEnabledMap(api.kv.get(KV_KEY, {})),
    [id]: enabled,
  })
}

export function listPluginStatus(state: RuntimeState): TuiPluginStatus[] {
  return state.plugins.map((plugin) => ({
    id: plugin.id,
    source: plugin.meta.source,
    spec: plugin.meta.spec,
    target: plugin.meta.target,
    enabled: plugin.enabled,
    active: plugin.scope !== undefined,
  }))
}
