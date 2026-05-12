import { describe, expect, test } from "bun:test"
import type { ZyalJankuraiReviewer } from "../../src/agent-script/schema"
import { DaemonReviewerPass } from "../../src/session/daemon-reviewer-pass"

describe("DaemonReviewerPass", () => {
  test("clean pass returns block=false and aggregate severity info", () => {
    const spec: ZyalJankuraiReviewer = {
      enabled: true,
      block_promotion: true,
      checklist: [
        { id: "edges", severity: "warning" },
        { id: "regress", severity: "blocker" },
      ],
    }
    const verdict = DaemonReviewerPass.evaluate({
      spec,
      results: [
        { id: "edges", outcome: "pass" },
        { id: "regress", outcome: "pass" },
      ],
    })
    expect(verdict.block).toBe(false)
    expect(verdict.severity).toBe("info")
    expect(verdict.gaps).toEqual([])
    expect(verdict.summary).toBe("reviewer:clean")
  })

  test("blocker outcome blocks promotion when block_promotion is on", () => {
    const spec: ZyalJankuraiReviewer = {
      enabled: true,
      block_promotion: true,
      checklist: [{ id: "regress", severity: "blocker" }],
    }
    const verdict = DaemonReviewerPass.evaluate({
      spec,
      results: [{ id: "regress", outcome: "block", notes: "new hard finding" }],
    })
    expect(verdict.block).toBe(true)
    expect(verdict.severity).toBe("blocker")
    expect(verdict.summary).toContain("reviewer:block")
    expect(verdict.gaps[0]?.severity).toBe("blocker")
  })

  test("block_promotion=false never blocks but still surfaces gaps", () => {
    const spec: ZyalJankuraiReviewer = {
      enabled: true,
      block_promotion: false,
      checklist: [{ id: "regress", severity: "blocker" }],
    }
    const verdict = DaemonReviewerPass.evaluate({
      spec,
      results: [{ id: "regress", outcome: "block" }],
    })
    expect(verdict.block).toBe(false)
    expect(verdict.gaps.length).toBe(1)
  })

  test("missing result for an item is treated as a gap, not a pass", () => {
    const spec: ZyalJankuraiReviewer = {
      enabled: true,
      block_promotion: true,
      checklist: [{ id: "edges", severity: "warning" }],
    }
    const verdict = DaemonReviewerPass.evaluate({ spec, results: [] })
    expect(verdict.block).toBe(false) // warning severity, not blocker
    expect(verdict.gaps.length).toBe(1)
    expect(verdict.gaps[0]?.notes).toBe("no result")
  })

  test("severityOverride trumps checklist severity", () => {
    const spec: ZyalJankuraiReviewer = {
      enabled: true,
      block_promotion: true,
      checklist: [{ id: "edges", severity: "info" }],
    }
    const verdict = DaemonReviewerPass.evaluate({
      spec,
      results: [{ id: "edges", outcome: "warn", severityOverride: "blocker" }],
    })
    expect(verdict.severity).toBe("blocker")
    // Note: outcome was `warn` not `block`, so block_from_results stays false;
    // but the override surfaces severity, AND the gap renders as blocker so
    // the block-on-blocker-severity rule kicks in.
    expect(verdict.block).toBe(true)
  })

  test("falls back to default checklist when none is declared", () => {
    expect(DaemonReviewerPass.effectiveChecklist(undefined).length).toBeGreaterThan(0)
    expect(DaemonReviewerPass.effectiveChecklist({ enabled: true })).toEqual(
      DaemonReviewerPass.DEFAULT_CHECKLIST,
    )
  })
})
