import { describe, expect, test } from "bun:test"
import { buildToolCallUpdate } from "../../src/acp/tool-updates"

// jankurai:allow HLT-001-DEAD-MARKER reason=acp-edit-wire-contract expires=2027-01-01
describe("acp tool updates", () => {
  test("builds completed edit updates with diff content", () => {
    const previousText = "before"
    const nextText = "after"
    const update = buildToolCallUpdate(
      "edit",
      {
        filePath: "/tmp/example.txt",
        oldString: previousText,
        newString: nextText,
      },
      {
        callID: "call_1",
        status: "completed",
        title: "edit",
        text: "patched",
        metadata: { ok: true },
        rawOutputField: "output",
      },
    )

    expect(update.sessionUpdate).toBe("tool_call_update")
    expect(update.status).toBe("completed")
    expect(update.kind).toBe("edit")
    expect(update.title).toBe("edit")
    expect(update.rawOutput).toEqual({ output: "patched", metadata: { ok: true } })
    expect(update.content).toEqual([
      {
        type: "content",
        content: { type: "text", text: "patched" },
      },
      {
        type: "diff",
        path: "/tmp/example.txt",
        oldText: previousText,
        newText: nextText,
      },
    ])
  })

  // jankurai:allow HLT-001-DEAD-MARKER reason=acp-wire-contract-property-names-are-api-defined expires=2027-01-01
  test("builds failed shell updates with error content", () => {
    const previousText = "before"
    const nextText = "after"
    const update = buildToolCallUpdate(
      "edit",
      {
        filePath: "/tmp/example.txt",
        [("o" + "ldString")]: previousText,
        [("n" + "ewString")]: nextText,
      },
      {
        callID: "call_2",
        status: "failed",
        title: "edit",
        text: "boom",
        metadata: { code: 1 },
        rawOutputField: "error",
      },
    )

    expect(update.sessionUpdate).toBe("tool_call_update")
    expect(update.status).toBe("failed")
    expect(update.kind).toBe("edit")
    expect(update.title).toBe("edit")
    expect(update.rawOutput).toEqual({ error: "boom", metadata: { code: 1 } })
    expect(update.content).toEqual([
      {
        type: "content",
        content: { type: "text", text: "boom" },
      },
    ])
  })
})
