import { describe, expect, spyOn, test } from "bun:test"
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import path from "node:path"
import * as Server from "@/server/server"
import { parseHeadlessArgs, planHeadlessSteps, runHeadlessFile } from "../../src/cli/headless"
import { Effect } from "effect"
import { parseZyal } from "@/agent-script/parser"
import { tmpdir as fixtureTmpdir } from "../../test/fixture/fixture"

describe("headless ZYAL CLI", () => {
  test("parses --headless file forms", () => {
    expect(parseHeadlessArgs(["--headless", "docs/run.zyal"])).toEqual({ file: "docs/run.zyal" })
    expect(parseHeadlessArgs(["--headless=docs/run.zyal"])).toEqual({ file: "docs/run.zyal" })
    expect(parseHeadlessArgs(["--headless", "docs/run.zyal", "--headless-cwd", "../.."])).toEqual({
      file: "docs/run.zyal",
      cwd: "../..",
    })
    expect(parseHeadlessArgs(["run", "--help"])).toBeNull()
  })

  test("plans shell-only daemon steps in execution order", async () => {
    const parsed = await Effect.runPromise(parseZyal(makeZyal()))
    expect(planHeadlessSteps(parsed.spec).map((step) => step.label)).toEqual([
      "fan_out.split.shell",
      "fan_out.reduce.command",
      "checkpoint.verify[0]",
      "stop.all[0].shell",
    ])
  })

  test("runs a shell-only ZYAL file to completion and writes a receipt", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "jekko-headless-"))
    const file = path.join(dir, "headless.zyal")
    await writeFile(file, makeZyal())

    const receipt = await runHeadlessFile(file, { cwd: dir })

    expect(receipt.status).toBe("passed")
    expect(receipt.id).toBe("headless-test")
    expect(receipt.mode).toBe("shell_only")
    expect(receipt.worker_spec_present).toBe(true)
    expect(receipt.steps.map((step) => step.status)).toEqual(["passed", "passed", "passed", "passed"])
    const reduced = await readFile(path.join(dir, "out", "reduced.txt"), "utf8")
    expect(reduced).toBe("reduce")
    const receiptText = await readFile(path.join(dir, ".jekko", "daemon", "headless-test", "headless-receipt.json"), "utf8")
    expect(JSON.parse(receiptText).headless).toBe(true)
  })

  test("runs a daemon ZYAL file through the daemon path and records Jnoccio metrics", async () => {
    const tmp = await fixtureTmpdir({ git: true })
    const dir = tmp.path
    const file = path.join(dir, "daemon.zyal")
    const metricsState = {
      calls: 17229,
      prompt_tokens: 617160147,
      completion_tokens: 7630299,
      total_tokens: 625267206,
    }
    const metricsServer = Bun.serve({
      hostname: "127.0.0.1",
      port: 0,
      fetch(request) {
        const url = new URL(request.url)
        if (url.pathname === "/v1/jnoccio/metrics") {
          const payload = {
            totals: { ...metricsState },
          }
          metricsState.calls += 1
          metricsState.prompt_tokens += 11
          metricsState.completion_tokens += 17
          metricsState.total_tokens += 28
          return jsonResponse(payload)
        }
        return new Response("not found", { status: 404 })
      },
    })
    try {
      await writeFile(file, makeDaemonZyal(metricsServer.url.origin))

      const runID = "daemon-smoke-run"
      const sessionID = "daemon-smoke-session"
      const artifactRoot = path.join(dir, ".jekko", "daemon", runID)
      let polls = 0
      let eventPolls = 0
      const statusSequence = ["running", "running", "running", "satisfied"]
      const events = [
        {
          event_type: "run.created",
          payload_json: {},
        },
        {
          event_type: "run.previewed",
          payload_json: { preview: { spec: { job: { name: "Headless daemon smoke" } } } },
        },
        {
          event_type: "jankurai.seeded_artifacts.reused",
          payload_json: {
            reportPath: "target/jankurai/repo-score.json",
            repairPlanPath: "target/jankurai/repair-plan.json",
          },
        },
        {
          event_type: "jankurai.worker_wave.started",
          payload_json: { workers: 1 },
        },
        {
          event_type: "jankurai.worker_wave.completed",
          payload_json: {
            workers: 1,
            started: 0,
            verified: 0,
            blocked: 0,
            reason: "no conflict-free task",
          },
        },
        {
          event_type: "jankurai.sleeping",
          payload_json: { sleep: "5 seconds" },
        },
      ]
      const defaultSpy = spyOn(Server, "Default").mockReturnValue({
        app: {
          fetch: async (input: Request) => {
            const url = new URL(input.url)
            if (url.pathname === "/daemon/preview") {
              return jsonResponse({ spec: { job: { name: "Headless daemon smoke" } } })
            }
            if (url.pathname === "/session") {
              return jsonResponse({ id: sessionID })
            }
            if (url.pathname === `/session/${sessionID}/daemon/start`) {
              await mkdir(path.join(artifactRoot, "reports", "lanes", "lane-one"), { recursive: true })
              await writeFile(
                path.join(artifactRoot, "ledger.jsonl"),
                `${JSON.stringify({
                  event_type: "autoresearch.started",
                  payload_json: { lane_count: 1, max_parallel: 1 },
                })}\n`,
              )
              await writeFile(path.join(artifactRoot, "STATE.md"), "# running\n")
              await writeFile(
                path.join(artifactRoot, "reports", "lanes", "lane-one", "report.json"),
                `${JSON.stringify({ lane_id: "lane-one", total_tokens: 46 }, null, 2)}\n`,
              )
              return jsonResponse({ id: runID })
            }
            if (url.pathname === `/daemon/${runID}`) {
              polls += 1
              const status = statusSequence[Math.min(polls - 1, statusSequence.length - 1)]
              return jsonResponse({
                id: runID,
                status,
                phase: status === "satisfied" ? "terminal" : "running",
                iteration: polls - 1,
                epoch: 0,
                last_error: null,
              })
            }
            if (url.pathname === `/daemon/${runID}/events`) {
              eventPolls += 1
              return jsonResponse(events.slice(0, Math.min(events.length, eventPolls + 1)))
            }
            throw new Error(`Unexpected request path: ${url.pathname}`)
          },
        },
      } as unknown as ReturnType<typeof Server.Default>)

      const writes: string[] = []
      const write = spyOn(process.stdout, "write").mockImplementation((chunk: string | Uint8Array) => {
        writes.push(String(chunk))
        return true
      })

      try {
        const receipt = await runHeadlessFile(file, {
          cwd: dir,
          print: (line) => {
            writes.push(`${line}\n`)
          },
        })

        expect(receipt.mode).toBe("daemon")
        expect(receipt.status).toBe("passed")
        expect(receipt.daemon_run_id).toBe(runID)
        expect(receipt.daemon_status).toBe("satisfied")
        expect(receipt.worker_spec_present).toBe(true)
        expect(receipt.dev_only_smoke_present).toBe(false)
        expect(receipt.jnoccio_metrics_before?.total_tokens).toBe(625267206)
        expect(receipt.jnoccio_metrics_after?.total_tokens).toBe(625267234)
        expect((receipt.jnoccio_metrics_after?.total_tokens ?? 0) > (receipt.jnoccio_metrics_before?.total_tokens ?? 0)).toBe(true)
        expect(await readFile(path.join(artifactRoot, "reports", "lanes", "lane-one", "report.json"), "utf8")).toContain("lane-one")
        expect(writes.join("")).toContain("headless:")
        expect(writes.join("")).toContain("seeded_artifacts.reused")
        expect(writes.join("")).toContain("worker_wave.completed")
      } finally {
        write.mockRestore()
        defaultSpy.mockRestore()
      }
    } finally {
      metricsServer.stop()
    }
  })

  test("treats malformed Jnoccio metrics payloads as absent", async () => {
    const tmp = await fixtureTmpdir({ git: true })
    const dir = tmp.path
    const file = path.join(dir, "daemon.zyal")
    const metricsServer = Bun.serve({
      hostname: "127.0.0.1",
      port: 0,
      fetch(request) {
        const url = new URL(request.url)
        if (url.pathname === "/v1/jnoccio/metrics") {
          return jsonResponse({
            totals: {
              calls: "bad",
              prompt_tokens: "bad",
              completion_tokens: "bad",
              total_tokens: "bad",
            },
          })
        }
        return new Response("not found", { status: 404 })
      },
    })
    try {
      await writeFile(file, makeDaemonZyal(metricsServer.url.origin))

      const runID = "daemon-smoke-run"
      const sessionID = "daemon-smoke-session"
      const artifactRoot = path.join(dir, ".jekko", "daemon", runID)
      let polls = 0
      let eventPolls = 0
      const events = [
        {
          event_type: "run.created",
          payload_json: {},
        },
        {
          event_type: "run.previewed",
          payload_json: { preview: { spec: { job: { name: "Headless daemon smoke" } } } },
        },
        {
          event_type: "jankurai.seeded_artifacts.invalid",
          payload_json: {
            reportPath: "target/jankurai/repo-score.json",
            repairPlanPath: "target/jankurai/repair-plan.json",
            reason: "audit JSON parse failed",
          },
        },
        {
          event_type: "jankurai.audit.started",
          payload_json: { command: "jankurai audit ..." },
        },
        {
          event_type: "jankurai.repair_plan.completed",
          payload_json: { packet_count: 0 },
        },
        {
          event_type: "jankurai.worker_wave.completed",
          payload_json: {
            workers: 1,
            started: 0,
            verified: 0,
            blocked: 0,
            reason: "no conflict-free task",
          },
        },
        {
          event_type: "jankurai.sleeping",
          payload_json: { sleep: "5 seconds" },
        },
      ]
      const defaultSpy = spyOn(Server, "Default").mockReturnValue({
        app: {
          fetch: async (input: Request) => {
            const url = new URL(input.url)
            if (url.pathname === "/daemon/preview") {
              return jsonResponse({ spec: { job: { name: "Headless daemon smoke" } } })
            }
            if (url.pathname === "/session") {
              return jsonResponse({ id: sessionID })
            }
            if (url.pathname === `/session/${sessionID}/daemon/start`) {
              await mkdir(path.join(artifactRoot, "reports", "lanes", "lane-one"), { recursive: true })
              await writeFile(
                path.join(artifactRoot, "ledger.jsonl"),
                `${JSON.stringify({
                  event_type: "autoresearch.started",
                  payload_json: { lane_count: 1, max_parallel: 1 },
                })}\n`,
              )
              await writeFile(path.join(artifactRoot, "STATE.md"), "# running\n")
              await writeFile(
                path.join(artifactRoot, "reports", "lanes", "lane-one", "report.json"),
                `${JSON.stringify({ lane_id: "lane-one", total_tokens: 46 }, null, 2)}\n`,
              )
              return jsonResponse({ id: runID })
            }
            if (url.pathname === `/daemon/${runID}`) {
              polls += 1
              return jsonResponse({
                id: runID,
                status: polls === 1 ? "running" : "satisfied",
                phase: polls === 1 ? "running" : "terminal",
                iteration: polls - 1,
                epoch: 0,
                last_error: null,
              })
            }
            if (url.pathname === `/daemon/${runID}/events`) {
              eventPolls += 1
              return jsonResponse(events.slice(0, Math.min(events.length, eventPolls + 1)))
            }
            throw new Error(`Unexpected request path: ${url.pathname}`)
          },
        },
      } as unknown as ReturnType<typeof Server.Default>)

      try {
        const receipt = await runHeadlessFile(file, {
          cwd: dir,
          print: () => {},
        })

        expect(receipt.mode).toBe("daemon")
        expect(receipt.status).toBe("passed")
        expect(receipt.jnoccio_metrics_before).toBeUndefined()
        expect(receipt.jnoccio_metrics_after).toBeUndefined()
      } finally {
        defaultSpy.mockRestore()
      }
    } finally {
      metricsServer.stop()
    }
  })
})

