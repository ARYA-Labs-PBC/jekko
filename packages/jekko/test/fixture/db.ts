import { rm, readdir, readFile } from "fs/promises"
import path from "path"
import { Database as BunSqlite } from "bun:sqlite"
import { Database } from "@/storage/db"
import { disposeAllInstances } from "./fixture"
import { migrationHash } from "@/storage/migration-repair"

const MIGRATION_DIR = path.join(import.meta.dir, "..", "..", "..", "..", "db", "migrations")

function migrationSqlPath(entry: string) {
  const safeEntry = path.basename(entry)
  if (safeEntry !== entry) {
    throw new Error(`Invalid migration directory name: ${entry}`)
  }
  return path.join(MIGRATION_DIR, safeEntry, "migration.sql")
}

async function seedSchema(dbPath: string) {
  const seedDb = new BunSqlite(dbPath)
  try {
    // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=static-pragma-no-input expires=2026-12-31
    seedDb.exec("PRAGMA foreign_keys = ON")
    seedDb.exec("PRAGMA journal_mode = WAL")

    const entries = (await readdir(MIGRATION_DIR, { withFileTypes: true }))
      .filter((item) => item.isDirectory())
      .map((item) => item.name)
      .sort()

    for (const entry of entries) {
      const sql = await readFile(migrationSqlPath(entry), "utf8")
      // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=repo-controlled migration replay in test fixture expires=2026-12-31
      seedDb.exec(sql)
    }

    // jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=repo-controlled migration bookkeeping in test fixture expires=2026-12-31
    seedDb.exec(`
      CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
        id INTEGER PRIMARY KEY,
        hash text NOT NULL,
        created_at numeric,
        name text,
        applied_at TEXT
      )
    `)
    const insert = seedDb.prepare(
      `INSERT INTO "__drizzle_migrations" ("hash", "created_at", "name", "applied_at") VALUES (?, ?, ?, ?)`,
    )
    const migrationTime = (tag: string) => {
      if (tag.length < 14) return 0
      const year = Number(tag.slice(0, 4))
      const month = Number(tag.slice(4, 6))
      const day = Number(tag.slice(6, 8))
      const hour = Number(tag.slice(8, 10))
      const minute = Number(tag.slice(10, 12))
      const second = Number(tag.slice(12, 14))
      if ([year, month, day, hour, minute, second].some((value) => Number.isNaN(value))) return 0
      return Date.UTC(year, month - 1, day, hour, minute, second)
    }
    const appliedAt = new Date().toISOString()
    for (const entry of entries) {
      const sql = await readFile(migrationSqlPath(entry), "utf8")
      insert.run(migrationHash({ sql }), migrationTime(entry), entry, appliedAt)
    }
  } finally {
    seedDb.close()
  }
}

export async function resetDatabase() {
  await disposeAllInstances().catch(() => undefined)
  Database.close()
  await rm(Database.Path, { force: true }).catch(() => undefined)
  await rm(`${Database.Path}-wal`, { force: true }).catch(() => undefined)
  await rm(`${Database.Path}-shm`, { force: true }).catch(() => undefined)
  await seedSchema(Database.Path)
}
