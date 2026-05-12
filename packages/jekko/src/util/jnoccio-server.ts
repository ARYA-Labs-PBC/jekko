// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import { spawn } from "child_process"
import fs from "fs"
import path from "path"
import * as Log from "@jekko-ai/core/util/log"
import { ensureJnoccioRpcToken } from "./jnoccio-token"

const JNOCCIO_FUSION_PORT = 4317
const HEALTH_TIMEOUT_MS = 2000
const POST_SPAWN_HEALTH_DELAY_MS = 1500
const POST_SPAWN_HEALTH_RETRIES = 6

const log = Log.create({ service: "jnoccio-server" })

let spawnInProgress = false

/**
 * Checks whether the Jnoccio Fusion server is reachable on its default port.
 */
async function isServerReachable(): Promise<boolean> {
  try {
    const ctrl = new AbortController()
    const timer = setTimeout(() => ctrl.abort(), HEALTH_TIMEOUT_MS)
    const response = await fetch(`http://127.0.0.1:${JNOCCIO_FUSION_PORT}/v1/models`, {
      method: "GET",
      signal: ctrl.signal,
    })
    clearTimeout(timer)
    return response.ok || response.status === 404 || response.status === 401
  } catch {
    return false
  }
}

/**
 * Finds the jnoccio-fusion binary inside the repo.
 * Prefers release build over debug build.
 */
function findBinary(repoRoot: string): string | undefined {
  const fusionRoot = path.join(repoRoot, "jnoccio-fusion")
  const candidates = [
    path.join(fusionRoot, "target", "release", "jnoccio-fusion"),
    path.join(fusionRoot, "target", "debug", "jnoccio-fusion"),
  ]

  for (const candidate of candidates) {
    try {
      const stat = fs.statSync(candidate)
      if (stat.isFile()) return candidate
    } catch {
      // Not found, try next
    }
  }

  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return undefined
}

/**
 * Spawns the jnoccio-fusion server as a detached background process.
 * The process survives jekko exit.
 */
function spawnServer(binaryPath: string, cwd: string): boolean {
  try {
    const logFile = path.join(cwd, "state", "server.log")

    // Ensure the state directory exists for the log file
    const stateDir = path.dirname(logFile)
    if (!fs.existsSync(stateDir)) {
      fs.mkdirSync(stateDir, { recursive: true })
    }

    // Resolve (or generate) the RPC bearer token so the /rpc endpoint
    // enforces auth from the moment it starts. The token is persisted
    // to ~/.env.jnoccio-rpc for OpenHuman cloud-mode connections.
    const rpcToken = ensureJnoccioRpcToken()

    // Discover the OpenHuman app version so the boot-check gate's strict
    // `coreVersion === APP_VERSION` comparison passes.  The compatibility
    // bridge echoes this back as its own version via `JNOCCIO_COMPAT_VERSION`.
    const compatVersion = discoverOpenHumanVersion()

    const out = fs.openSync(logFile, "a")
    const err = fs.openSync(logFile, "a")

    const child = spawn(binaryPath, [], {
      cwd,
      detached: true,
      stdio: ["ignore", out, err],
      env: {
        ...process.env,
        RUST_LOG: process.env.RUST_LOG ?? "info",
        JNOCCIO_CORE_TOKEN: rpcToken,
        ...(compatVersion.kind === "found" ? { JNOCCIO_COMPAT_VERSION: compatVersion.version } : {}),
      },
    })

    child.unref()

    log.info("jnoccio-fusion server spawned", {
      pid: child.pid,
      binary: binaryPath,
      cwd,
      logFile,
      rpcAuthEnabled: true,
      compatVersion: compatVersion.kind === "found" ? compatVersion.version : "none",
    })

    return true
  } catch (error) {
    log.error("failed to spawn jnoccio-fusion server", {
      error: error instanceof Error ? error.message : String(error),
      binary: binaryPath,
      cwd,
    })
    return false
  }
}

/**
 * Ensures the Jnoccio Fusion server is running.
 * If it's not reachable on port 4317, finds and spawns the binary.
 * This is safe to call multiple times — it's a no-op if the server is already up.
 *
 * Call this:
 * 1. After a successful unlock
 * 2. On provider initialization when jnoccio-fusion is configured
 */
