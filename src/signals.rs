//! Context signal extraction.
//!
//! W2-C2 of the Path B refactor moves the v1 fact set out of
//! `score_single_agent` so that scoring rules (W2-C8) can read pre-computed
//! signals instead of re-deriving them from the `DiscoveredAgent`. The
//! signal data shapes (`ContextSignals`, `GuardrailFlags`, `PermissionFlags`)
//! are owned here; `engine::matcher` imports them for the
//! `Matcher::matches_signals` evaluator added in W2-C1.
//!
//! The set is fixed by round3-synthesis §1.2: six canonical signals
//! (`tool_count`, `has_system_prompt`, `autonomy_tier`, `has_guardrail`,
//! `has_permission`, `data_source_count`). Two extra fields
//! (`unconfirmed_tool_count`, `has_audit_trail`) are computed because
//! today's hardcoded findings reference them; they're not in the v1 YAML
//! allowlist for now and the loader will reject rules that try to query
//! them. (Promotion to v1.1 with an RFC if scoring rules need them.)

use crate::scanner::{DiscoveredAgent, GuardrailKind, PermissionLevel};

/// Pre-computed v1 fact set for a single `DiscoveredAgent`. Computed once
/// at the top of `score_single_agent`; consumed by hardcoded findings (today)
/// and by `Matcher::matches_signals` (after W2-C8).
#[derive(Debug, Clone, Default)]
pub struct ContextSignals {
    pub tool_count: usize,
    pub has_system_prompt: bool,
    pub autonomy_tier: u8,
    pub data_source_count: usize,
    pub unconfirmed_tool_count: usize,
    pub has_audit_trail: bool,
    pub guardrails: GuardrailFlags,
    pub permissions: PermissionFlags,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GuardrailFlags {
    pub input_validation: bool,
    pub output_filtering: bool,
    pub rate_limit: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PermissionFlags {
    pub execute: bool,
    pub admin: bool,
    pub write: bool,
}

/// Build a `ContextSignals` snapshot from an agent. Pure function — no
/// I/O, no side effects, no mutation of the agent. Output is byte-stable
/// for a given input, which is what makes this safe to call once and reuse
/// for both score adjustments and finding generation.
pub fn compute_all_signals(agent: &DiscoveredAgent) -> ContextSignals {
    let unconfirmed_tool_count = agent.tools.iter().filter(|t| !t.has_confirmation).count();

    let guardrails = GuardrailFlags {
        input_validation: agent
            .guardrails
            .iter()
            .any(|g| matches!(g.kind, GuardrailKind::InputValidation)),
        output_filtering: agent
            .guardrails
            .iter()
            .any(|g| matches!(g.kind, GuardrailKind::OutputFiltering)),
        rate_limit: agent
            .guardrails
            .iter()
            .any(|g| matches!(g.kind, GuardrailKind::RateLimit)),
    };

    let permissions = PermissionFlags {
        execute: agent
            .permissions
            .iter()
            .any(|p| matches!(p.level, PermissionLevel::Execute)),
        admin: agent
            .permissions
            .iter()
            .any(|p| matches!(p.level, PermissionLevel::Admin)),
        write: agent
            .permissions
            .iter()
            .any(|p| matches!(p.level, PermissionLevel::Write)),
    };

    // The previous implementation substring-matched the guardrail
    // description text for "log"/"audit", which never matched any
    // canned description string and silently caused the
    // missing-audit-trail rule to fire on every agent. Match the
    // structured GuardrailKind directly — the only correct shape.
    let has_audit_trail = agent
        .guardrails
        .iter()
        .any(|g| g.kind == GuardrailKind::AuditTrail);

    ContextSignals {
        tool_count: agent.tools.len(),
        has_system_prompt: agent.system_prompt.is_some(),
        autonomy_tier: assess_autonomy_tier(agent),
        data_source_count: agent.data_access.len(),
        unconfirmed_tool_count,
        has_audit_trail,
        guardrails,
        permissions,
    }
}

/// NIST 4-tier autonomy assessment. Moved from `scoring.rs` in W2-C2.
///
/// Tier 1: fully supervised (human approval + scope restriction + ≤3 tools)
/// Tier 2: constrained autonomy (scope restriction, no exec)
/// Tier 3: broad autonomy with monitoring (no exec, <10 tools)
/// Tier 4: full autonomy (everything else)
fn assess_autonomy_tier(agent: &DiscoveredAgent) -> u8 {
    let has_human_approval = agent
        .guardrails
        .iter()
        .any(|g| matches!(g.kind, GuardrailKind::HumanApproval));
    let has_scope_restriction = agent
        .guardrails
        .iter()
        .any(|g| matches!(g.kind, GuardrailKind::ScopeRestriction));
    let has_exec = agent
        .permissions
        .iter()
        .any(|p| matches!(p.level, PermissionLevel::Execute));
    let tool_count = agent.tools.len();

    if has_human_approval && has_scope_restriction && tool_count <= 3 {
        1
    } else if has_scope_restriction && !has_exec {
        2
    } else if !has_exec && tool_count < 10 {
        3
    } else {
        4
    }
}
