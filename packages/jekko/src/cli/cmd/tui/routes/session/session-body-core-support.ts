import type { ScrollBoxRenderable, ParsedKey } from "@opentui/core"
import { createEffect } from "solid-js"
import type { PromptRef } from "@tui/component/prompt"
import { Locale } from "@/util/locale"
import { SessionRetry } from "@/session/retry"
import { DialogGoUpsell } from "../../component/dialog-go-upsell"
import { GO_UPSELL_DONT_SHOW, GO_UPSELL_LAST_SEEN_AT, GO_UPSELL_WINDOW, emptyPromptParts, scrollToMessage, toBottom } from "./session-helpers"
import { registerSessionCommands } from "./session-commands"
import { UI } from "@/cli/ui"
import { errorMessage } from "@/util/error"

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function recordField(value: unknown, key: string): unknown {
  return isRecord(value) ? value[key] : undefined
}

function stringField(value: unknown, key: string): string | undefined {
  const next = recordField(value, key)
  return typeof next === "string" && next.length > 0 ? next : undefined
}

export function bindSessionKeyboardExit(
  useKeyboard: (handler: (evt: ParsedKey) => void) => void,
  keybind: any,
  exit: () => void | Promise<void>,
  hasParentSession: () => boolean,
) {
  useKeyboard((evt: ParsedKey) => {
    if (!hasParentSession()) return
    if (keybind.match("app_exit", evt)) {
      void exit()
    }
  })
}

export function bindSessionScrollKeybinds(
  keybind: any,
  scrollRef: { current: ScrollBoxRenderable | undefined },
) {
  keybind.on("feed.scroll.pageUp", () => {
    if (!scrollRef.current) return
    scrollRef.current.scrollBy(-scrollRef.current.height)
  })
  keybind.on("feed.scroll.pageDown", () => {
    if (!scrollRef.current) return
    scrollRef.current.scrollBy(scrollRef.current.height)
  })
  keybind.on("feed.scroll.top", () => {
    scrollRef.current?.scrollTo(0)
  })
  keybind.on("feed.scroll.bottom", () => {
    scrollRef.current?.scrollTo(scrollRef.current.scrollHeight)
  })
}

export function bindSessionCommands(
  command: any,
  args: Record<string, unknown>,
) {
  registerSessionCommands(command, {
    ...args,
    toBottom,
    emptyPromptParts,
  })
}

export function bindSessionExitMessage(args: {
  exit: { message: { set: (value: string) => void } }
  session: () => { id?: string; title?: string } | undefined
}) {
  createEffect(() => {
    const title = Locale.truncate(args.session()?.title ?? "", 50)
    const dim = UI.Style.TEXT_DIM
    const reset = UI.Style.TEXT_NORMAL
    const bold = UI.Style.TEXT_NORMAL_BOLD
    args.exit.message.set([
      ``,
      `  ${dim}Session${reset}  ${bold}${title}${reset}`,
      `  ${dim}Resume${reset}   ${bold}jekko -s ${args.session()?.id}${reset}`,
      ``,
    ].join("\n"))
  })
}

export function bindSessionLoadEffect(args: {
  routeSessionID: () => string
  project: any
  sdk: any
  toast: any
  navigate: (value: any) => void
  sync: any
  editor: any
  scrollRef: { current: ScrollBoxRenderable | undefined }
}) {
  createEffect(() => {
    const sessionID = args.routeSessionID()
    void (async () => {
      const previousWorkspace = args.project.workspace.current()
      const result = await args.sdk.client.session.get({ sessionID }, { throwOnError: true })
      if (!result.data) {
        args.toast.show({
          message: `Session not found: ${sessionID}`,
          variant: "error",
          duration: 5000,
        })
        args.navigate({ type: "shell" })
        return
      }

      if (result.data.workspaceID !== previousWorkspace) {
        args.project.workspace.set(result.data.workspaceID)
        try {
          await args.sync.bootstrap({ fatal: false })
        } catch {}
      }
      args.editor.reconnect(result.data.directory)
      await args.sync.session.sync(sessionID)
      if (args.routeSessionID() === sessionID && args.scrollRef.current) {
        args.scrollRef.current.scrollBy(100_000)
      }
    })().catch((error) => {
      if (args.routeSessionID() !== sessionID) return
      args.toast.show({
        message: errorMessage(error),
        variant: "error",
        duration: 5000,
      })
      args.navigate({ type: "shell" })
    })
  })
}

export function bindSessionPartSwitchHandler(args: {
  event: any
  routeSessionID: () => string
  local: any
}) {
  let lastSwitch: string | undefined = undefined
  args.event.on("message.part.updated", (evt: unknown) => {
    const part = recordField(recordField(evt, "properties"), "part")
    if (!isRecord(part)) return
    if (stringField(part, "type") !== "tool") return
    if (stringField(part, "sessionID") !== args.routeSessionID()) return
    const state = recordField(part, "state")
    if (stringField(state, "status") !== "completed") return
    if (stringField(part, "id") === lastSwitch) return

    const tool = stringField(part, "tool")
    if (tool === "plan_exit") {
      args.local.agent.set("build")
      lastSwitch = stringField(part, "id")
    } else if (tool === "plan_enter") {
      args.local.agent.set("plan")
      lastSwitch = stringField(part, "id")
    }
  })
}

export function bindSessionUpsellHandler(args: {
  event: any
  routeSessionID: () => string
  kv: any
  dialog: any
}) {
  args.event.on("session.status", (evt: unknown) => {
    const properties = recordField(evt, "properties")
    if (stringField(properties, "sessionID") !== args.routeSessionID()) return
    const status = recordField(properties, "status")
    if (stringField(status, "type") !== "retry") return
    if (stringField(status, "message") !== SessionRetry.GO_UPSELL_MESSAGE) return
    if (args.dialog.stack.length > 0) return
    const seen = args.kv.get(GO_UPSELL_LAST_SEEN_AT)
    if (typeof seen === "number" && Date.now() - seen < GO_UPSELL_WINDOW) return
    if (args.kv.get(GO_UPSELL_DONT_SHOW)) return
    void DialogGoUpsell.show(args.dialog).then((dontShowAgain: boolean) => {
      if (dontShowAgain) args.kv.set(GO_UPSELL_DONT_SHOW, true)
      args.kv.set(GO_UPSELL_LAST_SEEN_AT, Date.now())
    })
  })
}
