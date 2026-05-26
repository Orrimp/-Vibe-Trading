//! Integration tests for the L0-L4 verdict priority tree (ADR-0039 § D1).
//!
//! ## Coverage contract
//!
//! Per ADR-0039 § D1.b + Wave G spec:
//! - Each L1..L4 trigger: at least one positive (fires) + one negative (doesn't fire).
//! - L0 PASS fixture.
//! - Priority order: when L1 + L2 both trigger, L1 wins.
//! - Priority order: when L2 + L3 both trigger, L2 wins.
//! - Priority order: when L3 + L4 both trigger, L3 wins.
//! - Mutual exclusivity: exactly one verdict per run.
//! - 2-run byte-identity on the L-verdict report body.
//!
//! ## Test naming convention
//!
//! `<verdict>_<positive|negative>_<scenario>` e.g. `l1_positive_hold_dominated`.

use trader::llm_forecaster::verdict::{LVerdict, LlmForecastRow, aggregate_rows, classify_l};

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn make_row(rating: &str, confidence: f64, trace: &str, sha: &str, cost: f64) -> LlmForecastRow {
    LlmForecastRow {
        rating: rating.to_string(),
        confidence_f64: confidence,
        reasoning_trace: trace.to_string(),
        trace_sha256: sha.to_string(),
        cost_usd_f64: cost,
    }
}

/// 100 rows with healthy ratings, long unique traces, low cost.
fn healthy_fixture() -> Vec<LlmForecastRow> {
    let traces = [
        "Strong bullish momentum confirmed by RSI above 60 and MACD positive crossover.",
        "Bearish divergence detected in MACD histogram; volume declining on recent up-bars.",
        "Neutral stance: price within Bollinger bands with RSI at 48, no directional edge.",
        "Strong buy signal: ATR expansion with confirmed breakout above prior resistance.",
        "Sell signal: MACD histogram turning negative with volume spike on down bar.",
    ];
    (0..100)
        .map(|i| {
            let rating = ["BUY", "SELL", "HOLD", "STRONG_BUY", "STRONG_SELL"][i % 5];
            let trace = traces[i % 5];
            make_row(rating, 0.7, trace, &format!("sha_{i:04x}"), 0.001)
        })
        .collect()
}

// ── L0 PASS ───────────────────────────────────────────────────────────────────

/// L0 fires on healthy fixture with calibrated correlation.
#[test]
fn l0_pass_healthy_fixture() {
    let rows = healthy_fixture();
    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l0-pass".to_string());
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L0 { .. }),
        "healthy fixture should yield L0 PASS, got {}",
        v.label()
    );
    assert!(v.is_pass(), "L0 must satisfy is_pass()");
    assert_eq!(v.label(), "L0");
    assert_eq!(v.follow_on(), "l_alpha_strategy_gate");
    assert!(
        v.routes_to().contains("L_ALPHA"),
        "L0 routes_to should reference L_ALPHA"
    );
}

// ── L1 — Bias collapse ────────────────────────────────────────────────────────

/// L1 fires when hold_frac ≥ 0.95 (97 HOLD out of 100).
#[test]
fn l1_positive_hold_dominated() {
    let mut rows: Vec<LlmForecastRow> = (0..97)
        .map(|i| {
            make_row(
                "HOLD",
                0.5,
                "No clear directional edge at this time in the market.",
                &format!("hold_sha_{i}"),
                0.001,
            )
        })
        .collect();
    rows.push(make_row(
        "BUY",
        0.7,
        "Bullish signal detected.",
        "buy_sha_0",
        0.001,
    ));
    rows.push(make_row(
        "SELL",
        0.6,
        "Bearish reversal seen.",
        "sell_sha_0",
        0.001,
    ));
    rows.push(make_row(
        "BUY",
        0.7,
        "MACD crossover confirmed.",
        "buy_sha_1",
        0.001,
    ));

    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l1-pos".to_string());
    // Sanity check: hold_frac ≥ 0.95
    assert!(
        stats.hold_frac() >= 0.95,
        "expected hold_frac >= 0.95, got {:.4}",
        stats.hold_frac()
    );
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L1 { .. }),
        "expected L1, got {}",
        v.label()
    );
    assert_eq!(v.follow_on(), "v3-llm-forecaster-prompt-redesign");
    assert!(
        v.evidence().contains("bias collapse"),
        "evidence should mention bias collapse"
    );
}

