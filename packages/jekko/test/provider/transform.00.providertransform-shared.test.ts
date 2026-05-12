import { expect, test } from "bun:test"
import { mimeToModality, sanitizeSurrogates, sdkKey } from "../../src/provider/transform-shared"

test("provider transform shared helpers keep SDK key and modality routing stable", () => {
  expect(sdkKey("@ai-sdk/anthropic")).toBe("anthropic")
  expect(sdkKey("@ai-sdk/google-vertex")).toBe("vertex")
  expect(sdkKey("ai-gateway-provider")).toBe("openaiCompatible")
  expect(sdkKey("unknown-provider")).toBeUndefined()

  expect(mimeToModality("image/png")).toEqual({ kind: "supported", modality: "image" })
  expect(mimeToModality("application/pdf")).toEqual({ kind: "supported", modality: "pdf" })
  expect(mimeToModality("text/plain")).toEqual({ kind: "unsupported" })

  expect(sanitizeSurrogates("ok")).toBe("ok")
})
