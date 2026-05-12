import { afterEach, expect, test } from "bun:test"
import { existsSync } from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import { isJnoccioFusionUnlocked, jnoccioUnlockSecretPath, unlockJnoccioFusion } from "../../src/util/jnoccio-unlock"

const repoRoot = path.resolve(import.meta.dir, "../../..", "..")
const secretPath = jnoccioUnlockSecretPath()
const tempDirs: string[] = []

function hasGitCrypt() {
  return Bun.spawnSync(["git-crypt", "version"], { stdout: "ignore", stderr: "ignore" }).exitCode === 0
}

async function run(command: string, args: string[], cwd: string) {
  const proc = Bun.spawn([command, ...args], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  })
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ])
  return { stdout, stderr, exitCode }
}

const localTest = !process.env.CI && secretPath && existsSync(secretPath) && hasGitCrypt() ? test : test.skip

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
})

localTest("unlocks a fresh clone with the cached Jnoccio secret", async () => {
  const cloneParent = await fsp.mkdtemp(path.join(os.tmpdir(), "jnoccio-real-unlock-"))
  tempDirs.push(cloneParent)
  const clone = path.join(cloneParent, "repo")

  const cloned = await run("git", ["clone", "--quiet", repoRoot, clone], cloneParent)
  expect(cloned.exitCode).toBe(0)

  expect(isJnoccioFusionUnlocked(clone)).toBe(false)

  const result = await unlockJnoccioFusion({}, { repoRoot: clone, secretPath })
  expect(result.status).toBe("unlocked")
  expect(result.envCreated).toBe(true)
  expect(result.secretSaved).toBe(false)
  expect(isJnoccioFusionUnlocked(clone)).toBe(true)
  await expect(fsp.readFile(path.join(clone, "jnoccio-fusion", ".env.jnoccio"), "utf8")).resolves.toContain(
    "OPENROUTER_API_KEY=",
  )

  const metadata = await run(
    "cargo",
    ["metadata", "--manifest-path", path.join(clone, "jnoccio-fusion", "Cargo.toml"), "--no-deps"],
    clone,
  )
  expect(metadata.exitCode).toBe(0)

  const encrypted = await run(path.join(repoRoot, "tools/check-encrypted-paths.sh"), ["--index"], clone)
  expect(encrypted.exitCode).toBe(0)
})
