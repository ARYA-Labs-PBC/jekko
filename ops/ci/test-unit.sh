#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

git config --global user.email "bot@jekko.ai"
git config --global user.name "jekko"
export JEKKO_EXPERIMENTAL_DISABLE_FILEWATCHER="${JEKKO_EXPERIMENTAL_DISABLE_FILEWATCHER:-false}"
bun turbo test:ci
