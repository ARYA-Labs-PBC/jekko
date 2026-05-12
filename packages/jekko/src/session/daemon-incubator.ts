// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import { Effect } from "effect"
import { ulid } from "ulid"
import path from "path"
import type { ZyalParsed, ZyalIncubatorPass } from "@/agent-script/schema"
import { SessionID } from "./schema"
import type { Session } from "./session"
import type { SessionPrompt } from "./prompt"
import type { DaemonStore } from "./daemon-store"
import type { MCP } from "@/mcp"
import type { Worktree } from "@/worktree"
import { DaemonMcp } from "./daemon-mcp"
import { DaemonTaskRouter } from "./daemon-task-router"
import { DaemonTaskMemory } from "./daemon-task-memory"
import { DaemonTaskPromote } from "./daemon-task-promote"
import { DaemonPass, type PassReceipt } from "./daemon-pass"

export type TickResult =
  | { action: "disabled" }
  | { action: "idle" }
  | { action: "routed"; taskID: string; lane: string; reasons: string[] }
  | { action: "pass"; taskID: string; passID: string; passType: string }
  | { action: "promoted"; taskID: string; score: number }
  | { action: "exhausted"; taskID: string; reason: string }
  | { action: "blocked"; reason: string }

function tickDisabled(): TickResult {
  return { action: "disabled" }
}

function tickIdle(): TickResult {
  return { action: "idle" }
}

function tickRouted(taskID: string, lane: string, reasons: string[]): TickResult {
  return { action: "routed", taskID, lane, reasons }
}

function tickPass(taskID: string, passID: string, passType: string): TickResult {
  return { action: "pass", taskID, passID, passType }
}

function tickPromoted(taskID: string, score: number): TickResult {
  return { action: "promoted", taskID, score }
}

function tickExhausted(taskID: string, reason: string): TickResult {
  return { action: "exhausted", taskID, reason }
}

function tickBlocked(reason: string): TickResult {
  return { action: "blocked", reason }
}

export function tick(input: {
  run: DaemonStore.RunInfo
  parsed: ZyalParsed
  sessions: Session.Interface
  store: DaemonStore.Interface
  prompt: SessionPrompt.Interface
  mcp: MCP.Interface
  worktree: Worktree.Interface
}): Effect.Effect<TickResult, any, any> {
  return Effect.gen(function* () {
    const incubator = input.parsed.spec.incubator
    if (!incubator?.enabled) return tickDisabled()

    const tasks = yield* input.store.listTasks(input.run.id)
    const active = tasks.find((task) => task.lane === "incubator" && ["incubating", "queued"].includes(task.status))
    if (active) return yield* runTaskPass({ ...input, task: active })

    const candidate = tasks.find((task) => ["queued", "leased"].includes(task.status))
    if (!candidate) return tickIdle()

    const body = candidate.body_json as Record<string, unknown>
    const touchedPaths = Array.isArray(body.touched_paths)
      ? body.touched_paths.filter((item): item is string => typeof item === "string")
      : Array.isArray(body.paths)
        ? body.paths.filter((item): item is string => typeof item === "string")
        : []
    const route = DaemonTaskRouter.routeTask({
      task: candidate,
      incubator,
      touchedPaths,
      evidence: readEvidence(body),
    })
    yield* input.store.assessTask({
      taskID: candidate.id,
      readinessScore: route.readinessScore,
      riskScore: route.riskScore,
      assessment: { route, touchedPaths },
    })
    if (route.lane !== "incubator") return tickRouted(candidate.id, "normal", [])
    yield* input.store.routeTask({
      taskID: candidate.id,
      lane: "incubator",
      phase: "routing_tasks",
      incubatorStatus: "active",
    })
    yield* input.store.appendTaskMemory({
      runID: candidate.run_id,
      taskID: candidate.id,
      kind: "problem_statement",
      title: candidate.title,
      summary: taskObjective(candidate),
      payload: { touchedPaths, route },
      importance: 0.8,
      confidence: 0.7,
    })
    return tickRouted(candidate.id, "incubator", route.reasons)
  })
}

