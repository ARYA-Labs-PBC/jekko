import { TextAttributes } from "@opentui/core"
import { useTheme } from "../context/theme"
import { useDialog } from "./dialog"
import type { JSX } from "solid-js"

export function DialogFrame(props: {
  title: string
  message: string
  children: JSX.Element
  onEscape?: () => void
}) {
  const dialog = useDialog()
  const { theme } = useTheme()

  return (
    <box paddingLeft={2} paddingRight={2} gap={1}>
      <box flexDirection="row" justifyContent="space-between">
        <text attributes={TextAttributes.BOLD} fg={theme.text}>
          {props.title}
        </text>
        <text
          fg={theme.textMuted}
          onMouseUp={() => {
            props.onEscape?.()
            dialog.clear()
          }}
        >
          esc
        </text>
      </box>
      <box paddingBottom={1}>
        <text fg={theme.textMuted}>{props.message}</text>
      </box>
      {props.children}
    </box>
  )
}
