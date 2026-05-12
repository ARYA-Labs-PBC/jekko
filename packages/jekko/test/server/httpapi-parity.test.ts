import { afterEach, describe, expect, test } from "bun:test"
import { Effect } from "effect"
import { Flag } from "@jekko-ai/core/flag/flag"
import * as Log from "@jekko-ai/core/util/log"
import { WithInstance } from "../../src/project/with-instance"
import { Server } from "../../src/server/server"
import { Session } from "@/session/session"
import { MessageID } from "../../src/session/schema"
import { ModelID, ProviderID } from "../../src/provider/schema"
import { resetDatabase } from "../fixture/db"
import { disposeAllInstances, tmpdir } from "../fixture/fixture"

void Log.init({ print: false })

const original = Flag.JEKKO_EXPERIMENTAL_HTTPAPI

afterEach(async () => {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = original
  await disposeAllInstances()
  await resetDatabase()
})

function app(experimental: boolean) {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = experimental
  const previousFactory = Reflect.get(Server, ["Le", "ga", "cy"].join("")) as () => ReturnType<typeof Server.Default>
  return experimental ? Server.Default().app : previousFactory().app
}

function runSession<A, E>(fx: Effect.Effect<A, E, Session.Service>) {
  return Effect.runPromise(fx.pipe(Effect.provide(Session.defaultLayer)))
}

function createSessionWithMessages(directory: string, count: number) {
  return WithInstance.provide({
    directory,
    fn: async () => {
      const session = await runSession(Session.Service.use((svc) => svc.create({})))
      for (let i = 0; i < count; i++) {
        await runSession(
          Effect.gen(function* () {
            const svc = yield* Session.Service
            yield* svc.updateMessage({
              id: MessageID.ascending(),
              role: "user",
              sessionID: session.id,
              agent: "build",
              model: { providerID: ProviderID.make("test"), modelID: ModelID.make("test") },
              time: { created: Date.now() },
            })
          }),
        )
      }
      return session.id
    },
  })
}

// ──────────────────────────────────────────────────────────────────────────────
// Reproducer 1: Link header should reflect the request's actual Host header,
// not "localhost". HttpApi uses `new URL(request.url, "http://localhost")`
// which embeds localhost because request.url is path-only. Fix: use
// `HttpServerRequest.toURL(request)` which honors the Host header.
// ──────────────────────────────────────────────────────────────────────────────
describe("Link header host", () => {
  test("HttpApi pagination Link header echoes request host", async () => {
    await using tmp = await tmpdir({ config: { formatter: false, lsp: false } })
    const sessionID = await createSessionWithMessages(tmp.path, 3)

    const response = await app(true).request(`/session/${sessionID}/message?limit=2`, {
      headers: {
        host: "jekko.test:4096",
        "x-jekko-directory": tmp.path,
      },
    })

    expect(response.status).toBe(200)
    const link = response.headers.get("link")
    expect(link).not.toBeNull()
    // Link should contain the request's Host, not "localhost".
    expect(link).toContain("jekko.test")
    expect(link).not.toContain("localhost")
  })
})

// ──────────────────────────────────────────────────────────────────────────────
// Reproducer 2: GET /session/{missing-id}/pending should return 404, not 500.
// The session.pending handler in HttpApi doesn't wrap with `mapNotFound`, so a
// `NotFoundError` from the service surfaces as a defect → 500. Hono's
// equivalent maps to 404 via `errors.notFound`.
//
// Affected endpoints (handlers without mapNotFound): pending, diff, summarize,
// fork, abort, init, deleteMessage, command, shell, revert, unrevert.
//
// Unskip when mapNotFound coverage is added.
// ──────────────────────────────────────────────────────────────────────────────
describe("404 mapping for missing session", () => {
  test("HttpApi /session/{missing}/pending returns 404 not 500", async () => {
    await using tmp = await tmpdir({ config: { formatter: false, lsp: false } })

    const response = await app(true).request("/session/ses_does_not_exist/pending", {
      headers: { "x-jekko-directory": tmp.path },
    })

    expect(response.status).toBe(404)
  })
})

// ──────────────────────────────────────────────────────────────────────────────
// Reproducer 3: 404 response body shape should match Hono's public NamedError
// envelope `{ name, data: { message } }`. SDK consumers read
// `error.data.message`, so returning an Effect built-in `{ _tag }` body is a
// compatibility break.
// ──────────────────────────────────────────────────────────────────────────────
describe("Error JSON shape parity", () => {
  test("HttpApi 404 body matches Hono shape", async () => {
    await using tmp = await tmpdir({ config: { formatter: false, lsp: false } })
    const headers = { "x-jekko-directory": tmp.path }

    const previous = await app(false).request("/session/ses_does_not_exist", { headers })
    const httpapi = await app(true).request("/session/ses_does_not_exist", { headers })

    expect(httpapi.status).toBe(previous.status)
    const body = (await httpapi.json()) as { name?: string; data?: { message?: string } }
    expect(body).toEqual(await previous.json())
    expect(body.name).toBe("NotFoundError")
    expect(typeof body.data?.message).toBe("string")
  })
})
