import { Effect } from "effect"
import { spawn } from "node:child_process"
import { mkdir, readFile, writeFile } from "node:fs/promises"
import path from "node:path"
import { parseZyal } from "@/agent-script/parser"
import type { ZyalParsed, ZyalScript } from "@/agent-script/schema"
import { AppRuntime } from "@/effect/app-runtime"
import { Instance } from "@/project/instance"
import { InstanceStore } from "@/project/instance-store"
import { Server } from "@/server/server"

export type HeadlessStepStatus = "passed" | "failed"
export type HeadlessMode = "shell_only" | "daemon"

export interface HeadlessJnoccioMetrics {
  calls: number
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
}

export interface HeadlessStepResult {
  label: string
  command: string
  cwd: string
  timeout?: string
  status: HeadlessStepStatus
  exitCode: number | null
  signal: NodeJS.Signals | null
  durationMs: number
}

export interface HeadlessRunReceipt {
  id: string
  file: string
  spec_hash: string
  status: HeadlessStepStatus
  started_at: string
  finished_at: string
  duration_ms: number
  headless: true
  mode: HeadlessMode
  worker_spec_present: boolean
  dev_only_fallback_present: boolean
  receipt_path: string
  steps: HeadlessStepResult[]
  daemon_run_id?: string
  daemon_status?: string
  jnoccio_metrics_before?: HeadlessJnoccioMetrics
  jnoccio_metrics_after?: HeadlessJnoccioMetrics
}

export interface HeadlessRunOptions {
  cwd?: string
  env?: NodeJS.ProcessEnv
  receiptPath?: string
  headlessTimeoutMs?: number
  idleTimeoutMs?: number
  print?: (line: string) => void
}

interface PlannedShellStep {
  label: string
  command: string
  cwd?: string
  timeout?: string
}

export class HeadlessRunError extends Error {
  constructor(
    message: string,
    readonly receipt: HeadlessRunReceipt,
  ) {
    super(message)
    this.name = "HeadlessRunError"
  }
}

export function parseHeadlessArgs(args: string[]): { file: string; cwd?: string; timeout?: string } | null {
  const cwd = readOption(args, "--headless-cwd")
  const timeout = readOption(args, "--headless-timeout")
  const equals = args.find((arg) => arg.startsWith("--headless="))
  if (equals) {
    const file = equals.slice("--headless=".length)
    if (!file) throw new Error("--headless requires a ZYAL file path")
    return { file, cwd, ...(timeout ? { timeout } : {}) }
  }

  const index = args.indexOf("--headless")
  if (index === -1) return null
  const file = args[index + 1]
  if (!file || file.startsWith("-")) throw new Error("--headless requires a ZYAL file path")
  return { file, cwd, ...(timeout ? { timeout } : {}) }
}

export async function runHeadlessCli(args: string[], options: HeadlessRunOptions = {}): Promise<HeadlessRunReceipt | null> {
  const parsed = parseHeadlessArgs(args)
  if (!parsed) return null
  return runHeadlessFile(parsed.file, {
    ...options,
    cwd: parsed.cwd ?? options.cwd,
    headlessTimeoutMs: parsed.timeout ? parseDurationMs(parsed.timeout) : options.headlessTimeoutMs,
  })
}

export async function runHeadlessFile(file: string, options: HeadlessRunOptions = {}): Promise<HeadlessRunReceipt> {
  const cwd = path.resolve(options.cwd ?? process.cwd())
  const filePath = path.resolve(cwd, file)
  const originalCwd = process.cwd()
  try {
    process.chdir(cwd)
    const text = await readFile(filePath, "utf8")
    const parsed = await Effect.runPromise(parseZyal(text, { source: filePath }))
    const receiptPath = path.resolve(cwd, options.receiptPath ?? defaultReceiptPath(parsed.spec.id))
    const started = new Date()

    const useDaemon = shouldUseDaemonPath(parsed.spec)
    options.print?.(
      `headless: mode ${useDaemon ? "daemon" : "shell_only"} reason=${
        useDaemon
          ? [parsed.spec.research !== undefined ? "research" : undefined, parsed.spec.experiments !== undefined ? "experiments" : undefined]
              .filter(Boolean)
              .join("+")
          : "no research or experiments"
      }`,
    )
    const receipt = useDaemon
      ? await runHeadlessWithInstance(cwd, async () =>
          runDaemonHeadless(parsed, text, {
            cwd,
            filePath,
            started,
            receiptPath,
            options,
          }),
        )
      : await runShellOnlyHeadless(parsed, {
          cwd,
          filePath,
          started,
          receiptPath,
          options,
        })

    options.print?.(`headless: receipt ${receiptPath}`)
    return receipt
  } finally {
    process.chdir(originalCwd)
  }
}

