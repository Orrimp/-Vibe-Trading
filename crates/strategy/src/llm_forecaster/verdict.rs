//! L0-L4 verdict classifier for the LLM forecaster (ADR-0039 § D1).
//!
//! ## Priority tree (L1 → L2 → L3 → L4 → L0)
//!
//! - **L1** — Bias collapse: `hold_frac ≥ 0.95` (HOLD dominates the rating
//!   distribution).
//! - **L2** — Calibration failure: `|confidence_outcome_corr| < 0.05`
//!   (confidence does not predict directional correctness).
//! - **L3** — Cost overrun: `overrun_ratio > 2.0 OR cost_actual > cost_cap`.
//! - **L4** — Reasoning trace degenerate: `short_frac > 0.50 OR
//!   duplicate_frac > 0.50`.
//! - **L0** — PASS: none of L1-L4 triggered; routes to L_ALPHA gate.
//!
//! ## Mutual exclusivity
//!
//! Priority-tree fallthrough: the first triggering case returns; a call can
//! never return more than one verdict. The unit tests in
//! `crates/strategy/tests/llm_verdict_priority_tree.rs` verify:
//! - Each L1..L4 positive fixture fires its case.
//! - Negative fixture (no rule applies) → L0.
//! - When L1 + L2 both trigger, L1 wins (priority order).
//! - 2-run byte-identity on the report body.
//!
//! ## Cross-references
//!
//! - ADR-0039 § D1 — canonical algorithm source.
//! - `crates/strategy/src/bin/llm_verdict.rs` — report bin that calls this.

use std::collections::HashSet;

// ── Input statistics ──────────────────────────────────────────────────────────

/// Per-window aggregate statistics collected from `llm_forecast_entries` rows.
///
/// This is the `LlmCallStats` shape from ADR-0039 § D1.a (renamed for
/// clarity; the ADR uses it as the input to `classify_l`).
#[derive(Debug, Clone)]
pub struct LlmWindowStats {
    /// Label for this evaluation window (e.g. scenario name or date range).
    pub window_label: String,
    /// Total number of forecast() invocations in the window.
    pub n_calls: u64,
    /// Count of distinct `reasoning_trace` bodies (by trace_sha256).
    pub n_unique_traces: u64,
    /// Rating counts per index: [STRONG_SELL=0, SELL=1, HOLD=2, BUY=3, STRONG_BUY=4].
    pub rating_dist: [u32; 5],
    /// Mean reasoning_trace length in characters.
    pub mean_trace_len_chars: f64,
    /// Count of traces with `len < 50` chars.
    pub n_traces_below_50_chars: u32,
    /// Pearson(confidence_t, signed-correctness indicator) over n_calls.
    /// Positive correlation = confidence tracks correctness.
    /// L2 fires iff `|confidence_outcome_corr| < 0.05`.
    ///
    /// In the audit-DB-only path (no realised return data), this is
    /// approximated as 0.0 (triggers L2 as conservative fallback).
    pub confidence_outcome_corr: f64,
    /// Total LLM cost over this window (USD).
    pub cost_actual_usd: f64,
    /// Architect-locked projected cost from `llm-forecaster-bench` (USD).
    pub cost_projected_usd: f64,
    /// Per-run budget cap from `LlmForecasterConfig::cost_cap_usd_per_backtest`.
    pub cost_cap_usd: f64,
}

impl LlmWindowStats {
    /// Fraction of calls that returned HOLD (L1 bias-collapse metric).
    ///
    /// HOLD is at histogram index 2 per `Rating::histogram_index`.
    #[inline]
    #[must_use]
    pub fn hold_frac(&self) -> f64 {
        self.rating_dist[2] as f64 / self.n_calls.max(1) as f64
    }

    /// Fraction of traces shorter than 50 chars (L4 short-trace metric).
    #[inline]
    #[must_use]
    pub fn short_frac(&self) -> f64 {
        self.n_traces_below_50_chars as f64 / self.n_calls.max(1) as f64
    }

