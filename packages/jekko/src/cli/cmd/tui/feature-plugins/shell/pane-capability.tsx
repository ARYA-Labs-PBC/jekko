/**
 * Shell LEFT panel — Capability / Repo-Intel pane (Phase 6C of TUIbomb).
 *
 * Default pane the user lands on after hitting Enter on Home. Registers into
 * the `shell_left_active_pane` slot and wins only when
 * `active_pane === "capability"`. Renders a compact, repo-wide intel summary
 * sourced from `<repoRoot>/agent/repo-score.json`:
 *
 *   • Title with relative "updated Xm ago" timestamp
 *   • 17-cell score sparkline + "<score> / 100"
 *   • Findings / caps table
 *   • Decision row (PASS / FAIL / advisory)
 *   • Conformance level (HL3 -> L3)
 *   • Standard version row
 *   • Mission gaps section (top 3 blockers or hard_rules secondary list)
 *   • Empty state when the score file is missing or unparseable
 *
 * The pane re-reads on file changes (debounced 300ms) and on SIGUSR2 — see
 * `context/capability.ts` for the watch implementation.
 */
import { createMemo, createSignal, For, onCleanup, onMount } from "solid-js"
import type { TuiPlugin, TuiPluginApi, TuiPluginModule } from "@jekko-ai/plugin/tui"
import {
  formatCapabilityAge,
  formatConformanceLevel,
  startCapabilityWatch,
  stopCapabilityWatch,
  useCapability,
  type CapabilityState,
} from "../../context/capability"

const id = "internal:shell-pane-capability"

const DEFAULT_PANE_WIDTH = 24

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function truncate(text: string, max: number): string {
  if (text.length <= max) return text
  if (max <= 1) return text.slice(0, max)
  return text.slice(0, max - 1) + "…"
}

function decisionLabel(decision: CapabilityState["decision"]): string {
  if (decision === "pass") return "PASS"
  if (decision === "fail") return "FAIL"
  if (decision === "advisory") return "ADVISORY"
  return "—"
}

