import { CliRenderEvents } from "@opentui/core"
import path from "path"
import { createEffect, createMemo, onCleanup, onMount } from "solid-js"
import { createSimpleContext } from "./helper"
import { generateYamlHighContrastSyntax } from "./theme-syntax"
import { Glob } from "@jekko-ai/core/util/glob"
import { generateSyntax, generateSubtleSyntax, generateSystem, resolveTheme, type Theme, type ThemeJson } from "./theme-core"
import { DEFAULT_THEMES } from "./theme-presets"
import { useKV } from "./kv"
import { useRenderer } from "@opentui/solid"
import { createStore, produce } from "solid-js/store"
import { Global } from "@jekko-ai/core/global"
import { Filesystem } from "@/util/filesystem"
import { useTuiConfig } from "./tui-config"
import { isRecord } from "@/util/record"
import { useZyalFlash, zyalFlashOverlayTheme } from "./zyal-flash"
import type { TerminalColors } from "@opentui/core"

export { resolveTheme } from "./theme-core"
export { tint } from "./theme-core"
export { selectedForeground } from "./theme-core"
export { DEFAULT_THEMES } from "./theme-presets"

type State = {
  themes: Record<string, ThemeJson>
  mode: "dark" | "light"
  lock: "dark" | "light" | undefined
  active: string
  overlay: string | undefined
  ready: boolean
}

const pluginThemes: Record<string, ThemeJson> = {}
let customThemes: Record<string, ThemeJson> = {}
let systemTheme: ThemeJson | undefined

function listThemes() {
  // Priority: defaults < plugin installs < custom files < generated system.
  const themes = {
    ...DEFAULT_THEMES,
    ...pluginThemes,
    ...customThemes,
  }
  if (!systemTheme) return themes
  return {
    ...themes,
    system: systemTheme,
  }
}

function syncThemes() {
  setStore("themes", listThemes())
}

const [store, setStore] = createStore<State>({
  themes: listThemes(),
  mode: "dark",
  lock: undefined,
  active: "jekko",
  overlay: undefined,
  ready: false,
})

export function allThemes() {
  return store.themes
}

function isTheme(theme: unknown): theme is ThemeJson {
  if (!isRecord(theme)) return false
  if (!isRecord(theme.theme)) return false
  return true
}

export function hasTheme(name: string) {
  if (!name) return false
  return allThemes()[name] !== undefined
}

export function addTheme(name: string, theme: unknown) {
  if (!name) return false
  if (!isTheme(theme)) return false
  if (hasTheme(name)) return false
  pluginThemes[name] = theme
  syncThemes()
  return true
}

export function upsertTheme(name: string, theme: unknown) {
  if (!name) return false
  if (!isTheme(theme)) return false
  if (customThemes[name] !== undefined) {
    customThemes[name] = theme
  } else {
    pluginThemes[name] = theme
  }
  syncThemes()
  return true
}

