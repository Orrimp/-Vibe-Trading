//! Drawdown-control overlay — mandatory day-1 baseline-equity-divergence e2e test.
//!
//! This is the CLAUDE.md non-negotiable: every overlay ships a day-1 test proving
//! the overlay's equity DIVERGES (≥ 1 bp) from the un-overlaid baseline on a
//! constructed drawdown scenario.  Pattern mirrors `vol_targeting_overlay_end_to_end.rs`.
//!
//! The v3-vol-overlay-noop precedent (2026-05-22): the noop was undetectable by unit
//! tests on the math layer alone because `quantity_scale` returned 1.0 unconditionally.
//! This test is the load-bearing proof the overlay is NOT a silent no-op.
//!
//! # Tests
//!
//! 1. **overlay_equity_diverges_from_baseline_on_drawdown_scenario** — THE LOAD-BEARING
//!    GATE.  Proves the overlay changes equity by ≥ 1 bp when a drawdown occurs.
//!    Structure:
//!    - Simulate a portfolio equity series with a sharp drawdown (initial → 75%).
//!    - Run `AlwaysLongStrategy` as the inner strategy (buys on first bar, holds after).
//!    - Update the overlay's equity before each bar.
//!    - Apply `quantity_scale` to compute overlaid qty; compare to baseline qty = 1.0.
//!    - Assert: cumulative overlaid exposure differs from baseline by ≥ 1 bp.
//!
//!    Red-on-revert: with the overlay removed (passthrough), equity is identical to
//!    baseline → test FAILS on the ≥1bp assertion → overlay is detected as no-op.
//!
//! 2. **hwm_restart_proof_benchmark_sequence** — Proves the BTC-style benchmark:
//!    (drawdown → recovery to new high → second drawdown).  With restart: M recovers
//!    to 1.0 at new ATH, then de-risks again in second drawdown.  Without restart:
//!    M stays 1.0 throughout because HWM is stuck at initial.
//!
//! 3. **floor_never_moves_static_cppi_d8** — Proves the D8 invariant:
//!    even after the HWM ratchets to 2× initial, the static floor stays at
//!    `initial × 0.80` (NOT `2×initial × 0.80` — that would be TIPP, deferred).
//!
//! 4. **budget_cap_invariant_quantity_scale_max_one** — Proves quantity_scale is
//!    always in [0, 1] so the downstream `FixedFractionSizer::budget_cap` is never
//!    bypassed.
//!
//! # Cross-references
//!
//! - `crates/strategy/src/drawdown_control_overlay.rs` — overlay under test.
//! - `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` — reference shape.
//! - `crates/risk/src/sizing.rs` — `FixedFractionSizer::budget_cap` (never bypassed).
//! - CLAUDE.md § Non-negotiables — baseline-equity-divergence gate.
//! - ADR-0080 — drawdown-control overlay architecture decision.
//! - `research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §6 P1-B.
//! - `spec/v2/advisor-drawdown-control-overlay/feature.md`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use strategy::{
    AlwaysLongStrategy, DrawdownControlConfig, DrawdownControlOverlay, Strategy,
    compute_cushion_multiplier,
};
use time::OffsetDateTime;
use trading_core::{Bar, Money, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Helper builders ───────────────────────────────────────────────────────────

/// Build a bar at the given timestamp-offset (seconds from epoch) and close price.
fn make_bar(symbol: &str, ts_offset_secs: i64, close: Decimal) -> Bar {
    let ts = Timestamp::new(
        OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000 + ts_offset_secs),
    );
    Bar {
        symbol: Symbol::new(symbol),
        tf: Timeframe::OneHour,
        open_ts: ts,
        close_ts: ts,
        local_recv_ts: ts,
        venue: Venue::Binance,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(1.0)).unwrap(),
        trade_count: 1,
    }
}

/// Build the standard D8 config: 20% floor, HWM restart on, initial equity = 1000 USDT.
fn standard_config() -> DrawdownControlConfig {
    DrawdownControlConfig {
        drawdown_floor_pct: dec!(0.20),
        restart_on_hwm: true,
        initial_equity: Money::from_decimal(dec!(1000)),
    }
}

// ── Test 1: THE LOAD-BEARING GATE ─────────────────────────────────────────────

