//! Translate parsed YAML rules into `CompiledRule`s for the engine.
//!
//! The loader is the **only** place schema policy lives. Anything ill-formed
//! is dropped and recorded as a `RuleDiagnostic`; the rest of the bundle
//! still loads. This is the quarantine model from round3-synthesis §1.4 —
//! never panic, never poison the rule set with a bad neighbor.
//!
//! Validation surface (rejected at parse time):
//! - `schema_version != "1.0"` — anything else is "future" until v1.1 lands.
//! - `extends != null` — overlay support is reserved for v1.1.
//! - Detection rules without a recognized `framework`.
//! - Detection rules whose `framework` field is set to a value outside
//!   `AgentFramework::all()`.
//! - Matcher nodes with zero or more than one populated slot.
//! - Regex bodies that fail to compile.
//! - Context signals whose `(name, param)` combination isn't recognized by
//!   the matcher's `evaluate_context_signal`.
//!
//! The signal allowlist intentionally includes `unconfirmed_tool_count` and
//! `has_audit_trail` even though round3 §1.2 names six canonical signals:
//! the existing scoring rules in W2-C6 need them. Promote-by-need rather
//! than break the byte-identical contract.
//!
//! Diagnostics print to stderr at engine init time (W2-C9). Stdout stays
//! reserved for JSON scan output.

use crate::engine::matcher::{LangSet, Matcher, SignalOp, SignalValue};
use crate::engine::CompiledRule;
use crate::frameworks::AgentFramework;
use crate::rules::types::{
    ParsedContextSignal, ParsedMatcher, ParsedRule, ParsedSignalOp, ParsedSignalValue,
};
use regex::Regex;

/// One diagnostic from a rule that didn't make it through validation.
///
/// `source` is the bundle key (typically the YAML file's slug). `rule_id`
/// is `None` when the rule failed to deserialize at all (so we couldn't
/// extract the id field).
#[derive(Debug, Clone)]
pub struct RuleDiagnostic {
    pub source: String,
    pub rule_id: Option<String>,
    pub message: String,
}

impl RuleDiagnostic {
    fn new(source: &str, rule_id: Option<&str>, message: impl Into<String>) -> Self {
        Self {
            source: source.to_string(),
            rule_id: rule_id.map(|s| s.to_string()),
            message: message.into(),
        }
    }
}

/// Parse a bundle of `(source_key, yaml_text)` pairs into compiled rules
/// plus diagnostics. Both lists are returned regardless of how many rules
/// failed — the caller decides whether to fail the build (tests) or print
/// and continue (binary).
pub fn parse_bundle(bundle: &[(&str, &str)]) -> (Vec<CompiledRule>, Vec<RuleDiagnostic>) {
    let mut compiled = Vec::with_capacity(bundle.len());
    let mut diags = Vec::new();
    for (source, content) in bundle {
        match parse_one(source, content) {
            Ok(rule) => compiled.push(rule),
            Err(d) => diags.push(d),
        }
    }
    (compiled, diags)
}

fn parse_one(source: &str, content: &str) -> Result<CompiledRule, RuleDiagnostic> {
    let parsed: ParsedRule = serde_yaml::from_str(content)
        .map_err(|e| RuleDiagnostic::new(source, None, format!("YAML parse error: {}", e)))?;

    if parsed.schema_version != "1.0" {
        return Err(RuleDiagnostic::new(
            source,
            Some(&parsed.id),
            format!(
                "unsupported schema_version `{}` — v1.0 is the only release",
                parsed.schema_version
            ),
        ));
    }

    if parsed.extends.is_some() {
        return Err(RuleDiagnostic::new(
            source,
            Some(&parsed.id),
            "`extends` is reserved for v1.1 — must be null in v1.0",
        ));
    }

    match parsed.category.as_str() {
        "detection" => translate_detection(source, parsed),
        // Scoring categories are recognized in W2-C7. For C4–C5 we keep
        // detection-only and reject everything else so the parse-equivalence
        // test against compile_builtin stays honest.
        other => Err(RuleDiagnostic::new(
            source,
            Some(&parsed.id),
            format!("unsupported category `{}` (W2-C4 handles `detection` only)", other),
        )),
    }
}

fn translate_detection(source: &str, parsed: ParsedRule) -> Result<CompiledRule, RuleDiagnostic> {
    let id = parsed.id.clone();
    let framework_name = parsed.framework.as_deref().ok_or_else(|| {
        RuleDiagnostic::new(source, Some(&id), "detection rules must declare `framework`")
    })?;
    let framework = resolve_framework(framework_name).ok_or_else(|| {
        RuleDiagnostic::new(
            source,
            Some(&id),
            format!("unknown framework `{}`", framework_name),
        )
    })?;
    let matcher = translate_matcher(source, &id, &parsed.when)?;
    Ok(CompiledRule {
        id,
        framework,
        matcher,
        min_match_count: parsed.min_match_count.unwrap_or(1),
        extends: None,
    })
}

