# ZYAL: Zero-Trust YAML Agent Language

## A Host-Enforced Operating Contract for Autonomous Software Engineering

**Authors:** Jekko Contributors
**Version:** 1.0 — May 2026
**Status:** Reference implementation shipped; preview control-plane blocks under active validation

---

## Abstract

We present **ZYAL** (Zero-trust YAML Agent Language), a declarative, host-enforced control language for governing long-running autonomous AI coding agents. ZYAL addresses a practical systems problem in agentic software engineering: how to let a language model work autonomously for hours, editing files, running tests, managing tasks, and checkpointing progress, without surrendering safety, observability, or human control.

ZYAL is not a prompt framework, an agent SDK, or a model fine-tuning strategy. It is a **typed contract between a human operator and an autonomous agent runtime**, expressed as a strict YAML runbook with 40+ declarative blocks covering iteration policy, stop conditions, evidence requirements, capability leases, anti-vibe engineering gates, hypothesis tournaments, multi-scope budgets, human approval chains, and zero-trust arming.

The key insight is structural: safety properties that matter, including bounded execution, evidence-gated promotion, secret protection, anti-vibe discipline, and cost caps, cannot be reliably enforced by instructing the model. They must be enforced by the host runtime where the model has no ability to circumvent them.

The Jekko reference implementation includes a typed parser and schema, daemon runtime modules, SQLite-backed state, HTTP preview/start APIs, and a TUI Run Card. Core daemon control, parsing, preview, checkpointing, stop checks, task state, and observability are shipped; several higher-risk v2.1+ blocks are intentionally preview-first contracts until the corresponding runtime enforcement paths are wired. This paper describes the language design, threat model, implementation boundary, evaluation plan, and remaining validity threats.

**Index Terms:** autonomous software engineering, agent safety, policy-as-code, YAML, zero trust, human-in-the-loop systems, software agents.

---

## 1. Introduction

### 1.1 The Problem

The current generation of AI coding assistants operates in a request-response loop: the human asks, the model answers, the human verifies. This interaction model works for small, bounded tasks — fixing a type error, writing a unit test, explaining a function — but fundamentally breaks at the scale where AI could deliver transformative productivity:

- **Multi-hour refactoring** across hundreds of files
- **Continuous integration daemons** that fix failures autonomously
- **Architecture migrations** requiring coordinated changes across service boundaries
- **Security audits** spanning entire codebases
- **Technical debt reduction** as an always-on background process

Extending AI agents to these tasks without a governance layer produces predictable, catastrophic failure modes:

| Failure Mode | Root Cause | Consequence |
|---|---|---|
| Unbounded autonomy | No iteration limits or circuit breakers | Model rewrites half the codebase chasing a red herring |
| No memory | Context window eviction | Rediscovers the same dead ends every session |
| No evidence discipline | "Tests pass" is the only gate | Broken code reaches production |
| No safety boundary | Model can execute any shell command | `git push --force`, `rm -rf`, credential exfiltration |
| No cost control | No budget enforcement | A runaway loop burns $500 before detection |
| Vibe coding | No structural quality gates | Deletes tests, weakens assertions, adds `@ts-ignore` |
| Model self-certification | Model decides when it's "done" | Claims completion without verification |

### 1.2 The Thesis

ZYAL's thesis is that **the host, not the model, must own the control loop**. Every safety property that matters must be enforced structurally by the runtime — not by asking the model to remember rules, follow a system prompt, or self-report its confidence honestly.

This is a zero-trust architecture applied to AI agents: the model is treated as an untrusted executor within a trusted control plane. The model does the thinking; the host does the governing; the human does the deciding.

### 1.3 Contributions

1. **A declarative agent contract** with 40+ typed blocks for behavior, safety constraints, preview metadata, and promotion criteria.
2. **A zero-trust safety model** that separates trusted host control from untrusted model output, including structural arming, capability leases, command floors, anti-vibe gates, and model confidence caps.
3. **A durable daemon architecture** backed by SQLite state, event logging, task ledgers, context rotation, checkpointing, and a human-readable filesystem mirror.
4. **A hard-task incubation model** with 8 maturation passes, risk/readiness scoring, negative memory, and evidence-gated promotion.
5. **A hypothesis tournament abstraction** for competing implementation strategies, isolated worktrees, and cross-provider review.
6. **A reference implementation and artifact plan** that distinguishes host-enforced core behavior from preview-only control-plane blocks, making the implementation auditable rather than aspirational.

