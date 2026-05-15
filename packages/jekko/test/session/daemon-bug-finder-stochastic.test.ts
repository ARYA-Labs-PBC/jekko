import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import { DaemonJankurai } from "../../src/session/daemon-jankurai"

describe("session.daemon-jankurai", () => {
  const config = (randomizeTies: boolean) =>
    ({
      enabled: true,
      root: ".",
      audit: {
        mode: "advisory",
        json: "target/jankurai/repo-score.json",
        md: "target/jankurai/repo-score.md",
        no_score_history: true,
      },
      repair_plan: {
        enabled: true,
        json: "target/jankurai/repair-plan.json",
        md: "target/jankurai/repair-plan.md",
      },
      task_source: "repair_plan",
      selection: {
        order: "blocker_first",
        randomize_ties: randomizeTies,
        max_risk: "high",
        skip_human_review_required: false,
        defer_rules: [],
        incubate_rules: [],
      },
      regression: {
        main_ref: "origin/main",
        compare_every_iterations: 5,
        mode: "advisory",
        max_new_hard_findings: 0,
        max_score_drop: 0,
      },
      verification: {
        require_clean_start: true,
        require_clean_after_checkpoint: true,
        proof_from_test_map: true,
        commands: [],
        audit_delta: "no_new_findings",
        rollback_unverified: true,
      },
    })

  const packet = {
    rule_id: "HLT-001-DEAD-MARKER",
    severity: "medium",
    risk_level: "medium",
    finding_path: "packages/jekko/src/a.ts",
    repair_eligibility: "agent-assisted",
  }

  test("adds priority jitter when randomize_ties is enabled", () => {
    const originalRandom = Math.random
    const values = [0.95, 0.05]
    Math.random = () => values.shift() ?? 0

    try {
      const first = DaemonJankurai.taskRoute({
        config: config(true),
        packet,
      })
      const second = DaemonJankurai.taskRoute({
        config: config(true),
        packet,
      })

      expect(first.status).toBe("queued")
      expect(second.status).toBe("queued")
      expect(first.priority).not.toBe(second.priority)
    } finally {
      Math.random = originalRandom
    }
  })

  test("keeps deterministic priorities when randomize_ties is disabled", () => {
    const originalRandom = Math.random
    const values = [0.95, 0.05]
    Math.random = () => values.shift() ?? 0

    try {
      const first = DaemonJankurai.taskRoute({
        config: config(false),
        packet,
      })
      const second = DaemonJankurai.taskRoute({
        config: config(false),
        packet,
      })

      expect(first.status).toBe("queued")
      expect(second.status).toBe("queued")
      expect(first.priority).toBe(second.priority)
    } finally {
      Math.random = originalRandom
    }
  })

  test("clamps worker pool growth to 10 even when run max is larger", async () => {
    const upserted: string[] = []
    const result = await Effect.runPromise(
      DaemonJankurai.runWorkerPool({
        cwd: "/tmp",
        run: { id: "run_workers" } as any,
        maxWorkers: 19,
        config: config(true),
        sessions: {} as any,
        prompt: {} as any,
        store: {
          listTasks: () => Effect.succeed([] as any),
          leaseSpecificTask: () => Effect.succeed(undefined),
          upsertWorker: (input: any) =>
            Effect.sync(() => {
              upserted.push(input.id)
              return input
            }),
        } as any,
        checks: {} as any,
        worktree: {} as any,
      }),
    )

    expect(result.workers).toBe(10)
    expect(result.results).toHaveLength(10)
    expect(upserted).toHaveLength(10)
    expect(result.started).toBe(0)
    expect(result.verified).toBe(0)
    expect(result.blocked).toBe(0)
  })
})
