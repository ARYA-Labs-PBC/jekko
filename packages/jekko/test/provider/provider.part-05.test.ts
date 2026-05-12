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

test("enabled_providers with empty array allows no providers", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          enabled_providers: [],
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      set("ANTHROPIC_API_KEY", "test-api-key")
      set("OPENAI_API_KEY", "test-openai-key")
      const providers = await list()
      expect(Object.keys(providers).length).toBe(0)
    },
  })
})


test("whitelist and blacklist can be combined", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            anthropic: {
              whitelist: ["claude-sonnet-4-20250514", "claude-opus-4-20250514"],
              blacklist: ["claude-opus-4-20250514"],
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
      expect(providers[ProviderID.anthropic]).toBeDefined()
      const models = Object.keys(providers[ProviderID.anthropic].models)
      expect(models).toContain("claude-sonnet-4-20250514")
      expect(models).not.toContain("claude-opus-4-20250514")
      expect(models.length).toBe(1)
    },
  })
})


test("model modalities default correctly", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            "test-provider": {
              name: "Test",
              npm: "@ai-sdk/openai-compatible",
              env: [],
              models: {
                "test-model": {
                  name: "Test Model",
                  tool_call: true,
                  limit: { context: 8000, output: 2000 },
                },
              },
              options: { apiKey: "test" },
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
      expect(model.capabilities.input.text).toBe(true)
      expect(model.capabilities.output.text).toBe(true)
    },
  })
})


test("model with custom cost values", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          provider: {
            "test-provider": {
              name: "Test",
              npm: "@ai-sdk/openai-compatible",
              env: [],
              models: {
                "test-model": {
                  name: "Test Model",
                  tool_call: true,
                  limit: { context: 8000, output: 2000 },
                  cost: {
                    input: 5,
                    output: 15,
                    cache_read: 2.5,
                    cache_write: 7.5,
                  },
                },
              },
              options: { apiKey: "test" },
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
      expect(model.cost.input).toBe(5)
      expect(model.cost.output).toBe(15)
      expect(model.cost.cache.read).toBe(2.5)
      expect(model.cost.cache.write).toBe(7.5)
    },
  })
})


test("getSmallModel returns appropriate small model", async () => {
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
      const model = await getSmallModel(ProviderID.anthropic)
      expect(model).toBeDefined()
      expect(model?.id).toContain("haiku")
    },
  })
})


test("getSmallModel respects config small_model override", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Bun.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          small_model: "anthropic/claude-sonnet-4-20250514",
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      set("ANTHROPIC_API_KEY", "test-api-key")
      const model = await getSmallModel(ProviderID.anthropic)
      expect(model).toBeDefined()
      expect(String(model?.providerID)).toBe("anthropic")
      expect(String(model?.id)).toBe("claude-sonnet-4-20250514")
    },
  })
})


test("provider.sort prioritizes preferred models", () => {
  const models = [
    { id: "random-model", name: "Random" },
    { id: "claude-sonnet-4-latest", name: "Claude Sonnet 4" },
    { id: "gpt-5-turbo", name: "GPT-5 Turbo" },
    { id: "other-model", name: "Other" },
  ] as any[]

  const sorted = Provider.sort(models)
  expect(sorted[0].id).toContain("sonnet-4")
  expect(sorted[0].id).toContain("latest")
  expect(sorted[sorted.length - 1].id).not.toContain("gpt-5")
  expect(sorted[sorted.length - 1].id).not.toContain("sonnet-4")
})
