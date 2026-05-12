import { describe, expect, test } from "bun:test"
import { migrationTime } from "./jnoccio-local-helpers"

describe("jnoccio local helpers", () => {
  test("rejects malformed migration timestamps", () => {
    expect(migrationTime("not-a-migration")).toBe(0)
    expect(migrationTime("2026041317595x_invalid")).toBe(0)
  })

  test("parses a migration timestamp prefix", () => {
    expect(migrationTime("20260413175956_chief_energizer")).toBe(Date.UTC(2026, 3, 13, 17, 59, 56))
  })
})
