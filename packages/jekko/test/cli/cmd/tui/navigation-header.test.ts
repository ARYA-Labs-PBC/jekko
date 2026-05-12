import { describe, expect, test } from "bun:test"
import { buildSystemCommands } from "../../../../src/cli/cmd/tui/app-commands-system"

/**
 * Tests for the navigation header logic and Ctrl+H shortcut registration.
 *
 * Ctrl+H now calls navigateBack() which returns to the previous route
 * (e.g. the session you were working in) rather than always going Home.
 */

function makeCommandInput(overrides: Partial<Record<string, any>> = {}): any {
  const navigatedBack: boolean[] = []
  return {
    navigatedBack,
    dialog: {
      replace: () => {},
      clear: () => {},
    },
    route: {
      navigate: () => {},
      navigateBack: () => navigatedBack.push(true),
      data: { type: "home" },
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
    ...overrides,
  }
}

describe("navigation header commands", () => {
  test("ctrl+h command is registered in system commands", () => {
    const input = makeCommandInput()
    const commands = buildSystemCommands(input)
    const navBack = commands.find((c) => c.value === "nav.back")
    expect(navBack).toBeDefined()
    expect(navBack!.keybind).toBe("ctrl+h")
    expect(navBack!.category).toBe("Navigation")
    expect(navBack!.hidden).toBe(true)
  })

  test("ctrl+h command calls navigateBack", () => {
    const input = makeCommandInput()
    const commands = buildSystemCommands(input)
    const navBack = commands.find((c) => c.value === "nav.back")!
    navBack.onSelect(input.dialog)
    expect(input.navigatedBack).toEqual([true])
  })

  test("ctrl+h from session route calls navigateBack", () => {
    const input = makeCommandInput({
      route: {
        navigate: () => {},
        navigateBack: () => input.navigatedBack.push(true),
        data: { type: "session", sessionID: "test-session-123" },
      },
    })
    const commands = buildSystemCommands(input)
    const navBack = commands.find((c) => c.value === "nav.back")!
    navBack.onSelect(input.dialog)
    expect(input.navigatedBack).toEqual([true])
  })

  test("ctrl+h from plugin route calls navigateBack", () => {
    const input = makeCommandInput({
      route: {
        navigate: () => {},
        navigateBack: () => input.navigatedBack.push(true),
        data: { type: "plugin", id: "jnoccio" },
      },
    })
    const commands = buildSystemCommands(input)
    const navBack = commands.find((c) => c.value === "nav.back")!
    navBack.onSelect(input.dialog)
    expect(input.navigatedBack).toEqual([true])
  })
})
