import { test, expect, describe, mock, afterEach, beforeEach } from "bun:test"
import { Effect, Layer, Option } from "effect"
import { NodeFileSystem, NodePath } from "@effect/platform-node"
import { Config } from "@/config/config"
import { ConfigManaged } from "@/config/managed"
import { ConfigParse } from "../../src/config/parse"
import { EffectFlock } from "@jekko-ai/core/util/effect-flock"

import { Instance } from "../../src/project/instance"
import { WithInstance } from "../../src/project/with-instance"
import { Auth } from "../../src/auth"
import { Account } from "../../src/account/account"
import { AccessToken, AccountID, OrgID } from "../../src/account/schema"
import { AppFileSystem } from "@jekko-ai/core/filesystem"
import { Env } from "../../src/env"
import { provideTestInstance, provideTmpdirInstance } from "../fixture/fixture"
import { tmpdir } from "../fixture/fixture"
import { InstanceRuntime } from "@/project/instance-runtime"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { testEffect } from "../lib/effect"

/** Infra layer that provides FileSystem, Path, ChildProcessSpawner for test fixtures */
const infra = CrossSpawnSpawner.defaultLayer.pipe(
  Layer.provideMerge(Layer.mergeAll(NodeFileSystem.layer, NodePath.layer)),
)
import path from "path"
import fs from "fs/promises"
import { pathToFileURL } from "url"
import { Global } from "@jekko-ai/core/global"
import { ProjectID } from "../../src/project/schema"
import { Filesystem } from "@/util/filesystem"
import { ConfigPlugin } from "@/config/plugin"
import { Npm } from "@jekko-ai/core/npm"

const emptyAccount = Layer.mock(Account.Service)({
  active: () => Effect.succeed(Option.none()),
  activeOrg: () => Effect.succeed(Option.none()),
})

const emptyAuth = Layer.mock(Auth.Service)({
  all: () => Effect.succeed({}),
})

const testFlock = EffectFlock.defaultLayer

const noopNpm = Layer.mock(Npm.Service)({
  install: () => Effect.succeed({ directory: "/tmp", entrypoint: Option.none() }),
  add: () => Effect.die(new Error("Npm.add should not be called in config tests")),
  which: () => Effect.succeed(Option.none()),
})

const layer = Config.layer.pipe(
  Layer.provide(testFlock),
  Layer.provide(AppFileSystem.defaultLayer),
  Layer.provide(Env.defaultLayer),
  Layer.provide(emptyAuth),
  Layer.provide(emptyAccount),
  Layer.provideMerge(infra),
  Layer.provide(noopNpm),
)

const it = testEffect(layer)

const load = () => Effect.runPromise(Config.Service.use((svc) => svc.get()).pipe(Effect.scoped, Effect.provide(layer)))
const save = (config: Config.Info) =>
  Effect.runPromise(Config.Service.use((svc) => svc.update(config)).pipe(Effect.scoped, Effect.provide(layer)))
const saveGlobal = (config: Config.Info) =>
  Effect.runPromise(
    Config.Service.use((svc) => svc.updateGlobal(config)).pipe(
      Effect.map((result) => result.info),
      Effect.scoped,
      Effect.provide(layer),
    ),
  )
const clear = async (wait = false) => {
  await Effect.runPromise(Config.Service.use((svc) => svc.invalidate()).pipe(Effect.scoped, Effect.provide(layer)))
  if (wait) await InstanceRuntime.disposeAllInstances()
}
const listDirs = () =>
  Effect.runPromise(Config.Service.use((svc) => svc.directories()).pipe(Effect.scoped, Effect.provide(layer)))
const ready = () =>
  Effect.runPromise(Config.Service.use((svc) => svc.waitForDependencies()).pipe(Effect.scoped, Effect.provide(layer)))

// Get managed config directory from environment (set in preload.ts)
const managedConfigDir = process.env.JEKKO_TEST_MANAGED_CONFIG_DIR!

beforeEach(async () => {
  await clear(true)
})

afterEach(async () => {
  await fs.rm(managedConfigDir, { force: true, recursive: true }).catch(() => {})
  await clear(true)
})

