#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if [ "${GITHUB_REPOSITORY:-}" != "neverhuman/jekko" ]; then
  exit 0
fi
if [ "${JEKKO_AUTOMATION_ENABLED:-0}" != "1" ]; then
  exit 0
fi
if [ "${GITHUB_EVENT_NAME:-}" != "issues" ]; then
  exit 0
fi
if [ "${GITHUB_EVENT_ACTION:-}" != "opened" ]; then
  exit 0
fi

bun i -g jekko-ai

issue_number="$(jq -r '.issue.number' "$GITHUB_EVENT_PATH")"
cat > /tmp/jekko-duplicate-issues-prompt.txt <<EOF
A new issue has been created:

Issue number: ${issue_number}

Lookup this issue with gh issue view ${issue_number}.

You have TWO tasks. Perform both, then post a SINGLE comment (if needed).

---

TASK 1: CONTRIBUTING GUIDELINES COMPLIANCE CHECK

Check whether the issue follows our contributing guidelines and issue templates.

This project has three issue templates that every issue MUST use one of:

1. Bug Report - requires a Description field with real content
2. Feature Request - requires a verification checkbox and description, title should start with [FEATURE]:
3. Question - requires the Question field with real content

Additionally check:
- No AI-generated walls of text (long, AI-generated descriptions are not acceptable)
- The issue has real content, not just template placeholder text left unchanged
- Bug reports should include some context about how to reproduce
- Feature requests should explain the problem or need
- We want to push for having the user provide system description & information

Do NOT be nitpicky about optional fields. Only flag real problems like: no template used, required fields empty or placeholder text only, obviously AI-generated walls of text, or completely empty/nonsensical content.

---

TASK 2: DUPLICATE CHECK

Search through existing issues (excluding #${issue_number}) to find potential duplicates.
Consider:
1. Similar titles or descriptions
2. Same error messages or symptoms
3. Related functionality or components
4. Similar feature requests

Additionally, if the issue mentions keybinds, keyboard shortcuts, or key bindings, note the pinned keybinds issue #4997.

---

POSTING YOUR COMMENT:

Based on your findings, post a SINGLE comment on issue #${issue_number}. Build the comment as follows:

If the issue is NOT compliant, start the comment with:
<!-- issue-compliance -->
Then explain what needs to be fixed and that they have 2 hours to edit the issue before it is automatically closed. Also add the label needs:compliance to the issue using: gh issue edit ${issue_number} --add-label needs:compliance

If duplicates were found, include a section about potential duplicates with links.

If the issue mentions keybinds/keyboard shortcuts, include a note about #4997.

If the issue IS compliant AND no duplicates were found, do not comment.
EOF

jekko run -m jekko/claude-sonnet-4-6 --prompt-file /tmp/jekko-duplicate-issues-prompt.txt
