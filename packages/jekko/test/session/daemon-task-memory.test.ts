import { describe, expect, test } from "bun:test"
import { DaemonTaskMemory } from "../../src/session/daemon-task-memory"

const task = {
  id: "task_1",
  run_id: "run_1",
  external_id: null,
  title: "Design incubator",
  body_json: { objective: "Strengthen the daemon incubator design." },
  status: "incubating",
  lane: "incubator",
  phase: "incubator_pass",
  difficulty_score: 0.8,
  risk_score: 0.7,
  readiness_score: 0.4,
  implementation_confidence: 0.2,
  verification_confidence: 0.2,
  attempt_count: 2,
  no_progress_count: 1,
  incubator_round: 0,
  incubator_status: "active",
  accepted_artifact_id: null,
  last_assessment_json: null,
  promotion_result_json: null,
  blocked_reason: null,
  priority: 1,
  lease_worker_id: null,
  lease_expires_at: null,
  locked_paths_json: null,
  evidence_json: null,
  time_created: Date.now(),
  time_updated: Date.now(),
} as any

const memories = [
  {
    id: "mem_1",
    run_id: "run_1",
    task_id: "task_1",
    kind: "current_best_plan",
    title: "Plan A",
    summary: "Use bounded passes and a host gate.",
    payload_json: null,
    source_pass_id: null,
    importance: 0.9,
    confidence: 0.7,
    time_created: Date.now(),
    time_updated: Date.now(),
  },
  {
    id: "mem_2",
    run_id: "run_1",
    task_id: "task_1",
    kind: "critic",
    title: "Risk",
    summary: "Prototype writes must be isolated.",
    payload_json: null,
    source_pass_id: null,
    importance: 0.8,
    confidence: 0.8,
    time_created: Date.now(),
    time_updated: Date.now(),
  },
] as any[]

describe("daemon task memory", () => {
  test("blind pass receives objective without prior scratch", () => {
    const packet = DaemonTaskMemory.buildContextPacket({ task, memories, passes: [], mode: "blind", passType: "scout" })
    expect(packet).toContain("Strengthen the daemon incubator design.")
    expect(packet).not.toContain("Plan A")
  })

  test("promotion context includes current best and objections", () => {
    const packet = DaemonTaskMemory.buildContextPacket({
      task,
      memories,
      passes: [],
      mode: "promotion",
      passType: "promotion_review",
    })
    expect(packet).toContain("Plan A")
    expect(packet).toContain("Prototype writes must be isolated")
    expect(packet).not.toContain("full transcript")
  })
})
