import { expect, test } from "bun:test"
import { defaultModel } from "../../src/acp/default-model"

test("defaultModel falls back to auto routing when nothing is configured", async () => {
  const result = await defaultModel({
    sdk: {
      config: {
        get: async () => ({ data: {} }),
        providers: async () => ({ data: { providers: [] } }),
      },
    } as any,
  })

  expect(result).toEqual({ providerID: "auto", modelID: "smart" })
})

test("defaultModel honors an explicit configured model", async () => {
  const result = await defaultModel({
    sdk: {
      config: {
        get: async () => ({ data: { model: "test/test-model" } }),
        providers: async () => ({ data: { providers: [] } }),
      },
    } as any,
  })

  expect(result).toEqual({ providerID: "test", modelID: "test-model" })
})
