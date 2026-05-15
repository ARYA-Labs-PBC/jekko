import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js"
import { TextAttributes, type RGBA } from "@opentui/core"
import { useTheme, tint } from "@tui/context/theme"
import { InstallationVersion } from "@jekko-ai/core/installation/version"
import { useTerminalDimensions } from "@opentui/solid"
import { useSync } from "@tui/context/sync"
import { useProject } from "@tui/context/project"
import { useJnoccioBootStatus } from "@tui/context/jnoccio-boot"
import { useStartupQuit } from "./startup-quit"
import fs from "fs"
import path from "path"

const SPLASH_DURATION_MULTIPLIER = 3
const MIN_SHOW_MS = 800 * SPLASH_DURATION_MULTIPLIER
const HARD_CAP_MS = 5000 * SPLASH_DURATION_MULTIPLIER
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
  status: BootStatus
}

const NEVER_HUMAN = "N · E · V · E · R · H · U · M · A · N"

export type SplashScreenProps = {
  ready: () => boolean
  onDismiss: () => void
  onQuit?: () => void
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
  const themeState = useTheme()
  const { theme } = themeState
  const dimensions = useTerminalDimensions()
  const sync = useSync()
  const project = useProject()
  const jnoccio = useJnoccioBootStatus()
  useStartupQuit(props.onQuit)
  const mountedAt = performance.now()
  const [entries, setEntries] = createSignal<BootEntry[]>([])
  const [now, setNow] = createSignal(performance.now())
  const [dismissed, setDismissed] = createSignal(false)
  const [revealed, setRevealed] = createSignal(1)

  const cursor = () => entries().length
  const elapsed = () => now() - mountedAt
  const readyAllowed = () => props.ready() && elapsed() >= MIN_SHOW_MS

  // Tick used for spinner glyph + pulse color animation.
  const tick = setInterval(() => setNow(performance.now()), 90)
  onCleanup(() => clearInterval(tick))

  const bootLines = createMemo<BootLine[]>(() => {
    const size = dimensions()
    const dir = project.instance.directory() || process.cwd()
    const scorePresent = fs.existsSync(path.join(dir, "agent", "repo-score.json"))
    const jnoccioStatus = jnoccio()
    const syncStatus = sync.status

    return [
      {
        phase: "renderer",
        message: "ready",
        status: "done",
      },
      {
        phase: "terminal",
        message: size.width && size.height ? `${size.width}x${size.height}` : "detecting size",
        status: size.width && size.height ? "done" : "running",
      },
      {
        phase: "theme",
        message: themeState.ready ? `${themeState.selected} ${themeState.mode()}` : "loading",
        status: themeState.ready ? "done" : "running",
      },
      {
        phase: "plugins",
        message: props.ready() ? "initialized" : "initializing",
        status: props.ready() ? "done" : "running",
      },
      {
        phase: "sync",
        message: syncStatus === "complete" ? "ready" : syncStatus,
        status: syncStatus === "complete" ? "done" : syncStatus === "partial" ? "warn" : "running",
      },
      {
        phase: "jnoccio",
        message:
          jnoccioStatus === "ready"
            ? "ready"
            : jnoccioStatus === "unavailable"
              ? "not installed"
              : jnoccioStatus,
        status:
          jnoccioStatus === "ready"
            ? "done"
            : jnoccioStatus === "failed" || jnoccioStatus === "unavailable"
              ? "warn"
              : "running",
      },
      {
        phase: "jankurai",
        message: scorePresent ? "score present" : "score missing",
        status: scorePresent ? "done" : "warn",
      },
      {
        phase: "Jekko",
        message: props.ready() ? "ready" : "waiting",
        status: props.ready() ? "done" : "running",
      },
    ]
  })

  const revealer = setInterval(() => {
    setRevealed((count) => Math.min(bootLines().length, count + 1))
  }, 180)
  onCleanup(() => clearInterval(revealer))

  createEffect(() => {
    const lines = bootLines().slice(0, revealed())
    setEntries(
      lines.map((line, index) => ({
        ts: Math.min(performance.now() - mountedAt, index * 180),
        msg: line.message,
        status: line.status,
      })),
    )
    if (readyAllowed()) setRevealed(bootLines().length)
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
          msg: "splash hit 15s cap with plugins not ready",
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

  const loadingBand = createMemo(() => {
    const width = dimensions().width ?? 0
    const cells = Math.max(12, Math.floor(width * 0.34) - 4)
    const dense = spinnerFrame().repeat(2)
    const sparse = " "
    const scan = Array.from({ length: cells }, (_, index) => {
      const cycle = index % 6
      if (cycle === 0) return dense
      if (cycle === 1) return dense
      if (cycle === 2) return sparse
      if (cycle === 3) return spinnerFrame()
      if (cycle === 4) return sparse
      return spinnerFrame()
    }).join("")
    return scan
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
            const isFinal = () => index() === visibleEntries().length - 1 && entry.msg === "ready"
            const line = bootLines()[index()]
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
        <text fg={theme.textMuted}>[q] quit</text>
        <box height={1} />
        <text fg={loadingPulse()}>{loadingBand()}</text>
      </box>
    </box>
  )
}
