import { describe, expect, test } from "bun:test"
import { ProviderTransform } from "@/provider/transform"

describe("ProviderTransform.message - tool call id scrubbing", () => {
  const createModel = () =>
    ({
      id: "mistral/test-model",
      providerID: "mistral",
      api: {
        id: "mistral",
        url: "https://api.test.com",
        npm: "@ai-sdk/openai",
      },
      name: "Test Model",
      capabilities: {
        temperature: true,
        reasoning: false,
        attachment: true,
        toolcall: true,
        input: { text: true, audio: true, image: true, video: true, pdf: true },
        output: { text: true, audio: true, image: true, video: true, pdf: true },
        interleaved: false,
      },
      cost: { input: 0.001, output: 0.002, cache: { read: 0.0001, write: 0.0002 } },
      limit: { context: 128000, output: 8192 },
      status: "active",
      options: {},
      headers: {},
    }) as any

  test("scrubs assistant and tool result ids to the mistral format", () => {
    const model = createModel()
    const msgs = [
      {
        role: "assistant",
        content: [
          { type: "tool-call", toolCallId: "abc-123", toolName: "lookup", input: { q: "one" } },
          { type: "tool-result", toolCallId: "abc-123", output: { type: "text", value: "done" } },
        ],
      },
      {
        role: "tool",
        content: [{ type: "tool-result", toolCallId: "tool.id", output: { type: "text", value: "done" } }],
      },
    ] as any[]

    const result = ProviderTransform.message(msgs, model, {}) as any[]

    expect(result[0].content[0].toolCallId).toBe("abc123000")
    expect(result[0].content[1].toolCallId).toBe("abc123000")
    expect(result[1].content[0].toolCallId).toBe("toolid000")
  })
})
