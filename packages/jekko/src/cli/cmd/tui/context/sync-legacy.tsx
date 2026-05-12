// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import type {
  Message,
  Agent,
  Provider,
  Session,
  Part,
  Config,
  Command,
  PermissionRequest,
  QuestionRequest,
  LspStatus,
  McpStatus,
  McpResource,
  FormatterStatus,
  SessionStatus,
  ProviderListResponse,
  ProviderAuthMethod,
  VcsInfo,
} from "@jekko-ai/sdk/v2"
import { createStore, produce, reconcile } from "solid-js/store"
import { batch, onMount } from "solid-js"
import { useProject } from "@tui/context/project"
import { useEvent } from "@tui/context/event"
import { useSDK } from "@tui/context/sdk"
import { Binary } from "@jekko-ai/core/util/binary"
import { createSimpleContext } from "./helper"
import type { Snapshot } from "@/snapshot"
import { emptyConsoleState, type ConsoleState } from "@/config/console-state"
import path from "path"
import { useKV } from "./kv"
import { useExit } from "./exit"
import * as Log from "@jekko-ai/core/util/log"

type PendingItem = import("@jekko-ai/sdk/v2").SessionPendingResponse extends Array<infer Item> ? Item : never

export const { use: useSync, provider: SyncProvider } = createSimpleContext({
  name: "Sync",
  init: () => {
    const [store, setStore] = createStore<{
      status: "loading" | "partial" | "complete"
      provider: Provider[]
      provider_default: Record<string, string>
      provider_next: ProviderListResponse
      console_state: ConsoleState
      provider_auth: Record<string, ProviderAuthMethod[]>
      agent: Agent[]
      command: Command[]
      permission: {
        [sessionID: string]: PermissionRequest[]
      }
      question: {
        [sessionID: string]: QuestionRequest[]
      }
      config: Config
      session: Session[]
      session_status: {
        [sessionID: string]: SessionStatus
      }
      session_diff: {
        [sessionID: string]: Snapshot.FileDiff[]
      }
      pending: {
        [sessionID: string]: PendingItem[]
      }
      message: {
        [sessionID: string]: Message[]
      }
      part: {
        [messageID: string]: Part[]
      }
      lsp: LspStatus[]
      mcp: {
        [key: string]: McpStatus
      }
      mcp_resource: {
        [key: string]: McpResource
      }
      formatter: FormatterStatus[]
      vcs: VcsInfo | undefined
    }>({
      provider_next: {
        all: [],
        default: {},
        connected: [],
      },
      console_state: emptyConsoleState,
      provider_auth: {},
      config: {},
      status: "loading",
      agent: [],
      permission: {},
      question: {},
      command: [],
      provider: [],
      provider_default: {},
      session: [],
      session_status: {},
      session_diff: {},
      pending: {},
      message: {},
      part: {},
      lsp: [],
      mcp: {},
      mcp_resource: {},
      formatter: [],
      vcs: undefined,
    })

    const event = useEvent()
    const project = useProject()
    const sdk = useSDK()
    const kv = useKV()
    const exit = useExit()

    const fullSyncedSessions = new Set<string>()
    let syncedWorkspace = project.workspace.current()
    const pendingUpdatedEvent = "pending.updated"

    function sessionListQuery(): { scope?: "project"; path?: string } {
      if (!kv.get("session_directory_filter_enabled", true)) return { scope: "project" }
      if (!project.data.instance.path.worktree || !project.data.instance.path.directory) return { scope: "project" }
      return {
        path: path
          .relative(path.resolve(project.data.instance.path.worktree), project.data.instance.path.directory)
          .replaceAll("\\", "/"),
      }
    }

    function listSessions() {
      return sdk.client.session
        .list({ start: Date.now() - 30 * 24 * 60 * 60 * 1000, ...sessionListQuery() })
        .then((x) => (x.data ?? []).toSorted((a, b) => a.id.localeCompare(b.id)))
    }

    async function refresh() {
      const session = await listSessions()
      setStore("session", session)
      return session
    }

    async function syncSessionMessages(sessionID: string) {
      const response = await sdk.client.session.messages({ sessionID })
      const items = response.data ?? []
      setStore(
        "message",
        sessionID,
        items.map((item) => item.info),
      )
      for (const item of items) {
        setStore("part", item.info.id, reconcile(item.parts))
      }
    }

    async function syncSession(sessionID: string) {
      if (fullSyncedSessions.has(sessionID)) return store.session.find((item) => item.id === sessionID)

      const session = await sdk.client.session.get({ sessionID })
      const pendingResponse = await sdk.client.session.pending({ sessionID })
      const diff = await sdk.client.session.diff({ sessionID })

      if (session.data) {
        const current = store.session
        const index = current.findIndex((item) => item.id === sessionID)
        if (index >= 0) setStore("session", index, reconcile(session.data))
        else setStore("session", current.length, session.data)
      }

      setStore("pending", sessionID, pendingResponse.data ?? [])
      setStore("session_diff", sessionID, diff.data ?? [])

      await syncSessionMessages(sessionID)
      fullSyncedSessions.add(sessionID)
      return session.data
    }

    async function bootstrap(options: { fatal?: boolean } = {}) {
      const fatal = options.fatal ?? true

      try {
        await project.sync()
        await project.workspace.sync()

        const workspace = project.workspace.current()
        if (workspace !== syncedWorkspace) {
          fullSyncedSessions.clear()
          syncedWorkspace = workspace
        }

        const [providers, providerList, agents, config, consoleState] = await Promise.all([
          sdk.client.config.providers({ workspace }, { throwOnError: true }),
          sdk.client.provider.list({ workspace }, { throwOnError: true }),
          sdk.client.app.agents({ workspace }, { throwOnError: true }),
          sdk.client.config.get({ workspace }, { throwOnError: true }),
          sdk.client.experimental.console
            .get({ workspace }, { throwOnError: true })
            .then((response) => response.data ?? emptyConsoleState)
            .catch(() => emptyConsoleState),
        ])

        batch(() => {
          setStore("provider", reconcile(providers.data.providers))
          setStore("provider_default", reconcile(providers.data.default))
          setStore("provider_next", reconcile(providerList.data))
          setStore("agent", reconcile(agents.data ?? []))
          setStore("config", reconcile(config.data))
          setStore("console_state", reconcile(consoleState))
        })

        if (store.status !== "complete") setStore("status", "partial")

        void Promise.all([
          refresh(),
          sdk.client.command.list({ workspace }).then((x) => setStore("command", reconcile(x.data ?? []))),
          sdk.client.lsp.status({ workspace }).then((x) => setStore("lsp", reconcile(x.data ?? []))),
          sdk.client.mcp.status({ workspace }).then((x) => setStore("mcp", reconcile(x.data ?? {}))),
          sdk.client.experimental.resource
            .list({ workspace })
            .then((x) => setStore("mcp_resource", reconcile(x.data ?? {}))),
          sdk.client.formatter.status({ workspace }).then((x) => setStore("formatter", reconcile(x.data ?? []))),
          sdk.client.session.status({ workspace }).then((x) => setStore("session_status", reconcile(x.data ?? {}))),
          sdk.client.provider.auth({ workspace }).then((x) => setStore("provider_auth", reconcile(x.data ?? {}))),
          sdk.client.vcs.get({ workspace }).then((x) => setStore("vcs", reconcile(x.data))),
          project.workspace.sync(),
        ]).then(() => {
          setStore("status", "complete")
        })
      } catch (error) {
        Log.Default.error("tui bootstrap failed", {
          error: error instanceof Error ? error.message : String(error),
          name: error instanceof Error ? error.name : undefined,
          stack: error instanceof Error ? error.stack : undefined,
        })
        if (fatal) await exit(error)
        else throw error
      }
    }

    event.subscribe((event) => {
      const evt = event as any
      switch (evt.type) {
        case "server.instance.disposed":
          void bootstrap()
          break
        case "permission.replied": {
          const requests = store.permission[evt.properties.sessionID]
          if (!requests) break
          const match = Binary.search(requests, evt.properties.requestID, (r) => r.id)
          if (!match.found) break
          setStore(
            "permission",
            evt.properties.sessionID,
            produce((draft) => {
              draft.splice(match.index, 1)
            }),
          )
          break
        }

        case "permission.asked": {
          const request = evt.properties
          const requests = store.permission[request.sessionID]
          if (!requests) {
            setStore("permission", request.sessionID, [request])
            break
          }
          const match = Binary.search(requests, request.id, (r) => r.id)
          if (match.found) {
            setStore("permission", request.sessionID, match.index, reconcile(request))
            break
          }
          setStore(
            "permission",
            request.sessionID,
            produce((draft) => {
              draft.splice(match.index, 0, request)
            }),
          )
          break
        }

        case "question.replied":
        case "question.rejected": {
          const requests = store.question[evt.properties.sessionID]
          if (!requests) break
          const match = Binary.search(requests, evt.properties.requestID, (r) => r.id)
          if (!match.found) break
          setStore(
            "question",
            evt.properties.sessionID,
            produce((draft) => {
              draft.splice(match.index, 1)
            }),
          )
          break
        }

        case "question.asked": {
          const request = evt.properties
          const requests = store.question[request.sessionID]
          if (!requests) {
            setStore("question", request.sessionID, [request])
            break
          }
          const match = Binary.search(requests, request.id, (r) => r.id)
          if (match.found) {
            setStore("question", request.sessionID, match.index, reconcile(request))
            break
          }
          setStore(
            "question",
            request.sessionID,
            produce((draft) => {
              draft.splice(match.index, 0, request)
            }),
          )
          break
        }

        case pendingUpdatedEvent:
          setStore("pending", evt.properties.sessionID, evt.properties.todos)
          break

        case "session.diff":
          setStore("session_diff", evt.properties.sessionID, evt.properties.diff)
          break

        case "session.deleted": {
          const result = Binary.search(store.session, evt.properties.info.id, (s) => s.id)
          if (result.found) {
            setStore(
              "session",
              produce((draft) => {
                draft.splice(result.index, 1)
              }),
            )
          }
          break
        }
        case "session.updated": {
          const result = Binary.search(store.session, evt.properties.info.id, (s) => s.id)
          if (result.found) {
            setStore("session", result.index, reconcile(evt.properties.info))
            break
          }
          setStore(
            "session",
            produce((draft) => {
              draft.splice(result.index, 0, evt.properties.info)
            }),
          )
          break
        }

        case "session.status": {
          setStore("session_status", evt.properties.sessionID, evt.properties.status)
          break
        }

        case "message.updated": {
          const messages = store.message[evt.properties.info.sessionID]
          if (!messages) {
            setStore("message", evt.properties.info.sessionID, [evt.properties.info])
            break
          }
          const result = Binary.search(messages, evt.properties.info.id, (m) => m.id)
          if (result.found) {
            setStore("message", evt.properties.info.sessionID, result.index, reconcile(evt.properties.info))
            break
          }
          setStore(
            "message",
            evt.properties.info.sessionID,
            produce((draft) => {
              draft.splice(result.index, 0, evt.properties.info)
            }),
          )
          break
        }
        case "message.removed": {
          const messages = store.message[evt.properties.sessionID]
          if (!messages) break
          const result = Binary.search(messages, evt.properties.messageID, (m) => m.id)
          if (!result.found) break
          setStore(
            "message",
            evt.properties.sessionID,
            produce((draft) => {
              draft.splice(result.index, 1)
            }),
          )
          break
        }
        case "message.part.updated": {
          const part = evt.properties.part
          const parts = store.part[part.messageID]
          if (!parts) {
            setStore("part", part.messageID, [part])
            break
          }
          const result = Binary.search(parts, part.id, (m) => m.id)
          if (result.found) {
            setStore("part", part.messageID, result.index, reconcile(part))
            break
          }
          setStore(
            "part",
            part.messageID,
            produce((draft) => {
              draft.splice(result.index, 0, part)
            }),
          )
          break
        }
        case "message.part.delta": {
          const parts = store.part[evt.properties.messageID]
          if (!parts) break
          const result = Binary.search(parts, evt.properties.partID, (part) => part.id)
          if (!result.found) break
          setStore(
            "part",
            evt.properties.messageID,
            result.index,
            produce((draft) => {
              const field = evt.properties.field
              const value = (draft as Record<string, unknown>)[field]
              if (typeof value === "string") (draft as Record<string, unknown>)[field] = value + evt.properties.delta
            }),
          )
          break
        }
        case "message.part.removed": {
          const parts = store.part[evt.properties.messageID]
          if (!parts) break
          const result = Binary.search(parts, evt.properties.partID, (part) => part.id)
          if (!result.found) break
          setStore(
            "part",
            evt.properties.messageID,
            produce((draft) => {
              draft.splice(result.index, 1)
            }),
          )
          break
        }

        case "lsp.updated":
          setStore("lsp", evt.properties.status)
          break

        case "mcp.updated":
          setStore("mcp", evt.properties.name, evt.properties.status)
          break
        case "mcp.resource.updated":
          setStore("mcp_resource", evt.properties.resource.key, evt.properties.resource)
          break

        case "formatter.updated":
          setStore("formatter", evt.properties.status)
          break

        case "session.updated":
          setStore("session", evt.properties.info)
          break
        case "project.updated":
          break
      }
    })

    onMount(() => {
      void bootstrap()
    })

    return {
      data: store,
      get path() {
        return project.instance.path()
      },
      get ready() {
        if (process.env.JEKKO_FAST_BOOT) return true
        return store.status !== "loading"
      },
      get status() {
        return store.status
      },
      set: setStore,
      bootstrap,
      session: {
        get(sessionID: string) {
          const match = Binary.search(store.session, sessionID, (s) => s.id)
          if (match.found) return store.session[match.index]
          // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
          return undefined
        },
        query() {
          return sessionListQuery()
        },
        status(sessionID: string) {
          const session = store.session.find((item) => item.id === sessionID)
          if (!session) return "idle"
          if (session.time.compacting) return "compacting"
          const messages = store.message[sessionID] ?? []
          const last = messages.at(-1)
          if (!last) return "idle"
          if (last.role === "user") return "working"
          return last.time.completed ? "idle" : "working"
        },
        refresh,
        sync: syncSession,
        message: {
          sync: syncSessionMessages,
          fromSession(sessionID: string) {
            return store.message[sessionID] ?? []
          },
        },
      },
    }
  },
})
