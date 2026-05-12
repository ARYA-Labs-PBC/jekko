import { afterEach, describe, expect, test } from "bun:test"
import { existsSync } from "fs"
import fsp from "fs/promises"
import path from "path"
import { isJnoccioFusionUnlocked } from "../../src/util/jnoccio-unlock"
import type { Provider } from "../../src/provider/provider"
import { cloneRepo, localUnlockPreflight, removeTempDirs, withEnv } from "./jnoccio-local-helpers"
import { Process } from "../../src/util/process"

const tempDirs: string[] = []
const installedPath = process.env.JNOCCIO_INSTALLED_JEKKO ?? "/opt/homebrew/bin/jekko"
const preflight = await localUnlockPreflight()
const installedEnabled = process.env.JNOCCIO_INSTALLED_UNLOCK_E2E === "1"
const installedTest = installedEnabled && preflight.ok && existsSync(installedPath) ? test : test.skip

async function waitForListening(proc: Bun.Subprocess<"pipe", "pipe", "pipe">) {
  const stdout = proc.stdout.getReader()
  const stderr = proc.stderr.getReader()
  const decoder = new TextDecoder()
  let stdoutBuffer = ""
  let stderrBuffer = ""
  const deadline = Date.now() + 15000
  type StreamRead = Awaited<ReturnType<typeof stdout.read>>
  let stdoutNext: Promise<StreamRead> | undefined = stdout.read()
  let stderrNext: Promise<StreamRead> | undefined = stderr.read()

  while (Date.now() < deadline) {
    const pending: Promise<{ stream: "stdout" | "stderr"; value: StreamRead } | undefined>[] = [
      Bun.sleep(100).then(() => undefined),
    ]
    if (stdoutNext) pending.push(stdoutNext.then((value) => ({ stream: "stdout" as const, value })))
    if (stderrNext) pending.push(stderrNext.then((value) => ({ stream: "stderr" as const, value })))
    const result = await Promise.race(pending)
    if (!result) {
      if (proc.exitCode !== null) break
      continue
    }
    if (result.value.value) {
      const text = decoder.decode(result.value.value, { stream: true })
      if (result.stream === "stdout") {
        stdoutBuffer += text
      } else {
        stderrBuffer += text
      }
      const match = `${stdoutBuffer}\n${stderrBuffer}`.match(/jekko server listening on (http:\/\/[^\s]+)/)
      if (match) return match[1]
    }
    if (result.stream === "stdout") stdoutNext = result.value.done ? undefined : stdout.read()
    if (result.stream === "stderr") stderrNext = result.value.done ? undefined : stderr.read()
    if (proc.exitCode !== null) break
  }

  throw new Error(
    `installed jekko did not report a listening URL; exit=${proc.exitCode}; stdout=${stdoutBuffer}; stderr=${stderrBuffer}`,
  )
}

async function postJson(url: string, route: string, directory: string, body?: unknown) {
  return await fetch(new URL(route, url), {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-jekko-directory": directory,
    },
    body: JSON.stringify(body ?? {}),
  })
}

async function getJson(url: string, route: string, directory: string) {
  return await fetch(new URL(route, url), {
    headers: {
      "x-jekko-directory": directory,
    },
  })
}

function installedServerEnv(input: { clone: string; root: string; secretCachePath: string }) {
  const env: Record<string, string> = {}
  for (const [key, value] of Object.entries(process.env)) {
    if (value !== undefined) env[key] = value
  }
  delete env.JEKKO_DB
  delete env.JEKKO_SKIP_MIGRATIONS
  env.XDG_DATA_HOME = path.join(input.root, "xdg", "data")
  env.XDG_CACHE_HOME = path.join(input.root, "xdg", "cache")
  env.XDG_CONFIG_HOME = path.join(input.root, "xdg", "config")
  env.XDG_STATE_HOME = path.join(input.root, "xdg", "state")
  env.JNOCCIO_REPO_ROOT = input.clone
  env.JNOCCIO_UNLOCK_SECRET_PATH = input.secretCachePath
  return env
}

async function assertInstalledStarts() {
  const out = await Process.run([installedPath, "--version"], { nothrow: true })
  if (out.code === 0) return
  throw new Error(
    [
      "installed jekko startup smoke failed",
      `path=${installedPath}`,
      `exit=${out.code}`,
      `signal=${out.signal ?? "none"}`,
      `stdout=${out.stdout.toString().trim()}`,
      `stderr=${out.stderr.toString().trim()}`,
    ].join("; "),
  )
}

afterEach(async () => {
  await removeTempDirs(tempDirs)
})

describe("installed Jnoccio unlock smoke", () => {
  installedTest(
    "unlocks a fresh clone through the installed jekko server",
    async () => {
      if (!installedEnabled) throw new Error("installed unlock proof skipped: JNOCCIO_INSTALLED_UNLOCK_E2E is not 1")
      if (!preflight.ok) throw new Error(`installed unlock proof skipped: ${preflight.reason}`)
      if (!existsSync(installedPath)) throw new Error(`installed unlock proof skipped: ${installedPath} is missing`)
      await assertInstalledStarts()

      const { clone, cloneParent } = await cloneRepo("jnoccio-installed-unlock-", tempDirs)
      const secretCachePath = path.join(cloneParent, "installed-jekko.unlock")
      expect(isJnoccioFusionUnlocked(clone)).toBe(false)

      let proc: Bun.Subprocess<"pipe", "pipe", "pipe"> | undefined
      try {
        await withEnv(
          {
            JNOCCIO_REPO_ROOT: clone,
            JNOCCIO_UNLOCK_SECRET_PATH: secretCachePath,
          },
          async () => {
            proc = Bun.spawn([installedPath, "serve", "--hostname", "127.0.0.1", "--port", "0", "--pure"], {
              cwd: clone,
              env: installedServerEnv({ clone, root: cloneParent, secretCachePath }),
              stdout: "pipe",
              stderr: "pipe",
            })

            const url = await waitForListening(proc)
            const unlock = await postJson(url, "/provider/jnoccio/unlock", clone, { unlockSecret: preflight.secret })
            expect(unlock.status).toBe(200)
            expect(await unlock.json()).toMatchObject({
              status: "unlocked",
              envCreated: true,
              secretSaved: true,
            })

            await expect(fsp.readFile(secretCachePath, "utf8")).resolves.toBe(preflight.secret)
            expect(isJnoccioFusionUnlocked(clone)).toBe(true)

            const providers = await getJson(url, "/provider", clone)
            expect(providers.status).toBe(200)
            const body = (await providers.json()) as Provider.ListResult
            expect(body.connected).toContain("jnoccio")
            expect(body.default.jnoccio).toBe("jnoccio-fusion")
          },
        )
      } finally {
        proc?.kill()
        await proc?.exited.catch(() => undefined)
      }
    },
    120000,
  )
})
