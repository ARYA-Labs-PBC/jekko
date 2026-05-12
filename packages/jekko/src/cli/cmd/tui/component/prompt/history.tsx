import path from "path"
import { Global } from "@jekko-ai/core/global"
import { Filesystem } from "@/util/filesystem"
import { onMount } from "solid-js"
import { createStore, produce, unwrap } from "solid-js/store"
import { createSimpleContext } from "../../context/helper"
import { appendFile, writeFile } from "fs/promises"
import type { AgentPart, FilePart, TextPart } from "@jekko-ai/sdk/v2"
import { z } from "zod"

export type PromptInfo = {
  input: string
  mode?: "normal" | "shell"
  parts: (
    | Omit<FilePart, "id" | "messageID" | "sessionID">
    | Omit<AgentPart, "id" | "messageID" | "sessionID">
    | (Omit<TextPart, "id" | "messageID" | "sessionID"> & {
        source?: {
          text: {
            start: number
            end: number
            value: string
          }
        }
      })
  )[]
}

const MAX_HISTORY_ENTRIES = 50
const HISTORY_WRITE_ATTEMPTS = 2
const HISTORY_REPAIR_ATTEMPTS = 3
const HISTORY_WRITE_RETRY_DELAY_MS = 25

type PromptHistoryLoadState =
  | { kind: "missing" }
  | { kind: "loaded"; entries: PromptInfo[]; dropped: number }
  | { kind: "error"; error: unknown }

type PromptHistoryWriteState =
  | { kind: "written" }
  | { kind: "failed"; operation: "append" | "repair"; error: unknown }

type PromptHistoryMoveResult =
  | { kind: "blocked" }
  | { kind: "selected"; prompt: PromptInfo }
  | { kind: "cleared"; prompt: PromptInfo }

function isMissingHistoryError(error: unknown) {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    (error as { code?: string }).code === "ENOENT"
  )
}

const PromptTextSourceSchema = z.object({
  start: z.number().finite(),
  end: z.number().finite(),
  value: z.string(),
})

const PromptTextPartSchema = z.object({
  type: z.literal("text"),
  text: z.string(),
  synthetic: z.boolean().optional(),
  ignored: z.boolean().optional(),
  time: z
    .object({
      start: z.number().finite(),
      end: z.number().finite().optional(),
    })
    .optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
  source: z
    .object({
      text: PromptTextSourceSchema,
    })
    .optional(),
})

const PromptFilePartSchema = z.object({
  type: z.literal("file"),
  mime: z.string(),
  filename: z.string().optional(),
  url: z.string(),
  source: z
    .union([
      z.object({
        type: z.literal("file"),
        path: z.string(),
        text: PromptTextSourceSchema,
      }),
      z.object({
        type: z.literal("symbol"),
        path: z.string(),
        range: z.object({
          start: z.object({
            line: z.number().finite(),
            character: z.number().finite(),
          }),
          end: z.object({
            line: z.number().finite(),
            character: z.number().finite(),
          }),
        }),
        name: z.string(),
        kind: z.number().finite(),
        text: PromptTextSourceSchema,
      }),
      z.object({
        type: z.literal("resource"),
        clientName: z.string(),
        uri: z.string(),
        text: PromptTextSourceSchema,
      }),
    ])
    .optional(),
})

const PromptAgentPartSchema = z.object({
  type: z.literal("agent"),
  name: z.string(),
  source: z
    .object({
      value: z.string(),
      start: z.number().finite(),
      end: z.number().finite(),
    })
    .optional(),
})

const PromptInfoSchema = z.object({
  input: z.string(),
  mode: z.enum(["normal", "shell"]).optional(),
  parts: z.array(z.union([PromptTextPartSchema, PromptFilePartSchema, PromptAgentPartSchema])),
})

type PromptHistoryLineState =
  | { kind: "valid"; value: PromptInfo }
  | { kind: "invalid"; reason: string }

export function decodePromptHistoryLine(line: string): PromptHistoryLineState {
  let parsed: unknown
  try {
    parsed = JSON.parse(line)
  } catch (error) {
    return { kind: "invalid", reason: error instanceof Error ? error.message : "invalid JSON" }
  }

  const decoded = PromptInfoSchema.safeParse(parsed)
  if (!decoded.success) {
    return { kind: "invalid", reason: decoded.error.issues[0]?.message ?? "invalid prompt history shape" }
  }

  return { kind: "valid", value: decoded.data }
}

export function parsePromptHistoryText(text: string) {
  const entries: PromptInfo[] = []
  let dropped = 0

  for (const line of text.split("\n")) {
    const trimmed = line.trim()
    if (!trimmed) continue

    const parsed = decodePromptHistoryLine(trimmed)
    if (parsed.kind === "valid") {
      entries.push(parsed.value)
      continue
    }

    dropped++
  }

  return {
    entries: entries.slice(-MAX_HISTORY_ENTRIES),
    dropped,
  }
}