/// Map the YAML `framework:` string back to the Rust enum. The string is the
/// variant identifier (`LangChain`, not the human-readable display name).
fn resolve_framework(name: &str) -> Option<AgentFramework> {
    AgentFramework::all()
        .into_iter()
        .find(|fw| variant_ident(fw) == name)
}

fn variant_ident(fw: &AgentFramework) -> &'static str {
    match fw {
        AgentFramework::LangChain => "LangChain",
        AgentFramework::LangGraph => "LangGraph",
        AgentFramework::CrewAI => "CrewAI",
        AgentFramework::AutoGen => "AutoGen",
        AgentFramework::OpenAIAssistants => "OpenAIAssistants",
        AgentFramework::AnthropicMCP => "AnthropicMCP",
        AgentFramework::AnthropicAgentSDK => "AnthropicAgentSDK",
        AgentFramework::AWSBedrock => "AWSBedrock",
        AgentFramework::VercelAI => "VercelAI",
        AgentFramework::CustomAgent => "CustomAgent",
    }
}

/// Recursive ParsedMatcher → Matcher translation. Validates exactly-one-slot
/// at every level. Returns the first error encountered; the caller wraps it
/// in a top-level diagnostic.
fn translate_matcher(
    source: &str,
    rule_id: &str,
    pm: &ParsedMatcher,
) -> Result<Matcher, RuleDiagnostic> {
    let n = pm.populated_slot_count();
    if n == 0 {
        return Err(RuleDiagnostic::new(
            source,
            Some(rule_id),
            "matcher node has no populated slot",
        ));
    }
    if n > 1 {
        return Err(RuleDiagnostic::new(
            source,
            Some(rule_id),
            format!("matcher node has {} populated slots; exactly one is required", n),
        ));
    }

    if let Some(needle) = &pm.import_contains {
        return Ok(Matcher::ImportContains {
            needle: needle.clone(),
            languages: LangSet::Any,
        });
    }
    if let Some(pattern) = &pm.code_regex {
        let re = Regex::new(pattern).map_err(|e| {
            RuleDiagnostic::new(
                source,
                Some(rule_id),
                format!("code_regex `{}` failed to compile: {}", pattern, e),
            )
        })?;
        return Ok(Matcher::CodeRegex {
            pattern: re,
            languages: LangSet::Any,
        });
    }
    if let Some(pattern) = &pm.multiline_regex {
        let re = Regex::new(pattern).map_err(|e| {
            RuleDiagnostic::new(
                source,
                Some(rule_id),
                format!("multiline_regex `{}` failed to compile: {}", pattern, e),
            )
        })?;
        return Ok(Matcher::MultilineRegex {
            pattern: re,
            languages: LangSet::Any,
        });
    }
    if let Some(name) = &pm.package_dep {
        return Ok(Matcher::PackageDep { name: name.clone() });
    }
    if let Some(path) = &pm.file_present {
        return Ok(Matcher::FilePresent { path: path.clone() });
    }
    if let Some(sig) = &pm.context_signal {
        return translate_context_signal(source, rule_id, sig);
    }
    if let Some(children) = &pm.all_of {
        return Ok(Matcher::AllOf(translate_children(source, rule_id, children)?));
    }
    if let Some(children) = &pm.any_of {
        return Ok(Matcher::AnyOf(translate_children(source, rule_id, children)?));
    }
    if let Some(inner) = &pm.not {
        return Ok(Matcher::Not(Box::new(translate_matcher(
            source, rule_id, inner,
        )?)));
    }
    unreachable!("populated_slot_count check above enforces one branch matches")
}

fn translate_children(
    source: &str,
    rule_id: &str,
    children: &[ParsedMatcher],
) -> Result<Vec<Matcher>, RuleDiagnostic> {
    if children.is_empty() {
        return Err(RuleDiagnostic::new(
            source,
            Some(rule_id),
            "all_of/any_of must contain at least one child",
        ));
    }
    children
        .iter()
        .map(|c| translate_matcher(source, rule_id, c))
        .collect()
}

