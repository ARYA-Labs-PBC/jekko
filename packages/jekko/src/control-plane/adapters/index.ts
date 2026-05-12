import type { ProjectID } from "@/project/schema"
import type { WorkspaceAdapter, WorkspaceAdapterEntry } from "../types"
import type { WorkspaceAdapter as PluginWorkspaceAdapter } from "@jekko-ai/plugin"
import { Schema } from "effect"
import { WorktreeAdapter } from "./worktree"
import { WorkspaceInfo } from "../types"

const decodeWorkspaceInfo = Schema.decodeUnknownSync(WorkspaceInfo)

const BUILTIN: Record<string, WorkspaceAdapter> = {
  worktree: WorktreeAdapter,
}

const state = new Map<ProjectID, Map<string, WorkspaceAdapter>>()

export function getAdapter(projectID: ProjectID, type: string): WorkspaceAdapter {
  const custom = state.get(projectID)?.get(type)
  if (custom) return custom

  const builtin = BUILTIN[type]
  if (builtin) return builtin

  throw new Error(`Unknown workspace adapter: ${type}`)
}

export async function listAdapters(projectID: ProjectID): Promise<WorkspaceAdapterEntry[]> {
  const builtin = await Promise.all(
    Object.entries(BUILTIN).map(async ([type, adapter]) => {
      return {
        type,
        name: adapter.name,
        description: adapter.description,
      }
    }),
  )
  const custom = [...(state.get(projectID)?.entries() ?? [])].map(([type, adapter]) => ({
    type,
    name: adapter.name,
    description: adapter.description,
  }))
  return [...builtin, ...custom]
}

// Plugins can be loaded per-project so we need to scope them. If you
// want to install a global one pass `ProjectID.global`
export function registerAdapter(projectID: ProjectID, type: string, adapter: PluginWorkspaceAdapter) {
  const wrapped: WorkspaceAdapter = {
    name: adapter.name,
    description: adapter.description,
    async configure(info) {
      return decodeWorkspaceInfo(await adapter.configure(decodeWorkspaceInfo(info)))
    },
    create(info, env, from) {
      return adapter.create(decodeWorkspaceInfo(info), env, from ? decodeWorkspaceInfo(from) : undefined)
    },
    remove(info) {
      return adapter.remove(decodeWorkspaceInfo(info))
    },
    target(info) {
      return adapter.target(decodeWorkspaceInfo(info))
    },
  }
  const adapters = state.get(projectID) ?? new Map<string, WorkspaceAdapter>()
  adapters.set(type, wrapped)
  state.set(projectID, adapters)
}
