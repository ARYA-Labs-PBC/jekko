import { describe, expect, test } from "bun:test"
import { buildDaemonIterationPrompt } from "../../src/session/daemon-context"

describe("session.daemon-context", () => {
  test("includes compact stage trace and blocked summary in the next prompt", () => {
    const prompt = buildDaemonIterationPrompt({
      parsed: {
        spec: {
          job: {
            name: "Daemon Prompt Trace",
            objective: "Show compact stage trace",
          },
          interaction: {},
          loop: {
            policy: "forever",
          },
        },
        preview: {
          stop_checks: ["shell:true"],
          research_enabled: false,
        },
      } as any,
      run: {
        id: "run-prompt-trace",
        iteration: 3,
        status: "running",
        phase: "running_iteration",
        epoch: 0,
        last_error: null,
      } as any,
      recentIterations: [],
      checkpointSha: "abc123",
      jankurai: {
        config: {
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
            order: "quick_wins_first",
            randomize_ties: false,
            max_risk: "low",
            skip_human_review_required: true,
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
            require_clean_start: false,
            require_clean_after_checkpoint: false,
            proof_from_test_map: false,
            commands: [],
            audit_delta: "no_new_findings",
            rollback_unverified: true,
          },
        } as any,
        tasks: [],
        workers: [],
        progress: {
          lastSuccessfulStage: "worker_wave.completed",
          recentStages: [
            { tone: "success", text: "bootstrap completed", stage: "bootstrap.completed" },
            { tone: "success", text: "integration_branch.ready", stage: "integration_branch.ready" },
            { tone: "warning", text: "worker_wave.completed", stage: "worker_wave.completed", reason: "no conflict-free task" },
          ],
          blockedReasons: ["no conflict-free task"],
          seededArtifacts: "reused",
          workerWave: {
            started: 1,
            verified: 0,
            blocked: 1,
            reason: "no conflict-free task",
          },
        },
      },
    })

    expect(prompt).toContain("Stage trace: bootstrap completed -> integration_branch.ready -> worker_wave.completed")
    expect(prompt).toContain("Jankurai blocked: no conflict-free task")
    expect(prompt).toContain("Jankurai seed: reused")
    expect(prompt).toContain("Jankurai worker wave: verified no")
  })
})
