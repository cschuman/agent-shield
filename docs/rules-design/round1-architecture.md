# Rules Architecture — Round 1 Proposal

**Author**: Backend Architect (Expert #3 of 4)
**Date**: 2026-04-24
**Scope**: Repository structure, distribution, versioning, trust, and lifecycle for the agent-shield rule collection.

---

## Top 3 Architectural Decisions

### Decision 1: Monorepo, Not Split Repo — For Now

The rule collection lives in `cschuman/agent-shield` alongside the CLI. Split to `agent-shield-rules` only when community PRs exceed ~20/month or when the SaaS dashboard requires a rule pipeline independent of CLI releases. At the current stage (sole maintainer, pre-GitHub) a separate repo adds coordination overhead with zero benefit.

**The rejection case for split repos at this stage**: Semgrep split early because they had a Go CLI team and a rules community that were completely different populations. We don't have that yet. Trivy's vulnerability DB is separate because it updates multiple times per day from CVE feeds — our rules update on human timescales. Copy Falco's early model instead: rules lived in `falco` monorepo until the community outgrew it (around v0.15, two years in).

**Community rule repos**: Yes, support arbitrary third-party rule sets from day one, but via a pull model only. `agent-shield scan --rules ./my-rules/` or `agent-shield scan --rules https://github.com/acme/agent-shield-rules@v1.2.0`. The CLI fetches, validates, and caches third-party rules locally. We never auto-pull community rules without an explicit user invocation. This is the Semgrep `--config p/owasp-top-ten` model, which we copy wholesale — it made Semgrep's registry a defensible distribution moat.

---

### Decision 2: Bundled-Primary, Updatable-Secondary Distribution

Rules ship in two layers:

1. **Bundled rules**: compiled into the binary at build time via `include_str!()` / `include_bytes!()`. These are the rules that existed at the CLI's release tag. They work offline forever.
2. **Registry rules**: downloaded to `~/.config/agent-shield/rules/` on `agent-shield update-rules`. The CLI checks for a cached registry bundle first; if absent or stale (>7 days), it falls back to bundled.

This mirrors `cargo-audit`'s advisory-db model: the binary ships a snapshot; `cargo audit fetch` refreshes it; the tool works on planes. Do not do lazy first-run downloads (the Homebrew `brew update` model) — it breaks CI, introduces network dependencies into security tooling, and degrades trust.

**Latency tradeoff**: bundled rules add ~200–500 KB to the binary. Acceptable given LTO + strip already optimize aggressively (see `Cargo.toml` release profile).

---

### Decision 3: Rule-Set Versioning Is Independent of CLI Versioning, With Explicit Compatibility Bounds

A rule file declares which CLI schema version it requires. The CLI declares which schema versions it can parse. These are the only coupling points — a rule's version (v1.4.2) has no meaning relative to the CLI's version (0.7.1) except through the schema compatibility field.

---

## Directory Layout

```
agent-shield/
├── Cargo.toml
├── src/
│   └── ...
├── rules/                          # The rule collection root
│   ├── registry.yaml               # Rule index: IDs, paths, checksums, tags
│   ├── schema/
│   │   └── rule-v1.schema.json     # JSON Schema for rule YAML validation
│   ├── frameworks/                 # Namespace: which AI framework
│   │   ├── langchain/
│   │   │   ├── lc-agent-init.yaml
│   │   │   ├── lc-tool-injection.yaml
│   │   │   └── lc-memory-persistence.yaml
│   │   ├── crewai/
│   │   │   ├── crew-multi-agent-scope.yaml
│   │   │   └── crew-process-hierarchy.yaml
│   │   ├── mcp/
│   │   │   ├── mcp-server-init.yaml
│   │   │   └── mcp-tool-exposure.yaml
│   │   ├── autogen/
│   │   ├── openai-assistants/
│   │   ├── anthropic-agent-sdk/
│   │   ├── aws-bedrock/
│   │   ├── vercel-ai/
│   │   └── custom-agent/
│   ├── compliance/                 # Namespace: which compliance regime
│   │   ├── nist-ai-rmf/
│   │   │   ├── govern-1-1.yaml     # Rule ID matches control ID
│   │   │   └── measure-2-5.yaml
│   │   ├── eu-ai-act/
│   │   │   └── art-9-risk-mgmt.yaml
│   │   ├── iso-42001/
│   │   └── owasp-agentic/
│   │       └── oat-01-prompt-injection.yaml
│   └── corpus/                     # CI test corpus (owned by dx-optimizer)
│       ├── true-positives/
│       └── false-positives/
├── docs/
│   └── rules-design/
│       └── round1-architecture.md
└── .github/
    └── workflows/
        └── rules-ci.yaml
```

**Why dual namespace?** A rule for LangChain's tool injection maps to OWASP Agentic OAT-01 AND NIST AI RMF Govern-1.1. The `frameworks/` namespace is the *detection* axis (where does this fire?); the `compliance/` namespace is the *reporting* axis (what control does it satisfy?). Rules live under `frameworks/` as their canonical home; `compliance/` files are either thin wrappers or symlinks that point to framework rules. The `registry.yaml` index holds the compliance tag mapping so the CLI can query either axis without parsing every file.

---

## Versioning

### Three Version Numbers in Play

| Version | What It Tracks | Owner |
|---|---|---|
| CLI version (`0.7.1`) | Binary capabilities, output format, flags | Cargo.toml, SemVer |
| Rule schema version (`v1`) | Shape of the YAML rule file | `schema/rule-v1.schema.json` |
| Rule set version (`1.4.2`) | Collection of rules at a point in time | `registry.yaml` tag, SemVer |

### Rule YAML Compatibility Header

Every rule file includes:

```yaml
meta:
  id: lc-agent-init-001
  schema_version: "1"           # integer; the rule-v{N}.schema.json it conforms to
  cli_min: "0.5.0"              # earliest CLI that can load this rule
  cli_max: null                 # null = no upper bound yet; set when a pattern breaks
  rule_set_version: "1.4.2"    # version of the rule-set release that shipped this rule
```

The CLI reads `cli_min` and `cli_max` at load time and silently skips rules outside its range. It emits a warning if >10% of rules are skipped, prompting `agent-shield update-rules`.

### Worked Example: Rule Set v1.4.2 with CLI 0.5–0.9

```
Rule: rules/frameworks/langchain/lc-agent-init.yaml
  meta.schema_version: "1"
  meta.cli_min: "0.5.0"
  meta.cli_max: null
  meta.rule_set_version: "1.4.2"

CLI 0.5.0 ships with rule set v1.0.0 bundled.
User runs: agent-shield update-rules
  → downloads rule set v1.4.2 registry bundle
  → CLI 0.5.0 checks: cli_min (0.5.0) <= current (0.5.0) <= cli_max (∞)  ✓
  → rule is loaded and active

User on CLI 0.4.0 runs: agent-shield update-rules
  → downloads same v1.4.2 bundle
  → CLI 0.4.0 checks: cli_min (0.5.0) > current (0.4.0)  ✗
  → rule is skipped; CLI warns "12 rules require CLI >= 0.5.0; run: cargo install agent-shield"

User on CLI 0.9.0 (schema v2 era, hypothetically):
  → schema v1 rules are still parseable via the v1 parser retained in CLI
  → cli_max remains null unless a breaking pattern change forces it to "0.8.99"
```

### Schema Version Bump Protocol

When `rule-v2.schema.json` is introduced:
1. The CLI ships a `v1` parser and a `v2` parser simultaneously.
2. Existing v1 rules are not touched — they continue working via the v1 parser path.
3. New rules target v2 only.
4. After 2 CLI major releases (e.g., CLI 1.0 and 2.0), the v1 parser is deprecated with a warning, then removed at CLI 3.0.

This is exactly how Kubernetes handles API deprecation and how Falco handled its v1→v2 rule schema transition. Do not require a mass migration of existing rules.

---

## Distribution Flow

```
┌─────────────────────────────────────────────────────────────┐
│                  RELEASE & UPDATE FLOW                      │
└─────────────────────────────────────────────────────────────┘

  RULE PR MERGED TO main
          │
          ▼
  rules-ci.yaml runs:
    - Schema validation (ajv, rule-v{N}.schema.json)
    - Regex compilation check (Rust test harness)
    - Corpus true-positive / false-positive sweep
    - Snapshot delta review (new findings vs. baseline)
          │
          ├─► [FAIL] PR blocked, author notified
          │
          ▼ [PASS]
  Rule lands in main. No release yet.
  Rules accumulate until trigger:
    - Weekly cadence (cron), OR
    - Critical detection gap (manual trigger)
          │
          ▼
  Tag: rules-v{X.Y.Z}  (independent of CLI tag)
  GitHub Actions:
    1. Generate registry.yaml with checksums
    2. Pack rules/ into rules-v{X.Y.Z}.tar.gz
    3. Sign bundle (minisign, see Trust section)
    4. Upload to GitHub Releases
    5. Update https://rules.agentshield.dev/latest.json
          │
          ▼
  CLI RELEASE CYCLE (slower, tied to Cargo.toml version):
    - Embed current rule bundle as compile-time bytes
    - cargo build --release bundles rules-v{current}
    - Published to crates.io
          │
          ▼
  USER: agent-shield update-rules
    1. Fetch https://rules.agentshield.dev/latest.json
    2. Compare to ~/.config/agent-shield/rules/installed.json
    3. If newer: download + verify minisign signature
    4. Decompress to ~/.config/agent-shield/rules/
    5. Print summary: "+3 new rules, 1 deprecated"
          │
          ▼
  USER: agent-shield scan .
    Priority: local cache > bundled binary fallback
    Offline: bundled binary rules always available
```

---

## Trust and Signing

### Current State (Regex Only)

Regex patterns against user code are low blast-radius — the worst a malicious regex does is cause catastrophic backtracking (ReDoS) or generate false positives. The CI regex compilation check catches ReDoS patterns. For this phase: no signing required, PR review is sufficient.

### Signing Protocol (for when AST/exec primitives land)

Use **minisign** (the same tool Trivy uses for its vulnerability DB). Not GPG — minisign has a single signing key, deterministic signatures, and a format that's trivial to verify in Rust via the `minisign-verify` crate.

```
Maintainer holds: agent-shield-rules.key (never committed)
Repository holds: agent-shield-rules.pub (committed to repo root)
Every release bundle: rules-v{X.Y.Z}.tar.gz.sig
CLI at load time: minisign_verify(bundle, sig, embedded_pubkey)
```

**Handling malicious PRs**: The barrier is PR review, not cryptography. Rules are plain YAML. The CI harness runs regex compilation in a sandboxed Rust test (no network, no filesystem writes). If we ever add exec-type primitives, those require a separate review label (`exec-primitive`) and a second maintainer approval. The signing key signs what passes review — signing is integrity verification in transit, not a replacement for code review.

**Key rotation**: If the signing key is compromised, bump `schema_version`, which forces a new pubkey embed in the CLI binary. Old CLIs will reject new bundles (correct behavior — the old pubkey is untrusted).

---

## Release Cadence

| Release Type | Trigger | SemVer Semantics |
|---|---|---|
| Rule set patch (1.4.x) | New rules, regex tweaks — no schema change | PATCH |
| Rule set minor (1.x.0) | New pattern types within existing schema — backward compat | MINOR |
| Rule set major (x.0.0) | Schema version bump, breaking field renames | MAJOR |
| CLI release | New subcommands, schema parser added, binary features | Cargo.toml SemVer |

**Cadence**: Rule set releases weekly via cron if any rules merged since last tag. CLI releases monthly or on significant feature gates. These clocks are independent — a CLI 0.8.0 release can ship with rule set v2.1.0 bundled.

---

## CI Bar for Rule PRs

The `rules-ci.yaml` workflow gates on all four:

1. **Schema validation**: `ajv validate --schema rules/schema/rule-vN.schema.json --data <rule.yaml>` — zero tolerance.
2. **Regex compilation**: A Rust test binary compiles every `CodePattern` regex; any compilation failure or >100ms match time on a 10KB synthetic file fails the build.
3. **Corpus sweep**: Every rule must produce ≥1 true positive match against the `corpus/true-positives/` fixtures and 0 matches against `corpus/false-positives/`. The dx-optimizer owns the corpus structure; this CI step consumes it.
4. **Snapshot delta**: A comment is posted to the PR showing exactly which new findings appear and which disappear against a pinned reference corpus. Human reviewer approves the delta is intentional.

**What is NOT enforced by CI**: quality of the rule description, compliance mapping accuracy, or whether the rule is useful. That's PR review judgment.

---

## The Deprecation Problem

When LangChain 0.3 breaks 0.2 import patterns, rules express it as:

```yaml
meta:
  id: lc-agent-init-001
  deprecated: false
  # When deprecating:
  # deprecated: true
  # deprecated_reason: "LangChain 0.3 changed import path to langchain_core.agents"
  # deprecated_since_rule_set: "2.1.0"
  # successor_id: lc-agent-init-002

detection:
  target_versions:            # applies to detected package version ranges
    langchain: ">=0.1.0,<0.3.0"
  patterns:
    - type: import
      value: "from langchain.agents"
```

The `target_versions` field (owned by the security-auditor's schema design) tells the CLI to skip this rule if the scanned project's `package.json` or `pyproject.toml` shows `langchain>=0.3.0`. The new rule `lc-agent-init-002` targets `>=0.3.0` with the updated pattern.

**Sunset protocol**:
- `deprecated: true` can be set without removing the rule — deprecated rules still fire but emit a note.
- After 2 rule-set major versions, deprecated rules move to `rules/deprecated/` and are excluded from the default scan.
- After 4 rule-set major versions, deprecated rules are permanently removed.

This matches `cargo-audit`'s approach to withdrawn advisories: they stay in the DB for history but are not surfaced by default.

---

## Biggest Open Question I'm Punting to Round 2

**How does `registry.yaml` stay fast when the rule collection reaches 500+ rules?**

The current design has the CLI parse `registry.yaml` on every scan to resolve which rules apply to the detected frameworks. At 10 rules this is trivial. At 500 rules with tag filtering, compliance mapping lookups, and version compatibility checks, it becomes a non-trivial startup cost. The options are: (a) pre-compiled binary index baked into the CLI at release time, (b) SQLite-backed local rule cache, (c) lazy loading by framework tag. I have a preference (SQLite, à la Trivy's bolt-based DB) but the Rust engine expert needs to weigh in on what the runtime loading trait design can support before I commit to a schema. Round 2 should resolve this with the rust-pro.
