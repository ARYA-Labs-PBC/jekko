import { WithInstance } from "@/project/with-instance"
import { TuiConfig } from "@/cli/cmd/tui/config/tui"
import { Flag } from "@jekko-ai/core/flag/flag"
import { fail, log, type Api, type RuntimeState, createMeta, loadInternalPlugin } from "./runtime-core"
import { activatePluginEntry, addPluginEntry, applyInitialPluginEnabledState } from "./runtime-api"
import { INTERNAL_TUI_PLUGINS } from "./internal"
import { setupSlots } from "./slots"
import { setRuntimeState } from "./runtime-state"
import {
  addExternalPluginEntries,
  addPluginBySpec,
  defaultPluginOrigin,
  installCause,
  installDetail,
  installPluginBySpec,
  resolveExternalPlugins,
} from "./runtime-load-shared"

async function load(input: { api: Api; config: TuiConfig.Info }) {
  const { api, config } = input
  const cwd = process.cwd()
  const slots = setupSlots(api)
  const next: RuntimeState = {
    directory: cwd,
    api,
    slots,
    plugins: [],
    plugins_by_id: new Map(),
    pending: new Map(),
  }
  setRuntimeState(next)
  try {
    await WithInstance.provide({
      directory: cwd,
      fn: async () => {
        const records = Flag.JEKKO_PURE ? [] : (config.plugin_origins ?? [])
        if (Flag.JEKKO_PURE && config.plugin_origins?.length) {
          log.info("skipping external tui plugins in pure mode", { count: config.plugin_origins.length })
        }

        for (const item of INTERNAL_TUI_PLUGINS) {
          log.info("loading internal tui plugin", { id: item.id })
          const entry = loadInternalPlugin(item)
          const meta = createMeta(entry.source, entry.spec, entry.target, undefined, entry.id)
          addPluginEntry(next, {
            id: entry.id,
            load: entry,
            meta,
            themes: {},
            plugin: entry.module.tui,
            enabled: true,
          })
        }

        const ready = await resolveExternalPlugins(records, () => TuiConfig.waitForDependencies())
        await addExternalPluginEntries(next, ready)

        applyInitialPluginEnabledState(next, config)
        for (const plugin of next.plugins) {
          if (!plugin.enabled) continue
          // Keep plugin execution sequential for deterministic side effects:
          // command registration order affects keybind/command precedence,
          // route registration is last-wins when ids collide,
          // and hook chains rely on stable plugin ordering.
          await activatePluginEntry(next, plugin, false)
        }
      },
    })
  } catch (error) {
    fail("failed to load tui plugins", { directory: cwd, error })
  }
}

export {
  addExternalPluginEntries,
  addPluginBySpec,
  defaultPluginOrigin,
  installCause,
  installDetail,
  installPluginBySpec,
  load,
  resolveExternalPlugins,
}
