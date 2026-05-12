import { describe, expect, test } from "bun:test"
import { collectPromptInputs, resolveProviderAuthMethod } from "../../../../src/cli/cmd/tui/component/dialog-provider-methods"

describe("collectPromptInputs", () => {
  test("returns a typed cancellation result when a prompt is dismissed", async () => {
    const dialog = {
      replace(_: () => unknown, onCancel: () => void) {
        onCancel()
      },
    } as any

    const result = await collectPromptInputs({
      dialog,
      prompts: [
        {
          key: "token",
          message: "Token",
          type: "text",
        },
      ] as any,
    })

    expect(result).toEqual({ kind: "cancelled", promptKey: "token" })
  })
})

describe("resolveProviderAuthMethod", () => {
  test("returns the default API method when none are configured", async () => {
    const dialog = {
      replace() {
        throw new Error("dialog should not be used for the default method")
      },
    } as any

    await expect(
      resolveProviderAuthMethod({
        dialog,
        methods: [],
      }),
    ).resolves.toEqual({
      kind: "selected",
      index: 0,
      method: {
        type: "api",
        label: "API key",
      },
    })
  })
})
