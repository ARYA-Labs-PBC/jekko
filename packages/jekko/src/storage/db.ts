import { sql } from "drizzle-orm"
import { type SQLiteBunDatabase } from "drizzle-orm/bun-sqlite"
import { type SQLiteTransaction } from "drizzle-orm/sqlite-core"
export * from "drizzle-orm"
import { LocalContext } from "@/util/local-context"
import { lazy } from "../util/lazy"
import { Global } from "@jekko-ai/core/global"
import * as Log from "@jekko-ai/core/util/log"
import { NamedError } from "@jekko-ai/core/util/error"
import z from "zod"
import path from "path"
import { readFileSync, readdirSync, existsSync } from "fs"
import { Flag } from "@jekko-ai/core/flag/flag"
import { InstallationChannel } from "@jekko-ai/core/installation/version"
import { InstanceState } from "@/effect/instance-state"
import { iife } from "@/util/iife"
import { init } from "#db"
import { migrationHash, repairSqliteMigrations, splitMigrationStatements, type MigrationEntry } from "./migration-repair"

declare const JEKKO_MIGRATIONS: { sql: string; timestamp: number; name: string; hash?: string }[] | undefined

export const NotFoundError = NamedError.create(
  "NotFoundError",
  z.object({
    message: z.string(),
  }),
)

const log = Log.create({ service: "db" })

export function channelDbPath(channel: string, options?: { disableChannel?: boolean }) {
  if (["latest", "beta", "prod"].includes(channel) || options?.disableChannel)
    return path.join(Global.Path.data, "jekko.db")
  const safe = channel.replace(/[^a-zA-Z0-9._-]/g, "-")
  return path.join(Global.Path.data, `jekko-${safe}.db`)
}

export function getChannelPath() {
  return channelDbPath(InstallationChannel, { disableChannel: Flag.JEKKO_DISABLE_CHANNEL_DB })
}

export const Path = iife(() => {
  if (Flag.JEKKO_DB) {
    if (Flag.JEKKO_DB === ":memory:" || path.isAbsolute(Flag.JEKKO_DB)) return Flag.JEKKO_DB
    return path.join(Global.Path.data, Flag.JEKKO_DB)
  }
  return getChannelPath()
})

export type Transaction = SQLiteTransaction<"sync", void>

type Client = SQLiteBunDatabase

type Journal = MigrationEntry[]

function quoteLiteral(value: string | number) {
  return `'${String(value).replaceAll("'", "''")}'`
}

function appliedMigrationNames(db: SQLiteBunDatabase) {
  try {
    const rows = db.all(
      sql.raw(`SELECT name FROM __drizzle_migrations WHERE name IS NOT NULL ORDER BY id ASC`),
    ) as Array<{ name: string | null }>
    return new Set(rows.map((row) => row.name).filter((name): name is string => !!name))
  } catch {
    return new Set<string>()
  }
}

function insertMigrationRow(db: SQLiteBunDatabase, entry: MigrationEntry) {
  db.run(
    sql.raw(
      `INSERT INTO "__drizzle_migrations" ("hash", "created_at", "name", "applied_at")
       VALUES (${quoteLiteral(entry.hash)}, ${entry.timestamp}, ${quoteLiteral(entry.name)}, ${quoteLiteral(new Date().toISOString())})`,
    ),
  )
}

export function applyMigrationJournal(db: SQLiteBunDatabase, entries: Journal) {
  db.run(
    sql.raw(
      `CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
        id INTEGER PRIMARY KEY,
        hash text NOT NULL,
        created_at numeric,
        name text,
        applied_at TEXT
      )`,
    ),
  )
  const applied = appliedMigrationNames(db)
  for (const entry of entries) {
    if (applied.has(entry.name)) continue
    for (const statement of splitMigrationStatements(entry.sql)) {
      db.run(sql.raw(statement))
    }
    insertMigrationRow(db, entry)
  }
}

export function migrationTimestamp(tag: string) {
  if (tag.length < 14) return 0
  const year = Number(tag.slice(0, 4))
  const month = Number(tag.slice(4, 6))
  const day = Number(tag.slice(6, 8))
  const hour = Number(tag.slice(8, 10))
  const minute = Number(tag.slice(10, 12))
  const second = Number(tag.slice(12, 14))
  if ([year, month, day, hour, minute, second].some((value) => Number.isNaN(value))) return 0
  return Date.UTC(
    year,
    month - 1,
    day,
    hour,
    minute,
    second,
  )
}

