import { Effect, Layer, Schema, Context, Stream } from "effect"
import { FetchHttpClient, HttpClient, HttpClientRequest, HttpClientResponse } from "effect/unstable/http"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { withTransientReadRetry } from "@/util/effect-http-client"
import { ChildProcess, ChildProcessSpawner } from "effect/unstable/process"
import path from "path"
import z from "zod"
import { BusEvent } from "@/bus/bus-event"
import { Flag } from "@jekko-ai/core/flag/flag"
import * as Log from "@jekko-ai/core/util/log"
import { makeRuntime } from "@jekko-ai/core/effect/runtime"
import semver from "semver"
import { InstallationChannel, InstallationVersion } from "@jekko-ai/core/installation/version"
import { NpmConfig } from "@jekko-ai/core/npm-config"
import { Process } from "@/util/process"

const log = Log.create({ service: "installation" })

export type Method = "curl" | "npm" | "yarn" | "pnpm" | "bun" | "brew" | "scoop" | "choco" | "unknown"

export type ReleaseType = "patch" | "minor" | "major"

export interface UpgradeOptions {
  repair?: boolean
  binaryPath?: string
}

export const Event = {
  Updated: BusEvent.define(
    "installation.updated",
    Schema.Struct({
      version: Schema.String,
    }),
  ),
  UpdateAvailable: BusEvent.define(
    "installation.update-available",
    Schema.Struct({
      version: Schema.String,
    }),
  ),
}

export function getReleaseType(current: string, latest: string): ReleaseType {
  const currMajor = semver.major(current)
  const currMinor = semver.minor(current)
  const newMajor = semver.major(latest)
  const newMinor = semver.minor(latest)

  if (newMajor > currMajor) return "major"
  if (newMinor > currMinor) return "minor"
  return "patch"
}

export const Info = z
  .object({
    version: z.string(),
    latest: z.string(),
  })
  .meta({
    ref: "InstallationInfo",
  })
export type Info = z.infer<typeof Info>

export const USER_AGENT = `jekko/${InstallationChannel}/${InstallationVersion}/${Flag.JEKKO_CLIENT}`

export function isPreview() {
  return InstallationChannel !== "latest"
}

export function isLocal() {
  return InstallationChannel === "local"
}

export class UpgradeFailedError extends Schema.TaggedErrorClass<UpgradeFailedError>()("UpgradeFailedError", {
  stderr: Schema.String,
  stdout: Schema.optional(Schema.String),
  code: Schema.optional(Schema.Number),
  signal: Schema.optional(Schema.NullOr(Schema.String)),
  command: Schema.optional(Schema.Array(Schema.String)),
  binary: Schema.optional(Schema.String),
  method: Schema.optional(Schema.String),
  formula: Schema.optional(Schema.String),
  hint: Schema.optional(Schema.String),
}) {}

// Response schemas for external version APIs
const GitHubRelease = Schema.Struct({ tag_name: Schema.String })
const NpmPackage = Schema.Struct({ version: Schema.String })
const BrewFormula = Schema.Struct({ versions: Schema.Struct({ stable: Schema.String }) })
const BrewInfoV2 = Schema.Struct({
  formulae: Schema.Array(Schema.Struct({ versions: Schema.Struct({ stable: Schema.String }) })),
})
const ChocoPackage = Schema.Struct({
  d: Schema.Struct({ results: Schema.Array(Schema.Struct({ Version: Schema.String })) }),
})
const ScoopManifest = NpmPackage

type CommandResult = {
  command: string[]
  code: number
  signal: NodeJS.Signals | null
  stdout: string
  stderr: string
}

function tail(text: string, max = 4_000) {
  if (text.length <= max) return text
  return text.slice(text.length - max)
}

function repairHint(signal: string | null | undefined, method: Method, binary: string) {
  if (method === "choco") {
    return `Chocolatey may be not running from an elevated command shell. Retry from Administrator, then retry jekko upgrade --repair -m ${method}.`
  }
  if (signal === "SIGKILL") {
    return `Executable was killed by the OS. On macOS inspect quarantine/signing/provenance for ${binary}, then retry jekko upgrade --repair -m ${method}.`
  }
  return `Retry jekko upgrade --repair -m ${method}; if it still fails, inspect package-manager provenance and installed files for ${binary}.`
}

