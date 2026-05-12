import { Effect } from "effect"
import * as Stream from "effect/Stream"

export function collectStreamOutput(stream: Stream.Stream<Uint8Array>, maxBytes?: number) {
  return Stream.runFold(
    stream,
    () => ({ chunks: [] as Uint8Array[], bytes: 0, truncated: false }),
    (acc, chunk) => {
      if (maxBytes === undefined || acc.bytes < maxBytes) {
        const remaining = maxBytes === undefined ? chunk.length : maxBytes - acc.bytes
        if (remaining > 0) {
          acc.chunks.push(remaining >= chunk.length ? chunk : chunk.slice(0, remaining))
        }
      }
      acc.bytes += chunk.length
      acc.truncated = acc.truncated || (maxBytes !== undefined && acc.bytes > maxBytes)
      return acc
    },
  ).pipe(Effect.map((x) => ({ buffer: Buffer.concat(x.chunks), truncated: x.truncated })))
}
