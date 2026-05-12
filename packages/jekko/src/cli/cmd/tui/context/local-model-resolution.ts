export type ProviderModel = { providerID: string; modelID: string }

export type ModelResolutionSource = "agent" | "agent-model" | "args" | "config" | "recent" | "provider"

export type ModelResolution =
  | {
      kind: "resolved"
      source: ModelResolutionSource
      model: ProviderModel
    }
  | {
      kind: "missing"
      source: ModelResolutionSource
      reason: string
      repairHint: string
      retryAfterMs: number
    }

export const MODEL_REPAIR_HINT =
  "Choose a valid provider/model in the workspace settings or refresh the recent-model history."
export const MODEL_RETRY_AFTER_MS = 30_000

export function resolvedModelResolution(source: ModelResolutionSource, model: ProviderModel): ModelResolution {
  return {
    kind: "resolved",
    source,
    model,
  }
}

export function missingModelResolution(source: ModelResolutionSource, reason: string): ModelResolution {
  return {
    kind: "missing",
    source,
    reason,
    repairHint: MODEL_REPAIR_HINT,
    retryAfterMs: MODEL_RETRY_AFTER_MS,
  }
}

export function resolveModelChoice(
  value: string | undefined,
  options: {
    source: Exclude<ModelResolutionSource, "agent" | "agent-model">
    parseModel: (model: string) => ProviderModel
    isModelValid: (model: ProviderModel) => boolean
  },
) {
  if (!value) {
    return missingModelResolution(options.source, "No model value was configured")
  }

  const choice = options.parseModel(value)
  if (!options.isModelValid(choice)) {
    return missingModelResolution(options.source, `Model ${choice.providerID}/${choice.modelID} is not valid`)
  }

  return resolvedModelResolution(options.source, choice)
}

export function resolveRecentModel(
  recent: ProviderModel[],
  isModelValid: (model: ProviderModel) => boolean,
) {
  for (const item of recent) {
    if (isModelValid(item)) return resolvedModelResolution("recent", item)
  }
  return missingModelResolution("recent", "No recent valid models were available")
}

export function resolveProviderModel(sync: {
  data: { provider: Array<{ id: string; models: Record<string, { id: string; status: string }> }>; config: { model?: string } }
}) {
  const provider = sync.data.provider.find((entry) =>
    Object.values(entry.models).some((model) => model.status !== "locked"),
  )
  if (!provider) return missingModelResolution("provider", "No provider had an unlocked model")

  const defaultModel = sync.data.config.model
  const defaultInfo = defaultModel ? provider.models[defaultModel] : undefined
  const fallbackInfo =
    defaultInfo && defaultInfo.status !== "locked"
      ? defaultInfo
      : Object.values(provider.models).find((model) => model.status !== "locked")

  if (!fallbackInfo?.id) {
    return missingModelResolution("provider", `Provider ${provider.id} has no unlocked models`)
  }

  return resolvedModelResolution("provider", {
    providerID: provider.id,
    modelID: fallbackInfo.id,
  })
}

export function chooseModelResolution(candidates: ModelResolution[]) {
  const notes: string[] = []
  for (const candidate of candidates) {
    if (candidate.kind === "resolved") return candidate
    notes.push(candidate.reason)
  }

  const reason = notes.length ? notes.join(" | ") : "No model candidates were available"
  return missingModelResolution("provider", reason)
}
