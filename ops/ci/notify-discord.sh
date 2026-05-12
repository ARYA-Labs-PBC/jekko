#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

: "${DISCORD_WEBHOOK:?DISCORD_WEBHOOK must be set}"
release_name="$(jq -r '.release.name // .release.tag_name // "release"' "$GITHUB_EVENT_PATH")"
release_url="$(jq -r '.release.html_url // ""' "$GITHUB_EVENT_PATH")"
tag_name="$(jq -r '.release.tag_name // ""' "$GITHUB_EVENT_PATH")"

payload="$(printf '{"content":"Published %s (%s)%s"}' "$release_name" "$tag_name" "${release_url:+ - $release_url}")"
curl -fsSL -H 'Content-Type: application/json' -d "$payload" "$DISCORD_WEBHOOK" >/dev/null
