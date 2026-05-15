import { readFile } from "node:fs/promises"
import path from "node:path"
import { AppRuntime } from "@/effect/app-runtime"
import { Instance } from "@/project/instance"
import { InstanceStore } from "@/project/instance-store"
import { Server } from "@/server/server"
import {
  delay,
  fetchJnoccioMetrics,
  formatMetrics,
  formatMetricsDelta,
  isRecord,
  parseJsonPayload,
  readNumberField,
  readRecordField,
  readStringField,
  requestJson,
  runShellStep,
  streamDaemonArtifacts,
  summarizeEventPayload,
  tokenNumber,
  writeReceipt,
} from "./headless-helpers"
import type {
  HeadlessJnoccioMetrics,
  HeadlessRunOptions,
  HeadlessRunReceipt,
  HeadlessStepResult,
  PlannedShellStep,
} from "./headless"
import {
  computeRetryDelay,
  resolveRetryPolicy,
  isRetryableReason,
} from "@/session/daemon-retry"
import type { ZyalParsed, ZyalScript } from "@/agent-script/schema"

export class HeadlessRunError extends Error {
  constructor(
    message: string,
    readonly receipt: HeadlessRunReceipt,
  ) {
    super(message)
    this.name = "HeadlessRunError"
  }
}

export async function runShellOnlyHeadless(
  parsed: ZyalParsed,
  input: {
    cwd: string
    filePath: string
    started: Date
    receiptPath: string
    options: HeadlessRunOptions
  },
  planSteps: (spec: ZyalScript) => PlannedShellStep[],
): Promise<HeadlessRunReceipt> {
  const steps = planSteps(parsed.spec)
  if (steps.length === 0) {
    throw new Error(`ZYAL ${parsed.spec.id} has no headless shell steps`)
  }
  if (parsed.spec.fan_out?.worker !== undefined) {
    input.options.print?.(
      "headless: unsupported mode for shell-only run; fan_out.worker requires daemon mode. Rerun in daemon mode for worker-enabled specs.",
    )
  }

  const results: HeadlessStepResult[] = []
  let failed = false
  let failureMessage = ""

  for (const step of steps) {
    input.options.print?.(`headless: ${step.label}`)
    const result = await runShellStepWithRetry(parsed.spec, step, {
      cwd: input.cwd,
      env: {
        ...process.env,
        ...input.options.env,
        JEKKO_HEADLESS: "1",
        ZYAL_HEADLESS: "1",
        ZYAL_RUN_ID: parsed.spec.id,
        ZYAL_FILE: input.filePath,
        ZYAL_WORKSPACE: input.cwd,
      },
    })
    results.push(result)
    if (result.status === "failed") {
      failed = true
      failureMessage = `${step.label} failed with exit code ${result.exitCode ?? "signal " + result.signal}`
      break
    }
  }

  const finished = new Date()
  const receipt: HeadlessRunReceipt = {
    id: parsed.spec.id,
    file: input.filePath,
    spec_hash: parsed.specHash,
    status: failed ? "failed" : "passed",
    started_at: input.started.toISOString(),
    finished_at: finished.toISOString(),
    duration_ms: finished.getTime() - input.started.getTime(),
    headless: true,
    mode: "shell_only",
    worker_spec_present: parsed.spec.fan_out?.worker !== undefined,
    dev_only_smoke_present: steps.some((step) => step.command.includes("memory_benchmark_dev_qbank=1")),
    receipt_path: input.receiptPath,
    steps: results,
  }

  await writeReceipt(input.receiptPath, receipt)
  if (failed) throw new HeadlessRunError(`${failureMessage}; receipt: ${input.receiptPath}`, receipt)
  return receipt
}

async function runShellStepWithRetry(
  spec: ZyalScript,
  step: PlannedShellStep,
  input: { cwd: string; env: NodeJS.ProcessEnv },
): Promise<HeadlessStepResult> {
  const policy = resolveRetryPolicy(spec.retry, "shell_checks")
  const attempts: HeadlessStepResult[] = []
  for (let attempt = 1; attempt <= policy.max_attempts; attempt++) {
    const result = await runShellStep(step, input)
    attempts.push(result)
    if (result.status === "passed") return { ...result, attempts }
    const reason = result.signal ? "timeout" : "exit_nonzero"
    if (!isRetryableReason(policy, reason) || attempt >= policy.max_attempts) {
      return { ...result, attempts }
    }
    await delay(computeRetryDelay(policy, attempt - 1))
  }
  return attempts[attempts.length - 1] ?? await runShellStep(step, input)
}

