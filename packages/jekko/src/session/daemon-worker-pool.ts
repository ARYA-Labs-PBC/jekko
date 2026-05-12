// Effect-based worker pool primitive. Slot count is resolved from
// `min(pool.size, hard_cap, jnoccio.spawn_batch_limit)` at construction. The
// daemon side hands the resolved size to the Rust runner (PR3) via env / CLI
// so both layers cooperate on the same cap.
//
// The pool itself is just a counted semaphore wrapper. Workers register their
// presence via `acquire`, do their work, and release via the disposable returned
// by `acquire`. Heartbeats are caller-driven — `recordHeartbeat` stamps the
// last_heartbeat_at column for the worker so the supervisor can reap dead
// slots.

import { Effect } from "effect"

export type PoolConfig = {
  /** Requested size. If undefined, defaults to 5. */
  requested?: number
  /** Hard cap on slots regardless of requested. Defaults to 20. */
  hardCap?: number
  /** jnoccio's `spawn_batch_limit`. If undefined, ignored. */
  jnoccioLimit?: number
}

export const DEFAULT_REQUESTED = 5
export const DEFAULT_HARD_CAP = 20

export function resolveSize(config: PoolConfig): number {
  const requested = Math.max(1, Math.floor(config.requested ?? DEFAULT_REQUESTED))
  const hardCap = Math.max(1, Math.floor(config.hardCap ?? DEFAULT_HARD_CAP))
  let size = Math.min(requested, hardCap)
  if (typeof config.jnoccioLimit === "number" && config.jnoccioLimit > 0) {
    size = Math.min(size, Math.floor(config.jnoccioLimit))
  }
  return Math.max(1, size)
}

export type PoolStatus = {
  size: number
  inFlight: number
  free: number
  totalAcquired: number
}

export type Pool = {
  acquire: () => Effect.Effect<PoolSlot, never, never>
  status: () => Effect.Effect<PoolStatus, never, never>
  size: number
}

export type PoolSlot = {
  release: () => Effect.Effect<void, never, never>
  ticket: number
}

/**
 * In-memory pool implementation. Uses a JS Promise queue for the wait path
 * and `Effect.promise` to surface it as an Effect — keeps the API surface
 * minimal without depending on a specific Effect version's `Effect.async`
 * shape.
 */
export function makePool(config: PoolConfig): Pool {
  const size = resolveSize(config)
  let inFlight = 0
  let totalAcquired = 0
  const waiters: Array<(slot: PoolSlot) => void> = []

  function nextSlot(): PoolSlot {
    totalAcquired += 1
    const ticket = totalAcquired
    return {
      ticket,
      release: () =>
        Effect.sync(() => {
          inFlight -= 1
          const next = waiters.shift()
          if (next) {
            inFlight += 1
            next(nextSlot())
          }
        }),
    }
  }

  function acquireSync(): Promise<PoolSlot> {
    if (inFlight < size) {
      inFlight += 1
      return Promise.resolve(nextSlot())
    }
    return new Promise<PoolSlot>((resolve) => {
      waiters.push((slot) => resolve(slot))
    })
  }

  return {
    size,
    acquire: () => Effect.promise(() => acquireSync()),
    status: () =>
      Effect.sync(() => ({
        size,
        inFlight,
        free: size - inFlight,
        totalAcquired,
      })),
  }
}

/**
 * Run `body` with an acquired slot; release happens in `Effect.ensuring` so
 * failures don't leak slots.
 */
export function withSlot<A, E, R>(pool: Pool, body: (slot: PoolSlot) => Effect.Effect<A, E, R>) {
  return Effect.gen(function* () {
    const slot = yield* pool.acquire()
    return yield* Effect.ensuring(body(slot), slot.release())
  })
}

export * as DaemonWorkerPool from "./daemon-worker-pool"
