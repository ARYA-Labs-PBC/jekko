import { SchemaAST } from "effect"
import z from "zod"

const EMPTY_PARSE_OPTIONS = {} as SchemaAST.ParseOptions

export function applyChecks(out: z.ZodTypeAny, checks: SchemaAST.Checks, ast: SchemaAST.AST): z.ZodTypeAny {
  const filters: SchemaAST.Filter<unknown>[] = []
  const collect = (c: SchemaAST.Check<unknown>) => {
    if (c._tag === "FilterGroup") c.checks.forEach(collect)
    else filters.push(c)
  }
  checks.forEach(collect)

  const unhandled: SchemaAST.Filter<unknown>[] = []
  const translated = filters.reduce<z.ZodTypeAny>((acc, filter) => {
    const next = translateFilter(acc, filter)
    if (next) return next
    unhandled.push(filter)
    return acc
  }, out)

  if (unhandled.length === 0) return translated

  return translated.superRefine((value, ctx) => {
    for (const filter of unhandled) {
      const issue = filter.run(value, ast, EMPTY_PARSE_OPTIONS)
      if (!issue) continue
      const message = issueMessage(issue) ?? (filter.annotations as any)?.message ?? "Validation failed"
      ctx.addIssue({ code: "custom", message })
    }
  })
}

// Translate a well-known Effect Schema filter into a native Zod method call on
// `out`. Dispatch is keyed on `filter.annotations.meta._tag`, which every
// built-in check factory (isInt, isGreaterThan, isPattern, …) attaches at
// construction time. Returns `undefined` for unrecognised filters so the
// caller can fall back to the generic .superRefine path.
function translateFilter(out: z.ZodTypeAny, filter: SchemaAST.Filter<unknown>): z.ZodTypeAny | undefined {
  const meta = (filter.annotations as { meta?: Record<string, unknown> } | undefined)?.meta
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  if (!meta || typeof meta._tag !== "string") return undefined
  switch (meta._tag) {
    case "isInt":
      return call(out, "int")
    case "isFinite":
      return call(out, "finite")
    case "isGreaterThan":
      return call(out, "gt", meta.exclusiveMinimum)
    case "isGreaterThanOrEqualTo":
      return call(out, "gte", meta.minimum)
    case "isLessThan":
      return call(out, "lt", meta.exclusiveMaximum)
    case "isLessThanOrEqualTo":
      return call(out, "lte", meta.maximum)
    case "isBetween": {
      const lo = meta.exclusiveMinimum ? call(out, "gt", meta.minimum) : call(out, "gte", meta.minimum)
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      if (!lo) return undefined
      return meta.exclusiveMaximum ? call(lo, "lt", meta.maximum) : call(lo, "lte", meta.maximum)
    }
    case "isMultipleOf":
      return call(out, "multipleOf", meta.divisor)
    case "isMinLength":
      return call(out, "min", meta.minLength)
    case "isMaxLength":
      return call(out, "max", meta.maxLength)
    case "isLengthBetween": {
      const lo = call(out, "min", meta.minimum)
      // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
      if (!lo) return undefined
      return call(lo, "max", meta.maximum)
    }
    case "isPattern":
      return call(out, "regex", meta.regExp)
    case "isStartsWith":
      return call(out, "startsWith", meta.startsWith)
    case "isEndsWith":
      return call(out, "endsWith", meta.endsWith)
    case "isIncludes":
      return call(out, "includes", meta.includes)
    case "isUUID":
      return call(out, "uuid")
    case "isULID":
      return call(out, "ulid")
    case "isBase64":
      return call(out, "base64")
    case "isBase64Url":
      return call(out, "base64url")
  }
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return undefined
}

// Invoke a named Zod method on `target` if it exists, otherwise return
// undefined so the caller can fall back. Using this helper instead of a
// typed cast keeps `translateFilter` free of per-case narrowing noise.
function call(target: z.ZodTypeAny, method: string, ...args: unknown[]): z.ZodTypeAny | undefined {
  const fn = (target as unknown as Record<string, ((...a: unknown[]) => z.ZodTypeAny) | undefined>)[method]
  return typeof fn === "function" ? fn.apply(target, args) : undefined
}

function issueMessage(issue: any): string | undefined {
  if (typeof issue?.annotations?.message === "string") return issue.annotations.message
  if (typeof issue?.message === "string") return issue.message
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return undefined
}

export { call, issueMessage, translateFilter }
