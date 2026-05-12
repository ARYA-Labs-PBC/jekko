import { describe, expect, test } from "bun:test"
import path from "path"
import { Effect, Layer } from "effect"
import { HttpClient, HttpClientResponse } from "effect/unstable/http"
import { Agent } from "../../src/agent/agent"
import { Truncate } from "@/tool/truncate"
import { ResearchTool } from "../../src/tool/research"
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
    return HttpClientResponse.fromWeb(
      request,
      new Response(
        `data: ${JSON.stringify({
          result: {
            content: [{ type: "text", text: `fallback results for ${pathname}` }],
          },
        })}\n\n`,
        {
          status: 200,
          headers: { "content-type": "text/event-stream; charset=utf-8" },
        },
      ),
    )
  }),
)

describe("tool.research", () => {
  test("uses the advanced backend when available", async () => {
    const originalFetch = globalThis.fetch
    globalThis.fetch = (async () =>
      new Response(
        JSON.stringify({
          result: {
            structuredContent: {
              hits: [{ title: "Primary", url: "https://example.com", snippet: "evidence" }],
              evidence: [{ id: "cite-1", citation: "Primary" }],
              receipts: [{ provider: "jnoccio", query: "evidence query" }],
              warnings: ["partial coverage"],
            },
          },
        }),
        {
          status: 200,
          headers: { "content-type": "application/json" },
        },
      )) as typeof fetch

    try {
      const result = await Effect.runPromise(
        provideInstance(projectRoot)(
          ResearchTool.pipe(
            Effect.flatMap((info) => info.init()),
            Effect.flatMap((tool) =>
              tool.execute(
                {
                  query: "evidence query",
                  mode: "mixed",
                  objective: "cite primary sources",
                  maxParallel: 4,
                  timeoutSeconds: 5,
                },
                ctx,
              ),
            ),
            Effect.provide(Layer.mergeAll(Truncate.defaultLayer, Agent.defaultLayer, Layer.succeed(HttpClient.HttpClient, mockHttpClient))),
          ),
        ),
      )
      expect(result.metadata.backend).toBe("jnoccio")
      expect(result.output).toContain("Primary")
      expect(result.metadata.receipts).toHaveLength(1)
    } finally {
      globalThis.fetch = originalFetch
    }
  })

  test("falls back to web search when the advanced backend is unavailable", async () => {
    const originalFetch = globalThis.fetch
    globalThis.fetch = (async () => new Response("missing", { status: 404 })) as typeof fetch

    try {
      const result = await Effect.runPromise(
        provideInstance(projectRoot)(
          ResearchTool.pipe(
            Effect.flatMap((info) => info.init()),
            Effect.flatMap((tool) =>
              tool.execute(
                {
                  query: "fallback search query",
                  mode: "web",
                  maxParallel: 2,
                },
                ctx,
              ),
            ),
            Effect.provide(Layer.mergeAll(Truncate.defaultLayer, Agent.defaultLayer, Layer.succeed(HttpClient.HttpClient, mockHttpClient))),
          ),
        ),
      )
      expect(result.metadata.backend).toBe("exa")
      expect(result.output).toContain("fallback results")
    } finally {
      globalThis.fetch = originalFetch
    }
  })
})
