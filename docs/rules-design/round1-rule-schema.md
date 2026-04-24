# Round 1 Rule Schema Proposal

**Author:** Security Auditor (Expert #2 of 4)
**Date:** 2026-04-24
**Version:** 0.1 (Round 1 draft)

---

## Top 3 Schema Decisions and the Biggest Tradeoff

### Decision 1: One Rule Kind, Three Logical Sections — Unified Rather Than Split

The most tempting fork in the road is splitting detection, scoring, and compliance into separate rule files linked by ID — the way OPA separates policy from data, or how Trivy separates advisory records from detection logic. I am explicitly rejecting that model.

The reason: agent-shield's strategic goal is to "own the rule taxonomy." A three-file join is a schema a team of six can maintain. A single unified file is a schema a researcher can contribute to in 20 minutes. Every PR that requires touching three files to add one new signal will slow community adoption, and community adoption is the entire point. Semgrep's success came from rules that are self-contained and auditable in one glance. We take that lesson wholesale.

A single rule therefore expresses all three concerns: what to detect, how much it contributes to the score, and which compliance controls it satisfies. Detection triggers a finding; findings carry scoring adjustments and compliance mappings. There is no situation where you want the detection without the scoring or the scoring without the compliance reference — they form a coherent atomic unit of agent risk knowledge.

### Decision 2: Detection Primitives Are a Typed Union, Not a Plugin Interface

Looking at Semgrep, Trivy, and Falco for precedent:

- **Semgrep** uses AST patterns with metavariables — powerful, but requires a language parser per target language and a non-trivial contributor learning curve.
- **Trivy misconfiguration rules** (Rego/OPA) are expressive but require learning Rego.
- **Falco** uses a field-expression DSL close to tcpdump — fast to read, but purpose-built for kernel events.
- **CodeQL** uses a full query language — accurate but heavy.

We are rejecting AST-level matching for this schema version. The current codebase operates on file content with regex, and our contributor audience (framework authors, GRC researchers) should not need to understand Python's AST or TypeScript's parse tree to contribute a rule. We take from Semgrep the concept of typed match primitives and from Trivy the concept of scope-restricted checks, but we stay above the AST layer.

The detection block supports a typed union of these primitives:

- `import_contains` — line-level import/require/use check, with optional `languages` scope filter
- `code_regex` — single-line regex across file content (what the current codebase calls `CodePattern`)
- `multiline_regex` — regex with `(?s)` dot-matches-newline semantics, for multi-line string literals
- `package_dep` — manifest-level dependency name substring match
- `file_present` — presence of a named file anywhere in the scanned tree
- `all_of` — AND combinator: every child primitive must match
- `any_of` — OR combinator: at least one child must match (this is the implicit default when a rule lists multiple triggers)
- `not` — negation: inverts the result of one child primitive

These eight primitives cover every detection shape currently hardcoded in `frameworks.rs` and `scanner.rs`. The `all_of`/`not` combinators unlock rules like "import langchain AND no rate limit detected."

### Decision 3: Scoring Is an Adjustment, Never an Absolute Override

The existing `scoring.rs` architecture uses a framework-specific baseline and then stacks signed integer adjustments. Rules in YAML continue this additive model: each rule carries a `score_adjustment` field (positive integer for risk-increasing findings, negative for guardrail credit). The Rust engine accumulates adjustments on top of the framework baseline. Rules never set the final score directly — they only shift it.

This is a deliberate constraint that preserves two invariants: (1) two rules firing simultaneously produce additive effects that are individually auditable, and (2) the framework baseline remains the single canonical starting point. If we allowed rules to set absolute scores, two rules could fight over final ownership and the result would be engine-dependent and untestable.

The framework baseline itself is NOT in YAML rules. It belongs to a separate `frameworks/*.yaml` catalog (owned by the backend-architect in Round 2), because baselines are framework metadata, not detection logic. A rule that matches LangChain imports doesn't know or care what LangChain's baseline is.

### Biggest Tradeoff

The unified single-file design means the schema carries three concerns simultaneously, making individual fields semantically coupled. A rule's `score_adjustment` is meaningless unless the rule's `when` block fires. Its `compliance` block is meaningless unless you know which framework the user selected. This coupling is a tradeoff against the modularity of a split design. I accept it because contributor ergonomics outweigh engine elegance at this stage, and the coupling is semantic, not structural — the engine can ignore compliance mappings when running `--no-compliance`, ignore scoring when running in detection-only mode, and so on.

---

## Schema Versioning

Every rule file declares the schema version it conforms to in the top-level `schema_version` field. This is a string with semantic versioning semantics, not a URL — the Rust engine gates on this field before loading the rest of the file. Unknown versions produce a hard error, not a warning, to prevent silently wrong behavior when old rules meet a new engine.

```
schema_version: "1.0"
```

The engine supports `schema_version: "1.x"` loading, where minor bumps add optional fields the engine ignores if absent. Major bumps (2.0) require engine changes and old rules will fail to load.

---

## Top-Level Schema Structure

```yaml
schema_version: "1.0"

id:           # globally unique, kebab-case, namespaced: framework/category/slug
name:         # human-readable, one line
description:  # Markdown, ≥1 sentence explaining the risk
severity:     # info | low | medium | high | critical
tags:         # free-form list for filtering; conventional tags below
category:     # FindingCategory enum value (maps directly to scoring.rs FindingCategory)
references:   # list of URLs or short strings (CWE, OWASP item, paper)

when:         # detection block — typed union of primitives
score_adjustment: # signed integer, applied when rule fires; negative = mitigation credit
compliance:   # per-framework control IDs; only the selected framework is used at runtime
remediation:  # Markdown — actionable fix, code example optional
```

### Tag Conventions

Conventional tag vocabulary (open, contributors may add more):

- `framework:langchain`, `framework:crewai`, `framework:mcp`, etc.
- `signal:permission`, `signal:guardrail`, `signal:tool`, `signal:prompt`, `signal:autonomy`
- `language:python`, `language:typescript`, `language:go`
- `tier:detection` (fires on framework presence), `tier:behavior` (fires on usage pattern), `tier:hygiene` (fires on absent control)

### Detection Primitive Reference

```yaml
# Primitive: import_contains
when:
  import_contains: "langchain"
  languages: [python]         # optional scope filter

# Primitive: code_regex
when:
  code_regex: 'subprocess\.(?:run|call|Popen)\s*\('
  languages: [python]

# Primitive: multiline_regex
when:
  multiline_regex: 'system_prompt\s*=\s*["\x60]{1,3}[\s\S]{0,2000}["\x60]{1,3}'

# Primitive: package_dep
when:
  package_dep: "crewai"

# Primitive: file_present
when:
  file_present: "mcp.json"

# Compound: all_of (AND)
when:
  all_of:
    - import_contains: "langchain"
    - code_regex: 'subprocess\.'

# Compound: any_of (OR — also the implicit default for lists)
when:
  any_of:
    - import_contains: "crewai"
    - package_dep: "crewai"

# Compound: not (negation)
when:
  all_of:
    - import_contains: "langchain"
    - not:
        code_regex: '(?i)rate.?limit|RateLimiter'
```

### Compliance Block

The compliance block uses per-framework namespaced keys. The engine selects only the relevant sub-block at runtime based on `--framework` flag.

```yaml
compliance:
  nist_ai_rmf:
    - control: "MAP 1.1"
      description: "Categorize AI risks in context"
    - control: "GOVERN 1.7"
      description: "Policies for third-party risk management"
  iso_42001:
    - control: "A.8.4"
      description: "Human oversight of AI systems"
  eu_ai_act:
    - control: "Article 14"
      description: "Human oversight measures"
  owasp_agentic:
    - control: "A01"
      description: "Excessive Agency"
```

---

## Hard Question: Non-Regex Signals

The signals that are NOT simple regex — autonomy tier inference, tool count, system prompt presence, guardrail detection, data access classification — deserve explicit treatment.

**My answer: these stay as Rust primitives, but rules can reference them via a `context_signal` assertion.**

The autonomy tier is a computed aggregate that depends on multiple extracted properties of a `DiscoveredAgent`. It cannot be re-expressed as a file-content regex because it operates on the structured output of the extraction phase, not raw text. The same applies to tool count and guardrail inventory.

Trying to encode these in YAML detection primitives would either require a mini query language over the `DiscoveredAgent` struct (significant complexity, unclear value) or would leak internal struct field names into the public schema (brittle coupling).

Instead, I introduce a `context_signal` primitive that references named signals the Rust engine computes and injects into the rule evaluation context:

```yaml
when:
  context_signal:
    signal: tool_count
    op: gt
    value: 10
```

Supported context signals in v1.0:

| Signal name | Type | Source |
|---|---|---|
| `tool_count` | integer | `DiscoveredAgent.tools.len()` |
| `has_system_prompt` | bool | `DiscoveredAgent.system_prompt.is_some()` |
| `autonomy_tier` | integer (1-4) | `scoring::assess_autonomy_tier()` |
| `has_guardrail` | bool with `guardrail_kind` param | `DiscoveredAgent.guardrails` |
| `has_permission` | bool with `permission_level` param | `DiscoveredAgent.permissions` |
| `data_source_count` | integer | `DiscoveredAgent.data_access.len()` |

This keeps the Rust engine as the canonical evaluator of complex signals while allowing YAML rules to compose those signals with standard boolean logic. New context signals require a Rust PR — they are part of the engine's public contract. This is a deliberate chokepoint: we do not want arbitrary code execution in rules.

The `all_of`/`not` combinators work on `context_signal` just as they do on regex primitives:

```yaml
when:
  all_of:
    - import_contains: "langchain"
    - context_signal:
        signal: tool_count
        op: gt
        value: 10
    - not:
        context_signal:
          signal: has_guardrail
          guardrail_kind: rate_limit
```

---

## Three Complete Worked Examples

### Example 1: LangChain Framework Detection

```yaml
schema_version: "1.0"

id: "framework/langchain/langchain-agent-detected"
name: "LangChain agent framework detected"
description: >
  A LangChain import or package dependency was found, indicating the codebase
  contains an AI agent built with LangChain. LangChain agents can execute chains
  of LLM calls, invoke tools, and manage memory — all of which expand the attack
  surface relative to a simple API call. This finding establishes the framework
  baseline; further behavioral rules will stack additional adjustments.
severity: medium
tags:
  - framework:langchain
  - tier:detection
  - language:python
  - language:typescript
category: FrameworkDetection
references:
  - "CWE-284: Improper Access Control"
  - "https://owasp.org/www-project-top-10-for-large-language-model-applications/"

when:
  any_of:
    - import_contains: "langchain"
      languages: [python]
    - import_contains: "from langchain"
      languages: [python]
    - import_contains: "@langchain/core"
      languages: [typescript]
    - package_dep: "langchain"
    - package_dep: "@langchain/core"

score_adjustment: 0
# Score adjustment is zero here because the framework baseline is set separately
# in the framework catalog (frameworks/langchain.yaml). This rule's job is purely
# detection and compliance tagging — the baseline handles the starting score.

compliance:
  nist_ai_rmf:
    - control: "MAP 1.1"
      description: "Identify and classify AI components in the system inventory"
    - control: "GOVERN 1.7"
      description: "Policies exist for third-party AI component risk"
  iso_42001:
    - control: "A.6.1.1"
      description: "Inventory of AI systems and components"
  eu_ai_act:
    - control: "Article 9"
      description: "Risk management system — identify and analyze known risks"
  owasp_agentic:
    - control: "A01"
      description: "Excessive Agency — establish framework presence before evaluating scope"

remediation: >
  Confirm that LangChain usage is intentional and documented in your AI system
  inventory. Apply the LangChain-specific behavioral rules in this ruleset to
  assess tool scope, prompt safety, and guardrail coverage. Ensure LangChain and
  its transitive dependencies are pinned to a reviewed version in requirements.txt
  or package-lock.json to prevent supply-chain drift.
```

---

### Example 2: CrewAI Multi-Agent Detection with High-Autonomy Scoring

```yaml
schema_version: "1.0"

id: "framework/crewai/multi-agent-no-human-oversight"
name: "CrewAI multi-agent crew operating without human oversight gate"
description: >
  A CrewAI crew was detected with multiple agents and no human approval guardrail.
  CrewAI's multi-agent architecture enables agents to delegate tasks to each other
  and synthesize decisions across agent boundaries — a pattern that significantly
  expands the blast radius of any single agent compromise or hallucination.
  Without an explicit human-in-the-loop gate, the crew can autonomously complete
  complex multi-step tasks including external tool calls, file writes, and API
  interactions without any human checkpoint.
severity: high
tags:
  - framework:crewai
  - signal:autonomy
  - signal:guardrail
  - tier:behavior
  - language:python
category: NoHumanOversight
references:
  - "OWASP Agentic Top 10: A01 Excessive Agency"
  - "OWASP Agentic Top 10: A06 Multi-Agent Trust"
  - "NIST AI RMF: GOVERN 1.3"
  - "CWE-306: Missing Authentication for Critical Function"

when:
  all_of:
    - any_of:
        - import_contains: "crewai"
          languages: [python]
        - package_dep: "crewai"
    - code_regex: 'Crew\s*\('
      languages: [python]
    - not:
        context_signal:
          signal: has_guardrail
          guardrail_kind: human_approval

score_adjustment: 20
# +20 applied on top of CrewAI's baseline (50). Multi-agent without HITL is
# a significant escalation — equivalent to system command access in risk weight.
# Rationale: a hallucinating sub-agent in a crew can instruct other agents,
# compounding errors with no human intercept point.

compliance:
  nist_ai_rmf:
    - control: "GOVERN 1.3"
      description: "Organizational roles and responsibilities for AI risk oversight"
    - control: "MANAGE 2.4"
      description: "Mechanisms to detect and respond to AI risks in deployment"
  iso_42001:
    - control: "A.8.4"
      description: "Human oversight — ensure humans can intervene in AI decisions"
    - control: "A.9.3"
      description: "Review of AI system performance and safety"
  eu_ai_act:
    - control: "Article 14"
      description: >
        Human oversight — high-risk AI systems must allow humans to intervene,
        interrupt, or override the system at any time
  owasp_agentic:
    - control: "A01"
      description: "Excessive Agency — autonomous multi-agent action without oversight"
    - control: "A06"
      description: "Multi-Agent Trust Exploitation — agents trusting each other without verification"

remediation: >
  Add a human approval step at crew kickoff and at any decision node where an
  agent delegates a destructive or irreversible action to another agent. In CrewAI,
  set `human_input=True` on the Crew or on individual Tasks that involve external
  tool calls, file writes, or financial operations. For automated pipelines where
  synchronous human approval is impractical, implement an async approval queue
  with a timeout-to-reject policy rather than a timeout-to-approve policy.

  Example (Python):
  ```python
  crew = Crew(
      agents=[researcher, writer],
      tasks=[research_task, write_task],
      human_input=True,           # require approval before crew executes
      verbose=True,
  )
  ```
```

---

### Example 3: System Command Access (Non-Framework Signal)

```yaml
schema_version: "1.0"

id: "behavior/permissions/system-command-access"
name: "Agent has access to system command execution"
description: >
  The agent or its tool definitions contain patterns consistent with system command
  execution — subprocess calls, shell invocations, or OS-level exec patterns.
  System command access is the highest-severity permission an agent can hold:
  it enables file system modification, process spawning, network reconfiguration,
  and privilege escalation if combined with insufficient sandboxing. In an agentic
  context, an adversary who can influence the agent's inputs (prompt injection)
  can potentially execute arbitrary commands on the host.
severity: critical
tags:
  - signal:permission
  - tier:behavior
  - language:python
  - language:typescript
  - language:go
  - language:rust
category: ExcessivePermission
references:
  - "CWE-78: OS Command Injection"
  - "CWE-250: Execution with Unnecessary Privileges"
  - "OWASP Agentic Top 10: A01 Excessive Agency"
  - "OWASP Agentic Top 10: A03 Prompt Injection → Command Execution"
  - "NIST AI RMF: GOVERN 1.7, MAP 3.4"

when:
  any_of:
    - code_regex: 'subprocess\.(?:run|call|Popen|check_output|check_call)\s*\('
      languages: [python]
    - code_regex: '(?:os\.system|os\.popen)\s*\('
      languages: [python]
    - code_regex: 'child_process\.(?:exec|execSync|spawn|spawnSync)\s*\('
      languages: [typescript]
    - code_regex: '\bexec\.Command\s*\('
      languages: [go]
    - code_regex: 'Command::new\s*\('
      languages: [rust]
    - code_regex: '(?:shell|bash|sh)\s*[=:]\s*True'
      languages: [python]

score_adjustment: 20
# +20 matches the existing hardcoded value in scoring.rs. This is the largest
# single positive adjustment in the scoring rubric — justified because system
# command access converts any prompt injection from data exfiltration to
# arbitrary code execution on the host.

compliance:
  nist_ai_rmf:
    - control: "GOVERN 1.7"
      description: "Policies for AI risk — operational context includes command execution risk"
    - control: "MAP 3.4"
      description: "Risk assessment — system command access identified as high impact"
    - control: "MANAGE 4.1"
      description: "Risk response — apply least privilege to agent runtime environment"
  iso_42001:
    - control: "A.8.2"
      description: "AI system security — protect against unauthorized command execution"
    - control: "A.6.2.3"
      description: "Access control for AI systems and their tools"
  eu_ai_act:
    - control: "Article 9"
      description: "Risk management — system command access classified as high-risk capability"
    - control: "Article 15"
      description: "Accuracy, robustness, cybersecurity — agent must resist manipulation to command execution"
  owasp_agentic:
    - control: "A01"
      description: "Excessive Agency — agent granted system-level execution capability"
    - control: "A03"
      description: "Prompt Injection enabling command execution — highest severity chain"

remediation: >
  Treat system command access as requiring explicit justification, not a default
  capability. Apply the following controls in order of preference:

  1. **Remove it**: If the agent's function does not require shell execution,
     remove the subprocess/exec calls and replace with purpose-built library APIs
     (e.g., use `pathlib` instead of `os.system('ls')`, use `requests` instead of
     `curl` via subprocess).

  2. **Allowlist strictly**: If shell execution is required, maintain an explicit
     allowlist of permitted commands. Never pass agent-controlled strings directly
     to a shell. Use argument arrays, not shell=True.

  3. **Sandbox the runtime**: Run the agent process in a container with no-new-privileges,
     read-only root filesystem, and dropped capabilities. Use seccomp to restrict
     syscalls to what the agent actually needs.

  4. **Add human confirmation**: Any tool that invokes a system command must require
     explicit human approval before execution — not just a log entry.

  Example fix (Python):
  ```python
  # BEFORE (dangerous):
  subprocess.run(user_input, shell=True)

  # AFTER (safer):
  ALLOWED_COMMANDS = {"ls": ["/bin/ls"], "echo": ["/bin/echo"]}
  cmd_name = parse_command_name(user_input)   # your own parser, never eval
  if cmd_name not in ALLOWED_COMMANDS:
      raise PermissionError(f"Command not permitted: {cmd_name}")
  subprocess.run(ALLOWED_COMMANDS[cmd_name], shell=False, capture_output=True)
  ```
```

---

## What I Am Borrowing and What I Am Rejecting

### Borrowed from Semgrep
- Self-contained, single-file rules with detection + metadata in one YAML document
- `id` as a namespaced kebab-case string (not a UUID) — human-readable, diffable, PR-reviewable
- `languages` scope filter on individual primitives, not on the whole rule
- Severity enum with conventional values

### Borrowed from Trivy Misconfiguration Rules
- The concept of a `compliance` block with per-framework control IDs
- Remediation as a required first-class field, not an afterthought

### Borrowed from Falco
- The `all_of`/`any_of`/`not` boolean logic layer over typed primitives, rather than a full expression language
- The idea that complex signals (in Falco: kernel events; here: autonomy tier, tool count) are computed by the engine and surfaced as named references in rules

### Rejected from Semgrep
- AST-level pattern matching with metavariables — too high a contribution barrier and requires per-language parsers
- The `pattern-inside` / `pattern-not-inside` structural context patterns — out of scope for text-level scanning

### Rejected from CodeQL / Rego (OPA)
- Full query languages — the contributor audience is security researchers and framework authors, not query language engineers
- Two-phase policy-as-code evaluation — adds engine complexity for marginal expressiveness gain at our current detection fidelity level

### Rejected from splitting (OPA-style data/policy separation)
- Separate detection rules, scoring rules, and compliance mapping files — the join cost hurts contributor velocity more than it helps engine flexibility

---

## Biggest Open Question I Am Punting to Round 2

**How does the schema handle rule inheritance and override?**

The current design has no mechanism for a downstream consumer (say, a GRC partner or an enterprise customer with custom rules) to override a single field of a built-in rule without forking the entire rule file. Semgrep handles this with `extend` and `focus-metavariable`. Trivy handles it with custom policy overlays.

For agent-shield, the SaaS angle makes this urgent: enterprise customers will want to tune `score_adjustment` values to match their internal risk appetite, suppress specific rules that don't apply to their stack, or extend compliance mappings with internal control IDs (SOC 2, PCI DSS). If the schema has no override story, those customers modify built-in rules directly — which breaks `git pull` upgrades of the rule bundle.

Round 2 should decide: do we adopt an `extends: <rule-id>` field that allows a rule to inherit from a parent and override specific fields? Or do we use a separate suppression/overlay file format that the engine merges at load time? The choice has direct implications for the Rust engine's rule loading trait (rust-pro's domain) and the distribution/versioning model (backend-architect's domain). I am not pre-deciding it here because it touches both of their pieces — but the schema needs to reserve the `extends` field as a future top-level key so we don't paint ourselves into a corner.
