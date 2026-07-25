//! R6 — Memory highlights body renderer.
//!
//! v1+ shipped a placeholder body. Reflection-memory (T1810)
//! replaces the placeholder with:
//! - the byte-locked Q7 empty-state constant
//!   [`REFLECTION_MEMORY_EMPTY_STATE`] when no cards are available,
//! - or one bullet per retrieved lesson card per R4.2's body shape.
//!
//! [`render_with_decay`] is preserved (delegated-to from
//! [`render_with_lessons`]) so the strategy-decay heuristic from
//! T811 still composes (R4.1).
//!
//! ## Body byte invariants (R10.4 / R5.3)
//!
//! - `closed_at` is the only timestamp in any rendered card line and
//!   sources from the audit ledger (`journal_transactions.ts`,
//!   RFC3339 microsecond precision via the journal-format helper).
//! - No wall-clock leakage; no `OffsetDateTime::now_utc()` reachable
//!   from this module.
//! - Numerics are `Decimal`-only (no `f64`).

use std::fmt::Write as _;

use audit::query::StrategyPnl;
use reflection::regime::RegimeTag;
use reflection::types::{LessonCard, RetrievalQuery, SymbolOrPair};
use rust_decimal::Decimal;
use trading_core::{StrategyId, Symbol};

use crate::render::risk_metrics::SharpeFn;

/// Q7 — operator-locked empty-state body.
///
/// Pinned as a `pub const` so a future architect grep-changes in one
/// place (and re-locks the two `report-sample-*` body-SHA-256
/// anchors at `evidence/anchors.toml:67-75`).
pub const REFLECTION_MEMORY_EMPTY_STATE: &str =
    "_no closed trades yet — lesson cards will appear after the first closed trade._\n";

/// Render the R6 memory-highlights body, including a one-line "decay
/// candidates" footer when the strategy-decay heuristic fired.
///
/// Equivalent to [`render_with_lessons`] called with an empty
/// `lessons` slice — emits the empty-state body.  Callers can use
/// either; this entry-point predates the lessons body and remains
/// for back-compat callers (e.g. fixture-only tests that don't
/// exercise the reflection store).
#[must_use]
pub fn render_with_decay(decayed: &[String]) -> String {
    render_with_lessons(decayed, &[])
}

/// Render the R6 memory-highlights body, with an optional list of
/// retrieved lesson cards.
///
/// Body shape per R4.2:
/// ```text
/// ## Memory highlights
///
/// Top {N} lesson cards retrieved this period:
/// - YYYY-MM-DD [Win|Loss|Scratch] strategy_id symbol_or_pair regime=bull|bear|chop held=N bars pnl=D
/// - …
///
/// decay candidates: alpha, beta
/// ```
///
/// When `lessons.is_empty()` the body emits
/// [`REFLECTION_MEMORY_EMPTY_STATE`] verbatim instead of the bullets.
#[must_use]
pub fn render_with_lessons(decayed: &[String], lessons: &[LessonCard]) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("## Memory highlights\n\n");
    if lessons.is_empty() {
        out.push_str(REFLECTION_MEMORY_EMPTY_STATE);
    } else {
        let _ = writeln!(
            out,
            "Top {n} lesson cards retrieved this period:",
            n = lessons.len()
        );
        for c in lessons {
            out.push_str(&format_card_line(c));
        }
    }
    if !decayed.is_empty() {
        out.push('\n');
        out.push_str("decay candidates: ");
        out.push_str(&decayed.join(", "));
        out.push('\n');
    }
    out
}

/// Format one card as a single bullet line.
///
/// `closed_at` is rendered as `YYYY-MM-DD` in UTC.  Format pinned —
/// any change re-anchors `report-sample-*`.
fn format_card_line(c: &LessonCard) -> String {
    let dt = c.closed_at.inner();
    let date = format!(
        "{:04}-{:02}-{:02}",
        dt.year(),
        u8::from(dt.month()),
        dt.day()
    );
    // Use exit_regime in the body line — it captures the regime at
    // trade close, which is the more decision-relevant data point
    // for the operator scanning the highlights paragraph.
    let regime = c.exit_regime;
    let pnl = c.signed_pnl.amount();
    format!(
        "- {date} [{outcome}] {strategy} {symbol} regime={regime} held={bars} bars pnl={pnl}\n",
        outcome = c.outcome_class,
        strategy = c.strategy_id.0,
        symbol = c.symbol_or_pair,
        regime = regime,
        bars = c.holding_period_bars,
        pnl = pnl,
    )
}

