import path from "path"
import { pathToFileURL } from "url"
import fsNode from "fs/promises"
import { existsSync } from "fs"
import { Effect } from "effect"
import { Global } from "@jekko-ai/core/global"
import { type Info, Info as ConfigInfoSchema } from "./config-schema"
import { ConfigParse } from "./parse"
import { ConfigVariable } from "./variable"
import {
  log,
  mergeConfig,
  normalizeLoadedConfig,
  resolveLoadedPlugins,
} from "./config-utils"

function readConfigFile(fs: any, filepath: string) {
  return fs.readFileStringSafe(filepath).pipe(Effect.orDie)
}

export const loadConfig = Effect.fnUntraced(function* (
  text: string,
  options: { path: string } | { dir: string; source: string },
) {
  const source = "path" in options ? options.path : options.source
  const expanded = yield* Effect.promise(() =>
    ConfigVariable.substitute(
      "path" in options ? { text, type: "path", path: options.path } : { text, type: "virtual", ...options },
    ),
  )
  const parsed = ConfigParse.jsonc(expanded, source)
  const data = ConfigParse.effectSchema(ConfigInfoSchema, mergeConfig({}, normalizeLoadedConfig(parsed, source)), source)
  if (!("path" in options)) return data

  yield* Effect.promise(() => resolveLoadedPlugins(data, options.path))
  if (!data.$schema) {
    data.$schema = "https://jekko.ai/config.json"
    const updated = text.replace(/^\s*\{/, '{\n  "$schema": "https://jekko.ai/config.json",')
    yield* Effect.promise(() => fsNode.writeFile(options.path, updated).catch(() => undefined))
  }
  return data
})

export const loadFile = Effect.fnUntraced(function* (fs: any, filepath: string) {
  log.info("loading", { path: filepath })
  const text = yield* readConfigFile(fs, filepath)
  if (!text) return {} as Info
  return yield* loadConfig(text, { path: filepath })
})

export const loadGlobal = Effect.fnUntraced(function* (fs: any) {
  let result: Info = {}
  result = mergeConfig(result, yield* loadFile(fs, path.join(Global.Path.config, "config.json")))
  result = mergeConfig(result, yield* loadFile(fs, path.join(Global.Path.config, "jekko.json")))
  result = mergeConfig(result, yield* loadFile(fs, path.join(Global.Path.config, "jekko.jsonc")))

  const historical = path.join(Global.Path.config, "config")
  if (existsSync(historical)) {
    yield* Effect.promise(() =>
      import(pathToFileURL(historical).href, { with: { type: "toml" } })
        .then(async (mod) => {
          const { provider, model, ...rest } = mod.default
          if (provider && model) result.model = `${provider}/${model}`
          result["$schema"] = "https://jekko.ai/config.json"
          result = mergeConfig(result, rest)
          await fsNode.writeFile(path.join(Global.Path.config, "config.json"), JSON.stringify(result, null, 2))
          await fsNode.unlink(historical)
        })
        .catch(() => {}),
    )
  }

  return result
})
