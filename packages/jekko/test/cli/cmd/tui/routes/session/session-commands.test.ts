import { describe, expect, test } from "bun:test"
import { buildSessionTranscript } from "@/cli/cmd/tui/routes/session/session-commands"
import { formatTranscript } from "@/cli/cmd/tui/util/transcript"

describe("session commands transcript helpers", () => {
  test("buildSessionTranscript matches the shared transcript formatter", () => {
    const session = {
      id: "ses_abc123",
      title: "Test Session",
      time: { created: 1000000000000, updated: 1000000001000 },
    }
    const messages = [
      {
        info: {
          id: "msg_1",
          sessionID: "ses_abc123",
          role: "assistant" as const,
          agent: "build",
          modelID: "claude-sonnet-4-20250514",
          providerID: "anthropic",
          mode: "",
          parentID: "msg_0",
          path: { cwd: "/test", root: "/test" },
          cost: 0.001,
          tokens: { input: 100, output: 50, reasoning: 0, cache: { read: 0, write: 0 } },
          time: { created: 1000000000100, completed: 1000000000600 },
        },
        parts: [{ id: "p1", sessionID: "ses_abc123", messageID: "msg_1", type: "text" as const, text: "Response" }],
      },
    ]
    const sync = {
      data: {
        part: {
          msg_1: messages[0].parts,
        },
        provider: [
          {
            id: "anthropic",
            models: {
              "claude-sonnet-4-20250514": {
                name: "Claude Sonnet 4",
                limit: { context: 200000 },
              },
            },
          },
        ],
      },
    }

    const expected = formatTranscript(session, messages, {
      thinking: false,
      toolDetails: false,
      assistantMetadata: true,
      providers: sync.data.provider,
    })

    const actual = buildSessionTranscript(session, [messages[0].info], sync, false, false, true)

    expect(actual).toBe(expected)
  })
})
