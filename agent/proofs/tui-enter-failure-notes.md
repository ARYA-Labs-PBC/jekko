# TUI Enter Failure Notes

- Root cause class: the TUI submit path already emitted the prompt request, but there was no test-only way to complete the round trip without a live model/provider response.
- Fix: `SessionPrompt.prompt` now recognizes `JEKKO_TUI_TEST_MOCK_LLM=1`, persists a synthetic assistant response, and returns it through the same session sync path the UI already renders.
- Test coverage: `crates/tuiwright-jekko-unlock/tests/tui_chat_enter_mock.rs` boots the real TUI, types a prompt, presses Enter, and asserts both the user text and mock assistant text render.
