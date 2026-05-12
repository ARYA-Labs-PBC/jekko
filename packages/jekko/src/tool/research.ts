import { Effect, Schema } from "effect"
import { HttpClient } from "effect/unstable/http"
import * as Tool from "./tool"
import DESCRIPTION from "./research.txt"
import * as McpExa from "./mcp-exa"

const DEFAULT_TIMEOUT_SECONDS = 30
const MAX_TIMEOUT_SECONDS = 120
const JNOCCIO_MCP_URL = (process.env.JNOCCIO_MCP_URL ?? `${(process.env.JNOCCIO_FUSION_URL ?? "http://127.0.0.1:4317").replace(/\/$/, "")}/mcp`).replace(/\/$/, "")

export const Parameters = Schema.Struct({
  query: Schema.String.annotate({ description: "Research query" }),
  mode: Schema.optional(Schema.Literals(["auto", "web", "academic", "news", "code", "mixed"])).annotate({
    description: "Research lane to prefer. Defaults to auto.",
  }),
  objective: Schema.optional(Schema.String).annotate({
    description: "Optional research objective to refine the query.",
  }),
  maxParallel: Schema.optional(Schema.Number).annotate({
    description: "Maximum parallel search fan-out. Defaults to 6.",
  }),
  timeoutSeconds: Schema.optional(Schema.Number).annotate({
    description: `Timeout for the advanced backend in seconds. Defaults to ${DEFAULT_TIMEOUT_SECONDS}.`,
  }),
})

type JnoccioResearchResult = {
  structuredContent?: {
    hits?: Array<{ title?: string; url?: string; snippet?: string }>
    evidence?: Array<{ id?: string; citation?: string }>
    receipts?: Array<Record<string, unknown>>
    warnings?: string[]
  }
  content?: Array<{ type?: string; text?: string }>
  isError?: boolean
}

async function callJnoccioResearch(arguments_: Record<string, unknown>, timeoutSeconds: number) {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), Math.max(1, timeoutSeconds) * 1000)
  try {
    const response = await fetch(JNOCCIO_MCP_URL, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      signal: controller.signal,
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "tools/call",
        params: {
          name: "jnoccio_research",
          arguments: arguments_,
        },
      }),
    })
    if (!response.ok) return undefined
    const payload = (await response.json().catch(() => undefined)) as { result?: JnoccioResearchResult } | undefined
    const result = payload?.result
    if (!result || result.isError) return undefined
    const structured = result.structuredContent
    if (structured) {
      return {
        text: JSON.stringify(structured, null, 2),
        receipts: structured.receipts ?? [],
        warnings: structured.warnings ?? [],
      }
    }
    const text = result.content?.find((part) => typeof part.text === "string")?.text
    if (!text) return undefined
    return {
      text,
      receipts: [],
      warnings: [],
    }
  } catch {
    return undefined
  } finally {
    clearTimeout(timer)
  }
}

function normalizeQuery(query: string, objective?: string) {
  const parts = [query.trim(), objective?.trim()].filter((part): part is string => Boolean(part))
  return parts.join(" ")
}

export const ResearchTool = Tool.define(
  "research",
  Effect.gen(function* () {
    const http = yield* HttpClient.HttpClient
    return {
      description: DESCRIPTION.replace("{{year}}", new Date().getFullYear().toString()),
      enabledByDefault: true,
      parameters: Parameters,
      execute: (params: Schema.Schema.Type<typeof Parameters>, ctx: Tool.Context) =>
        Effect.gen(function* () {
          const query = normalizeQuery(params.query, params.objective)
          yield* ctx.ask({
            permission: "research",
            patterns: [query],
            always: ["*"],
            metadata: {
              query,
              mode: params.mode ?? "auto",
              objective: params.objective,
              maxParallel: params.maxParallel,
              timeoutSeconds: params.timeoutSeconds,
            },
          })

          const timeoutSeconds = Math.min(Math.max(params.timeoutSeconds ?? DEFAULT_TIMEOUT_SECONDS, 1), MAX_TIMEOUT_SECONDS)
          const advanced = yield* Effect.promise(() =>
            callJnoccioResearch({
              query,
              mode: params.mode ?? "auto",
              objective: params.objective,
              max_parallel: params.maxParallel ?? 6,
              timeout_seconds: timeoutSeconds,
            }, timeoutSeconds),
          )

          if (advanced) {
            return {
              title: `Research: ${params.query}`,
              output: advanced.text,
              metadata: {
                backend: "jnoccio",
                warnings: advanced.warnings,
                receipts: advanced.receipts,
              },
            }
          }

          const fallback = yield* McpExa.call(
            http,
            "web_search_exa",
            McpExa.SearchArgs,
            {
              query,
              type: params.mode === "academic" || params.mode === "code" ? "deep" : "auto",
              numResults: params.maxParallel ? Math.max(1, Math.min(10, Math.round(params.maxParallel))) : 8,
              livecrawl: "preferred",
              contextMaxCharacters: 10_000,
            },
            `${timeoutSeconds} seconds`,
          )

          if (fallback) {
            return {
              title: `Research: ${params.query}`,
              output: fallback,
              metadata: {
                backend: "exa",
                warnings: ["advanced research backend unavailable; used Exa fallback"],
                receipts: [],
              },
            }
          }

          return {
            title: `Research: ${params.query}`,
            output: "No research results found. Please refine the query or enable the advanced research backend.",
            metadata: {
              backend: "unavailable",
              warnings: ["advanced research backend unavailable"],
              receipts: [],
            },
          }
        }).pipe(Effect.orDie),
    }
  }),
)
