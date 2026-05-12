import { describe, expect, test } from "bun:test"
import { WorkspaceID } from "../../src/control-plane/schema"
import { ProjectID } from "../../src/project/schema"
import * as WorkspaceOld from "../../src/control-plane/workspace"

describe("workspace-prior schemas and exports", () => {
  test("keeps the historical event type names", () => {
    expect(WorkspaceOld.Event.Ready.type).toBe("workspace.ready")
    expect(WorkspaceOld.Event.Failed.type).toBe("workspace.failed")
    expect(WorkspaceOld.Event.Status.type).toBe("workspace.status")
  })

  test("validates create input with workspace id, project id, branch, type, and extra", () => {
    const input = {
      id: WorkspaceID.ascending("wrk_schema_create"),
      type: "worktree",
      branch: "feature/schema",
      projectID: ProjectID.make("project-schema"),
      extra: { nested: true },
    }

    expect(WorkspaceOld.CreateInput.zod.parse(input)).toEqual(input)
    expect(() => WorkspaceOld.CreateInput.zod.parse({ ...input, id: "bad" })).toThrow()
    expect(() => WorkspaceOld.CreateInput.zod.parse({ ...input, branch: 1 })).toThrow()
  })
})