### 1.4 Threat Model

ZYAL assumes the model is useful but untrusted. It may hallucinate, omit evidence, follow hostile instructions embedded in web pages or tool output, overstate completion, request excessive permissions, or generate syntactically valid but unsafe commands. ZYAL also assumes that external content, MCP resources, issue comments, and assistant output are not trusted origins for arming a daemon.

The trusted computing base is the host runtime: parser, schema decoder, daemon state machine, tool permission layer, durable store, and TUI/API control surface. The host is expected to run on a machine whose operating-system account, filesystem permissions, and secret store are already trusted by the human operator. ZYAL does not claim to defend against a compromised host process, malicious kernel, tampered binary, or provider-side model logging. Its claim is narrower and testable: model output cannot bypass host-owned control-flow checks that are actually wired into the runtime.

---

## 2. Related Work

### 2.1 Agent Frameworks

| System | Year | Approach | ZYAL Difference |
|---|---|---|---|
| AutoGPT [1] | 2023 | Unbounded agent loop with plugin tools | No stop conditions, no budgets, no evidence gates. ZYAL enforces all three structurally. |
| BabyAGI [2] | 2023 | Task queue with LLM-driven prioritization | Model owns the task queue. ZYAL's task ledger is host-managed with readiness scoring. |
| MetaGPT [3] | 2023 | Multi-agent role-play with SOPs | Roles are prompt-based. ZYAL agents have capability leases and write scopes enforced at runtime. |
| CrewAI [4] | 2024 | Crew of agents with defined roles | No durable state, no checkpoint gating, no evidence requirements. |
| LangGraph [5] | 2024 | Graph-based agent orchestration | Workflow only — no safety model, no budgets, no anti-vibe gates, no zero-trust arming. |
| Devin [6] | 2024 | Full-stack autonomous SWE agent | Proprietary, closed. No user-configurable safety constraints or evidence requirements. |
| OpenHands [7] | 2024 | Open SWE agent with sandboxed execution | Sandbox-only safety. No declarative contracts, no host-enforced budgets, no evidence gates. |
| Aider [8] | 2024 | Pair-programming CLI with git integration | Single-turn. No daemon mode, no incubation, no multi-agent orchestration. |
| Claude Code [9] | 2025 | Agentic coding with permission system | Permission prompts only. No declarative runbooks, no evidence-gated promotion, no anti-vibe gates. |
| Codex CLI [10] | 2025 | Terminal agent with sandboxing | Network/filesystem sandbox. No declarative contracts, no durable state, no tournament model. |
| SWE-agent [16] | 2024 | Agent-computer interface for software engineering | Improves tool ergonomics; ZYAL focuses on host-owned governance and durable contracts. |

### 2.2 Agent Safety Research

| Work | Contribution | ZYAL Extension |
|---|---|---|
| Constitutional AI [11] | Model self-critique via principles | Principles are in the prompt → model can ignore. ZYAL enforces at runtime. |
| RLHF [12] | Reward model alignment | Training-time safety. ZYAL provides inference-time structural safety. |
| Tool-use sandboxing [13] | Restrict tool access | Coarse allow/deny. ZYAL provides per-tool, per-path, per-command, time-windowed capability leases. |
| Guardrails AI [14] | Output validation framework | Validation only. ZYAL integrates validation with lifecycle management, budgets, and promotion gates. |

### 2.3 Configuration Languages

| Language | Domain | ZYAL Relationship |
|---|---|---|
| Kubernetes YAML | Container orchestration | Declarative desired-state. ZYAL applies the same philosophy to agent orchestration. |
| GitHub Actions YAML | CI/CD pipelines | Workflow definitions. ZYAL extends this with evidence gates, budgets, and safety invariants. |
| Terraform HCL | Infrastructure-as-code | Declarative infrastructure. ZYAL is declarative agent governance. |
| Rego (OPA) [15] | Policy-as-code | Policy evaluation. ZYAL embeds policy evaluation into the agent lifecycle. |

