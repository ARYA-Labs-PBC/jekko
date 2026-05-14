import { afterEach, expect, test } from "bun:test"
import { Effect } from "effect"
import { AppRuntime } from "../../src/effect/app-runtime"
import { Provider } from "@/provider/provider"
import { ProviderID } from "../../src/provider/schema"
import { WithInstance } from "../../src/project/with-instance"
import { tmpdir } from "../fixture/fixture"

const ENV_KEYS = ["JEKKO_MODEL_KEYS_CONTENT", "JEKKO_MODEL_KEYS_FILE", "JNOCCIO_DEVELOPER_KEY"]

function withEnv(values: Record<string, string | undefined>) {
  const previous = new Map<string, string | undefined>()
  for (const key of Object.keys(values)) previous.set(key, process.env[key])
  for (const [key, value] of Object.entries(values)) {
    if (value === undefined) delete process.env[key]
    else process.env[key] = value
  }
  return () => {
    for (const [key, value] of previous) {
      if (value === undefined) delete process.env[key]
      else process.env[key] = value
    }
  }
}

afterEach(() => {
  for (const key of ENV_KEYS) delete process.env[key]
})

async function listProviders() {
  return AppRuntime.runPromise(
    Effect.gen(function* () {
      const provider = yield* Provider.Service
      return yield* provider.list()
    }),
  )
}

test("provider auth is redacted and omits raw key strings", async () => {
  const restore = withEnv({
    JEKKO_MODEL_KEYS_CONTENT: "OPENAI_API_KEY=super-secret-openai-key\n",
  })
  try {
    await using tmp = await tmpdir({
      config: {
        provider: {
          openai: {
            options: { baseURL: "http://localhost:1/v1" },
          },
        },
      },
    })

    const providers = await WithInstance.provide({
      directory: tmp.path,
      fn: async () => listProviders(),
    })

    const openai = providers[ProviderID.openai]
    expect(openai).toBeDefined()
    expect(openai?.auth?.configured).toBe(true)
    expect(openai?.auth?.active).toBe(true)
    expect(openai?.auth?.source).toBe("jekko.env")
    expect(openai?.auth?.inactiveReason).toBeUndefined()
    expect((openai as any)?.key).toBeUndefined()
    expect(JSON.stringify(providers)).not.toContain("super-secret-openai-key")
  } finally {
    restore()
  }
})
