mod engine;
mod frameworks;
mod report;
mod rules;
mod scanner;
mod scoring;
mod signals;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agent-shield")]
#[command(
    about = "AI Agent Audit Scanner — discover, score, and report on AI agents in your codebase"
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory for AI agents and generate a risk report
    Scan {
        /// Path to the codebase to scan
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format
        #[arg(short, long, default_value = "terminal")]
        format: report::OutputFormat,

        /// Output file path (for json/pdf formats)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Compliance framework to map against
        #[arg(long, default_value = "nist")]
        framework: scoring::Framework,

        /// Minimum risk level to report (0-100)
        #[arg(long, default_value = "0")]
        min_risk: u8,
    },
    /// List supported agent frameworks
    Frameworks,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            path,
            format,
            output,
            framework,
            min_risk,
        } => {
            let scan_result = scanner::scan_directory(&path);

            match scan_result {
                Ok(agents) => {
                    if agents.is_empty() {
                        println!("No AI agents detected in {}", path.display());
                        return;
                    }

                    let scored = scoring::score_agents(&agents, &framework);
                    let filtered: Vec<_> = scored
                        .into_iter()
                        .filter(|a| a.risk_score >= min_risk)
                        .collect();

                    if let Err(e) =
                        report::render(&filtered, &framework, &format, output.as_deref())
                    {
                        eprintln!("Error writing report: {e}");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error scanning {}: {}", path.display(), e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Frameworks => {
            frameworks::list_frameworks();
        }
    }
}
