import path from "path"
import { createEffect, onCleanup } from "solid-js"
import { connectJnoccio, disconnectJnoccio } from "@tui/context/jnoccio-ws"
import {
  daemonRunJnoccioConfig,
  daemonRunToZyalMetrics,
  isZyalFlashSourceActive,
  isZyalTerminalStatus,
  recordZyalExit,
  resetZyalMetrics,
  setZyalFlashSource,
  updateZyalMetrics,
} from "@tui/context/zyal-flash"
import {
  activateAutoResearch,
  deactivateAutoResearch,
  detectAutoResearch,
  deriveDaemonDirFromRun,
  updateAutoResearchScores,
} from "@tui/context/autoresearch-state"
import { parseBestState, parseScoreboard } from "@tui/context/autoresearch-parser"
import { zyalExitReasonFromExitJson } from "./session-helpers"

export const DAEMON_TERMINAL_STATUSES = new Set(["satisfied", "aborted", "failed", "paused"])

export function daemonRunIsLive(run: unknown): boolean {
  if (!run || typeof run !== "object") return false
  return !DAEMON_TERMINAL_STATUSES.has(String((run as { status?: unknown }).status))
}

export function daemonRunMatchesSession(run: unknown, sessionID: string): boolean {
  if (!run || typeof run !== "object") return false
  const record = run as { active_session_id?: unknown; root_session_id?: unknown }
  return record.active_session_id === sessionID || record.root_session_id === sessionID
}

export function selectDaemonRunForSession(runs: readonly unknown[], sessionID: string): unknown | undefined {
  const matchesSession = (run: unknown) => daemonRunMatchesSession(run, sessionID)
  return runs.find((run) => matchesSession(run) && daemonRunIsLive(run)) ?? runs.find(matchesSession)
}

export function daemonPollResultForSession(runs: readonly unknown[], sessionID: string) {
  const run = selectDaemonRunForSession(runs, sessionID)
  return {
    run,
    found: !!run,
    live: daemonRunIsLive(run),
    terminal: !!run && !daemonRunIsLive(run),
  }
}

export function shouldPreserveZyalStateOnDaemonPollError(input: {
  promptSubmitted: boolean
  currentRun: unknown | undefined
}): boolean {
  return input.promptSubmitted || daemonRunIsLive(input.currentRun)
}

type DaemonRunRecord = {
  id?: string | number
  run_id?: string | number
  status?: string
  last_error?: string | null
  last_exit_result_json?: unknown
  jnoccio?: unknown
  spec_json?: unknown
  spec?: unknown
  daemon_dir?: string | null
  daemonDir?: string | null
  artifact_root?: string | null
  artifactRoot?: string | null
  paths?: {
    daemon_dir?: string | null
    daemonDir?: string | null
    artifact_root?: string | null
    artifactRoot?: string | null
  }
}

type DaemonRunIdentity = {
  runId?: string
  status?: string
}

type DaemonTerminalReason =
  | { kind: "last_error"; reason: string }
  | { kind: "exit_json"; reason: string }
  | { kind: "status"; reason: string }

function daemonRunIdentity(run: DaemonRunRecord): DaemonRunIdentity {
  const runId = typeof run.id === "string" || typeof run.id === "number" ? String(run.id) : undefined
  const fallbackRunId =
    runId ?? (typeof run.run_id === "string" || typeof run.run_id === "number" ? String(run.run_id) : undefined)
  const status = typeof run.status === "string" ? run.status : undefined
  return { runId: fallbackRunId, status }
}

function daemonTerminalReason(run: DaemonRunRecord, status: string): DaemonTerminalReason {
  if (typeof run.last_error === "string" && run.last_error.trim().length > 0) {
    return { kind: "last_error", reason: run.last_error }
  }

  const exitReason = zyalExitReasonFromExitJson(run.last_exit_result_json)
  if (exitReason) {
    return { kind: "exit_json", reason: exitReason }
  }

  return { kind: "status", reason: status }
}

