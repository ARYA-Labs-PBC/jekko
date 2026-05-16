#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if [[ -n "${JEKKO_CHANNEL:-}" ]]; then
  CHANNEL="$JEKKO_CHANNEL"
elif [[ -n "${JEKKO_BUMP:-}" ]]; then
  CHANNEL="latest"
elif [[ -n "${JEKKO_VERSION:-}" && "${JEKKO_VERSION}" != 0.0.0-* ]]; then
  CHANNEL="latest"
else
  CHANNEL="$(git branch --show-current)"
  if [[ -z "$CHANNEL" ]]; then
    CHANNEL="latest"
  fi
fi

rtk cargo run -p xtask -- publish-release-packages --dist-root ./dist --tag "$CHANNEL"

if [[ "$CHANNEL" == "latest" ]]; then
  if [[ -z "${JEKKO_VERSION:-}" ]]; then
    echo "JEKKO_VERSION is required for release publication" >&2
    exit 1
  fi
  rtk cargo run -p xtask -- publish-release-artifacts --version "$JEKKO_VERSION" --channel "$CHANNEL"
fi