export const { use: useTheme, provider: ThemeProvider } = createSimpleContext({
  name: "Theme",
  init: (props: { mode: "dark" | "light" }) => {
    const renderer = useRenderer()
    const config = useTuiConfig()
    const kv = useKV()
    const pick = (value: unknown) => {
      if (value === "dark" || value === "light") return value
      return
    }

    setStore(
      produce((draft) => {
        const lock = pick(kv.get("theme_mode_lock"))
        const mode = lock ?? pick(renderer.themeMode) ?? props.mode
        if (!lock && pick(kv.get("theme_mode")) !== undefined) {
          kv.set("theme_mode", undefined)
        }
        draft.mode = mode
        draft.lock = lock
        const active = config.theme ?? kv.get("theme", "jekko")
        draft.active = typeof active === "string" ? active : "jekko"
        draft.ready = false
      }),
    )

    createEffect(() => {
      const theme = config.theme
      if (theme) setStore("active", theme)
    })

    function init() {
      void Promise.allSettled([
        resolveSystemTheme(store.mode),
        getCustomThemes()
          .then((custom) => {
            customThemes = custom
            syncThemes()
          })
          .catch(() => {
            setStore("active", "jekko")
          }),
      ]).finally(() => {
        setStore("ready", true)
      })
    }

    onMount(init)

    function resolveSystemTheme(mode: "dark" | "light" = store.mode) {
      return renderer
        .getPalette({
          size: 16,
        })
        .then((colors: TerminalColors) => {
          if (!colors.palette[0]) {
            systemTheme = undefined
            syncThemes()
            if (store.active === "system") {
              setStore("active", "jekko")
            }
            return
          }
          systemTheme = generateSystem(colors, mode)
          syncThemes()
        })
        .catch(() => {
          systemTheme = undefined
          syncThemes()
          if (store.active === "system") {
            setStore("active", "jekko")
          }
        })
    }

    function apply(mode: "dark" | "light") {
      if (store.lock !== undefined) kv.set("theme_mode", mode)
      if (store.mode === mode) return
      setStore("mode", mode)
      renderer.clearPaletteCache()
      void resolveSystemTheme(mode)
    }

    function pin(mode: "dark" | "light" = store.mode) {
      setStore("lock", mode)
      kv.set("theme_mode_lock", mode)
      apply(mode)
    }

    function free() {
      setStore("lock", undefined)
      kv.set("theme_mode_lock", undefined)
      kv.set("theme_mode", undefined)
      const mode = renderer.themeMode
      if (mode) apply(mode)
    }

    const handle = (mode: "dark" | "light") => {
      if (store.lock) return
      apply(mode)
    }
    renderer.on(CliRenderEvents.THEME_MODE, handle)

    const refresh = () => {
      renderer.clearPaletteCache()
      init()
    }
    process.on("SIGUSR2", refresh)

    onCleanup(() => {
      renderer.off(CliRenderEvents.THEME_MODE, handle)
      process.off("SIGUSR2", refresh)
    })

    const zyalFlash = useZyalFlash()

    const values = createMemo(() => {
      // ZYAL flash forces the gold overlay when ZYAL is detected.
      // Falls back gracefully if the gold theme is missing.
      const flashOverlay = zyalFlash().size > 0 ? zyalFlashOverlayTheme() : undefined
      const flashTheme = flashOverlay ? store.themes[flashOverlay] : undefined
      if (flashTheme) {
        return resolveTheme(flashTheme, store.mode)
      }

      const activeName = store.overlay ?? store.active
      const active = store.themes[activeName]
      if (active) {
        return resolveTheme(active, store.mode)
      }

      const saved = kv.get("theme")
      if (typeof saved === "string") {
        const theme = store.themes[saved]
        if (theme) {
          return resolveTheme(theme, store.mode)
        }
      }

      return resolveTheme(store.themes.jekko, store.mode)
    })

    createEffect(() => {
      renderer.setBackgroundColor(values().background)
    })

    const syntax = createMemo(() => generateSyntax(values()))
    const subtleSyntax = createMemo(() => generateSubtleSyntax(values()))
    const yamlSyntax = createMemo(() => generateYamlHighContrastSyntax(values()))

    return {
      theme: new Proxy(values(), {
        get(_target, prop) {
          // @ts-expect-error
          return values()[prop]
        },
      }),
      get selected() {
        return store.active
      },
      all() {
        return allThemes()
      },
      has(name: string) {
        return hasTheme(name)
      },
      syntax,
      subtleSyntax,
      yamlSyntax,
      mode() {
        return store.mode
      },
      locked() {
        return store.lock !== undefined
      },
      lock() {
        pin(store.mode)
      },
      unlock() {
        free()
      },
      setMode(mode: "dark" | "light") {
        pin(mode)
      },
      set(theme: string) {
        if (!hasTheme(theme)) return false
        setStore("active", theme)
        kv.set("theme", theme)
        return true
      },
      setOverlay(theme: string | undefined) {
        if (theme && !hasTheme(theme)) return false
        setStore("overlay", theme)
        return true
      },
      get ready() {
        return store.ready
      },
    }
  },
})

async function getCustomThemes() {
  const directories = [
    Global.Path.config,
    ...(await Array.fromAsync(
      Filesystem.up({
        targets: [".jekko"],
        start: process.cwd(),
      }),
    )),
  ]

  const result: Record<string, ThemeJson> = {}
  for (const dir of directories) {
    for (const item of await Glob.scan("themes/*.json", {
      cwd: dir,
      absolute: true,
      dot: true,
      symlink: true,
    })) {
      const name = path.basename(item, ".json")
      const theme = await Filesystem.readJson(item)
      if (isTheme(theme)) result[name] = theme
    }
  }
  return result
}
