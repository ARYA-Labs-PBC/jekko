import { createContext, useContext } from "solid-js"
import type { Provider } from "@jekko-ai/sdk/v2"
import { useSync } from "@tui/context/sync"
import { useTuiConfig } from "../../context/tui-config"

type SessionContextValue = {
  width: number
  sessionID: string
  conceal: () => boolean
  showThinking: () => boolean
  showTimestamps: () => boolean
  showDetails: () => boolean
  showGenericToolOutput: () => boolean
  diffWrapMode: () => "word" | "none"
  providers: () => ReadonlyMap<string, Provider>
  sync: ReturnType<typeof useSync>
  tui: ReturnType<typeof useTuiConfig>
}

export const context = createContext<SessionContextValue>()

export function use() {
  const ctx = useContext(context)
  if (!ctx) throw new Error("useContext must be used within a Session component")
  return ctx
}

export function isRecord(value: unknown): value is Record<string, any> {
  return !!value && typeof value === "object"
}
