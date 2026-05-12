import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import type { ZyalScript } from "../../src/agent-script/schema"
import { Permission } from "../../src/permission"
import {
  applyDaemonPermissions,
  buildDaemonPermissionRules,
  DAEMON_PERMISSIONS_APPLIED_EVENT,
} from "../../src/session/daemon-permissions"
import type { DaemonStore } from "../../src/session/daemon-store"
import type { Session } from "../../src/session/session"
import { SessionID } from "../../src/session/schema"

const sessionID = SessionID.make("ses_daemon_permissions")

function spec(input?: Partial<ZyalScript>): ZyalScript {
  return {
    version: "v1",
    intent: "daemon",
    confirm: "RUN_FOREVER",
    id: "daemon-permissions",
    job: { name: "daemon permissions", objective: "test" },
    stop: { all: [{ git_clean: {} }] },
    interaction: { user: "none" },
    ...input,
  } as ZyalScript
}

describe("daemon permissions", () => {
  test("builds unattended read-auto-allow markers and explicit ZYAL permissions", () => {
    const rules = buildDaemonPermissionRules(
      spec({
        permissions: {
          shell: "ask",
          edit: "allow",
          git_push: "deny",
          research: "allow",
          websearch: "deny",
          webfetch: "ask",
        },
      }),
    )

    expect(Permission.evaluate("zyal_auto_allow_reads", "enabled", rules).action).toBe("allow")
    expect(Permission.evaluate("zyal_unattended", "no_human_prompts", rules).action).toBe("allow")
    expect(Permission.evaluate("read", "/tmp/file", rules).action).toBe("allow")
    expect(Permission.evaluate("bash", "echo ok", rules).action).toBe("ask")
    expect(Permission.evaluate("edit", "src/file.ts", rules).action).toBe("allow")
    expect(Permission.evaluate("bash", "git push origin main", rules).action).toBe("deny")
    expect(Permission.evaluate("research", "external fact", rules).action).toBe("allow")
    expect(Permission.evaluate("websearch", "search terms", rules).action).toBe("deny")
    expect(Permission.evaluate("webfetch", "https://example.com", rules).action).toBe("ask")
  })

  test("applies markers to the session and writes an event receipt", async () => {
    let permission: Permission.Ruleset = [{ permission: "read", pattern: "secret/*", action: "deny" }]
    const events: Array<{ eventType: string; payload: Record<string, unknown> }> = []

    const sessions: Pick<Session.Interface, "get" | "setPermission"> = {
      get: () => Effect.succeed({ id: sessionID, permission } as any),
      setPermission: (input: { permission: Permission.Ruleset }) =>
        Effect.sync(() => {
          permission = input.permission
        }),
    }
    const store: Pick<DaemonStore.Interface, "appendEvent"> = {
      appendEvent: (input: { eventType: string; payload: Record<string, unknown> }) =>
        Effect.sync(() => {
          events.push(input)
          return input as any
        }),
    }

    const applied = applyDaemonPermissions({
      sessions,
      store,
      runID: "run_test",
      sessionID,
      spec: spec(),
      iteration: 0,
    }) as Effect.Effect<Permission.Ruleset, unknown, never>
    await Effect.runPromise(applied)

    expect(Permission.evaluate("zyal_auto_allow_reads", "enabled", permission).action).toBe("allow")
    expect(Permission.evaluate("zyal_unattended", "no_human_prompts", permission).action).toBe("allow")
    expect(Permission.evaluate("read", "secret/file", permission).action).toBe("deny")
    expect(events.map((event) => event.eventType)).toContain(DAEMON_PERMISSIONS_APPLIED_EVENT)
  })
})