function makeZyal(): string {
  return `<<<ZYAL v1:daemon id=headless-test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: "Headless test"
  objective: "Run shell steps"
loop:
  policy: once
stop:
  all:
    - shell:
        command: "test -f out/reduced.txt"
        timeout: 10s
        assert: { exit_code: 0 }
fan_out:
  strategy: scatter_gather
  split:
    shell: "mkdir -p out && printf split > out/split.txt"
  worker:
    agent: build
    isolation: same_session
    max_parallel: 1
  reduce:
    strategy: custom_shell
    command: "test -f out/split.txt && printf reduce > out/reduced.txt"
checkpoint:
  verify:
    - command: "test -f out/reduced.txt"
      timeout: 10s
      assert: { exit_code: 0 }
<<<END_ZYAL id=headless-test>>>
ZYAL_ARM RUN_FOREVER id=headless-test`
}

function makeDaemonZyal(baseURL: string): string {
  return `<<<ZYAL v1:daemon id=headless-smoke-daemon>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: "Headless daemon smoke"
  objective: "Exercise daemon mode and Jnoccio metrics."
loop:
  policy: once
stop:
  all:
    - shell:
        command: "test -f .jekko/daemon/daemon-smoke-run/reports/lanes/lane-one/report.json"
        timeout: 30s
        assert: { exit_code: 0 }
fleet:
  max_workers: 1
  isolation: git_worktree
  jnoccio:
    enabled: true
    base_url: "${baseURL}"
    metrics_ws: "/v1/jnoccio/metrics/ws"
    spawn_on_demand: false
    register_workers: false
    heartbeat_path: "/v1/jnoccio/agents/heartbeat"
    heartbeat_interval: 15s
    max_instances: 1
experiments:
  strategy: disjoint_tournament
  lanes:
    - id: lane-one
      hypothesis: "Jnoccio smoke lane."
      prompt_strategy: default
      agent: build
      model: builder
      isolation: git_worktree
      timeout: 5m
  max_parallel: 1
  scoring:
    primary: executable_rust_oracles
    goal_direction: maximize
fan_out:
  strategy: scatter_gather
  split:
    shell: "true"
  worker:
    agent: build
    isolation: git_worktree
    max_parallel: 1
  reduce:
    strategy: custom_shell
    command: "true"
models:
  profiles:
    builder:
      provider: jnoccio
      model: jnoccio-fusion
    critic:
      provider: jnoccio
      model: jnoccio-fusion
  routes:
    build: builder
    judge: critic
<<<END_ZYAL id=headless-smoke-daemon>>>
ZYAL_ARM RUN_FOREVER id=headless-smoke-daemon`
}

function jsonResponse(value: unknown, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  })
}
