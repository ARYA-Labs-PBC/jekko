import { createMemo } from "solid-js"
import { RGBA } from "@opentui/core"
import { useSync } from "@tui/context/sync"
import { map, pipe, sortBy } from "remeda"
import { useDialog } from "@tui/ui/dialog"
import { useSDK } from "../context/sdk"
import { useTheme } from "../context/theme"
import { DialogJnoccioUnlock } from "./dialog-jnoccio-unlock"
import { DialogModel } from "./dialog-model"
import { collectPromptInputs, resolveProviderAuthMethod } from "./dialog-provider-methods"
import { useToast } from "../ui/toast"
import { isConsoleManagedProvider } from "@tui/util/provider-origin"
import { useConnected } from "./use-connected"

const PROVIDER_PRIORITY: Record<string, number> = {
  jnoccio: -1,
  jekko: 0,
  "jekko-go": 1,
  openai: 2,
  "github-copilot": 3,
  anthropic: 4,
  google: 5,
}

const FUSION_GREEN = RGBA.fromInts(80, 255, 0)
const FUSION_CYAN = RGBA.fromInts(0, 235, 216)
const FUSION_GOLD = RGBA.fromInts(255, 200, 0)

export function createDialogProviderOptions() {
  const sync = useSync()
  const dialog = useDialog()
  const sdk = useSDK()
  const toast = useToast()
  const { theme } = useTheme()
  const onboarded = useConnected()
  const options = createMemo(() => {
    return pipe(
      sync.data.provider_next.all,
      sortBy((x) => PROVIDER_PRIORITY[x.id] ?? 99),
      map((provider) => {
        const consoleManaged = isConsoleManagedProvider(sync.data.console_state.consoleManagedProviders, provider.id)
        const connected = sync.data.provider_next.connected.includes(provider.id)
        const isJnoccio = provider.id === "jnoccio"
        const locked =
          isJnoccio && Object.values(provider.models).every((model) => model.status === "locked")

        return {
          title: provider.name,
          value: provider.id,
          description:
            isJnoccio
              ? (locked ? "Paste your 128-char secret to unlock" : "Local AI fusion engine")
              : {
                  jekko: "(Recommended)",
                  anthropic: "(API key)",
                  openai: "(ChatGPT Plus/Pro or API key)",
                  "jekko-go": "Low cost subscription for everyone",
                }[provider.id] ?? undefined,
          footer: locked ? "Locked" : consoleManaged ? sync.data.console_state.activeOrgName : undefined,
          category: isJnoccio
            ? "Fusion"
            : provider.id in PROVIDER_PRIORITY
              ? "Popular"
              : "Other",
          categoryView: isJnoccio
            ? (
                <text>
                  <span style={{ fg: FUSION_GREEN }}>⚡ </span>
                  <span style={{ fg: FUSION_CYAN }}>F</span>
                  <span style={{ fg: FUSION_GREEN }}>u</span>
                  <span style={{ fg: FUSION_GOLD }}>s</span>
                  <span style={{ fg: FUSION_CYAN }}>i</span>
                  <span style={{ fg: FUSION_GREEN }}>o</span>
                  <span style={{ fg: FUSION_GOLD }}>n</span>
                </text>
              )
            : undefined,
          gutter: isJnoccio
            ? () => <text fg={locked ? FUSION_GOLD : FUSION_GREEN}>{locked ? "🔒" : "✓"}</text>
            : connected && onboarded() ? () => <text fg={theme.success}>✓</text> : undefined,
          async onSelect() {
            if (consoleManaged) return
            if (locked) {
              dialog.replace(() => <DialogJnoccioUnlock />)
              return
            }

            const selection = await resolveProviderAuthMethod({
              dialog,
              methods: sync.data.provider_auth[provider.id] ?? [],
            })
            if (selection.kind === "cancelled") return
            const { index, method } = selection
            switch (method.type) {
              case "oauth": {
                let inputs: Record<string, string> | undefined
                if (method.prompts?.length) {
                  const promptInputs = await collectPromptInputs({
                    dialog,
                    prompts: method.prompts,
                  })
                  if (promptInputs.kind === "cancelled") return
                  inputs = promptInputs.inputs
                }

                const result = await sdk.client.provider.oauth.authorize({
                  providerID: provider.id,
                  method: index,
                  inputs,
                })
                if (result.error) {
                  toast.show({
                    variant: "error",
                    message: JSON.stringify(result.error),
                  })
                  dialog.clear()
                  return
                }
                if (result.data?.method === "code") {
                  dialog.replace(() => (
                    <CodeMethod
                      providerID={provider.id}
                      title={method.label}
                      index={index}
                      authorization={result.data!}
                    />
                  ))
                }
                if (result.data?.method === "auto") {
                  dialog.replace(() => (
                    <AutoMethod
                      providerID={provider.id}
                      title={method.label}
                      index={index}
                      authorization={result.data!}
                    />
                  ))
                }
                break
              }
              case "api": {
                let metadata: Record<string, string> | undefined
                if (method.prompts?.length) {
                  const promptInputs = await collectPromptInputs({ dialog, prompts: method.prompts })
                  if (promptInputs.kind === "cancelled") return
                  metadata = promptInputs.inputs
                }
                return dialog.replace(() => (
                  <ApiMethod providerID={provider.id} title={method.label} metadata={metadata} />
                ))
              }
            }
          },
        }
      }),
    )
  })
  return options
}