async function writeManagedSettings(settings: object) {
  await fs.mkdir(managedConfigDir, { recursive: true })
  await Filesystem.write(path.join(managedConfigDir, "jekko.json"), JSON.stringify(settings))
}

async function writeConfig(dir: string, config: object) {
  await Filesystem.write(path.join(dir, "jekko.json"), JSON.stringify(config))
}

async function check(map: (dir: string) => string) {
  if (process.platform !== "win32") return
  await using globalTmp = await tmpdir()
  await using tmp = await tmpdir({ git: true, config: { snapshot: true } })
  const prev = Global.Path.config
  ;(Global.Path as { config: string }).config = globalTmp.path
  await clear()
  try {
    await writeConfig(globalTmp.path, {
      $schema: "https://jekko.ai/config.json",
      snapshot: false,
    })
    await WithInstance.provide({
      directory: map(tmp.path),
      fn: async () => {
        const cfg = await load()
        expect(cfg.snapshot).toBe(true)
        expect(Instance.directory).toBe(Filesystem.resolve(tmp.path))
        expect(Instance.project.id).not.toBe(ProjectID.global)
      },
    })
  } finally {
    await InstanceRuntime.disposeAllInstances()
    ;(Global.Path as { config: string }).config = prev
    await clear()
  }
}

test("MCP config deep merges preserving base config properties", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      // Base config with full MCP definition
      await Filesystem.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          mcp: {
            myserver: {
              type: "remote",
              url: "https://myserver.example.com/mcp",
              enabled: false,
              headers: {
                "X-Custom-Header": "value",
              },
            },
          },
        }),
      )
      // Override just enables it, should preserve other properties
      await Filesystem.write(
        path.join(dir, "jekko.jsonc"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          mcp: {
            myserver: {
              type: "remote",
              url: "https://myserver.example.com/mcp",
              enabled: true,
            },
          },
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const config = await load()
      expect(config.mcp?.myserver).toEqual({
        type: "remote",
        url: "https://myserver.example.com/mcp",
        enabled: true,
        headers: {
          "X-Custom-Header": "value",
        },
      })
    },
  })
})

test("local .jekko config can override MCP from project config", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      // Project config with disabled MCP
      await Filesystem.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          mcp: {
            docs: {
              type: "remote",
              url: "https://docs.example.com/mcp",
              enabled: false,
            },
          },
        }),
      )
      // Local .jekko directory config enables it
      const jekkoDir = path.join(dir, ".jekko")
      await fs.mkdir(jekkoDir, { recursive: true })
      await Filesystem.write(
        path.join(jekkoDir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          mcp: {
            docs: {
              type: "remote",
              url: "https://docs.example.com/mcp",
              enabled: true,
            },
          },
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const config = await load()
      expect(config.mcp?.docs?.enabled).toBe(true)
    },
  })
})

test("project config overrides remote well-known config", async () => {
  const originalFetch = globalThis.fetch
  let fetchedUrl: string | undefined
  globalThis.fetch = mock((url: string | URL | Request) => {
    const urlStr = url instanceof Request ? url.url : url instanceof URL ? url.href : url
    if (urlStr.includes(".well-known/jekko")) {
      fetchedUrl = urlStr
      return Promise.resolve(
        new Response(
          JSON.stringify({
            config: {
              mcp: { jira: { type: "remote", url: "https://jira.example.com/mcp", enabled: false } },
            },
          }),
          { status: 200 },
        ),
      )
    }
    return originalFetch(url)
  }) as unknown as typeof fetch

  const fakeAuth = Layer.mock(Auth.Service)({
    all: () =>
      Effect.succeed({
        "https://example.com": new Auth.WellKnown({ type: "wellknown", key: "TEST_TOKEN", token: "test-token" }),
      }),
  })

  const layer = Config.layer.pipe(
    Layer.provide(testFlock),
    Layer.provide(AppFileSystem.defaultLayer),
    Layer.provide(Env.defaultLayer),
    Layer.provide(fakeAuth),
    Layer.provide(emptyAccount),
    Layer.provideMerge(infra),
    Layer.provide(noopNpm),
  )

  try {
    await provideTmpdirInstance(
      () =>
        Config.Service.use((svc) =>
          Effect.gen(function* () {
            const config = yield* svc.get()
            expect(fetchedUrl).toBe("https://example.com/.well-known/jekko")
            expect(config.mcp?.jira?.enabled).toBe(true)
          }),
        ),
      {
        git: true,
        config: { mcp: { jira: { type: "remote", url: "https://jira.example.com/mcp", enabled: true } } },
      },
    ).pipe(Effect.scoped, Effect.provide(layer), Effect.runPromise)
  } finally {
    globalThis.fetch = originalFetch
  }
})

