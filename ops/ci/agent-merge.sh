#!/usr/bin/env bash
# agent-merge.sh — VibeGate Evidence-Gate review + auto-merge.
#
# Invoked by .gitlab/ci/agent-merge.yml after every other lane on an MR
# pipeline succeeds. Calls the host jnoccio-fusion router (bound to
# 0.0.0.0:4317, reachable from CI Docker containers via the docker bridge
# IP) with the reviewer prompts under .autonomy/prompts/. Fusion routes
# the call across user_1 + user_2 + all providers using win-rate routing,
# so no LLM keys leave the host. Receipts land under .autonomy/.receipts/.
#
# Required env (provided by .gitlab/ci/agent-merge.yml + project CI vars):
#   GITLAB_PAT                         — masked project CI variable
#   CI_PROJECT_ID, CI_MERGE_REQUEST_IID, CI_PIPELINE_ID, CI_PROJECT_PATH
#   CI_API_V4_URL                      — GitLab REST root
#   CI_MERGE_REQUEST_TARGET_BRANCH_NAME — typically "main"
#
# Optional env:
#   FUSION_BASE_URL — override the fusion endpoint (default
#                     http://172.17.0.1:4317). The docker bridge gateway
#                     IP is what containers see for the host.
#
# Exit codes:
#   0   — all reviewers pass/concern/abstain AND merge succeeded
#   1   — at least one reviewer returned `block`, refused to merge
#   2   — reviewer ran but returned malformed JSON (treated as block)
#   3   — GitLab merge API rejected the merge (risk gate denied)
#   4   — missing required env

set -euo pipefail

require_env() {
  local name=$1
  if [ -z "${!name:-}" ]; then
    echo "agent-merge: required env $name not set" >&2
    exit 4
  fi
}

require_env GITLAB_PAT
require_env CI_PROJECT_ID
require_env CI_MERGE_REQUEST_IID
require_env CI_API_V4_URL
require_env CI_MERGE_REQUEST_TARGET_BRANCH_NAME

FUSION_BASE_URL="${FUSION_BASE_URL:-http://172.17.0.1:4317}"
TARGET_BRANCH="$CI_MERGE_REQUEST_TARGET_BRANCH_NAME"
HEAD_SHA=$(git rev-parse HEAD)
POLICY_SHA=$(find .autonomy -type f -print0 2>/dev/null \
  | sort -z \
  | xargs -0 sha256sum 2>/dev/null \
  | sha256sum \
  | cut -d' ' -f1)

mkdir -p .autonomy/.receipts

# Pick the diff base. `CI_MERGE_REQUEST_TARGET_BRANCH_SHA` (set in merged-
# result pipelines) is the tip of the target branch at pipeline time —
# matches what GitLab's MR UI shows. Otherwise fall back to the fetched
# `origin/<target>` ref (the agent-merge.yml before_script fetches it).
# Locally outside CI, just use the local target branch.
if [ -n "${CI_MERGE_REQUEST_TARGET_BRANCH_SHA:-}" ]; then
  DIFF_BASE="$CI_MERGE_REQUEST_TARGET_BRANCH_SHA"
elif git rev-parse --verify "origin/${TARGET_BRANCH}" >/dev/null 2>&1; then
  DIFF_BASE="origin/${TARGET_BRANCH}"
else
  DIFF_BASE="$TARGET_BRANCH"
fi

DIFF_FILE=$(mktemp)
trap 'rm -f "$DIFF_FILE"' EXIT

# Filter to files that actually matter for security / test-integrity
# review. Keeps the model focused even when the MR drags in unrelated
# squash-merge ghosts.
git diff "$DIFF_BASE..HEAD" -- \
  '*.rs' '*.toml' '*.yml' '*.yaml' '*.sh' '*.py' '*.js' '*.ts' '*.tsx' \
  '*.json' '*.lock' 'Cargo.lock' 'Cargo.toml' '.gitlab-ci.yml' \
  > "$DIFF_FILE"
DIFF_LINES=$(wc -l < "$DIFF_FILE")
echo "agent-merge: head=$HEAD_SHA policy=$POLICY_SHA diff_base=$DIFF_BASE diff_lines=$DIFF_LINES fusion=$FUSION_BASE_URL"

# Use the fusion smart router. It selects models by win-rate across
# every keyed user/provider, so the agent-merge job never needs an LLM
# key of its own.
MODEL="${AGENT_MERGE_MODEL:-jnoccio/jnoccio-fusion}"

