import { afterEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import os from "os"
import path from "path"

const repoRoot = path.resolve(import.meta.dir, "../../..", "..")
const script = path.join(repoRoot, "tools/check-encrypted-paths.sh")
const tempDirs: string[] = []

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

async function tempGitRepo() {
  const dir = await fsp.mkdtemp(path.join(os.tmpdir(), "jnoccio-encryption-test-"))
  tempDirs.push(dir)
  await run("git", ["init"], dir)
  await fsp.mkdir(path.join(dir, "jnoccio-fusion", "src"), { recursive: true })
  await fsp.writeFile(
    path.join(dir, ".gitattributes"),
    [
      "jnoccio-fusion/** filter=git-crypt diff=git-crypt",
      "jnoccio-fusion/README.md !filter !diff",
      "jnoccio-fusion/.gitignore !filter !diff",
      "jnoccio-fusion/.env.jnoccio.example !filter !diff",
      "jnoccio-fusion/KEYS.md !filter !diff",
      "jnoccio-fusion/ENCRYPTION.md !filter !diff",
      "",
    ].join("\n"),
  )
  return dir
}

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
})

describe("check-encrypted-paths", () => {
  test("index mode passes for the repository while the working tree may be unlocked", async () => {
    const result = await run(script, ["--index"], repoRoot)
    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain("index mode")
  })

  test("index mode catches plaintext protected blobs", async () => {
    const dir = await tempGitRepo()
    await fsp.writeFile(path.join(dir, "jnoccio-fusion", "src", "plain.rs"), "plaintext")
    await run("git", ["add", ".gitattributes", "jnoccio-fusion/src/plain.rs"], dir)

    const result = await run(script, ["--index"], dir)
    expect(result.exitCode).toBe(1)
    expect(result.stderr).toContain("PLAINTEXT DETECTED")
    expect(result.stderr).toContain("jnoccio-fusion/src/plain.rs")
  })

  test("fails if .env.jnoccio is tracked", async () => {
    const dir = await tempGitRepo()
    await fsp.writeFile(path.join(dir, "jnoccio-fusion", ".env.jnoccio"), "OPENROUTER_API_KEY=secret")
    await run("git", ["add", ".gitattributes", "jnoccio-fusion/.env.jnoccio"], dir)

    const result = await run(script, ["--index"], dir)
    expect(result.exitCode).toBe(1)
    expect(result.stderr).toContain(".env.jnoccio is tracked")
  })
})
