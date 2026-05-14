import type { LanguageModelV3, LanguageModelV3CallOptions, LanguageModelV3StreamPart, SharedV3ProviderMetadata } from "@ai-sdk/provider"

export type JnoccioRouteMetadata = {
  request_id?: string
  provider?: string
  model?: string
  route_mode?: string
  sampled?: boolean
  complexity_tier?: string
  primary_model_id?: string
  backup_model_ids?: string[]
  fusion_model_id?: string
  winner_model_id?: string
  confidence?: number
  model_decisions_hash?: string
  prompt_hash?: string
  context_hash?: string
  receipts_hash?: string
  route_confidence?: number
  token_usage?: {
    prompt_tokens?: number
    completion_tokens?: number
    total_tokens?: number
  }
  model_decisions?: Array<Record<string, unknown>>
}

const JNOCCIO_HEADER_PREFIX = "x-jnoccio-"
const JNOCCIO_HEADERS = {
  requestId: `${JNOCCIO_HEADER_PREFIX}request-id`,
  routeMode: `${JNOCCIO_HEADER_PREFIX}route-mode`,
  sampled: `${JNOCCIO_HEADER_PREFIX}sampled`,
  complexityTier: `${JNOCCIO_HEADER_PREFIX}complexity-tier`,
  primaryModelId: `${JNOCCIO_HEADER_PREFIX}primary-model-id`,
  backupModelIds: `${JNOCCIO_HEADER_PREFIX}backup-model-ids`,
  fusionModelId: `${JNOCCIO_HEADER_PREFIX}fusion-model-id`,
  winnerModelId: `${JNOCCIO_HEADER_PREFIX}winner-model-id`,
  confidence: `${JNOCCIO_HEADER_PREFIX}confidence`,
  modelDecisionsHash: `${JNOCCIO_HEADER_PREFIX}model-decisions-hash`,
} as const

type HeaderSource = Headers | Record<string, string | undefined | null>

function readHeader(source: HeaderSource | undefined, name: string): string | undefined {
  if (!source) return undefined
  if (source instanceof Headers) {
    const value = source.get(name)
    return value && value.length > 0 ? value : undefined
  }

  const direct = source[name]
  if (typeof direct === "string" && direct.length > 0) return direct

  const entry = Object.entries(source).find(([key]) => key.toLowerCase() === name.toLowerCase())
  if (!entry) return undefined
  const value = entry[1]
  return typeof value === "string" && value.length > 0 ? value : undefined
}

function parseBoolean(value: string | undefined): boolean | undefined {
  if (value == null) return undefined
  if (value === "true") return true
  if (value === "false") return false
  return undefined
}

function parseNumber(value: string | undefined): number | undefined {
  if (value == null) return undefined
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : undefined
}

function parseBackupModelIds(value: string | undefined): string[] | undefined {
  if (value == null) return undefined
  try {
    const parsed = JSON.parse(value) as unknown
    if (Array.isArray(parsed)) {
      const ids = parsed.filter((item): item is string => typeof item === "string" && item.length > 0)
      return ids.length > 0 ? ids : undefined
    }
  } catch {
    // fall back to comma-separated parsing below
  }

  const ids = value
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item.length > 0)
  return ids.length > 0 ? ids : undefined
}

export function jnoccioMetadataFromHeaders(source: HeaderSource | undefined): JnoccioRouteMetadata | undefined {
  const metadata: JnoccioRouteMetadata = {
    request_id: readHeader(source, JNOCCIO_HEADERS.requestId),
    route_mode: readHeader(source, JNOCCIO_HEADERS.routeMode),
    sampled: parseBoolean(readHeader(source, JNOCCIO_HEADERS.sampled)),
    complexity_tier: readHeader(source, JNOCCIO_HEADERS.complexityTier),
    primary_model_id: readHeader(source, JNOCCIO_HEADERS.primaryModelId),
    backup_model_ids: parseBackupModelIds(readHeader(source, JNOCCIO_HEADERS.backupModelIds)),
    fusion_model_id: readHeader(source, JNOCCIO_HEADERS.fusionModelId),
    winner_model_id: readHeader(source, JNOCCIO_HEADERS.winnerModelId),
    confidence: parseNumber(readHeader(source, JNOCCIO_HEADERS.confidence)),
    model_decisions_hash: readHeader(source, JNOCCIO_HEADERS.modelDecisionsHash),
  }

  return Object.values(metadata).some((value) => value != null) ? metadata : undefined
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return value != null && typeof value === "object" && !Array.isArray(value)
}

function stringField(record: Record<string, unknown>, key: string): string | undefined {
  const value = record[key]
  return typeof value === "string" && value.length > 0 ? value : undefined
}

function numberField(record: Record<string, unknown>, key: string): number | undefined {
  const value = record[key]
  return typeof value === "number" && Number.isFinite(value) ? value : undefined
}

function booleanField(record: Record<string, unknown>, key: string): boolean | undefined {
  const value = record[key]
  return typeof value === "boolean" ? value : undefined
}

function stringArrayField(record: Record<string, unknown>, key: string): string[] | undefined {
  const value = record[key]
  if (!Array.isArray(value)) return undefined
  const strings = value.filter((item): item is string => typeof item === "string" && item.length > 0)
  return strings.length > 0 ? strings : undefined
}