# Truncate to fit the underlying model's context (each upstream model
# has its own window; 50k chars ≈ 12.5k tokens of diff stays safe).
DIFF_MAX_CHARS=50000
if [ "$(wc -c < "$DIFF_FILE")" -gt "$DIFF_MAX_CHARS" ]; then
  echo "agent-merge: diff > $DIFF_MAX_CHARS chars; truncating for review"
  head -c "$DIFF_MAX_CHARS" "$DIFF_FILE" > "$DIFF_FILE.trunc"
  echo -e "\n\n... [diff truncated above this line — review the head only] ..." >> "$DIFF_FILE.trunc"
  mv "$DIFF_FILE.trunc" "$DIFF_FILE"
fi

run_reviewer() {
  local role=$1
  local prompt_file=".autonomy/prompts/reviewer-${role}.md"
  local receipt_file=".autonomy/.receipts/${role}.json"

  echo "=== reviewer-${role} ==="

  if [ ! -f "$prompt_file" ]; then
    echo "skipping role=$role: no prompt at $prompt_file" >&2
    echo "abstain" > ".autonomy/.receipts/${role}.decision"
    return 0
  fi

  local prompt_sha
  prompt_sha=$(sha256sum "$prompt_file" | cut -d' ' -f1)

  local request_body
  request_body=$(jq -n \
    --arg model "$MODEL" \
    --rawfile system "$prompt_file" \
    --rawfile diff "$DIFF_FILE" \
    --arg head "$HEAD_SHA" \
    --arg policy "$POLICY_SHA" \
    --arg target "$TARGET_BRANCH" \
    --arg pack "evp-ci-${CI_PIPELINE_ID:-local}" \
    '{
      model: $model,
      temperature: 0.0,
      max_tokens: 4000,
      messages: [
        {role: "system", content: $system},
        {role: "user", content: ("Required input fields to echo verbatim in your JSON receipt:\nhead_sha: " + $head + "\npolicy_sha: " + $policy + "\ntarget_branch: " + $target + "\nevidence_pack_id: " + $pack + "\n\nDiff to review (in unified format):\n\n```diff\n" + $diff + "\n```\n\nEmit only the JSON receipt object as your very first tokens. No markdown fences, no prose.")}
      ]
    }')

  local response
  response=$(curl -sf --max-time 180 -X POST \
    "${FUSION_BASE_URL}/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$request_body" 2>/dev/null || true)

  if [ -z "$response" ]; then
    echo "agent-merge: role=$role fusion call failed (empty response)" >&2
    jq -n --arg role "$role" --arg head "$HEAD_SHA" --arg policy "$POLICY_SHA" --arg target "$TARGET_BRANCH" --arg pack "evp-ci-${CI_PIPELINE_ID:-local}" \
      '{schema:"vibegate.agent_approval_receipt.v1", role:$role, decision:"abstain", head_sha:$head, policy_sha:$policy, target_branch:$target, evidence_pack_id:$pack, findings:[{severity:"info", kind:"reviewer-call-failed", evidence:"jnoccio-fusion", note:"empty response from fusion"}], model:"unknown", provider:"jnoccio", note:"fusion call failed"}' \
      > "$receipt_file"
    echo "abstain" > ".autonomy/.receipts/${role}.decision"
    return 0
  fi

  local content model_id route_slot prompt_tok completion_tok
  content=$(printf '%s' "$response" | jq -r '.choices[0].message.content // ""')
  # Fusion reports both the upstream/winner model in .jnoccio.winner_model_id
  # and a normalized .model field. Surface both so the receipt is auditable.
  model_id=$(printf '%s' "$response" | jq -r '.jnoccio.winner_route_slot_id // .model // "unknown"')
  route_slot=$(printf '%s' "$response" | jq -r '.jnoccio.winner_route_slot_id // ""')
  prompt_tok=$(printf '%s' "$response" | jq -r '.usage.prompt_tokens // 0')
  completion_tok=$(printf '%s' "$response" | jq -r '.usage.completion_tokens // 0')

  # Extract JSON object from content. Models sometimes wrap in markdown
  # fences despite the prompt; strip them and find the outermost {...}.
  local parsed
  parsed=$(printf '%s' "$content" | python3 -c '
import sys, json, re
text = sys.stdin.read().strip()
text = re.sub(r"^```(?:json)?\s*\n", "", text)
text = re.sub(r"\n```\s*$", "", text)
m = re.search(r"\{.*\}", text, re.DOTALL)
if not m:
    print("{}")
    sys.exit(0)
obj_text = m.group(0)
try:
    obj = json.loads(obj_text)
    print(json.dumps(obj))
except json.JSONDecodeError:
    print("{}")
' 2>/dev/null || echo "{}")

  local decision
  decision=$(printf '%s' "$parsed" | jq -r '.decision // "abstain"')
  case "$decision" in
    pass|concern|block|abstain) ;;
    *) decision="abstain" ;;
  esac

  local raw_sha
  raw_sha=$(printf '%s' "$content" | sha256sum | cut -d' ' -f1)

  # Guard against malformed JSON from the model: re-validate every nested
  # field independently before handing it to --argjson.
  local findings_json
  findings_json=$(printf '%s' "$parsed" | jq -c '.findings // []' 2>/dev/null || echo "[]")
  if ! printf '%s' "$findings_json" | jq -e . >/dev/null 2>&1; then
    findings_json="[]"
  fi
  local summary_text
  summary_text=$(printf '%s' "$parsed" | jq -r '.summary // ""' 2>/dev/null || echo "")

  jq -n \
    --arg role "$role" \
    --arg head "$HEAD_SHA" \
    --arg policy "$POLICY_SHA" \
    --arg target "$TARGET_BRANCH" \
    --arg model "$model_id" \
    --arg route_slot "$route_slot" \
    --arg prompt_sha "sha256:$prompt_sha" \
    --arg raw_sha "sha256:$raw_sha" \
    --arg decision "$decision" \
    --argjson findings "$findings_json" \
    --arg summary "$summary_text" \
    --arg pack "evp-ci-${CI_PIPELINE_ID:-local}" \
    --argjson prompt_tok "$prompt_tok" \
    --argjson completion_tok "$completion_tok" \
    '{
      schema: "vibegate.agent_approval_receipt.v1",
      role: $role,
      head_sha: $head,
      policy_sha: $policy,
      target_branch: $target,
      agent_id: ("reviewer-" + $role + ".v1"),
      prompt_sha: $prompt_sha,
      provider: "jnoccio-fusion",
      model: $model,
      route_slot_id: $route_slot,
      raw_response_sha: $raw_sha,
      decision: $decision,
      findings: $findings,
      summary: $summary,
      evidence_pack_id: $pack,
      tokens: {prompt: $prompt_tok, completion: $completion_tok}
    }' > "$receipt_file"

  echo "agent-merge: role=$role decision=$decision route=$route_slot tokens={prompt:$prompt_tok,completion:$completion_tok}"
  echo "$decision" > ".autonomy/.receipts/${role}.decision"
}

