# Agent Shield

**AI Agent Audit Scanner — `npm audit` for AI agents.**

Discover, score, and audit every AI agent in your codebase. Get a risk report in under 30 seconds.

## Status

Pre-1.0. Detection rules now live in [`rules/builtin/`](rules/builtin/) and scoring rules in [`rules/scoring/`](rules/scoring/), bundled into the binary at build time via `include_str!`. Schema is **experimental** (`schema_version: "1.0"`) and may change without notice — overlay support (`extends:`) is reserved for v1.1. Bad rules are quarantined to stderr at engine init and the scan continues; see [`docs/rules-design/round3-synthesis.md`](docs/rules-design/round3-synthesis.md) for the full design.

```
$ agent-shield scan ./my-project

╔══════════════════════════════════════════════════════════════╗
║              AGENT SHIELD — Risk Assessment Report          ║
╚══════════════════════════════════════════════════════════════╝

  Agents found:  7
  Overall risk:  72/100 (HIGH)

  [████████████████████████████████████░░░░░░░░░░░░░░] 72/100

  Agent                Framework        Risk     Tools  Permissions
  CustomerSupportBot   LangChain        72 HIGH      8  EXEC
  DataAnalysisAgent    CrewAI           58 MED       5  READ/WRITE
  CodeReviewAgent      Anthropic MCP    45 MED       4  READ
  SchedulingAgent      Custom Agent     28 LOW       2  READ

  CRITICAL: 1  HIGH: 4  MEDIUM: 8  LOW: 2
```

## Install

### Prebuilt binary (recommended)

Download the latest release archive for your platform from
[GitHub Releases](https://github.com/cschuman/agent-shield/releases/latest)
and extract it onto your `PATH`. One-liner per platform:

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/cschuman/agent-shield/releases/latest/download/agent-shield-aarch64-darwin.tar.gz | tar -xz

# macOS (Intel)
curl -fsSL https://github.com/cschuman/agent-shield/releases/latest/download/agent-shield-x86_64-darwin.tar.gz | tar -xz

# Linux (x86_64)
curl -fsSL https://github.com/cschuman/agent-shield/releases/latest/download/agent-shield-x86_64-linux.tar.gz | tar -xz
```

Each archive expands into `agent-shield-<version>-<platform>/agent-shield`.
Move the binary onto your `PATH` (e.g. `/usr/local/bin`) and verify with
`agent-shield --version`. Each release also publishes `SHA256SUMS` for
integrity verification.

Windows binaries are not yet shipped. If you need one, please
[open an issue](https://github.com/cschuman/agent-shield/issues).

### From source

```bash
cargo install --git https://github.com/cschuman/agent-shield agent-shield
```

Requires Rust 1.88 or newer (edition 2024 + let-chains).

## Usage

```bash
# Scan a directory
agent-shield scan .

# Scan with a specific compliance framework
agent-shield scan . --framework nist        # NIST AI RMF (default)
agent-shield scan . --framework iso42001    # ISO/IEC 42001
agent-shield scan . --framework eu-ai-act   # EU AI Act
agent-shield scan . --framework owasp-agentic  # OWASP Agentic Top 10

# Export as JSON
agent-shield scan . --format json -o report.json

# Only show high-risk agents
agent-shield scan . --min-risk 50

# List supported frameworks
agent-shield frameworks
```

## What It Detects

### 10 Agent Frameworks

| Framework | Baseline Risk | Detection |
|---|---|---|
| LangChain | 40/100 | Import patterns, package deps |
| LangGraph | 40/100 | Import patterns, package deps |
| CrewAI | 50/100 | Import patterns (multi-agent) |
| AutoGen (Microsoft) | 50/100 | Import patterns (multi-agent) |
| OpenAI Assistants | 35/100 | API patterns |
| Anthropic MCP | 30/100 | Config files, import patterns |
| Anthropic Agent SDK | 45/100 | Import patterns |
| AWS Bedrock Agents | 35/100 | SDK patterns |
| Vercel AI SDK | 25/100 | Import patterns |
| Custom Agents | 55/100 | System prompt + tool call patterns |

### Risk Dimensions

- **Autonomy Level** — NIST 4-tier classification (T1: Supervised → T4: Full Autonomy)
- **Tool Access** — Number and type of tools available to the agent
- **Permissions** — Read, write, execute, admin access detected
- **Guardrails** — Input validation, output filtering, rate limits, human-in-the-loop, content filters
- **Data Access** — Database, cloud storage, APIs, file system, email, webhooks, system commands
- **System Prompt** — Presence and content analysis

### Compliance Mapping

Every finding maps to specific controls in:
- **NIST AI RMF** — Govern, Map, Measure, Manage functions
- **ISO/IEC 42001** — AI management system controls
- **EU AI Act** — High-risk AI system requirements
- **OWASP Agentic Top 10** — Agent-specific security risks

## How Scoring Works

Each agent starts with a framework-specific baseline risk score, then adjustments are applied:

| Factor | Score Impact |
|---|---|
| Autonomy tier increase | +10 per tier |
| >10 tools | +15 |
| Tools without confirmation gates | +10 |
| No system prompt | +10 |
| Missing input validation | +10 |
| Missing output filtering | +5 |
| No rate limiting | +5 |
| System command access | +20 |
| Admin-level permissions | +15 |
| Broad data access (>3 sources) | +10 |
| No audit trail | +5 |
| Each guardrail detected | -5 (max -25) |

Final score clamped to 0-100:
- **0-25**: LOW
- **26-50**: MEDIUM
- **51-75**: HIGH
- **76-100**: CRITICAL

## Dashboard (Coming Soon)

The paid dashboard adds:
- Web UI with agent inventory, risk gauges, and compliance readiness
- Continuous monitoring with drift detection
- PDF board-ready risk reports
- Team collaboration and role-based access
- CI/CD integration
- Historical trend tracking

See [agentshield.dev](https://agent-shield-demo.netlify.app) for pricing and preview.

## License

MIT
