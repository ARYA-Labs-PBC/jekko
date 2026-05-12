import type { Argv } from "yargs"
import { UI } from "../ui"
import * as prompts from "@clack/prompts"
import { Installation } from "../../installation"
import { InstallationVersion } from "@jekko-ai/core/installation/version"

export const UpgradeCommand = {
  command: "upgrade [target]",
  describe: "upgrade jekko to the latest or a specific version",
  builder: (yargs: Argv) => {
    return yargs
      .positional("target", {
        describe: "version to upgrade to, for ex '0.1.48' or 'v0.1.48'",
        type: "string",
      })
      .option("method", {
        alias: "m",
        describe: "installation method to use",
        type: "string",
        choices: ["curl", "npm", "pnpm", "bun", "brew", "choco", "scoop"],
      })
      .option("repair", {
        describe: "force reinstall and smoke-check the installed jekko binary",
        type: "boolean",
        default: false,
      })
  },
  handler: async (args: { target?: string; method?: string; repair?: boolean }) => {
    UI.empty()
    UI.println(UI.logo("  "))
    UI.empty()
    prompts.intro("Upgrade")
    const detectedMethod = await Installation.method()
    const method = (args.method as Installation.Method) ?? detectedMethod
    if (method === "unknown") {
      prompts.log.error(`jekko is installed to ${process.execPath} and may be managed by a package manager`)
      const install = await prompts.select({
        message: "Install anyways?",
        options: [
          { label: "Yes", value: true },
          { label: "No", value: false },
        ],
        initialValue: false,
      })
      if (!install) {
        prompts.outro("Done")
        return
      }
    }
    prompts.log.info("Using method: " + method)
    const target = args.target ? args.target.replace(/^v/, "") : await Installation.latest()

    if (InstallationVersion === target && !args.repair) {
      prompts.log.warn(`jekko upgrade skipped: ${target} is already installed`)
      prompts.outro("Done")
      return
    }

    if (args.repair) {
      prompts.log.info(`Repairing ${target} with ${method}`)
    } else {
      prompts.log.info(`From ${InstallationVersion} → ${target}`)
    }
    const spinner = prompts.spinner()
    spinner.start(args.repair ? "Repairing..." : "Upgrading...")
    const err = await Installation.upgrade(method, target, { repair: args.repair }).catch((err) => err)
    if (err) {
      spinner.stop("Upgrade failed", 1)
      if (err instanceof Installation.UpgradeFailedError) {
        // necessary because choco only allows install/upgrade in elevated terminals
        if (method === "choco" && err.stderr.includes("not running from an elevated command shell")) {
          // jankurai:allow HLT-001-DEAD-MARKER reason=user-facing-error-guidance-not-retry-logic expires=2027-01-01
          prompts.log.error("Please run the terminal as Administrator and try again")
        } else {
          prompts.log.error(err.stderr)
        }
      } else if (err instanceof Error) prompts.log.error(err.message)
      prompts.outro("Done")
      return
    }
    spinner.stop(args.repair ? "Repair complete" : "Upgrade complete")
    prompts.outro("Done")
  },
}
