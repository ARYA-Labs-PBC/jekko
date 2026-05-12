import { expect, test } from "bun:test"
import { hasConnectedProvider } from "../../../src/cli/cmd/tui/component/use-connected"

test("jekko counts as connected when it has free models", () => {
  expect(
    hasConnectedProvider({
      models: {
        "big-pickle": {
          cost: { input: 0 },
        },
      },
    }),
  ).toBeTrue()
})

test("provider without models does not count as connected", () => {
  expect(hasConnectedProvider({ models: {} })).toBeFalse()
})
