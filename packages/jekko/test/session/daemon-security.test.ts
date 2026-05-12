import { describe, test, expect } from "bun:test"
import {
  checkTrustZone,
  checkRiskThreshold,
  scanForInjection,
  stripInjections,
  checkSecretAccess,
  shouldRedactFromLogs,
  getTrustZoneNames,
  getZonePaths,
} from "../../src/session/daemon-security"
import type { ZyalSecurity } from "../../src/agent-script/schema"

const testSecurity: ZyalSecurity = {
  trust_zones: {
    critical: {
      paths: ["src/auth", "src/payments"],
      require_approval: true,
      max_risk_score: 0.3,
    },
    infrastructure: {
      paths: ["infra/", "*.tf"],
      require_approval: false,
      max_risk_score: 0.5,
    },
  },
  injection: {
    scan_inputs: true,
    scan_outputs: true,
    deny_patterns: ["<script>*</script>", "widget(", "handler("],
    on_detect: "abort",
  },
  secrets: {
    allowed_env: ["API_KEY", "DATABASE_URL"],
    redact_from_logs: true,
    rotate_after: "30d",
  },
}

describe("daemon security", () => {
  test("checkTrustZone detects critical zone", () => {
    const result = checkTrustZone(testSecurity, "src/auth/login.ts")
    expect(result.zone).toBe("critical")
    expect(result.requiresApproval).toBe(true)
    expect(result.allowed).toBe(false)
  })

  test("checkTrustZone detects infrastructure zone", () => {
    const result = checkTrustZone(testSecurity, "infra/main.tf")
    expect(result.zone).toBe("infrastructure")
    expect(result.requiresApproval).toBe(false)
    expect(result.allowed).toBe(true)
  })

  test("checkTrustZone allows paths outside zones", () => {
    const result = checkTrustZone(testSecurity, "src/utils/helpers.ts")
    expect(result.zone).toBeNull()
    expect(result.allowed).toBe(true)
  })

  test("checkTrustZone handles no security config", () => {
    const result = checkTrustZone(undefined, "anything.ts")
    expect(result.allowed).toBe(true)
  })

  test("checkRiskThreshold blocks high-risk changes in critical zone", () => {
    const result = checkRiskThreshold(testSecurity, "src/auth/token.ts", 0.8)
    expect(result.allowed).toBe(false)
    expect(result.reason).toContain("exceeds")
  })

  test("checkRiskThreshold allows low-risk changes in critical zone", () => {
    const result = checkRiskThreshold(testSecurity, "src/auth/token.ts", 0.1)
    expect(result.allowed).toBe(true)
  })

  test("checkRiskThreshold allows changes outside zones", () => {
    const result = checkRiskThreshold(testSecurity, "src/utils/helpers.ts", 0.9)
    expect(result.allowed).toBe(true)
  })

  test("scanForInjection detects script tags", () => {
    const result = scanForInjection(testSecurity, "hello <script>alert(1)</script> world", "input")
    expect(result.clean).toBe(false)
    expect(result.detections).toHaveLength(1)
    expect(result.action).toBe("abort")
  })

  test("scanForInjection detects widget calls", () => {
    const result = scanForInjection(testSecurity, "widget('malicious')", "output")
    expect(result.clean).toBe(false)
    expect(result.detections[0].pattern).toBe("widget(")
  })

  test("scanForInjection returns clean for safe text", () => {
    const result = scanForInjection(testSecurity, "const x = 42; console.log(x);", "input")
    expect(result.clean).toBe(true)
    expect(result.detections).toHaveLength(0)
  })

  test("scanForInjection ignores empty deny patterns", () => {
    const security: ZyalSecurity = {
      injection: { scan_inputs: true, scan_outputs: true, deny_patterns: ["", "widget("], on_detect: "warn" },
    }
    const result = scanForInjection(security, "widget('x')", "input")
    expect(result.clean).toBe(false)
    expect(result.detections).toHaveLength(1)
    expect(result.detections[0].pattern).toBe("widget(")
  })

  test("scanForInjection skips when not scanning direction", () => {
    const security: ZyalSecurity = {
      injection: { scan_inputs: true, scan_outputs: false, deny_patterns: ["widget("] },
    }
    const result = scanForInjection(security, "widget('x')", "output")
    expect(result.clean).toBe(true) // not scanning outputs
  })

  test("stripInjections removes detected patterns", () => {
    const result = stripInjections(testSecurity, "safe text widget('x') more text handler('rm')")
    expect(result).not.toContain("widget(")
    expect(result).not.toContain("handler(")
    expect(result).toContain("[STRIPPED]")
    expect(result).toContain("safe text")
  })

  test("checkSecretAccess allows listed env vars", () => {
    expect(checkSecretAccess(testSecurity, "API_KEY").allowed).toBe(true)
    expect(checkSecretAccess(testSecurity, "DATABASE_URL").allowed).toBe(true)
  })

  test("checkSecretAccess denies unlisted env vars", () => {
    const result = checkSecretAccess(testSecurity, "RANDOM_SECRET")
    expect(result.allowed).toBe(false)
  })

  test("shouldRedactFromLogs returns config value", () => {
    expect(shouldRedactFromLogs(testSecurity)).toBe(true)
    expect(shouldRedactFromLogs(undefined)).toBe(false)
  })

  test("getTrustZoneNames returns zone names", () => {
    expect(getTrustZoneNames(testSecurity)).toEqual(["critical", "infrastructure"])
    expect(getTrustZoneNames(undefined)).toEqual([])
  })

  test("getZonePaths returns paths for zone", () => {
    expect(getZonePaths(testSecurity, "critical")).toEqual(["src/auth", "src/payments"])
    expect(getZonePaths(testSecurity, "nonexistent")).toEqual([])
  })
})
