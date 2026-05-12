import type { ToolCallContent, ToolKind } from "@agentclientprotocol/sdk"

function buildToolCallContent(
  toolKind: ToolKind,
  input: Record<string, any>,
  output: string,
  includeDiff: boolean,
): ToolCallContent[] {
  const content: ToolCallContent[] = [
    {
      type: "content",
      content: {
        type: "text",
        text: output,
      },
    },
  ]

  if (includeDiff && toolKind === "edit") {
    const filePath = typeof input["filePath"] === "string" ? input["filePath"] : ""
    const oldText = typeof input["oldString"] === "string" ? input["oldString"] : ""
    const newText =
      typeof input["newString"] === "string"
        ? input["newString"]
        : typeof input["content"] === "string"
          ? input["content"]
          : ""
    content.push({
      type: "diff",
      path: filePath,
      oldText,
      newText,
    })
  }

  return content
}

export function buildToolCallUpdate(
  toolKind: ToolKind,
  input: Record<string, any>,
  part: {
    callID: string
    status: "completed" | "failed"
    title: string
    text: string
    metadata: unknown
    rawOutputField: "output" | "error"
  },
) {
  const rawOutput =
    part.rawOutputField === "output"
    ? { output: part.text, metadata: part.metadata }
      : { error: part.text, metadata: part.metadata }
  const includeDiff = part.rawOutputField === "output"

  return {
    sessionUpdate: "tool_call_update" as const,
    toolCallId: part.callID,
    status: part.status,
    kind: toolKind,
    content: buildToolCallContent(toolKind, input, part.text, includeDiff),
    title: part.title,
    rawInput: input,
    rawOutput,
  }
}