async function runHeadlessWithInstance<A>(cwd: string, fn: () => Promise<A>): Promise<A> {
  const { store, ctx } = await AppRuntime.runPromise(
    InstanceStore.Service.use((store) =>
      store.load({ directory: cwd }).pipe(
        Effect.map((ctx) => ({
          store,
          ctx,
        })),
      ),
    ),
  )
  try {
    return await Instance.restore(ctx, fn)
  } finally {
    await AppRuntime.runPromise(store.dispose(ctx))
  }
}

export function planHeadlessSteps(spec: ZyalScript): PlannedShellStep[] {
  const steps: PlannedShellStep[] = []
  for (const [index, hook] of spec.hooks?.on_start?.entries() ?? []) {
    steps.push({ label: `hooks.on_start[${index}]`, command: hook.run, timeout: hook.timeout })
  }
  for (const [index, check] of spec.tasks?.discover?.entries() ?? []) {
    steps.push({
      label: `tasks.discover[${index}]`,
      command: check.command,
      cwd: check.cwd,
      timeout: check.timeout,
    })
  }
  if (spec.fan_out !== undefined) {
    if ("shell" in spec.fan_out.split) {
      steps.push({ label: "fan_out.split.shell", command: spec.fan_out.split.shell })
    }
    if (spec.fan_out.reduce.command !== undefined) {
      steps.push({ label: "fan_out.reduce.command", command: spec.fan_out.reduce.command })
    }
  }
  for (const [index, check] of spec.checkpoint?.verify?.entries() ?? []) {
    steps.push({
      label: `checkpoint.verify[${index}]`,
      command: check.command,
      cwd: check.cwd,
      timeout: check.timeout,
    })
  }
  for (const [index, condition] of spec.stop.all.entries()) {
    if ("shell" in condition) {
      steps.push({
        label: `stop.all[${index}].shell`,
        command: condition.shell.command,
        cwd: condition.shell.cwd,
        timeout: condition.shell.timeout,
      })
    }
  }
  return steps
}

function shouldUseDaemonPath(spec: ZyalScript): boolean {
  return spec.research !== undefined || spec.experiments !== undefined
}

