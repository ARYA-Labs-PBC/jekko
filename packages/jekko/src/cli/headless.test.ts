import { describe, expect, test } from "bun:test"
import { mkdtemp, readFile, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import path from "node:path"
import { parseHeadlessArgs, planHeadlessSteps, runHeadlessFile } from "./headless"
import { Effect } from "effect"
import { parseZyal } from "@/agent-script/parser"

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
