# Round 1 — Rust Engine Boundary for YAML-Driven Detection

**Author:** Rust engine expert (Round 1, expert #1 of 4)
**Status:** Opinionated proposal, ready for Round 2 critique.

---

## TL;DR — Top 3 Design Decisions

1. **Embed a baseline ruleset at compile time, layer user/org rules on top from disk.** The binary always works offline with sensible defaults (no "first run downloads rules" cliff). Override order is `embedded → /etc/agent-shield/rules → ~/.agent-shield/rules → ./agent-shield/rules → --rules <path>`, last write wins by `rule.id`. This is the `clippy` + `cargo-deny` model, not the `tree-sitter` "everything is a separate crate" model. Rules ship *with* the binary by default; power users override.

2. **The engine speaks one trait, `Matcher`, and the YAML schema compiles down to a small fixed set of `Matcher` impls.** YAML is *data*, not a DSL with its own evaluator. A rule's `when:` clause deserializes into an enum of concrete matcher variants (`ImportMatch`, `RegexMatch`, `ManifestDep`, `ConfigFile`, `AstQuery`-future). The `Engine` is just `Vec<CompiledRule>` and a per-file fan-out. No interpreter, no scripting, no Lua. If you need Turing completeness, write a Rust matcher and expose a new YAML variant — that's a deliberate barrier.

3. **Compile regex once at load time; on failure, *quarantine the rule and keep scanning*.** A bad rule emits a structured diagnostic (rule id, file, line, regex error) and is excluded from the run. The scan never aborts because someone PR'd a bad backreference. This is the single most important reliability property — it's what makes "open the gates to YAML PRs" tractable instead of terrifying.

The rest of this doc is the receipts.

---

## Current state, briefly

`scanner.rs` and `frameworks.rs` today do three things that need to become one thing:

- `AgentFramework::detection_patterns()` returns a `Vec<DetectionPattern>` of `Import | PackageDep | ConfigFile | CodePattern`. This *already is* a small declarative DSL — it just happens to be expressed as Rust enum literals.
- `scanner::check_framework_patterns` recompiles regexes inside a hot loop, per file, per pattern. That's the first thing to fix regardless of YAML.
- `detect_guardrails`, `detect_data_access`, `detect_permissions` hard-code their own pattern arrays. These are *also* rules, just second-class ones. The refactor needs to unify them — otherwise we end up with "frameworks are YAML but guardrails are still Rust" which defeats the contributor pitch.

Good news: the existing `DetectionPattern` enum is 80% of the YAML schema already. We're formalizing a thing that exists, not inventing one.

---

## Trait & struct API surface

```rust
// crates/agent-shield-rules/src/lib.rs (new internal lib crate)

/// Schema version this binary understands. Bump on breaking change.
pub const RULE_SCHEMA_VERSION: u32 = 1;

/// A rule as loaded from YAML, validated, with regexes pre-compiled.
#[derive(Debug)]
pub struct CompiledRule {
    pub id: RuleId,                     // e.g. "framework.langchain"
    pub kind: RuleKind,                 // Framework | Guardrail | DataAccess | Permission
    pub schema_version: u32,
    pub source: RuleSource,             // Embedded | File(PathBuf)
    pub min_matches: u8,                // confidence gate
    pub matchers: Vec<Matcher>,         // OR'd within a rule
    pub metadata: RuleMetadata,         // name, baseline, references, etc.
}

#[derive(Debug)]
pub enum Matcher {
    Import   { needle: String, langs: LangSet },
    Manifest { dep: String, files: ManifestFileSet },
    Config   { filename: String },
    Regex    { pattern: CompiledRegex, scope: RegexScope }, // FileWide | PerLine
    // Future: AstQuery { lang: Lang, query: TreeSitterQuery },
}

#[derive(Debug)]
pub struct CompiledRegex {
    pub re: regex::Regex,
    pub source: String,        // keep original for diagnostics
}

pub trait RuleLoader {
    fn load(&self) -> Result<LoadReport, LoaderError>;
}

pub struct LoadReport {
    pub rules: Vec<CompiledRule>,
    pub diagnostics: Vec<RuleDiagnostic>,  // quarantined rules go here
}

pub struct Engine {
    rules: Vec<CompiledRule>,
    // Pre-bucketed for fast dispatch (see Performance section)
    by_extension: HashMap<&'static str, Vec<RuleIdx>>,
    manifest_rules: Vec<RuleIdx>,
    config_rules: Vec<RuleIdx>,
}

impl Engine {
    pub fn from_report(report: &LoadReport) -> Self { /* … */ }

    /// Scan a single file. Stateless, parallelizable.
    pub fn scan_file(&self, path: &Path, content: &str) -> Vec<RuleHit>;

    /// Scan manifests once at scan start.
    pub fn scan_manifests(&self, root: &Path) -> Vec<RuleHit>;
}

pub struct RuleHit {
    pub rule_id: RuleId,
    pub file: PathBuf,
    pub line: usize,
    pub snippet: String,
    pub matcher_idx: usize,  // which matcher in the rule fired
}
```

Two things to call out:

- **`Matcher` is an enum, not a trait object.** No `Box<dyn Matcher>`. We get exhaustiveness checking, no vtable cost, and a closed set of behaviors that the YAML schema can describe completely. If we ever need user-pluggable matchers (Lua, WASM), we add one variant — but I'd resist that hard. See "open question" at the end.
- **`RuleHit` is the only thing that crosses the engine boundary.** `scoring.rs` consumes hits, decides what becomes a `Finding`. Today scoring reaches into `DiscoveredAgent` fields directly; that needs to invert — scoring asks "did rule `guardrail.human-approval` fire?" rather than "is `guardrails` Vec non-empty?".

---

## Loading, layering, validation

```text
embedded (compile-time include_dir!) ──┐
/etc/agent-shield/rules/ ──────────────┤
~/.agent-shield/rules/ ────────────────┤──► merge by rule.id, last write wins
./.agent-shield/rules/  ───────────────┤    (with --no-builtin to skip embedded)
--rules <path>          ───────────────┘
```

Use the `include_dir` crate to bake `rules/builtin/**/*.yaml` into the binary at build time. This means `cargo install agent-shield && agent-shield scan .` Just Works with zero network and zero `~/.agent-shield`. Override layers are read at startup, not lazily — startup cost is bounded and predictable.

**Validation pipeline per file:**

1. Parse YAML → `RuleFile` (serde). Hard error → diagnostic, skip file.
2. Schema-version check. If `rule.schema_version > RULE_SCHEMA_VERSION` and not in a `compat:` window → diagnostic, skip rule. If `< RULE_SCHEMA_VERSION` and we have a migration → upgrade in memory.
3. Compile each regex via `regex::Regex::new`. Per-rule failure → diagnostic, skip *just that matcher*; if all matchers fail, skip the rule.
4. Push into `LoadReport`.

`agent-shield rules check` runs steps 1–3 against a directory and exits non-zero on diagnostics — that's the CI hook for the rules repo (DX-optimizer's lane, but the engine has to expose it).

