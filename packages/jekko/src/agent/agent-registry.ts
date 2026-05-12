import path from "path"
import { mergeDeep } from "remeda"
import { Global } from "@jekko-ai/core/global"
import { Provider } from "@/provider/provider"
import { Permission } from "@/permission"
import type { Config } from "@/config/config"
import { Truncate } from "@/tool/truncate"
import type { Info } from "./agent"
import PROMPT_COMPACTION from "./prompt/compaction.txt"
import PROMPT_EXPLORE from "./prompt/explore.txt"
import PROMPT_SUMMARY from "./prompt/summary.txt"
import PROMPT_TITLE from "./prompt/title.txt"

export function buildAgentRegistry(input: { cfg: Config.Info; skillDirs: string[]; worktree: string }) {
  const defaults = Permission.fromConfig({
    "*": "allow",
    doom_loop: "ask",
    external_directory: {
      "*": "ask",
      ...Object.fromEntries(
        [Truncate.GLOB, path.join(Global.Path.tmp, "*"), ...input.skillDirs.map((dir) => path.join(dir, "*"))].map(
          (dir) => [dir, "allow"],
        ),
      ),
    },
    question: "deny",
    plan_enter: "deny",
    plan_exit: "deny",
    // mirrors github.com/github/gitignore Node.gitignore pattern for .env files
    read: {
      "*": "allow",
      "*.env": "ask",
      "*.env.*": "ask",
      "*.env.example": "allow",
    },
  })

  const user = Permission.fromConfig(input.cfg.permission ?? {})

  const agents: Record<string, Info> = {
    build: {
      name: "build",
      description: "The default agent. Executes tools based on configured permissions.",
      options: {},
      permission: Permission.merge(
        defaults,
        Permission.fromConfig({
          question: "allow",
          plan_enter: "allow",
        }),
        user,
      ),
      mode: "primary",
      native: true,
    },
    plan: {
      name: "plan",
      description: "Plan mode. Disallows all edit tools.",
      options: {},
      permission: Permission.merge(
        defaults,
        Permission.fromConfig({
          question: "allow",
          plan_exit: "allow",
          external_directory: {
            [path.join(Global.Path.data, "plans", "*")]: "allow",
          },
          edit: {
            "*": "deny",
            [path.join(".jekko", "plans", "*.md")]: "allow",
            [path.relative(input.worktree, path.join(Global.Path.data, path.join("plans", "*.md")))]: "allow",
          },
        }),
        user,
      ),
      mode: "primary",
      native: true,
    },
    general: {
      name: "general",
      description: `General-purpose agent for researching complex questions and executing multi-step tasks. Use this agent to execute multiple units of work in parallel and pair it with the \`research\` tool for cited external evidence.`,
      permission: Permission.merge(
        defaults,
        Permission.fromConfig({
          todowrite: "deny",
        }),
        user,
      ),
      options: {},
      mode: "subagent",
      native: true,
    },
    explore: {
      name: "explore",
      permission: Permission.merge(
        defaults,
        Permission.fromConfig({
          "*": "deny",
          grep: "allow",
          glob: "allow",
          list: "allow",
          bash: "allow",
          webfetch: "allow",
          websearch: "allow",
          read: "allow",
          external_directory: {
            "*": "ask",
            ...Object.fromEntries(
              [Truncate.GLOB, path.join(Global.Path.tmp, "*"), ...input.skillDirs.map((dir) => path.join(dir, "*"))].map(
                (dir) => [dir, "allow"],
              ),
            ),
          },
        }),
        user,
      ),
      description: `Fast agent specialized for exploring codebases. Use this when you need to quickly find files by patterns (eg. "src/components/**/*.tsx"), search code for keywords (eg. "API endpoints"), answer questions about the codebase (eg. "how do API endpoints work?"), or hand off external evidence gathering to the \`research\` tool first. When calling this agent, specify the desired thoroughness level: "quick" for basic searches, "medium" for moderate exploration, or "very thorough" for comprehensive analysis across multiple locations and naming conventions.`,
      prompt: PROMPT_EXPLORE,
      options: {},
      mode: "subagent",
      native: true,
    },
    compaction: {
      name: "compaction",
      mode: "primary",
      native: true,
      hidden: true,
      prompt: PROMPT_COMPACTION,
      permission: Permission.merge(
        defaults,
        Permission.fromConfig({
          "*": "deny",
        }),
        user,
      ),
      options: {},
    },
    title: {
      name: "title",
      mode: "primary",
      options: {},
      native: true,
      hidden: true,
      temperature: 0.5,
      permission: Permission.merge(
        defaults,
        Permission.fromConfig({
          "*": "deny",
        }),
        user,
      ),
      prompt: PROMPT_TITLE,
    },
    summary: {
      name: "summary",
      mode: "primary",
      options: {},
      native: true,
      hidden: true,
      permission: Permission.merge(
        defaults,
        Permission.fromConfig({
          "*": "deny",
        }),
        user,
      ),
      prompt: PROMPT_SUMMARY,
    },
  }

  for (const [key, value] of Object.entries(input.cfg.agent ?? {})) {
    if (value.disable) {
      delete agents[key]
      continue
    }
    let item = agents[key]
    if (!item)
      item = agents[key] = {
        name: key,
        mode: "all",
        permission: Permission.merge(defaults, user),
        options: {},
        native: false,
      }
    if (value.model) item.model = Provider.parseModel(value.model)
    item.variant = value.variant ?? item.variant
    item.prompt = value.prompt ?? item.prompt
    item.description = value.description ?? item.description
    item.temperature = value.temperature ?? item.temperature
    item.topP = value.top_p ?? item.topP
    item.mode = value.mode ?? item.mode
    item.color = value.color ?? item.color
    item.hidden = value.hidden ?? item.hidden
    item.name = value.name ?? item.name
    item.steps = value.steps ?? item.steps
    item.options = mergeDeep(item.options, value.options ?? {})
    item.permission = Permission.merge(item.permission, Permission.fromConfig(value.permission ?? {}))
  }

  for (const name in agents) {
    const agent = agents[name]
    const explicit = agent.permission.some((r) => {
      if (r.permission !== "external_directory") return false
      if (r.action !== "deny") return false
      return r.pattern === Truncate.GLOB
    })
    if (explicit) continue

    agents[name].permission = Permission.merge(
      agents[name].permission,
      Permission.fromConfig({ external_directory: { [Truncate.GLOB]: "allow" } }),
    )
  }

  return agents
}
