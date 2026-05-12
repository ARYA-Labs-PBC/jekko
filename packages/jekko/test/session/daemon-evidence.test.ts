import { describe, test, expect } from "bun:test"
import {
  createBundle,
  addArtifact,
  isBundleComplete,
  describeBundleStatus,
  verifyBundleHash,
  collectArtifact,
  getRequiredTypes,
} from "../../src/session/daemon-evidence"
import type { ZyalEvidence } from "../../src/agent-script/schema"

const testEvidence: ZyalEvidence = {
  require_before_promote: [
    { type: "test_results", must_pass: true },
    { type: "affected_files", must_be_known: true },
    { type: "rollback_plan", must_exist: true },
    { type: "risk_delta", max_increase: 0.1 },
  ],
  sign: "sha256",
  archive: true,
}

describe("daemon evidence", () => {
  test("createBundle with all requirements met is complete", () => {
    const artifacts = [
      collectArtifact({ type: "test_results", value: true }),
      collectArtifact({ type: "affected_files", value: ["a.ts", "b.ts"] }),
      collectArtifact({ type: "rollback_plan", value: "git revert HEAD" }),
      collectArtifact({ type: "risk_delta", value: 0.05 }),
    ]
    const bundle = createBundle(artifacts, testEvidence)
    expect(bundle.complete).toBe(true)
    expect(bundle.missing).toHaveLength(0)
    expect(bundle.violations).toHaveLength(0)
    expect(bundle.bundleHash).toBeDefined()
  })

  test("createBundle with missing evidence is incomplete", () => {
    const artifacts = [collectArtifact({ type: "test_results", value: true })]
    const bundle = createBundle(artifacts, testEvidence)
    expect(bundle.complete).toBe(false)
    expect(bundle.missing).toContain("affected_files")
    expect(bundle.missing).toContain("rollback_plan")
    expect(bundle.missing).toContain("risk_delta")
  })

  test("createBundle detects must_pass violation", () => {
    const artifacts = [
      collectArtifact({ type: "test_results", value: false }),
      collectArtifact({ type: "affected_files", value: ["a.ts"] }),
      collectArtifact({ type: "rollback_plan", value: "plan" }),
      collectArtifact({ type: "risk_delta", value: 0.05 }),
    ]
    const bundle = createBundle(artifacts, testEvidence)
    expect(bundle.complete).toBe(false)
    expect(bundle.violations).toHaveLength(1)
    expect(bundle.violations[0].type).toBe("test_results")
    expect(bundle.violations[0].requirement).toBe("must_pass")
  })

  test("createBundle detects max_increase violation", () => {
    const artifacts = [
      collectArtifact({ type: "test_results", value: true }),
      collectArtifact({ type: "affected_files", value: ["a.ts"] }),
      collectArtifact({ type: "rollback_plan", value: "plan" }),
      collectArtifact({ type: "risk_delta", value: 0.5 }),
    ]
    const bundle = createBundle(artifacts, testEvidence)
    expect(bundle.complete).toBe(false)
    expect(bundle.violations[0].type).toBe("risk_delta")
  })

  test("addArtifact updates bundle and re-evaluates", () => {
    const initial = createBundle([], testEvidence)
    expect(initial.complete).toBe(false)

    let bundle = addArtifact(initial, collectArtifact({ type: "test_results", value: true }), testEvidence)
    expect(bundle.missing).not.toContain("test_results")
    expect(bundle.complete).toBe(false)

    bundle = addArtifact(bundle, collectArtifact({ type: "affected_files", value: ["a.ts"] }), testEvidence)
    bundle = addArtifact(bundle, collectArtifact({ type: "rollback_plan", value: "plan" }), testEvidence)
    bundle = addArtifact(bundle, collectArtifact({ type: "risk_delta", value: 0.01 }), testEvidence)
    expect(bundle.complete).toBe(true)
  })

  test("isBundleComplete returns correct boolean", () => {
    const complete = createBundle(
      [
        collectArtifact({ type: "test_results", value: true }),
        collectArtifact({ type: "affected_files", value: ["a.ts"] }),
        collectArtifact({ type: "rollback_plan", value: "plan" }),
        collectArtifact({ type: "risk_delta", value: 0.01 }),
      ],
      testEvidence,
    )
    expect(isBundleComplete(complete)).toBe(true)

    const incomplete = createBundle([], testEvidence)
    expect(isBundleComplete(incomplete)).toBe(false)
  })

  test("describeBundleStatus summarizes status", () => {
    const incomplete = createBundle([], testEvidence)
    const desc = describeBundleStatus(incomplete)
    expect(desc).toContain("missing")
    expect(desc).toContain("test_results")

    const complete = createBundle(
      [
        collectArtifact({ type: "test_results", value: true }),
        collectArtifact({ type: "affected_files", value: ["a.ts"] }),
        collectArtifact({ type: "rollback_plan", value: "plan" }),
        collectArtifact({ type: "risk_delta", value: 0.01 }),
      ],
      testEvidence,
    )
    expect(describeBundleStatus(complete)).toContain("satisfied")
  })

  test("verifyBundleHash validates integrity", () => {
    const artifacts = [collectArtifact({ type: "test_results", value: true })]
    const bundle = createBundle(artifacts, testEvidence)
    expect(verifyBundleHash(bundle)).toBe(true)

    // Tamper with the hash
    const tampered = { ...bundle, bundleHash: "invalid" }
    expect(verifyBundleHash(tampered)).toBe(false)
  })

  test("verifyBundleHash returns true when no signing", () => {
    const bundle = createBundle([], { sign: "none" })
    expect(verifyBundleHash(bundle)).toBe(true)
  })

  test("collectArtifact with signing includes hash", () => {
    const artifact = collectArtifact({ type: "test", value: { pass: true }, sign: true })
    expect(artifact.hash).toBeDefined()
    expect(artifact.hash!.length).toBe(64) // sha256 hex
  })

  test("getRequiredTypes returns type list", () => {
    expect(getRequiredTypes(testEvidence)).toEqual(["test_results", "affected_files", "rollback_plan", "risk_delta"])
    expect(getRequiredTypes(undefined)).toEqual([])
  })

  test("createBundle with no config is always complete", () => {
    const bundle = createBundle([], undefined)
    expect(bundle.complete).toBe(true)
  })
})
