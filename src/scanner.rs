use crate::engine::Engine;
use crate::engine::matcher::{FileCtx, Lang};
use crate::frameworks::AgentFramework;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;

// Files larger than this are skipped during scanning. A malicious or
// generated tree can otherwise force the scanner to allocate the full
// file contents per regex pass; capping at 10 MB keeps memory bounded
// without losing real-world agent files (the largest agent file in the
// calibration corpus was ~120 KB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

// Per-file extractor regexes are compiled once at process start instead
// of per-file. Calibration showed 32 `Regex::new` calls per scanned file
// dominated wall-clock time; hoisting moved that cost to a single one-time
// init and yielded a measurable speedup on multi-thousand-file scans.
// Patterns that fail to compile here are programmer errors — fail loudly
// rather than swallowing the error.
static AGENT_NAME_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r#"agent_name\s*[=:]\s*["']([^"']+)["']"#,
        r#"Agent\(\s*(?:name\s*=\s*)?["']([^"']+)["']"#,
        r#"assistant.*?name["']?\s*[:=]\s*["']([^"']+)["']"#,
        r#"class\s+(\w+Agent)\b"#,
        r#"(?:def|const|let|var)\s+(\w*[Aa]gent\w*)\s*="#,
    ]
    .iter()
    .map(|p| Regex::new(p).expect("agent-name pattern"))
    .collect()
});

static TOOL_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r#"(?:@tool|Tool\(|\.tool\(|add_tool|tools\s*=)\s*.*?["'](\w+)["']"#,
        r#"function_call.*["'](\w+)["']"#,
        r#"tool_choice.*["'](\w+)["']"#,
        r#"name:\s*["'](\w+)["'].*?(?:description|desc)"#,
    ]
    .iter()
    .map(|p| Regex::new(p).expect("tool pattern"))
    .collect()
});

static SYSTEM_PROMPT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r#"system_prompt\s*[=:]\s*["'`]{1,3}([\s\S]*?)["'`]{1,3}"#,
        r#"systemPrompt\s*[=:]\s*["'`]{1,3}([\s\S]*?)["'`]{1,3}"#,
        r#"system_message\s*[=:]\s*["'`]{1,3}([\s\S]*?)["'`]{1,3}"#,
        r#"\{"role":\s*"system",\s*"content":\s*"((?:[^"\\]|\\.)*)"\}"#,
    ]
    .iter()
    .map(|p| Regex::new(p).expect("system-prompt pattern"))
    .collect()
});

static GUARDRAIL_CHECKS: LazyLock<Vec<(Regex, GuardrailKind, &'static str)>> = LazyLock::new(|| {
    let raw: &[(&str, GuardrailKind, &str)] = &[
        (
            "input.*valid|validate.*input|sanitize",
            GuardrailKind::InputValidation,
            "Input validation detected",
        ),
        (
            "output.*filter|filter.*output|content.*filter",
            GuardrailKind::OutputFiltering,
            "Output filtering detected",
        ),
        (
            "rate.*limit|throttle|RateLimiter",
            GuardrailKind::RateLimit,
            "Rate limiting detected",
        ),
        (
            "human.*approve|human.*loop|require.*approval|confirm.*action",
            GuardrailKind::HumanApproval,
            "Human approval gate detected",
        ),
        (
            "content.*policy|moderation|safety.*check|guardrail",
            GuardrailKind::ContentFilter,
            "Content filtering detected",
        ),
        (
            "max.*tokens|token.*limit|max_tokens",
            GuardrailKind::TokenLimit,
            "Token limit set",
        ),
        (
            "timeout|max.*time|deadline",
            GuardrailKind::TimeoutLimit,
            "Timeout limit set",
        ),
        (
            "allowed.*tools|tool.*whitelist|scope.*restrict",
            GuardrailKind::ScopeRestriction,
            "Scope restriction detected",
        ),
        // Audit-trail detection — looks for explicit logging or audit
        // hooks. Required by NIST AI RMF MEASURE-2.7 and OWASP A06.
        // Patterns intentionally narrow: structured logger imports,
        // standard logging packages, and explicit audit-event APIs.
        // Plain `print()` calls do NOT qualify as an audit trail.
        (
            r"\b(?:logger|logging|log4j|slf4j|winston|bunyan|pino|tracing|opentelemetry)\b\s*[\.\(:]|@(?:audit_log|audit)\b|audit_log\s*\(|log_event\s*\(|emit_audit\s*\(",
            GuardrailKind::AuditTrail,
            "Audit logging detected",
        ),
    ];
    raw.iter()
        .map(|(p, kind, desc)| {
            (
                Regex::new(&format!("(?i){p}")).expect("guardrail pattern"),
                kind.clone(),
                *desc,
            )
        })
        .collect()
});

