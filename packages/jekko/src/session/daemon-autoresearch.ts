import { mkdir, writeFile } from "fs/promises"
import path from "path"
import { Cause, Effect } from "effect"
import { InstanceState } from "@/effect/instance-state"
import { resolveInstanceRoot } from "@/project/instance-root"
import type { ZyalScript } from "@/agent-script/schema"
import { SessionID } from "./schema"
import type { DaemonStore } from "./daemon-store"
import type { Session } from "./session"
import type { SessionPrompt } from "./prompt"
import type { DaemonChecks } from "./daemon-checks"
import type { Worktree } from "@/worktree"
import type { InstanceStore } from "@/project/instance-store"

const JNOCCIO_PROVIDER_ID = "jnoccio"
const JNOCCIO_MODEL_ID = "jnoccio-fusion"

export type DaemonModelRef = {
  readonly providerID: string
  readonly modelID: string
}

export function resolveDaemonModel(): DaemonModelRef {
  return {
    providerID: JNOCCIO_PROVIDER_ID,
    modelID: JNOCCIO_MODEL_ID,
  }
}

export function normalizeDaemonSpec(spec: ZyalScript): {
  readonly spec: ZyalScript
  readonly reroutedProfiles: string[]
  readonly addedFleetJnoccio: boolean
} {
  const next = (typeof structuredClone === "function"
    ? structuredClone(spec)
    : JSON.parse(JSON.stringify(spec))) as ZyalScript

  const reroutedProfiles: string[] = []
  const profiles = next.models?.profiles
  if (profiles) {
    for (const [name, profile] of Object.entries(profiles)) {
      if (profile?.provider === JNOCCIO_PROVIDER_ID && profile?.model === JNOCCIO_MODEL_ID) continue
      profiles[name] = {
        ...profile,
        provider: JNOCCIO_PROVIDER_ID,
        model: JNOCCIO_MODEL_ID,
      }
      reroutedProfiles.push(name)
    }
    if (next.models.critic) {
      next.models.critic.must_use_different_provider = false
      next.models.critic.must_differ_from_builder = false
    }
  }

  const fleetMax = clampWorkerCount(
    next.fleet?.max_workers ??
      next.experiments?.max_parallel ??
      next.fan_out?.worker?.max_parallel ??
      next.experiments?.lanes?.length ??
      next.fan_out?.worker?.max_parallel ??
      1,
  )
  const addedFleetJnoccio = !next.fleet?.jnoccio
  next.fleet = {
    ...next.fleet,
    max_workers: fleetMax,
    isolation: next.fleet?.isolation ?? "git_worktree",
    jnoccio: next.fleet?.jnoccio ?? {
      enabled: true,
      base_url: "http://127.0.0.1:4317",
      metrics_ws: "/v1/jnoccio/metrics/ws",
      spawn_on_demand: false,
      register_workers: false,
      heartbeat_path: "/v1/jnoccio/agents/heartbeat",
      heartbeat_interval: "15s",
      max_instances: fleetMax,
    },
    telemetry: next.fleet?.telemetry,
  }

  if (next.fleet.jnoccio) {
    next.fleet.jnoccio.max_instances = next.fleet.jnoccio.max_instances ?? fleetMax
  }

  return { spec: next, reroutedProfiles, addedFleetJnoccio }
}

export function hasAutoResearch(spec: ZyalScript): boolean {
  return (spec.experiments?.lanes?.length ?? 0) > 0
}

export function daemonArtifactRoot(rootDir: string, runID: string) {
  return path.resolve(rootDir, ".jekko", "daemon", runID)
}

type LaneResult = {
  readonly laneId: string
  readonly sessionID: string
  readonly worktreePath: string
  readonly workItemID?: string
  readonly promptTokens: number
  readonly completionTokens: number
  readonly totalTokens: number
  readonly cost: number
  readonly diff: string
  readonly exit: string
  readonly error?: string
}

