// jankurai:allow HLT-000-SCORE-DIMENSION reason=large-structured-file-with-parallel-patterns-by-design expires=2027-01-01
import { describe, expect, test } from "bun:test"
import { Effect } from "effect"
import fs from "fs"
import path from "path"
import { detectZyal, scanZyalEnvelope } from "./activation"
import { getZyalExample, listZyalExamples } from "./examples"
import { extractZyalBlock, parseZyal } from "./parser"
import { ZYAL_CONTRACT_VERSION, ZYAL_RESEARCH_BLOCK_VERSION, ZYAL_RUNTIME_SENTINEL_VERSION } from "./version"

describe("ZYAL parser", () => {
  test("fast envelope scan recognises complete pasted ZYAL without schema parsing", () => {
    const text = `[# copied terminal buffer
  <<<ZYAL v1:daemon id=fast-loop>>>
version: v1
<<<END_ZYAL id=fast-loop>>>
ZYAL_ARM RUN_FOREVER id=fast-loop
]`
    expect(scanZyalEnvelope(text)).toEqual({
      kind: "zyal",
      id: "fast-loop",
      hasClose: true,
      hasArm: true,
      complete: true,
    })
  })

  test("fast envelope scan ignores regex metacharacters outside the sentinel", () => {
    const text = `prefix (safe) [noise] <<<ZYAL v1:daemon id=regex-safe>>>
version: v1
<<<END_ZYAL id=regex-safe>>>
ZYAL_ARM RUN_FOREVER id=regex-safe`
    expect(scanZyalEnvelope(text)).toEqual({
      kind: "zyal",
      id: "regex-safe",
      hasClose: true,
      hasArm: true,
      complete: true,
    })
  })

  test("rejects vv1 sentinels", async () => {
    const text = `<<<ZYAL vv1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("No valid ZYAL block found")
  })

  test("accepts a valid example", async () => {
    const example = getZyalExample("jankurai-clean-worktree")
    expect(example).toBeDefined()
    const parsed = await Effect.runPromise(parseZyal(example!.text))
    expect(parsed.spec.intent).toBe("daemon")
    expect(parsed.spec.confirm).toBe("RUN_FOREVER")
    expect(parsed.preview.contract_version).toBe(ZYAL_CONTRACT_VERSION)
    expect(parsed.preview.runtime_sentinel_version).toBe(ZYAL_RUNTIME_SENTINEL_VERSION)
    expect(parsed.preview.research_block_version).toBe(ZYAL_RESEARCH_BLOCK_VERSION)
    expect(parsed.preview.armed).toBe(true)
  })

  test("accepts the memory benchmark autoresearch chase example", async () => {
    const example = getZyalExample("memory-benchmark-autoresearch-chase")
    expect(example).toBeDefined()
    const parsed = await Effect.runPromise(parseZyal(example!.text))
    expect(parsed.spec.experiments?.lanes.length).toBe(20)
    expect(parsed.spec.experiments?.lanes[0]?.id).toBe("arena_lane_00")
    expect(parsed.spec.experiments?.lanes[19]?.id).toBe("arena_lane_19")
    expect(parsed.spec.experiments?.max_parallel).toBe(20)
    expect(parsed.spec.experiments?.scoring?.primary).toBe("executable_rust_oracles")
    expect(parsed.preview.fleet_max_workers).toBe(20)
    expect(parsed.preview.experiments_summary).toContain("best_verified_patch")
    expect(parsed.preview.experiments_summary).toContain("primary:executable_rust_oracles")
  })

  test("accepts the memory benchmark autoresearch basic example", async () => {
    const example = getZyalExample("memory-benchmark-autoresearch-basic")
    expect(example).toBeDefined()
    const parsed = await Effect.runPromise(parseZyal(example!.text))
    expect(parsed.spec.experiments?.lanes.length).toBe(4)
    expect(parsed.spec.experiments?.scoring?.primary).toBe("executable_rust_oracles")
    expect(parsed.preview.fleet_max_workers).toBe(4)
    expect(parsed.preview.experiments_summary).toContain("best_verified_patch")
    expect(parsed.preview.experiments_summary).toContain("primary:executable_rust_oracles")
  })

  test("parses the bundled memory benchmark docs examples recursively", async () => {
    const examplesDir = path.resolve(import.meta.dir, "../../../../docs/ZYAL/examples/memory-benchmark")
    const files = collectZyalFiles(examplesDir).sort()
    expect(files).toEqual([
      "autoresearch-basic.zyal",
      "autoresearch-chase.zyal",
      "executable-benchmark.zyal",
      "generated-challenge.zyal",
      "prompt-scoring.zyal",
    ])
    for (const file of files) {
      const parsed = await Effect.runPromise(parseZyal(fs.readFileSync(path.join(examplesDir, file), "utf8")))
      expect(parsed.spec.intent).toBe("daemon")
      expect(parsed.preview.id).toBe(parsed.spec.id)
    }
  })

  test("accepts experiments scoring primary and rejects unknown scoring keys", async () => {
    const text = makeZyal(`
experiments:
  lanes:
    - id: lane-a
      hypothesis: basic scoring lane
  scoring:
    command: "run-scoring"
    primary: executable_rust_oracles
`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.experiments?.scoring?.primary).toBe("executable_rust_oracles")
    expect(parsed.preview.experiments_summary).toContain("primary:executable_rust_oracles")

    const bad = makeZyal(`
experiments:
  lanes:
    - id: lane-a
      hypothesis: basic scoring lane
  scoring:
    command: "run-scoring"
    primary: executable_rust_oracles
    surprise: true
`)
    await expect(Effect.runPromise(parseZyal(bad))).rejects.toThrow(
      "Unknown ZYAL key: experiments.scoring.surprise",
    )
  })

  test("detects draft ZYAL blocks without arm", () => {
    const example = getZyalExample("fix-until-tests-pass")!
    const draft = example.text.replace(/ZYAL_ARM RUN_FOREVER id=.*\n?$/, "")
    const detected = detectZyal(draft)
    expect(detected.kind).toBe("preview")
    if (detected.kind === "preview") expect(detected.preview.armed).toBe(false)
  })

  test("rejects missing open block", async () => {
    await expect(Effect.runPromise(parseZyal("hello"))).rejects.toThrow("No valid ZYAL block found")
  })

  test("accepts leading blank and comment-only preambles", async () => {
    const text = `# generated example
# safe comment

<<<ZYAL v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.id).toBe("test")
    expect(detectZyal(text).kind).toBe("preview")
  })

  test("accepts indented sentinels from terminal paste", async () => {
    const text = `# copied from a terminal buffer
  <<<ZYAL v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
  <<<END_ZYAL id=test>>>
  ZYAL_ARM RUN_FOREVER id=test`
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.id).toBe("test")
    expect(detectZyal(text).kind).toBe("preview")
  })

  test("strips terminal escape noise before parsing", async () => {
    const text = makeZyal("").replace("<<<ZYAL", "\x1B[118;1:3u<<<ZYAL")
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.id).toBe("test")
  })

  test("detects wrapped malformed terminal ZYAL paste as invalid, not none", () => {
    const text = `[# ZYAL v1 - terminal transcript
  <<<ZYAL v1:daemon id=jankurai-min-loop>>>
  version: v1
  intent: daemon
  confirm: RUN_FOREVER
  job:
    name: "Jankurai min loop"
    objective: |
      first line
  broken continuation from terminal wrap
  \x1B[118;1:3u# selection marker noise
  <<<END_ZYAL id=jankurai-min-loop>>>
  ZYAL_ARM RUN_FOREVER id=jankurai-min-loop
]`
    const detected = detectZyal(text)
    expect(detected.kind).toBe("invalid")
  })

  test("rejects arbitrary prose before the ZYAL sentinel", async () => {
    const text = `This is not a comment.
<<<ZYAL v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("No valid ZYAL block found")
  })

  test("activation reports prose-prefixed sentinel as invalid (not none)", () => {
    // Parser is intentionally strict, but detection must still flip so the
    // gold flash, ✗ ZYAL footer, and sidebar panel surface to give the user
    // a diagnostic instead of silently swallowing the paste.
    const text = `here you go:
<<<ZYAL v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
    expect(detectZyal(text).kind).toBe("invalid")
  })

  test("rejects unknown top-level keys", async () => {
    const text = `<<<ZYAL v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
bogus: true
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("Unknown ZYAL top-level key: bogus")
  })

  test("rejects unknown nested keys outside incubator", async () => {
    const text = `<<<ZYAL v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
extra: nope
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("Unknown ZYAL top-level key: extra")
  })

  test("rejects archived sentinels and arm markers", async () => {
    const sentinel = ["O", "CAL"].join("")
    const text = `<<<${sentinel} v1:daemon id=test>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
<<<END_${sentinel} id=test>>>
${sentinel}_ARM RUN_FOREVER id=test`
    expect(extractZyalBlock(text)).toBeNull()
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("No valid ZYAL block found")
  })

  test("rejects code fences", async () => {
    const text = "```yaml\n<<<ZYAL v1:daemon id=test>>>\nversion: v1\nintent: daemon\nconfirm: RUN_FOREVER\njob:\n  name: test\n  objective: test\nstop:\n  all:\n    - git_clean: {}\n<<<END_ZYAL id=test>>>\nZYAL_ARM RUN_FOREVER id=test\n```"
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects mismatched block ids", async () => {
    const text = `<<<ZYAL v1:daemon id=one>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - git_clean: {}
<<<END_ZYAL id=two>>>
ZYAL_ARM RUN_FOREVER id=one`
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects duplicate ZYAL blocks and trailing content", async () => {
    const first = makeZyal("")
    const duplicate = `${first}\n${first}`
    await expect(Effect.runPromise(parseZyal(duplicate))).rejects.toThrow()

    const trailing = `${first}\n# unsafe trailing text`
    await expect(Effect.runPromise(parseZyal(trailing))).rejects.toThrow()
  })

  test("lists bundled examples", () => {
    expect(listZyalExamples().length).toBeGreaterThan(0)
    expect(extractZyalBlock(getZyalExample("multi-worker-audit")!.text)).toBeTruthy()
  })

  test("parses full docs ZYAL examples", async () => {
    const examplesDir = path.resolve(import.meta.dir, "../../../../docs/ZYAL/examples")
    const files = collectZyalFiles(examplesDir).sort()
    expect(files).toEqual([
      "01-fix-until-green.zyal",
      "02-hypothesis-tournament.zyal",
      "03-billion-loc-monorepo.zyal",
      "04-fleet-portfolio.zyal",
      "05-secure-mcp-lockdown.zyal",
      "06-evidence-graph-merge.zyal",
      "07-self-improving-skills.zyal",
      "08-full-power-runbook.zyal",
      "09-control-plane-preview.zyal",
      "10-jankurai-master-loop.zyal",
      "11-jankurai-fleet-loop.zyal",
      "12-jankurai-min-loop.zyal",
      "13-advanced-research-loop.zyal",
      "memory-benchmark/autoresearch-basic.zyal",
      "memory-benchmark/autoresearch-chase.zyal",
      "memory-benchmark/executable-benchmark.zyal",
      "memory-benchmark/generated-challenge.zyal",
      "memory-benchmark/prompt-scoring.zyal",
    ])
    for (const file of files) {
      const text = fs.readFileSync(path.join(examplesDir, file), "utf8")
      const parsed = await Effect.runPromise(parseZyal(text))
      expect(parsed.spec.intent).toBe("daemon")
      expect(parsed.preview.id).toBe(parsed.spec.id)
    }
  })

  test("master loop runbook hardens with v2.3 taint defence", async () => {
    const masterPath = path.resolve(
      import.meta.dir,
      "../../../../docs/ZYAL/examples/10-jankurai-master-loop.zyal",
    )
    const text = fs.readFileSync(masterPath, "utf8")
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.id).toBe("jankurai-porting-advanced")
    expect(parsed.preview.taint_enabled).toBe(true)
    expect(parsed.preview.taint_label_count).toBeGreaterThanOrEqual(5)
    expect(parsed.preview.taint_forbid_count).toBeGreaterThanOrEqual(1)
    expect(parsed.preview.taint_summary).toContain("injection:pause")
    expect(parsed.preview.fleet_summary).toContain("max:10")
    expect(parsed.preview.fleet_summary).toContain("jnoccio:on")
    expect(parsed.preview.jankurai_enabled).toBe(true)
    expect(parsed.preview.jankurai_summary).toContain("source:repair_plan")
  })

  test("accepts the control-plane preview example", async () => {
    const parsed = await Effect.runPromise(parseZyal(getZyalExample("control-plane-preview")!.text))
    expect(parsed.preview.interop_enabled).toBe(true)
    expect(parsed.preview.runtime_enabled).toBe(true)
    expect(parsed.preview.capability_negotiation_enabled).toBe(true)
    expect(parsed.preview.memory_kernel_enabled).toBe(true)
    expect(parsed.preview.evidence_graph_enabled).toBe(true)
    expect(parsed.preview.trust_enabled).toBe(true)
    expect(parsed.preview.taint_enabled).toBe(true)
    expect(parsed.preview.taint_label_count).toBe(3)
    expect(parsed.preview.requirements_enabled).toBe(true)
    expect(parsed.preview.evaluation_enabled).toBe(true)
    expect(parsed.preview.release_enabled).toBe(true)
    expect(parsed.preview.roles_count).toBe(2)
    expect(parsed.preview.channels_count).toBe(3)
    expect(parsed.preview.imports_count).toBe(2)
    expect(parsed.preview.reasoning_privacy_enabled).toBe(true)
    expect(parsed.preview.unsupported_feature_policy_enabled).toBe(true)
    expect(parsed.preview.unsupported_feature_policy_summary).toContain("required:14")
  })

  test("accepts the advanced research example", async () => {
    const parsed = await Effect.runPromise(parseZyal(getZyalExample("advanced-research-loop")!.text))
    expect(parsed.preview.research_enabled).toBe(true)
    expect(parsed.preview.research_mode).toBe("mixed")
    expect(parsed.preview.research_max_parallel).toBe(6)
    expect(parsed.preview.research_summary).toContain("autonomy:require_plan")
    expect(parsed.preview.research_provider_summary).toContain("prefer:official_api,primary_source,privacy_first")
    expect(parsed.preview.research_evidence_summary).toContain("citations")
    expect(parsed.preview.research_safety_summary).toContain("block_internal")
  })

  test("rejects unsupported required features in fail-closed preview policy", async () => {
    const text = makeZyal(`
unsupported_feature_policy:
  required: [totally_unknown_feature]
  fail_closed: true
  on_missing: reject`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid incubator block", async () => {
    const parsed = await Effect.runPromise(parseZyal(getZyalExample("hard-task-incubator")!.text))
    expect(parsed.spec.incubator?.enabled).toBe(true)
    expect(parsed.preview.incubator_enabled).toBe(true)
    expect(parsed.preview.incubator_passes.some((item) => item.includes("idea"))).toBe(true)
    expect(parsed.preview.promotion_threshold).toBe(0.78)
  })

  test("accepts the normal user incubator preset", async () => {
    const parsed = await Effect.runPromise(parseZyal(getZyalExample("normal-user-incubator")!.text))
    expect(parsed.spec.incubator?.enabled).toBe(true)
    expect(parsed.preview.cleanup_summary).toContain("archive_artifacts")
    expect(parsed.preview.readiness_summary).toContain("promote_at:0.7")
  })

  test("rejects unknown incubator keys", async () => {
    const text = getZyalExample("hard-task-incubator")!.text.replace("enabled: true", "enabled: true\n  allow_unbounded: true")
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects unbounded incubator budgets", async () => {
    const text = getZyalExample("hard-task-incubator")!.text.replace("max_passes_per_task: 7", "max_passes_per_task: .inf")
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects main worktree writes before promotion", async () => {
    const text = getZyalExample("hard-task-incubator")!.text.replace("writes: scratch_only", "writes: main_worktree")
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects prototype without isolation or scratch", async () => {
    const text = getZyalExample("safe-prototype-promotion")!.text.replace("writes: isolated_worktree", "writes: main_worktree")
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects idea count above configured parallel cap", async () => {
    const text = getZyalExample("hard-task-incubator")!.text.replace("count: 3", "count: 4")
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  // ─── v1.1 capability tests ────────────────────────────────────────────

  test("accepts a valid on handler block", async () => {
    const text = makeZyal(`
on:
  - signal: no_progress
    count_gte: 2
    do:
      - switch_agent: plan
  - signal: error
    do:
      - pause: true`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.on?.length).toBe(2)
    expect(parsed.preview.on_handler_count).toBe(2)
  })

  test("rejects on handler with empty do list", async () => {
    const text = makeZyal(`
on:
  - signal: no_progress
    do: []`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects on handler with invalid count_gte", async () => {
    const text = makeZyal(`
on:
  - signal: error
    count_gte: 0
    do:
      - abort: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid fan_out block", async () => {
    const text = makeZyal(`
fan_out:
  strategy: map_reduce
  split:
    items: ["auth", "database", "api"]
  worker:
    agent: build
    isolation: git_worktree
    max_parallel: 3
  reduce:
    strategy: merge_all
  on_partial_failure: continue`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.fan_out).toBeDefined()
    expect(parsed.preview.fan_out_enabled).toBe(true)
    expect(parsed.preview.fan_out_summary).toContain("merge_all")
  })

  test("rejects fan_out best_score without score_key", async () => {
    const text = makeZyal(`
fan_out:
  split:
    items: ["a"]
  worker:
    agent: build
  reduce:
    strategy: best_score`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects fan_out with non-positive max_parallel", async () => {
    const text = makeZyal(`
fan_out:
  split:
    items: ["a"]
  worker:
    max_parallel: 0
  reduce:
    strategy: merge_all`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid guardrails block", async () => {
    const text = makeZyal(`
guardrails:
  input:
    - name: no-force-push
      deny_patterns: ["git push --force"]
      action: block
  output:
    - name: type-safety
      shell: "npx tsgo --noEmit"
      on_fail: retry
      max_retries: 2`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.guardrails).toBeDefined()
    expect(parsed.preview.guardrail_count).toBe(2)
    expect(parsed.preview.guardrails_summary).toContain("input:1")
  })

  test("rejects guardrails with empty deny_patterns", async () => {
    const text = makeZyal(`
guardrails:
  input:
    - name: bad
      deny_patterns: []
      action: block`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid assertions block", async () => {
    const text = makeZyal(`
assertions:
  require_structured_output: true
  on_invalid: retry
  max_retries: 2`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.assertions?.require_structured_output).toBe(true)
    expect(parsed.preview.assertions_enabled).toBe(true)
  })

  test("accepts a valid retry block", async () => {
    const text = makeZyal(`
retry:
  default:
    max_attempts: 3
    backoff: exponential
    initial_delay: 2s
    jitter: true
  overrides:
    shell_checks:
      max_attempts: 5
      backoff: linear`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.retry).toBeDefined()
    expect(parsed.preview.retry_enabled).toBe(true)
  })

  test("rejects retry with non-positive max_attempts", async () => {
    const text = makeZyal(`
retry:
  default:
    max_attempts: 0`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid hooks block", async () => {
    const text = makeZyal(`
hooks:
  on_start:
    - run: "git fetch origin main"
  before_iteration:
    - run: "git rebase origin/main --autostash"
      on_fail: pause
  after_checkpoint:
    - run: "echo done"
      on_fail: warn`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.hooks).toBeDefined()
    expect(parsed.preview.hook_count).toBe(3)
    expect(parsed.preview.hooks_summary).toContain("on_start:1")
  })

  test("accepts a valid constraints block", async () => {
    const text = makeZyal(`
constraints:
  - name: test-count-stable
    check:
      shell: "bun test --dry-run 2>&1 | grep -c test"
    baseline: capture_on_start
    invariant: gte_baseline
    on_violation: pause
  - name: no-binaries
    check:
      shell: "find src/ -name '*.bin' | wc -l"
    invariant: equals_zero
    on_violation: block`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.constraints?.length).toBe(2)
    expect(parsed.preview.constraint_count).toBe(2)
    expect(parsed.preview.constraints_summary).toContain("test-count-stable:gte_baseline")
  })

  test("rejects constraints with duplicate names", async () => {
    const text = makeZyal(`
constraints:
  - name: dup
    check:
      shell: "echo 1"
    invariant: equals_zero
  - name: dup
    check:
      shell: "echo 2"
    invariant: non_zero`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects constraint with baseline incompatible with equals_zero", async () => {
    const text = makeZyal(`
constraints:
  - name: bad
    check:
      shell: "echo 0"
    baseline: capture_on_start
    invariant: equals_zero`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts full-featured v1.1 example", async () => {
    const parsed = await Effect.runPromise(parseZyal(getZyalExample("full-v1.1-kitchen-sink")!.text))
    expect(parsed.spec.on?.length).toBeGreaterThan(0)
    expect(parsed.spec.guardrails).toBeDefined()
    expect(parsed.spec.hooks).toBeDefined()
    expect(parsed.spec.constraints?.length).toBeGreaterThan(0)
    expect(parsed.preview.on_handler_count).toBeGreaterThan(0)
    expect(parsed.preview.guardrail_count).toBeGreaterThan(0)
    expect(parsed.preview.hook_count).toBeGreaterThan(0)
    expect(parsed.preview.constraint_count).toBeGreaterThan(0)
  })
})

/** Helper: produce a minimal armed ZYAL block with extra YAML appended */
function makeZyal(extra: string) {
  const body = `version: v1
intent: daemon
confirm: RUN_FOREVER
job:
  name: test
  objective: test
stop:
  all:
    - shell:
        command: "true"
        timeout: 1s
${extra.replace(/^\n/, "")}`
  return `<<<ZYAL v1:daemon id=test>>>
${body}
<<<END_ZYAL id=test>>>
ZYAL_ARM RUN_FOREVER id=test`
}

function collectZyalFiles(dir: string, root = dir): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) return collectZyalFiles(fullPath, root)
    if (!entry.isFile() || !entry.name.endsWith(".zyal")) return []
    return [path.relative(root, fullPath).split(path.sep).join("/")]
  })
}

// ─── v2 parser tests ────────────────────────────────────────────────────────

describe("ZYAL parser v2", () => {
  test("accepts read permission keys", async () => {
    const text = makeZyal(`
permissions:
  read: allow
  list: allow
  glob: allow
  grep: allow
  research: allow
  websearch: deny
  webfetch: ask
  external_directory: ask`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.permissions?.read).toBe("allow")
    expect(parsed.spec.permissions?.glob).toBe("allow")
    expect(parsed.spec.permissions?.external_directory).toBe("ask")
    expect(parsed.spec.permissions?.research).toBe("allow")
    expect(parsed.spec.permissions?.websearch).toBe("deny")
    expect(parsed.spec.permissions?.webfetch).toBe("ask")
    expect(parsed.preview.permissions).toContain("research:allow")
    expect(parsed.preview.permissions).toContain("websearch:deny")
    expect(parsed.preview.permissions).toContain("webfetch:ask")
  })

  test("rejects OpenQG-style ui.mode keys", async () => {
    const text = makeZyal(`
ui:
  mode: gold`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("Unknown ZYAL key: ui.mode")
  })

  test("accepts unattended interaction metadata", async () => {
    const text = makeZyal(`
interaction:
  user: none
  on_blocked: skip_and_next
  system_inject: "never ask a human"`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.interaction?.user).toBe("none")
    expect(parsed.spec.interaction?.on_blocked).toBe("skip_and_next")
  })

  test("accepts a valid workflow block", async () => {
    const text = makeZyal(`
workflow:
  type: state_machine
  initial: discover
  states:
    discover:
      agent: plan
      writes: scratch_only
      produces:
        - impact_map
      transitions:
        - to: done
          when:
            evidence_exists: impact_map
    done:
      terminal: true`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.workflow_enabled).toBe(true)
    expect(parsed.preview.workflow_summary).toContain("state_machine")
    expect(parsed.preview.workflow_summary).toContain("states:2")
  })

  test("rejects workflow with invalid initial state", async () => {
    const text = makeZyal(`
workflow:
  type: state_machine
  initial: nonexistent
  states:
    a:
      terminal: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects workflow with invalid transition target", async () => {
    const text = makeZyal(`
workflow:
  type: state_machine
  initial: a
  states:
    a:
      transitions:
        - to: nonexistent
          when:
            all_checks_pass: true
    b:
      terminal: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects workflow with terminal state having transitions", async () => {
    const text = makeZyal(`
workflow:
  type: state_machine
  initial: a
  states:
    a:
      terminal: true
      transitions:
        - to: a
          when:
            all_checks_pass: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects workflow with no terminal state", async () => {
    const text = makeZyal(`
workflow:
  type: state_machine
  initial: a
  states:
    a:
      transitions:
        - to: a
          when:
            all_checks_pass: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects workflow with unknown nested keys", async () => {
    const text = makeZyal(`
workflow:
  type: state_machine
  initial: a
  bogus: true
  states:
    a:
      terminal: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid memory block", async () => {
    const text = makeZyal(`
memory:
  stores:
    task_context:
      scope: task
      retention: until_promotion
      max_entries: 100
      write_policy: append_only
      read_policy: inject_at_start
    lessons:
      scope: global
      retention: permanent
  redaction:
    patterns:
      - "example-secret-*"
    action: mask
  provenance:
    track_source: true
    hash_chain: true`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.memory_store_count).toBe(2)
    expect(parsed.preview.memory_summary).toContain("task_context:task")
  })

  test("rejects memory with unknown store keys", async () => {
    const text = makeZyal(`
memory:
  stores:
    ctx:
      scope: task
      retention: permanent
      bogus: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid evidence block", async () => {
    const text = makeZyal(`
evidence:
  require_before_promote:
    - type: test_results
      must_pass: true
    - type: affected_files
      must_be_known: true
    - type: risk_delta
      max_increase: 0.1
  bundle_format: json
  sign: sha256
  archive: true`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.evidence_enabled).toBe(true)
    expect(parsed.preview.evidence_summary).toContain("test_results")
  })

  test("rejects evidence with duplicate types", async () => {
    const text = makeZyal(`
evidence:
  require_before_promote:
    - type: test_results
      must_pass: true
    - type: test_results
      must_exist: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects evidence with empty type", async () => {
    const text = makeZyal(`
evidence:
  require_before_promote:
    - type: ""
      must_pass: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid approvals block", async () => {
    const text = makeZyal(`
approvals:
  gates:
    plan_review:
      required_role: tech_lead
      timeout: 24h
      on_timeout: pause
      decisions:
        - approve
        - reject
      auto_approve_if:
        risk_score_lt: 0.3
        all_checks_pass: true
    merge_review:
      required_role: code_owner
  escalation:
    chain:
      - tech_lead
      - director
    auto_escalate_after: 48h`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.approval_gate_count).toBe(2)
    expect(parsed.preview.approvals_summary).toContain("plan_review:tech_lead")
  })

  test("rejects approvals with unknown gate keys", async () => {
    const text = makeZyal(`
approvals:
  gates:
    review:
      required_role: admin
      bogus: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("validates approval gate references in workflow", async () => {
    const text = makeZyal(`
workflow:
  type: state_machine
  initial: a
  states:
    a:
      approval: missing_gate
      transitions:
        - to: b
          when:
            approval_granted: missing_gate
    b:
      terminal: true
approvals:
  gates:
    other_gate:
      required_role: admin`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts combined v2 blocks", async () => {
    const text = makeZyal(`
workflow:
  type: pipeline
  initial: plan
  states:
    plan:
      agent: plan
      produces:
        - plan_doc
      transitions:
        - to: done
          when:
            evidence_exists: plan_doc
    done:
      terminal: true
memory:
  stores:
    ctx:
      scope: run
      retention: session
evidence:
  require_before_promote:
    - type: test_results
      must_pass: true
approvals:
  gates:
    final_review:
      required_role: admin`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.workflow_enabled).toBe(true)
    expect(parsed.preview.memory_store_count).toBe(1)
    expect(parsed.preview.evidence_enabled).toBe(true)
    expect(parsed.preview.approval_gate_count).toBe(1)
  })
})

// ─── v2 wave 2 parser tests ─────────────────────────────────────────────────

describe("ZYAL parser v2 wave 2", () => {
  test("accepts a valid skills block", async () => {
    const text = makeZyal(`
skills:
  registry:
    code_review:
      description: Review code
      agent: plan
      tools:
        - read_file
        - grep_search
      trust: builtin
      writes: none
    test_writer:
      description: Write tests
      trust: verified
      writes: isolated_worktree
  allow_creation: true
  max_skills: 10`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.skills_count).toBe(2)
    expect(parsed.preview.skills_summary).toContain("code_review:builtin")
  })

  test("rejects skills with unknown nested keys", async () => {
    const text = makeZyal(`
skills:
  registry:
    my_skill:
      description: test
      bogus: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects skills with non-positive max_skills", async () => {
    const text = makeZyal(`
skills:
  max_skills: 0`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid sandbox block", async () => {
    const text = makeZyal(`
sandbox:
  paths:
    - path: src/
      access: write
    - path: /etc
      access: deny
  network:
    outbound: allowlist
    allowlist:
      - api.openai.com
      - "*.github.com"
  resources:
    max_file_size: 10MB
    max_total_disk: 1GB
    max_processes: 4
  env_inherit:
    - HOME
    - PATH
  env_deny:
    - AWS_SECRET_KEY`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.sandbox_enabled).toBe(true)
    expect(parsed.preview.sandbox_summary).toContain("paths:2")
    expect(parsed.preview.sandbox_summary).toContain("net:allowlist")
  })

  test("rejects sandbox with unknown nested keys", async () => {
    const text = makeZyal(`
sandbox:
  paths:
    - path: src/
      access: write
      bogus: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects sandbox with allowlist but wrong outbound", async () => {
    const text = makeZyal(`
sandbox:
  network:
    outbound: deny
    allowlist:
      - example.com`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects sandbox with empty path", async () => {
    const text = makeZyal(`
sandbox:
  paths:
    - path: ""
      access: write`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid security block", async () => {
    const evalPattern = ["e", "val("].join("")
    const systemPattern = ["sys", "tem("].join("")
    const text = makeZyal(`
security:
  trust_zones:
    critical:
      paths:
        - src/auth
        - src/payments
      require_approval: true
      max_risk_score: 0.3
  injection:
    scan_inputs: true
    scan_outputs: true
    deny_patterns:
      - "${evalPattern}"
      - "${systemPattern}"
    on_detect: abort
  secrets:
    allowed_env:
      - API_KEY
    redact_from_logs: true
    rotate_after: 30d`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.security_enabled).toBe(true)
    expect(parsed.preview.security_summary).toContain("zones:1")
    expect(parsed.preview.security_summary).toContain("scan:input")
  })

  test("rejects security with unknown nested keys", async () => {
    const text = makeZyal(`
security:
  trust_zones:
    zone1:
      paths:
        - src/
      bogus: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects security with empty deny pattern", async () => {
    const text = makeZyal(`
security:
  injection:
    scan_inputs: true
    deny_patterns:
      - ""`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts a valid observability block", async () => {
    const text = makeZyal(`
observability:
  spans:
    emit: all
    include_tool_calls: true
    include_model_calls: true
  metrics:
    - name: tool_calls
      type: counter
      source: runtime
    - name: risk_score
      type: gauge
      source: analysis
  cost:
    budget: 10.0
    currency: USD
    alert_at_percent: 80
    on_budget_exceeded: pause
  report:
    format: json
    on_complete: true
    include:
      - spans
      - costs`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.observability_enabled).toBe(true)
    expect(parsed.preview.observability_summary).toContain("spans:all")
    expect(parsed.preview.observability_summary).toContain("metrics:2")
    expect(parsed.preview.observability_summary).toContain("budget:$10")
  })

  test("rejects observability with duplicate metric names", async () => {
    const text = makeZyal(`
observability:
  metrics:
    - name: x
      type: counter
      source: a
    - name: x
      type: gauge
      source: b`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects observability with non-positive budget", async () => {
    const text = makeZyal(`
observability:
  cost:
    budget: 0`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects observability with invalid alert_at_percent", async () => {
    const text = makeZyal(`
observability:
  cost:
    budget: 10
    alert_at_percent: 150`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts combined wave 2 blocks", async () => {
    const text = makeZyal(`
skills:
  registry:
    reviewer:
      trust: builtin
sandbox:
  paths:
    - path: src/
      access: write
security:
  injection:
    scan_inputs: true
observability:
  cost:
    budget: 50`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.skills_count).toBe(1)
    expect(parsed.preview.sandbox_enabled).toBe(true)
    expect(parsed.preview.security_enabled).toBe(true)
    expect(parsed.preview.observability_enabled).toBe(true)
  })
})

describe("ZYAL parser v2.1 and v2.2", () => {
  test("accepts power blocks and fleet jnoccio", async () => {
    const text = makeZyal(`
arming:
  preview_hash_required: true
  host_nonce_required: true
  accepted_origins: [trusted_user_message, signed_cli_input]
  arm_token_single_use: true
capabilities:
  default: deny
  rules:
    - id: read
      tool: read
      decision: allow
    - id: shell-tests
      tool: shell
      command_regex: "^bun test"
      decision: allow
  command_floor:
    always_block: ["git push --force"]
quality:
  anti_vibe:
    enabled: true
    fail_closed: true
  diff_budget:
    max_files_changed: 10
    max_added_lines: 500
    on_violation: require_approval
  checks:
    - name: no-only
      pattern: "test\\\\.only"
      scope: file_diff
      on_violation: block_promotion
experiments:
  strategy: disjoint_tournament
  lanes:
    - id: minimal
      hypothesis: smallest safe patch
      isolation: git_worktree
      budget:
        max_iterations: 2
        max_diff_lines: 200
  max_parallel: 1
  reduce:
    strategy: best_verified_patch
models:
  profiles:
    builder: { provider: anthropic, model: claude-sonnet-4-6 }
    critic: { provider: openai, model: gpt-5 }
  routes:
    implement: builder
    review: critic
  critic:
    must_use_different_provider: true
  confidence_cap: 0.6
budgets:
  run:
    iterations: 10
    cost_usd: 5
    on_exhaust: pause
triggers:
  anti_recursion: true
  list:
    - id: manual
      kind: manual
      max_runs_per_sha: 1
rollback:
  required_when:
    risk_score_gte: 0.6
  plan_required: true
done:
  require: [tests_pass]
  forbid: [test_skip]
repo_intelligence:
  scale: large
  indexes: [rg]
  scope_control:
    require_scope_before_edit: true
    max_initial_scope_files: 20
  blast_radius:
    compute_on: [diff]
    pause_when_score_gte: 0.9
fleet:
  max_workers: 3
  isolation: same_session
  jnoccio:
    enabled: true
    base_url: "http://127.0.0.1:4317"
    metrics_ws: "/v1/jnoccio/metrics/ws"
    max_instances: 2`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.arming_enabled).toBe(true)
    expect(parsed.preview.capabilities_rule_count).toBe(2)
    expect(parsed.preview.quality_enabled).toBe(true)
    expect(parsed.preview.experiments_enabled).toBe(true)
    expect(parsed.preview.models_enabled).toBe(true)
    expect(parsed.preview.budgets_enabled).toBe(true)
    expect(parsed.preview.triggers_count).toBe(1)
    expect(parsed.preview.rollback_enabled).toBe(true)
    expect(parsed.preview.done_enabled).toBe(true)
    expect(parsed.preview.repo_intel_enabled).toBe(true)
    expect(parsed.preview.fleet_summary).toContain("jnoccio:on")
  })

  test("rejects unknown nested keys in power blocks", async () => {
    const text = makeZyal(`
capabilities:
  rules:
    - id: bad
      decision: allow
      untracked: true`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects agents above fleet cap", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 2
agents:
  workers:
    - id: builders
      count: 3
      agent: build`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects fan-out above fleet cap", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 2
fan_out:
  split:
    items: ["a", "b", "c"]
  worker:
    max_parallel: 3
  reduce:
    strategy: merge_all`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects experiments above fleet cap", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
experiments:
  lanes:
    - id: a
      hypothesis: a
    - id: b
      hypothesis: b
  max_parallel: 2`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects incubator concurrency above fleet cap", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 2
incubator:
  enabled: true
  budget:
    max_passes_per_task: 3
    max_rounds_per_task: 1
    max_active_tasks: 2
    max_parallel_idea_passes: 2
  passes:
    - id: ideas
      type: idea
      context: blind
      writes: scratch_only
      count: 2
  promotion:
    promote_at: 0.7`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects fleet jnoccio instance cap violations", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 2
  jnoccio:
    enabled: true
    max_instances: 21`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects fleet.max_workers as a string at schema decode", async () => {
    const text = makeZyal(`
fleet:
  max_workers: "20"`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects fleet.max_workers below 1", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 0`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects fleet.max_workers above 20", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 21`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })
})

describe("ZYAL parser v2.3 taint", () => {
  test("accepts a fully-formed taint block and surfaces preview fields", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 2
taint:
  default_label: tool_output
  labels:
    trusted_user: { rank: high }
    repo_file: { rank: medium }
    tool_output: { rank: untrusted }
    web_content: { rank: hostile }
  forbid:
    - from: [web_content, tool_output]
      cannot: [arm, approve, exec_shell]
      unless: [human_review]
  prompt_injection:
    detect_patterns:
      - "ignore (all )?previous instructions"
      - "you are now (a|an)? \\\\w+"
    on_detect: pause
    scan_sources: [tool_output, web_content]`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.taint_enabled).toBe(true)
    expect(parsed.preview.taint_label_count).toBe(4)
    expect(parsed.preview.taint_forbid_count).toBe(1)
    expect(parsed.preview.taint_summary).toContain("labels:4")
    expect(parsed.preview.taint_summary).toContain("forbid:1")
    expect(parsed.preview.taint_summary).toContain("injection:pause")
  })

  test("rejects taint.labels missing", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  default_label: tool_output`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects empty taint.labels", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  labels: {}`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects taint.default_label not in labels", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  default_label: undeclared
  labels:
    trusted_user: { rank: high }`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects taint.forbid.from referencing undeclared label", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  labels:
    trusted_user: { rank: high }
  forbid:
    - from: [unknown_origin]
      cannot: [arm]`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects taint.forbid with empty cannot list", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  labels:
    web_content: { rank: hostile }
  forbid:
    - from: [web_content]
      cannot: []`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects taint.prompt_injection with empty detect_patterns", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  labels:
    web_content: { rank: hostile }
  prompt_injection:
    detect_patterns: []
    on_detect: pause`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects taint.prompt_injection with invalid regex", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  labels:
    web_content: { rank: hostile }
  prompt_injection:
    detect_patterns: ["[unclosed"]
    on_detect: pause`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects taint.labels[*].rank with unknown rank", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 1
taint:
  labels:
    web_content: { rank: bogus }`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })
})

// ─── 20-thread coordinated literature radar tests ────────────────────────────

describe("ZYAL parser 20thread literature radar", () => {
  test("parses the 20thread ZYAL file from OpenQG", async () => {
    const filePath = path.resolve(import.meta.dir, "../../../../Code/OpenQG/agent/zyal/openqg-literature-radar-20thread.zyal")
    if (!fs.existsSync(filePath)) return // skip if OpenQG repo not present
    const text = fs.readFileSync(filePath, "utf8")
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.id).toBe("openqg-literature-radar-20thread")
    expect(parsed.preview.armed).toBe(true)
  })

  test("fleet and research parallelism are independently controllable", async () => {
    // 4 fleet workers × 5 research threads = 20 total search threads
    const text = makeZyal(`
fleet:
  max_workers: 4
  isolation: same_session

research:
  version: v1
  mode: mixed
  max_parallel: 5
  timeout_seconds: 45

experiments:
  strategy: parallel_distill_refine
  max_parallel: 4
  lanes:
    - id: shard-a
      hypothesis: first domain shard
      isolation: same_session
    - id: shard-b
      hypothesis: second domain shard
      isolation: same_session
    - id: shard-c
      hypothesis: third domain shard
      isolation: same_session
    - id: shard-d
      hypothesis: fourth domain shard
      isolation: same_session
  reduce:
    strategy: synthesize_best`)
    const parsed = await Effect.runPromise(parseZyal(text))

    // Fleet independently controls agent count
    expect(parsed.preview.fleet_enabled).toBe(true)
    expect(parsed.preview.fleet_max_workers).toBe(4)

    // Research independently controls searches per agent
    expect(parsed.preview.research_enabled).toBe(true)
    expect(parsed.preview.research_max_parallel).toBe(5)

    // Total search threads = fleet × research = 20
    expect(parsed.preview.fleet_max_workers * parsed.preview.research_max_parallel).toBe(20)

    // Experiments lanes match fleet workers
    expect(parsed.preview.experiments_enabled).toBe(true)
    expect(parsed.preview.experiments_summary).toContain("lanes:4")
    expect(parsed.preview.experiments_summary).toContain("synthesize_best")
  })

  test("rejects experiments.max_parallel exceeding fleet.max_workers", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 4

experiments:
  strategy: parallel_distill_refine
  max_parallel: 5
  lanes:
    - id: a
      hypothesis: test
    - id: b
      hypothesis: test
    - id: c
      hypothesis: test
    - id: d
      hypothesis: test
    - id: e
      hypothesis: test
  reduce:
    strategy: synthesize_best`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow(
      "experiments.max_parallel (5) exceeds fleet.max_workers (4)",
    )
  })

  test("research.max_parallel caps at 20", async () => {
    const text = makeZyal(`
research:
  version: v1
  max_parallel: 21`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow(
      "research.max_parallel must not exceed 20",
    )
  })

  test("fleet.max_workers caps at 20", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 21`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts maximum 20 research threads with single worker", async () => {
    // Alternative config: 1 worker × 20 research threads
    const text = makeZyal(`
fleet:
  max_workers: 1

research:
  version: v1
  max_parallel: 20`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.fleet_max_workers).toBe(1)
    expect(parsed.preview.research_max_parallel).toBe(20)
  })

  test("accepts maximum fleet with minimal research threads", async () => {
    // Alternative config: 20 workers × 1 research thread
    const text = makeZyal(`
fleet:
  max_workers: 20

research:
  version: v1
  max_parallel: 1`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.fleet_max_workers).toBe(20)
    expect(parsed.preview.research_max_parallel).toBe(1)
  })

  test("domain-sharded coordination prevents lane id duplication", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 4

experiments:
  strategy: parallel_distill_refine
  max_parallel: 4
  lanes:
    - id: qg-core
      hypothesis: first shard
    - id: qg-core
      hypothesis: duplicate shard
    - id: cosmology
      hypothesis: third shard
    - id: math
      hypothesis: fourth shard
  reduce:
    strategy: synthesize_best`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("duplicated")
  })

  test("coordination memory stores are parseable", async () => {
    const text = makeZyal(`
memory:
  stores:
    literature_receipts:
      scope: run
      retention: session
      write_policy: append_only
      read_policy: on_demand
      searchable: true
    search_coordination:
      scope: run
      retention: session
      write_policy: upsert
      read_policy: search
      searchable: true
  provenance:
    track_source: true
    hash_chain: true`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.memory_store_count).toBe(2)
    expect(parsed.preview.memory_summary).toContain("literature_receipts:run")
    expect(parsed.preview.memory_summary).toContain("search_coordination:run")
  })
})

describe("ZYAL parser jankurai block", () => {
  test("accepts first-class jankurai repair configuration and previews it", async () => {
    const text = makeZyal(`
fleet:
  max_workers: 10
jankurai:
  enabled: true
  root: "."
  audit:
    mode: advisory
    json: target/jankurai/repo-score.json
    md: target/jankurai/repo-score.md
    repair_queue_jsonl: target/jankurai/repair-queue.jsonl
    sarif: target/jankurai/jankurai.sarif
    no_score_history: true
  repair_plan:
    enabled: true
    json: target/jankurai/repair-plan.json
    md: target/jankurai/repair-plan.md
  task_source: repair_plan
  selection:
    order: quick_wins_first
    randomize_ties: true
    max_risk: low
    skip_human_review_required: true
    incubate_risk_at: medium
    defer_rules: [HLT-010-SECRET-SPRAWL]
    incubate_rules: [HLT-006-DIRECT-DB-WRONG-LAYER]
  regression:
    main_ref: origin/main
    compare_every_iterations: 5
    mode: advisory
    max_new_hard_findings: 0
    max_score_drop: 0
  verification:
    require_clean_start: true
    require_clean_after_checkpoint: true
    proof_from_test_map: true
    commands:
      - "just fast"
    audit_delta: no_new_findings
    rollback_unverified: true
unsupported_feature_policy:
  required: [jankurai]`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.spec.jankurai?.enabled).toBe(true)
    expect(parsed.preview.jankurai_enabled).toBe(true)
    expect(parsed.preview.jankurai_summary).toContain("source:repair_plan")
    expect(parsed.preview.jankurai_summary).toContain("workers:10")
    expect(parsed.preview.jankurai_verification_summary).toContain("audit_delta:no_new_findings")
    expect(parsed.preview.unsupported_feature_policy_summary).toContain("required:1")
  })

  test("rejects unknown nested jankurai keys", async () => {
    const text = makeZyal(`
jankurai:
  enabled: true
  surprise: false`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow("Unknown ZYAL key: jankurai.surprise")
  })

  test("rejects invalid jankurai risk literals", async () => {
    const text = makeZyal(`
jankurai:
  enabled: true
  selection:
    max_risk: tiny`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("rejects invalid jankurai audit modes", async () => {
    const text = makeZyal(`
jankurai:
  enabled: true
  audit:
    mode: experimental`)
    await expect(Effect.runPromise(parseZyal(text))).rejects.toThrow()
  })

  test("accepts unsupported feature policy requiring jankurai", async () => {
    const text = makeZyal(`
jankurai:
  enabled: true
unsupported_feature_policy:
  required: [jankurai]
  fail_closed: true
  on_missing: reject`)
    const parsed = await Effect.runPromise(parseZyal(text))
    expect(parsed.preview.jankurai_enabled).toBe(true)
  })
})
