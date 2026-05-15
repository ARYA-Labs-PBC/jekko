import { describe, expect, test } from "bun:test"
import type { LanguageModelV3, LanguageModelV3CallOptions, LanguageModelV3Content, LanguageModelV3StreamPart } from "@ai-sdk/provider"
import {
  createJnoccioMetadataExtractor,
  jnoccioMetadataFromHeaders,
  wrapJnoccioLanguageModel,
} from "../../src/provider/jnoccio-route-metadata"

async function collectStream<T>(stream: ReadableStream<T>): Promise<T[]> {
  const reader = stream.getReader()
  const parts: T[] = []
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    parts.push(value)
  }
  return parts
}

describe("jnoccio route metadata", () => {
  test("extracts full route metadata from OpenAI-compatible response bodies", async () => {
    const extractor = createJnoccioMetadataExtractor()
    const metadata = await extractor.extractMetadata({
      parsedBody: {
        id: "chatcmpl-123",
        jnoccio: {
          request_id: "request-body-123",
          provider: "jnoccio",
          model: "jnoccio/jnoccio-fusion",
          route_mode: "fusion",
          prompt_hash: "prompt-hash",
          context_hash: "context-hash",
          receipts_hash: "receipts-hash",
          model_decisions_hash: "decisions-hash",
          token_usage: { prompt_tokens: 12, completion_tokens: 34, total_tokens: 46 },
          model_decisions: [
            {
              model_id: "primary-1",
              configured_score: 1,
              selection_score: 0.91,
              latency_ms: 1234,
              status: "ok",
              selected: true,
              token_usage: { prompt_tokens: 12, completion_tokens: 34, total_tokens: 46 },
            },
          ],
        },
      },
    })

    expect(metadata).toMatchObject({
      jnoccio: {
        request_id: "request-body-123",
        provider: "jnoccio",
        model: "jnoccio/jnoccio-fusion",
        route_mode: "fusion",
        prompt_hash: "prompt-hash",
        context_hash: "context-hash",
        receipts_hash: "receipts-hash",
        model_decisions_hash: "decisions-hash",
        token_usage: { prompt_tokens: 12, completion_tokens: 34, total_tokens: 46 },
        model_decisions: [{ model_id: "primary-1", selected: true }],
      },
    })
  })

  test("parses response headers into route metadata", () => {
    const metadata = jnoccioMetadataFromHeaders(
      new Headers({
        "x-jnoccio-request-id": "request-123",
        "x-jnoccio-route-mode": "daemon",
        "x-jnoccio-sampled": "true",
        "x-jnoccio-complexity-tier": "high",
        "x-jnoccio-primary-model-id": "primary-1",
        "x-jnoccio-backup-model-ids": JSON.stringify(["backup-1", "backup-2"]),
        "x-jnoccio-fusion-model-id": "fusion-1",
        "x-jnoccio-winner-model-id": "winner-1",
        "x-jnoccio-confidence": "0.875",
        "x-jnoccio-model-decisions-hash": "hash-123",
      }),
    )

    expect(metadata).toEqual({
      request_id: "request-123",
      route_mode: "daemon",
      sampled: true,
      complexity_tier: "high",
      primary_model_id: "primary-1",
      backup_model_ids: ["backup-1", "backup-2"],
      fusion_model_id: "fusion-1",
      winner_model_id: "winner-1",
      confidence: 0.875,
      model_decisions_hash: "hash-123",
    })
  })

  test("drops malformed header values without throwing", () => {
    const metadata = jnoccioMetadataFromHeaders(
      new Headers({
        "x-jnoccio-backup-model-ids": JSON.stringify(["backup-1", "", 2, "backup-2"]),
        "x-jnoccio-sampled": "maybe",
        "x-jnoccio-confidence": "not-a-number",
      }),
    )

    expect(metadata).toEqual({
      backup_model_ids: ["backup-1", "backup-2"],
    })
  })

  test("ignores malformed body payloads", async () => {
    const extractor = createJnoccioMetadataExtractor()
    const metadata = await extractor.extractMetadata({
      parsedBody: {
        jnoccio: {
          request_id: 123,
          sampled: "yes",
          token_usage: {
            prompt_tokens: "bad",
            completion_tokens: "bad",
            total_tokens: "bad",
          },
          model_decisions: [null, 1, "bad"],
        },
      },
    })

    expect(metadata).toBeUndefined()
  })

  test("merges jnoccio metadata into doGenerate results", async () => {
    const language = wrapJnoccioLanguageModel({
      specificationVersion: "v3",
      modelId: "jnoccio-fusion",
      provider: "jnoccio",
      doGenerate: async () => ({
        content: [{ type: "text", text: "ok" }] as LanguageModelV3Content[],
        finishReason: { unified: "stop", raw: "stop" },
        usage: {
          inputTokens: { total: 1, noCache: 1, cacheRead: 0, cacheWrite: undefined },
          outputTokens: { total: 2, text: 2, reasoning: undefined },
          raw: { prompt_tokens: 1, completion_tokens: 2, total_tokens: 3 },
        },
        providerMetadata: {
          copilot: { reasoningOpaque: "opaque" },
          jnoccio: {
            model_decisions: [{ model_id: "primary-1", selected: true }],
            token_usage: { prompt_tokens: 1, completion_tokens: 2, total_tokens: 3 },
          },
        },
        request: { body: "{}" },
        response: {
          headers: new Headers({
            "x-jnoccio-request-id": "request-123",
            "x-jnoccio-route-mode": "daemon",
            "x-jnoccio-primary-model-id": "primary-1",
            "x-jnoccio-backup-model-ids": JSON.stringify(["backup-1"]),
            "x-jnoccio-fusion-model-id": "fusion-1",
            "x-jnoccio-winner-model-id": "winner-1",
            "x-jnoccio-confidence": "0.875",
            "x-jnoccio-model-decisions-hash": "hash-456",
          }),
        },
        warnings: [],
      }),
      doStream: async () => {
        throw new Error("unexpected stream call")
      },
    } as unknown as LanguageModelV3)

    const result = await language.doGenerate({ prompt: [] } as LanguageModelV3CallOptions)
    expect(result.providerMetadata).toMatchObject({
      copilot: { reasoningOpaque: "opaque" },
      jnoccio: {
        request_id: "request-123",
        route_mode: "daemon",
        primary_model_id: "primary-1",
        backup_model_ids: ["backup-1"],
        fusion_model_id: "fusion-1",
        winner_model_id: "winner-1",
        confidence: 0.875,
        model_decisions_hash: "hash-456",
        model_decisions: [{ model_id: "primary-1", selected: true }],
        token_usage: { prompt_tokens: 1, completion_tokens: 2, total_tokens: 3 },
      },
    })
  })

  test("merges jnoccio metadata into streamed finish events", async () => {
    const language = wrapJnoccioLanguageModel({
      specificationVersion: "v3",
      modelId: "jnoccio-fusion",
      provider: "jnoccio",
      doGenerate: async () => {
        throw new Error("unexpected generate call")
      },
      doStream: async () => ({
        stream: new ReadableStream<LanguageModelV3StreamPart>({
          start(controller) {
            controller.enqueue({
              type: "finish",
              finishReason: { unified: "stop", raw: "stop" },
              usage: {
                inputTokens: { total: 1, noCache: 1, cacheRead: 0, cacheWrite: undefined },
                outputTokens: { total: 2, text: 2, reasoning: undefined },
                raw: { prompt_tokens: 1, completion_tokens: 2, total_tokens: 3 },
              },
              providerMetadata: {
                copilot: { reasoningOpaque: "opaque" },
                jnoccio: {
                  model_decisions: [{ model_id: "backup-1", selected: false }],
                },
              },
            } as unknown as LanguageModelV3StreamPart)
            controller.close()
          },
        }),
        request: { body: "{}" },
        response: {
          headers: new Headers({
            "x-jnoccio-request-id": "request-456",
            "x-jnoccio-route-mode": "daemon",
            "x-jnoccio-primary-model-id": "primary-2",
            "x-jnoccio-backup-model-ids": JSON.stringify(["backup-2"]),
            "x-jnoccio-fusion-model-id": "fusion-2",
            "x-jnoccio-winner-model-id": "winner-2",
            "x-jnoccio-confidence": "0.5",
            "x-jnoccio-model-decisions-hash": "hash-789",
          }),
        },
      }),
    } as unknown as LanguageModelV3)

    const { stream } = await language.doStream({ prompt: [] } as LanguageModelV3CallOptions)
    const parts = await collectStream(stream)
    const finish = parts.find((part) => part.type === "finish") as Extract<
      (typeof parts)[number],
      { type: "finish" }
    >

    expect(finish.providerMetadata).toMatchObject({
      copilot: { reasoningOpaque: "opaque" },
      jnoccio: {
        request_id: "request-456",
        route_mode: "daemon",
        primary_model_id: "primary-2",
        backup_model_ids: ["backup-2"],
        fusion_model_id: "fusion-2",
        winner_model_id: "winner-2",
        confidence: 0.5,
        model_decisions_hash: "hash-789",
        model_decisions: [{ model_id: "backup-1", selected: false }],
      },
    })
  })
})
