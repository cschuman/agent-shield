use colored::Colorize;
use comfy_table::{Table, ContentArrangement};

/// Agent framework detection patterns
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

    pub fn detection_patterns(&self) -> Vec<DetectionPattern> {
        match self {
            Self::LangChain => vec![
                DetectionPattern::Import("langchain"),
                DetectionPattern::Import("from langchain"),
                DetectionPattern::PackageDep("langchain"),
                DetectionPattern::PackageDep("@langchain/core"),
            ],
            Self::LangGraph => vec![
                DetectionPattern::Import("langgraph"),
                DetectionPattern::Import("from langgraph"),
                DetectionPattern::PackageDep("langgraph"),
                DetectionPattern::PackageDep("@langchain/langgraph"),
            ],
            Self::CrewAI => vec![
                DetectionPattern::Import("crewai"),
                DetectionPattern::Import("from crewai"),
                DetectionPattern::PackageDep("crewai"),
            ],
            Self::AutoGen => vec![
                DetectionPattern::Import("autogen"),
                DetectionPattern::Import("from autogen"),
                DetectionPattern::PackageDep("autogen"),
                DetectionPattern::PackageDep("pyautogen"),
            ],
            Self::OpenAIAssistants => vec![
                DetectionPattern::CodePattern(r"client\.beta\.assistants"),
                DetectionPattern::CodePattern(r"openai\.beta\.assistants"),
                DetectionPattern::CodePattern(r"assistants\.create"),
                DetectionPattern::CodePattern(r#"type.*=.*"assistant""#),
            ],
            Self::AnthropicMCP => vec![
                DetectionPattern::Import("@modelcontextprotocol"),
                DetectionPattern::Import("mcp"),
                DetectionPattern::ConfigFile("mcp.json"),
                DetectionPattern::ConfigFile(".mcp.json"),
                DetectionPattern::CodePattern(r"McpServer"),
                DetectionPattern::CodePattern(r"mcp_server"),
            ],
            Self::AnthropicAgentSDK => vec![
                DetectionPattern::Import("claude_agent_sdk"),
                DetectionPattern::Import("@anthropic-ai/agent-sdk"),
                DetectionPattern::PackageDep("claude-agent-sdk"),
            ],
            Self::AWSBedrock => vec![
                DetectionPattern::CodePattern(r"bedrock-agent"),
                DetectionPattern::CodePattern(r"BedrockAgent"),
                DetectionPattern::CodePattern(r"bedrock_agent"),
                DetectionPattern::Import("@aws-sdk/client-bedrock-agent"),
            ],
            Self::VercelAI => vec![
                DetectionPattern::Import("ai/"),
                DetectionPattern::Import("from 'ai'"),
                DetectionPattern::Import("from \"ai\""),
                DetectionPattern::CodePattern(r"\bgenerateText\s*\("),
                DetectionPattern::CodePattern(r"\bstreamText\s*\("),
                DetectionPattern::CodePattern(r"\btool\s*\(\s*\{"),
            ],
            Self::CustomAgent => vec![
                DetectionPattern::CodePattern(r"(?:system_prompt|systemPrompt)\s*[=:]"),
                DetectionPattern::CodePattern(r"(?:tool_call|toolCall|function_call)\s*[=:\(]"),
                DetectionPattern::CodePattern(r"agent[_.](?:loop|run|execute|step)\s*\("),
            ],
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

#[derive(Debug, Clone)]
pub enum DetectionPattern {
    Import(&'static str),
    PackageDep(&'static str),
    ConfigFile(&'static str),
    CodePattern(&'static str),
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

    for fw in AgentFramework::all() {
        let risk = fw.risk_baseline();
        let risk_display = if risk >= 50 {
            format!("{}/100", risk).red().to_string()
        } else if risk >= 35 {
            format!("{}/100", risk).yellow().to_string()
        } else {
            format!("{}/100", risk).green().to_string()
        };

        let methods: Vec<String> = fw
            .detection_patterns()
            .iter()
            .take(2)
            .map(|p| match p {
                DetectionPattern::Import(s) => format!("import: {}", s),
                DetectionPattern::PackageDep(s) => format!("package: {}", s),
                DetectionPattern::ConfigFile(s) => format!("config: {}", s),
                DetectionPattern::CodePattern(s) => format!("pattern: {}", s),
            })
            .collect();

        table.add_row(vec![fw.name().to_string(), risk_display, methods.join(", ")]);
    }

    println!("{table}");
}
