// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import crypto from "crypto"
import fs from "fs"
import { count, eq, sql } from "drizzle-orm"
import { type SQLiteBunDatabase } from "drizzle-orm/bun-sqlite"
import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core"

export type MigrationEntry = {
  sql: string
  timestamp: number
  name: string
  hash: string
}

export type MigrationRepairReport = {
  backedUp: string[]
  repaired: string[]
  recreatedMigrationTable: boolean
}

type TableInfoRow = {
  cid: number
  name: string
  type: string
  notnull: number
  dflt_value: string | null
  pk: number
}

type MasterRow = {
  type: string
  name: string
  tbl_name: string | null
  sql: string | null
}

type MigrationRow = {
  id: number
  hash: string
  created_at: number | string | null
  name: string | null
  applied_at: string | null
}

const MIGRATIONS_TABLE = "__drizzle_migrations"
const MigrationTable = sqliteTable(MIGRATIONS_TABLE, {
  id: integer("id").primaryKey(),
  hash: text("hash").notNull(),
  created_at: integer("created_at").notNull(),
  name: text("name"),
  applied_at: text("applied_at"),
})

function quoteIdentifier(name: string) {
  return `"${name.replaceAll('"', '""')}"`
}

function stripComments(statement: string) {
  return statement
    .split(/\r?\n/)
    .filter((line) => !line.trim().startsWith("--"))
    .join("\n")
    .trim()
    .replace(/;$/, "")
}

export function splitMigrationStatements(sqlText: string) {
  const statements: string[] = []

  for (const chunk of sqlText.split("--> statement-breakpoint")) {
    const cleaned = stripComments(chunk)
    let current = ""
    let single = false
    let double = false
    let backtick = false
    // CREATE TRIGGER ... BEGIN ... END; bodies contain inner `;` that must not
    // split the statement. Track BEGIN/END nesting and only honour `;` at depth 0.
    let blockDepth = 0

    for (let i = 0; i < cleaned.length; i += 1) {
      const char = cleaned[i]
      const prev = cleaned[i - 1]

      if (char === "'" && !double && !backtick && prev !== "\\") single = !single
      else if (char === '"' && !single && !backtick && prev !== "\\") double = !double
      else if (char === "`" && !single && !double && prev !== "\\") backtick = !backtick

      if (!single && !double && !backtick) {
        const atWordStart = i === 0 || !/[A-Za-z0-9_]/.test(prev ?? "")
        if (atWordStart) {
          if (matchKeyword(cleaned, i, "BEGIN")) blockDepth += 1
          else if (blockDepth > 0 && matchKeyword(cleaned, i, "END")) blockDepth -= 1
        }
      }

      if (char === ";" && !single && !double && !backtick && blockDepth === 0) {
        const statement = current.trim()
        if (statement) statements.push(statement)
        current = ""
        continue
      }

      current += char
    }

    const statement = current.trim()
    if (statement) statements.push(statement)
  }

  return statements.filter(Boolean)
}

function matchKeyword(source: string, index: number, keyword: string) {
  if (source.length - index < keyword.length) return false
  for (let i = 0; i < keyword.length; i += 1) {
    const a = source.charCodeAt(index + i)
    const b = keyword.charCodeAt(i)
    if (a === b) continue
    if (a >= 97 && a <= 122 && a - 32 === b) continue
    return false
  }
  const after = source[index + keyword.length]
  if (after === undefined) return true
  return !/[A-Za-z0-9_]/.test(after)
}

function sha256Hex(text: string) {
  return crypto.createHash("sha256").update(text).digest("hex")
}

function isInternalTable(name: string) {
  return name.startsWith("sqlite_") || name === MIGRATIONS_TABLE
}

function tableInfo(db: SQLiteBunDatabase, table: string) {
  return db.all(sql.raw(`PRAGMA table_info(${quoteIdentifier(table)})`)) as TableInfoRow[]
}

function tableExists(master: MasterRow[], table: string) {
  return master.some((row) => row.type === "table" && row.name === table)
}

function indexExists(master: MasterRow[], index: string) {
  return master.some((row) => row.type === "index" && row.name === index)
}

function countRows(db: SQLiteBunDatabase, master: MasterRow[], table: string) {
  if (!tableExists(master, table)) return 0
  const dynamicTable = sqliteTable(table, {} as never)
  const row = db.select({ count: count() }).from(dynamicTable).get() as { count: number } | undefined
  return Number(row?.count ?? 0)
}

