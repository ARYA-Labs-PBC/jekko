import fs from "fs/promises"
import { Flock } from "@jekko-ai/core/util/flock"
import { sleep } from "./flock-shared"
import type { Msg } from "./flock-shared"

function input() {
  const raw = process.argv[2]
  if (!raw) {
    throw new Error("Missing flock worker input")
  }

  const parsed = JSON.parse(raw)
  if (!isMsg(parsed)) {
    throw new Error("Invalid flock worker input")
  }
  return parsed
}

function isMsg(value: unknown): value is Msg {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false
  const record = value as Record<string, unknown>
  if (typeof record.key !== "string" || typeof record.dir !== "string") return false
  return (
    (record.maxAgeMs === undefined || typeof record.maxAgeMs === "number") &&
    (record.timeoutMs === undefined || typeof record.timeoutMs === "number") &&
    (record.baseDelayMs === undefined || typeof record.baseDelayMs === "number") &&
    (record.maxDelayMs === undefined || typeof record.maxDelayMs === "number") &&
    (record.holdMs === undefined || typeof record.holdMs === "number") &&
    (record.ready === undefined || typeof record.ready === "string") &&
    (record.active === undefined || typeof record.active === "string") &&
    (record.done === undefined || typeof record.done === "string")
  )
}

async function job(input: Msg) {
  if (input.ready) {
    await fs.writeFile(input.ready, String(process.pid))
  }

  if (input.active) {
    await fs.writeFile(input.active, String(process.pid), { flag: "wx" })
  }

  try {
    if (input.holdMs && input.holdMs > 0) {
      await sleep(input.holdMs)
    }

    if (input.done) {
      await fs.appendFile(input.done, "1\n")
    }
  } finally {
    if (input.active) {
      await fs.rm(input.active, { force: true })
    }
  }
}

async function main() {
  const msg: Msg = input()
  await Flock.withLock(msg.key, () => job(msg), {
    dir: msg.dir,
    maxAgeMs: msg.maxAgeMs,
    timeoutMs: msg.timeoutMs,
    baseDelayMs: msg.baseDelayMs,
    maxDelayMs: msg.maxDelayMs,
  })
}

await main().catch((err) => {
  const text = err instanceof Error ? (err.stack ?? err.message) : String(err)
  process.stderr.write(text)
  process.exitCode = 1
})
