import { cmd } from "@/cli/cmd/cmd"
import { tui } from "./app"
import { Rpc } from "@/util/rpc"
import { type rpc } from "./worker"
import path from "path"
import { fileURLToPath } from "url"
import { UI } from "@/cli/ui"
import * as Log from "@jekko-ai/core/util/log"
import { errorMessage } from "@/util/error"
import { withTimeout } from "@/util/timeout"
import { withNetworkOptions, resolveNetworkOptionsNoConfig } from "@/cli/network"
import { Filesystem } from "@/util/filesystem"
import type { GlobalEvent } from "@jekko-ai/sdk/v2"
import type { EventSource } from "./context/sdk"
import { writeHeapSnapshot } from "v8"
import { TuiConfig } from "./config/tui"
import {
  JEKKO_PROCESS_ROLE,
  JEKKO_RUN_ID,
  ensureRunID,
  sanitizedProcessEnv,
} from "@jekko-ai/core/util/jekko-process"
import { runValidatedTuiCommandWithConfig } from "./command-guards"

declare global {
  const JEKKO_WORKER_PATH: string
}

type RpcClient = ReturnType<typeof Rpc.client<typeof rpc>>

const bootLog = Log.create({ service: "tui.boot" })

function createWorkerFetch(client: RpcClient): typeof fetch {
  const fn = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const request = new Request(input, init)
    const body = request.body ? await request.text() : undefined
    const result = await client.call("fetch", {
      url: request.url,
      method: request.method,
      headers: Object.fromEntries(request.headers.entries()),
      body,
    })
    return new Response(result.body, {
      status: result.status,
      headers: result.headers,
    })
  }
  return fn as typeof fetch
}

function createEventSource(client: RpcClient): EventSource {
  return {
    subscribe: async (handler) => {
      return client.on<GlobalEvent>("global.event", (e) => {
        handler(e)
      })
    },
  }
}

async function target() {
  if (typeof JEKKO_WORKER_PATH !== "undefined") return JEKKO_WORKER_PATH
  const dist = new URL("./cli/cmd/tui/worker.js", import.meta.url)
  if (await Filesystem.exists(fileURLToPath(dist))) return dist
  return new URL("./worker.ts", import.meta.url)
}

async function input(value?: string) {
  const piped = process.stdin.isTTY ? undefined : await Bun.stdin.text()
  if (!value) return piped
  if (!piped) return value
  return piped + "\n" + value
}

export function resolveThreadDirectory(project?: string, envPWD = process.env.PWD, cwd = process.cwd()) {
  const root = Filesystem.resolve(envPWD ?? cwd)
  if (project) return Filesystem.resolve(path.isAbsolute(project) ? project : path.join(root, project))
  return Filesystem.resolve(cwd)
}

