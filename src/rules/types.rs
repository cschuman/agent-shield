//! Serde-deserializable types mirroring the v1.0 YAML rule schema.
//!
//! These types are intentionally separate from `engine::CompiledRule`: they
//! capture exactly what's on disk, including v1.1-reserved fields and
//! string-keyed enums that must be validated before translation. The
//! loader in W2-C4 maps `ParsedRule` → `CompiledRule`.
//!
//! Field naming follows the round3-synthesis schema (§2):
//! - YAML keys are `snake_case` (e.g. `import_contains`, `min_match_count`).
//! - Severity / signal-op tags are lowercase (`high`, `eq`).
//! - Compliance framework keys are `nist_ai_rmf`, `iso_42001`, `eu_ai_act`,
//!   `owasp_agentic` — pinned to match the synthesis example.

use serde::Deserialize;

/// One rule as it appears in a YAML file. Detection vs. scoring is
/// distinguished by `category`: detection rules use `category: detection`;
/// scoring rules use one of the finding-category names (e.g.
/// `prompt_injection_risk`, `missing_guardrail`).
///
/// Optional fields differ by category — the loader enforces:
/// - detection rules require `framework`; reject `score_adjustment` /
///   `compliance` / `title` / `remediation` / `finding`.
/// - scoring rules require `title`, `remediation`, `score_adjustment`,
///   `compliance`; reject `framework` / `min_match_count`.
///
/// `extends` is reserved for v1.1 and must be `null` (or absent) in v1.0;
/// the loader emits a quarantine diagnostic for any non-null value.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedRule {
    pub schema_version: String,
    pub id: String,
    pub category: String,
    pub severity: ParsedSeverity,
    pub description: String,
    pub when: ParsedMatcher,

    // Detection-only
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default)]
    pub min_match_count: Option<u8>,

    // Scoring-only
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub remediation: Option<String>,
    #[serde(default)]
    pub score_adjustment: Option<i32>,
    #[serde(default)]
    pub compliance: Option<Compliance>,

    // v1.1-reserved
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParsedSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// The closed set of detection primitives, in YAML form.
///
/// Represented as a struct with one optional field per primitive — the
/// loader validates that exactly one field is set per matcher node and
/// emits a quarantine diagnostic otherwise. This shape matches the natural
/// YAML form `{ import_contains: "langchain" }` / `{ any_of: [...] }`
/// without forcing YAML-tag syntax.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ParsedMatcher {
    #[serde(default)]
    pub import_contains: Option<String>,
    #[serde(default)]
    pub code_regex: Option<String>,
    #[serde(default)]
    pub multiline_regex: Option<String>,
    #[serde(default)]
    pub package_dep: Option<String>,
    #[serde(default)]
    pub file_present: Option<String>,
    #[serde(default)]
    pub context_signal: Option<ParsedContextSignal>,
    #[serde(default)]
    pub all_of: Option<Vec<ParsedMatcher>>,
    #[serde(default)]
    pub any_of: Option<Vec<ParsedMatcher>>,
    #[serde(default)]
    pub not: Option<Box<ParsedMatcher>>,
}