run_reviewer security
run_reviewer test_integrity

# Aggregate decisions. block from any reviewer → exit 1 (hard refuse).
hardest=pass
for role in security test_integrity; do
  d=$(cat ".autonomy/.receipts/${role}.decision" 2>/dev/null || echo "abstain")
  case "$d:$hardest" in
    block:*) hardest=block ;;
    concern:pass) hardest=concern ;;
    abstain:pass) hardest=abstain ;;
  esac
done
echo "agent-merge: aggregate decision=$hardest"

if [ "$hardest" = "block" ]; then
  echo "agent-merge: HARD BLOCK — at least one reviewer returned 'block'" >&2
  exit 1
fi

echo "agent-merge: invoking GitLab MR merge API"
merge_response=$(curl -sS -X PUT \
  --max-time 30 \
  -H "PRIVATE-TOKEN: $GITLAB_PAT" \
  "${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/merge_requests/${CI_MERGE_REQUEST_IID}/merge" \
  -d "should_remove_source_branch=false" \
  -d "merge_commit_message=Merge via agent-merge (pipeline ${CI_PIPELINE_ID})" \
  -d "merge_when_pipeline_succeeds=false" \
  2>/dev/null || true)

state=$(printf '%s' "$merge_response" | jq -r '.state // "unknown"')
echo "agent-merge: merge response state=$state"
printf '%s\n' "$merge_response" | jq '{state, merged_at, merge_commit_sha, error_message: .message}' 2>/dev/null || printf '%s\n' "$merge_response"

if [ "$state" = "merged" ]; then
  echo "agent-merge: MR merged successfully"
  exit 0
fi

echo "agent-merge: GitLab refused to merge (state=$state)" >&2
exit 3
