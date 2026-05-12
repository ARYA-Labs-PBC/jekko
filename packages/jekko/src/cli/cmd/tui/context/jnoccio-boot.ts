/**
 * Jnoccio Fusion server auto-boot lifecycle.
 *
 * Runs once at TUI startup (called from App onMount).
 * Exposes a live signal so UI components can show status.
 *
 * The boot sequence is:
 * 1. Always health-check 127.0.0.1:4317 first — if a server is already
 *    running (even from another repo or a manual launch), mark "ready".
 * 2. Only if unreachable AND the current repo has jnoccio-fusion configured
 *    do we attempt to auto-spawn the server.
 * 3. This means any Jekko instance, from any directory, can connect to an
 *    already-running jnoccio server without needing jnoccio-fusion/ locally.
 */
import { createSignal } from "solid-js"
import { ensureJnoccioFusionServer } from "@/util/jnoccio-server"
import {
  isJnoccioFusionConfigured,
  isJnoccioFusionUnlocked,
  repoRootFromSource,
  writeGlobalJnoccioRepoRoot,
} from "@/util/jnoccio-unlock"

const JNOCCIO_HEALTH_URL = "http://127.0.0.1:4317/health"
const HEALTH_TIMEOUT_MS = 3000

export type JnoccioBootStatus =
  | "idle"        // haven't checked yet
  | "checking"    // checking if server is up
  | "starting"    // spawning server
  | "ready"       // server is reachable
  | "unavailable" // not configured / binary missing AND server not running
  | "failed"      // spawn failed or unreachable after retries

const [bootStatus, setBootStatus] = createSignal<JnoccioBootStatus>("idle")
const [modelCount, setModelCount] = createSignal<number | null>(null)

export function useJnoccioBootStatus() {
  return bootStatus
}

export function useJnoccioModelCount() {
  return modelCount
}

let booted = false
let pollTimer: ReturnType<typeof setInterval> | undefined

// How often to re-check server reachability after the initial boot. Without
// this, a single transient health-check miss (e.g. server still starting,
// network blip) would lock the status to "unavailable"/"failed" forever even
// after the server comes up. 5s gives near-instant recovery without spamming.
const REPOLL_INTERVAL_MS = 5000

async function isServerReachable(): Promise<{ reachable: boolean; models: number | null }> {
  try {
    const ctrl = new AbortController()
    const timer = setTimeout(() => ctrl.abort(), HEALTH_TIMEOUT_MS)
    const res = await fetch(JNOCCIO_HEALTH_URL, {
      method: "GET",
      signal: ctrl.signal,
    })
    clearTimeout(timer)
    const ok = res.ok || res.status === 404 || res.status === 401
    let models: number | null = null
    if (res.ok) {
      try {
        // /health returns { ok, available_models, keyed_models, ... }
        // keyed_models = models with valid API keys (actually routable)
        const json = await res.json() as { available_models?: number; keyed_models?: number }
        if (json && typeof json.keyed_models === "number") {
          models = json.keyed_models
        } else if (json && typeof json.available_models === "number") {
          models = json.available_models
        }
      } catch {
        // response wasn't JSON, that's fine
      }
    }
    return { reachable: ok, models }
  } catch {
    return { reachable: false, models: null }
  }
}

function startBackgroundRepoll() {
  if (pollTimer) return
  pollTimer = setInterval(async () => {
    const result = await isServerReachable()
    const current = bootStatus()
    if (result.reachable && current !== "ready") {
      setBootStatus("ready")
    } else if (!result.reachable && current === "ready") {
      setBootStatus("unavailable")
    }
    if (result.models !== null) {
      setModelCount(result.models)
    } else if (!result.reachable) {
      setModelCount(null)
    }
  }, REPOLL_INTERVAL_MS)
  if (typeof pollTimer === "object" && pollTimer && "unref" in pollTimer) {
    ;(pollTimer as { unref?: () => void }).unref?.()
  }
}

/**
 * Call once at TUI startup. Safe to call multiple times — no-ops after first.
 */
export async function bootJnoccioFusion(): Promise<void> {
  if (booted) return
  booted = true

  try {
    if (process.env.JEKKO_DISABLE_JNOCCIO_BOOT === "1") {
      setBootStatus("unavailable")
      setModelCount(null)
      return
    }

    setBootStatus("checking")

    // Step 1: Always check if the server is already running.
    // Works from any directory — doesn't require jnoccio-fusion/ locally.
    const initial = await isServerReachable()
    if (initial.reachable) {
      setBootStatus("ready")
      if (initial.models !== null) setModelCount(initial.models)
      startBackgroundRepoll()
      return
    }

    // Step 2: Server is not running. Try auto-spawn only if this repo
    // has jnoccio-fusion configured (has the directory + .env.jnoccio).
    let root: string | undefined
    try {
      root = repoRootFromSource()
    } catch {
      // Can't find jnoccio-fusion in the repo tree — that's fine,
      // it just means we can't auto-spawn.
    }

    if (!root || !isJnoccioFusionConfigured(root)) {
      setBootStatus("unavailable")
      // Even when local repo is not configured, the server may come up later
      // (another worktree may unlock + spawn it). Keep polling so the UI
      // recovers automatically the moment the server becomes reachable.
      startBackgroundRepoll()
      return
    }

    // Promote this repo to the global jnoccio registry on every successful
    // discovery, so future jekko launches from any directory on the machine
    // (~/code/xdoug/, ~/anything-else/) find this jnoccio-fusion source.
    if (isJnoccioFusionUnlocked(root)) {
      writeGlobalJnoccioRepoRoot(root)
    }

    // Step 3: Auto-spawn and verify.
    setBootStatus("starting")
    await ensureJnoccioFusionServer(root)

    const postSpawn = await isServerReachable()
    if (postSpawn.reachable) {
      setBootStatus("ready")
      if (postSpawn.models !== null) setModelCount(postSpawn.models)
    } else {
      setBootStatus("failed")
    }
    startBackgroundRepoll()
  } catch {
    setBootStatus("failed")
  }
}
