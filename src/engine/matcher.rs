//! Closed `Matcher` enum, supporting types, and evaluation.
//!
//! The `Matcher` enum is **closed forever** — adding a new variant requires
//! a Rust PR and a binary release. This is the explicit Week-1 design call
//! (see `docs/rules-design/round3-synthesis.md` §1.2). Community contributors
//! compose existing variants in YAML; new primitives are out of scope for
//! community contribution.
//!
//! Two-pass model: the file pass walks every source file in the repo and
//! evaluates `matches_file`. The manifest pass walks `package.json`,
//! `pyproject.toml`, etc. once at the repo root and evaluates `matches_repo`.
//! Variants that don't apply to a pass simply return an empty `Vec`.

use regex::Regex;
use std::path::{Path, PathBuf};

/// Source language a file is written in.
///
/// Used to scope matchers via `LangSet::Only(...)`. Detected from file
/// extension by `lang_from_ext` in `scanner.rs` (added in C4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    Yaml,
    Json,
    Toml,
}

/// Restricts a matcher to a specific set of languages.
///
/// `Any` is the default for v1.0 detection rules translated from the existing
/// `frameworks.rs` enum; per-language scoping arrives with the YAML schema in
/// Week 2.
#[derive(Debug, Clone)]
pub enum LangSet {
    Any,
    Only(Vec<Lang>),
}

impl LangSet {
    pub fn allows(&self, lang: Lang) -> bool {
        match self {
            LangSet::Any => true,
            LangSet::Only(allowed) => allowed.contains(&lang),
        }
    }
}

/// A single match against a source file or manifest.
#[derive(Debug, Clone)]
pub struct MatchHit {
    /// 1-based line number, or 0 for repo-level matches (no line context).
    pub line: usize,
    /// Trimmed snippet of the matched line, or a synthetic descriptor for
    /// repo-level matches (e.g. `"package.json depends on \"langchain\""`).
    pub snippet: String,
}

/// File-level evaluation context.
pub struct FileCtx<'a> {
    pub path: &'a Path,
    pub lang: Lang,
    pub content: &'a str,
}

/// Repo-level evaluation context — used by manifest and file-presence matchers.
pub struct RepoCtx<'a> {
    pub root: &'a Path,
    /// Concatenated content of every manifest discovered at the repo root
    /// (`package.json`, `pyproject.toml`, `requirements.txt`, etc.).
    /// Mirrors the existing `detect_frameworks_from_manifests` logic.
    pub manifest_text: &'a str,
}

// `ContextSignals`, `GuardrailFlags`, and `PermissionFlags` were defined
// here in W2-C1 and moved to `crate::signals` in W2-C2. The matcher
// imports them for the `matches_signals` evaluator below.
pub use crate::signals::ContextSignals;

/// Comparison operator for a `Matcher::ContextSignal`. `Gt`/`Gte`/`Lt`/`Lte`
/// are valid only for integer signal values; the loader rejects ordering
/// ops on bool/string values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

/// Literal value a context signal is compared against.
///
/// Variants match the YAML side (`bool`/`integer`/`string`) — the loader
/// validates that the literal type makes sense for the signal name (e.g.
/// `has_system_prompt` → `Bool`, `tool_count` → `Int`).
#[derive(Debug, Clone, PartialEq)]
pub enum SignalValue {
    Bool(bool),
    Int(i64),
    Str(String),
}

