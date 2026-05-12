/** @jsxImportSource @opentui/solid */
import { describe, expect, test } from "bun:test"
import { testRender } from "@opentui/solid"
import { onMount } from "solid-js"
import { Global } from "@jekko-ai/core/global"
import { ArgsProvider } from "../../../../src/cli/cmd/tui/context/args"
import { ExitProvider } from "../../../../src/cli/cmd/tui/context/exit"
import { KVProvider, useKV } from "../../../../src/cli/cmd/tui/context/kv"
import { ProjectProvider } from "../../../../src/cli/cmd/tui/context/project"
import { SDKProvider, type EventSource } from "../../../../src/cli/cmd/tui/context/sdk"
import { SyncProvider, useSync } from "../../../../src/cli/cmd/tui/context/sync"
import { TuiConfigProvider } from "../../../../src/cli/cmd/tui/context/tui-config"
import { ThemeProvider } from "../../../../src/cli/cmd/tui/context/theme"
import { LocalProvider, useLocal } from "../../../../src/cli/cmd/tui/context/local"
import { ToastProvider } from "../../../../src/cli/cmd/tui/ui/toast"
import { tmpdir } from "../../../fixture/fixture"
import type { Event as SDKEvent, GlobalEvent, Message, Part, Provider } from "@jekko-ai/sdk/v2"

const worktree = "/tmp/jekko"
const directory = `${worktree}/packages/jekko`

async function wait(fn: () => boolean, timeout = 2000) {
  const start = Date.now()
  while (!fn()) {
    if (Date.now() - start > timeout) throw new Error("timed out waiting for condition")
    await Bun.sleep(10)
  }
}

function json(data: unknown) {
  return new Response(JSON.stringify(data), {
    headers: { "content-type": "application/json" },
  })
}

type ControlledEventSource = EventSource & {
  emit: (payload: SDKEvent, options?: Partial<Omit<GlobalEvent, "payload">>) => void
}

function eventSource(): ControlledEventSource {
  const handlers = new Set<(event: GlobalEvent) => void>()
  return {
    subscribe: async (handler) => {
      handlers.add(handler)
      return () => handlers.delete(handler)
    },
    emit: (payload, options) => {
      const event = {
        directory,
        ...options,
        payload,
      } as GlobalEvent
      for (const handler of handlers) handler(event)
    },
  }
}

function model(providerID: string, id: string, name: string, status: "active" | "locked" = "active") {
  return {
    id,
    providerID,
    api: {
      id: providerID,
      url: "https://example.test/v1",
      npm: "@ai-sdk/openai-compatible",
    },
    name,
    capabilities: {
      temperature: true,
      reasoning: true,
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
      context: 128000,
      output: 32000,
    },
    status,
    options: {},
    headers: {},
    release_date: "2026-05-07",
  } satisfies Provider["models"][string]
}

const jnoccioProvider = {
  id: "jnoccio",
  name: "Jnoccio",
  source: "config",
  env: [],
  options: {},
  models: {
    "jnoccio-fusion": model("jnoccio", "jnoccio-fusion", "Jnoccio Fusion"),
  },
} satisfies Provider

const lockedJnoccioProvider = {
  id: "jnoccio",
  name: "Jnoccio",
  source: "api",
  env: [],
  options: {},
  models: {
    "jnoccio-fusion": model("jnoccio", "jnoccio-fusion", "Jnoccio Fusion", "locked"),
  },
} satisfies Provider

const jekkoProvider = {
  id: "jekko",
  name: "jekko",
  source: "api",
  env: [],
  options: {},
  models: {
    "jekko-free": model("jekko", "jekko-free", "jekko Free"),
  },
} satisfies Provider

const anthropicProvider = {
  id: "anthropic",
  name: "Anthropic",
  source: "env",
  env: ["ANTHROPIC_API_KEY"],
  options: {},
  models: {
    "claude-test": model("anthropic", "claude-test", "Claude Test"),
  },
} satisfies Provider

