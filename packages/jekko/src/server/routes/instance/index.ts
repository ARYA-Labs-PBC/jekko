import { describeRoute, resolver, validator } from "hono-openapi"
import { Hono, type Context as HonoContext } from "hono"
import type { UpgradeWebSocket } from "hono/ws"
import { Context, Effect } from "effect"
import { Flag } from "@jekko-ai/core/flag/flag"
import z from "zod"
import { Format } from "@/format"
import { TuiRoutes } from "./tui"
import { Instance } from "@/project/instance"
import { InstanceRuntime } from "@/project/instance-runtime"
import { Vcs } from "@/project/vcs"
import { Agent } from "@/agent/agent"
import { Skill } from "@/skill"
import { Global } from "@jekko-ai/core/global"
import { LSP } from "@/lsp/lsp"
import { Command } from "@/command"
import { QuestionRoutes } from "./question"
import { PermissionRoutes } from "./permission"
import { ProjectRoutes } from "./project"
import { SessionRoutes } from "./session"
import { PtyRoutes } from "./pty"
import { McpRoutes } from "./mcp"
import { FileRoutes } from "./file"
import { ConfigRoutes } from "./config"
import { ExperimentalRoutes } from "./experimental"
import { ProviderRoutes } from "./provider"
import { EventRoutes } from "./event"
import { SyncRoutes } from "./sync"
import { InstanceMiddleware } from "./middleware"
import { jsonRequest } from "./trace"
import { ExperimentalHttpApiServer } from "./httpapi/server"
import { DaemonPaths } from "./httpapi/groups/daemon"
import { EventPaths } from "./httpapi/event"
import { ExperimentalPaths } from "./httpapi/groups/experimental"
import { FilePaths } from "./httpapi/groups/file"
import { InstancePaths } from "./httpapi/groups/instance"
import { McpPaths } from "./httpapi/groups/mcp"
import { PtyPaths } from "./httpapi/groups/pty"
import { SessionPaths } from "./httpapi/groups/session"
import { SyncPaths } from "./httpapi/groups/sync"
import { TuiPaths } from "./httpapi/groups/tui"
import { WorkspacePaths } from "./httpapi/groups/workspace"
import type { CorsOptions } from "@/server/cors"