static DATA_ACCESS_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    let raw: &[(&str, &str)] = &[
        (
            r#"(?i)(?:postgres|mysql|sqlite|mongodb|redis|prisma|kysely|drizzle)\s*[(\.\{]"#,
            "Database",
        ),
        (
            r#"(?i)(?:s3|S3Client|bucket|BlobService|Storage\(\))"#,
            "Cloud Storage",
        ),
        (
            r#"(?i)(?:api_key|API_KEY|bearer|Authorization)\s*[=:]"#,
            "External API",
        ),
        (
            r#"(?i)(?:readFile|read_file|open\(\s*['"]|fs\.read)"#,
            "File System",
        ),
        (
            r#"(?i)(?:sendEmail|smtp|sendgrid|ses\.send)"#,
            "Email Service",
        ),
        (
            r#"(?i)(?:webhook|callback_url)\s*[=:]"#,
            "Webhook/Notification",
        ),
        (
            r#"(?i)(?:subprocess|os\.system|child_process|Command::new)"#,
            "System Command",
        ),
    ];
    raw.iter()
        .map(|(p, src)| (Regex::new(p).expect("data-access pattern"), *src))
        .collect()
});

static PERMISSION_PATTERNS: LazyLock<Vec<(Regex, &'static str, PermissionLevel)>> =
    LazyLock::new(|| {
        let raw: &[(&str, &str, PermissionLevel)] = &[
            (
                r"(?i)(?:subprocess|os\.system|child_process|exec\s*\(|shell\s*\(|Command::new)",
                "system_exec",
                PermissionLevel::Execute,
            ),
            (
                r"(?i)(?:\.insert|\.update|\.delete|\.put|\.create)\s*\(",
                "data_write",
                PermissionLevel::Write,
            ),
            (
                r"(?i)(?:admin|superuser|root|sudo|elevated)\s*[=:]",
                "admin",
                PermissionLevel::Admin,
            ),
            (
                r"(?i)(?:\.query|\.select|\.find|\.get)\s*\(",
                "data_read",
                PermissionLevel::Read,
            ),
        ];
        raw.iter()
            .map(|(p, scope, level)| {
                (
                    Regex::new(p).expect("permission pattern"),
                    *scope,
                    level.clone(),
                )
            })
            .collect()
    });

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredAgent {
    pub name: String,
    pub framework: AgentFramework,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub tools: Vec<ToolDefinition>,
    // Skip serialization to avoid leaking extracted prompt text (which may
    // contain secrets or PII from the scanned source) into JSON reports.
    // The signals layer reads `Option::is_some()`, not the contents.
    #[serde(skip)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum GuardrailKind {
    InputValidation,
    OutputFiltering,
    RateLimit,
    HumanApproval,
    ContentFilter,
    TokenLimit,
    TimeoutLimit,
    ScopeRestriction,
    AuditTrail,
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
    // Test trees: real-world calibration on langgraph showed ~37% of all
    // detections came from `tests/` (143 of 385). Test code routinely sets
    // up agent-shaped fixtures that aren't deployed agents — they're
    // exercising the framework's own surface, not running in production.
    // A repo's tests are inside its own trust boundary and don't add to
    // an auditor's blast radius.
    "tests",
    "test",
    "__tests__",
    "__testfixtures__",
    "__mocks__",
    "spec",
];

const SCAN_EXTENSIONS: &[&str] = &[
    "py", "js", "ts", "tsx", "jsx", "rs", "go", "java", "yaml", "yml", "json", "toml",
];

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("failed to walk directory: {0}")]
    Walk(#[from] walkdir::Error),
}

