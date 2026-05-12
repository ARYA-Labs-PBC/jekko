type AssistantLike<TContent extends { type: string }> = {
  content: readonly TContent[]
}

type AssistantMessageLike = {
  type: "assistant"
  time: {
    completed?: unknown
  }
}

type CompactionMessageLike = {
  type: "compaction"
}

type ShellMessageLike = {
  type: "shell"
  callID: string
}

export function activeAssistant<T extends AssistantMessageLike>(messages: readonly T[]) {
  const index = messages.findLastIndex((message) => message.type === "assistant" && !message.time.completed)
  if (index < 0) return
  const message = messages[index]
  if (message?.type !== "assistant") return
  return message
}

export function activeCompaction<T extends CompactionMessageLike>(messages: readonly T[]) {
  const index = messages.findLastIndex((message) => message.type === "compaction")
  if (index < 0) return
  const message = messages[index]
  if (message?.type !== "compaction") return
  return message
}

export function activeShell<T extends ShellMessageLike>(messages: readonly T[], callID: string) {
  const index = messages.findLastIndex((message) => message.type === "shell" && message.callID === callID)
  if (index < 0) return
  const message = messages[index]
  if (message?.type !== "shell") return
  return message
}

export function latestTool<T extends { type: string; id: string }>(
  assistant: AssistantLike<T> | undefined,
  callID?: string,
) {
  return assistant?.content.findLast(
    (item): item is Extract<T, { type: "tool" }> =>
      item.type === "tool" && (callID === undefined || item.id === callID),
  )
}

export function latestText<T extends { type: string }>(assistant: AssistantLike<T> | undefined) {
  return assistant?.content.findLast((item): item is Extract<T, { type: "text" }> => item.type === "text")
}

export function latestReasoning<T extends { type: string; id: string }>(
  assistant: AssistantLike<T> | undefined,
  reasoningID: string,
) {
  return assistant?.content.findLast(
    (item): item is Extract<T, { type: "reasoning" }> => item.type === "reasoning" && item.id === reasoningID,
  )
}

export * as SessionMessageState from "./session-message-state"
