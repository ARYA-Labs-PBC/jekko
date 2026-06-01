# `dummy_agent_llm`

`dummy_agent_llm` is Jekko's deterministic local LLM provider for agent and TUI workflow tests. It uses the normal Rust provider adapter interface, but it never opens a network connection, sleeps, reads an API key, or spends tokens.

## Selecting it

Pass the provider explicitly and use the model id as the scenario id:

```bash
jekko run --provider dummy_agent_llm --model default "say hello"
jekko run --provider dummy_agent_llm --model tool-call "inspect README"
jekko run --provider dummy_agent_llm --model error "exercise failure handling"
```

If no model is provided for this provider, Jekko's catalog recommends `default`.

## Built-in scenarios

Scenarios are strict JSON fixtures embedded at `crates/jekko-provider/src/providers/dummy_agent_llm_scenarios.json`.

- `default` emits `StreamStart -> TextDelta -> Usage(0) -> StreamEnd` with stable assistant text.
- `tool-call` emits a deterministic `Read` tool call, including start/input/end frames, then emits a stable final answer when a tool-result message is present on the next runtime round.
- `error` emits `StreamStart` and then a scripted provider error.

A scenario can also be selected by adapter options named `scenario` or `dummy_agent_llm_scenario`, or by the `dummy_agent_llm.scenario` option object. Runtime CLI flows normally select it through `--model`.

## Adding a scenario

Add a new object to `dummy_agent_llm_scenarios.json` with a stable `id`, title, tags, `provider: "dummy_agent_llm"`, a non-empty `model`, and ordered `frames`. Fixture parsing denies unknown fields and rejects duplicate or blank ids, provider mismatches, blank models, and empty frame lists.

Keep outputs deterministic: no wall-clock text, random ids, absolute local paths, sleeps, network calls, API keys, or hidden chain-of-thought.

## Test coverage

Provider tests cover deterministic text output, completed tool-call frames, scripted errors, and option-based scenario selection. Runtime provider-selection tests cover `dummy_agent_llm` model construction and adapter resolution without credentials.
