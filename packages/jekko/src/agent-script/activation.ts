import { Cause, Effect } from "effect"
import { normalizeZyalEnvelopeText, parseZyal } from "./parser"
import type { ZyalParsed, ZyalPreview } from "./schema"

export type ZyalDetection =
  | { readonly kind: "none" }
  | { readonly kind: "invalid"; readonly error: string }
  | { readonly kind: "preview"; readonly parsed: ZyalParsed; readonly preview: ZyalPreview }

export type ZyalEnvelopeScan =
  | { readonly kind: "none" }
  | {
      readonly kind: "zyal"
      readonly id: string
      readonly hasClose: boolean
      readonly hasArm: boolean
      readonly complete: boolean
    }

const ZYAL_OPEN_FAST_RE = /<<<ZYAL v1:daemon id=([A-Za-z0-9._-]+)>>>/

export function scanZyalEnvelope(text: string | undefined | null): ZyalEnvelopeScan {
  if (!text) return { kind: "none" }
  const open = text.match(ZYAL_OPEN_FAST_RE)
  const id = open?.[1]
  if (!id) return { kind: "none" }
  const escaped = escapeRegExp(id)
  const hasClose = new RegExp(`<<<END_ZYAL id=${escaped}>>>`).test(text)
  const hasArm = new RegExp(`ZYAL_ARM RUN_FOREVER id=${escaped}\\b`).test(text)
  return { kind: "zyal", id, hasClose, hasArm, complete: hasClose && hasArm }
}

export function detectZyal(text: string): ZyalDetection {
  const normalized = normalizeZyalEnvelopeText(text)
  if (scanZyalEnvelope(normalized).kind === "none") return { kind: "none" }
  const exit = Effect.runSyncExit(parseZyal(normalized, { requireArm: false }))
  if (exit._tag === "Failure") {
    return { kind: "invalid", error: String(Cause.squash(exit.cause)) }
  }
  return { kind: "preview", parsed: exit.value, preview: exit.value.preview }
}

function escapeRegExp(input: string) {
  return input.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}
