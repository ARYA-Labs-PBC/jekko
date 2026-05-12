import { createSignal, onMount, Show } from "solid-js"
import { useDialog } from "@tui/ui/dialog"
import { DialogSecretPrompt } from "@tui/ui/dialog-secret-prompt"
import { useSDK } from "@tui/context/sdk"
import { useSync } from "@tui/context/sync"
import { useLocal } from "@tui/context/local"
import { useTheme } from "@tui/context/theme"
import { useToast } from "@tui/ui/toast"
import { normalizeJnoccioUnlockSecret } from "@/util/jnoccio-unlock"

export function DialogJnoccioUnlock() {
  const dialog = useDialog()
  const sdk = useSDK()
  const sync = useSync()
  const local = useLocal()
  const toast = useToast()
  const { theme } = useTheme()
  const [busy, setBusy] = createSignal(true)
  const [error, setError] = createSignal<string>()
  const [checking, setChecking] = createSignal(true)

  async function finalizeUnlocked(source: "cache" | "typed", secretSaved?: boolean) {
    await sdk.client.instance.dispose()
    await sync.bootstrap()
    local.model.set({ providerID: "jnoccio", modelID: "jnoccio-fusion" }, { recent: true })
    toast.show({
      variant: "info",
      message:
        source === "cache"
          ? "Jnoccio Fusion unlocked locally."
          : secretSaved
            ? "Jnoccio Fusion unlocked and cached your unlock secret."
            : "Jnoccio Fusion unlocked.",
    })
    dialog.clear()
  }

  async function attempt(payload: { unlockSecret?: string; keyPath?: string }, source: "cache" | "typed" = "cache") {
    setBusy(true)
    setError(undefined)
    try {
      const result = await sdk.client.provider.jnoccio.unlock({
        jnoccioUnlockInput: payload,
      })
      if (result.error) {
        setError("Unlock failed. Try again.")
        return
      }
      const data = result.data
      if (!data) {
        setError("Unlock failed. Try again.")
        return
      }
      if (data.status === "needs_secret") {
        setError(data.message)
        return
      }
      if (data.status === "error") {
        setError(data.message)
        return
      }

      await finalizeUnlocked(source, data.secretSaved)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unlock failed. Try again.")
    } finally {
      setBusy(false)
      setChecking(false)
    }
  }

  onMount(() => {
    void attempt({})
  })

  return (
    <DialogSecretPrompt
      title="Unlock Jnoccio Fusion"
      description={() => (
        <box gap={1}>
          <Show when={checking()}>
            <text fg={theme.textMuted}>Checking local unlock secret...</text>
          </Show>
          <text fg={theme.textMuted}>
            Paste the 128-character unlock secret to unlock Jnoccio Fusion and cache it locally.
          </text>
          <Show when={error()}>
            <text fg={theme.error}>{error()}</text>
          </Show>
        </box>
      )}
      busy={busy() && !checking()}
      busyText="Unlocking Jnoccio Fusion..."
      error={error()}
      onCancel={() => dialog.clear()}
      onConfirm={(value) => {
        const next = normalizeJnoccioUnlockSecret(value)
        void attempt({ unlockSecret: next }, "typed")
      }}
    />
  )
}
