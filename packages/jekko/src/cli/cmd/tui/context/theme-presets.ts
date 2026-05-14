import jekkoDark from "./theme/jekko.json" with { type: "json" }
import jekkoLight from "./theme/jekko-light.json" with { type: "json" }
import jekkoGold from "./theme/jekko-gold.json" with { type: "json" }
import type { ThemeJson } from "./theme-core"

type ThemeMap = ThemeJson["theme"]

const adaptiveTheme = (dark: ThemeJson, light: ThemeJson): ThemeJson => {
  const theme: Partial<Record<keyof ThemeMap, unknown>> = {}
  const keys = new Set([...Object.keys(dark.theme), ...Object.keys(light.theme)] as Array<keyof ThemeMap>)

  for (const key of keys) {
    if (key === "thinkingOpacity") {
      theme[key] = dark.theme[key]
      continue
    }
    const darkValue = dark.theme[key]
    const lightValue = light.theme[key]
    theme[key] = {
      dark: darkValue ?? lightValue,
      light: lightValue ?? darkValue,
    }
  }

  return {
    $schema: dark.$schema ?? light.$schema,
    defs: {
      ...(dark.defs ?? {}),
      ...(light.defs ?? {}),
    },
    theme: theme as ThemeJson["theme"],
  }
}

export const DEFAULT_THEMES: Record<string, ThemeJson> = {
  jekko: adaptiveTheme(jekkoDark as ThemeJson, jekkoLight as ThemeJson),
  // Compatibility aliases for existing configs. New users should stay on
  // `jekko` and toggle mode rather than selecting a separate light theme.
  ["jekko-light"]: jekkoLight as ThemeJson,
  ["jekko-gold"]: jekkoGold as ThemeJson,
}
