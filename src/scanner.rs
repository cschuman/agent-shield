use crate::engine::Engine;
use crate::engine::matcher::{FileCtx, Lang};
use crate::frameworks::AgentFramework;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredAgent {
    pub name: String,
    pub framework: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub tools: Vec<ToolDefinition>,
    pub system_prompt: Option<String>,
    pub permissions: Vec<Permission>,
    pub guardrails: Vec<Guardrail>,
    pub data_access: Vec<DataAccess>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub has_confirmation: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Permission {
    pub scope: String,
    pub level: PermissionLevel,
}

#[derive(Debug, Clone, Serialize)]
pub enum PermissionLevel {
    Read,
    Write,
    Execute,
    Admin,
}

#[derive(Debug, Clone, Serialize)]
pub struct Guardrail {
    pub kind: GuardrailKind,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum GuardrailKind {
    InputValidation,
    OutputFiltering,
    RateLimit,
    HumanApproval,
    ContentFilter,
    TokenLimit,
    TimeoutLimit,
    ScopeRestriction,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataAccess {
    pub source: String,
    pub access_type: String,
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".svelte-kit",
    "vendor",
];

const SCAN_EXTENSIONS: &[&str] = &[
    "py", "js", "ts", "tsx", "jsx", "rs", "go", "java", "yaml", "yml", "json", "toml",
];

pub fn scan_directory(path: &Path) -> Result<Vec<DiscoveredAgent>, Box<dyn std::error::Error>> {
    let mut agents = Vec::new();
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    let engine = Engine::compile_builtin();

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SKIP_DIRS.iter().any(|skip| name == *skip)
        })
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if !SCAN_EXTENSIONS.contains(&ext) {
            continue;
        }

        let lang = match lang_from_ext(ext) {
            Some(l) => l,
            None => continue,
        };

        if seen_files.contains(file_path) {
            continue;
        }
        seen_files.insert(file_path.to_path_buf());

        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue, // skip binary/unreadable files
        };

        let ctx = FileCtx {
            path: file_path,
            lang,
            content: &content,
        };

        for rule in &engine.rules.rules {
            let hits = rule.matcher.matches_file(&ctx);
            if hits.len() >= rule.min_match_count as usize {
                let raw_matches: Vec<(usize, String)> =
                    hits.into_iter().map(|h| (h.line, h.snippet)).collect();
                let agent = extract_agent_details(
                    file_path,
                    &content,
                    &rule.framework,
                    &raw_matches,
                );
                agents.push(agent);
            }
        }
    }

    // Deduplicate agents by file + framework
    agents.dedup_by(|a, b| a.file_path == b.file_path && a.framework == b.framework);

    Ok(agents)
}

fn lang_from_ext(ext: &str) -> Option<Lang> {
    Some(match ext {
        "py" => Lang::Python,
        "js" | "jsx" => Lang::JavaScript,
        "ts" | "tsx" => Lang::TypeScript,
        "rs" => Lang::Rust,
        "go" => Lang::Go,
        "java" => Lang::Java,
        "yaml" | "yml" => Lang::Yaml,
        "json" => Lang::Json,
        "toml" => Lang::Toml,
        _ => return None,
    })
}

fn extract_agent_details(
    file_path: &Path,
    content: &str,
    framework: &AgentFramework,
    matches: &[(usize, String)],
) -> DiscoveredAgent {
    let first_match = matches.first().map(|(l, _)| *l).unwrap_or(0);

    // Extract agent name from common patterns
    let name = extract_agent_name(content, framework)
        .unwrap_or_else(|| format!("{} agent", framework.name()));

    // Extract tools
    let tools = extract_tools(content, framework);

    // Extract system prompt
    let system_prompt = extract_system_prompt(content);

    // Detect guardrails
    let guardrails = detect_guardrails(content);

    // Detect data access patterns
    let data_access = detect_data_access(content);

    // Detect permissions
    let permissions = detect_permissions(content);

    DiscoveredAgent {
        name,
        framework: framework.name().to_string(),
        file_path: file_path.to_path_buf(),
        line_number: first_match,
        tools,
        system_prompt,
        permissions,
        guardrails,
        data_access,
    }
}

fn extract_agent_name(content: &str, framework: &AgentFramework) -> Option<String> {
    // Framework-specific naming patterns first
    let name_patterns = [
        r#"agent_name\s*[=:]\s*["']([^"']+)["']"#,
        r#"Agent\(\s*(?:name\s*=\s*)?["']([^"']+)["']"#,
        r#"assistant.*?name["']?\s*[:=]\s*["']([^"']+)["']"#,
        r#"class\s+(\w+Agent)\b"#,
        r#"(?:def|const|let|var)\s+(\w*[Aa]gent\w*)\s*="#,
    ];

    for pat in &name_patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(caps) = re.captures(content) {
                if let Some(name) = caps.get(1) {
                    let n = name.as_str();
                    // Filter out false positive names
                    if n.len() > 2
                        && n.len() < 60
                        && !n.contains("Error")
                        && !n.contains("Exception")
                        && !n.contains("Test")
                        && !n.contains("Mock")
                    {
                        return Some(n.to_string());
                    }
                }
            }
        }
    }

    None
}