test("wellknown URL with trailing slash is normalized", async () => {
  const originalFetch = globalThis.fetch
  let fetchedUrl: string | undefined
  globalThis.fetch = mock((url: string | URL | Request) => {
    const urlStr = url instanceof Request ? url.url : url instanceof URL ? url.href : url
    if (urlStr.includes(".well-known/jekko")) {
      fetchedUrl = urlStr
      return Promise.resolve(
        new Response(
          JSON.stringify({
            config: {
              mcp: { slack: { type: "remote", url: "https://slack.example.com/mcp", enabled: true } },
            },
          }),
          { status: 200 },
        ),
      )
    }
    return originalFetch(url)
  }) as unknown as typeof fetch

  const fakeAuth = Layer.mock(Auth.Service)({
    all: () =>
      Effect.succeed({
        "https://example.com/": new Auth.WellKnown({ type: "wellknown", key: "TEST_TOKEN", token: "test-token" }),
      }),
  })

  const layer = Config.layer.pipe(
    Layer.provide(testFlock),
    Layer.provide(AppFileSystem.defaultLayer),
    Layer.provide(Env.defaultLayer),
    Layer.provide(fakeAuth),
    Layer.provide(emptyAccount),
    Layer.provideMerge(infra),
    Layer.provide(noopNpm),
  )

  try {
    await provideTmpdirInstance(
      () =>
        Config.Service.use((svc) =>
          Effect.gen(function* () {
            yield* svc.get()
            expect(fetchedUrl).toBe("https://example.com/.well-known/jekko")
          }),
        ),
      { git: true },
    ).pipe(Effect.scoped, Effect.provide(layer), Effect.runPromise)
  } finally {
    globalThis.fetch = originalFetch
  }
})

test("wellknown remote_config supports templated env vars in headers", async () => {
  const originalFetch = globalThis.fetch
  const originalToken = process.env.TEST_TOKEN
  let wellknownFetchedUrl: string | undefined
  let remoteFetchedUrl: string | undefined
  let remoteHeaders: HeadersInit | undefined
  globalThis.fetch = mock((url: string | URL | Request, init?: RequestInit) => {
    const urlStr = url instanceof Request ? url.url : url instanceof URL ? url.href : url
    if (urlStr.includes(".well-known/jekko")) {
      wellknownFetchedUrl = urlStr
      return Promise.resolve(
        new Response(
          JSON.stringify({
            remote_config: {
              url: "https://config.example.com/jekko.json",
              headers: {
                Authorization: "Bearer {env:TEST_TOKEN}",
              },
            },
          }),
          { status: 200 },
        ),
      )
    }
    if (urlStr.includes("config.example.com")) {
      remoteFetchedUrl = urlStr
      remoteHeaders = init?.headers
      return Promise.resolve(
        new Response(
          JSON.stringify({
            mcp: { confluence: { type: "remote", url: "https://confluence.example.com/mcp", enabled: true } },
          }),
          { status: 200 },
        ),
      )
    }
    return originalFetch(url, init)
  }) as unknown as typeof fetch

  const fakeAuth = Layer.mock(Auth.Service)({
    all: () =>
      Effect.succeed({
        "https://example.com": new Auth.WellKnown({ type: "wellknown", key: "TEST_TOKEN", token: "test-token" }),
      }),
  })

  const layer = Config.layer.pipe(
    Layer.provide(testFlock),
    Layer.provide(AppFileSystem.defaultLayer),
    Layer.provide(Env.defaultLayer),
    Layer.provide(fakeAuth),
    Layer.provide(emptyAccount),
    Layer.provideMerge(infra),
    Layer.provide(noopNpm),
  )

  try {
    await provideTmpdirInstance(
      () =>
        Config.Service.use((svc) =>
          Effect.gen(function* () {
            const config = yield* svc.get()
            expect(wellknownFetchedUrl).toBe("https://example.com/.well-known/jekko")
            expect(remoteFetchedUrl).toBe("https://config.example.com/jekko.json")
            expect(remoteHeaders).toEqual({ Authorization: "Bearer test-token" })
            expect(config.mcp?.confluence?.enabled).toBe(true)
          }),
        ),
      { git: true },
    ).pipe(Effect.scoped, Effect.provide(layer), Effect.runPromise)
  } finally {
    globalThis.fetch = originalFetch
    if (originalToken === undefined) delete process.env.TEST_TOKEN
    else process.env.TEST_TOKEN = originalToken
  }
})

