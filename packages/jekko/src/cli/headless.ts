import { Effect } from "effect"
import { readFile } from "node:fs/promises"
import path from "node:path"
import { parseZyal } from "@/agent-script/parser"
import type { ZyalParsed, ZyalScript } from "@/agent-script/schema"
import { AppRuntime } from "@/effect/app-runtime"
import { Instance } from "@/project/instance"
import { InstanceStore } from "@/project/instance-store"
import { defaultReceiptPath, parseDurationMs, readOption } from "./headless-helpers"
import { runDaemonHeadless, runShellOnlyHeadless } from "./headless-runtime"

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
  attempts?: HeadlessStepResult[]
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
  dev_only_smoke_present: boolean
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

export interface PlannedShellStep {
  label: string
  command: string
  cwd?: string
  timeout?: string
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
  if (index === -1) {
    // jankurai:allow HLT-001-DEAD-MARKER reason=cli-arg-parser-no-headless-mode expires=2027-01-01
    return null
  }
  const file = args[index + 1]
  if (!file || file.startsWith("-")) throw new Error("--headless requires a ZYAL file path")
  return { file, cwd, ...(timeout ? { timeout } : {}) }
}

export async function runHeadlessCli(args: string[], options: HeadlessRunOptions = {}): Promise<HeadlessRunReceipt | null> {
  const parsed = parseHeadlessArgs(args)
  if (!parsed) {
    // jankurai:allow HLT-001-DEAD-MARKER reason=cli-arg-parser-no-headless-mode expires=2027-01-01
    return null
  }
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
      : await runShellOnlyHeadless(
          parsed,
          {
            cwd,
            filePath,
            started,
            receiptPath,
            options,
          },
          planHeadlessSteps,
        )

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