function migrationRows(db: SQLiteBunDatabase) {
  if (!tableExists(masterRows(db), MIGRATIONS_TABLE)) return []
  return db.select().from(MigrationTable).orderBy(MigrationTable.id).all() as MigrationRow[]
}

function masterRows(db: SQLiteBunDatabase) {
  return db.all(sql.raw(`SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY name ASC`)) as MasterRow[]
}

function backupDatabaseFiles(dbPath: string) {
  if (dbPath === ":memory:") return [] as string[]
  const stamp = new Date().toISOString().replace(/[:.]/g, "-")
  const base = `${dbPath}.repair-${stamp}`
  const backups: string[] = []
  const sources = [
    { source: dbPath, target: base },
    { source: `${dbPath}-wal`, target: `${base}-wal` },
    { source: `${dbPath}-shm`, target: `${base}-shm` },
  ]

  for (const item of sources) {
    if (!fs.existsSync(item.source)) continue
    fs.copyFileSync(item.source, item.target)
    backups.push(item.target)
  }

  return backups
}

function stripIdentifier(value: string) {
  return value.replaceAll(/^[`"]|[`"]$/g, "")
}

function parseAddColumn(statement: string) {
  const parts = statement.trim().split(/\s+/)
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts.length < 5) return undefined
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts[0]?.toUpperCase() !== "ALTER" || parts[1]?.toUpperCase() !== "TABLE") return undefined
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts[3]?.toUpperCase() !== "ADD") return undefined
  const columnIndex = parts[4]?.toUpperCase() === "COLUMN" ? 5 : 4
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts.length <= columnIndex + 1) return undefined
  return {
    table: stripIdentifier(parts[2] ?? ""),
    column: stripIdentifier(parts[columnIndex] ?? ""),
    definition: parts.slice(columnIndex + 1).join(" ").trim(),
    safe: !/\bNOT\s+NULL\b/i.test(parts.slice(columnIndex + 1).join(" ")) || /\bDEFAULT\b/i.test(parts.slice(columnIndex + 1).join(" ")),
  }
}

function parseCreateIndex(statement: string) {
  const parts = statement.trim().split(/\s+/)
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts.length < 5) return undefined
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts[0]?.toUpperCase() !== "CREATE") return undefined
  const indexIndex = parts[1]?.toUpperCase() === "UNIQUE" ? 3 : 2
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts[indexIndex - 1]?.toUpperCase() !== "INDEX") return undefined
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts[indexIndex + 1]?.toUpperCase() !== "ON") return undefined
  return {
    index: stripIdentifier(parts[indexIndex] ?? ""),
    table: stripIdentifier(parts[indexIndex + 2] ?? ""),
  }
}

function parseRenameTable(statement: string) {
  const parts = statement.trim().split(/\s+/)
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts.length !== 6) return undefined
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts[0]?.toUpperCase() !== "ALTER" || parts[1]?.toUpperCase() !== "TABLE") return undefined
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (parts[3]?.toUpperCase() !== "RENAME" || parts[4]?.toUpperCase() !== "TO") return undefined
  return {
    from: stripIdentifier(parts[2] ?? ""),
    to: stripIdentifier(parts[5] ?? ""),
  }
}

function rebuildMigrationTable(db: SQLiteBunDatabase) {
  db.run(
    sql.raw(
      `CREATE TABLE IF NOT EXISTS ${quoteIdentifier(MIGRATIONS_TABLE)} (
        id INTEGER PRIMARY KEY,
        hash text NOT NULL,
        created_at numeric,
        name text,
        applied_at TEXT
      )`,
    ),
  )
}

function upsertMigrationRow(db: SQLiteBunDatabase, entry: MigrationEntry) {
  const existing = db
    .select({ id: MigrationTable.id })
    .from(MigrationTable)
    .where(eq(MigrationTable.name, entry.name))
    .limit(1)
    .all() as Array<{ id: number }>

  if (existing.length > 0) {
    db.update(MigrationTable)
      .set({ hash: entry.hash, created_at: entry.timestamp, name: entry.name })
      .where(eq(MigrationTable.id, existing[0].id))
      .run()
    return
  }

  db.insert(MigrationTable)
    .values({
      hash: entry.hash,
      created_at: entry.timestamp,
      name: entry.name,
      applied_at: new Date().toISOString(),
    })
    .run()
}