export function runAutoResearch(input: {
  run: DaemonStore.RunInfo
  spec: ZyalScript
  store: DaemonStore.Interface
  sessions: Session.Interface
  prompt: SessionPrompt.Interface
  checks: DaemonChecks.Interface
  worktree: Worktree.Interface
  instanceStore: InstanceStore.Interface
  transitionRun: (runID: string, patch: Partial<{
    status: DaemonStore.RunInfo["status"]
    phase: DaemonStore.RunInfo["phase"]
    iteration: number
    epoch: number
    last_error: string | null
    last_exit_result_json: Record<string, unknown> | null
    stopped_at: number | null
    active_session_id: SessionID
  }>) => Effect.Effect<DaemonStore.RunInfo | undefined, any, any>
  }) {
  return Effect.gen(function* () {
    const rootCtx = yield* InstanceState.context
    const rootDir = resolveInstanceRoot(rootCtx)
    const artifactRoot = daemonArtifactRoot(rootDir, input.run.id)
    const normalized = normalizeDaemonSpec(input.spec)
    const spec = normalized.spec
    const lanes = spec.experiments?.lanes ?? []

    const maxParallel = clampWorkerCount(
      Math.min(spec.experiments?.max_parallel ?? lanes.length, spec.fleet?.max_workers ?? lanes.length),
    )

    yield* Effect.promise(() => mkdir(artifactRoot, { recursive: true }))
    yield* Effect.promise(() => mkdir(path.join(artifactRoot, "reports", "lanes"), { recursive: true }))
    yield* Effect.promise(() => mkdir(path.join(artifactRoot, "memory"), { recursive: true }))

    yield* input.store.appendEvent({
      runID: input.run.id,
      iteration: input.run.iteration,
      eventType: "autoresearch.started",
      payload: {
        lane_count: lanes.length,
        max_parallel: maxParallel,
        rerouted_profiles: normalized.reroutedProfiles,
        added_fleet_jnoccio: normalized.addedFleetJnoccio,
      },
    })

    if (spec.fan_out?.split?.shell) {
      const split = yield* input.checks.runShellCheck({
        cwd: rootDir,
        command: spec.fan_out.split.shell,
        timeout: spec.fan_out.worker?.timeout ?? "15 minutes",
      })
      yield* input.store.appendEvent({
        runID: input.run.id,
        iteration: input.run.iteration,
        eventType: "autoresearch.split.completed",
        payload: {
          command: spec.fan_out.split.shell,
          exitCode: split.exitCode,
          matched: split.matched,
        },
      })
    }

    if (spec.experiments?.scoring?.command) {
      const scoring = yield* input.checks.runShellCheck({
        cwd: rootDir,
        command: spec.experiments.scoring.command,
        timeout: "15 minutes",
      })
      yield* input.store.appendEvent({
        runID: input.run.id,
        iteration: input.run.iteration,
        eventType: "autoresearch.scoring.completed",
        payload: {
          command: spec.experiments.scoring.command,
          exitCode: scoring.exitCode,
          matched: scoring.matched,
        },
      })
    }

    const workerMax = maxParallel
    const laneResults = yield* Effect.forEach(
      lanes,
      (lane, index) =>
        runLane({
          input,
          spec,
          lane,
          laneIndex: index,
          artifactRoot,
          rootDir,
          project: rootCtx.project,
        }),
      { concurrency: workerMax, discard: false },
    )

    yield* writeLaneSummaryArtifacts({
      artifactRoot,
      lanes: laneResults,
      goalDirection: spec.experiments?.scoring?.goal_direction === "minimize" ? "minimize" : "maximize",
    })

    if (spec.fan_out?.reduce?.command) {
      try {
        const reduced = yield* input.checks.runShellCheck({
          cwd: rootDir,
          command: spec.fan_out.reduce.command,
          timeout: "15 minutes",
        })
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "autoresearch.reduce.finished",
          payload: {
            command: spec.fan_out.reduce.command,
            exitCode: reduced.exitCode,
            matched: reduced.matched,
          },
        })
      } catch (error) {
        yield* input.store.appendEvent({
          runID: input.run.id,
          iteration: input.run.iteration,
          eventType: "autoresearch.reduce.failed",
          payload: { message: error instanceof Error ? error.message : String(error) },
        })
        yield* writeFallbackReduceArtifacts({ artifactRoot, lanes: laneResults })
      }
    } else {
      yield* writeFallbackReduceArtifacts({ artifactRoot, lanes: laneResults })
      yield* input.store.appendEvent({
        runID: input.run.id,
        iteration: input.run.iteration,
        eventType: "autoresearch.reduce.finished",
        payload: { command: null, exitCode: 0, matched: true },
      })
    }

    const tokenUsage = laneResults.reduce(
      (acc, lane) => {
        acc.input += lane.promptTokens
        acc.output += lane.completionTokens
        acc.total += lane.totalTokens
        acc.cost += lane.cost
        return acc
      },
      { input: 0, output: 0, total: 0, cost: 0 },
    )
    yield* input.store.appendIteration({
      runID: input.run.id,
      iteration: input.run.iteration + 1,
      sessionID: input.run.active_session_id,
      terminalReason: "autoresearch",
      result: {
        lane_count: laneResults.length,
        artifact_root: artifactRoot,
      },
      tokenUsage: {
        input: tokenUsage.input,
        output: tokenUsage.output,
        cache: 0,
        total: tokenUsage.total,
      },
      cost: tokenUsage.cost,
    })

    yield* input.transitionRun(input.run.id, {
      status: "satisfied",
      phase: "terminal",
      stopped_at: Date.now(),
      iteration: input.run.iteration + 1,
      last_exit_result_json: {
        autoresearch: true,
        lane_count: laneResults.length,
      },
    })

    return { laneResults, artifactRoot }
  })
}

