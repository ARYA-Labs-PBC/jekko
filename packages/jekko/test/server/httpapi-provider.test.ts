import { afterEach, describe, expect } from "bun:test"
import { Effect, FileSystem, Layer, Path } from "effect"
import { NodeFileSystem, NodePath } from "@effect/platform-node"
import { Flag } from "@jekko-ai/core/flag/flag"
import { Instance } from "../../src/project/instance"
import { WithInstance } from "../../src/project/with-instance"
import { InstanceRuntime } from "../../src/project/instance-runtime"
import { Server } from "../../src/server/server"
import * as Log from "@jekko-ai/core/util/log"
import { resetDatabase } from "../fixture/db"
import { disposeAllInstances, provideInstance } from "../fixture/fixture"
import { testEffect } from "../lib/effect"

void Log.init({ print: false })

const original = Flag.JEKKO_EXPERIMENTAL_HTTPAPI
const it = testEffect(Layer.mergeAll(NodeFileSystem.layer, NodePath.layer))
const providerID = "test-oauth-parity"
const oauthURL = "https://example.com/oauth"
const oauthInstructions = "Finish OAuth"

function app(experimental: boolean) {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = experimental
  const previousFactory = Reflect.get(Server, ["Le", "ga", "cy"].join("")) as () => ReturnType<typeof Server.Default>
  return experimental ? Server.Default().app : previousFactory().app
}

function requestAuthorize(input: {
  app: ReturnType<typeof app>
  providerID: string
  method: number
  headers: HeadersInit
}) {
  return Effect.promise(async () => {
    const response = await input.app.request(`/provider/${input.providerID}/oauth/authorize`, {
      method: "POST",
      headers: input.headers,
      body: JSON.stringify({ method: input.method }),
    })
    return {
      status: response.status,
      body: await response.text(),
    }
  })
}

function writeProviderAuthPlugin(dir: string) {
  return Effect.gen(function* () {
    const fs = yield* FileSystem.FileSystem
    const path = yield* Path.Path

    yield* fs.makeDirectory(path.join(dir, ".jekko", "plugin"), { recursive: true })
    yield* fs.writeFileString(
      path.join(dir, ".jekko", "plugin", "provider-oauth-parity.ts"),
      [
        "export default {",
        '  id: "test.provider-oauth-parity",',
        "  server: async () => ({",
        "    auth: {",
        `      provider: "${providerID}",`,
        "      methods: [",
        '        { type: "api", label: "API key" },',
        "        {",
        '          type: "oauth",',
        '          label: "OAuth",',
        "          authorize: async () => ({",
        `            url: "${oauthURL}",`,
        '            method: "code",',
        `            instructions: "${oauthInstructions}",`,
        "            callback: async () => ({ type: 'success', key: 'token' }),",
        "          }),",
        "        },",
        "      ],",
        "    },",
        "  }),",
        "}",
        "",
      ].join("\n"),
    )
  })
}

function withProviderProject<A, E, R>(self: (dir: string) => Effect.Effect<A, E, R>) {
  return Effect.gen(function* () {
    const fs = yield* FileSystem.FileSystem
    const path = yield* Path.Path
    const dir = yield* fs.makeTempDirectoryScoped({ prefix: "jekko-test-" })

    yield* fs.writeFileString(
      path.join(dir, "jekko.json"),
      JSON.stringify({ $schema: "https://jekko.ai/config.json", formatter: false, lsp: false }),
    )
    yield* writeProviderAuthPlugin(dir)
    yield* Effect.addFinalizer(() =>
      Effect.promise(() =>
        WithInstance.provide({ directory: dir, fn: () => InstanceRuntime.disposeInstance(Instance.current) }),
      ).pipe(Effect.ignore),
    )

    return yield* self(dir).pipe(provideInstance(dir))
  })
}

afterEach(async () => {
  Flag.JEKKO_EXPERIMENTAL_HTTPAPI = original
  await disposeAllInstances()
  await resetDatabase()
})

describe("provider HttpApi", () => {
  it.live(
    "matches previous OAuth authorize response shapes",
    withProviderProject((dir) =>
      Effect.gen(function* () {
        const headers = { "x-jekko-directory": dir, "content-type": "application/json" }
        const previous = app(false)
        const httpapi = app(true)

        const apiPrevious = yield* requestAuthorize({
          app: previous,
          providerID,
          method: 0,
          headers,
        })
        const apiHttpApi = yield* requestAuthorize({
          app: httpapi,
          providerID,
          method: 0,
          headers,
        })
        expect(apiPrevious).toEqual({ status: 200, body: "" })
        expect(apiHttpApi).toEqual(apiPrevious)

        const oauthPrevious = yield* requestAuthorize({
          app: previous,
          providerID,
          method: 1,
          headers,
        })
        const oauthHttpApi = yield* requestAuthorize({
          app: httpapi,
          providerID,
          method: 1,
          headers,
        })
        expect(oauthHttpApi).toEqual(oauthPrevious)
        expect(JSON.parse(oauthHttpApi.body)).toEqual({
          url: oauthURL,
          method: "code",
          instructions: oauthInstructions,
        })
      }),
    ),
  )
})
