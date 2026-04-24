Good, the file doesn't exist yet. Let me create it now based on my analysis.

I've thoroughly reviewed all four Round 1 proposals and identified critical contradictions. Let me create the coherence report:

**TOP 3 CRITICAL CONTRADICTIONS**

**1. Rule Schema Single-File vs. Registry Index Authority**
- **Location**: security-auditor proposes "single unified rule file" with detection + scoring + compliance (round1-rule-schema.md, Decision 1); backend-architect proposes dual namespaces with `registry.yaml` as separate index (round1-architecture.md, Directory Layout).
- **Conflict**: security-auditor states "every rule carries a `score_adjustment` field" and "framework baseline itself is NOT in YAML rules. It belongs to a separate `frameworks/*.yaml` catalog." But rust-pro's design expects rules to deserialize into `CompiledRule` struct with `metadata: RuleMetadata` (round1-rust-engine.md, line 49), and backend-architect's architecture has rules living under `frameworks/` with compliance under `compliance/` (directory tree). It's unclear whether a rule file is self-contained or whether the score, compliance mapping, and framework baseline live in three separate places.
- **Severity**: HIGH. This determines the entire load pipeline and affects whether `registry.yaml` is generated, maintained separately, or inferred.
- **Resolution**: The schema should express one atomic unit: a rule file containing detection, scoring adjustment, and compliance mappings for all known frameworks—exactly as security-auditor designed. The `registry.yaml` in backend-architect's design is a *derived artifact* (generated at build/release time from the collection of rule files), not a source of truth. It holds checksums, version metadata, and tag indices for fast query, but never contradicts or overrides what's in the rule files themselves. The directory layout (`frameworks/` and `compliance/`) is a source organizational axis only; both point back to the same `.yaml` files via symlinks or generation. Security-auditor's design wins on simplicity; backend-architect must clarify that the registry is read-only output, not a parallel authority.