function runTaskPass(input: {
  run: DaemonStore.RunInfo
  parsed: ZyalParsed
  sessions: Session.Interface
  store: DaemonStore.Interface
  prompt: SessionPrompt.Interface
  task: DaemonStore.TaskInfo
  mcp: MCP.Interface
  worktree: Worktree.Interface
}): Effect.Effect<TickResult, any, any> {
  return Effect.gen(function* () {
    const incubator = input.parsed.spec.incubator
    if (!incubator?.enabled) return { action: "disabled" } as const
    const passes = yield* input.store.listTaskPasses({ runID: input.run.id, taskID: input.task.id })
    const memories = yield* input.store.listTaskMemory({ runID: input.run.id, taskID: input.task.id })
    const promotion = DaemonTaskPromote.evaluatePromotion({
      task: input.task,
      memories,
      passes,
      promotion: incubator.promotion,
      maxPasses: incubator.budget.max_passes_per_task,
    })
    if (promotion.promote) {
      yield* input.store.promoteTask({ taskID: input.task.id, result: promotion })
      return tickPromoted(input.task.id, promotion.score)
    }
    if (promotion.exhausted) {
      yield* input.store.exhaustTask({
        taskID: input.task.id,
        reason: promotion.blockers.join(", ") || "incubator budget exhausted",
        result: promotion,
      })
      return tickExhausted(input.task.id, "incubator budget exhausted")
    }

    const pass = nextPass(incubator.passes, passes.length)
    if (!pass) {
      yield* input.store.exhaustTask({ taskID: input.task.id, reason: "no incubator passes configured", result: promotion })
      return tickExhausted(input.task.id, "no incubator passes configured")
    }
    if (pass.mcp_profile) {
      const status = yield* input.mcp.status()
      const gate = DaemonMcp.checkRequiredProfiles({
        mcp: input.parsed.spec.mcp,
        profile: pass.mcp_profile,
        status,
      })
      if (!gate.ok) {
        const reason = gate.blocked.map((item) => `${item.server}:${item.status}`).join(", ") || "mcp profile blocked"
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "mcp.blocked",
          payload: { taskID: input.task.id, passID: pass.id, reason, blocked: gate.blocked },
        })
        yield* input.store.updateRun(input.run.id, {
          status: "paused",
          phase: "paused",
          last_error: reason,
        })
        return tickBlocked(reason)
      }
    }
    const prototype =
      pass.writes === "isolated_worktree"
        ? yield* createPrototype({ ...input, pass })
        : undefined
    const passSessionID =
      prototype !== undefined
        ? prototype.session.id
        : (yield* input.sessions.fork({ sessionID: SessionID.make(input.run.active_session_id), messageID: undefined })).id
    const started = yield* input.store.beginTaskPass({
      runID: input.run.id,
      taskID: input.task.id,
      passType: pass.type,
      contextMode: pass.context,
      agent: pass.agent,
      sessionID: passSessionID,
      worktreePath: prototype?.worktree.directory,
      worktreeBranch: prototype?.worktree.branch,
    })
    const receipt = yield* executePass({ ...input, pass, taskPass: started, memories, passes, prototype }).pipe(
      Effect.catch((error) =>
        Effect.gen(function* () {
          yield* input.store.failTaskPass({
            passID: started.id,
            error: { message: error instanceof Error ? error.message : String(error) },
            cleanupStatus: prototype ? "failed" : "none",
          })
          // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
          return undefined
        }),
      ),
    )
    if (!receipt) return tickPass(input.task.id, started.id, pass.type)
    const kind = DaemonPass.memoryKindForPass(pass.type, receipt)
    yield* input.store.appendTaskMemory({
      runID: input.run.id,
      taskID: input.task.id,
      kind,
      title: receipt.title ?? `${pass.id} ${pass.type}`,
      summary: receipt.summary,
      payload: receipt as Record<string, unknown>,
      sourcePassID: started.id,
      importance: pass.type === "promotion_review" || pass.type === "synthesize" ? 0.85 : 0.65,
      confidence: averageClaimConfidence(receipt),
    })
    const readiness = Math.max(
      input.task.readiness_score,
      Math.min(1, input.task.readiness_score + readinessDelta(pass, receipt)),
    )
    yield* input.store.assessTask({
      taskID: input.task.id,
      readinessScore: readiness,
      assessment: { lastPassID: started.id, receipt },
    })
    yield* input.store.completeTaskPass({
      passID: started.id,
      result: receipt as Record<string, unknown>,
      score: { readiness },
      cleanupStatus: prototype ? "removed" : "none",
    })
    return tickPass(input.task.id, started.id, pass.type)
  })
}