export function useSessionDaemonPolling(input: {
  sessionID: () => string
  sdk: any
  toast: any
  setOverlay: (value: string | undefined) => void
  setDaemonRun: (run: any) => void
  daemonRun: () => any
}) {
  createEffect(() => {
    const sessionID = input.sessionID()
    let alive = true
    const lastDaemonStatusByRun = new Map<string, string>()

    const refresh = async () => {
      try {
        const response = await input.sdk.fetch(new URL("/daemon", input.sdk.url))
        if (!response.ok) return
        const runs = await response.json()
        if (!Array.isArray(runs)) return
        const run = selectDaemonRunForSession(runs, sessionID)
        if (!isRecord(run)) return
        if (!alive) return

        const identity = daemonRunIdentity(run)
        if (!identity.runId || !identity.status) return
        const runId = identity.runId
        const status = identity.status
        const prevStatus = runId ? lastDaemonStatusByRun.get(runId) : undefined
        if (
          runId &&
          isZyalTerminalStatus(status) &&
          prevStatus !== status &&
          (prevStatus === undefined || !isZyalTerminalStatus(prevStatus))
        ) {
          const reason = daemonTerminalReason(run, status).reason
          recordZyalExit({ runId, status, reason })
          const tone = status === "satisfied" ? "success" : status === "paused" ? "warning" : "error"
          input.toast.show({
            variant: tone === "success" ? "success" : tone === "warning" ? "warning" : "error",
            message: `ZYAL ${status.toUpperCase()}: ${reason}`.slice(0, 200),
            duration: 8000,
          })
        }
        if (runId) lastDaemonStatusByRun.set(runId, status)

        input.setDaemonRun(run)
        const isLiveRun = daemonRunIsLive(run)
        if (isLiveRun) {
          input.setOverlay("jekko-gold")
          setZyalFlashSource("session:daemon", true)
          setZyalFlashSource("prompt:submitted", false)
          updateZyalMetrics(daemonRunToZyalMetrics(run, sessionID))

          const arConfig = detectAutoResearch(run)
          if (arConfig) {
            activateAutoResearch(arConfig)
            void pollAutoResearchScores(input.sdk, run, arConfig.runId)
          } else {
            deactivateAutoResearch()
          }

          const jnoccio = daemonRunJnoccioConfig(run)
          if (jnoccio) {
            connectJnoccio(jnoccio)
          } else {
            disconnectJnoccio()
          }
        } else {
          disconnectJnoccio()
          setZyalFlashSource("session:daemon", false)
          if (run) {
            setZyalFlashSource("prompt:submitted", false)
          }
          input.setOverlay(undefined)
          resetZyalMetrics()
          deactivateAutoResearch()
        }
      } catch {
        if (!alive) return
        if (
          shouldPreserveZyalStateOnDaemonPollError({
            promptSubmitted: isZyalFlashSourceActive("prompt:submitted"),
            currentRun: input.daemonRun(),
          })
        ) {
          return
        }
        input.setDaemonRun(undefined)
        disconnectJnoccio()
        setZyalFlashSource("session:daemon", false)
        input.setOverlay(undefined)
        resetZyalMetrics()
      }
    }

    void refresh()
    const timer = setInterval(refresh, 1000)
    onCleanup(() => {
      alive = false
      clearInterval(timer)
      disconnectJnoccio()
      setZyalFlashSource("session:daemon", false)
      setZyalFlashSource("prompt:submitted", false)
      input.setOverlay(undefined)
      resetZyalMetrics()
      deactivateAutoResearch()
    })
  })
}

async function pollAutoResearchScores(
  sdk: any,
  run: DaemonRunRecord,
  _runId: string,
) {
  try {
    const daemonDir = deriveDaemonDirFromRun(run)
    if (!daemonDir) return
    const baseDir = resolveReadBaseDir(sdk)
    if (!baseDir && !path.isAbsolute(daemonDir)) return
    const scoreboardResp = await readDaemonFile(baseDir, daemonDir, "scoreboard.tsv")
    const scores = scoreboardResp ? parseScoreboard(scoreboardResp) : []
    const bestStateResp = await readDaemonFile(baseDir, daemonDir, "best-state.json")
    const bestState = bestStateResp ? parseBestState(bestStateResp) : null
    const iteration = scores.length > 0 ? Math.max(...scores.map((score) => score.iteration)) : 0
    updateAutoResearchScores({ scores, bestState, iteration })
  } catch {
    // Score files may not exist yet — silently ignore.
  }
}

function resolveReadBaseDir(sdk: any): string | null {
  for (const candidate of [
    sdk.directory,
    sdk.state?.path?.directory,
    sdk.path?.directory,
    sdk.workspace?.directory,
    sdk.project?.instance?.path?.directory,
  ]) {
    if (typeof candidate !== "string") continue
    const trimmed = candidate.trim()
    if (trimmed.length > 0) return trimmed
  }
  return null
}

async function readDaemonFile(baseDir: string | null, daemonDir: string, filename: string): Promise<string | null> {
  try {
    const fs = await import("node:fs/promises")
    const resolved = path.isAbsolute(daemonDir)
      ? daemonDir
      : baseDir
        ? path.resolve(baseDir, daemonDir)
        : null
    if (!resolved) return null
    const content = await fs.readFile(path.join(resolved, filename), "utf-8")
    return content
  } catch {
    return null
  }
}

function isRecord(value: unknown): value is Record<string, any> {
  return !!value && typeof value === "object"
}
