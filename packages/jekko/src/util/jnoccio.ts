import { Flag } from "@jekko-ai/core/flag/flag"
import { InstallationVersion } from "@jekko-ai/core/installation/version"
import { ensureProcessMetadata } from "@jekko-ai/core/util/jekko-process"

const HEARTBEAT_INTERVAL_MS = 30_000
const JNOCCIO_HEARTBEAT_ENDPOINT = (process.env.JNOCCIO_FUSION_URL ?? "http://127.0.0.1:4317").replace(/\/$/, "")
const JNOCCIO_HEARTBEAT_PATH = "/v1/jnoccio/agents/heartbeat"

let heartbeatTimer: ReturnType<typeof setInterval> | null = null
let heartbeatStarted = false

export type JnoccioProcessMetadata = ReturnType<typeof ensureProcessMetadata>

export function jnoccioIdentityHeaders(metadata: JnoccioProcessMetadata) {
  return {
    "User-Agent": `jekko/${InstallationVersion}`,
    "x-jekko-client": Flag.JEKKO_CLIENT,
    "x-jekko-run-id": metadata.runID,
    "x-jekko-process-role": metadata.processRole,
    "x-jekko-pid": String(process.pid),
    "x-jekko-version": String(InstallationVersion),
  }
}

async function sendHeartbeat(metadata: JnoccioProcessMetadata) {
  try {
    await fetch(`${JNOCCIO_HEARTBEAT_ENDPOINT}${JNOCCIO_HEARTBEAT_PATH}`, {
      method: "POST",
      headers: jnoccioIdentityHeaders(metadata),
    })
  } catch {
    // Best-effort heartbeat. Ignore transient failures.
  }
}

export function startJnoccioHeartbeat(metadata: JnoccioProcessMetadata) {
  if (heartbeatStarted) return
  heartbeatStarted = true
  void sendHeartbeat(metadata)
  heartbeatTimer = setInterval(() => {
    void sendHeartbeat(metadata)
  }, HEARTBEAT_INTERVAL_MS)
  heartbeatTimer.unref?.()
}
