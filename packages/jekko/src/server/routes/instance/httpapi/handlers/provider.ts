import { ProviderAuth } from "@/provider/auth"
import { Config } from "@/config/config"
import { ModelsDev } from "@/provider/models"
import { Provider } from "@/provider/provider"
import { ProviderID } from "@/provider/schema"
import { unlockJnoccioFusion, findRepoRootFrom, repoRootFromSource, type JnoccioUnlockInput } from "@/util/jnoccio-unlock"
import * as InstanceState from "@/effect/instance-state"
import { mapValues } from "remeda"
import { Effect, Schema } from "effect"
import { HttpServerRequest, HttpServerResponse } from "effect/unstable/http"
import { HttpApiBuilder, HttpApiError } from "effect/unstable/httpapi"
import { InstanceHttpApi } from "../api"

export const providerHandlers = HttpApiBuilder.group(InstanceHttpApi, "provider", (handlers) =>
  Effect.gen(function* () {
    const list = Effect.fn("ProviderHttpApi.list")(function* () {
      const cfg = yield* Config.Service
      const provider = yield* Provider.Service
      const config = yield* cfg.get()
      const all = yield* ModelsDev.Service.use((s) => s.get())
      const disabled = new Set(config.disabled_providers ?? [])
      const enabled = config.enabled_providers ? new Set(config.enabled_providers) : undefined
      const filtered: Record<string, (typeof all)[string]> = {}
      for (const [key, value] of Object.entries(all)) {
        if ((enabled ? enabled.has(key) : true) && !disabled.has(key)) filtered[key] = value
      }
      const connected = yield* provider.list()
      const providers = Object.assign(
        mapValues(filtered, (item) => Provider.fromModelsDevProvider(item)),
        connected,
      )
      return {
        all: Object.values(providers),
        default: Provider.defaultModelIDs(providers),
        connected: Provider.connectedProviderIDs(connected),
      }
    })

    const unlock = Effect.fn("ProviderHttpApi.unlock")(function* (ctx: { payload?: JnoccioUnlockInput }) {
      const instCtx = yield* InstanceState.context
      const repoRoot = findRepoRootFrom(instCtx.worktree) ?? findRepoRootFrom(instCtx.directory) ?? repoRootFromSource()
      return yield* Effect.promise(() => unlockJnoccioFusion(ctx.payload ?? {}, { repoRoot }))
    })

    const auth = Effect.fn("ProviderHttpApi.auth")(function* () {
      const svc = yield* ProviderAuth.Service
      return yield* svc.methods()
    })

    const authorize = Effect.fn("ProviderHttpApi.authorize")(function* (ctx: {
      params: { providerID: ProviderID }
      payload: ProviderAuth.AuthorizeInput
    }) {
      const svc = yield* ProviderAuth.Service
      return yield* svc
        .authorize({
          providerID: ctx.params.providerID,
          method: ctx.payload.method,
          inputs: ctx.payload.inputs,
        })
        .pipe(Effect.catch(() => Effect.fail(new HttpApiError.BadRequest({}))))
    })

    const authorizeRaw = Effect.fn("ProviderHttpApi.authorizeRaw")(function* (ctx: {
      params: { providerID: ProviderID }
      request: HttpServerRequest.HttpServerRequest
    }) {
      const body = yield* Effect.orDie(ctx.request.text)
      const payload = yield* Schema.decodeUnknownEffect(Schema.fromJsonString(ProviderAuth.AuthorizeInput))(body).pipe(
        Effect.mapError(() => new HttpApiError.BadRequest({})),
      )
      const result = yield* authorize({ params: ctx.params, payload })
      if (result === undefined) return HttpServerResponse.empty({ status: 200 })
      return HttpServerResponse.jsonUnsafe(result)
    })

    const callback = Effect.fn("ProviderHttpApi.callback")(function* (ctx: {
      params: { providerID: ProviderID }
      payload: ProviderAuth.CallbackInput
    }) {
      const svc = yield* ProviderAuth.Service
      yield* svc
        .callback({
          providerID: ctx.params.providerID,
          method: ctx.payload.method,
          code: ctx.payload.code,
        })
        .pipe(Effect.catch(() => Effect.fail(new HttpApiError.BadRequest({}))))
      return true
    })

    return handlers
      .handle("list", list)
      .handle("auth", auth)
      .handle("unlock", unlock)
      .handleRaw("authorize", authorizeRaw)
      .handle("callback", callback)
  }),
)
