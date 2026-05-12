import { describe, test, expect } from "bun:test"
import {
  checkPathAccess,
  checkNetworkAccess,
  checkEnvAccess,
  parseResourceLimit,
  checkResourceLimits,
  describeSandbox,
} from "../../src/session/daemon-sandbox"
import type { ZyalSandbox } from "../../src/agent-script/schema"

const testSandbox: ZyalSandbox = {
  paths: [
    { path: "/Users/ben/project/src", access: "write" },
    { path: "/Users/ben/project/docs", access: "read" },
    { path: "/etc", access: "deny" },
    { path: "*.env", access: "deny" },
  ],
  network: {
    outbound: "allowlist",
    allowlist: ["api.openai.com", "*.github.com", "registry.npmjs.org"],
  },
  resources: {
    max_file_size: "10MB",
    max_total_disk: "1GB",
    max_processes: 4,
  },
  env_inherit: ["HOME", "PATH", "NODE_ENV"],
  env_deny: ["AWS_SECRET_ACCESS_KEY"],
}

describe("daemon sandbox", () => {
  test("checkPathAccess allows write to writable path", () => {
    const result = checkPathAccess(testSandbox, "/Users/ben/project/src/app.ts", "write")
    expect(result.allowed).toBe(true)
    expect(result.access).toBe("write")
  })

  test("checkPathAccess allows read to read-only path", () => {
    const result = checkPathAccess(testSandbox, "/Users/ben/project/docs/README.md", "read")
    expect(result.allowed).toBe(true)
  })

  test("checkPathAccess denies write to read-only path", () => {
    const result = checkPathAccess(testSandbox, "/Users/ben/project/docs/README.md", "write")
    expect(result.allowed).toBe(false)
    expect(result.reason).toContain("read-only")
  })

  test("checkPathAccess denies access to denied path", () => {
    const result = checkPathAccess(testSandbox, "/etc/passwd", "read")
    expect(result.allowed).toBe(false)
    expect(result.access).toBe("deny")
  })

  test("checkPathAccess denies unmatched paths when sandbox active", () => {
    const result = checkPathAccess(testSandbox, "/tmp/random", "read")
    expect(result.allowed).toBe(false)
    expect(result.reason).toContain("no matching")
  })

  test("checkPathAccess allows everything when no sandbox", () => {
    const result = checkPathAccess(undefined, "/any/path", "write")
    expect(result.allowed).toBe(true)
    expect(result.access).toBe("unrestricted")
  })

  test("checkNetworkAccess allows allowlisted domains", () => {
    expect(checkNetworkAccess(testSandbox, "api.openai.com").allowed).toBe(true)
    expect(checkNetworkAccess(testSandbox, "registry.npmjs.org").allowed).toBe(true)
  })

  test("checkNetworkAccess allows wildcard subdomains", () => {
    expect(checkNetworkAccess(testSandbox, "api.github.com").allowed).toBe(true)
    expect(checkNetworkAccess(testSandbox, "gist.github.com").allowed).toBe(true)
  })

  test("checkNetworkAccess denies non-allowlisted domains", () => {
    const result = checkNetworkAccess(testSandbox, "evil.com")
    expect(result.allowed).toBe(false)
    expect(result.reason).toContain("not in allowlist")
  })

  test("checkNetworkAccess allows when outbound is allow", () => {
    const sandbox: ZyalSandbox = { network: { outbound: "allow" } }
    expect(checkNetworkAccess(sandbox, "anything.com").allowed).toBe(true)
  })

  test("checkNetworkAccess denies when outbound is deny", () => {
    const sandbox: ZyalSandbox = { network: { outbound: "deny" } }
    expect(checkNetworkAccess(sandbox, "anything.com").allowed).toBe(false)
  })

  test("checkEnvAccess allows inherited vars", () => {
    expect(checkEnvAccess(testSandbox, "HOME").allowed).toBe(true)
    expect(checkEnvAccess(testSandbox, "PATH").allowed).toBe(true)
  })

  test("checkEnvAccess denies non-inherited vars", () => {
    const result = checkEnvAccess(testSandbox, "RANDOM_VAR")
    expect(result.allowed).toBe(false)
  })

  test("checkEnvAccess denies explicitly denied vars", () => {
    const result = checkEnvAccess(testSandbox, "AWS_SECRET_ACCESS_KEY")
    expect(result.allowed).toBe(false)
    expect(result.reason).toContain("denied")
  })

  test("parseResourceLimit parses various units", () => {
    expect(parseResourceLimit("10MB")).toBe(10 * 1024 * 1024)
    expect(parseResourceLimit("1GB")).toBe(1024 * 1024 * 1024)
    expect(parseResourceLimit("512KB")).toBe(512 * 1024)
    expect(parseResourceLimit("100B")).toBe(100)
  })

  test("checkResourceLimits detects disk violations", () => {
    const result = checkResourceLimits(testSandbox, {
      fileCount: 10,
      totalDiskBytes: 2 * 1024 ** 3, // 2GB > 1GB limit
      processCount: 2,
    })
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]).toContain("disk")
  })

  test("checkResourceLimits detects process violations", () => {
    const result = checkResourceLimits(testSandbox, {
      fileCount: 10,
      totalDiskBytes: 100,
      processCount: 10, // > 4 limit
    })
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]).toContain("process")
  })

  test("checkResourceLimits passes when under limits", () => {
    const result = checkResourceLimits(testSandbox, {
      fileCount: 10,
      totalDiskBytes: 100,
      processCount: 2,
    })
    expect(result.violations).toHaveLength(0)
  })

  test("describeSandbox summarizes config", () => {
    const desc = describeSandbox(testSandbox)
    expect(desc).toContain("path rules")
    expect(desc).toContain("network")
    expect(desc).toContain("resource limits")
  })
})
