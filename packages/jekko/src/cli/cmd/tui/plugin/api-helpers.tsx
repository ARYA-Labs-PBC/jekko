import type { ParsedKey } from "@opentui/core"
import { InstallationVersion } from "@jekko-ai/core/installation/version"
import type { TuiDialogSelectOption, TuiPluginApi, TuiRouteDefinition, TuiSlotProps } from "@jekko-ai/plugin/tui"
import { Dialog as DialogUI } from "@tui/ui/dialog"
import { DialogAlert } from "../ui/dialog-alert"
import { DialogConfirm } from "../ui/dialog-confirm"
import { DialogPrompt } from "../ui/dialog-prompt"
import { DialogSelect, type DialogSelectOption as SelectOption } from "../ui/dialog-select"
import { Prompt } from "../component/prompt"
import { Slot as HostSlot } from "./slots"
import { createPluginKeybind } from "../context/plugin-keybinds"
import type { useRoute } from "@tui/context/route"
import type { useSync } from "@tui/context/sync"

type RouteEntry = {
  key: symbol
  render: TuiRouteDefinition["render"]
}

export type RouteMap = Map<string, RouteEntry[]>

type PromptSuggestions = {
  normal?: string[]
  shell?: string[]
}

function hasPromptSuggestions(props: object): props is { promptSuggestions?: PromptSuggestions } {
  return "promptSuggestions" in props
}

export function routeRegister(routes: RouteMap, list: TuiRouteDefinition[], bump: () => void) {
  const key = Symbol()
  for (const item of list) {
    const prev = routes.get(item.name) ?? []
    prev.push({ key, render: item.render })
    routes.set(item.name, prev)
  }
  bump()

  return () => {
    for (const item of list) {
      const prev = routes.get(item.name)
      if (!prev) continue
      const next = prev.filter((x) => x.key !== key)
      if (!next.length) {
        routes.delete(item.name)
        continue
      }
      routes.set(item.name, next)
    }
    bump()
  }
}

export function routeNavigate(route: ReturnType<typeof useRoute>, name: string, params?: Record<string, unknown>) {
  if (name === "home") {
    route.navigate({ type: "home" })
    return
  }

  if (name === "session") {
    const sessionID = params?.sessionID
    if (typeof sessionID !== "string") return
    route.navigate({ type: "session", sessionID })
    return
  }

  route.navigate({ type: "plugin", id: name, data: params })
}

export function routeCurrent(route: ReturnType<typeof useRoute>): TuiPluginApi["route"]["current"] {
  if (route.data.type === "home") return { name: "home" }
  if (route.data.type === "session") {
    return {
      name: "session",
      params: {
        sessionID: route.data.sessionID,
        prompt: route.data.prompt,
      },
    }
  }

  return {
    name: route.data.id,
    params: route.data.data,
  }
}

function mapOption<Value>(item: TuiDialogSelectOption<Value>): SelectOption<Value> {
  return {
    ...item,
    onSelect: () => item.onSelect?.(),
  }
}

function pickOption<Value>(item: SelectOption<Value>): TuiDialogSelectOption<Value> {
  return {
    title: item.title,
    value: item.value,
    description: item.description,
    footer: item.footer,
    category: item.category,
    disabled: item.disabled,
  }
}

function mapOptionCb<Value>(cb?: (item: TuiDialogSelectOption<Value>) => void) {
  if (!cb) return
  return (item: SelectOption<Value>) => cb(pickOption(item))
}

function stateApi(sync: ReturnType<typeof useSync>) {
  return {
    get ready() {
      return sync.ready
    },
    get config() {
      return sync.data.config
    },
    get provider() {
      return sync.data.provider
    },
    get path() {
      return sync.path
    },
    get vcs() {
      if (!sync.data.vcs) return
      return {
        branch: sync.data.vcs.branch,
      }
    },
    session: {
      count() {
        return sync.data.session.length
      },
      diff(sessionID: string) {
        return sync.data.session_diff[sessionID] ?? []
      },
      pending(sessionID: string) {
        return sync.data.pending[sessionID] ?? []
      },
      messages(sessionID: string) {
        return sync.data.message[sessionID] ?? []
      },
      status(sessionID: string) {
        return sync.data.session_status[sessionID]
      },
      permission(sessionID: string) {
        return sync.data.permission[sessionID] ?? []
      },
      question(sessionID: string) {
        return sync.data.question[sessionID] ?? []
      },
    },
    part(messageID: string) {
      return sync.data.part[messageID] ?? []
    },
    lsp() {
      return sync.data.lsp.map((item) => ({ id: item.id, root: item.root, status: item.status }))
    },
    mcp() {
      return Object.entries(sync.data.mcp)
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([name, item]) => ({
          name,
          status: item.status,
          error: item.status === "failed" ? item.error : undefined,
        }))
    },
  }
}

export function appApi(): TuiPluginApi["app"] {
  return {
    get version() {
      return InstallationVersion
    },
  }
}

export { DialogUI, DialogAlert, DialogConfirm, DialogPrompt, DialogSelect, HostSlot, createPluginKeybind, hasPromptSuggestions, mapOption, mapOptionCb, pickOption, stateApi }
