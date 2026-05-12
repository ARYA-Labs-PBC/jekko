import semver from "semver"
import { useDialog } from "@tui/ui/dialog"
import { DialogAlert } from "./ui/dialog-alert"
import { DialogConfirm } from "./ui/dialog-confirm"
import { useCommandDialog } from "@tui/component/dialog-command"
import { useEvent } from "@tui/context/event"
import { useKV } from "./context/kv"
import { useRoute } from "@tui/context/route"
import { useSDK } from "@tui/context/sdk"
import { useToast } from "./ui/toast"
import { useExit } from "./context/exit"
import { FormatError, FormatUnknownError } from "@/cli/error"
import { TuiEvent } from "./event"

type Input = {
  command: ReturnType<typeof useCommandDialog>
  event: ReturnType<typeof useEvent>
  route: ReturnType<typeof useRoute>
  toast: ReturnType<typeof useToast>
  dialog: ReturnType<typeof useDialog>
  kv: ReturnType<typeof useKV>
  sdk: ReturnType<typeof useSDK>
  exit: ReturnType<typeof useExit>
}

function errorMessage(error: unknown): string {
  const formatted = FormatError(error) as string | undefined
  if (formatted !== undefined) return formatted
  if (
    typeof error === "object" &&
    error !== null &&
    "data" in error &&
    typeof error.data === "object" &&
    error.data !== null &&
    "message" in error.data &&
    typeof error.data.message === "string"
  ) {
    return error.data.message
  }
  return FormatUnknownError(error)
}

export function registerTuiEvents(input: Input) {
  input.event.on(TuiEvent.CommandExecute.type, (evt) => {
    input.command.trigger(evt.properties.command)
  })

  input.event.on(TuiEvent.ToastShow.type, (evt) => {
    input.toast.show({
      title: evt.properties.title,
      message: evt.properties.message,
      variant: evt.properties.variant,
      duration: evt.properties.duration,
    })
  })

  input.event.on(TuiEvent.SessionSelect.type, (evt) => {
    input.route.navigate({
      type: "session",
      sessionID: evt.properties.sessionID,
    })
  })

  input.event.on("session.deleted", (evt) => {
    if (input.route.data.type === "session" && input.route.data.sessionID === evt.properties.info.id) {
      input.route.navigate({ type: "home" })
      input.toast.show({
        variant: "info",
        message: "The current session was deleted",
      })
    }
  })

  input.event.on("session.error", (evt) => {
    const error = evt.properties.error
    if (error && typeof error === "object" && error.name === "MessageAbortedError") return
    const message = errorMessage(error)

    input.toast.show({
      variant: "error",
      message,
      duration: 5000,
    })
  })

  input.event.on("installation.update-available", async (evt) => {
    const version = evt.properties.version

    const skipped = input.kv.get("skipped_version")
    if (skipped && !semver.gt(version, skipped)) return

    const choice = await DialogConfirm.show(
      input.dialog,
      "Update Available",
      `A new release v${version} is available. Would you like to update now?`,
      "skip",
    )

    if (choice === false) {
      input.kv.set("skipped_version", version)
      return
    }

    if (choice !== true) return

    input.toast.show({
      variant: "info",
      message: `Updating to v${version}...`,
      duration: 30000,
    })

    const result = await input.sdk.client.global.upgrade({ target: version })

    if (result.error || !result.data?.success) {
      input.toast.show({
        variant: "error",
        title: "Update Failed",
        message: "Update failed",
        duration: 10000,
      })
      return
    }

    await DialogAlert.show(
      input.dialog,
      "Update Complete",
      `Successfully updated to Jekko v${result.data.version}. Please restart the application.`,
    )

    void input.exit()
  })
}