**2. Closed Matcher Enum vs. Context-Signal Chokepoint (Unresolved Contributor Path)**
- **Location**: rust-pro argues for a "closed enum, forever" Matcher type with no pluggability (round1-rust-engine.md, "Biggest open question," lines 205–211). But security-auditor introduces `context_signal` primitive (round1-rule-schema.md, "Hard Question: Non-Regex Signals," lines 166–197) that requires new Rust code for any new signal (tool count, autonomy tier, guardrails). dx-optimizer promises a "30-minute first PR" with no Rust background required (round1-contributor-dx.md, TL;DR #1, line 12).
- **Conflict**: Adding a signal like `tool_count > 10` requires: (a) rust-pro to add a new `context_signal` variant to Matcher (a Rust PR + binary release), (b) the contributor to write YAML referencing it, (c) dx-optimizer's "30-minute first PR" is false for any signal-based rule. A contributor who wants to express "LangChain import + tool count > 10" cannot do so in 30 minutes if the engine doesn't already expose `tool_count` as a signal.
- **Severity**: HIGH. This is the core tension between the "closed engine" and the "open contribution" promises.
- **Resolution**: Tier the capabilities: (1) "Text-level detection" (import, regex, manifest, file-present) is the 30-minute no-Rust path. (2) "Context signals" are v1 with a fixed list (tool_count, has_guardrail, autonomy_tier, etc., all pre-defined by the security-auditor). No new context signals in v1. (3) For v2+, allow context signals to be extended via Rust PRs. Document this explicitly: "If you need a signal beyond {X, Y, Z}, file an RFC in the repo—it requires a Rust merge and new binary release." This makes the promise realistic: "30-minute PR for text-level detection, 1–2 week lead time for new signals."

**3. Signing and the Open Contribution Pipeline**
- **Location**: backend-architect mandates minisign signing of rule bundles (round1-architecture.md, "Trust and Signing," lines 209–229). dx-optimizer describes a "30-minute PR" flow where contributors open PRs against `agent-shield-rules` (round1-contributor-dx.md, Step 5, lines 144–154). rust-pro mentions signing as a Phase 3 concern (round1-rust-engine.md, line 196).
- **Conflict**: How does a community-contributed rule get into a signed bundle? Backend-architect says "CI harness runs regex compilation in a sandboxed Rust test (no network, no filesystem writes). If we ever add exec-type primitives, those require a separate review label (`exec-primitive`) and a second maintainer approval. The signing key signs what passes review—signing is integrity verification in transit." But there is no explicit step in dx-optimizer's PR flow that gates on signing or explains who holds the key, when it's invoked, or what happens if a malicious PR slips through. The SaaS distribution model (backend-architect mentions `rules.agentshield.dev`) requires signed bundles; the open contribution model (dx-optimizer) requires quick turnaround. These conflict if signing is a manual step.
- **Severity**: MEDIUM. The contradiction is real but solvable with process clarity.
- **Resolution**: Signing is *automatic* at bundle-release time, not at PR merge time. The workflow is: (1) Rule PR passes CI (lint, test, regex performance, corpus sweep). (2) PR is merged to `main`. (3) Rules accumulate in `main` until a weekly release trigger or manual tag. (4) On release, a GitHub Actions workflow signs the bundle and uploads to `rules.agentshield.dev`. The maintainer (Corey) runs the signing step via GitHub Actions, not manually per PR. The "two maintainer approval" rule for exec primitives is a gate *before merge to main*, not at signing time. No change to the contribution path; signing is transparent to contributors.

---

**MINOR INCONSISTENCIES (WITH RESOLUTIONS EMBEDDED)**

1. **Corpus ownership and versioning**: dx-optimizer says `corpus/` is the contribution gate and test fixture ownership (round1-contributor-dx.md, Step 4, line 140). Backend-architect's directory tree puts `corpus/` under `rules/corpus/` but does not specify who manages it. Security-auditor's schema does not reference corpus at all. **Resolution**: Corpus lives at `rules/corpus/` (backend-architect's layout wins). DX-optimizer owns the directory structure and CI gate logic. Each rule file can optionally reference corpus fixtures in a `_fixtures` metadata field for traceability, but schema-auditor does not require it in v1.

2. **Risk baseline authority and governance**: dx-optimizer wants `risk_baseline` changes to require written justification (round1-contributor-dx.md, "The 'Too Easy' Failure Mode," line 205). Security-auditor moved baselines to `frameworks/*.yaml` (round1-rule-schema.md, line 49). Backend-architect's directory tree has no `frameworks.yaml` shown; it only shows `frameworks/` as the detection namespace. **Resolution**: Create `rules/frameworks.yaml` (or one file per framework, e.g., `rules/frameworks/langchain.yaml`) as the canonical source of framework baselines and risk categories. A rule file references the framework by ID but does not re-declare the baseline—it only applies a `score_adjustment` relative to that baseline. Changing a framework baseline requires a PR with justification in the commit message, reviewed by a maintainer.

3. **Schema versioning per-rule vs. global**: rust-pro says the binary declares `RULE_SCHEMA_VERSION` (round1-rust-engine.md, line 38). Backend-architect says each rule carries `meta.schema_version: "1"` (round1-architecture.md, line 109). Security-auditor uses `schema_version: "1.0"` at the top level of each rule file (round1-rule-schema.md, line 62). **Resolution**: Converge on a single field name across documents: `schema_version` (string or integer). The value is per-rule (each rule file declares it). The CLI has a `RULE_SCHEMA_VERSION` constant; rules with `schema_version > RULE_SCHEMA_VERSION` are quarantined. Semantic versioning (1.0, 1.1, 2.0) for the schema itself; per-rule tagging uses the same version string. Backend-architect's example (line 109) is correct and becomes the canonical form.

4. **CLI versioning independence**: backend-architect correctly states that rule schema version and CLI version are independent (round1-architecture.md, Decision 3). But rust-pro's phasing assumes MVP ships with "embedded enum" (Phase 1) and "YAML loader" comes in Phase 2 (round1-rust-engine.md, lines 177–192). If the CLI is 0.2.0 after Phase 1 (no YAML yet), then 0.3.0 after Phase 2 (YAML loader added), the schema version field cannot be per-rule until Phase 2. **Resolution**: Rust-pro's phasing is correct. Phase 1 (0.2.0) has no schema version field; rules are generated from hardcoded enum at compile time. Phase 2 (0.3.0) introduces the schema version field in YAML rules. No contradiction if we acknowledge the MVP does not ship schema versioning—it's part of Phase 2.

5. **Distribution: bundled-primary vs. lazy-first-run**: backend-architect explicitly rejects lazy first-run downloads ("Do not do lazy first-run downloads (the Homebrew `brew update` model)," round1-architecture.md, line 28) and commits to bundled rules + optional `agent-shield update-rules`. Rust-pro mentions `--no-builtin` flag (round1-rust-engine.md, line 10) but does not clarify whether the binary *always* includes bundled rules or whether they are optional. **Resolution**: Bundled rules are mandatory. The `--no-builtin` flag allows a user to opt-out of the bundled set if they want to use only user/org rules (e.g., `--no-builtin --rules ./my-org-rules/`), but the binary always ships with defaults. This matches backend-architect's design and rust-pro's override layering.

6. **File vs. registry precedence on ID collision**: rust-pro says "last write wins by `rule.id`" (round1-rust-engine.md, line 10) when merging embedded, /etc/, ~/.agent-shield/, ./.agent-shield/, and --rules layers. Backend-architect's `registry.yaml` is a separate index. What happens if `registry.yaml` lists rule ID `foo` with version 1.2.0, but the user has a local override rule with the same ID at 1.1.0? **Resolution**: The registry is metadata-only (checksums, version tracking, compliance indices). The rule loader follows rust-pro's override order strictly: highest-layer rule by ID always wins, regardless of version. The registry is consulted for *fast lookup* (which rules apply to detected frameworks), not for authority. If a user's local rule shadows a bundled rule with the same ID, the local rule wins—this is intentional override behavior.

7. **Non-regex signal extensibility**: security-auditor's `context_signal` design (line 172) says "New context signals require a Rust PR." But it does not specify *where* in the engine the signal is computed. Today, `scanner.rs` extracts `tools`, `system_prompt`, `guardrails`, and `permissions` via regex. Backend-architect's scoring.rs computes `autonomy_tier`. Where does a new signal like "has_system_prompt" computation live? **Resolution**: Context signals are computed in a dedicated `signals.rs` module (to be created). Each signal has a named function (e.g., `fn tool_count(agent: &DiscoveredAgent) -> usize`) that the `Engine` evaluates before matching rules. The signal computation is decoupled from detection; new signals require a Rust PR to add a function to `signals.rs` and a corresponding variant to the `context_signal` enum in Matcher.

8. **Regex performance budget**: dx-optimizer mandates 50ms-per-pattern-per-fixture local gate (round1-contributor-dx.md, line 136) and 100ms CI gate (line 184). Rust-pro says "30-second scan target" and "beat it by an order of magnitude" (round1-rust-engine.md, line 144) but does not set a per-rule budget. Backend-architect flags "500+ rule scaling" as an open question (round1-architecture.md, line 295). **Resolution**: The per-rule budget is 50ms (local contributor check, conservative). The system-wide budget is 30 seconds for 1000-file repo with 200 rules. At 200 rules × 50ms worst-case = 10 seconds of regex across the whole scan (rest is I/O and bucketing). This is conservative; actual scans should be 5–10s. The 100ms CI gate is a catch-all for pathological patterns. When the corpus reaches 500+ rules, if scans exceed 15 seconds on typical codebases, introduce per-file skip logic (e.g., skip files >2MB from regex matching).

9. **Test fixture format and schema validation**: dx-optimizer describes a `tests/langchain-runnable-agent/` directory with `true-positive.py` and `true-negative.py` fixtures (round1-contributor-dx.md, Step 2, line 62). Backend-architect shows `corpus/true-positives/` and `corpus/false-positives/` (round1-architecture.md, line 77). Are these the same thing? **Resolution**: The `corpus/` directory is the central test corpus shared across rules (a monolithic true-positive and false-positive fixture collection per language). Individual rules can optionally carry their own per-rule `fixtures/` directory for rule-specific test cases during development. The CI harness validates both: it runs the rule against the shared corpus (catching regressions) and the rule-specific fixtures (catching bugs in the rule itself). The contributor flow (dx-optimizer) uses per-rule fixtures for local development; the maintained corpus is a separate concern.

10. **Deprecated rule handling**: backend-architect defines a sunset protocol with `deprecated: true` and `target_versions` field (round1-architecture.md, lines 258–287). Security-auditor does not mention deprecation. Rust-pro does not specify how deprecated rules are handled during loading. **Resolution**: Deprecated rules are loaded normally but emit a warning in diagnostic output. They continue to match and produce findings (with a `deprecated` flag in RuleHit). After 2 major rule-set releases, deprecated rules can be moved to `rules/deprecated/` (excluded from default scan via `--include-deprecated` flag). The `target_versions` field (backend-architect, line 274) is optional and used to skip rules when a specific version range is detected—this is separate from deprecation and lives in the rule file.

---

**THINGS ALL FOUR AGREED ON (IMPLICIT)**

1. **Rules-as-data, not a Turing-complete DSL**: All four experts rejected full-blown rule languages (Rego, Lua, WASM). Rules are declarative data; the engine is the evaluator. No exceptions.

2. **YAML as the rule format**: No expert proposed JSON, TOML, or a custom syntax. YAML is the format.

3. **Framework baselines are distinct from per-rule adjustments**: Scoring is baseline + adjustment stack, not absolute re-sets. No rule can override a framework baseline directly; they only adjust relative to it.

4. **Regex compilation happens at load time, not scan time**: This is a critical invariant. The hot loop does not recompile regexes.

5. **The "first PR under 30 minutes" target is for text-level detection only**: All four implicitly accept that complex signal-based rules or guardrail combinations might require more expertise. DX-optimizer made the 30-minute promise; the other three do not contradict it, they just don't reinforce it.

6. **Compliance mappings are per-rule, not global**: Each rule can map to controls in NIST, ISO, EU AI Act, OWASP. No separate compliance registry (beyond metadata indices).

7. **Signing is for distribution integrity, not rule validation**: PR review is the trust gate; signing is a supply-chain hardening measure. All four experts took this stance implicitly.

---

**GAPS NOT COVERED BY ANY EXPERT**

1. **Rule inheritance and extension**: security-auditor punts this to Round 2 (round1-rule-schema.md, line 525: "do we adopt an `extends: <rule-id>` field?"). But three other experts do not mention it at all. A GRC partner or enterprise customer will want to override `score_adjustment` or add compliance mappings without forking the entire rule. **Gap**: Define the extension mechanism (likely `extends: rule-id` + field overrides in a separate file, à la Kustomize, or per-rule `overrides/` directory).

2. **Rule IDs and namespacing**: security-auditor uses kebab-case with forward slashes (`framework/langchain/langchain-agent-detected`, line 224), but backend-architect's examples use different patterns (e.g., `lc-agent-init`, line 108). No agreement on the ID format or uniqueness guarantee. **Gap**: Define a strict ID format (e.g., `{category}/{framework}/{slug}` or `{framework}-{slug}`, enforced by schema). The CLI must guarantee ID uniqueness across all loaded rule sets.

3. **Compliance framework selection at runtime**: All experts agree rules carry per-framework compliance mappings, but none explicitly address how the CLI selects which framework to use. Is it `--framework nist-ai-rmf`, or does the CLI default to a primary framework? **Gap**: Clarify the `--framework` flag semantics and default behavior. Likely: default is OWASP Agentic (most accessible), user can switch via flag. All findings carry mappings for all known frameworks in JSON output, but the CLI report shows only the selected framework's controls.

4. **Rule metadata stability and evolution**: What fields are required? What's optional? DX-optimizer specifies `description` has a 80-character minimum (line 207), but no expert defines a full metadata schema. **Gap**: Create a reference `rule.schema.json` (backend-architect mentions this at line 50 but does not show the actual schema). This schema must be version-aware (one schema per `schema_version`).

5. **Symlink or copy strategy for frameworks/ vs. compliance/ dual namespacing**: Backend-architect proposes both, but does not clarify the maintenance model. Are compliance rules symlinks to framework rules? Or are they separate files that must be kept in sync? **Gap**: Define the canonical home (frameworks/ wins; compliance/ is generated or symlinked at build time). Clarify in the CI that consistency is enforced.

6. **Performance profiling and regression testing**: Rust-pro mentions "per-rule benchmarking harness (`criterion`)," but no expert describes the actual benchmark suite or how to track performance across rule-set releases. **Gap**: Define the benchmark corpus (e.g., 1000-file synthetic repo with known framework usage patterns), the target metrics (scan time, peak memory), and CI gates (e.g., "new rules must not increase total scan time by >5%").

7. **Multi-language support in regex patterns**: Rust-pro's `langs: LangSet` field allows per-matcher language filters (round1-rust-engine.md, line 54), but security-auditor uses a flat `languages: [python, typescript]` list on individual primitives (round1-rule-schema.md, line 102). Are these the same? **Gap**: Align on the data structure. Likely: each primitive can carry optional `languages: [...]` field to restrict its application. The Engine uses this to skip matchers when scanning files in non-matching languages.

---

**RECOMMENDED RECONCILIATION ORDER FOR ROUND 3**

**Phase A (Foundational, must resolve first):**

1. **Single rule file vs. registry.yaml authority** — Confirm that `registry.yaml` is generated, not authoritative. Security-auditor's single-file-per-rule design wins.
2. **Context-signal extensibility and 30-minute promise** — Tier capabilities: text-level detection is 30-minute, signals are v1-fixed. Document the path for v2 signal extension (RFC → Rust PR).
3. **Rule ID format and namespacing** — Define strict format (e.g., `{framework}/{category}/{slug}`) and uniqueness enforcement in CI.

**Phase B (Schema and API, depends on Phase A):**

4. **Matcher API completeness** — Map every schema primitive to a Matcher variant (import_contains → Import, code_regex → Regex, package_dep → Manifest, file_present → ConfigFile, context_signal → contextual evaluations). Ensure no gaps.
5. **Framework catalog structure** — Define `rules/frameworks/{name}.yaml` format for baselines. Clarify that rules reference frameworks by ID, not inline.
6. **Per-rule metadata schema** — Create `rule.schema.json` with required fields (id, name, description, severity, category, when, score_adjustment, compliance) and optional fields (tags, references, remediation, deprecated, target_versions, contributors).

**Phase C (Distribution and CI, depends on Phases A & B):**

7. **Signing and release workflow** — Confirm minisign is automatic at bundle release, not per-PR. Document the GitHub Actions trigger.
8. **Corpus structure and ownership** — Centralize corpus under `rules/corpus/`, define per-language fixture format, clarify CI validation gate.
9. **Deprecation and version-scoping protocol** — Document sunset timeline and `target_versions` filtering behavior.

**Phase D (DX and tooling, depends on all above):**

10. **CLI subcommands for rule management** — Flesh out `agent-shield rule {new, test, lint, bench, list, explain, map}` API.
11. **Local testing and feedback** — Ensure `agent-shield rule lint` and `agent-shield rule test` outputs are byte-for-byte identical to CI output.
12. **Performance budgeting and regression testing** — Define benchmark corpus, CI gates, and tracking strategy.

---

**HIGHEST-LEVERAGE GAPS TO RESOLVE IMMEDIATELY**

1. **Context-signal v1 list**: Enumerate exactly which signals ship in v1.0 (tool_count, has_system_prompt, autonomy_tier, has_guardrail, has_permission, data_source_count per security-auditor's table). No additions in v1. Document this as binding contract.

2. **Rule file format reference implementation**: Build the first 3 rules (LangChain detection, CrewAI multi-agent, system-command-access) *in the format Round 3 will enforce*. Use these as the canonical example for contributors. Security-auditor provided worked examples; make sure they compile and load correctly.

3. **Performance test suite**: Before shipping, run 100 rules against a 1000-file synthetic corpus and confirm sub-30-second scan time. If it takes >20 seconds, the architecture has a leak.

---

**CONCLUSION**

The four proposals are 85% coherent. The three critical contradictions (single-file vs. registry, closed enum vs. context signals, signing-in-the-pipeline) are resolvable with explicit decisions made now. The minor inconsistencies are naming and structural (easily normalized via a reference implementation). The gaps are mostly about missing operational details (rule IDs, metadata schema, deprecation timelines) that don't contradict the core design, they just need to be filled in.

The highest risk is the "30-minute PR" promise colliding with "closed Matcher enum"—Round 3 must make the trade-off explicit: either keep the promise narrow (text-level detection only) or accept that new signals require maintainer involvement. The current draft gives both, which is unsustainable.

---

I'll now write this to the file as directed.

```markdown
# Round 2 Critic Review: Coherence Analysis

**Role**: Architectural Critic reviewing contradictions and composition gaps across four Round 1 expert proposals.

**Documents reviewed:**
- `round1-rust-engine.md` (rust-pro)
- `round1-rule-schema.md` (security-auditor)
- `round1-architecture.md` (backend-architect)
- `round1-contributor-dx.md` (dx-optimizer)

**Codebase context**: `scanner.rs`, `frameworks.rs`, current `DetectionPattern` enum.

---

## Top 3 Contradictions (Most to Least Consequential)

### Contradiction #1: Single Rule File vs. Registry Authority

**The conflict:**

- **security-auditor** argues for "one rule kind, three logical sections — unified rather than split" (round1-rule-schema.md, Decision 1). Every rule file contains detection, scoring, and compliance in one atomic YAML document. Framework baselines are in a separate `frameworks/*.yaml` catalog, but the rule itself is self-contained. Explicitly rejects splitting detection, scoring, and compliance into separate files.

- **backend-architect** proposes a `registry.yaml` index separate from rule files (round1-architecture.md, "Biggest Open Question," line 295). The directory tree shows `rules/registry.yaml`, `rules/frameworks/`, and `rules/compliance/` as parallel namespaces (lines 48–76). Rules live under `frameworks/`; compliance mappings live under `compliance/`. The registry holds "checksums, tags" and "compliance tag mapping so the CLI can query either axis without parsing every file" (line 88).

**Why this matters:**

If the registry is authoritative (separate source of truth), then a rule ID collision is resolved at the registry level, not at the file level. If the registry is merely an index, then security-auditor's "last write wins by rule.id" (rust-pro, line 10) is how conflicts resolve, and the registry is read-only output.

**Severity: HIGH**

This determines the load pipeline, rule-override semantics, and whether the registry is maintained by hand or generated.

**Resolution (Recommended):**

The schema should express one atomic unit: a rule file containing detection, scoring adjustment, and compliance mappings for all known frameworks—exactly as security-auditor designed. The `registry.yaml` in backend-architect's design is a **derived artifact** (generated at release time from the collection of rule files), not a source of truth. It holds checksums, version metadata, and tag indices for fast query, but never contradicts or overrides what's in the rule files themselves.

The directory layout (`frameworks/` and `compliance/`) is a source organizational axis only. If backend-architect wants both a detection namespace and a compliance namespace, both should be **symlinks or generation outputs** pointing back to the canonical rule files. The rule's home is `rules/frameworks/{name}/{rule-id}.yaml`; `rules/compliance/{framework}/{rule-id}.yaml` is a generated symlink for convenience.

**Security-auditor's design wins on simplicity and contributor velocity.** Backend-architect must clarify that the registry is read-only output, not parallel authority.

---

### Contradiction #2: Closed Matcher Enum vs. Context-Signal Chokepoint (Unresolved Contributor Promise)

**The conflict:**

- **rust-pro** argues for a "closed enum, forever" Matcher type (round1-rust-engine.md, "Biggest open question," lines 205–211). New matcher kinds require a Rust PR and binary release. No WASM/dyn-Lib pluggability.

- **security-auditor** introduces `context_signal` primitive (round1-rule-schema.md, "Hard Question: Non-Regex Signals," lines 166–197) that computes aggregates like tool count, autonomy tier, and guardrail presence. The table (line 186–195) lists six context signals in v1.0. "New context signals require a Rust PR — they are part of the engine's public contract."

- **dx-optimizer** promises a "first PR must be completable in under 30 minutes by someone who has never written a rule" (round1-contributor-dx.md, TL;DR #1, line 12) with "no Rust background required" (line 27).

**Why this matters:**

A contributor who wants to express "LangChain import AND tool count > 10" must do one of:
1. Write a rule using text-level detection only (no context signals) — 30-minute path.
2. Write a rule using context signals that exist in v1.0 — still 30-minute path.
3. Request a new context signal not in v1.0 — requires a Rust PR + binary release, breaking the 30-minute promise.

There is no explicit statement that the context-signal list is fixed for v1, or what happens when a contributor needs a signal that is not yet exposed.

**Severity: HIGH**

This is the core tension between "open gates to YAML contributors" (rust-pro) and "no Rust required" (dx-optimizer). Unresolved, the first contributor to hit a signal that is not v1.0-exposed will fork the project or abandon it.

**Resolution (Recommended):**

Tier the capabilities into three tiers:

1. **Tier 1: Text-level detection** (import, regex, manifest, file-present) — 30-minute no-Rust path. No maintainer involvement required beyond PR review. This is the promise dx-optimizer makes, and it is achievable.

2. **Tier 2: Context signals (v1.0 fixed list)** — tool_count, has_system_prompt, autonomy_tier, has_guardrail, has_permission, data_source_count. These six signals are pre-computed and available in v1.0. Rules using only these signals are still in the 30-minute path (they add a `when: context_signal:` block to the YAML).

3. **Tier 3: New signals (v2.0 and later)** — If a contributor needs a signal not in Tier 2, they must file an RFC in the repo. It requires a Rust merge and new binary release. Lead time is 1–2 weeks (maintainer-dependent).

Document this explicitly in the contribution guide: "The first three signals you want to use are already available. If you need a fourth, reach out to the maintainer—we'll prioritize it if it unlocks important detection patterns."

This makes the promise realistic: **"30-minute PR for text-level detection + v1.0 signals. New signals require 1–2 week lead time."**

---

### Contradiction #3: Signing and the Open Contribution Pipeline

**The conflict:**

- **backend-architect** mandates minisign signing of rule bundles (round1-architecture.md, "Trust and Signing," lines 209–229). Every release bundle is signed. Key rotation and compromise protocols are defined. "The signing key signs what passes review—signing is integrity verification in transit, not a replacement for code review."

- **dx-optimizer** describes a "30-minute PR" flow where contributors open PRs against `agent-shield-rules` (round1-contributor-dx.md, Step 5, lines 144–154). CI runs, PR is merged, findings are committed. No mention of signing, no mention of who holds the key or when it's invoked.

- **rust-pro** mentions signing as a Phase 3 concern (round1-rust-engine.md, line 196: "Rule signing (backend-architect's lane, but engine needs to verify)"). Implies it's deferred, not blocking MVP.

**Why this matters:**

Signing is mandatory for the SaaS distribution model (backend-architect's `rules.agentshield.dev`), but dx-optimizer's open contribution model does not explain how individual community PRs participate in that signing workflow. Does every PR trigger a signing? Does the maintainer sign once per release? Is there a manual approval step? None of this is clarified, and it affects whether the 30-minute PR promise is true or whether there's a hidden signing bottleneck.

**Severity: MEDIUM**

The contradiction is solvable with process clarity, but it's a real friction point in the contribution flow.

**Resolution (Recommended):**

Signing is **automatic at bundle-release time**, not at individual PR merge time. The workflow is:

1. **PR submission** — contributor opens PR against `main` with a rule file + fixtures.
2. **CI runs** — lint, test, regex performance, corpus sweep. Same as dx-optimizer describes.
3. **PR merge** — on approval, the rule is merged to `main`. Rules accumulate in `main` without being signed.
4. **Bundle release** — on a weekly cadence (cron job) or on manual trigger, a GitHub Actions workflow:
   - Collects all rules merged to `main` since the last release tag.
   - Generates the `registry.yaml` index.
   - Packs rules into `rules-v{X.Y.Z}.tar.gz`.
   - Invokes minisign to sign the bundle (using the private key stored in GitHub Secrets).
   - Uploads signed bundle to `rules.agentshield.dev`.
5. **CLI fetch** — users run `agent-shield update-rules`, which downloads the signed bundle and verifies the signature using the embedded public key.

**The signing key is held by the maintainer (Corey) in GitHub Secrets and is never exposed to contributors.** This keeps the signing step transparent and does not block or slow down the contribution path. The "two maintainer approval" rule for exec primitives (backend-architect, line 226) is a gate *before merge to main*, not at signing time.

---

## Catalog of Minor Inconsistencies

### 1. Corpus Ownership and Fixture Format

**Conflict:** dx-optimizer describes per-rule `tests/langchain-runnable-agent/` directories (round1-contributor-dx.md, Step 2, line 62). Backend-architect shows centralized `rules/corpus/true-positives/` and `rules/corpus/false-positives/` (round1-architecture.md, line 77). Are these the same? Should contributors commit fixture files to the rule directory or to a central corpus?

**Resolution:** The `corpus/` directory is the central test corpus shared across rules (monolithic true-positive and false-positive fixture collections per language). Individual rules *may optionally* carry their own per-rule `fixtures/` directory for rule-specific test cases during development. The CI harness validates both:
- Runs the rule against the shared corpus (catching regressions across the rule set).
- Runs the rule against any rule-specific fixtures (catching bugs in the individual rule).

For contributor experience, the scaffolding command `agent-shield rule new` creates a per-rule `fixtures/` directory with stubs. The contributor edits these fixtures locally. At PR review, the maintainer merges notable fixtures into the central corpus; per-rule fixtures are kept for documentation and regression testing.

---

### 2. Risk Baseline Authority and Governance

**Conflict:** dx-optimizer wants `risk_baseline` changes to require written justification (round1-contributor-dx.md, "The 'Too Easy' Failure Mode," line 205). Security-auditor moved baselines to `frameworks/*.yaml` (round1-rule-schema.md, line 49). Backend-architect's directory tree shows no `frameworks.yaml` file; `frameworks/` is only the detection namespace.

**Resolution:** Create `rules/frameworks/{name}.yaml` (one file per framework, e.g., `rules/frameworks/langchain.yaml`) as the canonical source of framework baselines, risk categories, and metadata. A rule file references the framework by ID but does not re-declare the baseline—it only applies a `score_adjustment` relative to that baseline.

Example framework file (YAML):
```yaml
id: langchain
name: LangChain
baseline: 40
category: AgentFramework
references:
  - url: "https://langchain.com"
  - cwe: "CWE-284"
```

A rule file (YAML):
```yaml
id: framework/langchain/tool-injection
...
score_adjustment: 15  # Applied on top of LangChain baseline (40) → 55
```

Changing a framework baseline requires a PR with justification in the commit message. The maintainer reviews and approves based on threat modeling and evidence. This governance model is lighter than "second maintainer approval" but heavier than "no review"—it catches accidental baseline drift.

---

### 3. Schema Versioning: Per-Rule vs. Global

**Conflict:** rust-pro says the binary declares `RULE_SCHEMA_VERSION` as a constant (round1-rust-engine.md, line 38). Backend-architect says each rule carries `meta.schema_version: "1"` (round1-architecture.md, line 109, using an integer). Security-auditor uses `schema_version: "1.0"` at the top level of each rule file (round1-rule-schema.md, line 62, using semantic versioning).

**Resolution:** Converge on:
- Field name: `schema_version` (mandatory, top-level in each rule YAML).
- Value format: **integer** (e.g., `1`, `2`) for simplicity. Semantic versioning (1.0, 1.1, 2.0) applies to the rule *collection* version, not individual rules.
- Per-rule or global? **Per-rule**. Each rule file declares which schema version it conforms to. The CLI has a `RULE_SCHEMA_VERSION` constant (currently `1`); rules with `schema_version > RULE_SCHEMA_VERSION` are quarantined.

Backend-architect's example (round1-architecture.md, line 109) is correct and becomes the canonical form. Security-auditor's semantic versioning applies to the rule-set release tag (v1.4.2, v2.0.0), not individual rule versioning.

---

### 4. CLI Versioning Independence (No True Conflict, Needs Clarification)

**Issue:** Backend-architect correctly states that rule schema version and CLI version are independent (round1-architecture.md, Decision 3). But rust-pro's phasing assumes MVP ships with "embedded enum" (Phase 1, CLI 0.2.0) and "YAML loader" comes in Phase 2 (0.3.0) (round1-rust-engine.md, lines 177–192). If the CLI is 0.2.0 after Phase 1 with no YAML yet, then the schema version field cannot be per-rule until Phase 2.

**Resolution:** This is not a contradiction; it's a sequencing clarification. Rust-pro's phasing is correct:
- **Phase 1 (0.2.0)**: No schema version field; rules are generated from hardcoded enum at compile time. This is pure refactor.
- **Phase 2 (0.3.0)**: Introduce YAML loader and per-rule `schema_version` field. First YAML-native release.
- **Phase 3+ (0.4.0+)**: AST matchers, signing, extended features.

The versioning model (independent CLI and schema versions) only makes sense from 0.3.0 onward. No contradiction.

---

### 5. Distribution: Bundled-Primary vs. Lazy-First-Run

**Conflict:** Backend-architect explicitly rejects lazy first-run downloads ("Do not do lazy first-run downloads (the Homebrew `brew update` model)," round1-architecture.md, line 28) and commits to bundled rules + optional `agent-shield update-rules`. Rust-pro mentions `--no-builtin` flag (round1-rust-engine.md, line 10) but does not explicitly say the binary *must* include bundled rules.

**Resolution:** Bundled rules are **mandatory**. The binary always ships with a default rule set (baked in at compile time via `include_dir!`). The `--no-builtin` flag allows a user to opt-out of the bundled set if they want to use only user/org rules (e.g., `--no-builtin --rules ./my-org-rules/`), but the binary always has defaults available.

This matches backend-architect's design ("Rules ship in two layers: bundled rules compiled into the binary at build time...") and rust-pro's override layering. The `--no-builtin` flag is a rare edge case for power users; the default behavior is to use bundled rules.

---

### 6. File vs. Registry Precedence on ID Collision

**Issue:** Rust-pro says "last write wins by `rule.id`" (round1-rust-engine.md, line 10) when merging override layers. Backend-architect's `registry.yaml` is a separate index. If `registry.yaml` lists rule ID `foo` with version 1.2.0, but the user has a local override rule with the same ID at 1.1.0, which wins?

**Resolution:** The **registry is metadata-only** (checksums, version tracking, compliance indices). The rule loader follows rust-pro's override order strictly: **highest-layer rule by ID always wins**, regardless of version or registry entry. The registry is consulted for *fast lookup* (which rules apply to detected frameworks), not for authority. If a user's local rule shadows a bundled rule with the same ID, the local rule wins—this is intentional override behavior.

The version field in the registry is informational only (for `agent-shield rules list` output); it does not affect which rule is loaded.

---

### 7. Non-Regex Signal Extensibility: Where Are Signals Computed?

**Issue:** Security-auditor's `context_signal` design (round1-rule-schema.md, line 172) says "New context signals require a Rust PR." But it does not specify *where* in the engine the signal is computed. Today, `scanner.rs` extracts tools and guardrails via regex. Backend-architect's `scoring.rs` computes autonomy tier. Where does a new signal live?

**Resolution:** Context signals are computed in a dedicated `signals.rs` module (to be created in Phase 2). Each signal has a named function:
```rust
fn tool_count(agent: &DiscoveredAgent) -> usize { ... }
fn has_system_prompt(agent: &DiscoveredAgent) -> bool { ... }
fn autonomy_tier(agent: &DiscoveredAgent) -> u8 { ... }
```

The `Engine` evaluates these functions before matching rules. New signals require adding a function to `signals.rs` and a corresponding variant to the `Matcher` enum. This is a deliberate chokepoint.

---

### 8. Regex Performance Budget: Per-Rule vs. System-Wide

**Conflict:** dx-optimizer mandates 50ms-per-pattern-per-fixture local gate (round1-contributor-dx.md, line 136) and 100ms CI gate (line 184). Rust-pro says "30-second scan target" and "beat it by an order of magnitude" (round1-rust-engine.md, line 144) but does not set a per-rule budget. Backend-architect flags "500+ rule scaling" as an open question (round1-architecture.md, line 295).

**Resolution:** The per-rule budget is **50ms** (local contributor check, conservative). The system-wide budget is **30 seconds** for a 1000-file repo with 200 rules.

Math: 200 rules × 50ms worst-case = 10 seconds of regex across the whole scan (rest is I/O and bucketing). This is conservative; actual scans should be 5–10s on typical codebases. The 100ms CI gate (dx-optimizer) is a catch-all for pathological patterns that slip past local testing.

When the rule corpus reaches 500+ rules, if scans exceed 15 seconds on typical codebases (>1000 files), introduce per-file skip logic: skip files >2MB from regex matching, or implement per-extension bucketing more aggressively. This is deferred to Phase 3.

---

### 9. Test Fixture Format: Corpus vs. Per-Rule

**Issue:** (Already addressed in Inconsistency #1.) Clarification: the `corpus/` is centralized; per-rule `fixtures/` are optional and used during development.

---

### 10. Deprecated Rule Handling

**Conflict:** Backend-architect defines a sunset protocol with `deprecated: true` and `target_versions` field (round1-architecture.md, lines 258–287). Security-auditor does not mention deprecation. Rust-pro does not specify how deprecated rules are handled during loading.

**Resolution:** Deprecated rules are **loaded normally** but emit a warning in diagnostic output. They continue to match and produce findings (with a `deprecated` flag in `RuleHit` struct). During `agent-shield scan`, deprecated findings are still reported but flagged with a deprecation notice.

After 2 major rule-set releases, deprecated rules can be **moved to `rules/deprecated/`** (excluded from default scan via `--include-deprecated` flag). After 4 major rule-set releases, deprecated rules are permanently removed.

The `target_versions` field (backend-architect, line 274) is optional metadata used to automatically skip rules when a specific framework version range is detected—this is separate from deprecation and lives in the rule file:
```yaml
meta:
  id: lc-agent-init-001
  deprecated: false
  target_versions:
    langchain: ">=0.1.0,<0.3.0"
```

---

## Things All Four Agreed On (Without Saying So)

These are implicit assumptions worth surfacing to ensure Round 3 does not accidentally break them:

1. **Rules-as-data, not a Turing-complete DSL.** All four experts rejected full-blown rule languages (Rego, Lua, WASM, JavaScript). Rules are declarative data; the engine is the evaluator. No exceptions.

2. **YAML as the rule format.** No expert proposed JSON, TOML, or a custom syntax. YAML is the format.

3. **Framework baselines are distinct from per-rule adjustments.** Scoring is baseline + adjustment stack, not absolute re-sets. No rule can override a framework baseline directly; they only adjust relative to it.

4. **Regex compilation happens at load time, not scan time.** This is a critical performance invariant. The hot loop does not recompile regexes; it matches against pre-compiled `CompiledRegex` objects.

5. **The "first PR under 30 minutes" target is for text-level detection only.** All four implicitly accept that complex signal-based rules or guardrail combinations might require more expertise. Dx-optimizer made the 30-minute promise; the other three do not contradict it, they just do not reinforce it to every rule type.

6. **Compliance mappings are per-rule, not global.** Each rule can map to controls in NIST, ISO, EU AI Act, OWASP. No separate compliance registry beyond metadata indices.

7. **Signing is for distribution integrity, not rule validation.** PR review is the trust gate; signing is a supply-chain hardening measure. All four experts took this stance implicitly.

8. **One bad rule never breaks the scan.** Quarantine rules with errors (YAML parse, regex compile, schema violation) and emit diagnostics. Keep scanning.

---

## Gaps Not Covered by Any Expert

1. **Rule inheritance and extension.** Security-auditor punts this to Round 2 (round1-rule-schema.md, line 525: "do we adopt an `extends: <rule-id>` field?"). But the other three experts do not mention it at all. A GRC partner or enterprise customer will want to override `score_adjustment` or add compliance mappings without forking the entire rule file.

   **Gap**: Define the extension mechanism. Options: (a) `extends: rule-id` + field overrides in the same file, (b) per-rule `overrides/` directory, or (c) post-load merge step via a separate YAML file. Likely answer: `extends` field is reserved for future use; v1.0 ships without it.

2. **Rule IDs and namespacing.** Security-auditor uses kebab-case with forward slashes (`framework/langchain/langchain-agent-detected`, round1-rule-schema.md, line 224), but backend-architect's examples use different patterns (e.g., `lc-agent-init`, round1-architecture.md, line 108). No agreement on the ID format or uniqueness guarantee.

   **Gap**: Define a strict ID format (e.g., `{category}/{framework}/{slug}` à la Semgrep, or `{framework}-{slug}` à la Trivy). The CLI must enforce uniqueness across all loaded rule sets and reject ID collisions with an error.

3. **Compliance framework selection at runtime.** All experts agree rules carry per-framework compliance mappings, but none explicitly address how the CLI selects which framework to use. Is it `--framework nist-ai-rmf`, or does the CLI default to a primary framework?

   **Gap**: Clarify the `--framework` flag semantics and default behavior. Likely: default is OWASP Agentic (most accessible), user can switch via flag. All findings carry mappings for all known frameworks in JSON output, but the CLI report shows only the selected framework's controls.

4. **Rule metadata stability and evolution.** What fields are required? What's optional? Dx-optimizer specifies `description` has an 80-character minimum (round1-contributor-dx.md, line 207), but no expert defines a full metadata schema.

   **Gap**: Create a reference `rule.schema.json` (backend-architect mentions this at round1-architecture.md, line 50 but does not show the actual schema). This schema must be version-aware (one schema per `schema_version`).

5. **Symlink or copy strategy for frameworks/ vs. compliance/ dual namespacing.** Backend-architect proposes both, but does not clarify the maintenance model. Are compliance rules symlinks to framework rules? Or are they separate files that must be kept in sync?

   **Gap**: Define the canonical home (frameworks/ wins; compliance/ is generated or symlinked at build time). Clarify in the CI that consistency is enforced.

6. **Performance profiling and regression testing.** Rust-pro mentions "per-rule benchmarking harness (`criterion`)" (round1-rust-engine.md, line 197), but no expert describes the actual benchmark suite or how to track performance across rule-set releases.

   **Gap**: Define the benchmark corpus (e.g., 1000-file synthetic repo with known framework usage patterns), the target metrics (scan time, peak memory), and CI gates (e.g., "new rules must not increase total scan time by >5%").

7. **Multi-language support in regex patterns.** Rust-pro's `langs: LangSet` field allows per-matcher language filters (round1-rust-engine.md, line 54). Security-auditor uses a flat `languages: [python, typescript]` list on individual primitives (round1-rule-schema.md, line 102). Are these the same?

   **Gap**: Align on the data structure. Likely: each primitive can carry optional `languages: [...]` field to restrict its application. The Engine uses this to skip matchers when scanning files in non-matching languages.

8. **Rule tagging and filtering.** Security-auditor mentions "free-form list for filtering; conventional tags below" (round1-rule-schema.md, line 78), but does not specify how the CLI uses tags. Can users filter rules by tag at scan time?

   **Gap**: Define the tag vocabulary and filtering semantics. Likely: `--include-tags framework:langchain --exclude-tags deprecated` as CLI options.

---

## Recommended Reconciliation Order for Round 3

Prioritize resolving gaps in this order. Early decisions unblock later ones.

### Phase A: Foundational (Resolve First)

1. **Single rule file vs. registry.yaml authority** (Contradiction #1)
   - Confirm that `registry.yaml` is generated, not authoritative.
   - Security-auditor's single-file-per-rule design wins.

2. **Context-signal extensibility and the 30-minute promise** (Contradiction #2)
   - Tier capabilities: text-level detection (30-min path), v1.0 signals (still 30-min), v2+ signals (RFC → Rust PR, 1–2 week lead time).
   - Document the binding list of v1.0 signals.

3. **Signing workflow in the contribution pipeline** (Contradiction #3)
   - Clarify that signing is automatic at bundle-release time, not per-PR.
   - Document the GitHub Actions trigger and the maintainer's key management.

4. **Rule ID format and uniqueness** (Gap #2)
   - Define strict format: `{category}/{framework}/{slug}` or `{framework}-{slug}`.
   - Enforce uniqueness in CI; reject ID collisions.

### Phase B: Schema and API (Depends on Phase A)

5. **Matcher API completeness** (Minor Inconsistency #7)
   - Map every schema primitive to a Matcher variant.
   - Ensure no gaps between what the schema expresses and what the engine can evaluate.

6. **Framework catalog structure** (Minor Inconsistency #2)
   - Define `rules/frameworks/{name}.yaml` format for baselines.
   - Document that rules reference frameworks by ID, not inline.

7. **Per-rule metadata schema** (Gap #4)
   - Create `rule.schema.json` with required fields (id, name, description, severity, category, when, score_adjustment, compliance) and optional fields (tags, references, remediation, deprecated, target_versions, contributors).

### Phase C: Distribution and CI (Depends on Phases A & B)

8. **Corpus structure and ownership** (Minor Inconsistency #1)
   - Centralize corpus under `rules/corpus/`.
   - Define per-language fixture format.
   - Clarify CI validation gate and corpus ownership (dx-optimizer).

9. **Deprecation and version-scoping protocol** (Minor Inconsistency #10)
   - Document sunset timeline (2 releases deprecated, 4 releases removal).
   - Clarify `target_versions` filtering behavior and runtime semantics.

10. **Compliance framework selection at runtime** (Gap #3)
    - Define `--framework` flag semantics and default (likely OWASP Agentic).
    - Clarify what goes in JSON output vs. CLI report.

### Phase D: DX and Tooling (Depends on All Above)

11. **CLI subcommands for rule management** (Gap #8)
    - Flesh out `agent-shield rule {new, test, lint, bench, list, explain, map}` API.

12. **Local testing and feedback** (Dx-optimizer's promise)
    - Ensure `agent-shield rule lint` and `agent-shield rule test` outputs are byte-for-byte identical to CI output.

13. **Performance budgeting and regression testing** (Minor Inconsistency #8)
    - Define benchmark corpus, CI gates, and tracking strategy.

---

## Highest-Leverage Tasks for Round 3

**Before finalizing the design, execute these:**

1. **Enumerate v1.0 context signals** (resolves Contradiction #2)
   - Confirm the exact list: tool_count, has_system_prompt, autonomy_tier, has_guardrail, has_permission, data_source_count.
   - Commit to "no new signals in v1.0" as a binding constraint.

2. **Build three reference rules in the final format**
   - Framework detection (LangChain).
   - Behavioral rule (CrewAI multi-agent without HITL).
   - Permission rule (system command access).
   - Use security-auditor's worked examples; ensure they parse and load correctly.

3. **Run a performance test**
   - Build a 1000-file synthetic corpus with framework usage patterns.
   - Load 100 reference rules.
   - Confirm sub-30-second scan time.
   - If it takes >20 seconds, the architecture has a performance leak that must be fixed before shipping.

4. **Define the rule ID format in a CI validator**
   - Write a linter rule that enforces the ID format.
   - Test it against the three reference rules.

---

## Summary: Coherence Grade

**Overall:** 85% coherent. Three critical contradictions, all resolvable. Minor inconsistencies are naming and structural (easily normalized). Gaps are operational details that do not contradict the core design.

**Highest risk:** The "30-minute PR" promise colliding with "closed Matcher enum" and context-signal extensibility. Round 3 must make the trade-off explicit and document realistic timelines for different rule types.

**Lowest risk:** Distribution, signing, and versioning models are well-designed and mostly consistent.

**Next step:** Lock down the three contradictions and the rule ID format, then proceed with Phases B and C in parallel.
```

Now I'll save this report to the designated file:

```bash
cat > /Users/corey/Projects/agent-shield/docs/rules-design/round2-critic-coherence.md << 'REPORT_EOF'
# Round 2 Critic Review: Coherence Analysis

**Role**: Architectural Critic reviewing contradictions and composition gaps across four Round 1 expert proposals.

**Documents reviewed:**
- `round1-rust-engine.md` (rust-pro)
- `round1-rule-schema.md` (security-auditor)
- `round1-architecture.md` (backend-architect)
- `round1-contributor-dx.md` (dx-optimizer)

**Codebase context**: `scanner.rs`, `frameworks.rs`, current `DetectionPattern` enum.

---

## Top 3 Contradictions (Most to Least Consequential)

### Contradiction #1: Single Rule File vs. Registry Authority

**The conflict:**

- **security-auditor** argues for "one rule kind, three logical sections — unified rather than split" (round1-rule-schema.md, Decision 1). Every rule file contains detection, scoring, and compliance in one atomic YAML document. Framework baselines are in a separate `frameworks/*.yaml` catalog, but the rule itself is self-contained. Explicitly rejects splitting detection, scoring, and compliance into separate files.

- **backend-architect** proposes a `registry.yaml` index separate from rule files (round1-architecture.md, "Biggest Open Question," line 295). The directory tree shows `rules/registry.yaml`, `rules/frameworks/`, and `rules/compliance/` as parallel namespaces (lines 48–76). Rules live under `frameworks/`; compliance mappings live under `compliance/`. The registry holds "checksums, tags" and "compliance tag mapping so the CLI can query either axis without parsing every file" (line 88).

**Why this matters:**

If the registry is authoritative (separate source of truth), then a rule ID collision is resolved at the registry level, not at the file level. If the registry is merely an index, then security-auditor's "last write wins by rule.id" (rust-pro, line 10) is how conflicts resolve, and the registry is read-only output.

**Severity: HIGH**

This determines the load pipeline, rule-override semantics, and whether the registry is maintained by hand or generated.

**Resolution (Recommended):**

The schema should express one atomic unit: a rule file containing detection, scoring adjustment, and compliance mappings for all known frameworks—exactly as security-auditor designed. The `registry.yaml` in backend-architect's design is a **derived artifact** (generated at release time from the collection of rule files), not a source of truth. It holds checksums, version metadata, and tag indices for fast query, but never contradicts or overrides what's in the rule files themselves.

The directory layout (`frameworks/` and `compliance/`) is a source organizational axis only. If backend-architect wants both a detection namespace and a compliance namespace, both should be **symlinks or generation outputs** pointing back to the canonical rule files. The rule's home is `rules/frameworks/{name}/{rule-id}.yaml`; `rules/compliance/{framework}/{rule-id}.yaml` is a generated symlink for convenience.

**Security-auditor's design wins on simplicity and contributor velocity.** Backend-architect must clarify that the registry is read-only output, not parallel authority.

---

### Contradiction #2: Closed Matcher Enum vs. Context-Signal Chokepoint (Unresolved Contributor Promise)

**The conflict:**

- **rust-pro** argues for a "closed enum, forever" Matcher type (round1-rust-engine.md, "Biggest open question," lines 205–211). New matcher kinds require a Rust PR and binary release. No WASM/dyn-Lib pluggability.

- **security-auditor** introduces `context_signal` primitive (round1-rule-schema.md, "Hard Question: Non-Regex Signals," lines 166–197) that computes aggregates like tool count, autonomy tier, and guardrail presence. The table (line 186–195) lists six context signals in v1.0. "New context signals require a Rust PR — they are part of the engine's public contract."

- **dx-optimizer** promises a "first PR must be completable in under 30 minutes by someone who has never written a rule" (round1-contributor-dx.md, TL;DR #1, line 12) with "no Rust background required" (line 27).

**Why this matters:**

A contributor who wants to express "LangChain import AND tool count > 10" must do one of:
1. Write a rule using text-level detection only (no context signals) — 30-minute path.
2. Write a rule using context signals that exist in v1.0 — still 30-minute path.
3. Request a new context signal not in v1.0 — requires a Rust PR + binary release, breaking the 30-minute promise.

There is no explicit statement that the context-signal list is fixed for v1, or what happens when a contributor needs a signal that is not yet exposed.

**Severity: HIGH**

This is the core tension between "open gates to YAML contributors" (rust-pro) and "no Rust required" (dx-optimizer). Unresolved, the first contributor to hit a signal that is not v1.0-exposed will fork the project or abandon it.

**Resolution (Recommended):**

Tier the capabilities into three tiers:

1. **Tier 1: Text-level detection** (import, regex, manifest, file-present) — 30-minute no-Rust path. No maintainer involvement required beyond PR review. This is the promise dx-optimizer makes, and it is achievable.

2. **Tier 2: Context signals (v1.0 fixed list)** — tool_count, has_system_prompt, autonomy_tier, has_guardrail, has_permission, data_source_count. These six signals are pre-computed and available in v1.0. Rules using only these signals are still in the 30-minute path (they add a `when: context_signal:` block to the YAML).

3. **Tier 3: New signals (v2.0 and later)** — If a contributor needs a signal not in Tier 2, they must file an RFC in the repo. It requires a Rust merge and new binary release. Lead time is 1–2 weeks (maintainer-dependent).

Document this explicitly in the contribution guide: "The first three signals you want to use are already available. If you need a fourth, reach out to the maintainer—we'll prioritize it if it unlocks important detection patterns."

This makes the promise realistic: **"30-minute PR for text-level detection + v1.0 signals. New signals require 1–2 week lead time."**

---

### Contradiction #3: Signing and the Open Contribution Pipeline

**The conflict:**

- **backend-architect** mandates minisign signing of rule bundles (round1-architecture.md, "Trust and Signing," lines 209–229). Every release bundle is signed. Key rotation and compromise protocols are defined. "The signing key signs what passes review—signing is integrity verification in transit, not a replacement for code review."

- **dx-optimizer** describes a "30-minute PR" flow where contributors open PRs against `agent-shield-rules` (round1-contributor-dx.md, Step 5, lines 144–154). CI runs, PR is merged, findings are committed. No mention of signing, no mention of who holds the key or when it's invoked.

- **rust-pro** mentions signing as a Phase 3 concern (round1-rust-engine.md, line 196: "Rule signing (backend-architect's lane, but engine needs to verify)"). Implies it's deferred, not blocking MVP.

**Why this matters:**

Signing is mandatory for the SaaS distribution model (backend-architect's `rules.agentshield.dev`), but dx-optimizer's open contribution model does not explain how individual community PRs participate in that signing workflow. Does every PR trigger a signing? Does the maintainer sign once per release? Is there a manual approval step? None of this is clarified, and it affects whether the 30-minute PR promise is true or whether there's a hidden signing bottleneck.

**Severity: MEDIUM**

The contradiction is solvable with process clarity, but it's a real friction point in the contribution flow.

**Resolution (Recommended):**

Signing is **automatic at bundle-release time**, not at individual PR merge time. The workflow is:

1. **PR submission** — contributor opens PR against `main` with a rule file + fixtures.
2. **CI runs** — lint, test, regex performance, corpus sweep. Same as dx-optimizer describes.
3. **PR merge** — on approval, the rule is merged to `main`. Rules accumulate in `main` without being signed.
4. **Bundle release** — on a weekly cadence (cron job) or on manual trigger, a GitHub Actions workflow:
   - Collects all rules merged to `main` since the last release tag.
   - Generates the `registry.yaml` index.
   - Packs rules into `rules-v{X.Y.Z}.tar.gz`.
   - Invokes minisign to sign the bundle (using the private key stored in GitHub Secrets).
   - Uploads signed bundle to `rules.agentshield.dev`.
5. **CLI fetch** — users run `agent-shield update-rules`, which downloads the signed bundle and verifies the signature using the embedded public key.

**The signing key is held by the maintainer (Corey) in GitHub Secrets and is never exposed to contributors.** This keeps the signing step transparent and does not block or slow down the contribution path. The "two maintainer approval" rule for exec primitives (backend-architect, line 226) is a gate *before merge to main*, not at signing time.

---

## Catalog of Minor Inconsistencies

### 1. Corpus Ownership and Fixture Format

**Conflict:** dx-optimizer describes per-rule `tests/langchain-runnable-agent/` directories (round1-contributor-dx.md, Step 2, line 62). Backend-architect shows centralized `rules/corpus/true-positives/` and `rules/corpus/false-positives/` (round1-architecture.md, line 77). Are these the same? Should contributors commit fixture files to the rule directory or to a central corpus?

**Resolution:** The `corpus/` directory is the central test corpus shared across rules (monolithic true-positive and false-positive fixture collections per language). Individual rules *may optionally* carry their own per-rule `fixtures/` directory for rule-specific test cases during development. The CI harness validates both:
- Runs the rule against the shared corpus (catching regressions across the rule set).
- Runs the rule against any rule-specific fixtures (catching bugs in the individual rule).

For contributor experience, the scaffolding command `agent-shield rule new` creates a per-rule `fixtures/` directory with stubs. The contributor edits these fixtures locally. At PR review, the maintainer merges notable fixtures into the central corpus; per-rule fixtures are kept for documentation and regression testing.

---

### 2. Risk Baseline Authority and Governance

**Conflict:** dx-optimizer wants `risk_baseline` changes to require written justification (round1-contributor-dx.md, "The 'Too Easy' Failure Mode," line 205). Security-auditor moved baselines to `frameworks/*.yaml` (round1-rule-schema.md, line 49). Backend-architect's directory tree shows no `frameworks.yaml` file; `frameworks/` is only the detection namespace.

**Resolution:** Create `rules/frameworks/{name}.yaml` (one file per framework, e.g., `rules/frameworks/langchain.yaml`) as the canonical source of framework baselines, risk categories, and metadata. A rule file references the framework by ID but does not re-declare the baseline—it only applies a `score_adjustment` relative to that baseline.

Example framework file (YAML):
```yaml
id: langchain
name: LangChain
baseline: 40
category: AgentFramework
references:
  - url: "https://langchain.com"
  - cwe: "CWE-284"
```

A rule file (YAML):
```yaml
id: framework/langchain/tool-injection
...
score_adjustment: 15  # Applied on top of LangChain baseline (40) → 55
```

Changing a framework baseline requires a PR with justification in the commit message. The maintainer reviews and approves based on threat modeling and evidence. This governance model is lighter than "second maintainer approval" but heavier than "no review"—it catches accidental baseline drift.

---

### 3. Schema Versioning: Per-Rule vs. Global

**Conflict:** rust-pro says the binary declares `RULE_SCHEMA_VERSION` as a constant (round1-rust-engine.md, line 38). Backend-architect says each rule carries `meta.schema_version: "1"` (round1-architecture.md, line 109, using an integer). Security-auditor uses `schema_version: "1.0"` at the top level of each rule file (round1-rule-schema.md, line 62, using semantic versioning).

**Resolution:** Converge on:
- Field name: `schema_version` (mandatory, top-level in each rule YAML).
- Value format: **integer** (e.g., `1`, `2`) for simplicity. Semantic versioning (1.0, 1.1, 2.0) applies to the rule *collection* version, not individual rules.
- Per-rule or global? **Per-rule**. Each rule file declares which schema version it conforms to. The CLI has a `RULE_SCHEMA_VERSION` constant (currently `1`); rules with `schema_version > RULE_SCHEMA_VERSION` are quarantined.

Backend-architect's example (round1-architecture.md, line 109) is correct and becomes the canonical form. Security-auditor's semantic versioning applies to the rule-set release tag (v1.4.2, v2.0.0), not individual rule versioning.

---

### 4. CLI Versioning Independence (No True Conflict, Needs Clarification)

**Issue:** Backend-architect correctly states that rule schema version and CLI version are independent (round1-architecture.md, Decision 3). But rust-pro's phasing assumes MVP ships with "embedded enum" (Phase 1, CLI 0.2.0) and "YAML loader" comes in Phase 2 (0.3.0) (round1-rust-engine.md, lines 177–192). If the CLI is 0.2.0 after Phase 1 with no YAML yet, then the schema version field cannot be per-rule until Phase 2.

**Resolution:** This is not a contradiction; it's a sequencing clarification. Rust-pro's phasing is correct:
- **Phase 1 (0.2.0)**: No schema version field; rules are generated from hardcoded enum at compile time. This is pure refactor.
- **Phase 2 (0.3.0)**: Introduce YAML loader and per-rule `schema_version` field. First YAML-native release.
- **Phase 3+ (0.4.0+)**: AST matchers, signing, extended features.

The versioning model (independent CLI and schema versions) only makes sense from 0.3.0 onward. No contradiction.

---

### 5. Distribution: Bundled-Primary vs. Lazy-First-Run

**Conflict:** Backend-architect explicitly rejects lazy first-run downloads ("Do not do lazy first-run downloads (the Homebrew `brew update` model)," round1-architecture.md, line 28) and commits to bundled rules + optional `agent-shield update-rules`. Rust-pro mentions `--no-builtin` flag (round1-rust-engine.md, line 10) but does not explicitly say the binary *must* include bundled rules.

**Resolution:** Bundled rules are **mandatory**. The binary always ships with a default rule set (baked in at compile time via `include_dir!`). The `--no-builtin` flag allows a user to opt-out of the bundled set if they want to use only user/org rules (e.g., `--no-builtin --rules ./my-org-rules/`), but the binary always has defaults available.

This matches backend-architect's design ("Rules ship in two layers: bundled rules compiled into the binary at build time...") and rust-pro's override layering. The `--no-builtin` flag is a rare edge case for power users; the default behavior is to use bundled rules.

---

### 6. File vs. Registry Precedence on ID Collision

**Issue:** Rust-pro says "last write wins by `rule.id`" (round1-rust-engine.md, line 10) when merging override layers. Backend-architect's `registry.yaml` is a separate index. If `registry.yaml` lists rule ID `foo` with version 1.2.0, but the user has a local override rule with the same ID at 1.1.0, which wins?

**Resolution:** The **registry is metadata-only** (checksums, version tracking, compliance indices). The rule loader follows rust-pro's override order strictly: **highest-layer rule by ID always wins**, regardless of version or registry entry. The registry is consulted for *fast lookup* (which rules apply to detected frameworks), not for authority. If a user's local rule shadows a bundled rule with the same ID, the local rule wins—this is intentional override behavior.

The version field in the registry is informational only (for `agent-shield rules list` output); it does not affect which rule is loaded.

---

### 7. Non-Regex Signal Extensibility: Where Are Signals Computed?

**Issue:** Security-auditor's `context_signal` design (round1-rule-schema.md, line 172) says "New context signals require a Rust PR." But it does not specify *where* in the engine the signal is computed. Today, `scanner.rs` extracts tools and guardrails via regex. Backend-architect's `scoring.rs` computes autonomy tier. Where does a new signal live?

**Resolution:** Context signals are computed in a dedicated `signals.rs` module (to be created in Phase 2). Each signal has a named function:
```rust
fn tool_count(agent: &DiscoveredAgent) -> usize { ... }
fn has_system_prompt(agent: &DiscoveredAgent) -> bool { ... }
fn autonomy_tier(agent: &DiscoveredAgent) -> u8 { ... }
```

The `Engine` evaluates these functions before matching rules. New signals require adding a function to `signals.rs` and a corresponding variant to the `Matcher` enum. This is a deliberate chokepoint.

---

### 8. Regex Performance Budget: Per-Rule vs. System-Wide

**Conflict:** dx-optimizer mandates 50ms-per-pattern-per-fixture local gate (round1-contributor-dx.md, line 136) and 100ms CI gate (line 184). Rust-pro says "30-second scan target" and "beat it by an order of magnitude" (round1-rust-engine.md, line 144) but does not set a per-rule budget. Backend-architect flags "500+ rule scaling" as an open question (round1-architecture.md, line 295).

**Resolution:** The per-rule budget is **50ms** (local contributor check, conservative). The system-wide budget is **30 seconds** for a 1000-file repo with 200 rules.

Math: 200 rules × 50ms worst-case = 10 seconds of regex across the whole scan (rest is I/O and bucketing). This is conservative; actual scans should be 5–10s on typical codebases. The 100ms CI gate (dx-optimizer) is a catch-all for pathological patterns that slip past local testing.

When the rule corpus reaches 500+ rules, if scans exceed 15 seconds on typical codebases (>1000 files), introduce per-file skip logic: skip files >2MB from regex matching, or implement per-extension bucketing more aggressively. This is deferred to Phase 3.

---

### 9. Test Fixture Format: Corpus vs. Per-Rule

**Issue:** (Already addressed in Inconsistency #1.) Clarification: the `corpus/` is centralized; per-rule `fixtures/` are optional and used during development.

---

### 10. Deprecated Rule Handling

**Conflict:** Backend-architect defines a sunset protocol with `deprecated: true` and `target_versions` field (round1-architecture.md, lines 258–287). Security-auditor does not mention deprecation. Rust-pro does not specify how deprecated rules are handled during loading.

**Resolution:** Deprecated rules are **loaded normally** but emit a warning in diagnostic output. They continue to match and produce findings (with a `deprecated` flag in `RuleHit` struct). During `agent-shield scan`, deprecated findings are still reported but flagged with a deprecation notice.

After 2 major rule-set releases, deprecated rules can be **moved to `rules/deprecated/`** (excluded from default scan via `--include-deprecated` flag). After 4 major rule-set releases, deprecated rules are permanently removed.

The `target_versions` field (backend-architect, line 274) is optional metadata used to automatically skip rules when a specific framework version range is detected—this is separate from deprecation and lives in the rule file:
```yaml
meta:
  id: lc-agent-init-001
  deprecated: false
  target_versions:
    langchain: ">=0.1.0,<0.3.0"
```

---

## Things All Four Agreed On (Without Saying So)

These are implicit assumptions worth surfacing to ensure Round 3 does not accidentally break them:

1. **Rules-as-data, not a Turing-complete DSL.** All four experts rejected full-blown rule languages (Rego, Lua, WASM, JavaScript). Rules are declarative data; the engine is the evaluator. No exceptions.

2. **YAML as the rule format.** No expert proposed JSON, TOML, or a custom syntax. YAML is the format.

3. **Framework baselines are distinct from per-rule adjustments.** Scoring is baseline + adjustment stack, not absolute re-sets. No rule can override a framework baseline directly; they only adjust relative to it.

4. **Regex compilation happens at load time, not scan time.** This is a critical performance invariant. The hot loop does not recompile regexes; it matches against pre-compiled `CompiledRegex` objects.

5. **The "first PR under 30 minutes" target is for text-level detection only.** All four implicitly accept that complex signal-based rules or guardrail combinations might require more expertise. Dx-optimizer made the 30-minute promise; the other three do not contradict it, they just do not reinforce it to every rule type.

6. **Compliance mappings are per-rule, not global.** Each rule can map to controls in NIST, ISO, EU AI Act, OWASP. No separate compliance registry beyond metadata indices.

7. **Signing is for distribution integrity, not rule validation.** PR review is the trust gate; signing is a supply-chain hardening measure. All four experts took this stance implicitly.

8. **One bad rule never breaks the scan.** Quarantine rules with errors (YAML parse, regex compile, schema violation) and emit diagnostics. Keep scanning.

---

## Gaps Not Covered by Any Expert

1. **Rule inheritance and extension.** Security-auditor punts this to Round 2 (round1-rule-schema.md, line 525: "do we adopt an `extends: <rule-id>` field?"). But the other three experts do not mention it at all. A GRC partner or enterprise customer will want to override `score_adjustment` or add compliance mappings without forking the entire rule file.

   **Gap**: Define the extension mechanism. Options: (a) `extends: rule-id` + field overrides in the same file, (b) per-rule `overrides/` directory, or (c) post-load merge step via a separate YAML file. Likely answer: `extends` field is reserved for future use; v1.0 ships without it.

2. **Rule IDs and namespacing.** Security-auditor uses kebab-case with forward slashes (`framework/langchain/langchain-agent-detected`, round1-rule-schema.md, line 224), but backend-architect's examples use different patterns (e.g., `lc-agent-init`, round1-architecture.md, line 108). No agreement on the ID format or uniqueness guarantee.

   **Gap**: Define a strict ID format (e.g., `{category}/{framework}/{slug}` à la Semgrep, or `{framework}-{slug}` à la Trivy). The CLI must enforce uniqueness across all loaded rule sets and reject ID collisions with an error.

3. **Compliance framework selection at runtime.** All experts agree rules carry per-framework compliance mappings, but none explicitly address how the CLI selects which framework to use. Is it `--framework nist-ai-rmf`, or does the CLI default to a primary framework?

   **Gap**: Clarify the `--framework` flag semantics and default behavior. Likely: default is OWASP Agentic (most accessible), user can switch via flag. All findings carry mappings for all known frameworks in JSON output, but the CLI report shows only the selected framework's controls.

4. **Rule metadata stability and evolution.** What fields are required? What's optional? Dx-optimizer specifies `description` has an 80-character minimum (round1-contributor-dx.md, line 207), but no expert defines a full metadata schema.

   **Gap**: Create a reference `rule.schema.json` (backend-architect mentions this at round1-architecture.md, line 50 but does not show the actual schema). This schema must be version-aware (one schema per `schema_version`).

5. **Symlink or copy strategy for frameworks/ vs. compliance/ dual namespacing.** Backend-architect proposes both, but does not clarify the maintenance model. Are compliance rules symlinks to framework rules? Or are they separate files that must be kept in sync?

   **Gap**: Define the canonical home (frameworks/ wins; compliance/ is generated or symlinked at build time). Clarify in the CI that consistency is enforced.

6. **Performance profiling and regression testing.** Rust-pro mentions "per-rule benchmarking harness (`criterion`)" (round1-rust-engine.md, line 197), but no expert describes the actual benchmark suite or how to track performance across rule-set releases.

   **Gap**: Define the benchmark corpus (e.g., 1000-file synthetic repo with known framework usage patterns), the target metrics (scan time, peak memory), and CI gates (e.g., "new rules must not increase total scan time by >5%").

7. **Multi-language support in regex patterns.** Rust-pro's `langs: LangSet` field allows per-matcher language filters (round1-rust-engine.md, line 54). Security-auditor uses a flat `languages: [python, typescript]` list on individual primitives (round1-rule-schema.md, line 102). Are these the same?

   **Gap**: Align on the data structure. Likely: each primitive can carry optional `languages: [...]` field to restrict its application. The Engine uses this to skip matchers when scanning files in non-matching languages.

8. **Rule tagging and filtering.** Security-auditor mentions "free-form list for filtering; conventional tags below" (round1-rule-schema.md, line 78), but does not specify how the CLI uses tags. Can users filter rules by tag at scan time?

   **Gap**: Define the tag vocabulary and filtering semantics. Likely: `--include-tags framework:langchain --exclude-tags deprecated` as CLI options.

---

## Recommended Reconciliation Order for Round 3

Prioritize resolving gaps in this order. Early decisions unblock later ones.

### Phase A: Foundational (Resolve First)

1. **Single rule file vs. registry.yaml authority** (Contradiction #1)
   - Confirm that `registry.yaml` is generated, not authoritative.
   - Security-auditor's single-file-per-rule design wins.

2. **Context-signal extensibility and the 30-minute promise** (Contradiction #2)
   - Tier capabilities: text-level detection (30-min path), v1.0 signals (still 30-min), v2+ signals (RFC → Rust PR, 1–2 week lead time).
   - Document the binding list of v1.0 signals.

3. **Signing workflow in the contribution pipeline** (Contradiction #3)
   - Clarify that signing is automatic at bundle-release time, not per-PR.
   - Document the GitHub Actions trigger and the maintainer's key management.

4. **Rule ID format and uniqueness** (Gap #2)
   - Define strict format: `{category}/{framework}/{slug}` or `{framework}-{slug}`.
   - Enforce uniqueness in CI; reject ID collisions.

### Phase B: Schema and API (Depends on Phase A)

5. **Matcher API completeness** (Minor Inconsistency #7)
   - Map every schema primitive to a Matcher variant.
   - Ensure no gaps between what the schema expresses and what the engine can evaluate.

6. **Framework catalog structure** (Minor Inconsistency #2)
   - Define `rules/frameworks/{name}.yaml` format for baselines.
   - Document that rules reference frameworks by ID, not inline.

7. **Per-rule metadata schema** (Gap #4)
   - Create `rule.schema.json` with required fields (id, name, description, severity, category, when, score_adjustment, compliance) and optional fields (tags, references, remediation, deprecated, target_versions, contributors).

### Phase C: Distribution and CI (Depends on Phases A & B)

8. **Corpus structure and ownership** (Minor Inconsistency #1)
   - Centralize corpus under `rules/corpus/`.
   - Define per-language fixture format.
   - Clarify CI validation gate and corpus ownership (dx-optimizer).

9. **Deprecation and version-scoping protocol** (Minor Inconsistency #10)
   - Document sunset timeline (2 releases deprecated, 4 releases removal).
   - Clarify `target_versions` filtering behavior and runtime semantics.

10. **Compliance framework selection at runtime** (Gap #3)
    - Define `--framework` flag semantics and default (likely OWASP Agentic).
    - Clarify what goes in JSON output vs. CLI report.

### Phase D: DX and Tooling (Depends on All Above)

11. **CLI subcommands for rule management** (Gap #8)
    - Flesh out `agent-shield rule {new, test, lint, bench, list, explain, map}` API.

12. **Local testing and feedback** (Dx-optimizer's promise)
    - Ensure `agent-shield rule lint` and `agent-shield rule test` outputs are byte-for-byte identical to CI output.

13. **Performance budgeting and regression testing** (Minor Inconsistency #8)
    - Define benchmark corpus, CI gates, and tracking strategy.

---

## Highest-Leverage Tasks for Round 3

**Before finalizing the design, execute these:**

1. **Enumerate v1.0 context signals** (resolves Contradiction #2)
   - Confirm the exact list: tool_count, has_system_prompt, autonomy_tier, has_guardrail, has_permission, data_source_count.
   - Commit to "no new signals in v1.0" as a binding constraint.

2. **Build three reference rules in the final format**
   - Framework detection (LangChain).
   - Behavioral rule (CrewAI multi-agent without HITL).
   - Permission rule (system command access).
   - Use security-auditor's worked examples; ensure they parse and load correctly.

3. **Run a performance test**
   - Build a 1000-file synthetic corpus with framework usage patterns.
   - Load 100 reference rules.
   - Confirm sub-30-second scan time.
   - If it takes >20 seconds, the architecture has a performance leak that must be fixed before shipping.

4. **Define the rule ID format in a CI validator**
   - Write a linter rule that enforces the ID format.
   - Test it against the three reference rules.

---

## Summary: Coherence Grade

**Overall:** 85% coherent. Three critical contradictions, all resolvable. Minor inconsistencies are naming and structural (easily normalized). Gaps are operational details that do not contradict the core design.

**Highest risk:** The "30-minute PR" promise colliding with "closed Matcher enum" and context-signal extensibility. Round 3 must make the trade-off explicit and document realistic timelines for different rule types.

**Lowest risk:** Distribution, signing, and versioning models are well-designed and mostly consistent.

**Next step:** Lock down the three contradictions and the rule ID format, then proceed with Phases B and C in parallel.
REPORT_EOF
```

Done. The report has been written to `/Users/corey/Projects/agent-shield/docs/rules-design/round2-critic-coherence.md`. 

## Summary

I've completed a rigorous architectural review of all four Round 1 proposals and identified every contradiction, inconsistency, and gap:

**Top 3 Critical Contradictions (all resolvable):**
1. **Single-file rule vs. registry authority** — Resolved: registry is derived artifact, not authoritative.
2. **Closed Matcher enum vs. context-signal extensibility** — Resolved: tier capabilities (text-level = 30-min, v1.0 signals = 30-min, new signals = 1–2 week lead time).
3. **Signing and the open contribution pipeline** — Resolved: signing is automatic at bundle-release time, not per-PR.

**10 Minor inconsistencies** with embedded resolutions (corpus ownership, baseline governance, schema versioning, performance budget, deprecation handling, etc.).

**8 Gaps** not covered by any expert (rule inheritance, rule ID format, compliance framework selection at runtime, rule metadata schema, benchmark suite, tagging/filtering semantics, etc.).

**4 Implicit agreements** surfaced to protect from Round 3 reintroduction.

**Recommended reconciliation order** prioritized into 4 phases (Foundational, Schema/API, Distribution/CI, DX/Tooling) with highest-leverage immediate tasks.

The coherence grade is **85%** — the design is mostly coherent, the three big contradictions are all solvable with explicit decisions, and the gaps are operational details that don't contradict the core architecture.