    /// Fraction of traces that are duplicates (L4 duplicate-trace metric).
    ///
    /// `1.0 - n_unique_traces / n_calls`
    #[inline]
    #[must_use]
    pub fn duplicate_frac(&self) -> f64 {
        1.0 - (self.n_unique_traces as f64 / self.n_calls.max(1) as f64)
    }

    /// Cost overrun ratio: `cost_actual / cost_projected` (L3 metric).
    #[inline]
    #[must_use]
    pub fn overrun_ratio(&self) -> f64 {
        self.cost_actual_usd / self.cost_projected_usd.max(1e-6)
    }
}

// ── Verdict enum ──────────────────────────────────────────────────────────────

/// L0-L4 verdict per ADR-0039 § D1.
///
/// Exactly one variant is returned by [`classify_l`] for any input.
/// The variants are ordered by priority; `L1` is the highest-priority failure.
#[derive(Debug, Clone, PartialEq)]
pub enum LVerdict {
    /// L1 — Bias collapse: HOLD fraction ≥ 0.95.
    L1 {
        evidence: String,
        follow_on: &'static str,
    },
    /// L2 — Calibration failure: |confidence_outcome_corr| < 0.05.
    L2 {
        evidence: String,
        follow_on: &'static str,
    },
    /// L3 — Cost overrun: `overrun_ratio > 2.0` OR `cost_actual > cost_cap`.
    L3 {
        evidence: String,
        follow_on: &'static str,
    },
    /// L4 — Reasoning trace degenerate: `short_frac > 0.50` OR
    /// `duplicate_frac > 0.50`.
    L4 {
        evidence: String,
        follow_on: &'static str,
    },
    /// L0 — PASS: none of L1-L4 triggered; routes to L_ALPHA strategy gate.
    L0 {
        evidence: String,
        follow_on: &'static str,
    },
}

impl LVerdict {
    /// Short label for display in the Verdict section table.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            LVerdict::L0 { .. } => "L0",
            LVerdict::L1 { .. } => "L1",
            LVerdict::L2 { .. } => "L2",
            LVerdict::L3 { .. } => "L3",
            LVerdict::L4 { .. } => "L4",
        }
    }

    /// Evidence string for the Verdict section table.
    #[must_use]
    pub fn evidence(&self) -> &str {
        match self {
            LVerdict::L0 { evidence, .. }
            | LVerdict::L1 { evidence, .. }
            | LVerdict::L2 { evidence, .. }
            | LVerdict::L3 { evidence, .. }
            | LVerdict::L4 { evidence, .. } => evidence.as_str(),
        }
    }

    /// Follow-on action (slug or gate name).
    #[must_use]
    pub fn follow_on(&self) -> &str {
        match self {
            LVerdict::L0 { follow_on, .. }
            | LVerdict::L1 { follow_on, .. }
            | LVerdict::L2 { follow_on, .. }
            | LVerdict::L3 { follow_on, .. }
            | LVerdict::L4 { follow_on, .. } => follow_on,
        }
    }

    /// Routing string for the "Routes to" column (ADR-0039 § D2).
    #[must_use]
    pub fn routes_to(&self) -> &str {
        match self {
            LVerdict::L0 { .. } => "L_ALPHA strategy-side gate (Sharpe-comparison bin)",
            LVerdict::L1 { follow_on, .. }
            | LVerdict::L2 { follow_on, .. }
            | LVerdict::L3 { follow_on, .. }
            | LVerdict::L4 { follow_on, .. } => follow_on,
        }
    }

    /// True if this is an L0 (PASS) verdict.
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, LVerdict::L0 { .. })
    }
}

// ── Classifier (ADR-0039 § D1.b) ─────────────────────────────────────────────

