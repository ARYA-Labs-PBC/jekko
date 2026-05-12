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

describe("deduplicatePluginOrigins", () => {
  const dedupe = (plugins: ConfigPlugin.Spec[]) =>
    ConfigPlugin.deduplicatePluginOrigins(
      plugins.map((spec) => ({
        spec,
        source: "",
        scope: "global" as const,
      })),
    ).map((item) => item.spec)

  test("removes duplicates keeping higher priority (later entries)", () => {
    const plugins = ["global-plugin@1.0.0", "shared-plugin@1.0.0", "local-plugin@2.0.0", "shared-plugin@2.0.0"]

    const result = dedupe(plugins)

    expect(result).toContain("global-plugin@1.0.0")
    expect(result).toContain("local-plugin@2.0.0")
    expect(result).toContain("shared-plugin@2.0.0")
    expect(result).not.toContain("shared-plugin@1.0.0")
    expect(result.length).toBe(3)
  })

  test("keeps path plugins separate from package plugins", () => {
    const plugins = ["oh-my-jekko@2.4.3", "file:///project/.jekko/plugin/oh-my-jekko.js"]

    const result = dedupe(plugins)

    expect(result).toEqual(plugins)
  })

  test("deduplicates direct path plugins by exact spec", () => {
    const plugins = ["file:///project/.jekko/plugin/demo.ts", "file:///project/.jekko/plugin/demo.ts"]

    const result = dedupe(plugins)

    expect(result).toEqual(["file:///project/.jekko/plugin/demo.ts"])
  })

  test("preserves order of remaining plugins", () => {
    const plugins = ["a-plugin@1.0.0", "b-plugin@1.0.0", "c-plugin@1.0.0"]

    const result = dedupe(plugins)

    expect(result).toEqual(["a-plugin@1.0.0", "b-plugin@1.0.0", "c-plugin@1.0.0"])
  })

  test("loads auto-discovered local plugins as file urls", async () => {
    await using tmp = await tmpdir({
      init: async (dir) => {
        const projectDir = path.join(dir, "project")
        const jekkoDir = path.join(projectDir, ".jekko")
        const pluginDir = path.join(jekkoDir, "plugin")
        await fs.mkdir(pluginDir, { recursive: true })

        await Filesystem.write(
          path.join(dir, "jekko.json"),
          JSON.stringify({
            $schema: "https://jekko.ai/config.json",
            plugin: ["my-plugin@1.0.0"],
          }),
        )

        await Filesystem.write(path.join(pluginDir, "my-plugin.js"), "export default {}")
      },
    })

    await provideTestInstance({
      directory: path.join(tmp.path, "project"),
      fn: async () => {
        const config = await load()
        const plugins = config.plugin ?? []

        expect(plugins.some((p) => ConfigPlugin.pluginSpecifier(p) === "my-plugin@1.0.0")).toBe(true)
        expect(plugins.some((p) => ConfigPlugin.pluginSpecifier(p).startsWith("file://"))).toBe(true)
      },
    })
  })
})

