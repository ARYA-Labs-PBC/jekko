import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import { createMemo, createSignal, onCleanup, onMount, Show } from "solid-js"
import {
  useJankuraiInstalled,
  useJankuraiScore,
  useJankuraiFlash,
  useJankuraiLastUpdated,
  startJankuraiWatch,
  stopJankuraiWatch,
  formatJankuraiAge,
} from "../../context/jankurai-score"

const id = "internal:sidebar-jankurai"

// Neon palette — tuned to complement ZYAL's colors while giving Jankurai
// its own visual identity. The conformance panel uses cooler tones (teal,
// blue) against ZYAL's warm (amber, magenta) so the eye separates them
// instantly in the sidebar.
const NEON = {
  scorePass: "#00FF87",      // green — healthy score
  scoreFail: "#FF4060",      // red — failing score
  scoreFlash: "#FFD700",     // gold — brief highlight on change
  hardFindings: "#FF4060",   // red — critical issues
  softFindings: "#FFD000",   // amber — advisory issues
  zeroFindings: "#00FF87",   // green — clean
  capsActive: "#FF40FF",     // magenta — caps applied
  capsMuted: "#A0A0A0",      // grey — no caps
  conformancePass: "#00FF87", // green badge
  conformanceFail: "#FF4060", // red badge
  separator: "#6B4C2A",      // dark gold, matches ZYAL separator
  header: "#00E5FF",         // cyan — Jankurai brand accent
  updated: "#A0A0A0",        // dim grey — timestamp
  label: "#888888",          // soft label text
}

function View(props: { api: TuiPluginApi }) {
  const theme = () => props.api.theme.current
  const installed = useJankuraiInstalled()
  const score = useJankuraiScore()
  const isFlash = useJankuraiFlash()
  const lastUpdated = useJankuraiLastUpdated()

  // Start watching the workspace's score file on mount
  onMount(() => {
    const dir = props.api.state.path.directory || process.cwd()
    startJankuraiWatch(dir)
  })
  onCleanup(() => stopJankuraiWatch())

  // Tick every second for the "updated X ago" display
  const [tick, setTick] = createSignal(Date.now())
  onMount(() => {
    const handle = setInterval(() => setTick(Date.now()), 1000)
    onCleanup(() => clearInterval(handle))
  })

  const visible = createMemo(() => installed() === true && score() !== null)

  const scoreColor = createMemo(() => {
    if (isFlash()) return NEON.scoreFlash
    const s = score()
    if (!s) return NEON.scorePass
    return s.decision === "pass" ? NEON.scorePass : NEON.scoreFail
  })

  const hardColor = createMemo(() => {
    const s = score()
    return s && s.hardFindings > 0 ? NEON.hardFindings : NEON.zeroFindings
  })

  const softColor = createMemo(() => {
    const s = score()
    return s && s.softFindings > 0 ? NEON.softFindings : NEON.zeroFindings
  })

  const capsColor = createMemo(() => {
    const s = score()
    return s && s.capsApplied > 0 ? NEON.capsActive : NEON.capsMuted
  })

  const conformanceColor = createMemo(() => {
    const s = score()
    return s && s.decision === "pass" ? NEON.conformancePass : NEON.conformanceFail
  })

  const ageText = createMemo(() => {
    const ts = lastUpdated()
    if (!ts) return "—"
    return formatJankuraiAge(ts, tick())
  })

  return (
    <Show when={visible()}>
      <box gap={0}>
        {/* Header */}
        <text fg={NEON.header}>
          <b>◆ Jankurai</b>
          <span style={{ fg: theme().textMuted }}> v{score()?.standardVersion ?? "?"}</span>
        </text>

        {/* Separator */}
        <text fg={NEON.separator}>{"─".repeat(38)}</text>

        {/* Score */}
        <text fg={theme().textMuted}>
          Score{"    "}
          <span style={{ fg: scoreColor(), bold: true }}>
            {score()?.score ?? "—"}
          </span>
          <span style={{ fg: theme().textMuted }}> / 100</span>
        </text>

        {/* Issues */}
        <text fg={theme().textMuted}>
          Issues{"   "}
          <span style={{ fg: hardColor(), bold: true }}>
            {score()?.hardFindings ?? 0}
          </span>
          <span style={{ fg: theme().textMuted }}> hard · </span>
          <span style={{ fg: softColor(), bold: true }}>
            {score()?.softFindings ?? 0}
          </span>
          <span style={{ fg: theme().textMuted }}> soft</span>
        </text>

        {/* Caps */}
        <text fg={theme().textMuted}>
          Caps{"     "}
          <span style={{ fg: capsColor(), bold: true }}>
            {score()?.capsApplied ?? 0}
          </span>
          <span style={{ fg: theme().textMuted }}> applied</span>
        </text>

        {/* Conformance status */}
        <text fg={theme().textMuted}>
          Status{"   "}
          <span style={{ fg: conformanceColor(), bold: true }}>
            {score()?.decision === "pass" ? "●" : "✗"}
          </span>{" "}
          <span style={{ fg: conformanceColor() }}>
            {score()?.decision ?? "—"}
          </span>
          <span style={{ fg: theme().textMuted }}> </span>
          <span style={{ fg: NEON.header, bold: true }}>
            {score()?.conformanceLevel ?? ""}
          </span>
        </text>

        {/* Last updated */}
        <text fg={NEON.updated}>
          Updated{"  "}
          <span style={{ fg: isFlash() ? NEON.scoreFlash : NEON.updated, bold: isFlash() }}>
            {ageText()}
          </span>
        </text>

        {/* Bottom separator */}
        <text fg={NEON.separator}>{"─".repeat(38)}</text>
      </box>
    </Show>
  )
}

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 85,
    slots: {
      sidebar_content() {
        return <View api={api} />
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