async function loadPromptHistory(historyPath: string): Promise<PromptHistoryLoadState> {
  try {
    const text = await Filesystem.readText(historyPath)
    const parsed = parsePromptHistoryText(text)
    return { kind: "loaded", ...parsed }
  } catch (error) {
    if (isMissingHistoryError(error)) return { kind: "missing" }
    return { kind: "error", error }
  }
}

async function writePromptHistory(historyPath: string, entries: PromptInfo[], operation: "append" | "repair") {
  const content = entries.length > 0 ? entries.map((line) => JSON.stringify(line)).join("\n") + "\n" : ""
  const attempts = operation === "repair" ? HISTORY_REPAIR_ATTEMPTS : HISTORY_WRITE_ATTEMPTS
  let lastError: unknown

  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      await writeFile(historyPath, content)
      return { kind: "written" } as const
    } catch (error) {
      lastError = error
      if (attempt < attempts) {
        await new Promise((resolve) => setTimeout(resolve, HISTORY_WRITE_RETRY_DELAY_MS * attempt))
      }
    }
  }

  return { kind: "failed", operation, error: lastError } as const
}

async function appendPromptHistory(historyPath: string, entry: PromptInfo): Promise<PromptHistoryWriteState> {
  let lastError: unknown

  for (let attempt = 1; attempt <= HISTORY_WRITE_ATTEMPTS; attempt++) {
    try {
      await appendFile(historyPath, JSON.stringify(entry) + "\n")
      return { kind: "written" }
    } catch (error) {
      lastError = error
      if (attempt < HISTORY_WRITE_ATTEMPTS) {
        await new Promise((resolve) => setTimeout(resolve, HISTORY_WRITE_RETRY_DELAY_MS * attempt))
      }
    }
  }

  return { kind: "failed", operation: "append", error: lastError }
}

export const { use: usePromptHistory, provider: PromptHistoryProvider } = createSimpleContext({
  name: "PromptHistory",
  init: () => {
    const historyPath = path.join(Global.Path.state, "prompt-history.jsonl")
    onMount(async () => {
      const loaded = await loadPromptHistory(historyPath)

      if (loaded.kind === "missing") return

      if (loaded.kind === "error") {
        console.error("[prompt-history] failed to load history", {
          historyPath,
          error: loaded.error,
        })
        return
      }

      setStore("history", loaded.entries)

      if (loaded.dropped > 0) {
        console.warn("[prompt-history] repaired invalid history entries", {
          historyPath,
          dropped: loaded.dropped,
          kept: loaded.entries.length,
          maxEntries: MAX_HISTORY_ENTRIES,
        })
        const repair = await writePromptHistory(historyPath, loaded.entries, "repair")
        if (repair.kind === "failed") {
          console.error("[prompt-history] failed to rewrite repaired history", {
            historyPath,
            error: repair.error,
          })
        }
      }
    })

    const [store, setStore] = createStore({
      index: 0,
      history: [] as PromptInfo[],
    })

    return {
      move(direction: 1 | -1, input: string): PromptHistoryMoveResult {
        if (!store.history.length) return { kind: "blocked" }
        const current = store.history.at(store.index)
        if (!current) return { kind: "blocked" }
        if (current.input !== input && input.length) return { kind: "blocked" }

        const next = store.index + direction
        if (Math.abs(next) > store.history.length) return { kind: "blocked" }
        if (next > 0) return { kind: "blocked" }

        setStore(
          produce((draft) => {
            draft.index = next
          }),
        )

        if (next === 0) return { kind: "cleared", prompt: { input: "", parts: [] } }

        const prompt = store.history.at(next)
        if (!prompt) return { kind: "blocked" }
        return { kind: "selected", prompt }
      },
      append(item: PromptInfo) {
        const entry = structuredClone(unwrap(item))
        let trimmed = false
        setStore(
          produce((draft) => {
            draft.history.push(entry)
            if (draft.history.length > MAX_HISTORY_ENTRIES) {
              draft.history = draft.history.slice(-MAX_HISTORY_ENTRIES)
              trimmed = true
            }
            draft.index = 0
          }),
        )

        if (trimmed) {
          void writePromptHistory(historyPath, store.history, "repair").then((result) => {
            if (result.kind === "failed") {
              console.error("[prompt-history] failed to rewrite trimmed history", {
                historyPath,
                error: result.error,
              })
            }
          })
          return
        }

        void appendPromptHistory(historyPath, entry).then((result) => {
          if (result.kind === "failed") {
            console.error("[prompt-history] failed to append history entry", {
              historyPath,
              error: result.error,
            })
          }
        })
      },
    }
  },
})