function executePass(input: {
  run: DaemonStore.RunInfo
  parsed: ZyalParsed
  sessions: Session.Interface
  store: DaemonStore.Interface
  prompt: SessionPrompt.Interface
  mcp: MCP.Interface
  worktree: Worktree.Interface
  task: DaemonStore.TaskInfo
  pass: ZyalIncubatorPass
  taskPass: DaemonStore.TaskPassInfo
  memories: DaemonStore.TaskMemoryInfo[]
  passes: DaemonStore.TaskPassInfo[]
  prototype?: {
    session: Session.Info
    worktree: { directory: string }
  }
}): Effect.Effect<PassReceipt | undefined, any, any> {
  return Effect.gen(function* () {
    const parent = yield* input.sessions.get(SessionID.make(input.run.active_session_id))
    const packet = DaemonTaskMemory.buildContextPacket({
      task: input.task,
      memories: input.memories,
      passes: input.passes,
      mode: input.pass.context,
      passType: input.pass.type,
    })
    const text = [packet, "", DaemonPass.passInstruction(input.pass)].join("\n")
    if (input.pass.writes === "isolated_worktree") {
      const prototype = input.prototype
      if (!prototype) throw new Error("Prototype pass missing worktree/session")
      try {
        yield* input.prompt.prompt({
          sessionID: prototype.session.id,
          agent: input.pass.agent ?? parent.agent,
          model: parent.model
            ? {
                providerID: parent.model.providerID,
                modelID: parent.model.id,
              }
            : undefined,
          tools: incubatorToolMap(input.pass, input.parsed.spec.mcp),
          parts: [{ type: "text", text }],
        })
        const diff = yield* input.sessions.diff(prototype.session.id)
        const artifact = yield* input.store.upsertArtifact({
          id: ulid(),
          run_id: input.run.id,
          task_id: input.task.id,
          pass_id: input.taskPass.id,
          kind: "prototype_diff",
          path_or_ref: `.jekko/daemon/${input.run.id}/tasks/${input.task.id}/PASSES/${input.taskPass.pass_number}-prototype.diff`,
          sha: null,
          payload_json: { worktree: prototype.worktree.directory, diff },
        } as any)
        const last = yield* input.sessions.messages({ sessionID: prototype.session.id, limit: 1 })
        const assistant = last.find((message) => message.info.role === "assistant") ?? last[0]
        const content = assistant?.parts
          .filter((part) => part.type === "text")
          .map((part) => part.text)
          .join("\n")
        return DaemonPass.normalizeReceipt(content, {
          passType: input.pass.type,
          title: input.pass.id,
        })
      } finally {
        yield* input.worktree.remove({ directory: prototype.worktree.directory }).pipe(
          Effect.catch((error) =>
            input.store
              .appendEvent({
                runID: input.run.id,
                iteration: input.run.iteration,
                eventType: "prototype.cleanup_failed",
                payload: { taskID: input.task.id, passID: input.taskPass.id, error: String(error) },
              })
              .pipe(Effect.ignore),
          ),
        )
      }
    }

    const childID = input.taskPass.session_id ? SessionID.make(input.taskPass.session_id) : parent.id
    const child = yield* input.sessions.get(childID)
    yield* input.prompt.prompt({
      sessionID: child.id,
      agent: input.pass.agent ?? parent.agent,
      model: parent.model
        ? {
            providerID: parent.model.providerID,
            modelID: parent.model.id,
          }
        : undefined,
      tools: incubatorToolMap(input.pass, input.parsed.spec.mcp),
      parts: [{ type: "text", text }],
    })
    const last = yield* input.sessions.messages({ sessionID: child.id, limit: 1 })
    const assistant = last.find((message) => message.info.role === "assistant") ?? last[0]
    const content = assistant?.parts
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n")
    return DaemonPass.normalizeReceipt(content, { passType: input.pass.type, title: input.pass.id })
  })
}

