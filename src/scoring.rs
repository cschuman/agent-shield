use crate::scanner::DiscoveredAgent;
use crate::frameworks::AgentFramework;
use crate::signals::compute_all_signals;
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
    agents.iter().map(|agent| score_single_agent(agent, compliance_framework)).collect()
}

fn score_single_agent(agent: &DiscoveredAgent, compliance_framework: &Framework) -> ScoredAgent {
    // Compute the v1 fact set once. After W2-C8 the scoring rules will read
    // these signals via Matcher::matches_signals; for now the inline findings
    // below consume the pre-computed fields directly.
    let signals = compute_all_signals(agent);

    let mut score: i16 = get_framework_baseline(&agent.framework);
    let mut findings = Vec::new();

    // Autonomy tier assessment (NIST 4-tier)
    let autonomy_tier = signals.autonomy_tier;
    score += (autonomy_tier as i16 - 1) * 10;

    // Tool count risk
    if signals.tool_count == 0 {
        // No tools detected = might be missing detection, slight risk bump
        score += 5;
    } else if signals.tool_count > 10 {
        score += 15;
        findings.push(Finding {
            category: FindingCategory::UnboundedAutonomy,
            severity: Severity::Medium,
            title: format!("Agent has {} tools", signals.tool_count),
            description: "Agents with many tools have a larger attack surface and blast radius.".into(),
            remediation: "Apply principle of least privilege — only grant tools the agent needs for its specific task.".into(),
            framework_ref: framework_reference(compliance_framework, "tool-scope"),
        });
    }

    // Tools without confirmation gates
    if signals.unconfirmed_tool_count > 0 && signals.tool_count > 3 {
        score += 10;
        findings.push(Finding {
            category: FindingCategory::NoHumanOversight,
            severity: Severity::High,
            title: "Tools execute without human confirmation".into(),
            description: format!(
                "{} of {} tools can execute without human approval.",
                signals.unconfirmed_tool_count,
                signals.tool_count
            ),
            remediation: "Add human-in-the-loop confirmation for destructive or high-impact tool calls.".into(),
            framework_ref: framework_reference(compliance_framework, "human-oversight"),
        });
    }

    // System prompt analysis
    if !signals.has_system_prompt {
        score += 10;
        findings.push(Finding {
            category: FindingCategory::PromptInjectionRisk,
            severity: Severity::Medium,
            title: "No system prompt detected".into(),
            description: "Agent has no detectable system prompt defining its role, boundaries, or behavioral constraints.".into(),
            remediation: "Add an explicit system prompt that defines the agent's role, scope, and prohibited actions.".into(),
            framework_ref: framework_reference(compliance_framework, "prompt-safety"),
        });
    }

    // Guardrail assessment
    if !signals.guardrails.input_validation {
        score += 10;
        findings.push(Finding {
            category: FindingCategory::MissingGuardrail,
            severity: Severity::High,
            title: "No input validation detected".into(),
            description: "Agent does not appear to validate or sanitize inputs before processing.".into(),
            remediation: "Implement input validation and sanitization to prevent prompt injection attacks.".into(),
            framework_ref: framework_reference(compliance_framework, "input-validation"),
        });
    }

    if !signals.guardrails.output_filtering {
        score += 5;
        findings.push(Finding {
            category: FindingCategory::MissingGuardrail,
            severity: Severity::Medium,
            title: "No output filtering detected".into(),
            description: "Agent outputs are not filtered for sensitive data, PII, or inappropriate content.".into(),
            remediation: "Add output filtering to prevent data leakage and inappropriate responses.".into(),
            framework_ref: framework_reference(compliance_framework, "output-safety"),
        });
    }

    if !signals.guardrails.rate_limit {
        score += 5;
        findings.push(Finding {
            category: FindingCategory::MissingGuardrail,
            severity: Severity::Low,
            title: "No rate limiting detected".into(),
            description: "Agent has no apparent rate limiting, allowing unbounded execution.".into(),
            remediation: "Add rate limiting to prevent runaway costs and denial-of-service scenarios.".into(),
            framework_ref: framework_reference(compliance_framework, "rate-limit"),
        });
    }

    // Permission assessment — bound locally so the permission_summary block
    // below stays readable.
    let has_exec = signals.permissions.execute;
    let has_admin = signals.permissions.admin;
    let has_write = signals.permissions.write;

    if has_exec {
        score += 20;
        findings.push(Finding {
            category: FindingCategory::ExcessivePermission,
            severity: Severity::Critical,
            title: "Agent can execute system commands".into(),
            description: "Agent has access to system command execution (subprocess, shell, exec).".into(),
            remediation: "Remove system command access unless absolutely required. If required, implement strict allowlisting.".into(),
            framework_ref: framework_reference(compliance_framework, "exec-permission"),
        });
    }

    if has_admin {
        score += 15;
        findings.push(Finding {
            category: FindingCategory::ExcessivePermission,
            severity: Severity::Critical,
            title: "Agent has admin-level permissions".into(),
            description: "Agent operates with elevated/admin privileges.".into(),
            remediation: "Apply principle of least privilege. Run agents with minimal required permissions.".into(),
            framework_ref: framework_reference(compliance_framework, "least-privilege"),
        });
    }

    // Data access assessment
    if signals.data_source_count > 3 {
        score += 10;
        findings.push(Finding {
            category: FindingCategory::DataExposure,
            severity: Severity::Medium,
            title: format!("Agent accesses {} data sources", signals.data_source_count),
            description: "Agent has broad data access across multiple sources, increasing blast radius.".into(),
            remediation: "Restrict data access to only the sources required for the agent's specific function.".into(),
            framework_ref: framework_reference(compliance_framework, "data-access"),
        });
    }

    // No audit trail detection
    if !signals.has_audit_trail {
        score += 5;
        findings.push(Finding {
            category: FindingCategory::MissingAuditTrail,
            severity: Severity::Medium,
            title: "No audit trail detected".into(),
            description: "Agent actions are not logged for audit or forensic purposes.".into(),
            remediation: "Implement comprehensive logging of all agent decisions, tool calls, and outputs.".into(),
            framework_ref: framework_reference(compliance_framework, "audit-trail"),
        });
    }

    // Guardrail credit (reduce score for good practices)
    let guardrail_credit = (agent.guardrails.len() as i16 * 5).min(25);
    score -= guardrail_credit;

    // Clamp to 0-100
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

fn get_framework_baseline(framework_name: &str) -> i16 {
    for fw in AgentFramework::all() {
        if fw.name() == framework_name {
            return fw.risk_baseline() as i16;
        }
    }
    50 // unknown framework
}

fn framework_reference(framework: &Framework, control: &str) -> String {
    match framework {
        Framework::Nist => match control {
            "tool-scope" => "NIST AI RMF: MAP 1.1, MANAGE 2.2".into(),
            "human-oversight" => "NIST AI RMF: GOVERN 1.3, MANAGE 2.4".into(),
            "prompt-safety" => "NIST AI RMF: MAP 2.3, MEASURE 2.6".into(),
            "input-validation" => "NIST AI RMF: MANAGE 2.2, MEASURE 2.5".into(),
            "output-safety" => "NIST AI RMF: MANAGE 2.3, MEASURE 2.7".into(),
            "rate-limit" => "NIST AI RMF: MANAGE 3.1".into(),
            "exec-permission" => "NIST AI RMF: GOVERN 1.7, MAP 3.4".into(),
            "least-privilege" => "NIST AI RMF: GOVERN 1.7, MANAGE 4.1".into(),
            "data-access" => "NIST AI RMF: MAP 5.1, MANAGE 2.2".into(),
            "audit-trail" => "NIST AI RMF: GOVERN 1.5, MEASURE 4.1".into(),
            _ => "NIST AI RMF".into(),
        },
        Framework::Iso42001 => match control {
            "human-oversight" => "ISO 42001: A.8.4 Human oversight".into(),
            "input-validation" => "ISO 42001: A.6.2.6 Data quality".into(),
            "output-safety" => "ISO 42001: A.6.2.7 Output management".into(),
            "audit-trail" => "ISO 42001: A.8.5 Logging and monitoring".into(),
            _ => "ISO/IEC 42001".into(),
        },
        Framework::EuAiAct => match control {
            "human-oversight" => "EU AI Act: Article 14 Human oversight".into(),
            "audit-trail" => "EU AI Act: Article 12 Record-keeping".into(),
            "data-access" => "EU AI Act: Article 10 Data governance".into(),
            _ => "EU AI Act".into(),
        },
        Framework::OwaspAgentic => match control {
            "exec-permission" => "OWASP Agentic: A01 Excessive Agency".into(),
            "tool-scope" => "OWASP Agentic: A01 Excessive Agency".into(),
            "prompt-safety" => "OWASP Agentic: A05 Improper Output Handling".into(),
            "input-validation" => "OWASP Agentic: A02 Inadequate Sandboxing".into(),
            _ => "OWASP Agentic Top 10".into(),
        },
    }
}
