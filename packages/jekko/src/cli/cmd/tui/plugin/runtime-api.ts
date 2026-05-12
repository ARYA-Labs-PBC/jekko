import type { TuiPluginApi, TuiSlotPlugin } from "@jekko-ai/plugin/tui"
import { TuiConfig } from "@/cli/cmd/tui/config/tui"
import {
  createPluginScope,
  createThemeInstaller,
  KV_KEY,
  fail,
  listPluginStatus,
  readPluginEnabledMap,
  syncPluginThemes,
  type Api,
  type PluginEntry,
  type PluginScope,
  type RuntimeState,
  writePluginEnabledState,
} from "./runtime-core"
import { addPluginBySpec, installPluginBySpec } from "./runtime-load"

export function addPluginEntry(state: RuntimeState, plugin: PluginEntry) {
  if (state.plugins_by_id.has(plugin.id)) {
    fail("duplicate tui plugin id", {
      id: plugin.id,
      path: plugin.load.spec,
    })
    return false
  }

  state.plugins_by_id.set(plugin.id, plugin)
  state.plugins.push(plugin)
  return true
}

export function applyInitialPluginEnabledState(state: RuntimeState, config: TuiConfig.Info) {
  const map = {
    ...readPluginEnabledMap(config.plugin_enabled),
    ...readPluginEnabledMap(state.api.kv.get(KV_KEY, {})),
  }
  for (const plugin of state.plugins) {
    const enabled = map[plugin.id]
    if (enabled === undefined) continue
    plugin.enabled = enabled
  }
}

async function deactivatePluginEntry(state: RuntimeState, plugin: PluginEntry, persist: boolean) {
  plugin.enabled = false
  if (persist) writePluginEnabledState(state.api, plugin.id, false)
  if (!plugin.scope) return true
  const scope = plugin.scope
  plugin.scope = undefined
  await scope.dispose()
  return true
}

async function activatePluginEntry(state: RuntimeState, plugin: PluginEntry, persist: boolean) {
  plugin.enabled = true
  if (persist) writePluginEnabledState(state.api, plugin.id, true)
  if (plugin.scope) return true

  const scope = createPluginScope(plugin.load, plugin.id)
  const api = pluginApi(state, plugin, scope, plugin.id)
  const ok = await Promise.resolve()
    .then(async () => {
      await syncPluginThemes(plugin)
      await plugin.plugin(api, plugin.load.options, plugin.meta)
      return true
    })
    .catch((error) => {
      fail("failed to initialize tui plugin", {
        path: plugin.load.spec,
        id: plugin.id,
        error,
      })
      return false
    })

  if (!ok) {
    await scope.dispose()
    return false
  }

  if (!plugin.enabled) {
    await scope.dispose()
    return true
  }

  plugin.scope = scope
  return true
}

async function activatePluginById(state: RuntimeState | undefined, id: string, persist: boolean) {
  if (!state) return false
  const plugin = state.plugins_by_id.get(id)
  if (!plugin) return false
  return activatePluginEntry(state, plugin, persist)
}

async function deactivatePluginById(state: RuntimeState | undefined, id: string, persist: boolean) {
  if (!state) return false
  const plugin = state.plugins_by_id.get(id)
  if (!plugin) return false
  return deactivatePluginEntry(state, plugin, persist)
}

function pluginApi(runtime: RuntimeState, plugin: PluginEntry, scope: PluginScope, base: string): TuiPluginApi {
  const api = runtime.api
  const host = runtime.slots
  const load = plugin.load
  const command: TuiPluginApi["command"] = {
    register(cb) {
      return scope.track(api.command.register(cb))
    },
    trigger(value) {
      api.command.trigger(value)
    },
    show() {
      api.command.show()
    },
  }

  const route: TuiPluginApi["route"] = {
    register(list) {
      return scope.track(api.route.register(list))
    },
    navigate(name, params) {
      api.route.navigate(name, params)
    },
    navigateBack() {
      api.route.navigateBack()
    },
    get current() {
      return api.route.current
    },
  }

  const theme: TuiPluginApi["theme"] = Object.assign(Object.create(api.theme), {
    install: createThemeInstaller(load.origin, load.theme_root, load.spec, plugin),
  })

  const event: TuiPluginApi["event"] = {
    on(type, handler) {
      return scope.track(api.event.on(type, handler))
    },
  }

  let count = 0

  const slots: TuiPluginApi["slots"] = {
    register(plugin: TuiSlotPlugin) {
      const id = count ? `${base}:${count}` : base
      count += 1
      scope.track(host.register({ ...plugin, id }))
      return id
    },
  }

  return {
    app: api.app,
    command,
    route,
    ui: api.ui,
    keybind: api.keybind,
    tuiConfig: api.tuiConfig,
    kv: api.kv,
    state: api.state,
    theme,
    get client() {
      return api.client
    },
    event,
    renderer: api.renderer,
    slots,
    plugins: {
      list() {
        return listPluginStatus(runtime)
      },
      activate(id) {
        return activatePluginById(runtime, id, true)
      },
      deactivate(id) {
        return deactivatePluginById(runtime, id, true)
      },
      add(spec) {
        return addPluginBySpec(runtime, spec)
      },
      install(spec, options) {
        return installPluginBySpec(runtime, spec, options?.global)
      },
    },
    lifecycle: scope.lifecycle,
  }
}

export {
  activatePluginById,
  deactivatePluginById,
  deactivatePluginEntry,
  activatePluginEntry,
  pluginApi,
}
