export type ProviderModelRef = {
  providerID: string
  modelID: string
}

export type RoutingPurpose = "chat" | "title" | "summary" | "compaction" | "subtask"

export type RoutingDecision = {
  requested: ProviderModelRef
  selected?: ProviderModelRef
  purpose: RoutingPurpose
  setupRequired: boolean
  developerUnlocked: boolean
  protectedRouterUsed: boolean
  activeCredentialEnv?: string
  rejected: Array<{ envName: string; reason: string }>
  receiptID: string
}
