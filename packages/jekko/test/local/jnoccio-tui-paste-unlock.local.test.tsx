/** @jsxImportSource @opentui/solid */
import { describe, expect, test } from "bun:test"
import path from "path"
import { testRender } from "@opentui/solid"
import { onMount } from "solid-js"
import type { Provider } from "@jekko-ai/sdk/v2"
import { Global } from "@jekko-ai/core/global"
import { ArgsProvider } from "../../src/cli/cmd/tui/context/args"
import { ExitProvider } from "../../src/cli/cmd/tui/context/exit"
import { KVProvider } from "../../src/cli/cmd/tui/context/kv"
import { ProjectProvider } from "../../src/cli/cmd/tui/context/project"
import { SDKProvider, type EventSource } from "../../src/cli/cmd/tui/context/sdk"
import { SyncProvider, useSync } from "../../src/cli/cmd/tui/context/sync"
import { TuiConfigProvider } from "../../src/cli/cmd/tui/context/tui-config"
import { ThemeProvider } from "../../src/cli/cmd/tui/context/theme"
import { LocalProvider, useLocal } from "../../src/cli/cmd/tui/context/local"
import { DialogProvider, useDialog, type DialogContext } from "../../src/cli/cmd/tui/ui/dialog"
import { ToastProvider } from "../../src/cli/cmd/tui/ui/toast"
import { DialogJnoccioUnlock } from "../../src/cli/cmd/tui/component/dialog-jnoccio-unlock"
import { isJnoccioFusionUnlocked, unlockJnoccioFusion } from "../../src/util/jnoccio-unlock"
import { tmpdir } from "../fixture/fixture"
import { cloneRepo, localUnlockPreflight, removeTempDirs, withEnv } from "./jnoccio-local-helpers"

const preflight = await localUnlockPreflight()
const localTest = preflight.ok ? test : test.skip

const tempDirs: string[] = []
const worktree = "/tmp/jekko"
const directory = `${worktree}/packages/jekko`

async function wait(fn: () => boolean | Promise<boolean>, timeout = 60000) {
  const start = Date.now()
  while (true) {
    if (await fn()) return
    if (Date.now() - start > timeout) throw new Error("timed out waiting for condition")
    await Bun.sleep(50)
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
    api: { id: providerID, url: "https://example.test/v1", npm: "@ai-sdk/openai-compatible" },
    name,
    capabilities: {
      temperature: true,
      reasoning: true,
      attachment: false,
      toolcall: true,
      input: { text: true, audio: false, image: false, video: false, pdf: false },
      output: { text: true, audio: false, image: false, video: false, pdf: false },
      interleaved: false,
    },
    cost: { input: 0, output: 0, cache: { read: 0, write: 0 } },
    limit: { context: 128000, output: 32000 },
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
    models: { "jnoccio-fusion": model("jnoccio", "jnoccio-fusion", "Jnoccio Fusion", status) },
  } satisfies Provider
}

const jekkoProvider = {
  id: "jekko",
  name: "jekko",
  source: "api",
  env: [],
  options: {},
  models: { "jekko-free": model("jekko", "jekko-free", "jekko Free") },
} satisfies Provider

function createRealUnlockFetch(repoRoot: string, secretCachePath: string) {
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
          default: { ...(unlocked ? { jnoccio: "jnoccio-fusion" } : {}), jekko: "jekko-free" },
          connected: unlocked ? ["jnoccio"] : [],
        })
      case "/provider/jnoccio/unlock": {
        const body = await request.text()
        const parsed = body ? JSON.parse(body) : {}
        unlockBodies.push(parsed)
        const result = await unlockJnoccioFusion(parsed, {
          repoRoot,
          secretPath: secretCachePath,
        })
        if (result.status === "unlocked") unlocked = true
        return json(result)
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
  return { subscribe: async () => () => {} }
}

function OpenUnlockDialog(props: {
  onReady: (ctx: { dialog: DialogContext; local: ReturnType<typeof useLocal>; sync: ReturnType<typeof useSync> }) => void
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

async function mount(repoClone: string, secretCachePath: string) {
  const calls = createRealUnlockFetch(repoClone, secretCachePath)
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

describe("Jnoccio TUI paste-to-unlock real proof", () => {
  localTest(
    "pasted unlock secret decrypts the embedded envelope and unlocks the cloned repo",
    async () => {
      if (!preflight.ok) throw new Error(`tui paste unlock skipped: ${preflight.reason}`)
      const previous = Global.Path.state
      await using tmp = await tmpdir()
      Global.Path.state = tmp.path
      await Bun.write(`${tmp.path}/kv.json`, "{}")
      await Bun.write(
        `${tmp.path}/model.json`,
        JSON.stringify({ recent: [], favorite: [], variant: {} }),
      )

      const { clone, cloneParent } = await cloneRepo("jnoccio-tui-paste-unlock-", tempDirs)
      const secretCachePath = path.join(cloneParent, "tui-paste.unlock")
      expect(isJnoccioFusionUnlocked(clone)).toBe(false)

      try {
        await withEnv(
          {
            JNOCCIO_REPO_ROOT: clone,
            JNOCCIO_UNLOCK_SECRET_PATH: secretCachePath,
          },
          async () => {
            const { app, dialog, dispose, local, unlockBodies } = await mount(clone, secretCachePath)
            try {
              await wait(() => unlockBodies.length === 1)
              expect(unlockBodies[0]).toEqual({})

              await app.mockInput.pasteBracketedText(preflight.secret)
              await wait(() => app.captureCharFrame().includes("128/128"))
              app.mockInput.pressEnter()

              await wait(() => unlockBodies.length === 2)
              await wait(() => dialog.stack.length === 0)
              await wait(() => local.model.current()?.providerID === "jnoccio")

              expect(unlockBodies[1]).toEqual({ unlockSecret: preflight.secret })
              expect((unlockBodies[1] as { unlockSecret: string }).unlockSecret).toHaveLength(128)
              expect(dispose.length).toBeGreaterThanOrEqual(1)
              expect(local.model.current()).toEqual({ providerID: "jnoccio", modelID: "jnoccio-fusion" })
              expect(isJnoccioFusionUnlocked(clone)).toBe(true)
              await expect(Bun.file(secretCachePath).text()).resolves.toBe(preflight.secret)
            } finally {
              app.renderer.destroy()
            }
          },
        )
      } finally {
        Global.Path.state = previous
        await removeTempDirs(tempDirs)
      }
    },
    180000,
  )
})
