#!/usr/bin/env bash
# tools/sandbox-wrap.sh — forwarder used by lane.command entries.
# Usage: sandbox-wrap.sh --lane <name> [--run-id <id>] -- <cmd...>
# Looks up an existing run for the lane (creating one if absent) and executes
# the trailing command via `sandboxctl run`. Permission gating + meta capture
# are handled by sandboxctl itself.

set -euo pipefail

LANE=""
RUN_ID=""
ARGV=()
SEEN_DOUBLEDASH=0
while [ $# -gt 0 ]; do
    if [ "$SEEN_DOUBLEDASH" -eq 1 ]; then
        ARGV+=("$1")
        shift
        continue
    fi
    case "$1" in
        --lane)
            LANE="$2"; shift 2 ;;
        --run-id)
            RUN_ID="$2"; shift 2 ;;
        --)
            SEEN_DOUBLEDASH=1; shift ;;
        *)
            echo "sandbox-wrap: unknown arg '$1'" >&2; exit 64 ;;
    esac
done

if [ -z "$LANE" ]; then
    echo "sandbox-wrap: --lane required" >&2
    exit 64
fi
if [ "${#ARGV[@]}" -eq 0 ]; then
    echo "sandbox-wrap: command argv required after --" >&2
    exit 64
fi

if ! command -v sandboxctl >/dev/null 2>&1; then
    echo "sandbox-wrap: sandboxctl not on PATH; build with 'just sandboxctl-build'" >&2
    exit 127
fi

if [ -z "$RUN_ID" ]; then
    RUN_ID=$(sandboxctl create "$LANE" --json | sed -n 's/.*"run_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)
fi
if [ -z "$RUN_ID" ]; then
    echo "sandbox-wrap: failed to obtain run-id" >&2
    exit 70
fi

exec sandboxctl run "$RUN_ID" -- "${ARGV[@]}"