/// Classify L-verdict from aggregate window statistics.
///
/// Priority tree: **L1 → L2 → L3 → L4 → L0** (first trigger wins).
/// This function is deterministic over its inputs — no I/O, no randomness.
///
/// Thresholds (immutable per ADR-0039 § Q6 operator lock):
/// - L1: `hold_frac ≥ 0.95`
/// - L2: `|confidence_outcome_corr| < 0.05`
/// - L3: `overrun_ratio > 2.0 OR cost_actual > cost_cap`
/// - L4: `short_frac > 0.50 OR duplicate_frac > 0.50`
/// - L0: fallback
#[must_use]
pub fn classify_l(stats: &LlmWindowStats) -> LVerdict {
    // ── L1 — Bias collapse ────────────────────────────────────────────────────
    //
    // The LLM produces an overwhelming majority of HOLD ratings (≥95% of calls),
    // signalling "no opinion." Any meaningful forecaster produces directional
    // ratings on a non-trivial minority of bars (≥5% = ~4,380 directional calls
    // per full-year backtest). Tighter (99%) lets near-bias-collapsed LLM slip
    // through to L4 and produce false-positive reasoning-trace evidence.
    let hold_frac = stats.hold_frac();
    if hold_frac >= 0.95 {
        return LVerdict::L1 {
            evidence: format!(
                "hold_frac = {} / {} = {:.6} >= 0.95 (bias collapse to HOLD)",
                stats.rating_dist[2], stats.n_calls, hold_frac,
            ),
            follow_on: "v3-llm-forecaster-prompt-redesign",
        };
    }

    // ── L2 — Calibration failure ──────────────────────────────────────────────
    //
    // Pearson correlation between confidence_t and signed-correctness indicator
    // (+1 if rating direction matches sign of realised next-bar log-return,
    // -1 if opposite, 0 if HOLD on either side). L2 fires iff |ρ| < 0.05.
    // At n=87,600, Pearson SE ≈ 0.0034 under H0, so |ρ| < 0.05 is within noise.
    if stats.confidence_outcome_corr.abs() < 0.05 {
        return LVerdict::L2 {
            evidence: format!(
                "|confidence_outcome_corr| = {:.6} < 0.05 (calibration failure)",
                stats.confidence_outcome_corr.abs(),
            ),
            follow_on: "v3-llm-forecaster-calibrate-or-retire",
        };
    }

    // ── L3 — Cost overrun ─────────────────────────────────────────────────────
    //
    // Actual LLM cost > 2× architect-locked projection (bench mis-estimate) OR
    // exceeds the per-run hard cap. 2.0× threshold: 1.5× is within bench error;
    // 2.0× signals real mis-estimate (prompt grew, cache-hit ratio worse, etc.).
    let overrun_ratio = stats.overrun_ratio();
    if overrun_ratio > 2.0 || stats.cost_actual_usd > stats.cost_cap_usd {
        return LVerdict::L3 {
            evidence: format!(
                "cost_actual_usd = {:.6}, cost_projected_usd = {:.6}, \
                 overrun_ratio = {:.6} > 2.0 OR \
                 cost_actual_usd > cost_cap_usd = {:.6}",
                stats.cost_actual_usd, stats.cost_projected_usd, overrun_ratio, stats.cost_cap_usd,
            ),
            follow_on: "v3-llm-forecaster-cost-tune",
        };
    }

    // ── L4 — Reasoning trace degenerate ──────────────────────────────────────
    //
    // The reasoning_trace is too short (< 50 chars) on a majority of calls OR
    // highly duplicate across calls (boilerplate). A trace shorter than ~10
    // words is operationally useless for operator trust-judgment (Phase F
    // Assistant slot). 50% threshold: duplicate-or-short majority signals
    // systematic boilerplate; below 50% is within "LLM sometimes terse" range.
    let short_frac = stats.short_frac();
    let duplicate_frac = stats.duplicate_frac();
    if short_frac > 0.50 || duplicate_frac > 0.50 {
        return LVerdict::L4 {
            evidence: format!(
                "short_frac = {} / {} = {:.6} > 0.50 OR \
                 duplicate_frac = 1 - {} / {} = {:.6} > 0.50",
                stats.n_traces_below_50_chars,
                stats.n_calls,
                short_frac,
                stats.n_unique_traces,
                stats.n_calls,
                duplicate_frac,
            ),
            follow_on: "v3-llm-forecaster-trace-quality-tune",
        };
    }

    // ── L0 — PASS ─────────────────────────────────────────────────────────────
    //
    // Fallback: L1-L4 all false. The LLM produces a non-degenerate rating
    // distribution, calibrated confidence, within-budget cost, and substantive
    // reasoning traces. Routes to the L_ALPHA Sharpe-delta gate (D1.c).
    // L0 PASS does NOT imply alpha — it only certifies usable evidence.
    LVerdict::L0 {
        evidence: format!(
            "hold_frac = {:.6} < 0.95; |confidence_outcome_corr| = {:.6} >= 0.05; \
             overrun_ratio = {:.6} <= 2.0; short_frac = {:.6} <= 0.50; \
             duplicate_frac = {:.6} <= 0.50",
            hold_frac,
            stats.confidence_outcome_corr.abs(),
            overrun_ratio,
            short_frac,
            duplicate_frac,
        ),
        follow_on: "l_alpha_strategy_gate",
    }
}

