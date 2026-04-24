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
}
