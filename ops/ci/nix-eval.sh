#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

set -euo pipefail
nix --version
echo "=== Flake metadata ==="
nix flake metadata
echo
echo "=== Flake structure ==="
nix flake show --all-systems
SYSTEMS="x86_64-linux aarch64-linux x86_64-darwin aarch64-darwin"
PACKAGES="jekko"
echo
echo "=== Evaluating packages for all systems ==="
for system in $SYSTEMS; do
  echo
  echo "--- $system ---"
  for pkg in $PACKAGES; do
    printf "  %s: " "$pkg"
    if output=$(nix eval ".#packages.$system.$pkg.drvPath" --raw 2>&1); then
      echo "✓"
    else
      echo "✗"
      echo "::error::Evaluation failed for packages.$system.$pkg"
      echo "$output"
      exit 1
    fi
  done
done
echo
echo "=== Evaluating devShells for all systems ==="
for system in $SYSTEMS; do
  printf "%s: " "$system"
  if output=$(nix eval ".#devShells.$system.default.drvPath" --raw 2>&1); then
    echo "✓"
  else
    echo "✗"
    echo "::error::Evaluation failed for devShells.$system.default"
    echo "$output"
    exit 1
  fi
done
echo
echo "=== All evaluations passed ==="
