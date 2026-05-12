import path from "path"
import { fileURLToPath } from "url"
import * as Log from "@jekko-ai/core/util/log"
import { Global } from "@jekko-ai/core/global"
import { Flock } from "@jekko-ai/core/util/flock"
import { Filesystem } from "@/util/filesystem"
import { readPackageThemes, type PluginPackage } from "@/plugin/shared"
import { PluginMeta } from "@/plugin/meta"
import { hasTheme, upsertTheme } from "../context/theme"
import { isRecord } from "@/util/record"
import type { TuiTheme } from "@jekko-ai/plugin/tui"
import type { PluginEntry } from "./runtime-core"

const log = Log.create({ service: "tui.plugin" })

function isTheme(value: unknown) {
  if (!isRecord(value)) return false
  if (!("theme" in value)) return false
  if (!isRecord(value.theme)) return false
  return true
}

export function createThemeInstaller(
  meta: PluginEntry["load"]["origin"],
  root: string,
  spec: string,
  plugin: PluginEntry,
): TuiTheme["install"] {
  return async (file) => {
    const raw = file.startsWith("file://") ? fileURLToPath(file) : file
    const src = path.isAbsolute(raw) ? raw : path.resolve(root, raw)
    const name = path.basename(src, path.extname(src))
    const source_dir = path.dirname(meta.source)
    const local_dir =
      path.basename(source_dir) === ".jekko" ? path.join(source_dir, "themes") : path.join(source_dir, ".jekko", "themes")
    const dest_dir = meta.scope === "local" ? local_dir : path.join(Global.Path.config, "themes")
    const dest = path.join(dest_dir, `${name}.json`)
    const stat = await Filesystem.statAsync(src)
    const mtime = stat ? Math.floor(typeof stat.mtimeMs === "bigint" ? Number(stat.mtimeMs) : stat.mtimeMs) : undefined
    const size = stat ? (typeof stat.size === "bigint" ? Number(stat.size) : stat.size) : undefined
    const info = {
      src,
      dest,
      mtime,
      size,
    }

    await Flock.withLock(`tui-theme:${dest}`, async () => {
      const save = async () => {
        plugin.themes[name] = info
        await PluginMeta.setTheme(plugin.id, name, info).catch((error) => {
          log.warn("failed to track tui plugin theme", {
            path: spec,
            id: plugin.id,
            theme: src,
            dest,
            error,
          })
        })
      }

      const exists = hasTheme(name)
      const prev = plugin.themes[name]
      if (exists) {
        if (plugin.meta.state !== "updated") {
          if (!prev && (await Filesystem.exists(dest))) {
            await save()
          }
          return
        }
        if (prev?.dest === dest && prev.mtime === mtime && prev.size === size) return
      }

      const text = await Filesystem.readText(src).catch((error) => {
        log.warn("failed to read tui plugin theme", { path: spec, theme: src, error })
        return
      })
      if (text === undefined) return

      const fail = Symbol()
      const data = await Promise.resolve(text)
        .then((x) => JSON.parse(x))
        .catch((error) => {
          log.warn("failed to parse tui plugin theme", { path: spec, theme: src, error })
          return fail
        })
      if (data === fail) return

      if (!isTheme(data)) {
        log.warn("invalid tui plugin theme", { path: spec, theme: src })
        return
      }

      if (exists || !(await Filesystem.exists(dest))) {
        await Filesystem.write(dest, text).catch((error) => {
          log.warn("failed to persist tui plugin theme", { path: spec, theme: src, dest, error })
        })
      }

      upsertTheme(name, data)
      await save()
    }).catch((error) => {
      log.warn("failed to lock tui plugin theme install", { path: spec, theme: src, dest, error })
    })
  }
}

export async function readThemeFiles(spec: string, pkg?: PluginPackage) {
  if (!pkg) return [] as string[]
  return Promise.resolve()
    .then(() => readPackageThemes(spec, pkg))
    .catch((error) => {
      log.warn("invalid tui plugin oc-themes", {
        path: spec,
        pkg: pkg.pkg,
        error,
      })
      return [] as string[]
    })
}

export async function syncPluginThemes(plugin: PluginEntry) {
  if (!plugin.load.theme_files.length) return
  if (plugin.meta.state === "same") return
  const install = createThemeInstaller(plugin.load.origin, plugin.load.theme_root, plugin.load.spec, plugin)
  for (const file of plugin.load.theme_files) {
    await install(file).catch((error) => {
      log.warn("failed to sync tui plugin oc-themes", { path: plugin.load.spec, id: plugin.id, theme: file, error })
    })
  }
}