describe("resolvePluginSpec", () => {
  test("keeps package specs unchanged", async () => {
    await using tmp = await tmpdir()
    const file = path.join(tmp.path, "jekko.json")
    expect(await ConfigPlugin.resolvePluginSpec("oh-my-jekko@2.4.3", file)).toBe("oh-my-jekko@2.4.3")
    expect(await ConfigPlugin.resolvePluginSpec("@scope/pkg", file)).toBe("@scope/pkg")
  })

  test("resolves windows-style relative plugin directory specs", async () => {
    if (process.platform !== "win32") return

    await using tmp = await tmpdir({
      init: async (dir) => {
        const plugin = path.join(dir, "plugin")
        await fs.mkdir(plugin, { recursive: true })
        await Filesystem.write(path.join(plugin, "index.ts"), "export default {}")
      },
    })

    const file = path.join(tmp.path, "jekko.json")
    const hit = await ConfigPlugin.resolvePluginSpec(".\\plugin", file)
    expect(ConfigPlugin.pluginSpecifier(hit)).toBe(pathToFileURL(path.join(tmp.path, "plugin", "index.ts")).href)
  })

  test("resolves relative file plugin paths to file urls", async () => {
    await using tmp = await tmpdir({
      init: async (dir) => {
        await Filesystem.write(path.join(dir, "plugin.ts"), "export default {}")
      },
    })

    const file = path.join(tmp.path, "jekko.json")
    const hit = await ConfigPlugin.resolvePluginSpec("./plugin.ts", file)
    expect(ConfigPlugin.pluginSpecifier(hit)).toBe(pathToFileURL(path.join(tmp.path, "plugin.ts")).href)
  })

  test("resolves plugin directory paths to directory urls", async () => {
    await using tmp = await tmpdir({
      init: async (dir) => {
        const plugin = path.join(dir, "plugin")
        await fs.mkdir(plugin, { recursive: true })
        await Filesystem.writeJson(path.join(plugin, "package.json"), {
          name: "demo-plugin",
          type: "module",
          main: "./index.ts",
        })
        await Filesystem.write(path.join(plugin, "index.ts"), "export default {}")
      },
    })

    const file = path.join(tmp.path, "jekko.json")
    const hit = await ConfigPlugin.resolvePluginSpec("./plugin", file)
    expect(ConfigPlugin.pluginSpecifier(hit)).toBe(pathToFileURL(path.join(tmp.path, "plugin")).href)
  })

  test("resolves plugin directories without package.json to index.ts", async () => {
    await using tmp = await tmpdir({
      init: async (dir) => {
        const plugin = path.join(dir, "plugin")
        await fs.mkdir(plugin, { recursive: true })
        await Filesystem.write(path.join(plugin, "index.ts"), "export default {}")
      },
    })

    const file = path.join(tmp.path, "jekko.json")
    const hit = await ConfigPlugin.resolvePluginSpec("./plugin", file)
    expect(ConfigPlugin.pluginSpecifier(hit)).toBe(pathToFileURL(path.join(tmp.path, "plugin", "index.ts")).href)
  })
})
