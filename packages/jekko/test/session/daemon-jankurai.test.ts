import { describe, expect } from "bun:test"
import { Effect, Layer, Schema } from "effect"
import { CrossSpawnSpawner } from "@jekko-ai/core/cross-spawn-spawner"
import { ZyalScriptSchema } from "../../src/agent-script/schema"
import { DaemonJankurai } from "../../src/session/daemon-jankurai"
import { DaemonStore } from "../../src/session/daemon-store"
import { ProjectTable } from "../../src/project/project.sql"
import { ProjectID } from "../../src/project/schema"
import { SessionTable } from "../../src/session/session.sql"
import { SessionID } from "../../src/session/schema"
import { Database } from "../../src/storage/db"
import { testEffect } from "../lib/effect"
import { provideTmpdirInstance } from "../fixture/fixture"

function spec() {
  return Schema.decodeUnknownSync(ZyalScriptSchema)({
    version: "v1",
    intent: "daemon",
    confirm: "RUN_FOREVER",
    id: "daemon-jankurai-test",
    job: {
      name: "Jankurai test",
      objective: "Exercise Jankurai task ingestion.",
    },
    stop: {
      all: [{ git_clean: { allow_untracked: false } }],
    },
    jankurai: {
      enabled: true,
      selection: {
        max_risk: "low",
        incubate_risk_at: "medium",
        defer_rules: ["HLT-010-SECRET-SPRAWL"],
      },
      verification: {
        commands: ["just fast"],
      },
    },
  })
}

function report() {
  return {
    score: 80,
    hard_findings: 2,
    soft_findings: 0,
    findings: [
      {
        fingerprint: "sha256:low",
        rule_id: "HLT-001-DEAD-MARKER",
        severity: "low",
        hardness: "soft",
        path: "packages/jekko/src/a.ts",
        problem: "quick repair",
        rerun_command: "just fast",
      },
      {
        fingerprint: "sha256:medium",
        rule_id: "HLT-006-DIRECT-DB-WRONG-LAYER",
        severity: "medium",
        hardness: "hard",
        path: "packages/jekko/src/b.ts",
        problem: "needs incubator",
      },
      {
        fingerprint: "sha256:secret",
        rule_id: "HLT-010-SECRET-SPRAWL",
        severity: "critical",
        hardness: "hard",
        path: "packages/jekko/src/secret.ts",
        problem: "secret-like value",
      },
    ],
  }
}

function repairPlan() {
  return {
    packets: [
      {
        finding_fingerprint: "sha256:low",
        finding_path: "packages/jekko/src/a.ts",
        rule_id: "HLT-001-DEAD-MARKER",
        severity: "low",
        risk_level: "low",
        repair_eligibility: "agent-assisted",
        allowed_paths: ["packages/jekko/src/a.ts"],
        forbidden_paths: ["agent/repo-score.json"],
        required_proof: ["just fast"],
      },
      {
        finding_fingerprint: "sha256:medium",
        finding_path: "packages/jekko/src/b.ts",
        rule_id: "HLT-006-DIRECT-DB-WRONG-LAYER",
        severity: "medium",
        risk_level: "medium",
        repair_eligibility: "agent-assisted",
        allowed_paths: ["packages/jekko/src/b.ts"],
      },
      {
        finding_fingerprint: "sha256:secret",
        finding_path: "packages/jekko/src/secret.ts",
        rule_id: "HLT-010-SECRET-SPRAWL",
        severity: "critical",
        risk_level: "critical",
        repair_eligibility: "never-auto",
        human_review_required: true,
        allowed_paths: ["packages/jekko/src/secret.ts"],
      },
    ],
  }
}