/// L1 does NOT fire when hold_frac = 0.94 (< 0.95).
#[test]
fn l1_negative_below_threshold() {
    let mut rows: Vec<LlmForecastRow> = (0..94)
        .map(|i| {
            make_row(
                "HOLD",
                0.5,
                "No clear directional signal present.",
                &format!("hold_{i}"),
                0.001,
            )
        })
        .collect();
    // 6 non-HOLD rows
    for i in 0..6 {
        rows.push(make_row(
            "BUY",
            0.7,
            "Bullish momentum confirmed with RSI and MACD.",
            &format!("buy_{i}"),
            0.001,
        ));
    }
    // With corr=0.15 and healthy costs, L0 or L2 but NOT L1.
    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l1-neg".to_string());
    assert!(
        stats.hold_frac() < 0.95,
        "hold_frac should be < 0.95, got {:.4}",
        stats.hold_frac()
    );
    let v = classify_l(&stats);
    assert!(
        !matches!(v, LVerdict::L1 { .. }),
        "L1 should NOT fire with hold_frac < 0.95, got {}",
        v.label()
    );
}

// ── L2 — Calibration failure ──────────────────────────────────────────────────

/// L2 fires when confidence_outcome_corr = 0.0 (< 0.05).
#[test]
fn l2_positive_zero_correlation() {
    let rows = healthy_fixture(); // hold_frac < 0.95 → L1 won't fire
    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.0, "l2-pos".to_string());
    assert!(stats.hold_frac() < 0.95, "L1 must not fire before L2 test");
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L2 { .. }),
        "expected L2 with zero corr, got {}",
        v.label()
    );
    assert_eq!(v.follow_on(), "v3-llm-forecaster-calibrate-or-retire");
    assert!(
        v.evidence().contains("calibration failure"),
        "evidence should mention calibration failure"
    );
}

/// L2 fires when |corr| = 0.04 (< 0.05, just below threshold).
#[test]
fn l2_positive_near_threshold() {
    let rows = healthy_fixture();
    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.04, "l2-near".to_string());
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L2 { .. }),
        "expected L2 with |corr|=0.04, got {}",
        v.label()
    );
}

/// L2 does NOT fire when |corr| = 0.05 (exactly at threshold → PASS).
#[test]
fn l2_negative_at_threshold() {
    let rows = healthy_fixture();
    // |0.05| = 0.05 is NOT < 0.05 → L2 must NOT fire.
    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.05, "l2-neg-exact".to_string());
    let v = classify_l(&stats);
    assert!(
        !matches!(v, LVerdict::L2 { .. }),
        "L2 should NOT fire at |corr|=0.05 (not strictly less than), got {}",
        v.label()
    );
}

/// L2 does NOT fire when |corr| = 0.20 (well above threshold).
#[test]
fn l2_negative_above_threshold() {
    let rows = healthy_fixture();
    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.20, "l2-neg".to_string());
    // hold_frac < 0.95 and |corr| >= 0.05 → not L1 or L2
    let v = classify_l(&stats);
    assert!(
        !matches!(v, LVerdict::L2 { .. }),
        "L2 should NOT fire with |corr|=0.20, got {}",
        v.label()
    );
}

// ── L3 — Cost overrun ─────────────────────────────────────────────────────────

/// L3 fires when overrun_ratio > 2.0.
#[test]
fn l3_positive_overrun_ratio() {
    // healthy_fixture: 100 rows × $0.001 = $0.10 actual
    // projected = $0.04 → ratio = 0.10/0.04 = 2.5 > 2.0 → L3
    let rows = healthy_fixture();
    let stats = aggregate_rows(&rows, 0.04, 100.0, 0.15, "l3-overrun".to_string());
    assert!(
        stats.overrun_ratio() > 2.0,
        "overrun_ratio should > 2.0, got {:.4}",
        stats.overrun_ratio()
    );
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L3 { .. }),
        "expected L3 on overrun ratio, got {}",
        v.label()
    );
    assert_eq!(v.follow_on(), "v3-llm-forecaster-cost-tune");
    assert!(
        v.evidence().contains("cost_actual_usd"),
        "evidence should mention cost_actual_usd"
    );
}

