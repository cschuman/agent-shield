//! Detection engine.
//!
//! In Week 1 of the rules-as-data refactor (Path B), this module hosts a closed
//! `Matcher` enum and a `CompiledRuleSet` shape that mirrors the hardcoded
//! detection patterns currently in `frameworks.rs`. Week 2 swaps the source
//! from compiled-in Rust to YAML on disk; the consuming API in `scanner.rs`
//! does not change.
//!
//! See `docs/rules-design/round3-synthesis.md` for the full design context.

pub mod matcher;

use crate::frameworks::AgentFramework;
use crate::rules::types::{Compliance, ParsedSeverity};
use matcher::Matcher;

/// A single compiled detection rule.
///
/// `extends` and `min_match_count` are present from C2 even though the
/// Rust-source rule loader (C3) only sets them via straightforward translation.
/// Both fields round-trip through the YAML schema in Week 2; reserving them
/// now prevents a re-refactor of every call site.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// Stable identifier, e.g. `"framework/langchain/agent-detected"`.
    /// Currently only consulted in diagnostics + tests; reserved for
    /// per-rule reporting in v1.1.
    #[allow(dead_code)]
    pub id: String,
    /// Framework this rule binds to. Stays as a Rust enum through Week 1;
    /// Week 2 may transition to a string-keyed reference into the framework
    /// catalog (synthesis §1.7).
    pub framework: AgentFramework,
    /// The detection logic.
    pub matcher: Matcher,
    /// Minimum number of matches required to fire the rule. Mirrors today's
    /// confidence gating in `scanner.rs::scan_directory` (2 for `VercelAI`
    /// and `CustomAgent`, 1 otherwise).
    pub min_match_count: u8,
    /// Reserved for v1.1 schema overlay support. Always `None` in v1.0; the
    /// YAML loader in Week 2 must reject non-null values.
    #[allow(dead_code)]
    pub extends: Option<String>,
}

/// A single compiled scoring rule (Tier-2). Differs from `CompiledRule`
/// in that it carries finding-emission metadata: title/description templates,
/// severity, and per-framework compliance references. The `matcher` for a
/// scoring rule must evaluate against `ContextSignals` only — file/repo
/// primitives are loader-rejected so a scoring rule can never produce a
/// silently-empty match (which would happen if it tried to read the source
/// tree, since scoring runs after scanning is done).
#[derive(Debug, Clone)]
pub struct CompiledScoringRule {
    /// Stable identifier, e.g. `"scoring/missing-system-prompt"`. Reserved
    /// for per-rule diagnostics in v1.1.
    #[allow(dead_code)]
    pub id: String,
    /// Snake_case category from the YAML, e.g. `prompt_injection_risk`.
    /// `scoring.rs::materialize_finding` maps this to `FindingCategory`.
    pub category: String,
    pub severity: ParsedSeverity,
    /// Optional title — when absent, the rule is a "silent" score
    /// adjustment that does not push a `Finding`. `empty-tools.yaml` is
    /// the only example today.
    pub title: Option<String>,
    pub description: String,
    pub remediation: Option<String>,
    pub matcher: Matcher,
    pub score_adjustment: i32,
    pub compliance: Compliance,
    /// Reserved for v1.1 schema overlay support; loader rejects non-null.
    #[allow(dead_code)]
    pub extends: Option<String>,
}

/// A bundle of compiled rules ready for evaluation.
///
/// Detection rules drive the file/repo scan in `scanner.rs`; scoring rules
/// drive finding emission in `scoring.rs::score_single_agent`. Both vecs
/// preserve YAML-load order, which is the source-order invariant that
/// keeps JSON output byte-identical across scanner runs.
#[derive(Debug, Clone, Default)]
pub struct CompiledRuleSet {
    pub rules: Vec<CompiledRule>,
    pub scoring_rules: Vec<CompiledScoringRule>,
}


