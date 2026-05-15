import { describe, expect, test } from "bun:test"
import { DaemonPass } from "../../src/session/daemon-pass"

describe("daemon pass helpers", () => {
  test("maps pass types to durable memory kinds", () => {
    expect(DaemonPass.memoryKindForPass("scout")).toBe("problem_statement")
    expect(DaemonPass.memoryKindForPass("strengthen")).toBe("current_best_plan")
    expect(DaemonPass.memoryKindForPass("promotion_review")).toBe("risk_review")
    expect(DaemonPass.memoryKindForPass("critical_reviewer")).toBe("critical_reviewer")
  })

  test("normalizes receipt objects without exposing raw structure", () => {
    const receipt = DaemonPass.normalizeReceipt(
      {
        summary: "done",
        claims: [{ claim: "ok", confidence: 0.8 }],
        recommended_next: "review",
        block: true,
        severity: "blocker",
        gaps: [{ id: "proof", severity: "blocker" }],
      },
      { passType: "idea", title: "Ideas" },
    )
    expect(receipt.summary).toBe("done")
    expect(receipt.recommended_next).toBe("review")
    expect(receipt.claims?.[0]?.claim).toBe("ok")
    expect(receipt.block).toBe(true)
    expect(receipt.gaps?.[0]?.id).toBe("proof")
  })
})
