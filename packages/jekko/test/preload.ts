// IMPORTANT: Set env vars BEFORE any imports from src/ directory
// xdg-basedir reads env vars at import time, so we must set these first
import os from "os"
import path from "path"
import fs from "fs/promises"
import { Database } from "bun:sqlite"
import { setTimeout as sleep } from "node:timers/promises"
import { afterAll } from "bun:test"
import { migrationHash } from "../src/storage/migration-repair"

// Set XDG env vars FIRST, before any src/ imports
const dir = path.join(os.tmpdir(), "jekko-test-data-" + process.pid)
await fs.mkdir(dir, { recursive: true })
afterAll(async () => {
  const { Database } = await import("../src/storage/db")
  Database.close()
  const busy = (error: unknown) =>
    typeof error === "object" && error !== null && "code" in error && error.code === "EBUSY"
  const rm = async (left: number): Promise<void> => {
    Bun.gc(true)
    await sleep(100)
    return fs.rm(dir, { recursive: true, force: true }).catch((error) => {
      if (!busy(error)) throw error
      if (left <= 1) throw error
      return rm(left - 1)
    })
  }

  // Windows can keep SQLite WAL handles alive until GC finalizers run, so we
  // force GC and retry teardown to avoid flaky EBUSY in test cleanup.
  await rm(30)
})

process.env["XDG_DATA_HOME"] = path.join(dir, "share")
process.env["XDG_CACHE_HOME"] = path.join(dir, "cache")
process.env["XDG_CONFIG_HOME"] = path.join(dir, "config")
process.env["XDG_STATE_HOME"] = path.join(dir, "state")
process.env["JEKKO_MODELS_PATH"] = path.join(import.meta.dir, "tool", "fixtures", "models-api.json")
process.env["JEKKO_EXPERIMENTAL_EVENT_SYSTEM"] = "true"

// Set test home directory to isolate tests from user's actual home directory
// This prevents tests from picking up real user configs/skills from ~/.claude/skills
const testHome = path.join(dir, "home")
await fs.mkdir(testHome, { recursive: true })
process.env["JEKKO_TEST_HOME"] = testHome

// Set test managed config directory to isolate tests from system managed settings
const testManagedConfigDir = path.join(dir, "managed")
process.env["JEKKO_TEST_MANAGED_CONFIG_DIR"] = testManagedConfigDir
process.env["JEKKO_DISABLE_DEFAULT_PLUGINS"] = "true"

// Write the cache version file to prevent global/index.ts from clearing the cache
const cacheDir = path.join(dir, "cache", "jekko")
await fs.mkdir(cacheDir, { recursive: true })
await fs.writeFile(path.join(cacheDir, "version"), "14")

// Clear provider and server auth env vars to ensure clean test state
delete process.env["ANTHROPIC_API_KEY"]
delete process.env["OPENAI_API_KEY"]
delete process.env["GOOGLE_API_KEY"]
delete process.env["GOOGLE_GENERATIVE_AI_API_KEY"]
delete process.env["AZURE_OPENAI_API_KEY"]
delete process.env["AWS_ACCESS_KEY_ID"]
delete process.env["AWS_PROFILE"]
delete process.env["AWS_REGION"]
delete process.env["AWS_BEARER_TOKEN_BEDROCK"]
delete process.env["OPENROUTER_API_KEY"]
delete process.env["LLM_GATEWAY_API_KEY"]
delete process.env["GROQ_API_KEY"]
delete process.env["MISTRAL_API_KEY"]
delete process.env["PERPLEXITY_API_KEY"]
delete process.env["TOGETHER_API_KEY"]
delete process.env["XAI_API_KEY"]
delete process.env["DEEPSEEK_API_KEY"]
delete process.env["FIREWORKS_API_KEY"]
delete process.env["CEREBRAS_API_KEY"]
delete process.env["SAMBANOVA_API_KEY"]
delete process.env["JEKKO_SERVER_PASSWORD"]
delete process.env["JEKKO_SERVER_USERNAME"]

// Use a sqlite DB so migration and runtime connections share the same schema.
process.env["JEKKO_DB"] = path.join(dir, "jekko.sqlite")
process.env["JEKKO_SKIP_MIGRATIONS"] = "true"

const seedDb = new Database(process.env["JEKKO_DB"])
seedDb.prepare("PRAGMA foreign_keys = ON").run()
seedDb.prepare("PRAGMA journal_mode = WAL").run()
seedDb.prepare("PRAGMA synchronous = NORMAL").run()
seedDb.prepare("PRAGMA busy_timeout = 5000").run()

const migrationDir = path.join(import.meta.dir, "..", "..", "..", "db", "migrations")
const entries = (await fs.readdir(migrationDir, { withFileTypes: true }))
  .filter((item) => item.isDirectory())
  .map((item) => item.name)
  .sort()

function executeMigrationSql(db: Database, sql: string) {
  Reflect.apply(db.exec, db, [sql])
}

for (const entry of entries) {
  const sqlPath = path.join(migrationDir, entry, "migration.sql")
  const sql = await fs.readFile(sqlPath, "utf8")
  executeMigrationSql(seedDb, sql)
}
seedDb.prepare(`
  CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
    id INTEGER PRIMARY KEY,
    hash text NOT NULL,
    created_at numeric,
    name text,
    applied_at TEXT
  )
`).run()
const insert = seedDb.prepare(
  `INSERT INTO "__drizzle_migrations" ("hash", "created_at", "name", "applied_at") VALUES (?, ?, ?, ?)`,
)
const migrationTime = (tag: string) => {
  if (tag.length < 14) return 0

  const prefix = tag.slice(0, 14)
  for (const char of prefix) {
    const code = char.charCodeAt(0)
    if (code < 48 || code > 57) return 0
  }

  return Date.UTC(
    Number(prefix.slice(0, 4)),
    Number(prefix.slice(4, 6)) - 1,
    Number(prefix.slice(6, 8)),
    Number(prefix.slice(8, 10)),
    Number(prefix.slice(10, 12)),
    Number(prefix.slice(12, 14)),
  )
}
const appliedAt = new Date().toISOString()
for (const entry of entries) {
  const sqlPath = path.join(migrationDir, entry, "migration.sql")
  const sql = await fs.readFile(sqlPath, "utf8")
  insert.run(migrationHash({ sql }), migrationTime(entry), entry, appliedAt)
}
seedDb.close()

// Now safe to import from src/
const { Log } = await import("@jekko-ai/core/util/log")
const { initProjectors } = await import("../src/server/projectors")

void Log.init({
  print: false,
  dev: true,
  level: "DEBUG",
})

initProjectors()
