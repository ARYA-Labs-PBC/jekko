#!/usr/bin/env bun

import fs from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"

import { hostBinaryPath } from "./host-binary-path"

const repoRoot = path.resolve(import.meta.dir, "..", "..", "..")
const manifest = path.join(repoRoot, "crates", "tuiwright-jekko-unlock", "Cargo.toml")
const envPath = process.env.JEKKO_LIVE_PROD_ENV || path.join(os.homedir(), ".config", "jekko", "live-prod.env")

function stripQuotes(value: string) {
  const trimmed = value.trim()
  if ((trimmed.startsWith('"') && trimmed.endsWith('"')) || (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
    return trimmed.slice(1, -1)
  }
  return trimmed
}

function expandHome(value: string) {
  if (value === "~") return os.homedir()
  if (value.startsWith("~/")) return path.join(os.homedir(), value.slice(2))
  if (value.startsWith("$HOME/")) return path.join(os.homedir(), value.slice("$HOME/".length))
  if (value.startsWith("${HOME}/")) return path.join(os.homedir(), value.slice("${HOME}/".length))
  return value
}

async function readEnvFile(file: string) {
  const text = await fsp.readFile(file, "utf8")
  const env: Record<string, string> = {}
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith("#")) continue
    const match = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$/.exec(trimmed)
    if (!match) continue
    const key = match[1]
    const value = stripQuotes(match[2])
    env[key] = key.endsWith("_PATH") || key.endsWith("_FILE") ? expandHome(value) : value
  }
  return env
}

function runCargo(name: string, args: string[], env: Record<string, string>) {
  console.log(`Running ${name}`)
  const proc = Bun.spawnSync(["cargo", "test", "--manifest-path", manifest, ...args], {
    cwd: repoRoot,
    stdout: "inherit",
    stderr: "inherit",
    env,
  })
  if (proc.exitCode !== 0) {
    throw new Error(`${name} failed with exit ${proc.exitCode}`)
  }
}

if (process.env.CI === "true") {
  throw new Error("Refusing to run live production TUI tests in CI")
}
if (!fs.existsSync(envPath)) {
  throw new Error(`Local live prod env file is missing: ${envPath}\nRun: just tui-live-prod-init`)
}

const fileEnv = await readEnvFile(envPath)
const binary = process.env.JEKKO_BIN || fileEnv.JEKKO_BIN || hostBinaryPath()
if (!fs.existsSync(binary)) {
  throw new Error(`Jekko binary is missing at ${binary}. Run: just jekko-build-host-fast`)
}
if (!fileEnv.JEKKO_API_KEY && !process.env.JEKKO_API_KEY) {
  throw new Error(`JEKKO_API_KEY is required in ${envPath} or the process environment`)
}

const defaultUnlockPath = path.join(os.homedir(), "jnoccio-fusion.unlock")
const unlockPath = fileEnv.JNOCCIO_UNLOCK_SECRET_PATH || process.env.JNOCCIO_UNLOCK_SECRET_PATH || defaultUnlockPath
const hasUnlockSecret = fs.existsSync(unlockPath)

const env: Record<string, string> = {
  ...process.env,
  ...fileEnv,
  JEKKO_BIN: binary,
  JEKKO_TUI_LIVE_PROD: "1",
  JNOCCIO_TUI_TEST: fileEnv.JNOCCIO_TUI_TEST || process.env.JNOCCIO_TUI_TEST || "1",
}
if (hasUnlockSecret) {
  env.JNOCCIO_UNLOCK_SECRET_PATH = unlockPath
  env.JNOCCIO_TUIWRIGHT_E2E = fileEnv.JNOCCIO_TUIWRIGHT_E2E || process.env.JNOCCIO_TUIWRIGHT_E2E || "1"
}

console.log(`Using local live prod env: ${envPath}`)
console.log(`Using Jekko binary: ${binary}`)
console.log(`JEKKO_API_KEY=<redacted>`)
if (hasUnlockSecret) {
  console.log(`JNOCCIO_UNLOCK_SECRET_PATH=${unlockPath}`)
} else {
  console.log(`Jnoccio unlock secret not found at ${unlockPath}; unlock-specific TUI test will be skipped`)
}

runCargo(
  "live Jekko TUI prompt",
  ["live_jekko_prompt_round_trips_through_tui", "--", "--ignored", "--nocapture"],
  env,
)
runCargo("Jnoccio dashboard TUI checks", ["jnoccio_", "--", "--ignored", "--nocapture"], env)
if (hasUnlockSecret) {
  runCargo("Jnoccio unlock PTY check", ["jekko_tui_paste_unlocks_jnoccio_fusion", "--", "--nocapture"], env)
}
