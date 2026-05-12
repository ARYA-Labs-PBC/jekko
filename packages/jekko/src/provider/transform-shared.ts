import { Flag } from "@jekko-ai/core/flag/flag"
import type * as ModelsDev from "./models"

export type Modality = NonNullable<ModelsDev.Model["modalities"]>["input"][number]
export type MimeToModalityResult =
  | { kind: "supported"; modality: Modality }
  | { kind: "unsupported" }

export const OUTPUT_TOKEN_MAX = Flag.JEKKO_EXPERIMENTAL_OUTPUT_TOKEN_MAX || 32_000

export function sanitizeSurrogates(content: string) {
  return content.replace(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/g, "\uFFFD")
}

export function mimeToModality(mime: string): MimeToModalityResult {
  if (mime.startsWith("image/")) return { kind: "supported", modality: "image" }
  if (mime.startsWith("audio/")) return { kind: "supported", modality: "audio" }
  if (mime.startsWith("video/")) return { kind: "supported", modality: "video" }
  if (mime === "application/pdf") return { kind: "supported", modality: "pdf" }
  return { kind: "unsupported" }
}

const SDK_PROVIDER_KEYS: Record<string, string> = {
  "@ai-sdk/github-copilot": "copilot",
  "@ai-sdk/azure": "azure",
  "@ai-sdk/openai": "openai",
  "@ai-sdk/amazon-bedrock": "bedrock",
  "@ai-sdk/anthropic": "anthropic",
  "@ai-sdk/google-vertex/anthropic": "anthropic",
  "@ai-sdk/google-vertex": "vertex",
  "@ai-sdk/google": "google",
  "@ai-sdk/gateway": "gateway",
  "@openrouter/ai-sdk-provider": "openrouter",
  "ai-gateway-provider":
    // ai-gateway-provider/unified wraps createOpenAICompatible({ name: "Unified" }),
    // and @ai-sdk/openai-compatible parses compatibleOptions from one of
    // "openai-compatible" / "openaiCompatible" / "Unified" / "unified". The
    // "openai-compatible" key emits a deprecation warning at runtime, so we
    // pick the camelCase form the SDK now treats as canonical.
    "openaiCompatible",
}

// Maps npm package to the key the AI SDK expects for providerOptions.
export function sdkKey(npm: string): string | undefined {
  return SDK_PROVIDER_KEYS[npm]
}

export const WIDELY_SUPPORTED_EFFORTS = ["low", "medium", "high"]
export const OPENAI_EFFORTS = ["none", "minimal", ...WIDELY_SUPPORTED_EFFORTS, "xhigh"]

// OpenAI rolled out the `none` reasoning_effort tier on this date (Responses API).
// Models released before it 400 on `reasoning_effort: "none"`, so we only expose
// it as a variant for models new enough to accept it.
export const OPENAI_NONE_EFFORT_RELEASE_DATE = "2025-11-13"

// OpenAI rolled out the `xhigh` reasoning_effort tier on this date. Same reasoning.
export const OPENAI_XHIGH_EFFORT_RELEASE_DATE = "2025-12-04"

// Matches members of the gpt-5 family across the id formats we encounter:
//   "gpt-5", "gpt-5-nano", "gpt-5.4", "openai/gpt-5.4-codex".
// Anchored to start-of-string or "/" so it doesn't false-match "gpt-50" or "gpt-5o".
export const GPT5_FAMILY_RE = /(?:^|\/)gpt-5(?:[.-]|$)/
