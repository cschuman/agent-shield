# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Agent Shield is a Rust CLI tool (`agent-shield`) that statically scans a codebase for AI agents (LangChain, CrewAI, MCP, etc.), scores them for risk, and maps findings to compliance frameworks (NIST AI RMF, ISO 42001, EU AI Act, OWASP Agentic Top 10). Think "`npm audit` for AI agents."

Rust edition 2024. Binary crate, no library target, no test suite yet.

## Commands

```bash
cargo build                        # debug build → target/debug/agent-shield
cargo build --release              # release build (LTO, stripped)
cargo run -- scan .                # run scanner against cwd
cargo run -- scan <path> --framework owasp-agentic --min-risk 50
cargo run -- scan . --format json -o report.json
cargo run -- frameworks            # list supported frameworks + baselines
cargo check                        # fast type check
cargo clippy -- -D warnings        # lint
cargo fmt                          # format
```

There are no tests in the repo yet; `cargo test` is a no-op.

## Architecture

Three-stage pipeline in `src/main.rs`, one module per stage:

1. **`scanner.rs`** — walks the directory (skipping `node_modules`, `target`, `.venv`, etc.), reads files with extensions in `SCAN_EXTENSIONS`, and applies per-framework `DetectionPattern`s. Also does a first pass over `package.json` / `pyproject.toml` / `requirements.txt` for dependency-based detection. Produces `Vec<DiscoveredAgent>` with extracted tools, system prompt, guardrails, data-access sources, and permissions — all via regex over file contents. Confidence gating: broad frameworks (`VercelAI`, `CustomAgent`) require ≥2 matches per file; others require 1.

2. **`scoring.rs`** — takes `DiscoveredAgent`s and produces `ScoredAgent`s. Each agent starts at the framework baseline (`AgentFramework::risk_baseline`, 25–55), then adjustments are added/subtracted per factor (autonomy tier, tool count, missing guardrails, exec/admin permissions, data-access breadth, audit-trail absence). Guardrails give up to −25 credit. Final score clamped 0–100 → `RiskLevel` (Low/Medium/High/Critical). Each adjustment also emits a `Finding` with a `framework_reference(...)` pointing at the selected compliance framework's control ID.

3. **`report.rs`** — renders `ScoredAgent`s as terminal output (colored, with ASCII gauge + `comfy-table`) or JSON (`serde_json`). `OutputFormat` and the CLI `--format` flag are the entry points.

`frameworks.rs` is the single source of truth for supported agent frameworks: enum `AgentFramework`, their `detection_patterns()`, and `risk_baseline()`. **Adding a new framework requires changes only here** — the scanner iterates `AgentFramework::all()` and scoring reads the baseline by name via `get_framework_baseline`.

### Key invariants when editing

- `scoring::get_framework_baseline` looks up by `AgentFramework::name()` *string*. If you rename a variant's display name, update any hard-coded string comparisons.
- `scanner.rs` deduplicates agents by `(file_path, framework)` — if you change how agents are constructed, preserve this key or results will multi-count.
- `framework_reference(framework, control)` in `scoring.rs` uses string `control` keys (`"tool-scope"`, `"human-oversight"`, …). New findings must use an existing key or add a match arm in **every** compliance framework branch.
- Risk-level thresholds (0-25 / 26-50 / 51-75 / 76+) and scoring factor values are duplicated between `README.md` and `scoring.rs`. Keep them in sync.

## Repo layout beyond `src/`

- `docs/` — product/business docs (BRD, data model, roadmap, revenue thesis, infra plans). These describe the *paid dashboard* product, not the current OSS CLI. Treat as context for future direction, not as current spec.
- `site/` — static marketing site deployed to Netlify (`site/netlify.toml`). Separate from the Rust crate.
