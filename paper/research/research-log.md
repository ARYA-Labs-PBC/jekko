# ZYAL Research Log

Access date: 2026-05-09.

## Local Inventory

Commands run:

- `rtk sed -n ... agent/JANKURAI_STANDARD.md`
- `rtk sed -n ... paper/ZYAL.md ZYAL_MISSION.md docs/ZYAL_MISSION.md`
- `rtk sed -n ... docs/ZYAL/examples/README.md`
- `rtk wc -l -c packages/jekko/src/agent-script/schema.ts packages/jekko/src/agent-script/parser.ts packages/jekko/src/agent-script/parser.test.ts docs/ZYAL/examples/*.zyal`
- `rtk rg -n ... packages/jekko/src/session/daemon*.ts packages/jekko/test/session/daemon*.test.ts`

Findings:

- Parser/schema/test sizes: `schema.ts` 2,078 lines / 85,727 bytes; `parser.ts` 1,803 lines / 75,355 bytes; parser test 1,388 lines.
- Docs example corpus uses canonical `.zyal` files. The older draft phrase "9 examples" is stale.
- Runtime has roughly 5,657 lines across `packages/jekko/src/session/daemon*.ts`.
- SQLite truth tables are in `daemon.sql.ts`.
- Taint and arming hash/nonce are preview-only in docs/TUI wording.

## Web Queries

Accepted source searches:

- `LangGraph persistence docs checkpoints human in the loop memory fault tolerance official`
- `OpenHands paper arXiv 2407.16741`
- `OpenAI Agents SDK documentation official agents guide 2026`
- `Microsoft Agent Framework overview official docs`
- `CrewAI documentation official crews agents tasks flows memory tools`
- `AutoGen paper arxiv 2308.08155 multi-agent conversation framework`
- `Hermes Agent NousResearch GitHub repository agent framework`
- `OWASP Agentic AI Threats and Mitigations prompt injection tool misuse`
- `LlamaIndex agents workflows documentation official`
- `PydanticAI documentation agents official`
- `Haystack agents documentation official`
- `Semantic Kernel agents documentation official`
- `MetaGPT paper arxiv multi-agent framework software company SOP 2023`
- `CAMEL Communicative Agents arxiv role-playing autonomous cooperation paper`
- `AutoGPT GitHub repository autonomous AI agent platform official`
- `BabyAGI GitHub repository task-driven autonomous agent official`
- `Google Agent Development Kit ADK documentation official agents tools workflows`
- `Hugging Face smolagents documentation official agent framework`
- `Mastra AI agents documentation official workflows memory evals`
- `VoltAgent TypeScript AI agent framework docs official`
- `SWE-agent paper arxiv repository software engineering agent computer interface`
- `Aider AI pair programming in terminal GitHub official documentation`
- `Claude Code documentation official Anthropic overview permissions`
- `OpenAI Codex CLI GitHub official docs sandbox approval`
- `Devin AI software engineer official Cognition docs sandbox code agent`
- `Cursor AI coding agent official docs agent mode rules review`
- `Windsurf Cascade agentic IDE official docs workspace rules terminal commands`
- `Model Context Protocol specification official MCP docs tools resources prompts`
- `OpenAPI Specification official tool schemas REST API description`
- `Open Policy Agent Rego policy language official docs policy as code`
- `GitHub Actions workflow syntax official documentation YAML`
- `Kubernetes declarative management configuration official docs YAML`
- `Terraform configuration language official docs infrastructure as code`
- `AI Agentic Programming survey arXiv 2508.11126`
- `Failed agentic pull requests study arXiv 2601.15195`

Social searches:

- `site:reddit.com/r/cursor agent mode ignores rules approval coding agents`
- `site:reddit.com/r/ClaudeAI Claude Code ignores instructions deletes tests permission agent`
- `site:reddit.com/r/OpenAI Codex CLI approval sandbox agent rules`
- `site:reddit.com/r/AI_Agents agent framework observability guardrails production pain points`

## Gaps

- Several proprietary IDE agents expose product pages but not detailed implementation designs. The matrix therefore treats them as feature surfaces, not as audited safety architectures.
- The social corpus is noisy and not representative. It is retained only to motivate pain-point categories already supported by papers and official docs.
- No runtime benchmark of ZYAL task success exists yet. The paper must frame evaluation as artifact/proof coverage plus an evaluation agenda, not a model-performance result.
