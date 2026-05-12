#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

sudo apt-get update
sudo apt-get install -y pacman-package-manager
mkdir -p "$RUNNER_TEMP"
printf '%s\n' "${AUR_KEY}" > "$RUNNER_TEMP/aur-key"
chmod 600 "$RUNNER_TEMP/aur-key"
git config --global user.email "jekko@sst.dev"
git config --global user.name "jekko"
ssh-keyscan -H aur.archlinux.org >> "$RUNNER_TEMP/known_hosts" || true
