/**
 * Shell CENTER region — activity feed (Phase 6A of TUIbomb).
 *
 * Registers into the `shell_center_feed` slot (single_winner) and mounts
 * the existing session-body pipeline inside the shell route's CENTER
 * region. No new message-render code lives here — the diff renderer,
 * tool calls, reasoning blocks, permission/question prompts, daemon
 * banner, and Prompt input all come for free from `SessionBody`.
 *
 * Active session: most-recently-updated root session from sync data.
 * When no session exists yet, render a centered hint instead of the
 * full session pipeline.
 */
import { createEffect, createMemo, createSignal } from "solid-js"
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { useSync } from "@tui/context/sync"
import { SessionBody } from "@tui/routes/session"
import { Prompt, type PromptRef } from "@tui/component/prompt"
import { useArgs } from "@tui/context/args"
import { useLocal } from "@tui/context/local"
import { ShellEmptyHero } from "./empty-hero"

const id = "internal:shell-activity-feed"

function ActivityFeedView(props: { api: TuiPluginApi; centerContentWidth?: number }) {
  const sync = useSync()
  const args = useArgs()
  const local = useLocal()
  const [shellSessionID, setShellSessionID] = createSignal<string>()
  const [promptRef, setPromptRef] = createSignal<PromptRef>()
  const [autoSubmitted, setAutoSubmitted] = createSignal(false)

  // Most-recently-updated root (parentless) session. Mirrors the existing
  // resume logic in `app-view.tsx` / `app-bindings.tsx` so the shell route
  // shows the same session the user would resume with ctrl+R / --continue.
  const latestSessionID = createMemo<string | undefined>(() => {
    const sessions = sync.data.session ?? []
    return sessions
      .toSorted((a, b) => b.time.updated - a.time.updated)
      .find((s) => s.parentID === undefined)?.id
  })

  const waitingOnStartupPrompt = createMemo(() => Boolean(args.prompt && !autoSubmitted() && !shellSessionID()))
  const activeSessionID = createMemo<string | null | undefined>(() =>
    waitingOnStartupPrompt() ? null : shellSessionID() ?? latestSessionID(),
  )

  createEffect(() => {
    if (!args.prompt || autoSubmitted()) return
    if (!sync.ready || !local.model.ready) return
    const ref = promptRef()
    if (!ref) return
    setAutoSubmitted(true)
    ref.set({ input: args.prompt, parts: [] })
    setTimeout(() => ref.submit(), 0)
  })

  const sessionID = activeSessionID()
  return sessionID ? (
    <SessionBody sessionID={sessionID} />
  ) : (
    <box flexGrow={1} minHeight={0} flexDirection="column" justifyContent="flex-end" paddingBottom={1}>
      <box flexGrow={1} minHeight={0} flexDirection="column" alignItems="center" justifyContent="center">
        <ShellEmptyHero
          version={props.api.app.version}
          width={props.centerContentWidth ?? 48}
          mode={props.api.theme.mode()}
        />
      </box>
      <Prompt
        ref={setPromptRef}
        navigateOnNewSession={false}
        onSessionCreated={setShellSessionID}
        onSubmit={() => {}}
      />
    </box>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 50,
    slots: {
      shell_center_feed(_ctx, props) {
        return <ActivityFeedView api={api} centerContentWidth={props.center_content_width} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