---

## Performance: 30-second scan with hundreds of rules

Today's hot path is O(files × frameworks × patterns × lines) with regex re-compilation inside the inner loops. That's the bug. Fixes, in order of impact:

1. **Compile-once.** Already covered above. This alone is probably 5–10× on large repos.
2. **Pre-bucket rules by file extension.** When we see `foo.py`, we only consider matchers whose `langs:` includes `python`. The `Engine.by_extension` map does this at load time, not per file.
3. **Prefilter with `aho-corasick` for literal substrings.** Most `Import` and `Manifest` matchers are literal needle searches; a single multi-pattern Aho-Corasick automaton over the whole file content gives us a "candidate rules" set in one linear pass before we touch any regex. The `regex` crate uses A-C internally for literal prefixes anyway, but unifying it across rules avoids N regex evaluations on files that match nothing.
4. **Parallelize per-file with `rayon`.** `Engine::scan_file` is already designed stateless. Add `rayon` to deps, change the walkdir loop to `par_bridge().for_each(...)`. The scoring stage stays serial — it's tiny.
5. **Cap file size at 2 MiB for regex matching** (configurable). Most agent-defining files are small; minified bundles destroy regex performance. Skip-with-diagnostic, don't silently drop.
6. **Avoid `Regex::is_match` *and* `captures` on the same content.** Today `extract_*` functions compile a fresh regex and re-walk content. Move all extraction into the same pass as detection — one regex set per file, one walk.

