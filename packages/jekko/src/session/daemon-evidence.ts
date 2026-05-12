// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
import type { ZyalEvidence, ZyalEvidenceRequirement } from "@/agent-script/schema"
import { createHash } from "crypto"

/**
 * Evidence bundle builder and validator for ZYAL v2.
 *
 * Manages typed proof bundles that must be satisfied before promotion.
 * Supports hash-chain signing for audit integrity.
 */

export type EvidenceArtifact = {
  readonly type: string
  readonly value: unknown
  readonly collectedAt: number
  readonly source?: string
  readonly hash?: string
}

export type EvidenceBundle = {
  readonly artifacts: EvidenceArtifact[]
  readonly bundleHash?: string
  readonly complete: boolean
  readonly missing: string[]
  readonly violations: EvidenceViolation[]
}

export type EvidenceViolation = {
  readonly type: string
  readonly requirement: string
  readonly actual: unknown
}

/**
 * Create a new evidence bundle from collected artifacts.
 */
export function createBundle(
  artifacts: EvidenceArtifact[],
  config: ZyalEvidence | undefined,
): EvidenceBundle {
  if (!config?.require_before_promote?.length) {
    return { artifacts, complete: true, missing: [], violations: [], bundleHash: undefined }
  }

  const missing: string[] = []
  const violations: EvidenceViolation[] = []

  for (const req of config.require_before_promote) {
    const artifact = artifacts.find((a) => a.type === req.type)
    if (!artifact) {
      missing.push(req.type)
      continue
    }
    const violation = checkRequirement(req, artifact)
    if (violation) violations.push(violation)
  }

  const bundleHash = config.sign === "sha256" ? hashBundle(artifacts) : undefined
  return {
    artifacts,
    bundleHash,
    complete: missing.length === 0 && violations.length === 0,
    missing,
    violations,
  }
}

/**
 * Add an artifact to an existing bundle and re-evaluate completeness.
 */
export function addArtifact(
  bundle: EvidenceBundle,
  artifact: EvidenceArtifact,
  config: ZyalEvidence | undefined,
): EvidenceBundle {
  const existing = bundle.artifacts.findIndex((a) => a.type === artifact.type)
  let artifacts: EvidenceArtifact[]
  if (existing >= 0) {
    artifacts = [...bundle.artifacts]
    artifacts[existing] = artifact
  } else {
    artifacts = [...bundle.artifacts, artifact]
  }
  return createBundle(artifacts, config)
}

/**
 * Check if a bundle satisfies all evidence requirements.
 */
export function isBundleComplete(bundle: EvidenceBundle): boolean {
  return bundle.complete
}

/**
 * Get human-readable summary of bundle status.
 */
export function describeBundleStatus(bundle: EvidenceBundle): string {
  if (bundle.complete) return "all evidence requirements satisfied"
  const parts: string[] = []
  if (bundle.missing.length > 0) parts.push(`missing: ${bundle.missing.join(", ")}`)
  if (bundle.violations.length > 0) {
    parts.push(`violations: ${bundle.violations.map((v) => `${v.type}(${v.requirement})`).join(", ")}`)
  }
  return parts.join("; ")
}

/**
 * Verify bundle integrity via hash chain.
 */
export function verifyBundleHash(bundle: EvidenceBundle): boolean {
  if (!bundle.bundleHash) return true
  const computed = hashBundle(bundle.artifacts)
  return computed === bundle.bundleHash
}

/**
 * Collect an artifact from a value, with optional signing.
 */
export function collectArtifact(input: {
  type: string
  value: unknown
  source?: string
  sign?: boolean
}): EvidenceArtifact {
  return {
    type: input.type,
    value: input.value,
    collectedAt: Date.now(),
    source: input.source,
    hash: input.sign ? hashValue(input.value) : undefined,
  }
}

/**
 * Get the list of required evidence types from config.
 */
export function getRequiredTypes(config: ZyalEvidence | undefined): string[] {
  return config?.require_before_promote?.map((r) => r.type) ?? []
}

function checkRequirement(req: ZyalEvidenceRequirement, artifact: EvidenceArtifact): EvidenceViolation | null {
  if (req.must_pass === true && artifact.value !== true) {
    return { type: req.type, requirement: "must_pass", actual: artifact.value }
  }
  if (req.must_exist === true && artifact.value === undefined) {
    return { type: req.type, requirement: "must_exist", actual: undefined }
  }
  if (req.must_be_known === true && artifact.value === undefined) {
    return { type: req.type, requirement: "must_be_known", actual: undefined }
  }
  if (req.max_increase !== undefined && typeof artifact.value === "number") {
    if (artifact.value > req.max_increase) {
      return { type: req.type, requirement: `max_increase:${req.max_increase}`, actual: artifact.value }
    }
  }
  // jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
  return null
}

function hashValue(value: unknown): string {
  return createHash("sha256").update(JSON.stringify(value)).digest("hex")
}

function hashBundle(artifacts: EvidenceArtifact[]): string {
  const data = artifacts
    .map((a) => `${a.type}:${JSON.stringify(a.value)}:${a.collectedAt}`)
    .sort()
    .join("|")
  return createHash("sha256").update(data).digest("hex")
}