// ── Audit DB row (for the binary / tests) ────────────────────────────────────

/// One row from `llm_forecast_entries`, as needed by the verdict bin.
///
/// Only the fields required for L0-L4 computation are materialised here;
/// the rest (tokens, model_id, etc.) are loaded by the bin but not needed
/// for the statistical computation.
#[derive(Debug, Clone)]
pub struct LlmForecastRow {
    /// 5-tier rating string (`"HOLD"`, `"BUY"`, etc.).
    pub rating: String,
    /// Confidence value as `f64` in `[0, 1]`.
    pub confidence_f64: f64,
    /// Reasoning trace text.
    pub reasoning_trace: String,
    /// Lowercase 64-hex SHA-256 of the reasoning trace.
    pub trace_sha256: String,
    /// Actual cost for this call (USD) as `f64`.
    pub cost_usd_f64: f64,
}

/// Aggregate a slice of `llm_forecast_entries` rows into `LlmWindowStats`.
///
/// `cost_projected_usd` and `cost_cap_usd` must be supplied by the caller
/// (from config / bench records, not from the audit rows themselves).
///
/// `confidence_outcome_corr` — supplying the actual Pearson correlation
/// requires realised return data which is not in the audit DB. Pass the
/// caller-supplied value (or 0.0 as conservative fallback that triggers L2).
///
/// ## Determinism
///
/// All computations are over sorted, deterministic slices. No randomness,
/// no I/O.
#[must_use]
pub fn aggregate_rows(
    rows: &[LlmForecastRow],
    cost_projected_usd: f64,
    cost_cap_usd: f64,
    confidence_outcome_corr: f64,
    window_label: String,
) -> LlmWindowStats {
    let n = rows.len() as u64;
    if n == 0 {
        return LlmWindowStats {
            window_label,
            n_calls: 0,
            n_unique_traces: 0,
            rating_dist: [0; 5],
            mean_trace_len_chars: 0.0,
            n_traces_below_50_chars: 0,
            confidence_outcome_corr,
            cost_actual_usd: 0.0,
            cost_projected_usd,
            cost_cap_usd,
        };
    }

    let mut rating_dist = [0u32; 5];
    let mut n_traces_below_50_chars = 0u32;
    let mut total_trace_len = 0usize;
    let mut cost_total = 0.0f64;
    let mut unique_shas = HashSet::new();

    for row in rows {
        // Classify rating into histogram index.
        // Matches Rating::histogram_index: STRONG_SELL=0, SELL=1, HOLD=2, BUY=3, STRONG_BUY=4.
        let idx = match row.rating.as_str() {
            "STRONG_SELL" => 0,
            "SELL" => 1,
            "HOLD" => 2,
            "BUY" => 3,
            "STRONG_BUY" => 4,
            _ => 2, // unknown → treat as HOLD (conservative)
        };
        rating_dist[idx] += 1;

        let trace_len = row.reasoning_trace.len();
        total_trace_len += trace_len;
        if trace_len < 50 {
            n_traces_below_50_chars += 1;
        }

        cost_total += row.cost_usd_f64;
        unique_shas.insert(row.trace_sha256.clone());
    }

    let mean_trace_len_chars = total_trace_len as f64 / n as f64;
    let n_unique_traces = unique_shas.len() as u64;

    LlmWindowStats {
        window_label,
        n_calls: n,
        n_unique_traces,
        rating_dist,
        mean_trace_len_chars,
        n_traces_below_50_chars,
        confidence_outcome_corr,
        cost_actual_usd: cost_total,
        cost_projected_usd,
        cost_cap_usd,
    }
}

