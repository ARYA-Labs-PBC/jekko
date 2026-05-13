import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js"
import { TextAttributes, type RGBA } from "@opentui/core"
import { useTheme, tint } from "@tui/context/theme"
import { Spinner } from "@tui/component/spinner"
import { InstallationVersion } from "@jekko-ai/core/installation/version"

const MIN_SHOW_MS = 800
const HARD_CAP_MS = 5000
const STEP_MS = 275
const PULSE_PERIOD_MS = 4600

const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

type BootStatus = "running" | "done" | "warn" | "error"

type BootEntry = {
  ts: number
  msg: string
  status: BootStatus
}

type BootLine = {
  phase: string
  message: string
}

const BOOT_SCRIPT: BootLine[] = [
  { phase: "runtime", message: "initialized" },
  { phase: "plugins", message: "hydrated" },
  { phase: "workspace", message: "indexed" },
  { phase: "daemon", message: "connected" },
  { phase: "sync", message: "ready" },
  { phase: "jnoccio", message: "detected" },
  { phase: "jankurai", message: "watching score" },
  { phase: "All systems", message: "Ready" },
]

const NEVER_HUMAN = "N · E · V · E · R · H · U · M · A · N"

export type SplashScreenProps = {
  ready: () => boolean
  onDismiss: () => void
}

function padEnd(value: string, width: number): string {
  if (value.length >= width) return value
  return value + " ".repeat(width - value.length)
}

function formatTimestamp(elapsedMs: number): string {
  const secs = Math.max(0, elapsedMs) / 1000
  return `[+${secs.toFixed(2)}s]`
}

