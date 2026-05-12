import { afterEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import os from "os"
import path from "path"

const tempDirs: string[] = []
const repoRoot = path.resolve(import.meta.dir, "../../../..")
const loopScript = path.join(repoRoot, "jekko-jankurai-loop.sh")

async function tempFixture() {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "jekko-loop-script-"))
  tempDirs.push(root)

  const promptFile = path.join(root, "prompt.md")
  const fakeLog = path.join(root, "fake-jekko.log")
  const stdinCapture = path.join(root, "stdin-capture.txt")
  const fakeBinary = path.join(root, "fake-jekko.sh")
  const runsDir = path.join(root, "runs")

  await fsp.mkdir(runsDir, { recursive: true })
  await fsp.writeFile(promptFile, "alpha\nbeta\n")
  await fsp.writeFile(
    fakeBinary,
    `#!/usr/bin/env bash
set -euo pipefail

log="\${FAKE_LOG:?}"
printf '%s\n' "$*" >> "$log"

if [[ "\${1:-}" == "providers" && "\${2:-}" == "unlock" ]]; then
  printf '%s\n' '{"status":"unlocked","message":"ok","envCreated":true,"secretSaved":false}'
  exit "\${FAKE_UNLOCK_EXIT:-0}"
fi

if [[ "\${1:-}" == "run" ]]; then
  final=""
  while (($#)); do
    case "$1" in
      --output-last-message)
        final="$2"
        shift 2
        ;;
      *)
        shift
        ;;
    esac
  done

  if [[ -n "\${FAKE_STDIN_CAPTURE:-}" ]]; then
    cat > "$FAKE_STDIN_CAPTURE"
  else
    cat >/dev/null
  fi

  if [[ -n "$final" ]]; then
    printf '%s\n' "\${FAKE_FINAL_MESSAGE:-fake final message}" > "$final"
  fi

  if [[ -n "\${FAKE_RUN_STDOUT:-}" ]]; then
    printf '%s\n' "$FAKE_RUN_STDOUT"
  fi

  exit "\${FAKE_RUN_EXIT:-0}"
fi

echo "unexpected invocation: $*" >&2
exit 99
`,
  )
  await fsp.chmod(fakeBinary, 0o755)

  return {
    root,
    promptFile,
    fakeLog,
    stdinCapture,
    fakeBinary,
    runsDir,
  }
}

async function runLoop(env: Record<string, string>) {
  const proc = Bun.spawn(["bash", loopScript], {
    cwd: repoRoot,
    env: {
      ...process.env,
      ...env,
      NO_COLOR: env.NO_COLOR ?? "",
    },
    stdout: "pipe",
    stderr: "pipe",
  })

  const [stdout, stderr, status] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ])

  return { stdout, stderr, status }
}

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
  await fsp.rm(path.join(repoRoot, ".jekko-loop-runs"), { recursive: true, force: true })
})

describe("jekko-jankurai-loop.sh", () => {
  test("unlocks Jnoccio before each run, keeps prompt stdin intact, and cycles colors", async () => {
    const fixture = await tempFixture()

    const result = await runLoop({
      PROMPT_FILE: fixture.promptFile,
      LOG_DIR: fixture.runsDir,
      JEKKO_BIN: fixture.fakeBinary,
      FAKE_LOG: fixture.fakeLog,
      FAKE_STDIN_CAPTURE: fixture.stdinCapture,
      FAKE_FINAL_MESSAGE: "assistant final",
      MAX_RUNS: "2",
      SLEEP_SECONDS: "0",
      LOOP_COLOR: "always",
      NO_COLOR: "",
      MODEL: "jnoccio/jnoccio-fusion",
      AUTO_UNLOCK: "1",
      REQUIRE_UNLOCK: "1",
      STOP_ON_FAILURE: "1",
    })

    expect(result.status).toBe(0)

    const runs = (await fsp.readdir(fixture.runsDir)).filter((entry) => entry.startsWith("run-"))
    expect(runs).toHaveLength(2)

    const logLines = (await fsp.readFile(fixture.fakeLog, "utf8")).trim().split("\n")
    expect(logLines.filter((line) => line.startsWith("providers unlock jnoccio"))).toHaveLength(2)
    expect(logLines.filter((line) => line.startsWith("run "))).toHaveLength(2)

    await expect(fsp.readFile(fixture.stdinCapture, "utf8")).resolves.toBe("alpha\nbeta\n")

    const colors = [...result.stdout.matchAll(/\x1b\[38;5;(\d+)mJekko run #([0-9]+)/g)].map((match) => ({
      run: Number(match[2]),
      color: match[1],
    }))
    expect(colors).toHaveLength(2)
    expect(colors[0].color).not.toBe(colors[1].color)

    const latestRun = path.join(fixture.runsDir, runs.sort().at(-1)!)
    await expect(fsp.readFile(path.join(latestRun, "prompt.md"), "utf8")).resolves.toBe("alpha\nbeta\n")
    await expect(fsp.readFile(path.join(latestRun, "final-message.md"), "utf8")).resolves.toBe("assistant final\n")
    await expect(fsp.readFile(path.join(latestRun, "command.txt"), "utf8")).resolves.toContain(
      "--output-last-message",
    )
    await expect(fsp.readFile(path.join(latestRun, "env.txt"), "utf8")).resolves.toContain(
      "JEKKO_AUTO_ALLOW_READS=1",
    )
    await expect(fsp.readFile(path.join(latestRun, "unlock-command.txt"), "utf8")).resolves.toContain(
      "providers unlock jnoccio",
    )
    const metadata = JSON.parse(await fsp.readFile(path.join(latestRun, "metadata.json"), "utf8")) as {
      unlock_ran: boolean
      unlock_status: number
      run_started: boolean
      run_status: number
    }
    expect(metadata.unlock_ran).toBe(true)
    expect(metadata.unlock_status).toBe(0)
    expect(metadata.run_started).toBe(true)
    expect(metadata.run_status).toBe(0)
  })

  test("skips unlock for non-jnoccio models and stops on failure", async () => {
    const fixture = await tempFixture()

    const result = await runLoop({
      PROMPT_FILE: fixture.promptFile,
      LOG_DIR: fixture.runsDir,
      JEKKO_BIN: fixture.fakeBinary,
      FAKE_LOG: fixture.fakeLog,
      FAKE_STDIN_CAPTURE: fixture.stdinCapture,
      FAKE_FINAL_MESSAGE: "assistant final",
      FAKE_RUN_EXIT: "7",
      MAX_RUNS: "2",
      SLEEP_SECONDS: "0",
      LOOP_COLOR: "always",
      NO_COLOR: "",
      MODEL: "openai/gpt-5",
      AUTO_UNLOCK: "1",
      REQUIRE_UNLOCK: "1",
      STOP_ON_FAILURE: "1",
    })

    expect(result.status).toBe(7)

    const runs = (await fsp.readdir(fixture.runsDir)).filter((entry) => entry.startsWith("run-"))
    expect(runs).toHaveLength(1)

    const logLines = (await fsp.readFile(fixture.fakeLog, "utf8")).trim().split("\n")
    expect(logLines.some((line) => line.startsWith("providers unlock"))).toBe(false)
    expect(logLines.filter((line) => line.startsWith("run "))).toHaveLength(1)
    expect(result.stdout).toContain("Model:  openai/gpt-5")
    await expect(fsp.readFile(fixture.stdinCapture, "utf8")).resolves.toBe("alpha\nbeta\n")
  })
})
