import { spawn, type ChildProcess } from "child_process"
import fs from "fs/promises"
import os from "os"
import path from "path"
import { Hash } from "@jekko-ai/core/util/hash"

export type WorkerResult = {
  code: number
  stdout: Buffer
  stderr: Buffer
}

export function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, ms)
  })
}

export async function exists(file: string) {
  return fs
    .stat(file)
    .then(() => true)
    .catch(() => false)
}

export async function readJson<T>(file: string): Promise<T> {
  return JSON.parse(await fs.readFile(file, "utf8"))
}

export function lock(dir: string, key: string) {
  return path.join(dir, Hash.fast(key) + ".lock")
}

export async function tmpdir(prefix = "worker-test-") {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), prefix))
  return {
    path: dir,
    async [Symbol.asyncDispose]() {
      await fs.rm(dir, { recursive: true, force: true })
    },
  }
}

export function runWorker(worker: string, root: string, msg: unknown) {
  return new Promise<WorkerResult>((resolve) => {
    const proc = spawn(process.execPath, [worker, JSON.stringify(msg)], { cwd: root })
    const stdout: Buffer[] = []
    const stderr: Buffer[] = []
    proc.stdout?.on("data", (data) => stdout.push(Buffer.from(data)))
    proc.stderr?.on("data", (data) => stderr.push(Buffer.from(data)))
    proc.on("close", (code) => {
      resolve({ code: code ?? 1, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) })
    })
  })
}

export function spawnWorker(worker: string, root: string, msg: unknown): ChildProcess {
  return spawn(process.execPath, [worker, JSON.stringify(msg)], {
    cwd: root,
    stdio: ["ignore", "pipe", "pipe"],
  })
}

export function stopWorker(proc: ChildProcess) {
  if (proc.exitCode !== null || proc.signalCode !== null) return Promise.resolve()
  if (process.platform !== "win32" || !proc.pid) {
    proc.kill()
    return Promise.resolve()
  }
  return new Promise<void>((resolve) => {
    const killProc = spawn("taskkill", ["/pid", String(proc.pid), "/T", "/F"])
    killProc.on("close", () => {
      proc.kill()
      resolve()
    })
  })
}

export async function waitForFile(file: string, timeout = 3_000) {
  const stop = Date.now() + timeout
  while (Date.now() < stop) {
    if (await exists(file)) return
    await sleep(20)
  }
  throw new Error(`Timed out waiting for file: ${file}`)
}