function formatFailure(input: {
  stage: string
  method: Method
  target: string
  binary: string
  formula?: string
  result?: CommandResult
  hint?: string
}) {
  const lines = [
    `${input.stage} failed`,
    `method: ${input.method}`,
    `target: ${input.target}`,
    `binary: ${input.binary}`,
  ]
  if (input.formula) lines.push(`formula: ${input.formula}`)
  if (input.result) {
    lines.push(`command: ${input.result.command.join(" ")}`)
    lines.push(`exit code: ${input.result.code}`)
    lines.push(`signal: ${input.result.signal ?? "none"}`)
    const stdout = tail(input.result.stdout).trim()
    const stderr = tail(input.result.stderr).trim()
    if (stdout) lines.push(`stdout tail:\n${stdout}`)
    if (stderr) lines.push(`stderr tail:\n${stderr}`)
  }
  if (input.hint) lines.push(`hint: ${input.hint}`)
  return lines.join("\n")
}

export interface Interface {
  readonly info: () => Effect.Effect<Info>
  readonly method: () => Effect.Effect<Method>
  readonly latest: (method?: Method) => Effect.Effect<string>
  readonly upgrade: (method: Method, target: string, options?: UpgradeOptions) => Effect.Effect<void, UpgradeFailedError>
}

export class Service extends Context.Service<Service, Interface>()("@jekko/Installation") {}