### 2.4 Benchmarks and Evaluation Context

SWE-bench [17] and related agent benchmarks measure whether agents can resolve real repository issues. ZYAL is complementary: it is not a model benchmark, but a governance layer for running agents against such tasks with bounded autonomy, durable evidence, and reproducible stop conditions. A mature ZYAL evaluation should therefore report both task success rates and governance outcomes: prevented unsafe actions, false-positive gates, budget exhaustion behavior, rollback success, and human approval latency.

---

## 3. Language Design

### 3.1 Sentinel Format

Every ZYAL script is wrapped in typed sentinels that are structurally distinct from markdown, code fences, and model output:

```
<<<ZYAL v1:daemon id=my-script>>>
version: v1
intent: daemon
confirm: RUN_FOREVER
# ... YAML body ...
<<<END_ZYAL id=my-script>>>
ZYAL_ARM RUN_FOREVER id=my-script
```

**Design decisions:**
- Triple angle brackets (`<<<>>>`) cannot appear in normal markdown or code output
- Opening and closing IDs must match (parser-enforced)
- Code fences around the block are **rejected** — the sentinels are the format
- The `ZYAL_ARM` sentinel is a **separate line** requiring a distinct human action
- Arming can be hash-bound (`sha256=...`) and nonce-bound (`nonce=...`) for zero-trust environments

### 3.2 Block Taxonomy

ZYAL's 40+ blocks are organized into 5 evolutionary layers:

#### Layer 1: Core (v1) — 15 blocks
The minimum viable daemon: what to do, when to stop, how to commit.

`version` · `intent` · `confirm` · `id` · `job` · `loop` · `stop` · `context` · `checkpoint` · `tasks` · `incubator` · `agents` · `mcp` · `permissions` · `ui`

#### Layer 2: Safety (v1.1) — 7 blocks
Runtime safety beyond permissions: event-driven reactions, guardrails, invariants.

`on` · `fan_out` · `guardrails` · `assertions` · `retry` · `hooks` · `constraints`

#### Layer 3: Evidence (v2 Wave 1) — 4 blocks
Durable state machines, governed memory, typed proof bundles, human approval gates.

`workflow` · `memory` · `evidence` · `approvals`

#### Layer 4: Security (v2 Wave 2) — 4 blocks
Execution isolation, secrets management, skill governance, structured observability.

`skills` · `sandbox` · `security` · `observability`

#### Layer 5: Power (v2.1) — 10 blocks
The full autonomous engineering contract: zero-trust arming, capability leases, anti-vibe gates, hypothesis tournaments, multi-provider routing, nested budgets, external triggers, rollback plans, definition-of-done, and codebase intelligence.

`arming` · `capabilities` · `quality` · `experiments` · `models` · `budgets` · `triggers` · `rollback` · `done` · `repo_intelligence`

#### Layer 6: Control Plane Preview (v2.2+) — preview blocks
Interoperability and release metadata that should parse, preview, and fail closed before it becomes runtime-enforced policy.

`interop` · `runtime` · `capability_negotiation` · `memory_kernel` · `evidence_graph` · `trust` · `taint` · `requirements` · `evaluation` · `release` · `roles` · `channels` · `imports` · `reasoning_privacy` · `unsupported_feature_policy`

### 3.3 Schema and Parsing

The ZYAL schema is defined in TypeScript (`schema.ts`, approximately 2.1K lines / 85 KB at the time of this paper) with:
- Exhaustive type definitions for every block, sub-block, and enum
- Normalized canonical form for hash computation
- Semantic validation rules (e.g., workflow terminal states, evidence type uniqueness, approval gate cross-references)