export function repairSqliteMigrations(
  db: SQLiteBunDatabase,
  input: { dbPath: string; migrations: MigrationEntry[] },
): MigrationRepairReport {
  let master = masterRows(db)
  const rows = migrationRows(db)
  const nonInternalTables = master.filter((row) => row.type === "table" && !isInternalTable(row.name))
  const userTableNames = new Set(nonInternalTables.map((row) => row.name))
  const report: MigrationRepairReport = {
    backedUp: [],
    repaired: [],
    recreatedMigrationTable: false,
  }

  if (master.length === 1 && master[0]?.type === "table" && master[0].name === MIGRATIONS_TABLE) {
    report.backedUp = backupDatabaseFiles(input.dbPath)
    db.run(sql.raw(`DROP TABLE IF EXISTS ${quoteIdentifier(MIGRATIONS_TABLE)}`))
    report.recreatedMigrationTable = true
    return report
  }

  const byName = new Map(rows.filter((row) => row.name).map((row) => [row.name as string, row]))
  const missingRepair: Array<{ entry: MigrationEntry; statements: string[] }> = []

  for (const entry of input.migrations) {
    const existing = byName.get(entry.name)
    if (existing) {
      const needsUpdate =
        existing.hash !== entry.hash ||
        Number(existing.created_at ?? 0) !== entry.timestamp ||
        existing.name !== entry.name
      if (needsUpdate) {
        if (report.backedUp.length === 0) report.backedUp = backupDatabaseFiles(input.dbPath)
        upsertMigrationRow(db, entry)
        report.repaired.push(entry.name)
      }
      continue
    }

    const statements = splitMigrationStatements(entry.sql)
    const missingStatements: string[] = []
    let unsafe = false

    for (const statement of statements) {
      const addColumn = parseAddColumn(statement)
      if (addColumn) {
        if (!tableExists(master, addColumn.table)) {
          unsafe = true
          break
        }
        const columns = new Set(tableInfo(db, addColumn.table).map((row) => row.name))
        if (columns.has(addColumn.column)) continue
        if (countRows(db, master, addColumn.table) > 0 && !addColumn.safe) {
          unsafe = true
          break
        }
        missingStatements.push(statement)
        continue
      }

      const createIndex = parseCreateIndex(statement)
      if (createIndex) {
        if (indexExists(master, createIndex.index)) continue
        missingStatements.push(statement)
        continue
      }

      const renameTable = parseRenameTable(statement)
      if (renameTable) {
        const sourceExists = tableExists(master, renameTable.from)
        const targetExists = tableExists(master, renameTable.to)
        if (sourceExists && !targetExists) {
          missingStatements.push(statement)
          continue
        }
        if (!sourceExists && targetExists) continue
        unsafe = true
        break
      }

      unsafe = true
      break
    }

    if (unsafe) {
      if (report.backedUp.length === 0) report.backedUp = backupDatabaseFiles(input.dbPath)
      if (userTableNames.size > 0) {
        throw new Error(
          [
            `SQLite migrations are out of sync and could not be repaired automatically.`,
            `Backups: ${report.backedUp.join(", ") || "created on demand"}`,
            `Repair command: restore the backup, then rerun Jekko after fixing the migration state.`,
          ].join(" "),
        )
      }
      continue
    }

    if (missingStatements.length === 0) {
      if (report.backedUp.length === 0) report.backedUp = backupDatabaseFiles(input.dbPath)
      rebuildMigrationTable(db)
      upsertMigrationRow(db, entry)
      report.repaired.push(entry.name)
      master = masterRows(db)
      continue
    }

    if (report.backedUp.length === 0) report.backedUp = backupDatabaseFiles(input.dbPath)
    rebuildMigrationTable(db)
    for (const statement of missingStatements) {
      db.run(sql.raw(statement))
    }
    upsertMigrationRow(db, entry)
    report.repaired.push(entry.name)
    master = masterRows(db)
  }

  if (rows.length === 0 && nonInternalTables.length > 0) {
    rebuildMigrationTable(db)
  }

  return report
}

export function migrationHash(entry: Pick<MigrationEntry, "sql">) {
  return sha256Hex(entry.sql)
}