async function runShellOnlyHeadless(
  parsed: ZyalParsed,
  input: {
    cwd: string
    filePath: string
    started: Date
    receiptPath: string
    options: HeadlessRunOptions
  },
): Promise<HeadlessRunReceipt> {
  const steps = planHeadlessSteps(parsed.spec)
  if (steps.length === 0) {
    throw new Error(`ZYAL ${parsed.spec.id} has no headless shell steps`)
  }
  if (parsed.spec.fan_out?.worker !== undefined) {
    input.options.print?.("headless: shell-only mode; model/agent worker spawning is not implemented")
  }

  const results: HeadlessStepResult[] = []
  let failed = false
  let failureMessage = ""

  for (const step of steps) {
    input.options.print?.(`headless: ${step.label}`)
    const result = await runShellStep(step, {
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
    dev_only_fallback_present: steps.some((step) => step.command.includes("memory_benchmark_dev_qbank=1")),
    receipt_path: input.receiptPath,
    steps: results,
  }

  await writeReceipt(input.receiptPath, receipt)
  if (failed) throw new HeadlessRunError(`${failureMessage}; receipt: ${input.receiptPath}`, receipt)
  return receipt
}

async function runDaemonHeadless(
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
  if (typeof preview === "object" && preview !== null && "spec" in preview) {
    const session = await requestJson(target, "/session", {
      method: "POST",
      body: JSON.stringify({
        title: (preview as any)?.spec?.job?.name ?? "ZYAL daemon",
      }),
    })
    sessionID = typeof session === "object" && session !== null && typeof (session as any).id === "string" ? (session as any).id : ""
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
  const runID = typeof startedRun === "object" && startedRun !== null && typeof (startedRun as any).id === "string"
    ? (startedRun as any).id
    : ""
  if (!runID) {
    throw new Error("Failed to start daemon run")
  }
  const artifactRoot = path.join(input.cwd, ".jekko", "daemon", runID)
  input.options.print?.(`headless: daemon run ${runID} artifacts ${artifactRoot}`)

  const pollStarted = Date.now()
  const timeoutMs = input.options.headlessTimeoutMs
  const idleTimeoutMs = input.options.idleTimeoutMs ?? Math.min(timeoutMs ?? 15 * 60 * 1000, 15 * 60 * 1000)
  let finalRun: any = undefined
  let lastActivityAt = Date.now()
  let lastPollState = ""
  const streamed = { ledgerLines: 0, stateText: "" }
  while (true) {
    const poll = await requestJson(target, `/daemon/${runID}`)
    finalRun = poll
    const status = typeof poll === "object" && poll !== null ? String((poll as any).status ?? "running") : "running"
    const phase = typeof poll === "object" && poll !== null ? String((poll as any).phase ?? "running") : "running"
    const iteration = typeof poll === "object" && poll !== null ? Number((poll as any).iteration ?? 0) : 0
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
  const status = typeof finalRun === "object" && finalRun !== null ? String((finalRun as any).status ?? "failed") : "failed"
  const timeoutReason = typeof finalRun === "object" && finalRun !== null && (finalRun as any).timeout
    ? "headless_timeout"
    : typeof finalRun === "object" && finalRun !== null && (finalRun as any).idle_timeout
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
    dev_only_fallback_present: false,
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

async function fetchJnoccioMetrics(baseUrl?: string): Promise<HeadlessJnoccioMetrics | undefined> {
  if (!baseUrl) return undefined
  try {
    const response = await fetch(new URL("/v1/jnoccio/metrics", baseUrl))
    if (!response.ok) return undefined
    const text = await response.text()
    if (!text.trim()) return undefined
    return normalizeJnoccioMetrics(JSON.parse(text))
  } catch {
    return undefined
  }
}

function normalizeJnoccioMetrics(value: unknown): HeadlessJnoccioMetrics | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined
  const record = value as Record<string, any>
  const totals = isRecord(record.totals) ? record.totals : isRecord(record.metrics) ? record.metrics : {}
  const calls = tokenNumber(record.calls ?? totals.calls ?? record.total_calls ?? totals.total_calls)
  const promptTokens = tokenNumber(
    record.prompt_tokens ?? record.promptTokens ?? record.input_tokens ?? record.inputTokens ?? totals.prompt_tokens ?? totals.input_tokens,
  )
  const completionTokens = tokenNumber(
    record.completion_tokens ?? record.completionTokens ?? record.output_tokens ?? record.outputTokens ?? totals.completion_tokens ?? totals.output_tokens,
  )
  const totalTokens = tokenNumber(record.total_tokens ?? record.totalTokens ?? totals.total_tokens ?? totals.totalTokens) || promptTokens + completionTokens
  if (calls === 0 && promptTokens === 0 && completionTokens === 0 && totalTokens === 0) return undefined
  return {
    calls,
    prompt_tokens: promptTokens,
    completion_tokens: completionTokens,
    total_tokens: totalTokens,
  }
}

function tokenNumber(value: unknown): number {
  const next = Number(value)
  return Number.isFinite(next) ? next : 0
}

function isRecord(input: unknown): input is Record<string, unknown> {
  return typeof input === "object" && input !== null && !Array.isArray(input)
}

function readOption(args: string[], name: string): string | undefined {
  const idx = args.indexOf(name)
  if (idx === -1) return undefined
  const value = args[idx + 1]
  return value && !value.startsWith("-") ? value : undefined
}

async function requestJson(
  target: { baseUrl: string; fetch: typeof fetch },
  path: string,
  init: RequestInit = {},
): Promise<unknown> {
  const response = await target.fetch(new URL(path, target.baseUrl), {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init.headers ?? {}),
    },
  })
  const text = await response.text()
  if (!response.ok) throw new Error(text || `${response.status} ${response.statusText}`)
  return text ? JSON.parse(text) : undefined
}

async function writeReceipt(receiptPath: string, receipt: HeadlessRunReceipt) {
  await mkdir(path.dirname(receiptPath), { recursive: true })
  await writeFile(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`)
}

async function streamDaemonArtifacts(
  root: string,
  print: ((line: string) => void) | undefined,
  cursor: { ledgerLines: number; stateText: string },
): Promise<boolean> {
  let changed = false
  const ledgerPath = path.join(root, "ledger.jsonl")
  try {
    const text = await readFile(ledgerPath, "utf8")
    const lines = text.split("\n").filter((line) => line.trim().length > 0)
    if (lines.length > cursor.ledgerLines) {
      for (const line of lines.slice(cursor.ledgerLines)) {
        try {
          const event = JSON.parse(line) as Record<string, any>
          const kind = typeof event.event_type === "string" ? event.event_type : "event"
          const payload = event.payload_json && typeof event.payload_json === "object" ? summarizeEventPayload(event.payload_json) : ""
          print?.(`headless: ledger ${kind}${payload ? ` ${payload}` : ""}`)
        } catch {
          print?.(`headless: ledger ${line}`)
        }
      }
      cursor.ledgerLines = lines.length
      changed = true
    }
  } catch {
    // ignore until the daemon materializes the ledger
  }

  const statePath = path.join(root, "STATE.md")
  try {
    const stateText = await readFile(statePath, "utf8")
    if (stateText !== cursor.stateText) {
      cursor.stateText = stateText
      print?.(`headless: state\n${stateText.trimEnd()}`)
      changed = true
    }
  } catch {
    // ignore until the daemon writes the state snapshot
  }
  return changed
}

function summarizeEventPayload(payload: Record<string, any>): string {
  const entries = Object.entries(payload)
    .filter(([, value]) => typeof value === "string" || typeof value === "number" || typeof value === "boolean")
    .map(([key, value]) => `${key}=${value}`)
  return entries.slice(0, 6).join(" ")
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function formatMetrics(metrics: HeadlessJnoccioMetrics | undefined) {
  if (!metrics) return "n/a"
  return `${metrics.calls}/${metrics.prompt_tokens}/${metrics.completion_tokens}/${metrics.total_tokens}`
}

function formatMetricsDelta(
  before: HeadlessJnoccioMetrics | undefined,
  after: HeadlessJnoccioMetrics | undefined,
) {
  if (!before || !after) return "n/a"
  return `${after.calls - before.calls}/${after.prompt_tokens - before.prompt_tokens}/${after.completion_tokens - before.completion_tokens}/${after.total_tokens - before.total_tokens}`
}

async function runShellStep(
  step: PlannedShellStep,
  options: { cwd: string; env: NodeJS.ProcessEnv },
): Promise<HeadlessStepResult> {
  const started = Date.now()
  const cwd = path.resolve(options.cwd, step.cwd ?? ".")
  const timeoutMs = parseDurationMs(step.timeout)
  return new Promise((resolve) => {
    const child = spawn(step.command, {
      cwd,
      env: options.env,
      shell: true,
      stdio: ["ignore", "inherit", "inherit"],
    })
    let timer: NodeJS.Timeout | undefined
    if (timeoutMs !== undefined) {
      timer = setTimeout(() => {
        child.kill("SIGTERM")
      }, timeoutMs)
    }
    child.on("close", (exitCode, signal) => {
      if (timer !== undefined) clearTimeout(timer)
      resolve({
        label: step.label,
        command: step.command,
        cwd,
        timeout: step.timeout,
        status: exitCode === 0 ? "passed" : "failed",
        exitCode,
        signal,
        durationMs: Date.now() - started,
      })
    })
  })
}

function parseDurationMs(input: string | undefined): number | undefined {
  if (input === undefined) return undefined
  const match = input.trim().match(/^(\d+)(ms|s|m|h)?$/)
  if (!match) return undefined
  const value = Number(match[1])
  const unit = match[2] ?? "ms"
  if (unit === "ms") return value
  if (unit === "s") return value * 1000
  if (unit === "m") return value * 60 * 1000
  return value * 60 * 60 * 1000
}

function defaultReceiptPath(id: string): string {
  return path.join(".jekko", "daemon", id, "headless-receipt.json")
}
