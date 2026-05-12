// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import { cmd } from "./cmd"
import { ProvidersListCommand } from "./providers/list"
import { ProvidersLoginCommand } from "./providers/login"
import { ProvidersLogoutCommand } from "./providers/logout"
import { ProvidersUnlockCommand } from "./providers/unlock"

export const ProvidersCommand = cmd({
  command: "providers",
  aliases: ["auth"],
  describe: "manage AI providers and credentials",
  builder: (yargs) =>
    yargs
      .command(ProvidersListCommand)
      .command(ProvidersLoginCommand)
      .command(ProvidersLogoutCommand)
      .command(ProvidersUnlockCommand)
      .demandCommand(),
  async handler() {},
})