describe("JEKKO_DISABLE_PROJECT_CONFIG", () => {
  test("skips project config files when flag is set", async () => {
    const originalEnv = process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
    process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = "true"

    try {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create a project config that would normally be loaded
          await Filesystem.write(
            path.join(dir, "jekko.json"),
            JSON.stringify({
              $schema: "https://jekko.ai/config.json",
              model: "project/model",
              username: "project-user",
            }),
          )
        },
      })
      await WithInstance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await load()
          // Project config should NOT be loaded - model should be default, not "project/model"
          expect(config.model).not.toBe("project/model")
          expect(config.username).not.toBe("project-user")
        },
      })
    } finally {
      if (originalEnv === undefined) {
        delete process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
      } else {
        process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = originalEnv
      }
    }
  })

  test("skips project .jekko/ directories when flag is set", async () => {
    const originalEnv = process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
    process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = "true"

    try {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create a .jekko directory with a command
          const jekkoDir = path.join(dir, ".jekko", "command")
          await fs.mkdir(jekkoDir, { recursive: true })
          await Filesystem.write(path.join(jekkoDir, "test-cmd.md"), "# Test Command\nThis is a test command.")
        },
      })
      await WithInstance.provide({
        directory: tmp.path,
        fn: async () => {
          const directories = await listDirs()
          // Project .jekko should NOT be in directories list
          const hasProjectOpencode = directories.some((d) => d.startsWith(tmp.path))
          expect(hasProjectOpencode).toBe(false)
        },
      })
    } finally {
      if (originalEnv === undefined) {
        delete process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
      } else {
        process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = originalEnv
      }
    }
  })

  test("still loads global config when flag is set", async () => {
    const originalEnv = process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
    process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = "true"

    try {
      await using tmp = await tmpdir()
      await WithInstance.provide({
        directory: tmp.path,
        fn: async () => {
          // Should still get default config (from global or defaults)
          const config = await load()
          expect(config).toBeDefined()
          expect(config.username).toBeDefined()
        },
      })
    } finally {
      if (originalEnv === undefined) {
        delete process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
      } else {
        process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = originalEnv
      }
    }
  })

  test("skips relative instructions with warning when flag is set but no config dir", async () => {
    const originalDisable = process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
    const originalConfigDir = process.env["JEKKO_CONFIG_DIR"]

    try {
      // Ensure no config dir is set
      delete process.env["JEKKO_CONFIG_DIR"]
      process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = "true"

      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create a config with relative instruction path
          await Filesystem.write(
            path.join(dir, "jekko.json"),
            JSON.stringify({
              $schema: "https://jekko.ai/config.json",
              instructions: ["./CUSTOM.md"],
            }),
          )
          // Create the instruction file (should be skipped)
          await Filesystem.write(path.join(dir, "CUSTOM.md"), "# Custom Instructions")
        },
      })

      await WithInstance.provide({
        directory: tmp.path,
        fn: async () => {
          // The relative instruction should be skipped without error
          // We're mainly verifying this doesn't throw and the config loads
          const config = await load()
          expect(config).toBeDefined()
          // The instruction should have been skipped (warning logged)
          // We can't easily test the warning was logged, but we verify
          // the relative path didn't cause an error
        },
      })
    } finally {
      if (originalDisable === undefined) {
        delete process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
      } else {
        process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = originalDisable
      }
      if (originalConfigDir === undefined) {
        delete process.env["JEKKO_CONFIG_DIR"]
      } else {
        process.env["JEKKO_CONFIG_DIR"] = originalConfigDir
      }
    }
  })

  test("JEKKO_CONFIG_DIR still works when flag is set", async () => {
    const originalDisable = process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
    const originalConfigDir = process.env["JEKKO_CONFIG_DIR"]

    try {
      await using configDirTmp = await tmpdir({
        init: async (dir) => {
          // Create config in the custom config dir
          await Filesystem.write(
            path.join(dir, "jekko.json"),
            JSON.stringify({
              $schema: "https://jekko.ai/config.json",
              model: "configdir/model",
            }),
          )
        },
      })

      await using projectTmp = await tmpdir({
        init: async (dir) => {
          // Create config in project (should be ignored)
          await Filesystem.write(
            path.join(dir, "jekko.json"),
            JSON.stringify({
              $schema: "https://jekko.ai/config.json",
              model: "project/model",
            }),
          )
        },
      })

      process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = "true"
      process.env["JEKKO_CONFIG_DIR"] = configDirTmp.path

      await WithInstance.provide({
        directory: projectTmp.path,
        fn: async () => {
          const config = await load()
          // Should load from JEKKO_CONFIG_DIR, not project
          expect(config.model).toBe("configdir/model")
        },
      })
    } finally {
      if (originalDisable === undefined) {
        delete process.env["JEKKO_DISABLE_PROJECT_CONFIG"]
      } else {
        process.env["JEKKO_DISABLE_PROJECT_CONFIG"] = originalDisable
      }
      if (originalConfigDir === undefined) {
        delete process.env["JEKKO_CONFIG_DIR"]
      } else {
        process.env["JEKKO_CONFIG_DIR"] = originalConfigDir
      }
    }
  })
})

