# Install Jekko

Jekko ships as a Rust binary. Pick the install path that matches your
environment. Pre-built binary distribution and the Nix flake are still being
finalized as part of the Rust port; source install is the canonical path for
`v2.0.0`.

## Build from source

The most reliable path during the Rust port. Requires a recent stable Rust
toolchain (see the workspace `rust-toolchain.toml` if pinned) and a working
C toolchain for native crates.

```sh
git clone https://github.com/neverhuman/jekko
cd jekko
cargo build -p jekko-cli --release --locked
```

The host binary lands at `target/release/jekko`. Verify it:

```sh
target/release/jekko --version
target/release/jekko --help
```

Drop it onto your `PATH` (for example into `~/.local/bin`) or invoke it
directly.

## cargo install

Tagged Rust releases can be installed directly from the repo:

```sh
cargo install --git https://github.com/neverhuman/jekko --tag v2.0.0
```

The exact tag scheme tracks the release flow in `docs/release.md`.

## Nix flake (TBD)

A flake is checked in under `nix/`, but it is still being migrated to the
Rust-only target. Once migrated:

```sh
nix profile install github:neverhuman/jekko#jekko
```

For now, prefer the build-from-source path until the flake migration
completes. Track progress in the Rust port plan.

## Pre-built binaries (TBD)

Per-platform tarballs and zips are intended to be attached to each GitHub
Release. For `v2.0.0`, use the source or `cargo install --git` path unless
the GitHub Release includes a matching archive for your platform.

The expected layout is one archive per supported `(os, arch)` target with
`jekko` (or `jekko.exe`) inside, plus checksums.

## Verifying the install

After install, the canonical health check is:

```sh
jekko --version
jekko --help
```

Both should return `0`. For a deeper smoke that exercises the TUI surface,
run the host binary in pure mode with stderr logging visible:

```sh
jekko --pure --print-logs --log-level DEBUG
```

The TUI should paint a home frame within seconds. See `docs/testing-tui.md`
for the diagnostic harness if the screen stays blank.
