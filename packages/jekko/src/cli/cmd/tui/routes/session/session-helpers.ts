import parsers from "../../../../../../parsers-config.ts"
import { addDefaultParsers } from "@opentui/core"
import type { PromptInfo } from "../../component/prompt/history"

export const defaultParsers = parsers.parsers.map((entry) => ({
  ...entry,
  queries: {
    highlights: [...entry.queries.highlights] as string[],
    ...(entry.queries.locals ? { locals: [...entry.queries.locals] as string[] } : {}),
    ...(entry.queries.injections ? { injections: [...entry.queries.injections] as string[] } : {}),
  },
  ...(entry.injectionMapping
    ? {
        injectionMapping: {
          ...entry.injectionMapping,
          ...(entry.injectionMapping.nodeTypes ? { nodeTypes: { ...entry.injectionMapping.nodeTypes } } : {}),
          ...(entry.injectionMapping.infoStringMap
            ? { infoStringMap: { ...entry.injectionMapping.infoStringMap } }
            : {}),
        },
      }
    : {}),
})) as Parameters<typeof addDefaultParsers>[0]

addDefaultParsers(defaultParsers)

export const GO_UPSELL_LAST_SEEN_AT = "go_upsell_last_seen_at"
export const GO_UPSELL_DONT_SHOW = "go_upsell_dont_show"
export const GO_UPSELL_WINDOW = 86_400_000 // 24 hrs

export function emptyPromptParts(): PromptInfo["parts"] {
  return []
}

export function zyalExitReasonFromExitJson(value: unknown): string | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!value || typeof value !== "object") return undefined
  const record = value as Record<string, unknown>
  for (const key of ["reason", "message", "summary", "signal", "condition"]) {
    const v = record[key]
    if (typeof v === "string" && v.trim().length > 0) return v
  }
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return undefined
}

export function findNextVisibleMessage(input: {
  scroll: any
  messages: any[]
  sync: any
  direction: "next" | "prev"
}) {
  const { scroll, messages, sync, direction } = input
  const children = scroll.getChildren()
  const scrollTop = scroll.y

  const visibleMessages = children
    .filter((c: any) => {
      if (!c.id) return false
      const message = messages.find((m) => m.id === c.id)
      if (!message) return false
      const parts = sync.data.part[message.id]
      if (!parts || !Array.isArray(parts)) return false
      return parts.some((part: any) => part && part.type === "text" && !part.synthetic && !part.ignored)
    })
    .sort((a: any, b: any) => a.y - b.y)

  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (visibleMessages.length === 0) return null

  if (direction === "next") {
    return visibleMessages.find((c: any) => c.y > scrollTop + 10)?.id ?? null
  }
  return [...visibleMessages].reverse().find((c: any) => c.y < scrollTop - 10)?.id ?? null
}

export function scrollToMessage(input: {
  scroll: any
  dialog: any
  messages: any[]
  sync: any
  direction: "next" | "prev"
}) {
  const { scroll, dialog, messages, sync, direction } = input
  const targetID = findNextVisibleMessage({ scroll, messages, sync, direction })
  if (!targetID) {
    scroll.scrollBy(direction === "next" ? scroll.height : -scroll.height)
    dialog.clear()
    return
  }
  const child = scroll.getChildren().find((c: any) => c.id === targetID)
  if (child) scroll.scrollBy(child.y - scroll.y - 1)
  dialog.clear()
}

export function toBottom(scroll: any) {
  setTimeout(() => {
    if (!scroll || scroll.isDestroyed) return
    scroll.scrollTo(scroll.scrollHeight)
  }, 50)
}
