import type { Setter } from "solid-js"
import type { useCommandDialog } from "@tui/component/dialog-command"
import type { useDialog } from "@tui/ui/dialog"
import type { useConnected } from "@tui/component/use-connected"
import type { useLocal } from "@tui/context/local"
import type { useKV } from "./context/kv"
import type { useRoute } from "@tui/context/route"
import type { useSync } from "@tui/context/sync"
import type { useSDK } from "@tui/context/sdk"
import type { useTheme } from "@tui/context/theme"
import type { useRenderer } from "@opentui/solid"
import type { useExit } from "./context/exit"
import type { useToast } from "./ui/toast"
import type { TuiConfig } from "@/cli/cmd/tui/config/tui"

export type TuiCommandInput = {
  command: ReturnType<typeof useCommandDialog>
  route: ReturnType<typeof useRoute>
  local: ReturnType<typeof useLocal>
  dialog: ReturnType<typeof useDialog>
  kv: ReturnType<typeof useKV>
  sync: ReturnType<typeof useSync>
  sdk: ReturnType<typeof useSDK>
  renderer: ReturnType<typeof useRenderer>
  toast: ReturnType<typeof useToast>
  theme: ReturnType<typeof useTheme>
  exit: ReturnType<typeof useExit>
  connected: ReturnType<typeof useConnected>
  tuiConfig: TuiConfig.Info
  terminalTitleEnabled: () => boolean
  setTerminalTitleEnabled: Setter<boolean>
  pasteSummaryEnabled: () => boolean
  setPasteSummaryEnabled: Setter<boolean>
  onSnapshot?: () => Promise<string[]>
}
