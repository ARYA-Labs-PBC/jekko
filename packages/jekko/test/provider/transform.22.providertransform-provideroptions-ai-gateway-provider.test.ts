import { describe, expect, test } from "bun:test"
import { ProviderTransform } from "@/provider/transform"
import { ModelID, ProviderID } from "../../src/provider/schema"

describe("ProviderTransform.providerOptions - ai-gateway-provider", () => {
  const createModel = (overrides: Partial<any> = {}) =>
    ({
      id: "cloudflare-ai-gateway/openai/gpt-5.4",
      providerID: "cloudflare-ai-gateway",
      api: {
        id: "openai/gpt-5.4",
        url: "https://gateway.ai.cloudflare.com/v1/interop",
        npm: "ai-gateway-provider",
      },
      capabilities: {
        temperature: false,
        reasoning: true,
        attachment: true,
        toolcall: true,
        input: { text: true, audio: false, image: true, video: false, pdf: true },
        output: { text: true, audio: false, image: false, video: false, pdf: false },
        interleaved: false,
      },
      cost: { input: 1, output: 1, cache: { read: 0, write: 0 } },
      limit: { context: 1_000_000, output: 128_000 },
      status: "active",
      options: {},
      headers: {},
      release_date: "2026-03-05",
      ...overrides,
    }) as any

  test("routes options under openaiCompatible (the key @ai-sdk/openai-compatible reads)", () => {
    // Regression: previously fell back to providerID="cloudflare-ai-gateway",
    // which @ai-sdk/openai-compatible never reads, silently dropping reasoningEffort.
    const result = ProviderTransform.providerOptions(createModel(), { reasoningEffort: "high" })
    expect(result).toEqual({ openaiCompatible: { reasoningEffort: "high" } })
  })
})
