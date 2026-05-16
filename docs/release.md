# Release

Jekko releases produce a tagged Rust binary, a GitHub Release entry with
per-platform artifacts, and updated documentation. The flow is end-to-end
Rust; there is no JS publish step.

## Version source

The release version comes from the Git tag (for example `v1.2.3`). The tag
is the source of record for published artifacts. CI and release workflows
consume this tag as the canonical version input. The workspace
`Cargo.toml` `[workspace.package].version` must match the tag.

## Cutting a release

1. **Tag the version.** Pick the next semver, create a signed annotated tag:

   ```sh
   git tag -s vX.Y.Z -m "vX.Y.Z"
   ```

2. **Build the release binary.** The host build is via `cargo`:

   ```sh
   cargo build -p jekko-cli --release --locked
   ```

   The binary lands at `target/release/jekko`.

3. **Smoke the binary.** Both checks must exit `0`:

   ```sh
   target/release/jekko --version
   target/release/jekko --help
   ```

4. **Run release proof lanes.** At a minimum, the comprehensive lanes:

   - `cargo test --workspace --locked --no-fail-fast`
   - `just tui-ci`
   - `cargo run -p xtask -- baseline-diff` (with the threshold the release
     targets)
   - `cargo run -p xtask -- guard-forbidden-runtime --mode advisory`

   Record the exit status of each in the release notes.

5. **Cross-compile per-platform artifacts.** Currently **TBD**: the
   cross-compile lane will land via either `cross` or the Nix flake once the
   Rust port finishes. Document the chosen path here before the first
   Rust-cut release.

6. **Create the GitHub Release.** Attach one archive per supported target
   tuple plus a `SHA256SUMS` file. Link the release notes to the proof
   command output for each lane in step 4.

7. **Publish docs.** Update any version-bound docs and confirm
   `docs/install.md` reflects the new tag.

## Changelog and evidence

- Each release entry includes noteworthy behavior changes, migration notes,
  and verification status.
- Each lane in the proof section links to the captured `cargo` or `xtask`
  output for that lane.
- Migration notes covering the Rust port stay attached to the release that
  introduces the cut-over, then drop out of subsequent releases.

## Integrity and provenance

- Artifacts are produced only from CI pipeline runs that match the release
  workflow configuration and the tagged commit SHA.
- Integrity is preserved by immutable tags and pinned workflow versions.
- The `SHA256SUMS` file is signed alongside the tag.

## Rollback guidance

- If a release is found faulty, cut a patch release for the corrected
  change and deprecate the affected version in the release notes.
- For urgent rollback, stop deployment, publish a replacement release, and
  post a postmortem with remediation evidence linked from the affected
  release entry.

## TBD

- Cross-compile path (`cross` vs Nix flake) is not yet finalized.
- `cargo install --git ... --tag <release>` becomes the canonical user
  install path once the first Rust release ships; until then, see
  `docs/install.md` for the build-from-source path.
