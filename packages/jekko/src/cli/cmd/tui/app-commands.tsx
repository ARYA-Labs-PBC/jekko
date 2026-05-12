import type { TuiCommandInput } from "./app-commands-shared"
import { buildAgentCommands } from "./app-commands-agent"
import { buildProviderCommands } from "./app-commands-provider"
import { buildSessionCommands } from "./app-commands-session"
import { buildSystemCommands } from "./app-commands-system"

export function registerTuiCommands(input: TuiCommandInput) {
  input.command.register(() => [
    ...buildSessionCommands(input),
    ...buildAgentCommands(input),
    ...buildProviderCommands(input),
    ...buildSystemCommands(input),
  ])
}
