/** @jsxImportSource @opentui/solid */
import { describe, expect, test } from "bun:test"
import { testRender } from "@opentui/solid"
import { onMount } from "solid-js"
import type { Provider } from "@jekko-ai/sdk/v2"
import { Global } from "@jekko-ai/core/global"
import { ArgsProvider } from "../../../../src/cli/cmd/tui/context/args"
import { ExitProvider } from "../../../../src/cli/cmd/tui/context/exit"
import { KVProvider } from "../../../../src/cli/cmd/tui/context/kv"
import { ProjectProvider } from "../../../../src/cli/cmd/tui/context/project"
import { SDKProvider, type EventSource } from "../../../../src/cli/cmd/tui/context/sdk"
import { SyncProvider, useSync } from "../../../../src/cli/cmd/tui/context/sync"
import { TuiConfigProvider } from "../../../../src/cli/cmd/tui/context/tui-config"
import { ThemeProvider } from "../../../../src/cli/cmd/tui/context/theme"
import { LocalProvider, useLocal } from "../../../../src/cli/cmd/tui/context/local"
import { DialogProvider, useDialog, type DialogContext } from "../../../../src/cli/cmd/tui/ui/dialog"
import { ToastProvider } from "../../../../src/cli/cmd/tui/ui/toast"
import { DialogJnoccioUnlock } from "../../../../src/cli/cmd/tui/component/dialog-jnoccio-unlock"
import { tmpdir } from "../../../fixture/fixture"

const worktree = "/tmp/jekko"
const directory = `${worktree}/packages/jekko`
const secret = "A".repeat(128)

async function wait(fn: () => boolean, timeout = 3000) {
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

function jnoccioProvider(status: "active" | "locked") {
  return {
    id: "jnoccio",
    name: "Jnoccio",
    source: status === "active" ? "config" : "api",
    env: [],
    options: status === "active" ? { baseURL: "http://127.0.0.1:4317/v1" } : {},
    models: {
      "jnoccio-fusion": model("jnoccio", "jnoccio-fusion", "Jnoccio Fusion", status),
    },
  } satisfies Provider
}

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

function createFetch() {
  const unlockBodies: unknown[] = []
  const dispose: URL[] = []
  let unlocked = false

  const fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const request = input instanceof Request ? input : new Request(input, init)
    const url = new URL(request.url)
    const activeJnoccio = jnoccioProvider(unlocked ? "active" : "locked")

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
          default: unlocked ? { jnoccio: "jnoccio-fusion" } : {},
        })
      case "/experimental/console":
        return json({ consoleManagedProviders: [], switchableOrgCount: 0 })
      case "/path":
        return json({ home: "", state: "", config: "", worktree, directory })
      case "/project/current":
        return json({ id: "proj_test" })
      case "/provider":
        return json({
          all: [activeJnoccio, jekkoProvider],
          default: {
            ...(unlocked ? { jnoccio: "jnoccio-fusion" } : {}),
            jekko: "jekko-free",
          },
          connected: unlocked ? ["jnoccio"] : [],
        })
      case "/provider/jnoccio/unlock": {
        const body = await request.text()
        const parsed = body ? JSON.parse(body) : {}
        unlockBodies.push(parsed)
        if (!parsed.unlockSecret) {
          return json({
            status: "needs_secret",
            message: "Enter your 128-character Jnoccio unlock secret to unlock Jnoccio Fusion.",
            envCreated: false,
          })
        }
        unlocked = true
        return json({
          status: "unlocked",
          message: "Jnoccio Fusion is unlocked.",
          envCreated: true,
          secretSaved: true,
        })
      }
      case "/instance/dispose":
        dispose.push(url)
        return json(true)
      case "/session":
        return json([])
      case "/vcs":
        return json({ branch: "main" })
    }

    throw new Error(`unexpected request: ${url.pathname}`)
  }) as typeof globalThis.fetch

  return { dispose, fetch, unlockBodies }
}

