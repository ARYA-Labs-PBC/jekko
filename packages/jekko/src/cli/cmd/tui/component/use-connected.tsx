import { createMemo } from "solid-js"
import { useSync } from "@tui/context/sync"

export function hasConnectedProvider(provider: { models: Record<string, { status?: string } | unknown> }) {
  return Object.values(provider.models).some((model) => {
    if (!model || typeof model !== "object" || !("status" in model)) return true
    return model.status !== "locked"
  })
}

export function useConnected() {
  const sync = useSync()
  return createMemo(() => sync.data.provider.some(hasConnectedProvider))
}
