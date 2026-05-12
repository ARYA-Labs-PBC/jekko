# crates/tuiwright-jekko-unlock/AGENTS.md

<!-- jankurai generated adapter -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
Owns `crates/tuiwright-jekko-unlock/`.
Boundary: Rust crate implementing the jnoccio-fusion unlock TUI flow — derives a git-crypt key from a 128-char ASCII secret via AES-GCM envelope.
Forbidden: persisting plaintext secrets, embedding hardcoded keys, IO outside the documented unlock command.
Proof lane: `cargo audit` + Rust witness build via `jankurai rust witness build .`.
Security: all key material handling must be reviewed; no `--no-verify` or unsigned commits in this crate.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
