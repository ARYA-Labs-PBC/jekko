import { createEffect, onMount, Show } from "solid-js"
import { Logo } from "../component/logo"
import { useSync } from "../context/sync"
import { Toast } from "../ui/toast"
import { useArgs } from "../context/args"
import { useRoute } from "@tui/context/route"
import { useLocal } from "../context/local"
import { useKeybind } from "@tui/context/keybind"
import { TuiPluginRuntime } from "@/cli/cmd/tui/plugin/runtime"
import { useEditorContext } from "@tui/context/editor"
import { useTheme } from "@tui/context/theme"
import { FooterBand } from "@tui/component/footer-band"

export function Home() {
  const sync = useSync()
  const args = useArgs()
  const route = useRoute()
  const local = useLocal()
  const editor = useEditorContext()
  const keybind = useKeybind()
  const { theme } = useTheme()
  let engaged = false

  function engage() {
    if (engaged) return
    engaged = true
    route.navigate({ type: "shell" })
  }

  onMount(() => {
    editor.clearSelection()
  })

  // Auto-engage when the user launches with --prompt or --continue. The shell's
  // activity-feed picks up `args.prompt` and submits once sync is ready.
  createEffect(() => {
    if (engaged) return
    if (!sync.ready || !local.model.ready) return
    if (!args.prompt && !args.continue && !args.sessionID) return
    engage()
  })

  keybind.on("engage", () => engage())

  return (
    <>
      <box flexGrow={1} flexDirection="column" alignItems="center" justifyContent="center" paddingLeft={2} paddingRight={2}>
        <box flexGrow={1} minHeight={0} />
        <box flexShrink={0}>
          <TuiPluginRuntime.Slot name="home_logo" mode="replace">
            <Logo idle />
          </TuiPluginRuntime.Slot>
        </box>
        <box height={2} minHeight={0} flexShrink={0} />
        <Show when={sync.ready}>
          <text fg={theme.textMuted} selectable={false}>
            <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Enter </span> engage
            {"  "}
            <span style={{ bg: theme.backgroundElement, fg: theme.text }}> ? </span> help
            {"  "}
            <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Ctrl+P </span> commands
          </text>
        </Show>
        <Show when={!sync.ready}>
          <text fg={theme.textMuted} selectable={false}>
            connecting…
          </text>
        </Show>
        <TuiPluginRuntime.Slot name="home_bottom" />
        <box flexGrow={1} minHeight={0} />
        <Toast />
      </box>
      <FooterBand backgroundColor={theme.backgroundPanel} borderColor={theme.borderSubtle}>
        <TuiPluginRuntime.Slot name="home_footer" mode="single_winner" />
      </FooterBand>
    </>
  )
}