/// The closed set of detection primitives.
///
/// Three text-level primitives operate on a single file's content; two
/// repo-level primitives operate on manifests or filesystem layout; three
/// combinators compose any subset.
///
/// Mixing file-level and repo-level primitives inside the same combinator
/// is undefined for v1.0 — translations from `frameworks.rs::detection_patterns()`
/// never produce such combinations. The YAML loader in Week 2 may forbid it
/// at schema-validation time.
#[derive(Debug, Clone)]
pub enum Matcher {
    /// Substring match on a line that also contains an import-like keyword
    /// (`import`, `require`, `use `). Mirrors the current `Import` detection
    /// behavior in `scanner.rs::check_framework_patterns`.
    ImportContains {
        needle: String,
        languages: LangSet,
    },
    /// Regex match against each line of the file.
    CodeRegex {
        pattern: Regex,
        languages: LangSet,
    },
    /// Regex match against the entire file content (cross-line). Reserved
    /// for v1.0 schema completeness; no `frameworks.rs` pattern translates
    /// to this variant in C3.
    MultilineRegex {
        pattern: Regex,
        languages: LangSet,
    },
    /// Substring match against the concatenated manifest text at the repo
    /// root. Mirrors `detect_frameworks_from_manifests`.
    PackageDep { name: String },
    /// File or relative path exists somewhere under the repo root. Today
    /// only used for `mcp.json` / `.mcp.json` in Anthropic MCP detection.
    FilePresent { path: String },
    /// All sub-matchers must produce at least one hit. Hits are concatenated.
    AllOf(Vec<Matcher>),
    /// Any sub-matcher producing a hit yields a hit. First-match wins for
    /// the returned `MatchHit` set (all matching sub-matchers contribute).
    AnyOf(Vec<Matcher>),
    /// Inverts the underlying matcher: empty hits → one synthetic hit;
    /// non-empty hits → empty. Used by behavioral rules like
    /// "multi-agent without human approval guardrail."
    Not(Box<Matcher>),
    /// Compares a v1.0 context signal to a literal value. Evaluated by
    /// `matches_signals`; file/repo passes return empty for this variant.
    /// `param` is required for `has_guardrail` / `has_permission` and
    /// ignored for the others (the loader enforces this).
    ContextSignal {
        name: String,
        param: Option<String>,
        op: SignalOp,
        value: SignalValue,
    },
}

