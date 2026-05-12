// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import type { ZyalParsed } from "@/agent-script/schema"
import type { RunInfo, IterationInfo, TaskInfo, WorkerInfo } from "./daemon-store"
import type { JankuraiConfig } from "./daemon-jankurai"
import { DaemonJankurai } from "./daemon-jankurai"

// Threshold for stall detection. If the same terminal reason appears in this
// many consecutive iterations without progress, we escalate the injected text
// to force the model to skip-and-continue rather than retrying the same path.
const STALL_THRESHOLD = 2

export function buildDaemonIterationPrompt(input: {
  parsed: ZyalParsed
  run: RunInfo
  lastIteration?: IterationInfo
  recentIterations?: IterationInfo[]
  locks?: string[]
  checkpointSha?: string
  jankurai?: {
    config: JankuraiConfig
    report?: unknown
    tasks: readonly TaskInfo[]
    workers: readonly WorkerInfo[]
    currentTask?: TaskInfo
    regression?: unknown
  }
}) {
  const interaction = input.parsed.spec.interaction
  const policy = input.parsed.spec.loop?.policy ?? "bounded"
  const lines = [`<system-reminder>`]

  // 1. system_inject (per-turn schema/tool-hygiene rules from the spec).
  if (
    interaction?.system_inject &&
    (interaction.user === "none" || interaction.user === "async")
  ) {
    lines.push(interaction.system_inject.trimEnd())
    lines.push(``)
  }

  // 2. Stall detection. If the last N iterations all returned the same terminal
  // reason without progress (esp. assistant_stop with no edits, or repeated
  // errors), inject an escalating "abandon this path" instruction so the loop
  // never gets stuck retrying a dead-end task in a forever daemon.
  const stall = detectStall(input.recentIterations)
  if (stall && policy === "forever") {
    lines.push(buildStallInjection(stall))
    lines.push(``)
  }

  lines.push(
    `ZYAL daemon iteration ${input.run.iteration + 1} for ${input.parsed.spec.job.name}.`,
    `Objective: ${input.parsed.spec.job.objective}`,
    `Loop policy: ${policy}`,
    `Stop checks: ${input.parsed.preview.stop_checks.join("; ") || "(none)"}`,
    input.parsed.preview.research_enabled
      ? `Research policy: ${input.parsed.preview.research_summary ?? "(configured)"}`
      : `Research policy: (none)`,
    input.parsed.preview.research_enabled
      ? `Research evidence: ${input.parsed.preview.research_evidence_summary ?? "(configured)"}`
      : `Research evidence: (none)`,
    `Checkpoint: ${input.checkpointSha ?? "(none)"}`,
    `Active locks: ${input.locks?.join(", ") || "(none)"}`,
    `Last iteration: ${input.lastIteration?.terminal_reason ?? "(none)"}`,
    ...(input.jankurai ? DaemonJankurai.promptSummaryLines(input.jankurai) : []),
    policy === "forever"
      ? `Continue with one bounded unit of work. The daemon runs forever — only the stop conditions in the spec end the run, never the model. If the current task is blocked, mark it Status: Blocked and CLAIM THE NEXT ONE. Never stop.`
      : `Continue with one bounded unit of work. The runtime decides whether the daemon is finished.`,
    `</system-reminder>`,
  )

  return lines.join("\n")
}

type StallInfo = {
  reason: string
  count: number
}

function detectStall(recent: IterationInfo[] | undefined): StallInfo | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!recent || recent.length < STALL_THRESHOLD) return undefined
  const tail = recent.slice(-STALL_THRESHOLD)
  const reason = tail[0]?.terminal_reason
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!reason) return undefined
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!tail.every((it) => it.terminal_reason === reason)) return undefined
  return { reason, count: tail.length }
}

function buildStallInjection(stall: StallInfo): string {
  return [
    `<stall-detected>`,
    `The last ${stall.count} iterations all ended with terminal reason "${stall.reason}".`,
    `This is a stall. Stop retrying the same approach.`,
    `If you are working a task: mark it Status: Blocked with the failure reason in JANKURAI_TASKLIST.md and CLAIM THE NEXT pending task immediately.`,
    `Do NOT pause. Do NOT ask the user. Do NOT repeat the same tool calls. Move to a different task.`,
    `</stall-detected>`,
  ].join("\n")
}

export function buildDaemonStateSummary(input: {
  parsed: ZyalParsed
  run: RunInfo
  iterations: IterationInfo[]
  lastError?: string | null
}) {
  return [
    `Run ${input.run.id}`,
    `Objective: ${input.parsed.spec.job.objective}`,
    `Status: ${input.run.status} / ${input.run.phase}`,
    `Iteration: ${input.run.iteration}`,
    `Epoch: ${input.run.epoch}`,
    `Last error: ${input.lastError ?? input.run.last_error ?? "(none)"}`,
    `Iterations recorded: ${input.iterations.length}`,
  ].join("\n")
}

export * as DaemonContext from "./daemon-context"
