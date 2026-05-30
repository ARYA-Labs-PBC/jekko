# jankurai Standard Agent Bootstrap

Standard version: `0.8.0`

Read `docs/agent-native-standard.md` when policy detail matters. Use `agent/owner-map.json`, `agent/test-map.json`, `agent/generated-zones.toml`, `agent/proof-lanes.toml`, `agent/tool-adoption.toml`, and `agent/boundaries.toml` before editing.

## Autonomy guardrails (instruction-layer)

The `[autonomy]` section of `agent/boundaries.toml` defines a soft, instruction-layer guardrail on agent autonomy scope. Honor it before any of the following operations: launching training runs, synthesizing atoms, pushing to HuggingFace or GCS, provisioning or resizing VMs, running GP search, modifying bundled atoms, or starting multi-hour experiments. Each of these requires explicit per-session user instruction. Merges, pushes to `main`, releases, branch deletions, and force pushes require explicit per-action user confirmation. After 20 consecutive actions OR 30 minutes without user interaction, surface status and wait for a continuation token before proceeding.

Runtime enforcement is layered on by the QantmOrchstrtr-RSI Safety Kernel via the `autonomous_action.*` policy type (ARY-2293) and the policy sidecar session budget (ARY-2294). The instruction-layer rule remains in force everywhere, including in environments where the runtime layer is not yet reachable.
