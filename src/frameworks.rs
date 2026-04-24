use crate::engine::{Engine, describe_matcher};
use colored::Colorize;
use comfy_table::{Table, ContentArrangement};

/// Supported agent framework families.
///
/// As of C4, the per-framework *detection patterns* live in `engine::compile_builtin`
/// (with translation kept in lockstep with the variants below). This enum
/// retains identity, display name, and risk baseline; the legacy
/// `detection_patterns()` accessor and `DetectionPattern` enum were removed
/// alongside `scanner.rs`'s switch to the `Engine`.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentFramework {
    LangChain,
    LangGraph,
    CrewAI,
    AutoGen,
    OpenAIAssistants,
    AnthropicMCP,
    AnthropicAgentSDK,
    AWSBedrock,
    VercelAI,
    CustomAgent,
}

impl AgentFramework {
    pub fn name(&self) -> &str {
        match self {
            Self::LangChain => "LangChain",
            Self::LangGraph => "LangGraph",
            Self::CrewAI => "CrewAI",
            Self::AutoGen => "AutoGen (Microsoft)",
            Self::OpenAIAssistants => "OpenAI Assistants",
            Self::AnthropicMCP => "Anthropic MCP",
            Self::AnthropicAgentSDK => "Anthropic Agent SDK",
            Self::AWSBedrock => "AWS Bedrock Agents",
            Self::VercelAI => "Vercel AI SDK",
            Self::CustomAgent => "Custom Agent",
        }
    }

    pub fn risk_baseline(&self) -> u8 {
        match self {
            Self::LangChain | Self::LangGraph => 40,
            Self::CrewAI => 50,       // multi-agent = higher baseline
            Self::AutoGen => 50,      // multi-agent
            Self::OpenAIAssistants => 35,
            Self::AnthropicMCP => 30, // tool-use focused
            Self::AnthropicAgentSDK => 45,
            Self::AWSBedrock => 35,
            Self::VercelAI => 25,     // typically simpler
            Self::CustomAgent => 55,  // unknown = higher risk
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::LangChain,
            Self::LangGraph,
            Self::CrewAI,
            Self::AutoGen,
            Self::OpenAIAssistants,
            Self::AnthropicMCP,
            Self::AnthropicAgentSDK,
            Self::AWSBedrock,
            Self::VercelAI,
            Self::CustomAgent,
        ]
    }
}

impl std::fmt::Display for AgentFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub fn list_frameworks() {
    println!("{}", "Supported Agent Frameworks".bold().underline());
    println!();

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Framework", "Risk Baseline", "Detection Method"]);

    let engine = Engine::compile_builtin();

    for rule in &engine.rules.rules {
        let fw = &rule.framework;
        let risk = fw.risk_baseline();
        let risk_display = if risk >= 50 {
            format!("{}/100", risk).red().to_string()
        } else if risk >= 35 {
            format!("{}/100", risk).yellow().to_string()
        } else {
            format!("{}/100", risk).green().to_string()
        };

        let methods: Vec<String> = describe_matcher(&rule.matcher)
            .into_iter()
            .take(2)
            .collect();

        table.add_row(vec![fw.name().to_string(), risk_display, methods.join(", ")]);
    }

    println!("{table}");
}
