#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
import json, os, subprocess, sys, time

repo = os.environ["GITHUB_REPOSITORY"]
owner, name = repo.split("/", 1)

items = json.loads(subprocess.check_output([
  "gh", "api", f"/repos/{repo}/issues", "-f", "labels=needs:compliance", "-f", "state=open"
], text=True))

if not items:
  print("No open issues/PRs with needs:compliance label")
  sys.exit(0)

two_hours = 2 * 60 * 60
now = time.time()
for item in items:
  issue_number = item["number"]
  comments = json.loads(subprocess.check_output([
    "gh", "api", f"/repos/{repo}/issues/{issue_number}/comments"
  ], text=True))
  compliance_comment = next((c for c in comments if "<!-- issue-compliance -->" in c["body"]), None)
  if not compliance_comment:
    continue
  age = now - __import__("datetime").datetime.fromisoformat(compliance_comment["created_at"].replace("Z", "+00:00")).timestamp()
  if age < two_hours:
    continue
  kind = "PR" if item.get("pull_request") else "issue"
  close_message = (
    "This pull request has been automatically closed because it was not updated to meet our [contributing guidelines](../blob/dev/CONTRIBUTING.md) within the 2-hour window.\n\nFeel free to open a new pull request that follows our guidelines."
    if item.get("pull_request")
    else "This issue has been automatically closed because it was not updated to meet our [contributing guidelines](../blob/dev/CONTRIBUTING.md) within the 2-hour window.\n\nFeel free to open a new issue that follows our issue templates."
  )
  subprocess.check_call(["gh", "api", "--method", "POST", f"/repos/{repo}/issues/{issue_number}/comments", "-f", f"body={close_message}"])
  subprocess.run(["gh", "api", "--method", "DELETE", f"/repos/{repo}/issues/{issue_number}/labels/needs:compliance"], check=False)
  if item.get("pull_request"):
    subprocess.check_call(["gh", "api", "--method", "PATCH", f"/repos/{repo}/pulls/{issue_number}", "-f", "state=closed"])
  else:
    subprocess.check_call(["gh", "api", "--method", "PATCH", f"/repos/{repo}/issues/{issue_number}", "-f", "state=closed", "-f", "state_reason=not_planned"])
  print(f"Closed non-compliant {kind} #{issue_number} after 2-hour window")
PY