export const InstanceRoutes = (upgrade: UpgradeWebSocket, opts?: CorsOptions): Hono => {
  const app = new Hono()
  const handler = ExperimentalHttpApiServer.webHandler(opts).handler
  const context = Context.empty() as Context.Context<unknown>
  const httpApiHandler = (c: HonoContext) => handler(c.req.raw, context)

  app.all("/api/*", httpApiHandler)
  app.post(DaemonPaths.preview, httpApiHandler)
  app.get(DaemonPaths.list, httpApiHandler)
  app.get(DaemonPaths.get, httpApiHandler)
  app.get(DaemonPaths.events, httpApiHandler)
  app.post(DaemonPaths.pause, httpApiHandler)
  app.post(DaemonPaths.resume, httpApiHandler)
  app.post(DaemonPaths.abort, httpApiHandler)
  app.post(DaemonPaths.compact, httpApiHandler)
  app.post(DaemonPaths.rotateSession, httpApiHandler)
  app.post(DaemonPaths.start, httpApiHandler)
  app.get(DaemonPaths.tasks, httpApiHandler)
  app.get(DaemonPaths.task, httpApiHandler)
  app.get(DaemonPaths.taskPasses, httpApiHandler)
  app.get(DaemonPaths.taskMemory, httpApiHandler)
  app.post(DaemonPaths.incubateTask, httpApiHandler)
  app.post(DaemonPaths.promoteTask, httpApiHandler)
  app.post(DaemonPaths.blockTask, httpApiHandler)
  app.post(DaemonPaths.archiveTask, httpApiHandler)
  app.get(DaemonPaths.incubator, httpApiHandler)

  if (Flag.JEKKO_EXPERIMENTAL_HTTPAPI) {
    app.get(EventPaths.event, httpApiHandler)
    app.get("/question", httpApiHandler)
    app.post("/question/:requestID/reply", httpApiHandler)
    app.post("/question/:requestID/reject", httpApiHandler)
    app.get("/permission", httpApiHandler)
    app.post("/permission/:requestID/reply", httpApiHandler)
    app.get("/config", httpApiHandler)
    app.patch("/config", httpApiHandler)
    app.get("/config/providers", httpApiHandler)
    app.get(ExperimentalPaths.console, httpApiHandler)
    app.get(ExperimentalPaths.consoleOrgs, httpApiHandler)
    app.post(ExperimentalPaths.consoleSwitch, httpApiHandler)
    app.get(ExperimentalPaths.tool, httpApiHandler)
    app.get(ExperimentalPaths.toolIDs, httpApiHandler)
    app.get(ExperimentalPaths.worktree, httpApiHandler)
    app.post(ExperimentalPaths.worktree, httpApiHandler)
    app.delete(ExperimentalPaths.worktree, httpApiHandler)
    app.post(ExperimentalPaths.worktreeReset, httpApiHandler)
    app.get(ExperimentalPaths.session, httpApiHandler)
    app.get(ExperimentalPaths.resource, httpApiHandler)
    app.get("/provider", httpApiHandler)
    app.get("/provider/auth", httpApiHandler)
    app.post("/provider/jnoccio/unlock", httpApiHandler)
    app.post("/provider/:providerID/oauth/authorize", httpApiHandler)
    app.post("/provider/:providerID/oauth/callback", httpApiHandler)
    app.get("/project", httpApiHandler)
    app.get("/project/current", httpApiHandler)
    app.post("/project/git/init", httpApiHandler)
    app.patch("/project/:projectID", httpApiHandler)
    app.get(FilePaths.findText, httpApiHandler)
    app.get(FilePaths.findFile, httpApiHandler)
    app.get(FilePaths.findSymbol, httpApiHandler)
    app.get(FilePaths.list, httpApiHandler)
    app.get(FilePaths.content, httpApiHandler)
    app.get(FilePaths.status, httpApiHandler)
    app.get(InstancePaths.path, httpApiHandler)
    app.post(InstancePaths.dispose, httpApiHandler)
    app.get(InstancePaths.vcs, httpApiHandler)
    app.get(InstancePaths.vcsDiff, httpApiHandler)
    app.get(InstancePaths.command, httpApiHandler)
    app.get(InstancePaths.agent, httpApiHandler)
    app.get(InstancePaths.skill, httpApiHandler)
    app.get(InstancePaths.lsp, httpApiHandler)
    app.get(InstancePaths.formatter, httpApiHandler)
    app.get(McpPaths.status, httpApiHandler)
    app.post(McpPaths.status, httpApiHandler)
    app.post(McpPaths.auth, httpApiHandler)
    app.post(McpPaths.authCallback, httpApiHandler)
    app.post(McpPaths.authAuthenticate, httpApiHandler)
    app.delete(McpPaths.auth, httpApiHandler)
    app.post(McpPaths.connect, httpApiHandler)
    app.post(McpPaths.disconnect, httpApiHandler)
    app.post(SyncPaths.start, httpApiHandler)
    app.post(SyncPaths.replay, httpApiHandler)
    app.post(SyncPaths.history, httpApiHandler)
    app.get(PtyPaths.list, httpApiHandler)
    app.post(PtyPaths.create, httpApiHandler)
    app.get(PtyPaths.get, httpApiHandler)
    app.put(PtyPaths.update, httpApiHandler)
    app.delete(PtyPaths.remove, httpApiHandler)
    app.post(PtyPaths.connectToken, httpApiHandler)
    app.get(PtyPaths.connect, httpApiHandler)
    app.get(SessionPaths.list, httpApiHandler)
    app.get(SessionPaths.status, httpApiHandler)
    app.get(SessionPaths.get, httpApiHandler)
    app.get(SessionPaths.children, httpApiHandler)
    app.get(SessionPaths.pending, httpApiHandler)
    app.get(SessionPaths.task, httpApiHandler)
    app.get(SessionPaths.diff, httpApiHandler)
    app.get(SessionPaths.messages, httpApiHandler)
    app.get(SessionPaths.message, httpApiHandler)
    app.post(SessionPaths.create, httpApiHandler)
    app.delete(SessionPaths.remove, httpApiHandler)
    app.patch(SessionPaths.update, httpApiHandler)
    app.post(SessionPaths.init, httpApiHandler)
    app.post(SessionPaths.fork, httpApiHandler)
    app.post(SessionPaths.abort, httpApiHandler)
    app.post(SessionPaths.share, httpApiHandler)
    app.delete(SessionPaths.share, httpApiHandler)
    app.post(SessionPaths.summarize, httpApiHandler)
    app.post(SessionPaths.prompt, httpApiHandler)
    app.post(SessionPaths.promptAsync, httpApiHandler)
    app.post(SessionPaths.command, httpApiHandler)
    app.post(SessionPaths.shell, httpApiHandler)
    app.post(SessionPaths.revert, httpApiHandler)
    app.post(SessionPaths.unrevert, httpApiHandler)
    app.post(SessionPaths.permissions, httpApiHandler)
    app.delete(SessionPaths.deleteMessage, httpApiHandler)
    app.delete(SessionPaths.deletePart, httpApiHandler)
    app.patch(SessionPaths.updatePart, httpApiHandler)
    app.post(TuiPaths.appendPrompt, httpApiHandler)
    app.post(TuiPaths.openHelp, httpApiHandler)
    app.post(TuiPaths.openSessions, httpApiHandler)
    app.post(TuiPaths.openThemes, httpApiHandler)
    app.post(TuiPaths.openModels, httpApiHandler)
    app.post(TuiPaths.submitPrompt, httpApiHandler)
    app.post(TuiPaths.clearPrompt, httpApiHandler)
    app.post(TuiPaths.executeCommand, httpApiHandler)
    app.post(TuiPaths.showToast, httpApiHandler)
    app.post(TuiPaths.publish, httpApiHandler)
    app.post(TuiPaths.selectSession, httpApiHandler)
    app.get(TuiPaths.controlNext, httpApiHandler)
    app.post(TuiPaths.controlResponse, httpApiHandler)
    app.get(WorkspacePaths.adapters, httpApiHandler)
    app.post(WorkspacePaths.list, httpApiHandler)
    app.get(WorkspacePaths.list, httpApiHandler)
    app.get(WorkspacePaths.status, httpApiHandler)
    app.delete(WorkspacePaths.remove, httpApiHandler)
    app.post(WorkspacePaths.warp, httpApiHandler)
  }

  return app
    .route("/project", ProjectRoutes())
    .route("/pty", PtyRoutes(upgrade, opts))
    .route("/config", ConfigRoutes())
    .route("/experimental", ExperimentalRoutes())
    .route("/session", SessionRoutes())
    .route("/permission", PermissionRoutes())
    .route("/question", QuestionRoutes())
    .route("/provider", ProviderRoutes())
    .route("/sync", SyncRoutes())
    .route("/", FileRoutes())
    .route("/", EventRoutes())
    .route("/mcp", McpRoutes())
    .route("/tui", TuiRoutes())
    .post(
      "/instance/dispose",
      describeRoute({
        summary: "Dispose instance",
        description: "Clean up and dispose the current Jekko instance, releasing all resources.",
        operationId: "instance.dispose",
        responses: {
          200: {
            description: "Instance disposed",
            content: {
              "application/json": {
                schema: resolver(z.boolean()),
              },
            },
          },
        },
      }),
      async (c) => {
        await InstanceRuntime.disposeInstance(Instance.current)
        return c.json(true)
      },
    )
    .get(
      "/path",
      describeRoute({
        summary: "Get paths",
        description: "Retrieve the current working directory and related path information for the Jekko instance.",
        operationId: "path.get",
        responses: {
          200: {
            description: "Path",
            content: {
              "application/json": {
                schema: resolver(
                  z
                    .object({
                      home: z.string(),
                      state: z.string(),
                      config: z.string(),
                      worktree: z.string(),
                      directory: z.string(),
                    })
                    .meta({
                      ref: "Path",
                    }),
                ),
              },
            },
          },
        },
      }),
      async (c) => {
        return c.json({
          home: Global.Path.home,
          state: Global.Path.state,
          config: Global.Path.config,
          worktree: Instance.worktree,
          directory: Instance.directory,
        })
      },
    )
    .get(
      "/vcs",
      describeRoute({
        summary: "Get VCS info",
        description: "Retrieve version control system (VCS) information for the current project, such as git branch.",
        operationId: "vcs.get",
        responses: {
          200: {
            description: "VCS info",
            content: {
              "application/json": {
                schema: resolver(Vcs.Info.zod),
              },
            },
          },
        },
      }),
      async (c) =>
        jsonRequest("InstanceRoutes.vcs.get", c, function* () {
          const vcs = yield* Vcs.Service
          const [branch, default_branch] = yield* Effect.all([vcs.branch(), vcs.defaultBranch()], {
            concurrency: 2,
          })
          return { branch, default_branch }
        }),
    )
    .get(
      "/vcs/diff",
      describeRoute({
        summary: "Get VCS diff",
        description: "Retrieve the current git diff for the working tree or against the default branch.",
        operationId: "vcs.diff",
        responses: {
          200: {
            description: "VCS diff",
            content: {
              "application/json": {
                schema: resolver(Vcs.FileDiff.zod.array()),
              },
            },
          },
        },
      }),
      validator(
        "query",
        z.object({
          mode: Vcs.Mode.zod,
        }),
      ),
      async (c) =>
        jsonRequest("InstanceRoutes.vcs.diff", c, function* () {
          const vcs = yield* Vcs.Service
          return yield* vcs.diff(c.req.valid("query").mode)
        }),
    )
    .get(
      "/command",
      describeRoute({
        summary: "List commands",
        description: "Get a list of all available commands in the Jekko system.",
        operationId: "command.list",
        responses: {
          200: {
            description: "List of commands",
            content: {
              "application/json": {
                schema: resolver(Command.Info.zod.array()),
              },
            },
          },
        },
      }),
      async (c) =>
        jsonRequest("InstanceRoutes.command.list", c, function* () {
          const svc = yield* Command.Service
          return yield* svc.list()
        }),
    )
    .get(
      "/agent",
      describeRoute({
        summary: "List agents",
        description: "Get a list of all available AI agents in the Jekko system.",
        operationId: "app.agents",
        responses: {
          200: {
            description: "List of agents",
            content: {
              "application/json": {
                schema: resolver(Agent.Info.zod.array()),
              },
            },
          },
        },
      }),
      async (c) =>
        jsonRequest("InstanceRoutes.agent.list", c, function* () {
          const svc = yield* Agent.Service
          return yield* svc.list()
        }),
    )
    .get(
      "/skill",
      describeRoute({
        summary: "List skills",
        description: "Get a list of all available skills in the Jekko system.",
        operationId: "app.skills",
        responses: {
          200: {
            description: "List of skills",
            content: {
              "application/json": {
                schema: resolver(Skill.Info.zod.array()),
              },
            },
          },
        },
      }),
      async (c) =>
        jsonRequest("InstanceRoutes.skill.list", c, function* () {
          const skill = yield* Skill.Service
          return yield* skill.all()
        }),
    )
    .get(
      "/lsp",
      describeRoute({
        summary: "Get LSP status",
        description: "Get LSP server status",
        operationId: "lsp.status",
        responses: {
          200: {
            description: "LSP server status",
            content: {
              "application/json": {
                schema: resolver(LSP.Status.zod.array()),
              },
            },
          },
        },
      }),
      async (c) =>
        jsonRequest("InstanceRoutes.lsp.status", c, function* () {
          const lsp = yield* LSP.Service
          return yield* lsp.status()
        }),
    )
    .get(
      "/formatter",
      describeRoute({
        summary: "Get formatter status",
        description: "Get formatter status",
        operationId: "formatter.status",
        responses: {
          200: {
            description: "Formatter status",
            content: {
              "application/json": {
                schema: resolver(Format.Status.zod.array()),
              },
            },
          },
        },
      }),
      async (c) =>
        jsonRequest("InstanceRoutes.formatter.status", c, function* () {
          const svc = yield* Format.Service
          return yield* svc.status()
        }),
    )
}
