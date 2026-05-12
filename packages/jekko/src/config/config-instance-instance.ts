import path from "path"
import os from "os"
import { existsSync } from "fs"
import { mergeDeep } from "remeda"
import { Effect, Exit, Fiber, Option } from "effect"
import { Flag } from "@jekko-ai/core/flag/flag"
import { InstallationLocal, InstallationVersion } from "@jekko-ai/core/installation/version"
import { Global } from "@jekko-ai/core/global"
import { AppFileSystem } from "@jekko-ai/core/filesystem"
import { InstanceState } from "@/effect/instance-state"
import { containsPath } from "../project/instance-context"
import { type Info } from "./config-schema"
import { ConfigAgent } from "./agent"
import { ConfigCommand } from "./command"
import { ConfigManaged } from "./managed"
import { ConfigPermission } from "./permission"
import { ConfigPaths } from "./paths"
import { ConfigPlugin } from "./plugin"
import { type InstanceContext } from "../project/instance"
import { isRecord } from "@/util/record"
import { log, mergeConfig, mergeConfigConcatArrays, parseWellKnownConfig, substituteWellKnownRemoteConfig } from "./config-utils"
import { loadConfig, loadFile, loadGlobal } from "./config-instance-load"
import { ensureGitignore } from "./config-instance-gitignore"

