#!/usr/bin/env bash
# Host-side auto-deploy for jekko.
#
# Polls jeryu (the local GitLab) for the latest main pipeline status. When
# main advances AND the head pipeline has succeeded, fetches main into a
# DEDICATED deploy clone (not the user's working dir), rebuilds the jekko
# CLI in release mode there, and installs the binary to ~/.local/bin/jekko.
#
# Important: the script never touches the user's working repo at
# ~/jekko — it operates on a separate clone at ~/.jekko/deploy-repo so a
# `git fetch` here cannot wipe in-progress changes.
#
# Designed to run as a systemd user timer (see ops/host-deploy.service +
# ops/host-deploy.timer) or under tmux/screen as a polling loop.
#
# State file: ~/.jekko/host-deploy/last-deployed-sha
# Logs:        ~/.jekko/host-deploy/deploy.log

set -euo pipefail

JERYU_BASE="${JERYU_BASE:-http://localhost:8929}"
JERYU_PROJECT_ID="${JERYU_PROJECT_ID:-148}"
JERYU_USER="${JERYU_USER:-root}"
JERYU_PASS="${JERYU_PASS:-rvpjShN9cMhQVrCGzJCmWDU4}"

DEPLOY_REPO="${JEKKO_DEPLOY_REPO:-$HOME/.jekko/deploy-repo}"
INSTALL_ROOT="${JEKKO_INSTALL_ROOT:-$HOME/.local}"
STATE_DIR="${HOME}/.jekko/host-deploy"
STATE_FILE="${STATE_DIR}/last-deployed-sha"
LOG_FILE="${STATE_DIR}/deploy.log"

mkdir -p "${STATE_DIR}" "${DEPLOY_REPO%/*}"

log() {
  local msg="$(date -u +%Y-%m-%dT%H:%M:%SZ) $*"
  echo "${msg}" | tee -a "${LOG_FILE}"
}

get_token() {
  curl -sf -X POST "${JERYU_BASE}/oauth/token" \
    -d "grant_type=password&username=${JERYU_USER}&password=${JERYU_PASS}" \
    | jq -r '.access_token'
}

main_sha() {
  local token="$1"
  curl -sf -H "Authorization: Bearer ${token}" \
    "${JERYU_BASE}/api/v4/projects/${JERYU_PROJECT_ID}/repository/commits/main" \
    | jq -r '.id'
}

# Returns "success", "failed", "running", or "" if no pipeline exists.
main_pipeline_status() {
  local token="$1"
  local sha="$2"
  curl -sf -H "Authorization: Bearer ${token}" \
    "${JERYU_BASE}/api/v4/projects/${JERYU_PROJECT_ID}/pipelines?ref=main&sha=${sha}&per_page=1" \
    | jq -r '.[0].status // ""'
}

ensure_deploy_clone() {
  local token="$1"
  if [ ! -d "${DEPLOY_REPO}/.git" ]; then
    log "initial clone to ${DEPLOY_REPO}"
    # GitLab git-over-HTTP accepts HTTP Basic auth (oauth2:<token>) but
    # rejects `Authorization: Bearer`. Pass the credential as a Basic
    # header so it never lands in the URL or .git/config (jankurai HLT-035
    # `git.remote.credential-url` detector only flags inline token URLs).
    local basic
    basic="$(printf 'oauth2:%s' "${token}" | base64 -w0)"
    git -c "http.extraheader=Authorization: Basic ${basic}" \
      clone "${JERYU_BASE}/root/jekko.git" "${DEPLOY_REPO}" 2>&1 | tee -a "${LOG_FILE}"
  fi
}

deploy_one() {
  local token
  token="$(get_token)" || { log "ERROR: cannot get jeryu token"; return 1; }

  local current_sha
  current_sha="$(main_sha "${token}")" || { log "ERROR: cannot read main SHA"; return 1; }

  local last_sha=""
  [ -f "${STATE_FILE}" ] && last_sha="$(cat "${STATE_FILE}")"

  if [ "${current_sha}" = "${last_sha}" ]; then
    return 0  # already deployed
  fi

  local pipe_status
  pipe_status="$(main_pipeline_status "${token}" "${current_sha}")"

  # Post-merge main pipelines typically end in status `manual` (not `success`)
  # because the deploy stage's `jankurai:audit` + `jankurai:sandbox-backends`
  # are intentionally `when: manual` opt-in lanes — they don't block the
  # required-pass invariants of the static + test stages. Treat `manual` as
  # deployable: it means the required jobs all succeeded and only optional
  # manual jobs are unplayed.
  case "${pipe_status}" in
    success|manual)
      log "main advanced to ${current_sha:0:8} with pipeline=${pipe_status} — deploying"
      ;;
    "")
      log "main at ${current_sha:0:8} has no pipeline yet — skipping"
      return 0
      ;;
    running|pending|created)
      log "main at ${current_sha:0:8} pipeline=${pipe_status} — waiting"
      return 0
      ;;
    failed|canceled|skipped)
      log "main at ${current_sha:0:8} pipeline=${pipe_status} — not deploying"
      return 0
      ;;
    *)
      log "main at ${current_sha:0:8} pipeline=${pipe_status} (unknown) — skipping"
      return 0
      ;;
  esac

  ensure_deploy_clone "${token}"

  cd "${DEPLOY_REPO}"
  log "git fetch + checkout in ${DEPLOY_REPO}"
  # Operates only on the dedicated deploy clone — never touches ~/jekko.
  # Basic auth header keeps the token out of URL + .git/config.
  local basic
  basic="$(printf 'oauth2:%s' "${token}" | base64 -w0)"
  git -c "http.extraheader=Authorization: Basic ${basic}" \
    fetch origin main 2>&1 | tee -a "${LOG_FILE}"
  git checkout --force "${current_sha}" 2>&1 | tee -a "${LOG_FILE}"

  log "cargo install --path crates/jekko-cli --root ${INSTALL_ROOT} --locked --force"
  if ! cargo install --path crates/jekko-cli --root "${INSTALL_ROOT}" --locked --force 2>&1 | tee -a "${LOG_FILE}"; then
    log "ERROR: cargo install failed"
    return 1
  fi

  log "installed: $("${INSTALL_ROOT}/bin/jekko" --version 2>&1 | head -1)"
  echo "${current_sha}" > "${STATE_FILE}"
  log "host-deploy success: sha=${current_sha:0:8}"
}

case "${1:-once}" in
  once)
    deploy_one
    ;;
  loop)
    log "host-deploy starting polling loop (interval=${POLL_INTERVAL:-120}s)"
    while true; do
      deploy_one || log "deploy_one returned non-zero (continuing)"
      sleep "${POLL_INTERVAL:-120}"
    done
    ;;
  *)
    echo "usage: $0 {once|loop}" >&2
    exit 2
    ;;
esac
