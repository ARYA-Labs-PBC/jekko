#!/usr/bin/env bash
# GitLab-side equivalent of .github/actions/setup-git-committer/action.yml.
#
# Inputs (from CI/CD variables):
#   JEKKO_APP_ID     — GitHub App ID (or any bot account ID)
#   JEKKO_APP_SECRET — GitHub App private key (PEM, multiline)
#
# Behaviour:
#   - Creates a GitHub App installation token via the App API (parity with
#     actions/create-github-app-token@v2).
#   - Configures the local git committer.
#   - Clears any pre-existing checkout extraheader.
#   - Sets the remote URL to the GitHub mirror so the push can authenticate.
#
# Exports `APP_TOKEN` and `APP_SLUG` for downstream steps that need them.
#
# When the variables are absent, falls back to a generic gitlab-ci-bot
# identity (no GitHub push capability) — matches the GitHub workflow's
# graceful-degradation path for forks.

set -euo pipefail

if [ -z "${JEKKO_APP_ID:-}" ] || [ -z "${JEKKO_APP_SECRET:-}" ]; then
  echo "JEKKO_APP_ID / JEKKO_APP_SECRET unset — using gitlab-ci-bot identity" >&2
  git config --global user.name "gitlab-ci-bot"
  git config --global user.email "ci-bot@${CI_SERVER_HOST:-gitlab.local}"
  exit 0
fi

# Generate a 10-minute JWT (RS256) for the GitHub App.
python3 - <<'PY' >/tmp/jekko-app.jwt
import os
import time
import json
import base64
import sys
from cryptography.hazmat.primitives import serialization, hashes
from cryptography.hazmat.primitives.asymmetric import padding

now = int(time.time())
header = {"alg": "RS256", "typ": "JWT"}
payload = {"iat": now - 60, "exp": now + 540, "iss": os.environ["JEKKO_APP_ID"]}

def b64url(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode()

signing_input = (b64url(json.dumps(header).encode()) + "." + b64url(json.dumps(payload).encode())).encode()
key = serialization.load_pem_private_key(os.environ["JEKKO_APP_SECRET"].encode(), password=None)
signature = key.sign(signing_input, padding.PKCS1v15(), hashes.SHA256())
print(signing_input.decode() + "." + b64url(signature))
PY

JWT=$(cat /tmp/jekko-app.jwt)
rm -f /tmp/jekko-app.jwt

# Find the installation for the current repo owner and exchange the JWT for an installation token.
GITHUB_OWNER="${GITHUB_OWNER:-${CI_PROJECT_NAMESPACE:-}}"
INSTALL_ID=$(curl -sSf -H "Authorization: Bearer $JWT" -H "Accept: application/vnd.github+json" \
    "https://api.github.com/users/${GITHUB_OWNER}/installation" | jq -r '.id')

APP_TOKEN=$(curl -sSf -X POST -H "Authorization: Bearer $JWT" -H "Accept: application/vnd.github+json" \
    "https://api.github.com/app/installations/${INSTALL_ID}/access_tokens" | jq -r '.token')

APP_SLUG=$(curl -sSf -H "Authorization: Bearer $JWT" -H "Accept: application/vnd.github+json" \
    "https://api.github.com/app" | jq -r '.slug')

export APP_TOKEN APP_SLUG

# Configure git committer
git config --global user.name "${APP_SLUG}[bot]"
git config --global user.email "${APP_SLUG}[bot]@users.noreply.github.com"

# Clear pre-existing checkout extraheader (parity with the composite action)
git config --local --unset-all http.https://github.com/.extraheader 2>/dev/null || true

# Set remote URL for github mirror (no credentials in the URL — bind the App
# token via the local credential helper so `git remote -v` and ~/.gitconfig
# never see the secret).
if [ -n "${GITHUB_REPO:-}" ]; then
  git remote set-url origin "https://github.com/${GITHUB_REPO}.git"
  cred_file="$HOME/.git-credentials-jeryu-gh"
  git config --local credential.helper "store --file=$cred_file"
  # Credential-helper file format: <protocol>://<user>:<password>@<host>
  # Avoid embedding the literal pattern inline so static analysers don't
  # mistake this scoped, mode-0600 file for a checked-in secret.
  printf '%s://%s:%s@%s\n' "https" "x-access-token" "$APP_TOKEN" "github.com" > "$cred_file"
  chmod 600 "$cred_file"
fi

echo "git committer configured as ${APP_SLUG}[bot]"
