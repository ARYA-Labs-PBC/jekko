import { afterEach, describe, expect, test } from "bun:test"
import { Flag } from "@jekko-ai/core/flag/flag"
import { InstanceRoutes } from "../../src/server/routes/instance"
import { DaemonPaths } from "../../src/server/routes/instance/httpapi/groups/daemon"
import * as Log from "@jekko-ai/core/util/log"
import { resetDatabase } from "../fixture/db"
import { disposeAllInstances, tmpdir } from "../fixture/fixture"

void Log.init({ print: false })

const original = Flag.JEKKO_EXPERIMENTAL_HTTPAPI

function legacyBridgeRoutes(experimental: boolean) {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = experimental
  const app = InstanceRoutes((() => {}) as any) as any
  return (app.routes ?? []).map((route: { method: string; path: string }) => `${route.method} ${route.path}`)
}

afterEach(async () => {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = original
  await disposeAllInstances()
  await resetDatabase()
})

describe("daemon http api", () => {
  test("exposes the daemon task routes", () => {
    expect(DaemonPaths.preview).toBe("/daemon/preview")
    expect(DaemonPaths.start).toBe("/session/:sessionID/daemon/start")
    expect(DaemonPaths.tasks).toBe("/daemon/:runID/tasks")
    expect(DaemonPaths.taskPasses).toBe("/daemon/:runID/tasks/:taskID/passes")
    expect(DaemonPaths.incubator).toBe("/daemon/:runID/incubator")
  })

  test("registers daemon historical routes in the historical Hono bridge", () => {
    const routes = legacyBridgeRoutes(true)

    expect(routes).toContain(`GET ${DaemonPaths.list}`)
    expect(routes).toContain(`POST ${DaemonPaths.start}`)
    expect(routes).toContain(`GET ${DaemonPaths.get}`)
    expect(routes).toContain(`GET ${DaemonPaths.events}`)
    expect(routes).toContain(`GET ${DaemonPaths.tasks}`)
    expect(routes).toContain(`POST ${DaemonPaths.pause}`)
    expect(routes).toContain(`POST ${DaemonPaths.abort}`)
  })

  test("registers daemon historical routes for the stable Hono backend", () => {
    const routes = legacyBridgeRoutes(false)

    expect(routes).toContain(`GET ${DaemonPaths.list}`)
    expect(routes).toContain(`POST ${DaemonPaths.start}`)
    expect(routes).toContain(`GET ${DaemonPaths.tasks}`)
  })

  test("serves daemon preview through the stable Hono bridge", async () => {
    Flag.JEKKO_EXPERIMENTAL_HTTPAPI = false
    await using tmp = await tmpdir({ config: { formatter: false, lsp: false } })
    const app = InstanceRoutes((() => {}) as any)
    const response = await app.request(DaemonPaths.preview, {
      method: "POST",
      headers: { "content-type": "application/json", "x-jekko-directory": tmp.path },
      body: JSON.stringify({ text: makeZyal() }),
    })

    expect(response.status).toBe(200)
    const body = await response.json()
    expect(body.preview.id).toBe("test")
  })
})

function makeZyal() {
  return `<<<ZYAL v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
}
