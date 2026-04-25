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
    pub extends: Option<String>,
}

/// A bundle of compiled rules ready for evaluation.
#[derive(Debug, Clone, Default)]
pub struct CompiledRuleSet {
    pub rules: Vec<CompiledRule>,
}

impl CompiledRuleSet {
    pub fn new() -> Self {
        Self::default()
    }
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
    pub fn new() -> Self {
        Self::default()
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
    /// As of W2-C5 this is a thin wrapper over `compile_yaml(EMBEDDED_DETECTION_RULES)`.
    /// The hardcoded matcher tree is gone; rule definitions live in
    /// `rules/builtin/*.yaml`. Adding a framework now means: enum variant
    /// + YAML file + EMBEDDED_DETECTION_RULES entry.
    pub fn compile_builtin() -> Self {
        Self::compile_yaml(EMBEDDED_DETECTION_RULES)
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
        let (rules, diags) = crate::rules::loader::parse_bundle(bundle);
        (
            Self {
                rules: CompiledRuleSet { rules },
            },
            diags,
        )
    }
}

/// Bundled detection rules — concatenated at build time via `include_str!`.
///
/// Order matches the variant order of `AgentFramework::all()` so users
/// reading rule IDs can predict scan output without grepping a slug map.
/// When adding a framework: add the variant to `AgentFramework`, the YAML
/// file to `rules/builtin/`, and a new entry here in the same position.
pub const EMBEDDED_DETECTION_RULES: &[(&str, &str)] = &[
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
        let (engine, diags) = Engine::compile_yaml_with_diagnostics(EMBEDDED_DETECTION_RULES);
        assert!(
            diags.is_empty(),
            "expected zero diagnostics, got: {:#?}",
            diags
        );
        assert_eq!(engine.rules.rules.len(), AgentFramework::all().len());
    }

}
