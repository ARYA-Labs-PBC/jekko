import { DialogConsoleOrg } from "@tui/component/dialog-console-org"
import { DialogProvider as DialogProviderList } from "@tui/component/dialog-provider"
import type { TuiCommandInput } from "./app-commands-shared"

export function buildProviderCommands(input: TuiCommandInput) {
  return [
    {
      title: "Connect provider",
      value: "provider.connect",
      suggested: !input.connected(),
      slash: {
        name: "connect",
      },
      onSelect: () => {
        input.dialog.replace(() => <DialogProviderList />)
      },
      category: "Provider",
    },
    ...(input.sync.data.console_state.switchableOrgCount > 1
      ? [
          {
            title: "Switch org",
            value: "console.org.switch",
            suggested: Boolean(input.sync.data.console_state.activeOrgName),
            slash: {
              name: "org",
              aliases: ["orgs", "switch-org"],
            },
            onSelect: () => {
              input.dialog.replace(() => <DialogConsoleOrg />)
            },
            category: "Provider",
          },
        ]
      : []),
  ]
}
