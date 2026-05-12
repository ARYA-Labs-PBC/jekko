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
  install: () => Effect.void,
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
  await Filesystem.write(path.resolve(managedConfigDir, "jekko.json"), JSON.stringify(settings))
}

async function writeConfig(dir: string, config: object) {
  await Filesystem.write(path.resolve(dir, "jekko.json"), JSON.stringify(config))
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

test("loads config with defaults when no files exist", async () => {
  await using tmp = await tmpdir()
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const config = await load()
      expect(config.username).toBeDefined()
    },
  })
})

test("loads JSON config file", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await writeConfig(dir, {
        $schema: "https://jekko.ai/config.json",
        model: "test/model",
        username: "testuser",
      })
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const config = await load()
      expect(config.model).toBe("test/model")
      expect(config.username).toBe("testuser")
    },
  })
})

test("writes managed settings to the canonical filename", async () => {
  await writeManagedSettings({ snapshot: true })
  const written = await Filesystem.readJson<{ snapshot?: boolean }>(path.join(managedConfigDir, "jekko.json"))
  expect(written.snapshot).toBe(true)
})

test("loads shell config field", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await writeConfig(dir, {
        $schema: "https://jekko.ai/config.json",
        shell: "bash",
      })
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      const config = await load()
      expect(config.shell).toBe("bash")
    },
  })
})

test("updates config and preserves empty shell sentinel", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Filesystem.write(
        path.resolve(dir, "config.json"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          shell: "bash",
        }),
      )
    },
  })
  await WithInstance.provide({
    directory: tmp.path,
    fn: async () => {
      await save({ shell: "" })

      const writtenConfig = await Filesystem.readJson<{ shell?: string }>(path.join(tmp.path, "config.json"))
      expect(writtenConfig.shell).toBe("")
    },
  })
})

test("updates global config and omits empty shell key in json", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await writeConfig(dir, {
        $schema: "https://jekko.ai/config.json",
        shell: "bash",
      })
    },
  })

  const prev = Global.Path.config
  ;(Global.Path as { config: string }).config = tmp.path
  await clear(true)

  try {
    await saveGlobal({ shell: "" })

    const writtenConfig = await Filesystem.readJson<{ shell?: string }>(path.join(tmp.path, "jekko.json"))
    expect("shell" in writtenConfig).toBe(false)
  } finally {
    ;(Global.Path as { config: string }).config = prev
    await clear(true)
  }
})

test("updates global config and omits empty shell key in jsonc", async () => {
  await using tmp = await tmpdir({
    init: async (dir) => {
      await Filesystem.write(
        path.join(dir, "jekko.jsonc"),
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          shell: "bash",
          model: "test/model",
        }),
      )
    },
  })

  const prev = Global.Path.config
  ;(Global.Path as { config: string }).config = tmp.path
  await clear(true)

  try {
    await saveGlobal({ shell: "" })

    const file = path.join(tmp.path, "jekko.jsonc")
    const writtenConfig = await Filesystem.readText(file)
    const parsed = ConfigParse.schema(Config.Info.zod, ConfigParse.jsonc(writtenConfig, file), file)
    expect(writtenConfig).not.toContain('"shell"')
    expect(parsed.shell).toBeUndefined()
    expect(parsed.model).toBe("test/model")
  } finally {
    ;(Global.Path as { config: string }).config = prev
    await clear(true)
  }
})
