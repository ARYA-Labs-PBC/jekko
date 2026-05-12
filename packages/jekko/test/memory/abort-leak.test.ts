import { describe, test, expect } from "bun:test"
import path from "path"

const projectRoot = path.join(import.meta.dir, "../..")
const worker = path.join(import.meta.dir, "abort-leak-webfetch.ts")

const MB = 1024 * 1024
const ITERATIONS = 50

const getHeapMB = () => {
  Bun.gc(true)
  return process.memoryUsage().heapUsed / MB
}

describe("memory: abort controller leak", () => {
  test("webfetch does not leak memory over many invocations", async () => {
    // Measure the abort-timed fetch path in a fresh process so shared tool
    // runtime state does not dominate the heap signal.
    const proc = Bun.spawn({
      cmd: [process.execPath, worker],
      cwd: projectRoot,
      stdout: "pipe",
      stderr: "pipe",
      env: process.env,
    })

    const [code, stdout, stderr] = await Promise.all([
      proc.exited,
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ])

    if (code !== 0) {
      throw new Error(stderr.trim() || stdout.trim() || `worker exited with code ${code}`)
    }

    const result = JSON.parse(stdout.trim()) as {
      baseline: number
      after: number
      growth: number
    }

    console.log(`Baseline: ${result.baseline.toFixed(2)} MB`)
    console.log(`After ${ITERATIONS} fetches: ${result.after.toFixed(2)} MB`)
    console.log(`Growth: ${result.growth.toFixed(2)} MB`)

    // Memory growth should be minimal - less than 1MB per 10 requests.
    expect(result.growth).toBeLessThan(ITERATIONS / 10)
  }, 60000)
})