export const TuiThreadCommand = cmd({
  command: "$0 [project]",
  describe: "start jekko tui",
  builder: (yargs) =>
    withNetworkOptions(yargs)
      .positional("project", {
        type: "string",
        describe: "path to start jekko in",
      })
      .option("model", {
        type: "string",
        alias: ["m"],
        describe: "model to use in the format of provider/model",
      })
      .option("continue", {
        alias: ["c"],
        describe: "continue the last session",
        type: "boolean",
      })
      .option("session", {
        alias: ["s"],
        type: "string",
        describe: "session id to continue",
      })
      .option("fork", {
        type: "boolean",
        describe: "fork the session when continuing (use with --continue or --session)",
      })
      .option("prompt", {
        type: "string",
        describe: "prompt to use",
      })
      .option("agent", {
        type: "string",
        describe: "agent to use",
      }),
  handler: async (args) => {
    // Resolve relative --project paths from PWD, then use the real cwd after
    // chdir so the thread and worker share the same directory key.
    const next = resolveThreadDirectory(args.project)
    const file = await target()
    bootLog.info("thread start", {
      cwd: process.cwd(),
      target: String(file),
      project: args.project,
      log: Log.file(),
      has_api_key: Boolean(process.env.JEKKO_API_KEY),
      tuiwright: process.env.TUIWRIGHT === "1",
    })
    try {
      process.chdir(next)
    } catch {
      bootLog.error("thread chdir failed", {
        directory: next,
        log: Log.file(),
      })
      UI.error("Failed to change directory to " + next)
      return
    }
    const cwd = Filesystem.resolve(process.cwd())
    const env = sanitizedProcessEnv({
      [JEKKO_PROCESS_ROLE]: "worker",
      [JEKKO_RUN_ID]: ensureRunID(),
    })

    const worker = new Worker(file, {
      env,
    })
    bootLog.info("thread worker created", {
      cwd,
      target: String(file),
      run_id: env[JEKKO_RUN_ID],
      log: Log.file(),
    })
    worker.onerror = (e) => {
      bootLog.error("thread error", {
        message: e.message,
        filename: e.filename,
        lineno: e.lineno,
        colno: e.colno,
        error: e.error,
      })
    }

    const client = Rpc.client<typeof rpc>(worker)
    const error = (e: unknown) => {
      bootLog.error("process error", { error: errorMessage(e), log: Log.file() })
    }
    const reload = () => {
      client.call("reload", undefined).catch((err) => {
        Log.Default.warn("worker reload failed", {
          error: errorMessage(err),
        })
      })
    }
    process.on("uncaughtException", error)
    process.on("unhandledRejection", error)
    process.on("SIGUSR2", reload)

    let stopped = false
    const stop = async () => {
      if (stopped) return
      stopped = true
      process.off("uncaughtException", error)
      process.off("unhandledRejection", error)
      process.off("SIGUSR2", reload)
      await withTimeout(client.call("shutdown", undefined), 5000).catch((error) => {
        Log.Default.warn("worker shutdown failed", {
          error: errorMessage(error),
        })
      })
      worker.terminate()
    }

    const prompt = await input(args.prompt)
    const network = resolveNetworkOptionsNoConfig(args)
    const external =
      process.argv.includes("--port") ||
      process.argv.includes("--hostname") ||
      process.argv.includes("--mdns") ||
      network.mdns ||
      network.port !== 0 ||
      network.hostname !== "127.0.0.1"

    const transport = external
      ? {
          url: (await client.call("server", network)).url,
          fetch: undefined,
          events: undefined,
        }
      : {
          url: "http://jekko.internal",
          fetch: createWorkerFetch(client),
          events: createEventSource(client),
        }
    bootLog.info("thread transport ready", {
      cwd,
      mode: external ? "external" : "worker-rpc",
      url: transport.url,
      log: Log.file(),
    })

    await runValidatedTuiCommandWithConfig(
      args,
      {
        url: transport.url,
        sessionID: args.session,
        directory: cwd,
        fetch: transport.fetch,
      },
      async () => TuiConfig.get(),
      async (config) => {
        setTimeout(() => {
          client.call("checkUpgrade", { directory: cwd }).catch(() => {})
        }, 1000).unref?.()

        try {
          bootLog.info("thread tui start", {
            cwd,
            transport: external ? "external" : "worker-rpc",
            log: Log.file(),
          })
          await tui({
            url: transport.url,
            async onSnapshot() {
              const tui = writeHeapSnapshot("tui.heapsnapshot")
              const server = await client.call("snapshot", undefined)
              return [tui, server]
            },
            config,
            directory: cwd,
            fetch: transport.fetch,
            events: transport.events,
            args: {
              continue: args.continue,
              sessionID: args.session,
              agent: args.agent,
              model: args.model,
              prompt,
              fork: args.fork,
            },
          })
          bootLog.info("thread tui stopped", {
            cwd,
            log: Log.file(),
          })
        } finally {
          await stop()
        }
      },
    )
    process.exit(0)
  },
})