function createFetch(options: { lockedJnoccio?: boolean } = {}) {
  const session = [] as URL[]
  const activeJnoccio = options.lockedJnoccio ? lockedJnoccioProvider : jnoccioProvider
  const fetch = (async (input: RequestInfo | URL) => {
    const url = new URL(input instanceof Request ? input.url : String(input))
    if (url.pathname === "/session") session.push(url)

    switch (url.pathname) {
      case "/command":
      case "/experimental/workspace":
      case "/experimental/workspace/status":
      case "/formatter":
      case "/lsp":
        return json([])
      case "/agent":
        return json([])
      case "/config":
        return json({ model: "jnoccio/jnoccio-fusion" })
      case "/experimental/resource":
      case "/mcp":
      case "/provider/auth":
      case "/session/status":
        return json({})
      case "/config/providers":
        return json({
          providers: [activeJnoccio],
          default: options.lockedJnoccio ? {} : { jnoccio: "jnoccio-fusion" },
        })
      case "/experimental/console":
        return json({ consoleManagedProviders: [], switchableOrgCount: 0 })
      case "/path":
        return json({ home: "", state: "", config: "", worktree, directory })
      case "/project/current":
        return json({ id: "proj_test" })
      case "/provider":
        return json({
          all: [activeJnoccio, jekkoProvider, anthropicProvider],
          default: {
            ...(options.lockedJnoccio ? {} : { jnoccio: "jnoccio-fusion" }),
            jekko: "jekko-free",
            anthropic: "claude-test",
          },
          connected: options.lockedJnoccio ? [] : ["jnoccio"],
        })
      case "/session":
        return json([])
      case "/vcs":
        return json({ branch: "main" })
    }

    throw new Error(`unexpected request: ${url.pathname}`)
  }) as typeof globalThis.fetch

  return { fetch, session }
}

async function mount(options?: Parameters<typeof createFetch>[0]) {
  const calls = createFetch(options)
  const events = eventSource()
  let sync!: ReturnType<typeof useSync>
  let kv!: ReturnType<typeof useKV>
  let local!: ReturnType<typeof useLocal>
  let done!: () => void
  const ready = new Promise<void>((resolve) => {
    done = resolve
  })

  const app = await testRender(() => (
    <ArgsProvider>
      <ExitProvider>
        <KVProvider>
          <ToastProvider>
            <TuiConfigProvider config={{}}>
              <SDKProvider url="http://test" directory={directory} fetch={calls.fetch} events={events}>
                <ProjectProvider>
                  <SyncProvider>
                    <ThemeProvider mode="dark">
                      <LocalProvider>
                        <Probe
                          onReady={(ctx) => {
                            sync = ctx.sync
                            kv = ctx.kv
                            local = ctx.local
                            done()
                          }}
                        />
                      </LocalProvider>
                    </ThemeProvider>
                  </SyncProvider>
                </ProjectProvider>
              </SDKProvider>
            </TuiConfigProvider>
          </ToastProvider>
        </KVProvider>
      </ExitProvider>
    </ArgsProvider>
  ))

  await ready
  await wait(() => sync.status === "complete")
  await wait(() => local.model.ready)
  return { app, events, kv, local, sync, session: calls.session }
}

function Probe(props: {
  onReady: (ctx: {
    kv: ReturnType<typeof useKV>
    local: ReturnType<typeof useLocal>
    sync: ReturnType<typeof useSync>
  }) => void
}) {
  const kv = useKV()
  const sync = useSync()
  const local = useLocal()

  onMount(() => {
    props.onReady({ kv, local, sync })
  })

  return <box />
}

function assistantMessage(id: string, sessionID: string) {
  return {
    id,
    sessionID,
    role: "assistant",
    time: { created: 1 },
    parentID: "msg_parent",
    modelID: "jnoccio-fusion",
    providerID: "jnoccio",
    mode: "build",
    agent: "build",
    path: { cwd: directory, root: worktree },
    cost: 0,
    tokens: {
      input: 0,
      output: 0,
      reasoning: 0,
      cache: { read: 0, write: 0 },
    },
  } satisfies Message
}

function reasoningPart(id: string, sessionID: string, messageID: string) {
  return {
    id,
    sessionID,
    messageID,
    type: "reasoning",
    text: "",
    time: { start: 1 },
  } satisfies Part
}

function textPart(id: string, sessionID: string, messageID: string) {
  return {
    id,
    sessionID,
    messageID,
    type: "text",
    text: "",
  } satisfies Part
}

function streamedText(part: Part | undefined) {
  return part && "text" in part ? part.text : undefined
}