/// Build a retrieval query from a per-strategy P&L summary + the
/// current regime + the period-end timestamp.
///
/// Per Q3f's largest-abs-PnL rule:
/// 1. `strategy_id` = the non-`(unattributed)` strategy with the
///    largest absolute realised P&L this period (tie-break:
///    lex-sorted ASC).
/// 2. `symbol_or_pair` = picked from the caller (architect's spec
///    routes this through a `Ledger` lookup); the v1 helper takes
///    a pre-resolved `Symbol` because the report renderer already
///    has it from the strategy-attribution table.
/// 3. `current_regime_tag` = passed in by the caller (it sources
///    from `classify_regime(btc_closes, period_end)`).
///
/// Returns `None` iff no non-unattributed strategy has any P&L this
/// period — the renderer then emits the empty-state body (R4.4).
#[must_use]
pub fn build_retrieval_query(
    pnls: &[StrategyPnl],
    current_regime: RegimeTag,
    fallback_symbol: &Symbol,
) -> Option<RetrievalQuery> {
    let chosen = pick_largest_abs_pnl_strategy(pnls)?;
    Some(RetrievalQuery {
        strategy_id: chosen.0.clone(),
        // V1 default: `(BTCUSDT)` is the canonical fallback symbol
        // for non-pair strategies — caller supplies it.  Pair
        // strategies will surface here as the `a` leg of the pair
        // once the report wires the v1.5a `PairMembership` lookup;
        // keeping the helper symbol-only matches Q3f's "picked from
        // the strategy-attribution table" framing.
        symbol_or_pair: SymbolOrPair::Single(fallback_symbol.clone()),
        current_regime,
    })
}

/// Pick the largest-abs-PnL non-`(unattributed)` strategy.  Tie-break
/// by lex-sorted `strategy_id` ASC.  Returns `None` if no
/// non-unattributed strategy has any non-zero P&L.
fn pick_largest_abs_pnl_strategy(pnls: &[StrategyPnl]) -> Option<(&StrategyId, Decimal)> {
    let unattributed = StrategyId::new("(unattributed)");
    let mut candidates: Vec<(&StrategyId, Decimal)> = pnls
        .iter()
        .filter(|p| p.strategy_id != unattributed)
        .map(|p| (&p.strategy_id, p.realized.amount().abs()))
        .filter(|(_, abs_pnl)| *abs_pnl > Decimal::ZERO)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    // Sort by abs PnL DESC; tie-break on strategy_id ASC.
    candidates.sort_by(|a, b| match b.1.cmp(&a.1) {
        std::cmp::Ordering::Equal => a.0.0.as_str().cmp(b.0.0.as_str()),
        other => other,
    });
    candidates.into_iter().next()
}

// ── Strategy-decay heuristic (T811) ──────────────────────────────────────────

/// One strategy's per-window equity slice + identifier.  Used by
/// [`decay_fired`] to thread per-strategy series into the heuristic
/// without depending on the audit query types.
#[derive(Debug, Clone)]
pub struct StrategyEquitySlice {
    /// Strategy identifier (display only — the heuristic does not key
    /// on it; sorting is the caller's responsibility).
    pub strategy_id: String,
    /// Restricted equity curve at 1m / 5m cadence (Decimal-only, no
    /// `f64`).
    pub equity: Vec<Decimal>,
    /// Same curve restricted to the trailing 7 days.
    pub last_7d_equity: Vec<Decimal>,
}

/// Compute "decay fired?" for the strategy decay R9 risk.
///
/// Returns `true` iff any strategy in `slices` has `last_7d_sharpe < 0`
/// **and** `inception_sharpe > 0`.
#[must_use]
pub fn decay_fired(slices: &[StrategyEquitySlice], sharpe_fn: SharpeFn) -> bool {
    for s in slices {
        let inception = sharpe_fn(&s.equity);
        let last_7d = sharpe_fn(&s.last_7d_equity);
        #[allow(clippy::float_cmp)]
        if last_7d.last_7d < 0.0 && inception.inception > 0.0 {
            return true;
        }
    }
    false
}

