import type { ZyalConstraint } from "@/agent-script/schema"

/**
 * Runtime invariant checker.
 * Manages baselines and evaluates constraints against current state.
 */

export type ConstraintBaselines = Record<string, number>

export type ConstraintVerdict =
  | { pass: true }
  | { pass: false; name: string; action: string; reason: string }

/**
 * Capture baselines for all constraints that require them.
 * The shellRunner returns the numeric output of the check command.
 */
export function captureBaselines(
  constraints: readonly ZyalConstraint[],
  shellRunner: (command: string) => number,
  trigger: "capture_on_start" | "capture_on_checkpoint",
): ConstraintBaselines {
  const baselines: ConstraintBaselines = {}
  for (const constraint of constraints) {
    if (constraint.baseline === trigger) {
      baselines[constraint.name] = shellRunner(constraint.check.shell)
    }
  }
  return baselines
}

/**
 * Evaluate a single constraint against its baseline or absolute invariant.
 */
export function evaluateConstraint(
  constraint: ZyalConstraint,
  currentValue: number,
  baselines: ConstraintBaselines,
): ConstraintVerdict {
  const { invariant, name } = constraint
  const baseline = baselines[name]

  switch (invariant) {
    case "equals_zero":
      if (currentValue !== 0) {
        return {
          pass: false,
          name,
          action: constraint.on_violation ?? "warn",
          reason: `Expected 0, got ${currentValue}`,
        }
      }
      break

    case "non_zero":
      if (currentValue === 0) {
        return {
          pass: false,
          name,
          action: constraint.on_violation ?? "warn",
          reason: `Expected non-zero, got 0`,
        }
      }
      break

    case "gte_baseline":
      if (baseline !== undefined && currentValue < baseline) {
        return {
          pass: false,
          name,
          action: constraint.on_violation ?? "warn",
          reason: `Expected >= ${baseline} (baseline), got ${currentValue}`,
        }
      }
      break

    case "lte_baseline":
      if (baseline !== undefined && currentValue > baseline) {
        return {
          pass: false,
          name,
          action: constraint.on_violation ?? "warn",
          reason: `Expected <= ${baseline} (baseline), got ${currentValue}`,
        }
      }
      break

    case "equals_baseline":
      if (baseline !== undefined && currentValue !== baseline) {
        return {
          pass: false,
          name,
          action: constraint.on_violation ?? "warn",
          reason: `Expected ${baseline} (baseline), got ${currentValue}`,
        }
      }
      break
  }

  return { pass: true }
}

/**
 * Evaluate all constraints, returning the first violation or pass.
 */
export function evaluateAllConstraints(
  constraints: readonly ZyalConstraint[],
  shellRunner: (command: string) => number,
  baselines: ConstraintBaselines,
): ConstraintVerdict {
  for (const constraint of constraints) {
    const currentValue = shellRunner(constraint.check.shell)
    const verdict = evaluateConstraint(constraint, currentValue, baselines)
    if (!verdict.pass) return verdict
  }
  return { pass: true }
}