describe("session.daemon-jankurai", () => {
  const it = testEffect(Layer.mergeAll(DaemonStore.layer, CrossSpawnSpawner.defaultLayer))

  it.effect(
    "ingests repair-plan packets into queued, incubator, and blocked tasks",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const projectID = ProjectID.make("proj_daemon_jankurai")
          const sessionID = SessionID.make("ses_daemon_jankurai")
          const specValue = spec()

          yield* Effect.sync(() =>
            Database.use((db) => {
              db.insert(ProjectTable)
                .values({
                  id: projectID,
                  worktree: directory,
                  vcs: "git",
                  name: "Daemon Jankurai Test",
                  sandboxes: [],
                  time_created: Date.now(),
                  time_updated: Date.now(),
                })
                .run()
              db.insert(SessionTable)
                .values({
                  id: sessionID,
                  project_id: projectID,
                  slug: "daemon-jankurai",
                  directory,
                  title: "Daemon Jankurai Test",
                  version: "1.0.0",
                  time_created: Date.now(),
                  time_updated: Date.now(),
                })
                .run()
            }),
          )

          const store = yield* DaemonStore.Service
          const run = yield* store.createRun({
            rootSessionID: sessionID,
            activeSessionID: sessionID,
            spec: specValue,
            specHash: "sha256:jankurai",
          })
          const config = DaemonJankurai.resolveJankuraiConfig(specValue)!
          const result = yield* DaemonJankurai.ingestTasks({
            runID: run.id,
            config,
            store,
            report: report(),
            repairPlan: repairPlan(),
          })

          expect(result.upserted).toBe(3)
          expect(result.queued).toBe(1)
          expect(result.incubating).toBe(1)
          expect(result.blocked).toBe(1)

          const tasks = yield* store.listTasks(run.id)
          const low = tasks.find((task) => task.external_id === "sha256:low")
          const medium = tasks.find((task) => task.external_id === "sha256:medium")
          const secret = tasks.find((task) => task.external_id === "sha256:secret")
          expect(low?.id).toBe(DaemonJankurai.jankuraiTaskID("sha256:low"))
          expect(low?.status).toBe("queued")
          expect(low?.locked_paths_json).toEqual(["packages/jekko/src/a.ts"])
          expect(medium?.lane).toBe("incubator")
          expect(secret?.status).toBe("blocked")
          expect(secret?.blocked_reason).toContain("policy blocks")
        }),
      { git: true },
    ),
  )

  it.effect(
    "leases a specific task only when path locks do not overlap",
    provideTmpdirInstance(
      (directory) =>
        Effect.gen(function* () {
          const projectID = ProjectID.make("proj_daemon_jankurai_lease")
          const sessionID = SessionID.make("ses_daemon_jankurai_lease")
          const specValue = spec()

          yield* Effect.sync(() =>
            Database.use((db) => {
              db.insert(ProjectTable)
                .values({
                  id: projectID,
                  worktree: directory,
                  vcs: "git",
                  name: "Daemon Jankurai Lease Test",
                  sandboxes: [],
                  time_created: Date.now(),
                  time_updated: Date.now(),
                })
                .run()
              db.insert(SessionTable)
                .values({
                  id: sessionID,
                  project_id: projectID,
                  slug: "daemon-jankurai-lease",
                  directory,
                  title: "Daemon Jankurai Lease Test",
                  version: "1.0.0",
                  time_created: Date.now(),
                  time_updated: Date.now(),
                })
                .run()
            }),
          )

          const store = yield* DaemonStore.Service
          const run = yield* store.createRun({
            rootSessionID: sessionID,
            activeSessionID: sessionID,
            spec: specValue,
            specHash: "sha256:jankurai-lease",
          })
          yield* store.upsertTask({
            id: "jankurai-a",
            run_id: run.id,
            external_id: "sha256:a",
            title: "A",
            body_json: { risk: "low", locked_paths: ["packages/jekko/src/a.ts"] },
            status: "queued",
            priority: 10,
            lease_worker_id: null,
            lease_expires_at: null,
            locked_paths_json: ["packages/jekko/src/a.ts"],
            evidence_json: null,
          } as any)
          yield* store.upsertTask({
            id: "jankurai-b",
            run_id: run.id,
            external_id: "sha256:b",
            title: "B",
            body_json: { risk: "low", locked_paths: ["packages/jekko/src/a.ts"] },
            status: "queued",
            priority: 9,
            lease_worker_id: null,
            lease_expires_at: null,
            locked_paths_json: ["packages/jekko/src/a.ts"],
            evidence_json: null,
          } as any)

          const config = DaemonJankurai.resolveJankuraiConfig(specValue)!
          const first = yield* DaemonJankurai.leaseConflictFreeTask({
            runID: run.id,
            workerID: "worker-a",
            config,
            store,
          })
          const second = yield* DaemonJankurai.leaseConflictFreeTask({
            runID: run.id,
            workerID: "worker-b",
            config,
            store,
          })

          expect(first?.id).toBe("jankurai-a")
          expect(second).toBeUndefined()
        }),
      { git: true },
    ),
  )

  it.effect("compares reports by score drop and new hard findings", Effect.gen(function* () {
    const comparison = DaemonJankurai.compareReports({
      before: {
        score: 90,
        findings: [{ fingerprint: "old", severity: "low", hardness: "soft" }],
      },
      after: {
        score: 88,
        findings: [
          { fingerprint: "old", severity: "low", hardness: "soft" },
          { fingerprint: "new", severity: "high", hardness: "hard" },
        ],
      },
      maxNewHardFindings: 0,
      maxScoreDrop: 0,
    })
    expect(comparison.ok).toBe(false)
    expect(comparison.new_hard_findings).toEqual(["new"])
    expect(comparison.score_drop).toBe(2)
    yield* Effect.void
  }))
})
