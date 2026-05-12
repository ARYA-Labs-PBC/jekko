/**
 * Jnoccio TUI help overlay.
 *
 * Shows a reference card of all keyboard shortcuts.
 * Toggled with the `?` key.
 */
import type { TuiPluginApi } from "@jekko-ai/plugin/tui"
import { RGBA } from "@opentui/core"

const GOLD = RGBA.fromHex("#F5A623")
const TEAL = RGBA.fromHex("#36D7B7")

type KeyHint = { key: string; desc: string }

const SECTIONS: { title: string; hints: KeyHint[] }[] = [
  {
    title: "Navigation",
    hints: [
      { key: "1-6", desc: "Switch to tab directly" },
      { key: "←/→", desc: "Cycle tabs" },
      { key: "Tab", desc: "Next tab" },
      { key: "j/↓", desc: "Move selection down" },
      { key: "k/↑", desc: "Move selection up" },
      { key: "g", desc: "Jump to top" },
      { key: "G", desc: "Jump to bottom" },
    ],
  },
  {
    title: "Actions",
    hints: [
      { key: "Enter", desc: "Open model detail drawer" },
      { key: "/", desc: "Start search / filter" },
      { key: "Esc", desc: "Close search/drawer/help or exit" },
      { key: "s", desc: "Cycle sort mode (Board tab)" },
      { key: "p", desc: "Pause / resume live updates" },
      { key: "f", desc: "Cycle phase filter (Feed tab)" },
      { key: "r", desc: "Force refresh (re-fetch snapshot)" },
    ],
  },
  {
    title: "Global",
    hints: [
      { key: "?", desc: "Toggle this help overlay" },
      { key: "Ctrl+H", desc: "Back (return to previous view)" },
      { key: "Ctrl+J", desc: "Toggle Jnoccio dashboard" },
      { key: "q", desc: "Exit Jnoccio dashboard" },
    ],
  },
]

export function HelpOverlay(props: {
  api: TuiPluginApi
  onClose: () => void
}) {
  const theme = () => props.api.theme.current

  return (
    <box
      flexDirection="column"
      width="100%"
      paddingLeft={3}
      paddingRight={3}
      paddingTop={1}
      paddingBottom={1}
      gap={1}
    >
      <box flexDirection="row" justifyContent="space-between">
        <text fg={GOLD}>
          <b>⌨ Jnoccio Keyboard Shortcuts</b>
        </text>
        <text fg={theme().textMuted}>? or Esc to close</text>
      </box>

      {SECTIONS.map((section) => (
        <box flexDirection="column" gap={0}>
          <text fg={TEAL}>
            <b>─── {section.title} ───</b>
          </text>
          {section.hints.map((hint) => (
            <box flexDirection="row" gap={1}>
              <text fg={theme().text} width={12}>
                <b>{hint.key}</b>
              </text>
              <text fg={theme().textMuted}>{hint.desc}</text>
            </box>
          ))}
        </box>
      ))}
    </box>
  )
}
