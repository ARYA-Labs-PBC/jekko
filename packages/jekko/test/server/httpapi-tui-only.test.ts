import { describe, expect, test } from "bun:test"
import { TuiPaths } from "../../src/server/routes/instance/httpapi/groups/tui"
import { isAllowedCorsOrigin } from "../../src/server/cors"
import { PublicApi } from "../../src/server/routes/instance/httpapi/public"
import { ExperimentalHttpApiServer } from "../../src/server/routes/instance/httpapi/server"
import { Server } from "../../src/server/server"
import * as Log from "@jekko-ai/core/util/log"
import { ConfigProvider, Layer } from "effect"
import { HttpRouter } from "effect/unstable/http"
import { OpenApi } from "effect/unstable/httpapi"
import { tmpdir } from "../fixture/fixture"

void Log.init({ print: false })

const methods = ["get", "post", "put", "delete", "patch"] as const

function effectApp(input?: { password?: string; username?: string }) {
  const handler = HttpRouter.toWebHandler(
    ExperimentalHttpApiServer.routes.pipe(
      Layer.provide(
        ConfigProvider.layer(
          ConfigProvider.fromUnknown({
            JEKKO_SERVER_PASSWORD: input?.password,
            JEKKO_SERVER_USERNAME: input?.username,
          }),
        ),
      ),
    ),
    { disableLogger: true },
  ).handler
  return {
    request(input: string | URL | Request, init?: RequestInit) {
      return handler(
        input instanceof Request ? input : new Request(new URL(input, "http://localhost"), init),
        ExperimentalHttpApiServer.context,
      )
    },
  }
}

function authorization(username: string, password: string) {
  return `Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`
}

function openApiRouteKeys(spec: { paths: Record<string, Partial<Record<(typeof methods)[number], unknown>>> }) {
  return Object.entries(spec.paths)
    .flatMap(([path, item]) =>
      methods.filter((method) => item[method]).map((method) => `${method.toUpperCase()} ${path}`),
    )
    .sort()
}

async function expectNotBrowserUi(response: Response) {
  const contentType = response.headers.get("content-type") ?? ""
  const body = await response.text()

  expect(response.status).not.toBe(200)
  expect(contentType).not.toContain("text/html")
  expect(body).not.toContain("<html")
  expect(body).not.toContain("app.jekko.ai")
}

describe("TUI-only HTTP server surface", () => {
  test("does not allow removed desktop renderer origins", () => {
    for (const origin of ["oc://renderer", "oc://renderer/dev", "tauri://localhost", "http://tauri.localhost"]) {
      expect(isAllowedCorsOrigin(origin)).toBe(false)
    }

    expect(isAllowedCorsOrigin("http://localhost:3000")).toBe(true)
    expect(isAllowedCorsOrigin("https://jekko.ai")).toBe(true)
  })

  test("does not serve browser UI from root or PWA asset paths", async () => {
    for (const path of ["/", "/site.webmanifest", "/web-app-manifest-192x192.png"]) {
      await expectNotBrowserUi(await effectApp().request(path))
    }
  })

  test("keeps API routes behind configured auth", async () => {
    await using tmp = await tmpdir({ git: true })
    const headers = { "x-jekko-directory": tmp.path, "content-type": "application/json" }

    const missing = await effectApp({ password: "secret" }).request(TuiPaths.appendPrompt, {
      method: "POST",
      headers,
      body: JSON.stringify({ text: "blocked" }),
    })
    const good = await effectApp({ password: "secret" }).request(TuiPaths.appendPrompt, {
      method: "POST",
      headers: { ...headers, authorization: authorization("jekko", "secret") },
      body: JSON.stringify({ text: "accepted" }),
    })

    expect(missing.status).toBe(401)
    expect(good.status).toBe(200)
    expect(await good.json()).toBe(true)
  })

  test("continues to serve TUI control routes", async () => {
    await using tmp = await tmpdir({ git: true, config: { formatter: false, lsp: false } })

    const response = await effectApp().request(TuiPaths.openHelp, {
      method: "POST",
      headers: { "x-jekko-directory": tmp.path, "content-type": "application/json" },
      body: JSON.stringify({}),
    })

    expect(response.status).toBe(200)
    expect(await response.json()).toBe(true)
  })

  test("keeps generated OpenAPI route parity", async () => {
    const honoRoutes = openApiRouteKeys(await Server.openapiHono())
    const effectRoutes = openApiRouteKeys(OpenApi.fromApi(PublicApi))

    expect(honoRoutes.filter((route) => !effectRoutes.includes(route))).toEqual([])
    expect(effectRoutes.filter((route) => !honoRoutes.includes(route))).toEqual([
      "GET /api/session",
      "GET /api/session/{sessionID}/context",
      "GET /api/session/{sessionID}/message",
      "GET /daemon",
      "GET /daemon/{runID}",
      "GET /daemon/{runID}/events",
      "GET /daemon/{runID}/incubator",
      "GET /daemon/{runID}/tasks",
      "GET /daemon/{runID}/tasks/{taskID}",
      "GET /daemon/{runID}/tasks/{taskID}/memory",
      "GET /daemon/{runID}/tasks/{taskID}/passes",
      "POST /api/session/{sessionID}/compact",
      "POST /api/session/{sessionID}/prompt",
      "POST /api/session/{sessionID}/wait",
      "POST /daemon/preview",
      "POST /daemon/{runID}/abort",
      "POST /daemon/{runID}/compact",
      "POST /daemon/{runID}/pause",
      "POST /daemon/{runID}/resume",
      "POST /daemon/{runID}/rotate-session",
      "POST /daemon/{runID}/tasks/{taskID}/archive",
      "POST /daemon/{runID}/tasks/{taskID}/block",
      "POST /daemon/{runID}/tasks/{taskID}/incubate",
      "POST /daemon/{runID}/tasks/{taskID}/promote",
      "POST /session/{sessionID}/daemon/start",
    ])
  })
})
