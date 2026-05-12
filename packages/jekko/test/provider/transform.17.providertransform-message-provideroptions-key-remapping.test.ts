import { describe, expect, test } from "bun:test"
import { ProviderTransform } from "@/provider/transform"
import { ModelID, ProviderID } from "../../src/provider/schema"

describe("ProviderTransform.message - providerOptions key remapping", () => {
  const createModel = (providerID: string, npm: string) =>
    ({
      id: `${providerID}/test-model`,
      providerID,
      api: {
        id: "test-model",
        url: "https://api.test.com",
        npm,
      },
      name: "Test Model",
      capabilities: {
        temperature: true,
        reasoning: false,
        attachment: true,
        toolcall: true,
        input: { text: true, audio: false, image: true, video: false, pdf: true },
        output: { text: true, audio: false, image: false, video: false, pdf: false },
        interleaved: false,
      },
      cost: { input: 0.001, output: 0.002, cache: { read: 0.0001, write: 0.0002 } },
      limit: { context: 128000, output: 8192 },
      status: "active",
      options: {},
      headers: {},
    }) as any

  test("azure keeps 'azure' key and does not remap to 'openai'", () => {
    const model = createModel("azure", "@ai-sdk/azure")
    const msgs = [
      {
        role: "user",
        content: "Hello",
        providerOptions: {
          azure: { someOption: "value" },
        },
      },
    ] as any[]

    const result = ProviderTransform.message(msgs, model, {})

    expect(result[0].providerOptions?.azure).toEqual({ someOption: "value" })
    expect(result[0].providerOptions?.openai).toBeUndefined()
  })

  test("azure cognitive services remaps providerID to 'azure' key", () => {
    const model = createModel("azure-cognitive-services", "@ai-sdk/azure")
    const msgs = [
      {
        role: "user",
        content: [
          {
            type: "text",
            text: "Hello",
            providerOptions: {
              "azure-cognitive-services": { part: true },
            },
          },
        ],
        providerOptions: {
          "azure-cognitive-services": { someOption: "value" },
        },
      },
    ] as any[]

    const result = ProviderTransform.message(msgs, model, {}) as any[]
    const part = result[0].content[0] as any

    expect(result[0].providerOptions?.azure).toEqual({ someOption: "value" })
    expect(result[0].providerOptions?.["azure-cognitive-services"]).toBeUndefined()
    expect(part.providerOptions?.azure).toEqual({ part: true })
    expect(part.providerOptions?.["azure-cognitive-services"]).toBeUndefined()
  })

  test("copilot remaps providerID to 'copilot' key", () => {
    const model = createModel("github-copilot", "@ai-sdk/github-copilot")
    const msgs = [
      {
        role: "user",
        content: "Hello",
        providerOptions: {
          copilot: { someOption: "value" },
        },
      },
    ] as any[]

    const result = ProviderTransform.message(msgs, model, {})

    expect(result[0].providerOptions?.copilot).toEqual({ someOption: "value" })
    expect(result[0].providerOptions?.["github-copilot"]).toBeUndefined()
  })

  test("bedrock remaps providerID to 'bedrock' key", () => {
    const model = createModel("my-bedrock", "@ai-sdk/amazon-bedrock")
    const msgs = [
      {
        role: "user",
        content: "Hello",
        providerOptions: {
          "my-bedrock": { someOption: "value" },
        },
      },
    ] as any[]

    const result = ProviderTransform.message(msgs, model, {})

    expect(result[0].providerOptions?.bedrock).toEqual({ someOption: "value" })
    expect(result[0].providerOptions?.["my-bedrock"]).toBeUndefined()
  })
})
