import { Effect, Option } from "effect"
import { UI } from "../ui"
import * as Prompt from "../effect/prompt"
import { Auth } from "../../auth"
import { CliError, fail } from "../effect-cmd"
import { errorMessage } from "@/util/error"
import type { Hooks } from "@jekko-ai/plugin"

type PluginAuth = NonNullable<Hooks["auth"]>

export type ProviderAuthSuccess =
  | { provider?: string; refresh: string; access: string; expires: number; accountId?: string; enterpriseUrl?: string }
  | { provider?: string; key: string }

const promptValue = <Value>(value: Option.Option<Value>) => {
  if (Option.isNone(value)) return Effect.die(new UI.CancelledError())
  return Effect.succeed(value.value)
}

const put = Effect.fn("Cli.providers.put")(function* (key: string, info: Auth.Info) {
  const auth = yield* Auth.Service
  yield* Effect.orDie(auth.set(key, info))
})

const persistProviderAuthSuccess = Effect.fn("Cli.providers.persistProviderAuthSuccess")(function* (
  result: ProviderAuthSuccess,
  fallbackProvider: string,
) {
  const saveProvider = result.provider ?? fallbackProvider

  if ("refresh" in result) {
    yield* put(saveProvider, {
      type: "oauth",
      refresh: result.refresh,
      access: result.access,
      expires: result.expires,
      accountId: result.accountId,
      enterpriseUrl: result.enterpriseUrl,
    })
  }

  if ("key" in result) {
    yield* put(saveProvider, {
      type: "api",
      key: result.key,
    })
  }
})

const cliTry = <Value>(message: string, fn: () => PromiseLike<Value>) =>
  Effect.tryPromise({
    try: fn,
    catch: (error) => new CliError({ message: message + errorMessage(error) }),
  })

export const handlePluginAuth = Effect.fn("Cli.providers.pluginAuth")(function* (
  plugin: { auth: PluginAuth },
  provider: string,
  methodName?: string,
) {
  const index = yield* Effect.gen(function* () {
    if (!methodName) {
      if (plugin.auth.methods.length <= 1) return 0
      return yield* promptValue(
        yield* Prompt.select({
          message: "Login method",
          options: plugin.auth.methods.map((x, index) => ({
            label: x.label,
            value: index,
          })),
        }),
      )
    }
    const match = plugin.auth.methods.findIndex((x) => x.label.toLowerCase() === methodName.toLowerCase())
    if (match === -1) {
      return yield* fail(
        `Unknown method "${methodName}" for ${provider}. Available: ${plugin.auth.methods.map((x) => x.label).join(", ")}`,
      )
    }
    return match
  })
  const method = plugin.auth.methods[index]

  yield* Effect.sleep("10 millis")
  const inputs: Record<string, string> = {}
  if (method.prompts) {
    for (const prompt of method.prompts) {
      if (prompt.when) {
        const value = inputs[prompt.when.key]
        if (value === undefined) continue
        const matches = prompt.when.op === "eq" ? value === prompt.when.value : value !== prompt.when.value
        if (!matches) continue
      }
      if (prompt.condition && !prompt.condition(inputs)) continue
      if (prompt.type === "select") {
        const value = yield* Prompt.select({
          message: prompt.message,
          options: prompt.options,
        })
        inputs[prompt.key] = yield* promptValue(value)
        continue
      }
      const value = yield* Prompt.text({
        message: prompt.message,
        defaultValue: prompt.default_value,
        validate: prompt.validate ? (v) => prompt.validate!(v ?? "") : undefined,
      })
      inputs[prompt.key] = yield* promptValue(value)
    }
  }

  if (method.type === "oauth") {
    const authorize = yield* cliTry("Failed to authorize: ", () => method.authorize(inputs))

    if (authorize.url) {
      yield* Prompt.log.info("Go to: " + authorize.url)
    }

    if (authorize.method === "auto") {
      if (authorize.instructions) {
        yield* Prompt.log.info(authorize.instructions)
      }
      const spinner = Prompt.spinner()
      yield* spinner.start("Waiting for authorization...")
      const result = yield* cliTry("Failed to authorize: ", () => authorize.callback())
      if (result.type === "failed") {
        yield* spinner.stop("Failed to authorize", 1)
      }
      if (result.type === "success") {
        yield* persistProviderAuthSuccess(result as ProviderAuthSuccess, provider)
        yield* spinner.stop("Login successful")
      }
    }

    if (authorize.method === "code") {
      const code = yield* Prompt.text({
        message: "Paste the authorization code here: ",
        validate: (x) => (x && x.length > 0 ? undefined : "Required"),
      })
      const authorizationCode = yield* promptValue(code)
      const result = yield* cliTry("Failed to authorize: ", () => authorize.callback(authorizationCode))
      if (result.type === "failed") {
        yield* Prompt.log.error("Failed to authorize")
      }
      if (result.type === "success") {
        yield* persistProviderAuthSuccess(result as ProviderAuthSuccess, provider)
        yield* Prompt.log.success("Login successful")
      }
    }

    yield* Prompt.outro("Done")
    return true
  }

  if (method.type === "api") {
    const key = yield* Prompt.password({
      message: "Enter your API key",
      validate: (x) => (x && x.length > 0 ? undefined : "Required"),
    })
    const apiKey = yield* promptValue(key)

    const metadata = Object.keys(inputs).length ? { metadata: inputs } : {}
    const authorizeApi = method.authorize
    if (!authorizeApi) {
      yield* put(provider, {
        type: "api",
        key: apiKey,
        ...metadata,
      })
      yield* Prompt.outro("Done")
      return true
    }

    const result = yield* cliTry("Failed to authorize: ", () => authorizeApi(inputs))
    if (result.type === "failed") {
      yield* Prompt.log.error("Failed to authorize")
    }
    if (result.type === "success") {
      const saveProvider = result.provider ?? provider
      yield* put(saveProvider, {
        type: "api",
        key: result.key ?? apiKey,
        ...metadata,
      })
      yield* Prompt.log.success("Login successful")
    }
    yield* Prompt.outro("Done")
    return true
  }

  return false
})

export function resolvePluginProviders(input: {
  hooks: Hooks[]
  existingProviders: Record<string, unknown>
  disabled: Set<string>
  enabled?: Set<string>
  providerNames: Record<string, string | undefined>
}): Array<{ id: string; name: string }> {
  const seen = new Set<string>()
  const result: Array<{ id: string; name: string }> = []

  for (const hook of input.hooks) {
    if (!hook.auth) continue
    const id = hook.auth.provider
    if (seen.has(id)) continue
    seen.add(id)
    if (Object.hasOwn(input.existingProviders, id)) continue
    if (input.disabled.has(id)) continue
    if (input.enabled && !input.enabled.has(id)) continue
    result.push({
      id,
      name: input.providerNames[id] ?? id,
    })
  }

  return result
}