The parser (`parser.ts`, approximately 1.7K lines / 72 KB at the time of this paper) implements:
- Sentinel extraction with code-fence rejection
- YAML parsing with strict key validation
- SHA-256 hash computation of the canonical normalized YAML
- ARM sentinel matching for the shipped simple arming path, with hash/nonce fields represented as preview contracts until the start API carries host-issued arm tokens
- Preview mode (parse without arming) for validation

---

## 4. Safety Model

### 4.1 Zero-Trust Arming

The most dangerous attack surface in any agent system is unauthorized execution. ZYAL's `arming` block makes model-initiated execution structurally impossible:

1. **Origin rejection**: ARM sentinels from `assistant_output`, `tool_output`, `web_content`, or `mcp_resource` are rejected by the parser
2. **Hash binding**: The ARM token can include `sha256=<hash>` of the canonical script so modification invalidates the arm
3. **Nonce binding**: The host can generate a single-use nonce, preventing replay once the start API accepts host-issued tokens
4. **Context binding**: ARM can be bound to `user_id`, `repo`, `branch` — a script armed for staging cannot run in production
5. **Expiration**: Preview hashes expire after a configurable duration (default: 10 minutes)

The current shipped start path accepts the simple trailing `ZYAL_ARM RUN_FOREVER id=<run-id>` sentinel. Hash-bound, nonce-bound, and context-bound arming are parsed and previewed, but are not yet claimed as end-to-end runtime enforcement.

### 4.2 Capability Leases

Traditional permission systems use coarse `allow/deny` per tool. ZYAL's `capabilities` block provides fine-grained, time-windowed, path-scoped capability leases:

```yaml
capabilities:
  default: deny
  rules:
    - id: edit-src
      tool: edit
      paths: ["src/**"]
      decision: allow
    - id: shell-tests
      tool: shell
      command_regex: "^bun test"
      decision: allow
      expires: 2h
  command_floor:
    always_block: ["git push --force", "rm -rf /", "sudo ", "DROP DATABASE"]
```

The `command_floor` is specified as absolute — no `allow` rule may override it. Full tool-gate wiring is a runtime-enforcement requirement; until every tool path is covered, the block is treated as a preview contract rather than a completed security boundary.

### 4.3 Anti-Vibe Engineering

ZYAL's `quality` block structurally prevents the canonical vibe-coding failure modes:

| Gate | What it blocks |
|---|---|
| `block_test_deletion` | Removing or commenting out existing tests |
| `block_assertion_weakening` | Weakening assertions (e.g., `toBe` → `toBeTruthy`) |
| `block_silent_catch` | Empty catch blocks (`catch (e) {}`) |
| `block_fake_data_fallback` | Hardcoded mock data in production paths |
| `block_ts_ignore` | `@ts-ignore` / `@ts-expect-error` additions |
| `require_failing_test_first_for_bugfix` | Bug fixes require a reproduction first |

The gates are designed to fail closed when enabled. Diff/promotion integration is the enforcement boundary; preview-only deployments must display the policy without claiming complete host blocking.

### 4.4 Model Confidence Capping

ZYAL includes a `confidence_cap` (default: 0.6) that prevents the model from inflating its own readiness score past a threshold. The host evaluates readiness based on **structural evidence** (tests pass, scope bounded, plan reviewed, rollback known), not model self-reports.

---

## 5. Runtime Architecture

### 5.1 Durable State

All daemon state is persisted in SQLite across 9 relational tables:

| Table | Records |
|---|---|
| `daemon_run` | Run metadata: status, phase, iteration, epoch, spec hash |
| `daemon_iteration` | Per-iteration: token usage, cost, terminal reason, session ID |
| `daemon_event` | Append-only event log: every state transition, hook, handler |
| `daemon_task` | Task ledger: discovery, routing, status, lane, readiness |
| `daemon_task_pass` | Incubator passes: type, context mode, write scope, result |
| `daemon_task_memory` | Per-task memories: context capsules, findings, objections |
| `daemon_worker` | Worker pool: heartbeat, lease, session binding |
| `daemon_artifact` | Generated artifacts: evidence bundles, rollback patches |

### 5.2 Iteration Loop

Each daemon iteration follows a strict host-controlled sequence:

```
1. Check run status (abort/pause/fail → exit)
2. Execute before_iteration hooks
3. Run agent loop (model generates + executes tool calls)
4. Record iteration (tokens, cost, terminal reason)
5. Evaluate on-handlers for terminal signal
6. Check circuit breaker (consecutive errors)
7. Execute before_checkpoint hooks
8. Run checkpoint (verify → git add → commit → push)
9. Evaluate stop conditions (all + any)
10. Route tasks through incubator (if enabled)
11. Execute after_iteration hooks
12. Build continuation prompt
13. Rotate session (if hard_clear_every threshold crossed)
14. Sleep
```

### 5.3 Incubator

The incubator is ZYAL's answer to hard tasks. When a task exceeds routing thresholds (repeated attempts, high risk score), it enters an 8-pass maturation pipeline:

| # | Pass | Context | Write Scope | Purpose |
|---|---|---|---|---|
| 1 | Scout | Blind | Scratch | Independent exploration without bias |
| 2 | Idea (×3) | Inherit | Scratch | Generate competing approaches |
| 3 | Strengthen | Strengthen | Scratch | Stress-test ideas with adversarial probing |
| 4 | Critic | Critic | Scratch | Cross-provider critique (different model) |
| 5 | Synthesize | Pool | Scratch | Merge best elements from all ideas |
| 6 | Prototype | Inherit | Isolated worktree | Build in isolated git branch |
| 7 | Review | Promotion | Scratch | Evidence-gated promotion review |
| 8 | Compress | Ledger only | Scratch | Summarize for memory + archive |

Promotion requires **host-evaluated evidence**: `tests_identified ≥ 1`, `scope_bounded ≥ 1`, `plan_reviewed ≥ 1`, and `unresolved_critical_objections = 0`. The model's own readiness score is capped at 0.6.

### 5.4 Hypothesis Tournaments

The `experiments` block runs competing implementation strategies in parallel:

1. Each lane gets an isolated git worktree forked from the last green checkpoint
2. Plans must be distinct (minimum edit distance enforced by `min_plan_distance`)
3. A **blind** critic agent (required to use a different provider) scores each lane
4. The winning lane is promoted via `best_verified_patch` with a final verification pass
5. Failed lanes are preserved as **negative memory** — the system never repeats a dead end

---

## 6. Observability

### 6.1 TUI Gold Mode

When ZYAL is active, the Jekko TUI transforms:

- **Gold theme overlay**: 44-property custom theme (noir backgrounds, gold primaries, amber accents)
- **∞ ZYAL MODE sidebar**: Live iteration count, total tokens, total cost, uptime, daemon phase, job name
- **Instant validation**: `✓ ZYAL` / `✗ ZYAL: <error>` badge in prompt footer
- **Border tinting**: Gold for valid ZYAL, red for invalid
- **Fleet metrics**: Worker count, token breakdown, tasks completed/incubated

### 6.2 API Surface

The daemon exposes a complete HTTP API for programmatic observability and control:

| Category | Endpoints |
|---|---|
| Runs | `GET /daemon`, `GET /daemon/:id` (with `_stats` aggregation) |
| Events | `GET /daemon/:id/events` |
| Tasks | `GET /daemon/:id/tasks`, `GET /daemon/:id/tasks/:tid` |
| Passes | `GET /daemon/:id/tasks/:tid/passes` |
| Memory | `GET /daemon/:id/tasks/:tid/memory` |
| Control | `POST .../pause`, `POST .../resume`, `POST .../abort` |
| Incubator | `GET /daemon/:id/incubator`, `POST .../incubate`, `POST .../promote` |
| Preview | `POST /daemon/preview` |

### 6.3 Filesystem Mirror

All state is mirrored to `.jekko/daemon/<runID>/` for human inspection:

```
├── ledger.jsonl          # Append-only event log
├── STATE.md              # Human-readable run summary
└── tasks/<taskID>/
    ├── CAPSULE.md         # Context capsule
    ├── SCORE.json         # Readiness scores
    └── PASSES/
        ├── 001-scout.md
        ├── 002-idea.md
        └── 003-strengthen.md
```

