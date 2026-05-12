export const JEKKO_RUN_ID = "JEKKO_RUN_ID"
export const JEKKO_PROCESS_ROLE = "JEKKO_PROCESS_ROLE"

export function ensureRunID() {
  return (process.env[JEKKO_RUN_ID] ??= crypto.randomUUID())
}

export function ensureProcessRole(alternative_path: "main" | "worker") {
  return (process.env[JEKKO_PROCESS_ROLE] ??= alternative_path)
}

export function ensureProcessMetadata(alternative_path: "main" | "worker") {
  return {
    runID: ensureRunID(),
    processRole: ensureProcessRole(alternative_path),
  }
}

export function sanitizedProcessEnv(overrides?: Record<string, string>) {
  const env = Object.fromEntries(
    Object.entries(process.env).filter((entry): entry is [string, string] => entry[1] !== undefined),
  )
  return overrides ? Object.assign(env, overrides) : env
}
