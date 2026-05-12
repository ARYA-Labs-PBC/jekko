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

test("updates config and writes to file", async () => {
  await using tmp = await tmpdir()
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const newConfig = { model: "updated/model" }
      await save(newConfig as any)

      const writtenConfig = await Filesystem.readJson<{ model: string }>(path.join(tmp.path, "config.json"))
      expect(writtenConfig.model).toBe("updated/model")
    },
  })
})

test("gets config directories", async () => {
  await using tmp = await tmpdir()
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const dirs = await listDirs()
      expect(dirs.length).toBeGreaterThanOrEqual(1)
    },
  })
})

test("does not try to install dependencies in read-only JEKKO_CONFIG_DIR", async () => {
  if (process.platform === "win32") return

  await using tmp = await tmpdir<string>({
    init: async (dir) => {
      const ro = path.join(dir, "readonly")
      await fs.mkdir(ro, { recursive: true })
      await fs.chmod(ro, 0o555)
      return ro
    },
    dispose: async (dir) => {
      const ro = path.join(dir, "readonly")
      await fs.chmod(ro, 0o755).catch(() => {})
      return ro
    },
  })

  const prev = process.env.JEKKO_CONFIG_DIR
  process.env.JEKKO_CONFIG_DIR = tmp.extra

  try {
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        await load()
      },
    })
  } finally {
    if (prev === undefined) delete process.env.JEKKO_CONFIG_DIR
    else process.env.JEKKO_CONFIG_DIR = prev
  }
})

test("installs dependencies in writable JEKKO_CONFIG_DIR", async () => {
  await using tmp = await tmpdir<string>({
    init: async (dir) => {
      const cfg = path.join(dir, "configdir")
      await fs.mkdir(cfg, { recursive: true })
      return cfg
    },
  })

  const prev = process.env.JEKKO_CONFIG_DIR
  process.env.JEKKO_CONFIG_DIR = tmp.extra

  const testLayer = Config.layer.pipe(
    Layer.provide(testFlock),
    Layer.provide(AppFileSystem.defaultLayer),
    Layer.provide(Env.defaultLayer),
    Layer.provide(emptyAuth),
    Layer.provide(emptyAccount),
    Layer.provideMerge(infra),
    Layer.provide(noopNpm),
  )

  try {
    await WithInstance.provide({
      directory: tmp.path,
      fn: async () => {
        await Effect.runPromise(Config.Service.use((svc) => svc.get()).pipe(Effect.scoped, Effect.provide(testLayer)))
        await Effect.runPromise(
          Config.Service.use((svc) => svc.waitForDependencies()).pipe(Effect.scoped, Effect.provide(testLayer)),
        )
      },
    })

    // pending: this is a shortcut to wait for backgruounded gitignore
    await new Promise((resolve) => setTimeout(resolve, 1000))

    expect(await Filesystem.exists(path.join(tmp.extra, ".gitignore"))).toBe(true)
    expect(await Filesystem.readText(path.join(tmp.extra, ".gitignore"))).toContain("package-lock.json")
  } finally {
    if (prev === undefined) delete process.env.JEKKO_CONFIG_DIR
    else process.env.JEKKO_CONFIG_DIR = prev
  }
})

// Note: deduplication and serialization of npm installs is now handled by the
// core Npm.Service (via EffectFlock). Those behaviors are tested in the core
// package's npm tests, not here.

test("resolves scoped npm plugins in config", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      const pluginDir = path.join(dir, "node_modules", "@scope", "plugin")
      await fs.mkdir(pluginDir, { recursive: true })

      await Filesystem.write(
        path.join(dir, "package.json"),
        JSON.stringify({ name: "config-fixture", version: "1.0.0", type: "module" }, null, 2),
      )

      await Filesystem.write(
        path.join(pluginDir, "package.json"),
        JSON.stringify(
          {
            name: "@scope/plugin",
            version: "1.0.0",
            type: "module",
            main: "./index.js",
          },
          null,
          2,
        ),
      )

      await Filesystem.write(path.join(pluginDir, "index.js"), "export default {}\n")

      await Filesystem.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({ $schema: "https://jekko.ai/config.json", plugin: ["@scope/plugin"] }, null, 2),
      )
    },
  })

  await provideTestInstance({
    directory: tmp.path,
    fn: async () => {
      const config = await load()
      const pluginEntries = config.plugin ?? []
      expect(pluginEntries).toContain("@scope/plugin")
    },
  })
})

test("merges plugin arrays from global and local configs", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      // Create a nested project structure with local .jekko config
      const projectDir = path.join(dir, "project")
      const jekkoDir = path.join(projectDir, ".jekko")
      await fs.mkdir(jekkoDir, { recursive: true })

      // Global config with plugins
      await Filesystem.write(
        path.join(dir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          plugin: ["global-plugin-1", "global-plugin-2"],
        }),
      )

      // Local .jekko config with different plugins
      await Filesystem.write(
        path.join(jekkoDir, "jekko.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          plugin: ["local-plugin-1"],
        }),
      )
    },
  })

  await provideTestInstance({
    directory: path.join(tmp.path, "project"),
    fn: async () => {
      const config = await load()
      const plugins = config.plugin ?? []

      // Should contain both global and local plugins
      expect(plugins.some((p) => p.includes("global-plugin-1"))).toBe(true)
      expect(plugins.some((p) => p.includes("global-plugin-2"))).toBe(true)
      expect(plugins.some((p) => p.includes("local-plugin-1"))).toBe(true)

      // Should have all 3 plugins (not replaced, but merged)
      const pluginNames = plugins.filter((p) => p.includes("global-plugin") || p.includes("local-plugin"))
      expect(pluginNames.length).toBeGreaterThanOrEqual(3)
    },
  })
})
