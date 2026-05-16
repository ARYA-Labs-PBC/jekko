# Jankurai Agent Context

Canonical knowledge for every agent working in this repository.

## Binary

Name: `jankurai`
Install: https://github.com/neverhuman/jankurai/
Check: `which jankurai` — if not found, install before running audits.

## Key Commands

| Command | Description |
|---|---|
| `jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md` | Full audit (canonical form) |
| `jankurai update` | Update binary to latest |
| `jankurai check-dupes` | Check for duplicate code |
| `jankurai status` | Show current score from `agent/repo-score.json` |
| `jankurai init` | Scaffold config in a new repo |

## Output Files

- `agent/repo-score.json` — machine-readable score document (structured)
- `agent/repo-score.md` — human-readable audit report
- `agent/score-history.csv` — historical score trend
- `agent/score-history.jsonl` — historical score events (structured)

## Score Interpretation

- 0–59: Fail
- 60–74: Warning
- 75–89: Pass (level B)
- 90–100: Pass (level A)

The `decision` field in `repo-score.json` is `"pass"` or `"fail"`.

## TUI Integration

The Jankurai inspector panel (Repo Intel tab, `F2`) shows:
- Current score and sparkline history
- Delta vs baseline (caps, hard findings, soft findings)
- Worker roster from the ZYAL runner
- Install prompt when binary not in PATH

Run `/audit` in the prompt or type "run audit" / "jankurai audit" / "audit the repo"
in chat to trigger an audit from the TUI.

## Advisory Mode

`--mode advisory` is the default for CI/agent use — it reports findings without
blocking. Remove the flag for interactive developer mode which gates on score.
