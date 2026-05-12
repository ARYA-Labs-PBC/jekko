import { UI } from "@/cli/ui"
import { errorMessage } from "@/util/error"
import { validateSession } from "./validate-session"

export async function ensureValidSession(
  options: Parameters<typeof validateSession>[0],
): Promise<boolean> {
  try {
    await validateSession(options)
    return true
  } catch (error) {
    UI.error(errorMessage(error))
    process.exitCode = 1
    return false
  }
}