function runLane(input: {
  input: Parameters<typeof runAutoResearch>[0]
  spec: ZyalScript
  lane: NonNullable<ZyalScript["experiments"]>["lanes"][number]
  laneIndex: number
  artifactRoot: string
  rootDir: string
  project: {
    id: string
    worktree: string
    vcs?: string
    name?: string
    sandboxes: string[]
    time: { created: number; updated: number }
  }
}) {
  return Effect.gen(function* () {
    const laneID = input.lane.id
    const workItems = input.spec.research?.question_bank?.work_items ?? []
    const workItem = workItems[input.laneIndex % Math.max(workItems.length, 1)]
    const laneRoot = path.join(input.artifactRoot, "reports", "lanes", laneID)
    const laneArtifactDir = laneRoot
    yield* Effect.promise(() => mkdir(laneArtifactDir, { recursive: true }))

    const isolation = input.lane.isolation ?? input.spec.fleet?.isolation ?? "git_worktree"
    const worktreeInfo =
      isolation === "git_worktree"
        ? yield* input.input.worktree.create({ name: laneID })
        : undefined
    const laneDirectory = worktreeInfo?.directory ?? input.rootDir

    const laneEffect = Effect.gen(function* () {
      const sessions = input.input.sessions
      const prompt = input.input.prompt
      const session = yield* sessions.fork({
        sessionID: SessionID.make(input.input.run.active_session_id),
        messageID: undefined,
      })
      const promptText = buildLanePrompt(input.spec, input.lane, laneID, input.laneIndex, workItem)
      yield* input.input.store.appendEvent({
        runID: input.input.run.id,
        iteration: input.input.run.iteration,
        eventType: "autoresearch.lane.started",
        payload: {
          lane_id: laneID,
          work_item_id: workItem?.id ?? null,
          hypothesis: input.lane.hypothesis,
          prompt_strategy: input.lane.prompt_strategy ?? null,
          worktree: laneDirectory,
          session_id: session.id,
        },
      })

      yield* prompt.prompt({
        sessionID: session.id,
        agent: input.lane.agent ?? "build",
        model: resolveDaemonModel() as any,
        noReply: true,
        parts: [{ type: "text", text: promptText }],
      })
      const loop = yield* prompt.loopResult({ sessionID: session.id })
      const assistant = loop.message.info as Record<string, any>
      const routeMetadata = routeMetadataFromAssistant(assistant, input.input.run.id, laneID, input.lane.agent ?? "build")

      const diff = yield* input.input.checks.runShellCheck({
        cwd: laneDirectory,
        command: "git diff --binary HEAD",
        timeout: input.lane.timeout ?? "15 minutes",
      })
      const patch = diff.stdout

      const laneSummary = {
        lane_id: laneID,
        lane_index: input.laneIndex,
        hypothesis: input.lane.hypothesis,
        prompt_strategy: input.lane.prompt_strategy ?? null,
        worktree: laneDirectory,
        session_id: session.id,
        assistant_message_id: assistant?.id ?? null,
        assistant_finish: assistant?.finish ?? null,
        work_item: workItem ?? null,
        route_metadata: routeMetadata,
        prompt_tokens: numberFrom(assistant?.tokens?.input ?? assistant?.tokens?.inputTokens),
        completion_tokens: numberFrom(assistant?.tokens?.output ?? assistant?.tokens?.outputTokens),
        total_tokens: numberFrom(assistant?.tokens?.total ?? assistant?.tokens?.totalTokens),
        cost_usd: numberFrom(assistant?.cost),
        diff_bytes: patch.length,
      }

      yield* Effect.promise(() => writeFile(path.join(laneArtifactDir, "report.json"), JSON.stringify(laneSummary, null, 2) + "\n"))
      yield* Effect.promise(() =>
        writeFile(
          path.join(laneArtifactDir, "agent-attempt.json"),
          JSON.stringify(
            {
              lane_id: laneID,
              work_item: workItem ?? null,
              route_metadata: routeMetadata,
              terminal: loop.terminal,
              accepted: false,
            },
            null,
            2,
          ) + "\n",
        ),
      )
      yield* Effect.promise(() => writeFile(path.join(laneArtifactDir, "patch.diff"), patch))

      yield* input.input.store.appendEvent({
        runID: input.input.run.id,
        iteration: input.input.run.iteration,
        eventType: "autoresearch.lane.finished",
        payload: laneSummary,
      })

      return {
        laneId: laneID,
        sessionID: session.id,
        worktreePath: laneDirectory,
        workItemID: workItem?.id,
        promptTokens: numberFrom(assistant?.tokens?.input ?? assistant?.tokens?.inputTokens),
        completionTokens: numberFrom(assistant?.tokens?.output ?? assistant?.tokens?.outputTokens),
        totalTokens: numberFrom(assistant?.tokens?.total ?? assistant?.tokens?.totalTokens),
        cost: numberFrom(assistant?.cost),
        diff: patch,
        exit: loop.terminal,
      } satisfies LaneResult
    }).pipe(
      Effect.catchCause((cause) =>
        Effect.gen(function* () {
          const error = Cause.squash(cause)
          const message = error instanceof Error ? error.message : Cause.pretty(cause)
          const failureSummary = {
            lane_id: laneID,
            lane_index: input.laneIndex,
            hypothesis: input.lane.hypothesis,
            prompt_strategy: input.lane.prompt_strategy ?? null,
            work_item: workItem ?? null,
            route_metadata: routeMetadataFromAssistant(undefined, input.input.run.id, laneID, input.lane.agent ?? "build"),
            worktree: laneDirectory,
            session_id: null,
            assistant_message_id: null,
            assistant_finish: null,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cost_usd: 0,
            diff_bytes: 0,
            error: message,
          }

          yield* Effect.promise(() => writeFile(path.join(laneArtifactDir, "report.json"), JSON.stringify(failureSummary, null, 2) + "\n"))
          yield* Effect.promise(() => writeFile(path.join(laneArtifactDir, "agent-attempt.json"), JSON.stringify(failureSummary, null, 2) + "\n"))
          yield* Effect.promise(() => writeFile(path.join(laneArtifactDir, "patch.diff"), ""))
          yield* input.input.store.appendEvent({
            runID: input.input.run.id,
            iteration: input.input.run.iteration,
            eventType: "autoresearch.lane.failed",
            payload: failureSummary,
          })

          return {
            laneId: laneID,
            sessionID: "",
            worktreePath: laneDirectory,
            workItemID: workItem?.id,
            promptTokens: 0,
            completionTokens: 0,
            totalTokens: 0,
            cost: 0,
            diff: "",
            exit: "error",
            error: message,
          } satisfies LaneResult
        }),
      ),
    )

    if (worktreeInfo) {
      return yield* input.input.instanceStore.provide(
        {
          directory: worktreeInfo.directory,
          worktree: input.rootDir,
          project: input.project,
        },
        laneEffect,
      )
    }

    return yield* laneEffect
  })
}

