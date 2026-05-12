import { afterEach, beforeEach, describe, expect, test } from "bun:test"
import fsp from "fs/promises"
import path from "path"
import { Flag } from "@jekko-ai/core/flag/flag"
import { isJnoccioFusionUnlocked } from "../../src/util/jnoccio-unlock"
import { Provider } from "../../src/provider/provider"
import { Server } from "../../src/server/server"
import { disposeAllInstances } from "../fixture/fixture"
import { resetDatabase } from "../fixture/db"
import { cloneRepo, localUnlockPreflight, removeTempDirs, run, seedTestDatabase, withEnv } from "./jnoccio-local-helpers"

const tempDirs: string[] = []
const originalHttpApi = Flag.JEKKO_EXPERIMENTAL_HTTPAPI
const preflight = await localUnlockPreflight()
const localTest = preflight.ok ? test : test.skip

type ServerLike = {
  request(input: string | URL | Request, init?: RequestInit): Promise<Response> | Response
}

function app() {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = false
  return Server.Historical().app
}

function experimentalApp() {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = true
  return Server.Default().app
}

async function postJson(serverApp: ServerLike, route: string, directory: string, body?: unknown) {
  return await serverApp.request(route, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-jekko-directory": directory,
    },
    body: JSON.stringify(body ?? {}),
  })
}

async function getJson(serverApp: ServerLike, route: string, directory: string) {
  return await serverApp.request(route, {
    headers: {
      "x-jekko-directory": directory,
    },
  })
}

async function exerciseHonoBackend(secret: string) {
  const { clone, cloneParent } = await cloneRepo("jnoccio-unlock-route-historical-", tempDirs)
  const secretCachePath = path.join(cloneParent, "jnoccio-fusion.unlock")

  expect(isJnoccioFusionUnlocked(clone)).toBe(false)

  return await withEnv(
    {
      JNOCCIO_REPO_ROOT: clone,
      JNOCCIO_UNLOCK_SECRET_PATH: secretCachePath,
    },
    async () => {
      const serverApp = app()
      const unlock = await postJson(serverApp, "/provider/jnoccio/unlock", clone, { unlockSecret: secret })
      const unlockText = await unlock.text()
      expect({ body: unlockText, status: unlock.status }).toMatchObject({ status: 200 })
      expect(JSON.parse(unlockText)).toMatchObject({
        status: "unlocked",
        envCreated: true,
        secretSaved: true,
      })

      await expect(fsp.readFile(secretCachePath, "utf8")).resolves.toBe(secret)
      await expect(fsp.readFile(path.join(clone, "jnoccio-fusion", ".env.jnoccio"), "utf8")).resolves.toContain(
        "OPENROUTER_API_KEY=",
      )
      expect(isJnoccioFusionUnlocked(clone)).toBe(true)

      const metadata = await run(
        "cargo",
        ["metadata", "--manifest-path", path.join(clone, "jnoccio-fusion", "Cargo.toml"), "--no-deps", "--format-version", "1"],
        clone,
      )
      expect(metadata.exitCode).toBe(0)

      const providers = await getJson(serverApp, "/provider", clone)
      expect(providers.status).toBe(200)
      const body = (await providers.json()) as Provider.ListResult
      const jnoccio = body.all.find((provider) => provider.id === "jnoccio")
      expect(jnoccio?.models["jnoccio-fusion"]?.status).toBe("active")
      expect(body.default.jnoccio).toBe("jnoccio-fusion")
      expect(body.connected).toContain("jnoccio")

      return { backend: "hono" as const, clone }
    },
  )
}

beforeEach(async () => {
  await disposeAllInstances()
  await resetDatabase()
  await seedTestDatabase()
})

afterEach(async () => {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = originalHttpApi
  await disposeAllInstances()
  await resetDatabase()
  await removeTempDirs(tempDirs)
})

describe("Jnoccio unlock route local proof", () => {
  localTest(
    "unlocks a fresh clone through the historical Hono route",
    async () => {
      if (!preflight.ok) throw new Error(`local unlock proof skipped: ${preflight.reason}`)

      const historical = await exerciseHonoBackend(preflight.secret)

      expect(historical.backend).toBe("hono")
    },
    120000,
  )

  localTest(
    "unlocks a fresh clone through the experimental HttpApi route",
    async () => {
      if (!preflight.ok) throw new Error(`local unlock proof skipped: ${preflight.reason}`)

      const { clone, cloneParent } = await cloneRepo("jnoccio-unlock-route-httpapi-", tempDirs)
      const secretCachePath = path.join(cloneParent, "jnoccio-fusion.unlock")

      expect(isJnoccioFusionUnlocked(clone)).toBe(false)

      await withEnv(
        {
          JNOCCIO_REPO_ROOT: clone,
          JNOCCIO_UNLOCK_SECRET_PATH: secretCachePath,
        },
        async () => {
          const serverApp = experimentalApp()
          const unlock = await postJson(serverApp, "/provider/jnoccio/unlock", clone, { unlockSecret: preflight.secret })
          const unlockText = await unlock.text()
          expect({ body: unlockText, status: unlock.status }).toMatchObject({ status: 200 })
          expect(JSON.parse(unlockText)).toMatchObject({
            status: "unlocked",
            envCreated: true,
            secretSaved: true,
          })

          await expect(fsp.readFile(secretCachePath, "utf8")).resolves.toBe(preflight.secret)
          await expect(fsp.readFile(path.join(clone, "jnoccio-fusion", ".env.jnoccio"), "utf8")).resolves.toContain(
            "OPENROUTER_API_KEY=",
          )
          expect(isJnoccioFusionUnlocked(clone)).toBe(true)
        },
      )
    },
    120000,
  )
})
