use crate::scoring::{FindingCategory, Framework, RiskLevel, ScoredAgent, Severity};
use chrono::Utc;
use colored::Colorize;
use comfy_table::{Cell, Color as TableColor, ContentArrangement, Table};
use std::path::Path;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Terminal,
    Json,
}

pub fn render(
    agents: &[ScoredAgent],
    framework: &Framework,
    format: &OutputFormat,
    output_path: Option<&Path>,
) {
    match format {
        OutputFormat::Terminal => render_terminal(agents, framework),
        OutputFormat::Json => render_json(agents, output_path),
    }
}

fn render_terminal(agents: &[ScoredAgent], framework: &Framework) {
    let now = Utc::now().format("%Y-%m-%d %H:%M UTC");
    let overall_score = calculate_overall_score(agents);
    let (overall_level, level_color) = risk_level_display(overall_score);

    // Header
    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════════╗".bold()
    );
    println!(
        "{}",
        "║              AGENT SHIELD — Risk Assessment Report          ║".bold()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".bold()
    );
    println!();
    println!("  {}  {}", "Scan date:".dimmed(), now);
    println!("  {}  {}", "Framework:".dimmed(), framework);
    println!("  {}  {}", "Agents found:".dimmed(), agents.len());
    println!(
        "  {}  {}",
        "Overall risk:".dimmed(),
        format!("{}/100 ({})", overall_score, overall_level).color(level_color)
    );
    println!();

    // Overall risk gauge
    print_risk_gauge(overall_score);
    println!();

    // Agent summary table
    println!(
        "{}",
        "─── Agent Inventory ───────────────────────────────────────────".dimmed()
    );
    println!();

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "Agent",
        "Framework",
        "Risk",
        "Tools",
        "Guardrails",
        "Permissions",
        "Autonomy",
    ]);

    for agent in agents {
        let risk_color = match agent.risk_level {
            RiskLevel::Low => TableColor::Green,
            RiskLevel::Medium => TableColor::Yellow,
            RiskLevel::High => TableColor::Red,
            RiskLevel::Critical => TableColor::DarkRed,
        };

        let autonomy_label = match agent.autonomy_tier {
            1 => "T1: Supervised",
            2 => "T2: Constrained",
            3 => "T3: Broad",
            _ => "T4: Full",
        };

        table.add_row(vec![
            Cell::new(&agent.name),
            Cell::new(&agent.framework),
            Cell::new(format!("{}/100 {}", agent.risk_score, agent.risk_level)).fg(risk_color),
            Cell::new(agent.tool_count.to_string()),
            Cell::new(agent.guardrail_count.to_string()),
            Cell::new(&agent.permission_summary),
            Cell::new(autonomy_label),
        ]);
    }

    println!("{table}");
    println!();

    // Findings by severity
    let mut all_findings: Vec<_> = agents
        .iter()
        .flat_map(|a| a.findings.iter().map(move |f| (&a.name, f)))
        .collect();

    // Sort by severity (critical first)
    all_findings.sort_by(|a, b| severity_rank(&b.1.severity).cmp(&severity_rank(&a.1.severity)));

    if !all_findings.is_empty() {
        println!(
            "{}",
            "─── Findings ─────────────────────────────────────────────────".dimmed()
        );
        println!();

        for (agent_name, finding) in &all_findings {
            let severity_display = match finding.severity {
                Severity::Critical => "CRITICAL".red().bold().to_string(),
                Severity::High => "HIGH".red().to_string(),
                Severity::Medium => "MEDIUM".yellow().to_string(),
                Severity::Low => "LOW".blue().to_string(),
                Severity::Info => "INFO".dimmed().to_string(),
            };

            let category_icon = match finding.category {
                FindingCategory::MissingGuardrail => "🛡",
                FindingCategory::ExcessivePermission => "🔓",
                FindingCategory::DataExposure => "💾",
                FindingCategory::PromptInjectionRisk => "💉",
                FindingCategory::NoHumanOversight => "👤",
                FindingCategory::UnboundedAutonomy => "🤖",
                FindingCategory::MissingAuditTrail => "📋",
            };

            println!(
                "  {} [{}] {} — {}",
                category_icon,
                severity_display,
                finding.title.bold(),
                agent_name
            );
            println!("    {}", finding.description.dimmed());
            println!("    {} {}", "Fix:".green(), finding.remediation);
            println!("    {} {}", "Ref:".dimmed(), finding.framework_ref.dimmed());
            println!();
        }
    }

    // Summary stats
    println!(
        "{}",
        "─── Summary ──────────────────────────────────────────────────".dimmed()
    );
    println!();

    let critical_count = all_findings
        .iter()
        .filter(|(_, f)| matches!(f.severity, Severity::Critical))
        .count();
    let high_count = all_findings
        .iter()
        .filter(|(_, f)| matches!(f.severity, Severity::High))
        .count();
    let medium_count = all_findings
        .iter()
        .filter(|(_, f)| matches!(f.severity, Severity::Medium))
        .count();
    let low_count = all_findings
        .iter()
        .filter(|(_, f)| matches!(f.severity, Severity::Low))
        .count();

    println!(
        "  {} {}  {} {}  {} {}  {} {}",
        "CRITICAL:".red().bold(),
        critical_count,
        "HIGH:".red(),
        high_count,
        "MEDIUM:".yellow(),
        medium_count,
        "LOW:".blue(),
        low_count,
    );
    println!();

    let total_guardrails: usize = agents.iter().map(|a| a.guardrail_count).sum();
    let agents_with_prompts = agents.iter().filter(|a| a.has_system_prompt).count();
    let total_tools: usize = agents.iter().map(|a| a.tool_count).sum();

    println!(
        "  {}  {} across {} agents",
        "Total tools:".dimmed(),
        total_tools,
        agents.len()
    );
    println!("  {}  {}", "Total guardrails:".dimmed(), total_guardrails);
    println!(
        "  {}  {}/{}",
        "System prompts:".dimmed(),
        agents_with_prompts,
        agents.len()
    );
    println!();

    // Footer
    println!(
        "{}",
        "──────────────────────────────────────────────────────────────".dimmed()
    );
    println!(
        "  {} https://agentshield.dev",
        "Full report & remediation guide:".dimmed()
    );
    println!(
        "  {} agent-shield scan --format json -o report.json",
        "Export JSON:".dimmed()
    );
    println!(
        "{}",
        "──────────────────────────────────────────────────────────────".dimmed()
    );
    println!();
}

