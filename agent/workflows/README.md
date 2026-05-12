# `.zyal` workflow sources

This directory holds **source-of-truth** `.zyal` files (Profile C — pragma
`# zyal: declarative target=github-workflow schema=actions/workflow@1`) that
compile to `.github/workflows/*.yml` via `zyalc`.

The compiler is registered as a generated zone in
`agent/generated-zones.toml`. CI runs `zyalc compile --check --all` to fail
fast if a `.zyal` was edited without re-running the compiler.

Workflows that are NOT mirrored here remain authored directly under
`.github/workflows/*.yml`. The migration is incremental — start with high-traffic
proof workflows (jankurai, security, test, typecheck) and add more as the
ZYAL profile grammar stabilises.

## Local commands

```
just zyalc-check          # cargo check the compiler crate
just zyalc-test           # run compiler tests (round-trip + idempotency)
just zyalc-compile-check  # drift detector across every registered .zyal
```

## Editing rules

1. Edit the `.zyal` source, not the generated `.yml`.
2. Run `cargo run -p zyalc -- compile --all` locally before pushing.
3. Both the `.zyal` and the regenerated `.yml` must be committed in the same
   change set so CI's drift detector stays green.
