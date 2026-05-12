import type { ZyalMemory, ZyalMemoryStore, ZyalMemoryRedaction } from "@/agent-script/schema"
import { createHash } from "crypto"

/**
 * Governed agent memory manager for ZYAL v2.
 *
 * Manages typed memory stores with scoping, retention, write policies,
 * redaction, compression triggers, and provenance tracking.
 */

export type MemoryEntry = {
  readonly id: string
  readonly store: string
  readonly key: string
  readonly value: unknown
  readonly source?: { agent?: string; passID?: string; iteration?: number }
  readonly createdAt: number
  readonly hash?: string
}

export type MemoryStoreState = {
  readonly name: string
  readonly config: ZyalMemoryStore
  readonly entries: MemoryEntry[]
  readonly entryCount: number
  readonly needsCompression: boolean
}

/**
 * Initialize store states from memory config.
 */
export function initializeStores(memory: ZyalMemory | undefined): Map<string, MemoryStoreState> {
  const result = new Map<string, MemoryStoreState>()
  if (!memory?.stores) return result
  for (const [name, config] of Object.entries(memory.stores)) {
    result.set(name, {
      name,
      config,
      entries: [],
      entryCount: 0,
      needsCompression: false,
    })
  }
  return result
}

/**
 * Write an entry to a memory store, respecting write policy.
 */
export function writeEntry(
  store: MemoryStoreState,
  entry: { key: string; value: unknown; source?: MemoryEntry["source"] },
  provenance?: { track_source?: boolean; hash_chain?: boolean },
): { store: MemoryStoreState; written: boolean; reason?: string } {
  const existing = store.entries.findIndex((e) => e.key === entry.key)

  if (store.config.write_policy === "append_only" && existing >= 0) {
    return { store, written: false, reason: "append_only: key already exists" }
  }

  const newEntry: MemoryEntry = {
    id: `${store.name}:${entry.key}:${Date.now()}`,
    store: store.name,
    key: entry.key,
    value: entry.value,
    source: provenance?.track_source ? entry.source : undefined,
    createdAt: Date.now(),
    hash: provenance?.hash_chain ? hashEntry(entry.value) : undefined,
  }

  let entries: MemoryEntry[]
  if (store.config.write_policy === "overwrite" && existing >= 0) {
    entries = [...store.entries]
    entries[existing] = newEntry
  } else if (store.config.write_policy === "upsert" && existing >= 0) {
    entries = [...store.entries]
    entries[existing] = newEntry
  } else {
    entries = [...store.entries, newEntry]
  }

  // Check max_entries
  if (store.config.max_entries && entries.length > store.config.max_entries) {
    entries = entries.slice(entries.length - store.config.max_entries)
  }

  const needsCompression = checkCompressionTrigger(store.config, entries.length)

  return {
    store: { ...store, entries, entryCount: entries.length, needsCompression },
    written: true,
  }
}

/**
 * Read entries from a memory store.
 */
export function readEntries(store: MemoryStoreState, filter?: { key?: string }): MemoryEntry[] {
  if (filter?.key) {
    return store.entries.filter((e) => e.key === filter.key)
  }
  return store.entries
}

/**
 * Apply redaction patterns to a value.
 */
export function redactValue(value: string, redaction: ZyalMemoryRedaction | undefined): string {
  if (!redaction?.patterns?.length) return value
  let result = value
  for (const pattern of redaction.patterns) {
    const regex = patternToRegex(pattern)
    if (redaction.action === "mask") {
      result = result.replace(regex, "***REDACTED***")
    } else if (redaction.action === "remove") {
      result = result.replace(regex, "")
    } else if (redaction.action === "hash") {
      result = result.replace(regex, (match) => `[hash:${hashEntry(match).slice(0, 8)}]`)
    }
  }
  return result
}

/**
 * Get entries that should be injected at session start.
 */
export function getInjectionEntries(stores: Map<string, MemoryStoreState>): MemoryEntry[] {
  const result: MemoryEntry[] = []
  for (const store of stores.values()) {
    if (store.config.read_policy === "inject_at_start") {
      result.push(...store.entries)
    }
  }
  return result
}

/**
 * Filter stores by scope.
 */
export function getStoresByScope(
  stores: Map<string, MemoryStoreState>,
  scope: ZyalMemoryStore["scope"],
): MemoryStoreState[] {
  return Array.from(stores.values()).filter((s) => s.config.scope === scope)
}

/**
 * Check if store should be retained based on retention policy.
 */
export function shouldRetain(store: MemoryStoreState, event: "promotion" | "archive" | "session_end"): boolean {
  switch (store.config.retention) {
    case "permanent":
      return true
    case "session":
      return event !== "session_end"
    case "until_promotion":
      return event !== "promotion"
    case "until_archive":
      return event !== "archive"
    default:
      return true
  }
}

function checkCompressionTrigger(config: ZyalMemoryStore, count: number): boolean {
  if (!config.compression) return false
  const match = config.compression.match(/summarize_after_(\d+)/)
  if (match) return count >= parseInt(match[1], 10)
  return false
}

function hashEntry(value: unknown): string {
  return createHash("sha256").update(JSON.stringify(value)).digest("hex")
}

function patternToRegex(pattern: string): RegExp {
  // Support glob-like patterns: * = .+
  const escaped = pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\\\*/g, ".+")
  return new RegExp(escaped, "g")
}
