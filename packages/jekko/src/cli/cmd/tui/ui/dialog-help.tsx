import { TextAttributes } from "@opentui/core"
import { useTheme } from "@tui/context/theme"
import { useDialog } from "./dialog"
import { useKeyboard } from "@opentui/solid"
import { For } from "solid-js"

// Shortcut entries grouped by the surface that subscribes to them.
// Each row's `binding` is the schema-registered keybind name; `display`
// is the human-facing keyboard grammar rendered in the overlay.
type HelpRow = { binding: string; label: string; display: string }

const GROUPS: Array<{ title: string; rows: HelpRow[] }> = [
  {
    title: "Global",
    rows: [
      { binding: "command_list", label: "Open command palette", display: "Ctrl+P" },
      { binding: "theme.mode.toggle", label: "Toggle dark / light theme", display: "Ctrl+Shift+T" },
      { binding: "session.new", label: "New session", display: "Ctrl+N" },
      { binding: "session.resume", label: "Resume most recent session", display: "Ctrl+R" },
      { binding: "app_exit", label: "Quit (Ctrl+C twice in prompt)", display: "Ctrl+C" },
      { binding: "help.show", label: "This help overlay", display: "?" },
    ],
  },
  {
    title: "Shell",
    rows: [
      { binding: "shell.tab.cycle", label: "Next left pane", display: "Tab" },
      { binding: "shell.tab.cycleBack", label: "Previous left pane", display: "Shift+Tab" },
      { binding: "shell.tab.set", label: "Jump directly to tab", display: "1 / 2 / 3" },
      { binding: "shell.left.toggle", label: "Toggle LEFT panel", display: "Ctrl+B" },
    ],
  },
  {
    title: "Feed",
    rows: [
      { binding: "feed.scroll.pageUp", label: "Page up", display: "Page Up" },
      { binding: "feed.scroll.pageDown", label: "Page down", display: "Page Down" },
      { binding: "feed.scroll.top", label: "Jump to top", display: "g g" },
      { binding: "feed.scroll.bottom", label: "Jump to bottom", display: "Shift+G" },
    ],
  },
]

export function DialogHelp() {
  const dialog = useDialog()
  const { theme } = useTheme()

  useKeyboard((evt) => {
    if (evt.name === "return" || evt.name === "escape") {
      evt.preventDefault()
      evt.stopPropagation()
      dialog.clear()
    }
  })

  return (
    <box paddingLeft={2} paddingRight={2} paddingTop={1} paddingBottom={1} gap={1}>
      <box flexDirection="row" justifyContent="space-between">
        <text attributes={TextAttributes.BOLD} fg={theme.text}>
          Jekko shortcuts
        </text>
        <text fg={theme.textMuted} onMouseUp={() => dialog.clear()}>
          [Esc] / [Enter]
        </text>
      </box>

      <For each={GROUPS}>
        {(group) => (
          <box flexDirection="column" gap={0}>
            <text attributes={TextAttributes.BOLD} fg={theme.accent}>
              {group.title}
            </text>
            <For each={group.rows}>
              {(row) => (
                <box flexDirection="row" justifyContent="space-between">
                  <text fg={theme.text}>{row.label}</text>
                  <text fg={theme.textMuted}>{row.display}</text>
                </box>
              )}
            </For>
          </box>
        )}
      </For>

      <box paddingTop={1}>
        <text fg={theme.textMuted}>
          Press Ctrl+P for the full command palette.
        </text>
      </box>

      <box flexDirection="row" justifyContent="flex-end" paddingTop={1}>
        <box paddingLeft={3} paddingRight={3} backgroundColor={theme.primary} onMouseUp={() => dialog.clear()}>
          <text fg={theme.selectedListItemText}>ok</text>
        </box>
      </box>
    </box>
  )
}
