/**
 * Capability / Repo-Intel context.
 *
 * Reads and watches `<repoRoot>/agent/repo-score.json` for the Phase 6C
 * "Capability" pane. Mirrors the watch pattern from `context/theme.tsx`:
 *
 *   - `fs.watch()` for change events (debounced to 300ms)
 *   - falls back to creation-polling when the file doesn't yet exist
 *   - subscribes to SIGUSR2 for a manual reload (same hook theme.tsx uses)
 *
 * Exposes a `useCapability()` accessor returning a stable `CapabilityState`
 * shape backed by `solid-js/store`. Consumers stay reactive by calling the
 * accessor inside their JSX (e.g. `state().score`).
 *
 * The watcher is started imperatively via `startCapabilityWatch(directory)`
 * (see pane-capability.tsx onMount) — the same pattern jankurai-score.ts
 * uses — so the repo root can come from `props.api.state.path.directory`
 * with a `process.cwd()` fallback.
 */
import fs from "fs"
import path from "path"
import { createStore } from "solid-js/store"

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type CapabilityDecision = "pass" | "fail" | "advisory" | "unknown"

export type CapabilityState = {
  score: number
  conformanceClaimed: string
  conformanceObserved: string
  decision: CapabilityDecision
  hardFindings: number
  softFindings: number
  capsApplied: number
  blockers: string[]
  hardRules: { id: string; max_score: number }[]
  generatedAt: number | undefined // ms since epoch
  standard: string
  standardVersion: string
  loaded: boolean
  error: string | undefined
}

const EMPTY: CapabilityState = {
  score: 0,
  conformanceClaimed: "",
  conformanceObserved: "",
  decision: "unknown",
  hardFindings: 0,
  softFindings: 0,
  capsApplied: 0,
  blockers: [],
  hardRules: [],
  generatedAt: undefined,
  standard: "",
  standardVersion: "",
  loaded: false,
  error: undefined,
}

// ---------------------------------------------------------------------------
// Reactive store + accessor
// ---------------------------------------------------------------------------

const [store, setStore] = createStore<CapabilityState>({ ...EMPTY })

/**
 * Accessor returning the latest CapabilityState. Reading any property inside
 * a Solid tracking scope (createMemo / JSX) will re-run on change.
 */
export function useCapability(): () => CapabilityState {
  return () => store
}

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

/**
 * Map the raw `repo-score.json` shape to CapabilityState. Returns `undefined`
 * when the JSON is malformed in a way that prevents us from rendering at all
 * (e.g. score is not a number). Missing optional fields fall back to sane
 * defaults.
 */
export function parseCapabilityJson(raw: string): CapabilityState | undefined {
  let obj: unknown
  try {
    obj = JSON.parse(raw)
  } catch {
    return undefined
  }
  if (!isPlainObject(obj)) return undefined
  if (typeof obj.score !== "number") return undefined

  const decision = isPlainObject(obj.decision) ? obj.decision : {}

  // generated_at is a string-encoded unix-seconds timestamp in this schema.
  // Convert to ms for downstream consumers; treat anything non-finite as
  // undefined so the UI can show a meaningful empty state instead of NaN.
  let generatedAt: number | undefined = undefined
  const ga = obj.generated_at
  if (typeof ga === "string" || typeof ga === "number") {
    const seconds = Number(ga)
    if (Number.isFinite(seconds) && seconds > 0) {
      generatedAt = Math.floor(seconds * 1000)
    }
  }

  const blockers = Array.isArray(obj.conformance_blockers)
    ? obj.conformance_blockers.filter((x): x is string => typeof x === "string")
    : []

  const hardRules = Array.isArray(obj.hard_rules)
    ? obj.hard_rules
        .filter(isPlainObject)
        .filter((r): r is { id: string; max_score: number } => typeof r.id === "string" && typeof r.max_score === "number")
        .map((r) => ({ id: r.id, max_score: r.max_score }))
    : []

  const capsApplied = Array.isArray(obj.caps_applied) ? obj.caps_applied.length : 0

  // Prefer the explicit `conformance_decision` string. Fall back to
  // decision.passed (boolean) when the explicit field is missing.
  const decisionRaw =
    typeof obj.conformance_decision === "string" ? obj.conformance_decision : undefined
  let decisionNorm: CapabilityDecision
  if (decisionRaw === "pass" || decisionRaw === "fail" || decisionRaw === "advisory") {
    decisionNorm = decisionRaw
  } else if (typeof decision.passed === "boolean") {
    decisionNorm = decision.passed ? "pass" : "fail"
  } else {
    decisionNorm = "unknown"
  }

  return {
    score: obj.score,
    conformanceClaimed:
      typeof obj.claimed_conformance_level === "string" ? obj.claimed_conformance_level : "",
    conformanceObserved:
      typeof obj.observed_conformance_level === "string" ? obj.observed_conformance_level : "",
    decision: decisionNorm,
    hardFindings: typeof decision.hard_findings === "number" ? decision.hard_findings : 0,
    softFindings: typeof decision.soft_findings === "number" ? decision.soft_findings : 0,
    capsApplied,
    blockers,
    hardRules,
    generatedAt,
    standard: typeof obj.standard === "string" ? obj.standard : "",
    standardVersion: typeof obj.standard_version === "string" ? obj.standard_version : "",
    loaded: true,
    error: undefined,
  }
}

