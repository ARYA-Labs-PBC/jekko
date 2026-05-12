// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import type { Argv } from "yargs"
import path from "path"
import { pathToFileURL } from "url"
import { Effect, Schema } from "effect"
import { UI } from "../ui"
import { effectCmd } from "../effect-cmd"
import { Flag } from "@jekko-ai/core/flag/flag"
import { ServerAuth } from "@/server/auth"
import { EOL } from "os"
import { Filesystem } from "@/util/filesystem"
import { createOpencodeClient, type OpencodeClient, type ToolPart } from "@jekko-ai/sdk/v2"
import { Server } from "../../server/server"
import { Provider } from "@/provider/provider"
import { Agent } from "../../agent/agent"
import { Permission } from "../../permission"
import { ShellID } from "../../tool/shell/id"
import { PendingWriteTool } from "../../tool/pending"
import { Locale } from "@/util/locale"
import { Parameters as GlobParameters } from "../../tool/glob"
import { Parameters as GrepParameters } from "../../tool/grep"
import { Parameters as ReadParameters } from "../../tool/read"
import { Parameters as WebFetchParameters } from "../../tool/webfetch"
import { Parameters as EditParameters } from "../../tool/edit"
import { Parameters as WriteParameters } from "../../tool/write"
import { Parameters as WebSearchParameters } from "../../tool/websearch"
import { Parameters as TaskParameters } from "../../tool/task"
import { Parameters as SkillParameters } from "../../tool/skill"
import { Parameters as ShellParameters } from "../../tool/shell"
import { Parameters as PendingParameters } from "../../tool/pending"

type ToolProps<Input, Metadata extends object = never> = {
  input: Input
  metadata?: Metadata
  part: ToolPart
}

type GlobMetadata = {
  count?: number
  truncated?: boolean
}

type GrepMetadata = {
  matches?: number
  truncated?: boolean
}

type EditMetadata = {
  diff?: string
}

type DirectoryResolution =
  | { kind: "unset" }
  | { kind: "attached"; directory: string }
  | { kind: "changed"; directory: string }
  | { kind: "error"; message: string }

type AgentResolution =
  | { kind: "default"; reason: "not-requested" }
  | { kind: "default"; reason: "remote-list-failed" | "missing" | "subagent" }
  | { kind: "selected"; agent: string }

type GlobInput = Schema.Schema.Type<typeof GlobParameters>
type GrepInput = Schema.Schema.Type<typeof GrepParameters>
type ReadInput = Schema.Schema.Type<typeof ReadParameters>
type WebFetchInput = Schema.Schema.Type<typeof WebFetchParameters>
type EditInput = Schema.Schema.Type<typeof EditParameters>
type WriteInput = Schema.Schema.Type<typeof WriteParameters>
type WebSearchInput = Schema.Schema.Type<typeof WebSearchParameters>
type TaskInput = Schema.Schema.Type<typeof TaskParameters>
type SkillInput = Schema.Schema.Type<typeof SkillParameters>
type ShellInput = Schema.Schema.Type<typeof ShellParameters>
type PendingInput = Schema.Schema.Type<typeof PendingParameters>

function isRecord(input: unknown): input is Record<string, unknown> {
  return typeof input === "object" && input !== null
}

function isGlobMetadata(input: unknown): input is GlobMetadata {
  return isRecord(input)
}

function isGrepMetadata(input: unknown): input is GrepMetadata {
  return isRecord(input)
}

function isEditMetadata(input: unknown): input is EditMetadata {
  return isRecord(input)
}

function props<Input, Metadata extends object = never>(
  part: ToolPart,
  decode: (input: unknown) => Input,
  isMetadata?: (input: unknown) => input is Metadata,
): ToolProps<Input, Metadata> {
  const state = part.state
  const metadata = "metadata" in state && isMetadata && isMetadata(state.metadata) ? state.metadata : undefined
  return {
    input: decode(state.input),
    metadata,
    part,
  }
}

