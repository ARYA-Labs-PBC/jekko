# Contributing to Jekko

We want to make it easy for you to contribute to Jekko. Here are the most common type of changes that get merged:

- Bug fixes
- Additional LSPs / Formatters
- Improvements to LLM performance
- Support for new providers
- Fixes for environment-specific quirks
- Missing standard behavior
- Documentation improvements

However, any UI or core product feature must go through a design review with the core team before implementation.

If you are unsure if a PR would be accepted, feel free to ask a maintainer or look for issues with any of the following labels:

- [`help wanted`](https://github.com/anomalyco/jekko/issues?q=is%3Aissue%20state%3Aopen%20label%3Ahelp-wanted)
- [`good first issue`](https://github.com/anomalyco/jekko/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22good%20first%20issue%22)
- [`bug`](https://github.com/anomalyco/jekko/issues?q=is%3Aissue%20state%3Aopen%20label%3Abug)
- [`perf`](https://github.com/anomalyco/jekko/issues?q=is%3Aopen%20is%3Aissue%20label%3A%22perf%22)

> [!NOTE]
> PRs that ignore these guardrails will likely be closed.

Want to take on an issue? Leave a comment and a maintainer may assign it to you unless it is something we are already working on.

## Adding New Providers

New providers shouldn't require many if ANY code changes, but if you want to add support for a new provider first make a PR to:
https://github.com/anomalyco/models.dev

## Developing Jekko

- Requirements: the pinned Rust toolchain, Cargo, and `just`. The reproducible
  shell provides them:

  ```bash
  nix develop
  ```

- From the repo root, use the Rust lanes:

  ```bash
  cargo run -p jekko-cli -- --help
  cargo run -p jekko-cli -- .
  cargo run -p jekko-cli -- serve --port 4096
  just fast
  just tui-ci
  ```

### Running against a different directory

By default, `jekko` starts in the current directory. To run it against a
different directory or repository:

```bash
cargo run -p jekko-cli -- <directory>
```

To run Jekko in the root of the jekko repo itself:

```bash
cargo run -p jekko-cli -- .
```

### Building a Local Binary

To compile a standalone executable:

```bash
cargo run -p xtask -- publish-build-script --single
```

Then run it with:

```bash
./packages/jekko/dist/jekko-<platform>/bin/jekko
```

Replace `<platform>` with your platform (e.g., `darwin-arm64`, `linux-x64`).

- Core pieces:
  - `crates/jekko-cli`: argument parsing, subcommand dispatch, and the user-facing binary.
  - `crates/jekko-tui`: the Ratatui + Crossterm terminal UI.
  - `crates/jekko-server`: the Axum HTTP API and OpenAPI surface.
  - `crates/jekko-store`: SQLite persistence and embedded migrations.
  - `crates/jekko-runtime`: agent loop, tools, and session orchestration.
  - `crates/jekko-plugin-api`: declarative plugin manifest contract.

### Understanding cargo run vs jekko

During development, `cargo run -p jekko-cli -- ...` is the source-built
equivalent of the installed `jekko` command. Both run the same CLI interface:

```bash
# Development (from project root)
cargo run -p jekko-cli -- --help      # Show all available commands
cargo run -p jekko-cli -- serve       # Start headless API server
cargo run -p jekko-cli -- <directory> # Start TUI in a specific directory

# Production
jekko --help                          # Show all available commands
jekko serve                           # Start headless API server
jekko <directory>                     # Start TUI in a specific directory
```

### Running the API Server

To start the Jekko headless API server:

```bash
cargo run -p jekko-cli -- serve
```

This starts the headless server on port 4096 by default. You can specify a different port:

```bash
cargo run -p jekko-cli -- serve --port 8080
```

### Testing the TUI

TUI changes should be verified through the host-binary PTY lane:

```bash
just tui-ci
```

> [!NOTE]
> If you make changes to the HTTP API or OpenAPI output, run
> `cargo run -p xtask -- openapi-check --strict` and update the checked snapshot
> only when the API change is intentional.

Please try to follow the [style guide](./AGENTS.md)

### Setting up a Debugger

Rust debugging works through standard Cargo targets. For CLI/server work, run
`cargo run -p jekko-cli -- <subcommand>` under your debugger. For TUI boot and
PTY issues, build the host binary first and point the harness at it:

```bash
JEKKO_BIN="$(cargo run -p xtask -- host-binary-path)" just tui-startup-smoke
```

## Pull Request Expectations

### Issue First Policy

**All PRs must reference an existing issue.** Before opening a PR, open an issue describing the bug or feature. This helps maintainers triage and prevents duplicate work. PRs without a linked issue may be closed without review.

- Use `Fixes #123` or `Closes #123` in your PR description to link the issue
- For small fixes, a brief issue is fine - just enough context for maintainers to understand the problem

### General Requirements

- Keep pull requests small and focused
- Explain the issue and why your change fixes it
- Before adding new functionality, ensure it doesn't already exist elsewhere in the codebase

### UI Changes

If your PR includes UI changes, please include screenshots or videos showing the before and after. This helps maintainers review faster and gives you quicker feedback.

### Logic Changes

For non-UI changes (bug fixes, new features, refactors), explain **how you verified it works**:

- What did you test?
- How can a reviewer reproduce/confirm the fix?

### No AI-Generated Walls of Text

Long, AI-generated PR descriptions and issues are not acceptable and may be ignored. Respect the maintainers' time:

- Write short, focused descriptions
- Explain what changed and why in your own words
- If you can't explain it briefly, your PR might be too large

### PR Titles

PR titles should follow conventional commit standards:

- `feat:` new feature or functionality
- `fix:` bug fix
- `docs:` documentation or README changes
- `chore:` maintenance tasks, dependency updates, etc.
- `refactor:` code refactoring without changing behavior
- `test:` adding or updating tests

You can optionally include a scope to indicate which package is affected:

- `feat(app):` feature in the app package
- `chore(jekko):` maintenance in the jekko package

Examples:

- `docs: update contributing guidelines`
- `fix: resolve crash on startup`
- `feat: add dark mode support`
- `feat(app): add dark mode support`
- `chore: bump dependency versions`

### Style Preferences

These are not strictly enforced, they are just general guidelines:

- **Functions:** Keep logic within a single function unless breaking it out adds clear reuse or composition benefits.
- **Destructuring:** Do not do unnecessary destructuring of variables.
- **Control flow:** Avoid `else` statements.
- **Error handling:** Prefer `.catch(...)` instead of `try`/`catch` when possible.
- **Types:** Reach for precise types and avoid `any`.
- **Variables:** Stick to immutable patterns and avoid `let`.
- **Naming:** Choose concise single-word identifiers when they remain descriptive.
- **Runtime APIs:** Prefer the standard library and existing workspace helpers before adding a dependency.

## Feature Requests

For net-new functionality, start with a design conversation. Open an issue describing the problem, your proposed approach (optional), and why it belongs in Jekko. The core team will help decide whether it should move forward; please wait for that approval instead of opening a feature PR directly.

## Trust & Vouch System

This project uses [vouch](https://github.com/mitchellh/vouch) to manage contributor trust. The vouch list is maintained in [`.github/VOUCHED.td`](.github/VOUCHED.td).

### How it works

- **Vouched users** are explicitly trusted contributors.
- **Denounced users** are explicitly blocked. Issues and pull requests from denounced users are automatically closed. If you have been denounced, you can request to be unvouched by reaching out to a maintainer on [Discord](https://jekko.ai/discord)
- **Everyone else** can participate normally — you don't need to be vouched to open issues or PRs.

### For maintainers

Collaborators with write access can manage the vouch list by commenting on any issue:

- `vouch` — vouch for the issue author
- `vouch @username` — vouch for a specific user
- `denounce` — denounce the issue author
- `denounce @username` — denounce a specific user
- `denounce @username <reason>` — denounce with a reason
- `unvouch` / `unvouch @username` — remove someone from the list

Changes are committed automatically to `.github/VOUCHED.td`.

### Denouncement policy

Denouncement is reserved for users who repeatedly submit low-quality AI-generated contributions, spam, or otherwise act in bad faith. It is not used for disagreements or honest mistakes.

## Issue Requirements

All issues **must** use one of our issue templates:

- **Bug report** — for reporting bugs (requires a description)
- **Feature request** — for suggesting enhancements (requires verification checkbox and description)
- **Question** — for asking questions (requires the question)

Blank issues are not allowed. When a new issue is opened, an automated check verifies that it follows a template and meets our contributing guidelines. If an issue doesn't meet the requirements, you'll receive a comment explaining what needs to be fixed and have **2 hours** to edit the issue. After that, it will be automatically closed.

Issues may be flagged for:

- Not using a template
- Required fields left empty or filled with placeholder text
- AI-generated walls of text
- Missing meaningful content

If you believe your issue was incorrectly flagged, let a maintainer know.
