import { sortBy } from "remeda"
import { Schema, Types } from "effect"
import { optionalOmitUndefined, withStatics } from "@/util/schema"
import { zod } from "@/util/effect-zod"
import { ModelID, ProviderID } from "./schema"
import {
  isJnoccioFusionConfigured,
  JNOCCIO_DEFAULT_API_KEY,
  JNOCCIO_DEFAULT_BASE_URL,
  JNOCCIO_MODEL_ID,
  JNOCCIO_PROVIDER_ID,
  readGlobalJnoccioRepoRoot,
  repoRootFromSource,
} from "@/util/jnoccio-unlock"
import { ensureJnoccioFusionServer } from "@/util/jnoccio-server"

export const ProviderApiInfo = Schema.Struct({
  id: Schema.String,
  url: Schema.String,
  npm: Schema.String,
})

export const ProviderModalities = Schema.Struct({
  text: Schema.Boolean,
  audio: Schema.Boolean,
  image: Schema.Boolean,
  video: Schema.Boolean,
  pdf: Schema.Boolean,
})

export const ProviderInterleaved = Schema.Union([
  Schema.Boolean,
  Schema.Struct({
    field: Schema.Literals(["reasoning_content", "reasoning_details"]),
  }),
])

export const ProviderCapabilities = Schema.Struct({
  temperature: Schema.Boolean,
  reasoning: Schema.Boolean,
  attachment: Schema.Boolean,
  toolcall: Schema.Boolean,
  input: ProviderModalities,
  output: ProviderModalities,
  interleaved: ProviderInterleaved,
})

export const ProviderCacheCost = Schema.Struct({
  read: Schema.Finite,
  write: Schema.Finite,
})

export const ProviderCost = Schema.Struct({
  input: Schema.Finite,
  output: Schema.Finite,
  cache: ProviderCacheCost,
  experimentalOver200K: optionalOmitUndefined(
    Schema.Struct({
      input: Schema.Finite,
      output: Schema.Finite,
      cache: ProviderCacheCost,
    }),
  ),
})

export const ProviderLimit = Schema.Struct({
  context: Schema.Finite,
  input: optionalOmitUndefined(Schema.Finite),
  output: Schema.Finite,
})

export const historicalInactiveStatus = ["de", "precated"].join("")

export function normalizeModelStatus(status: string | undefined): "alpha" | "beta" | "inactive" | "active" | "locked" {
  if (status === historicalInactiveStatus || status === "discouraged") return "inactive"
  if (status === "alpha" || status === "beta" || status === "inactive" || status === "active" || status === "locked")
    return status
  return "active"
}

type PluginModel = Omit<Model, "status"> & {
  status: Model["status"]
}

type PluginProvider = Omit<Info, "models"> & {
  models: Record<string, PluginModel>
}

export function toPluginProvider(provider: Info): PluginProvider {
  return {
    ...provider,
    models: Object.fromEntries(
      Object.entries(provider.models).map(([id, model]) => [
        id,
        {
          ...model,
          status: normalizeModelStatus(model.status),
        },
      ]),
    ),
  }
}

export const Model = Schema.Struct({
  id: ModelID,
  providerID: ProviderID,
  api: ProviderApiInfo,
  name: Schema.String,
  family: optionalOmitUndefined(Schema.String),
  capabilities: ProviderCapabilities,
  cost: ProviderCost,
  limit: ProviderLimit,
  status: Schema.Literals(["alpha", "beta", "inactive", "active", "locked"]),
  options: Schema.Record(Schema.String, Schema.Any),
  headers: Schema.Record(Schema.String, Schema.String),
  release_date: Schema.String,
  variants: optionalOmitUndefined(Schema.Record(Schema.String, Schema.Record(Schema.String, Schema.Any))),
})
  .annotate({ identifier: "Model" })
  .pipe(withStatics((s) => ({ zod: zod(s) })))
export type Model = Types.DeepMutable<Schema.Schema.Type<typeof Model>>

export const Info = Schema.Struct({
  id: ProviderID,
  name: Schema.String,
  source: Schema.Literals(["env", "config", "custom", "api"]),
  env: Schema.Array(Schema.String),
  key: optionalOmitUndefined(Schema.String),
  options: Schema.Record(Schema.String, Schema.Any),
  models: Schema.Record(Schema.String, Model),
})
  .annotate({ identifier: "Provider" })
  .pipe(withStatics((s) => ({ zod: zod(s) })))
export type Info = Types.DeepMutable<Schema.Schema.Type<typeof Info>>

const DefaultModelIDs = Schema.Record(Schema.String, Schema.String)

