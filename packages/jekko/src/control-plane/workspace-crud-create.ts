import { Slug } from "@jekko-ai/core/util/slug"
import { Effect } from "effect"
import { WorkspaceID } from "./schema"
import * as WS from "./workspace-shared"
import type { CrudDeps } from "./workspace-crud-types"

export function makeWorkspaceCreate(deps: CrudDeps) {
  return Effect.fn("Workspace.create")(function* (input: WS.CreateInput) {
    const id = WorkspaceID.ascending(input.id)
    const adapter = WS.getAdapter(input.projectID, input.type)
    const config = yield* WS.EffectBridge.fromPromise(() =>
      adapter.configure({ ...input, id, name: Slug.create(), directory: null, extra: input.extra ?? null }),
    )

    const info: WS.Info = {
      id,
      type: config.type,
      branch: config.branch ?? null,
      name: config.name ?? null,
      directory: config.directory ?? null,
      extra: config.extra ?? null,
      projectID: input.projectID,
    }

    yield* WS.db((db) => {
      db.insert(WS.WorkspaceTable)
        .values({
          id: info.id,
          type: info.type,
          branch: info.branch,
          name: info.name,
          directory: info.directory,
          extra: info.extra,
          project_id: info.projectID,
        })
        .run()
    })

    const env = {
      JEKKO_AUTH_CONTENT: JSON.stringify(yield* deps.auth.all()),
      JEKKO_WORKSPACE_ID: config.id,
      JEKKO_EXPERIMENTAL_WORKSPACES: "true",
      OTEL_EXPORTER_OTLP_HEADERS: process.env.OTEL_EXPORTER_OTLP_HEADERS,
      OTEL_EXPORTER_OTLP_ENDPOINT: process.env.OTEL_EXPORTER_OTLP_ENDPOINT,
      OTEL_RESOURCE_ATTRIBUTES: process.env.OTEL_RESOURCE_ATTRIBUTES,
    }

    yield* WS.EffectBridge.fromPromise(() => adapter.create(config, env))
    yield* deps.startSync(info).pipe(Effect.forkDetach)
    for (let attempt = 0; attempt < 200; attempt++) {
      const statuses = yield* deps.status()
      if (statuses.find((item) => item.workspaceID === info.id)?.status === "connected") break
      yield* Effect.sleep("100 millis")
    }

    return info
  })
}

