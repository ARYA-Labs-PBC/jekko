import { describe, expect, test } from "bun:test"
import fs from "node:fs/promises"
import path from "node:path"
import { Effect } from "effect"
import { Instance } from "@/project/instance"
import { Session as SessionNs } from "@/session/session"
import { AppRuntime } from "../../src/effect/app-runtime"
import { registerAdapter } from "../../src/control-plane/adapters"
import {
  attachSessionToWorkspace,
  eventuallyEffect,
  insertWorkspace,
  localAdapter,
  removeWorkspace,
  startWorkspaceSyncing,
  unique,
  workspaceInfo,
  workspaceStatus,
  withInstance,
} from "./workspace.test"

describe("workspace-prior sync state", () => {
  test("duplicate local status updates are suppressed", async () => {
    await withInstance(async (dir) => {
      const type = unique("dedupe-local")
      const info = workspaceInfo(Instance.project.id, type)
      const target = path.join(dir, "dedupe-local")
      await fs.mkdir(target, { recursive: true })
      insertWorkspace(info)
      registerAdapter(Instance.project.id, type, localAdapter(target).adapter)
      attachSessionToWorkspace((await AppRuntime.runPromise(SessionNs.Service.use((svc) => svc.create({})))).id, info.id)

      startWorkspaceSyncing(Instance.project.id)
      startWorkspaceSyncing(Instance.project.id)

      await eventuallyEffect(
        Effect.gen(function* () {
          const statuses = yield* workspaceStatus()
          expect(statuses.filter((item) => item.workspaceID === info.id).length).toBeLessThanOrEqual(1)
        }),
      )
      await removeWorkspace(info.id)
    })
  })
})
