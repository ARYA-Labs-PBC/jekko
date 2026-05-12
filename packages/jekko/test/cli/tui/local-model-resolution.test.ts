import { expect, test } from "bun:test"
import {
  chooseModelResolution,
  missingModelResolution,
  resolveModelChoice,
  resolveProviderModel,
  resolveRecentModel,
  resolvedModelResolution,
} from "../../../src/cli/cmd/tui/context/local-model-resolution"

test("resolveModelChoice returns an explicit missing resolution for empty values", () => {
  const result = resolveModelChoice(undefined, {
    source: "args",
    parseModel: (value) => ({ providerID: "p", modelID: value }),
    isModelValid: () => true,
  })

  expect(result).toEqual({
    kind: "missing",
    source: "args",
    reason: "No model value was configured",
    repairHint: expect.any(String),
    retryAfterMs: expect.any(Number),
  })
})

test("chooseModelResolution prefers the first resolved candidate", () => {
  expect(
    chooseModelResolution([
      missingModelResolution("args", "args missing"),
      resolvedModelResolution("recent", { providerID: "p", modelID: "recent" }),
      resolvedModelResolution("provider", { providerID: "p", modelID: "provider" }),
    ]),
  ).toEqual({
    kind: "resolved",
    source: "recent",
    model: { providerID: "p", modelID: "recent" },
  })
})

test("resolveProviderModel selects the first unlocked provider model", () => {
  expect(
    resolveProviderModel({
      data: {
        config: { model: "fallback" },
        provider: [
          {
            id: "one",
            models: {
              locked: { id: "locked", status: "locked" },
            },
          },
          {
            id: "two",
            models: {
              fallback: { id: "fallback", status: "ready" },
              alt: { id: "alt", status: "ready" },
            },
          },
        ],
      },
    }),
  ).toEqual({
    kind: "resolved",
    source: "provider",
    model: { providerID: "two", modelID: "fallback" },
  })
})

test("resolveRecentModel returns a missing resolution when no recent models are valid", () => {
  expect(
    resolveRecentModel(
      [
        { providerID: "p", modelID: "locked" },
        { providerID: "p", modelID: "also-locked" },
      ],
      () => false,
    ),
  ).toEqual({
    kind: "missing",
    source: "recent",
    reason: "No recent valid models were available",
    repairHint: expect.any(String),
    retryAfterMs: expect.any(Number),
  })
})
