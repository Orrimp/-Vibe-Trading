//! T1908 acceptance — `CachedSystemPrompt` builder integration test.
//!
//! Verifies the four T1908 acceptance criteria from
//! `spec/v2-llm-strategy/tasks.md`:
//!
//! - (a) Anthropic build emits exactly 2 `Cached` markers.
//! - (b) OpenAI / Ollama builds emit zero markers.
//! - (c) byte-stability proptest over 1000 random inputs returns
//!   identical `Vec<SystemBlock>` bytes across two calls.
//! - (d) same project_ctx + role_ctx → consistent cache-key-relevant
//!   content across Anthropic and OpenAI flatten paths.

use llm::{CachedSystemPrompt, ProviderKind, SystemBlock};

#[test]
fn t1908_a_anthropic_emits_two_cached_markers() {
    let blocks = CachedSystemPrompt::builder()
        .project("project context")
        .role("role context")
        .dynamic("dynamic context")
        .build_for(&ProviderKind::Anthropic);
    let cached: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, SystemBlock::Cached(_, _)))
        .collect();
    assert_eq!(
        cached.len(),
        2,
        "Anthropic build must emit exactly 2 Cached markers, got {} blocks: {blocks:?}",
        cached.len()
    );
}

#[test]
fn t1908_b_openai_and_ollama_emit_zero_markers() {
    for provider in [
        ProviderKind::OpenAi,
        ProviderKind::OpenRouter,
        ProviderKind::DeepSeek,
        ProviderKind::Other("ollama".to_string()),
    ] {
        let blocks = CachedSystemPrompt::builder()
            .project("p")
            .role("r")
            .dynamic("d")
            .build_for(&provider);
        let cached_count = blocks
            .iter()
            .filter(|b| matches!(b, SystemBlock::Cached(_, _)))
            .count();
        assert_eq!(
            cached_count, 0,
            "provider {provider:?} must drop cache markers"
        );
    }
}

/// T1908 (c): byte-stability over 1000 random-content inputs.
///
/// We use a deterministic LCG seeded with `0xC0FFEE` so the 1000-iteration
/// gate runs hermetically without dragging in `proptest` (already a dev-dep
/// but a deterministic gate is cheaper).
#[test]
fn t1908_c_byte_stable_over_1000_inputs() {
    let mut state: u64 = 0x00C0_FFEE;
    let lcg = |s: &mut u64| {
        *s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        *s
    };
    for _ in 0..1_000 {
        let project = format!("p_{}", lcg(&mut state));
        let role = format!("r_{}", lcg(&mut state));
        let dynamic = format!("d_{}", lcg(&mut state));

        let blocks_a = CachedSystemPrompt::builder()
            .project(project.clone())
            .role(role.clone())
            .dynamic(dynamic.clone())
            .build_for(&ProviderKind::Anthropic);
        let blocks_b = CachedSystemPrompt::builder()
            .project(project)
            .role(role)
            .dynamic(dynamic)
            .build_for(&ProviderKind::Anthropic);
        let json_a = serde_json::to_string(&blocks_a).unwrap();
        let json_b = serde_json::to_string(&blocks_b).unwrap();
        assert_eq!(json_a, json_b, "build_for non-deterministic on iteration");
    }
}

/// T1908 (d): the same `(project, role, dynamic)` content renders the
/// same underlying text across Anthropic (3 blocks) and OpenAI (1
/// flattened block) — different shapes, identical content alignment.
#[test]
fn t1908_d_content_alignment_consistent_across_shapes() {
    let project = "PROJECT";
    let role = "ROLE";
    let dynamic = "DYNAMIC";

    let anthropic = CachedSystemPrompt::builder()
        .project(project)
        .role(role)
        .dynamic(dynamic)
        .build_for(&ProviderKind::Anthropic);
    let openai = CachedSystemPrompt::builder()
        .project(project)
        .role(role)
        .dynamic(dynamic)
        .build_for(&ProviderKind::OpenAi);

    let anthropic_text = anthropic
        .iter()
        .map(|b| match b {
            SystemBlock::Plain(t) | SystemBlock::Cached(t, _) => t.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let openai_text = openai
        .iter()
        .map(|b| match b {
            SystemBlock::Plain(t) | SystemBlock::Cached(t, _) => t.as_str(),
        })
        .collect::<Vec<_>>()
        .join("");

    assert_eq!(anthropic_text, openai_text);
}
