//! Test-only fixture builders for the ui-test-harness-bootstrap v0.1
//! visual-snapshot suite.
//!
//! ## Why a tests-tree module?
//!
//! Operator-locked Q9 (see
//! `spec/ui-test-harness-bootstrap/feature.md`) requires a richer
//! Charts-screen scene that includes a hovered marker
//! (`Cockpit.chart_tooltip = Some(ChartTooltipView{...})`). The
//! existing production `ui::fixtures` builders don't carry that state
//! because the cockpit binary itself never pre-populates a tooltip —
//! the tooltip is canvas-driven at runtime. Authoring this fixture in
//! `ui::fixtures` would expand the production-reachable fixture
//! surface for a test-only need; per Q6 we instead keep it in the
//! tests tree.
//!
//! ## Include from an integration test
//!
//! ```ignore
//! #[path = "fixtures/mod.rs"]
//! mod fixtures;
//!
//! let cockpit = fixtures::charts_screen_with_hovered_marker();
//! ```

// Clippy: tests can use expect / unwrap for fixture construction.
// The Phase D+ and Phase E snapshot fn names use double-underscore
// separators that match baseline PNG filenames exactly. Suppressing
// the non_snake_case lint is the lowest-noise approach — renaming
// would de-sync fn names from baselines and confuse the operator.
#![allow(clippy::expect_used, clippy::unwrap_used, dead_code, non_snake_case)]

pub mod visual_diff;

use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Quantity, Side, SignalView, StrategyId, Symbol, Timestamp, Venue};

use ui::state::{ChartTooltipKind, ChartTooltipView, Cockpit, PanelState};

/// Construct the Q9 hovered-marker Charts-screen fixture.
///
/// Builds on top of [`ui::test_support::charts_screen_cockpit`] (which
/// seeds the same scene the cockpit binary's `App::boot` would render)
/// and additionally pre-populates `cockpit.chart_tooltip` against the
/// first fill marker so the canvas tooltip card paints on first frame
/// — no live cursor input required.
///
/// The signal layer is seeded with two ghost signals (Buy + clamped
/// Sell) so the chart's full layer stack is exercised by the snapshot.
/// Position-mirror data is left to `test_support::charts_screen_cockpit`'s
/// pairs-steady-state seed; we don't override it here.
#[must_use]
pub fn charts_screen_with_hovered_marker() -> Cockpit {
    let mut cockpit = ui::test_support::charts_screen_cockpit();

    // Seed the ghost-signal layer (R5.4) so the visual snapshot
    // captures both marker kinds. Timestamps deliberately use the
    // same fixed `0` offset that `synthetic_candles` anchors to, so
    // the snapshot has a stable position for these markers.
    let sig_ts = Timestamp::new(
        time::OffsetDateTime::from_unix_timestamp(1_705_320_000)
            .expect("static unix timestamp must parse"),
    );
    cockpit.chart_signals = PanelState::Ready(vec![
        SignalView {
            signal_id: SmolStr::new("sig-1"),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Buy,
            intended_qty: Quantity::new(dec!(0.05)).expect("fixture qty must be > 0"),
            signal_ts: sig_ts,
            strategy_id: StrategyId::new("sma_crossover"),
            was_clamped: false,
            clamp_reason: None,
        },
        SignalView {
            signal_id: SmolStr::new("sig-2"),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Sell,
            intended_qty: Quantity::new(dec!(0.04)).expect("fixture qty must be > 0"),
            signal_ts: sig_ts,
            strategy_id: StrategyId::new("sma_crossover"),
            was_clamped: true,
            clamp_reason: Some(SmolStr::new("per_symbol_cap")),
        },
    ]);

    // Seed the hovered-marker tooltip against the first fill (Q9
    // operator lock). Mirrors `state::build_tooltip_view` for a
    // Fill(0) hover: same six fields, derived from the fill the
    // fixture seeded via `synthetic_fills_for`.
    if let PanelState::Ready(fills) = &cockpit.chart_markers
        && let Some(first) = fills.first()
    {
        let price = first.price.get();
        let qty = first.qty.get();
        cockpit.chart_tooltip = Some(ChartTooltipView {
            kind: ChartTooltipKind::Fill,
            side: first.side,
            price: Some(price),
            qty,
            notional: Some(price.saturating_mul(qty)),
            ts: first.venue_ts,
            strategy_id: None,
            was_clamped: false,
            clamp_reason: None,
        });
    }

    // Defence-in-depth: BTC universe + Binance is the architect's
    // committed default for the floor / typical / operator
    // baselines. The factory already sets these, but assert here
    // so a future refactor of `charts_screen_cockpit` doesn't
    // silently shift the baseline scene.
    debug_assert_eq!(
        cockpit.selected_symbol,
        Some((Venue::Binance, Symbol::new("BTCUSDT"))),
        "Q10 baselines expect BTC selected"
    );
    // Notional should land at 0.1 * price_first_fill — sanity-check
    // the cockpit hasn't been re-seeded with empty fills.
    debug_assert!(
        cockpit.chart_tooltip.is_some(),
        "Q9 hovered-marker fixture must set chart_tooltip"
    );

    cockpit
}

