import { Prompt, type PromptRef } from "@tui/component/prompt"
import { createEffect, createMemo, createSignal, onMount, Show } from "solid-js"
import { RGBA } from "@opentui/core"
import { useTerminalDimensions } from "@opentui/solid"
import { Logo } from "../component/logo"
import { useProject } from "../context/project"
import { useSync } from "../context/sync"
import { Toast } from "../ui/toast"
import { useArgs } from "../context/args"
import { useRouteData } from "@tui/context/route"
import { usePromptRef } from "../context/prompt"
import { useLocal } from "../context/local"
import { TuiPluginRuntime } from "@/cli/cmd/tui/plugin/runtime"
import { useEditorContext } from "@tui/context/editor"
import { useTheme } from "@tui/context/theme"
import { useZyalFlash } from "@tui/context/zyal-flash"

let once = false
const default_value = {
  normal: ["Fix a pending in the codebase", "What is the tech stack of this project?", "Fix broken tests"],
  shell: ["ls -la", "git status", "pwd"],
}

export function Home() {
  const sync = useSync()
  const project = useProject()
  const route = useRouteData("home")
  const promptRef = usePromptRef()
  const [ref, setRef] = createSignal<PromptRef | undefined>()
  const args = useArgs()
  const local = useLocal()
  const editor = useEditorContext()
  const dimensions = useTerminalDimensions()
  const { theme } = useTheme()
  const zyalFlash = useZyalFlash()
  const showZyalPanel = createMemo(() => zyalFlash().size > 0 && dimensions().width > 120)
  let sent = false

  onMount(() => {
    editor.clearSelection()
  })

  const bind = (r: PromptRef | undefined) => {
    setRef(r)
    promptRef.set(r)
    if (once || !r) return
    if (route.prompt) {
      r.set(route.prompt)
      once = true
      return
    }
    if (!args.prompt) return
    r.set({ input: args.prompt, parts: [] })
    once = true
  }

  // Wait for sync and model store to be ready before auto-submitting --prompt
  createEffect(() => {
    const r = ref()
    if (sent) return
    if (!r) return
    if (!sync.ready || !local.model.ready) return
    if (!args.prompt) return
    if (r.current.input !== args.prompt) return
    sent = true
    r.submit()
  })

  return (
    <>
      <box flexGrow={1} flexDirection="row" minHeight={0}>
        <box flexGrow={1} minWidth={0} alignItems="center" paddingLeft={2} paddingRight={showZyalPanel() ? 1 : 2}>
          <box flexGrow={1} minHeight={0} />
          <box height={4} minHeight={0} flexShrink={1} />
          <box flexShrink={0}>
            <TuiPluginRuntime.Slot name="home_logo" mode="replace">
              <Logo ink={RGBA.fromInts(212, 168, 67)} idle />{/* JEKKO amber/gold */}
            </TuiPluginRuntime.Slot>
          </box>
          <box height={1} minHeight={0} flexShrink={1} />
          <box width="100%" maxWidth={75} zIndex={1000} paddingTop={1} flexShrink={0}>
            <TuiPluginRuntime.Slot
              name="home_prompt"
              mode="replace"
              workspace_id={project.workspace.current()}
              ref={bind}
            >
              <Prompt
                ref={bind}
                workspaceID={project.workspace.current()}
                right={<TuiPluginRuntime.Slot name="home_prompt_right" workspace_id={project.workspace.current()} />}
                promptSuggestions={default_value}
              />
            </TuiPluginRuntime.Slot>
          </box>
          <TuiPluginRuntime.Slot name="home_bottom" />
          <box flexGrow={1} minHeight={0} />
          <Toast />
        </box>
        <Show when={showZyalPanel()}>
          <box
            backgroundColor={theme.backgroundPanel}
            width={42}
            height="100%"
            flexShrink={0}
            paddingTop={1}
            paddingBottom={1}
            paddingLeft={2}
            paddingRight={2}
          >
            <TuiPluginRuntime.Slot name="home_zyal_panel" workspace_id={project.workspace.current()} />
          </box>
        </Show>
      </box>
      <box width="100%" flexShrink={0}>
        <TuiPluginRuntime.Slot name="home_footer" mode="single_winner" />
      </box>
    </>
  )
}
