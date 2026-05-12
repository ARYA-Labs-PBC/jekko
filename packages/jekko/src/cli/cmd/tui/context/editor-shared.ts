import { readdirSync, readFileSync, statSync } from "node:fs"
import os from "node:os"
import path from "node:path"
import z from "zod"

export const MCP_PROTOCOL_VERSION = "2025-11-25"

export const JsonRpcMessageSchema = z.object({
  id: z.union([z.number(), z.string(), z.null()]).optional(),
  method: z.string().optional(),
  params: z.unknown().optional(),
  result: z.unknown().optional(),
  error: z
    .object({
      code: z.number().optional(),
      message: z.string().optional(),
    })
    .optional(),
})

const PositionSchema = z.object({
  line: z.number(),
  character: z.number(),
})

const EditorSelectionRangeSchema = z.object({
  text: z.string(),
  selection: z.object({
    start: PositionSchema,
    end: PositionSchema,
  }),
})

export const EditorSelectionSchema = z
  .union([
    z.object({
      filePath: z.string(),
      source: z.enum(["websocket", "zed"]).optional(),
      ranges: z.array(EditorSelectionRangeSchema).min(1),
    }),
    z.object({
      text: z.string(),
      filePath: z.string(),
      source: z.enum(["websocket", "zed"]).optional(),
      selection: z.object({
        start: PositionSchema,
        end: PositionSchema,
      }),
    }),
  ])
  .transform((value) =>
    "ranges" in value
      ? value
      : {
          filePath: value.filePath,
          source: value.source,
          ranges: [
            {
              text: value.text,
              selection: value.selection,
            },
          ],
        },
  )

export const EditorMentionSchema = z.object({
  filePath: z.string(),
  lineStart: z.number(),
  lineEnd: z.number(),
})

export const EditorServerInfoSchema = z.object({
  protocolVersion: z.string().optional(),
  serverInfo: z
    .object({
      name: z.string().optional(),
      version: z.string().optional(),
    })
    .optional(),
})

export const EditorLockFileSchema = z.object({
  authToken: z.string().optional(),
  transport: z.literal("ws").optional(),
  workspaceFolders: z.array(z.string()).optional(),
})

export type JsonRpcMessage = z.infer<typeof JsonRpcMessageSchema>
export type EditorSelection = z.infer<typeof EditorSelectionSchema>
export type EditorMention = z.infer<typeof EditorMentionSchema>
export type EditorLabelState = "pending" | "sent" | "none"
export type EditorServerInfo = z.infer<typeof EditorServerInfoSchema>

export type EditorConnection = {
  url: string
  authToken?: string
  source: string
}

export type EditorLockFile = {
  port: number
  authToken?: string
  transport?: string
  workspaceFolders: string[]
  mtimeMs: number
}

export function parsePort(value: string | undefined) {
  if (!value) return

  const parsed = Number.parseInt(value, 10)
  if (!Number.isInteger(parsed) || parsed <= 0 || parsed > 65535) return
  return parsed
}

export function resolveEditorConnection(directory: string): EditorConnection | undefined {
  const port = parsePort(process.env.CLAUDE_CODE_SSE_PORT || process.env.JEKKO_EDITOR_SSE_PORT)
  if (port) {
    return {
      url: `ws://127.0.0.1:${port}`,
      source: `env:${port}`,
    }
  }

  const lock = resolveEditorLockFile(directory)
  if (lock) {
    return {
      url: `ws://127.0.0.1:${lock.port}`,
      authToken: lock.authToken,
      source: `lock:${lock.port}`,
    }
  }
}

export function resolveEditorLockFile(activeDirectory: string) {
  const directory = path.join(os.homedir(), ".claude", "ide")
  let entries: string[]

  try {
    entries = readdirSync(directory)
  } catch {
    return
  }

  const bestMatchLength = (lock: EditorLockFile) =>
    Math.max(0, ...lock.workspaceFolders.map((folder) => pathContainsLength(folder, activeDirectory)))
  const locks = entries
    .filter((entry) => entry.endsWith(".lock"))
    .map((entry) => readEditorLockFile(path.join(directory, entry)))
    .filter((entry): entry is EditorLockFile => Boolean(entry))
    .filter((entry) => bestMatchLength(entry) > 0)
    .sort((left, right) => bestMatchLength(right) - bestMatchLength(left) || right.mtimeMs - left.mtimeMs)
  return locks[0]
}

export function readEditorLockFile(filePath: string): EditorLockFile | undefined {
  const port = parsePort(path.basename(filePath, ".lock"))
  if (!port) return

  try {
    const parsed = EditorLockFileSchema.safeParse(JSON.parse(readFileSync(filePath, "utf-8")))
    if (!parsed.success) return

    return {
      port,
      authToken: parsed.data.authToken,
      transport: parsed.data.transport,
      workspaceFolders: parsed.data.workspaceFolders ?? [],
      mtimeMs: statSync(filePath).mtimeMs,
    }
  } catch {
    return
  }
}

export function editorSelectionKey(selection: EditorSelection | undefined) {
  if (!selection) return ""
  return [
    selection.filePath,
    ...selection.ranges.flatMap((range) => [
      range.selection.start.line,
      range.selection.start.character,
      range.selection.end.line,
      range.selection.end.character,
      range.text,
    ]),
  ].join("\0")
}

function pathContainsLength(parent: string, child: string) {
  const resolved = path.resolve(parent)
  const relative = path.relative(resolved, path.resolve(child))
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative)) ? resolved.length : 0
}

export function openEditorSocket(connection: EditorConnection, WebSocketImpl: typeof WebSocket) {
  if (!connection.authToken) return new WebSocketImpl(connection.url)

  return new WebSocketImpl(connection.url, {
    headers: {
      "x-claude-code-ide-authorization": connection.authToken,
    },
  } as any)
}

export function parseMessage(value: unknown) {
  if (typeof value !== "string") return

  try {
    return JsonRpcMessageSchema.parse(JSON.parse(value))
  } catch {
    return
  }
}