fn extract_tools(content: &str, _framework: &AgentFramework) -> Vec<ToolDefinition> {
    let mut tools = Vec::new();

    // Detect tool definitions across frameworks
    let tool_patterns = [
        r#"(?:@tool|Tool\(|\.tool\(|add_tool|tools\s*=)\s*.*?["'](\w+)["']"#,
        r#"function_call.*["'](\w+)["']"#,
        r#"tool_choice.*["'](\w+)["']"#,
        r#"name:\s*["'](\w+)["'].*?(?:description|desc)"#,
    ];

    for pat in &tool_patterns {
        if let Ok(re) = Regex::new(pat) {
            for caps in re.captures_iter(content) {
                if let Some(name) = caps.get(1) {
                    let tool_name = name.as_str().to_string();
                    if !tools.iter().any(|t: &ToolDefinition| t.name == tool_name) {
                        let has_confirmation = content.contains("confirm")
                            || content.contains("approval")
                            || content.contains("human_in_the_loop");

                        tools.push(ToolDefinition {
                            name: tool_name,
                            description: None,
                            has_confirmation,
                        });
                    }
                }
            }
        }
    }

    tools
}

fn extract_system_prompt(content: &str) -> Option<String> {
    let prompt_patterns = [
        r#"system_prompt\s*[=:]\s*["'`]{1,3}([\s\S]*?)["'`]{1,3}"#,
        r#"systemPrompt\s*[=:]\s*["'`]{1,3}([\s\S]*?)["'`]{1,3}"#,
        r#"system_message\s*[=:]\s*["'`]{1,3}([\s\S]*?)["'`]{1,3}"#,
        r#"\{"role":\s*"system",\s*"content":\s*"((?:[^"\\]|\\.)*)"\}"#,
    ];

    for pat in &prompt_patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(caps) = re.captures(content) {
                if let Some(prompt) = caps.get(1) {
                    let text = prompt.as_str().to_string();
                    // Truncate for storage
                    if text.len() > 500 {
                        return Some(format!("{}...", &text[..500]));
                    }
                    return Some(text);
                }
            }
        }
    }

    None
}

fn detect_guardrails(content: &str) -> Vec<Guardrail> {
    let mut guardrails = Vec::new();

    let checks = [
        ("input.*valid|validate.*input|sanitize", GuardrailKind::InputValidation, "Input validation detected"),
        ("output.*filter|filter.*output|content.*filter", GuardrailKind::OutputFiltering, "Output filtering detected"),
        ("rate.*limit|throttle|RateLimiter", GuardrailKind::RateLimit, "Rate limiting detected"),
        ("human.*approve|human.*loop|require.*approval|confirm.*action", GuardrailKind::HumanApproval, "Human approval gate detected"),
        ("content.*policy|moderation|safety.*check|guardrail", GuardrailKind::ContentFilter, "Content filtering detected"),
        ("max.*tokens|token.*limit|max_tokens", GuardrailKind::TokenLimit, "Token limit set"),
        ("timeout|max.*time|deadline", GuardrailKind::TimeoutLimit, "Timeout limit set"),
        ("allowed.*tools|tool.*whitelist|scope.*restrict", GuardrailKind::ScopeRestriction, "Scope restriction detected"),
    ];

    for (pattern, kind, description) in &checks {
        if let Ok(re) = Regex::new(&format!("(?i){}", pattern)) {
            if re.is_match(content) {
                guardrails.push(Guardrail {
                    kind: kind.clone(),
                    description: description.to_string(),
                });
            }
        }
    }

    guardrails
}

fn detect_data_access(content: &str) -> Vec<DataAccess> {
    let mut access = Vec::new();

    let patterns = [
        (r#"(?i)(?:postgres|mysql|sqlite|mongodb|redis|prisma|kysely|drizzle)\s*[(\.\{]"#, "Database"),
        (r#"(?i)(?:s3|S3Client|bucket|BlobService|Storage\(\))"#, "Cloud Storage"),
        (r#"(?i)(?:api_key|API_KEY|bearer|Authorization)\s*[=:]"#, "External API"),
        (r#"(?i)(?:readFile|read_file|open\(\s*['"]|fs\.read)"#, "File System"),
        (r#"(?i)(?:sendEmail|smtp|sendgrid|ses\.send)"#, "Email Service"),
        (r#"(?i)(?:webhook|callback_url)\s*[=:]"#, "Webhook/Notification"),
        (r#"(?i)(?:subprocess|os\.system|child_process|Command::new)"#, "System Command"),
    ];

    for (pattern, source) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(content) {
                let access_type = if content.contains("write") || content.contains("insert")
                    || content.contains("update") || content.contains("delete")
                    || content.contains("put") || content.contains("post")
                {
                    "read/write"
                } else {
                    "read"
                };
                access.push(DataAccess {
                    source: source.to_string(),
                    access_type: access_type.to_string(),
                });
            }
        }
    }

    access
}

fn detect_permissions(content: &str) -> Vec<Permission> {
    let mut permissions = Vec::new();

    let patterns = [
        (r"(?i)(?:subprocess|os\.system|child_process|exec\s*\(|shell\s*\(|Command::new)", "system_exec", PermissionLevel::Execute),
        (r"(?i)(?:\.insert|\.update|\.delete|\.put|\.create)\s*\(", "data_write", PermissionLevel::Write),
        (r"(?i)(?:admin|superuser|root|sudo|elevated)\s*[=:]", "admin", PermissionLevel::Admin),
        (r"(?i)(?:\.query|\.select|\.find|\.get)\s*\(", "data_read", PermissionLevel::Read),
    ];

    for (pattern, scope, level) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(content) {
                permissions.push(Permission {
                    scope: scope.to_string(),
                    level: level.clone(),
                });
            }
        }
    }

    permissions
}

