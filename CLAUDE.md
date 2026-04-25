# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Agent Shield is a Rust CLI tool (`agent-shield`) that statically scans a codebase for AI agents (LangChain, CrewAI, MCP, etc.), scores them for risk, and maps findings to compliance frameworks (NIST AI RMF, ISO 42001, EU AI Act, OWASP Agentic Top 10). Think "`npm audit` for AI agents."

Rust edition 2024. Binary crate, no library target.

## Commands

```bash
cargo build                        # debug build → target/debug/agent-shield
cargo build --release              # release build (LTO, stripped)
cargo run -- scan .                # run scanner against cwd
cargo run -- scan <path> --framework owasp-agentic --min-risk 50
cargo run -- scan . --format json -o report.json
cargo run -- frameworks            # list supported frameworks + baselines
cargo check                        # fast type check
cargo test                         # 68+ unit tests (loader, matcher, scoring, signals)
cargo clippy -- -D warnings        # lint (must stay clean)
cargo fmt                          # format
bash scripts/snapshot.sh verify    # byte-identical regression oracle (6 fixtures)
```

## Architecture

Three-stage pipeline in `src/main.rs`, one module per stage:

1. **`scanner.rs`** — walks the directory (skipping `node_modules`, `target`, `.venv`, etc.), reads files with extensions in `SCAN_EXTENSIONS`, and runs `engine.detection_rules()` over each. Produces `Vec<DiscoveredAgent>` with extracted tools, system prompt, guardrails, data-access sources, and permissions. Confidence gating uses each rule's `min_match_count` (2 for broad frameworks like VercelAI/CustomAgent, 1 otherwise).

2. **`signals.rs`** — `compute_all_signals(&DiscoveredAgent) -> ContextSignals`. The fact set scoring rules read from: `tool_count`, `unconfirmed_tool_count`, `has_system_prompt`, `autonomy_tier`, `guardrails`, `permissions`, `data_source_count`, `has_audit_trail`. Adding a new signal requires coordinated edits in `signals.rs`, `engine/matcher.rs::evaluate_context_signal`, and `rules/loader.rs::is_known_signal`.

3. **`scoring.rs`** — takes `DiscoveredAgent`s and produces `ScoredAgent`s. Each agent starts at the framework baseline (`AgentFramework::risk_baseline`, 25–55), gains/loses points from the autonomy-tier scalar, then loops over `engine.scoring_rules()` applying `score_adjustment` and materializing findings. Guardrails give up to −25 credit. Final score clamped 0–100 → `RiskLevel` (Low/Medium/High/Critical). Each finding's `framework_ref` is sourced from the rule's per-framework `compliance:` block via `pick_compliance`.

4. **`report.rs`** — renders `ScoredAgent`s as terminal output (colored, with ASCII gauge + `comfy-table`) or JSON (`serde_json`). `OutputFormat` and the CLI `--format` flag are the entry points.

### Rule data layout (post-W2)

Detection and scoring rules live as YAML in:

- `rules/builtin/<framework>.yaml` — 10 detection rules, one per `AgentFramework` variant.
- `rules/scoring/<slug>.yaml` — 11 Tier-2 scoring rules. The order of these in `EMBEDDED_RULES` (`src/engine/mod.rs`) is **load-bearing**: it pins the W1 finding-emission order which the snapshot fixtures hash against.

The loader (`src/rules/loader.rs`) deserializes via `serde_yaml`, validates (schema_version, allowlisted signals, exactly-one matcher slot, etc.), and translates `ParsedRule` → `CompiledRule` / `CompiledScoringRule`. Bad rules quarantine into `RuleDiagnostic`s printed to stderr at engine init; the scan continues with the surviving rules.

`AgentFramework` (`src/frameworks.rs`) keeps its enum identity — `name()`, `risk_baseline()`, `all()` — but no longer carries detection logic. Adding a new framework requires:

1. New variant in `AgentFramework` + update `name()`, `risk_baseline()`, `all()` in `src/frameworks.rs`.
2. Update `variant_ident()` in `src/rules/loader.rs` so YAML `framework:` strings parse to the new variant.
3. New `rules/builtin/<slug>.yaml` mirroring the existing detection-rule schema.
4. Insert the entry into `EMBEDDED_RULES` (`src/engine/mod.rs`) in matching position relative to `AgentFramework::all()`.
5. `bash scripts/snapshot.sh verify` (existing fixtures must still match; new framework needs a new fixture if you want regression coverage).

Adding a new scoring rule requires:

1. New `rules/scoring/<slug>.yaml`.
2. Insert into `EMBEDDED_RULES` at the correct **firing-order position** (the W2-C8 byte-identical contract).
3. `bash scripts/snapshot.sh verify` to confirm no drift.

### Key invariants when editing

- `EMBEDDED_RULES` order in `src/engine/mod.rs` is load-bearing for snapshot tests. Detection entries must mirror `AgentFramework::all()` order; scoring entries must mirror W1 firing order. Reorder ⇒ snapshot diff.
- `scoring::get_framework_baseline` looks up by `AgentFramework::name()` *string*. Renaming a variant's display name requires hunting any hard-coded string compares.
- `scanner.rs` deduplicates agents by `(file_path, framework)` — preserve this key or counts double.
- The signal allowlist appears in two places (`is_known_signal` in loader.rs, `evaluate_context_signal` in matcher.rs). They must stay in lockstep; a parity test would catch drift.
- `Matcher::matches_repo` is wired but the v1.0 scanner only invokes `matches_file`. `package_dep:` and `file_present:` matchers in YAML are dormant until W3 lands the manifest pass.
- Risk-level thresholds (0-25 / 26-50 / 51-75 / 76+) are duplicated between `README.md` and `scoring.rs`. Keep them in sync.

## Repo layout beyond `src/`

- `rules/` — bundled rule data (detection + scoring YAML).
- `tests/bad-rules/` — fixtures asserting the loader's quarantine paths.
- `fixtures/` and `snapshots/` — corpus + golden JSON for `scripts/snapshot.sh`.
- `docs/` — product/business docs (BRD, data model, roadmap, revenue thesis, infra plans). These describe the *paid dashboard* product, not the current OSS CLI. Treat as context for future direction, not as current spec.
- `docs/rules-design/` — Path B refactor synthesis. `round3-synthesis.md` is the W2 scope source of truth.
- `site/` — static marketing site deployed to Netlify (`site/netlify.toml`). Separate from the Rust crate.

## Snapshot drift recovery

CI (`.github/workflows/ci.yml`) runs `bash scripts/snapshot.sh verify` on every PR. When that job goes red, follow this exact 3-step workflow:

```bash
cargo build --release
bash scripts/snapshot.sh verify       # see the diff
bash scripts/snapshot.sh capture      # only if intentional; review every byte before committing
```

If the diff is **intentional** (you changed a rule, scoring weight, or finding text on purpose): run `capture`, then commit the changed `snapshots/*.json` files in their own commit titled `snapshot: <reason>`. Reviewers should be able to scrutinize byte-level changes in isolation, separate from the source change that produced them.

If the diff is **unintentional**: revert the offending source change. The snapshots are the source of truth, not the code. The byte-identical contract has survived W1, W2, and every review round — preserve it.

Never let CI auto-update snapshots. The whole point of the oracle is that drift is a human decision.
