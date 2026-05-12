import { describe, expect, test } from "bun:test"
import type { ConceptRecord } from "../../src/session/daemon-concept-memory"
import { DaemonTaskMemory, renderConceptsInScope } from "../../src/session/daemon-task-memory"

const baseTask = {
  id: "t1",
  run_id: "r1",
  external_id: null,
  title: "Demo",
  body_json: { objective: "exercise concept injection" },
  status: "incubating",
  lane: "incubator",
  phase: "incubator_pass",
  difficulty_score: 0,
  risk_score: 0,
  readiness_score: 0,
  implementation_confidence: 0,
  verification_confidence: 0,
  attempt_count: 0,
  no_progress_count: 0,
  incubator_round: 0,
  incubator_status: "none",
  accepted_artifact_id: null,
  last_assessment_json: null,
  promotion_result_json: null,
  blocked_reason: null,
  priority: 0,
  lease_worker_id: null,
  lease_expires_at: null,
  locked_paths_json: null,
  evidence_json: null,
  time_created: 1,
  time_updated: 1,
}

const concept: ConceptRecord = {
  conceptID: "auth.session",
  definition: "Sessions stored as JWTs with sliding expiry.",
  derivedFrom: ["auth.token"],
  proofRefs: ["receipt:abc"],
  confidence: 0.8,
}

describe("renderConceptsInScope", () => {
  test("empty returns empty string", () => {
    expect(renderConceptsInScope([])).toBe("")
  })

  test("renders id, derived_from, confidence", () => {
    const out = renderConceptsInScope([concept])
    expect(out.startsWith("<concepts-in-scope>")).toBe(true)
    expect(out.endsWith("</concepts-in-scope>")).toBe(true)
    expect(out).toContain("auth.session")
    expect(out).toContain("derived_from=[auth.token]")
    expect(out).toContain("conf=0.80")
  })

  test("hides invalidated concepts", () => {
    const out = renderConceptsInScope([{ ...concept, invalidatedAt: 1, invalidatedReason: "stale" }])
    expect(out).not.toContain("auth.session")
  })
})

describe("DaemonTaskMemory.buildContextPacket with concepts", () => {
  test("injects concepts block when concepts provided", () => {
    const packet = DaemonTaskMemory.buildContextPacket({
      task: baseTask as any,
      memories: [],
      passes: [],
      mode: "inherit",
      passType: "synthesize",
      concepts: [concept],
    })
    expect(packet).toContain("<concepts-in-scope>")
    expect(packet).toContain("auth.session")
  })

  test("omits concepts block when none are provided", () => {
    const packet = DaemonTaskMemory.buildContextPacket({
      task: baseTask as any,
      memories: [],
      passes: [],
      mode: "blind",
    })
    expect(packet).not.toContain("<concepts-in-scope>")
  })

  test("renders concepts under blind mode", () => {
    const packet = DaemonTaskMemory.buildContextPacket({
      task: baseTask as any,
      memories: [],
      passes: [],
      mode: "blind",
      concepts: [concept],
    })
    expect(packet).toContain("<concepts-in-scope>")
  })

  test("renders concepts under promotion mode", () => {
    const packet = DaemonTaskMemory.buildContextPacket({
      task: baseTask as any,
      memories: [],
      passes: [],
      mode: "promotion",
      concepts: [concept],
    })
    expect(packet).toContain("<concepts-in-scope>")
  })
})
