import * as Log from "@jekko-ai/core/util/log"
import path from "path"
import { pathToFileURL } from "url"
import os from "os"
import { mergeDeep } from "remeda"
import { Global } from "@jekko-ai/core/global"
import { NamedError } from "@jekko-ai/core/util/error"
import { Flag } from "@jekko-ai/core/flag/flag"
import fsNode from "fs/promises"
import { applyEdits, modify } from "jsonc-parser"
import { existsSync } from "fs"
import { isRecord } from "@/util/record"
import { ConfigVariable } from "./variable"
import { ConfigPlugin } from "./plugin"
import { ConfigParse } from "./parse"
import { type Info } from "./config-schema"
import z from "zod"

export const log = Log.create({ service: "config" })

// Custom merge function that concatenates array fields instead of replacing them
// Keep remeda's deep conditional merge type out of hot config-loading paths; TS profiling showed it dominates here.
export function mergeConfig(target: Info, source: Info): Info {
  return mergeDeep(target, source) as Info
}

export function mergeConfigConcatArrays(target: Info, source: Info): Info {
  const merged = mergeConfig(target, source)
  if (target.instructions && source.instructions) {
    merged.instructions = Array.from(new Set([...target.instructions, ...source.instructions]))
  }
  return merged
}

export function normalizeLoadedConfig(data: unknown, source: string) {
  if (!isRecord(data)) return data
  const copy = { ...data }
  const hadLegacy = "theme" in copy || "keybinds" in copy || "tui" in copy
  if (!hadLegacy) return copy
  delete copy.theme
  delete copy.keybinds
  delete copy.tui
  log.warn("tui keys in jekko config are discouraged; move them to tui.json", { path: source })
  return copy
}

export async function substituteWellKnownRemoteConfig(input: { value: unknown; dir: string; source: string }) {
  if (!isRecord(input.value) || typeof input.value.url !== "string") return

  const url = await ConfigVariable.substitute({
    text: input.value.url,
    type: "virtual",
    dir: input.dir,
    source: input.source,
  })
  const headers = isRecord(input.value.headers)
    ? Object.fromEntries(
        await Promise.all(
          Object.entries(input.value.headers)
            .filter((entry): entry is [string, string] => typeof entry[1] === "string")
            .map(async ([key, value]) => [
              key,
              await ConfigVariable.substitute({
                text: value,
                type: "virtual",
                dir: input.dir,
                source: input.source,
              }),
            ]),
        ),
      )
    : undefined

  return { url, headers }
}

export function parseWellKnownConfig(data: unknown): { config?: Record<string, unknown>; remote_config?: unknown } {
  if (!isRecord(data)) return {}
  return {
    config: isRecord(data.config) ? data.config : undefined,
    remote_config: data.remote_config,
  }
}

export async function resolveLoadedPlugins<T extends { plugin?: ConfigPlugin.Spec[] }>(config: T, filepath: string) {
  if (!config.plugin) return config
  for (let i = 0; i < config.plugin.length; i++) {
    // Normalize path-like plugin specs while we still know which config file declared them.
    // This prevents `./plugin.ts` from being reinterpreted relative to some later merge location.
    config.plugin[i] = await ConfigPlugin.resolvePluginSpec(config.plugin[i], filepath)
  }
  return config
}

export function globalConfigFile() {
  const candidates = ["jekko.jsonc", "jekko.json", "config.json"].map((file) =>
    path.join(Global.Path.config, file),
  )
  for (const file of candidates) {
    if (existsSync(file)) return file
  }
  return candidates[0]
}

export function patchJsonc(input: string, patch: unknown, path: string[] = []): string {
  if (!isRecord(patch)) {
    const edits = modify(input, path, patch, {
      formattingOptions: {
        insertSpaces: true,
        tabSize: 2,
      },
    })
    return applyEdits(input, edits)
  }

  return Object.entries(patch).reduce((result, [key, value]) => patchJsonc(result, value, [...path, key]), input)
}

export function writable(info: Info) {
  const { plugin_origins: _plugin_origins, ...next } = info
  return next
}

export function writableGlobal(info: Info) {
  const next = writable(info)
  // When a user changes config from a value back to default, we don't want to leave a blank `"shell": "",` key
  if ("shell" in next && next.shell === "") return { ...next, shell: undefined }
  return next
}

export const ConfigDirectoryTypoError = NamedError.create(
  "ConfigDirectoryTypoError",
  z.object({
    path: z.string(),
    dir: z.string(),
    suggestion: z.string(),
  }),
)

export { Flag, pathToFileURL, os, fsNode, ConfigParse }
