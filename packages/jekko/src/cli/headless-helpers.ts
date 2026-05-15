import { spawn } from "node:child_process"
import { mkdir, readFile, writeFile } from "node:fs/promises"
import path from "node:path"
import type {
  HeadlessJnoccioMetrics,
  HeadlessRunReceipt,
  HeadlessStepResult,
  PlannedShellStep,
} from "./headless"

export async function fetchJnoccioMetrics(baseUrl?: string): Promise<HeadlessJnoccioMetrics | undefined> {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!baseUrl) return undefined
  try {
    const response = await fetch(new URL("/v1/jnoccio/metrics", baseUrl))
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (!response.ok) return undefined
    const text = await response.text()
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    if (!text.trim()) return undefined
    return normalizeJnoccioMetrics(JSON.parse(text))
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    return undefined
  }
}

function normalizeJnoccioMetrics(value: unknown): HeadlessJnoccioMetrics | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!isRecord(value)) return undefined
  const record = value
  const totals = isRecord(record.totals) ? record.totals : isRecord(record.metrics) ? record.metrics : undefined
  const calls = tokenNumber(record.calls ?? totals?.calls ?? record.total_calls ?? totals?.total_calls)
  const promptTokens = tokenNumber(
    record.prompt_tokens ?? record.promptTokens ?? record.input_tokens ?? record.inputTokens ?? totals?.prompt_tokens ?? totals?.input_tokens,
  )
  const completionTokens = tokenNumber(
    record.completion_tokens ?? record.completionTokens ?? record.output_tokens ?? record.outputTokens ?? totals?.completion_tokens ?? totals?.output_tokens,
  )
  const totalTokens =
    tokenNumber(record.total_tokens ?? record.totalTokens ?? totals?.total_tokens ?? totals?.totalTokens) || promptTokens + completionTokens
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (calls === 0 && promptTokens === 0 && completionTokens === 0 && totalTokens === 0) return undefined
  return {
    calls,
    prompt_tokens: promptTokens,
    completion_tokens: completionTokens,
    total_tokens: totalTokens,
  }
}

export function tokenNumber(value: unknown): number {
  const next = Number(value)
  return Number.isFinite(next) ? next : 0
}

export function isRecord(input: unknown): input is Record<string, unknown> {
  return typeof input === "object" && input !== null && !Array.isArray(input)
}

export function readRecordField(record: Record<string, unknown> | undefined, key: string): Record<string, unknown> | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!record) return undefined
  const value = record[key]
  return isRecord(value) ? value : undefined
}

export function readStringField(record: Record<string, unknown> | undefined, key: string): string | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!record) return undefined
  const value = record[key]
  return typeof value === "string" && value.length > 0 ? value : undefined
}

export function readNumberField(record: Record<string, unknown> | undefined, key: string): number | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!record) return undefined
  const value = record[key]
  return typeof value === "number" && Number.isFinite(value) ? value : undefined
}

export function readOption(args: string[], name: string): string | undefined {
  const idx = args.indexOf(name)
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (idx === -1) return undefined
  const value = args[idx + 1]
  return value && !value.startsWith("-") ? value : undefined
}

export async function requestJson(
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
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!text.trim()) return undefined
  const parsed = parseJsonPayload(text)
  if (parsed === undefined) throw new Error(`headless: invalid JSON response from ${path}`)
  return parsed
}

export async function writeReceipt(receiptPath: string, receipt: HeadlessRunReceipt) {
  await mkdir(path.dirname(receiptPath), { recursive: true })
  await writeFile(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`)
}

export async function streamDaemonArtifacts(
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
          const event = parseJsonPayload(line)
          if (!isRecord(event)) continue
          const kind = typeof event.event_type === "string" ? event.event_type : "event"
          const payload = isRecord(event.payload_json) ? summarizeEventPayload(event.payload_json) : ""
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

export function summarizeEventPayload(payload: Record<string, unknown>): string {
  const entries = Object.entries(payload)
    .filter(([, value]) => typeof value === "string" || typeof value === "number" || typeof value === "boolean")
    .map(([key, value]) => `${key}=${value}`)
  return entries.slice(0, 6).join(" ")
}

export function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

export function parseJsonPayload(text: string): unknown | undefined {
  try {
    return JSON.parse(text)
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
    return undefined
  }
}

export function formatMetrics(metrics: HeadlessJnoccioMetrics | undefined) {
  if (!metrics) return "n/a"
  return `${metrics.calls}/${metrics.prompt_tokens}/${metrics.completion_tokens}/${metrics.total_tokens}`
}

export function formatMetricsDelta(
  before: HeadlessJnoccioMetrics | undefined,
  after: HeadlessJnoccioMetrics | undefined,
) {
  if (!before || !after) return "n/a"
  return `${after.calls - before.calls}/${after.prompt_tokens - before.prompt_tokens}/${after.completion_tokens - before.completion_tokens}/${after.total_tokens - before.total_tokens}`
}

export async function runShellStep(
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

export function parseDurationMs(input: string | undefined): number | undefined {
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (input === undefined) return undefined
  const match = input.trim().match(/^(\d+)(ms|s|m|h)?$/)
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!match) return undefined
  const value = Number(match[1])
  const unit = match[2] ?? "ms"
  if (unit === "ms") return value
  if (unit === "s") return value * 1000
  if (unit === "m") return value * 60 * 1000
  return value * 60 * 60 * 1000
}

export function defaultReceiptPath(id: string): string {
  return path.join(".jekko", "daemon", id, "headless-receipt.json")
}