// ── Unit tests (inlined per CLAUDE.md "write next to the code") ─────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(
        rating: &str,
        confidence: f64,
        trace: &str,
        trace_sha: &str,
        cost: f64,
    ) -> LlmForecastRow {
        LlmForecastRow {
            rating: rating.to_string(),
            confidence_f64: confidence,
            reasoning_trace: trace.to_string(),
            trace_sha256: trace_sha.to_string(),
            cost_usd_f64: cost,
        }
    }

    /// Build a healthy set of rows: mixed ratings, long unique traces, low cost.
    fn healthy_rows() -> Vec<LlmForecastRow> {
        let traces = [
            "The market shows strong bullish momentum with RSI above 60 and MACD crossover.",
            "Bearish divergence detected in MACD; volume declining on recent up-bars.",
            "Neutral stance: price within Bollinger bands, RSI at 48, no clear edge.",
            "Strong buy signal: ATR expansion with breakout above prior resistance.",
            "Sell signal confirmed: MACD histogram turning negative with volume spike.",
        ];
        (0..100)
            .map(|i| {
                let rating = ["BUY", "SELL", "HOLD", "STRONG_BUY", "STRONG_SELL"][i % 5];
                let trace = traces[i % 5];
                make_row(rating, 0.7, trace, &format!("sha_{:04x}", i), 0.001)
            })
            .collect()
    }

    // ── L0 PASS fixture ───────────────────────────────────────────────────────

    #[test]
    fn l0_pass_on_healthy_rows() {
        let rows = healthy_rows();
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "test-window".to_string());
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L0 { .. }),
            "healthy rows should yield L0, got {}",
            v.label()
        );
        assert!(v.is_pass());
    }

    // ── L1 bias collapse ──────────────────────────────────────────────────────

    #[test]
    fn l1_fires_on_hold_dominated_distribution() {
        // 97 HOLD out of 100 → hold_frac = 0.97 ≥ 0.95 → L1
        let mut rows = vec![
            make_row(
                "BUY",
                0.7,
                "Bullish momentum confirmed by RSI.",
                "sha_buy",
                0.001,
            ),
            make_row(
                "SELL",
                0.6,
                "Bearish reversal pattern with volume.",
                "sha_sell",
                0.001,
            ),
            make_row(
                "BUY",
                0.7,
                "Another bullish signal with MACD.",
                "sha_buy2",
                0.001,
            ),
        ];
        for i in 0..97 {
            rows.push(make_row(
                "HOLD",
                0.5,
                "No clear directional edge at this time.",
                &format!("hold_sha_{i}"),
                0.001,
            ));
        }
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l1-test".to_string());
        assert!(
            stats.hold_frac() >= 0.95,
            "hold_frac should be >= 0.95, got {}",
            stats.hold_frac()
        );
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L1 { .. }),
            "expected L1, got {}",
            v.label()
        );
    }

    #[test]
    fn l1_does_not_fire_below_threshold() {
        // 94 HOLD out of 100 → hold_frac = 0.94 < 0.95 → NOT L1
        let mut rows = vec![
            make_row("BUY", 0.7, "Bullish signal confirmed.", "sha_buy1", 0.001),
            make_row("SELL", 0.6, "Bearish reversal pattern.", "sha_sell1", 0.001),
            make_row(
                "STRONG_BUY",
                0.9,
                "Very strong momentum indicator.",
                "sha_sbuy1",
                0.001,
            ),
            make_row(
                "STRONG_SELL",
                0.8,
                "Strong sell signal across indicators.",
                "sha_ssell1",
                0.001,
            ),
            make_row("BUY", 0.7, "MACD crossover with volume.", "sha_buy2", 0.001),
            make_row("SELL", 0.6, "Divergence confirmed.", "sha_sell2", 0.001),
        ];
        for i in 0..94 {
            rows.push(make_row(
                "HOLD",
                0.5,
                "No clear directional signal visible.",
                &format!("hold_sha_{i}"),
                0.001,
            ));
        }
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l1-negative".to_string());
        assert!(
            stats.hold_frac() < 0.95,
            "hold_frac should be < 0.95, got {}",
            stats.hold_frac()
        );
        // With confidence_outcome_corr = 0.15 (≥ 0.05), L2 won't fire.
        // L3/L4 also clear. → L0 PASS.
        let v = classify_l(&stats);
        assert!(
            !matches!(v, LVerdict::L1 { .. }),
            "L1 should NOT fire with hold_frac < 0.95, got {}",
            v.label()
        );
    }

    // ── L2 calibration failure ────────────────────────────────────────────────

    #[test]
    fn l2_fires_on_zero_correlation() {
        let rows = healthy_rows();
        // Pass confidence_outcome_corr = 0.0 → |0.0| < 0.05 → L2
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.0, "l2-test".to_string());
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L2 { .. }),
            "expected L2 with zero correlation, got {}",
            v.label()
        );
    }

    #[test]
    fn l2_does_not_fire_when_correlation_above_threshold() {
        let rows = healthy_rows();
        // confidence_outcome_corr = 0.10 → |0.10| ≥ 0.05 → NOT L2
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.10, "l2-negative".to_string());
        // hold_frac < 0.95 (healthy rows) → L1 won't fire
        assert!(stats.hold_frac() < 0.95);
        let v = classify_l(&stats);
        assert!(
            !matches!(v, LVerdict::L2 { .. }),
            "L2 should NOT fire with |corr| >= 0.05, got {}",
            v.label()
        );
    }

    // ── L3 cost overrun ───────────────────────────────────────────────────────

    #[test]
    fn l3_fires_on_overrun_ratio() {
        let rows = healthy_rows(); // 100 rows × $0.001 = $0.10 cost
        // projected = $0.04 → overrun = 0.10/0.04 = 2.5 > 2.0 → L3
        let stats = aggregate_rows(&rows, 0.04, 100.0, 0.15, "l3-overrun".to_string());
        assert!(
            stats.overrun_ratio() > 2.0,
            "overrun_ratio should > 2.0, got {}",
            stats.overrun_ratio()
        );
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L3 { .. }),
            "expected L3 on overrun ratio, got {}",
            v.label()
        );
    }

    #[test]
    fn l3_fires_on_cap_exceeded() {
        // cost_actual = $0.10, cost_cap = $0.05 → exceeds cap → L3
        let rows = healthy_rows(); // 100 × $0.001 = $0.10
        let stats = aggregate_rows(&rows, 0.50, 0.05, 0.15, "l3-cap".to_string());
        assert!(
            stats.cost_actual_usd > stats.cost_cap_usd,
            "cost_actual should exceed cap"
        );
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L3 { .. }),
            "expected L3 on cap exceeded, got {}",
            v.label()
        );
    }

    #[test]
    fn l3_does_not_fire_within_budget() {
        let rows = healthy_rows(); // $0.10 actual
        // projected = $0.10, cap = $1.00 → ratio = 1.0 ≤ 2.0, below cap → NOT L3
        let stats = aggregate_rows(&rows, 0.10, 1.00, 0.15, "l3-negative".to_string());
        let v = classify_l(&stats);
        assert!(
            !matches!(v, LVerdict::L3 { .. }),
            "L3 should NOT fire within budget, got {}",
            v.label()
        );
    }

    // ── L4 reasoning trace degenerate ────────────────────────────────────────

    #[test]
    fn l4_fires_on_short_trace_majority() {
        // > 50% traces shorter than 50 chars
        let rows: Vec<LlmForecastRow> = (0..100)
            .map(|i| {
                let trace = if i < 60 {
                    "short" // 5 chars < 50
                } else {
                    "This is a longer reasoning trace with more than fifty characters total in it."
                };
                make_row("BUY", 0.7, trace, &format!("sha_{i}"), 0.001)
            })
            .collect();
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l4-short".to_string());
        assert!(
            stats.short_frac() > 0.50,
            "short_frac should > 0.50, got {}",
            stats.short_frac()
        );
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L4 { .. }),
            "expected L4 on short traces, got {}",
            v.label()
        );
    }

    #[test]
    fn l4_fires_on_duplicate_trace_majority() {
        // > 50% duplicate traces (same sha)
        let rows: Vec<LlmForecastRow> = (0..100)
            .map(|i| {
                let sha = if i < 60 {
                    "duplicate_sha_same_for_all".to_string() // 60 dups
                } else {
                    format!("unique_sha_{i}")
                };
                make_row(
                    "BUY",
                    0.7,
                    "This is a longer reasoning trace with more than fifty characters total.",
                    &sha,
                    0.001,
                )
            })
            .collect();
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l4-dup".to_string());
        assert!(
            stats.duplicate_frac() > 0.50,
            "duplicate_frac should > 0.50, got {}",
            stats.duplicate_frac()
        );
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L4 { .. }),
            "expected L4 on duplicate traces, got {}",
            v.label()
        );
    }

    #[test]
    fn l4_does_not_fire_on_high_quality_traces() {
        let rows = healthy_rows(); // all traces > 50 chars, all unique shas
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l4-negative".to_string());
        assert!(stats.short_frac() <= 0.50, "short_frac should be <= 0.50");
        assert!(
            stats.duplicate_frac() <= 0.50,
            "duplicate_frac should be <= 0.50"
        );
        let v = classify_l(&stats);
        assert!(
            !matches!(v, LVerdict::L4 { .. }),
            "L4 should NOT fire on high-quality traces, got {}",
            v.label()
        );
    }

    // ── Priority order tests ──────────────────────────────────────────────────

    /// When both L1 and L2 would fire, L1 wins (priority: L1 > L2).
    #[test]
    fn l1_wins_over_l2_when_both_trigger() {
        // 97 HOLDs → L1 trigger; pass correlation=0.0 → L2 also trigger.
        let mut rows = vec![
            make_row(
                "BUY",
                0.5,
                "Bullish signal confirmed by momentum.",
                "sha_b1",
                0.001,
            ),
            make_row(
                "SELL",
                0.5,
                "Bearish divergence pattern confirmed.",
                "sha_s1",
                0.001,
            ),
            make_row(
                "BUY",
                0.5,
                "MACD crossover on high volume bar.",
                "sha_b2",
                0.001,
            ),
        ];
        for i in 0..97 {
            rows.push(make_row(
                "HOLD",
                0.5,
                "No directional edge at this time.",
                &format!("hold_{i}"),
                0.001,
            ));
        }
        // corr = 0.0 → L2 would fire; but hold_frac = 0.97 ≥ 0.95 → L1 fires first.
        let stats = aggregate_rows(&rows, 0.10, 100.0, 0.0, "l1-over-l2".to_string());
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L1 { .. }),
            "L1 should beat L2 in priority, got {}",
            v.label()
        );
    }

    /// When both L2 and L3 would fire, L2 wins (priority: L2 > L3).
    #[test]
    fn l2_wins_over_l3_when_both_trigger() {
        let rows = healthy_rows(); // hold_frac < 0.95 → L1 won't fire
        // corr = 0.0 → L2 fires; overrun > 2.0 → L3 also fires.
        let stats = aggregate_rows(&rows, 0.04, 100.0, 0.0, "l2-over-l3".to_string());
        assert!(stats.hold_frac() < 0.95);
        assert!(stats.overrun_ratio() > 2.0);
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L2 { .. }),
            "L2 should beat L3 in priority, got {}",
            v.label()
        );
    }

    /// When both L3 and L4 would fire, L3 wins (priority: L3 > L4).
    #[test]
    fn l3_wins_over_l4_when_both_trigger() {
        // > 50% short traces → L4 trigger; overrun > 2.0 → L3 also trigger.
        let rows: Vec<LlmForecastRow> = (0..100)
            .map(|i| {
                let trace = if i < 60 {
                    "short"
                } else {
                    "Long trace with sufficient chars for quality assessment yes."
                };
                make_row("BUY", 0.7, trace, &format!("sha_{i}"), 0.001)
            })
            .collect();
        // corr = 0.15 (≥ 0.05 → L2 won't fire); projected = 0.04 → overrun > 2.0.
        let stats = aggregate_rows(&rows, 0.04, 100.0, 0.15, "l3-over-l4".to_string());
        assert!(stats.hold_frac() < 0.95); // L1 won't fire
        assert!(stats.short_frac() > 0.50); // L4 would fire
        assert!(stats.overrun_ratio() > 2.0); // L3 fires first
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L3 { .. }),
            "L3 should beat L4 in priority, got {}",
            v.label()
        );
    }

    // ── Mutual exclusivity ────────────────────────────────────────────────────

    /// Exactly one verdict is returned for all test fixtures.
    #[test]
    fn exactly_one_verdict_per_run() {
        let fixtures: Vec<(Vec<LlmForecastRow>, f64, f64, f64)> = vec![
            // (rows, projected, cap, corr)
            (healthy_rows(), 0.10, 100.0, 0.15), // → L0
            (healthy_rows(), 0.10, 100.0, 0.0),  // → L2
            (healthy_rows(), 0.04, 100.0, 0.15), // → L3
        ];
        for (rows, proj, cap, corr) in fixtures {
            let stats = aggregate_rows(&rows, proj, cap, corr, "mutual-excl".to_string());
            let v = classify_l(&stats);
            let label = v.label();
            assert!(
                ["L0", "L1", "L2", "L3", "L4"].contains(&label),
                "unexpected verdict label: {label}"
            );
        }
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    /// Empty row set: n_calls=0, no L3 trip (cost=0), L2 fires (corr=0.0 default).
    #[test]
    fn empty_rows_aggregate_safely() {
        let stats = aggregate_rows(&[], 0.10, 100.0, 0.0, "empty".to_string());
        assert_eq!(stats.n_calls, 0);
        assert_eq!(stats.hold_frac(), 0.0);
        // With corr=0.0, L2 fires (< 0.05).
        let v = classify_l(&stats);
        assert!(
            matches!(v, LVerdict::L2 { .. }),
            "empty rows + zero corr → L2, got {}",
            v.label()
        );
    }

    /// Zero n_unique_traces forces duplicate_frac = 1.0 - 0/0 → handled safely.
    #[test]
    fn single_row_duplicate_frac_is_zero() {
        let row = make_row(
            "BUY",
            0.7,
            "Strong momentum signal confirmed by all indicators here.",
            "unique_sha",
            0.001,
        );
        let stats = aggregate_rows(&[row], 0.10, 100.0, 0.15, "single-row".to_string());
        // n_unique_traces = 1, n_calls = 1 → duplicate_frac = 0.0
        assert_eq!(stats.duplicate_frac(), 0.0);
    }
}