/// L3 fires when cost_actual > cost_cap (even if overrun_ratio ≤ 2.0).
#[test]
fn l3_positive_cap_exceeded() {
    // $0.10 actual, cap = $0.05 → exceeds cap → L3
    let rows = healthy_fixture();
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

/// L3 does NOT fire when costs are within budget.
#[test]
fn l3_negative_within_budget() {
    // $0.10 actual, projected = $0.10, cap = $1.00 → ratio = 1.0, below cap
    let rows = healthy_fixture();
    let stats = aggregate_rows(&rows, 0.10, 1.00, 0.15, "l3-neg".to_string());
    let v = classify_l(&stats);
    assert!(
        !matches!(v, LVerdict::L3 { .. }),
        "L3 should NOT fire within budget, got {}",
        v.label()
    );
}

// ── L4 — Reasoning trace degenerate ──────────────────────────────────────────

/// L4 fires when short_frac > 0.50 (> 50% traces < 50 chars).
#[test]
fn l4_positive_short_trace_majority() {
    // 60 short traces (5 chars each) out of 100 → short_frac = 0.60 > 0.50
    let rows: Vec<LlmForecastRow> = (0..100)
        .map(|i| {
            let trace = if i < 60 {
                "short" // 5 chars < 50
            } else {
                "This is a long reasoning trace with many characters beyond fifty total."
            };
            make_row("BUY", 0.7, trace, &format!("sha_{i}"), 0.001)
        })
        .collect();

    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l4-short".to_string());
    assert!(
        stats.short_frac() > 0.50,
        "short_frac should > 0.50, got {:.4}",
        stats.short_frac()
    );
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L4 { .. }),
        "expected L4 on short traces, got {}",
        v.label()
    );
    assert_eq!(v.follow_on(), "v3-llm-forecaster-trace-quality-tune");
    assert!(
        v.evidence().contains("short_frac"),
        "evidence should mention short_frac"
    );
}

/// L4 fires when duplicate_frac > 0.50 (> 50% traces share the same sha).
#[test]
fn l4_positive_duplicate_trace_majority() {
    // 60 rows share "dup_sha", 40 rows have unique shas
    // → n_unique_traces = 41, n_calls = 100
    // → duplicate_frac = 1 - 41/100 = 0.59 > 0.50
    let rows: Vec<LlmForecastRow> = (0..100)
        .map(|i| {
            let sha = if i < 60 {
                "dup_sha_all_identical".to_string()
            } else {
                format!("unique_sha_{i}")
            };
            make_row(
                "SELL",
                0.6,
                "Bearish signal confirmed by volume and momentum indicators here.",
                &sha,
                0.001,
            )
        })
        .collect();

    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l4-dup".to_string());
    assert!(
        stats.duplicate_frac() > 0.50,
        "duplicate_frac should > 0.50, got {:.4}",
        stats.duplicate_frac()
    );
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L4 { .. }),
        "expected L4 on duplicate traces, got {}",
        v.label()
    );
    assert!(
        v.evidence().contains("duplicate_frac"),
        "evidence should mention duplicate_frac"
    );
}

/// L4 does NOT fire on high-quality traces (all long, all unique).
#[test]
fn l4_negative_high_quality_traces() {
    let rows = healthy_fixture(); // all > 70 chars, all unique shas
    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.15, "l4-neg".to_string());
    assert!(stats.short_frac() <= 0.50, "short_frac should <= 0.50");
    assert!(
        stats.duplicate_frac() <= 0.50,
        "duplicate_frac should <= 0.50"
    );
    let v = classify_l(&stats);
    assert!(
        !matches!(v, LVerdict::L4 { .. }),
        "L4 should NOT fire on high-quality traces, got {}",
        v.label()
    );
}

// ── Priority order ────────────────────────────────────────────────────────────

