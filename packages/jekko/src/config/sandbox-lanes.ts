// jankurai:allow HLT-001-DEAD-MARKER reason=functional-optional-returns-by-design expires=2027-01-01
//
// TypeScript mirror of crates/sandboxctl/src/spec.rs. The Rust crate is the
// runtime source of truth; this module exists so PR-time `bun turbo test:ci`
// can parse + validate `agent/sandbox-lanes.toml` without requiring a Rust
// toolchain on every checker. Drift between the two surfaces is caught by
// `tests/agent/sandbox-lanes-schema.test.ts` (which loads the same fixtures
// as `crates/sandboxctl/tests/spec_schema.rs`).
export * as SandboxLanes from "./sandbox-lanes"

import { Schema } from "effect"

const LaneKind = Schema.Literals(["sandbox", "validation", "audit", "security", "release"])
const WorkspaceKind = Schema.Literals(["worktree", "clone"])
const Backend = Schema.Literals(["worktree", "bubblewrap", "docker", "podman"])
const NetworkPolicy = Schema.Literals(["none", "bridge", "host"])

const WorkspaceCfg = Schema.Struct({
  kind: WorkspaceKind,
  base_branch: Schema.optional(Schema.String),
  branch_template: Schema.optional(Schema.String),
})

const RuntimeCfg = Schema.Struct({
  backend: Backend,
  network: Schema.optional(NetworkPolicy),
  memory_limit: Schema.optional(Schema.String),
  cpu_limit: Schema.optional(Schema.String),
  timeout_seconds: Schema.Int.pipe(Schema.greaterThan(0)),
  image: Schema.optional(Schema.String),
})

const CommandsCfg = Schema.Struct({
  allowed_patterns: Schema.Array(Schema.String).pipe(Schema.minItems(1)),
  denied_patterns: Schema.optional(Schema.Array(Schema.String)),
  wrapper: Schema.String,
  allowed_env: Schema.optional(Schema.Array(Schema.String)),
})

const EnvCfg = Schema.Struct({
  home: Schema.String,
  tmpdir: Schema.String,
  cache_home: Schema.String,
})

const FeedbackCfg = Schema.Struct({
  capture_stdout: Schema.optional(Schema.Boolean),
  capture_stderr: Schema.optional(Schema.Boolean),
  capture_exit_code: Schema.optional(Schema.Boolean),
  tail_lines: Schema.optional(Schema.Int),
})

const ExportCfg = Schema.Struct({
  patch_path: Schema.String,
  artifacts: Schema.optional(Schema.Array(Schema.String)),
})

const CleanupCfg = Schema.Struct({
  auto_remove: Schema.optional(Schema.Boolean),
  preserve_logs: Schema.optional(Schema.Boolean),
  preserve_on_failure: Schema.optional(Schema.Boolean),
})

const SuccessCfg = Schema.Struct({
  exit_code_expected: Schema.optional(Schema.Int),
  changed_files_max: Schema.optional(Schema.Int),
  required_patch_present: Schema.optional(Schema.Boolean),
})

export const Lane = Schema.Struct({
  name: Schema.String,
  command_id: Schema.String,
  kind: LaneKind,
  purpose: Schema.String,
  command: Schema.String,
  cost: Schema.Int.pipe(Schema.greaterThanOrEqualTo(0)),
  destructive: Schema.optional(Schema.Boolean),
  timeout_seconds: Schema.Int.pipe(Schema.greaterThan(0)),
  requires_network: Schema.optional(Schema.Boolean),
  rules_covered: Schema.optional(Schema.Array(Schema.String)),
  required_artifacts: Schema.optional(Schema.Array(Schema.String)),
  workspace: WorkspaceCfg,
  runtime: RuntimeCfg,
  commands: CommandsCfg,
  environment: EnvCfg,
  feedback: Schema.optional(FeedbackCfg),
  export: ExportCfg,
  cleanup: Schema.optional(CleanupCfg),
  success: Schema.optional(SuccessCfg),
})

export const Doc = Schema.Struct({
  schema_version: Schema.String,
  sandbox_root: Schema.optional(Schema.String),
  lane: Schema.Array(Lane),
})

export type Doc = typeof Doc.Type
export type Lane = typeof Lane.Type

/**
 * Cross-cutting validation rules that aren't expressible in Effect.Schema:
 * - command_id and name must be unique
 * - bare `*` is forbidden in allow/deny patterns (defeats the whitelist)
 * - docker/podman backends require `runtime.image`
 * - environment paths must include `{run_id}` interpolation or be absolute
 * - export.patch_path must include `{run_id}` interpolation
 */
export function semanticValidate(doc: Doc): string[] {
  const errors: string[] = []
  const seenNames = new Set<string>()
  const seenIds = new Set<string>()
  for (const lane of doc.lane) {
    if (seenNames.has(lane.name)) errors.push(`duplicate lane name: ${lane.name}`)
    seenNames.add(lane.name)
    if (seenIds.has(lane.command_id)) errors.push(`duplicate command_id: ${lane.command_id}`)
    seenIds.add(lane.command_id)
    for (const pat of [...lane.commands.allowed_patterns, ...(lane.commands.denied_patterns ?? [])]) {
      if (pat.trim() === "") errors.push(`lane ${lane.name}: empty pattern not allowed`)
      if (pat === "*") errors.push(`lane ${lane.name}: bare '*' pattern defeats the allowlist`)
    }
    if (lane.runtime.backend === "docker" || lane.runtime.backend === "podman") {
      if (!lane.runtime.image) errors.push(`lane ${lane.name}: runtime.image is required for docker/podman`)
    }
    for (const [field, value] of [
      ["environment.home", lane.environment.home],
      ["environment.tmpdir", lane.environment.tmpdir],
      ["environment.cache_home", lane.environment.cache_home],
    ] as const) {
      if (!value.includes("{run_id}") && !value.startsWith("/") && !value.startsWith("~/"))
        errors.push(`lane ${lane.name}: ${field} '${value}' must be absolute or include {run_id}`)
    }
    if (!lane.export.patch_path.includes("{run_id}"))
      errors.push(`lane ${lane.name}: export.patch_path must include the {run_id} interpolation`)
  }
  return errors
}