type Inline = {
  icon: string
  title: string
  description?: string
}

function inline(info: Inline) {
  const suffix = info.description ? UI.Style.TEXT_DIM + ` ${info.description}` + UI.Style.TEXT_NORMAL : ""
  UI.println(UI.Style.TEXT_NORMAL + info.icon, UI.Style.TEXT_NORMAL + info.title + suffix)
}

function block(info: Inline, output?: string) {
  UI.empty()
  inline(info)
  if (!output?.trim()) return
  UI.println(output)
  UI.empty()
}

function alternative_path(part: ToolPart) {
  const state = part.state
  const input = "input" in state ? state.input : undefined
  const title =
    ("title" in state && state.title ? state.title : undefined) ||
    (input && typeof input === "object" && Object.keys(input).length > 0 ? JSON.stringify(input) : "Unknown")
  inline({
    icon: "⚙",
    title: `${part.tool} ${title}`,
  })
}

function glob(info: ToolProps<GlobInput, GlobMetadata>) {
  const root = info.input.path ?? ""
  const title = `Glob "${info.input.pattern}"`
  const suffix = root ? `in ${normalizePath(root)}` : ""
  const num = info.metadata?.count
  const description =
    num === undefined ? suffix : `${suffix}${suffix ? " · " : ""}${num} ${num === 1 ? "match" : "matches"}`
  inline({
    icon: "✱",
    title,
    ...(description && { description }),
  })
}

function grep(info: ToolProps<GrepInput, GrepMetadata>) {
  const root = info.input.path ?? ""
  const title = `Grep "${info.input.pattern}"`
  const suffix = root ? `in ${normalizePath(root)}` : ""
  const num = info.metadata?.matches
  const description =
    num === undefined ? suffix : `${suffix}${suffix ? " · " : ""}${num} ${num === 1 ? "match" : "matches"}`
  inline({
    icon: "✱",
    title,
    ...(description && { description }),
  })
}

function read(info: ToolProps<ReadInput>) {
  const file = normalizePath(info.input.filePath)
  const pairs = Object.entries(info.input).filter(([key, value]) => {
    if (key === "filePath") return false
    return typeof value === "string" || typeof value === "number" || typeof value === "boolean"
  })
  const description = pairs.length ? `[${pairs.map(([key, value]) => `${key}=${value}`).join(", ")}]` : undefined
  inline({
    icon: "→",
    title: `Read ${file}`,
    ...(description && { description }),
  })
}

function write(info: ToolProps<WriteInput>) {
  block(
    {
      icon: "←",
      title: `Write ${normalizePath(info.input.filePath)}`,
    },
    info.part.state.status === "completed" ? info.part.state.output : undefined,
  )
}

function webfetch(info: ToolProps<WebFetchInput>) {
  inline({
    icon: "%",
    title: `WebFetch ${info.input.url}`,
  })
}

function edit(info: ToolProps<EditInput, EditMetadata>) {
  const title = normalizePath(info.input.filePath)
  const diff = info.metadata?.diff
  block(
    {
      icon: "←",
      title: `Edit ${title}`,
    },
    diff,
  )
}

function websearch(info: ToolProps<WebSearchInput>) {
  inline({
    icon: "◈",
    title: `Exa Web Search "${info.input.query}"`,
  })
}

function task(info: ToolProps<TaskInput>) {
  const input = info.input
  const status = info.part.state.status
  const subagent =
    typeof input.subagent_type === "string" && input.subagent_type.trim().length > 0 ? input.subagent_type : "unknown"
  const agent = Locale.titlecase(subagent)
  const desc =
    typeof input.description === "string" && input.description.trim().length > 0 ? input.description : undefined
  const icon = status === "error" ? "✗" : status === "running" ? "•" : "✓"
  const name = desc ?? `${agent} Task`
  inline({
    icon,
    title: name,
    description: desc ? `${agent} Agent` : undefined,
  })
}

