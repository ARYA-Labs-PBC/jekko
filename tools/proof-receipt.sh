#!/usr/bin/env bash
# Canonical proof-receipt emitter.
#
# Usage: tools/proof-receipt.sh <lane> <command...>
#
# Runs `<command...>`, captures exit code + elapsed time + git head, and writes
# `target/jankurai/receipts/<lane>-<unix>.json` matching the canonical
# proof-receipt schema (see jankurai/schemas/proof-receipt.schema.json).
#
# Streams the command's stdout/stderr to stdout/stderr unchanged. Always exits
# with the underlying command's exit code. The receipt is written even on
# failure so jankurai audit can still consume it.

set -uo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: tools/proof-receipt.sh <lane> <command...>" >&2
  exit 64
fi

lane="$1"
shift

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
unix_now="$(date -u +%s)"
iso_now="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
git_head="$(git -C "$repo_root" rev-parse HEAD 2>/dev/null || echo "unknown")"
receipts_dir="$repo_root/target/jankurai/receipts"
mkdir -p "$receipts_dir"
log_path="$receipts_dir/${lane}-${unix_now}.log"
receipt_path="$receipts_dir/${lane}-${unix_now}.json"

start_ms="$(python3 -c 'import time; print(int(time.time()*1000))')"

# Capture both stdout + stderr to log; let user see them too.
set +e
"$@" 2>&1 | tee "$log_path"
exit_code=${PIPESTATUS[0]}
set -e

end_ms="$(python3 -c 'import time; print(int(time.time()*1000))')"
elapsed_ms=$((end_ms - start_ms))

command_str="$(printf '%q ' "$@")"

# Emit canonical receipt JSON.
python3 - "$receipt_path" "$lane" "$command_str" "$exit_code" "$elapsed_ms" "$git_head" "$iso_now" "$log_path" <<'PY'
import hashlib, json, os, sys
receipt_path, lane, command, exit_code, elapsed_ms, git_head, iso_now, log_path = sys.argv[1:9]

def digest(path):
    if not os.path.isfile(path):
        return None
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return f"sha256:{h.hexdigest()}"

receipt = {
    "schema_version": "1.0.0",
    "lane": lane,
    "command": command.strip(),
    "exit_code": int(exit_code),
    "elapsed_ms": int(elapsed_ms),
    "git_head": git_head,
    "timestamp_utc": iso_now,
    "log_path": os.path.relpath(log_path),
    "log_digest": digest(log_path),
    "receipt_path": os.path.relpath(receipt_path),
    "evidence_kind": "command-output",
}
with open(receipt_path, "w") as fh:
    json.dump(receipt, fh, indent=2)
    fh.write("\n")
PY

echo "proof-receipt: wrote $receipt_path (exit=$exit_code, elapsed_ms=$elapsed_ms)" >&2
exit "$exit_code"
