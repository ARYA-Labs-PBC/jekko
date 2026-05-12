import { Config } from "effect"
import { InstallationChannel } from "../installation/version"

function truthy(key: string) {
  const value = process.env[key]?.toLowerCase()
  return value === "true" || value === "1"
}

function falsy(key: string) {
  const value = process.env[key]?.toLowerCase()
  return value === "false" || value === "0"
}

// Channels that default to the new effect-httpapi server backend. The historical
// hono backend remains the default for stable (`prod`/`latest`) installs.
const HTTPAPI_DEFAULT_ON_CHANNELS = new Set(["dev", "beta", "local"])

function number(key: string) {
  const parsed = Number(process.env[key])
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined
}

const JEKKO_EXPERIMENTAL = truthy("JEKKO_EXPERIMENTAL")
const JEKKO_DISABLE_CLAUDE_CODE = truthy("JEKKO_DISABLE_CLAUDE_CODE")
const JEKKO_DISABLE_CLAUDE_CODE_SKILLS =
  JEKKO_DISABLE_CLAUDE_CODE || truthy("JEKKO_DISABLE_CLAUDE_CODE_SKILLS")
const copy = process.env["JEKKO_EXPERIMENTAL_DISABLE_COPY_ON_SELECT"]

export const Flag = {
  OTEL_EXPORTER_OTLP_ENDPOINT: process.env["OTEL_EXPORTER_OTLP_ENDPOINT"],
  OTEL_EXPORTER_OTLP_HEADERS: process.env["OTEL_EXPORTER_OTLP_HEADERS"],

  JEKKO_AUTO_SHARE: truthy("JEKKO_AUTO_SHARE"),
  JEKKO_AUTO_HEAP_SNAPSHOT: truthy("JEKKO_AUTO_HEAP_SNAPSHOT"),
  JEKKO_GIT_BASH_PATH: process.env["JEKKO_GIT_BASH_PATH"],
  JEKKO_CONFIG: process.env["JEKKO_CONFIG"],
  JEKKO_CONFIG_CONTENT: process.env["JEKKO_CONFIG_CONTENT"],
  JEKKO_DISABLE_AUTOUPDATE: truthy("JEKKO_DISABLE_AUTOUPDATE"),
  JEKKO_ALWAYS_NOTIFY_UPDATE: truthy("JEKKO_ALWAYS_NOTIFY_UPDATE"),
  JEKKO_DISABLE_PRUNE: truthy("JEKKO_DISABLE_PRUNE"),
  JEKKO_DISABLE_TERMINAL_TITLE: truthy("JEKKO_DISABLE_TERMINAL_TITLE"),
  JEKKO_SHOW_TTFD: truthy("JEKKO_SHOW_TTFD"),
  JEKKO_PERMISSION: process.env["JEKKO_PERMISSION"],
  JEKKO_DISABLE_DEFAULT_PLUGINS: truthy("JEKKO_DISABLE_DEFAULT_PLUGINS"),
  JEKKO_DISABLE_LSP_DOWNLOAD: truthy("JEKKO_DISABLE_LSP_DOWNLOAD"),
  JEKKO_ENABLE_EXPERIMENTAL_MODELS: truthy("JEKKO_ENABLE_EXPERIMENTAL_MODELS"),
  JEKKO_DISABLE_AUTOCOMPACT: truthy("JEKKO_DISABLE_AUTOCOMPACT"),
  JEKKO_DISABLE_MODELS_FETCH: truthy("JEKKO_DISABLE_MODELS_FETCH"),
  JEKKO_DISABLE_MOUSE: truthy("JEKKO_DISABLE_MOUSE"),
  JEKKO_DISABLE_CLAUDE_CODE,
  JEKKO_DISABLE_CLAUDE_CODE_PROMPT: JEKKO_DISABLE_CLAUDE_CODE || truthy("JEKKO_DISABLE_CLAUDE_CODE_PROMPT"),
  JEKKO_DISABLE_CLAUDE_CODE_SKILLS,
  JEKKO_DISABLE_EXTERNAL_SKILLS: truthy("JEKKO_DISABLE_EXTERNAL_SKILLS"),
  JEKKO_FAKE_VCS: process.env["JEKKO_FAKE_VCS"],
  JEKKO_SERVER_PASSWORD: process.env["JEKKO_SERVER_PASSWORD"],
  JEKKO_SERVER_USERNAME: process.env["JEKKO_SERVER_USERNAME"],
  JEKKO_ENABLE_QUESTION_TOOL: truthy("JEKKO_ENABLE_QUESTION_TOOL"),

  // Experimental
  JEKKO_EXPERIMENTAL,
  JEKKO_EXPERIMENTAL_FILEWATCHER: Config.boolean("JEKKO_EXPERIMENTAL_FILEWATCHER").pipe(
    Config.withDefault(false),
  ),
  JEKKO_EXPERIMENTAL_DISABLE_FILEWATCHER: Config.boolean("JEKKO_EXPERIMENTAL_DISABLE_FILEWATCHER").pipe(
    Config.withDefault(false),
  ),
  JEKKO_EXPERIMENTAL_ICON_DISCOVERY: JEKKO_EXPERIMENTAL || truthy("JEKKO_EXPERIMENTAL_ICON_DISCOVERY"),
  JEKKO_EXPERIMENTAL_DISABLE_COPY_ON_SELECT:
    copy === undefined ? process.platform === "win32" : truthy("JEKKO_EXPERIMENTAL_DISABLE_COPY_ON_SELECT"),
  JEKKO_ENABLE_EXA: truthy("JEKKO_ENABLE_EXA") || JEKKO_EXPERIMENTAL || truthy("JEKKO_EXPERIMENTAL_EXA"),
  JEKKO_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS: number("JEKKO_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS"),
  JEKKO_EXPERIMENTAL_OUTPUT_TOKEN_MAX: number("JEKKO_EXPERIMENTAL_OUTPUT_TOKEN_MAX"),
  JEKKO_EXPERIMENTAL_OXFMT: JEKKO_EXPERIMENTAL || truthy("JEKKO_EXPERIMENTAL_OXFMT"),
  JEKKO_EXPERIMENTAL_LSP_TY: truthy("JEKKO_EXPERIMENTAL_LSP_TY"),
  JEKKO_EXPERIMENTAL_LSP_TOOL: JEKKO_EXPERIMENTAL || truthy("JEKKO_EXPERIMENTAL_LSP_TOOL"),
  JEKKO_EXPERIMENTAL_PLAN_MODE: JEKKO_EXPERIMENTAL || truthy("JEKKO_EXPERIMENTAL_PLAN_MODE"),
  JEKKO_EXPERIMENTAL_MARKDOWN: !falsy("JEKKO_EXPERIMENTAL_MARKDOWN"),
  JEKKO_MODELS_URL: process.env["JEKKO_MODELS_URL"],
  JEKKO_MODELS_PATH: process.env["JEKKO_MODELS_PATH"],
  JEKKO_DB: process.env["JEKKO_DB"],
  JEKKO_DISABLE_CHANNEL_DB: truthy("JEKKO_DISABLE_CHANNEL_DB"),
  JEKKO_SKIP_MIGRATIONS: truthy("JEKKO_SKIP_MIGRATIONS"),
  JEKKO_STRICT_CONFIG_DEPS: truthy("JEKKO_STRICT_CONFIG_DEPS"),

  JEKKO_WORKSPACE_ID: process.env["JEKKO_WORKSPACE_ID"],
  // Defaults to true on dev/beta/local channels so internal users exercise the
  // new effect-httpapi server backend. Stable (`prod`/`latest`) installs stay
  // on the historical hono backend until the rollout is complete. An explicit env
  // var ("true"/"1" or "false"/"0") always wins, providing an opt-in for
  // stable users and an escape hatch for dev/beta users.
  JEKKO_EXPERIMENTAL_HTTPAPI:
    truthy("JEKKO_EXPERIMENTAL_HTTPAPI") ||
    (!falsy("JEKKO_EXPERIMENTAL_HTTPAPI") && HTTPAPI_DEFAULT_ON_CHANNELS.has(InstallationChannel)),
  JEKKO_EXPERIMENTAL_WORKSPACES: JEKKO_EXPERIMENTAL || truthy("JEKKO_EXPERIMENTAL_WORKSPACES"),
  JEKKO_EXPERIMENTAL_EVENT_SYSTEM: JEKKO_EXPERIMENTAL || truthy("JEKKO_EXPERIMENTAL_EVENT_SYSTEM"),

  // Evaluated at access time (not module load) because tests, the CLI, and
  // external tooling set these env vars at runtime.
  get JEKKO_DISABLE_PROJECT_CONFIG() {
    return truthy("JEKKO_DISABLE_PROJECT_CONFIG")
  },
  get JEKKO_TUI_CONFIG() {
    return process.env["JEKKO_TUI_CONFIG"]
  },
  get JEKKO_CONFIG_DIR() {
    return process.env["JEKKO_CONFIG_DIR"]
  },
  get JEKKO_PURE() {
    return truthy("JEKKO_PURE")
  },
  get JEKKO_PLUGIN_META_FILE() {
    return process.env["JEKKO_PLUGIN_META_FILE"]
  },
  get JEKKO_CLIENT() {
    return process.env["JEKKO_CLIENT"] ?? "cli"
  },
}
