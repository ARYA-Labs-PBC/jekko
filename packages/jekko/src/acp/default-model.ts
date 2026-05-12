import * as Log from "@jekko-ai/core/util/log"
import { Provider } from "../provider/provider"
import { ModelID, ProviderID } from "../provider/schema"
import type { ACPConfig } from "./types"

const log = Log.create({ service: "acp-default-model" })

type ModelSelection = { providerID: ProviderID; modelID: ModelID }
type ProviderModel = { id: string; providerID: string; variants?: Record<string, any> }
type ProviderEntry = { id: string; models: Record<string, ProviderModel> }

async function loadConfiguredModel(config: ACPConfig, directory: string): Promise<ModelSelection | null> {
  const configured = config.defaultModel
  if (configured) return configured

  let specified: ModelSelection | null = null
  try {
    const resp = await config.sdk.config.get({ directory }, { throwOnError: true })
    const model = resp.data?.model
    if (model) {
      specified = Provider.parseModel(model)
    } else {
      log.warn("no configured default model found in user config", {
        directory,
        repair: "set config.model or let provider selection choose one",
      })
    }
  } catch (error) {
    log.error("failed to load user config for default model", { error })
  }

  return specified
}

function resolveProviderSelection(
  providers: ProviderEntry[],
  specified: ModelSelection | null,
): ModelSelection | null {
  if (specified && providers.length) {
    const provider = providers.find((p) => p.id === specified.providerID)
    if (provider && provider.models[specified.modelID]) return specified
  }

  if (specified && !providers.length) return specified

  const jekkoProvider = providers.find((p) => p.id === "jekko")
  if (jekkoProvider) {
    if (jekkoProvider.models["big-pickle"]) {
      return { providerID: ProviderID.jekko, modelID: ModelID.make("big-pickle") }
    }
    const [best] = Provider.sort(Object.values(jekkoProvider.models))
    if (best) {
      return {
        providerID: ProviderID.make(best.providerID),
        modelID: ModelID.make(best.id),
      }
    }
  }

  const models = providers.flatMap((p) => Object.values(p.models))
  const [best] = Provider.sort(models)
  if (best) {
    return {
      providerID: ProviderID.make(best.providerID),
      modelID: ModelID.make(best.id),
    }
  }

  return specified
}

export async function defaultModel(config: ACPConfig, cwd?: string): Promise<ModelSelection> {
  const directory = cwd ?? process.cwd()
  const specified = await loadConfiguredModel(config, directory)
  const providers = await config.sdk.config
    .providers({ directory }, { throwOnError: true })
    .catch((error: unknown) => {
      log.error("failed to list providers for default model", { error })
      return { data: null }
    })
    .then((x) => x.data?.providers ?? [])

  const resolved = resolveProviderSelection(providers, specified)
  if (resolved) return resolved

  return { providerID: ProviderID.jekko, modelID: ModelID.make("big-pickle") }
}
