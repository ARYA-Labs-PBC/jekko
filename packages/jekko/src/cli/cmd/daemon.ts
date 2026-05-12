import type { Argv } from "yargs"
import path from "path"
import { readFile } from "fs/promises"
import { Effect } from "effect"
import { cmd } from "./cmd"
import { effectCmd, fail } from "../effect-cmd"
import { UI } from "../ui"
import { Filesystem } from "@/util/filesystem"
import { Server } from "../../server/server"
import { ServerAuth } from "@/server/auth"

type RequestTarget = {
  readonly baseUrl: string
  readonly fetch: typeof fetch
  readonly headers?: HeadersInit
}

function target(args: { attach?: string; password?: string; username?: string }): RequestTarget {
  if (args.attach) {
    return {
      baseUrl: args.attach,
      fetch,
      headers: ServerAuth.headers({ password: args.password, username: args.username }),
    }
  }
  return {
    baseUrl: "http://jekko.internal",
    fetch: (async (input: RequestInfo | URL, init?: RequestInit) => {
      const request = new Request(input, init)
      return Server.Default().app.fetch(request)
    }) as typeof fetch,
  }
}

async function requestJson(input: RequestTarget, path: string, init: RequestInit = {}) {
  const response = await input.fetch(new URL(path, input.baseUrl), {
    ...init,
    headers: {
      "content-type": "application/json",
      ...input.headers,
      ...(init.headers ?? {}),
    },
  })
  const text = await response.text()
  if (!response.ok) throw new Error(text || `${response.status} ${response.statusText}`)
  return text ? JSON.parse(text) : undefined
}

async function requestJsonArray(input: RequestTarget, path: string, init: RequestInit = {}) {
  const value = await requestJson(input, path, init)
  if (Array.isArray(value)) return value
  throw new Error(`Expected array response from ${path}`)
}

async function loadFile(file: string) {
  const resolved = path.resolve(process.cwd(), file)
  if (!(await Filesystem.exists(resolved))) throw new Error(`File not found: ${file}`)
  return readFile(resolved, "utf8")
}

function formatScore(value?: number) {
  if (value === undefined || Number.isNaN(value)) return "?"
  return `${Math.round(value * 100)}%`
}

export function formatDaemonRunSummary(run: any, taskCount = 0) {
  return [
    `run ${run.id}`,
    `status ${run.status}`,
    `phase ${run.phase}`,
    `iter ${run.iteration}`,
    `epoch ${run.epoch}`,
    `tasks ${taskCount}`,
    run.last_error ? `error ${run.last_error}` : undefined,
  ]
    .filter(Boolean)
    .join(" · ")
}

export function formatDaemonRunList(runs: readonly any[]) {
  if (!runs.length) return "No daemon runs."
  return runs.map((run) => formatDaemonRunSummary(run)).join("\n")
}

export function formatDaemonTaskSummary(task: any, passCount = 0) {
  return [
    `task ${task.title}`,
    `lane ${task.lane}`,
    `status ${task.status}`,
    `ready ${formatScore(task.readiness_score)}`,
    `risk ${formatScore(task.risk_score)}`,
    `passes ${passCount}`,
    task.blocked_reason ? `blocked ${task.blocked_reason}` : undefined,
  ]
    .filter(Boolean)
    .join(" · ")
}

export function formatDaemonTaskList(tasks: readonly any[], passCounts = new Map<string, number>()) {
  if (!tasks.length) return "No daemon tasks."
  return tasks.map((task) => formatDaemonTaskSummary(task, passCounts.get(task.id) ?? 0)).join("\n")
}

export const DaemonCommand = cmd({
  command: "daemon",
  describe: "manage daemon runs",
  builder: (yargs: Argv) =>
    yargs
      .command(DaemonPreviewCommand)
      .command(DaemonStartCommand)
      .command(DaemonStatusCommand)
      .command(DaemonPauseCommand)
      .command(DaemonResumeCommand)
      .command(DaemonAbortCommand)
      .command(DaemonTasksCommand)
      .command(DaemonTaskCommand)
      .command(DaemonPassesCommand)
      .command(DaemonMemoryCommand)
      .command(DaemonIncubateCommand)
      .command(DaemonPromoteCommand)
      .demandCommand(),
  async handler() {},
})

export const DaemonPreviewCommand = effectCmd({
  command: "preview",
  describe: "preview an ZYAL daemon file",
  instance: false,
  builder: (yargs) =>
    yargs.option("file", {
      alias: "f",
      type: "string",
      demandOption: true,
      describe: "ZYAL file to preview",
    }),
  handler: Effect.fn("Cli.daemon.preview")(function* (args) {
    const input = target(args)
    const text = yield* Effect.promise(() => loadFile(args.file))
    const preview = yield* Effect.promise(() =>
      requestJson(input, "/daemon/preview", {
        method: "POST",
        body: JSON.stringify({ text }),
      }),
    )
    UI.println(JSON.stringify(preview, null, 2))
  }),
})