impl ParsedMatcher {
    /// Count how many primitive/combinator slots are populated. The loader
    /// enforces exactly one; a count of 0 or >1 is a malformed rule.
    pub fn populated_slot_count(&self) -> usize {
        [
            self.import_contains.is_some(),
            self.code_regex.is_some(),
            self.multiline_regex.is_some(),
            self.package_dep.is_some(),
            self.file_present.is_some(),
            self.context_signal.is_some(),
            self.all_of.is_some(),
            self.any_of.is_some(),
            self.not.is_some(),
        ]
        .iter()
        .filter(|b| **b)
        .count()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedContextSignal {
    pub name: String,
    /// Optional sub-key for parametrized signals like
    /// `has_guardrail[input_validation]` or `has_permission[execute]`.
    /// Scalar signals (`tool_count`, `has_system_prompt`, etc.) leave this
    /// unset; the loader validates name+param combinations against the
    /// matcher's evaluator surface.
    #[serde(default)]
    pub param: Option<String>,
    pub op: ParsedSignalOp,
    pub value: ParsedSignalValue,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParsedSignalOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

/// Either a bool, integer, or string literal. Untagged so that
/// `value: false` / `value: 10` / `value: "exec"` all deserialize directly.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ParsedSignalValue {
    Bool(bool),
    Int(i64),
    Str(String),
}

/// Per-rule compliance metadata, surfaced on each emitted Finding.
///
/// Each framework's value is a list of control identifiers. Multi-control
/// strings (e.g. `"MAP 2.3, MEASURE 2.6"`) are stored as a single element
/// to preserve byte-identical output of today's `framework_ref` strings —
/// the loader does not split on commas.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Compliance {
    #[serde(default)]
    pub nist_ai_rmf: Vec<String>,
    #[serde(default)]
    pub iso_42001: Vec<String>,
    #[serde(default)]
    pub eu_ai_act: Vec<String>,
    #[serde(default)]
    pub owasp_agentic: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: a minimal detection-rule YAML round-trips into
    /// `ParsedRule` without error. Real schema fixtures live in
    /// `rules/builtin/` (W2-C3); this just verifies the type shape compiles
    /// and the snake_case mapping works.
    #[test]
    fn parses_minimal_detection_rule() {
        let yaml = r#"
schema_version: "1.0"
id: "framework/langchain/agent-detected"
category: detection
severity: high
description: "LangChain agent detected"
framework: LangChain
min_match_count: 1
extends: null
when:
  any_of:
    - import_contains: "langchain"
    - package_dep: "langchain"
"#;
        let rule: ParsedRule = serde_yaml::from_str(yaml).expect("must parse");
        assert_eq!(rule.id, "framework/langchain/agent-detected");
        assert_eq!(rule.category, "detection");
        assert_eq!(rule.severity, ParsedSeverity::High);
        assert_eq!(rule.when.populated_slot_count(), 1);
        let any_of = rule.when.any_of.as_ref().expect("any_of populated");
        assert_eq!(any_of.len(), 2);
        assert_eq!(any_of[0].import_contains.as_deref(), Some("langchain"));
        assert_eq!(any_of[1].package_dep.as_deref(), Some("langchain"));
        assert_eq!(rule.framework.as_deref(), Some("LangChain"));
        assert!(rule.extends.is_none());
    }

    /// Smoke test: a minimal scoring-rule YAML with context_signal +
    /// compliance round-trips correctly.
    #[test]
    fn parses_minimal_scoring_rule() {
        let yaml = r#"
schema_version: "1.0"
id: "scoring/missing-system-prompt"
category: prompt_injection_risk
severity: medium
description: "Agent has no detectable system prompt"
title: "No system prompt detected"
remediation: "Add an explicit system prompt"
score_adjustment: 10
when:
  context_signal:
    name: has_system_prompt
    op: eq
    value: false
compliance:
  nist_ai_rmf:
    - "MAP 2.3, MEASURE 2.6"
  iso_42001:
    - "A.6.1.4"
  eu_ai_act:
    - "Article 14"
  owasp_agentic:
    - "AGT-01"
"#;
        let rule: ParsedRule = serde_yaml::from_str(yaml).expect("must parse");
        assert_eq!(rule.score_adjustment, Some(10));
        let compliance = rule.compliance.expect("compliance present");
        assert_eq!(compliance.nist_ai_rmf, vec!["MAP 2.3, MEASURE 2.6"]);
        assert_eq!(compliance.iso_42001, vec!["A.6.1.4"]);
        let sig = rule.when.context_signal.as_ref().expect("context_signal");
        assert_eq!(sig.name, "has_system_prompt");
        assert_eq!(sig.op, ParsedSignalOp::Eq);
        assert_eq!(sig.value, ParsedSignalValue::Bool(false));
        assert_eq!(rule.when.populated_slot_count(), 1);
    }

    /// `deny_unknown_fields` must reject typos so contributors can't
    /// silently misspell a field name (e.g. `severty:`) and have it ignored.
    #[test]
    fn rejects_unknown_top_level_fields() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
typo_field: "should fail"
when:
  import_contains: "x"
"#;
        let result: Result<ParsedRule, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err(), "unknown field should be rejected");
    }
}
