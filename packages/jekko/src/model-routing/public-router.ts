import { recommendedModelID } from "./recommendations"
import type { ModelKeyStatus } from "@/model-setup/model-keys"
import type { ProviderModelRef, RoutingDecision, RoutingPurpose } from "./types"

export function routePublicModel(input: {
  purpose: RoutingPurpose
  requested: ProviderModelRef
  statuses: ModelKeyStatus[]
  developerUnlocked: boolean
  receiptID: string
}): RoutingDecision {
  const active = input.statuses.filter((status) => status.active && status.configured)
  const rejected = input.statuses
    .filter((status) => status.configured && !status.active)
    .map((status) => ({ envName: status.envName, reason: status.inactiveReason ?? "inactive" }))

  if (active.length === 0) {
    return {
      requested: input.requested,
      purpose: input.purpose,
      setupRequired: true,
      developerUnlocked: input.developerUnlocked,
      protectedRouterUsed: false,
      rejected,
      receiptID: input.receiptID,
    }
  }

  const selectedProvider =
    input.requested.providerID === "auto"
      ? active[0]
      : active.find((status) => status.providerID === input.requested.providerID) ?? active[0]

  const selectedModelID = selectedProvider.recommendedModelID ?? recommendedModelID(selectedProvider.providerID)
  return {
    requested: input.requested,
    selected: {
      providerID: selectedProvider.providerID,
      modelID: selectedModelID ?? input.requested.modelID,
    },
    purpose: input.purpose,
    setupRequired: false,
    developerUnlocked: input.developerUnlocked,
    protectedRouterUsed: false,
    activeCredentialEnv: selectedProvider.envName,
    rejected,
    receiptID: input.receiptID,
  }
}
