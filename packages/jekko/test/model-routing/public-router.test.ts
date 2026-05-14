import { expect, test } from "bun:test"
import { routePublicModel } from "../../src/model-routing/public-router"

test("routePublicModel returns setupRequired when no keys are active", () => {
  const decision = routePublicModel({
    purpose: "chat",
    requested: { providerID: "auto", modelID: "smart" },
    statuses: [
      {
        envName: "OPENAI_API_KEY",
        providerID: "openai",
        configured: false,
        active: false,
      },
    ],
    developerUnlocked: false,
    receiptID: "receipt-1",
  })

  expect(decision.setupRequired).toBe(true)
  expect(decision.selected).toBeUndefined()
  expect(decision.requested).toEqual({ providerID: "auto", modelID: "smart" })
})

test("routePublicModel picks the first active provider and recommended model", () => {
  const decision = routePublicModel({
    purpose: "chat",
    requested: { providerID: "auto", modelID: "smart" },
    statuses: [
      {
        envName: "OPENAI_API_KEY",
        providerID: "openai",
        configured: true,
        active: true,
        recommendedModelID: "gpt-5.3-codex",
      },
      {
        envName: "GROQ_API_KEY",
        providerID: "groq",
        configured: true,
        active: false,
        inactiveReason: "no-developer-key",
        recommendedModelID: "groq-qwen3-32b",
      },
    ],
    developerUnlocked: false,
    receiptID: "receipt-2",
  })

  expect(decision.setupRequired).toBe(false)
  expect(decision.selected).toEqual({ providerID: "openai", modelID: "gpt-5.3-codex" })
  expect(decision.rejected).toEqual([
    { envName: "GROQ_API_KEY", reason: "no-developer-key" },
  ])
})