/// Detection engine.
///
/// A thin wrapper over a `CompiledRuleSet`. Construction methods —
/// `compile_builtin` (C3), `load_yaml` (Week 2) — produce rule sets;
/// evaluation lives on `Matcher` itself for now.
#[derive(Debug, Clone, Default)]
pub struct Engine {
    pub rules: CompiledRuleSet,
}

impl Engine {
    /// Iterate detection rules in source order.
    pub fn detection_rules(&self) -> &[CompiledRule] {
        &self.rules.rules
    }

    /// Iterate scoring rules in source order. Order is the C8 finding-emit
    /// invariant — `scoring.rs::score_single_agent` consumes this directly.
    pub fn scoring_rules(&self) -> &[CompiledScoringRule] {
        &self.rules.scoring_rules
    }

    /// Compile the built-in detection rules.
    ///
    /// One `CompiledRule` per `AgentFramework` variant. Each rule's matcher
    /// is an `AnyOf` over the framework's primitives, declared inline below.
    /// Before C4 these primitives lived as `DetectionPattern` values on
    /// `AgentFramework::detection_patterns()`; that intermediate enum was
    /// removed and the matcher tree is now the single source of truth.
    ///
    /// At the file pass, `PackageDep` and `FilePresent` matchers return zero
    /// hits, so the legacy "Import + CodePattern only contribute to firing"
    /// behavior is preserved exactly.
    ///
    /// As of W2-C5 this is a thin wrapper over `compile_yaml(EMBEDDED_RULES)`.
    /// The hardcoded matcher tree is gone; rule definitions live in
    /// `rules/builtin/*.yaml` (detection) and `rules/scoring/*.yaml` (scoring).
    /// Adding a framework now means: enum variant + YAML file + EMBEDDED_RULES entry.
    ///
    /// Behind a `OnceLock` so a process-wide scan only parses + regex-compiles
    /// the bundle once. `Clone` is cheap (`Arc`-shaped fields), so the public
    /// API hands out owned values and callers don't have to thread a
    /// `&'static Engine` everywhere.
    pub fn compile_builtin() -> Self {
        static BUILTIN: std::sync::OnceLock<Engine> = std::sync::OnceLock::new();
        BUILTIN
            .get_or_init(|| Self::compile_yaml(EMBEDDED_RULES))
            .clone()
    }

    /// Compile from a bundle of `(source_key, yaml_text)` pairs.
    ///
    /// Bad rules are quarantined into diagnostics rather than panicking.
    /// Diagnostics are printed to stderr at engine init time; the caller
    /// can also inspect the returned vec via `compile_yaml_with_diagnostics`.
    /// The unconditional `.0` access here is the binary path — use the
    /// `_with_diagnostics` variant in tests where you need to assert on
    /// quarantined rules.
    pub fn compile_yaml(bundle: &[(&str, &str)]) -> Self {
        let (rules, diags) = Self::compile_yaml_with_diagnostics(bundle);
        for d in &diags {
            eprintln!(
                "agent-shield: rule {} from {} quarantined: {}",
                d.rule_id.as_deref().unwrap_or("<unparseable>"),
                d.source,
                d.message
            );
        }
        rules
    }

    /// Variant that returns diagnostics instead of printing them. Used by
    /// the C4 equivalence test and the C9 bad-rule fixtures.
    pub fn compile_yaml_with_diagnostics(
        bundle: &[(&str, &str)],
    ) -> (Self, Vec<crate::rules::loader::RuleDiagnostic>) {
        let parsed = crate::rules::loader::parse_bundle(bundle);
        (
            Self {
                rules: CompiledRuleSet {
                    rules: parsed.detection,
                    scoring_rules: parsed.scoring,
                },
            },
            parsed.diagnostics,
        )
    }
}

