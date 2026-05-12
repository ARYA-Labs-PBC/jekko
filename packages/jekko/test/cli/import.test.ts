import { test, expect } from "bun:test"
import {
  parseShareUrl,
  shouldAttachShareAuthHeaders,
  transformShareData,
  type ShareData,
} from "../../src/cli/cmd/import"

// parseShareUrl tests
test("parses valid share URLs", () => {
  expect(parseShareUrl("https://opncd.ai/share/Jsj3hNIW")).toBe("Jsj3hNIW")
  expect(parseShareUrl("https://custom.example.com/share/abc123")).toBe("abc123")
  expect(parseShareUrl("http://localhost:3000/share/test_id-123")).toBe("test_id-123")
})

test("rejects invalid URLs", () => {
  expect(parseShareUrl("https://opncd.ai/s/Jsj3hNIW")).toBeNull() // historical format
  expect(parseShareUrl("https://opncd.ai/share/")).toBeNull()
  expect(parseShareUrl("https://opncd.ai/share/id/extra")).toBeNull()
  expect(parseShareUrl("not-a-url")).toBeNull()
})

test("only attaches share auth headers for same-origin URLs", () => {
  expect(shouldAttachShareAuthHeaders("https://control.example.com/share/abc", "https://control.example.com")).toBe(
    true,
  )
  expect(shouldAttachShareAuthHeaders("https://other.example.com/share/abc", "https://control.example.com")).toBe(false)
  expect(shouldAttachShareAuthHeaders("https://control.example.com:443/share/abc", "https://control.example.com")).toBe(
    true,
  )
  expect(shouldAttachShareAuthHeaders("not-a-url", "https://control.example.com")).toBe(false)
})

// transformShareData tests
test("transforms share data to storage format", () => {
  const data = [
    { type: "session", data: { id: "sess-1", title: "Test" } as any },
    { type: "message", data: { id: "msg-1", sessionID: "sess-1" } as any },
    { type: "part", data: { id: "part-1", messageID: "msg-1" } as any },
    { type: "part", data: { id: "part-2", messageID: "msg-1" } as any },
  ] as unknown as ShareData[]

  const result = transformShareData(data)

  expect(result.kind).toBe("ok")
  if (result.kind !== "ok") throw new Error("expected transformed share data")
  expect(String(result.data.info.id)).toBe("sess-1")
  expect(result.data.messages).toHaveLength(1)
  expect(result.data.messages[0].parts).toHaveLength(2)
})

test("returns empty states for invalid share data", () => {
  expect(transformShareData([])).toEqual({ kind: "empty", reason: "missing-session" })
  expect(transformShareData([{ type: "message", data: {} as any }])).toEqual({
    kind: "empty",
    reason: "missing-session",
  })
  expect(transformShareData([{ type: "session", data: { id: "s" } as any }])).toEqual({
    kind: "empty",
    reason: "missing-messages",
  })
})
