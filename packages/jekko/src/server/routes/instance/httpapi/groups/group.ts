import { HttpApiGroup, OpenApi } from "effect/unstable/httpapi"
import { Authorization } from "../middleware/authorization"
import { InstanceContextMiddleware } from "../middleware/instance-context"
import { WorkspaceRoutingMiddleware } from "../middleware/workspace-routing"

export function withInstanceGroupDefaults(
  group: ReturnType<typeof HttpApiGroup.make>,
  title: string,
  description: string,
) {
  return group
    .annotateMerge(
      OpenApi.annotations({
        title,
        description,
      }),
    )
    .middleware(InstanceContextMiddleware)
    .middleware(WorkspaceRoutingMiddleware)
    .middleware(Authorization)
}
