//! Prompt builder for the LLM-forecaster (T-D-N(B2)).
//!
//! ## Design (T-AR-2 architect-locked)
//!
//! The system prompt is composed via [`llm::CachedSystemPromptBuilder`] with
//! exactly 2 cache breakpoints (project ~800 tokens, role ~1200 tokens) per the
//! architect lock at `decomp.md § T-AR-2`.
//!
//! The per-call dynamic block is rendered as **markdown** (architect-pick over
//! JSON per decomp.md § T-AR-2 rationale). The rendered markdown is passed as
//! the `dynamic` section — it is NOT cached.
//!
//! ## Cache stability
//!
//! The project and role blocks are byte-stable: they do not reference
//! `ForecastContext` fields. The dynamic block changes on every call (bar
//! timestamps, price values, indicator snapshots). This is the expected pattern
//! for the `CachedSystemPromptBuilder` (T1908 tested in `crates/llm`).
//!
//! ## Cross-references
//!
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-2` — prompt structure + cache breakpoints.
//! - `spec/dev-notes/v3-llm-forecaster-prompt-spike-2026-05-22.md § v3` — final template.
//! - `crates/llm/src/prompt_cache.rs` — `CachedSystemPromptBuilder`.
//! - `spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md § D1.b` — L4 trace gate.

use llm::CachedSystemPromptBuilder;

use super::types::ForecastContext;

// ── Project context block (cached, ~800 tokens) ───────────────────────────────

/// Project-level context block for the system prompt (the first cached breakpoint).
///
/// This text is byte-stable across all calls — it describes the agent's
/// architectural context and does NOT depend on `ForecastContext` fields.
/// Anthropic caches this block with a 5-minute TTL (`CacheBreakpoint::Ephemeral`).
pub const PROJECT_CONTEXT: &str = "\
You are the `llm_forecaster` agent inside a Rust crypto trading binary.

Architecture context:
- The audit ledger is double-entry SQLite at `data/audit/audit.db`. Every decision \
you make is journaled as a `JournalEntry { kind: \"llm_forecast\", payload }` row.
- Reflection memory is the lesson-card store at `data/reflection/reflection.db`. \
Closed trades become retrievable lesson cards via 32-dim deterministic embedding. \
You will be given the top-K most relevant cards for each forecast call.
- Replay-cache at `data/llm-forecaster-replay.db` (live) / \
`crates/strategy/tests/fixtures/llm-forecaster-replay.db.gz` (test fixture). \
`temperature = 0` is pinned; your responses are cached for replay-determinism.
- You operate in a disciplined research environment. Every forecast is measured \
against realized outcomes. Be calibrated — your confidence score will be \
correlated against actual next-bar direction to compute a calibration metric.";

// ── Role context block (cached, ~1200 tokens) ─────────────────────────────────

/// Role context block for the system prompt (the second cached breakpoint).
///
/// Describes the LLM's specific task and the `propose_forecast` tool contract.
/// Byte-stable across all calls; does NOT reference `ForecastContext` fields.
pub const ROLE_CONTEXT: &str = "\
You are forecasting the next-1 hour direction of a cryptocurrency trading pair.

Your task:
1. Analyze the provided recent OHLCV bars (last 24 hourly candles).
2. Analyze the provided technical indicator snapshot.
3. Review the top-K retrieved lesson cards from past trades on this pair.
4. Review recent audit decisions to understand recent forecast history.
5. Emit your forecast using the `propose_forecast` tool.

Rating scale (5-tier, discrete):
- STRONG_BUY  — high conviction bullish; price likely up >1% over next 1h
- BUY         — moderate bullish; price likely up 0.3–1% over next 1h
- HOLD        — no directional edge; stay flat
- SELL        — moderate bearish; price likely down 0.3–1% over next 1h
- STRONG_SELL — high conviction bearish; price likely down >1% over next 1h

Calibration contract:
- `confidence` is a float in [0.0, 1.0]. It will be correlated against realized \
outcome. High-confidence WRONG forecasts are penalized. Be honest about uncertainty.
- `reasoning_trace` must be 50–2000 characters of structured prose explaining WHICH \
signals drove the rating. Reference specific indicator values, lesson card insights, \
and recent decision patterns.
- `cited_lesson_ids` should list any lesson card IDs that influenced the rating. \
If none are relevant, pass an empty array.
- `horizon` is always \"short\" at v0.1.0 (next 1h).

Call `propose_forecast` with the structured payload. Do not emit free-form text.";

// ── Dynamic context renderer ──────────────────────────────────────────────────

/// Render the per-call dynamic block from a [`ForecastContext`].
///
/// This markdown text is NOT cached — it changes on every call. It is passed
/// to the `CachedSystemPromptBuilder::dynamic(...)` setter. The format is
/// architect-locked at `decomp.md § T-AR-2` (markdown tables for OHLCV,
/// bullet list for indicators, ordered lists for lesson cards and decisions).
///
/// ## Distilled summary section
///
/// The `## Distilled summary:` section is **omitted entirely** when
/// `reflection::DISTILLATION_ENABLED = false` (the v0.1.0 default per
/// `crates/reflection/src/lib.rs:20-24`). The renderer must NOT emit an empty
/// section heading — per the spike dev-note (`spec/dev-notes/...` Notes).
#[must_use]
pub fn render_dynamic_block(ctx: &ForecastContext) -> String {
    let mut out = String::with_capacity(4096);

    // Symbol + timestamp header
    let now_iso = ctx
        .now
        .0
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    out.push_str(&format!(
        "## Symbol: {}  (now = {})\n\n",
        ctx.symbol, now_iso
    ));

    // OHLCV table
    out.push_str("## Recent OHLCV bars (last 24h, hourly):\n");
    out.push_str("| timestamp | open | high | low | close | volume |\n");
    out.push_str("|-----------|------|------|-----|-------|--------|\n");
    for bar in &ctx.recent_bars {
        let ts_iso = bar
            .open_ts
            .0
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        // Format prices to 2 decimal places; volume as integer.
        let open = bar.open.get();
        let high = bar.high.get();
        let low = bar.low.get();
        let close = bar.close.get();
        let vol = bar.volume.get();
        out.push_str(&format!(
            "| {ts_iso} | {open:.2} | {high:.2} | {low:.2} | {close:.2} | {vol:.0} |\n"
        ));
    }
    out.push('\n');

    // Technical indicators
    let ind = &ctx.indicators;
    out.push_str("## Technical indicators:\n");
    out.push_str(&format!("- RSI(14) = {:.2}\n", ind.rsi_14));
    out.push_str(&format!(
        "- MACD(12,26,9) = {:.4} / {:.4} / {:.4}  (macd / signal / hist)\n",
        ind.macd, ind.macd_signal, ind.macd_hist
    ));
    out.push_str(&format!(
        "- BB(20, 2) = {:.2} / {:.2}  (upper / lower band)\n",
        ind.bb_upper, ind.bb_lower
    ));
    out.push_str(&format!("- ATR(14) = {:.4}\n", ind.atr_14));
    out.push_str(&format!(
        "- realized_vol_24h = {:.6}  (annualized)\n",
        ind.realized_vol_24h
    ));
    out.push_str(&format!("- vol_of_vol_7d = {:.6}\n", ind.vol_of_vol_7d));
    out.push('\n');

    // Retrieved lesson cards
    out.push_str("## Retrieved lesson cards (top K=5 by similarity):\n");
    if ctx.top_k_lessons.is_empty() {
        out.push_str("(none retrieved)\n");
    } else {
        for (i, card) in ctx.top_k_lessons.iter().enumerate() {
            let note_str = card.note.as_deref().unwrap_or("(none)");
            out.push_str(&format!(
                "{}. [card_id: {}] symbol={} entry_regime={:?} exit_regime={:?} outcome={:?} note={}\n",
                i + 1,
                card.card_id,
                card.symbol_or_pair,
                card.entry_regime,
                card.exit_regime,
                card.outcome_class,
                note_str,
            ));
        }
    }
    out.push('\n');

    // Recent audit decisions
    out.push_str("## Recent audit decisions (last N=10 forecasts on this symbol):\n");
    if ctx.recent_decisions.is_empty() {
        out.push_str("(no prior decisions)\n");
    } else {
        for (i, decision) in ctx.recent_decisions.iter().enumerate() {
            let outcome_str = decision.outcome.as_deref().unwrap_or("open");
            out.push_str(&format!(
                "{}. [audit_id: {}] rating={} confidence={:.4} outcome={}\n",
                i + 1,
                decision.audit_id,
                decision.rating.as_str(),
                decision.confidence.value(),
                outcome_str,
            ));
        }
    }

    // Distilled summary: section is OMITTED when DISTILLATION_ENABLED = false
    // (v0.1.0 default per decomp § T-AR-3 + spike notes).
    // When reflection::DISTILLATION_ENABLED flips to true in a future brief,
    // this section activates via the optional distilled_summary field on ForecastContext.
    // No empty heading emitted here.

    out
}

/// Build the full `Vec<SystemBlock>` system prompt for a `ChatRequest`.
///
/// Returns the `Vec<SystemBlock>` produced by `CachedSystemPromptBuilder::build_for`.
/// Caller passes `ctx` for the dynamic block; `provider_kind` for the cache-marker
/// translation.
///
/// ## Cache breakpoints
///
/// For `ProviderKind::Anthropic`:
/// - Block 0: `SystemBlock::Cached(PROJECT_CONTEXT, Ephemeral)` — breakpoint 1.
/// - Block 1: `SystemBlock::Cached(ROLE_CONTEXT, Ephemeral)` — breakpoint 2.
/// - Block 2: `SystemBlock::Plain(dynamic_block)` — NOT cached.
///
/// For other providers: single flattened `SystemBlock::Plain` (cache markers dropped).
#[must_use]
pub fn build_system_prompt(
    ctx: &ForecastContext,
    provider_kind: &llm::ProviderKind,
) -> Vec<llm::trait_def::SystemBlock> {
    let dynamic = render_dynamic_block(ctx);
    CachedSystemPromptBuilder::default()
        .project(PROJECT_CONTEXT)
        .role(ROLE_CONTEXT)
        .dynamic(dynamic)
        .build_for(provider_kind)
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use llm::{ProviderKind, trait_def::SystemBlock};
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

    use crate::llm_forecaster::types::{ForecastContext, TechnicalIndicators};

    fn make_symbol(s: &str) -> Symbol {
        Symbol::new(s)
    }

    fn make_ts(epoch_s: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
    }

    fn make_bar(symbol: &str, open_ts_s: i64) -> Bar {
        let sym = make_symbol(symbol);
        let ts = make_ts(open_ts_s);
        Bar {
            symbol: sym,
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: make_ts(open_ts_s + 3600),
            open: Price::new(dec!(45000)).expect("positive price"),
            high: Price::new(dec!(45100)).expect("positive price"),
            low: Price::new(dec!(44900)).expect("positive price"),
            close: Price::new(dec!(45050)).expect("positive price"),
            volume: Quantity::new(dec!(1000)).expect("positive qty"),
            trade_count: 100,
            local_recv_ts: ts,
            venue: Venue::Binance,
        }
    }

    fn minimal_ctx() -> ForecastContext {
        ForecastContext::test_fixture(
            make_symbol("BTCUSDT"),
            make_ts(1_700_000_000),
            vec![make_bar("BTCUSDT", 1_700_000_000)],
        )
    }

    /// Anthropic build emits exactly 2 `Cached` markers (T-D-N(B5) cache-breakpoint assert).
    #[test]
    fn anthropic_prompt_emits_exactly_two_cache_breakpoints() {
        let ctx = minimal_ctx();
        let blocks = build_system_prompt(&ctx, &ProviderKind::Anthropic);
        assert_eq!(blocks.len(), 3, "Anthropic must emit 3 blocks");
        let cached_count = blocks
            .iter()
            .filter(|b| matches!(b, SystemBlock::Cached(_, _)))
            .count();
        assert_eq!(cached_count, 2, "exactly 2 cache breakpoints for Anthropic");
        // Block order: project (Cached), role (Cached), dynamic (Plain).
        assert!(
            matches!(&blocks[0], SystemBlock::Cached(t, _) if t.contains("llm_forecaster")),
            "block[0] must be the project context"
        );
        assert!(
            matches!(&blocks[1], SystemBlock::Cached(t, _) if t.contains("propose_forecast")),
            "block[1] must be the role context"
        );
        assert!(
            matches!(&blocks[2], SystemBlock::Plain(t) if t.contains("BTCUSDT")),
            "block[2] must be the dynamic block containing the symbol"
        );
    }

    /// Non-Anthropic provider flattens to 1 block with 0 cache markers.
    #[test]
    fn non_anthropic_prompt_emits_zero_cache_breakpoints() {
        let ctx = minimal_ctx();
        let blocks = build_system_prompt(&ctx, &ProviderKind::OpenAi);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], SystemBlock::Plain(_)));
        let cached_count = blocks
            .iter()
            .filter(|b| matches!(b, SystemBlock::Cached(_, _)))
            .count();
        assert_eq!(cached_count, 0);
    }

    /// Dynamic block contains the symbol and timestamp.
    #[test]
    fn dynamic_block_contains_symbol_and_timestamp() {
        let ctx = minimal_ctx();
        let dynamic = render_dynamic_block(&ctx);
        assert!(
            dynamic.contains("BTCUSDT"),
            "dynamic must contain the symbol"
        );
        assert!(
            dynamic.contains("## Symbol:"),
            "dynamic must have the Symbol header"
        );
        assert!(
            dynamic.contains("## Recent OHLCV bars"),
            "dynamic must have the OHLCV header"
        );
    }

    /// Dynamic block includes the OHLCV table header row.
    #[test]
    fn dynamic_block_includes_ohlcv_table_header() {
        let ctx = minimal_ctx();
        let dynamic = render_dynamic_block(&ctx);
        assert!(dynamic.contains("| timestamp | open | high | low | close | volume |"));
    }

    /// Dynamic block includes technical indicator values.
    #[test]
    fn dynamic_block_includes_indicator_values() {
        let mut ctx = minimal_ctx();
        ctx.indicators = TechnicalIndicators {
            rsi_14: dec!(65),
            macd: dec!(0.1234),
            macd_signal: dec!(0.0987),
            macd_hist: dec!(0.0247),
            bb_upper: dec!(46000),
            bb_lower: dec!(44000),
            atr_14: dec!(500),
            realized_vol_24h: dec!(0.02),
            vol_of_vol_7d: dec!(0.005),
        };
        let dynamic = render_dynamic_block(&ctx);
        assert!(dynamic.contains("RSI(14) = 65.00"), "RSI must appear");
        assert!(dynamic.contains("MACD(12,26,9)"), "MACD header must appear");
        assert!(dynamic.contains("ATR(14)"), "ATR header must appear");
    }

    /// Dynamic block does NOT include a Distilled summary section when
    /// distillation is disabled (v0.1.0 default — no empty heading).
    #[test]
    fn dynamic_block_omits_distilled_summary_section_when_distillation_disabled() {
        let ctx = minimal_ctx();
        let dynamic = render_dynamic_block(&ctx);
        assert!(
            !dynamic.contains("## Distilled summary"),
            "distilled summary section must be absent when DISTILLATION_ENABLED = false"
        );
    }

    /// Project context block is byte-stable across two fresh builds.
    #[test]
    fn project_context_is_byte_stable() {
        let ctx1 = minimal_ctx();
        let ctx2 = minimal_ctx();
        let blocks1 = build_system_prompt(&ctx1, &ProviderKind::Anthropic);
        let blocks2 = build_system_prompt(&ctx2, &ProviderKind::Anthropic);
        // The project and role blocks (indices 0 and 1) must be byte-identical.
        let proj1 = match &blocks1[0] {
            SystemBlock::Cached(t, _) => t.clone(),
            _ => panic!(),
        };
        let proj2 = match &blocks2[0] {
            SystemBlock::Cached(t, _) => t.clone(),
            _ => panic!(),
        };
        assert_eq!(proj1, proj2, "project context must be byte-stable");
        let role1 = match &blocks1[1] {
            SystemBlock::Cached(t, _) => t.clone(),
            _ => panic!(),
        };
        let role2 = match &blocks2[1] {
            SystemBlock::Cached(t, _) => t.clone(),
            _ => panic!(),
        };
        assert_eq!(role1, role2, "role context must be byte-stable");
    }

    /// Role context contains the `propose_forecast` tool name.
    #[test]
    fn role_context_contains_propose_forecast() {
        assert!(
            ROLE_CONTEXT.contains("propose_forecast"),
            "role context must instruct LLM to call propose_forecast"
        );
    }

    /// Dynamic block mentions "(none retrieved)" when lesson cards are empty.
    #[test]
    fn dynamic_block_handles_empty_lessons() {
        let ctx = minimal_ctx(); // test_fixture produces empty top_k_lessons
        let dynamic = render_dynamic_block(&ctx);
        assert!(
            dynamic.contains("(none retrieved)"),
            "empty lesson cards must show placeholder"
        );
    }

    /// Dynamic block mentions "(no prior decisions)" when decisions are empty.
    #[test]
    fn dynamic_block_handles_empty_decisions() {
        let ctx = minimal_ctx(); // test_fixture produces empty recent_decisions
        let dynamic = render_dynamic_block(&ctx);
        assert!(
            dynamic.contains("(no prior decisions)"),
            "empty decisions must show placeholder"
        );
    }
}
