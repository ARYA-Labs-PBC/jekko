import { describe, expect, test } from "bun:test"
import { createTuiPluginApi } from "../../../fixture/tui-plugin"

/**
 * Integration tests for the full navigation shortcut flow.
 *
 * Tests validate:
 *  - Jnoccio plugin route/command/slot registration
 *  - Ctrl+H navigateBack behavior from all route types
 *  - Keybind collision prevention
 *  - Route-based navigation with back-navigation support
 */

// Dynamically import the Jnoccio plugin
const jnoccioPlugin = (await import(
  "../../../../src/cli/cmd/tui/feature-plugins/jnoccio/index"
)).default

// Import command builder for Ctrl+H
const { buildSystemCommands } = await import(
  "../../../../src/cli/cmd/tui/app-commands-system"
)

describe("navigation shortcut flow", () => {
  test("jnoccio plugin registers jnoccio route", async () => {
    const registeredRoutes: any[] = []
    const api = createTuiPluginApi()

    api.route.register = (routes: any[]) => {
      registeredRoutes.push(...routes)
      return () => {}
    }

    await jnoccioPlugin.tui(api)

    const jnoccioRoute = registeredRoutes.find(
      (r: any) => r.name === "jnoccio",
    )
    expect(jnoccioRoute).toBeDefined()
    expect(typeof jnoccioRoute.render).toBe("function")
  })

  test("jnoccio plugin registers a command factory", async () => {
    let commandFactory: (() => any[]) | undefined
    const api = createTuiPluginApi()

    api.command.register = (cb: () => any[]) => {
      commandFactory = cb
      return () => {}
    }

    await jnoccioPlugin.tui(api)

    expect(commandFactory).toBeDefined()
    expect(typeof commandFactory).toBe("function")
  })

  test("ctrl+h calls navigateBack regardless of current route", () => {
    const backCalls: boolean[] = []
    const input = makeSystemInput({ backCalls })

    const commands = buildSystemCommands(input)
    const ctrlH = commands.find((c: any) => c.value === "nav.back")!

    // From home
    ctrlH.onSelect(input.dialog)
    expect(backCalls).toEqual([true])

    // From jnoccio
    backCalls.length = 0
    input.route.data = { type: "plugin", id: "jnoccio" }
    ctrlH.onSelect(input.dialog)
    expect(backCalls).toEqual([true])

    // From session
    backCalls.length = 0
    input.route.data = { type: "session", sessionID: "s1" }
    ctrlH.onSelect(input.dialog)
    expect(backCalls).toEqual([true])
  })

  test("ctrl+h and jnoccio commands coexist without keybind collision", () => {
    const input = makeSystemInput({ backCalls: [] })
    const systemCommands = buildSystemCommands(input)

    // Ctrl+H is in system commands
    const ctrlH = systemCommands.find((c: any) => c.value === "nav.back")!
    expect(ctrlH.keybind).toBe("ctrl+h")

    // No other system command uses ctrl+h
    const ctrlHDups = systemCommands.filter(
      (c: any) => c.keybind === "ctrl+h",
    )
    expect(ctrlHDups).toHaveLength(1)

    // No system command uses ctrl+j (plugin domain)
    const ctrlJ = systemCommands.find((c: any) => c.keybind === "ctrl+j")
    expect(ctrlJ).toBeUndefined()
  })

  test("jnoccio plugin registers home_footer slot", async () => {
    const registeredSlots: any[] = []
    const api = createTuiPluginApi()

    api.slots.register = (plugin: any) => {
      registeredSlots.push(plugin)
      return "test-slot"
    }

    await jnoccioPlugin.tui(api)

    const footerSlot = registeredSlots.find(
      (s: any) => s.slots?.home_footer,
    )
    expect(footerSlot).toBeDefined()
    expect(typeof footerSlot.slots.home_footer).toBe("function")
  })

  test("jnoccio plugin has correct id", () => {
    expect(jnoccioPlugin.id).toBe("internal:jnoccio-dashboard")
  })

  test("ctrl+h command is hidden from command palette", () => {
    const input = makeSystemInput({ backCalls: [] })
    const commands = buildSystemCommands(input)
    const ctrlH = commands.find((c: any) => c.value === "nav.back")!
    expect(ctrlH.hidden).toBe(true)
  })

  test("route-based navigation: plugin navigate with id jnoccio", async () => {
    const { routeNavigate } = await import(
      "../../../../src/cli/cmd/tui/plugin/api-helpers"
    )

    const navigated: any[] = []
    const route: any = {
      data: { type: "home" },
      navigate: (r: any) => navigated.push(r),
    }

    // Navigate to jnoccio (plugin route)
    routeNavigate(route, "jnoccio")
    expect(navigated).toEqual([{ type: "plugin", id: "jnoccio", data: undefined }])

    // Navigate back to home
    navigated.length = 0
    routeNavigate(route, "home")
    expect(navigated).toEqual([{ type: "home" }])
  })
})

// ── Helpers ─────────────────────────────────────────────────────────────

function makeSystemInput(opts: { backCalls: boolean[] }): any {
  return {
    navigated: [],
    dialog: {
      replace: () => {},
      clear: () => {},
    },
    route: {
      navigate: () => {},
      navigateBack: () => opts.backCalls.push(true),
      data: { type: "home" } as any,
    },
    local: {},
    kv: {
      get: (_name: string, fallback?: unknown) => fallback,
      set: () => {},
    },
    sdk: {},
    renderer: {
      toggleDebugOverlay: () => {},
      console: { toggle: () => {} },
      setTerminalTitle: () => {},
    },
    toast: {
      show: () => {},
    },
    sync: {
      session: { refresh: async () => {} },
    },
    exit: () => {},
    connected: () => true,
    tuiConfig: {},
    theme: {
      mode: () => "dark",
      setMode: () => {},
      locked: () => false,
      lock: () => {},
      unlock: () => {},
    },
    terminalTitleEnabled: () => true,
    setTerminalTitleEnabled: () => {},
    pasteSummaryEnabled: () => true,
    setPasteSummaryEnabled: () => {},
  }
}
