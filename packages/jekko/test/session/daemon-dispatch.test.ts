import { describe, expect, test } from "bun:test"
import fs from "fs"
import os from "os"
import path from "path"
import { DaemonDispatch, parseTarget, resolveDispatch } from "../../src/session/daemon-dispatch"
import type { ZyalDispatch } from "../../src/agent-script/schema"

function tempRoot(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "daemon-dispatch-"))
}

function writeRoute(root: string, file: string, content: unknown) {
  const abs = path.join(root, file)
  fs.mkdirSync(path.dirname(abs), { recursive: true })
  fs.writeFileSync(abs, JSON.stringify(content), "utf-8")
}

describe("parseTarget", () => {
  test("primitive targets", () => {
    expect(parseTarget("fan_out")).toEqual({ kind: "fan_out" })
    expect(parseTarget("experiments")).toEqual({ kind: "experiments" })
    expect(parseTarget("incubator")).toEqual({ kind: "incubator" })
    expect(parseTarget("research")).toEqual({ kind: "research" })
  })

  test("approval gate target carries gate id", () => {
    expect(parseTarget("approvals.gates.human_review")).toEqual({
      kind: "approval_gate",
      gateID: "human_review",
    })
  })

  test("unknown target is preserved as custom", () => {
    expect(parseTarget("my_custom_pipeline")).toEqual({ kind: "custom", rawTarget: "my_custom_pipeline" })
  })
})

const baseSpec: ZyalDispatch = {
  enabled: true,
  classifier: { command: "node classify.mjs", write_to: "target/dispatch/route.json" },
  lanes: [
    { id: "simple_parallel_batch", dispatch_to: "fan_out" },
    { id: "cap_incubator", dispatch_to: "incubator" },
    { id: "human_required", dispatch_to: "approvals.gates.human_review" },
  ],
  default_lane: "simple_parallel_batch",
  on_no_match: "default",
}

describe("resolveDispatch", () => {
  test("no spec -> skip", () => {
    const result = resolveDispatch({ spec: undefined, repoRoot: "/tmp" })
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.fallback).toBe("skip")
  })

  test("disabled spec -> skip", () => {
    const result = resolveDispatch({ spec: { enabled: false }, repoRoot: "/tmp" })
    expect(result.ok).toBe(false)
  })

  test("happy path resolves lane to primitive target", () => {
    const root = tempRoot()
    writeRoute(root, "target/dispatch/route.json", { lane: "cap_incubator", reason: "cap_finding" })
    const result = resolveDispatch({ spec: baseSpec, repoRoot: root })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.plan.laneID).toBe("cap_incubator")
      expect(result.plan.reason).toBe("cap_finding")
      expect(result.plan.target).toEqual({ kind: "incubator" })
    }
  })

  test("approval-gate lane resolves to gate target", () => {
    const root = tempRoot()
    writeRoute(root, "target/dispatch/route.json", { lane: "human_required" })
    const result = resolveDispatch({ spec: baseSpec, repoRoot: root })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.plan.target).toEqual({ kind: "approval_gate", gateID: "human_review" })
    }
  })

  test("missing lane id falls back to default_lane", () => {
    const root = tempRoot()
    writeRoute(root, "target/dispatch/route.json", {})
    const result = resolveDispatch({ spec: baseSpec, repoRoot: root })
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.plan.laneID).toBe("simple_parallel_batch")
    }
  })

  test("unknown lane returns error with declared fallback", () => {
    const root = tempRoot()
    writeRoute(root, "target/dispatch/route.json", { lane: "ghost" })
    const result = resolveDispatch({ spec: baseSpec, repoRoot: root })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.kind).toBe("unknown_lane")
      expect(result.fallback).toBe("default")
    }
  })

  test("missing route file surfaces error", () => {
    const root = tempRoot()
    const result = resolveDispatch({ spec: baseSpec, repoRoot: root })
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.kind).toBe("missing_route_file")
    }
  })

  test("absolute routeFile override is honored", () => {
    const root = tempRoot()
    const abs = path.join(root, "custom.json")
    fs.writeFileSync(abs, JSON.stringify({ lane: "simple_parallel_batch" }), "utf-8")
    const result = resolveDispatch({ spec: baseSpec, repoRoot: root, routeFile: abs })
    expect(result.ok).toBe(true)
  })

  test("namespace export mirrors top-level fns", () => {
    expect(DaemonDispatch.resolveDispatch).toBe(resolveDispatch)
    expect(DaemonDispatch.parseTarget).toBe(parseTarget)
  })
})
