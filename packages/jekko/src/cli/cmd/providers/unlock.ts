import { EOL } from "os"
import { effectCmd, fail } from "../../effect-cmd"
import { UI } from "../../ui"
import { findRepoRootFrom, repoRootFromSource, unlockJnoccioFusion } from "@/util/jnoccio-unlock"
import { Effect } from "effect"

export const ProvidersUnlockCommand = effectCmd({
  command: "unlock <provider>",
  describe: "unlock a provider",
  instance: false,
  builder: (yargs) =>
    yargs
      .positional("provider", {
        describe: "provider to unlock",
        type: "string",
        demandOption: true,
      })
      .option("repo", {
        type: "string",
        describe: "repository path to inspect",
      })
      .option("key-file", {
        type: "string",
        describe: "git-crypt key file path",
      })
      .option("secret-file", {
        type: "string",
        describe: "unlock secret cache file path",
      })
      .option("json", {
        type: "boolean",
        describe: "print JSON only",
        default: false,
      }),
  handler: Effect.fn("Cli.providers.unlock")(function* (args) {
    const provider = args.provider.trim().toLowerCase()
    if (provider !== "jnoccio") {
      return yield* fail(`Unsupported provider "${args.provider}". Only jnoccio is supported.`)
    }

    const repoRoot = findRepoRootFrom(args.repo) ?? repoRootFromSource()
    const result = yield* Effect.promise(() =>
      unlockJnoccioFusion(
        {
          keyPath: args.keyFile,
        },
        {
          repoRoot,
          secretPath: args.secretFile,
        },
      ),
    )

    if (args.json) {
      process.stdout.write(JSON.stringify(result) + EOL)
    } else {
      UI.println(result.message)
    }

    if (result.status === "unlocked") {
      return
    }

    process.exitCode = result.status === "needs_secret" ? 2 : 1
  }),
})