export const layer: Layer.Layer<Service, never, HttpClient.HttpClient | ChildProcessSpawner.ChildProcessSpawner> =
  Layer.effect(
    Service,
    Effect.gen(function* () {
      const http = yield* HttpClient.HttpClient
      const httpOk = HttpClient.filterStatusOk(withTransientReadRetry(http))
      const spawner = yield* ChildProcessSpawner.ChildProcessSpawner

      const text = Effect.fnUntraced(
        function* (cmd: string[], opts?: { cwd?: string; env?: Record<string, string> }) {
          const proc = ChildProcess.make(cmd[0], cmd.slice(1), {
            cwd: opts?.cwd,
            env: opts?.env,
            extendEnv: true,
          })
          const handle = yield* spawner.spawn(proc)
          const out = yield* Stream.mkString(Stream.decodeText(handle.stdout))
          yield* handle.exitCode
          return out
        },
        Effect.scoped,
        Effect.catch(() => Effect.succeed("")),
      )

      const run = Effect.fnUntraced(function* (
        cmd: string[],
        opts?: { cwd?: string; env?: Record<string, string>; input?: string | Buffer | Uint8Array },
      ) {
        const out = yield* Effect.promise(() =>
          Process.run(cmd, {
            cwd: opts?.cwd,
            env: opts?.env,
            input: opts?.input,
            nothrow: true,
          }),
        )
        return {
          command: [...cmd],
          code: out.code,
          signal: out.signal,
          stdout: out.stdout.toString(),
          stderr: out.stderr.toString(),
        } satisfies CommandResult
      })

      const getBrewFormula = Effect.fnUntraced(function* () {
        const tapFormula = yield* text(["brew", "list", "--formula", "anomalyco/tap/jekko"])
        if (tapFormula.includes("jekko")) return "anomalyco/tap/jekko"
        const coreFormula = yield* text(["brew", "list", "--formula", "jekko"])
        if (coreFormula.includes("jekko")) return "jekko"
        return "jekko"
      })

      const upgradeCurl = Effect.fnUntraced(
        function* (target: string) {
          const response = yield* httpOk.execute(HttpClientRequest.get("https://jekko.ai/install"))
          const body = yield* response.text
          const bodyBytes = new TextEncoder().encode(body)
          return yield* run(["bash"], { env: { VERSION: target }, input: bodyBytes })
        },
        Effect.orDie,
      )

      const result: Interface = {
        info: Effect.fn("Installation.info")(function* () {
          return {
            version: InstallationVersion,
            latest: yield* result.latest(),
          }
        }),
        method: Effect.fn("Installation.method")(function* () {
          if (process.execPath.includes(path.join(".jekko", "bin"))) return "curl" as Method
          if (process.execPath.includes(path.join(".local", "bin"))) return "curl" as Method
          const exec = process.execPath.toLowerCase()

          const checks: Array<{ name: Method; command: () => Effect.Effect<string> }> = [
            { name: "npm", command: () => text(["npm", "list", "-g", "--depth=0"]) },
            { name: "yarn", command: () => text(["yarn", "global", "list"]) },
            { name: "pnpm", command: () => text(["pnpm", "list", "-g", "--depth=0"]) },
            { name: "bun", command: () => text(["bun", "pm", "ls", "-g"]) },
            { name: "brew", command: () => text(["brew", "list", "--formula", "jekko"]) },
            { name: "scoop", command: () => text(["scoop", "list", "jekko"]) },
            { name: "choco", command: () => text(["choco", "list", "--limit-output", "jekko"]) },
          ]

          checks.sort((a, b) => {
            const aMatches = exec.includes(a.name)
            const bMatches = exec.includes(b.name)
            if (aMatches && !bMatches) return -1
            if (!aMatches && bMatches) return 1
            return 0
          })

          for (const check of checks) {
            const output = yield* check.command()
            const installedName =
              check.name === "brew" || check.name === "choco" || check.name === "scoop" ? "jekko" : "jekko-ai"
            if (output.includes(installedName)) {
              return check.name
            }
          }

          return "unknown" as Method
        }),
        latest: Effect.fn("Installation.latest")(function* (installMethod?: Method) {
          const detectedMethod = installMethod || (yield* result.method())

          if (detectedMethod === "brew") {
            const formula = yield* getBrewFormula()
            if (formula.includes("/")) {
              const infoJson = yield* text(["brew", "info", "--json=v2", formula])
              const info = yield* Schema.decodeUnknownEffect(Schema.fromJsonString(BrewInfoV2))(infoJson)
              return info.formulae[0].versions.stable
            }
            const response = yield* httpOk.execute(
              HttpClientRequest.get("https://formulae.brew.sh/api/formula/jekko.json").pipe(
                HttpClientRequest.acceptJson,
              ),
            )
            const data = yield* HttpClientResponse.schemaBodyJson(BrewFormula)(response)
            return data.versions.stable
          }

          if (detectedMethod === "npm" || detectedMethod === "bun" || detectedMethod === "pnpm") {
            const response = yield* httpOk.execute(
              HttpClientRequest.get(
                `${yield* NpmConfig.registry(process.cwd())}/jekko-ai/${InstallationChannel}`,
              ).pipe(HttpClientRequest.acceptJson),
            )
            const data = yield* HttpClientResponse.schemaBodyJson(NpmPackage)(response)
            return data.version
          }

          if (detectedMethod === "choco") {
            const response = yield* httpOk.execute(
              HttpClientRequest.get(
                "https://community.chocolatey.org/api/v2/Packages?$filter=Id%20eq%20%27jekko%27%20and%20IsLatestVersion&$select=Version",
              ).pipe(HttpClientRequest.setHeaders({ Accept: "application/json;odata=verbose" })),
            )
            const data = yield* HttpClientResponse.schemaBodyJson(ChocoPackage)(response)
            return data.d.results[0].Version
          }

          if (detectedMethod === "scoop") {
            const response = yield* httpOk.execute(
              HttpClientRequest.get(
                "https://raw.githubusercontent.com/ScoopInstaller/Main/master/bucket/jekko.json",
              ).pipe(HttpClientRequest.setHeaders({ Accept: "application/json" })),
            )
            const data = yield* HttpClientResponse.schemaBodyJson(ScoopManifest)(response)
            return data.version
          }

          const response = yield* httpOk.execute(
            HttpClientRequest.get("https://api.github.com/repos/neverhuman/jekko/releases/latest").pipe(
              HttpClientRequest.acceptJson,
            ),
          )
          const data = yield* HttpClientResponse.schemaBodyJson(GitHubRelease)(response)
          return data.tag_name.replace(/^v/, "")
        }, Effect.orDie),
        upgrade: Effect.fn("Installation.upgrade")(function* (m: Method, target: string, options: UpgradeOptions = {}) {
          let upgradeResult: CommandResult | undefined
          let formula: string | undefined
          const binary = options.binaryPath ?? process.execPath
          const fail = (stage: string, result?: CommandResult) => {
            const hint = repairHint(result?.signal, m, binary)
            const stderr = formatFailure({ stage, method: m, target, binary, formula, result, hint })
            log.error(stage, {
              method: m,
              target,
              binary,
              formula,
              command: result?.command,
              code: result?.code,
              signal: result?.signal,
              stdout: result ? tail(result.stdout) : undefined,
              stderr: result ? tail(result.stderr) : undefined,
              hint,
            })
            return new UpgradeFailedError({
              stderr,
              stdout: result?.stdout,
              code: result?.code,
              signal: result?.signal ?? null,
              command: result?.command,
              binary,
              method: m,
              formula,
              hint,
            })
          }
          switch (m) {
            case "curl":
              upgradeResult = yield* upgradeCurl(target)
              break
            case "npm":
              upgradeResult = yield* run(["npm", "install", "-g", `jekko-ai@${target}`])
              break
            case "pnpm":
              upgradeResult = yield* run(["pnpm", "install", "-g", `jekko-ai@${target}`])
              break
            case "bun":
              upgradeResult = yield* run(["bun", "install", "-g", `jekko-ai@${target}`])
              break
            case "brew": {
              formula = yield* getBrewFormula()
              const env = { HOMEBREW_NO_AUTO_UPDATE: "1" }
              if (formula.includes("/")) {
                const tap = yield* run(["brew", "tap", "anomalyco/tap"], { env })
                if (tap.code !== 0) {
                  upgradeResult = tap
                  break
                }
                const repo = yield* text(["brew", "--repo", "anomalyco/tap"])
                const dir = repo.trim()
                if (dir) {
                  const pull = yield* run(["git", "pull", "--ff-only"], { cwd: dir, env })
                  if (pull.code !== 0) {
                    upgradeResult = pull
                    break
                  }
                }
              }
              upgradeResult = yield* run(["brew", options.repair ? "reinstall" : "upgrade", formula], { env })
              break
            }
            case "choco":
              upgradeResult = yield* run(["choco", "upgrade", "jekko", `--version=${target}`, "-y"])
              break
            case "scoop":
              upgradeResult = yield* run(["scoop", "install", `jekko@${target}`])
              break
            default:
              return yield* new UpgradeFailedError({ stderr: `Unknown method: ${m}` })
          }
          if (!upgradeResult || upgradeResult.code !== 0) {
            if (m === "choco" && !upgradeResult) {
              return yield* new UpgradeFailedError({ stderr: "not running from an elevated command shell" })
            }
            return yield* fail("Upgrade command", upgradeResult)
          }
          log.info("upgraded", {
            method: m,
            target,
            repair: options.repair === true,
            stdout: upgradeResult.stdout,
            stderr: upgradeResult.stderr,
          })
          const smoke = yield* run([binary, "--version"])
          if (smoke.code !== 0) return yield* fail("Post-install smoke check", smoke)
        }),
      }

      return Service.of(result)
    }),
  )

export const defaultLayer = layer.pipe(
  Layer.provide(FetchHttpClient.layer),
  Layer.provide(CrossSpawnSpawner.defaultLayer),
)

const { runPromise } = makeRuntime(Service, defaultLayer)

export const latest = (...args: Parameters<Interface["latest"]>) => runPromise((s) => s.latest(...args))
export const method = () => runPromise((s) => s.method())
export const upgrade = (...args: Parameters<Interface["upgrade"]>) => runPromise((s) => s.upgrade(...args))

export * as Installation from "."