function migrations(dir: string): Journal {
  const dirs = readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)

  const sql = dirs
    .map((name) => {
      const file = path.join(dir, name, "migration.sql")
      if (!existsSync(file)) return
      const text = readFileSync(file, "utf-8")
      return {
        sql: text,
        timestamp: migrationTimestamp(name),
        name,
        hash: migrationHash({ sql: text }),
      }
    })
    .filter(Boolean) as Journal

  return sql.sort((a, b) => a.timestamp - b.timestamp)
}

function withMigrationHashes(entries: { sql: string; timestamp: number; name: string; hash?: string }[]): Journal {
  return entries
    .map((entry) => ({
      ...entry,
      hash: entry.hash ?? migrationHash({ sql: entry.sql }),
    }))
    .sort((a, b) => a.timestamp - b.timestamp)
}

export const Client = lazy(() => {
  log.info("opening database", { path: Path })

  const db = init(Path)

  db.run("PRAGMA journal_mode = WAL")
  db.run("PRAGMA synchronous = NORMAL")
  db.run("PRAGMA busy_timeout = 5000")
  db.run("PRAGMA cache_size = -64000")
  db.run("PRAGMA foreign_keys = ON")
  db.run("PRAGMA wal_checkpoint(PASSIVE)")

  // Apply schema migrations
  const entries =
    typeof JEKKO_MIGRATIONS !== "undefined"
      ? withMigrationHashes(JEKKO_MIGRATIONS)
      : migrations(path.join(import.meta.dirname, "../../../../db/migrations"))
  if (entries.length > 0) {
    log.info("applying migrations", {
      count: entries.length,
      mode: typeof JEKKO_MIGRATIONS !== "undefined" ? "bundled" : "dev",
    })
    const repair = repairSqliteMigrations(db, { dbPath: Path, migrations: entries })
    if (repair.backedUp.length > 0 || repair.repaired.length > 0 || repair.recreatedMigrationTable) {
      log.info("repaired sqlite migrations", {
        backups: repair.backedUp.length,
        repaired: repair.repaired.length,
        recreatedMigrationTable: repair.recreatedMigrationTable,
      })
    }
    if (Flag.JEKKO_SKIP_MIGRATIONS) {
      for (const item of entries) {
        item.sql = "select 1;"
      }
    }
    applyMigrationJournal(db, entries)
  }

  return db
})

export function close() {
  if (!Client.loaded()) return
  Client().$client.close()
  Client.reset()
}

export type TxOrDb = Transaction | Client

const ctx = LocalContext.create<{
  tx: TxOrDb
  effects: (() => void | Promise<void>)[]
}>("database")

export function use<T>(callback: (trx: TxOrDb) => T): T {
  try {
    return callback(ctx.use().tx)
  } catch (err) {
    if (err instanceof LocalContext.NotFound) {
      const effects: (() => void | Promise<void>)[] = []
      const result = ctx.provide({ effects, tx: Client() }, () => callback(Client()))
      for (const effect of effects) effect()
      return result
    }
    throw err
  }
}

export function effect(fn: () => any | Promise<any>) {
  const bound = InstanceState.bind(fn)
  try {
    ctx.use().effects.push(bound)
  } catch {
    bound()
  }
}

type NotPromise<T> = T extends Promise<any> ? never : T

export function transaction<T>(
  callback: (tx: TxOrDb) => NotPromise<T>,
  options?: {
    behavior?: "deferred" | "immediate" | "exclusive"
  },
): NotPromise<T> {
  try {
    return callback(ctx.use().tx)
  } catch (err) {
    if (err instanceof LocalContext.NotFound) {
      const effects: (() => void | Promise<void>)[] = []
      const txCallback = InstanceState.bind((tx: TxOrDb) => ctx.provide({ tx, effects }, () => callback(tx)))
      const result = Client().transaction(txCallback, { behavior: options?.behavior })
      for (const effect of effects) effect()
      return result as NotPromise<T>
    }
    throw err
  }
}

export * as Database from "./db"
