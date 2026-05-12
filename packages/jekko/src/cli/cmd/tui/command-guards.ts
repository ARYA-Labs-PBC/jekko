import { UI } from "@/cli/ui"
import { ensureValidSession } from "./session-validation"
import { win32DisableProcessedInput, win32InstallCtrlCGuard } from "./win32"

type ForkableArgs = {
  fork?: boolean
  continue?: boolean
  session?: string
}

export async function runTuiCommandWithGuards<T extends ForkableArgs>(
  args: T,
  body: () => Promise<void>,
): Promise<void> {
  const unguard = win32InstallCtrlCGuard()
  try {
    win32DisableProcessedInput()
    if (args.fork && !args.continue && !args.session) {
      UI.error("--fork requires --continue or --session")
      process.exitCode = 1
      return
    }
    await body()
  } finally {
    unguard?.()
  }
}

export async function runValidatedTuiCommand<T extends ForkableArgs>(
  args: T,
  sessionOptions: Parameters<typeof ensureValidSession>[0],
  body: () => Promise<void>,
): Promise<void> {
  await runTuiCommandWithGuards(args, async () => {
    if (!(await ensureValidSession(sessionOptions))) return
    await body()
  })
}

export async function runValidatedTuiCommandWithConfig<T extends ForkableArgs, TConfig>(
  args: T,
  sessionOptions: Parameters<typeof ensureValidSession>[0],
  getConfig: () => Promise<TConfig>,
  body: (config: TConfig) => Promise<void>,
): Promise<void> {
  await runValidatedTuiCommand(args, sessionOptions, async () => {
    const config = await getConfig()
    await body(config)
  })
}
