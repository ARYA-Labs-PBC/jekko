#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

rtk cargo run -p xtask -- schema

if [ -z "$(git status --porcelain)" ]; then
  printf '%s\n' 'No changes to commit'
  exit 0
fi

while IFS= read -r file; do
  [ -n "$file" ] && git add "$file"
done < <(git status --name-only)

git commit -m "chore: generate" --allow-empty
git push origin HEAD:"${GITHUB_REF_NAME:-$(git branch --show-current)}"