fn translate_context_signal(
    source: &str,
    rule_id: &str,
    sig: &ParsedContextSignal,
) -> Result<Matcher, RuleDiagnostic> {
    if !is_known_signal(&sig.name, sig.param.as_deref()) {
        return Err(RuleDiagnostic::new(
            source,
            Some(rule_id),
            format!(
                "unknown context signal `{}{}`; allowlist: {}",
                sig.name,
                sig.param
                    .as_deref()
                    .map(|p| format!("[{}]", p))
                    .unwrap_or_default(),
                allowlist_help()
            ),
        ));
    }
    let op = match sig.op {
        ParsedSignalOp::Eq => SignalOp::Eq,
        ParsedSignalOp::Ne => SignalOp::Ne,
        ParsedSignalOp::Gt => SignalOp::Gt,
        ParsedSignalOp::Gte => SignalOp::Gte,
        ParsedSignalOp::Lt => SignalOp::Lt,
        ParsedSignalOp::Lte => SignalOp::Lte,
    };
    let value = match &sig.value {
        ParsedSignalValue::Bool(b) => SignalValue::Bool(*b),
        ParsedSignalValue::Int(i) => SignalValue::Int(*i),
        ParsedSignalValue::Str(s) => SignalValue::Str(s.clone()),
    };
    // Reject ordering ops on boolean signals — the matcher silently returns
    // false for that combination, but the loader catches it as a likely typo.
    if matches!(op, SignalOp::Gt | SignalOp::Gte | SignalOp::Lt | SignalOp::Lte)
        && matches!(value, SignalValue::Bool(_))
    {
        return Err(RuleDiagnostic::new(
            source,
            Some(rule_id),
            format!(
                "ordering op `{:?}` on boolean signal `{}` is meaningless",
                op, sig.name
            ),
        ));
    }
    Ok(Matcher::ContextSignal {
        name: sig.name.clone(),
        param: sig.param.clone(),
        op,
        value,
    })
}

/// Allowlist of `(signal_name, param)` pairs the matcher knows how to
/// evaluate. Mirror of `evaluate_context_signal` in matcher.rs — keep them
/// in lockstep.
fn is_known_signal(name: &str, param: Option<&str>) -> bool {
    match (name, param) {
        ("tool_count", None)
        | ("autonomy_tier", None)
        | ("data_source_count", None)
        | ("unconfirmed_tool_count", None)
        | ("has_system_prompt", None)
        | ("has_audit_trail", None) => true,
        ("has_guardrail", Some("input_validation"))
        | ("has_guardrail", Some("output_filtering"))
        | ("has_guardrail", Some("rate_limit")) => true,
        ("has_permission", Some("execute"))
        | ("has_permission", Some("admin"))
        | ("has_permission", Some("write")) => true,
        _ => false,
    }
}

fn allowlist_help() -> &'static str {
    "tool_count|autonomy_tier|data_source_count|unconfirmed_tool_count|has_system_prompt|has_audit_trail \
     |has_guardrail[input_validation|output_filtering|rate_limit] \
     |has_permission[execute|admin|write]"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(yaml: &str) -> Result<CompiledRule, RuleDiagnostic> {
        parse_one("test", yaml)
    }

    #[test]
    fn detection_rule_round_trips() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  any_of:
    - import_contains: "langchain"
    - package_dep: "langchain"
"#;
        let rule = one(yaml).expect("must parse");
        assert_eq!(rule.id, "x");
        assert_eq!(rule.framework, AgentFramework::LangChain);
        assert_eq!(rule.min_match_count, 1);
        assert!(matches!(rule.matcher, Matcher::AnyOf(ref c) if c.len() == 2));
    }

    #[test]
    fn rejects_future_schema_version() {
        let yaml = r#"
schema_version: "9.9"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  import_contains: "x"
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("schema_version"));
    }

    #[test]
    fn rejects_non_null_extends() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
extends: "framework/other/agent-detected"
when:
  import_contains: "x"
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("extends"));
    }

    #[test]
    fn rejects_unknown_framework() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: NotAFramework
when:
  import_contains: "x"
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("NotAFramework"));
    }

    #[test]
    fn rejects_bad_regex() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  code_regex: "(unclosed"
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("failed to compile"));
    }

    #[test]
    fn rejects_zero_matcher_slots() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
when: {}
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("no populated slot"));
    }

    #[test]
    fn rejects_multiple_matcher_slots() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  import_contains: "a"
  package_dep: "b"
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("populated slots"));
    }

    #[test]
    fn rejects_unknown_context_signal() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  context_signal:
    name: not_a_real_signal
    op: eq
    value: true
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("unknown context signal"));
    }

    #[test]
    fn parse_bundle_quarantines_individually() {
        let good = r#"
schema_version: "1.0"
id: "good"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  import_contains: "langchain"
"#;
        let bad = r#"
schema_version: "9.9"
id: "bad"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  import_contains: "x"
"#;
        let (rules, diags) = parse_bundle(&[("g", good), ("b", bad)]);
        assert_eq!(rules.len(), 1, "good rule loaded");
        assert_eq!(diags.len(), 1, "bad rule quarantined");
        assert_eq!(rules[0].id, "good");
        assert_eq!(diags[0].rule_id.as_deref(), Some("bad"));
    }

    #[test]
    fn rejects_ordering_op_on_bool_signal() {
        let yaml = r#"
schema_version: "1.0"
id: "x"
category: detection
severity: high
description: "x"
framework: LangChain
when:
  context_signal:
    name: has_system_prompt
    op: gt
    value: false
"#;
        let err = one(yaml).expect_err("must reject");
        assert!(err.message.contains("meaningless"));
    }
}