function tokenUsageField(record: Record<string, unknown>): JnoccioRouteMetadata["token_usage"] {
  const value = record.token_usage
  if (!isPlainRecord(value)) return undefined
  const tokenUsage = {
    prompt_tokens: numberField(value, "prompt_tokens"),
    completion_tokens: numberField(value, "completion_tokens"),
    total_tokens: numberField(value, "total_tokens"),
  }
  return Object.values(tokenUsage).some((item) => item != null) ? tokenUsage : undefined
}

export function jnoccioMetadataFromBody(parsedBody: unknown): JnoccioRouteMetadata | undefined {
  if (!isPlainRecord(parsedBody) || !isPlainRecord(parsedBody.jnoccio)) return undefined
  const body = parsedBody.jnoccio
  const metadata: JnoccioRouteMetadata = {
    request_id: stringField(body, "request_id"),
    provider: stringField(body, "provider"),
    model: stringField(body, "model"),
    route_mode: stringField(body, "route_mode"),
    sampled: booleanField(body, "sampled"),
    complexity_tier: stringField(body, "complexity_tier"),
    primary_model_id: stringField(body, "primary_model_id"),
    backup_model_ids: stringArrayField(body, "backup_model_ids"),
    fusion_model_id: stringField(body, "fusion_model_id"),
    winner_model_id: stringField(body, "winner_model_id"),
    confidence: numberField(body, "confidence"),
    route_confidence: numberField(body, "route_confidence"),
    prompt_hash: stringField(body, "prompt_hash"),
    context_hash: stringField(body, "context_hash"),
    receipts_hash: stringField(body, "receipts_hash"),
    model_decisions_hash: stringField(body, "model_decisions_hash"),
    token_usage: tokenUsageField(body),
    model_decisions: Array.isArray(body.model_decisions)
      ? body.model_decisions.filter((item): item is Record<string, unknown> => isPlainRecord(item))
      : undefined,
  }

  return Object.values(metadata).some((value) => value != null) ? metadata : undefined
}

export function createJnoccioMetadataExtractor() {
  return {
    async extractMetadata({ parsedBody }: { parsedBody: unknown }): Promise<SharedV3ProviderMetadata | undefined> {
      const jnoccio = jnoccioMetadataFromBody(parsedBody)
      return jnoccio ? ({ jnoccio } as SharedV3ProviderMetadata) : undefined
    },
    createStreamExtractor() {
      let jnoccio: JnoccioRouteMetadata | undefined
      return {
        processChunk(parsedChunk: unknown) {
          jnoccio = mergeJnoccioObjects(jnoccio, jnoccioMetadataFromBody(parsedChunk))
        },
        buildMetadata(): SharedV3ProviderMetadata | undefined {
          return jnoccio ? ({ jnoccio } as SharedV3ProviderMetadata) : undefined
        },
      }
    },
  }
}

function mergeJnoccioObjects(
  left: JnoccioRouteMetadata | undefined,
  right: JnoccioRouteMetadata | undefined,
): JnoccioRouteMetadata | undefined {
  if (!left) return right
  if (!right) return left
  return { ...left, ...right }
}

export function mergeProviderMetadata(
  providerMetadata: SharedV3ProviderMetadata | undefined,
  jnoccio: JnoccioRouteMetadata | undefined,
): SharedV3ProviderMetadata | undefined {
  if (!jnoccio) return providerMetadata

  const next = { ...(providerMetadata ?? {}) } as SharedV3ProviderMetadata
  const existing = (next as Record<string, any>).jnoccio
  ;(next as Record<string, any>).jnoccio = {
    ...(existing && typeof existing === "object" && !Array.isArray(existing) ? existing : {}),
    ...jnoccio,
  }
  return next
}

function enrichGenerateResult<T extends { providerMetadata?: SharedV3ProviderMetadata; response?: { headers?: HeaderSource } }>(
  result: T,
): T {
  const metadata = jnoccioMetadataFromHeaders(result.response?.headers)
  if (!metadata) return result
  return {
    ...result,
    providerMetadata: mergeProviderMetadata(result.providerMetadata, metadata),
  }
}

function enrichStreamResult<T extends { stream: ReadableStream<LanguageModelV3StreamPart>; response?: { headers?: HeaderSource } }>(
  result: T,
): T {
  const metadata = jnoccioMetadataFromHeaders(result.response?.headers)
  if (!metadata) return result

  const stream = result.stream.pipeThrough(
    new TransformStream<LanguageModelV3StreamPart, LanguageModelV3StreamPart>({
      transform(part, controller) {
        if (part.type === "finish") {
          controller.enqueue({
            ...part,
            providerMetadata: mergeProviderMetadata(part.providerMetadata, metadata),
          })
          return
        }
        controller.enqueue(part)
      },
    }),
  )

  return {
    ...result,
    stream,
  }
}

export function wrapJnoccioLanguageModel<T extends LanguageModelV3>(language: T): T {
  return new Proxy(language, {
    get(target, prop, receiver) {
      if (prop === "doGenerate") {
        return async (options: LanguageModelV3CallOptions) => enrichGenerateResult(await target.doGenerate.call(target, options))
      }

      if (prop === "doStream") {
        return async (options: LanguageModelV3CallOptions) => enrichStreamResult(await target.doStream.call(target, options))
      }

      return Reflect.get(target, prop, receiver)
    },
  }) as T
}
