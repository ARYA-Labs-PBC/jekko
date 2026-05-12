import { UI } from "@/cli/ui"
import { win32DisableProcessedInput, win32InstallCtrlCGuard } from "./win32"

type ForkableArgs = {
  fork?: boolean
  continue?: boolean
  session?: string
}

export async function withTuiCommandGuard(args: ForkableArgs, run: () => Promise<void>) {
  const unguard = win32InstallCtrlCGuard()
  try {
    win32DisableProcessedInput()

    if (args.fork && !args.continue && !args.session) {
      UI.error("--fork requires --continue or --session")
      process.exitCode = 1
      return
    }

    await run()
  } finally {
    unguard?.()
  }
}