function buildLanePrompt(
  spec: ZyalScript,
  lane: NonNullable<ZyalScript["experiments"]>["lanes"][number],
  laneID: string,
  laneIndex: number,
  workItem?: NonNullable<NonNullable<ZyalScript["research"]>["question_bank"]>["work_items"][number],
) {
  return [
    `AutoResearch lane ${laneIndex + 1}: ${laneID}`,
    `Objective: ${spec.job.objective}`,
    `Hypothesis: ${lane.hypothesis}`,
    `Prompt strategy: ${lane.prompt_strategy ?? "default"}`,
    workItem ? `Question-bank work item: ${JSON.stringify(workItem)}` : undefined,
    spec.research?.context_packing ? `Context packing: ${JSON.stringify(spec.research.context_packing)}` : undefined,
    spec.research?.question_bank?.acceptance ? `Acceptance thresholds: ${JSON.stringify(spec.research.question_bank.acceptance)}` : undefined,
    `Use the Jnoccio model identity for all assistant calls.`,
    `Make the smallest useful change for this lane and preserve deterministic outputs.`,
  ].filter(Boolean).join("\n")
}

function routeMetadataFromAssistant(
  assistant: Record<string, any> | undefined,
  runID: string,
  laneID: string,
  agentRole: string,
) {
  const jnoccio = assistant?.jnoccio ?? assistant?.metadata?.jnoccio ?? {}
  return {
    request_id: stringOrNull(jnoccio.request_id),
    route_mode: stringOrNull(jnoccio.route_mode),
    primary_model_id: stringOrNull(jnoccio.primary_model_id),
    backup_model_ids: Array.isArray(jnoccio.backup_model_ids) ? jnoccio.backup_model_ids.filter((value: unknown) => typeof value === "string") : [],
    fusion_model_id: stringOrNull(jnoccio.fusion_model_id),
    winner_model_id: stringOrNull(jnoccio.winner_model_id),
    confidence: numberOrNull(jnoccio.confidence),
    provider: stringOrNull(assistant?.providerID),
    model: stringOrNull(assistant?.modelID),
    agent_role: agentRole,
    zyal_run_id: runID,
    zyal_lane_id: laneID,
  }
}

