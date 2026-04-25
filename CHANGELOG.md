# Changelog

All notable changes to agent-shield are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

The crate version (`Cargo.toml`) and the YAML rule schema version
(`schema_version` in each rule file) evolve independently. The crate version
covers CLI surface, JSON output, and Rust API. The schema version covers the
shape of bundled and contributor-authored rule YAML.

## [Unreleased]

## [0.1.0] - 2026-04-25

First tagged release. Freezes the W2 rules-as-data refactor as a stable,
downloadable reference point.

### Added
- **Detection rules as YAML** — 10 framework-detection rules under
  `rules/builtin/<framework>.yaml`, bundled into the binary via `include_str!`
  at build time. Covers LangChain, LangGraph, CrewAI, AutoGen, OpenAI
  Assistants, Anthropic MCP, Anthropic Agent SDK, AWS Bedrock Agents,
  Vercel AI SDK, and Custom Agents.
- **Scoring rules as YAML** — 11 scoring rules under `rules/scoring/<slug>.yaml`,
  driving every finding emitted today (no-system-prompt, missing-input-validation,
  excessive-admin-permission, etc.). Each rule carries per-framework compliance
  references for NIST AI RMF, ISO/IEC 42001, EU AI Act, and OWASP Agentic Top 10.
- **Closed `Matcher` enum** — explicit primitives (`import_contains`,
  `code_regex`, `package_dep`, `file_present`, `context_signal`) plus combinators
  (`any_of`, `all_of`, `not`). No WASM, Lua, or Rego pluggability.
- **Quarantine error model** — bad rules become `RuleDiagnostic`s printed to
  stderr at engine init, never panic. The scan continues with surviving rules.
- **9-signal allowlist** — `tool_count`, `has_system_prompt`, `autonomy_tier`,
  `has_guardrail{input_validation,output_filtering,rate_limit}`,
  `has_permission{execute,admin,write}`, `data_source_count`,
  `unconfirmed_tool_count`, `has_audit_trail`. Loader and matcher are kept in
  lockstep by an integration test.
- **Schema versioning** — every YAML rule declares `schema_version: "1.0"`;
  unsupported versions are quarantined.
- **Snapshot regression oracle** — 6 fixture-based snapshot tests under
  `snapshots/`, driven by `bash scripts/snapshot.sh verify`. JSON output is
  byte-identical across releases unless explicitly updated.
- **69 unit tests** covering loader validation, matcher evaluation, scoring
  pipeline, and signal-allowlist parity.

### Schema
- Initial `schema_version: "1.0"`. The `extends:` field is reserved (must be
  null) and will gain overlay semantics in a future schema minor bump.

[Unreleased]: https://github.com/cschuman/agent-shield/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/cschuman/agent-shield/releases/tag/v0.1.0
