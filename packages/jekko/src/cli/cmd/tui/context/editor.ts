import { onCleanup, onMount } from "solid-js"
import { createStore } from "solid-js/store"
import { createSimpleContext } from "./helper"
import {
  MCP_PROTOCOL_VERSION,
  type EditorLabelState,
  type EditorMention,
  EditorMentionSchema,
  type EditorSelection,
  EditorSelectionSchema,
  type EditorServerInfo,
  EditorServerInfoSchema,
  type JsonRpcMessage,
  editorSelectionKey,
  openEditorSocket,
  parseMessage,
  resolveEditorConnection,
} from "./editor-shared"
import { resolveZedDbPath, resolveZedSelection } from "./editor-zed"

export { editorSelectionKey } from "./editor-shared"
export type { EditorSelection, EditorLabelState, EditorMention } from "./editor-shared"

export const { use: useEditorContext, provider: EditorContextProvider } = createSimpleContext({
  name: "EditorContext",
  init: (props: { WebSocketImpl?: typeof WebSocket }) => {
    const mentionListeners = new Set<(mention: EditorMention) => void>()
    const WebSocketImpl = props.WebSocketImpl ?? WebSocket
    const [store, setStore] = createStore<{
      status: "disabled" | "connecting" | "connected"
      selection: EditorSelection | undefined
      selectionSent: boolean
      server: EditorServerInfo | undefined
    }>({
      status: "disabled",
      selection: undefined,
      selectionSent: false,
      server: undefined,
    })

    let socket: WebSocket | undefined
    let closed = false
    let reconnect: ReturnType<typeof setTimeout> | undefined
    let attempt = 0
    let requestID = 0
    let zedSelection: Promise<void> | undefined
    let lastZedSelectionKey: string | undefined
    let directory = process.cwd()
    let preserveSelectionOnReconnect = false
    const pending = new Map<number, string>()

    const setSelection = (selection: EditorSelection | undefined) => {
      const changed = editorSelectionKey(selection) !== editorSelectionKey(store.selection)
      setStore("selection", selection)
      if (changed) setStore("selectionSent", false)
    }

    const clearSelectionForReconnect = (options?: { resetZedSelectionKey?: boolean }) => {
      if (preserveSelectionOnReconnect) {
        preserveSelectionOnReconnect = false
        return
      }
      if (options?.resetZedSelectionKey) lastZedSelectionKey = undefined
      setSelection(undefined)
    }

    const send = (payload: JsonRpcMessage) => {
      if (!socket || socket.readyState !== 1) return
      socket.send(JSON.stringify({ jsonrpc: "2.0", ...payload }))
    }

    const request = (method: string, params?: unknown) => {
      requestID += 1
      pending.set(requestID, method)
      send({ id: requestID, method, params })
    }

    const connect = () => {
      if (closed) return

      const connection = resolveEditorConnection(directory)
      if (!connection) {
        const dbPath = resolveZedDbPath()
        if (!dbPath) {
          setStore("status", "disabled")
          scheduleReconnect()
          return
        }
        zedSelection ??= resolveZedSelection(dbPath, directory)
          .then((result) => {
            if (closed || socket) return
            if (result.type === "unavailable") return
            const selection = result.type === "selection" ? result.selection : undefined
            const key = editorSelectionKey(selection)
            if (key !== lastZedSelectionKey) {
              lastZedSelectionKey = key
              setSelection(selection)
              setStore("status", selection ? "connected" : "disabled")
            }
          })
          .catch(() => {
            // Keep the last known Zed selection for transient polling failures.
          })
          .finally(() => {
            zedSelection = undefined
          })
        scheduleZedPoll()
        return
      }

      setStore("status", "connecting")
      const current = openEditorSocket(connection, WebSocketImpl)
      socket = current

      current.addEventListener("open", () => {
        if (socket !== current) {
          current.close()
          return
        }

        attempt = 0
        setStore("status", "connected")
        request("initialize", {
          protocolVersion: MCP_PROTOCOL_VERSION,
          capabilities: {},
          clientInfo: { name: "jekko", version: "0.0.0" },
        })
      })

      current.addEventListener("message", (event) => {
        const message = parseMessage(event.data)
        if (!message) return

        const selection =
          message.method === "selection_changed" ? EditorSelectionSchema.safeParse(message.params) : undefined
        if (selection?.success) {
          setSelection({ ...selection.data, source: "websocket" })
          return
        }

        const mention = message.method === "at_mentioned" ? EditorMentionSchema.safeParse(message.params) : undefined
        if (mention?.success) {
          mentionListeners.forEach((listener) => listener(mention.data))
          return
        }

        if (typeof message.id !== "number") return

        const method = pending.get(message.id)
        if (!method) return

        pending.delete(message.id)
        if (message.error) return

        const initialize = method === "initialize" ? EditorServerInfoSchema.safeParse(message.result) : undefined
        if (initialize?.success) {
          setStore("server", initialize.data)
          send({ method: "notifications/initialized" })
          return
        }
      })

      current.addEventListener("close", () => {
        if (socket !== current) return

        socket = undefined
        pending.clear()
        if (closed) return

        setStore("status", "connecting")
        scheduleReconnect()
      })
    }

    const scheduleReconnect = () => {
      if (closed) return
      if (reconnect) clearTimeout(reconnect)
      attempt += 1
      const delay = Math.min(1000 * 2 ** (attempt - 1), 10_000)
      reconnect = setTimeout(connect, delay)
    }

    const scheduleZedPoll = () => {
      if (closed) return
      if (reconnect) clearTimeout(reconnect)
      reconnect = setTimeout(connect, 1000)
    }

    const reconnectWithDirectory = (nextDirectory?: string) => {
      const resolved = nextDirectory || process.cwd()
      const sameDirectory = directory === resolved
      clearSelectionForReconnect({ resetZedSelectionKey: !sameDirectory })
      if (sameDirectory) return

      directory = resolved
      attempt = 0
      pending.clear()
      if (reconnect) clearTimeout(reconnect)
      reconnect = undefined
      if (socket) {
        const current = socket
        socket = undefined
        current.close()
      }
      setStore("status", "disabled")
      setStore("server", undefined)
      connect()
    }

    onMount(() => {
      connect()

      onCleanup(() => {
        closed = true
        if (reconnect) clearTimeout(reconnect)
        socket?.close()
      })
    })

    return {
      enabled() {
        return Boolean(resolveEditorConnection(directory) || resolveZedDbPath())
      },
      connected() {
        return store.status === "connected"
      },
      selection() {
        return store.selection
      },
      clearSelection() {
        lastZedSelectionKey = undefined
        zedSelection = undefined
        setSelection(undefined)
      },
      preserveSelectionFromNewSession() {
        preserveSelectionOnReconnect = true
      },
      markSelectionSent() {
        if (!store.selection) return
        setStore("selectionSent", true)
      },
      labelState(): EditorLabelState {
        if (!store.selection) return "none"
        return store.selectionSent ? "sent" : "pending"
      },
      onMention(listener: (mention: EditorMention) => void) {
        mentionListeners.add(listener)
        return () => mentionListeners.delete(listener)
      },
      server() {
        return store.server
      },
      reconnect(directory?: string) {
        reconnectWithDirectory(directory)
      },
    }
  },
})
