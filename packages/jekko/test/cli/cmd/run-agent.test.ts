import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import { resolveRunAgent } from "../../../src/cli/cmd/run"

describe("resolveRunAgent", () => {
  test("returns default when no agent is requested", async () => {
    const result = await resolveRunAgent({}, { app: {} as any }, { get: () => Effect.succeed(undefined) })

    expect(result).toEqual({ kind: "default", reason: "not-requested" })
  })

  test("selects a local primary agent", async () => {
    const result = await resolveRunAgent(
      { agent: "coder" },
      { app: {} as any },
      { get: () => Effect.succeed({ mode: "primary" }) },
    )

    expect(result).toEqual({ kind: "selected", agent: "coder" })
  })

  test("falls back to default when an attached agent list omits the requested name", async () => {
    const sdk = {
      app: {
        agents: async () => ({ data: [{ name: "planner", mode: "primary" }] }),
      },
    } as any

    const result = await resolveRunAgent({ agent: "coder", attach: "http://localhost:4096" }, sdk, {
      get: () => Effect.succeed(undefined),
    })

    expect(result).toEqual({ kind: "default", reason: "missing" })
  })
})
