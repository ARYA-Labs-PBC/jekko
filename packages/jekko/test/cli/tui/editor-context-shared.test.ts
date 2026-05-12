import { mkdir, writeFile } from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import { expect, spyOn, test } from "bun:test"
import {
  editorSelectionKey,
  resolveEditorConnection,
  type EditorSelection,
} from "../../../src/cli/cmd/tui/context/editor-shared"
import { tmpdir } from "../../fixture/fixture"

test("editorSelectionKey keeps the editor selection identity stable", () => {
  const selection: EditorSelection = {
    filePath: "/tmp/example.ts",
    source: "websocket",
    ranges: [
      {
        text: "hello",
        selection: {
          start: { line: 1, character: 1 },
          end: { line: 1, character: 6 },
        },
      },
      {
        text: "world",
        selection: {
          start: { line: 2, character: 1 },
          end: { line: 2, character: 6 },
        },
      },
    ],
  }

  expect(editorSelectionKey(selection)).toBe(
    [
      "/tmp/example.ts",
      1,
      1,
      1,
      6,
      "hello",
      2,
      1,
      2,
      6,
      "world",
    ].join("\0"),
  )
  expect(editorSelectionKey(undefined)).toBe("")
})

test("resolveEditorConnection prefers the configured port over lock files", async () => {
  await using tmp = await tmpdir()
  const ideDirectory = path.join(tmp.path, ".claude", "ide")
  await mkdir(ideDirectory, { recursive: true })
  await writeFile(
    path.join(ideDirectory, "3001.lock"),
    JSON.stringify({
      transport: "ws",
      workspaceFolders: [tmp.path],
    }),
  )

  const homedir = spyOn(os, "homedir").mockImplementation(() => tmp.path)
  const previousClaudePort = process.env.CLAUDE_CODE_SSE_PORT
  const previousOpencodePort = process.env.JEKKO_EDITOR_SSE_PORT
  process.env.CLAUDE_CODE_SSE_PORT = "4010"
  process.env.JEKKO_EDITOR_SSE_PORT = undefined

  try {
    expect(resolveEditorConnection(tmp.path)).toEqual({
      url: "ws://127.0.0.1:4010",
      source: "env:4010",
    })
  } finally {
    if (previousClaudePort === undefined) delete process.env.CLAUDE_CODE_SSE_PORT
    else process.env.CLAUDE_CODE_SSE_PORT = previousClaudePort
    if (previousOpencodePort === undefined) delete process.env.JEKKO_EDITOR_SSE_PORT
    else process.env.JEKKO_EDITOR_SSE_PORT = previousOpencodePort
    homedir.mockRestore()
  }
})
