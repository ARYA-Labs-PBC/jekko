import HomeFooter from "../feature-plugins/home/footer"
import HomeTips from "../feature-plugins/home/tips"
import SidebarContext from "../feature-plugins/sidebar/context"
import SidebarZyal from "../feature-plugins/sidebar/zyal"
import SidebarJankurai from "../feature-plugins/sidebar/jankurai"
import SidebarMcp from "../feature-plugins/sidebar/mcp"
import SidebarLsp from "../feature-plugins/sidebar/lsp"
import SidebarPending from "../feature-plugins/sidebar/pending"
import SidebarFiles from "../feature-plugins/sidebar/files"
import SidebarFooter from "../feature-plugins/sidebar/footer"
import JnoccioDashboard from "../feature-plugins/jnoccio/index"
import PluginManager from "../feature-plugins/system/plugins"
import SessionV2Debug from "../feature-plugins/system/session-debug"
import type { TuiPlugin, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { Flag } from "@jekko-ai/core/flag/flag"

export type InternalTuiPlugin = TuiPluginModule & {
  id: string
  tui: TuiPlugin
}

export const INTERNAL_TUI_PLUGINS: InternalTuiPlugin[] = [
  HomeFooter,
  HomeTips,
  SidebarContext,
  SidebarZyal,
  SidebarJankurai,
  SidebarMcp,
  SidebarLsp,
  SidebarPending,
  SidebarFiles,
  SidebarFooter,
  JnoccioDashboard,
  PluginManager,
  ...(Flag.JEKKO_EXPERIMENTAL_EVENT_SYSTEM ? [SessionV2Debug] : []),
]