export function loadInstanceState(
  fs: typeof AppFileSystem.Type,
  authSvc: any,
  accountSvc: any,
  env: any,
  npmSvc: any,
  ctx: InstanceContext,
) {
  return Effect.gen(function* () {
    const auth = yield* authSvc.all().pipe(Effect.orDie)

    let result: Info = {}
    const consoleManagedProviders = new Set<string>()
    let activeOrgName: string | undefined

    const pluginScopeForSource = Effect.fnUntraced(function* (source: string) {
      if (source.startsWith("http://") || source.startsWith("https://")) return "global"
      if (source === "JEKKO_CONFIG_CONTENT") return "local"
      if (containsPath(source, ctx)) return "local"
      return "global"
    })

    const mergePluginOrigins = Effect.fnUntraced(function* (
      source: string,
      list: ConfigPlugin.Spec[] | undefined,
      kind?: ConfigPlugin.Scope,
    ) {
      if (!list?.length) return
      const hit = kind ?? (yield* pluginScopeForSource(source))
      const plugins = ConfigPlugin.deduplicatePluginOrigins([
        ...(result.plugin_origins ?? []),
        ...list.map((spec) => ({ spec, source, scope: hit })),
      ])
      result.plugin = plugins.map((item) => item.spec)
      result.plugin_origins = plugins
    })

    const merge = (source: string, next: Info, kind?: ConfigPlugin.Scope) => {
      result = mergeConfigConcatArrays(result, next)
      return mergePluginOrigins(source, next.plugin, kind)
    }

    for (const [key, value] of Object.entries(auth)) {
      if (value.type === "wellknown") {
        const url = key.replace(/\/+$/, "")
        process.env[value.key] = value.token
        log.debug("fetching remote config", { url: `${url}/.well-known/jekko` })
        const response = yield* Effect.promise(() => fetch(`${url}/.well-known/jekko`))
        if (!response.ok) {
          throw new Error(`failed to fetch remote config from ${url}: ${response.status}`)
        }
        const wellknown = parseWellKnownConfig(yield* Effect.promise(() => response.json()))
        const remote = yield* Effect.promise(() =>
          substituteWellKnownRemoteConfig({
            value: wellknown.remote_config,
            dir: url,
            source: `${url}/.well-known/jekko`,
          }),
        )
        const fetchedConfig = remote
          ? ((yield* Effect.promise(async () => {
              log.debug("fetching remote config", { url: remote.url })
              const response = await fetch(remote.url, { headers: remote.headers })
              if (!response.ok) throw new Error(`failed to fetch remote config from ${remote.url}: ${response.status}`)
              const data = await response.json()
              return isRecord(data) && isRecord(data.config) ? data.config : data
            })) as Record<string, unknown>)
          : {}
        const remoteConfig = mergeConfig(wellknown.config ?? {}, fetchedConfig as Info)
        if (!remoteConfig.$schema) remoteConfig.$schema = "https://jekko.ai/config.json"
        const source = `${url}/.well-known/jekko`
        const next = yield* loadConfig(JSON.stringify(remoteConfig), {
          dir: path.dirname(source),
          source,
        })
        yield* merge(source, next, "global")
        log.debug("loaded remote config from well-known", { url })
      }
    }

    const global = yield* loadGlobal(fs)
    yield* merge(Global.Path.config, global, "global")

    if (Flag.JEKKO_CONFIG) {
      yield* merge(Flag.JEKKO_CONFIG, yield* loadFile(fs, Flag.JEKKO_CONFIG))
      log.debug("loaded custom config", { path: Flag.JEKKO_CONFIG })
    }

    if (!Flag.JEKKO_DISABLE_PROJECT_CONFIG) {
      for (const file of yield* ConfigPaths.files("jekko", ctx.directory, ctx.worktree).pipe(Effect.orDie)) {
        yield* merge(file, yield* loadFile(fs, file), "local")
      }
    }

    result.agent = result.agent || {}
    result.mode = result.mode || {}
    result.plugin = result.plugin || []

    const directories = yield* ConfigPaths.directories(ctx.directory, ctx.worktree)

    const deps: Fiber.Fiber<void, never>[] = []

    for (const dir of directories) {
      if (dir.endsWith(".jekko") || dir === Flag.JEKKO_CONFIG_DIR) {
        for (const file of ["jekko.json", "jekko.jsonc"]) {
          const source = path.join(dir, file)
          log.debug(`loading config from ${source}`)
          yield* merge(source, yield* loadFile(fs, source))
          result.agent ??= {}
          result.mode ??= {}
          result.plugin ??= []
        }
      }

      yield* ensureGitignore(fs, dir).pipe(Effect.orDie)

      const dep = yield* npmSvc
        .install(dir, {
          add: [
            {
              name: "@jekko-ai/plugin",
              version: InstallationLocal ? undefined : InstallationVersion,
            },
          ],
        })
        .pipe(
          Effect.exit,
          Effect.tap((exit) =>
            Exit.isFailure(exit)
              ? Effect.sync(() => {
                  log.warn("background dependency install failed", { dir, error: String(exit.cause) })
                })
              : Effect.void,
          ),
          Effect.asVoid,
          Effect.forkDetach,
        )
      deps.push(dep)

      result.command = mergeConfig(result.command ?? {}, yield* Effect.promise(() => ConfigCommand.load(dir)))
      result.agent = mergeConfig(result.agent ?? {}, yield* Effect.promise(() => ConfigAgent.load(dir)))
      result.agent = mergeConfig(result.agent ?? {}, yield* Effect.promise(() => ConfigAgent.loadMode(dir)))
      const list = yield* Effect.promise(() => ConfigPlugin.load(dir))
      yield* mergePluginOrigins(dir, list)
    }

    if (process.env.JEKKO_CONFIG_CONTENT) {
      const source = "JEKKO_CONFIG_CONTENT"
      const next = yield* loadConfig(process.env.JEKKO_CONFIG_CONTENT, {
        dir: ctx.directory,
        source,
      })
      yield* merge(source, next, "local")
      log.debug("loaded custom config from JEKKO_CONFIG_CONTENT")
    }

    const activeAccount = Option.getOrUndefined(
      yield* accountSvc.active().pipe(Effect.catch(() => Effect.succeed(Option.none()))),
    )
    if (activeAccount?.active_org_id) {
      const accountID = activeAccount.id
      const orgID = activeAccount.active_org_id
      const url = activeAccount.url
      yield* Effect.gen(function* () {
        const [configOpt, tokenOpt] = yield* Effect.all(
          [accountSvc.config(accountID, orgID), accountSvc.token(accountID)],
          { concurrency: 2 },
        )
        if (Option.isSome(tokenOpt)) {
          process.env["JEKKO_CONSOLE_TOKEN"] = tokenOpt.value
          yield* env.set("JEKKO_CONSOLE_TOKEN", tokenOpt.value)
        }

        if (Option.isSome(configOpt)) {
          const source = `${url}/api/config`
          const next = yield* loadConfig(JSON.stringify(configOpt.value), {
            dir: path.dirname(source),
            source,
          })
          for (const providerID of Object.keys(next.provider ?? {})) {
            consoleManagedProviders.add(providerID)
          }
          yield* merge(source, next, "global")
        }
      }).pipe(
        Effect.withSpan("Config.loadActiveOrgConfig"),
        Effect.catch((err) => {
          log.debug("failed to fetch remote account config", {
            error: err instanceof Error ? err.message : String(err),
          })
          return Effect.void
        }),
      )
    }

    const managedDir = ConfigManaged.managedConfigDir()
    if (existsSync(managedDir)) {
      for (const file of ["jekko.json", "jekko.jsonc"]) {
        const source = path.join(managedDir, file)
        yield* merge(source, yield* loadFile(fs, source), "global")
      }
    }

    const managed = yield* Effect.promise(() => ConfigManaged.readManagedPreferences())
    if (managed) {
      result = mergeConfigConcatArrays(
        result,
        yield* loadConfig(managed.text, {
          dir: path.dirname(managed.source),
          source: managed.source,
        }),
      )
    }

    for (const [name, mode] of Object.entries(result.mode ?? {})) {
      result.agent = mergeConfig(result.agent ?? {}, {
        [name]: {
          ...mode,
          mode: "primary" as const,
        },
      })
    }

    if (Flag.JEKKO_PERMISSION) {
      result.permission = mergeConfig(result.permission ?? {}, JSON.parse(Flag.JEKKO_PERMISSION))
    }

    if (result.tools) {
      const perms: Record<string, ConfigPermission.Action> = {}
      for (const [tool, enabled] of Object.entries(result.tools)) {
        const action: ConfigPermission.Action = enabled ? "allow" : "deny"
        if (tool === "write" || tool === "edit" || tool === "patch") {
          perms.edit = action
          continue
        }
        perms[tool] = action
      }
      result.permission = mergeDeep(perms, result.permission ?? {})
    }

    if (!result.username) result.username = os.userInfo().username
    if (result.autoshare === true && !result.share) {
      result.share = "auto"
    }
    if (Flag.JEKKO_DISABLE_AUTOCOMPACT) {
      result.compaction = { ...result.compaction, auto: false }
    }
    if (Flag.JEKKO_DISABLE_PRUNE) {
      result.compaction = { ...result.compaction, prune: false }
    }

    return {
      config: result,
      directories,
      deps,
      consoleState: {
        consoleManagedProviders: Array.from(consoleManagedProviders),
        activeOrgName,
        switchableOrgCount: 0,
      },
    }
  })
}
