import { describe, test, expect } from "bun:test"
import {
  initializeSkillRegistry,
  getSkill,
  canInvoke,
  recordInvocation,
  registerSkill,
  getSkillsByTrust,
  getSkillTools,
  deactivateSkill,
  getActiveSkills,
} from "../../src/session/daemon-skills"
import type { ZyalSkills } from "../../src/agent-script/schema"

const testSkills: ZyalSkills = {
  registry: {
    code_review: {
      description: "Review code changes",
      agent: "plan",
      tools: ["read_file", "grep_search"],
      trust: "builtin",
      writes: "none",
    },
    test_writer: {
      description: "Generate unit tests",
      agent: "build",
      tools: ["write_file", "run_command"],
      trust: "verified",
      writes: "isolated_worktree",
      timeout: "5m",
    },
    experimental: {
      description: "Experimental tool",
      trust: "community",
    },
  },
  allow_creation: true,
  max_skills: 5,
}

describe("daemon skills", () => {
  test("initializeSkillRegistry creates registry from config", () => {
    const registry = initializeSkillRegistry(testSkills)
    expect(registry.skills.size).toBe(3)
    expect(registry.allowCreation).toBe(true)
    expect(registry.maxSkills).toBe(5)
  })

  test("initializeSkillRegistry handles undefined config", () => {
    const registry = initializeSkillRegistry(undefined)
    expect(registry.skills.size).toBe(0)
    expect(registry.allowCreation).toBe(false)
  })

  test("getSkill returns skill definition", () => {
    const registry = initializeSkillRegistry(testSkills)
    const skill = getSkill(registry, "code_review")
    expect(skill).toBeDefined()
    expect(skill!.definition.trust).toBe("builtin")
  })

  test("getSkill returns undefined for unknown skill", () => {
    const registry = initializeSkillRegistry(testSkills)
    expect(getSkill(registry, "nonexistent")).toBeUndefined()
  })

  test("canInvoke allows active skills", () => {
    const registry = initializeSkillRegistry(testSkills)
    expect(canInvoke(registry, "code_review")).toEqual({ allowed: true })
  })

  test("canInvoke rejects unknown skills", () => {
    const registry = initializeSkillRegistry(testSkills)
    const result = canInvoke(registry, "nonexistent")
    expect(result.allowed).toBe(false)
    expect(result.reason).toContain("not found")
  })

  test("canInvoke rejects inactive skills", () => {
    let registry = initializeSkillRegistry(testSkills)
    registry = deactivateSkill(registry, "code_review")
    const result = canInvoke(registry, "code_review")
    expect(result.allowed).toBe(false)
    expect(result.reason).toContain("inactive")
  })

  test("recordInvocation updates count", () => {
    let registry = initializeSkillRegistry(testSkills)
    registry = recordInvocation(registry, "code_review")
    registry = recordInvocation(registry, "code_review")
    expect(getSkill(registry, "code_review")!.invokeCount).toBe(2)
    expect(getSkill(registry, "code_review")!.lastInvokedAt).toBeDefined()
  })

  test("registerSkill adds new skill", () => {
    const registry = initializeSkillRegistry(testSkills)
    const result = registerSkill(registry, "new_skill", { description: "new", trust: "local" })
    expect(result.registered).toBe(true)
    expect(result.registry.skills.size).toBe(4)
  })

  test("registerSkill rejects when creation disabled", () => {
    const registry = initializeSkillRegistry({ ...testSkills, allow_creation: false })
    const result = registerSkill(registry, "new_skill", { description: "new" })
    expect(result.registered).toBe(false)
    expect(result.reason).toContain("disabled")
  })

  test("registerSkill rejects when max reached", () => {
    const registry = initializeSkillRegistry({ ...testSkills, max_skills: 3 })
    const result = registerSkill(registry, "new_skill", { description: "new" })
    expect(result.registered).toBe(false)
    expect(result.reason).toContain("max skills")
  })

  test("registerSkill rejects duplicates", () => {
    const registry = initializeSkillRegistry(testSkills)
    const result = registerSkill(registry, "code_review", { description: "dup" })
    expect(result.registered).toBe(false)
    expect(result.reason).toContain("already exists")
  })

  test("getSkillsByTrust filters correctly", () => {
    const registry = initializeSkillRegistry(testSkills)
    expect(getSkillsByTrust(registry, "builtin")).toHaveLength(1)
    expect(getSkillsByTrust(registry, "verified")).toHaveLength(1)
    expect(getSkillsByTrust(registry, "community")).toHaveLength(1)
    expect(getSkillsByTrust(registry, "local")).toHaveLength(0)
  })

  test("getSkillTools returns tool list", () => {
    const registry = initializeSkillRegistry(testSkills)
    expect(getSkillTools(registry, "code_review")).toEqual(["read_file", "grep_search"])
    expect(getSkillTools(registry, "nonexistent")).toEqual([])
  })

  test("deactivateSkill marks skill inactive", () => {
    let registry = initializeSkillRegistry(testSkills)
    registry = deactivateSkill(registry, "code_review")
    expect(getSkill(registry, "code_review")!.active).toBe(false)
  })

  test("getActiveSkills returns only active skills", () => {
    let registry = initializeSkillRegistry(testSkills)
    expect(getActiveSkills(registry)).toHaveLength(3)
    registry = deactivateSkill(registry, "experimental")
    expect(getActiveSkills(registry)).toHaveLength(2)
  })
})
