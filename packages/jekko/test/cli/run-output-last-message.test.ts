import { afterEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import { createLastAssistantMessageTracker } from "../../src/cli/cmd/run"

const tempDirs: string[] = []

async function tempFile() {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "jekko-run-last-message-"))
  tempDirs.push(root)
  return {
    root,
    file: path.join(root, "final-message.md"),
  }
}

function assistantMessage(sessionID: string, id: string) {
  return {
    type: "message.updated",
    properties: {
      sessionID,
      info: {
        id,
        role: "assistant",
      },
    },
  }
}

function textPart(sessionID: string, messageID: string, id: string, text: string) {
  return {
    type: "message.part.updated",
    properties: {
      sessionID,
      time: 1,
      part: {
        id,
        sessionID,
        messageID,
        type: "text",
        text,
        time: { start: 1, end: 2 },
      },
    },
  }
}

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
})

describe("createLastAssistantMessageTracker", () => {
  test("tracks the latest assistant message and preserves text part order", async () => {
    const tracker = createLastAssistantMessageTracker({ sessionID: "ses_1" })
    tracker.observe(assistantMessage("ses_1", "msg_1"))
    tracker.observe(textPart("ses_1", "msg_1", "part_1", "hello "))
    tracker.observe(textPart("ses_1", "msg_1", "part_2", "world"))
    tracker.observe(textPart("ses_1", "msg_1", "part_1", "good "))
    tracker.observe(textPart("ses_1", "msg_1", "part_2", "bye"))
    tracker.observe(assistantMessage("ses_1", "msg_2"))
    tracker.observe(textPart("ses_1", "msg_2", "part_3", "fresh "))
    tracker.observe(textPart("ses_1", "msg_2", "part_4", "output"))

    const tmp = await tempFile()
    await tracker.write(tmp.file)

    expect(tracker.text()).toBe("fresh output")
    await expect(fsp.readFile(tmp.file, "utf8")).resolves.toBe("fresh output")
  })

  test("writes the same file even when the event stream is JSON-shaped", async () => {
    const tracker = createLastAssistantMessageTracker({ sessionID: "ses_json" })
    tracker.observe(assistantMessage("ses_json", "msg_json"))
    tracker.observe(textPart("ses_json", "msg_json", "part_json", "json final"))

    const tmp = await tempFile()
    await tracker.write(tmp.file)

    await expect(fsp.readFile(tmp.file, "utf8")).resolves.toBe("json final")
  })

  test("writes an empty file when no assistant text exists", async () => {
    const tracker = createLastAssistantMessageTracker({ sessionID: "ses_empty" })

    const tmp = await tempFile()
    await tracker.write(tmp.file)

    await expect(fsp.readFile(tmp.file, "utf8")).resolves.toBe("")
  })
})
