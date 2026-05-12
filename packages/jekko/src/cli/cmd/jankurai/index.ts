import type { Argv } from "yargs"
import { cmd } from "../cmd"
import { JankuraiBootstrapCommand } from "./bootstrap"

export const JankuraiCommand = cmd({
  command: "jankurai",
  describe: "jankurai integration commands (audit standard configuration helpers)",
  builder: (yargs: Argv) => yargs.command(JankuraiBootstrapCommand).demandCommand(),
  async handler() {},
})

export { JankuraiBootstrapCommand }