describe("JEKKO_CONFIG_CONTENT token substitution", () => {
  test("substitutes {env:} tokens in JEKKO_CONFIG_CONTENT", async () => {
    const originalEnv = process.env["JEKKO_CONFIG_CONTENT"]
    const originalTestVar = process.env["TEST_CONFIG_VAR"]
    process.env["TEST_CONFIG_VAR"] = "test_api_key_12345"
    process.env["JEKKO_CONFIG_CONTENT"] = JSON.stringify({
      $schema: "https://jekko.ai/config.json",
      username: "{env:TEST_CONFIG_VAR}",
    })

    try {
      await using tmp = await tmpdir()
      await WithInstance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await load()
          expect(config.username).toBe("test_api_key_12345")
        },
      })
    } finally {
      if (originalEnv !== undefined) {
        process.env["JEKKO_CONFIG_CONTENT"] = originalEnv
      } else {
        delete process.env["JEKKO_CONFIG_CONTENT"]
      }
      if (originalTestVar !== undefined) {
        process.env["TEST_CONFIG_VAR"] = originalTestVar
      } else {
        delete process.env["TEST_CONFIG_VAR"]
      }
    }
  })

  test("substitutes {file:} tokens in JEKKO_CONFIG_CONTENT", async () => {
    const originalEnv = process.env["JEKKO_CONFIG_CONTENT"]

    try {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Filesystem.write(path.join(dir, "api_key.txt"), "secret_key_from_file")
          process.env["JEKKO_CONFIG_CONTENT"] = JSON.stringify({
            $schema: "https://jekko.ai/config.json",
            username: "{file:./api_key.txt}",
          })
        },
      })
      await WithInstance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await load()
          expect(config.username).toBe("secret_key_from_file")
        },
      })
    } finally {
      if (originalEnv !== undefined) {
        process.env["JEKKO_CONFIG_CONTENT"] = originalEnv
      } else {
        delete process.env["JEKKO_CONFIG_CONTENT"]
      }
    }
  })
})

// parseManagedPlist unit tests — pure function, no OS interaction

test("parseManagedPlist strips MDM metadata keys", async () => {
  const config = ConfigParse.effectSchema(
    Config.Info,
    ConfigParse.jsonc(
      await ConfigManaged.parseManagedPlist(
        JSON.stringify({
          PayloadDisplayName: "Jekko Managed",
          PayloadIdentifier: "ai.jekko.managed.test",
          PayloadType: "ai.jekko.managed",
          PayloadUUID: "AAAA-BBBB-CCCC",
          PayloadVersion: 1,
          _manualProfile: true,
          share: "disabled",
          model: "mdm/model",
        }),
      ),
      "test:mobileconfig",
    ),
    "test:mobileconfig",
  )
  expect(config.share).toBe("disabled")
  expect(config.model).toBe("mdm/model")
  // MDM keys must not leak into the parsed config
  expect((config as any).PayloadUUID).toBeUndefined()
  expect((config as any).PayloadType).toBeUndefined()
  expect((config as any)._manualProfile).toBeUndefined()
})

test("parseManagedPlist parses server settings", async () => {
  const config = ConfigParse.effectSchema(
    Config.Info,
    ConfigParse.jsonc(
      await ConfigManaged.parseManagedPlist(
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          server: { hostname: "127.0.0.1", mdns: false },
          autoupdate: true,
        }),
      ),
      "test:mobileconfig",
    ),
    "test:mobileconfig",
  )
  expect(config.server?.hostname).toBe("127.0.0.1")
  expect(config.server?.mdns).toBe(false)
  expect(config.autoupdate).toBe(true)
})

test("parseManagedPlist parses permission rules", async () => {
  const config = ConfigParse.effectSchema(
    Config.Info,
    ConfigParse.jsonc(
      await ConfigManaged.parseManagedPlist(
        JSON.stringify({
          $schema: "https://jekko.ai/config.json",
          permission: {
            "*": "ask",
            bash: { "*": "ask", "rm -rf *": "deny", "curl *": "deny" },
            grep: "allow",
            glob: "allow",
            webfetch: "ask",
            "~/.ssh/*": "deny",
          },
        }),
      ),
      "test:mobileconfig",
    ),
    "test:mobileconfig",
  )
  expect(config.permission?.["*"]).toBe("ask")
  expect(config.permission?.grep).toBe("allow")
  expect(config.permission?.webfetch).toBe("ask")
  expect(config.permission?.["~/.ssh/*"]).toBe("deny")
  const bash = config.permission?.bash as Record<string, string>
  expect(bash?.["rm -rf *"]).toBe("deny")
  expect(bash?.["curl *"]).toBe("deny")
})
