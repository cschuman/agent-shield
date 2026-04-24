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
            Matcher::PackageDep { .. } | Matcher::FilePresent { .. } => Vec::new(),
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
            | Matcher::MultilineRegex { .. } => Vec::new(),
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
}