fn render_json(agents: &[ScoredAgent], output_path: Option<&Path>) {
    let report = serde_json::json!({
        "agent_shield_version": env!("CARGO_PKG_VERSION"),
        "scan_date": Utc::now().to_rfc3339(),
        "overall_risk_score": calculate_overall_score(agents),
        "agent_count": agents.len(),
        "agents": agents,
    });

    let json = serde_json::to_string_pretty(&report).unwrap();

    if let Some(path) = output_path {
        std::fs::write(path, &json).unwrap();
        println!("Report written to {}", path.display());
    } else {
        println!("{}", json);
    }
}

fn calculate_overall_score(agents: &[ScoredAgent]) -> u8 {
    if agents.is_empty() {
        return 0;
    }
    // Overall score is the max individual score (weakest link)
    agents.iter().map(|a| a.risk_score).max().unwrap_or(0)
}

fn risk_level_display(score: u8) -> (&'static str, colored::Color) {
    match score {
        0..=25 => ("LOW", colored::Color::Green),
        26..=50 => ("MEDIUM", colored::Color::Yellow),
        51..=75 => ("HIGH", colored::Color::Red),
        _ => ("CRITICAL", colored::Color::Red),
    }
}

fn print_risk_gauge(score: u8) {
    let filled = (score as usize) / 2; // 50 char width
    let _empty = 50 - filled;

    print!("  [");
    for i in 0..50 {
        if i < filled {
            if i < 12 {
                print!("{}", "█".green());
            } else if i < 25 {
                print!("{}", "█".yellow());
            } else if i < 37 {
                print!("{}", "█".red());
            } else {
                print!("{}", "█".red().bold());
            }
        } else {
            print!("{}", "░".dimmed());
        }
    }
    println!("] {}/100", score);
}

fn severity_rank(severity: &Severity) -> u8 {
    match severity {
        Severity::Critical => 5,
        Severity::High => 4,
        Severity::Medium => 3,
        Severity::Low => 2,
        Severity::Info => 1,
    }
}
