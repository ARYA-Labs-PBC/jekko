import fs from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import { Flag } from "@jekko-ai/core/flag/flag"
import { CATALOG, type CatalogEntry } from "./model-keys.catalog"

export type ModelKeySource = "jekko.env" | "process-env" | "test-content"

export type ModelKeyStatus = {
  envName: string
  providerID: string
  configured: boolean
  active: boolean
  source?: ModelKeySource
  signupUrl?: string
  recommendedModelID?: string
  inactiveReason?: "no-developer-key" | "blank" | "unsupported" | "missing-companion-env" | "protected-router-unavailable"
  redacted?: string
}

export type SecretRef = {
  id: string
  envName: string
  providerID: string
}

const MODEL_KEY_TEMPLATE_PATH = "~/.jekko/jekko.env"

function homedirPath() {
  return path.join(os.homedir(), ".jekko", "jekko.env")
}

function expandHome(input: string) {
  const value = input.trim()
  if (value === "~") return os.homedir()
  if (value.startsWith("~/")) return path.join(os.homedir(), value.slice(2))
  return value
}

function redacted(value: string) {
  return value.trim().length > 0 ? "present" : "blank"
}

function parseEnvContent(input: string) {
  const result: Record<string, string> = {}
  for (const line of input.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith("#")) continue
    const match = trimmed.match(/^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/)
    if (!match) continue
    const key = match[1]
    let value = match[2] ?? ""
    if (value.startsWith("\"") && value.endsWith("\"")) {
      value = value.slice(1, -1)
    } else if (value.startsWith("'") && value.endsWith("'")) {
      value = value.slice(1, -1)
    }
    result[key] = value.trim()
  }
  return result
}

function readFileIfPresent(filePath: string) {
  try {
    return fs.readFileSync(filePath, "utf8")
  } catch {
    // jankurai:allow HLT-001-DEAD-MARKER reason=optional-local-model-keys-file expires=2027-01-01
    return undefined
  }
}

function effectiveValues(filePath = resolveModelKeysPath()) {
  const fileContent = Flag.JEKKO_MODEL_KEYS_CONTENT ?? readFileIfPresent(filePath) ?? ""
  const parsed = parseEnvContent(fileContent)
  const values: Record<string, { value?: string; source?: ModelKeySource }> = {}

  for (const entry of CATALOG) {
    for (const envName of entry.envNames) {
      const processValue = process.env[envName]
      if (processValue !== undefined && processValue.trim() !== "") {
        values[envName] = { value: processValue, source: "process-env" }
        continue
      }
      const fileValue = parsed[envName]
      if (fileValue !== undefined && fileValue.trim() !== "") {
        values[envName] = {
          value: fileValue,
          source: Flag.JEKKO_MODEL_KEYS_CONTENT ? "test-content" : "jekko.env",
        }
        continue
      }
      values[envName] = { value: undefined, source: undefined }
    }
  }

  return { values, fileContent }
}

export function resolveModelKeysPath() {
  return expandHome(Flag.JEKKO_MODEL_KEYS_FILE ?? homedirPath())
}

