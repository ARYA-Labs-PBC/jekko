import { DialogPrompt } from "@tui/ui/dialog-prompt"
import { DialogSelect } from "@tui/ui/dialog-select"
import { useDialog } from "@tui/ui/dialog"

import type { ProviderAuthMethod } from "@jekko-ai/sdk/v2"

interface PromptsMethodProps {
  dialog: ReturnType<typeof useDialog>
  prompts: NonNullable<ProviderAuthMethod["prompts"]>[number][]
}

export type PromptCollectionResult =
  | { kind: "completed"; inputs: Record<string, string> }
  | { kind: "cancelled"; promptKey: string }

export type ProviderAuthSelection =
  | { kind: "selected"; index: number; method: ProviderAuthMethod }
  | { kind: "cancelled" }

export const DEFAULT_PROVIDER_AUTH_METHOD: ProviderAuthMethod = {
  type: "api",
  label: "API key",
}

export async function collectPromptInputs(props: PromptsMethodProps): Promise<PromptCollectionResult> {
  const inputs: Record<string, string> = {}
  for (const prompt of props.prompts) {
    if (prompt.when) {
      const value = inputs[prompt.when.key]
      if (value === undefined) continue
      const matches = prompt.when.op === "eq" ? value === prompt.when.value : value !== prompt.when.value
      if (!matches) continue
    }

    if (prompt.type === "select") {
      const value = await new Promise<{ kind: "selected"; value: string } | { kind: "cancelled" }>((resolve) => {
        props.dialog.replace(
          () => (
            <DialogSelect
              title={prompt.message}
              options={prompt.options.map((x) => ({
                title: x.label,
                value: x.value,
                description: x.hint,
              }))}
              onSelect={(option) => resolve({ kind: "selected", value: option.value })}
            />
          ),
          () => resolve({ kind: "cancelled" }),
        )
      })
      if (value.kind === "cancelled") return { kind: "cancelled", promptKey: prompt.key }
      inputs[prompt.key] = value.value
      continue
    }

    const value = await new Promise<{ kind: "selected"; value: string } | { kind: "cancelled" }>((resolve) => {
      props.dialog.replace(
        () => (
          <DialogPrompt
            title={prompt.message}
            default_value={prompt.default_value ?? "Enter text"}
            onConfirm={(value) => resolve({ kind: "selected", value })}
          />
        ),
        () => resolve({ kind: "cancelled" }),
      )
    })
    if (value.kind === "cancelled") return { kind: "cancelled", promptKey: prompt.key }
    inputs[prompt.key] = value.value
  }
  return { kind: "completed", inputs }
}

export async function resolveProviderAuthMethod(props: {
  dialog: ReturnType<typeof useDialog>
  methods: ProviderAuthMethod[]
}): Promise<ProviderAuthSelection> {
  const methods = props.methods.length ? props.methods : [DEFAULT_PROVIDER_AUTH_METHOD]
  if (methods.length === 1) {
    return { kind: "selected", index: 0, method: methods[0] }
  }

  const index = await new Promise<number | null>((resolve) => {
    props.dialog.replace(
      () => (
        <DialogSelect
          title="Select auth method"
          options={methods.map((x, index) => ({
            title: x.label,
            value: index,
          }))}
          onSelect={(option) => resolve(option.value)}
        />
      ),
      () => resolve(null),
    )
  })

  if (index == null) return { kind: "cancelled" }
  const method = methods[index]
  if (!method) return { kind: "cancelled" }
  return { kind: "selected", index, method }
}