/// When L1 and L2 both trigger, L1 wins (priority: L1 > L2 > L3 > L4 > L0).
#[test]
fn priority_l1_beats_l2() {
    // 97 HOLDs → hold_frac = 0.97 ≥ 0.95 → L1
    // corr = 0.0 → |0.0| < 0.05 → L2 also fires
    let mut rows: Vec<LlmForecastRow> = (0..97)
        .map(|i| {
            make_row(
                "HOLD",
                0.5,
                "No directional edge visible in current market.",
                &format!("hold_{i}"),
                0.001,
            )
        })
        .collect();
    rows.push(make_row(
        "BUY",
        0.5,
        "Momentum signal with MACD.",
        "buy0",
        0.001,
    ));
    rows.push(make_row(
        "SELL",
        0.5,
        "Reversal pattern detected.",
        "sell0",
        0.001,
    ));
    rows.push(make_row(
        "BUY",
        0.5,
        "Volume breakout confirmed.",
        "buy1",
        0.001,
    ));

    let stats = aggregate_rows(&rows, 0.10, 100.0, 0.0, "l1-beats-l2".to_string());
    // Verify both conditions trigger.
    assert!(stats.hold_frac() >= 0.95, "L1 condition: hold_frac >= 0.95");
    assert!(stats.confidence_outcome_corr.abs() < 0.05, "L2 condition");
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L1 { .. }),
        "L1 must beat L2 in priority, got {}",
        v.label()
    );
}

/// When L2 and L3 both trigger, L2 wins.
#[test]
fn priority_l2_beats_l3() {
    // healthy fixture: hold_frac < 0.95 → L1 won't fire
    // corr = 0.0 → L2 fires
    // overrun > 2.0 → L3 fires
    let rows = healthy_fixture();
    let stats = aggregate_rows(&rows, 0.04, 100.0, 0.0, "l2-beats-l3".to_string());
    assert!(stats.hold_frac() < 0.95, "L1 must not fire");
    assert!(stats.confidence_outcome_corr.abs() < 0.05, "L2 condition");
    assert!(stats.overrun_ratio() > 2.0, "L3 condition");
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L2 { .. }),
        "L2 must beat L3 in priority, got {}",
        v.label()
    );
}

/// When L3 and L4 both trigger, L3 wins.
#[test]
fn priority_l3_beats_l4() {
    // 60 short traces → L4 would fire (short_frac > 0.50)
    // overrun > 2.0 → L3 also fires
    // corr = 0.15 (≥ 0.05 → L2 won't fire); hold_frac < 0.95 → L1 won't fire
    let rows: Vec<LlmForecastRow> = (0..100)
        .map(|i| {
            let trace = if i < 60 {
                "short"
            } else {
                "A long enough reasoning trace to exceed fifty characters total."
            };
            make_row("BUY", 0.7, trace, &format!("sha_{i}"), 0.001)
        })
        .collect();
    let stats = aggregate_rows(&rows, 0.04, 100.0, 0.15, "l3-beats-l4".to_string());
    assert!(stats.hold_frac() < 0.95, "L1 must not fire");
    assert!(
        stats.confidence_outcome_corr.abs() >= 0.05,
        "L2 must not fire"
    );
    assert!(stats.overrun_ratio() > 2.0, "L3 condition");
    assert!(stats.short_frac() > 0.50, "L4 condition");
    let v = classify_l(&stats);
    assert!(
        matches!(v, LVerdict::L3 { .. }),
        "L3 must beat L4 in priority, got {}",
        v.label()
    );
}

// ── Mutual exclusivity ────────────────────────────────────────────────────────

/// Exactly one verdict is returned for every fixture combination.
///
/// This is the core mutual-exclusivity invariant from ADR-0039 § D1.b.
#[test]
fn mutual_exclusivity_exactly_one_verdict_per_run() {
    // All distinct expected verdicts (one fixture each).
    let cases: Vec<(Vec<LlmForecastRow>, f64, f64, f64, &str)> = vec![
        // (rows, projected, cap, corr, expected_label)
        (healthy_fixture(), 0.10, 100.0, 0.15, "L0"), // L0 PASS
        // L1: 97 HOLD rows
        (
            {
                let mut rows: Vec<_> = (0..97)
                    .map(|i| {
                        make_row(
                            "HOLD",
                            0.5,
                            "No directional signal.",
                            &format!("h{i}"),
                            0.001,
                        )
                    })
                    .collect();
                rows.push(make_row("BUY", 0.7, "Bullish momentum.", "b0", 0.001));
                rows.push(make_row("SELL", 0.6, "Bearish reversal.", "s0", 0.001));
                rows.push(make_row("BUY", 0.7, "Volume breakout.", "b1", 0.001));
                rows
            },
            0.10,
            100.0,
            0.15,
            "L1",
        ),
        (healthy_fixture(), 0.10, 100.0, 0.0, "L2"), // L2: corr=0
        (healthy_fixture(), 0.04, 100.0, 0.15, "L3"), // L3: overrun
        // L4: 60 short traces
        (
            (0..100)
                .map(|i| {
                    let trace = if i < 60 {
                        "tiny"
                    } else {
                        "Long enough reasoning trace for quality threshold."
                    };
                    make_row("BUY", 0.7, trace, &format!("s{i}"), 0.001)
                })
                .collect(),
            0.10,
            100.0,
            0.15,
            "L4",
        ),
    ];

    for (rows, proj, cap, corr, expected) in cases {
        let stats = aggregate_rows(&rows, proj, cap, corr, "mutex-test".to_string());
        let v = classify_l(&stats);
        assert_eq!(
            v.label(),
            expected,
            "mutual exclusivity violated: expected {expected}, got {}",
            v.label()
        );
    }
}

