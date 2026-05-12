import { makeWorkspaceCreate } from "./workspace-crud-create"
import { makeWorkspaceRead } from "./workspace-crud-read"
import { makeWorkspaceRemove } from "./workspace-crud-remove"
import { makeWorkspaceSessionWarp } from "./workspace-crud-session-warp"
import type { CrudDeps } from "./workspace-crud-types"

export function makeWorkspaceCrud(deps: CrudDeps) {
  const { list, get } = makeWorkspaceRead()
  const create = makeWorkspaceCreate(deps)
  const remove = makeWorkspaceRemove(deps)
  const sessionWarp = makeWorkspaceSessionWarp(deps, get)

  return {
    create,
    sessionWarp,
    list,
    get,
    remove,
  }
}

export const createWorkspaceCrud = makeWorkspaceCrud
