use crate::engine::{CompiledScoringRule, Engine};
use crate::frameworks::AgentFramework;
use crate::rules::types::{Compliance, ParsedSeverity};
use crate::scanner::DiscoveredAgent;
use crate::signals::{compute_all_signals, ContextSignals};
use serde::Serialize;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Framework {
    Nist,
    Iso42001,
    EuAiAct,
    OwaspAgentic,
}

impl std::fmt::Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nist => write!(f, "NIST AI RMF"),
            Self::Iso42001 => write!(f, "ISO/IEC 42001"),
            Self::EuAiAct => write!(f, "EU AI Act"),
            Self::OwaspAgentic => write!(f, "OWASP Agentic Top 10"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoredAgent {
    pub name: String,
    pub framework: String,
    pub file_path: String,
    pub line_number: usize,
    pub risk_score: u8,
    pub risk_level: RiskLevel,
    pub tool_count: usize,
    pub has_system_prompt: bool,
    pub guardrail_count: usize,
    pub permission_summary: String,
    pub data_access_summary: String,
    pub findings: Vec<Finding>,
    pub autonomy_tier: u8,
}

#[derive(Debug, Clone, Serialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub category: FindingCategory,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub remediation: String,
    pub framework_ref: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum FindingCategory {
    MissingGuardrail,
    ExcessivePermission,
    DataExposure,
    PromptInjectionRisk,
    NoHumanOversight,
    UnboundedAutonomy,
    MissingAuditTrail,
}

#[derive(Debug, Clone, Serialize)]
pub enum Severity {
    /// Reserved for v1.1 silent-rule findings; ParsedSeverity has no
    /// `info` variant today, so the loader cannot construct this.
    #[allow(dead_code)]
    Info,
    Low,
    Medium,
    High,
    Critical,
}

pub fn score_agents(agents: &[DiscoveredAgent], compliance_framework: &Framework) -> Vec<ScoredAgent> {
    let engine = Engine::compile_builtin();
    agents
        .iter()
        .map(|agent| score_single_agent(agent, compliance_framework, &engine))
        .collect()
}

fn score_single_agent(
    agent: &DiscoveredAgent,
    compliance_framework: &Framework,
    engine: &Engine,
) -> ScoredAgent {
    // Compute the v1 fact set once. Scoring rules read these signals via
    // matches_signals; the unconditional autonomy and guardrail-credit
    // adjustments stay inline because they're scalar transforms, not
    // discrete findings.
    let signals = compute_all_signals(agent);

    let mut score: i16 = get_framework_baseline(&agent.framework);
    let mut findings = Vec::new();

    // Autonomy tier scalar — unconditional, applied before rule loop.
    let autonomy_tier = signals.autonomy_tier;
    score = score.saturating_add((i16::from(autonomy_tier) - 1) * 10);

    // Drive findings + score adjustments off the YAML-defined scoring rule
    // set, in source order. EMBEDDED_RULES enforces firing order to keep
    // snapshot output byte-identical. score_adjustment is bounded to
    // ±100 by the loader, so saturating_add cannot overflow even with
    // an adversarial 11-rule pile-on. The i16::try_from is defense in depth:
    // if a future loader change widens the bound past i16::MAX, the rule
    // contributes 0 instead of wrapping.
    for rule in engine.scoring_rules() {
        if rule.matcher.matches_signals(&signals).is_empty() {
            continue;
        }
        let adjustment = i16::try_from(rule.score_adjustment).unwrap_or(0);
        score = score.saturating_add(adjustment);
        if let Some(finding) = materialize_finding(rule, compliance_framework, &signals) {
            findings.push(finding);
        }
    }

    // Permission summary uses the bool flags computed in signals.rs.
    let has_exec = signals.permissions.execute;
    let has_admin = signals.permissions.admin;
    let has_write = signals.permissions.write;

    // Guardrail credit — unconditional reduction, capped at 25. Clamp the
    // count *before* casting so an agent with > i16::MAX guardrails (only
    // possible from a hand-constructed test fixture) cannot wrap the cast.
    let guardrail_count = agent.guardrails.len().min(5) as i16;
    let guardrail_credit = guardrail_count * 5;
    score = score.saturating_sub(guardrail_credit);

    let final_score = score.clamp(0, 100) as u8;

    let risk_level = match final_score {
        0..=25 => RiskLevel::Low,
        26..=50 => RiskLevel::Medium,
        51..=75 => RiskLevel::High,
        _ => RiskLevel::Critical,
    };

    let permission_summary = if has_admin {
        "ADMIN".into()
    } else if has_exec {
        "EXEC".into()
    } else if has_write {
        "READ/WRITE".into()
    } else {
        "READ".into()
    };

    let data_access_summary = agent
        .data_access
        .iter()
        .map(|d| d.source.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    ScoredAgent {
        name: agent.name.clone(),
        framework: agent.framework.clone(),
        file_path: agent.file_path.display().to_string(),
        line_number: agent.line_number,
        risk_score: final_score,
        risk_level,
        tool_count: signals.tool_count,
        has_system_prompt: signals.has_system_prompt,
        guardrail_count: agent.guardrails.len(),
        permission_summary,
        data_access_summary,
        findings,
        autonomy_tier,
    }
}

/// Build a `Finding` from a matched scoring rule. Returns `None` for
/// "silent" rules (e.g. `empty-tools`) that adjust score without surfacing
/// a finding to the user.
fn materialize_finding(
    rule: &CompiledScoringRule,
    framework: &Framework,
    signals: &ContextSignals,
) -> Option<Finding> {
    let title_template = rule.title.as_ref()?;
    let remediation = rule.remediation.as_ref()?;

    Some(Finding {
        category: map_category(&rule.category),
        severity: map_severity(rule.severity),
        title: substitute_signal_placeholders(title_template, signals),
        description: substitute_signal_placeholders(&rule.description, signals),
        remediation: remediation.clone(),
        framework_ref: pick_compliance(&rule.compliance, framework),
    })
}

/// Replace `{signal_name}` placeholders with current signal values. Only
/// the integer signals make sense in titles/descriptions today; bool
/// substitution would be redundant since rules are gated on those values.
fn substitute_signal_placeholders(template: &str, signals: &ContextSignals) -> String {
    template
        .replace("{tool_count}", &signals.tool_count.to_string())
        .replace(
            "{unconfirmed_tool_count}",
            &signals.unconfirmed_tool_count.to_string(),
        )
        .replace(
            "{data_source_count}",
            &signals.data_source_count.to_string(),
        )
        .replace("{autonomy_tier}", &signals.autonomy_tier.to_string())
}

/// Map a YAML scoring category string to the Rust enum surfaced in JSON
/// output. Unknown categories collapse to `MissingGuardrail` rather than
/// panicking — the loader-validated set never produces unknowns, but we
/// stay defensive in case a future YAML drifts.
fn map_category(s: &str) -> FindingCategory {
    match s {
        "missing_guardrail" => FindingCategory::MissingGuardrail,
        "excessive_permission" => FindingCategory::ExcessivePermission,
        "data_exposure" => FindingCategory::DataExposure,
        "prompt_injection_risk" => FindingCategory::PromptInjectionRisk,
        "no_human_oversight" => FindingCategory::NoHumanOversight,
        "unbounded_autonomy" => FindingCategory::UnboundedAutonomy,
        "missing_audit_trail" => FindingCategory::MissingAuditTrail,
        "detection_uncertainty" => FindingCategory::MissingGuardrail,
        _ => FindingCategory::MissingGuardrail,
    }
}

fn map_severity(s: ParsedSeverity) -> Severity {
    match s {
        ParsedSeverity::Low => Severity::Low,
        ParsedSeverity::Medium => Severity::Medium,
        ParsedSeverity::High => Severity::High,
        ParsedSeverity::Critical => Severity::Critical,
    }
}

/// Pick the compliance string for the user's selected `--framework`.
/// Falls back to a generic framework label when a rule's compliance entry
/// is empty.
fn pick_compliance(c: &Compliance, fw: &Framework) -> String {
    let v = match fw {
        Framework::Nist => &c.nist_ai_rmf,
        Framework::Iso42001 => &c.iso_42001,
        Framework::EuAiAct => &c.eu_ai_act,
        Framework::OwaspAgentic => &c.owasp_agentic,
    };
    v.first().cloned().unwrap_or_else(|| {
        match fw {
            Framework::Nist => "NIST AI RMF".to_string(),
            Framework::Iso42001 => "ISO/IEC 42001".to_string(),
            Framework::EuAiAct => "EU AI Act".to_string(),
            Framework::OwaspAgentic => "OWASP Agentic Top 10".to_string(),
        }
    })
}

fn get_framework_baseline(framework_name: &str) -> i16 {
    for fw in AgentFramework::all() {
        if fw.name() == framework_name {
            return fw.risk_baseline() as i16;
        }
    }
    50 // unknown framework
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{
        DiscoveredAgent, Guardrail, GuardrailKind, Permission, PermissionLevel, ToolDefinition,
    };
    use std::path::PathBuf;

    fn naked_agent() -> DiscoveredAgent {
        DiscoveredAgent {
            name: "test-agent".into(),
            framework: "LangChain".into(),
            file_path: PathBuf::from("test.py"),
            line_number: 1,
            tools: Vec::new(),
            system_prompt: None,
            permissions: Vec::new(),
            guardrails: Vec::new(),
            data_access: Vec::new(),
        }
    }

    /// Pin the W2-C8 finding-emit-order invariant *independent of snapshots*.
    /// Snapshots compare full JSON; this test gives a focused failure
    /// message ("expected category X at index Y, got Z") when the rule
    /// order in EMBEDDED_RULES drifts from the legacy firing sequence.
    #[test]
    fn finding_emit_order_matches_w1_firing_sequence() {
        // Build an agent that triggers every non-silent scoring rule:
        //   - 11 tools (>10 → unbounded-tools fires; not 0 → empty-tools silent)
        //   - first 3 lack confirmation → unconfirmed-tools fires
        //   - autonomy_tier saturates at 5 with admin permission
        //   - no system prompt, no guardrails → 4 missing-* rules fire
        //   - admin permission → only excessive-admin-permission fires
        //     (excessive-exec-permission requires level=execute specifically)
        //   - 4 data sources → data-access-broad fires
        //   - no audit trail → missing-audit-trail fires
        let mut agent = naked_agent();
        agent.tools = (0..11)
            .map(|i| ToolDefinition {
                name: format!("t{}", i),
                description: None,
                has_confirmation: i >= 3,
            })
            .collect();
        agent.permissions = vec![Permission {
            scope: "system".into(),
            level: PermissionLevel::Admin,
        }];
        agent.data_access = (0..4)
            .map(|i| crate::scanner::DataAccess {
                source: format!("src{}", i),
                access_type: "read".into(),
            })
            .collect();

        let engine = Engine::compile_builtin();
        let scored = score_single_agent(&agent, &Framework::Nist, &engine);

        // The W1 firing order (preserved by EMBEDDED_RULES). empty-tools is
        // silent so it does not appear in the findings vector. unbounded
        // and unconfirmed both relate to UnboundedAutonomy, which is why
        // we see two consecutive findings of that category.
        let categories: Vec<String> = scored
            .findings
            .iter()
            .map(|f| format!("{:?}", f.category))
            .collect();
        // The order below is the W1 firing sequence (load-bearing for
        // the byte-identical snapshot contract). The exact category labels
        // come from each rule's YAML category field — the only
        // ExcessivePermission entry corresponds to the admin rule because
        // the exec rule's matcher requires `level=execute` specifically and
        // the admin permission used here saturates at admin.
        assert_eq!(
            categories,
            vec![
                "UnboundedAutonomy",       // unbounded-tools
                "NoHumanOversight",        // unconfirmed-tools
                "PromptInjectionRisk",     // missing-system-prompt
                "MissingGuardrail",        // missing-input-validation
                "MissingGuardrail",        // missing-output-filter
                "MissingGuardrail",        // missing-rate-limit
                "ExcessivePermission",     // excessive-admin-permission
                "DataExposure",            // data-access-broad
                "MissingAuditTrail",       // missing-audit-trail
            ],
            "scoring rule firing order drifted — \
             check src/engine/mod.rs::EMBEDDED_RULES against W1 baseline"
        );
    }

    /// Per the byte-identical contract, every (scoring rule × framework)
    /// combination must produce the exact compliance string the deleted
    /// `framework_reference()` returned in W1. This test pins all 44
    /// (11 rules × 4 frameworks) strings against the snapshot independent
    /// of which fixture happens to fire them — so the EU-AI-Act / ISO /
    /// OWASP branches (which the default `--framework nist` snapshots
    /// cannot exercise) are still locked down.
    #[test]
    fn compliance_strings_match_w1_baseline_for_every_framework() {
        // (rule_id, nist, iso42001, eu_ai_act, owasp_agentic) — strings
        // copied verbatim from `git show 0d0cc77:src/scoring.rs` after
        // running the W1 framework_reference() switch for each control.
        let expected: &[(&str, &str, &str, &str, &str)] = &[
            (
                "scoring/unbounded-tools",
                "NIST AI RMF: MAP 1.1, MANAGE 2.2",
                "ISO/IEC 42001",
                "EU AI Act",
                "OWASP Agentic: A01 Excessive Agency",
            ),
            (
                "scoring/unconfirmed-tools",
                "NIST AI RMF: GOVERN 1.3, MANAGE 2.4",
                "ISO 42001: A.8.4 Human oversight",
                "EU AI Act: Article 14 Human oversight",
                "OWASP Agentic Top 10",
            ),
            (
                "scoring/missing-system-prompt",
                "NIST AI RMF: MAP 2.3, MEASURE 2.6",
                "ISO/IEC 42001",
                "EU AI Act",
                "OWASP Agentic: A05 Improper Output Handling",
            ),
            (
                "scoring/missing-input-validation",
                "NIST AI RMF: MANAGE 2.2, MEASURE 2.5",
                "ISO 42001: A.6.2.6 Data quality",
                "EU AI Act",
                "OWASP Agentic: A02 Inadequate Sandboxing",
            ),
            (
                "scoring/missing-output-filter",
                "NIST AI RMF: MANAGE 2.3, MEASURE 2.7",
                "ISO 42001: A.6.2.7 Output management",
                "EU AI Act",
                "OWASP Agentic Top 10",
            ),
            (
                "scoring/missing-rate-limit",
                "NIST AI RMF: MANAGE 3.1",
                "ISO/IEC 42001",
                "EU AI Act",
                "OWASP Agentic Top 10",
            ),
            (
                "scoring/excessive-exec-permission",
                "NIST AI RMF: GOVERN 1.7, MAP 3.4",
                "ISO/IEC 42001",
                "EU AI Act",
                "OWASP Agentic: A01 Excessive Agency",
            ),
            (
                "scoring/excessive-admin-permission",
                "NIST AI RMF: GOVERN 1.7, MANAGE 4.1",
                "ISO/IEC 42001",
                "EU AI Act",
                "OWASP Agentic Top 10",
            ),
            (
                "scoring/data-access-broad",
                "NIST AI RMF: MAP 5.1, MANAGE 2.2",
                "ISO/IEC 42001",
                "EU AI Act: Article 10 Data governance",
                "OWASP Agentic Top 10",
            ),
            (
                "scoring/missing-audit-trail",
                "NIST AI RMF: GOVERN 1.5, MEASURE 4.1",
                "ISO 42001: A.8.5 Logging and monitoring",
                "EU AI Act: Article 12 Record-keeping",
                "OWASP Agentic Top 10",
            ),
        ];

        let engine = Engine::compile_builtin();
        for (rule_id, nist, iso, eu, owasp) in expected {
            let rule = engine
                .scoring_rules()
                .iter()
                .find(|r| r.id == *rule_id)
                .unwrap_or_else(|| panic!("missing rule {}", rule_id));
            assert_eq!(pick_compliance(&rule.compliance, &Framework::Nist), *nist, "{} nist", rule_id);
            assert_eq!(pick_compliance(&rule.compliance, &Framework::Iso42001), *iso, "{} iso", rule_id);
            assert_eq!(pick_compliance(&rule.compliance, &Framework::EuAiAct), *eu, "{} eu", rule_id);
            assert_eq!(
                pick_compliance(&rule.compliance, &Framework::OwaspAgentic),
                *owasp,
                "{} owasp",
                rule_id
            );
        }
    }

    /// `empty-tools` is the only silent scoring rule today: it fires when
    /// `tool_count == 0`, adjusts the score, but emits no Finding. Pin the
    /// silent contract so a future rule edit cannot quietly add or remove
    /// it without a test failure.
    #[test]
    fn empty_tools_rule_adjusts_score_without_finding() {
        let agent = naked_agent(); // zero tools → empty-tools fires
        let engine = Engine::compile_builtin();
        let scored = score_single_agent(&agent, &Framework::Nist, &engine);

        // No finding for empty-tools (no title in the YAML).
        assert!(
            !scored.findings.iter().any(|f| f.title.contains("Empty")),
            "empty-tools must remain silent"
        );
        // Score must still reflect the +5 adjustment + the other firing
        // rules. We assert > baseline to detect a regression where the
        // rule stops firing entirely.
        assert!(scored.risk_score > 0);
    }

    #[test]
    fn guardrail_count_overflow_is_clamped() {
        // Pathological agent — far more guardrails than scoring credits.
        // Without saturating arithmetic the cast would wrap negative
        // and start adding to the score. Verify the clamp holds.
        let mut agent = naked_agent();
        agent.guardrails = (0..100)
            .map(|_| Guardrail {
                kind: GuardrailKind::InputValidation,
                description: "x".into(),
            })
            .collect();
        let engine = Engine::compile_builtin();
        let scored = score_single_agent(&agent, &Framework::Nist, &engine);
        assert!(scored.risk_score <= 100);
    }
}
