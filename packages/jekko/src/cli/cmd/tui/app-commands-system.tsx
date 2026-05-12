import open from "open"
import { DialogHelp } from "./ui/dialog-help"
import { DialogStatus } from "@tui/component/dialog-status"
import { DialogThemeList } from "@tui/component/dialog-theme-list"
import type { TuiCommandInput } from "./app-commands-shared"

export function buildSystemCommands(input: TuiCommandInput) {
  return [
    {
      title: "Navigate back",
      value: "nav.back",
      keybind: "ctrl+h",
      category: "Navigation",
      hidden: true,
      onSelect: () => {
        input.route.navigateBack()
      },
    },
    {
      title: "View status",
      keybind: "status_view",
      value: "jekko.status",
      slash: {
        name: "status",
      },
      onSelect: () => {
        input.dialog.replace(() => <DialogStatus />)
      },
      category: "System",
    },
    {
      title: "Switch theme",
      value: "theme.switch",
      keybind: "theme_list",
      slash: {
        name: "themes",
      },
      onSelect: () => {
        input.dialog.replace(() => <DialogThemeList />)
      },
      category: "System",
    },
    {
      title: input.theme.mode() === "dark" ? "Switch to light mode" : "Switch to dark mode",
      value: "theme.switch_mode",
      onSelect: (dialog) => {
        input.theme.setMode(input.theme.mode() === "dark" ? "light" : "dark")
        dialog.clear()
      },
      category: "System",
    },
    {
      title: input.theme.locked() ? "Unlock theme mode" : "Lock theme mode",
      value: "theme.mode.lock",
      onSelect: (dialog) => {
        if (input.theme.locked()) input.theme.unlock()
        else input.theme.lock()
        dialog.clear()
      },
      category: "System",
    },
    {
      title: "Help",
      value: "help.show",
      slash: {
        name: "help",
      },
      onSelect: () => {
        input.dialog.replace(() => <DialogHelp />)
      },
      category: "System",
    },
    {
      title: "Open docs",
      value: "docs.open",
      onSelect: () => {
        open("https://jekko.ai/docs").catch(() => {})
        input.dialog.clear()
      },
      category: "System",
    },
    {
      title: "Exit the app",
      value: "app.exit",
      slash: {
        name: "exit",
        aliases: ["quit", "q"],
      },
      onSelect: () => input.exit(),
      category: "System",
    },
    {
      title: "Toggle debug panel",
      category: "System",
      value: "app.debug",
      onSelect: (dialog) => {
        input.renderer.toggleDebugOverlay()
        dialog.clear()
      },
    },
    {
      title: "Toggle console",
      category: "System",
      value: "app.console",
      onSelect: (dialog) => {
        input.renderer.console.toggle()
        dialog.clear()
      },
    },
    {
      title: "Write heap snapshot",
      category: "System",
      value: "app.heap_snapshot",
      onSelect: async (dialog) => {
        const files = await input.onSnapshot?.()
        input.toast.show({
          variant: "info",
          message: `Heap snapshot written to ${files?.join(", ")}`,
          duration: 5000,
        })
        dialog.clear()
      },
    },
    {
      title: "Suspend terminal",
      value: "terminal.suspend",
      keybind: "terminal_suspend",
      category: "System",
      hidden: true,
      enabled: input.tuiConfig.keybinds?.terminal_suspend !== "none",
      onSelect: () => {
        process.once("SIGCONT", () => {
          input.renderer.resume()
        })

        input.renderer.suspend()
        process.kill(0, "SIGTSTP")
      },
    },
    {
      title: input.terminalTitleEnabled() ? "Disable terminal title" : "Enable terminal title",
      value: "terminal.title.toggle",
      keybind: "terminal_title_toggle",
      category: "System",
      onSelect: (dialog) => {
        input.setTerminalTitleEnabled((prev) => {
          const next = !prev
          input.kv.set("terminal_title_enabled", next)
          if (!next) input.renderer.setTerminalTitle("")
          return next
        })
        dialog.clear()
      },
    },
    {
      title: input.kv.get("animations_enabled", true) ? "Disable animations" : "Enable animations",
      value: "app.toggle.animations",
      category: "System",
      onSelect: (dialog) => {
        input.kv.set("animations_enabled", !input.kv.get("animations_enabled", true))
        dialog.clear()
      },
    },
    {
      title: input.kv.get("file_context_enabled", true) ? "Disable file context" : "Enable file context",
      value: "app.toggle.file_context",
      category: "System",
      onSelect: (dialog) => {
        input.kv.set("file_context_enabled", !input.kv.get("file_context_enabled", true))
        dialog.clear()
      },
    },
    {
      title: input.pasteSummaryEnabled() ? "Disable paste summary" : "Enable paste summary",
      value: "app.toggle.paste_summary",
      category: "System",
      onSelect: (dialog) => {
        input.setPasteSummaryEnabled((prev) => {
          const next = !prev
          input.kv.set("paste_summary_enabled", next)
          return next
        })
        dialog.clear()
      },
    },
    {
      title: input.kv.get("session_directory_filter_enabled", true)
        ? "Disable session directory filtering"
        : "Enable session directory filtering",
      value: "app.toggle.session_directory_filter",
      category: "System",
      onSelect: async (dialog) => {
        input.kv.set("session_directory_filter_enabled", !input.kv.get("session_directory_filter_enabled", true))
        await input.sync.session.refresh()
        dialog.clear()
      },
    },
    {
      title: input.kv.get("diff_wrap_mode", "word") === "word" ? "Disable diff wrapping" : "Enable diff wrapping",
      value: "app.toggle.diffwrap",
      category: "System",
      onSelect: (dialog) => {
        const current = input.kv.get("diff_wrap_mode", "word")
        input.kv.set("diff_wrap_mode", current === "word" ? "none" : "word")
        dialog.clear()
      },
    },
  ]
}
