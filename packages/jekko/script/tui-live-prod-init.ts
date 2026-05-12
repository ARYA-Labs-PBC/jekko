#!/usr/bin/env bun

import fs from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"

const allowedKeys = new Set([
  "JEKKO_API_KEY",
  "JEKKO_LIVE_MODEL",
  "JEKKO_TUI_LIVE_PROD",
  "JNOCCIO_DEFAULT_API_KEY",
  "JNOCCIO_DEFAULT_BASE_URL",
  "JNOCCIO_TUIWRIGHT_E2E",
  "JNOCCIO_TUI_TEST",
  "JNOCCIO_UNLOCK_SECRET_PATH",
])

const home = os.homedir()
const destination = process.env.JEKKO_LIVE_PROD_ENV || path.join(home, ".config", "jekko", "live-prod.env")

function parseEnvLine(line: string) {
  const trimmed = line.trim()
  if (!trimmed || trimmed.startsWith("#")) return
  const match = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$/.exec(trimmed)
  if (!match) return
  const key = match[1]
  if (!allowedKeys.has(key)) return
  return { key, assignment: `${key}=${match[2].trim()}` }
}

function candidateHomeEnvFiles() {
  return fs
    .readdirSync(home, { withFileTypes: true })
    .filter((entry) => entry.isFile() && (entry.name === ".env" || entry.name.endsWith(".env")))
    .map((entry) => path.join(home, entry.name))
    .sort()
}

function existingKeys(text: string) {
  const keys = new Set<string>()
  for (const line of text.split(/\r?\n/)) {
    const parsed = parseEnvLine(line)
    if (parsed) keys.add(parsed.key)
  }
  return keys
}

const files = candidateHomeEnvFiles()
const discovered = new Map<string, string>()
for (const file of files) {
  const text = await fsp.readFile(file, "utf8").catch(() => "")
  for (const line of text.split(/\r?\n/)) {
    const parsed = parseEnvLine(line)
    if (parsed && !discovered.has(parsed.key)) discovered.set(parsed.key, parsed.assignment)
  }
}

if (discovered.size === 0) {
  console.log("No approved Jekko/Jnoccio keys found in ~/*.env, ~/.env, or ~/.*.env. No changes made.")
  process.exit(0)
}

await fsp.mkdir(path.dirname(destination), { recursive: true })
const current = await fsp.readFile(destination, "utf8").catch(() => "")
const currentKeys = existingKeys(current)
const additions = [...discovered.entries()].filter(([key]) => !currentKeys.has(key))

if (additions.length === 0) {
  console.log(`Live prod env already has approved keys: ${destination}`)
  console.log([...discovered.keys()].sort().map((key) => `${key}=<redacted>`).join("\n"))
  process.exit(0)
}

const header = current.trim()
  ? "\n"
  : "# Local live production TUI test keys. Do not commit this file.\n"
const body = additions.map(([, assignment]) => assignment).join("\n") + "\n"
await fsp.writeFile(destination, current + header + body, { mode: 0o600 })
await fsp.chmod(destination, 0o600).catch(() => {})

console.log(`Updated local live prod env: ${destination}`)
for (const [key] of additions) {
  console.log(`${key}=<redacted>`)
}
