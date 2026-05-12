import { TuiConfig } from "@/cli/cmd/tui/config/tui"
import { activatePluginById, deactivatePluginById } from "./runtime-api"
import { listPluginStatus, type Api } from "./runtime-core"
import { addPluginBySpec, installPluginBySpec, load } from "./runtime-load"
import { Slot as View } from "./slots"
import {
  getRuntimeDir,
  getRuntimeLoad,
  getRuntimeState,
  setRuntimeDir,
  setRuntimeLoad,
  setRuntimeState,
} from "./runtime-state"

export const Slot = View

export async function init(input: { api: Api; config: TuiConfig.Info }) {
  const cwd = process.cwd()
  const loaded = getRuntimeLoad()
  if (loaded) {
    if (getRuntimeDir() !== cwd) {
      throw new Error(`TuiPluginRuntime.init() called with a different working directory. expected=${getRuntimeDir()} got=${cwd}`)
    }
    return loaded
  }

  setRuntimeDir(cwd)
  const task = load(input)
  setRuntimeLoad(task)
  return task
}

export function list() {
  const runtime = getRuntimeState()
  if (!runtime) return []
  return listPluginStatus(runtime)
}

export async function activatePlugin(id: string) {
  return activatePluginById(getRuntimeState(), id, true)
}

export async function deactivatePlugin(id: string) {
  return deactivatePluginById(getRuntimeState(), id, true)
}

export async function addPlugin(spec: string) {
  return addPluginBySpec(getRuntimeState(), spec)
}

export async function installPlugin(spec: string, options?: { global?: boolean }) {
  return installPluginBySpec(getRuntimeState(), spec, options?.global)
}

export async function dispose() {
  const task = getRuntimeLoad()
  setRuntimeLoad(undefined)
  setRuntimeDir("")
  if (task) await task

  const state = getRuntimeState()
  setRuntimeState(undefined)
  if (!state) return

  const queue = [...state.plugins].reverse()
  for (const plugin of queue) {
    await deactivatePluginById(state, plugin.id, false)
  }
}
