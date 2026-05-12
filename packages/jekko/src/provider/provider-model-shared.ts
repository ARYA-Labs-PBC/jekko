import { Schema } from "effect"

const ModelModalitiesTypes = Schema.Literals(["text", "audio", "image", "video", "pdf"])

export const ModelCost = Schema.Struct({
  input: Schema.Finite,
  output: Schema.Finite,
  cache_read: Schema.optional(Schema.Finite),
  cache_write: Schema.optional(Schema.Finite),
  context_over_200k: Schema.optional(
    Schema.Struct({
      input: Schema.Finite,
      output: Schema.Finite,
      cache_read: Schema.optional(Schema.Finite),
      cache_write: Schema.optional(Schema.Finite),
    }),
  ),
})

export const ModelLimit = Schema.Struct({
  context: Schema.Finite,
  input: Schema.optional(Schema.Finite),
  output: Schema.Finite,
})

export const ModelProvider = Schema.Struct({
  npm: Schema.optional(Schema.String),
  api: Schema.optional(Schema.String),
})

const wellKnownProviderIds = [
  ["jekko", "jekko"],
  ["anthropic", "anthropic"],
  ["openai", "openai"],
  ["google", "google"],
  ["googleVertex", "google-vertex"],
  ["githubCopilot", "github-copilot"],
  ["amazonBedrock", "amazon-bedrock"],
  ["azure", "azure"],
  ["openrouter", "openrouter"],
  ["mistral", "mistral"],
  ["gitlab", "gitlab"],
] as const

export function providerIDStatics<T extends { make: (value: string) => unknown }>(schema: T) {
  return Object.fromEntries(wellKnownProviderIds.map(([name, value]) => [name, schema.make(value)])) as Record<
    (typeof wellKnownProviderIds)[number][0],
    unknown
  >
}

export const ModelModalities = {
  immutable: Schema.Struct({
    input: Schema.Array(ModelModalitiesTypes),
    output: Schema.Array(ModelModalitiesTypes),
  }),
  mutable: Schema.Struct({
    input: Schema.mutable(Schema.Array(ModelModalitiesTypes)),
    output: Schema.mutable(Schema.Array(ModelModalitiesTypes)),
  }),
}

export function providerIdentityFields(input?: { optional?: boolean; mutableEnv?: boolean }) {
  const env = input?.mutableEnv ? Schema.mutable(Schema.Array(Schema.String)) : Schema.Array(Schema.String)

  return {
    api: Schema.optional(Schema.String),
    name: input?.optional ? Schema.optional(Schema.String) : Schema.String,
    env: input?.optional ? Schema.optional(env) : env,
    id: input?.optional ? Schema.optional(Schema.String) : Schema.String,
    npm: Schema.optional(Schema.String),
  } as const
}

export function modelFields(input?: { optionalLimit?: boolean; mutableModalities?: boolean }) {
  const limit = input?.optionalLimit ? Schema.optional(ModelLimit) : ModelLimit
  const modalities = input?.mutableModalities ? ModelModalities.mutable : ModelModalities.immutable

  return {
    cost: Schema.optional(ModelCost),
    limit,
    modalities: Schema.optional(modalities),
    provider: Schema.optional(ModelProvider),
  } as const
}