function eventSource(): EventSource {
  return {
    subscribe: async () => () => {},
  }
}

async function mount() {
  const calls = createFetch()
  let sync!: ReturnType<typeof useSync>
  let local!: ReturnType<typeof useLocal>
  let dialog!: DialogContext
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
              <SDKProvider url="http://test" directory={directory} fetch={calls.fetch} events={eventSource()}>
                <ProjectProvider>
                  <SyncProvider>
                    <ThemeProvider mode="dark">
                      <LocalProvider>
                        <DialogProvider>
                          <OpenUnlockDialog
                            onReady={(ctx) => {
                              sync = ctx.sync
                              local = ctx.local
                              dialog = ctx.dialog
                              done()
                            }}
                          />
                        </DialogProvider>
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
  return { ...calls, app, dialog, local, sync }
}

function OpenUnlockDialog(props: {
  onReady: (ctx: {
    dialog: DialogContext
    local: ReturnType<typeof useLocal>
    sync: ReturnType<typeof useSync>
  }) => void
}) {
  const dialog = useDialog()
  const local = useLocal()
  const sync = useSync()

  onMount(() => {
    dialog.replace(() => <DialogJnoccioUnlock />)
    props.onReady({ dialog, local, sync })
  })

  return <box />
}

describe("DialogJnoccioUnlock", () => {
  test("posts the pasted unlock secret, refreshes providers, and selects Jnoccio", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    await Bun.write(
      `${tmp.path}/model.json`,
      JSON.stringify({
        recent: [],
        favorite: [],
        variant: {},
      }),
    )
    const { app, dialog, dispose, local, unlockBodies } = await mount()

    try {
      await wait(() => unlockBodies.length === 1)
      expect(unlockBodies[0]).toEqual({})

      await app.mockInput.pasteBracketedText(secret)
      await wait(() => app.captureCharFrame().includes("128/128"))
      app.mockInput.pressEnter()

      await wait(() => unlockBodies.length === 2)
      await wait(() => dialog.stack.length === 0)
      await wait(() => local.model.current()?.providerID === "jnoccio")

      expect(unlockBodies[1]).toEqual({ unlockSecret: secret })
      expect((unlockBodies[1] as { unlockSecret: string }).unlockSecret).toHaveLength(128)
      expect(dispose).toHaveLength(1)
      expect(local.model.current()).toEqual({ providerID: "jnoccio", modelID: "jnoccio-fusion" })
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })

  test("posts the exact unlock secret when terminal paste markers arrive as keypresses", async () => {
    const previous = Global.Path.state
    await using tmp = await tmpdir()
    Global.Path.state = tmp.path
    await Bun.write(`${tmp.path}/kv.json`, "{}")
    await Bun.write(
      `${tmp.path}/model.json`,
      JSON.stringify({
        recent: [],
        favorite: [],
        variant: {},
      }),
    )
    const { app, dialog, dispose, local, unlockBodies } = await mount()

    try {
      await wait(() => unlockBodies.length === 1)
      expect(unlockBodies[0]).toEqual({})

      await app.mockInput.typeText(`200${secret}201`)
      await wait(() => app.captureCharFrame().includes("128/128"))
      app.mockInput.pressEnter()

      await wait(() => unlockBodies.length === 2)
      await wait(() => dialog.stack.length === 0)
      await wait(() => local.model.current()?.providerID === "jnoccio")

      expect(unlockBodies[1]).toEqual({ unlockSecret: secret })
      expect((unlockBodies[1] as { unlockSecret: string }).unlockSecret).toHaveLength(128)
      expect(dispose).toHaveLength(1)
      expect(local.model.current()).toEqual({ providerID: "jnoccio", modelID: "jnoccio-fusion" })
    } finally {
      app.renderer.destroy()
      Global.Path.state = previous
    }
  })
})