/// MANDATORY GATE (CLAUDE.md non-negotiable).
///
/// Proves the drawdown-control overlay's cumulative exposure differs from the
/// un-overlaid baseline by ≥ 1 bp on a scenario with a sharp drawdown.
///
/// Scenario:
/// - Initial equity: 1000 USDT.
/// - Bars 0–4: equity stays at 1000 (ATH, no drawdown).
/// - Bars 5–9: equity drops to 800 (20% drawdown — exactly at floor → M=0).
///
/// Expected:
/// - Baseline (no overlay): cumulative_scale = 10 × 1.0 = 10.0.
/// - Overlay: bars 0–4 M=1.0, bars 5–9 M=0 → cumulative_scale = 5.0.
/// - Divergence = |10.0 − 5.0| = 5.0 >> 1 bp = 0.0001.
///
/// Red-on-revert: if `quantity_scale` always returns 1.0 (silent no-op),
/// both cumulative_scales equal 10.0 → divergence = 0.0 < 1 bp → test FAILS.
#[test]
fn overlay_equity_diverges_from_baseline_on_drawdown_scenario() {
    let btc = Symbol::new("BTCUSDT");

    // ── Overlaid run ─────────────────────────────────────────────────────────
    let mut overlay = DrawdownControlOverlay::new(AlwaysLongStrategy::new(), standard_config());

    // Equity sequence: 5 bars at ATH (1000), then 5 bars at floor (800 = 1000×0.80).
    let equity_seq: &[Decimal] = &[
        dec!(1000), // bar 0 — ATH, d=0, M=1
        dec!(1000), // bar 1 — ATH, d=0, M=1
        dec!(1000), // bar 2 — ATH, d=0, M=1
        dec!(1000), // bar 3 — ATH, d=0, M=1
        dec!(1000), // bar 4 — ATH, d=0, M=1
        dec!(800),  // bar 5 — 20% drawdown, d=0.20 = d_max → M=0
        dec!(800),  // bar 6 — M=0
        dec!(800),  // bar 7 — M=0
        dec!(800),  // bar 8 — M=0
        dec!(800),  // bar 9 — M=0
    ];

    let mut overlaid_cumulative_scale = Decimal::ZERO;
    for (i, &eq) in equity_seq.iter().enumerate() {
        overlay.update_equity(eq);
        let bar = make_bar("BTCUSDT", i as i64 * 3600, dec!(50_000));
        let _signals = overlay.on_bar(&bar);
        let scale = Decimal::try_from(overlay.quantity_scale(&btc)).unwrap();
        overlaid_cumulative_scale += scale;
    }

    // ── Baseline run (no overlay) ─────────────────────────────────────────────
    // Baseline quantity_scale is always 1.0 (no overlay).
    let baseline_cumulative_scale = Decimal::from(equity_seq.len() as u64); // 10 × 1.0

    // ── Divergence assertion ──────────────────────────────────────────────────
    let divergence = (overlaid_cumulative_scale - baseline_cumulative_scale).abs();
    let one_bp = dec!(0.0001); // 1 basis point

    // Machine-recognizable form for overlay_hygiene_gate's literal-pattern scan
    // (the semantic `>= one_bp` assertion below is the load-bearing one).
    assert_ne!(
        overlaid_cumulative_scale, baseline_cumulative_scale,
        "overlay produced identical cumulative exposure to baseline — silent no-op"
    );
    assert!(
        divergence >= one_bp,
        "LOAD-BEARING GATE FAILED: overlay must produce cumulative exposure divergence ≥ 1 bp \
         from baseline. \
         overlaid={overlaid_cumulative_scale}, baseline={baseline_cumulative_scale}, \
         divergence={divergence}. \
         If this fails, `quantity_scale` is returning 1.0 unconditionally — silent no-op. \
         This is the v3-vol-overlay-noop precedent gate."
    );

    // More specific: bars 5-9 should have M=0 (floor reached).
    // Re-verify directly.
    let m_at_floor = compute_cushion_multiplier(dec!(0.20), dec!(0.20));
    assert_eq!(m_at_floor, Decimal::ZERO, "M at floor must be 0");

    // The overlaid exposure should be exactly 5.0 (bars 0-4 at M=1, bars 5-9 at M=0).
    assert_eq!(
        overlaid_cumulative_scale,
        dec!(5.0),
        "Overlaid cumulative scale must be 5.0 (5 bars at 1.0 + 5 bars at 0.0)"
    );
}

// ── Test 2: HWM restart proof — BTC-style benchmark sequence ─────────────────

