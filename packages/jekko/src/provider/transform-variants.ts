import type * as Provider from "./provider"
import { buildModelVariants } from "./transform-variants-logic"

export function samplingParams(model: Provider.Model): { temperature?: number; topP?: number; topK?: number } {
  const id = model.id.toLowerCase()
  if (id.includes("qwen")) return { temperature: 0.55, topP: 1 }
  if (id.includes("gemini")) return { temperature: 1.0, topP: 0.95, topK: 64 }
  if (id.includes("glm-4.6") || id.includes("glm-4.7")) return { temperature: 1.0 }
  if (id.includes("minimax-m2")) {
    const topK = ["m2.", "m25", "m21"].some((s) => id.includes(s)) ? 40 : 20
    return { temperature: 1.0, topP: 0.95, topK }
  }
  if (id.includes("kimi-k2.5") || id.includes("kimi-k2p5") || id.includes("kimi-k2-5")) {
    return { temperature: 1.0, topP: 0.95 }
  }
  if (id.includes("kimi-k2")) {
    const temperature = ["thinking", "k2.", "k2p"].some((s) => id.includes(s)) ? 1.0 : 0.6
    return { temperature }
  }
  return {}
}

export function temperature(model: Provider.Model) {
  return samplingParams(model).temperature
}

export function topP(model: Provider.Model) {
  return samplingParams(model).topP
}

export function topK(model: Provider.Model) {
  return samplingParams(model).topK
}

export function variants(model: Provider.Model): Record<string, Record<string, any>> {
  return buildModelVariants(model)
}