// ---------------------------------------------------------------------------
// Watch lifecycle
// ---------------------------------------------------------------------------

let watcher: fs.FSWatcher | undefined
let creationPollTimer: ReturnType<typeof setInterval> | undefined
let debounceTimer: ReturnType<typeof setTimeout> | undefined
let activePath: string | undefined
let sigusr2Handler: (() => void) | undefined

function applyState(next: CapabilityState) {
  setStore(next)
}

function applyError(message: string) {
  setStore({
    ...EMPTY,
    loaded: true,
    error: message,
  })
}

function readAndUpdate(scorePath: string) {
  let raw: string
  try {
    raw = fs.readFileSync(scorePath, "utf-8")
  } catch (err) {
    // File deleted or mid-rotation. Treat as error so the empty state shows.
    applyError(err instanceof Error ? err.message : "read failed")
    return
  }
  const parsed = parseCapabilityJson(raw)
  if (!parsed) {
    applyError("agent/repo-score.json is malformed")
    return
  }
  applyState(parsed)
}

function debouncedRead(p: string) {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => readAndUpdate(p), 300)
}

function startWatcher(p: string) {
  if (watcher) {
    try {
      watcher.close()
    } catch {
      // ignore close errors
    }
    watcher = undefined
  }
  try {
    watcher = fs.watch(p, { persistent: false }, () => debouncedRead(p))
    watcher.on("error", () => {
      try {
        watcher?.close()
      } catch {
        // ignore
      }
      watcher = undefined
      startCreationPoll(p)
    })
  } catch {
    // fs.watch throws if path disappears between exists() and watch()
    startCreationPoll(p)
  }
}

function startCreationPoll(p: string) {
  if (creationPollTimer) return
  creationPollTimer = setInterval(() => {
    if (fs.existsSync(p)) {
      clearInterval(creationPollTimer!)
      creationPollTimer = undefined
      readAndUpdate(p)
      startWatcher(p)
    }
  }, 10_000)
  if (typeof creationPollTimer === "object" && creationPollTimer && "unref" in creationPollTimer) {
    ;(creationPollTimer as { unref?: () => void }).unref?.()
  }
}

function ensureSigusr2() {
  if (sigusr2Handler) return
  sigusr2Handler = () => {
    if (activePath) readAndUpdate(activePath)
  }
  process.on("SIGUSR2", sigusr2Handler)
}

function clearSigusr2() {
  if (sigusr2Handler) {
    process.off("SIGUSR2", sigusr2Handler)
    sigusr2Handler = undefined
  }
}

/**
 * Begin watching `<directory>/agent/repo-score.json`. Safe to call multiple
 * times — switches to a new path if `directory` changes. On first call also
 * installs a SIGUSR2 handler for manual reload (matches theme.tsx pattern).
 */
export function startCapabilityWatch(directory: string) {
  const scorePath = path.join(directory, "agent", "repo-score.json")
  if (activePath === scorePath) return
  stopCapabilityWatch()
  activePath = scorePath
  ensureSigusr2()

  if (fs.existsSync(scorePath)) {
    readAndUpdate(scorePath)
    startWatcher(scorePath)
  } else {
    // Mark loaded so the empty state renders instead of a blank pane while
    // we wait for the file to appear.
    setStore({
      ...EMPTY,
      loaded: true,
      error: "agent/repo-score.json not found",
    })
    startCreationPoll(scorePath)
  }
}

/**
 * Tear down all timers, watchers, and the SIGUSR2 handler. Resets the store
 * back to its empty initial state.
 */
export function stopCapabilityWatch() {
  if (watcher) {
    try {
      watcher.close()
    } catch {
      // ignore
    }
    watcher = undefined
  }
  if (creationPollTimer) {
    clearInterval(creationPollTimer)
    creationPollTimer = undefined
  }
  if (debounceTimer) {
    clearTimeout(debounceTimer)
    debounceTimer = undefined
  }
  clearSigusr2()
  activePath = undefined
  setStore({ ...EMPTY })
}

// ---------------------------------------------------------------------------
// Formatting helpers (exported for unit testing / reuse)
// ---------------------------------------------------------------------------

/**
 * Render an epoch-ms timestamp as "Xs ago" / "Xm ago" / "Xh ago" / "Xd ago".
 * Returns "just now" for ages under a minute and "—" when `generatedAt` is
 * undefined.
 */
export function formatCapabilityAge(generatedAtMs: number | undefined, nowMs: number): string {
  if (!generatedAtMs) return "—"
  const ageSec = Math.max(0, Math.floor((nowMs - generatedAtMs) / 1000))
  if (ageSec < 60) return "just now"
  if (ageSec < 3600) return `${Math.floor(ageSec / 60)}m ago`
  if (ageSec < 86400) return `${Math.floor(ageSec / 3600)}h ago`
  return `${Math.floor(ageSec / 86400)}d ago`
}

/**
 * Strip the "HL" prefix from a conformance level (e.g. "HL3" -> "L3"). When
 * the input doesn't start with "HL" the original value is returned
 * unchanged. Empty/missing input falls back to "—".
 */
export function formatConformanceLevel(level: string): string {
  if (!level) return "—"
  if (level.startsWith("HL")) return "L" + level.slice(2)
  return level
}
