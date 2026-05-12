import { describe, test, expect } from "bun:test"
import {
  initializeStores,
  writeEntry,
  readEntries,
  redactValue,
  getInjectionEntries,
  getStoresByScope,
  shouldRetain,
} from "../../src/session/daemon-memory"
import type { ZyalMemory } from "../../src/agent-script/schema"

const testMemory: ZyalMemory = {
  stores: {
    task_context: {
      scope: "task",
      retention: "until_promotion",
      max_entries: 3,
      write_policy: "append_only",
      read_policy: "inject_at_start",
    },
    lessons: {
      scope: "global",
      retention: "permanent",
      write_policy: "upsert",
      read_policy: "on_demand",
      searchable: true,
    },
    scratch: {
      scope: "run",
      retention: "session",
      write_policy: "overwrite",
      read_policy: "on_demand",
      compression: "summarize_after_5",
    },
  },
  redaction: {
    patterns: ["example-secret-*", "example-aws-key-*"],
    action: "mask",
  },
  provenance: {
    track_source: true,
    hash_chain: true,
  },
}

describe("daemon memory", () => {
  test("initializeStores creates empty stores from config", () => {
    const stores = initializeStores(testMemory)
    expect(stores.size).toBe(3)
    expect(stores.get("task_context")!.entryCount).toBe(0)
    expect(stores.get("lessons")!.config.scope).toBe("global")
  })

  test("initializeStores returns empty map for undefined config", () => {
    const stores = initializeStores(undefined)
    expect(stores.size).toBe(0)
  })

  test("writeEntry appends to append_only store", () => {
    const stores = initializeStores(testMemory)
    const store = stores.get("task_context")!
    const result = writeEntry(store, { key: "objective", value: "fix tests" }, testMemory.provenance)
    expect(result.written).toBe(true)
    expect(result.store.entryCount).toBe(1)
    expect(result.store.entries[0].key).toBe("objective")
    expect(result.store.entries[0].source).toBeUndefined() // no source provided
    expect(result.store.entries[0].hash).toBeDefined() // hash_chain enabled
  })

  test("writeEntry rejects duplicate key in append_only store", () => {
    const stores = initializeStores(testMemory)
    let store = stores.get("task_context")!
    const r1 = writeEntry(store, { key: "obj", value: "v1" })
    store = r1.store
    const r2 = writeEntry(store, { key: "obj", value: "v2" })
    expect(r2.written).toBe(false)
    expect(r2.reason).toContain("append_only")
  })

  test("writeEntry upserts in upsert store", () => {
    const stores = initializeStores(testMemory)
    let store = stores.get("lessons")!
    store = writeEntry(store, { key: "lesson1", value: "initial" }).store
    store = writeEntry(store, { key: "lesson1", value: "new" }).store
    expect(store.entryCount).toBe(1)
    expect(store.entries[0].value).toBe("new")
  })

  test("writeEntry respects max_entries", () => {
    const stores = initializeStores(testMemory)
    let store = stores.get("task_context")!
    // max_entries is 3, append_only won't conflict since keys differ
    store = writeEntry(store, { key: "a", value: 1 }).store
    store = writeEntry(store, { key: "b", value: 2 }).store
    store = writeEntry(store, { key: "c", value: 3 }).store
    store = writeEntry(store, { key: "d", value: 4 }).store
    // Should truncate to last 3
    expect(store.entryCount).toBe(3)
    expect(store.entries[0].key).toBe("b")
  })

   test("writeEntry overwrites in overwrite store", () => {
     const stores = initializeStores(testMemory)
     let store = stores.get("scratch")!
     store = writeEntry(store, { key: "test-key", value: "initial" }).store
     store = writeEntry(store, { key: "test-key", value: "new" }).store
     expect(store.entryCount).toBe(1)
     expect(store.entries[0].value).toBe("new")
   })

  test("readEntries returns all entries", () => {
    const stores = initializeStores(testMemory)
    let store = stores.get("task_context")!
    store = writeEntry(store, { key: "a", value: 1 }).store
    store = writeEntry(store, { key: "b", value: 2 }).store
    expect(readEntries(store)).toHaveLength(2)
  })

  test("readEntries filters by key", () => {
    const stores = initializeStores(testMemory)
    let store = stores.get("task_context")!
    store = writeEntry(store, { key: "a", value: 1 }).store
    store = writeEntry(store, { key: "b", value: 2 }).store
    expect(readEntries(store, { key: "a" })).toHaveLength(1)
    expect(readEntries(store, { key: "c" })).toHaveLength(0)
  })

  test("redactValue masks matching patterns", () => {
    const result = redactValue(
      "my key is example-secret-abc123xyz and also example-aws-key-something",
      testMemory.redaction,
    )
    expect(result).toContain("***REDACTED***")
    expect(result).not.toContain("example-secret-abc123xyz")
    expect(result).not.toContain("example-aws-key-something")
  })

  test("redactValue returns original when no redaction config", () => {
    expect(redactValue("example-secret", undefined)).toBe("example-secret")
  })

  test("getInjectionEntries returns inject_at_start entries", () => {
    const stores = initializeStores(testMemory)
    let taskStore = stores.get("task_context")!
    taskStore = writeEntry(taskStore, { key: "obj", value: "test" }).store
    stores.set("task_context", taskStore)
    const entries = getInjectionEntries(stores)
    expect(entries).toHaveLength(1)
    expect(entries[0].key).toBe("obj")
  })

  test("getStoresByScope filters correctly", () => {
    const stores = initializeStores(testMemory)
    expect(getStoresByScope(stores, "global")).toHaveLength(1)
    expect(getStoresByScope(stores, "task")).toHaveLength(1)
    expect(getStoresByScope(stores, "run")).toHaveLength(1)
    expect(getStoresByScope(stores, "agent")).toHaveLength(0)
  })

  test("shouldRetain respects retention policy", () => {
    const stores = initializeStores(testMemory)
    const permanent = stores.get("lessons")!
    const session = stores.get("scratch")!
    const untilPromo = stores.get("task_context")!

    expect(shouldRetain(permanent, "promotion")).toBe(true)
    expect(shouldRetain(permanent, "session_end")).toBe(true)
    expect(shouldRetain(session, "session_end")).toBe(false)
    expect(shouldRetain(untilPromo, "promotion")).toBe(false)
    expect(shouldRetain(untilPromo, "archive")).toBe(true)
  })

  test("compression trigger fires at threshold", () => {
    const stores = initializeStores(testMemory)
    let store = stores.get("scratch")!
    // summarize_after_5, add 5 entries
    for (let i = 0; i < 5; i++) {
      store = writeEntry(store, { key: `k${i}`, value: i }).store
    }
    expect(store.needsCompression).toBe(true)
  })

  test("provenance tracks source when enabled", () => {
    const stores = initializeStores(testMemory)
    const store = stores.get("lessons")!
    const result = writeEntry(
      store,
      { key: "lesson", value: "test", source: { agent: "plan", passID: "p1" } },
      testMemory.provenance,
    )
    expect(result.store.entries[0].source).toEqual({ agent: "plan", passID: "p1" })
  })
})
