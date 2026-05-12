import { describe, expect, test } from "bun:test"
import { Schema } from "effect"

import { namedSchemaError } from "../../src/util/named-schema-error"

describe("util.named-schema-error", () => {
  const TestError = namedSchemaError("TestError", {
    reason: Schema.String,
  })

  test("isInstance only accepts values that match the wire schema", () => {
    const instance = new TestError({ reason: "boom" })

    expect(TestError.isInstance(instance)).toBe(true)
    expect(TestError.isInstance({ name: "TestError", data: { reason: "boom" } })).toBe(true)
    expect(TestError.isInstance({ name: "TestError", data: { reason: 123 } })).toBe(false)
    expect(TestError.isInstance({ name: "OtherError", data: { reason: "boom" } })).toBe(false)
    expect(TestError.isInstance(null)).toBe(false)
  })
})
