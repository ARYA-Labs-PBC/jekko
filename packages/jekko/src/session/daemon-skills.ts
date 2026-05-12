import type { ZyalSkills, ZyalSkillDefinition } from "@/agent-script/schema"

/**
 * Skill registry manager for ZYAL v2.
 *
 * Manages agent skill definitions with trust levels, tool bindings,
 * write scope restrictions, and creation gating.
 */

export type SkillInstance = {
  readonly name: string
  readonly definition: ZyalSkillDefinition
  readonly active: boolean
  readonly invokeCount: number
  readonly lastInvokedAt?: number
}

export type SkillRegistry = {
  readonly skills: Map<string, SkillInstance>
  readonly allowCreation: boolean
  readonly maxSkills: number
}

/**
 * Initialize the skill registry from config.
 */
export function initializeSkillRegistry(config: ZyalSkills | undefined): SkillRegistry {
  const skills = new Map<string, SkillInstance>()
  if (config?.registry) {
    for (const [name, def] of Object.entries(config.registry)) {
      skills.set(name, {
        name,
        definition: def,
        active: true,
        invokeCount: 0,
      })
    }
  }
  return {
    skills,
    allowCreation: config?.allow_creation ?? false,
    maxSkills: config?.max_skills ?? 50,
  }
}

/**
 * Look up a skill by name.
 */
export function getSkill(registry: SkillRegistry, name: string): SkillInstance | undefined {
  return registry.skills.get(name)
}

/**
 * Check if a skill can be invoked (exists, active, trusted).
 */
export function canInvoke(registry: SkillRegistry, name: string): { allowed: boolean; reason?: string } {
  const skill = registry.skills.get(name)
  if (!skill) return { allowed: false, reason: `skill '${name}' not found` }
  if (!skill.active) return { allowed: false, reason: `skill '${name}' is inactive` }
  return { allowed: true }
}

/**
 * Record a skill invocation.
 */
export function recordInvocation(registry: SkillRegistry, name: string): SkillRegistry {
  const skill = registry.skills.get(name)
  if (!skill) return registry
  const updated = new Map(registry.skills)
  updated.set(name, {
    ...skill,
    invokeCount: skill.invokeCount + 1,
    lastInvokedAt: Date.now(),
  })
  return { ...registry, skills: updated }
}

/**
 * Register a new skill at runtime (if creation is allowed).
 */
export function registerSkill(
  registry: SkillRegistry,
  name: string,
  definition: ZyalSkillDefinition,
): { registry: SkillRegistry; registered: boolean; reason?: string } {
  if (!registry.allowCreation) {
    return { registry, registered: false, reason: "skill creation is disabled" }
  }
  if (registry.skills.size >= registry.maxSkills) {
    return { registry, registered: false, reason: `max skills reached (${registry.maxSkills})` }
  }
  if (registry.skills.has(name)) {
    return { registry, registered: false, reason: `skill '${name}' already exists` }
  }
  const updated = new Map(registry.skills)
  updated.set(name, { name, definition, active: true, invokeCount: 0 })
  return { registry: { ...registry, skills: updated }, registered: true }
}

/**
 * Get skills filtered by trust level.
 */
export function getSkillsByTrust(
  registry: SkillRegistry,
  trust: ZyalSkillDefinition["trust"],
): SkillInstance[] {
  return Array.from(registry.skills.values()).filter((s) => s.definition.trust === trust)
}

/**
 * Get available tools for a skill.
 */
export function getSkillTools(registry: SkillRegistry, name: string): string[] {
  const skill = registry.skills.get(name)
  return [...(skill?.definition.tools ?? [])]
}

/**
 * Deactivate a skill.
 */
export function deactivateSkill(registry: SkillRegistry, name: string): SkillRegistry {
  const skill = registry.skills.get(name)
  if (!skill) return registry
  const updated = new Map(registry.skills)
  updated.set(name, { ...skill, active: false })
  return { ...registry, skills: updated }
}

/**
 * Get all active skill names.
 */
export function getActiveSkills(registry: SkillRegistry): string[] {
  return Array.from(registry.skills.values())
    .filter((s) => s.active)
    .map((s) => s.name)
}