/// Return the per-strategy slices that triggered the decay risk.
#[must_use]
pub fn decayed_strategies(slices: &[StrategyEquitySlice], sharpe_fn: SharpeFn) -> Vec<String> {
    let mut out = Vec::new();
    for s in slices {
        let inception = sharpe_fn(&s.equity);
        let last_7d = sharpe_fn(&s.last_7d_equity);
        #[allow(clippy::float_cmp)]
        if last_7d.last_7d < 0.0 && inception.inception > 0.0 {
            out.push(s.strategy_id.clone());
        }
    }
    out.sort();
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::render::risk_metrics::SharpeStats;
    use rust_decimal_macros::dec;

    /// Synthetic Sharpe provider used so we can exercise the heuristic
    /// without the real R4 module.
    fn synthetic_sharpe(values: &[Decimal]) -> SharpeStats {
        let parse_first = values
            .first()
            .copied()
            .map_or(0.0, |d| d.to_string().parse::<f64>().unwrap_or(0.0));
        let parse_last = values
            .last()
            .copied()
            .map_or(0.0, |d| d.to_string().parse::<f64>().unwrap_or(0.0));
        SharpeStats {
            inception: parse_first,
            last_7d: parse_last,
        }
    }

    #[test]
    fn empty_lessons_emits_empty_state_constant() {
        let body = render_with_lessons(&[], &[]);
        assert!(body.starts_with("## Memory highlights\n\n"));
        assert!(body.contains(REFLECTION_MEMORY_EMPTY_STATE));
        assert!(!body.contains("decay candidates:"));
    }

    #[test]
    fn empty_state_constant_byte_locked() {
        // Q7 — operator-locked.  Any change re-anchors the two
        // `report-sample-*` SHAs.  This unit test is the byte
        // guard.
        assert_eq!(
            REFLECTION_MEMORY_EMPTY_STATE,
            "_no closed trades yet — lesson cards will appear after the first closed trade._\n"
        );
    }

    #[test]
    fn render_with_decay_back_compat_no_lessons() {
        let body = render_with_decay(&[]);
        assert!(body.contains(REFLECTION_MEMORY_EMPTY_STATE));
    }

    #[test]
    fn render_with_decay_emits_one_line_per_decay() {
        let decayed = vec!["alpha".to_string(), "zeta".to_string()];
        let body = render_with_decay(&decayed);
        assert!(body.contains("decay candidates: alpha, zeta"));
    }

    #[test]
    fn body_contains_no_run_varying_fields() {
        // No timestamps, no run-id, no hostname leak into the body
        // (R5.3 negative invariant).
        let body = render_with_lessons(&[], &[]);
        for forbidden in [
            "generated:",
            "run_id:",
            "wall_clock_s:",
            "ledger_snapshot_sha:",
            "data_source:",
            "agent_pid:",
            "host:",
            "git_commit:",
        ] {
            assert!(
                !body.contains(forbidden),
                "body leaks volatile field {forbidden}"
            );
        }
    }

    #[test]
    fn t811_decay_fires_when_inception_pos_and_last7d_neg() {
        let slice = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0), dec!(2.0), dec!(3.0)],
            last_7d_equity: vec![dec!(2.0), dec!(1.0), dec!(-1.0)],
        };
        assert!(decay_fired(&[slice], synthetic_sharpe));
    }

    #[test]
    fn t811_decay_does_not_fire_when_both_positive() {
        let slice = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0), dec!(2.0), dec!(3.0)],
            last_7d_equity: vec![dec!(2.0), dec!(3.0), dec!(4.0)],
        };
        assert!(!decay_fired(&[slice], synthetic_sharpe));
    }

    #[test]
    fn t811_decayed_strategies_returns_sorted_ids() {
        let zeta = StrategyEquitySlice {
            strategy_id: "zeta".into(),
            equity: vec![dec!(1.0)],
            last_7d_equity: vec![dec!(-1.0)],
        };
        let alpha = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0)],
            last_7d_equity: vec![dec!(-1.0)],
        };
        let names = decayed_strategies(&[zeta, alpha], synthetic_sharpe);
        assert_eq!(names, vec!["alpha".to_string(), "zeta".to_string()]);
    }
}
