import { describe, expect, test } from "bun:test"
import { MessageV2 } from "../../src/session/message"
import { ModelID, ProviderID } from "../../src/provider/schema"
import { SessionID, MessageID, PartID } from "../../src/session/schema"
import type { Provider } from "@/provider/provider"

export const sessionID = SessionID.make("session")
export const providerID = ProviderID.make("test")
export const model: Provider.Model = {
  id: ModelID.make("test-model"),
  providerID,
  api: {
    id: "test-model",
    url: "https://example.com",
    npm: "@ai-sdk/openai",
  },
  name: "Test Model",
  capabilities: {
    temperature: true,
    reasoning: false,
    attachment: false,
    toolcall: true,
    input: {
      text: true,
      audio: false,
      image: false,
      video: false,
      pdf: false,
    },
    output: {
      text: true,
      audio: false,
      image: false,
      video: false,
      pdf: false,
    },
    interleaved: false,
  },
  cost: {
    input: 0,
    output: 0,
    cache: {
      read: 0,
      write: 0,
    },
  },
  limit: {
    context: 0,
    input: 0,
    output: 0,
  },
  status: "active",
  options: {},
  headers: {},
  release_date: "2026-01-01",
}

export function userInfo(id: string): MessageV2.User {
  return {
    id,
    sessionID,
    role: "user",
    time: { created: 0 },
    agent: "user",
    model: { providerID, modelID: ModelID.make("test") },
    tools: {},
    mode: "",
  } as unknown as MessageV2.User
}

export function assistantInfo(
  id: string,
  parentID: string,
  error?: MessageV2.Assistant["error"],
  meta?: { providerID: string; modelID: string },
): MessageV2.Assistant {
  const infoModel = meta ?? { providerID: model.providerID, modelID: model.api.id }
  return {
    id,
    sessionID,
    role: "assistant",
    time: { created: 0 },
    error,
    parentID,
    modelID: infoModel.modelID,
    providerID: infoModel.providerID,
    mode: "",
    agent: "agent",
    path: { cwd: "/", root: "/" },
    cost: 0,
    tokens: {
      input: 0,
      output: 0,
      reasoning: 0,
      cache: { read: 0, write: 0 },
    },
  } as unknown as MessageV2.Assistant
}

export function basePart(messageID: string, id: string) {
  return {
    id: PartID.make(id),
    sessionID,
    messageID: MessageID.make(messageID),
  }
}

describe("session.message input part schemas", () => {
  test("accept drafts with and without ids across all input variants", () => {
    const cases = [
      [
        MessageV2.TextPartInput,
        {
          type: "text",
          text: "hello",
          synthetic: true,
          ignored: false,
          time: { start: 1, end: 2 },
          metadata: { source: "test" },
        },
      ],
      [
        MessageV2.FilePartInput,
        {
          type: "file",
          mime: "image/png",
          filename: "image.png",
          url: "data:image/png;base64,AAAA",
        },
      ],
      [
        MessageV2.AgentPartInput,
        {
          type: "agent",
          name: "worker",
          source: { value: "src", start: 1, end: 2 },
        },
      ],
      [
        MessageV2.SubtaskPartInput,
        {
          type: "subtask",
          prompt: "do the thing",
          description: "task",
          agent: "worker",
          model: { providerID, modelID: ModelID.make("subtask-model") },
          command: "run",
        },
      ],
    ] as const

    for (const [schema, draft] of cases) {
      const parsed = schema.zod.parse(draft)
      expect(parsed).toMatchObject(draft)
      expect(schema.zod.parse({ ...draft, id: PartID.make("prt_test") })).toMatchObject({
        ...draft,
        id: PartID.make("prt_test"),
      })
    }
  })
})
