import { iife } from "@/util/iife"
import type * as Provider from "./provider"
import { OPENAI_EFFORTS, WIDELY_SUPPORTED_EFFORTS } from "./transform-shared"
import {
  anthropicAdaptiveEfforts,
  anthropicSimpleAdaptive,
  anthropicStaticBudgets,
  googleThinkingConfig,
  oaiEncryptedEfforts,
  openaiReasoningEfforts,
} from "./transform-variants-core"

export function buildModelVariants(model: Provider.Model): Record<string, Record<string, any>> {
  if (!model.capabilities.reasoning) return {}

  const id = model.id.toLowerCase()
  const adaptiveEfforts = anthropicAdaptiveEfforts(model.api.id)
  const hasAdaptiveEfforts = adaptiveEfforts.length > 0
  if (
    id.includes("deepseek-chat") ||
    id.includes("deepseek-reasoner") ||
    id.includes("deepseek-r1") ||
    id.includes("deepseek-v3") ||
    id.includes("minimax") ||
    id.includes("glm") ||
    id.includes("kimi") ||
    id.includes("k2p") ||
    id.includes("qwen") ||
    id.includes("big-pickle")
  )
    return {}

  if (id.includes("grok") && id.includes("grok-3-mini")) {
    if (model.api.npm === "@openrouter/ai-sdk-provider") {
      return {
        low: { reasoning: { effort: "low" } },
        high: { reasoning: { effort: "high" } },
      }
    }
    return {
      low: { reasoningEffort: "low" },
      high: { reasoningEffort: "high" },
    }
  }
  if (id.includes("grok")) return {}

  switch (model.api.npm) {
    case "@openrouter/ai-sdk-provider":
      if (!model.id.includes("gpt") && !model.id.includes("gemini-3") && !model.id.includes("claude")) return {}
      return Object.fromEntries(OPENAI_EFFORTS.map((effort) => [effort, { reasoning: { effort } }]))

    case "ai-gateway-provider": {
      if (model.api.id.startsWith("openai/")) {
        const efforts = openaiReasoningEfforts(model.api.id, model.release_date)
        if (efforts.length === 0) return {}
        return Object.fromEntries(efforts.map((effort) => [effort, { reasoningEffort: effort }]))
      }
      return Object.fromEntries(WIDELY_SUPPORTED_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }]))
    }

    case "@ai-sdk/gateway":
      if (model.id.includes("anthropic")) {
        if (hasAdaptiveEfforts) {
          return anthropicSimpleAdaptive(adaptiveEfforts)
        }
        return anthropicStaticBudgets()
      }
      if (model.id.includes("google")) {
        if (id.includes("2.5")) {
          return googleThinkingConfig()
        }
        return Object.fromEntries(
          ["low", "high"].map((effort) => [
            effort,
            {
              includeThoughts: true,
              thinkingLevel: effort,
            },
          ]),
        )
      }
      return Object.fromEntries(OPENAI_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }]))

    case "@ai-sdk/github-copilot":
      if (model.id.includes("gemini")) {
        return {}
      }
      if (model.id.includes("claude")) {
        return Object.fromEntries(WIDELY_SUPPORTED_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }]))
      }
      const copilotEfforts = iife(() => {
        if (id.includes("5.1-codex-max") || id.includes("5.2") || id.includes("5.3"))
          return [...WIDELY_SUPPORTED_EFFORTS, "xhigh"]
        const arr = [...WIDELY_SUPPORTED_EFFORTS]
        if (id.includes("gpt-5") && model.release_date >= "2025-12-04") arr.push("xhigh")
        return arr
      })
      return oaiEncryptedEfforts(copilotEfforts)

    case "@ai-sdk/cerebras":
    case "@ai-sdk/togetherai":
    case "@ai-sdk/xai":
    case "@ai-sdk/deepinfra":
    case "venice-ai-sdk-provider":
    case "@ai-sdk/openai-compatible":
      const efforts = [...WIDELY_SUPPORTED_EFFORTS]
      if (model.api.id.toLowerCase().includes("deepseek-v4")) {
        efforts.push("max")
      }
      return Object.fromEntries(efforts.map((effort) => [effort, { reasoningEffort: effort }]))

    case "@ai-sdk/azure":
      if (id === "o1-mini") return {}
      const azureEfforts = ["low", "medium", "high"]
      if (id.includes("gpt-5-") || id === "gpt-5") {
        azureEfforts.unshift("minimal")
      }
      return oaiEncryptedEfforts(azureEfforts)

    case "@ai-sdk/openai": {
      const efforts = openaiReasoningEfforts(model.api.id, model.release_date)
      if (efforts.length === 0) return {}
      return oaiEncryptedEfforts(efforts)
    }

    case "@ai-sdk/anthropic":
    case "@ai-sdk/google-vertex/anthropic":
      if (hasAdaptiveEfforts) {
        let efforts = [...adaptiveEfforts]
        if (model.providerID === "github-copilot") {
          if (model.api.id.includes("opus-4.7")) {
            efforts = ["medium"]
          }
          efforts = efforts.filter((v) => v !== "max" && v !== "xhigh")
        }
        return Object.fromEntries(
          efforts.map((effort) => [
            effort,
            {
              thinking: {
                type: "adaptive",
                ...(model.api.id.includes("opus-4-7") || model.api.id.includes("opus-4.7")
                  ? { display: "summarized" }
                  : {}),
              },
              effort,
            },
          ]),
        )
      }

      return {
        high: {
          thinking: {
            type: "enabled",
            budgetTokens: Math.min(16_000, Math.floor(model.limit.output / 2 - 1)),
          },
        },
        max: {
          thinking: {
            type: "enabled",
            budgetTokens: Math.min(31_999, model.limit.output - 1),
          },
        },
      }

    case "@ai-sdk/amazon-bedrock":
      if (hasAdaptiveEfforts) {
        return Object.fromEntries(
          adaptiveEfforts.map((effort) => [
            effort,
            {
              reasoningConfig: {
                type: "adaptive",
                maxReasoningEffort: effort,
                ...(model.api.id.includes("opus-4-7") || model.api.id.includes("opus-4.7")
                  ? { display: "summarized" }
                  : {}),
              },
            },
          ]),
        )
      }
      if (model.api.id.includes("anthropic")) {
        return {
          high: {
            reasoningConfig: {
              type: "enabled",
              budgetTokens: 16000,
            },
          },
          max: {
            reasoningConfig: {
              type: "enabled",
              budgetTokens: 31999,
            },
          },
        }
      }

      return Object.fromEntries(
        WIDELY_SUPPORTED_EFFORTS.map((effort) => [
          effort,
          {
            reasoningConfig: {
              type: "enabled",
              maxReasoningEffort: effort,
            },
          },
        ]),
      )

    case "@ai-sdk/google-vertex":
    case "@ai-sdk/google":
      if (id.includes("2.5")) {
        return googleThinkingConfig()
      }
      let levels = ["low", "high"]
      if (id.includes("3.1")) {
        levels = ["low", "medium", "high"]
      }

      return Object.fromEntries(
        levels.map((effort) => [
          effort,
          {
            thinkingConfig: {
              includeThoughts: true,
              thinkingLevel: effort,
            },
          },
        ]),
      )

    case "@ai-sdk/mistral":
      if (!model.capabilities.reasoning) return {}
      const MISTRAL_REASONING_IDS = [
        "mistral-small-2603",
        "mistral-small-latest",
        "mistral-medium-3.5",
        "mistral-medium-2604",
      ]
      const mistralId = model.api.id.toLowerCase()
      if (!MISTRAL_REASONING_IDS.some((id) => mistralId.includes(id))) return {}
      return {
        high: { reasoningEffort: "high" },
      }

    case "@ai-sdk/cohere":
      return {}

    case "@ai-sdk/groq":
      const groqEffort = ["none", ...WIDELY_SUPPORTED_EFFORTS]
      return Object.fromEntries(
        groqEffort.map((effort) => [
          effort,
          {
            reasoningEffort: effort,
          },
        ]),
      )

    case "@ai-sdk/perplexity":
      return {}

    case "@jerome-benoit/sap-ai-provider-v2":
      if (model.api.id.includes("anthropic")) {
        if (hasAdaptiveEfforts) {
          return anthropicSimpleAdaptive(adaptiveEfforts)
        }
        return anthropicStaticBudgets()
      }
      if (model.api.id.includes("gemini") && id.includes("2.5")) {
        return googleThinkingConfig()
      }
      if (model.api.id.includes("gpt") || /\bo[1-9]/.test(model.api.id)) {
        return Object.fromEntries(WIDELY_SUPPORTED_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }]))
      }
      return {}
  }
  return {}
}
