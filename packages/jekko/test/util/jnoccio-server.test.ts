import { afterEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import path from "path"
import { Global } from "@jekko-ai/core/global"
import { resolveJnoccioFusionConfigPath } from "../../src/util/jnoccio-server"

const tempDirs: string[] = []

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
})

describe("jnoccio server config path", () => {
  test("prefers the seeded user-local bundle over the repo checkout", async () => {
    const repoRoot = await fsp.mkdtemp(path.join(import.meta.dir, "jnoccio-server-"))
    tempDirs.push(repoRoot)
    const repoConfig = path.join(repoRoot, "jnoccio-fusion", "config")
    await fsp.mkdir(repoConfig, { recursive: true })
    await fsp.writeFile(path.join(repoConfig, "server.json"), JSON.stringify({ source: "repo" }))

    const bundleDir = path.join(Global.Path.config, "jnoccio-fusion")
    await fsp.mkdir(bundleDir, { recursive: true })
    tempDirs.push(bundleDir)
    const bundleConfig = path.join(bundleDir, "server.jsonc")
    await fsp.writeFile(bundleConfig, '{\n  "source": "bundle"\n}\n')

    expect(resolveJnoccioFusionConfigPath(repoRoot)).toBe(bundleConfig)
  })
})
