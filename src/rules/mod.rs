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

pub mod loader;
pub mod types;

#[cfg(test)]
mod builtin_yaml_tests {
    use super::types::ParsedRule;

    /// Round-trip every bundled detection YAML through serde to catch
    /// schema drift before W2-C4 wires the loader up. The contents must
    /// stay in lockstep with `engine::builtin_matchers_for`; this test
    /// only proves the files parse cleanly into `ParsedRule`. Semantic
    /// equivalence is asserted in C4.
    macro_rules! roundtrip_builtin {
        ($name:ident, $path:literal) => {
            #[test]
            fn $name() {
                let yaml = include_str!(concat!("../../rules/builtin/", $path));
                let rule: ParsedRule = serde_yaml::from_str(yaml)
                    .unwrap_or_else(|e| panic!("rules/builtin/{} failed to parse: {}", $path, e));
                assert_eq!(rule.schema_version, "1.0");
                assert_eq!(rule.category, "detection");
                assert!(rule.framework.is_some(), "detection rule must declare framework");
                assert!(rule.extends.is_none(), "extends must be null in v1.0");
                assert_eq!(
                    rule.when.populated_slot_count(),
                    1,
                    "exactly one matcher slot must be set"
                );
            }
        };
    }

    roundtrip_builtin!(roundtrip_langchain, "langchain.yaml");
    roundtrip_builtin!(roundtrip_langgraph, "langgraph.yaml");
    roundtrip_builtin!(roundtrip_crewai, "crewai.yaml");
    roundtrip_builtin!(roundtrip_autogen, "autogen.yaml");
    roundtrip_builtin!(roundtrip_openai_assistants, "openai-assistants.yaml");
    roundtrip_builtin!(roundtrip_anthropic_mcp, "anthropic-mcp.yaml");
    roundtrip_builtin!(roundtrip_anthropic_agent_sdk, "anthropic-agent-sdk.yaml");
    roundtrip_builtin!(roundtrip_aws_bedrock, "aws-bedrock.yaml");
    roundtrip_builtin!(roundtrip_vercel_ai, "vercel-ai.yaml");
    roundtrip_builtin!(roundtrip_custom_agent, "custom-agent.yaml");
}