pub fn scan_directory(path: &Path) -> Result<Vec<DiscoveredAgent>, ScanError> {
    let mut agents = Vec::new();
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Dedupe by (file_path, framework_name) at push time. The previous
    // `Vec::dedup_by` only collapses *adjacent* duplicates and silently
    // missed cases where the same (file, framework) pair fired non-
    // contiguously through the walk.
    let mut emitted: HashSet<(PathBuf, &'static str)> = HashSet::new();
    let engine = Engine::compile_builtin();

    for entry in WalkDir::new(path)
        .follow_links(false)
        .sort_by_file_name()
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
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !SCAN_EXTENSIONS.contains(&ext) {
            continue;
        }

        let Some(lang) = lang_from_ext(ext) else {
            continue;
        };

        if seen_files.contains(file_path) {
            continue;
        }
        seen_files.insert(file_path.to_path_buf());

        // File-size guard. A pathological 1+ GB source file would otherwise
        // force read_to_string to allocate the full contents and run every
        // detection regex over it. Skip silently — unreadable metadata
        // means we can't make a safe decision either way.
        if let Ok(meta) = std::fs::metadata(file_path)
            && meta.len() > MAX_FILE_SIZE
        {
            continue;
        }

        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue, // skip binary/unreadable files
        };

        let ctx = FileCtx {
            path: file_path,
            lang,
            content: &content,
        };

        for rule in engine.detection_rules() {
            let hits = rule.matcher.matches_file(&ctx);
            if hits.len() >= rule.min_match_count as usize {
                let key = (file_path.to_path_buf(), rule.framework.name());
                if emitted.contains(&key) {
                    continue;
                }
                let raw_matches: Vec<(usize, String)> =
                    hits.into_iter().map(|h| (h.line, h.snippet)).collect();
                let agent =
                    extract_agent_details(file_path, &content, &rule.framework, &raw_matches);
                agents.push(agent);
                emitted.insert(key);
            }
        }
    }

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
        framework: *framework,
        file_path: file_path.to_path_buf(),
        line_number: first_match,
        tools,
        system_prompt,
        permissions,
        guardrails,
        data_access,
    }
}

fn extract_agent_name(content: &str, _framework: &AgentFramework) -> Option<String> {
    for re in AGENT_NAME_PATTERNS.iter() {
        if let Some(caps) = re.captures(content)
            && let Some(name) = caps.get(1)
        {
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

    None
}

fn extract_tools(content: &str, _framework: &AgentFramework) -> Vec<ToolDefinition> {
    let mut tools = Vec::new();

    // Hoist the per-tool confirmation check out of the per-match loop —
    // it's a property of the file, not of any specific tool.
    let has_confirmation = content.contains("confirm")
        || content.contains("approval")
        || content.contains("human_in_the_loop");

    for re in TOOL_PATTERNS.iter() {
        for caps in re.captures_iter(content) {
            if let Some(name) = caps.get(1) {
                let tool_name = name.as_str().to_string();
                if !tools.iter().any(|t: &ToolDefinition| t.name == tool_name) {
                    tools.push(ToolDefinition {
                        name: tool_name,
                        description: None,
                        has_confirmation,
                    });
                }
            }
        }
    }

    tools
}

fn extract_system_prompt(content: &str) -> Option<String> {
    for re in SYSTEM_PROMPT_PATTERNS.iter() {
        if let Some(caps) = re.captures(content)
            && let Some(prompt) = caps.get(1)
        {
            let text = prompt.as_str().to_string();
            // Truncate for storage
            if text.len() > 500 {
                return Some(format!("{}...", &text[..500]));
            }
            return Some(text);
        }
    }

    None
}

fn detect_guardrails(content: &str) -> Vec<Guardrail> {
    let mut guardrails = Vec::new();

    for (re, kind, description) in GUARDRAIL_CHECKS.iter() {
        if re.is_match(content) {
            guardrails.push(Guardrail {
                kind: kind.clone(),
                description: (*description).to_string(),
            });
        }
    }

    guardrails
}

fn detect_data_access(content: &str) -> Vec<DataAccess> {
    let mut access = Vec::new();

    // Hoist the read-vs-write classification out of the per-pattern loop —
    // it's a property of the file as a whole.
    let access_type = if content.contains("write")
        || content.contains("insert")
        || content.contains("update")
        || content.contains("delete")
        || content.contains("put")
        || content.contains("post")
    {
        "read/write"
    } else {
        "read"
    };

    for (re, source) in DATA_ACCESS_PATTERNS.iter() {
        if re.is_match(content) {
            access.push(DataAccess {
                source: (*source).to_string(),
                access_type: access_type.to_string(),
            });
        }
    }

    access
}

fn detect_permissions(content: &str) -> Vec<Permission> {
    let mut permissions = Vec::new();

    for (re, scope, level) in PERMISSION_PATTERNS.iter() {
        if re.is_match(content) {
            permissions.push(Permission {
                scope: (*scope).to_string(),
                level: level.clone(),
            });
        }
    }

    permissions
}
