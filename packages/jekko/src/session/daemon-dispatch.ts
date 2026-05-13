// Runtime evaluator for the `dispatch` block from PR1. The classifier writes
// `{lane,reason}` JSON to a known path; this module reads it, resolves the
// lane id back to the dispatch target (`fan_out` / `experiments` / `incubator`
// / `research` / `approvals.gates.<id>`) declared in the spec, and returns a
// typed plan the daemon can act on.
//
// PR5 ships parse + resolve only. Live dispatch into each downstream primitive
// is owned by the daemon loop; this module's job is to convert the route.json
// + dispatch spec into a single `DispatchPlan` value so the loop can stay
// decision-free.

import fs from "fs"
import path from "path"

import type { ZyalDispatch } from "@/agent-script/schema"

export type DispatchTarget =
  | { kind: "fan_out" }
  | { kind: "experiments" }
  | { kind: "incubator" }
  | { kind: "research" }
  | { kind: "approval_gate"; gateID: string }
  | { kind: "custom"; rawTarget: string }

export type DispatchPlan = {
  laneID: string
  reason?: string
  target: DispatchTarget
  fallback?: DispatchFallback
}

export type DispatchFallback = "pause" | "abort" | "skip" | "default"

export type DispatchResolveError = {
  kind: "no_dispatch_block" | "missing_route_file" | "invalid_route_json" | "unknown_lane" | "no_lanes"
  message: string
}

export type ResolveResult =
  | { ok: true; plan: DispatchPlan }
  | { ok: false; error: DispatchResolveError; fallback: DispatchFallback }

/** Resolve a dispatch decision from spec + route.json on disk. */
export function resolveDispatch(input: {
  spec: ZyalDispatch | undefined
  repoRoot: string
  /** Override the route.json location; defaults to `spec.classifier.write_to`. */
  routeFile?: string
}): ResolveResult {
  if (!input.spec) {
    return failure({ kind: "no_dispatch_block", message: "spec has no dispatch block" }, "skip")
  }
  if (input.spec.enabled === false) {
    return failure({ kind: "no_dispatch_block", message: "dispatch is disabled" }, "skip")
  }
  const fallback = (input.spec.on_no_match ?? "skip") as DispatchFallback
  const routePath = resolveRoutePath(input)
  if (!routePath) {
    return failure(
      { kind: "missing_route_file", message: "no write_to configured and no routeFile override" },
      fallback,
    )
  }
  let parsed: unknown
  try {
    const text = fs.readFileSync(routePath, "utf-8")
    parsed = JSON.parse(text)
  } catch (err) {
    return failure(
      { kind: "missing_route_file", message: `read ${routePath}: ${(err as Error).message}` },
      fallback,
    )
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    return failure(
      { kind: "invalid_route_json", message: `${routePath}: not a JSON object` },
      fallback,
    )
  }
  const route = parsed as Record<string, unknown>
  const laneID = typeof route.lane === "string" ? route.lane : undefined
  const reason = typeof route.reason === "string" ? (route.reason as string) : undefined
  const effectiveLane = laneID && laneID.length > 0 ? laneID : input.spec.default_lane
  if (!effectiveLane) {
    return failure({ kind: "unknown_lane", message: "no lane in route.json and no default_lane" }, fallback)
  }
  const lanes = input.spec.lanes ?? []
  if (lanes.length === 0) {
    return failure({ kind: "no_lanes", message: "spec.lanes is empty" }, fallback)
  }
  const found = lanes.find((lane) => lane.id === effectiveLane)
  if (!found) {
    return failure(
      {
        kind: "unknown_lane",
        message: `lane ${effectiveLane!} not declared in spec.lanes (have: ${lanes.map((l) => l.id).join(", ")})`,
      },
      fallback,
    )
  }
  return {
    ok: true,
    plan: {
      laneID: found.id,
      reason,
      target: parseTarget(found.dispatch_to),
      fallback,
    },
  }
}

export function parseTarget(rawTarget: string): DispatchTarget {
  if (rawTarget === "fan_out") return { kind: "fan_out" }
  if (rawTarget === "experiments") return { kind: "experiments" }
  if (rawTarget === "incubator") return { kind: "incubator" }
  if (rawTarget === "research") return { kind: "research" }
  const gateMatch = /^approvals\.gates\.(.+)$/.exec(rawTarget)
  if (gateMatch) return { kind: "approval_gate", gateID: gateMatch[1]! }
  return { kind: "custom", rawTarget }
}

function resolveRoutePath(input: { spec: ZyalDispatch | undefined; repoRoot: string; routeFile?: string }): string | undefined {
  if (input.routeFile) {
    return path.isAbsolute(input.routeFile) ? input.routeFile : path.join(input.repoRoot, input.routeFile)
  }
  const writeTo = input.spec?.classifier?.write_to
  if (!writeTo) return undefined
  return path.isAbsolute(writeTo) ? writeTo : path.join(input.repoRoot, writeTo)
}

function failure(error: DispatchResolveError, fallback: DispatchFallback): ResolveResult {
  return { ok: false, error, fallback }
}

export * as DaemonDispatch from "./daemon-dispatch"
