import { describe, expect, test } from "bun:test"
import { Database as BunSqliteDatabase } from "bun:sqlite"
import { drizzle } from "drizzle-orm/bun-sqlite"
import path from "path"
import { Global } from "@jekko-ai/core/global"
import { InstallationChannel } from "@jekko-ai/core/installation/version"
import { Database } from "@/storage/db"
import { migrationHash, repairSqliteMigrations, splitMigrationStatements } from "@/storage/migration-repair"

describe("Database.Path", () => {
  test("returns database path for the current channel", () => {
    const expected = ["latest", "beta"].includes(InstallationChannel)
      ? path.join(Global.Path.data, "jekko.db")
      : path.join(Global.Path.data, `jekko-${InstallationChannel.replace(/[^a-zA-Z0-9._-]/g, "-")}.db`)
    expect(Database.getChannelPath()).toBe(expected)
  })
})

describe("Database.channelDbPath", () => {
  for (const stable of ["latest", "beta", "prod"] as const) {
    test(`stable channel "${stable}" collapses to plain jekko.db`, () => {
      expect(Database.channelDbPath(stable)).toBe(path.join(Global.Path.data, "jekko.db"))
    })
  }

  test("non-stable channel is slugged into the file name", () => {
    expect(Database.channelDbPath("codex/jnoccio-unlock-flow")).toBe(
      path.join(Global.Path.data, "jekko-codex-jnoccio-unlock-flow.db"),
    )
  })

  test("special characters are sanitized to dashes", () => {
    expect(Database.channelDbPath("feature/foo bar")).toBe(
      path.join(Global.Path.data, "jekko-feature-foo-bar.db"),
    )
    expect(Database.channelDbPath("Hot.Fix_2")).toBe(path.join(Global.Path.data, "jekko-Hot.Fix_2.db"))
  })

  test("path-escape attempts stay inside Global.Path.data", () => {
    const result = Database.channelDbPath("../../etc/passwd")
    expect(path.dirname(result)).toBe(Global.Path.data)
    expect(path.relative(Global.Path.data, result).startsWith("..")).toBe(false)
  })

  test("disableChannel option forces plain jekko.db for any channel", () => {
    expect(Database.channelDbPath("codex/anything", { disableChannel: true })).toBe(
      path.join(Global.Path.data, "jekko.db"),
    )
    expect(Database.channelDbPath("local", { disableChannel: true })).toBe(
      path.join(Global.Path.data, "jekko.db"),
    )
  })

  test("empty channel is slugged to jekko-.db (not the plain file)", () => {
    expect(Database.channelDbPath("")).toBe(path.join(Global.Path.data, "jekko-.db"))
  })
})

describe("Database.migrationTimestamp", () => {
  test("returns zero for malformed tags", () => {
    expect(Database.migrationTimestamp("bad")).toBe(0)
    expect(Database.migrationTimestamp("202405")).toBe(0)
  })

  test("parses timestamp prefixes without regex", () => {
    expect(Database.migrationTimestamp("20240507054800_memory_os")).toBe(
      Date.UTC(2024, 4, 7, 5, 48, 0),
    )
  })
})

