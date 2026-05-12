import { TextAttributes } from "@opentui/core"
import { usePaste, useKeyboard } from "@opentui/solid"
import { createSignal, onMount, Show, type JSX } from "solid-js"
import { useDialog, type DialogContext } from "./dialog"
import { useTheme } from "../context/theme"
import { Spinner } from "../component/spinner"
import { normalizeJnoccioUnlockSecret } from "@/util/jnoccio-unlock"

export type DialogSecretPromptProps = {
  title: string
  description?: () => JSX.Element
  value?: string
  busy?: boolean
  busyText?: string
  error?: string
  onConfirm?: (value: string) => void
  onCancel?: () => void
}

function maskSecret(value: string) {
  if (!value) return ""
  const visible = Math.min(value.length, 24)
  const hidden = Math.max(0, value.length - visible)
  return `${"•".repeat(visible)}${hidden > 0 ? ` … ${value.length}/128` : ` ${value.length}/128`}`
}

function sanitizePaste(value: string) {
  return normalizeJnoccioUnlockSecret(value)
}

export function DialogSecretPrompt(props: DialogSecretPromptProps) {
  const dialog = useDialog()
  const { theme } = useTheme()
  const [value, setValue] = createSignal(props.value ?? "")

  useKeyboard((evt) => {
    if (props.busy) {
      if (evt.name === "escape") return
      evt.preventDefault()
      evt.stopPropagation()
      return
    }

    if (evt.name === "escape") {
      evt.preventDefault()
      evt.stopPropagation()
      props.onCancel?.()
      return
    }

    if (evt.name === "return") {
      evt.preventDefault()
      evt.stopPropagation()
      props.onConfirm?.(sanitizePaste(value()))
      return
    }

    if (evt.ctrl && evt.name === "u") {
      evt.preventDefault()
      evt.stopPropagation()
      setValue("")
      return
    }

    if (evt.name === "backspace" || evt.name === "delete") {
      evt.preventDefault()
      evt.stopPropagation()
      setValue((current) => current.slice(0, -1))
      return
    }

    if (evt.ctrl || evt.meta) return
    const text = evt.sequence.length === 1 ? evt.sequence : evt.name
    if (text.length !== 1) return
    if (!/^[A-Za-z0-9_-]$/.test(text)) return
    evt.preventDefault()
    evt.stopPropagation()
    setValue((current) => sanitizePaste(current + text))
  })

  usePaste((evt) => {
    if (props.busy) return
    evt.preventDefault()
    evt.stopPropagation()
    const pasted = sanitizePaste(new TextDecoder().decode(evt.bytes))
    if (!pasted) return
    setValue((current) => sanitizePaste(current + pasted))
  })

  onMount(() => {
    dialog.setSize("medium")
  })

  return (
    <box paddingLeft={2} paddingRight={2} gap={1}>
      <box flexDirection="row" justifyContent="space-between">
        <text attributes={TextAttributes.BOLD} fg={theme.text}>
          {props.title}
        </text>
        <text fg={theme.textMuted} onMouseUp={() => props.onCancel?.()}>
          esc
        </text>
      </box>
      <box gap={1}>
        {props.description}
        <box paddingLeft={1} paddingRight={1} paddingTop={1} paddingBottom={1} borderColor={theme.border} border>
          <text fg={value().length ? theme.text : theme.textMuted}>{value().length ? maskSecret(value()) : "Paste your 128-character unlock secret"}</text>
        </box>
        <Show when={props.error}>
          <text fg={theme.error}>{props.error}</text>
        </Show>
        <Show when={props.busy}>
          <Spinner color={theme.textMuted}>{props.busyText ?? "Working..."}</Spinner>
        </Show>
      </box>
      <box paddingBottom={1} gap={1} flexDirection="row">
        {props.busy ? (
          <text fg={theme.textMuted}>processing...</text>
        ) : (
          <>
            <text fg={theme.text}>
              enter <span style={{ fg: theme.textMuted }}>submit</span>
            </text>
            <text fg={theme.textMuted}>
              ctrl+u <span style={{ fg: theme.textMuted }}>clear</span>
            </text>
          </>
        )}
      </box>
    </box>
  )
}

DialogSecretPrompt.show = (
  dialog: DialogContext,
  title: string,
  options?: Omit<DialogSecretPromptProps, "title">,
) => {
  return new Promise<string | null>((resolve) => {
    dialog.replace(
      () => (
        <DialogSecretPrompt
          title={title}
          {...options}
          onConfirm={(value) => resolve(value)}
          onCancel={() => resolve(null)}
        />
      ),
      () => resolve(null),
    )
  })
}