export async function runDaemonHeadless(
  parsed: ZyalParsed,
  text: string,
  input: {
    cwd: string
    filePath: string
    started: Date
    receiptPath: string
    options: HeadlessRunOptions
  },
): Promise<HeadlessRunReceipt> {
  const localFetch = async (request: RequestInfo | URL, init?: RequestInit) => {
    const req = request instanceof Request ? request : new Request(request, init)
    return Server.Default().app.fetch(req)
  }
  const target = { baseUrl: "http://jekko.internal", fetch: localFetch as typeof fetch }
  const jnoccioBaseUrl = parsed.spec.fleet?.jnoccio?.base_url
  const metricsBefore = await fetchJnoccioMetrics(jnoccioBaseUrl)

  input.options.print?.(`headless: daemon preview ${parsed.spec.id}`)
  const preview = await requestJson(target, "/daemon/preview", {
    method: "POST",
    body: JSON.stringify({ text }),
  })

  let sessionID = ""
  if (isRecord(preview) && isRecord(preview.spec)) {
    const session = await requestJson(target, "/session", {
      method: "POST",
      body: JSON.stringify({
        title: readStringField(readRecordField(preview.spec, "job"), "name") ?? "ZYAL daemon",
      }),
    })
    sessionID = readStringField(isRecord(session) ? session : undefined, "id") ?? ""
  }
  if (!sessionID) {
    throw new Error("Failed to create or resolve a session for daemon headless run")
  }

  input.options.print?.(`headless: daemon start ${parsed.spec.id}`)
  const startedRun = await requestJson(target, `/session/${sessionID}/daemon/start`, {
    method: "POST",
    body: JSON.stringify({
      parts: [{ type: "text", text }],
    }),
  })
  const runID = readStringField(isRecord(startedRun) ? startedRun : undefined, "id") ?? ""
  if (!runID) {
    throw new Error("Failed to start daemon run")
  }
  const artifactRoot = path.join(input.cwd, ".jekko", "daemon", runID)
  input.options.print?.(`headless: daemon run ${runID} artifacts ${artifactRoot}`)

  const pollStarted = Date.now()
  const timeoutMs = input.options.headlessTimeoutMs
  const idleTimeoutMs = input.options.idleTimeoutMs ?? Math.min(timeoutMs ?? 15 * 60 * 1000, 15 * 60 * 1000)
  let finalRun: unknown = undefined
  let lastActivityAt = Date.now()
  let lastPollState = ""
  const streamed = { ledgerLines: 0, stateText: "" }
  while (true) {
    const poll = await requestJson(target, `/daemon/${runID}`)
    finalRun = poll
    const pollRecord = isRecord(poll) ? poll : undefined
    const status = readStringField(pollRecord, "status") ?? "running"
    const phase = readStringField(pollRecord, "phase") ?? "running"
    const iteration = readNumberField(pollRecord, "iteration") ?? 0
    input.options.print?.(`headless: daemon ${status} · ${phase} · iter ${iteration}`)
    const pollState = `${status}:${phase}:${iteration}`
    if (pollState !== lastPollState) {
      lastPollState = pollState
      lastActivityAt = Date.now()
    }
    if (await streamDaemonArtifacts(artifactRoot, input.options.print, streamed)) {
      lastActivityAt = Date.now()
    }
    if (["satisfied", "aborted", "failed"].includes(status)) break
    if (status === "paused") break
    if (timeoutMs !== undefined && Date.now() - pollStarted > timeoutMs) {
      finalRun = { ...(typeof finalRun === "object" && finalRun !== null ? finalRun : {}), status: "failed", timeout: true }
      break
    }
    if (Date.now() - lastActivityAt > idleTimeoutMs) {
      finalRun = { ...(typeof finalRun === "object" && finalRun !== null ? finalRun : {}), status: "failed", idle_timeout: true }
      break
    }
    await delay(1000)
  }

  const finished = new Date()
  const finalRunRecord = isRecord(finalRun) ? finalRun : undefined
  const status = readStringField(finalRunRecord, "status") ?? "failed"
  const timeoutReason = finalRunRecord?.timeout
    ? "headless_timeout"
    : finalRunRecord?.idle_timeout
      ? "idle_timeout"
      : undefined
  const terminal = status === "satisfied" ? "passed" : "failed"
  const metricsAfter = await fetchJnoccioMetrics(jnoccioBaseUrl)
  if (metricsBefore || metricsAfter) {
    input.options.print?.(
      `headless: jnoccio tokens before ${formatMetrics(metricsBefore)} after ${formatMetrics(metricsAfter)} delta ${formatMetricsDelta(metricsBefore, metricsAfter)}`,
    )
  }
  const receipt: HeadlessRunReceipt = {
    id: parsed.spec.id,
    file: input.filePath,
    spec_hash: parsed.specHash,
    status: terminal,
    started_at: input.started.toISOString(),
    finished_at: finished.toISOString(),
    duration_ms: finished.getTime() - input.started.getTime(),
    headless: true,
    mode: "daemon",
    worker_spec_present: parsed.spec.fan_out?.worker !== undefined,
    dev_only_smoke_present: false,
    receipt_path: input.receiptPath,
    steps: [
      {
        label: "daemon.run",
        command: `/session/${sessionID}/daemon/start`,
        cwd: input.cwd,
        status: terminal,
        exitCode: terminal === "passed" ? 0 : 1,
        signal: timeoutReason ? "SIGTERM" : null,
        durationMs: finished.getTime() - pollStarted,
      },
    ],
    daemon_run_id: runID,
    daemon_status: status,
    jnoccio_metrics_before: metricsBefore,
    jnoccio_metrics_after: metricsAfter,
  }

  await writeReceipt(input.receiptPath, receipt)
  if (terminal === "failed") {
    throw new HeadlessRunError(`daemon run ${runID} ended with ${timeoutReason ?? status}; receipt: ${input.receiptPath}`, receipt)
  }
  return receipt
}
