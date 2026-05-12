import { describe, expect, test } from "bun:test"
import path from "path"
import { Effect, Layer } from "effect"
import { HttpClient, HttpClientResponse } from "effect/unstable/http"
import { Agent } from "../../src/agent/agent"
import { Truncate } from "@/tool/truncate"
import { WebFetchTool } from "../../src/tool/webfetch"
import { SessionID, MessageID } from "../../src/session/schema"
import { provideInstance } from "../fixture/fixture"

const projectRoot = path.join(import.meta.dir, "../..")

const ctx = {
  sessionID: SessionID.make("ses_test"),
  messageID: MessageID.make("message"),
  callID: "",
  agent: "build",
  abort: AbortSignal.any([]),
  messages: [],
  metadata: () => Effect.void,
  ask: () => Effect.void,
}

const mockHttpClient = HttpClient.make((request) =>
  Effect.sync(() => {
    const pathname = new URL(request.url).pathname

    switch (pathname) {
      case "/image.png":
        return HttpClientResponse.fromWeb(
          request,
          new Response(new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]), {
            status: 200,
            headers: { "content-type": "IMAGE/PNG; charset=binary" },
          }),
        )
      case "/image.svg":
        return HttpClientResponse.fromWeb(
          request,
          new Response('<svg xmlns="http://www.w3.org/2000/svg"><text>hello</text></svg>', {
            status: 200,
            headers: { "content-type": "image/svg+xml; charset=UTF-8" },
          }),
        )
      case "/file.txt":
        return HttpClientResponse.fromWeb(
          request,
          new Response("hello from webfetch", {
            status: 200,
            headers: { "content-type": "text/plain; charset=utf-8" },
          }),
        )
      default:
        return HttpClientResponse.fromWeb(
          request,
          new Response(`unexpected request path: ${pathname}`, {
            status: 404,
            headers: { "content-type": "text/plain; charset=utf-8" },
          }),
        )
    }
  }),
)

describe("tool.webfetch", () => {
  test("keeps text responses as text output", async () => {
    const result = await Effect.runPromise(
      provideInstance(projectRoot)(
        WebFetchTool.pipe(
          Effect.flatMap((info) => info.init()),
          Effect.flatMap((tool) => tool.execute({ url: "https://example.test/file.txt", format: "text" }, ctx)),
          Effect.provide(Layer.mergeAll(Truncate.defaultLayer, Agent.defaultLayer, Layer.succeed(HttpClient.HttpClient, mockHttpClient))),
        ),
      ),
    )
    expect(result.output).toBe("hello from webfetch")
    expect(result.attachments).toBeUndefined()
  })
})
