import { Effect } from "effect"
import { spawn } from "node:child_process"
import { mkdir, readFile, writeFile } from "node:fs/promises"
import path from "node:path"
import { parseZyal } from "@/agent-script/parser"
import type { ZyalScript } from "@/agent-script/schema"

export type HeadlessStepStatus = "passed" | "failed"

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
  mode: "shell_only"
  worker_spec_present: boolean
  dev_only_fallback_present: boolean
  receipt_path: string
  steps: HeadlessStepResult[]
}

export interface HeadlessRunOptions {
  cwd?: string
  env?: NodeJS.ProcessEnv
  receiptPath?: string
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

export function parseHeadlessArgs(args: string[]): { file: string; cwd?: string } | null {
  const cwd = readOption(args, "--headless-cwd")
  const equals = args.find((arg) => arg.startsWith("--headless="))
  if (equals) {
    const file = equals.slice("--headless=".length)
    if (!file) throw new Error("--headless requires a ZYAL file path")
    return { file, cwd }
  }

  const index = args.indexOf("--headless")
  if (index === -1) return null
  const file = args[index + 1]
  if (!file || file.startsWith("-")) throw new Error("--headless requires a ZYAL file path")
  return { file, cwd }
}

export async function runHeadlessCli(args: string[], options: HeadlessRunOptions = {}): Promise<HeadlessRunReceipt | null> {
  const parsed = parseHeadlessArgs(args)
  if (!parsed) return null
  return runHeadlessFile(parsed.file, { ...options, cwd: parsed.cwd ?? options.cwd })
}

export async function runHeadlessFile(file: string, options: HeadlessRunOptions = {}): Promise<HeadlessRunReceipt> {
  const cwd = path.resolve(options.cwd ?? process.cwd())
  const filePath = path.resolve(cwd, file)
  const text = await readFile(filePath, "utf8")
  const parsed = await Effect.runPromise(parseZyal(text, { source: filePath }))
  const steps = planHeadlessSteps(parsed.spec)
  if (steps.length === 0) {
    throw new Error(`ZYAL ${parsed.spec.id} has no headless shell steps`)
  }
  if (parsed.spec.fan_out?.worker !== undefined) {
    options.print?.("headless: shell-only mode; model/agent worker spawning is not implemented")
  }

  const started = new Date()
  const results: HeadlessStepResult[] = []
  const receiptPath = path.resolve(cwd, options.receiptPath ?? defaultReceiptPath(parsed.spec.id))
  let failed = false
  let failureMessage = ""

  for (const step of steps) {
    options.print?.(`headless: ${step.label}`)
    const result = await runShellStep(step, {
      cwd,
      env: {
        ...process.env,
        ...options.env,
        JEKKO_HEADLESS: "1",
        ZYAL_HEADLESS: "1",
        ZYAL_RUN_ID: parsed.spec.id,
        ZYAL_FILE: filePath,
        ZYAL_WORKSPACE: cwd,
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
    file: filePath,
    spec_hash: parsed.specHash,
    status: failed ? "failed" : "passed",
    started_at: started.toISOString(),
    finished_at: finished.toISOString(),
    duration_ms: finished.getTime() - started.getTime(),
    headless: true,
    mode: "shell_only",
    worker_spec_present: parsed.spec.fan_out?.worker !== undefined,
    dev_only_fallback_present: steps.some((step) => step.command.includes("memory_benchmark_dev_qbank=1")),
    receipt_path: receiptPath,
    steps: results,
  }

  await mkdir(path.dirname(receiptPath), { recursive: true })
  await writeFile(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`)
  if (failed) throw new HeadlessRunError(`${failureMessage}; receipt: ${receiptPath}`, receipt)
  options.print?.(`headless: receipt ${receiptPath}`)
  return receipt
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

function readOption(args: string[], name: string): string | undefined {
  const equals = args.find((arg) => arg.startsWith(`${name}=`))
  if (equals) return equals.slice(name.length + 1) || undefined
  const index = args.indexOf(name)
  const value = index === -1 ? undefined : args[index + 1]
  return value && !value.startsWith("-") ? value : undefined
}
