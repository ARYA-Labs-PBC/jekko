import { TextAttributes } from "@opentui/core"
import { useTheme } from "@tui/context/theme"
import { useDialog } from "./dialog"
import { useKeyboard } from "@opentui/solid"
import { useKeybind } from "@tui/context/keybind"
import { For } from "solid-js"

// Shortcut entries grouped by the surface that subscribes to them.
// Each row's `binding` is the schema-registered keybind name; the
// dialog prints the active chord via `keybind.print(binding)`. Where
// a row covers multiple chords (e.g. shell.tab.set binds 1/2/3),
// `display` overrides the printed chord with a human label.
type HelpRow = { binding: string; label: string; display?: string }

const GROUPS: Array<{ title: string; rows: HelpRow[] }> = [
  {
    title: "Global",
    rows: [
      { binding: "command_list", label: "Open command palette" },
      { binding: "theme.mode.toggle", label: "Toggle dark / light theme" },
      { binding: "session.new", label: "New session" },
      { binding: "session.resume", label: "Resume most recent session" },
      { binding: "app_exit", label: "Quit (Ctrl+C twice in prompt)" },
      { binding: "help.show", label: "This help overlay" },
    ],
  },
  {
    title: "Home",
    rows: [{ binding: "engage", label: "Engage → shell route" }],
  },
  {
    title: "Shell",
    rows: [
      { binding: "shell.tab.cycle", label: "Cycle left-tab forward" },
      { binding: "shell.tab.cycleBack", label: "Cycle left-tab backward" },
      { binding: "shell.tab.set", label: "Jump directly to tab", display: "1 / 2 / 3" },
      { binding: "shell.left.toggle", label: "Toggle LEFT panel" },
    ],
  },
  {
    title: "Feed",
    rows: [
      { binding: "feed.scroll.pageUp", label: "Page up" },
      { binding: "feed.scroll.pageDown", label: "Page down" },
      { binding: "feed.scroll.top", label: "Jump to top", display: "g g" },
      { binding: "feed.scroll.bottom", label: "Jump to bottom", display: "G" },
      { binding: "feed.yank", label: "Yank current diff / code block" },
      { binding: "feed.reasoning.toggle", label: "Expand / collapse reasoning" },
    ],
  },
]

export function DialogHelp() {
  const dialog = useDialog()
  const { theme } = useTheme()
  const keybind = useKeybind()

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
          esc / enter
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
                  <text fg={theme.textMuted}>{row.display ?? keybind.print(row.binding)}</text>
                </box>
              )}
            </For>
          </box>
        )}
      </For>

      <box paddingTop={1}>
        <text fg={theme.textMuted}>
          Press {keybind.print("command_list")} for the full command palette.
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
