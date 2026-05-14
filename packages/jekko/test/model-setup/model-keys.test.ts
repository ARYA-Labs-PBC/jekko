import { afterEach, expect, test } from "bun:test"
import { mkdtemp, readFile, stat, unlink } from "fs/promises"
import os from "os"
import path from "path"
import {
  ensureModelKeysFile,
  buildModelKeysTemplate,
  providerKeyStatusSummary,
  readModelKeyStatuses,
  resolveModelKeysPath,
} from "@/model-setup/model-keys"
const ENV_KEYS = [
  "JEKKO_MODEL_KEYS_FILE",
  "JEKKO_MODEL_KEYS_CONTENT",
  "JEKKO_IGNORE_DEVELOPER_KEY",
  "JNOCCIO_DEVELOPER_KEY",
  "OPENAI_API_KEY",
  "ANTHROPIC_API_KEY",
  "GROQ_API_KEY",
]

function withEnv(values: Record<string, string | undefined>) {
  const previous = new Map<string, string | undefined>()
  for (const key of Object.keys(values)) previous.set(key, process.env[key])
  for (const [key, value] of Object.entries(values)) {
    if (value === undefined) delete process.env[key]
    else process.env[key] = value
  }
  return () => {
    for (const [key, value] of previous) {
      if (value === undefined) delete process.env[key]
      else process.env[key] = value
    }
  }
}

afterEach(() => {
  for (const key of ENV_KEYS) delete process.env[key]
})

test("buildModelKeysTemplate includes canonical provider keys and signup URLs", () => {
  const template = buildModelKeysTemplate()
  expect(template).toContain("OPENAI_API_KEY=")
  expect(template).toContain("ANTHROPIC_API_KEY=")
  expect(template).toContain("JNOCCIO_DEVELOPER_KEY=")
  expect(template).toContain("platform.openai.com/api-keys")
  expect(template).toContain("console.anthropic.com/settings/keys")
})

test("ensureModelKeysFile creates the canonical file with restrictive permissions", async () => {
  const dir = await mkdtemp(path.join(os.tmpdir(), "jekko-model-keys-"))
  const file = path.join(dir, "custom", "jekko.env")
  const restore = withEnv({
    JEKKO_MODEL_KEYS_FILE: file,
    JEKKO_MODEL_KEYS_CONTENT: undefined,
  })
  try {
    const result = await ensureModelKeysFile()
    expect(result.path).toBe(file)
    expect(result.created).toBe(true)
    const content = await readFile(file, "utf8")
    expect(content).toContain("OPENAI_API_KEY=")
    const mode = (await stat(file)).mode & 0o777
    expect(mode).toBe(0o600)
  } finally {
    restore()
    await unlink(file).catch(() => undefined)
  }
})

test("blank values stay inactive and do not activate providers", async () => {
  const restore = withEnv({
    JEKKO_MODEL_KEYS_CONTENT: "OPENAI_API_KEY=\nANTHROPIC_API_KEY=\n",
  })
  try {
    const status = await readModelKeyStatuses()
    const openai = status.statuses.find((item) => item.providerID === "openai")
    expect(status.activeProviderID).toBeUndefined()
    expect(openai?.configured).toBe(false)
    expect(openai?.active).toBe(false)
    expect(openai?.redacted).toBeUndefined()
  } finally {
    restore()
  }
})

test("process env overrides blank file values", async () => {
  const restore = withEnv({
    JEKKO_MODEL_KEYS_CONTENT: "OPENAI_API_KEY=\n",
    OPENAI_API_KEY: "env-openai-secret",
  })
  try {
    const status = await readModelKeyStatuses()
    const openai = status.statuses.find((item) => item.providerID === "openai")
    expect(openai?.configured).toBe(true)
    expect(openai?.active).toBe(true)
    expect(openai?.source).toBe("process-env")
    expect(openai?.redacted).toBe("present")
  } finally {
    restore()
  }
})

test("multiple keys without developer unlock keep exactly one active provider", async () => {
  const restore = withEnv({
    JEKKO_MODEL_KEYS_CONTENT: "OPENAI_API_KEY=file-openai-secret\nGROQ_API_KEY=file-groq-secret\n",
    JNOCCIO_DEVELOPER_KEY: undefined,
  })
  try {
    const status = await readModelKeyStatuses()
    const active = status.statuses.filter((item) => item.active)
    expect(active).toHaveLength(1)
    expect(active[0]?.providerID).toBe("openai")
    expect(status.statuses.find((item) => item.providerID === "groq")?.inactiveReason).toBe("no-developer-key")
  } finally {
    restore()
  }
})

test("developer unlock is ignored when JEKKO_IGNORE_DEVELOPER_KEY is set", async () => {
  const restore = withEnv({
    JEKKO_MODEL_KEYS_CONTENT: "OPENAI_API_KEY=file-openai-secret\nJNOCCIO_DEVELOPER_KEY=file-jnoccio-secret\n",
    JNOCCIO_DEVELOPER_KEY: "process-jnoccio-secret",
    JEKKO_IGNORE_DEVELOPER_KEY: "1",
  })
  try {
    const status = await readModelKeyStatuses()
    const active = status.statuses.filter((item) => item.active)
    expect(active).toHaveLength(1)
    expect(active[0]?.providerID).toBe("openai")
    expect(status.developerUnlocked).toBe(false)
  } finally {
    restore()
  }
})

test("provider key status summaries never include raw secret values", async () => {
  const restore = withEnv({
    JEKKO_MODEL_KEYS_CONTENT: "OPENAI_API_KEY=summary-secret-key\n",
  })
  try {
    const status = await readModelKeyStatuses()
    const summary = providerKeyStatusSummary(status.statuses)
    expect(JSON.stringify(summary)).not.toContain("summary-secret-key")
    expect(summary[0]?.redacted).toBe("present")
  } finally {
    restore()
  }
})

test("resolveModelKeysPath honors overrides without touching home", () => {
  const custom = "/tmp/jekko-model-keys-test.env"
  const restore = withEnv({ JEKKO_MODEL_KEYS_FILE: custom })
  try {
    expect(resolveModelKeysPath()).toBe(custom)
  } finally {
    restore()
  }
})
