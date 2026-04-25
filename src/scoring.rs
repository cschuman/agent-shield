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
    score += (autonomy_tier as i16 - 1) * 10;

    // Drive findings + score adjustments off the YAML-defined scoring rule
    // set, in source order. EMBEDDED_RULES enforces firing order to keep
    // snapshot output byte-identical.
    for rule in engine.scoring_rules() {
        if rule.matcher.matches_signals(&signals).is_empty() {
            continue;
        }
        score += rule.score_adjustment as i16;
        if let Some(finding) = materialize_finding(rule, compliance_framework, &signals) {
            findings.push(finding);
        }
    }

    // Permission summary uses the bool flags computed in signals.rs.
    let has_exec = signals.permissions.execute;
    let has_admin = signals.permissions.admin;
    let has_write = signals.permissions.write;

    // Guardrail credit — unconditional reduction, scalar not finding.
    let guardrail_credit = (agent.guardrails.len() as i16 * 5).min(25);
    score -= guardrail_credit;

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