/// Bundled rules — both detection (Tier-1) and scoring (Tier-2),
/// concatenated at build time via `include_str!`.
///
/// **CRITICAL — DO NOT REORDER WITHOUT RUNNING `bash scripts/snapshot.sh verify`.**
///
/// Ordering is load-bearing in two distinct ways:
///
/// - **Detection rules** (entries 1–10): order matches `AgentFramework::all()`
///   enum declaration order. Inserting a new framework requires inserting its
///   YAML entry in the matching position. A mismatch produces silently
///   misaligned rule IDs — the snapshot diff will catch it but the failure
///   mode is "rule X is now at position Y" which is hard to read.
///
/// - **Scoring rules** (entries 11–21): order matches the firing order of
///   the legacy inline `findings.push` blocks in W1's `score_single_agent`
///   (see `git show 0d0cc77:src/scoring.rs`). This is the byte-identical
///   contract for W2-C8 — JSON output preserves finding-emission order,
///   which is rule-load order. Reorder this list and snapshot tests fail.
///
/// When adding a framework: enum variant + `rules/builtin/<slug>.yaml` +
/// new detection entry here in matching position. When adding a scoring
/// rule: write the YAML + add an entry in the firing-order position +
/// run `bash scripts/snapshot.sh verify` before committing.
pub const EMBEDDED_RULES: &[(&str, &str)] = &[
    // ===== Detection rules (Tier-1) =====
    ("langchain", include_str!("../../rules/builtin/langchain.yaml")),
    ("langgraph", include_str!("../../rules/builtin/langgraph.yaml")),
    ("crewai", include_str!("../../rules/builtin/crewai.yaml")),
    ("autogen", include_str!("../../rules/builtin/autogen.yaml")),
    (
        "openai-assistants",
        include_str!("../../rules/builtin/openai-assistants.yaml"),
    ),
    (
        "anthropic-mcp",
        include_str!("../../rules/builtin/anthropic-mcp.yaml"),
    ),
    (
        "anthropic-agent-sdk",
        include_str!("../../rules/builtin/anthropic-agent-sdk.yaml"),
    ),
    (
        "aws-bedrock",
        include_str!("../../rules/builtin/aws-bedrock.yaml"),
    ),
    ("vercel-ai", include_str!("../../rules/builtin/vercel-ai.yaml")),
    (
        "custom-agent",
        include_str!("../../rules/builtin/custom-agent.yaml"),
    ),
    // ===== Scoring rules (Tier-2), in legacy firing order =====
    // 1. Tool count == 0 → silent +5
    (
        "scoring/empty-tools",
        include_str!("../../rules/scoring/empty-tools.yaml"),
    ),
    // 2. Tool count > 10 → +15 + UnboundedAutonomy finding
    (
        "scoring/unbounded-tools",
        include_str!("../../rules/scoring/unbounded-tools.yaml"),
    ),
    // 3. Unconfirmed tools && tool_count > 3 → +10 + NoHumanOversight finding
    (
        "scoring/unconfirmed-tools",
        include_str!("../../rules/scoring/unconfirmed-tools.yaml"),
    ),
    // 4. !has_system_prompt → +10 + PromptInjectionRisk finding
    (
        "scoring/missing-system-prompt",
        include_str!("../../rules/scoring/missing-system-prompt.yaml"),
    ),
    // 5. !input_validation → +10 + MissingGuardrail/High finding
    (
        "scoring/missing-input-validation",
        include_str!("../../rules/scoring/missing-input-validation.yaml"),
    ),
    // 6. !output_filtering → +5 + MissingGuardrail/Medium finding
    (
        "scoring/missing-output-filter",
        include_str!("../../rules/scoring/missing-output-filter.yaml"),
    ),
    // 7. !rate_limit → +5 + MissingGuardrail/Low finding
    (
        "scoring/missing-rate-limit",
        include_str!("../../rules/scoring/missing-rate-limit.yaml"),
    ),
    // 8. has_exec → +20 + ExcessivePermission/Critical finding
    (
        "scoring/excessive-exec-permission",
        include_str!("../../rules/scoring/excessive-exec-permission.yaml"),
    ),
    // 9. has_admin → +15 + ExcessivePermission/Critical finding
    (
        "scoring/excessive-admin-permission",
        include_str!("../../rules/scoring/excessive-admin-permission.yaml"),
    ),
    // 10. data_source_count > 3 → +10 + DataExposure/Medium finding
    (
        "scoring/data-access-broad",
        include_str!("../../rules/scoring/data-access-broad.yaml"),
    ),
    // 11. !has_audit_trail → +5 + MissingAuditTrail finding
    (
        "scoring/missing-audit-trail",
        include_str!("../../rules/scoring/missing-audit-trail.yaml"),
    ),
];

