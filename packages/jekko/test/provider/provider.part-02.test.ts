import { test, expect } from "bun:test"
import { mkdir, unlink } from "fs/promises"
import path from "path"

import { disposeAllInstances, tmpdir } from "../fixture/fixture"
import { Global } from "@jekko-ai/core/global"
import { Instance } from "../../src/project/instance"
import { WithInstance } from "../../src/project/with-instance"
import { Plugin } from "../../src/plugin/index"
import { ModelsDev } from "@/provider/models"
import { Provider } from "@/provider/provider"
import { ProviderID, ModelID } from "../../src/provider/schema"
import { Filesystem } from "@/util/filesystem"
import { Env } from "../../src/env"
import { Effect } from "effect"
import { AppRuntime } from "../../src/effect/app-runtime"
import { makeRuntime } from "../../src/effect/run-service"

const env = makeRuntime(Env.Service, Env.defaultLayer)
const set = (k: string, v: string) => env.runSync((svc) => svc.set(k, v))
const providerKey = "apiKey"

async function run<A, E>(fn: (provider: Provider.Interface) => Effect.Effect<A, E, never>) {
  return AppRuntime.runPromise(
    Effect.gen(function* () {
      const provider = yield* Provider.Service
      return yield* fn(provider)
    }),
  )
}

async function list() {
  return run((provider) => provider.list())
}

async function getProvider(providerID: ProviderID) {
  return run((provider) => provider.getProvider(providerID))
}

async function getModel(providerID: ProviderID, modelID: ModelID) {
  return run((provider) => provider.getModel(providerID, modelID))
}

async function getLanguage(model: Provider.Model) {
  return run((provider) => provider.getLanguage(model))
}

async function closest(providerID: ProviderID, query: string[]) {
  return run((provider) => provider.closest(providerID, query))
}

async function getSmallModel(providerID: ProviderID) {
  return run((provider) => provider.getSmallModel(providerID))
}

async function defaultModel() {
  return run((provider) => provider.defaultModel())
}

async function markPluginDependenciesReady(dir: string) {
  await mkdir(path.join(dir, "node_modules"), { recursive: true })
  await Bun.write(
    path.join(dir, "package-lock.json"),
    JSON.stringify({ packages: { "": { dependencies: { "@jekko-ai/plugin": "0.0.0" } } } }),
  )
}

function paid(providers: Awaited<ReturnType<typeof list>>) {
  const item = providers[ProviderID.make("jekko")]
  expect(item).toBeDefined()
  return Object.values(item.models).filter((model) => model.cost.input > 0).length
}

test("custom provider with npm package", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            "custom-provider": {
              name: "Custom Provider",
              npm: "@ai-sdk/openai-compatible",
              api: "https://api.custom.com/v1",
              env: ["CUSTOM_API_KEY"],
              models: {
                "custom-model": {
                  name: "Custom Model",
                  tool_call: true,
                  limit: {
                    context: 128000,
                    output: 4096,
                  },
                },
              },
              options: {
                [providerKey]: "example-custom-token",
              },
            },
          },
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const providers = await list()
      expect(providers[ProviderID.make("custom-provider")]).toBeDefined()
      expect(providers[ProviderID.make("custom-provider")].name).toBe("Custom Provider")
      expect(providers[ProviderID.make("custom-provider")].models["custom-model"]).toBeDefined()
    },
  })
})


test("custom DeepSeek openai-compatible model defaults interleaved reasoning field", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            "custom-provider": {
              name: "Custom Provider",
              npm: "@ai-sdk/openai-compatible",
              api: "https://api.custom.com/v1",
              models: {
                "deepseek-r1": {
                  name: "DeepSeek R1",
                },
                "deepseek-details": {
                  name: "DeepSeek Details",
                  interleaved: { field: "reasoning_details" },
                },
                "custom-model": {
                  name: "Custom Model",
                },
              },
              options: {
                [providerKey]: "example-custom-token",
              },
            },
            "custom-anthropic-provider": {
              name: "Custom Anthropic Provider",
              npm: "@ai-sdk/anthropic",
              api: "https://api.custom.com/v1",
              models: {
                "deepseek-r1": {
                  name: "DeepSeek R1",
                },
              },
              options: {
                [providerKey]: "example-custom-token",
              },
            },
          },
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const providers = await list()
      const provider = providers[ProviderID.make("custom-provider")]
      expect(provider.models["deepseek-r1"].capabilities.interleaved).toEqual({ field: "reasoning_content" })
      expect(provider.models["deepseek-details"].capabilities.interleaved).toEqual({ field: "reasoning_details" })
      expect(provider.models["custom-model"].capabilities.interleaved).toBe(false)
      expect(
        providers[ProviderID.make("custom-anthropic-provider")].models["deepseek-r1"].capabilities.interleaved,
      ).toBe(false)
    },
  })
})


test("env variable takes precedence, config merges options", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            anthropic: {
              options: {
                timeout: 60000,
                chunkTimeout: 15000,
              },
            },
          },
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      set("ANTHROPIC_API_KEY", "env-api-key")
      const providers = await list()
      expect(providers[ProviderID.anthropic]).toBeDefined()
      // Config options should be merged
      expect(providers[ProviderID.anthropic].options.timeout).toBe(60000)
      expect(providers[ProviderID.anthropic].options.chunkTimeout).toBe(15000)
    },
  })
})


test("getModel returns model for valid provider/model", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      set("ANTHROPIC_API_KEY", "test-api-key")
      const model = await getModel(ProviderID.anthropic, ModelID.make("claude-sonnet-4-20250514"))
      expect(model).toBeDefined()
      expect(String(model.providerID)).toBe("anthropic")
      expect(String(model.id)).toBe("claude-sonnet-4-20250514")
      const language = await getLanguage(model)
      expect(language).toBeDefined()
    },
  })
})


test("getModel throws ModelNotFoundError for invalid model", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      set("ANTHROPIC_API_KEY", "test-api-key")
      expect(getModel(ProviderID.anthropic, ModelID.make("nonexistent-model"))).rejects.toThrow()
    },
  })
})


test("getModel throws ModelNotFoundError for invalid provider", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      expect(getModel(ProviderID.make("nonexistent-provider"), ModelID.make("some-model"))).rejects.toThrow()
    },
  })
})


test("parseModel correctly parses provider/model string", () => {
  const result = Provider.parseModel("anthropic/claude-sonnet-4")
  expect(String(result.providerID)).toBe("anthropic")
  expect(String(result.modelID)).toBe("claude-sonnet-4")
})