export const DaemonStartCommand = effectCmd({
  command: "start",
  describe: "start a daemon run from a ZYAL file",
  instance: false,
  builder: (yargs) =>
    yargs
      .option("file", {
        alias: "f",
        type: "string",
        demandOption: true,
        describe: "ZYAL file to start",
      })
      .option("session", {
        alias: "s",
        type: "string",
        describe: "session id to attach the daemon to",
      })
      .option("arm", {
        type: "string",
        describe: "required arm sentinel",
        default: "RUN_FOREVER",
      })
      .option("attach", {
        type: "string",
        describe: "attach to a running jekko server (e.g. http://localhost:4096)",
      })
      .option("password", {
        alias: ["p"],
        type: "string",
        describe: "basic auth password (defaults to JEKKO_SERVER_PASSWORD)",
      })
      .option("username", {
        alias: ["u"],
        type: "string",
        describe: "basic auth username (defaults to JEKKO_SERVER_USERNAME or 'jekko')",
      }),
  handler: Effect.fn("Cli.daemon.start")(function* (args) {
    if (args.arm !== "RUN_FOREVER") {
      return yield* fail(`Unsupported arm: ${args.arm}`)
    }

    const input = target(args)
    const text = yield* Effect.promise(() => loadFile(args.file))
    const preview = yield* Effect.promise(() =>
      requestJson(input, "/daemon/preview", {
        method: "POST",
        body: JSON.stringify({ text }),
      }),
    )

    let sessionID = args.session
    if (!sessionID) {
      const session = yield* Effect.promise(() =>
        requestJson(input, "/session", {
          method: "POST",
          body: JSON.stringify({
            title: (preview as any)?.spec?.job?.name ?? "ZYAL daemon",
          }),
        }),
      )
      sessionID = (session as any)?.id
    }
    if (!sessionID) return yield* fail("Failed to create or resolve a session")

    const run = yield* Effect.promise(() =>
      requestJson(input, `/session/${sessionID}/daemon/start`, {
        method: "POST",
        body: JSON.stringify({
          parts: [{ type: "text", text }],
        }),
      }),
    )
    UI.println(JSON.stringify(run, null, 2))
  }),
})

export const DaemonStatusCommand = effectCmd({
  command: "status [runID]",
  describe: "show daemon run status",
  instance: false,
  builder: (yargs) =>
    yargs.positional("runID", {
      type: "string",
      describe: "daemon run id",
    }),
  handler: Effect.fn("Cli.daemon.status")(function* (args) {
    const input = target(args)
    if (!args.runID) {
      const runs = yield* Effect.promise(() => requestJsonArray(input, "/daemon"))
      UI.println(formatDaemonRunList(runs))
      return
    }
    const run = args.runID
      ? yield* Effect.promise(() => requestJson(input, `/daemon/${args.runID}`))
      : undefined
    if (!run) return
    const tasks = yield* Effect.promise(() => requestJsonArray(input, `/daemon/${args.runID}/tasks`))
    UI.println(formatDaemonRunSummary(run as any, tasks.length))
    if (tasks.length) UI.println(formatDaemonTaskList(tasks))
  }),
})

export const DaemonPauseCommand = effectCmd({
  command: "pause <runID>",
  describe: "pause a daemon run",
  instance: false,
  builder: (yargs) =>
    yargs.positional("runID", {
      type: "string",
      demandOption: true,
    }),
  handler: Effect.fn("Cli.daemon.pause")(function* (args) {
    const input = target(args)
    const run = yield* Effect.promise(() =>
      requestJson(input, `/daemon/${args.runID}/pause`, {
        method: "POST",
      }),
    )
    UI.println(JSON.stringify(run, null, 2))
  }),
})

export const DaemonResumeCommand = effectCmd({
  command: "resume <runID>",
  describe: "resume a daemon run",
  instance: false,
  builder: (yargs) =>
    yargs.positional("runID", {
      type: "string",
      demandOption: true,
    }),
  handler: Effect.fn("Cli.daemon.resume")(function* (args) {
    const input = target(args)
    const run = yield* Effect.promise(() =>
      requestJson(input, `/daemon/${args.runID}/resume`, {
        method: "POST",
      }),
    )
    UI.println(JSON.stringify(run, null, 2))
  }),
})

