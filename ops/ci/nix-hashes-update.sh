#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

set -euo pipefail
git pull --rebase --autostash origin "$GITHUB_REF_NAME"
HASH_FILE="nix/hashes.json"
[ -f "$HASH_FILE" ] || echo '{"nodeModules":{}}' > "$HASH_FILE"
for SYSTEM in x86_64-linux aarch64-linux x86_64-darwin aarch64-darwin; do
  FILE="hashes/hash-${SYSTEM}/hash.txt"
  if [ -f "$FILE" ]; then
    HASH="$(tr -d '[:space:]' < "$FILE")"
    echo "${SYSTEM}: ${HASH}"
    jq --arg sys "$SYSTEM" --arg h "$HASH" '.nodeModules[$sys] = $h' "$HASH_FILE" > tmp.json
    mv tmp.json "$HASH_FILE"
  else
    echo "::warning::Missing hash for ${SYSTEM}"
  fi
done
cat "$HASH_FILE"
if [ -z "$(git status --short -- "$HASH_FILE")" ]; then
  echo "No changes to commit"
  echo "### Nix hashes" >> "$GITHUB_STEP_SUMMARY"
  echo "Status: no changes" >> "$GITHUB_STEP_SUMMARY"
  exit 0
fi
git add "$HASH_FILE"
git commit -m "chore: update nix node_modules hashes"
git pull --rebase --autostash origin "$GITHUB_REF_NAME"
git push origin HEAD:"$GITHUB_REF_NAME"
echo "### Nix hashes" >> "$GITHUB_STEP_SUMMARY"
echo "Status: committed $(git rev-parse --short HEAD)" >> "$GITHUB_STEP_SUMMARY"
