//! `CachedSystemPrompt` builder — provider-aware system-prompt
//! composition (T1908).
//!
//! Design § Q5 resolution (`spec/v2-llm-strategy/feature.md:1335`):
//!
//! - **TTL-driven**, no explicit invalidation — Anthropic's 5-minute
//!   ephemeral TTL evicts edited entries automatically.
//! - **2 breakpoints** — `(project_ctx, role_ctx, dynamic_ctx)` layered
//!   exactly per the brief's strawman. Project + role each get a
//!   [`CacheBreakpoint::Ephemeral`] marker.
//! - **Provider-aware emission** — `build_for(ProviderKind)` switch:
//!     - `Anthropic` → two `SystemBlock::Cached` markers + one
//!       `SystemBlock::Plain` (dynamic).
//!     - any other variant → single flattened `SystemBlock::Plain`
//!       with the three sections joined by `\n\n`, plus one
//!       `tracing::debug!(target: "llm.cache",
//!       "cache_markers_dropped_for_provider")` line per builder
//!       construction so the operator can grep the forensic record.
//! - **Byte-stable.** Same inputs → identical `Vec<SystemBlock>` bytes
//!   across calls (verified by proptest in the integration test).

use crate::trait_def::{CacheBreakpoint, SystemBlock};
use crate::ProviderKind;

/// Composed system prompt ready for provider-specific emission.
///
/// Construct via [`CachedSystemPrompt::builder`] and finalize with
/// [`CachedSystemPromptBuilder::build_for`]. Direct field access is
/// `pub(crate)` so the builder is the only public construction path —
/// keeps the byte-stability invariant in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedSystemPrompt {
    pub(crate) project_ctx: String,
    pub(crate) role_ctx: String,
    pub(crate) dynamic_ctx: String,
}

impl CachedSystemPrompt {
    /// Start a builder with empty project / role / dynamic sections.
    #[must_use]
    pub fn builder() -> CachedSystemPromptBuilder {
        CachedSystemPromptBuilder::default()
    }
}

/// Builder for [`CachedSystemPrompt`].
///
/// Three chained setters (`.project(...)`, `.role(...)`, `.dynamic(...)`)
/// followed by `.build_for(ProviderKind)` returning the
/// `Vec<SystemBlock>` for the request's `system` field.
#[derive(Debug, Default, Clone)]
pub struct CachedSystemPromptBuilder {
    project_ctx: String,
    role_ctx: String,
    dynamic_ctx: String,
}

impl CachedSystemPromptBuilder {
    /// Set the project context (the most stable layer — rarely changes).
    #[must_use]
    pub fn project(mut self, text: impl Into<String>) -> Self {
        self.project_ctx = text.into();
        self
    }

    /// Set the role context (changes per consumer brief, not per call).
    #[must_use]
    pub fn role(mut self, text: impl Into<String>) -> Self {
        self.role_ctx = text.into();
        self
    }

    /// Set the dynamic context (per-call changes — never cached).
    #[must_use]
    pub fn dynamic(mut self, text: impl Into<String>) -> Self {
        self.dynamic_ctx = text.into();
        self
    }

    /// Finalize for `provider` — emits cache markers only for Anthropic;
    /// every other provider receives a single flattened block plus a
    /// `cache_markers_dropped_for_provider` debug-trace breadcrumb.
    #[must_use]
    pub fn build_for(self, provider: &ProviderKind) -> Vec<SystemBlock> {
        let prompt = CachedSystemPrompt {
            project_ctx: self.project_ctx,
            role_ctx: self.role_ctx,
            dynamic_ctx: self.dynamic_ctx,
        };
        prompt.emit(provider)
    }
}

impl CachedSystemPrompt {
    /// Provider-aware emission. Public via the builder; internal helper
    /// otherwise so the byte-stable contract has a single implementation.
    pub(crate) fn emit(&self, provider: &ProviderKind) -> Vec<SystemBlock> {
        match provider {
            ProviderKind::Anthropic => self.emit_anthropic(),
            other => {
                tracing::debug!(
                    target: "llm.cache",
                    provider = ?other,
                    "cache_markers_dropped_for_provider"
                );
                self.emit_flattened()
            }
        }
    }

    fn emit_anthropic(&self) -> Vec<SystemBlock> {
        // Empty sections are still emitted as their own block so the
        // wire-format builder can decide whether to skip them — keeps
        // the per-section identity stable for hash-keyed replay.
        vec![
            SystemBlock::Cached(self.project_ctx.clone(), CacheBreakpoint::Ephemeral),
            SystemBlock::Cached(self.role_ctx.clone(), CacheBreakpoint::Ephemeral),
            SystemBlock::Plain(self.dynamic_ctx.clone()),
        ]
    }