impl Matcher {
    /// Evaluate against a single source file. Repo-level variants
    /// (`PackageDep`, `FilePresent`) return an empty `Vec`.
    pub fn matches_file(&self, ctx: &FileCtx) -> Vec<MatchHit> {
        match self {
            Matcher::ImportContains { needle, languages } => {
                if !languages.allows(ctx.lang) {
                    return Vec::new();
                }
                let mut hits = Vec::new();
                for (i, line) in ctx.content.lines().enumerate() {
                    if line.contains(needle.as_str())
                        && (line.contains("import")
                            || line.contains("require")
                            || line.contains("use "))
                    {
                        hits.push(MatchHit {
                            line: i + 1,
                            snippet: line.trim().to_string(),
                        });
                    }
                }
                hits
            }
            Matcher::CodeRegex { pattern, languages } => {
                if !languages.allows(ctx.lang) {
                    return Vec::new();
                }
                let mut hits = Vec::new();
                for (i, line) in ctx.content.lines().enumerate() {
                    if pattern.is_match(line) {
                        hits.push(MatchHit {
                            line: i + 1,
                            snippet: line.trim().to_string(),
                        });
                    }
                }
                hits
            }
            Matcher::MultilineRegex { pattern, languages } => {
                if !languages.allows(ctx.lang) {
                    return Vec::new();
                }
                if pattern.is_match(ctx.content) {
                    vec![MatchHit {
                        line: 0,
                        snippet: format!("multiline match: /{}/", pattern.as_str()),
                    }]
                } else {
                    Vec::new()
                }
            }
            Matcher::PackageDep { .. }
            | Matcher::FilePresent { .. }
            | Matcher::ContextSignal { .. } => Vec::new(),
            Matcher::AllOf(children) => {
                let per_child: Vec<Vec<MatchHit>> =
                    children.iter().map(|c| c.matches_file(ctx)).collect();
                if per_child.iter().all(|hits| !hits.is_empty()) {
                    per_child.into_iter().flatten().collect()
                } else {
                    Vec::new()
                }
            }
            Matcher::AnyOf(children) => children
                .iter()
                .flat_map(|c| c.matches_file(ctx))
                .collect(),
            Matcher::Not(inner) => {
                if inner.matches_file(ctx).is_empty() {
                    vec![MatchHit {
                        line: 0,
                        snippet: "negated condition holds".to_string(),
                    }]
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Evaluate against the repo as a whole. File-level variants
    /// (`ImportContains`, `CodeRegex`, `MultilineRegex`) return an empty `Vec`.
    pub fn matches_repo(&self, ctx: &RepoCtx) -> Vec<MatchHit> {
        match self {
            Matcher::PackageDep { name } => {
                if ctx.manifest_text.contains(name.as_str()) {
                    vec![MatchHit {
                        line: 0,
                        snippet: format!("manifest depends on \"{}\"", name),
                    }]
                } else {
                    Vec::new()
                }
            }
            Matcher::FilePresent { path } => {
                let candidate: PathBuf = ctx.root.join(path);
                if candidate.exists() {
                    vec![MatchHit {
                        line: 0,
                        snippet: format!("config file present: {}", path),
                    }]
                } else {
                    Vec::new()
                }
            }
            Matcher::ImportContains { .. }
            | Matcher::CodeRegex { .. }
            | Matcher::MultilineRegex { .. }
            | Matcher::ContextSignal { .. } => Vec::new(),
            Matcher::AllOf(children) => {
                let per_child: Vec<Vec<MatchHit>> =
                    children.iter().map(|c| c.matches_repo(ctx)).collect();
                if per_child.iter().all(|hits| !hits.is_empty()) {
                    per_child.into_iter().flatten().collect()
                } else {
                    Vec::new()
                }
            }
            Matcher::AnyOf(children) => children
                .iter()
                .flat_map(|c| c.matches_repo(ctx))
                .collect(),
            Matcher::Not(inner) => {
                if inner.matches_repo(ctx).is_empty() {
                    vec![MatchHit {
                        line: 0,
                        snippet: "negated repo condition holds".to_string(),
                    }]
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Evaluate against a `ContextSignals` snapshot. Used by scoring rules
    /// (Tier-2) — file/repo primitives return empty here, so a
    /// `ContextSignal` matcher inside an `AllOf` with a `CodeRegex` will
    /// silently never fire (the loader rejects this kind of cross-pass
    /// composition for scoring rules).
    pub fn matches_signals(&self, signals: &ContextSignals) -> Vec<MatchHit> {
        match self {
            Matcher::ContextSignal {
                name,
                param,
                op,
                value,
            } => evaluate_context_signal(name, param.as_deref(), *op, value, signals),
            Matcher::ImportContains { .. }
            | Matcher::CodeRegex { .. }
            | Matcher::MultilineRegex { .. }
            | Matcher::PackageDep { .. }
            | Matcher::FilePresent { .. } => Vec::new(),
            Matcher::AllOf(children) => {
                let per_child: Vec<Vec<MatchHit>> =
                    children.iter().map(|c| c.matches_signals(signals)).collect();
                if per_child.iter().all(|hits| !hits.is_empty()) {
                    per_child.into_iter().flatten().collect()
                } else {
                    Vec::new()
                }
            }
            Matcher::AnyOf(children) => children
                .iter()
                .flat_map(|c| c.matches_signals(signals))
                .collect(),
            Matcher::Not(inner) => {
                if inner.matches_signals(signals).is_empty() {
                    vec![MatchHit {
                        line: 0,
                        snippet: "negated signal condition holds".to_string(),
                    }]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

/// Resolve a (signal_name, param, op, value) tuple against the current
/// `ContextSignals`. Returns one synthetic `MatchHit` if the comparison
/// holds, empty otherwise. Unknown signal names return empty (defensive
/// — the loader pre-validates against the v1 allowlist, but we don't trust
/// it inside the matcher).
fn evaluate_context_signal(
    name: &str,
    param: Option<&str>,
    op: SignalOp,
    value: &SignalValue,
    signals: &ContextSignals,
) -> Vec<MatchHit> {
    let result = match name {
        "tool_count" => compare_int(signals.tool_count as i64, op, value),
        "autonomy_tier" => compare_int(signals.autonomy_tier as i64, op, value),
        "data_source_count" => compare_int(signals.data_source_count as i64, op, value),
        "unconfirmed_tool_count" => compare_int(signals.unconfirmed_tool_count as i64, op, value),
        "has_system_prompt" => compare_bool(signals.has_system_prompt, op, value),
        "has_audit_trail" => compare_bool(signals.has_audit_trail, op, value),
        "has_guardrail" => match param {
            Some("input_validation") => compare_bool(signals.guardrails.input_validation, op, value),
            Some("output_filtering") => compare_bool(signals.guardrails.output_filtering, op, value),
            Some("rate_limit") => compare_bool(signals.guardrails.rate_limit, op, value),
            _ => false,
        },
        "has_permission" => match param {
            Some("execute") => compare_bool(signals.permissions.execute, op, value),
            Some("admin") => compare_bool(signals.permissions.admin, op, value),
            Some("write") => compare_bool(signals.permissions.write, op, value),
            _ => false,
        },
        _ => false,
    };
    if result {
        vec![MatchHit {
            line: 0,
            snippet: format!(
                "signal {}{} {:?} {:?}",
                name,
                param.map(|p| format!("[{}]", p)).unwrap_or_default(),
                op,
                value
            ),
        }]
    } else {
        Vec::new()
    }
}

fn compare_int(actual: i64, op: SignalOp, expected: &SignalValue) -> bool {
    let SignalValue::Int(rhs) = expected else {
        return false;
    };
    match op {
        SignalOp::Eq => actual == *rhs,
        SignalOp::Ne => actual != *rhs,
        SignalOp::Gt => actual > *rhs,
        SignalOp::Gte => actual >= *rhs,
        SignalOp::Lt => actual < *rhs,
        SignalOp::Lte => actual <= *rhs,
    }
}

fn compare_bool(actual: bool, op: SignalOp, expected: &SignalValue) -> bool {
    let SignalValue::Bool(rhs) = expected else {
        return false;
    };
    match op {
        SignalOp::Eq => actual == *rhs,
        SignalOp::Ne => actual != *rhs,
        // Ordering ops on bool are loader-rejected; defensive false here.
        SignalOp::Gt | SignalOp::Gte | SignalOp::Lt | SignalOp::Lte => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals_with_tools(n: usize) -> ContextSignals {
        ContextSignals {
            tool_count: n,
            ..Default::default()
        }
    }

    fn cs(name: &str, op: SignalOp, value: SignalValue) -> Matcher {
        Matcher::ContextSignal {
            name: name.to_string(),
            param: None,
            op,
            value,
        }
    }

    fn cs_param(name: &str, param: &str, op: SignalOp, value: SignalValue) -> Matcher {
        Matcher::ContextSignal {
            name: name.to_string(),
            param: Some(param.to_string()),
            op,
            value,
        }
    }

    #[test]
    fn integer_signal_eq_ne_ordering() {
        let signals = signals_with_tools(11);
        // 11 != 10
        assert!(cs("tool_count", SignalOp::Eq, SignalValue::Int(10))
            .matches_signals(&signals)
            .is_empty());
        // 11 > 10
        assert!(!cs("tool_count", SignalOp::Gt, SignalValue::Int(10))
            .matches_signals(&signals)
            .is_empty());
        // 11 >= 11
        assert!(!cs("tool_count", SignalOp::Gte, SignalValue::Int(11))
            .matches_signals(&signals)
            .is_empty());
        // 11 not < 11
        assert!(cs("tool_count", SignalOp::Lt, SignalValue::Int(11))
            .matches_signals(&signals)
            .is_empty());
        // 11 != 10
        assert!(!cs("tool_count", SignalOp::Ne, SignalValue::Int(10))
            .matches_signals(&signals)
            .is_empty());
    }

    #[test]
    fn bool_signal_eq_ne() {
        let signals = ContextSignals {
            has_system_prompt: false,
            ..Default::default()
        };
        assert!(!cs("has_system_prompt", SignalOp::Eq, SignalValue::Bool(false))
            .matches_signals(&signals)
            .is_empty());
        assert!(cs("has_system_prompt", SignalOp::Eq, SignalValue::Bool(true))
            .matches_signals(&signals)
            .is_empty());
    }

    #[test]
    fn bool_signal_rejects_ordering_ops() {
        let signals = ContextSignals {
            has_system_prompt: true,
            ..Default::default()
        };
        // Ordering on bool returns empty (defensive — loader rejects this).
        assert!(cs("has_system_prompt", SignalOp::Gt, SignalValue::Bool(false))
            .matches_signals(&signals)
            .is_empty());
    }

    #[test]
    fn parametrized_guardrail_signal() {
        let signals = ContextSignals {
            guardrails: GuardrailFlags {
                input_validation: true,
                output_filtering: false,
                rate_limit: false,
            },
            ..Default::default()
        };
        assert!(!cs_param(
            "has_guardrail",
            "input_validation",
            SignalOp::Eq,
            SignalValue::Bool(true)
        )
        .matches_signals(&signals)
        .is_empty());
        assert!(!cs_param(
            "has_guardrail",
            "output_filtering",
            SignalOp::Eq,
            SignalValue::Bool(false)
        )
        .matches_signals(&signals)
        .is_empty());
    }

    #[test]
    fn parametrized_permission_signal() {
        let signals = ContextSignals {
            permissions: PermissionFlags {
                execute: true,
                admin: false,
                write: true,
            },
            ..Default::default()
        };
        assert!(!cs_param(
            "has_permission",
            "execute",
            SignalOp::Eq,
            SignalValue::Bool(true)
        )
        .matches_signals(&signals)
        .is_empty());
        assert!(cs_param(
            "has_permission",
            "admin",
            SignalOp::Eq,
            SignalValue::Bool(true)
        )
        .matches_signals(&signals)
        .is_empty());
    }

    #[test]
    fn unknown_signal_name_is_empty_defensively() {
        let signals = ContextSignals::default();
        assert!(cs("not_a_real_signal", SignalOp::Eq, SignalValue::Bool(true))
            .matches_signals(&signals)
            .is_empty());
    }

    #[test]
    fn missing_param_for_parametrized_signal_is_empty() {
        let signals = ContextSignals {
            guardrails: GuardrailFlags {
                input_validation: true,
                ..Default::default()
            },
            ..Default::default()
        };
        // No param provided — defensive empty (loader rejects this shape).
        assert!(cs("has_guardrail", SignalOp::Eq, SignalValue::Bool(true))
            .matches_signals(&signals)
            .is_empty());
    }

    #[test]
    fn combinators_compose_over_signals() {
        let signals = ContextSignals {
            tool_count: 12,
            has_system_prompt: false,
            ..Default::default()
        };
        let any = Matcher::AnyOf(vec![
            cs("tool_count", SignalOp::Lt, SignalValue::Int(5)),
            cs("has_system_prompt", SignalOp::Eq, SignalValue::Bool(false)),
        ]);
        assert!(!any.matches_signals(&signals).is_empty());

        let all = Matcher::AllOf(vec![
            cs("tool_count", SignalOp::Gt, SignalValue::Int(10)),
            cs("has_system_prompt", SignalOp::Eq, SignalValue::Bool(false)),
        ]);
        assert!(!all.matches_signals(&signals).is_empty());

        let not = Matcher::Not(Box::new(cs(
            "tool_count",
            SignalOp::Lt,
            SignalValue::Int(5),
        )));
        assert!(!not.matches_signals(&signals).is_empty());
    }

    #[test]
    fn file_and_repo_passes_skip_context_signal() {
        // Defensive: ContextSignal must return empty from file/repo passes
        // even if it's nested inside a combinator.
        let m = cs("tool_count", SignalOp::Gt, SignalValue::Int(0));
        let ctx = FileCtx {
            path: std::path::Path::new("x.rs"),
            lang: Lang::Rust,
            content: "irrelevant",
        };
        assert!(m.matches_file(&ctx).is_empty());

        let repo_ctx = RepoCtx {
            root: std::path::Path::new("."),
            manifest_text: "",
        };
        assert!(m.matches_repo(&repo_ctx).is_empty());
    }
}