// ── Phase D+ snapshot fixtures (ui-rethink-phase-d-trail-followup Wave C) ─────

/// Construct the `trail__steady_state` fixture: Trail screen in list mode
/// (byte-identical to `audit::view` per R2.2).
///
/// The cockpit is set to `Screen::Trail` with `selected_audit_id = None`
/// so the trail screen delegates to `screens::audit::view` (list mode).
/// Seeded with 5 journal rows so the audit table renders `PanelState::Ready`
/// (avoids the `ThrottledSpinner` whose frame counter is non-deterministic
/// across consecutive `iced_test::screenshot` calls).
#[must_use]
pub fn trail_steady_state_cockpit() -> Cockpit {
    use ui::state::{AuditScreenState, Screen};
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Trail;
    // Ensure list mode (no row selected).
    cockpit.trail_screen_state = Default::default();
    // Seed Ready rows — prevents the loading spinner (non-deterministic
    // frame position) from appearing and invalidating the baseline.
    cockpit.audit_screen_state = AuditScreenState {
        rows: PanelState::Ready(ui::fixtures::fake_journal_rows(5)),
        total_count: Some(5),
        ..Default::default()
    };
    cockpit
}

/// Construct the `trail__side_drawer_open` fixture: Trail screen in trail
/// mode with a Forecast-stage payload and the side-drawer open.
///
/// Uses a deterministic `ReconstructedTrailUi` fixture (fixed-seed strings).
/// Drawer is open to `TrailNodeKind::Forecast`.
#[must_use]
pub fn trail_side_drawer_open_cockpit() -> Cockpit {
    use smol_str::SmolStr;
    use ui::state::{ReconstructedTrailUi, Screen, TrailScreenState, TrailStageUi};
    use ui::widgets::trail_node::{TrailNode, TrailNodeKind};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Trail;

    // Build a deterministic reconstructed trail fixture.
    let fill_ts = SmolStr::new("12:34:56.789");
    let sig_ts = SmolStr::new("12:34:55.123");
    let fc_ts = SmolStr::new("12:34:54.001");

    let fill = TrailStageUi {
        timestamp: Some(fill_ts.to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy 0.05 BTCUSDT @ 42000.00".to_string()),
        raw_payload: Some(r#"{"fill_id":"abc123","qty":0.05}"#.to_string()),
    };
    let signal = TrailStageUi {
        timestamp: Some(sig_ts.to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy signal triggered (SMA crossover)".to_string()),
        raw_payload: Some(r#"{"signal_id":"sig001"}"#.to_string()),
    };
    let forecast = TrailStageUi {
        timestamp: Some(fc_ts.to_string()),
        actor: Some("tcn:abc12345".to_string()),
        headline: Some("Bullish p=0.72 horizon=15m".to_string()),
        raw_payload: Some(r#"{"forecast_id":"fc001","confidence":0.72}"#.to_string()),
    };
    let debate = TrailStageUi::default();

    // Pre-build nodes (upstream-first: Forecast, LlmDebate, Signal, Fill).
    let nodes = vec![
        TrailNode {
            kind: TrailNodeKind::Forecast,
            timestamp: forecast.timestamp.clone(),
            actor: forecast.actor.clone(),
            headline: forecast.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::LlmDebate,
            timestamp: None,
            actor: None,
            headline: None,
        },
        TrailNode {
            kind: TrailNodeKind::Signal,
            timestamp: signal.timestamp.clone(),
            actor: signal.actor.clone(),
            headline: signal.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::Fill,
            timestamp: fill.timestamp.clone(),
            actor: fill.actor.clone(),
            headline: fill.headline.clone(),
        },
    ];

    let trail = ReconstructedTrailUi {
        audit_id: SmolStr::new("fixture-audit-id-001"),
        fill,
        signal,
        forecast,
        debate,
        nodes,
    };

    cockpit.trail_screen_state = TrailScreenState {
        selected_audit_id: Some(SmolStr::new("fixture-audit-id-001")),
        drawer_selected_node: Some(TrailNodeKind::Forecast),
        reconstructed_trail: Some(trail),
        pending_trail_audit_id: None,
    };
    cockpit
}

/// Construct the `live__recent_activity_with_chevron` fixture: Live screen
/// with 5 rows in `agent_feed::ready_body` (the recent-activity tape) and
/// the universal chevron rendered on every row (Phase D R5.1).
///
/// The cockpit is set to `Screen::Live` with 5 fill rows in `tape`.
#[must_use]
pub fn live_recent_activity_with_chevron_cockpit() -> Cockpit {
    use ui::state::Screen;
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Live;
    // 5-row tape — matches the R2.3 fixture spec.
    cockpit.tape =
        ui::state::PanelState::Ready(ui::fixtures::fake_fill_feed(5).into_iter().collect());
    cockpit
}

// ── Phase E snapshot fixtures (ui-rethink-phase-e-compare Wave D) ──────────────

/// Construct the `compare__cold_boot_all_empty` fixture.
///
/// Compare screen in cold-boot state: `compare_screen_state.cache = BTreeMap::new()`.
/// Every legal cell renders the "Run" affordance; every non-universe cell renders
/// the blanked `—`. K7 subtitle is absent (no multi-symbol cells populated yet).
///
/// Two strategies seeded: one BTC-only (`btc_sma`) + one top10 (`top10_momentum`),
/// to exercise both the blanked-cell path (top10 has many pairs the btc-only col
/// doesn't cover) and the run-affordance path.
#[must_use]
pub fn compare__cold_boot_all_empty_cockpit() -> ui::state::Cockpit {
    use smol_str::SmolStr;
    use std::collections::BTreeMap;
    use trading_core::StrategyId;
    use ui::compare::state::{CompareKpiAxis, CompareScreenState};
    use ui::lab::state::{DateRange, Preset};
    use ui::state::{Screen, StrategiesConfig, StrategyConfigEntry};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Compare;

    cockpit.strategies_config = Some(StrategiesConfig {
        strategies: vec![
            StrategyConfigEntry {
                id: StrategyId::new("btc_sma"),
                source_path: SmolStr::new("config/strategies/btc_sma.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: StrategyId::new("top10_momentum"),
                source_path: SmolStr::new("config/strategies/top10_momentum.toml"),
                params: vec![],
            },
        ],
    });

    cockpit.compare_screen_state = CompareScreenState {
        range: DateRange::Preset(Preset::Last90d),
        kpi_axis: CompareKpiAxis::Sharpe,
        cache: BTreeMap::new(),
        last_indexed_at: None,
    };

    cockpit
}

/// Construct the `compare__steady_state_populated` fixture.
///
/// All 24 populated cells filled per the T-T1-2 census (deterministic values
/// with a consistent Sharpe above 0.5 so the positive-Sharpe color path fires).
/// K7 multi-symbol disclaimer subtitle is visible (any top10 cell is_multi_symbol).
#[must_use]
pub fn compare__steady_state_populated_cockpit() -> ui::state::Cockpit {
    use smol_str::SmolStr;
    use std::collections::BTreeMap;
    use trading_core::{StrategyId, Symbol, Venue};
    use ui::compare::state::{CachedCell, CompareKpiAxis, CompareScreenState};
    use ui::lab::state::{DateRange, Preset};
    use ui::state::{Screen, StrategiesConfig, StrategyConfigEntry};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Compare;

    // Six registered strategies (mirrors the H1 census in decomp.md §1.2).
    cockpit.strategies_config = Some(StrategiesConfig {
        strategies: vec![
            StrategyConfigEntry {
                id: StrategyId::new("btc_sma"),
                source_path: SmolStr::new("config/strategies/btc_sma.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: StrategyId::new("btc_macd"),
                source_path: SmolStr::new("config/strategies/btc_macd.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: StrategyId::new("top10_momentum"),
                source_path: SmolStr::new("config/strategies/top10_momentum.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: StrategyId::new("tcn_alpha"),
                source_path: SmolStr::new("config/strategies/tcn_alpha.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: StrategyId::new("pairs_mr"),
                source_path: SmolStr::new("config/strategies/pairs_mr.toml"),
                params: vec![],
            },
        ],
    });

    let range = DateRange::Preset(Preset::Last90d);
    let mut cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> = BTreeMap::new();

    // Helper: insert a cell for a given strategy + symbol.
    let mut insert_cell = |strategy: &str, sym: &str, sharpe: f64, is_multi: bool| {
        let key = (SmolStr::new(strategy), Symbol::new(sym), range.clone());
        cache.insert(
            key,
            CachedCell {
                sharpe,
                total_return_pct: sharpe * 10.0,
                max_drawdown_pct: -sharpe * 3.0,
                trade_count: 42,
                equity_curve_tail: (0..10).map(|i| 100.0 + i as f64 * sharpe).collect(),
                source_report_path: SmolStr::new(format!(
                    "spec/{strategy}/reports/backtest-fixture.md"
                )),
                generated_at: SmolStr::new("2026-04-29T19:51:48Z"),
                is_multi_symbol: is_multi,
            },
        );
    };

    // btc_sma: 1 BTC cell.
    insert_cell("btc_sma", "BTCUSDT", 1.42, false);
    // btc_macd: 1 BTC cell.
    insert_cell("btc_macd", "BTCUSDT", 0.87, false);
    // top10_momentum: 10 cells (multi-symbol).
    let top10 = [
        "XRPUSDT", "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "ADAUSDT", "DOTUSDT", "DOGEUSDT",
        "LINKUSDT", "AVAXUSDT",
    ];
    for (i, sym) in top10.iter().enumerate() {
        insert_cell("top10_momentum", sym, 0.9 + i as f64 * 0.02, true);
    }
    // tcn_alpha: 10 cells (multi-symbol).
    for (i, sym) in top10.iter().enumerate() {
        insert_cell("tcn_alpha", sym, 1.1 + i as f64 * 0.03, true);
    }
    // pairs_mr: 2 cells (BTC + ETH).
    insert_cell("pairs_mr", "BTCUSDT", 0.75, true);
    insert_cell("pairs_mr", "ETHUSDT", 0.68, true);

    let _ = Venue::Binance; // imported for fixture symmetry

    cockpit.compare_screen_state = CompareScreenState {
        range,
        kpi_axis: CompareKpiAxis::Sharpe,
        cache,
        last_indexed_at: None,
    };

    cockpit
}

/// Construct the `compare__empty_cell_run_affordance` fixture.
///
/// 20 of 24 legal cells populated; 4 cells (the last 4 top10 symbols for
/// `top10_momentum`) show the "Run" affordance — exercises the active
/// `ACCENT_500` hairline button path (R2.3).
#[must_use]
pub fn compare__empty_cell_run_affordance_cockpit() -> ui::state::Cockpit {
    use smol_str::SmolStr;
    use std::collections::BTreeMap;
    use trading_core::{StrategyId, Symbol, Venue};
    use ui::compare::state::{CachedCell, CompareKpiAxis, CompareScreenState};
    use ui::lab::state::{DateRange, Preset};
    use ui::state::{Screen, StrategiesConfig, StrategyConfigEntry};

    let _ = Venue::Binance; // imported for fixture symmetry

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Compare;

    cockpit.strategies_config = Some(StrategiesConfig {
        strategies: vec![
            StrategyConfigEntry {
                id: StrategyId::new("btc_sma"),
                source_path: SmolStr::new("config/strategies/btc_sma.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: StrategyId::new("top10_momentum"),
                source_path: SmolStr::new("config/strategies/top10_momentum.toml"),
                params: vec![],
            },
        ],
    });

    let range = DateRange::Preset(Preset::Last90d);
    let mut cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> = BTreeMap::new();

    // btc_sma × BTCUSDT — populated.
    cache.insert(
        (
            SmolStr::new("btc_sma"),
            Symbol::new("BTCUSDT"),
            range.clone(),
        ),
        CachedCell {
            sharpe: 1.42,
            total_return_pct: 14.2,
            max_drawdown_pct: -4.3,
            trade_count: 55,
            equity_curve_tail: vec![100.0, 103.0, 107.0, 111.0, 116.0],
            source_report_path: SmolStr::new("spec/v0.sma/reports/backtest-fixture.md"),
            generated_at: SmolStr::new("2026-04-29T19:51:48Z"),
            is_multi_symbol: false,
        },
    );

    // top10_momentum: first 6 symbols populated, last 4 leave the "Run" affordance.
    let populated_syms = [
        "XRPUSDT", "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "ADAUSDT",
    ];
    for (i, sym) in populated_syms.iter().enumerate() {
        cache.insert(
            (
                SmolStr::new("top10_momentum"),
                Symbol::new(*sym),
                range.clone(),
            ),
            CachedCell {
                sharpe: 0.8 + i as f64 * 0.1,
                total_return_pct: 8.0 + i as f64 * 1.0,
                max_drawdown_pct: -3.0,
                trade_count: 30 + i as u32,
                equity_curve_tail: vec![100.0, 101.0, 102.0, 103.0, 104.0],
                source_report_path: SmolStr::new("spec/v1.momentum/reports/backtest-fixture.md"),
                generated_at: SmolStr::new("2026-04-29T19:51:48Z"),
                is_multi_symbol: true,
            },
        );
    }
    // Last 4 (DOTUSDT, DOGEUSDT, LINKUSDT, AVAXUSDT) intentionally omitted → Run affordance.

    cockpit.compare_screen_state = CompareScreenState {
        range,
        kpi_axis: CompareKpiAxis::Sharpe,
        cache,
        last_indexed_at: None,
    };

    cockpit
}

/// Construct the `compare__column_header_hover` fixture.
///
/// Matrix with cursor hovering a column header (e.g. "BTCUSDT").
/// Per R2.4 v0.1.0 the column header is non-interactive (label only).
/// The fixture asserts the header does NOT render the active_row border
/// tint — it is visually the same as `cold_boot_all_empty` but serves
/// as a snapshot anchor for the non-interactive header path.
#[must_use]
pub fn compare__column_header_hover_cockpit() -> ui::state::Cockpit {
    // Column headers are non-interactive (R2.4 v0.1.0) — they are plain
    // Container/Text widgets with no on_press. The "hover" state is
    // ephemeral cursor position that iced_test's screenshot path doesn't
    // capture (no cursor event is injected). This fixture is identical to
    // cold_boot_all_empty but names the baseline separately so:
    //   (a) the snapshot confirms the header NEVER gets a tinted border, and
    //   (b) the test name precisely mirrors the T-D-N13 wording.
    compare__cold_boot_all_empty_cockpit()
}

/// Silence dead-code warning for fixtures only consumed via `mod`
/// glob in integration tests. (Cargo runs each `tests/*.rs` as a
/// separate crate, so some fixture exports look unused per-target
/// even when other targets exercise them.)
#[allow(dead_code)]
pub fn _ensure_decimal_dep() -> rust_decimal::Decimal {
    dec!(0)
}
