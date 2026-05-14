import { createMemo } from "solid-js"
import { TextAttributes } from "@opentui/core"
import { useDialog } from "@tui/ui/dialog"
import { useKeyboard } from "@opentui/solid"
import { useSync } from "@tui/context/sync"
import { useTheme } from "../context/theme"
import { modelKeyCatalog } from "@/model-setup/model-keys"

export function DialogKeySetup() {
  const dialog = useDialog()
  const sync = useSync()
  const { theme } = useTheme()
  const catalog = createMemo(() => modelKeyCatalog())
  const providers = createMemo(
    () =>
      sync.data.provider as Array<{
        id: string
        name: string
        auth?: { active?: boolean; configured?: boolean }
      }>,
  )
  const active = createMemo(() => providers().filter((provider) => provider.auth?.active))
  const configured = createMemo(() => providers().filter((provider) => provider.auth?.configured))

  useKeyboard((evt) => {
    if (evt.name === "escape") dialog.clear()
  })

  return (
    <box paddingLeft={2} paddingRight={2} paddingBottom={1} gap={1}>
      <box flexDirection="row" justifyContent="space-between">
        <text fg={theme.text} attributes={TextAttributes.BOLD}>
          No model keys found
        </text>
        <text fg={theme.textMuted}>esc</text>
      </box>
      <text fg={theme.textMuted}>Put keys in <span style={{ fg: theme.primary }}>~/.jekko/jekko.env</span></text>
      <text fg={theme.textMuted}>
        Jekko creates the file on first startup and keeps blank entries inactive.
      </text>
      <text fg={theme.textMuted}>
        Active providers: <span style={{ fg: theme.text }}>{active().length}</span>
        {"  "}Configured providers: <span style={{ fg: theme.text }}>{configured().length}</span>
      </text>
      <box gap={1}>
        {catalog().map((item) => (
          <text fg={theme.textMuted}>
            <span style={{ fg: theme.text }}>{item.providerID}</span>
            {"  "}
            {item.signupUrl ?? "local key"}
          </text>
        ))}
      </box>
      <text fg={theme.textMuted}>Paste one key first. Auto routing picks the model.</text>
    </box>
  )
}
