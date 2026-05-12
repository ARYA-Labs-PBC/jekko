# ZYAL Source Register

Access date for web sources: 2026-05-09.

Reliability tiers:

- Tier 1: peer-reviewed paper, arXiv/preprint from identifiable authors, official specification, official documentation, or first-party repository.
- Tier 2: vendor/product page with implementation claims.
- Tier 3: community discussion, news, or social post. Use only as anecdotal context.

| Key | Tier | Source | URL | Claim supported |
|---|---:|---|---|---|
| `agenticProgrammingSurvey2025` | 1 | Wang et al., "AI Agentic Programming: A Survey of Techniques, Challenges, and Opportunities" | https://arxiv.org/abs/2508.11126 | Agentic programming spans planning, memory/context, tool use, execution monitoring, safety, and human collaboration challenges. |
| `failedAgenticPRs2026` | 1 | Ehsani et al., "Where Do AI Coding Agents Fail?" | https://arxiv.org/abs/2601.15195 | Agent-authored PR failures correlate with larger diffs, more touched files, CI failures, missing review engagement, duplicate/unwanted work, and misalignment. |
| `codingAgentPromptInjection2026` | 1 | "Are AI-assisted Development Tools Immune to Prompt Injection?" | https://arxiv.org/abs/2603.21642 | Coding agents need static validation, parameter visibility, injection detection, sandboxing, warnings, and audit logging. |
| `agentproof2026` | 1 | "Agentproof: Static Verification of Agent Workflow Graphs" | https://arxiv.org/abs/2603.20356 | Workflow-graph safety can be checked structurally across frameworks such as LangGraph, CrewAI, AutoGen, and Google ADK. |
| `openhands2024` | 1 | Wang et al., "OpenHands: An Open Platform for AI Software Developers as Generalist Agents" | https://arxiv.org/abs/2407.16741 | OpenHands provides an open SWE-agent platform with code editing, shell/browser interaction, sandboxing, multi-agent coordination, and benchmarks. |
| `sweAgent2024` | 1 | Yang et al., "SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering" | https://arxiv.org/abs/2405.15793 | Agent-computer interfaces improve repository navigation, editing, and test execution for SWE agents. |
| `autogen2023` | 1 | Wu et al., "AutoGen: Enabling Next-Gen LLM Applications via Multi-Agent Conversation" | https://arxiv.org/abs/2308.08155 | AutoGen frames multi-agent applications as conversable agents that coordinate to complete tasks. |
| `metagpt2023` | 1 | Hong et al., "MetaGPT" | https://arxiv.org/abs/2308.00352 | MetaGPT encodes software-company SOPs and role specialization into multi-agent collaboration. |
| `camel2023` | 1 | Li et al., "CAMEL" | https://arxiv.org/abs/2303.17760 | CAMEL studies role-playing communicative agents for autonomous cooperation. |
| `langgraphPersistence2026` | 1 | LangGraph persistence docs | https://docs.langchain.com/oss/python/langgraph/persistence | LangGraph provides checkpoints for graph state, human-in-the-loop, memory, time travel, and fault tolerance. |
| `openaiAgentsSdk2026` | 1 | OpenAI Agents SDK documentation | https://openai.github.io/openai-agents-js/guides/agents/ | OpenAI Agents SDK packages agents, tools, handoffs, guardrails, tracing, and model integration. |
| `microsoftAgentFramework2026` | 1 | Microsoft Agent Framework overview | https://learn.microsoft.com/en-us/agent-framework/overview/ | Microsoft Agent Framework combines agents, graph workflows, sessions, memory/context providers, middleware, MCP clients, and telemetry. |
| `crewaiDocs2026` | 1 | CrewAI documentation | https://docs.crewai.com/en/index | CrewAI organizes role-playing agents, crews, tasks, tools, memory, knowledge, and flows. |
| `llamaIndexAgents2026` | 1 | LlamaIndex agents docs | https://docs.llamaindex.ai/en/stable/use_cases/agents/ | LlamaIndex supports tool-using agents, workflows, RAG, planning, and memory modules. |
| `pydanticAiAgents2026` | 1 | Pydantic AI agents docs | https://pydantic.dev/docs/ai/core-concepts/agent/ | Pydantic AI centers type-safe agents with instructions, tools, structured output, dependencies, and model settings. |
| `haystackAgents2026` | 1 | Haystack Agent docs | https://docs.haystack.deepset.ai/docs/agent | Haystack Agent is an iterative tool-using component with state schema and exit conditions. |
| `semanticKernelAgents2026` | 1 | Semantic Kernel Agent Framework docs | https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/ | Semantic Kernel provides an agent framework inside the SK ecosystem. |
| `googleAdk2026` | 1 | Google Agent Development Kit docs | https://google.github.io/adk-docs/ | Google ADK supports workflow agents, LLM-driven routing, tools, broad LLM support, and deployment. |
| `smolagents2026` | 1 | Hugging Face smolagents docs | https://huggingface.co/docs/smolagents/v1.17.0/en/index | smolagents provides a small framework for building tool-using agents. |
| `mastra2026` | 1 | Mastra framework docs/product page | https://mastra.ai/framework | Mastra supports tools, MCP, memory, approval, workspaces, supervisor agents, evals, guardrails, tracing, and workflow suspend/resume. |
| `voltagent2026` | 1 | VoltAgent docs/product page | https://voltagent.dev/ | VoltAgent is a TypeScript agent engineering platform with memory, RAG, guardrails, tools, MCP, workflow, observability, evals, and deployment. |
| `autogptRepo2026` | 1 | AutoGPT GitHub repository | https://github.com/Significant-Gravitas/AutoGPT | AutoGPT is a platform for building, deploying, and managing continuous AI agents. |
| `babyagiRepo2026` | 1 | BabyAGI GitHub repository | https://github.com/yoheinakajima/babyagi | BabyAGI is an early task-queue/autonomous-agent reference point. |
| `aiderDocs2026` | 1 | Aider documentation | https://aider.chat/docs/ | Aider is terminal pair programming with repository editing and git integration. |
| `claudeCodePermissions2026` | 1 | Claude Code permissions docs | https://code.claude.com/docs/en/permissions | Claude Code has permission rules, modes, and managed policies. |
| `codexCliHelp2026` | 1 | OpenAI Codex CLI Help Center | https://help.openai.com/en/articles/11096431-openai-codex-cli-getting-tarted | Codex CLI is an open-source terminal agent with sandbox and approval modes. |
| `devinDocs2026` | 2 | Devin enterprise overview | https://docs.devin.ai/enterprise/overview | Devin is a proprietary autonomous SWE-agent product with enterprise controls and data terms. |
| `cursorAgentDocs2026` | 1 | Cursor Agent docs | https://docs.cursor.com/en/chat/agent | Cursor Agent can explore code, edit multiple files, run commands, and fix errors from the editor. |
| `windsurfCascade2026` | 1 | Windsurf Cascade docs | https://docs.windsurf.com/windsurf/cascade/cascade | Cascade is an agentic IDE assistant with code/chat modes, tool calling, checkpoints, and linter integration. |
| `hermesAgentRepo2026` | 1 | NousResearch Hermes Agent GitHub repository | https://github.com/NousResearch/hermes-agent | Hermes Agent/OpenClaw is an open agent with tools, skills, memory, and multi-platform messaging. |
| `mcpDocs2026` | 1 | Model Context Protocol documentation | https://docs.anthropic.com/en/docs/mcp | MCP standardizes how models connect to external data sources and tools. |
| `openApiSpec2026` | 1 | OpenAPI Specification | https://spec.openapis.org/oas/ | OpenAPI provides a machine-readable API description format with official versions and schemas. |
| `opaRego2026` | 1 | Open Policy Agent Rego docs | https://www.openpolicyagent.org/docs/latest/policy-language/ | OPA/Rego expresses policy decisions over structured data as code. |
| `githubActionsSyntax2026` | 1 | GitHub Actions workflow syntax | https://docs.github.com/actions/using-workflows/workflow-syntax-for-github-actions | GitHub Actions uses YAML workflow files for automation. |
| `kubernetesDeclarative2023` | 1 | Kubernetes declarative object management docs | https://kubernetes.io/docs/tasks/manage-kubernetes-objects/declarative-config/ | Kubernetes uses declarative configuration and server-side reconciliation. |
| `terraformIac2026` | 1 | Terraform infrastructure-as-code docs | https://developer.hashicorp.com/terraform/tutorials/aws-get-started/infrastructure-as-code | Terraform uses declarative configuration for desired infrastructure state. |
| `owaspAgentic2026` | 1 | OWASP Agentic AI Threats and Mitigations | https://genai.owasp.org/resource/agentic-ai-threats-and-mitigations/ | Agentic AI introduces threats around prompt injection, tool misuse, context/resource exposure, and human oversight. |

Rejected or non-authoritative sources:

- Vendor comparison blogs and SEO summaries were not used for core claims when official docs or papers were available.
- Reddit, X, LinkedIn, and Quora discussions were treated as anecdotal signal only.
- Wikipedia pages were not used as authoritative citations.
