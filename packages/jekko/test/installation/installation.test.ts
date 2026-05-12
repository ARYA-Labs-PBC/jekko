import { describe, expect, test } from "bun:test"
import fs from "fs/promises"
import path from "path"
import { Effect, Layer, Stream } from "effect"
import { HttpClient, HttpClientRequest, HttpClientResponse } from "effect/unstable/http"
import { ChildProcess, ChildProcessSpawner } from "effect/unstable/process"
import { Installation } from "../../src/installation"
import { InstallationChannel, InstallationVersion } from "@jekko-ai/core/installation/version"
import { tmpdir } from "../fixture/fixture"

const encoder = new TextEncoder()

function mockHttpClient(handler: (request: HttpClientRequest.HttpClientRequest) => Response) {
  const client = HttpClient.make((request) => Effect.succeed(HttpClientResponse.fromWeb(request, handler(request))))
  return Layer.succeed(HttpClient.HttpClient, client)
}

function mockSpawner(handler: (cmd: string, args: readonly string[]) => string = () => "") {
  const spawner = ChildProcessSpawner.make((command) => {
    const std = ChildProcess.isStandardCommand(command) ? command : undefined
    const output = handler(std?.command ?? "", std?.args ?? [])
    return Effect.succeed(
      ChildProcessSpawner.makeHandle({
        pid: ChildProcessSpawner.ProcessId(0),
        exitCode: Effect.succeed(ChildProcessSpawner.ExitCode(0)),
        isRunning: Effect.succeed(false),
        kill: () => Effect.void,
        stdin: { [Symbol.for("effect/Sink/TypeId")]: Symbol.for("effect/Sink/TypeId") } as any,
        stdout: output ? Stream.make(encoder.encode(output)) : Stream.empty,
        stderr: Stream.empty,
        all: Stream.empty,
        getInputFd: () => ({ [Symbol.for("effect/Sink/TypeId")]: Symbol.for("effect/Sink/TypeId") }) as any,
        getOutputFd: () => Stream.empty,
        unref: Effect.succeed(Effect.void),
      }),
    )
  })
  return Layer.succeed(ChildProcessSpawner.ChildProcessSpawner, spawner)
}

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
  })
}

function testLayer(
  httpHandler: (request: HttpClientRequest.HttpClientRequest) => Response,
  spawnHandler?: (cmd: string, args: readonly string[]) => string,
) {
  return Installation.layer.pipe(Layer.provide(mockHttpClient(httpHandler)), Layer.provide(mockSpawner(spawnHandler)))
}

