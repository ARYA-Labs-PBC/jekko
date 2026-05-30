#!/usr/bin/env bash
# Shared fast CI gate. Used by pre-push and the local CI runner.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

step "just fast"
just fast