---

## 7. Evaluation

### 7.1 Safety Properties

| Property | Current Enforcement Boundary | Status |
|---|---|---|
| Sentinel extraction, code-fence rejection, duplicate-block rejection | Parser | Shipped |
| Basic arming sentinel and ID matching | Parser / start path | Shipped |
| Hash/nonce/context-bound arming | Parser preview metadata; start API token path pending | Preview |
| Bounded execution | Loop policy, circuit breaker, stop checks | Shipped for core daemon loop |
| Evidence-gated promotion | Task readiness and promotion helpers | Partially wired |
| Secret protection | Security/redaction helpers and capability preview | Partial / preview depending on tool path |
| Anti-vibe discipline | Quality block schema and preview | Preview until diff gates are wired everywhere |
| Cost control | Observability and budget helpers | Partial; nested v2.1 budgets are advisory in the start loop |
| Model confidence | Readiness cap in task scoring | Shipped where promotion helpers are used |

### 7.2 Expressiveness

The 9 flagship example runbooks in `docs/ZYAL/examples/` demonstrate that ZYAL can express:

1. Simple fix-until-green loops
2. Multi-worker parallel audits with guardrails
3. Hard task incubation with 8-pass maturation
4. Fan-out parallel map-reduce
5. Hypothesis tournaments with cross-provider judging
6. Billion-LOC monorepo scope control
7. Fleet portfolio management with external triggers
8. Full end-to-end autonomous engineering contracts
9. Preview-only control-plane metadata with fail-closed unsupported-feature policy

### 7.3 Reproducibility and Artifact Checks

The implementation is evaluated as an artifact-backed systems paper rather than a benchmark-only paper. A reviewer should be able to reproduce the claims with local source inspection and tests:

| Claim | Artifact / Command | Expected Result |
|---|---|---|
| All docs examples parse as ZYAL | `rtk bun test src/agent-script/parser.test.ts` from `packages/jekko` | Passes and includes all canonical `*.zyal` examples |
| Forbidden legacy language markers are absent | exact-token `rtk rg` scan from repository root | No source/docs hits except intentional workflow history |
| TUI ZYAL metrics and Jnoccio WebSocket behavior are covered | `rtk bun test test/cli/tui/zyal-flash.test.ts test/cli/tui/jnoccio-ws.test.ts` | Passes |
| Daemon APIs and session behavior still compile/test | daemon/session/API test group | Passes |
| Type safety | `rtk bun run typecheck` from `packages/jekko` | Passes |

### 7.4 Limitations and Threats to Validity

ZYAL is a runtime contract, not a proof that arbitrary autonomous code editing is safe. Its guarantees are only as strong as the enforcement points that are actually wired into the host. Preview-only blocks must not be represented as production security boundaries. The paper's evaluation is also implementation-centric: it demonstrates parser coverage, runtime surface tests, and architectural safety properties, but does not yet report large-scale SWE-bench outcomes, human-subject productivity studies, or red-team exploit rates across heterogeneous repositories.

The largest validity risks are incomplete tool-gate coverage, stale documentation drift, provider-specific behavior, race conditions in live observability channels, and accidental trust expansion when ZYAL is embedded into external chat or MCP surfaces. The `taint`, `trust`, and `unsupported_feature_policy` blocks are therefore important: they make unsafe assumptions explicit and fail closed for required preview capabilities.

---

## 8. Future Work

1. **Distributed daemon fleets** — multiple ZYAL daemons coordinating across repositories
2. **Learning from negative memory** — cross-run skill transfer from failed experiments
3. **Complete enforcement wiring for preview blocks** — hash/nonce arming tokens, capability gates on every tool path, diff-level anti-vibe checks, and nested budget exhaustion
4. **Formal verification** — proving safety properties of ZYAL scripts statically
5. **IDE integration** — ZYAL validation and authoring in VS Code, Zed, JetBrains
6. **Benchmark suite** — standardized evaluation of ZYAL daemons on SWE-bench tasks
7. **Security red-team suite** — adversarial prompts, malicious tool output, hostile MCP resources, and supply-chain skill attacks