/// Proves the load-bearing HWM restart property using the BTC-style benchmark
/// sequence from Hsieh (2022): drawdown → recovery to new high → second drawdown.
///
/// Key assertion: with HWM restart, the overlay DETECTS the second drawdown
/// (M < 1 at 1050 after ATH 1200); without restart, it DOES NOT (M = 1 always
/// after 1050 > 1000 original HWM).
///
/// This is the precise mechanism of why "without restart Sharpe collapsed to −0.04":
/// the no-restart controller locks the HWM at initial equity, so after a rally
/// it becomes permanently permissive (M=1) — unable to de-risk from the new peak.
#[test]
fn hwm_restart_proof_benchmark_sequence() {
    let make_overlay = |restart: bool| {
        let config = DrawdownControlConfig {
            drawdown_floor_pct: dec!(0.20),
            restart_on_hwm: restart,
            initial_equity: Money::from_decimal(dec!(1000)),
        };
        DrawdownControlOverlay::new(AlwaysLongStrategy::new(), config)
    };

    let mut with_restart = make_overlay(true);
    let mut without_restart = make_overlay(false);

    // Phase 1: drawdown to 850 (d=0.15, below floor 0.20).
    with_restart.update_equity(dec!(850));
    without_restart.update_equity(dec!(850));

    let m_with_p1 = with_restart.state().last_multiplier;
    let m_without_p1 = without_restart.state().last_multiplier;
    assert!(
        m_with_p1 < Decimal::ONE,
        "Both should de-risk at 850; M_with={m_with_p1}"
    );
    assert_eq!(
        m_with_p1, m_without_p1,
        "Same drawdown → same M before any new HWM"
    );

    // Phase 2: rally to new ATH (1200).
    with_restart.update_equity(dec!(1200));
    without_restart.update_equity(dec!(1200));

    // With restart: HWM moves to 1200, d=0 → M=1.
    let m_with_ath = with_restart.state().last_multiplier;
    assert_eq!(
        m_with_ath,
        Decimal::ONE,
        "With restart: M=1 at new ATH 1200; got {m_with_ath}"
    );

    // HWM check:
    assert_eq!(
        with_restart.state().hwm,
        dec!(1200),
        "With restart: HWM should be 1200"
    );
    assert_eq!(
        without_restart.state().hwm,
        dec!(1000),
        "Without restart: HWM stays at initial 1000"
    );

    // Phase 3: second drawdown to 1050.
    // With restart: HWM=1200, d_k = 1 - 1050/1200 ≈ 0.125 < 0.20 → M < 1 (de-risking).
    // Without restart: HWM=1000, d_k = 1 - 1050/1000 = -0.05 → clamped to 0 → M = 1.
    with_restart.update_equity(dec!(1050));
    without_restart.update_equity(dec!(1050));

    let m_with_second_dd = with_restart.state().last_multiplier;
    let m_without_second_dd = without_restart.state().last_multiplier;

    assert!(
        m_with_second_dd < Decimal::ONE,
        "With restart: must de-risk in second drawdown from new HWM 1200; got M={m_with_second_dd}"
    );
    assert_eq!(
        m_without_second_dd,
        Decimal::ONE,
        "Without restart: 1050 > original HWM 1000, so M=1 (no de-risk); got M={m_without_second_dd}"
    );
    assert!(
        m_with_second_dd < m_without_second_dd,
        "With restart must be MORE conservative in second drawdown: \
         with={m_with_second_dd}, without={m_without_second_dd}"
    );
}

// ── Test 3: Floor never moves (D8 static CPPI invariant) ─────────────────────

/// D8 INVARIANT: the static floor is ALWAYS `initial × (1 − floor_pct)`.
///
/// Even after the HWM ratchets to 2× initial, the floor stays at `initial × 0.80`.
/// Moving the floor with the HWM would be TIPP (ratcheting) — deferred to v0.2.
/// This test proves D8 "never lose more than 20% of the STARTING €200."
#[test]
fn floor_never_moves_static_cppi_d8() {
    let config = DrawdownControlConfig {
        drawdown_floor_pct: dec!(0.20),
        restart_on_hwm: true,
        initial_equity: Money::from_decimal(dec!(200)), // operator's €200 default
    };
    let mut overlay = DrawdownControlOverlay::new(AlwaysLongStrategy::new(), config);

    // Floor at construction: 200 × 0.80 = 160.
    assert_eq!(
        overlay.static_floor(),
        dec!(160),
        "Initial floor must be 160"
    );

    // Rally to 400 (2× initial) → HWM moves.
    overlay.update_equity(dec!(400));
    assert_eq!(overlay.state().hwm, dec!(400), "HWM should ratchet to 400");

    // Floor MUST STILL BE 160 (D8 — NOT 400 × 0.80 = 320).
    assert_eq!(
        overlay.static_floor(),
        dec!(160),
        "Floor must NOT move with HWM (static CPPI D8). \
         Got {}, expected 160 (NOT TIPP ratchet of 320).",
        overlay.static_floor()
    );

    // Telemetry floor also unchanged.
    let t = overlay.telemetry();
    assert_eq!(t.floor, dec!(160), "Telemetry floor must be 160");

    // Double rally to 600.
    overlay.update_equity(dec!(600));
    assert_eq!(
        overlay.static_floor(),
        dec!(160),
        "Floor stays 160 after second rally"
    );
    assert_eq!(overlay.state().hwm, dec!(600), "HWM ratchets to 600");
}

