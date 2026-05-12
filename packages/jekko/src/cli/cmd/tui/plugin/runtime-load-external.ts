import { EMPTY_TUI, fail, log, resolveRoot, readThemeFiles, warn, type PluginEntry, type PluginLoad, type RuntimeState, createMeta } from "./runtime-core"
import { activatePluginEntry, addPluginEntry } from "./runtime-api"
import { PluginLoader } from "@/plugin/loader"
import { readPluginId, readV1Plugin, resolvePluginId } from "@/plugin/shared"
import { PluginMeta } from "@/plugin/meta"
import type { ConfigPlugin } from "@/config/plugin"
import { type TuiPluginModule } from "@jekko-ai/plugin/tui"

export async function resolveExternalPlugins(list: ConfigPlugin.Origin[], wait: () => Promise<void>) {
  return PluginLoader.loadExternal({
    items: list,
    kind: "tui",
    wait: async () => {
      await wait().catch((error) => {
        log.warn("failed waiting for tui plugin dependencies", { error })
      })
    },
    finish: async (loaded, origin, retry) => {
      const mod = await Promise.resolve()
        .then(() => readV1Plugin(loaded.mod as Record<string, unknown>, loaded.spec, "tui") as TuiPluginModule)
        .catch((error) => {
          fail("failed to load tui plugin", {
            path: loaded.spec,
            target: loaded.entry,
            retry,
            error,
          })
          return
        })
      if (!mod) return

      const id = await resolvePluginId(
        loaded.source,
        loaded.spec,
        loaded.target,
        readPluginId(mod.id, loaded.spec),
        loaded.pkg,
      ).catch((error) => {
        fail("failed to load tui plugin", { path: loaded.spec, target: loaded.target, retry, error })
        return
      })
      if (!id) return

      const theme_files = await readThemeFiles(loaded.spec, loaded.pkg)

      return {
        options: loaded.options,
        spec: loaded.spec,
        target: loaded.target,
        retry,
        source: loaded.source,
        id,
        module: mod,
        origin,
        theme_root: loaded.pkg?.dir ?? resolveRoot(loaded.target),
        theme_files,
      }
    },
    missing: async (loaded, origin, retry) => {
      const theme_files = await readThemeFiles(loaded.spec, loaded.pkg)
      if (!theme_files.length) return

      const name =
        typeof loaded.pkg?.json.name === "string" && loaded.pkg.json.name.trim().length > 0
          ? loaded.pkg.json.name.trim()
          : undefined
      const id = await resolvePluginId(loaded.source, loaded.spec, loaded.target, name, loaded.pkg).catch((error) => {
        fail("failed to load tui plugin", { path: loaded.spec, target: loaded.target, retry, error })
        return
      })
      if (!id) return

      return {
        options: loaded.options,
        spec: loaded.spec,
        target: loaded.target,
        retry,
        source: loaded.source,
        id,
        module: EMPTY_TUI,
        origin,
        theme_root: loaded.pkg?.dir ?? resolveRoot(loaded.target),
        theme_files,
      }
    },
    report: {
      start(candidate, retry) {
        log.info("loading tui plugin", { path: candidate.plan.spec, retry })
      },
      missing(candidate, retry, message) {
        warn("tui plugin has no entrypoint", { path: candidate.plan.spec, retry, message })
      },
      error(candidate, retry, stage, error, resolved) {
        const spec = candidate.plan.spec
        if (stage === "install") {
          fail("failed to resolve tui plugin", { path: spec, retry, error })
          return
        }
        if (stage === "compatibility") {
          fail("tui plugin incompatible", { path: spec, retry, error })
          return
        }
        if (stage === "entry") {
          fail("failed to resolve tui plugin entry", { path: spec, retry, error })
          return
        }
        fail("failed to load tui plugin", { path: spec, target: resolved?.entry, retry, error })
      },
    },
  })
}

export async function addExternalPluginEntries(state: RuntimeState, ready: PluginLoad[]) {
  if (!ready.length) return { plugins: [] as PluginEntry[], ok: true }

  const meta = await PluginMeta.touchMany(
    ready.map((item) => ({
      spec: item.spec,
      target: item.target,
      id: item.id,
    })),
  ).catch((error) => {
    log.warn("failed to track tui plugins", { error })
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    return undefined
  })

  const plugins: PluginEntry[] = []
  let ok = true
  for (let i = 0; i < ready.length; i++) {
    const entry = ready[i]
    if (!entry) continue
    const hit = meta?.[i]
    if (hit && hit.state !== "same") {
      log.info("tui plugin metadata updated", {
        path: entry.spec,
        retry: entry.retry,
        state: hit.state,
        source: hit.entry.source,
        version: hit.entry.version,
        modified: hit.entry.modified,
      })
    }

    const info = createMeta(entry.source, entry.spec, entry.target, hit, entry.id)
    const themes = hit?.entry.themes ? { ...hit.entry.themes } : {}
    const plugin: PluginEntry = {
      id: entry.id,
      load: entry,
      meta: info,
      themes,
      plugin: entry.module.tui,
      enabled: true,
    }
    if (!addPluginEntry(state, plugin)) {
      ok = false
      continue
    }
    plugins.push(plugin)
  }

  return { plugins, ok }
}
