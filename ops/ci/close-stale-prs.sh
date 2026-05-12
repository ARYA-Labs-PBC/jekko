#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
import json, os, subprocess, sys, time
from datetime import datetime, timezone

repo = os.environ["GITHUB_REPOSITORY"]
owner, name = repo.split("/", 1)
event = json.load(open(os.environ["GITHUB_EVENT_PATH"]))
dry_run = event.get("inputs", {}).get("dryRun") == "true"

def gh(*args):
    return subprocess.check_output(["gh", *args], text=True)

cutoff = datetime.now(timezone.utc).timestamp() - 60 * 24 * 60 * 60
query = """
query($owner: String!, $repo: String!, $cursor: String) {
  repository(owner: $owner, name: $repo) {
    pullRequests(first: 100, states: OPEN, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number
        title
        author { login }
        createdAt
        commits(last: 1) { nodes { commit { committedDate } } }
        comments(last: 1) { nodes { createdAt } }
        reviews(last: 1) { nodes { createdAt } }
      }
    }
  }
}
"""

all_prs = []
cursor = None
while True:
  out = gh("api", "graphql", "-f", f"owner={owner}", "-f", f"repo={name}", "-F", f"cursor={cursor or ''}", "-f", f"query={query}", "--jq", ".data.repository.pullRequests")
  data = json.loads(out)
  all_prs.extend(data["nodes"])
  if not data["pageInfo"]["hasNextPage"]:
    break
  cursor = data["pageInfo"]["endCursor"]

def last_activity(pr):
  dates = [pr["createdAt"]]
  if pr["commits"]["nodes"]:
    dates.append(pr["commits"]["nodes"][0]["commit"]["committedDate"])
  if pr["comments"]["nodes"]:
    dates.append(pr["comments"]["nodes"][0]["createdAt"])
  if pr["reviews"]["nodes"]:
    dates.append(pr["reviews"]["nodes"][0]["createdAt"])
  return max(dates)

stale = []
for pr in all_prs:
  ts = datetime.fromisoformat(last_activity(pr).replace("Z", "+00:00")).timestamp()
  if ts <= cutoff:
    stale.append(pr)

if not stale:
  print("No stale pull requests found.")
  sys.exit(0)

for pr in stale:
  n = pr["number"]
  if dry_run:
    print(f"[dry-run] Would close PR #{n} from {pr['author']['login'] if pr.get('author') else 'unknown'}: {pr['title']}")
    continue
  subprocess.check_call(["gh", "pr", "comment", str(n), "--body", f"Closing this pull request because it has had no updates for more than 60 days. If you plan to continue working on it, feel free to reopen or open a new PR."])
  subprocess.check_call(["gh", "pr", "close", str(n)])

print(f"Processed {len(stale)} stale pull requests.")
PY