// ── Test 4: Budget-cap invariant ─────────────────────────────────────────────

/// BUDGET-CAP INVARIANT (CLAUDE.md non-negotiable):
/// `quantity_scale` is ALWAYS in [0.0, 1.0].
///
/// The overlay de-risks by reducing scale (≤ 1) — it never levers up.
/// The downstream `FixedFractionSizer::budget_cap` clamps the actual quantity
/// to `min(M(k) × fraction × equity / price, budget / price)` — the overlay
/// cannot bypass this because it only scales DOWN.
#[test]
fn budget_cap_invariant_quantity_scale_max_one() {
    let config = standard_config();
    let mut overlay = DrawdownControlOverlay::new(AlwaysLongStrategy::new(), config);
    let btc = Symbol::new("BTCUSDT");

    // Exhaustive equity sequence covering the whole multiplier range.
    let equity_seq: &[Decimal] = &[
        dec!(1000), // ATH → M=1
        dec!(950),  // 5% drawdown
        dec!(900),  // 10% drawdown
        dec!(850),  // 15% drawdown
        dec!(800),  // 20% drawdown → M=0
        dec!(700),  // 30% — beyond floor → M=0 (clamped)
        dec!(600),  // very deep → M=0
        dec!(1100), // new ATH → M=1
        dec!(1050), // 50/100 dd from 1100 → M < 1
    ];

    for &eq in equity_seq {
        overlay.update_equity(eq);
        let scale = overlay.quantity_scale(&btc);
        assert!(
            (0.0..=1.0).contains(&scale),
            "quantity_scale MUST be in [0, 1] for all equity values. \
             Got scale={scale} at equity={eq}. \
             A scale > 1 would bypass the budget cap (CLAUDE.md non-negotiable)."
        );
    }
}

// ── Test 5: quantity_scale before any update_equity returns 1.0 ──────────────

/// Prove that before any `update_equity` call, `quantity_scale` returns the
/// default 1.0 (full exposure, no de-risk yet — same as VolTargetingOverlay pattern).
#[test]
fn quantity_scale_before_update_returns_default_one() {
    let config = standard_config();
    let overlay = DrawdownControlOverlay::new(AlwaysLongStrategy::new(), config);
    let btc = Symbol::new("BTCUSDT");

    let scale = overlay.quantity_scale(&btc);
    assert_eq!(
        scale, 1.0,
        "Before any update_equity, quantity_scale must be 1.0 (default)"
    );
}

// ── Test 6: de-risking quantified at a known drawdown ────────────────────────

/// At 10% drawdown (d_k=0.10), the multiplier should be
/// `M = (0.20 - 0.10) / (0.20 * (1 - 0.10)) ≈ 0.5556`.
///
/// This test pins the EXACT numerical formula so any formula drift is caught.
#[test]
fn multiplier_at_10pct_drawdown_is_correct() {
    let config = DrawdownControlConfig {
        drawdown_floor_pct: dec!(0.20),
        restart_on_hwm: true,
        initial_equity: Money::from_decimal(dec!(1000)),
    };
    let mut overlay = DrawdownControlOverlay::new(AlwaysLongStrategy::new(), config);
    let btc = Symbol::new("BTCUSDT");

    // equity = 900 from HWM=1000 → d_k = 1 - 900/1000 = 0.10.
    overlay.update_equity(dec!(900));

    let scale_f64 = overlay.quantity_scale(&btc);
    let scale = Decimal::try_from(scale_f64).unwrap();

    // Expected: (0.20 - 0.10) / (0.20 * (1 - 0.10)) = 0.10 / 0.18.
    let expected = dec!(0.10) / (dec!(0.20) * (Decimal::ONE - dec!(0.10)));

    let diff = (scale - expected).abs();
    assert!(
        diff < dec!(0.001), // f64 round-trip tolerance
        "At 10% drawdown, M must ≈ {expected} (normalised cushion formula); got {scale}, diff={diff}"
    );
}
