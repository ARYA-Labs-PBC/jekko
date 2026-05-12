import { cmd } from "../cmd"
import { tui } from "./app"
import { TuiConfig } from "@/cli/cmd/tui/config/tui"
import { ServerAuth } from "@/server/auth"
import { runValidatedTuiCommandWithConfig } from "./command-guards"

type AttachDirectoryResolution =
  | { kind: "local"; directory: string }
  | { kind: "remote"; directory: string }

export function resolveAttachDirectory(args: { dir?: string }) {
  if (!args.dir) return { kind: "local", directory: process.cwd() } as const
  try {
    process.chdir(args.dir)
    return { kind: "local", directory: process.cwd() } as const
  } catch {
    return { kind: "remote", directory: args.dir } as const
  }
}

export const AttachCommand = cmd({
  command: "attach <url>",
  describe: "attach to a running jekko server",
  builder: (yargs) =>
    yargs
      .positional("url", {
        type: "string",
        describe: "http://localhost:4096",
        demandOption: true,
      })
      .option("dir", {
        type: "string",
        description: "directory to run in",
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
  handler: async (args) => {
    const directory = resolveAttachDirectory(args).directory
    const headers = ServerAuth.headers({ password: args.password, username: args.username })

    await runValidatedTuiCommandWithConfig(
      args,
      {
        url: args.url,
        sessionID: args.session,
        directory,
        headers,
      },
      async () => TuiConfig.get(),
      async (config) => {
        await tui({
          url: args.url,
          config,
          args: {
            continue: args.continue,
            sessionID: args.session,
            fork: args.fork,
          },
          directory,
          headers,
        })
      },
    )
  },
})
