<p align="center">The open source AI coding agent.</p>
<p align="center">
  <a href="https://jekko.ai/discord"><img alt="Discord" src="https://img.shields.io/discord/1391832426048651334?style=flat-square&label=discord" /></a>
  <a href="https://www.npmjs.com/package/jekko-ai"><img alt="npm" src="https://img.shields.io/npm/v/jekko-ai?style=flat-square" /></a>
  <a href="https://github.com/neverhuman/jekko/actions/workflows/publish.yml"><img alt="Build status" src="https://img.shields.io/github/actions/workflow/status/neverhuman/jekko/publish.yml?style=flat-square&branch=main" /></a>
</p>

<p align="center">
  <img src="docs/assets/jekko-tui-zyal-demo.gif" alt="Jekko TUI launching and running a ZYAL research loop" width="100%" />
</p>

<p align="center">
  <a href="README.md">English</a> |
  <a href="README.zh.md">简体中文</a> |
  <a href="README.zht.md">繁體中文</a> |
  <a href="README.ko.md">한국어</a> |
  <a href="README.de.md">Deutsch</a> |
  <a href="README.es.md">Español</a> |
  <a href="README.fr.md">Français</a> |
  <a href="README.it.md">Italiano</a> |
  <a href="README.da.md">Dansk</a> |
  <a href="README.ja.md">日本語</a> |
  <a href="README.pl.md">Polski</a> |
  <a href="README.ru.md">Русский</a> |
  <a href="README.bs.md">Bosanski</a> |
  <a href="README.ar.md">العربية</a> |
  <a href="README.no.md">Norsk</a> |
  <a href="README.br.md">Português (Brasil)</a> |
  <a href="README.th.md">ไทย</a> |
  <a href="README.tr.md">Türkçe</a> |
  <a href="README.uk.md">Українська</a> |
  <a href="README.bn.md">বাংলা</a> |
  <a href="README.gr.md">Ελληνικά</a> |
  <a href="README.vi.md">Tiếng Việt</a>
</p>

[![Jekko Terminal UI](docs/assets/jekko-tui-zyal-demo.gif)](https://jekko.ai)

---

### Installation

```bash
# YOLO
curl -fsSL https://jekko.ai/install | bash

# Package managers
npm i -g jekko-ai@latest        # or bun/pnpm/yarn
scoop install jekko             # Windows
choco install jekko             # Windows
brew install neverhuman/tap/jekko # macOS and Linux (recommended, always up to date)
brew install jekko              # macOS and Linux (official brew formula, updated less)
sudo pacman -S jekko            # Arch Linux (Stable)
paru -S jekko-bin               # Arch Linux (Latest from AUR)
mise use -g jekko               # Any OS
nix run nixpkgs#jekko           # or github:neverhuman/jekko for latest dev branch
```

> [!TIP]
> Remove versions older than 0.1.x before installing.

#### Installation Directory

The install script respects the following priority order for the installation path:

1. `$JEKKO_INSTALL_DIR` - Custom installation directory
2. `$XDG_BIN_DIR` - XDG Base Directory Specification compliant path
3. `$HOME/bin` - Standard user binary directory (if it exists or can be created)
4. `$HOME/.jekko/bin` - Default fallback

```bash
# Examples
JEKKO_INSTALL_DIR=/usr/local/bin curl -fsSL https://jekko.ai/install | bash
XDG_BIN_DIR=$HOME/.local/bin curl -fsSL https://jekko.ai/install | bash
```

### Agents

Jekko includes two built-in agents you can switch between with the `Tab` key.

- **build** - Default, full-access agent for development work
- **plan** - Read-only agent for analysis and code exploration
  - Denies file edits by default
  - Asks permission before running bash commands
  - Ideal for exploring unfamiliar codebases or planning changes

Also included is a **general** subagent for complex searches and multistep tasks.
This is used internally and can be invoked using `@general` in messages.

Learn more about [agents](https://jekko.ai/docs/agents).

### Documentation

For more info on how to configure Jekko, [**head over to our docs**](https://jekko.ai/docs).

### Audits

These are the supported repo audit and release gates:

```bash
just check-fast
just score-fast
just score
just security
tools/check-encrypted-paths.sh --index
jankurai doctor --fail-on critical
jankurai doctor --fail-on high
```

`just score-fast` is the quick release check. `just score` is the full repo score refresh. `tools/check-encrypted-paths.sh --index` verifies the git-crypt policy on staged content.

### ZYAL

ZYAL is Jekko's daemon/runbook contract. Start here:

- [ZYAL mission and runtime model](docs/ZYAL_MISSION.md)
- [ZYAL example runbooks](docs/ZYAL/examples/README.md)
- [ZYAL daemon spec](packages/jekko/specs/daemon-zyal.md)

The README demo GIF above shows the current ZYAL research loop preview in the TUI.

### Local Iteration

Use the narrow `just` targets for fast, deterministic agent work. These are the
fast deterministic build/test targets for agent iteration:

```bash
just fast    # cheap lane for local health checks
just build   # workspace build gate
just test    # workspace test gate
just score   # repo-score refresh
```

Keep proofs short and targeted. Use the smallest lane that covers the change,
then record the command and result in the task entry. Reuse the locked
dependency graph and the existing CI cache hints instead of inventing new build
paths.

For narrower iteration, prefer these lane-specific targets:

```bash
just workspace-typecheck-fast
just workspace-build-fast
just workspace-test-fast
just workspace-fast
```

### Contributing

If you're interested in contributing to Jekko, please read our [contributing docs](./CONTRIBUTING.md) before submitting a pull request.

### Building on Jekko

If you are working on a project that's related to Jekko and is using "jekko" as part of its name, for example "jekko-dashboard" or "jekko-mobile", please add a note to your README to clarify that it is not built by the Jekko team and is not affiliated with us in any way.

### FAQ

#### How is this different from Claude Code?

It's very similar to Claude Code in terms of capability. Here are the key differences:

- 100% open source
- Not coupled to any provider. Although we recommend the models we provide through [Jekko Zen](https://jekko.ai/zen), Jekko can be used with Claude, OpenAI, Google, or even local models. As models evolve, the gaps between them will close and pricing will drop, so being provider-agnostic is important.
- Built-in opt-in LSP support
- A focus on TUI. Jekko is built by neovim users and the creators of [terminal.shop](https://terminal.shop); we are going to push the limits of what's possible in the terminal.
- A client/server architecture. This, for example, can allow Jekko to run on your computer while you drive it remotely from a mobile app, meaning that the TUI frontend is just one of the possible clients.

---

**Join our community** [Discord](https://discord.gg/jekko) | [X.com](https://x.com/jekko)