    fn emit_flattened(&self) -> Vec<SystemBlock> {
        // Identical separator (`\n\n`) regardless of empty sections so
        // the same `(project, role, dynamic)` always renders the same
        // bytes — byte-stability invariant.
        let combined = format!(
            "{}\n\n{}\n\n{}",
            self.project_ctx, self.role_ctx, self.dynamic_ctx
        );
        vec![SystemBlock::Plain(combined)]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T1908 (a): Anthropic build emits exactly 2 `Cached` markers.
    #[test]
    fn t1908_anthropic_emits_two_cached_markers() {
        let blocks = CachedSystemPrompt::builder()
            .project("p")
            .role("r")
            .dynamic("d")
            .build_for(&ProviderKind::Anthropic);
        assert_eq!(blocks.len(), 3);
        let cached_count = blocks
            .iter()
            .filter(|b| matches!(b, SystemBlock::Cached(_, _)))
            .count();
        assert_eq!(cached_count, 2, "exactly 2 cache markers for Anthropic");
        // Order: project (Cached), role (Cached), dynamic (Plain).
        assert!(matches!(&blocks[0], SystemBlock::Cached(t, _) if t == "p"));
        assert!(matches!(&blocks[1], SystemBlock::Cached(t, _) if t == "r"));
        assert!(matches!(&blocks[2], SystemBlock::Plain(t) if t == "d"));
    }

    /// T1908 (b): OpenAI / Ollama builds emit zero markers.
    #[test]
    fn t1908_non_anthropic_emits_zero_markers() {
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
            assert_eq!(blocks.len(), 1, "provider {provider:?} flattens to 1 block");
            assert!(
                matches!(&blocks[0], SystemBlock::Plain(_)),
                "no Cached marker for provider {provider:?}"
            );
            let cached_count = blocks
                .iter()
                .filter(|b| matches!(b, SystemBlock::Cached(_, _)))
                .count();
            assert_eq!(cached_count, 0);
        }
    }

    /// T1908 (c): byte-stability — same inputs → identical `Vec<SystemBlock>`
    /// bytes across calls. Hand-rolled `Vec<u8>` equality via serde-json
    /// keeps the assertion proptest-ready without taking a new dep.
    #[test]
    fn t1908_byte_stable_across_repeated_builds() {
        let make = || {
            CachedSystemPrompt::builder()
                .project("PROJECT_CTX")
                .role("ROLE_CTX")
                .dynamic("DYNAMIC_CTX_42")
                .build_for(&ProviderKind::Anthropic)
        };
        let a = make();
        let b = make();
        assert_eq!(a, b);
        let json_a = serde_json::to_string(&a).unwrap();
        let json_b = serde_json::to_string(&b).unwrap();
        assert_eq!(json_a, json_b);
    }

    /// T1908 (d): same project + role + dynamic content under the
    /// Anthropic shape vs the flattened shape — the underlying text is
    /// the same (provider-specific structure differs, but content is
    /// consistent so a record/replay cache key built off content
    /// alignment stays predictable).
    #[test]
    fn t1908_content_consistency_across_shapes() {
        let project = "shared project ctx";
        let role = "shared role ctx";
        let dynamic = "shared dynamic ctx";

        let anthropic_blocks = CachedSystemPrompt::builder()
            .project(project)
            .role(role)
            .dynamic(dynamic)
            .build_for(&ProviderKind::Anthropic);
        let openai_blocks = CachedSystemPrompt::builder()
            .project(project)
            .role(role)
            .dynamic(dynamic)
            .build_for(&ProviderKind::OpenAi);

        // Anthropic: 3 blocks → concatenate sections.
        let anthropic_text: String = anthropic_blocks
            .iter()
            .map(|b| match b {
                SystemBlock::Plain(t) | SystemBlock::Cached(t, _) => t.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // OpenAI: 1 flattened block.
        let openai_text: String = openai_blocks
            .iter()
            .map(|b| match b {
                SystemBlock::Plain(t) | SystemBlock::Cached(t, _) => t.as_str(),
            })
            .collect::<Vec<_>>()
            .join("");

        assert_eq!(anthropic_text, openai_text);
    }

    /// Empty sections still emit byte-stable output.
    #[test]
    fn t1908_empty_sections_render_byte_stable() {
        let a = CachedSystemPrompt::builder()
            .project("")
            .role("")
            .dynamic("")
            .build_for(&ProviderKind::Anthropic);
        let b = CachedSystemPrompt::builder()
            .project("")
            .role("")
            .dynamic("")
            .build_for(&ProviderKind::Anthropic);
        assert_eq!(a, b);
    }
}