describe("installation", () => {
  describe("latest", () => {
    test("reads release version from GitHub releases", async () => {
      const layer = testLayer(() => jsonResponse({ tag_name: "v1.2.3" }))

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("unknown")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("1.2.3")
    })

    test("strips v prefix from GitHub release tag", async () => {
      const layer = testLayer(() => jsonResponse({ tag_name: "v4.0.0-beta.1" }))

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("curl")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("4.0.0-beta.1")
    })

    test("reads npm versions via registry", async () => {
      const calls: string[] = []
      const layer = testLayer((request) => {
        calls.push(request.url)
        return jsonResponse({ version: "1.5.0" })
      })

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("npm")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("1.5.0")
      expect(calls).toContain(`https://registry.npmjs.org/jekko-ai/${InstallationChannel}`)
    })

    test("reads bun versions via registry", async () => {
      const calls: string[] = []
      const layer = testLayer((request) => {
        calls.push(request.url)
        return jsonResponse({ version: "1.6.0" })
      })

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("bun")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("1.6.0")
      expect(calls).toContain(`https://registry.npmjs.org/jekko-ai/${InstallationChannel}`)
    })

    test("reads pnpm versions via registry", async () => {
      const calls: string[] = []
      const layer = testLayer((request) => {
        calls.push(request.url)
        return jsonResponse({ version: "1.7.0" })
      })

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("pnpm")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("1.7.0")
      expect(calls).toContain(`https://registry.npmjs.org/jekko-ai/${InstallationChannel}`)
    })

    test("reads scoop manifest versions", async () => {
      const layer = testLayer(() => jsonResponse({ version: "2.3.4" }))

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("scoop")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("2.3.4")
    })

    test("reads chocolatey feed versions", async () => {
      const layer = testLayer(() => jsonResponse({ d: { results: [{ Version: "3.4.5" }] } }))

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("choco")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("3.4.5")
    })

    test("reads brew formulae API versions", async () => {
      const layer = testLayer(
        () => jsonResponse({ versions: { stable: "2.0.0" } }),
        (cmd, args) => {
          // getBrewFormula: return core formula (no tap)
          if (cmd === "brew" && args.includes("--formula") && args.includes("anomalyco/tap/jekko")) return ""
          if (cmd === "brew" && args.includes("--formula") && args.includes("jekko")) return "jekko"
          return ""
        },
      )

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("brew")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("2.0.0")
    })

    test("reads brew tap info JSON via CLI", async () => {
      const brewInfoJson = JSON.stringify({
        formulae: [{ versions: { stable: "2.1.0" } }],
      })
      const layer = testLayer(
        () => jsonResponse({}), // HTTP not used for tap formula
        (cmd, args) => {
          if (cmd === "brew" && args.includes("anomalyco/tap/jekko") && args.includes("--formula")) return "jekko"
          if (cmd === "brew" && args.includes("--json=v2")) return brewInfoJson
          return ""
        },
      )

      const result = await Effect.runPromise(
        Installation.Service.use((svc) => svc.latest("brew")).pipe(Effect.provide(layer)),
      )
      expect(result).toBe("2.1.0")
    })
  })

  describe("upgrade", () => {
    test("repair mode forces brew reinstall and reports a killed smoke binary", async () => {
      if (process.platform === "win32") return

      await using tmp = await tmpdir()
      const brewLog = path.join(tmp.path, "brew.log")
      const brew = path.join(tmp.path, "brew")
      const brokenJekko = path.join(tmp.path, "jekko")

      await fs.writeFile(
        brew,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$JEKKO_BREW_LOG\"\nexit 0\n",
        "utf8",
      )
      await fs.writeFile(brokenJekko, "#!/bin/sh\nkill -9 $$\n", "utf8")
      await fs.chmod(brew, 0o755)
      await fs.chmod(brokenJekko, 0o755)

      const previousPath = process.env.PATH
      const previousLog = process.env.JEKKO_BREW_LOG
      process.env.PATH = `${tmp.path}${path.delimiter}${previousPath ?? ""}`
      process.env.JEKKO_BREW_LOG = brewLog
      try {
        const layer = testLayer(
          () => jsonResponse({}),
          (cmd, args) => {
            if (cmd === "brew" && args.includes("--formula") && args.includes("anomalyco/tap/jekko")) return ""
            if (cmd === "brew" && args.includes("--formula") && args.includes("jekko")) return "jekko"
            return ""
          },
        )

        const err = await Effect.runPromise(
          Installation.Service.use((svc) =>
            svc.upgrade("brew", InstallationVersion, { repair: true, binaryPath: brokenJekko }),
          ).pipe(Effect.provide(layer)),
        ).catch((err) => err)

        expect(await fs.readFile(brewLog, "utf8")).toContain("reinstall jekko")
        expect(err).toBeInstanceOf(Installation.UpgradeFailedError)
        expect(err).toMatchObject({
          code: 137,
          signal: "SIGKILL",
          binary: brokenJekko,
          method: "brew",
          formula: "jekko",
        })
        expect(err.stderr).toContain("Post-install smoke check failed")
        expect(err.stderr).toContain("signal: SIGKILL")
        expect(err.stderr).toContain("quarantine/signing/provenance")
      } finally {
        if (previousPath === undefined) {
          delete process.env.PATH
        } else {
          process.env.PATH = previousPath
        }
        if (previousLog === undefined) {
          delete process.env.JEKKO_BREW_LOG
        } else {
          process.env.JEKKO_BREW_LOG = previousLog
        }
      }
    })
  })
})
