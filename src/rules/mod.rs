//! YAML rule loading.
//!
//! Week 2 of the Path B refactor moves detection and scoring rules from
//! compile-time Rust (`engine::builtin_matchers_for` and the inline blocks
//! in `scoring.rs`) into YAML files bundled via `include_str!()`. This
//! module hosts:
//!
//! - `types`: the serde shape of a rule on disk (what we deserialize from
//!   YAML before validating + translating to `engine::CompiledRule`).
//! - `loader` (added in W2-C4): validation, quarantine-on-error, and the
//!   translation step.
//!
//! Schema is locked at `1.0`. Anything beyond — overlay support via
//! `extends:`, new context signals, AST-based detection — is reserved for
//! v1.1 and the loader rejects it explicitly. See
//! `docs/rules-design/round3-synthesis.md` §1.3, §1.5.

pub mod types;