export async function ensureJnoccioFusionServer(repoRoot: string): Promise<void> {
  // Guard against concurrent spawn attempts
  if (spawnInProgress) return
  spawnInProgress = true

  try {
    // Already running?
    if (await isServerReachable()) {
      log.info("jnoccio-fusion server already running")
      return
    }

    const fusionRoot = path.join(repoRoot, "jnoccio-fusion")
    if (!fs.existsSync(fusionRoot)) {
      log.info("jnoccio-fusion directory not found, skipping server start")
      return
    }

    const binaryPath = findBinary(repoRoot)
    if (!binaryPath) {
      log.warn("jnoccio-fusion binary not found, run 'cargo build' in jnoccio-fusion/")
      return
    }

    log.info("starting jnoccio-fusion server", { binary: binaryPath })
    const spawned = spawnServer(binaryPath, fusionRoot)
    if (!spawned) return

    // Wait for the server to become reachable
    for (let i = 0; i < POST_SPAWN_HEALTH_RETRIES; i++) {
      await new Promise((resolve) => setTimeout(resolve, POST_SPAWN_HEALTH_DELAY_MS))
      if (await isServerReachable()) {
        log.info("jnoccio-fusion server is ready")
        return
      }
    }

    log.warn("jnoccio-fusion server spawned but not yet reachable after retries")
  } finally {
    spawnInProgress = false
  }
}

/**
 * Try to discover the OpenHuman desktop app version.
 * This is needed so the jnoccio-fusion RPC bridge can echo back the correct
 * version during the boot-check gate's strict equality comparison.
 *
 * Search order:
 *   1. JNOCCIO_COMPAT_VERSION env var (explicit override)
 *   2. Installed OpenHuman.app (macOS Info.plist — the actual running app)
 *   3. ~/Code/openhuman/app/package.json (dev source)
 */
type VersionLookup = { kind: "found"; version: string } | { kind: "missing"; reason: string }

function discoverOpenHumanVersion(): VersionLookup {
  // Explicit override wins.
  const fromEnv = process.env.JNOCCIO_COMPAT_VERSION?.trim()
  if (fromEnv) return { kind: "found", version: fromEnv }

  // Check the installed macOS app first — this is what the user actually runs.
  const installedVersion = readInstalledAppVersion()
  if (installedVersion.kind === "found") return installedVersion

  // Fall back to source package.json for dev builds.
  const home = process.env.HOME ?? process.env.USERPROFILE ?? ""
  const candidates = [
    path.join(home, "Code", "openhuman", "app", "package.json"),
    path.join(home, "code", "openhuman", "app", "package.json"),
  ]

  for (const candidate of candidates) {
    try {
      const raw = fs.readFileSync(candidate, "utf8")
      const parsed = JSON.parse(raw)
      if (
        typeof parsed === "object" &&
        parsed !== null &&
        !Array.isArray(parsed) &&
        "version" in parsed &&
        typeof (parsed as Record<string, unknown>).version === "string"
      ) {
        const version = (parsed as Record<string, string>).version.trim()
        if (version) {
          log.info("discovered openhuman version from source", { version, source: candidate })
          return { kind: "found", version }
        }
      }
    } catch {
      // Not found, try next.
    }
  }

  return { kind: "missing", reason: "openhuman version not discovered" }
}

/**
 * Read the version from the installed OpenHuman.app on macOS.
 * Checks both /Applications and ~/Applications.
 */
function readInstalledAppVersion(): VersionLookup {
  if (process.platform !== "darwin") return { kind: "missing", reason: "non-mac platform" }

  const home = process.env.HOME ?? ""
  const plistPaths = [
    path.join(home, "Applications", "OpenHuman.app", "Contents", "Info.plist"),
    "/Applications/OpenHuman.app/Contents/Info.plist",
  ]

  for (const plist of plistPaths) {
    try {
      if (!fs.existsSync(plist)) continue
      const { execSync } = require("child_process") as typeof import("child_process")
      const version = execSync(
        `defaults read "${path.dirname(plist).replace(/\/Contents$/, "")}" CFBundleShortVersionString`,
        { encoding: "utf8", timeout: 2000 }
      ).trim()
      if (version && /^\d+\.\d+/.test(version)) {
        log.info("discovered openhuman version from installed app", { version, source: plist })
        return { kind: "found", version }
      }
    } catch {
      // Not readable, try next.
    }
  }

  return { kind: "missing", reason: "installed app not found" }
}
