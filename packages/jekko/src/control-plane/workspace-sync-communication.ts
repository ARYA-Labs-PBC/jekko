import { Effect, Option, Stream } from "effect"
import { HttpClientRequest } from "effect/unstable/http"
import { route, decodeSSEData, SyncHttpError } from "./workspace-shared"

export type SyncCommunicationDeps = {
  http: { execute: (request: Parameters<typeof HttpClientRequest.post>[1]) => Effect.Effect<any> }
}

export function createWorkspaceSyncCommunication(deps: SyncCommunicationDeps) {
  const connectSSE = Effect.fn("Workspace.connectSSE")(function* (
    url: URL | string,
    headers: HeadersInit | undefined,
  ) {
    const response = yield* deps.http.execute(
      HttpClientRequest.get(route(url, "/global/event"), {
        headers: new Headers(headers),
        accept: "text/event-stream",
      }),
    )
    if (response.status < 200 || response.status >= 300) {
      return yield* new SyncHttpError({
        message: `Workspace sync HTTP failure: ${response.status}`,
        status: response.status,
      })
    }
    return response.stream
  })

  const parseSSE = Effect.fn("Workspace.parseSSE")(function* (
    stream: Stream.Stream<Uint8Array, unknown>,
    onEvent: (event: unknown) => Effect.Effect<void>,
  ) {
    yield* stream.pipe(
      Stream.decodeText(),
      Stream.splitLines,
      Stream.mapAccum(
        () => ({ data: [] as string[], id: undefined as string | undefined, retry: 1000 }),
        (state, line) => {
          if (line === "") {
            if (!state.data.length) return [state, []]
            return [{ ...state, data: [] }, [{ data: state.data.join("\n"), id: state.id, retry: state.retry }]]
          }

          const index = line.indexOf(":")
          const field = index === -1 ? line : line.slice(0, index)
          const value = index === -1 ? "" : line.slice(index + (line[index + 1] === " " ? 2 : 1))

          if (field === "data") return [{ ...state, data: [...state.data, value] }, []]
          if (field === "id") return [{ ...state, id: value }, []]
          if (field === "retry") {
            const retry = Number.parseInt(value, 10)
            return [Number.isNaN(retry) ? state : { ...state, retry }, []]
          }
          return [state, []]
        },
        {
          onHalt: (state) =>
            state.data.length ? [{ data: state.data.join("\n"), id: state.id, retry: state.retry }] : [],
        },
      ),
      Stream.map((event) => {
        const parsed = decodeSSEData(event.data)
        if (Option.isSome(parsed)) return parsed.value
        return {
          type: "sse.message",
          properties: {
            data: event.data,
            id: event.id || undefined,
            retry: event.retry,
          },
        }
      }),
      Stream.runForEach(onEvent),
    )
  })

  return { connectSSE, parseSSE }
}