export function SplashScreen(props: SplashScreenProps) {
  const { theme } = useTheme()
  const mountedAt = performance.now()
  const [entries, setEntries] = createSignal<BootEntry[]>([])
  const [now, setNow] = createSignal(performance.now())
  const [dismissed, setDismissed] = createSignal(false)

  const cursor = () => entries().length
  const elapsed = () => now() - mountedAt
  const readyAllowed = () => props.ready() && elapsed() >= MIN_SHOW_MS

  // Tick used for spinner glyph + pulse color animation.
  const tick = setInterval(() => setNow(performance.now()), 90)
  onCleanup(() => clearInterval(tick))

  // Advance the canonical boot script on a fixed cadence so the splash
  // always reads as activity even when plugin init is instant.
  const stepper = setInterval(() => {
    setEntries((current) => {
      if (current.length >= BOOT_SCRIPT.length - 1) return current
      const next = [...current]
      // Flip the previous running line to done.
      if (next.length > 0) {
        const last = next[next.length - 1]
        if (last && last.status === "running") {
          next[next.length - 1] = { ...last, status: "done" }
        }
      }
      // Push the next line as running.
      const line = BOOT_SCRIPT[next.length]
      if (line) {
        next.push({
          ts: performance.now() - mountedAt,
          msg: line.message,
          status: "running",
        })
      }
      return next
    })
  }, STEP_MS)
  onCleanup(() => clearInterval(stepper))

  // When ready() flips true (and the min-show has elapsed), append the final
  // "All systems Ready" line and mark all earlier lines done.
  createEffect(() => {
    if (!readyAllowed()) return
    setEntries((current) => {
      const next = current.map<BootEntry>((entry) =>
        entry.status === "running" ? { ...entry, status: "done" } : entry,
      )
      if (next.length < BOOT_SCRIPT.length) {
        // Backfill any earlier lines we haven't reached yet so the log
        // doesn't look truncated when boot beats the stepper.
        while (next.length < BOOT_SCRIPT.length - 1) {
          const line = BOOT_SCRIPT[next.length]
          if (!line) break
          next.push({
            ts: performance.now() - mountedAt,
            msg: line.message,
            status: "done",
          })
        }
        const finalLine = BOOT_SCRIPT[BOOT_SCRIPT.length - 1]
        if (finalLine && (next.length === 0 || next[next.length - 1]?.msg !== finalLine.message)) {
          next.push({
            ts: performance.now() - mountedAt,
            msg: finalLine.message,
            status: "done",
          })
        }
      }
      return next
    })
  })

  // Dismiss logic: either ready + min-show, or hard cap.
  let dismissTimer: ReturnType<typeof setTimeout> | undefined
  const dismiss = () => {
    if (dismissed()) return
    setDismissed(true)
    props.onDismiss()
  }
  createEffect(() => {
    if (dismissed()) return
    if (readyAllowed()) {
      // Give the final "Ready" line one frame to render before tearing down.
      if (!dismissTimer) {
        dismissTimer = setTimeout(dismiss, 120)
      }
    }
  })
  const capTimer = setTimeout(() => {
    if (dismissed()) return
    if (!props.ready()) {
      setEntries((current) => {
        const next = [...current]
        if (next.length > 0) {
          const last = next[next.length - 1]
          if (last) {
            next[next.length - 1] = { ...last, status: "warn" }
          }
        }
        next.push({
          ts: performance.now() - mountedAt,
          msg: "splash hit 5s cap with plugins not ready",
          status: "warn",
        })
        return next
      })
    }
    dismiss()
  }, HARD_CAP_MS)
  onCleanup(() => {
    if (dismissTimer) clearTimeout(dismissTimer)
    clearTimeout(capTimer)
  })

  const spinnerFrame = createMemo(() => {
    const idx = Math.floor(now() / 90) % SPINNER_FRAMES.length
    return SPINNER_FRAMES[idx] ?? SPINNER_FRAMES[0]
  })

  function statusGlyph(entry: BootEntry, isFinal: boolean): { glyph: string; color: RGBA } {
    if (entry.status === "done") {
      return {
        glyph: isFinal ? "●" : "✓",
        color: theme.success,
      }
    }
    if (entry.status === "warn") return { glyph: "!", color: theme.warning }
    if (entry.status === "error") return { glyph: "✗", color: theme.error }
    return { glyph: spinnerFrame() ?? "⏵", color: theme.accent }
  }

  const loadingPulse = createMemo(() => {
    const phase = (now() % PULSE_PERIOD_MS) / PULSE_PERIOD_MS
    const eased = 0.5 + 0.5 * Math.sin(phase * Math.PI * 2)
    return tint(theme.accent, theme.text, eased * 0.45)
  })

  const visibleEntries = createMemo(() => entries())

  return (
    <box
      width="100%"
      height="100%"
      backgroundColor={theme.background}
      flexDirection="row"
    >
      <box
        flexBasis={0}
        flexGrow={6}
        minWidth={0}
        paddingLeft={2}
        paddingRight={2}
        paddingTop={1}
        paddingBottom={1}
        flexDirection="column"
        justifyContent="flex-end"
      >
        <For each={visibleEntries()}>
          {(entry, index) => {
            const isFinal = () => index() === visibleEntries().length - 1 && entry.msg === "Ready"
            const line = BOOT_SCRIPT[index()]
            const phaseLabel = line?.phase ?? ""
            const glyph = createMemo(() => statusGlyph(entry, isFinal()))
            return (
              <box flexDirection="row" gap={1}>
                <text fg={theme.textMuted}>{formatTimestamp(entry.ts)}</text>
                <text fg={theme.accent}>▸</text>
                <text fg={theme.text}>{padEnd(phaseLabel, 12)}</text>
                <text fg={theme.textMuted}>{padEnd(entry.msg, 16)}</text>
                <text fg={glyph().color}>{glyph().glyph}</text>
              </box>
            )
          }}
        </For>
        <Show when={cursor() === 0}>
          <text fg={theme.textMuted}>warming up…</text>
        </Show>
      </box>
      <box
        flexBasis={0}
        flexGrow={4}
        minWidth={0}
        flexDirection="column"
        justifyContent="center"
        alignItems="center"
        paddingLeft={2}
        paddingRight={2}
      >
        <box flexDirection="column" alignItems="center">
          <text fg={theme.text} attributes={TextAttributes.BOLD}>
            {NEVER_HUMAN}
          </text>
          <text fg={theme.accent}>{"▔".repeat(NEVER_HUMAN.length)}</text>
        </box>
        <box height={1} />
        <box flexDirection="row" gap={1}>
          <text fg={theme.text}>{`Jekko v${InstallationVersion}`}</text>
          <text fg={loadingPulse()}>loading…</text>
        </box>
        <box height={1} />
        <Spinner color={theme.accent} />
      </box>
    </box>
  )
}
