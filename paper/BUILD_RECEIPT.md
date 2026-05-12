# ZYAL Paper Build Receipt

Date: 2026-05-09

## Artifacts

- PDF: `paper/ZYAL.pdf`
- Source: `paper/ZYAL.tex`
- Bibliography: `paper/ZYAL.bib`
- Figures: `paper/figures/*.tex`
- Appendix listings: `paper/listings/*.zyal`
- Research artifacts: `paper/research/source-register.md`, `framework-matrix.md`, `social-discussion-digest.md`, `claim-audit.md`, `research-log.md`
- Build wrapper: `paper/bin/latexminted`

## Build Result

- Command: `rtk make -C paper pdf`
- Result: pass
- PDF page count: 15
- PDF file size: 182,872 bytes
- PDF creation date: Sat May 9 00:17:33 2026 MDT
- Citation sources in `ZYAL.bib`: 37
- Source access date recorded in `paper/research/source-register.md`: 2026-05-09

## Validation Commands

- `rtk make -C paper check`: pass
  - Rebuilt `paper/ZYAL.pdf`.
  - Verified PDF exists.
  - Verified page count is between 8 and 20.
  - Smoke-tested extracted PDF text for `ZYAL`, `LangGraph`, `OpenHands`, `Hermes`, `References`, and `Appendix`.
  - Scanned paper sources for unresolved draft markers.
  - Ran the ZYAL parser test.
- `rtk /bin/test -f paper/ZYAL.pdf`: pass
- `rtk pdfinfo paper/ZYAL.pdf`: pass, `Pages: 15`
- `rtk pdftotext paper/ZYAL.pdf - | rtk rg "ZYAL|LangGraph|OpenHands|Hermes|References|Appendix"`: pass
- Explicit draft-marker scan over paper sources: no matches
- `cd packages/jekko && rtk bun test src/agent-script/parser.test.ts`: pass, 88 tests
- `rtk just fast`: pass

## Notes and Caveats

- The Makefile prepends `paper/bin` so `minted` uses `paper/bin/latexminted`. This local wrapper runs the Homebrew `latexminted` launcher with Python 3.13 because the installed Python 3.14 path raises an `ArgParser.__init__(color=...)` compatibility error.
- Direct `rtk test -f paper/ZYAL.pdf` is not used in the receipt because `rtk` treats bare `test` ambiguously in this environment. The equivalent `rtk /bin/test -f paper/ZYAL.pdf` and the Makefile `test -f ZYAL.pdf` both passed.
- `rtk just fast` reported no critical failures. It still prints existing advisory findings: missing committed lockfile, missing `syft`, and stale security evidence head.
- The paper intentionally labels high-risk ZYAL blocks as preview or partial where runtime enforcement is not complete. Preview/partial areas include hash/nonce/origin arming, capability leases, quality gates, nested budgets, triggers, rollback, done definitions, repo intelligence, fleet, taint, workflow, memory, evidence, approvals, skills, sandbox, security, observability, and control-plane metadata.
- The artifact is an architecture and implementation-evidence paper. It does not claim a large-scale task-success benchmark.
- Social sources are summarized only in `paper/research/social-discussion-digest.md` as anecdotal context and are not used as authoritative paper citations.
