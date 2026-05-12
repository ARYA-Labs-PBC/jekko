import { type ChildProcess } from "child_process"
import launch from "cross-spawn"
import os from "node:os"
import { buffer } from "node:stream/consumers"
import { errorMessage } from "./error"

export type Stdio = "inherit" | "pipe" | "ignore"
export type Shell = boolean | string

export interface Options {
  cwd?: string
  env?: NodeJS.ProcessEnv | null
  stdin?: Stdio
  stdout?: Stdio
  stderr?: Stdio
  shell?: Shell
  abort?: AbortSignal
  kill?: NodeJS.Signals | number
  timeout?: number
}

export interface RunOptions extends Omit<Options, "stdout" | "stderr"> {
  nothrow?: boolean
  input?: string | Buffer | Uint8Array
}

export interface Result {
  code: number
  signal: NodeJS.Signals | null
  stdout: Buffer
  stderr: Buffer
}

export interface TextResult extends Result {
  text: string
}

export class RunFailedError extends Error {
  readonly cmd: string[]
  readonly code: number
  readonly signal: NodeJS.Signals | null
  readonly stdout: Buffer
  readonly stderr: Buffer

  constructor(cmd: string[], code: number, stdout: Buffer, stderr: Buffer, signal: NodeJS.Signals | null = null) {
    const text = stderr.toString().trim()
    const status = signal ? `signal ${signal} (code ${code})` : `code ${code}`
    super(
      text
        ? `Command failed with ${status}: ${cmd.join(" ")}\n${text}`
        : `Command failed with ${status}: ${cmd.join(" ")}`,
    )
    this.name = "ProcessRunFailedError"
    this.cmd = [...cmd]
    this.code = code
    this.signal = signal
    this.stdout = stdout
    this.stderr = stderr
  }
}

export interface ExitStatus {
  code: number
  signal: NodeJS.Signals | null
}

export type Child = ChildProcess & { exit: Promise<ExitStatus>; exited: Promise<number> }

function signalExitCode(signal: NodeJS.Signals | null) {
  if (!signal) return 0
  const number = os.constants.signals[signal]
  return typeof number === "number" ? 128 + number : 1
}

export function spawn(cmd: string[], opts: Options = {}): Child {
  if (cmd.length === 0) throw new Error("Command is required")
  opts.abort?.throwIfAborted()

  const proc = launch(cmd[0], cmd.slice(1), {
    cwd: opts.cwd,
    shell: opts.shell,
    env: opts.env === null ? {} : opts.env ? { ...process.env, ...opts.env } : undefined,
    stdio: [opts.stdin ?? "ignore", opts.stdout ?? "ignore", opts.stderr ?? "ignore"],
    windowsHide: process.platform === "win32",
  })

  let closed = false
  let timer: ReturnType<typeof setTimeout> | undefined

  const abort = () => {
    if (closed) return
    if (proc.exitCode !== null || proc.signalCode !== null) return
    closed = true

    proc.kill(opts.kill ?? "SIGTERM")

    const ms = opts.timeout ?? 5_000
    if (ms <= 0) return
    timer = setTimeout(() => proc.kill("SIGKILL"), ms)
  }

  const exit = new Promise<ExitStatus>((resolve, reject) => {
    const done = () => {
      opts.abort?.removeEventListener("abort", abort)
      if (timer) clearTimeout(timer)
    }

    proc.once("exit", (code, signal) => {
      done()
      resolve({
        code: code ?? signalExitCode(signal),
        signal,
      })
    })

    proc.once("error", (error) => {
      done()
      reject(error)
    })
  })
  const exited = exit.then((status) => status.code)
  void exited.catch(() => undefined)
  void exit.catch(() => undefined)

  if (opts.abort) {
    opts.abort.addEventListener("abort", abort, { once: true })
    if (opts.abort.aborted) abort()
  }

  const child = proc as Child
  child.exit = exit
  child.exited = exited
  return child
}

export async function run(cmd: string[], opts: RunOptions = {}): Promise<Result> {
  const proc = spawn(cmd, {
    cwd: opts.cwd,
    env: opts.env,
    shell: opts.shell,
    abort: opts.abort,
    kill: opts.kill,
    timeout: opts.timeout,
    stdin: opts.input === undefined ? opts.stdin : "pipe",
    stdout: "pipe",
    stderr: "pipe",
  })

  if (!proc.stdout || !proc.stderr) throw new Error("Process output not available")
  if (opts.input !== undefined) {
    if (!proc.stdin) throw new Error("Process input not available")
    proc.stdin.end(opts.input)
  }

  const out = await Promise.all([proc.exit, buffer(proc.stdout), buffer(proc.stderr)])
    .then(([status, stdout, stderr]) => ({
      code: status.code,
      signal: status.signal,
      stdout,
      stderr,
    }))
    .catch((err: unknown) => {
      if (!opts.nothrow) throw err
      return {
        code: 1,
        signal: null,
        stdout: Buffer.alloc(0),
        stderr: Buffer.from(errorMessage(err)),
      }
    })
  if (out.code === 0 || opts.nothrow) return out
  throw new RunFailedError(cmd, out.code, out.stdout, out.stderr, out.signal)
}

// Duplicated in `packages/sdk/js/src/process.ts` because the SDK cannot import
// `jekko` without creating a cycle. Keep both copies in sync.
export async function stop(proc: ChildProcess) {
  if (proc.exitCode !== null || proc.signalCode !== null) return

  if (process.platform !== "win32" || !proc.pid) {
    proc.kill()
    return
  }

  const out = await run(["taskkill", "/pid", String(proc.pid), "/T", "/F"], {
    nothrow: true,
  })

  if (out.code === 0) return
  proc.kill()
}

export async function text(cmd: string[], opts: RunOptions = {}): Promise<TextResult> {
  const out = await run(cmd, opts)
  return {
    ...out,
    text: out.stdout.toString(),
  }
}

export async function lines(cmd: string[], opts: RunOptions = {}): Promise<string[]> {
  return (await text(cmd, opts)).text.split(/\r?\n/).filter(Boolean)
}

export * as Process from "./process"