// ── 2-run byte-identity (determinism contract) ────────────────────────────────

/// Running classify_l twice on identical inputs produces byte-identical output.
///
/// This covers the determinism contract: the evidence string format uses
/// `format!("{:.6}", x)` which must be deterministic across runs.
#[test]
fn two_run_byte_identity_on_report_body() {
    let rows = healthy_fixture();
    let stats1 = aggregate_rows(&rows, 0.10, 100.0, 0.15, "det-test".to_string());
    let stats2 = aggregate_rows(&rows, 0.10, 100.0, 0.15, "det-test".to_string());

    let v1 = classify_l(&stats1);
    let v2 = classify_l(&stats2);

    assert_eq!(
        v1.label(),
        v2.label(),
        "verdict label must be identical across runs"
    );
    assert_eq!(
        v1.evidence(),
        v2.evidence(),
        "verdict evidence string must be byte-identical across runs"
    );
    assert_eq!(
        v1.follow_on(),
        v2.follow_on(),
        "verdict follow_on must be identical across runs"
    );

    // For L2 (corr = 0.0).
    let stats_l2_a = aggregate_rows(&rows, 0.10, 100.0, 0.0, "det-l2".to_string());
    let stats_l2_b = aggregate_rows(&rows, 0.10, 100.0, 0.0, "det-l2".to_string());
    let v_l2_a = classify_l(&stats_l2_a);
    let v_l2_b = classify_l(&stats_l2_b);
    assert_eq!(
        v_l2_a.evidence(),
        v_l2_b.evidence(),
        "L2 evidence must be byte-identical across runs"
    );
}

// ── LVerdict helper methods ───────────────────────────────────────────────────

/// `is_pass()` returns true only for L0.
#[test]
fn is_pass_only_for_l0() {
    let rows = healthy_fixture();

    // L0
    let stats_l0 = aggregate_rows(&rows, 0.10, 100.0, 0.15, "is-pass-l0".to_string());
    assert!(classify_l(&stats_l0).is_pass(), "L0 should be pass");

    // L2 (corr=0)
    let stats_l2 = aggregate_rows(&rows, 0.10, 100.0, 0.0, "is-pass-l2".to_string());
    assert!(!classify_l(&stats_l2).is_pass(), "L2 should not be pass");

    // L3 (overrun)
    let stats_l3 = aggregate_rows(&rows, 0.04, 100.0, 0.15, "is-pass-l3".to_string());
    assert!(!classify_l(&stats_l3).is_pass(), "L3 should not be pass");
}

/// `routes_to()` returns the L_ALPHA gate string for L0 and the `follow_on`
/// slug for L1-L4.
#[test]
fn routes_to_correct_for_all_verdicts() {
    let rows = healthy_fixture();

    let v_l0 = classify_l(&aggregate_rows(
        &rows,
        0.10,
        100.0,
        0.15,
        "rt-l0".to_string(),
    ));
    assert!(
        v_l0.routes_to().contains("L_ALPHA"),
        "L0 should route to L_ALPHA"
    );

    let v_l2 = classify_l(&aggregate_rows(
        &rows,
        0.10,
        100.0,
        0.0,
        "rt-l2".to_string(),
    ));
    assert_eq!(v_l2.routes_to(), "v3-llm-forecaster-calibrate-or-retire");

    let v_l3 = classify_l(&aggregate_rows(
        &rows,
        0.04,
        100.0,
        0.15,
        "rt-l3".to_string(),
    ));
    assert_eq!(v_l3.routes_to(), "v3-llm-forecaster-cost-tune");
}
