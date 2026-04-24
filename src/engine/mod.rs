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

use crate::frameworks::{AgentFramework, DetectionPattern};
use matcher::{LangSet, Matcher};
use regex::Regex;

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

    /// Compile the built-in detection rules from `frameworks.rs`.
    ///
    /// One `CompiledRule` per `AgentFramework` variant. Each rule's matcher
    /// is an `AnyOf` over every `DetectionPattern` declared for that variant.
    /// This is intentionally lossless w.r.t. today's behavior: at the file
    /// pass, `PackageDep` and `FilePresent` matchers return zero hits, so
    /// the existing "Import + CodePattern only contribute to firing" logic
    /// is preserved exactly.
    ///
    /// In Week 2 this method is replaced by a YAML loader; the return type
    /// stays the same.
    pub fn compile_builtin() -> Self {
        let mut rules = Vec::with_capacity(AgentFramework::all().len());

        for fw in AgentFramework::all() {
            let sub_matchers: Vec<Matcher> = fw
                .detection_patterns()
                .into_iter()
                .map(translate_pattern)
                .collect();

            let min_match_count: u8 = match fw {
                AgentFramework::VercelAI | AgentFramework::CustomAgent => 2,
                _ => 1,
            };

            rules.push(CompiledRule {
                id: format!("framework/{}/agent-detected", framework_slug(&fw)),
                framework: fw,
                matcher: Matcher::AnyOf(sub_matchers),
                min_match_count,
                extends: None,
            });
        }

        Self {
            rules: CompiledRuleSet { rules },
        }
    }
}

/// Translate a single legacy `DetectionPattern` into a `Matcher`.
///
/// Regex compilation panics on malformed patterns — acceptable for the
/// hardcoded translation since all patterns are authored in `frameworks.rs`
/// and verified at first run. The YAML loader in Week 2 must surface the
/// same error as a quarantined-rule diagnostic instead of a panic.
fn translate_pattern(pat: DetectionPattern) -> Matcher {
    match pat {
        DetectionPattern::Import(s) => Matcher::ImportContains {
            needle: s.to_string(),
            languages: LangSet::Any,
        },
        DetectionPattern::CodePattern(p) => Matcher::CodeRegex {
            pattern: Regex::new(p)
                .unwrap_or_else(|e| panic!("invalid regex in built-in rule {:?}: {}", p, e)),
            languages: LangSet::Any,
        },
        DetectionPattern::PackageDep(s) => Matcher::PackageDep {
            name: s.to_string(),
        },
        DetectionPattern::ConfigFile(s) => Matcher::FilePresent {
            path: s.to_string(),
        },
    }
}

fn framework_slug(fw: &AgentFramework) -> &'static str {
    match fw {
        AgentFramework::LangChain => "langchain",
        AgentFramework::LangGraph => "langgraph",
        AgentFramework::CrewAI => "crewai",
        AgentFramework::AutoGen => "autogen",
        AgentFramework::OpenAIAssistants => "openai-assistants",
        AgentFramework::AnthropicMCP => "anthropic-mcp",
        AgentFramework::AnthropicAgentSDK => "anthropic-agent-sdk",
        AgentFramework::AWSBedrock => "aws-bedrock",
        AgentFramework::VercelAI => "vercel-ai",
        AgentFramework::CustomAgent => "custom-agent",
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
}
