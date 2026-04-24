# Round 2 — Forensic Report from Q2 2028
**Author:** Critic C (Legacy Modernization Architect)
**Date:** 2026-04-24 (written prospectively; simulated vantage point Q2 2028)
**Status:** Adversarial critique of Round 1 design, for Round 3 lock-in decisions.

---

## 1. 2028 Status Snapshot

Modest-case assumptions apply throughout. The project found its audience but did not break out.

- **Rule corpus: 147 rules across 23 namespaces.** Growth stalled at 60 rules in early 2027 before a burst of MCP-specific contributions pushed it to current levels. The LangChain, CrewAI, and MCP namespaces account for 89 of the 147 rules; everything else is thin.
- **Contributor count: 22 merged contributors across two years.** The distribution is pathological: 6 contributors account for 103 of the 147 rules. Four of the 22 have made a single merged PR and never returned. Three are framework-adjacent employees (two from Anthropic tooling, one from a MCP vendor) who contributed rules that flatter their own frameworks' risk posture.
- **CLI installs: ~8,400 estimated unique monthly actives** (derived from update-rules telemetry added in v0.6.0). SaaS dashboard has 41 paying customers. OSS-to-SaaS conversion rate is 0.49% — below the 2-3% thesis from the revenue docs.
- **Corey is still the sole maintainer.** A co-maintainer was provisionally onboarded in October 2027 (GitHub: `reedmoor`) but went quiet after two months. Governance is informal: Corey reviews everything. Review latency on community PRs averages 11 days.
- **Schema v1 is still the only schema.** No v2 has shipped. Three features that need v2 are being held back. The `extends` escape hatch was never reserved in v1, which is now causing active pain.

---

## 2. The Two Design Decisions That Didn't Age Well

### Decision A: `context_signal` as a closed Rust-enumerated set

The security auditor's `context_signal` primitive — the mechanism for rules to reference computed engine signals like `tool_count`, `autonomy_tier`, and `has_guardrail` — was designed as a fixed, closed set of six named signals requiring a Rust PR to extend.

**The 2027 incident that exposed it: PR #214, "mcp-tool-count-per-server."**

In November 2027, a contributor named `vessel_ops` tried to write a rule detecting MCP servers with more than five tools exposed per server endpoint (distinct from total tool count). The existing `tool_count` context signal counted tools globally across all agents. There was no `tools_per_server` signal. The contributor opened PR #214 implementing the detection as a `code_regex` workaround — a regex that pattern-matches the MCP server manifest JSON to infer per-server tool counts from raw text. It was a terrible regex that matched on structure instead of semantics, generated false positives against multi-line JSON with coincidental numeric fields, and failed the corpus sweep three times.

The PR was open for 47 days. It was eventually closed unmerged. The correct fix — a new `mcp_tool_count_per_server` Rust context signal — required touching the engine crate, which `vessel_ops` could not do. The correct fix was never submitted. The gap remains undetected as of this writing.

**Why Round 1 didn't see this coming:** The six signals enumerated in the schema doc all map naturally to the 2026-era `DiscoveredAgent` struct fields. The design assumed that "what signals the engine can compute" would grow slowly and in parallel with the rule corpus. In practice, MCP's architecture introduced a nested resource model (servers containing tools containing endpoints) that doesn't flatten into the flat signal namespace the engine exposes. Every MCP-specific behavioral rule has had to work around this with regex hacks. The closed enum became a glass ceiling precisely where the most interesting detection logic needed to go.

**What this locked in:** Any rule that needs to reason about per-resource signal aggregation is either wrong (regex hack) or not written. The compliance/behavior rules for MCP — the most important 2028 target — are systematically weaker than the framework-detection rules for LangChain, which predates this limitation.

---

### Decision B: The unified single-file rule as the only contribution unit

The security auditor explicitly rejected splitting detection, scoring, and compliance into separate files, citing contributor velocity. The argument was sound for the 2026 cohort they imagined: security researchers who could do all three in one sitting.

**The 2027 incident that exposed it: The Anthropic SDK compliance backfill.**

