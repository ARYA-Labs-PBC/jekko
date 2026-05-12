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

test("parseModel handles model IDs with slashes", () => {
  const result = Provider.parseModel("openrouter/anthropic/claude-3-opus")
  expect(String(result.providerID)).toBe("openrouter")
  expect(String(result.modelID)).toBe("anthropic/claude-3-opus")
})


test("defaultModel returns first available model when no config set", async () => {
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
      const model = await defaultModel()
      expect(model.providerID).toBeDefined()
      expect(model.modelID).toBeDefined()
    },
  })
})


test("defaultModel respects config model setting", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          model: "anthropic/claude-sonnet-4-20250514",
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      set("ANTHROPIC_API_KEY", "test-api-key")
      const model = await defaultModel()
      expect(String(model.providerID)).toBe("anthropic")
      expect(String(model.modelID)).toBe("claude-sonnet-4-20250514")
    },
  })
})


test("provider with baseURL from config", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            "custom-openai": {
              name: "Custom OpenAI",
              npm: "@ai-sdk/openai-compatible",
              env: [],
              models: {
                "gpt-4": {
                  name: "GPT-4",
                  tool_call: true,
                  limit: { context: 128000, output: 4096 },
                },
              },
              options: {
                [providerKey]: "example-provider-token",
                baseURL: "https://custom.openai.com/v1",
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
      expect(providers[ProviderID.make("custom-openai")]).toBeDefined()
      expect(providers[ProviderID.make("custom-openai")].options.baseURL).toBe("https://custom.openai.com/v1")
    },
  })
})


test("model cost defaults to zero when not specified", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            "test-provider": {
              name: "Test Provider",
              npm: "@ai-sdk/openai-compatible",
              env: [],
              models: {
                "test-model": {
                  name: "Test Model",
                  tool_call: true,
                  limit: { context: 128000, output: 4096 },
                },
              },
              options: {
                [providerKey]: "example-provider-token",
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
      const model = providers[ProviderID.make("test-provider")].models["test-model"]
      expect(model.cost.input).toBe(0)
      expect(model.cost.output).toBe(0)
      expect(model.cost.cache.read).toBe(0)
      expect(model.cost.cache.write).toBe(0)
    },
  })
})


test("model options are merged from existing model", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            anthropic: {
              models: {
                "claude-sonnet-4-20250514": {
                  options: {
                    customOption: "custom-value",
                  },
                },
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
      set("ANTHROPIC_API_KEY", "test-api-key")
      const providers = await list()
      const model = providers[ProviderID.anthropic].models["claude-sonnet-4-20250514"]
      expect(model.options.customOption).toBe("custom-value")
    },
  })
})


test("provider removed when all models filtered out", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            anthropic: {
              whitelist: ["nonexistent-model"],
            },
          },
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      set("ANTHROPIC_API_KEY", "test-api-key")
      const providers = await list()
      expect(providers[ProviderID.anthropic]).toBeUndefined()
    },
  })
})