---

## 9. Conclusion

ZYAL demonstrates that the gap between "AI coding assistant" and "autonomous software engineer" is not a model capability gap — it is a **governance gap**. Current models are capable of sustained, multi-hour autonomous work. What they lack is a control plane that makes that work safe, observable, bounded, and evidence-gated.

ZYAL fills this gap with a declarative language that gives the host total control over the agent lifecycle while giving the model maximum freedom within well-defined boundaries. The result is a system where the model does the thinking, ZYAL does the governing, and the human does the deciding.

That is the zero-trust contract for autonomous software engineering.

---

## References

[1] T. Richards et al., "Auto-GPT: An Autonomous GPT-4 Experiment," GitHub, 2023. [Online]. Available: https://github.com/Significant-Gravitas/AutoGPT

[2] Y. Nakajima, "BabyAGI: Task-Driven Autonomous Agent," GitHub, 2023. [Online]. Available: https://github.com/yoheinakajima/babyagi

[3] S. Hong et al., "MetaGPT: Meta Programming for A Multi-Agent Collaborative Framework," arXiv:2308.00352, 2023. [Online]. Available: https://arxiv.org/abs/2308.00352

[4] J. Moura, "CrewAI: Framework for orchestrating role-playing autonomous AI agents," GitHub, 2024. [Online]. Available: https://github.com/crewAIInc/crewAI

[5] LangChain Inc., "LangGraph: Building language agents as graphs," 2024. [Online]. Available: https://github.com/langchain-ai/langgraph

[6] Cognition Labs, "Devin: The first AI software engineer," 2024. [Online]. Available: https://www.cognition.ai/blog/introducing-devin

[7] X. Wang et al., "OpenHands: An Open Platform for AI Software Developers as Generalist Agents," arXiv:2407.16741, 2024. [Online]. Available: https://arxiv.org/abs/2407.16741

[8] P. Gauthier, "Aider: AI pair programming in your terminal," GitHub, 2024. [Online]. Available: https://github.com/Aider-AI/aider

[9] Anthropic, "Claude Code: Agentic coding tool," 2025. [Online]. Available: https://docs.anthropic.com/en/docs/claude-code

[10] OpenAI, "Codex CLI: Lightweight coding agent," GitHub, 2025. [Online]. Available: https://github.com/openai/codex

[11] Y. Bai et al., "Constitutional AI: Harmlessness from AI Feedback," arXiv:2212.08073, 2022. [Online]. Available: https://arxiv.org/abs/2212.08073

[12] L. Ouyang et al., "Training language models to follow instructions with human feedback," in *Advances in Neural Information Processing Systems*, 2022. [Online]. Available: https://proceedings.neurips.cc/paper_files/paper/2022/hash/b1efde53be364a73914f58805a001731-Abstract.html

[13] M. Chen et al., "Evaluating Large Language Models Trained on Code," arXiv:2107.03374, 2021. [Online]. Available: https://arxiv.org/abs/2107.03374

[14] Guardrails AI, "guardrails: Adding guardrails to large language models," GitHub, 2023. [Online]. Available: https://github.com/guardrails-ai/guardrails

[15] Open Policy Agent, "Policy Language," 2026. [Online]. Available: https://www.openpolicyagent.org/docs/latest/policy-language/

[16] J. Yang et al., "SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering," arXiv:2405.15793, 2024. [Online]. Available: https://arxiv.org/abs/2405.15793

[17] C. Jimenez et al., "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?," arXiv:2310.06770, 2023. [Online]. Available: https://arxiv.org/abs/2310.06770

---

## Appendix A: Complete Block Reference

See [`docs/ZYAL_MISSION.md`](../docs/ZYAL_MISSION.md) for the complete syntax reference with YAML examples for all 40+ blocks.

## Appendix B: Example Runbooks

See [`docs/ZYAL/examples/`](../docs/ZYAL/examples/) for 9 runbooks. Examples 1-8 exercise daemon patterns; example 9 is intentionally preview-only control-plane metadata:

| # | File | Blocks Demonstrated |
|---|---|---|
| 1 | `01-fix-until-green.zyal` | `job`, `loop`, `stop`, `checkpoint`, `quality`, `rollback`, `done` |
| 2 | `02-hypothesis-tournament.zyal` | `experiments`, `models`, `scoring`, `reduce`, `negative_memory` |
| 3 | `03-billion-loc-monorepo.zyal` | `repo_intelligence`, `scope_control`, `blast_radius`, `capabilities` |
| 4 | `04-fleet-portfolio.zyal` | `triggers`, `budgets`, `anti_recursion`, `idempotency` |
| 5 | `05-secure-mcp-lockdown.zyal` | `capabilities`, `command_floor`, `security`, `sandbox` |
| 6 | `06-evidence-graph-merge.zyal` | `evidence`, `workflow`, `approvals`, `rollback` |
| 7 | `07-self-improving-skills.zyal` | `skills`, `quarantine`, `promotion`, `memory` |
| 8 | `08-full-power-runbook.zyal` | All 40+ blocks end-to-end |
| 9 | `09-control-plane-preview.zyal` | `interop`, `runtime`, `capability_negotiation`, `memory_kernel`, `evidence_graph`, `trust`, `taint`, `unsupported_feature_policy` |

## Appendix C: Runtime Module Inventory

| Module | Size at review | Purpose |
|---|---|---|
| `schema.ts` | ~2.1K lines / 85 KB | Type definitions for all blocks |
| `parser.ts` | ~1.7K lines / 72 KB | Sentinel extraction, YAML parsing, hash computation |
| `parser.test.ts` | ~1.2K lines / 35 KB | Parser and example coverage |
| `activation.ts` | 27 lines / 1 KB | Real-time ZYAL detection |
| `daemon.ts` | ~809 lines / 31 KB | Core daemon loop |
| `daemon-store.ts` | ~991 lines / 37 KB | SQLite-backed durable state |
| `daemon-incubator.ts` | ~438 lines / 15 KB | 8-pass task maturation |
| `daemon-task-router.ts` | ~220 lines / 7.6 KB | Risk/readiness scoring, lane routing |
| `daemon-observability.ts` | ~260 lines / 6.7 KB | Spans, metrics, cost tracking |
| `daemon-sandbox.ts` | ~203 lines / 6.5 KB | Execution isolation helpers |
| `daemon-workflow.ts` | ~191 lines / 6 KB | Durable state machine |
| `daemon-security.ts` | ~196 lines / 5.5 KB | Secrets brokering, audit logging helpers |
| `daemon-memory.ts` | ~180 lines / 5.4 KB | Governed memory stores |
| `daemon-evidence.ts` | ~141 lines / 4.9 KB | Proof bundles with SHA-256 signing |
| `daemon-assertions.ts` | ~151 lines / 5 KB | Structured output contracts |
| `daemon-approvals.ts` | ~132 lines / 4.5 KB | Human approval gates |
| `daemon-reduce.ts` | ~158 lines / 4.1 KB | Result reduction strategies |
| `daemon-skills.ts` | ~136 lines / 3.9 KB | Skill governance helpers |
| `daemon-constraints.ts` | ~112 lines / 3.2 KB | Runtime invariants |
| `daemon-guardrails.ts` | ~98 lines / 2.9 KB | Input/output validation |
| `daemon-retry.ts` | ~99 lines / 2.7 KB | Backoff policies |
| `daemon-on-handler.ts` | ~60 lines / 2.4 KB | Signal-driven event handlers |
| `daemon-fanout.ts` | ~84 lines / 2.1 KB | Parallel execution |
| `daemon-hooks.ts` | ~54 lines / 1.7 KB | Lifecycle hook resolution |
| `daemon-context.ts` | ~59 lines / 1.5 KB | Iteration prompt builder |
| `daemon-event.ts` | ~54 lines / 1.7 KB | Typed event system |
| `zyal-flash.ts` | ~294 lines / 11 KB | Multi-source gold overlay + fleet metrics |
| `zyal-sidebar.tsx` | — | Live ∞ ZYAL MODE sidebar |
