import { Effect } from "effect"
import { effectCmd } from "../effect-cmd"
import { UI } from "../ui"
import {
  ensureModelKeysFile,
  providerKeyStatusSummary,
  readModelKeyStatuses,
  resolveModelKeysPath,
} from "@/model-setup/model-keys"

export const KeysCommand = effectCmd({
  command: "keys <action>",
  describe: "manage canonical model keys",
  instance: false,
  builder: (yargs) =>
    yargs
      .positional("action", {
        choices: ["path", "init", "status"],
        type: "string",
      })
      .option("json", {
        describe: "print machine-readable status",
        type: "boolean",
      }),
  handler: Effect.fn("Cli.keys")(function* (args) {
    switch (args.action) {
      case "path": {
        UI.println(resolveModelKeysPath())
        return
      }
      case "init": {
        const result = yield* Effect.promise(() => ensureModelKeysFile())
        UI.println(result.path)
        return
      }
      case "status": {
        const status = yield* Effect.promise(() => readModelKeyStatuses())
        const publicStatus = {
          path: status.path,
          created: status.created,
          developerUnlocked: status.developerUnlocked,
          activeProviderID: status.activeProviderID,
          statuses: providerKeyStatusSummary(status.statuses),
        }
        if (args.json) {
          UI.println(JSON.stringify(publicStatus, null, 2))
          return
        }
        UI.println(`Path: ${publicStatus.path}`)
        UI.println(`Developer unlock: ${publicStatus.developerUnlocked ? "yes" : "no"}`)
        for (const item of publicStatus.statuses) {
          UI.println(
            [
              item.providerID,
              item.active ? "active" : "inactive",
              item.configured ? item.redacted ?? "present" : "blank",
              item.inactiveReason ? `(${item.inactiveReason})` : "",
            ]
              .filter(Boolean)
              .join(" "),
          )
        }
        return
      }
    }
  }),
})
