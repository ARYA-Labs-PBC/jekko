import { RECOMMENDED_MODELS } from "./recommendations.generated"

export function recommendedModelID(providerID: string) {
  return RECOMMENDED_MODELS[providerID]
}

export function isKnownRecommendedModel(providerID: string, modelID: string) {
  return recommendedModelID(providerID) === modelID
}