/// Flatten a `Matcher` tree into human-readable descriptors, preserving the
/// declaration order of `AnyOf`/`AllOf` children.
///
/// Used by `frameworks::list_frameworks` to render the "Detection Method"
/// column in the CLI's `frameworks` subcommand. After C4 this is the only
/// way to introspect a rule's primitives — `DetectionPattern` is gone.
pub fn describe_matcher(m: &Matcher) -> Vec<String> {
    match m {
        Matcher::ImportContains { needle, .. } => vec![format!("import: {}", needle)],
        Matcher::CodeRegex { pattern, .. } => vec![format!("pattern: {}", pattern.as_str())],
        Matcher::MultilineRegex { pattern, .. } => {
            vec![format!("multiline: {}", pattern.as_str())]
        }
        Matcher::PackageDep { name } => vec![format!("package: {}", name)],
        Matcher::FilePresent { path } => vec![format!("config: {}", path)],
        Matcher::AnyOf(children) | Matcher::AllOf(children) => {
            children.iter().flat_map(describe_matcher).collect()
        }
        Matcher::Not(inner) => describe_matcher(inner)
            .into_iter()
            .map(|s| format!("not({})", s))
            .collect(),
        Matcher::ContextSignal { name, param, op, value } => {
            let p = param.as_deref().map(|p| format!("[{}]", p)).unwrap_or_default();
            vec![format!("signal: {}{} {:?} {:?}", name, p, op, value)]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_builtin_emits_one_rule_per_framework() {
        let engine = Engine::compile_builtin();
        assert_eq!(
            engine.rules.rules.len(),
            AgentFramework::all().len(),
            "expected one CompiledRule per AgentFramework variant"
        );
    }

    #[test]
    fn compile_builtin_preserves_confidence_gating() {
        let engine = Engine::compile_builtin();
        for rule in &engine.rules.rules {
            let expected = match rule.framework {
                AgentFramework::VercelAI | AgentFramework::CustomAgent => 2,
                _ => 1,
            };
            assert_eq!(
                rule.min_match_count, expected,
                "min_match_count drift for {:?}",
                rule.framework
            );
        }
    }

    #[test]
    fn compile_builtin_rule_ids_are_unique() {
        let engine = Engine::compile_builtin();
        let mut ids: Vec<&str> = engine.rules.rules.iter().map(|r| r.id.as_str()).collect();
        ids.sort();
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, ids.len(), "rule IDs must be unique");
    }

    #[test]
    fn compile_builtin_extends_is_none() {
        let engine = Engine::compile_builtin();
        assert!(
            engine.rules.rules.iter().all(|r| r.extends.is_none()),
            "extends must be None in v1.0 — overlay support arrives in v1.1"
        );
    }

    /// The YAML bundle must compile cleanly with zero diagnostics.
    #[test]
    fn embedded_yaml_loads_without_diagnostics() {
        let (engine, diags) = Engine::compile_yaml_with_diagnostics(EMBEDDED_RULES);
        assert!(
            diags.is_empty(),
            "expected zero diagnostics, got: {:#?}",
            diags
        );
        assert_eq!(
            engine.detection_rules().len(),
            AgentFramework::all().len(),
            "one detection rule per framework"
        );
        assert_eq!(
            engine.scoring_rules().len(),
            11,
            "11 scoring rules — one per legacy inline finding plus the empty-tools silent bump"
        );
    }

}