function createPrototype(input: {
  run: DaemonStore.RunInfo
  parsed: ZyalParsed
  sessions: Session.Interface
  worktree: Worktree.Interface
  task: DaemonStore.TaskInfo
  pass: ZyalIncubatorPass
}) {
  return Effect.gen(function* () {
    const parent = yield* input.sessions.get(SessionID.make(input.run.active_session_id))
    const worktree = yield* input.worktree.create({
      name: `daemon-${input.run.id}-${input.task.id}-${input.pass.id}`,
    })
    const session = yield* input.sessions.create({
      parentID: SessionID.make(input.run.active_session_id),
      title: `Prototype ${input.task.title}`,
      agent: input.pass.agent ?? parent.agent,
      model: parent.model
        ? {
            providerID: parent.model.providerID,
            id: parent.model.id,
            variant: parent.model.variant,
          }
        : undefined,
      permission: parent.permission,
      workspaceID: parent.workspaceID,
      directory: worktree.directory,
      path: path.relative(parent.directory, worktree.directory),
    })
    return { session, worktree }
  })
}

function nextPass(passes: readonly ZyalIncubatorPass[], completedCount: number) {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (passes.length === 0) return undefined
  return passes[completedCount % passes.length]
}

function incubatorToolMap(pass: ZyalIncubatorPass, mcp?: ZyalParsed["spec"]["mcp"]) {
  const writeDenied = pass.writes === "scratch_only"
  const mcpAllow = DaemonMcp.buildMcpToolAllowMap({ mcp, pass })
  return {
    read: true,
    grep: true,
    glob: true,
    edit: !writeDenied,
    write: !writeDenied,
    apply_patch: !writeDenied,
    shell: false,
    daemon_report_confidence: true,
    daemon_emit_idea: true,
    daemon_report_pass_result: true,
    daemon_memory_write: true,
    daemon_memory_read: true,
    daemon_request_incubation: true,
    daemon_request_promotion: true,
    daemon_context_snapshot: true,
    ...mcpAllow,
  }
}

function readEvidence(body: Record<string, unknown>) {
  const evidence = body.evidence
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!evidence || typeof evidence !== "object" || Array.isArray(evidence)) return undefined
  return evidence as Record<string, number>
}

function taskObjective(task: DaemonStore.TaskInfo) {
  const body = task.body_json as Record<string, unknown>
  return typeof body.objective === "string" ? body.objective : typeof body.goal === "string" ? body.goal : task.title
}

function readinessDelta(pass: ZyalIncubatorPass, receipt: PassReceipt) {
  if (receipt.readiness_delta !== undefined) return receipt.readiness_delta
  switch (pass.type) {
    case "scout":
      return 0.08
    case "idea":
      return 0.05
    case "strengthen":
      return 0.12
    case "critic":
      return 0.04
    case "synthesize":
      return 0.14
    case "prototype":
      return 0.1
    case "promotion_review":
      return 0.08
    case "compress":
      return 0.02
  }
}

function averageClaimConfidence(receipt: PassReceipt) {
  const claims = receipt.claims?.flatMap((claim) => (typeof claim.confidence === "number" ? [claim.confidence] : [])) ?? []
  if (!claims.length) return 0.5
  return claims.reduce((sum, value) => sum + value, 0) / claims.length
}

export * as DaemonIncubator from "./daemon-incubator"