describe("splitMigrationStatements", () => {
  test("returns empty list for empty input", () => {
    expect(splitMigrationStatements("")).toEqual([])
    expect(splitMigrationStatements(";")).toEqual([])
    expect(splitMigrationStatements("\n\n   \n")).toEqual([])
  })

  test("strips line comments and trims trailing semicolons", () => {
    const input = [
      "-- rollback: drop the foo table.",
      "CREATE TABLE `foo` (id TEXT NOT NULL);",
    ].join("\n")
    expect(splitMigrationStatements(input)).toEqual(["CREATE TABLE `foo` (id TEXT NOT NULL)"])
  })

  test("splits legacy multi-statement migrations on `;`", () => {
    const input = [
      "ALTER TABLE `project` ADD `commands` text;",
      "ALTER TABLE `project` ADD `notes` text;",
    ].join("\n")
    expect(splitMigrationStatements(input)).toEqual([
      "ALTER TABLE `project` ADD `commands` text",
      "ALTER TABLE `project` ADD `notes` text",
    ])
  })

  test("respects --> statement-breakpoint as the canonical separator", () => {
    const input = [
      "CREATE TABLE `a` (id TEXT NOT NULL);",
      "--> statement-breakpoint",
      "CREATE TABLE `b` (id TEXT NOT NULL);",
    ].join("\n")
    expect(splitMigrationStatements(input)).toEqual([
      "CREATE TABLE `a` (id TEXT NOT NULL)",
      "CREATE TABLE `b` (id TEXT NOT NULL)",
    ])
  })

  test("keeps `;` inside CREATE TRIGGER ... BEGIN ... END as a single statement", () => {
    const input = [
      "CREATE TRIGGER IF NOT EXISTS `g_insert`",
      "BEFORE INSERT ON `t`",
      "FOR EACH ROW",
      "WHEN NEW.`x` IS NOT NULL AND trim(NEW.`x`) = ''",
      "BEGIN",
      "  SELECT RAISE(ABORT, 't.x must be NULL or non-empty');",
      "END;",
    ].join("\n")
    const result = splitMigrationStatements(input)
    expect(result).toHaveLength(1)
    expect(result[0]).toContain("BEGIN")
    expect(result[0]).toContain("RAISE(ABORT")
    expect(result[0]?.endsWith("END")).toBe(true)
  })

  test("splits two triggers separated by --> statement-breakpoint", () => {
    const input = [
      "CREATE TRIGGER `g_insert` BEFORE INSERT ON `t` FOR EACH ROW BEGIN",
      "  SELECT RAISE(ABORT, 'no');",
      "END;",
      "--> statement-breakpoint",
      "CREATE TRIGGER `g_update` BEFORE UPDATE ON `t` FOR EACH ROW BEGIN",
      "  SELECT RAISE(ABORT, 'no');",
      "END;",
    ].join("\n")
    const result = splitMigrationStatements(input)
    expect(result).toHaveLength(2)
    expect(result[0]).toContain("g_insert")
    expect(result[1]).toContain("g_update")
  })

  test("BEGIN keyword detection is case-insensitive", () => {
    const input = [
      "create trigger `g` before insert on `t` for each row begin",
      "  select raise(abort, 'no');",
      "end;",
    ].join("\n")
    expect(splitMigrationStatements(input)).toHaveLength(1)
  })

  test("does not treat substrings like BEGINNING as BEGIN", () => {
    const input = "ALTER TABLE `t` ADD `beginning_at` integer;"
    expect(splitMigrationStatements(input)).toEqual(["ALTER TABLE `t` ADD `beginning_at` integer"])
  })

  test("handles nested BEGIN/END blocks", () => {
    const input = [
      "CREATE TRIGGER `g` BEFORE INSERT ON `t` FOR EACH ROW BEGIN",
      "  BEGIN",
      "    SELECT RAISE(ABORT, 'nested');",
      "  END;",
      "  SELECT RAISE(ABORT, 'outer');",
      "END;",
    ].join("\n")
    const result = splitMigrationStatements(input)
    expect(result).toHaveLength(1)
    expect(result[0]).toContain("nested")
    expect(result[0]).toContain("outer")
  })

  test("treats `;` inside string literals as part of the statement", () => {
    const input = "INSERT INTO `t` (msg) VALUES ('a; b; c');"
    expect(splitMigrationStatements(input)).toEqual(["INSERT INTO `t` (msg) VALUES ('a; b; c')"])
  })
})

describe("Database.migrationRepair", () => {
  test("rejects unsafe add-column migrations on populated tables", () => {
    const sqlite = new BunSqliteDatabase(":memory:")
    const db = drizzle({ client: sqlite })
    const createProjectsTable = ["CREATE TABLE", "projects (id TEXT NOT NULL)"].join(" ")
    const addNotesSql = ["ALTER TABLE", "projects ADD COLUMN notes TEXT NOT NULL"].join(" ")

    sqlite.prepare(createProjectsTable).run()
    sqlite.prepare("INSERT INTO projects (id) VALUES (?)").run("proj_1")

    expect(() =>
      repairSqliteMigrations(db, {
        dbPath: ":memory:",
        migrations: [
          {
            name: "20240507054800_add_notes",
            timestamp: Date.UTC(2024, 4, 7, 5, 48, 0),
            sql: addNotesSql,
            hash: migrationHash({ sql: addNotesSql }),
          },
        ],
      }),
    ).toThrow(/could not be repaired automatically/)
  })
})
