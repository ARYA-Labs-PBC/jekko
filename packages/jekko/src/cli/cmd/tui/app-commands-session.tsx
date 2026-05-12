import { DialogSessionList } from "@tui/component/dialog-session-list"
import type { TuiCommandInput } from "./app-commands-shared"

export function buildSessionCommands(input: TuiCommandInput) {
  return [
    {
      title: "Switch session",
      value: "session.list",
      keybind: "session_list",
      category: "Session",
      suggested: input.sync.data.session.length > 0,
      slash: {
        name: "sessions",
        aliases: ["resume", "continue"],
      },
      onSelect: () => {
        input.dialog.replace(() => <DialogSessionList />)
      },
    },
    {
      title: "New session",
      suggested: input.route.data.type === "session",
      value: "session.new",
      keybind: "session_new",
      category: "Session",
      slash: {
        name: "new",
        aliases: ["clear"],
      },
      onSelect: () => {
        input.route.navigate({
          type: "home",
        })
        input.dialog.clear()
      },
    },
  ]
}