function skill(info: ToolProps<SkillInput>) {
  inline({
    icon: "→",
    title: `Skill "${info.input.name}"`,
  })
}

function shell(info: ToolProps<ShellInput>) {
  const output = info.part.state.status === "completed" ? info.part.state.output?.trim() : undefined
  block(
    {
      icon: "$",
      title: `${info.input.command}`,
    },
    output,
  )
}

function pending(info: ToolProps<PendingInput>) {
  block(
    {
      icon: "#",
      title: "Pending items",
    },
    info.input.todos.map((item) => `${item.status === "completed" ? "[x]" : "[ ]"} ${item.content}`).join("\n"),
  )
}

function normalizePath(input?: string) {
  if (!input) return ""
  if (path.isAbsolute(input)) return path.relative(process.cwd(), input) || "."
  return input
}

export function resolveRunDirectory(args: { dir?: string; attach?: string }) {
  if (!args.dir) return { kind: "unset" } as const
  if (args.attach) return { kind: "attached", directory: args.dir } as const
  try {
    process.chdir(args.dir)
    return { kind: "changed", directory: process.cwd() } as const
  } catch {
    return {
      kind: "error",
      message: "Failed to change directory to " + args.dir,
    } as const
  }
}

type RunAgentSdk = Pick<OpencodeClient, "app">
type RunAgentService = {
  get(name: string): Effect.Effect<{ mode?: string } | undefined>
}

export async function resolveRunAgent(args: { agent?: string; attach?: string }, sdk: RunAgentSdk, agentSvc: RunAgentService) {
  if (!args.agent) return { kind: "default", reason: "not-requested" } as const
  const name = args.agent

  if (args.attach) {
    const modes = await sdk.app
      .agents(undefined, { throwOnError: true })
      .then((x) => x.data ?? [])
      .catch(() => undefined)

    if (!modes) {
      return { kind: "default", reason: "remote-list-failed" } as const
    }

    const agent = modes.find((entry) => entry.name === name)
    if (!agent) {
      return { kind: "default", reason: "missing" } as const
    }

    if (agent.mode === "subagent") {
      return { kind: "default", reason: "subagent" } as const
    }

    return { kind: "selected", agent: name } as const
  }

  const entry = await Effect.runPromise(agentSvc.get(name))
  if (!entry) {
    return { kind: "default", reason: "missing" } as const
  }
  if (entry.mode === "subagent") {
    return { kind: "default", reason: "subagent" } as const
  }
  return { kind: "selected", agent: name } as const
}

export function createLastAssistantMessageTracker(options?: { sessionID?: string }) {
  let assistantMessageID: string | undefined
  let partOrder: string[] = []
  const partText = new Map<string, string>()

  function reset(messageID: string) {
    assistantMessageID = messageID
    partOrder = []
    partText.clear()
  }

  return {
    observe(event: { type: string; properties: any }) {
      if (event.type === "message.updated") {
        const info = event.properties.info
        if (options?.sessionID && event.properties.sessionID !== options.sessionID) return
        if (info?.role === "assistant" && info.id !== assistantMessageID) {
          reset(info.id)
        }
      }

      if (event.type !== "message.part.updated") return
      const part = event.properties.part
      if (!assistantMessageID || part.sessionID !== event.properties.sessionID) return
      if (options?.sessionID && part.sessionID !== options.sessionID) return
      if (part.messageID !== assistantMessageID) return
      if (part.type !== "text") return
      if (!partText.has(part.id)) partOrder.push(part.id)
      partText.set(part.id, part.text ?? "")
    },
    text() {
      return partOrder.map((id) => partText.get(id) ?? "").join("")
    },
    async write(filePath: string) {
      await Filesystem.write(filePath, this.text())
    },
  }
}