function stringOrNull(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null
}

function numberOrNull(value: unknown): number | null {
  const next = Number(value)
  return Number.isFinite(next) ? next : null
}

function clampWorkerCount(value: number | undefined) {
  const next = Number(value)
  if (!Number.isFinite(next) || next <= 0) return 1
  return Math.min(20, Math.max(1, Math.floor(next)))
}

function numberFrom(value: unknown): number {
  const next = Number(value)
  return Number.isFinite(next) ? next : 0
}

function writeLaneSummaryArtifacts(input: {
  artifactRoot: string
  lanes: readonly LaneResult[]
  goalDirection: "maximize" | "minimize"
}) {
  return Effect.gen(function* () {
    const rows = [
      ["rank", "name", "source", "ci95_low", "total", "stress_total", "gate_count", "cost_usd", "delta", "status"].join("\t"),
      ...input.lanes.map((lane, index) =>
        [
          String(index + 1),
          lane.laneId,
          lane.exit,
          "0",
          String(lane.totalTokens),
          String(lane.totalTokens),
          "0",
          lane.cost.toFixed(4),
          "0",
          lane.error ? "fail" : "pass",
        ].join("\t"),
      ),
    ].join("\n")
    yield* Effect.promise(() => writeFile(path.join(input.artifactRoot, "scoreboard.tsv"), rows + "\n"))

    const best = input.lanes.length === 0
      ? undefined
      : [...input.lanes].sort((a, b) =>
          input.goalDirection === "maximize"
            ? b.totalTokens - a.totalTokens
            : a.totalTokens - b.totalTokens,
        )[0]
    const bestState = {
      goal_direction: input.goalDirection,
      winner: best
        ? {
            score: best.totalTokens,
            lane_id: best.laneId,
            iteration: 0,
            timestamp: Date.now(),
          }
        : null,
      selected: best
        ? {
            score: best.totalTokens,
            lane_id: best.laneId,
            iteration: 0,
            timestamp: Date.now(),
          }
        : null,
      current: best
        ? {
            score: best.totalTokens,
            lane_id: best.laneId,
            iteration: 0,
            timestamp: Date.now(),
          }
        : null,
    }
    yield* Effect.promise(() => writeFile(path.join(input.artifactRoot, "best-state.json"), JSON.stringify(bestState, null, 2) + "\n"))
    yield* Effect.promise(() =>
      writeFile(
        path.join(input.artifactRoot, "promotion-decision.json"),
        JSON.stringify({ promoted: !!best, lane_id: best?.laneId ?? null }, null, 2) + "\n",
      ),
    )
    yield* Effect.promise(() =>
      writeFile(
        path.join(input.artifactRoot, "negative-memory.jsonl"),
        input.lanes
          .filter((lane) => lane.error)
          .map((lane) => JSON.stringify({ lane_id: lane.laneId, reason: lane.error }))
          .join("\n") + "\n",
      ),
    )
    yield* Effect.promise(() =>
      writeFile(
        path.join(input.artifactRoot, "curriculum-proposals.json"),
        JSON.stringify(
          {
            proposals: input.lanes.map((lane) => ({
              lane_id: lane.laneId,
              hypothesis: lane.hypothesis,
              score: lane.totalTokens,
            })),
          },
          null,
          2,
        ) + "\n",
      ),
    )
    yield* Effect.promise(() => writeFile(path.join(input.artifactRoot, "best.patch"), best?.diff ?? ""))
    yield* Effect.promise(() =>
      writeFile(
        path.join(input.artifactRoot, "reports", "final-score.json"),
        JSON.stringify({ lanes: input.lanes.length, best: best?.laneId ?? null }, null, 2) + "\n",
      ),
    )
    yield* Effect.promise(() =>
      writeFile(
        path.join(input.artifactRoot, "reports", "final-score.md"),
        `# AutoResearch final score\n\n- lanes: ${input.lanes.length}\n- best: ${best?.laneId ?? "none"}\n`,
      ),
    )
  })
}

function writeFallbackReduceArtifacts(input: { artifactRoot: string; lanes: readonly LaneResult[] }) {
  return writeLaneSummaryArtifacts({
    artifactRoot: input.artifactRoot,
    lanes: input.lanes,
    goalDirection: "maximize",
  })
}
