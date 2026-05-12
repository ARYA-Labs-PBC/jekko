export type PromptUsageTokens = {
  input: number
  output: number
  reasoning: number
  cache: {
    read: number
    write: number
  }
}

export type PromptUsageMessage = {
  role: "assistant" | "user"
  cost?: number
  modelID?: string
  providerID?: string
  tokens?: PromptUsageTokens
}

export type PromptUsageProvider = {
  id: string
  models?: Record<string, { limit?: { context?: number | null } }>
}

export type PromptUsageState =
  | {
      kind: "missing-session"
      reason: string
    }
  | {
      kind: "missing-assistant"
      reason: string
    }
  | {
      kind: "zero-tokens"
      reason: string
    }
  | {
      kind: "ready"
      tokens: number
      cost: number
      contextLimit?: number
    }

function totalTokens(tokens: PromptUsageTokens) {
  return tokens.input + tokens.output + tokens.reasoning + tokens.cache.read + tokens.cache.write
}

export function computePromptUsage(
  sessionID: string | undefined,
  messages: PromptUsageMessage[],
  providers: PromptUsageProvider[],
): PromptUsageState {
  if (!sessionID) {
    return {
      kind: "missing-session",
      reason: "Open a session to review prompt usage.",
    }
  }

  const lastAssistant = messages.findLast((item): item is PromptUsageMessage & { role: "assistant"; tokens: PromptUsageTokens } => {
    return item.role === "assistant" && item.tokens !== undefined && item.tokens.output > 0
  })

  if (!lastAssistant) {
    return {
      kind: "missing-assistant",
      reason: "Wait for the first assistant reply to measure prompt usage.",
    }
  }

  const tokens = totalTokens(lastAssistant.tokens)
  if (tokens <= 0) {
    return {
      kind: "zero-tokens",
      reason: "Prompt usage is still zero for this session.",
    }
  }

  const cost = messages.reduce((sum, item) => sum + (item.role === "assistant" ? item.cost ?? 0 : 0), 0)
  const model = providers.find((item) => item.id === lastAssistant.providerID)?.models?.[lastAssistant.modelID ?? ""]
  const contextLimit = model?.limit?.context ?? undefined

  return {
    kind: "ready",
    tokens,
    cost,
    contextLimit,
  }
}
