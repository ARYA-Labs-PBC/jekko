/**
 * Thin persistent navigation header for the Jekko TUI shell.
 *
 * Shows pane shortcuts with active-state highlighting. The header keeps the
 * visible grammar human-readable (`Ctrl+H`, `Ctrl+J`) instead of raw control
 * glyphs. The Jnoccio tab only appears when the Jnoccio server is reachable
 * (boot ready or WS connected).
 */
import { createMemo, Show } from "solid-js"
import { RGBA } from "@opentui/core"
import { useKeyboard } from "@opentui/solid"
import { useJnoccioBootStatus } from "@tui/context/jnoccio-boot"
import { useZyalMetrics } from "@tui/context/zyal-flash"
import { useTheme } from "@tui/context/theme"
import { useRoute } from "@tui/context/route"

const GOLD = RGBA.fromHex("#F5A623")

export function NavigationHeader() {
  const { theme } = useTheme()
  const route = useRoute()
  const bootStatus = useJnoccioBootStatus()
  const metrics = useZyalMetrics()

  const jnoccioAvailable = createMemo(
    () => metrics().jnoccioConnected || bootStatus() === "ready",
  )

  const isHome = createMemo(
    () => route.data.type === "home" || route.data.type === "session",
  )

  const isJnoccio = createMemo(
    () => route.data.type === "plugin" && route.data.id === "jnoccio",
  )

  useKeyboard((evt) => {
    if (evt.defaultPrevented) return
    const name = evt.name?.toLowerCase()
    if (evt.ctrl && name === "h") {
      evt.preventDefault()
      route.navigateBack()
      return
    }
    if (!jnoccioAvailable()) return
    if ((evt.ctrl && name === "j") || name === "linefeed" || evt.name === "\n") {
      evt.preventDefault()
      if (isJnoccio()) {
        route.navigateBack()
      } else {
        route.navigate({ type: "plugin", id: "jnoccio" })
      }
    }
  })

  return (
    <box
      flexDirection="row"
      width="100%"
      height={1}
      flexShrink={0}
      paddingLeft={2}
      paddingRight={2}
      gap={1}
    >
      {/* Home tab */}
      <box flexDirection="row" gap={0}>
        <text fg={isHome() ? GOLD : theme.textMuted}>
          <span
            style={{
              bg: isHome() ? theme.backgroundMenu : theme.backgroundElement,
              fg: isHome() ? GOLD : theme.textMuted,
            }}
          >
            {" Ctrl+H "}
          </span>
        </text>
        <text fg={isHome() ? GOLD : theme.textMuted}> Back</text>
      </box>

      <Show when={jnoccioAvailable()}>
        <text fg={theme.borderSubtle}>│</text>

        {/* Jnoccio tab */}
        <box flexDirection="row" gap={0}>
          <text fg={isJnoccio() ? GOLD : theme.textMuted}>
            <span
              style={{
                bg: isJnoccio() ? theme.backgroundMenu : theme.backgroundElement,
                fg: isJnoccio() ? GOLD : theme.textMuted,
              }}
            >
              {" Ctrl+J "}
            </span>
          </text>
          <text fg={isJnoccio() ? GOLD : theme.textMuted}> Jnoccio</text>
        </box>
      </Show>
    </box>
  )
}
