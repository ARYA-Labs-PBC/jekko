import HomeFooter from "../feature-plugins/home/footer"
import HomeTips from "../feature-plugins/home/tips"
import SidebarContext from "../feature-plugins/sidebar/context"
import SidebarZyal from "../feature-plugins/sidebar/zyal"
import SidebarJankurai from "../feature-plugins/sidebar/jankurai"
import SidebarFooter from "../feature-plugins/sidebar/footer"
import ShellTabs from "../feature-plugins/shell/tabs"
import ShellActivityFeed from "../feature-plugins/shell/activity-feed"
import ShellPaneJnoccio from "../feature-plugins/shell/pane-jnoccio"
import ShellPaneCapability from "../feature-plugins/shell/pane-capability"
import ShellPaneHistory from "../feature-plugins/shell/pane-history"
import JnoccioDashboard from "../feature-plugins/jnoccio/index"
import ResearchDashboard from "../feature-plugins/research/index"
import PluginManager from "../feature-plugins/system/plugins"
import SessionV2Debug from "../feature-plugins/system/session-debug"
import type { TuiPlugin, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { Flag } from "@jekko-ai/core/flag/flag"

export type InternalTuiPlugin = TuiPluginModule & {
  id: string
  tui: TuiPlugin
}

// TUIbomb Phase 6A: file-tree sidebar plugins (SidebarFiles, SidebarLsp,
// SidebarMcp, SidebarPending) are no longer registered. Their source files
// remain on disk for reference but the shell route's LEFT panel now holds
// the Phase 6 tabs/panes instead. Phase 6A also registers ShellTabs (the
// LEFT panel tab bar) and ShellActivityFeed (mounts the existing session
// pipeline inside the shell route's CENTER region).
export const INTERNAL_TUI_PLUGINS: InternalTuiPlugin[] = [
  HomeFooter,
  HomeTips,
  SidebarContext,
  SidebarZyal,
  SidebarJankurai,
  SidebarFooter,
  ShellTabs,
  ShellActivityFeed,
  ShellPaneJnoccio,
  ShellPaneCapability,
  ShellPaneHistory,
  JnoccioDashboard,
  ResearchDashboard,
  PluginManager,
  ...(Flag.JEKKO_EXPERIMENTAL_EVENT_SYSTEM ? [SessionV2Debug] : []),
]
