import fs from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import { Flag } from "@jekko-ai/core/flag/flag"

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

type CatalogEntry = {
  providerID: string
  envNames: string[]
  signupUrl?: string
  recommendedModelID?: string
  priority: number
  companionEnvNames?: string[]
}

const MODEL_KEY_TEMPLATE_PATH = "~/.jekko/jekko.env"

const CATALOG: CatalogEntry[] = [
  {
    providerID: "openai",
    envNames: ["OPENAI_API_KEY"],
    signupUrl: "https://platform.openai.com/api-keys",
    recommendedModelID: "gpt-5.3-codex",
    priority: 90,
  },
  {
    providerID: "anthropic",
    envNames: ["ANTHROPIC_API_KEY"],
    signupUrl: "https://console.anthropic.com/settings/keys",
    recommendedModelID: "claude-sonnet-4-5",
    priority: 88,
  },
  {
    providerID: "google",
    envNames: ["GOOGLE_GENERATIVE_AI_API_KEY", "GEMINI_API_KEY", "GOOGLE_API_KEY"],
    signupUrl: "https://aistudio.google.com/apikey",
    recommendedModelID: "gemini-2.5-flash",
    priority: 86,
  },
  {
    providerID: "openrouter",
    envNames: ["OPENROUTER_API_KEY"],
    signupUrl: "https://openrouter.ai/keys",
    recommendedModelID: "openrouter-gpt-oss-120b-free",
    priority: 80,
  },
  {
    providerID: "groq",
    envNames: ["GROQ_API_KEY"],
    signupUrl: "https://console.groq.com/keys",
    recommendedModelID: "groq-qwen3-32b",
    priority: 78,
  },
  {
    providerID: "cerebras",
    envNames: ["CEREBRAS_API_KEY"],
    signupUrl: "https://cloud.cerebras.ai",
    recommendedModelID: "cerebras-qwen-3-235b-a22b-instruct-2507",
    priority: 77,
  },
  {
    providerID: "mistral",
    envNames: ["MISTRAL_API_KEY"],
    signupUrl: "https://console.mistral.ai/api-keys",
    recommendedModelID: "mistral-devstral-latest",
    priority: 76,
  },
  {
    providerID: "github",
    envNames: ["GITHUB_TOKEN"],
    signupUrl: "https://github.com/marketplace/models",
    recommendedModelID: "github-codestral-2501",
    priority: 75,
  },
  {
    providerID: "nvidia",
    envNames: ["NVIDIA_API_KEY"],
    signupUrl: "https://build.nvidia.com",
    recommendedModelID: "nvidia-deepseek-v4-pro",
    priority: 74,
  },
  {
    providerID: "fireworks",
    envNames: ["FIREWORKS_API_KEY"],
    signupUrl: "https://fireworks.ai/pricing",
    recommendedModelID: "fireworks-deepseek-v4-pro",
    priority: 73,
  },
  {
    providerID: "dashscope",
    envNames: ["DASHSCOPE_API_KEY"],
    signupUrl: "https://www.alibabacloud.com/help/en/model-studio/qwen-coder",
    recommendedModelID: "alibaba-qwen3-coder-plus",
    priority: 72,
  },
  {
    providerID: "sambanova",
    envNames: ["SAMBANOVA_API_KEY"],
    signupUrl: "https://cloud.sambanova.ai",
    recommendedModelID: "sambanova-gpt-oss-120b",
    priority: 71,
  },
  {
    providerID: "huggingface",
    envNames: ["HF_TOKEN"],
    signupUrl: "https://huggingface.co/settings/tokens",
    recommendedModelID: "huggingface-qwen3-coder-next",
    priority: 70,
  },
  {
    providerID: "zai",
    envNames: ["ZAI_API_KEY"],
    signupUrl: "https://z.ai/manage-apikey/apikey-list",
    recommendedModelID: "zai-glm-47-flash",
    priority: 69,
  },
  {
    providerID: "inception",
    envNames: ["INCEPTION_API_KEY"],
    signupUrl: "https://platform.inceptionlabs.ai",
    recommendedModelID: "inception-mercury-2",
    priority: 68,
  },
  {
    providerID: "ai-gateway",
    envNames: ["AI_GATEWAY_API_KEY"],
    signupUrl: "https://vercel.com/ai-gateway",
    recommendedModelID: "vercel-claude-sonnet-46",
    priority: 67,
  },
  {
    providerID: "kilo",
    envNames: ["KILO_API_KEY"],
    signupUrl: "https://app.kilo.ai",
    recommendedModelID: "kilo-ling-26-1t-free",
    priority: 66,
  },
  {
    providerID: "cloudflare",
    envNames: ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"],
    signupUrl: "https://dash.cloudflare.com/profile/api-tokens",
    recommendedModelID: "cloudflare-gpt-oss-120b",
    priority: 65,
    companionEnvNames: ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"],
  },
  {
    providerID: "jekko",
    envNames: ["JEKKO_API_KEY"],
    signupUrl: "https://jekko.ai/zen",
    recommendedModelID: "big-pickle",
    priority: 95,
  },
  {
    providerID: "jnoccio",
    envNames: ["JNOCCIO_DEVELOPER_KEY"],
    recommendedModelID: "jnoccio-fusion",
    priority: 96,
  },
]

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
    .map((entry) => {
      const present = entry.envNames
        .map((envName) => ({ envName, ...values[envName] }))
        .find((item) => item.value !== undefined && item.value.trim() !== "")
      if (!present) return undefined
      const configuredCount = entry.envNames.filter((envName) => {
        const item = values[envName]
        return item.value !== undefined && item.value.trim() !== ""
      }).length
      const missingCompanionEnv =
        entry.companionEnvNames?.length && configuredCount < entry.companionEnvNames.length
          ? true
          : false
      return {
        entry,
        envName: present.envName,
        secret: present.value!,
        source: present.source!,
        configuredCount,
        missingCompanionEnv,
      }
    })
    .filter((item): item is NonNullable<typeof item> => !!item)

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
