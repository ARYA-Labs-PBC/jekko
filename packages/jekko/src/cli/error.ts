import { NamedError } from "@jekko-ai/core/util/error"
import { errorFormat } from "@/util/error"
import { isRecord } from "@/util/record"

function isTaggedError(error: unknown, tag: string): error is Record<string, unknown> & { _tag: string } {
  return isRecord(error) && error._tag === tag
}

function errorData(error: unknown): Record<string, unknown> | undefined {
  return isRecord(error) && isRecord(error.data) ? error.data : undefined
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : []
}

function isIssue(value: unknown): value is { message: string; path: string[] } {
  return isRecord(value) && typeof value.message === "string" && Array.isArray(value.path)
}

export function FormatError(input: unknown) {
  // CliError: domain failure surfaced from an effectCmd handler via fail("...")
  if (isTaggedError(input, "CliError")) {
    if (typeof input.exitCode === "number") process.exitCode = input.exitCode
    return typeof input.message === "string" ? input.message : ""
  }

  // MCPFailed: { name: string }
  if (NamedError.hasName(input, "MCPFailed")) {
    const data = errorData(input)
    return `MCP server "${data?.name}" failed. Note, jekko does not support MCP authentication yet.`
  }

  // AccountServiceError, AccountTransportError: TaggedErrorClass
  if (isTaggedError(input, "AccountServiceError") || isTaggedError(input, "AccountTransportError")) {
    return typeof input.message === "string" ? input.message : ""
  }

  // ProviderModelNotFoundError: { providerID: string, modelID: string, suggestions?: string[] }
  if (NamedError.hasName(input, "ProviderModelNotFoundError")) {
    const data = errorData(input)
    const suggestions = stringArray(data?.suggestions)
    return [
      `Model not found: ${data?.providerID}/${data?.modelID}`,
      ...(suggestions.length ? ["Did you mean: " + suggestions.join(", ")] : []),
      `Try: \`jekko models\` to list available models`,
      `Or check your config (jekko.json) provider/model names`,
    ].join("\n")
  }

  // ProviderInitError: { providerID: string }
  if (NamedError.hasName(input, "ProviderInitError")) {
    const data = errorData(input)
    return `Failed to initialize provider "${data?.providerID}". Check credentials and configuration.`
  }

  // ConfigJsonError: { path: string, message?: string }
  if (NamedError.hasName(input, "ConfigJsonError")) {
    const data = errorData(input)
    return `Config file at ${data?.path} is not valid JSON(C)` + (data?.message ? `: ${data.message}` : "")
  }

  // ConfigDirectoryTypoError: { dir: string, path: string, suggestion: string }
  if (NamedError.hasName(input, "ConfigDirectoryTypoError")) {
    const data = errorData(input)
    return `Directory "${data?.dir}" in ${data?.path} is not valid. Rename the directory to "${data?.suggestion}" or remove it. This is a common typo.`
  }

  // ConfigFrontmatterError: { message: string }
  if (NamedError.hasName(input, "ConfigFrontmatterError")) {
    return errorData(input)?.message ?? ""
  }

  // ConfigInvalidError: { path?: string, message?: string, issues?: Array<{ message: string, path: string[] }> }
  if (NamedError.hasName(input, "ConfigInvalidError")) {
    const data = errorData(input)
    const path = data?.path
    const message = data?.message
    const issues = Array.isArray(data?.issues) ? data.issues.filter(isIssue) : []
    return [
      `Configuration is invalid${path && path !== "config" ? ` at ${path}` : ""}` + (message ? `: ${message}` : ""),
      ...issues.map((issue) => "↳ " + issue.message + " " + issue.path.join(".")),
    ].join("\n")
  }

  // UICancelledError: void (no data)
  if (NamedError.hasName(input, "UICancelledError")) {
    return ""
  }
}

export function FormatUnknownError(input: unknown): string {
  return errorFormat(input)
}
