import fs from "fs/promises"
import { Effect } from "effect"
import { EffectFlock } from "@jekko-ai/core/util/effect-flock"
import { sleep, testLayer } from "./effect-flock-shared"
import type { Msg } from "./effect-flock-shared"

const msg: Msg = JSON.parse(process.argv[2])

async function job() {
  if (msg.ready) await fs.writeFile(msg.ready, String(process.pid))
  if (msg.active) await fs.writeFile(msg.active, String(process.pid), { flag: "wx" })

  try {
    if (msg.holdMs && msg.holdMs > 0) await sleep(msg.holdMs)
    if (msg.done) await fs.appendFile(msg.done, "1\n")
  } finally {
    if (msg.active) await fs.rm(msg.active, { force: true })
  }
}

await Effect.runPromise(
  Effect.gen(function* () {
    const flock = yield* EffectFlock.Service
    yield* flock.withLock(
      Effect.promise(() => job()),
      msg.key,
      msg.dir,
    )
}).pipe(Effect.provide(testLayer)),
).catch((err) => {
  const text = err instanceof Error ? (err.stack ?? err.message) : String(err)
  process.stderr.write(text)
  throw new Error(text)
})
