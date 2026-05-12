import { Auth } from "../../../auth"
import { ModelsDev } from "@/provider/models"
import { effectCmd } from "../../effect-cmd"
import { UI } from "../../ui"
import * as Prompt from "../../effect/prompt"
import { Effect } from "effect"
import { promptValue } from "./helpers"

export const ProvidersLogoutCommand = effectCmd({
  command: "logout",
  describe: "log out from a configured provider",
  // Removes a global auth credential; no project instance needed.
  instance: false,
  handler: Effect.fn("Cli.providers.logout")(function* (_args) {
    const authSvc = yield* Auth.Service
    const modelsDev = yield* ModelsDev.Service

    UI.empty()
    const credentials: Array<[string, Auth.Info]> = Object.entries(yield* Effect.orDie(authSvc.all()))
    yield* Prompt.intro("Remove credential")
    if (credentials.length === 0) {
      yield* Prompt.log.error("No credentials found")
      return
    }
    const database = yield* modelsDev.get()
    const selected = yield* Prompt.select({
      message: "Select provider",
      options: credentials.map(([key, value]) => ({
        label: (database[key]?.name || key) + UI.Style.TEXT_DIM + " (" + value.type + ")",
        value: key,
      })),
    })
    yield* Effect.orDie(authSvc.remove(yield* promptValue(selected)))
    yield* Prompt.outro("Logout successful")
  }),
})