export const RunCommand = effectCmd({
  command: "run [message..]",
  describe: "run jekko with a message",
  // --attach connects to a remote server (no local instance needed); the
  // default path runs an in-process server and needs the project instance.
  instance: (args) => !args.attach,
  // For --dir without --attach, load instance for the resolved target dir.
  // The handler also chdirs (preserving the historical order: chdir → file resolution).
  directory: (args) => (args.dir && !args.attach ? path.resolve(process.cwd(), args.dir) : process.cwd()),
  builder: (yargs: Argv) =>
    yargs
      .positional("message", {
        describe: "message to send",
        type: "string",
        array: true,
        default: [],
      })
      .option("command", {
        describe: "the command to run, use message for args",
        type: "string",
      })
      .option("continue", {
        alias: ["c"],
        describe: "continue the last session",
        type: "boolean",
      })
      .option("session", {
        alias: ["s"],
        describe: "session id to continue",
        type: "string",
      })
      .option("fork", {
        describe: "fork the session before continuing (requires --continue or --session)",
        type: "boolean",
      })
      .option("share", {
        type: "boolean",
        describe: "share the session",
      })
      .option("model", {
        type: "string",
        alias: ["m"],
        describe: "model to use in the format of provider/model",
      })
      .option("agent", {
        type: "string",
        describe: "agent to use",
      })
      .option("format", {
        type: "string",
        choices: ["default", "json"],
        default: "default",
        describe: "format: default (formatted) or json (raw JSON events)",
      })
      .option("file", {
        alias: ["f"],
        type: "string",
        array: true,
        describe: "file(s) to attach to message",
      })
      .option("daemon", {
        type: "boolean",
        describe: "run as a daemon that continues until host stop checks pass",
        default: false,
      })
      .option("daemonFile", {
        alias: ["daemon-file"],
        type: "string",
        describe: "ZYAL file to use when --daemon is enabled",
      })
      .option("daemonArm", {
        alias: ["daemon-arm"],
        type: "string",
        describe: "required ZYAL arm sentinel for daemon mode",
        default: "RUN_FOREVER",
      })
      .option("title", {
        type: "string",
        describe: "title for the session (uses truncated prompt if no value provided)",
      })
      .option("output-last-message", {
        type: "string",
        describe: "write the final assistant message text to a file",
      })
      .option("attach", {
        type: "string",
        describe: "attach to a running jekko server (e.g., http://localhost:4096)",
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
      })
      .option("dir", {
        type: "string",
        describe: "directory to run in, path on remote server if attaching",
      })
      .option("port", {
        type: "number",
        describe: "port for the local server (defaults to random port if no value provided)",
      })
      .option("variant", {
        type: "string",
        describe: "model variant (provider-specific reasoning effort, e.g., high, max, minimal)",
      })
      .option("thinking", {
        type: "boolean",
        describe: "show thinking blocks",
        default: false,
      })
      .option("dangerously-skip-permissions", {
        type: "boolean",
        describe: "auto-approve permissions that are not explicitly denied (dangerous!)",
        default: false,
      }),
  handler: Effect.fn("Cli.run")(function* (args) {
    const agentSvc = yield* Agent.Service
    yield* Effect.promise(async () => {
      const outputLastMessage = args["output-last-message"]
      if (outputLastMessage) {
        await Filesystem.write(outputLastMessage, "")
      }

      let message = [...args.message, ...(args["--"] || [])]
        .map((arg) => (arg.includes(" ") ? `"${arg.replace(/"/g, '\\"')}"` : arg))
        .join(" ")

      const directoryResolution = resolveRunDirectory(args)
      if (directoryResolution.kind === "error") {
        UI.error(directoryResolution.message)
        process.exit(1)
      }
      const directory = directoryResolution.kind === "unset" ? undefined : directoryResolution.directory

      const files: { type: "file"; url: string; filename: string; mime: string }[] = []
      if (args.file) {
        const list = Array.isArray(args.file) ? args.file : [args.file]

        for (const filePath of list) {
          const resolvedPath = path.resolve(process.cwd(), filePath)
          if (!(await Filesystem.exists(resolvedPath))) {
            UI.error(`File not found: ${filePath}`)
            process.exit(1)
          }

          const mime = (await Filesystem.isDir(resolvedPath)) ? "application/x-directory" : "text/plain"

          files.push({
            type: "file",
            url: pathToFileURL(resolvedPath).href,
            filename: path.basename(resolvedPath),
            mime,
          })
        }
      }

      if (!process.stdin.isTTY) message += "\n" + (await Bun.stdin.text())

      if (message.trim().length === 0 && !args.command) {
        UI.error("You must provide a message or a command")
        process.exit(1)
      }

      if (args.fork && !args.continue && !args.session) {
        UI.error("--fork requires --continue or --session")
        process.exit(1)
      }

      const rules: Permission.Ruleset = [
        {
          permission: "question",
          action: "deny",
          pattern: "*",
        },
        {
          permission: "plan_enter",
          action: "deny",
          pattern: "*",
        },
        {
          permission: "plan_exit",
          action: "deny",
          pattern: "*",
        },
      ]

      function title() {
        if (args.title === undefined) return
        if (args.title !== "") return args.title
        return message.slice(0, 50) + (message.length > 50 ? "..." : "")
      }

      async function session(sdk: OpencodeClient) {
        const baseID = args.continue ? (await sdk.session.list()).data?.find((s) => !s.parentID)?.id : args.session

        if (baseID && args.fork) {
          const forked = await sdk.session.fork({ sessionID: baseID })
          return forked.data?.id
        }

        if (baseID) return baseID

        const name = title()
        const result = await sdk.session.create({ title: name, permission: rules })
        return result.data?.id
      }

      async function share(sdk: OpencodeClient, sessionID: string) {
        const cfg = await sdk.config.get()
        if (!cfg.data) return
        if (cfg.data.share !== "auto" && !Flag.JEKKO_AUTO_SHARE && !args.share) return
        const res = await sdk.session.share({ sessionID }).catch((error) => {
          if (error instanceof Error && error.message.includes("disabled")) {
            UI.println(UI.Style.TEXT_DANGER_BOLD + "!  " + error.message)
          }
          return { error }
        })
        if (!res.error && "data" in res && res.data?.share?.url) {
          UI.println(UI.Style.TEXT_INFO_BOLD + "~  " + res.data.share.url)
        }
      }

      async function execute(sdk: OpencodeClient) {
        function tool(part: ToolPart) {
          try {
            if (part.tool === ShellID.ToolID) return shell(props(part, Schema.decodeUnknownSync(ShellParameters)))
            if (part.tool === "glob") return glob(props(part, Schema.decodeUnknownSync(GlobParameters), isGlobMetadata))
            if (part.tool === "grep") return grep(props(part, Schema.decodeUnknownSync(GrepParameters), isGrepMetadata))
            if (part.tool === "read") return read(props(part, Schema.decodeUnknownSync(ReadParameters)))
            if (part.tool === "write") return write(props(part, Schema.decodeUnknownSync(WriteParameters)))
            if (part.tool === "webfetch") return webfetch(props(part, Schema.decodeUnknownSync(WebFetchParameters)))
            if (part.tool === "edit") return edit(props(part, Schema.decodeUnknownSync(EditParameters), isEditMetadata))
            if (part.tool === "websearch") return websearch(props(part, Schema.decodeUnknownSync(WebSearchParameters)))
            if (part.tool === "task") return task(props(part, Schema.decodeUnknownSync(TaskParameters)))
            if (part.tool === PendingWriteTool.id) return pending(props(part, Schema.decodeUnknownSync(PendingParameters)))
            if (part.tool === "skill") return skill(props(part, Schema.decodeUnknownSync(SkillParameters)))
            return alternative_path(part)
          } catch {
            return alternative_path(part)
          }
        }

        function emit(type: string, data: Record<string, unknown>) {
          if (args.format === "json") {
            process.stdout.write(JSON.stringify({ type, timestamp: Date.now(), sessionID, ...data }) + EOL)
            return true
          }
          return false
        }

        const events = await sdk.event.subscribe()
        let error: string | undefined

        async function loop() {
          const toggles = new Map<string, boolean>()

          for await (const event of events.stream) {
            tracker.observe(event)

            if (
              event.type === "message.updated" &&
              event.properties.info.role === "assistant" &&
              args.format !== "json" &&
              toggles.get("start") !== true
            ) {
              UI.empty()
              UI.println(`> ${event.properties.info.agent} · ${event.properties.info.modelID}`)
              UI.empty()
              toggles.set("start", true)
            }

            if (event.type === "message.part.updated") {
              const part = event.properties.part
              if (part.sessionID !== sessionID) continue

              if (part.type === "tool" && (part.state.status === "completed" || part.state.status === "error")) {
                if (emit("tool_use", { part })) continue
                if (part.state.status === "completed") {
                  tool(part)
                  continue
                }
                inline({
                  icon: "✗",
                  title: `${part.tool} failed`,
                })
                UI.error(part.state.error)
              }

              if (
                part.type === "tool" &&
                part.tool === "task" &&
                part.state.status === "running" &&
                args.format !== "json"
              ) {
                if (toggles.get(part.id) === true) continue
                task(props(part, Schema.decodeUnknownSync(TaskParameters)))
                toggles.set(part.id, true)
              }

              if (part.type === "step-start") {
                if (emit("step_start", { part })) continue
              }

              if (part.type === "step-finish") {
                if (emit("step_finish", { part })) continue
              }

              if (part.type === "text" && part.time?.end) {
                if (emit("text", { part })) continue
                const text = part.text.trim()
                if (!text) continue
                if (!process.stdout.isTTY) {
                  process.stdout.write(text + EOL)
                  continue
                }
                UI.empty()
                UI.println(text)
                UI.empty()
              }

              if (part.type === "reasoning" && part.time?.end && args.thinking) {
                if (emit("reasoning", { part })) continue
                const text = part.text.trim()
                if (!text) continue
                const line = `Thinking: ${text}`
                if (process.stdout.isTTY) {
                  UI.empty()
                  UI.println(`${UI.Style.TEXT_DIM}\u001b[3m${line}\u001b[0m${UI.Style.TEXT_NORMAL}`)
                  UI.empty()
                  continue
                }
                process.stdout.write(line + EOL)
              }
            }

            if (event.type === "session.error") {
              const props = event.properties
              if (props.sessionID !== sessionID || !props.error) continue
              let err = String(props.error.name)
              if ("data" in props.error && props.error.data && "message" in props.error.data) {
                err = String(props.error.data.message)
              }
              error = error ? error + EOL + err : err
              if (emit("error", { error: props.error })) continue
              UI.error(err)
            }

            if (
              event.type === "session.status" &&
              event.properties.sessionID === sessionID &&
              event.properties.status.type === "idle"
            ) {
              if (outputLastMessage) {
                await tracker.write(outputLastMessage)
              }
              break
            }

            if (event.type === "permission.asked") {
              const permission = event.properties
              if (permission.sessionID !== sessionID) continue

              if (args["dangerously-skip-permissions"]) {
                await sdk.permission.reply({
                  requestID: permission.id,
                  reply: "always",
                })
              } else {
                UI.println(
                  UI.Style.TEXT_WARNING_BOLD + "!",
                  UI.Style.TEXT_NORMAL +
                    `permission requested: ${permission.permission} (${permission.patterns.join(", ")}); auto-rejecting (set JEKKO_AUTO_ALLOW_READS=1 for read-only auto-allow, configure an explicit allow rule for this permission, or pass --dangerously-skip-permissions to bypass the gate)`,
                )
                await sdk.permission.reply({
                  requestID: permission.id,
                  reply: "reject",
                })
              }
            }
          }
        }

        // Validate agent if specified
        const agentResolution = await resolveRunAgent(args, sdk, agentSvc)
        if (agentResolution.kind === "default") {
          if (agentResolution.reason !== "not-requested") {
            const reason =
              agentResolution.reason === "remote-list-failed"
                ? `failed to list agents from ${args.attach}`
                : agentResolution.reason === "missing"
                  ? `agent "${args.agent}" not found`
                  : `agent "${args.agent}" is a subagent, not a primary agent`
            UI.println(
              UI.Style.TEXT_WARNING_BOLD + "!",
              UI.Style.TEXT_NORMAL,
              `${reason}. Using the default agent`,
            )
          }
        }
        const agent = agentResolution.kind === "selected" ? agentResolution.agent : undefined

        const sessionID = await session(sdk)
        if (!sessionID) {
          UI.error("Session not found")
          process.exit(1)
        }
        const tracker = createLastAssistantMessageTracker({ sessionID })
        await share(sdk, sessionID)

        if (args.daemon) {
          const daemonFile = args.daemonFile ?? args.file?.[0]
          if (!daemonFile) {
            UI.error("--daemon requires --daemon-file (or --file)")
            process.exit(1)
          }
          if (args.daemonArm !== "RUN_FOREVER") {
            UI.error(`Unsupported daemon arm: ${args.daemonArm}`)
            process.exit(1)
          }

          const resolved = path.resolve(process.cwd(), daemonFile)
          const daemonText = await Bun.file(resolved).text()
          const client = (sdk as any)._client
          const preview = await client.request({
            method: "POST",
            url: "/daemon/preview",
            body: JSON.stringify({ text: daemonText }),
            headers: { "Content-Type": "application/json" },
          })
          if (preview.error) {
            UI.error(JSON.stringify(preview.error))
            process.exit(1)
          }

          const started = await client.request({
            method: "POST",
            url: `/session/${sessionID}/daemon/start`,
            body: JSON.stringify({ parts: [{ type: "text", text: daemonText }] }),
            headers: { "Content-Type": "application/json" },
          })
          if (started.error || !started.data) {
            UI.error(JSON.stringify(started.error ?? "failed to start daemon"))
            process.exit(1)
          }

          const runID = started.data.id
          while (true) {
            const poll = await client.request({
              method: "GET",
              url: `/daemon/${runID}`,
            })
            if (poll.error || !poll.data) {
              UI.error(JSON.stringify(poll.error ?? "failed to poll daemon"))
              process.exit(1)
            }
            const run = poll.data as { status?: string; phase?: string; iteration?: number; last_error?: string | null }
            UI.println(
              `∞ ${run.status ?? "running"} · ${run.phase ?? "running"} · iter ${run.iteration ?? 0}${run.last_error ? ` · ${run.last_error}` : ""}`,
            )
            if (["satisfied", "aborted", "failed"].includes(String(run.status))) break
            await new Promise((resolve) => setTimeout(resolve, 1000))
          }
          return
        }

        loop().catch((e) => {
          console.error(e)
          process.exit(1)
        })

        if (args.command) {
          await sdk.session.command({
            sessionID,
            agent,
            model: args.model,
            command: args.command,
            arguments: message,
            variant: args.variant,
          })
        } else {
          const model = args.model ? Provider.parseModel(args.model) : undefined
          await sdk.session.prompt({
            sessionID,
            agent,
            model,
            variant: args.variant,
            parts: [...files, { type: "text", text: message }],
          })
        }
      }

      if (args.attach) {
        const headers = ServerAuth.headers({ password: args.password, username: args.username })
        const sdk = createOpencodeClient({ baseUrl: args.attach, directory, headers })
        return await execute(sdk)
      }

      const fetchFn = (async (input: RequestInfo | URL, init?: RequestInit) => {
        const request = new Request(input, init)
        return Server.Default().app.fetch(request)
      }) as typeof globalThis.fetch
      const sdk = createOpencodeClient({ baseUrl: "http://jekko.internal", fetch: fetchFn })
      await execute(sdk)
    })
  }),
})
