#!/usr/bin/env bash
set -euo pipefail

# ── git-crypt encryption verifier ────────────────────────────────
# Ensures every committed blob under jnoccio-fusion/ (minus
# documented exceptions) starts with the \0GITCRYPT\0 magic header.
#
# This script does NOT need the decryption key.
# It runs in CI as a required status check and can be run locally.
#
# When git-crypt is unlocked locally, files appear as plaintext in
# the working tree — that is expected. Use --force to check anyway,
# or use --index to verify staged/index blobs without locking.
# ─────────────────────────────────────────────────────────────────

MODE="worktree"
FORCE=0

usage() {
  cat <<'USAGE'
Usage: tools/check-encrypted-paths.sh [--force] [--index] [--head]

  --force   Check the working tree even when git-crypt appears unlocked.
  --index   Check staged/index blobs. Use this while locally unlocked.
  --head    Check committed HEAD blobs.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force)
      FORCE=1
      ;;
    --index)
      MODE="index"
      ;;
    --head)
      MODE="head"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

# Hex representation of the git-crypt magic header: \0GITCRYPT\0
MAGIC_HEX="00474954435259505400"

# Files explicitly excluded from encryption (must match .gitattributes exceptions)
EXCEPTIONS=(
  "jnoccio-fusion/README.md"
  "jnoccio-fusion/.gitignore"
  "jnoccio-fusion/.env.jnoccio.example"
  "jnoccio-fusion/KEYS.md"
  "jnoccio-fusion/ENCRYPTION.md"
  "jnoccio-fusion/VERSION.md"
  "jnoccio-fusion/CHANGELOG.md"
)

is_exception() {
  local path="$1"
  for exc in "${EXCEPTIONS[@]}"; do
    [[ "$path" == "$exc" ]] && return 0
  done
  return 1
}

tracked_protected_files() {
  git ls-files 'jnoccio-fusion/' | while IFS= read -r file; do
    is_exception "$file" && continue
    printf '%s\n' "$file"
  done
}

worktree_hex() {
  local file="$1"
  head -c 10 "$file" | od -An -tx1 -v | tr -d ' \n'
}

blob_hex() {
  local spec="$1"
  local tmp
  tmp=$(mktemp)
  git cat-file blob "$spec" >"$tmp"
  head -c 10 "$tmp" | od -An -tx1 -v | tr -d ' \n'
  rm -f "$tmp"
}

blob_exists() {
  local spec="$1"
  git cat-file -e "$spec" 2>/dev/null
}

verify_env_not_tracked() {
  if git ls-files --error-unmatch 'jnoccio-fusion/.env.jnoccio' >/dev/null 2>&1; then
    echo "❌ jnoccio-fusion/.env.jnoccio is tracked. It must remain local-only." >&2
    ERRORS=$((ERRORS + 1))
  fi
}

verify_worktree_not_unlocked_unless_forced() {
  [[ "$MODE" == "worktree" ]] || return 0
  [[ "$FORCE" -eq 0 ]] || return 0
  command -v git-crypt &>/dev/null || return 0

  local sample_file=""
  while IFS= read -r file; do
    [[ -f "$file" ]] || continue
    sample_file="$file"
    break
  done < <(tracked_protected_files)
  [[ -n "$sample_file" ]] || return 0

  local sample_hex
  sample_hex=$(worktree_hex "$sample_file")
  if [[ "$sample_hex" != "$MAGIC_HEX" ]]; then
    echo "ℹ️  git-crypt is unlocked locally. Protected files are plaintext in the working tree (expected)."
    echo "   Run with --index to verify staged/index blobs, or --force to inspect the working tree anyway."
    echo "   CI will verify ciphertext where the key is absent."
    exit 0
  fi
}

ERRORS=0
CHECKED=0
verify_worktree_not_unlocked_unless_forced
verify_env_not_tracked

# Check every tracked protected file under jnoccio-fusion/
while IFS= read -r file; do
  hex=""
  case "$MODE" in
    worktree)
      [[ -f "$file" ]] || continue
      hex=$(worktree_hex "$file")
      ;;
    index)
      blob_exists ":$file" || continue
      hex=$(blob_hex ":$file")
      ;;
    head)
      blob_exists "HEAD:$file" || continue
      hex=$(blob_hex "HEAD:$file")
      ;;
  esac

  CHECKED=$((CHECKED + 1))

  if [[ "$hex" != "$MAGIC_HEX" ]]; then
    echo "❌ PLAINTEXT DETECTED: $file ($MODE)" >&2
    echo "   Protected files must be committed as git-crypt ciphertext." >&2
    echo "   Fix: git-crypt unlock, then:" >&2
    echo "     git rm --cached '$file'" >&2
    echo "     git add '$file'" >&2
    echo "     git commit --amend" >&2
    echo "" >&2
    ERRORS=$((ERRORS + 1))
  fi
done < <(tracked_protected_files)

# Verify .gitattributes still contains the git-crypt rule
if ! grep -q 'jnoccio-fusion/\*\* filter=git-crypt' .gitattributes 2>/dev/null; then
  echo "❌ .gitattributes is missing the git-crypt filter rule for jnoccio-fusion/**" >&2
  ERRORS=$((ERRORS + 1))
fi

# Verify .gitattributes has not had the diff=git-crypt rule stripped
if ! grep -q 'jnoccio-fusion/\*\* .*diff=git-crypt' .gitattributes 2>/dev/null; then
  echo "❌ .gitattributes is missing the diff=git-crypt rule for jnoccio-fusion/**" >&2
  ERRORS=$((ERRORS + 1))
fi


if [[ "$ERRORS" -gt 0 ]]; then
  echo "" >&2
  echo "🚫 $ERRORS problem(s) found during encryption verification." >&2
  echo "   Protected paths must be git-crypt ciphertext before merging." >&2
  exit 1
fi

echo "✅ All $CHECKED protected jnoccio-fusion file(s) are encrypted in $MODE mode."
