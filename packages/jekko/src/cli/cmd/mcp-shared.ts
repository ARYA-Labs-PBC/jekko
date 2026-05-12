import { Effect } from "effect"
import path from "path"
import { applyEdits, modify } from "jsonc-parser"
import { Config } from "@/config/config"
import { Filesystem } from "@/util/filesystem"
import { MCP } from "../../mcp"
import { ConfigMCP } from "../../config/mcp"

type McpEntry = unknown

type McpConfigured = ConfigMCP.Info

export function isMcpConfigured(config: McpEntry): config is McpConfigured {
  return typeof config === "object" && config !== null && "type" in config
}

type McpRemote = Extract<McpConfigured, { type: "remote" }>

export function isMcpRemote(config: McpEntry): config is McpRemote {
  return isMcpConfigured(config) && config.type === "remote"
}

type AuthStatus = "authenticated" | "expired" | "not_authenticated"

export function getAuthStatusIcon(status: AuthStatus): string {
  switch (status) {
    case "authenticated":
      return "✓"
    case "expired":
      return "⚠"
    case "not_authenticated":
      return "✗"
  }
}

export function getAuthStatusText(status: AuthStatus): string {
  switch (status) {
    case "authenticated":
      return "authenticated"
    case "expired":
      return "expired"
    case "not_authenticated":
      return "not authenticated"
  }
}

type ConfigWithMcp = { mcp?: Record<string, McpEntry> }

export function configuredServers(config: ConfigWithMcp) {
  return Object.entries(config.mcp ?? {}).filter((entry): entry is [string, McpConfigured] => isMcpConfigured(entry[1]))
}

export function oauthServers(config: ConfigWithMcp) {
  return configuredServers(config).filter(
    (entry): entry is [string, McpRemote] => isMcpRemote(entry[1]) && entry[1].oauth !== false,
  )
}

export function listState() {
  return Effect.gen(function* () {
    const cfg = yield* Config.Service
    const mcp = yield* MCP.Service
    const config = (yield* cfg.get()) as { mcp?: Record<string, McpEntry> }
    const statuses = yield* mcp.status()
    const stored = yield* Effect.all(
      Object.fromEntries(configuredServers(config).map(([name]) => [name, mcp.hasStoredTokens(name)])),
      { concurrency: "unbounded" },
    )
    return { config, statuses, stored }
  })
}

export function authState() {
  return Effect.gen(function* () {
    const cfg = yield* Config.Service
    const mcp = yield* MCP.Service
    const config = (yield* cfg.get()) as { mcp?: Record<string, McpEntry> }
    const auth = yield* Effect.all(
      Object.fromEntries(oauthServers(config).map(([name]) => [name, mcp.getAuthStatus(name)])),
      { concurrency: "unbounded" },
    )
    return { config, auth }
  })
}

export async function resolveConfigPath(baseDir: string, global = false) {
  const candidates = [path.join(baseDir, "jekko.json"), path.join(baseDir, "jekko.jsonc")]

  if (!global) {
    candidates.push(path.join(baseDir, ".jekko", "jekko.json"), path.join(baseDir, ".jekko", "jekko.jsonc"))
  }

  for (const candidate of candidates) {
    if (await Filesystem.exists(candidate)) {
      return candidate
    }
  }

  return candidates[0]
}

export async function addMcpToConfig(name: string, mcpConfig: ConfigMCP.Info, configPath: string) {
  let text = "{}"
  if (await Filesystem.exists(configPath)) {
    text = await Filesystem.readText(configPath)
  }

  const edits = modify(text, ["mcp", name], mcpConfig, {
    formattingOptions: { tabSize: 2, insertSpaces: true },
  })
  const result = applyEdits(text, edits)

  await Filesystem.write(configPath, result)

  return configPath
}
