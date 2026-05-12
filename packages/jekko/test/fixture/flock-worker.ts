import fs from "fs/promises"
import { Flock } from "@jekko-ai/core/util/flock"

type Msg = {
  key: string
  dir: string
  maxAgeMs?: number
  timeoutMs?: number
  baseDelayMs?: number
  maxDelayMs?: number
  holdMs?: number
  ready?: string
  active?: string
  done?: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function isMsg(value: unknown): value is Msg {
  if (!isRecord(value)) return false
  if (typeof value.key !== "string" || typeof value.dir !== "string") return false
  for (const field of ["maxAgeMs", "timeoutMs", "baseDelayMs", "maxDelayMs", "holdMs"] as const) {
    if (field in value && typeof value[field] !== "number") return false
  }
  for (const field of ["ready", "active", "done"] as const) {
    if (field in value && typeof value[field] !== "string") return false
  }
  return true
}

function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, ms)
  })
}

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
  const msg = input()

  await Flock.withLock(msg.key, () => job(msg), {
    dir: msg.dir,
    maxAgeMs: msg.maxAgeMs,
    timeoutMs: msg.timeoutMs,
    baseDelayMs: msg.baseDelayMs,
    maxDelayMs: msg.maxDelayMs,
  })
}

await main()