function gapsFor(state: CapabilityState, limit: number): string[] {
  if (state.blockers.length > 0) {
    return state.blockers.slice(0, limit)
  }
  // No blockers means the audit passed. Use the hard rules with the lowest
  // cap (i.e. most punishing) as a "what could go wrong" teaser. The pane
  // stays useful even on a clean repo.
  return state.hardRules
    .slice()
    .sort((a, b) => a.max_score - b.max_score)
    .slice(0, limit)
    .map((r) => r.id)
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function Sparkline(props: { api: TuiPluginApi; score: number; width: number }) {
  const theme = () => props.api.theme.current
  const cells = createMemo(() => Math.max(8, Math.min(17, props.width - 9)))
  const filled = createMemo(() => {
    const raw = Math.round((props.score / 100) * cells())
    return Math.max(0, Math.min(cells(), raw))
  })

  return (
    <box flexDirection="row" gap={1}>
      <text>
        <span style={{ fg: theme().accent }}>{"▰".repeat(filled())}</span>
        <span style={{ fg: theme().borderSubtle }}>{"▱".repeat(cells() - filled())}</span>
      </text>
      <text fg={theme().text}>
        {props.score}
        <span style={{ fg: theme().textMuted }}> / 100</span>
      </text>
    </box>
  )
}

function FindingsTable(props: { api: TuiPluginApi; state: CapabilityState }) {
  const theme = () => props.api.theme.current
  const hard = createMemo(() => props.state.hardFindings)
  const soft = createMemo(() => props.state.softFindings)
  const total = createMemo(() => hard() + soft())
  const caps = createMemo(() => props.state.capsApplied)
  const hardColor = createMemo(() => (hard() > 0 ? theme().error : theme().success))

  return (
    <box flexDirection="column">
      <text fg={theme().textMuted}>
        hard{" "}
        <span style={{ fg: hardColor(), bold: true }}>{hard()}</span>
        {"    "}
        soft <span style={{ fg: theme().text, bold: true }}>{soft()}</span>
      </text>
      <text fg={theme().textMuted}>
        total <span style={{ fg: theme().text, bold: true }}>{total()}</span>
        {"   "}
        caps <span style={{ fg: theme().text, bold: true }}>{caps()}</span>
      </text>
    </box>
  )
}

function DecisionRow(props: { api: TuiPluginApi; state: CapabilityState }) {
  const theme = () => props.api.theme.current
  const label = createMemo(() => decisionLabel(props.state.decision))
  const color = createMemo(() => {
    if (props.state.decision === "pass") return theme().success
    if (props.state.decision === "fail") return theme().error
    if (props.state.decision === "advisory") return theme().warning
    return theme().textMuted
  })

  return (
    <text fg={theme().textMuted}>
      Decision{"     "}
      <span style={{ fg: color(), bold: true }}>{label()}</span>
    </text>
  )
}

function ConformanceRow(props: { api: TuiPluginApi; state: CapabilityState }) {
  const theme = () => props.api.theme.current
  const level = createMemo(() => formatConformanceLevel(props.state.conformanceObserved))
  return (
    <text fg={theme().textMuted}>
      Conformance{"   "}
      <span style={{ fg: theme().text, bold: true }}>{level()}</span>
    </text>
  )
}

function StandardRow(props: { api: TuiPluginApi; state: CapabilityState }) {
  const theme = () => props.api.theme.current
  const version = createMemo(() => props.state.standardVersion || "—")
  return (
    <text fg={theme().textMuted}>
      Standard{"      "}
      <span style={{ fg: theme().text }}>v{version()}</span>
    </text>
  )
}

function SectionHeader(props: { api: TuiPluginApi; label: string; width: number }) {
  const theme = () => props.api.theme.current
  // "─ <label> ─" padded to pane width with a trailing rule so the section
  // visually separates from the row above without burning a whole line.
  const line = createMemo(() => {
    const label = ` ${props.label} `
    const remaining = Math.max(0, props.width - label.length - 2)
    const left = "─"
    const right = "─".repeat(remaining + 1)
    return `${left}${label}${right}`
  })
  return <text fg={theme().textMuted}>{line()}</text>
}

function MissionGaps(props: { api: TuiPluginApi; state: CapabilityState; width: number }) {
  const theme = () => props.api.theme.current
  const gaps = createMemo(() => gapsFor(props.state, 3))
  const gapList = gaps()
  return gapList.length > 0 ? (
    <For each={gapList}>
      {(gap) => (
        <text>
          <span style={{ fg: theme().accent }}>▸ </span>
          <span style={{ fg: theme().text }}>{truncate(gap, props.width - 2)}</span>
        </text>
      )}
    </For>
  ) : (
    <text fg={theme().textMuted}>{truncate("No outstanding gaps.", props.width)}</text>
  )
}

function EmptyState(props: { api: TuiPluginApi; width: number; message: string | undefined }) {
  const theme = () => props.api.theme.current
  const message = createMemo(
    () => props.message || "Jankurai score unavailable. Run `jankurai init` or refresh the audit.",
  )
  return (
    <box flexDirection="column" gap={1}>
      <text fg={theme().textMuted}>
        {truncate(message(), props.width)}
      </text>
    </box>
  )
}

// ---------------------------------------------------------------------------
// Root
// ---------------------------------------------------------------------------

function PaneCapability(props: { api: TuiPluginApi; contentWidth: number }) {
  const theme = () => props.api.theme.current
  const state = useCapability()

  // Start watching the workspace's score file on mount. Idempotent — safe to
  // mount/unmount the pane repeatedly when the user switches tabs.
  onMount(() => {
    const dir = props.api.state.path.directory || process.cwd()
    startCapabilityWatch(dir)
  })
  onCleanup(() => stopCapabilityWatch())

  // Tick every second so the "updated Xm ago" label stays fresh without
  // re-reading the file.
  const [tick, setTick] = createSignal(Date.now())
  onMount(() => {
    const handle = setInterval(() => setTick(Date.now()), 1000)
    onCleanup(() => clearInterval(handle))
  })

  const ageText = createMemo(() => formatCapabilityAge(state().generatedAt, tick()))
  const hasError = createMemo(() => Boolean(state().error) || !state().loaded)
  const paneWidth = createMemo(() => Math.max(16, props.contentWidth || DEFAULT_PANE_WIDTH))
  const divider = createMemo(() => "─".repeat(paneWidth()))

  return (
    <scrollbox
      viewportOptions={{ paddingRight: 0 }}
      verticalScrollbarOptions={{ visible: false }}
      flexGrow={1}
    >
      <box flexDirection="column" width="100%">
        <text fg={theme().text}>
          <b>{truncate(`Repo-Intel · updated ${ageText()}`, paneWidth())}</b>
        </text>
        <text fg={theme().borderSubtle}>{divider()}</text>
        {!hasError() ? (
          <>
            <box flexDirection="column" paddingTop={1}>
              <Sparkline api={props.api} score={state().score} width={paneWidth()} />
            </box>
            <box flexDirection="column" paddingTop={1}>
              <FindingsTable api={props.api} state={state()} />
            </box>
            <box flexDirection="column" paddingTop={1}>
              <DecisionRow api={props.api} state={state()} />
              <ConformanceRow api={props.api} state={state()} />
              <StandardRow api={props.api} state={state()} />
            </box>
            <box flexDirection="column" paddingTop={1}>
              <SectionHeader api={props.api} label="Mission gaps" width={paneWidth()} />
              <MissionGaps api={props.api} state={state()} width={paneWidth()} />
            </box>
          </>
        ) : (
          <EmptyState api={props.api} width={paneWidth()} message={state().error} />
        )}
      </box>
    </scrollbox>
  )
}

// ---------------------------------------------------------------------------
// Plugin export
// ---------------------------------------------------------------------------

const tui: TuiPlugin = async (api) => {
  api.slots.register({
    order: 92,
    slots: {
      shell_left_active_pane(_ctx, props) {
        return props.active_pane === "capability" ? (
          <PaneCapability api={api} contentWidth={props.left_content_width ?? DEFAULT_PANE_WIDTH} />
        ) : null
      },
    },
  })
}

const plugin: TuiPluginModule & { id: string } = {
  id,
  tui,
}

export default plugin
