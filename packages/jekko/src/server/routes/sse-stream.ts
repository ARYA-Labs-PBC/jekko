import type { Context } from "hono"
import { streamSSE } from "hono/streaming"
import * as Log from "@jekko-ai/core/util/log"
import { AsyncQueue } from "@/util/queue"

const log = Log.create({ service: "server" })

export type StreamSseOptions = {
  connectedMessage: string
  disconnectedMessage: string
  initialEvent: () => string
  heartbeatEvent: () => string
  subscribe: (q: AsyncQueue<string | null>, stop: () => void) => () => void
}

export async function streamQueuedSSE(c: Context, options: StreamSseOptions) {
  return streamSSE(c, async (stream) => {
    const q = new AsyncQueue<string | null>()
    let done = false

    log.info(options.connectedMessage)

    q.push(options.initialEvent())

    // Send heartbeat every 10s to prevent stalled proxy streams.
    const heartbeat = setInterval(() => {
      q.push(options.heartbeatEvent())
    }, 10_000)

    const stop = () => {
      if (done) return
      done = true
      clearInterval(heartbeat)
      unsub()
      q.push(null)
      log.info(options.disconnectedMessage)
    }

    const unsub = options.subscribe(q, stop)

    stream.onAbort(stop)

    try {
      for await (const data of q) {
        if (data === null) return
        await stream.writeSSE({ data })
      }
    } finally {
      stop()
    }
  })
}
