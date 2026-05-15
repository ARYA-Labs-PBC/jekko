import { afterEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import path from "path"
import { seedJnoccioFusionBundle } from "../../script/jnoccio-install-bundle.mjs"

const tempDirs: string[] = []
const seedRoot = path.join(import.meta.dir, "..", "..", "script", "seed", "jnoccio-fusion")

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
})

describe("jnoccio install bundle", () => {
  test("seeds a new bundle once and keeps later user edits intact", async () => {
    const root = await fsp.mkdtemp(path.join(import.meta.dir, "jnoccio-install-"))
    tempDirs.push(root)

    const first = seedJnoccioFusionBundle({ configRoot: root, seedRoot })
    await expect(fsp.readFile(first.serverPath, "utf8")).resolves.toContain('"routing"')
    await expect(fsp.readFile(first.modelsPath, "utf8")).resolves.toContain('"schema_version"')
    expect(first.bundleCreated).toBe(true)
    expect(first.serverCreated).toBe(true)
    expect(first.modelsCreated).toBe(true)

    const edited = '{\n  "edited": true\n}\n'
    await fsp.writeFile(first.serverPath, edited)

    const second = seedJnoccioFusionBundle({ configRoot: root, seedRoot })
    expect(second.bundleCreated).toBe(false)
    expect(second.serverCreated).toBe(false)
    expect(second.modelsCreated).toBe(false)
    await expect(fsp.readFile(second.serverPath, "utf8")).resolves.toBe(edited)
  })
})
