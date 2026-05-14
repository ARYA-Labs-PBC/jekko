# Real Paper QBank

Checked-in native QBank artifacts for the memory benchmark live here. The
production bank must use `opencode-qbank-challenge-v2` challenge artifacts and
an `opencode-qbank-manifest-v2` manifest.

- `papers/`: redistributable open-access paper records, keyed by `publication_hash`.
- `challenges/`: accepted, validated question records, keyed by `challenge_hash`.
- `rejected/`: rejected challenge records and publication receipts safe to commit.
- `manifests/`: deterministic run manifests and top-N selections.

Runtime discovery, extraction, worker attempts, and model receipts belong under
`.jekko/daemon/<run-id>/...`, not in this directory. Do not commit private
seeds, API keys, non-redistributable full text, or license-ambiguous content.
Synthetic or fixture-shaped QBank data is dev-only and must not be placed in the
production `papers/` or `challenges/` directories.