In mid-2027, a GRC analyst at a financial services firm (GitHub: `fsec_jana`) wanted to add EU AI Act Article 14 mappings to the 23 existing framework-detection rules that lacked them. The mapping was mechanical: she had the control text, she knew which rules needed it, and she had zero interest in touching detection logic. Under the unified file model, she had to open 23 separate PRs — one per rule file — each touching a YAML file that also contained detection logic and score adjustments she did not understand and was not qualified to review.

She opened 7 PRs. Three were merged. She abandoned the project after Corey left PR #189 in review for 19 days because the detection regex in the same file had a subtle issue unrelated to her changes. The compliance gap she was fixing remains partially done.

The deeper problem is that the "compliance mappings as optional afterthoughts" design in both the schema (the `compliance:` block can be empty on first submission) and the contributor DX (the `--suggest` command for mappings is advisory only) meant that most rules shipped with thin or absent compliance mappings. As of Q2 2028, 61 of the 147 rules have no ISO 42001 mapping. 44 have no EU AI Act mapping. The GRC path — the path that doesn't require regex literacy — was designed but never made easy enough to actually attract GRC contributors.

**What this locked in:** The compliance mapping corpus is permanently behind the detection corpus because the only way to contribute compliance mappings is to open a PR against a file that contains detection logic, scoring, and metadata simultaneously. GRC analysts don't do that. Security researchers fill in compliance mappings perfunctorily or not at all.

**The folk wisdom that emerged from this:** "Never touch a detection file to fix a compliance field." Contributors learned to simply not file the PR. The community workaround — a `rules/compliance-overlays/` directory that the backend architect explicitly rejected in Round 1 — was added ad hoc in v0.7.0 anyway, under community pressure, without a schema change. The overlay files are JSON, not YAML, are not covered by the linter, and have no corpus test requirement. They are the duct tape.

---

## 3. The Two Decisions That Aged Surprisingly Well

### Decision C: Quarantine-on-bad-rule, never abort

The Rust engine's error handling philosophy — quarantine a broken rule and continue scanning rather than halting — has proven to be one of the best decisions in the design. In Q1 2028, a contributor submitted a rule with a pathological regex (catastrophic backtracking on files with repeated import blocks) that slipped past the 100ms-per-fixture performance gate because the fixture file was too small to trigger it. The rule shipped in the bundled binary in v0.5.2.

On a real-world scan of a monorepo with 4,200 files, the rule caused per-file scan time to blow out on 23 files. Because the engine quarantines rules that exceed a per-file runtime budget (added in v0.4.0 as a runtime analog to the CI gate), the scan completed in 34 seconds instead of hanging. The quarantine diagnostic surfaced in JSON output, the dashboard caught it, and the rule was patched in 48 hours. Without quarantine, the scan would have hung and the user would have killed it, filed a bug, and possibly churned.

The SaaS angle the Rust engineer mentioned in Round 1 — "your fleet has 3 broken rules" as a dashboard feature — turned out to be the feature that converted two enterprise customers who were previously blocked by a bad community rule they didn't know about.

### Decision D: The monorepo decision and the `registry.yaml` index

The architecture proposal's decision to stay in the monorepo rather than split to a separate rules repo has been consistently correct. The one time a split was seriously proposed (v0.6.0, when a vendor offered to host a "partner rules registry"), Corey declined and the vendor's rules never materialized. Every project in the agentic tooling space that split their rules repo early — there are two direct competitors worth naming: `agentlens` and `orbitguard` — has suffered from rule corpus staleness. Their CLI ships, the rules repo falls behind, and users get different results depending on when they last ran `update-rules`. The bundled-primary model has made agent-shield's offline behavior a genuine differentiator; it's specifically called out in three enterprise procurement comparisons on file.

The `registry.yaml` index scaling problem that the architect punted to Round 2 was resolved pragmatically: at 147 rules it is not a problem. The SQLite option was never needed. The architect's instinct to punt was correct.

---

## 4. The Schema V2 Question

**v2 did not ship.**

Three features that require v2 are currently held in limbo:

1. **`extends:` field for rule inheritance.** The security auditor explicitly said "reserve this field" in Round 1. It was not reserved. When enterprise customers began asking for score-adjustment overrides in Q4 2027, there was no extension mechanism. The workaround is the ad-hoc JSON overlay system described above. It works but it is not the same schema, not linted, and not in the corpus pipeline.

2. **`target_versions:` field for framework-version-scoped rules.** The architecture doc described this field in the deprecation section. It was never implemented in the v1 schema because it required the engine to read package lock files and compare version ranges — scope that kept getting deferred. LangChain is now on a version with completely different import paths than the patterns in `lc-agent-init.yaml`. The rule fires false positives on LangChain v0.3+ projects that use `langchain_core` instead of `langchain`. The rule is flagged `deprecated: true` with `successor_id: lc-agent-init-002` — but `lc-agent-init-002` was never written. The deprecated field is informational only; the rule still fires.

3. **Nested signal queries for MCP.** Described in detail under Decision A above.

**Why v2 hasn't shipped:** Shipping schema v2 requires simultaneous updates to the engine (new parser), the JSON Schema validator, the `rule lint` tool, the `rule new` scaffolder, and all CI pipelines. That is three to five days of coordinated work for a sole maintainer. The cost-benefit calculus for Corey has been: v1 with duct tape works for 147 rules; the pain is real but not yet unbearable. The moment a significant enterprise customer or a co-maintainer applies pressure, v2 ships in a week. It has not happened yet.

**The duct tape that extended v1:**

- JSON overlay files in `rules/compliance-overlays/` for compliance field overrides.
- A convention (not enforced by schema) of including `x-deprecated-reason:` as a YAML extension key — it's ignored by the engine but visible to human readers.
- `cli_min` / `cli_max` version fields in the `meta:` block (from the architecture doc) were implemented but the engine silently skips rather than warning when >10% of rules are skipped — the >10% warning threshold was never implemented.

---

## 5. What Round 1 Should Change Today

**Priority 1 (do this before anything else): Reserve the `extends` field and design the compliance-overlay format as a first-class schema concept, not an implementation afterthought.**

The single most consequential thing Round 1 can do today is force the schema design to answer the override/inheritance question it explicitly punted. The security auditor named it: "Round 2 should decide." Round 2 must decide. The choice is binary: either add `extends: <rule-id>` as a top-level key in v1 (even if the engine ignores it until implemented), or define a separate overlay format with its own schema and linter. Doing neither means the first enterprise customer who wants to tune score adjustments gets a workaround that lives outside the linting pipeline and will never be cleaned up. The compliance-overlay duct tape is exactly what happens when this question is deferred — it doesn't disappear, it calcifies outside the schema.

The concrete action: the schema doc needs one more worked example — an overlay file that overrides `score_adjustment` and adds an `x-internal-control-id` on an existing rule without touching the source rule file. If that example is ugly or impossible under v1, that is diagnostic. Fix it now while the rule corpus is 0 instead of 147.

**Priority 2 (do this in Round 3, not later): Decouple compliance mappings from detection files at the filesystem level, not just the schema level.**

The unified single-file design is correct for rules written by a single contributor who does detection and compliance in one sitting. It is wrong for rules where detection and compliance are contributed by different people on different timescales — which turns out to be most rules. The fix is not a schema change: it is a directory convention. Allow `rules/frameworks/langchain/lc-agent-init.compliance.yaml` as a sibling file that the engine merges at load time. The detection file is immutable for GRC contributors; the compliance sibling is safely editable by anyone with YAML literacy. The `rule lint` command must validate both files together. This does not require a schema version bump — it is a loader change and a directory convention. It directly unblocks the largest untapped contributor cohort.

---

## 6. The Post-Mortem One-Liner

The design optimized for the contributor it hoped to attract — the security researcher who does detection, scoring, and compliance in one PR — and under-invested in the contributor it actually got: the specialist who knows exactly one of those three things and will never touch the other two.

---

*Written for Round 3 lock-in. The goal is not to predict the future accurately — it is to surface the assumptions that feel like facts today so Round 3 can test them before they compound.*
