import { mergeDeep, unique } from "remeda"
import type { ModelMessage } from "ai"
import type * as Provider from "./provider"

const providerCacheOptions = {
  anthropic: {
    cacheControl: { type: "ephemeral" },
  },
  openrouter: {
    cacheControl: { type: "ephemeral" },
  },
  bedrock: {
    cachePoint: { type: "default" },
  },
  openaiCompatible: {
    cache_control: { type: "ephemeral" },
  },
  copilot: {
    copilot_cache_control: { type: "ephemeral" },
  },
  alibaba: {
    cacheControl: { type: "ephemeral" },
  },
}

export function applyCaching(msgs: ModelMessage[], model: Provider.Model): ModelMessage[] {
  const system = msgs.filter((msg) => msg.role === "system").slice(0, 2)
  const final = msgs.filter((msg) => msg.role !== "system").slice(-2)

  for (const msg of unique([...system, ...final])) {
    const useMessageLevelOptions =
      model.providerID === "anthropic" ||
      model.providerID.includes("bedrock") ||
      model.api.npm === "@ai-sdk/amazon-bedrock"
    const shouldUseContentOptions = !useMessageLevelOptions && Array.isArray(msg.content) && msg.content.length > 0

    if (shouldUseContentOptions) {
      const lastContent = msg.content[msg.content.length - 1]
      if (
        lastContent &&
        typeof lastContent === "object" &&
        lastContent.type !== "tool-approval-request" &&
        lastContent.type !== "tool-approval-response"
      ) {
        lastContent.providerOptions = mergeDeep(lastContent.providerOptions ?? {}, providerCacheOptions)
        continue
      }
    }

    msg.providerOptions = mergeDeep(msg.providerOptions ?? {}, providerCacheOptions)
  }

  return msgs
}