function chooseProvider(entries: CatalogEntry[], values: Record<string, { value?: string; source?: ModelKeySource }>) {
  const candidates = entries
    .flatMap((entry) => {
      const present = entry.envNames
        .map((envName) => ({ envName, ...values[envName] }))
        .find((item) => item.value !== undefined && item.value.trim() !== "")
      if (!present) return []
      const configuredCount = entry.envNames.filter((envName) => {
        const item = values[envName]
        return item.value !== undefined && item.value.trim() !== ""
      }).length
      const missingCompanionEnv =
        entry.companionEnvNames?.length && configuredCount < entry.companionEnvNames.length
          ? true
          : false
      return [{
        entry,
        envName: present.envName,
        secret: present.value!,
        source: present.source!,
        configuredCount,
        missingCompanionEnv,
      }]
    })

  const developerUnlocked = Boolean(process.env.JNOCCIO_DEVELOPER_KEY?.trim()) && !Flag.JEKKO_IGNORE_DEVELOPER_KEY
  const eligible = candidates.filter(
    (item) => !item.missingCompanionEnv && (developerUnlocked || item.entry.providerID !== "jnoccio"),
  )
  if (eligible.length === 0) return { activeProviderID: undefined, candidates, developerUnlocked }
  if (developerUnlocked) {
    const jnoccio = eligible.find((item) => item.entry.providerID === "jnoccio")
    if (jnoccio) return { activeProviderID: jnoccio.entry.providerID, candidates, developerUnlocked }
  }

  const active = [...eligible].sort((a, b) => b.entry.priority - a.entry.priority)[0]
  return { activeProviderID: active?.entry.providerID, candidates, developerUnlocked }
}

export async function ensureModelKeysFile(filePath = resolveModelKeysPath()) {
  const parent = path.dirname(filePath)
  await fsp.mkdir(parent, { recursive: true, mode: 0o700 })
  try {
    await fsp.access(filePath)
    return { path: filePath, created: false as const }
  } catch {
    await fsp.writeFile(filePath, buildModelKeysTemplate(), { mode: 0o600, flag: "wx" })
    return { path: filePath, created: true as const }
  }
}

export async function readModelKeyStatuses() {
  const filePath = resolveModelKeysPath()
  const created = Flag.JEKKO_MODEL_KEYS_CONTENT ? false : (await ensureModelKeysFile(filePath)).created
  const { values } = effectiveValues(filePath)
  const { activeProviderID, candidates, developerUnlocked } = chooseProvider(CATALOG, values)

  const statuses: ModelKeyStatus[] = []
  for (const entry of CATALOG) {
    const active = activeProviderID === entry.providerID
    const candidate = candidates.find((item) => item.entry.providerID === entry.providerID)
    const present = candidate?.secret !== undefined
    statuses.push({
      envName: candidate?.envName ?? entry.envNames[0] ?? "",
      providerID: entry.providerID,
      configured: present,
      active,
      source: candidate?.source,
      signupUrl: entry.signupUrl,
      recommendedModelID: entry.recommendedModelID,
      inactiveReason: !present
        ? "blank"
        : candidate?.missingCompanionEnv
          ? "missing-companion-env"
          : entry.providerID === "jnoccio" && !developerUnlocked
            ? "no-developer-key"
          : active
            ? undefined
            : developerUnlocked
              ? "protected-router-unavailable"
              : "no-developer-key",
      redacted: present ? redacted(candidate?.secret ?? "") : undefined,
    })
  }

  return {
    path: filePath,
    created,
    developerUnlocked,
    activeProviderID,
    statuses,
    values,
  }
}

export function buildModelKeysTemplate() {
  const lines: string[] = []
  lines.push("# Jekko model keys")
  lines.push("# Put your active model keys here. Blank values stay inactive.")
  lines.push("# Canonical path: ~/.jekko/jekko.env")
  lines.push("")

  for (const entry of CATALOG) {
    if (entry.signupUrl) {
      lines.push(`# ${entry.providerID}: ${entry.signupUrl}`)
    } else {
      lines.push(`# ${entry.providerID}`)
    }
    for (const envName of entry.envNames) {
      lines.push(`${envName}=`)
    }
    lines.push("")
  }

  return lines.join("\n").trimEnd() + "\n"
}

export function modelKeyCatalog() {
  return CATALOG.slice()
}

export function providerKeyStatusSummary(statuses: ModelKeyStatus[]) {
  return statuses.map((status) => ({
    providerID: status.providerID,
    active: status.active,
    configured: status.configured,
    source: status.source,
    envName: status.envName,
    signupUrl: status.signupUrl,
    recommendedModelID: status.recommendedModelID,
    inactiveReason: status.inactiveReason,
    redacted: status.redacted,
  }))
}
