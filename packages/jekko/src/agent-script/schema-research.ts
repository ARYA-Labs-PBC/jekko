import { Schema } from "effect"
import { ZYAL_RESEARCH_BLOCK_VERSION } from "./version"

export const ZyalResearchMode = Schema.Union([
  Schema.Literal("auto"),
  Schema.Literal("web"),
  Schema.Literal("academic"),
  Schema.Literal("news"),
  Schema.Literal("code"),
  Schema.Literal("mixed"),
])
export type ZyalResearchMode = Schema.Schema.Type<typeof ZyalResearchMode>

export const ZyalResearchAutonomy = Schema.Union([
  Schema.Literal("agent_decides"),
  Schema.Literal("require_plan"),
  Schema.Literal("fixed_sources"),
])
export type ZyalResearchAutonomy = Schema.Schema.Type<typeof ZyalResearchAutonomy>

export const ZyalResearchProviderPolicy = Schema.Struct({
  prefer: Schema.optional(
    Schema.Array(
      Schema.Union([
        Schema.Literal("official_api"),
        Schema.Literal("primary_source"),
        Schema.Literal("privacy_first"),
      ]),
    ),
  ),
  allow: Schema.optional(Schema.Array(Schema.String)),
  missing_provider: Schema.optional(
    Schema.Union([Schema.Literal("skip_with_receipt"), Schema.Literal("pause"), Schema.Literal("fail")]),
  ),
})
export type ZyalResearchProviderPolicy = Schema.Schema.Type<typeof ZyalResearchProviderPolicy>

export const ZyalResearchExtraction = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  max_pages: Schema.optional(Schema.Number),
  allowed_extractors: Schema.optional(
    Schema.Array(Schema.Union([Schema.Literal("built_in"), Schema.Literal("jina"), Schema.Literal("firecrawl")])),
  ),
})
export type ZyalResearchExtraction = Schema.Schema.Type<typeof ZyalResearchExtraction>

export const ZyalResearchEvidence = Schema.Struct({
  require_citations: Schema.optional(Schema.Boolean),
  claim_level: Schema.optional(Schema.Boolean),
  store: Schema.optional(Schema.Literal("sqlite")),
})
export type ZyalResearchEvidence = Schema.Schema.Type<typeof ZyalResearchEvidence>

export const ZyalResearchSafety = Schema.Struct({
  redact_secrets: Schema.optional(Schema.Boolean),
  block_internal_urls: Schema.optional(Schema.Boolean),
  prompt_injection: Schema.optional(Schema.Literal("quarantine")),
  taint_label: Schema.optional(Schema.Literal("web_content")),
})
export type ZyalResearchSafety = Schema.Schema.Type<typeof ZyalResearchSafety>

export const ZyalResearchBudgets = Schema.Struct({
  max_queries: Schema.optional(Schema.Number),
  max_pages: Schema.optional(Schema.Number),
  max_cost_usd: Schema.optional(Schema.Number),
})
export type ZyalResearchBudgets = Schema.Schema.Type<typeof ZyalResearchBudgets>

export const ZyalResearch = Schema.Struct({
  version: Schema.Literal(ZYAL_RESEARCH_BLOCK_VERSION),
  mode: Schema.optional(ZyalResearchMode),
  autonomy: Schema.optional(ZyalResearchAutonomy),
  max_parallel: Schema.optional(Schema.Number),
  timeout_seconds: Schema.optional(Schema.Number),
  provider_policy: Schema.optional(ZyalResearchProviderPolicy),
  extraction: Schema.optional(ZyalResearchExtraction),
  evidence: Schema.optional(ZyalResearchEvidence),
  safety: Schema.optional(ZyalResearchSafety),
  budgets: Schema.optional(ZyalResearchBudgets),
})
export type ZyalResearch = Schema.Schema.Type<typeof ZyalResearch>
