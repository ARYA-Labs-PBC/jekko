import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import { DaemonWorkerPool } from "../../src/session/daemon-worker-pool"

describe("DaemonWorkerPool.resolveSize", () => {
  test("defaults requested to 5 and hard cap to 20", () => {
    expect(DaemonWorkerPool.resolveSize({})).toBe(5)
  })

  test("hard cap clamps a large requested", () => {
    expect(DaemonWorkerPool.resolveSize({ requested: 99, hardCap: 8 })).toBe(8)
  })

  test("jnoccio limit clamps below the requested", () => {
    expect(DaemonWorkerPool.resolveSize({ requested: 12, hardCap: 20, jnoccioLimit: 5 })).toBe(5)
  })

  test("never returns below 1", () => {
    expect(DaemonWorkerPool.resolveSize({ requested: 0, hardCap: 0 })).toBe(1)
  })

  test("ignores non-positive jnoccio limit", () => {
    expect(DaemonWorkerPool.resolveSize({ requested: 4, jnoccioLimit: 0 })).toBe(4)
    expect(DaemonWorkerPool.resolveSize({ requested: 4, jnoccioLimit: -1 })).toBe(4)
  })
})

describe("DaemonWorkerPool.makePool", () => {
  test("acquire + release roundtrips through status", async () => {
    const pool = DaemonWorkerPool.makePool({ requested: 2 })
    const slot1 = await Effect.runPromise(pool.acquire())
    const slot2 = await Effect.runPromise(pool.acquire())
    const status = await Effect.runPromise(pool.status())
    expect(status.size).toBe(2)
    expect(status.inFlight).toBe(2)
    expect(status.free).toBe(0)
    await Effect.runPromise(slot1.release())
    await Effect.runPromise(slot2.release())
    const after = await Effect.runPromise(pool.status())
    expect(after.inFlight).toBe(0)
    expect(after.free).toBe(2)
  })

  test("over-capacity acquire waits until a release", async () => {
    const pool = DaemonWorkerPool.makePool({ requested: 1 })
    const slot1 = await Effect.runPromise(pool.acquire())
    let resolved = false
    const waiting = Effect.runPromise(pool.acquire()).then((slot) => {
      resolved = true
      return slot
    })
    // give the pending acquire a tick to register as waiter
    await new Promise((r) => setTimeout(r, 5))
    expect(resolved).toBe(false)
    await Effect.runPromise(slot1.release())
    const slot2 = await waiting
    expect(resolved).toBe(true)
    await Effect.runPromise(slot2.release())
  })

  test("withSlot releases automatically on success", async () => {
    const pool = DaemonWorkerPool.makePool({ requested: 1 })
    await Effect.runPromise(
      DaemonWorkerPool.withSlot(pool, () => Effect.sync(() => 42)),
    )
    const status = await Effect.runPromise(pool.status())
    expect(status.inFlight).toBe(0)
  })

  test("withSlot releases on failure", async () => {
    const pool = DaemonWorkerPool.makePool({ requested: 1 })
    const result = Effect.runPromiseExit(
      DaemonWorkerPool.withSlot(pool, () => Effect.fail("boom" as const)),
    )
    await result
    const status = await Effect.runPromise(pool.status())
    expect(status.inFlight).toBe(0)
  })

  test("ticket increments per acquire", async () => {
    const pool = DaemonWorkerPool.makePool({ requested: 2 })
    const slot1 = await Effect.runPromise(pool.acquire())
    const slot2 = await Effect.runPromise(pool.acquire())
    expect(slot2.ticket).toBe(slot1.ticket + 1)
    await Effect.runPromise(slot1.release())
    await Effect.runPromise(slot2.release())
  })
})