describe("tui sync", () => {
  test("refresh scopes sessions by default and lists project sessions when disabled", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    const { app, kv, sync, session } = await mount()

    try {
      expect(kv.get("session_directory_filter_enabled", true)).toBe(true)
      expect(session.at(-1)?.searchParams.get("scope")).toBeNull()
      expect(session.at(-1)?.searchParams.get("path")).toBe("packages/jekko")

      kv.set("session_directory_filter_enabled", false)
      await sync.session.refresh()

      expect(session.at(-1)?.searchParams.get("scope")).toBe("project")
      expect(session.at(-1)?.searchParams.get("path")).toBeNull()
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })

  test("bootstrap loads providers before complete and config model is selected", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    await Bun.write(
      `${tmp.path}/model.json`,
      JSON.stringify({
        recent: [{ providerID: "jekko", modelID: "jekko-free" }],
        favorite: [],
        variant: {},
      }),
    )
    const { app, local, sync } = await mount()

    try {
      expect(sync.status).toBe("complete")
      expect(sync.data.provider.length).toBeGreaterThan(0)
      expect(sync.data.provider_next.all.length).toBeGreaterThan(1)
      expect(sync.data.provider_next.all.some((provider) => provider.id === "anthropic")).toBe(true)
      expect(sync.data.provider_default.jnoccio).toBe("jnoccio-fusion")
      expect(sync.data.config.model).toBe("jnoccio/jnoccio-fusion")
      expect(local.model.current()).toEqual({ providerID: "jnoccio", modelID: "jnoccio-fusion" })
      expect(local.model.parsed()).toMatchObject({
        provider: "Jnoccio",
        model: "Jnoccio Fusion",
      })
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })

  test("locked Jnoccio is listed but skipped for current model selection", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    await Bun.write(
      `${tmp.path}/model.json`,
      JSON.stringify({
        recent: [{ providerID: "jnoccio", modelID: "jnoccio-fusion" }],
        favorite: [{ providerID: "jnoccio", modelID: "jnoccio-fusion" }],
        variant: {},
      }),
    )
    const { app, local, sync } = await mount({ lockedJnoccio: true })

    try {
      expect(sync.status).toBe("complete")
      expect(sync.data.provider[0]?.models["jnoccio-fusion"]?.status).toBe("locked")
      expect(sync.data.provider_next.all.some((provider) => provider.id === "jnoccio")).toBe(true)
      expect(sync.data.provider_next.connected).not.toContain("jnoccio")
      expect(sync.data.provider_default.jnoccio).toBeUndefined()
      expect(local.model.current()).toBeUndefined()
      local.model.set({ providerID: "jnoccio", modelID: "jnoccio-fusion" }, { recent: true })
      expect(local.model.current()).toBeUndefined()
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })

  test("streams message part deltas into reasoning and text parts", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    const { app, events, sync } = await mount()

    const sessionID = "ses_stream"
    const messageID = "msg_stream"
    const reasoningID = "part_reasoning"
    const textID = "part_text"

    try {
      events.emit({
        id: "evt_message",
        type: "message.updated",
        properties: { sessionID, info: assistantMessage(messageID, sessionID) },
      })
      events.emit({
        id: "evt_reasoning",
        type: "message.part.updated",
        properties: {
          sessionID,
          part: reasoningPart(reasoningID, sessionID, messageID),
          time: 1,
        },
      })
      events.emit({
        id: "evt_reasoning_delta",
        type: "message.part.delta",
        properties: {
          sessionID,
          messageID,
          partID: reasoningID,
          field: "text",
          delta: "thinking...",
        },
      })
      events.emit({
        id: "evt_text",
        type: "message.part.updated",
        properties: {
          sessionID,
          part: textPart(textID, sessionID, messageID),
          time: 2,
        },
      })
      events.emit({
        id: "evt_text_delta",
        type: "message.part.delta",
        properties: {
          sessionID,
          messageID,
          partID: textID,
          field: "text",
          delta: "answer",
        },
      })

      await wait(
        () => streamedText(sync.data.part[messageID]?.find((part) => part.id === reasoningID)) === "thinking...",
      )
      expect(streamedText(sync.data.part[messageID]?.find((part) => part.id === textID))).toBe("answer")

      events.emit({
        id: "evt_text_removed",
        type: "message.part.removed",
        properties: { sessionID, messageID, partID: textID },
      })
      await wait(() => !sync.data.part[messageID]?.some((part) => part.id === textID))

      events.emit({
        id: "evt_message_removed",
        type: "message.removed",
        properties: { sessionID, messageID },
      })
      await wait(() => !sync.data.message[sessionID]?.some((message) => message.id === messageID))
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })
})
