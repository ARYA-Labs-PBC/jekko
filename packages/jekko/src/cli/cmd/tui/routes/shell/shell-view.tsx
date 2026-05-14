/**
 * Shell route (Phase 5 of TUIbomb).
 *
 * Post-Enter main surface that replaces the session-centric UI. Layout is:
 *   ┌─ NavigationHeader (mounted in app-view; not duplicated here) ─┐
 *   ├─ LEFT panel ── CENTER activity feed ─────────────────────────┤
 *   └─ Footer hints ──────────────────────────────────────────────┘
 *
 * The view itself is an empty shell: the LEFT tab bar, the LEFT pane body,
 * the CENTER feed, and the bottom footer are each rendered through plugin
 * slots so Phase 6 panes can plug in without touching this file. Slot modes
 * are all `single_winner` so only the registered plugin wins (additive
 * registration is the default — see `routes/home.tsx` for that pattern).
 */
import { createMemo, Show } from "solid-js"
import { useTerminalDimensions } from "@opentui/solid"
import { useTheme } from "@tui/context/theme"
import { useLocal } from "@tui/context/local"
import { useKeybind } from "@tui/context/keybind"
import { useSync } from "@tui/context/sync"
import { TuiPluginRuntime } from "@/cli/cmd/tui/plugin/runtime"
import { Toast } from "@tui/ui/toast"
import { FooterBand } from "@tui/component/footer-band"

type LeftSize =
  | { kind: "shown"; width: number; overlay: boolean }
  | { kind: "hidden" }

function resolveLeft(width: number, visible: boolean): LeftSize {
  if (width < 80) return { kind: "hidden" }
  if (width < 100) return visible ? { kind: "shown", width: 28, overlay: true } : { kind: "hidden" }
  if (width < 120) return { kind: "shown", width: 28, overlay: false }
  if (width < 160) return { kind: "shown", width: 38, overlay: false }
  return { kind: "shown", width: 44, overlay: false }
}

export function Shell() {
  const themeState = useTheme()
  const { theme } = themeState
  const local = useLocal()
  const keybind = useKeybind()
  const sync = useSync()
  const dimensions = useTerminalDimensions()

  const left = createMemo<LeftSize>(() => resolveLeft(dimensions().width, local.shellLeftVisible.get()))
  const leftWidth = createMemo(() => {
    const current = left()
    return current.kind === "shown" ? current.width : 0
  })
  const leftContentWidth = createMemo(() => Math.max(0, leftWidth() - 4))
  const centerWidth = createMemo(() => Math.max(0, dimensions().width - leftWidth()))
  const centerContentWidth = createMemo(() => Math.max(0, centerWidth() - 4))

  keybind.on("shell.left.toggle", () => {
    local.shellLeftVisible.toggle()
  })

  const activeSessionID = createMemo<string | undefined>(() => {
    const sessions = sync.data.session ?? []
    return sessions
      .toSorted((a, b) => b.time.updated - a.time.updated)
      .find((s) => s.parentID === undefined)?.id
  })

  return (
    <>
      <box flexGrow={1} minHeight={0} flexDirection="row">
        <Show when={left().kind === "shown" ? (left() as Extract<LeftSize, { kind: "shown" }>) : null}>
          {(sized) => (
            <box
              backgroundColor={theme.backgroundPanel}
              width={sized().width}
              height="100%"
              flexShrink={0}
              flexDirection="column"
              paddingTop={1}
              paddingBottom={1}
              paddingLeft={2}
              paddingRight={2}
              position={sized().overlay ? "absolute" : "relative"}
              top={sized().overlay ? 0 : undefined}
              left={sized().overlay ? 0 : undefined}
              zIndex={sized().overlay ? 200 : undefined}
            >
              <box flexShrink={0}>
                <TuiPluginRuntime.Slot
                  name="shell_left_tabs"
                  mode="single_winner"
                  active_pane={local.shellPane.get()}
                  left_width={sized().width}
                  left_content_width={Math.max(0, sized().width - 4)}
                />
              </box>
              <scrollbox
                flexGrow={1}
                verticalScrollbarOptions={{
                  trackOptions: {
                    backgroundColor: theme.background,
                    foregroundColor: theme.borderActive,
                  },
                }}
              >
                <box gap={1} paddingTop={1} paddingRight={1}>
                  <TuiPluginRuntime.Slot
                    name="shell_left_active_pane"
                    mode="single_winner"
                    active_pane={local.shellPane.get()}
                    left_width={sized().width}
                    left_content_width={Math.max(0, sized().width - 4)}
                  />
                  <Show when={activeSessionID()}>
                    {(sessionID) => (
                      <>
                        <TuiPluginRuntime.Slot name="sidebar_content" session_id={sessionID()} />
                        <TuiPluginRuntime.Slot name="sidebar_footer" mode="single_winner" session_id={sessionID()} />
                      </>
                    )}
                  </Show>
                </box>
              </scrollbox>
            </box>
          )}
        </Show>

        <box flexGrow={1} minWidth={0} flexDirection="column" paddingLeft={2} paddingRight={2}>
          <TuiPluginRuntime.Slot
            name="shell_center_feed"
            mode="single_winner"
            left_width={leftWidth()}
            left_content_width={leftContentWidth()}
            center_width={centerWidth()}
            center_content_width={centerContentWidth()}
          >
            <box flexGrow={1} alignItems="center" justifyContent="center">
              <text fg={theme.textMuted}>No active feed</text>
            </box>
          </TuiPluginRuntime.Slot>
          <Toast />
        </box>
      </box>

      <FooterBand backgroundColor={theme.backgroundPanel} borderColor={theme.borderSubtle}>
        <box width="100%" paddingLeft={2} paddingRight={2} paddingTop={1} paddingBottom={1}>
          <TuiPluginRuntime.Slot name="shell_footer" mode="single_winner">
            <text fg={theme.textMuted}>
              <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Enter </span> send{"  "}
              <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Ctrl+P </span> commands{"  "}
              <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Tab </span> pane{"  "}
              <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Ctrl+B </span> left{"  "}
              <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Ctrl+Shift+T </span>{" "}
              {themeState.mode() === "dark" ? "light" : "dark"}{"  "}
              <span style={{ bg: theme.backgroundElement, fg: theme.text }}> ? </span> help{"  "}
              <span style={{ bg: theme.backgroundElement, fg: theme.text }}> Ctrl+C </span> quit
            </text>
          </TuiPluginRuntime.Slot>
        </box>
      </FooterBand>
    </>
  )
}
