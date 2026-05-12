import { existsSync } from "fs"
import fsp from "fs/promises"
import os from "os"
import path from "path"
import { Database as BunDatabase } from "bun:sqlite"
import { Database } from "../../src/storage/db"
import { migrationHash } from "../../src/storage/migration-repair"
import { expandHome, isValidUnlockSecret, jnoccioUnlockSecretPath } from "../../src/util/jnoccio-unlock"

export const repoRoot = path.resolve(import.meta.dir, "../../..", "..")

export type CommandResult = {
  stdout: string
  stderr: string
  exitCode: number
}

export type LocalUnlockPreflight =
  | {
      ok: true
      secret: string
      secretPath: string
    }
  | {
      ok: false
      reason: string
      secretPath?: string
    }

export function hasGitCrypt() {
  return Bun.spawnSync(["git-crypt", "version"], { stdout: "ignore", stderr: "ignore" }).exitCode === 0
}

export async function run(command: string, args: string[], cwd: string, env?: Record<string, string | undefined>) {
  const proc = Bun.spawn([command, ...args], {
    cwd,
    env: env ? { ...process.env, ...env } : process.env,
    stdout: "pipe",
    stderr: "pipe",
  })
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ])
  return { stdout, stderr, exitCode }
}

export async function cloneRepo(prefix: string, tempDirs: string[]) {
  const cloneParent = await fsp.mkdtemp(path.join(os.tmpdir(), prefix))
  tempDirs.push(cloneParent)
  const clone = path.join(cloneParent, "repo")
  const cloned = await run("git", ["clone", "--quiet", repoRoot, clone], cloneParent)
  if (cloned.exitCode !== 0) {
    throw new Error(`git clone failed: ${cloned.stderr || cloned.stdout}`)
  }
  return { clone, cloneParent }
}

export async function localUnlockPreflight(): Promise<LocalUnlockPreflight> {
  const secretPath = expandHome(process.env.JNOCCIO_UNLOCK_SECRET_PATH ?? jnoccioUnlockSecretPath())
  if (process.env.CI === "true") return { ok: false, reason: "CI=true", secretPath }
  if (!hasGitCrypt()) return { ok: false, reason: "git-crypt is not installed", secretPath }
  if (!existsSync(secretPath)) return { ok: false, reason: `unlock secret file is missing: ${secretPath}`, secretPath }

  let secret: string
  try {
    secret = (await fsp.readFile(secretPath, "utf8")).trim()
  } catch {
    return { ok: false, reason: `unlock secret file is not readable: ${secretPath}`, secretPath }
  }

  if (!isValidUnlockSecret(secret)) {
    return { ok: false, reason: `unlock secret file is invalid: ${secretPath}`, secretPath }
  }

  return { ok: true, secret, secretPath }
}

export async function withEnv<T>(env: Record<string, string | undefined>, fn: () => Promise<T>) {
  const previous = Object.fromEntries(Object.keys(env).map((key) => [key, process.env[key]]))
  try {
    for (const [key, value] of Object.entries(env)) {
      if (value === undefined) {
        delete process.env[key]
      } else {
        process.env[key] = value
      }
    }
    return await fn()
  } finally {
    for (const [key, value] of Object.entries(previous)) {
      if (value === undefined) {
        delete process.env[key]
      } else {
        process.env[key] = value
      }
    }
  }
}

export async function removeTempDirs(tempDirs: string[]) {
  await Promise.all(tempDirs.splice(0).map((dir) => fsp.rm(dir, { recursive: true, force: true })))
}

export function migrationTime(name: string) {
  if (name.length < 14) return 0

  const prefix = name.slice(0, 14)
  for (const char of prefix) {
    const code = char.charCodeAt(0)
    if (code < 48 || code > 57) return 0
  }

  const year = Number(prefix.slice(0, 4))
  const month = Number(prefix.slice(4, 6))
  const day = Number(prefix.slice(6, 8))
  const hour = Number(prefix.slice(8, 10))
  const minute = Number(prefix.slice(10, 12))
  const second = Number(prefix.slice(12, 14))

  return Date.UTC(
    year,
    month - 1,
    day,
    hour,
    minute,
    second,
  )
}

function migrationSqlPath(entry: string) {
  const safeEntry = path.basename(entry)
  if (safeEntry !== entry) {
    throw new Error(`Invalid migration directory name: ${entry}`)
  }

  return path.join(repoRoot, "db", "migrations", safeEntry, "migration.sql")
}

export async function seedTestDatabase() {
  const db = new BunDatabase(Database.Path)
  // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=static-pragma-no-input expires=2026-12-31
  db.exec("PRAGMA foreign_keys = ON")
  // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=static-pragma-no-input expires=2026-12-31
  db.exec("PRAGMA journal_mode = WAL")
  // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=static-pragma-no-input expires=2026-12-31
  db.exec("PRAGMA synchronous = NORMAL")
  // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=static-pragma-no-input expires=2026-12-31
  db.exec("PRAGMA busy_timeout = 5000")

  const migrationDir = path.join(import.meta.dir, "..", "..", "..", "..", "db", "migrations")
  const entries = (await fsp.readdir(migrationDir, { withFileTypes: true }))
    .filter((item) => item.isDirectory())
    .map((item) => item.name)
    .sort()
  for (const entry of entries) {
    const sqlPath = migrationSqlPath(entry)
    if (sqlPath !== path.join(migrationDir, entry, "migration.sql")) {
      throw new Error(`Unexpected migration path: ${sqlPath}`)
    }
    // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=repo-controlled migration replay in test fixture expires=2026-12-31
    db.exec(await fsp.readFile(sqlPath, "utf8"))
  }
  // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=repo-controlled migration bookkeeping in test fixture expires=2026-12-31
  db.exec(`
    CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
      id INTEGER PRIMARY KEY,
      hash text NOT NULL,
      created_at numeric,
      name text,
      applied_at TEXT
    )
  `)
  const insert = db.prepare(
    `INSERT INTO "__drizzle_migrations" ("hash", "created_at", "name", "applied_at") VALUES (?, ?, ?, ?)`,
  )
  const appliedAt = new Date().toISOString()
  for (const entry of entries) {
    const sqlPath = migrationSqlPath(entry)
    const sql = await fsp.readFile(sqlPath, "utf8")
    insert.run(migrationHash({ sql }), migrationTime(entry), entry, appliedAt)
  }
  db.close()
}
