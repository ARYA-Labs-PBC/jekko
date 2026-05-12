import { Effect } from "effect"
import type { ZyalScript } from "@/agent-script/schema"
import { Permission, UNATTENDED_READ_AUTO_ALLOW_RULES } from "@/permission"
import type { Session } from "./session"
import type { DaemonStore } from "./daemon-store"
import { SessionID } from "./schema"

export const DAEMON_PERMISSIONS_APPLIED_EVENT = "permissions.unattended_read_auto_allow_applied"

type Mode = "ask" | "allow" | "deny"

const READ_PERMISSIONS = ["read", "list", "glob", "grep"] as const

function rule(permission: string, action: Mode, pattern = "*"): Permission.Rule {
  return { permission, pattern, action }
}

function addMode(rules: Permission.Ruleset, permission: string, mode?: Mode, pattern = "*") {
  if (!mode) return
  rules.push(rule(permission, mode, pattern))
}

function key(item: Permission.Rule) {
  return `${item.permission}\0${item.pattern}\0${item.action}`
}

export function buildDaemonPermissionRules(spec: ZyalScript): Permission.Ruleset {
  const rules: Permission.Ruleset = [...UNATTENDED_READ_AUTO_ALLOW_RULES]

  for (const permission of READ_PERMISSIONS) {
    rules.push(rule(permission, "allow"))
  }

  const modes = spec.permissions
  addMode(rules, "read", modes?.read)
  addMode(rules, "list", modes?.list)
  addMode(rules, "glob", modes?.glob)
  addMode(rules, "grep", modes?.grep)
  addMode(rules, "external_directory", modes?.external_directory)
  addMode(rules, "bash", modes?.shell)
  addMode(rules, "edit", modes?.edit)
  addMode(rules, "bash", modes?.git_commit, "git commit *")
  addMode(rules, "bash", modes?.git_push, "git push *")
  addMode(rules, "task", modes?.workers)
  addMode(rules, "mcp_*", modes?.mcp)
  addMode(rules, "research", modes?.research)
  addMode(rules, "websearch", modes?.websearch)
  addMode(rules, "webfetch", modes?.webfetch)

  return rules
}

export function mergeDaemonPermissionRules(existing: Permission.Ruleset, spec: ZyalScript): Permission.Ruleset {
  const generated = buildDaemonPermissionRules(spec)
  const seen = new Set(generated.map(key))
  const merged = [...generated]
  for (const item of existing) {
    const marker = key(item)
    if (seen.has(marker)) continue
    seen.add(marker)
    merged.push(item)
  }
  return merged
}

export const applyDaemonPermissions = Effect.fn("Daemon.applyPermissions")(function* (input: {
  sessions: Pick<Session.Interface, "get" | "setPermission">
  store?: Pick<DaemonStore.Interface, "appendEvent">
  runID?: string
  sessionID: SessionID
  spec: ZyalScript
  iteration?: number
}) {
  const session = yield* input.sessions.get(input.sessionID)
  const permission = mergeDaemonPermissionRules(session.permission ?? [], input.spec)
  yield* input.sessions.setPermission({ sessionID: session.id, permission })
  if (input.store && input.runID) {
    yield* input.store.appendEvent({
      runID: input.runID,
      iteration: input.iteration ?? 0,
      eventType: DAEMON_PERMISSIONS_APPLIED_EVENT,
      payload: {
        sessionID: session.id,
        permissions: permission.map((item) => `${item.permission}:${item.pattern}:${item.action}`),
      },
    })
  }
  return permission
})
