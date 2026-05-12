import type { MiddlewareHandler } from "hono"
import { WithInstance } from "@/project/with-instance"
import { AppFileSystem } from "@jekko-ai/core/filesystem"
import { WorkspaceContext } from "@/control-plane/workspace-context"
import { WorkspaceID } from "@/control-plane/schema"
import { Instance } from "@/project/instance"
import { LocalContext } from "@/util/local-context"

function ambientDirectory(): string | undefined {
  try {
    return Instance.directory
  } catch (err) {
    if (err instanceof LocalContext.NotFound) return undefined
    throw err
  }
}

export function InstanceMiddleware(workspaceID?: WorkspaceID): MiddlewareHandler {
  return async (c, next) => {
    const raw = c.req.query("directory") || c.req.header("x-jekko-directory") || ambientDirectory() || process.cwd()
    const directory = AppFileSystem.resolve(
      (() => {
        try {
          return decodeURIComponent(raw)
        } catch {
          return raw
        }
      })(),
    )

    return WorkspaceContext.provide({
      workspaceID,
      async fn() {
        return WithInstance.provide({
          directory,
          async fn() {
            return next()
          },
        })
      },
    })
  }
}
