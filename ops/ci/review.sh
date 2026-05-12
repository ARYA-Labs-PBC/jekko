#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PR_NUMBER="$(jq -r '.issue.number' "$GITHUB_EVENT_PATH")"
if [ -z "$PR_NUMBER" ] || [ "$PR_NUMBER" = "null" ]; then
  PR_NUMBER="$(jq -r '.pull_request.number' "$GITHUB_EVENT_PATH")"
fi

bun i -g jekko-ai

gh api "/repos/${GITHUB_REPOSITORY}/pulls/${PR_NUMBER}" > pr_data.json
PR_TITLE="$(jq -r .title pr_data.json)"
PR_BODY="$(jq -r .body pr_data.json)"
PR_SHA="$(jq -r .head.sha pr_data.json)"

export PR_TITLE PR_BODY
jekko run -m jekko/gpt-5.5 --variant medium "A new pull request has been created: '${PR_TITLE}'

<pr-number>
${PR_NUMBER}
</pr-number>

<pr-description>
${PR_BODY}
</pr-description>

Please check all the code changes in this pull request against the style guide, also look for any bugs if they exist. Diffs are important but make sure you read the entire file to get proper context. Make it clear the suggestions are merely suggestions and the human can decide what to do

When critiquing code against the style guide, be sure that the code is ACTUALLY in violation, don't complain about else statements if they already use early returns there. You may complain about excessive nesting though, regardless of else statement usage.
When critiquing code style don't be a zealot, we don't like \"let\" statements but sometimes they are the simplest option, if someone does a bunch of nesting with let, they should consider using iife (see packages/jekko/src/util.iife.ts)

Use the gh cli to create comments on the files for the violations. Try to leave the comment on the exact line number. If you have a suggested fix include it in a suggestion code block.
If you are writing suggested fixes, BE SURE THAT the change you are recommending is actually valid typescript, often I have seen missing closing \"}\" or other syntax errors.
Generally, write a comment instead of writing suggested change if you can help it.

Command MUST be like this.
gh api \
  --method POST \
  -H \"Accept: application/vnd.github+json\" \
  -H \"X-GitHub-Api-Version: 2022-11-28\" \
  /repos/${GITHUB_REPOSITORY}/pulls/${PR_NUMBER}/comments \
  -f 'body=[summary of issue]' -f 'commit_id=${PR_SHA}' -f 'path=[path-to-file]' -F \"line=[line]\" -f 'side=RIGHT'

Only create comments for actual violations. If the code follows all guidelines, post a structured review receipt comment to the issue using a reproducible command.
Replay command:
gh api --method POST -H \"Accept: application/vnd.github+json\" -H \"X-GitHub-Api-Version: 2022-11-28\" /repos/${GITHUB_REPOSITORY}/issues/${PR_NUMBER}/comments -f \"body={\\\"review\\\":\\\"pass\\\",\\\"lane\\\":\\\"audit\\\",\\\"timestamp_utc\\\":\\\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\\\",\\\"proof\\\":\\\"no_violations_found\\\",\\\"replay\\\":\\\"gh api /repos/${GITHUB_REPOSITORY}/pulls/${PR_NUMBER}/files\\\"}\""
