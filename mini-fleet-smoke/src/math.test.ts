import { expect, test } from "bun:test"
import { sum } from "./math"

test("sum adds", () => {
  expect(sum(2, 3)).toBe(5)
})