I'd target: 1000-file repo, 200 rules → under 5 seconds on an M-series laptop. The 30-second budget is generous; we should beat it by an order of magnitude and bank the headroom for AST queries later.

---

## Versioning

YAML rules carry a `schema_version: 1` field. The binary declares `RULE_SCHEMA_VERSION` and a `min_supported: 1` constant.

- **Forward compat:** rules with `schema_version > supported_max` are quarantined with a "binary too old, run `agent-shield self-update`" diagnostic. Never silently ignore — security tooling that silently skips rules is a vulnerability.
- **Backward compat:** for one major version, keep an in-memory migration shim (e.g. v1 → v2 renames a field). Past one version: hard error, force the rules repo to maintain a v2 branch.
- **Binary version ↔ rule version is *not* coupled.** `agent-shield 1.4.0` may understand schema v1 and v2. The CLI flag `--rule-schema 1` lets a CI pipeline pin behavior.

Print both at scan start: `agent-shield 0.2.0 (rule schema v1, 247 rules loaded, 2 quarantined)`.

---

## Error handling philosophy

One bad rule never breaks the scan. Three failure modes, all non-fatal:

| Failure                              | Behavior                                                |
| ------------------------------------ | ------------------------------------------------------- |
| YAML parse error                     | Quarantine rule, diagnostic with file + line            |
| Regex compile error                  | Quarantine matcher; rule survives if other matchers OK  |
| Rule references unknown matcher kind | Quarantine rule, diagnostic naming the unknown kind     |
| Schema version too new               | Quarantine rule, diagnostic suggesting binary upgrade   |
| File read error during scan          | Skip file, diagnostic only at `--verbose`               |

All diagnostics surface in the JSON output under `meta.rule_diagnostics` so dashboards (the SaaS play) can surface "your fleet has 3 broken rules." That's actually a *feature* worth charging for.

---

## Phasing: minimum viable refactor vs. ideal end state

**MVP (one-week refactor, ship as 0.2.0):**

- Introduce `agent-shield-rules` internal crate. Define `CompiledRule`, `Matcher` enum, `Engine`.
- Keep current `AgentFramework` enum *as-is*, but generate `CompiledRule` instances from it via a `From` impl. **No YAML yet.**
- Move regex compilation out of hot loops into rule loading.
- Add `aho-corasick` prefilter and `rayon` parallelism.
- This is pure refactor — outputs are byte-identical, but the internal seam exists.

**Phase 2 (the actual story, 0.3.0):**

- Add YAML loader. Bake current rules into `rules/builtin/*.yaml` via `include_dir!`.
- Delete the hardcoded enum branches; `AgentFramework` becomes a thin `&str` newtype that just refers to a rule id.
- Ship `agent-shield rules list / check / explain <id>`.
- Open the `rules/` directory in the repo to PRs. This is the announceable moment.

**Phase 3 (the moat, 0.4.0+):**

- AST matchers via tree-sitter for Python/TS/Rust. Add `AstQuery` matcher variant.
- Rule signing (backend-architect's lane, but engine needs to verify).
- Per-rule benchmarking harness (`criterion`) so PRs that regress scan time get caught.

The MVP-to-Phase-2 ordering matters. Refactoring *and* introducing YAML at once means we can't tell whether a behavior change is a refactor bug or a YAML semantics bug. Two PRs, two release notes.

---

## Biggest open question I'm punting to Round 2

**Should the matcher set be fixed-and-closed, or pluggable via WASM/dyn-Lib?**

I argued above for closed: a finite enum, new matcher kinds require a Rust PR. The cost is that some clever detection ideas (e.g. "follow the import graph and check what `agent.run()` actually calls") will hit the wall and require core changes.

The alternative — WASM-loaded custom matchers — opens the door to arbitrary contributor logic without our review, which is a *security-tool-distributing-untrusted-code* problem of exactly the kind agent-shield itself exists to flag. The irony writes itself.

My current vote: closed enum, forever. But I want to hear the dx-optimizer's lane on this before locking it in — if the contributor experience for "I want to add a slightly novel detection idea" is "wait three months for a Rust release," we'll bleed contributors to forks. There may be a middle ground (e.g. a constrained `expression:` mini-language with explicit AST nodes, à la Semgrep's pattern syntax) that I haven't designed here. That's Round 2's fight.
