import { afterEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import { Provider } from "../../src/provider/provider"

const tempDirs: string[] = []

async function tempRepo(options: { configured?: boolean } = {}) {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "jnoccio-provider-test-"))
  tempDirs.push(root)
  const fusion = path.join(root, "jnoccio-fusion")
  await fsp.mkdir(path.join(fusion, "config"), { recursive: true })
  await fsp.writeFile(path.join(fusion, "Cargo.toml"), '[package]\nname = "jnoccio-fusion"\n')
  await fsp.writeFile(
    path.join(fusion, "config/server.json"),
    JSON.stringify({ provider: "jnoccio", model: "jnoccio/jnoccio-fusion" }),
  )
  if (options.configured) await fsp.writeFile(path.join(fusion, ".env.jnoccio"), "OPENROUTER_API_KEY=\n")
  return root
}

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
})

describe("Jnoccio locked provider metadata", () => {
  test("is public, locked, and skipped by default model selection until configured", async () => {
    const root = await tempRepo()
    const provider = Provider.jnoccioProviderInfo(root)

    expect(provider).toMatchObject({
      id: "jnoccio",
      source: "api",
      models: {
        "jnoccio-fusion": {
          status: "locked",
        },
      },
    })
    expect(Provider.defaultModelIDs({ jnoccio: provider })).toEqual({})
    expect(Provider.connectedProviderIDs({ jnoccio: provider })).toEqual([])
  })

  test("becomes active when plaintext files and .env.jnoccio are present", async () => {
    const root = await tempRepo({ configured: true })
    const provider = Provider.jnoccioProviderInfo(root)

    expect(provider.models["jnoccio-fusion"].status).toBe("active")
    expect(Provider.defaultModelIDs({ jnoccio: provider })).toEqual({ jnoccio: "jnoccio-fusion" })
    expect(Provider.connectedProviderIDs({ jnoccio: provider })).toEqual(["jnoccio"])
  })

  test("active configured provider data wins over locked metadata during provider list merging", async () => {
    const locked = Provider.jnoccioProviderInfo(await tempRepo())
    const active = Provider.jnoccioProviderInfo(await tempRepo({ configured: true }))
    const merged = Object.assign({ jnoccio: locked }, { jnoccio: active })

    expect(merged.jnoccio.models["jnoccio-fusion"].status).toBe("active")
  })
})
