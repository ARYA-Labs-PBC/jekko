import { expect, test } from "bun:test"
import { RECOMMENDED_MODELS } from "../../src/model-routing/recommendations.generated"
import { recommendedModelID, isKnownRecommendedModel } from "../../src/model-routing/recommendations"

test("generated recommendations include the expected public providers", () => {
  expect(RECOMMENDED_MODELS.openai).toBe("gpt-5.3-codex")
  expect(RECOMMENDED_MODELS.openrouter).toBe("openrouter-gpt-oss-120b-free")
  expect(RECOMMENDED_MODELS.cloudflare).toBe("cloudflare-gpt-oss-120b")
})

test("recommendedModelID resolves known provider models", () => {
  expect(recommendedModelID("openai")).toBe("gpt-5.3-codex")
  expect(isKnownRecommendedModel("openai", "gpt-5.3-codex")).toBe(true)
  expect(isKnownRecommendedModel("openai", "other")).toBe(false)
})
