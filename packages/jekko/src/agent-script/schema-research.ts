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

export const ZyalResearchPaperScan = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  domains: Schema.optional(Schema.Array(Schema.String)),
  queries: Schema.optional(Schema.Array(Schema.String)),
  open_access: Schema.optional(Schema.Union([Schema.Literal("required"), Schema.Literal("preferred")])),
  max_papers: Schema.optional(Schema.Number),
  output_root: Schema.optional(Schema.String),
  raw_receipts: Schema.optional(Schema.String),
})
export type ZyalResearchPaperScan = Schema.Schema.Type<typeof ZyalResearchPaperScan>

export const ZyalResearchFullText = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  store: Schema.optional(Schema.Union([Schema.Literal("checked_in_json"), Schema.Literal("target_only")])),
  raw_receipts: Schema.optional(Schema.String),
  extraction_receipts: Schema.optional(Schema.String),
  license_policy: Schema.optional(Schema.Union([Schema.Literal("oa_only"), Schema.Literal("public_license_only")])),
})
export type ZyalResearchFullText = Schema.Schema.Type<typeof ZyalResearchFullText>

export const ZyalResearchDedupe = Schema.Struct({
  enabled: Schema.optional(Schema.Boolean),
  state_root: Schema.optional(Schema.String),
  duplicate_policy: Schema.optional(Schema.Union([Schema.Literal("skip_existing"), Schema.Literal("fail")])),
  hash_keys: Schema.optional(Schema.Array(Schema.String)),
})
export type ZyalResearchDedupe = Schema.Schema.Type<typeof ZyalResearchDedupe>

export const ZyalResearchContextPacking = Schema.Struct({
  strategy: Schema.optional(Schema.Union([Schema.Literal("hard"), Schema.Literal("best_effort")])),
  target_fill_ratio: Schema.optional(Schema.Number),
  output_reserve_tokens: Schema.optional(Schema.Number),
  safe_window_tokens: Schema.optional(Schema.Number),
})
export type ZyalResearchContextPacking = Schema.Schema.Type<typeof ZyalResearchContextPacking>

export const ZyalResearchQuestionBankAcceptance = Schema.Struct({
  min_auditor_agreement: Schema.optional(Schema.Number),
  min_answerability: Schema.optional(Schema.Number),
  max_blind_correct_rate_for_hard: Schema.optional(Schema.Number),
  reject_if_ambiguous: Schema.optional(Schema.Boolean),
})
export type ZyalResearchQuestionBankAcceptance = Schema.Schema.Type<typeof ZyalResearchQuestionBankAcceptance>

export const ZyalResearchQuestionBankWorkItem = Schema.Struct({
  id: Schema.String,
  publication_hash: Schema.String,
  paper_path: Schema.optional(Schema.String),
  challenge_path: Schema.optional(Schema.String),
  role: Schema.optional(Schema.Union([
    Schema.Literal("question_generator"),
    Schema.Literal("publication_extractor"),
    Schema.Literal("answerer"),
    Schema.Literal("saturated_answerer"),
    Schema.Literal("focused_auditor"),
    Schema.Literal("critic"),
    Schema.Literal("auditor"),
    Schema.Literal("judge_reducer"),
    Schema.Literal("reducer"),
    Schema.Literal("scorer"),
  ])),
})
export type ZyalResearchQuestionBankWorkItem = Schema.Schema.Type<typeof ZyalResearchQuestionBankWorkItem>

export const ZyalResearchQuestionBank = Schema.Struct({
  output_root: Schema.optional(Schema.String),
  papers_root: Schema.optional(Schema.String),
  challenges_root: Schema.optional(Schema.String),
  rejected_root: Schema.optional(Schema.String),
  work_items: Schema.optional(Schema.Array(ZyalResearchQuestionBankWorkItem)),
  acceptance: Schema.optional(ZyalResearchQuestionBankAcceptance),
})
export type ZyalResearchQuestionBank = Schema.Schema.Type<typeof ZyalResearchQuestionBank>

export const ZyalResearchAgentTrials = Schema.Struct({
  question_generators: Schema.optional(Schema.Number),
  answerers: Schema.optional(Schema.Number),
  model_profile: Schema.optional(Schema.String),
})
export type ZyalResearchAgentTrials = Schema.Schema.Type<typeof ZyalResearchAgentTrials>

export const ZyalResearchAudit = Schema.Struct({
  critics: Schema.optional(Schema.Number),
  focused_auditors: Schema.optional(Schema.Number),
  min_auditor_agreement: Schema.optional(Schema.Number),
  min_answerability: Schema.optional(Schema.Number),
})
export type ZyalResearchAudit = Schema.Schema.Type<typeof ZyalResearchAudit>

export const ZyalResearchRouteMetadata = Schema.Struct({
  required: Schema.optional(Schema.Boolean),
  require_request_id: Schema.optional(Schema.Boolean),
  require_provider: Schema.optional(Schema.String),
  require_model_profile: Schema.optional(Schema.Boolean),
})
export type ZyalResearchRouteMetadata = Schema.Schema.Type<typeof ZyalResearchRouteMetadata>

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
  paper_scan: Schema.optional(ZyalResearchPaperScan),
  full_text: Schema.optional(ZyalResearchFullText),
  dedupe: Schema.optional(ZyalResearchDedupe),
  context_packing: Schema.optional(ZyalResearchContextPacking),
  question_bank: Schema.optional(ZyalResearchQuestionBank),
  agent_trials: Schema.optional(ZyalResearchAgentTrials),
  audit: Schema.optional(ZyalResearchAudit),
  route_metadata: Schema.optional(ZyalResearchRouteMetadata),
})
export type ZyalResearch = Schema.Schema.Type<typeof ZyalResearch>