export const DaemonAbortCommand = effectCmd({
  command: "abort <runID>",
  describe: "abort a daemon run",
  instance: false,
  builder: (yargs) =>
    yargs.positional("runID", {
      type: "string",
      demandOption: true,
    }),
  handler: Effect.fn("Cli.daemon.abort")(function* (args) {
    const input = target(args)
    const run = yield* Effect.promise(() =>
      requestJson(input, `/daemon/${args.runID}/abort`, {
        method: "POST",
      }),
    )
    UI.println(JSON.stringify(run, null, 2))
  }),
})

export const DaemonTasksCommand = effectCmd({
  command: "tasks <runID>",
  describe: "list daemon tasks",
  instance: false,
  builder: (yargs) => yargs.positional("runID", { type: "string", demandOption: true }),
  handler: Effect.fn("Cli.daemon.tasks")(function* (args) {
    const input = target(args)
    const tasks = yield* Effect.promise(() => requestJsonArray(input, `/daemon/${args.runID}/tasks`))
    const passCounts = new Map<string, number>()
    yield* Effect.forEach(
      tasks,
      Effect.fnUntraced(function* (task: any) {
        const passes = yield* Effect.promise(() =>
          requestJsonArray(input, `/daemon/${args.runID}/tasks/${task.id}/passes`),
        )
        passCounts.set(task.id, passes.length)
      }),
      { concurrency: 1 },
    )
    UI.println(formatDaemonTaskList(tasks, passCounts))
  }),
})

export const DaemonTaskCommand = effectCmd({
  command: "task <runID> <taskID>",
  describe: "show daemon task",
  instance: false,
  builder: (yargs) =>
    yargs
      .positional("runID", { type: "string", demandOption: true })
      .positional("taskID", { type: "string", demandOption: true }),
  handler: Effect.fn("Cli.daemon.task")(function* (args) {
    const input = target(args)
    const task = yield* Effect.promise(() => requestJson(input, `/daemon/${args.runID}/tasks/${args.taskID}`))
    UI.println(JSON.stringify(task, null, 2))
  }),
})

export const DaemonPassesCommand = effectCmd({
  command: "passes <runID> <taskID>",
  describe: "list daemon task passes",
  instance: false,
  builder: (yargs) =>
    yargs
      .positional("runID", { type: "string", demandOption: true })
      .positional("taskID", { type: "string", demandOption: true }),
  handler: Effect.fn("Cli.daemon.passes")(function* (args) {
    const input = target(args)
    const passes = yield* Effect.promise(() => requestJson(input, `/daemon/${args.runID}/tasks/${args.taskID}/passes`))
    UI.println(JSON.stringify(passes, null, 2))
  }),
})

export const DaemonMemoryCommand = effectCmd({
  command: "memory <runID> <taskID>",
  describe: "list daemon task memory",
  instance: false,
  builder: (yargs) =>
    yargs
      .positional("runID", { type: "string", demandOption: true })
      .positional("taskID", { type: "string", demandOption: true }),
  handler: Effect.fn("Cli.daemon.memory")(function* (args) {
    const input = target(args)
    const memory = yield* Effect.promise(() => requestJson(input, `/daemon/${args.runID}/tasks/${args.taskID}/memory`))
    UI.println(JSON.stringify(memory, null, 2))
  }),
})

export const DaemonIncubateCommand = effectCmd({
  command: "incubate <runID> <taskID>",
  describe: "move daemon task to incubator",
  instance: false,
  builder: (yargs) =>
    yargs
      .positional("runID", { type: "string", demandOption: true })
      .positional("taskID", { type: "string", demandOption: true }),
  handler: Effect.fn("Cli.daemon.incubate")(function* (args) {
    const input = target(args)
    const task = yield* Effect.promise(() =>
      requestJson(input, `/daemon/${args.runID}/tasks/${args.taskID}/incubate`, {
        method: "POST",
        body: JSON.stringify({}),
      }),
    )
    UI.println(JSON.stringify(task, null, 2))
  }),
})

export const DaemonPromoteCommand = effectCmd({
  command: "promote <runID> <taskID>",
  describe: "move daemon task to ready queue",
  instance: false,
  builder: (yargs) =>
    yargs
      .positional("runID", { type: "string", demandOption: true })
      .positional("taskID", { type: "string", demandOption: true }),
  handler: Effect.fn("Cli.daemon.promote")(function* (args) {
    const input = target(args)
    const task = yield* Effect.promise(() =>
      requestJson(input, `/daemon/${args.runID}/tasks/${args.taskID}/promote`, {
        method: "POST",
        body: JSON.stringify({}),
      }),
    )
    UI.println(JSON.stringify(task, null, 2))
  }),
})