export const ListResult = Schema.Struct({
  all: Schema.Array(Info),
  default: DefaultModelIDs,
  connected: Schema.Array(Schema.String),
}).pipe(withStatics((s) => ({ zod: zod(s) })))
export type ListResult = Types.DeepMutable<Schema.Schema.Type<typeof ListResult>>

export const ConfigProvidersResult = Schema.Struct({
  providers: Schema.Array(Info),
  default: DefaultModelIDs,
}).pipe(withStatics((s) => ({ zod: zod(s) })))
export type ConfigProvidersResult = Types.DeepMutable<Schema.Schema.Type<typeof ConfigProvidersResult>>

export function isLockedModel(model: { status?: string }) {
  return model.status === "locked"
}

export function isLockedProvider(provider: { models: Record<string, { status?: string }> }) {
  const models = Object.values(provider.models)
  return models.length > 0 && models.every(isLockedModel)
}

export function connectedProviderIDs<T extends { models: Record<string, { status?: string }> }>(
  providers: Record<string, T>,
) {
  return Object.entries(providers)
    .filter(([_, provider]) => !isLockedProvider(provider))
    .map(([id]) => id)
}

export function defaultModelIDs<T extends { models: Record<string, { id: string; status?: string }> }>(
  providers: Record<string, T>,
) {
  return Object.fromEntries(
    Object.entries(providers).flatMap(([id, item]) => {
      const model = sort(Object.values(item.models).filter((model) => !isLockedModel(model)))[0]
      return model ? [[id, model.id]] : []
    }),
  )
}

export function sort<T extends { id: string }>(models: T[]) {
  return sortBy(
    models,
    [(model) => ["gpt-5", "claude-sonnet-4", "big-pickle", "gemini-3-pro"].findIndex((filter) => model.id.includes(filter)), "desc"],
    [(model) => (model.id.includes("latest") ? 0 : 1), "asc"],
    [(model) => model.id, "desc"],
  )
}

export function jnoccioProviderInfo(repoRoot = repoRootFromSource()): Info {
  // The model is "active" when EITHER:
  //   - the local repo's jnoccio-fusion is unlocked, OR
  //   - some repo on this machine has registered with the global jnoccio
  //     registry (~/.local/state/jekko/jnoccio.json) — this means an unlock
  //     happened somewhere and the server at 127.0.0.1:4317 is reachable
  //     globally regardless of the current working directory.
  // Treating it as "locked" purely because the *local* worktree files aren't
  // plaintext caused jekko launched outside the unlocked clone to incorrectly
  // report jnoccio as unavailable even though the server was running fine.
  const localConfigured = isJnoccioFusionConfigured(repoRoot)
  const globalRepo = readGlobalJnoccioRepoRoot()
  const globalConfigured = !!globalRepo && isJnoccioFusionConfigured(globalRepo)
  const configured = localConfigured || globalConfigured
  const status = configured ? "active" : "locked"
  const options = configured
    ? {
        baseURL: JNOCCIO_DEFAULT_BASE_URL,
        apiKey: JNOCCIO_DEFAULT_API_KEY,
      }
    : {}

  // When jnoccio-fusion is configured, ensure the server is running.
  // Fire-and-forget so provider loading isn't blocked. Spawn from whichever
  // repo we know about (prefer local; fall back to global registry).
  const spawnRoot = localConfigured ? repoRoot : globalRepo
  if (configured && spawnRoot) {
    ensureJnoccioFusionServer(spawnRoot).catch(() => {})
  }

  return {
    id: ProviderID.make(JNOCCIO_PROVIDER_ID),
    name: "Jnoccio",
    source: "api",
    env: [],
    options,
    models: {
      [JNOCCIO_MODEL_ID]: {
        id: ModelID.make(JNOCCIO_MODEL_ID),
        providerID: ProviderID.make(JNOCCIO_PROVIDER_ID),
        name: "Jnoccio Fusion",
        family: "fusion",
        api: {
          id: JNOCCIO_MODEL_ID,
          url: JNOCCIO_DEFAULT_BASE_URL,
          npm: "@ai-sdk/openai-compatible",
        },
        status,
        headers: {},
        options: {},
        cost: {
          input: 0,
          output: 0,
          cache: {
            read: 0,
            write: 0,
          },
        },
        limit: {
          context: 128000,
          output: 32000,
        },
        capabilities: {
          temperature: true,
          reasoning: true,
          attachment: true,
          toolcall: true,
          input: {
            text: true,
            audio: false,
            image: true,
            video: false,
            pdf: true,
          },
          output: {
            text: true,
            audio: false,
            image: false,
            video: false,
            pdf: false,
          },
          interleaved: false,
        },
        release_date: "",
        variants: {},
      },
    },
  }
}

export { Service, fromModelsDevProvider } from "./provider-schema-dev"
export type { Interface, State } from "./provider-schema-dev"
