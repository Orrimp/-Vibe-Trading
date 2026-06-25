//! Deterministic in-memory generators for UI development and testing.
//! Only compiled when the `fixtures` feature is enabled.
//!
//! These exist so the cockpit can be smoke-tested without a running agent
//! (`cargo run --bin cockpit --features fixtures`) and so snapshot tests
//! against `insta` have stable inputs.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::OffsetDateTime;
use trading_core::{
    BacktestMetrics, Bar, EquitySeries, FeeTier, FillView, Money, PnlSnapshot, PositionView, Price,
    Quantity, Side, StrategyEventKind, StrategyEventView, StrategyId, Symbol, Tick, Timeframe,
    Timestamp, Usdt, Venue,
};

use crate::state::{
    AgentMode, AuditKindLabel, Cockpit, JournalRow, MarketHealthState, PanelState, RiskState,
    StrategiesConfig, StrategyConfigEntry, StrategyRow, StrategyStatus, VetoEvent,
};

/// A fixed epoch used to generate deterministic timestamps.
/// 2024-01-15T12:00:00Z — arbitrary but stable, so snapshot outputs
/// don't change day-to-day.
const FIXED_EPOCH_SECS: i64 = 1_705_320_000;

/// Deterministic fixed timestamp.
#[must_use]
pub fn fixed_ts(offset_sec: i64) -> Timestamp {
    let dt = OffsetDateTime::from_unix_timestamp(FIXED_EPOCH_SECS + offset_sec)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
    Timestamp::new(dt)
}

/// A bar at a fixed minute offset.
#[must_use]
pub fn fake_bar(offset_min: i64) -> Bar {
    let open_ts = fixed_ts(offset_min * 60);
    let close_ts = fixed_ts(offset_min * 60 + 59);
    let price = |d: Decimal| -> Price { Price::new(d).unwrap_or_else(|_| unreachable!()) };
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneMinute,
        open_ts,
        close_ts,
        open: price(dec!(40000.00)),
        high: price(dec!(40100.00)),
        low: price(dec!(39900.00)),
        close: price(dec!(40050.00)),
        volume: Quantity::new(dec!(12.5)).unwrap_or_else(|_| unreachable!()),
        trade_count: 100,
        local_recv_ts: close_ts,
        venue: Venue::Binance,
    }
}

/// A tick with an explicit skew (venue - local in ms) so the latency
/// badge can be exercised deterministically.
#[must_use]
pub fn fake_tick_with_skew_ms(skew_ms: i64) -> Tick {
    let venue_ts = fixed_ts(0);
    let local = {
        let dt = OffsetDateTime::from_unix_timestamp_nanos(
            i128::from(FIXED_EPOCH_SECS) * 1_000_000_000 + i128::from(skew_ms) * 1_000_000,
        )
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        Timestamp::new(dt)
    };
    Tick {
        symbol: Symbol::new("BTCUSDT"),
        venue_ts,
        local_recv_ts: local,
        price: Price::new(dec!(40050.00)).unwrap_or_else(|_| unreachable!()),
        qty: Quantity::new(dec!(0.05)).unwrap_or_else(|_| unreachable!()),
        side: Side::Buy,
        trade_id: 1,
        venue: Venue::Binance,
    }
}

/// A single deterministic fill view. `n` controls minor price drift so a
/// sequence of fills renders with distinct rows.
///
/// `transaction_id` is stamped as a deterministic per-`n` fixture id
/// (`"fixture-tx-{n}"`) so the tape-row → audit-modal click flow has a
/// stable, reproducible UUID-shaped string in fixtures-mode (T1206 /
/// `tape-row-audit-modal` Q5). The existing `tape_summary` snapshot
/// helper does not inspect `transaction_id`, so existing snapshots stay
/// byte-identical (R11 + V7).
#[must_use]
pub fn fake_fill_view(n: i64) -> FillView {
    let price = Price::new(dec!(40000.00) + Decimal::from(n) * dec!(0.5))
        .unwrap_or_else(|_| unreachable!());
    let side = if n % 2 == 0 { Side::Buy } else { Side::Sell };
    FillView {
        symbol: Symbol::new("BTCUSDT"),
        side,
        price,
        qty: Quantity::new(dec!(0.1)).unwrap_or_else(|_| unreachable!()),
        fee: Money::from_decimal(dec!(1.6003)),
        fee_tier: FeeTier::Taker,
        venue_ts: fixed_ts(n),
        transaction_id: smol_str::SmolStr::new(format!("fixture-tx-{n}")),
    }
}

/// Generate `n` fills, most recent first (suitable for prepending into the
/// tape's `VecDeque<FillView>`).
#[must_use]
pub fn fake_fill_feed(n: usize) -> Vec<FillView> {
    // `n` is fixture-sized (≤ 200) so the cast is always lossless.
    #[allow(clippy::cast_possible_wrap)]
    let upper = n as i64;
    (0..upper).rev().map(fake_fill_view).collect()
}

/// A P&L snapshot with positive daily return.
#[must_use]
pub fn fake_pnl_positive() -> PnlSnapshot {
    PnlSnapshot {
        cash: Money::from_decimal(dec!(90_000.00)),
        unrealized: Money::from_decimal(dec!(250.00)),
        realized: Money::from_decimal(dec!(-120.50)),
        total_equity: Money::from_decimal(dec!(90_129.50)),
        daily_return: Money::from_decimal(dec!(129.50)),
        as_of: fixed_ts(0),
        bar_ts: None,
    }
}

/// A P&L snapshot with a losing day (exercises `color::DOWN_500`).
#[must_use]
pub fn fake_pnl_negative() -> PnlSnapshot {
    PnlSnapshot {
        cash: Money::from_decimal(dec!(99_500.00)),
        unrealized: Money::from_decimal(dec!(-75.00)),
        realized: Money::from_decimal(dec!(-325.00)),
        total_equity: Money::from_decimal(dec!(99_100.00)),
        daily_return: Money::from_decimal(dec!(-900.00)),
        as_of: fixed_ts(0),
        bar_ts: None,
    }
}

/// Single position, long BTC.
#[must_use]
pub fn fake_position_btc() -> PositionView {
    PositionView {
        symbol: Symbol::new("BTCUSDT"),
        base_qty: dec!(0.25),
        cost_basis: Money::from_decimal(dec!(10_000.00)),
        last_mark: Price::new(dec!(40_050.00)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(12.50)),
        pnl_pct: dec!(0.13),
        exposure_pct: dec!(11.10),
    }
}

/// Short list of positions for the happy path.
#[must_use]
pub fn fake_positions() -> Vec<PositionView> {
    vec![fake_position_btc()]
}

/// A SHORT BTC position with a NEGATIVE (honest, not-clamped) mark-to-market
/// P&L (advisor-short-selling, ADR-0068 § D8 — T-U1). `base_qty < 0` is the
/// signed sell-to-open short the audit reader now emits; the position is under
/// water (price rose against the short), so `pnl` / `pnl_pct` are NEGATIVE and
/// render as `DOWN_500` clay — the down/clay treatment R-SS.4 mandates,
/// NEVER hidden or clamped at 0. The mirror image of [`fake_position_btc`] (a
/// long), which is the render guard's negative control.
#[must_use]
pub fn fake_position_short_btc() -> PositionView {
    PositionView {
        symbol: Symbol::new("BTCUSDT"),
        // Negative qty = SHORT (sold-to-open ~0.30 BTC of notional).
        base_qty: dec!(-0.30),
        // The weighted-average OPEN (proceeds) price of the short.
        cost_basis: Money::from_decimal(dec!(12_000.00)),
        // Mark rose ABOVE the open → the short is losing.
        last_mark: Price::new(dec!(41_200.00)).unwrap_or_else(|_| unreachable!()),
        // Honest NEGATIVE P&L (price moved against the short). NOT clamped.
        pnl: Money::from_decimal(dec!(-360.00)),
        pnl_pct: dec!(-3.00),
        exposure_pct: dec!(13.40),
    }
}

/// A two-row position list mixing a LONG and a SHORT (advisor-short-selling
/// T-U1). The render guard asserts the SHORT badge + negative-P&L clay paint
/// AND that the long still renders correctly (the negative control held inside
/// one frame).
#[must_use]
pub fn fake_positions_with_short() -> Vec<PositionView> {
    vec![fake_position_btc(), fake_position_short_btc()]
}

/// Build the fixture market-health map: all three canonical venues are
/// `Fresh`. The fixtures bin has no watchdog running, so health never
/// updates — this is the "everything is fine" static demo state.
#[must_use]
pub fn fake_market_health() -> std::collections::HashMap<Venue, MarketHealthState> {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(Venue::Binance, MarketHealthState::Fresh);
    map
}

/// Static account label for the fixtures bin (no `Config` available).
/// Per T1508: `"Paper · Demo 3-symbol"`.
pub const FIXTURE_ACCOUNT_LABEL: &str = "Paper \u{00b7} Demo 3-symbol";

/// A cockpit booted fully ready — for manual smoke tests with
/// `cargo run --bin cockpit --features fixtures`.
#[must_use]
pub fn fake_cockpit_ready() -> Cockpit {
    let mut c = Cockpit::ready(fake_fill_feed(12), fake_positions(), fake_pnl_positive());
    c.mode = AgentMode::Paper;
    c.market_health = fake_market_health();
    c.account_label = smol_str::SmolStr::new(FIXTURE_ACCOUNT_LABEL);
    c
}

/// Small deterministic cockpit with exactly three fills — used by the
/// tape snapshot tests so the snapshot body stays reviewable.
#[must_use]
pub fn fake_cockpit_ready_with_three_fills() -> Cockpit {
    let mut c = Cockpit::ready(fake_fill_feed(3), fake_positions(), fake_pnl_positive());
    c.mode = AgentMode::Paper;
    c
}

/// A `Cockpit` routed to `Screen::Live` running a SHORT forward paper-trade
/// (advisor-short-selling, ADR-0068 § D8 — T-U1 / T-U4). Carries:
///   - a positions panel holding a LONG **and** a SHORT (so the SHORT badge +
///     the honest NEGATIVE-P&L clay paint, with the long as the in-frame
///     negative control — `fake_positions_with_short`);
///   - `forward_budget = Some(€200)` so the forward P/L block renders, which is
///     where the load-bearing "a short can lose more than your €200"
///     unbounded-loss disclaimer is carried on the Live surface.
///
/// The `pnl` snapshot is UNDER WATER (equity < budget) — the honest losing
/// short, so the forward P/L prints NEGATIVE (never clamped). Synthetic — no
/// engine, no I/O.
#[must_use]
pub fn fake_cockpit_live_short() -> Cockpit {
    // Equity below the €200 budget — the short is losing (price rose against
    // it). The forward P/L block prints a negative figure, honestly.
    let pnl = PnlSnapshot {
        cash: Money::from_decimal(dec!(200.00)),
        unrealized: Money::from_decimal(dec!(-40.00)),
        realized: Money::from_decimal(dec!(0.00)),
        total_equity: Money::from_decimal(dec!(160.00)),
        daily_return: Money::from_decimal(dec!(-40.00)),
        as_of: fixed_ts(0),
        bar_ts: None,
    };
    let mut c = Cockpit::ready(fake_fill_feed(6), fake_positions_with_short(), pnl);
    c.mode = AgentMode::Paper;
    c.current_screen = crate::state::Screen::Live;
    c.market_health = fake_market_health();
    c.account_label = smol_str::SmolStr::new(FIXTURE_ACCOUNT_LABEL);
    // The forward paper-trade budget — gates the forward P/L block (+ the
    // unbounded-loss disclaimer) on the Live screen.
    c.forward_budget = Some(Money::<Usdt>::from_decimal(dec!(200)));
    c
}

/// The negative control for [`fake_cockpit_live_short`] — the SAME Live screen
/// with a LONG-ONLY position list (`fake_positions`) and a forward budget. Used
/// by the render guard to prove the SHORT badge is ABSENT on a long-only frame
/// (so the positive guard is not a tautology). Still carries the forward P/L
/// block; a long-only forward run does NOT carry the short unbounded-loss
/// disclaimer.
#[must_use]
pub fn fake_cockpit_live_long_only() -> Cockpit {
    let mut c = Cockpit::ready(fake_fill_feed(6), fake_positions(), fake_pnl_positive());
    c.mode = AgentMode::Paper;
    c.current_screen = crate::state::Screen::Live;
    c.market_health = fake_market_health();
    c.account_label = smol_str::SmolStr::new(FIXTURE_ACCOUNT_LABEL);
    c.forward_budget = Some(Money::<Usdt>::from_decimal(dec!(200)));
    c
}

/// Phase 5 (T1910) — deterministic seed for a single `VetoEvent`.
/// `veto_id` is a stable label so snapshot baselines stay reviewable.
#[must_use]
pub fn fake_veto_event(veto_id: &str, strategy: &str, reason: &str) -> VetoEvent {
    use trading_core::{Signal, SignalEvidence, SignalKind};
    VetoEvent {
        veto_id: SmolStr::new(veto_id),
        ts: fixed_ts(0),
        strategy_id: StrategyId::new(strategy),
        reason: SmolStr::new(reason),
        blocked_signal: Signal {
            strategy_id: StrategyId::new(strategy),
            symbol: Symbol::new("BTCUSDT"),
            ts: fixed_ts(0),
            kind: SignalKind::Buy,
            evidence: SignalEvidence::empty(),
            pair_data: None,
        },
    }
}

/// Phase 5 (T1910) — fixture cockpit with one surfaced veto event +
/// the override-modal in `Idle` state. Used by the
/// `panel_snapshots__strategies_screen__override_button_idle.snap`
/// baseline + the override-risk-veto round-trip integration test.
#[must_use]
pub fn fake_cockpit_with_one_veto() -> Cockpit {
    let mut c = fake_cockpit_ready();
    c.risk_veto_events.push(fake_veto_event(
        "veto-1",
        "btc_macd_trend",
        "daily_loss_cap",
    ));
    c
}

// ── v0.5 strategies panel fixtures (T525) ──────────────────────────────────
//
// Deterministic generators so the panel renders the same rows on every run.
// Keeps one Ready, one Loading, and one Error row to cover every status
// pill and the per-row error badge in the widget snapshot suite.

const RECIPE_MACD: &str = "config/strategies/btc_macd_trend.toml";
const RECIPE_RSI: &str = "config/strategies/btc_rsi_reversion.toml";
const RECIPE_BB: &str = "config/strategies/btc_bbands_mean_revert.toml";

/// A full `StrategyRow` in the `Ready` state — healthy strategy, active signals.
#[must_use]
pub fn fake_strategy_row_ready() -> StrategyRow {
    StrategyRow {
        id: StrategyId::new("btc_macd_trend"),
        short_hash: SmolStr::new("a1b2c3d"),
        full_hash: SmolStr::new("a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890"),
        status: StrategyStatus::Ready,
        last_event: Some(fake_event_load("btc_macd_trend", RECIPE_MACD)),
        signals_60s: 3,
        has_position: true,
        source_path: SmolStr::new(RECIPE_MACD),
    }
}

/// A row that is still warming up (no hash assigned yet; `short_hash` and
/// `full_hash` are empty placeholders). Renders the `Loading` status pill
/// and no signal-count number (shows the placeholder dash).
#[must_use]
pub fn fake_strategy_row_loading() -> StrategyRow {
    StrategyRow {
        id: StrategyId::new("btc_rsi_reversion"),
        short_hash: SmolStr::new(""),
        full_hash: SmolStr::new(""),
        status: StrategyStatus::Loading,
        last_event: None,
        signals_60s: 0,
        has_position: false,
        source_path: SmolStr::new(RECIPE_RSI),
    }
}

/// A row that failed its last load attempt. The `error_summary` surfaces in
/// the per-row error badge; the previous `short_hash` is retained so the
/// operator can still see what version was running before the bad swap.
#[must_use]
pub fn fake_strategy_row_error() -> StrategyRow {
    StrategyRow {
        id: StrategyId::new("btc_bbands_mean_revert"),
        short_hash: SmolStr::new("e5f6a7b"),
        full_hash: SmolStr::new("e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6"),
        status: StrategyStatus::Error(SmolStr::new("arity_mismatch: macd_cross(12)")),
        last_event: Some(fake_event_reject("btc_bbands_mean_revert", RECIPE_BB)),
        signals_60s: 0,
        has_position: false,
        source_path: SmolStr::new(RECIPE_BB),
    }
}

/// Three-row fixture covering every status pill — the canonical demo set
/// for `cargo run --bin cockpit --features fixtures` and the widget
/// snapshot tests.
#[must_use]
pub fn fake_strategy_rows() -> Vec<StrategyRow> {
    vec![
        fake_strategy_row_ready(),
        fake_strategy_row_loading(),
        fake_strategy_row_error(),
    ]
}

/// Deterministic `StrategyEventView` of a `Load` event — most recent event
/// for the healthy row. Matches the shape `audit::query::strategy_history`
/// returns.
#[must_use]
pub fn fake_event_load(id: &str, path: &str) -> StrategyEventView {
    StrategyEventView {
        id: SmolStr::new("11111111-1111-1111-1111-111111111111"),
        ts: fixed_ts(0),
        kind: StrategyEventKind::Load,
        strategy_id: Some(StrategyId::new(id)),
        old_hash: None,
        new_hash: Some(SmolStr::new(
            "a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890",
        )),
        source_path: Some(SmolStr::new(path)),
        operator: SmolStr::new("system"),
        error_code: None,
        error_summary: None,
    }
}

/// Deterministic `StrategyEventView` of a `Swap` event.
#[must_use]
pub fn fake_event_swap(id: &str, path: &str) -> StrategyEventView {
    StrategyEventView {
        id: SmolStr::new("22222222-2222-2222-2222-222222222222"),
        ts: fixed_ts(30),
        kind: StrategyEventKind::Swap,
        strategy_id: Some(StrategyId::new(id)),
        old_hash: Some(SmolStr::new(
            "0000000000000000000000000000000000000000000000000000000000000000",
        )),
        new_hash: Some(SmolStr::new(
            "a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890",
        )),
        source_path: Some(SmolStr::new(path)),
        operator: SmolStr::new("system"),
        error_code: None,
        error_summary: None,
    }
}

/// Deterministic `StrategyEventView` of a `Reject` event — used for the per-
/// row error badge.
#[must_use]
pub fn fake_event_reject(id: &str, path: &str) -> StrategyEventView {
    StrategyEventView {
        id: SmolStr::new("33333333-3333-3333-3333-333333333333"),
        ts: fixed_ts(60),
        kind: StrategyEventKind::Reject,
        strategy_id: Some(StrategyId::new(id)),
        old_hash: None,
        new_hash: None,
        source_path: Some(SmolStr::new(path)),
        operator: SmolStr::new("system"),
        error_code: Some(SmolStr::new("arity_mismatch")),
        error_summary: Some(SmolStr::new("arity_mismatch: macd_cross(12)")),
    }
}

/// Ten recent events for the footer list, covering Load / Swap / Reject.
/// Newest-first so the iced renderer iterates without reversing.
#[must_use]
pub fn fake_recent_events() -> Vec<StrategyEventView> {
    vec![
        fake_event_reject("btc_bbands_mean_revert", RECIPE_BB),
        fake_event_swap("btc_macd_trend", RECIPE_MACD),
        fake_event_load("btc_rsi_reversion", RECIPE_RSI),
        fake_event_load("btc_macd_trend", RECIPE_MACD),
    ]
}

/// A cockpit in the strategies-panel Ready state with three deterministic
/// rows. Used by `cargo run --bin cockpit --features fixtures` and by the
/// T527 full-cockpit snapshot.
#[must_use]
pub fn fake_cockpit_with_strategies() -> Cockpit {
    let mut c = fake_cockpit_ready();
    c.strategies = PanelState::Ready(fake_strategy_rows());
    c.strategies_recent_events = fake_recent_events().into_iter().collect();
    c
}

// ── v1 cross-sectional momentum fixtures (T623) ─────────────────────────────
//
// R11 says the v0 positions panel "already supports N rows" and v1 needs
// **zero new widget code** — fixtures just feed it a multi-symbol portfolio
// so `cargo run --bin cockpit --features fixtures` can demo the multi-row
// steady state of `top10_momentum_h1`'s `K_long = 3` selection.
//
// The three rows below are tuned to exercise every branch of
// `theme::color_for_delta` in one screen: BTCUSDT carries a positive P&L
// (`POS` green), ETHUSDT a negative P&L (`NEG` red), and SOLUSDT a zero
// P&L (`FG_MUTED`). Exposure totals ~33% so the operator sees a
// comfortable headroom over the strategy's notional cap.

const RECIPE_TOP10_MOMENTUM: &str = "config/strategies/top10_momentum_h1.toml";

/// Long BTC leg of the top-3 momentum portfolio. Positive unrealized.
#[must_use]
pub fn fake_v1_position_btc() -> PositionView {
    PositionView {
        symbol: Symbol::new("BTCUSDT"),
        base_qty: dec!(0.30),
        cost_basis: Money::from_decimal(dec!(12_000.00)),
        last_mark: Price::new(dec!(40_500.00)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(150.00)),
        pnl_pct: dec!(1.25),
        exposure_pct: dec!(12.15),
    }
}

/// Long ETH leg — slightly underwater. Drives the `NEG` color path.
#[must_use]
pub fn fake_v1_position_eth() -> PositionView {
    PositionView {
        symbol: Symbol::new("ETHUSDT"),
        base_qty: dec!(4.50),
        cost_basis: Money::from_decimal(dec!(11_000.00)),
        last_mark: Price::new(dec!(2_400.00)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(-200.00)),
        pnl_pct: dec!(-1.82),
        exposure_pct: dec!(10.80),
    }
}

/// Long SOL leg — flat (zero unrealized). Drives the `FG_MUTED` color path
/// of `color_for_delta`, which the v0 single-row fixture never exercised.
#[must_use]
pub fn fake_v1_position_sol() -> PositionView {
    PositionView {
        symbol: Symbol::new("SOLUSDT"),
        base_qty: dec!(110.00),
        cost_basis: Money::from_decimal(dec!(11_000.00)),
        last_mark: Price::new(dec!(100.00)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(0)),
        pnl_pct: dec!(0),
        exposure_pct: dec!(9.90),
    }
}

/// Top-3 long-only momentum portfolio: BTC (`POS`), ETH (`NEG`), SOL (`FG_MUTED`).
/// Order matches the rebalance-output convention: highest momentum first.
/// This is the canonical v1 R11 steady-state input.
#[must_use]
pub fn fake_v1_three_symbol_portfolio() -> Vec<PositionView> {
    vec![
        fake_v1_position_btc(),
        fake_v1_position_eth(),
        fake_v1_position_sol(),
    ]
}

/// Single strategy row for `top10_momentum_h1` in the v1 cockpit demo.
/// Mirrors what `MomentumStrategy` reports once it's running steady-state:
/// loaded, holding K=3 positions, signals fired on the last rebalance.
#[must_use]
pub fn fake_v1_strategy_row_momentum() -> StrategyRow {
    StrategyRow {
        id: StrategyId::new("top10_momentum_h1"),
        short_hash: SmolStr::new("c0ffee0"),
        full_hash: SmolStr::new("c0ffee00deadbeefcafef00d12345678c0ffee00deadbeefcafef00d12345678"),
        status: StrategyStatus::Ready,
        last_event: Some(fake_event_load("top10_momentum_h1", RECIPE_TOP10_MOMENTUM)),
        signals_60s: 3,
        has_position: true,
        source_path: SmolStr::new(RECIPE_TOP10_MOMENTUM),
    }
}

/// One-row strategies-panel input for v1 — the only deployed strategy is
/// `top10_momentum_h1`.
#[must_use]
pub fn fake_v1_strategy_rows() -> Vec<StrategyRow> {
    vec![fake_v1_strategy_row_momentum()]
}

/// Recent-events footer for v1 demo: the most recent boot event for the
/// momentum strategy. Newest-first to match `fake_recent_events`.
#[must_use]
pub fn fake_v1_recent_events() -> Vec<StrategyEventView> {
    vec![fake_event_load("top10_momentum_h1", RECIPE_TOP10_MOMENTUM)]
}

/// Cockpit booted into the v1 steady-state: top-3 long momentum portfolio
/// (3 position rows: `POS` / `NEG` / `FG_MUTED`) plus one strategy row
/// for `top10_momentum_h1`. Used by `cargo run --bin cockpit --features
/// fixtures` and by the `T_FINAL_B_v1` multi-row snapshot. R11 negative
/// confirmation — pure data, no widget code change.
#[must_use]
pub fn fake_cockpit_v1_steady_state() -> Cockpit {
    let mut c = Cockpit::ready(
        fake_fill_feed(12),
        fake_v1_three_symbol_portfolio(),
        fake_pnl_positive(),
    );
    c.mode = AgentMode::Paper;
    c.strategies = PanelState::Ready(fake_v1_strategy_rows());
    c.strategies_recent_events = fake_v1_recent_events().into_iter().collect();
    c
}

// ── v1.5a mean-reversion-pairs fixtures (T719) ──────────────────────────────
//
// R11 (v1.5a) is a **negative confirmation** — the v0 multi-row positions
// panel and the v0.5 strategies panel already render the v1.5a steady state
// without a widget code change. Per architecture.md Q3 (formulation C), only
// the long `a` legs of each pair appear on-book; the would-have-shorted `b`
// legs surface as `pair_short_observation` strategy-event rows in the
// recent-events footer (zero money columns). The architect's Q8 added two
// new `StrategyEventKind` variants (`MeanReversionStop`,
// `PairShortObservation`); both are already exhaustively matched by the
// strategies widget and the snapshot helper.
//
// Steady state for the canonical `pairs_mr_h1` config (T714):
//
// - 3 long-leg position rows on the `a` legs of the three pairs:
//     (BTCUSDT, ETHUSDT) → BTCUSDT long
//     (ETHUSDT, SOLUSDT) → ETHUSDT long
//     (BNBUSDT, BTCUSDT) → BNBUSDT long
// - 1 strategies-panel row for `pairs_mr_h1` with kind
//   `mean_reversion_pairs` (panel renders the id verbatim; kind is implicit
//   in the row's source path).
// - Recent-events footer carries the two new v1.5a kinds plus a `Load`
//   row, exercising every `StrategyEventKind` arm in the widget's `match`.
// - Live tape shows mixed `Side::Buy` / `Side::Sell` fills from the
//   long-leg round trips so the `POS` / `NEG` color paths are exercised.
//
// Strings: zero new copy expected — the strategies widget already maps
// the new `MeanReversionStop` / `PairShortObservation` kinds onto the
// existing `STRATEGIES_EVENT_LOAD` label (informational; muted color).
// Adding a v1.5a-specific copy string would create a code smell per the
// design-system contract, so we route through the existing label.

const RECIPE_PAIRS_MR_H1: &str = "config/strategies/pairs_mr_h1.toml";

/// Long-leg BTC position from the `(BTCUSDT, ETHUSDT)` pair. Positive
/// unrealized — drives the `POS` color path on row 1.
#[must_use]
pub fn fake_v15a_position_btc() -> PositionView {
    PositionView {
        symbol: Symbol::new("BTCUSDT"),
        base_qty: dec!(0.45),
        cost_basis: Money::from_decimal(dec!(18_000.00)),
        last_mark: Price::new(dec!(40_500.00)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(225.00)),
        pnl_pct: dec!(1.25),
        exposure_pct: dec!(18.30),
    }
}

/// Long-leg ETH position from the `(ETHUSDT, SOLUSDT)` pair. Slightly
/// underwater — drives the `NEG` color path on row 2.
#[must_use]
pub fn fake_v15a_position_eth() -> PositionView {
    PositionView {
        symbol: Symbol::new("ETHUSDT"),
        base_qty: dec!(7.50),
        cost_basis: Money::from_decimal(dec!(18_300.00)),
        last_mark: Price::new(dec!(2_400.00)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(-300.00)),
        pnl_pct: dec!(-1.64),
        exposure_pct: dec!(18.20),
    }
}

/// Long-leg BNB position from the `(BNBUSDT, BTCUSDT)` pair. Flat —
/// drives the `FG_MUTED` zero-delta color path on row 3.
#[must_use]
pub fn fake_v15a_position_bnb() -> PositionView {
    PositionView {
        symbol: Symbol::new("BNBUSDT"),
        base_qty: dec!(60.00),
        cost_basis: Money::from_decimal(dec!(18_000.00)),
        last_mark: Price::new(dec!(300.00)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(0)),
        pnl_pct: dec!(0),
        exposure_pct: dec!(18.00),
    }
}

/// Three long-leg position rows for the v1.5a steady state of
/// `pairs_mr_h1`. Order matches the lex-sorted `BTreeMap<PairKey, _>`
/// iteration the strategy uses (R9.3): `(BTCUSDT, ETHUSDT)`,
/// `(BNBUSDT, BTCUSDT)`, `(ETHUSDT, SOLUSDT)` — but the position rows
/// surface only the traded `a` leg of each pair, so the rendered order
/// is the lex-sorted `a` legs: BTCUSDT, BNBUSDT, ETHUSDT. Per Q3
/// formulation-C, no short-leg rows appear on-book.
#[must_use]
pub fn fake_v15a_three_long_legs() -> Vec<PositionView> {
    vec![
        fake_v15a_position_btc(),
        fake_v15a_position_bnb(),
        fake_v15a_position_eth(),
    ]
}

/// Strategy row for `pairs_mr_h1` in steady state. The id, source path
/// and short hash are stable; the rendered kind (`mean_reversion_pairs`)
/// is implicit in the row — the strategies widget reads it via the
/// `StrategyId` and source-path tooltip, not via a separate column.
#[must_use]
pub fn fake_v15a_strategy_row_pairs_mr_h1() -> StrategyRow {
    StrategyRow {
        id: StrategyId::new("pairs_mr_h1"),
        short_hash: SmolStr::new("90591a0"),
        full_hash: SmolStr::new("90591a0e1f2c3d4a5b6e7f8091a2b3c4d5e6f70819a0b1c2d3e4f5061728394a"),
        status: StrategyStatus::Ready,
        last_event: Some(fake_event_load("pairs_mr_h1", RECIPE_PAIRS_MR_H1)),
        signals_60s: 6,
        has_position: true,
        source_path: SmolStr::new(RECIPE_PAIRS_MR_H1),
    }
}

/// One-row strategies-panel input for v1.5a — only deployed strategy is
/// `pairs_mr_h1`.
#[must_use]
pub fn fake_v15a_strategy_rows() -> Vec<StrategyRow> {
    vec![fake_v15a_strategy_row_pairs_mr_h1()]
}

/// Build a `StrategyEventView` carrying a v1.5a `MeanReversionStop` row
/// — the architect's Q8 hard-stop event written to `strategy_events`.
/// Rendered in the recent-events footer with `FG_MUTED` color and the
/// `STRATEGIES_EVENT_LOAD` label per the strategies widget's mapping
/// (observation-only kind; not a control event).
#[must_use]
pub fn fake_event_mean_reversion_stop(id: &str, path: &str) -> StrategyEventView {
    StrategyEventView {
        id: SmolStr::new("44444444-4444-4444-4444-444444444444"),
        ts: fixed_ts(120),
        kind: StrategyEventKind::MeanReversionStop,
        strategy_id: Some(StrategyId::new(id)),
        old_hash: None,
        new_hash: None,
        source_path: Some(SmolStr::new(path)),
        operator: SmolStr::new("system"),
        error_code: Some(SmolStr::new("mean_reversion_stop")),
        error_summary: Some(SmolStr::new("z=4.12 >= z_stop=4.0 on (BTCUSDT, ETHUSDT)")),
    }
}

/// Build a `StrategyEventView` carrying a v1.5a `PairShortObservation`
/// row — the architect's Q8 observation-only event written alongside
/// every long-leg buy on entry. `error_code` and `error_summary` are
/// `None` (this is informational, not an error).
#[must_use]
pub fn fake_event_pair_short_observation(id: &str, path: &str) -> StrategyEventView {
    StrategyEventView {
        id: SmolStr::new("55555555-5555-5555-5555-555555555555"),
        ts: fixed_ts(180),
        kind: StrategyEventKind::PairShortObservation,
        strategy_id: Some(StrategyId::new(id)),
        old_hash: None,
        new_hash: None,
        source_path: Some(SmolStr::new(path)),
        operator: SmolStr::new("system"),
        error_code: None,
        error_summary: None,
    }
}

/// Recent-events footer for v1.5a steady state. Newest-first to match the
/// `fake_recent_events` ordering convention. Exercises the two new v1.5a
/// `StrategyEventKind` variants (`MeanReversionStop`,
/// `PairShortObservation`) plus a `Load` row so every footer color path
/// is covered.
#[must_use]
pub fn fake_v15a_recent_events() -> Vec<StrategyEventView> {
    vec![
        fake_event_pair_short_observation("pairs_mr_h1", RECIPE_PAIRS_MR_H1),
        fake_event_mean_reversion_stop("pairs_mr_h1", RECIPE_PAIRS_MR_H1),
        fake_event_pair_short_observation("pairs_mr_h1", RECIPE_PAIRS_MR_H1),
        fake_event_load("pairs_mr_h1", RECIPE_PAIRS_MR_H1),
    ]
}

/// Cockpit booted into the v1.5a steady-state: 3 long-leg position rows
/// (BTCUSDT / BNBUSDT / ETHUSDT — formulation C, only `a` legs trade)
/// plus one strategy row for `pairs_mr_h1`, with recent-events footer
/// listing `MeanReversionStop` + `PairShortObservation` kinds. R11
/// negative confirmation — pure data, no widget code change.
///
/// This is the **default** fixtures-mode cockpit boot: operators that
/// run `cargo run --bin cockpit --features fixtures` see the most
/// recent feature set. Earlier presets (`fake_cockpit_v1_steady_state`,
/// `fake_cockpit_with_strategies`) remain available for snapshot tests.
#[must_use]
pub fn fake_cockpit_v15a_pairs_steady_state() -> Cockpit {
    let mut c = Cockpit::ready(
        fake_fill_feed(8),
        fake_v15a_three_long_legs(),
        fake_pnl_positive(),
    );
    c.mode = AgentMode::Paper;
    c.strategies = PanelState::Ready(fake_v15a_strategy_rows());
    c.strategies_recent_events = fake_v15a_recent_events().into_iter().collect();
    c
}

// ── Phase 2 — synthetic candles + per-symbol fills (T1607) ─────────────────

/// Per-symbol starting price + volatility for the deterministic random
/// walk. Reflects the rough magnitude of each pair's actual price level so
/// the fixtures-mode chart has visually appropriate amplitude.
fn symbol_table(symbol: &Symbol) -> (Decimal, f64) {
    match symbol.0.as_str() {
        "BTCUSDT" => (dec!(40_000), 50.0),
        "ETHUSDT" => (dec!(2_400), 8.0),
        "SOLUSDT" => (dec!(90), 1.5),
        _ => (dec!(100), 1.0),
    }
}

/// Phase 2 (Q6) — per-symbol seed via `DefaultHasher` over
/// `format!("{venue:?}/{symbol}")`. In-process determinism is sufficient
/// for Phase 2 (snapshot baselines pinned per CI run).
#[must_use]
pub fn seed_for(venue: Venue, symbol: &Symbol) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    format!("{venue:?}/{symbol}").hash(&mut h);
    h.finish()
}

/// Phase 2 — deterministic OHLC random walk for fixtures-mode chart
/// seeding. `seed` controls the random walk; each call with the same
/// `(seed, venue, symbol, count)` produces a byte-equal `Vec<Bar>`.
///
/// `open_ts` is anchored at `fixed_ts(offset_min * 60)` so the fixtures
/// bin's chart sits at the same epoch the rest of the fixtures share.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn synthetic_candles(seed: u64, venue: Venue, symbol: Symbol, count: usize) -> Vec<Bar> {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    // Broadcast the u64 seed across the 32-byte ChaCha20 seed array
    // (8 bytes seed + 24 zero bytes — the simplest stable shape).
    let mut seed_bytes = [0u8; 32];
    seed_bytes[..8].copy_from_slice(&seed.to_le_bytes());
    let mut rng = ChaCha20Rng::from_seed(seed_bytes);

    let (start_price, vol) = symbol_table(&symbol);
    let mut prev_close = start_price;
    let mut out = Vec::with_capacity(count);

    let p = |d: Decimal| -> Price {
        // Floor to at least 1 cent so Price::new doesn't reject. Random
        // walks at extreme negative values clamp to 1 USDT.
        let clamped = d.max(dec!(1));
        Price::new(clamped).unwrap_or_else(|_| unreachable!())
    };

    // `count` is fixture-bounded (≤ 60 typical); the cast is lossless.
    #[allow(clippy::cast_possible_wrap)]
    for i in 0..count as i64 {
        let drift: f64 = rng.random_range(-vol..vol);
        let drift_dec = Decimal::try_from(drift).unwrap_or(dec!(0));
        let open = prev_close;
        let close = open + drift_dec;
        let wick_high: f64 = rng.random_range(0.0..vol / 2.0);
        let wick_low: f64 = rng.random_range(0.0..vol / 2.0);
        let wick_high_dec = Decimal::try_from(wick_high).unwrap_or(dec!(0));
        let wick_low_dec = Decimal::try_from(wick_low).unwrap_or(dec!(0));
        let high = open.max(close) + wick_high_dec;
        let low = open.min(close) - wick_low_dec;

        let open_ts = fixed_ts(i * 60);
        let close_ts = fixed_ts(i * 60 + 59);
        out.push(Bar {
            symbol: symbol.clone(),
            tf: Timeframe::OneMinute,
            open_ts,
            close_ts,
            open: p(open),
            high: p(high),
            low: p(low),
            close: p(close),
            volume: Quantity::new(dec!(12.5)).unwrap_or_else(|_| unreachable!()),
            trade_count: 100,
            local_recv_ts: close_ts,
            venue,
        });
        prev_close = close;
    }
    out
}

/// Phase 2 — produce `count` deterministic fills alternating `Buy` /
/// `Sell` for the given `(venue, symbol)`. Mirrors the existing
/// `fake_fill_view` `n % 2 == 0` side-alternation rule with `symbol`
/// substituted in; when `count >= 2` the result holds at least one
/// buy and one sell.
///
/// Fill timestamps are evenly distributed across the 60-minute bar
/// range produced by [`synthetic_candles`] (60 bars × 60s each) so
/// markers render visibly along the chart x-axis. Fill `i` lands at
/// fraction `(i + 1) / (count + 1)` of the bar range — for `count = 4`
/// the fills sit at minutes 12, 24, 36, 48 (no marker at either edge).
/// Earlier shape used `fixed_ts(n)` which clustered every fill into
/// the leftmost ~0.1% of the chart canvas — invisible to the operator.
#[must_use]
pub fn synthetic_fills_for(_venue: Venue, symbol: &Symbol, count: usize) -> Vec<FillView> {
    // 60 bars × 60s per bar — see `synthetic_candles` doc.
    const BAR_RANGE_SECS: i64 = 60 * 60;
    let (start_price, _vol) = symbol_table(symbol);
    let mut out = Vec::with_capacity(count);
    // `count` is fixture-sized (≤ 200); the cast is lossless.
    #[allow(clippy::cast_possible_wrap)]
    let denom = count as i64 + 1;
    for i in 0..count {
        // `i` is fixture-sized (≤ 200); the cast is lossless.
        #[allow(clippy::cast_possible_wrap)]
        let n = i as i64;
        let side = if n % 2 == 0 { Side::Buy } else { Side::Sell };
        let price = Price::new(start_price + Decimal::from(n)).unwrap_or_else(|_| unreachable!());
        let venue_ts_secs = ((n + 1) * BAR_RANGE_SECS) / denom;
        out.push(FillView {
            symbol: symbol.clone(),
            side,
            price,
            qty: Quantity::new(dec!(0.1)).unwrap_or_else(|_| unreachable!()),
            fee: Money::from_decimal(dec!(0.5)),
            fee_tier: FeeTier::Taker,
            venue_ts: fixed_ts(venue_ts_secs),
            transaction_id: SmolStr::new(format!("fixture-{symbol}-{n}")),
        });
    }
    out
}

// ── Phase 3 (Lumen detail screens) — Strategies / Risk / Audit fixtures ─────
//
// Deterministic builders the fixtures bin pre-seeds and the snapshot
// tests consume. Each helper returns canonical demo data with stable
// timestamps / hashes so insta baselines stay reviewable.

/// T1707 — Risk-screen fixture covering all three colour bands per V5:
/// one venue/symbol < 70 % (`ACCENT`), one ≥ 80 % (`WARN_500`), one
/// ≥ 95 % (`DOWN_500`). `daily_loss_used_pct` and `heartbeat_age_ms`
/// stay in the green band so the snapshot foregrounds the per-symbol
/// bars (the focus of the screen).
#[must_use]
pub fn fake_risk_state() -> RiskState {
    use std::collections::HashMap;
    let mut exposure = HashMap::new();
    let mut caps = HashMap::new();
    // (Binance, BTCUSDT) — 50 / 100 = 50 % → ACCENT band.
    exposure.insert((Venue::Binance, Symbol::new("BTCUSDT")), Decimal::from(50));
    caps.insert((Venue::Binance, Symbol::new("BTCUSDT")), Decimal::from(100));
    // (Binance, ETHUSDT) — 80 / 100 = 80 % → WARN_500 band.
    exposure.insert((Venue::Binance, Symbol::new("ETHUSDT")), Decimal::from(80));
    caps.insert((Venue::Binance, Symbol::new("ETHUSDT")), Decimal::from(100));
    // (Coinbase, SOLUSDT) — 95 / 100 = 95 % → DOWN_500 band.
    exposure.insert((Venue::Coinbase, Symbol::new("SOLUSDT")), Decimal::from(95));
    caps.insert(
        (Venue::Coinbase, Symbol::new("SOLUSDT")),
        Decimal::from(100),
    );
    RiskState {
        per_symbol_exposure: exposure,
        per_symbol_caps: caps,
        daily_loss_used_pct: Decimal::from(20),
        daily_loss_cap_pct: Decimal::from(100),
        heartbeat_age_ms: 120,
        heartbeat_timeout_ms: 30_000,
    }
}

/// T1704 — Strategies-detail fixture covering ≥ 3 params rows per the
/// snapshot acceptance. Three strategies: SMA crossover, MACD trend,
/// RSI reversion (matches `fake_strategy_row_*` ids so the chip row +
/// params block render together end-to-end).
#[must_use]
pub fn fake_strategies_config() -> StrategiesConfig {
    StrategiesConfig {
        strategies: vec![
            StrategyConfigEntry {
                id: StrategyId::new("btc_macd_trend"),
                source_path: SmolStr::new(RECIPE_MACD),
                params: vec![
                    (SmolStr::new("symbol"), SmolStr::new("BTCUSDT")),
                    (SmolStr::new("fast_period"), SmolStr::new("12")),
                    (SmolStr::new("slow_period"), SmolStr::new("26")),
                    (SmolStr::new("signal_period"), SmolStr::new("9")),
                ],
            },
            StrategyConfigEntry {
                id: StrategyId::new("btc_rsi_reversion"),
                source_path: SmolStr::new(RECIPE_RSI),
                params: vec![
                    (SmolStr::new("symbol"), SmolStr::new("BTCUSDT")),
                    (SmolStr::new("period"), SmolStr::new("14")),
                    (SmolStr::new("oversold"), SmolStr::new("30")),
                    (SmolStr::new("overbought"), SmolStr::new("70")),
                ],
            },
            StrategyConfigEntry {
                id: StrategyId::new("btc_bbands_mean_revert"),
                source_path: SmolStr::new(RECIPE_BB),
                params: vec![
                    (SmolStr::new("symbol"), SmolStr::new("BTCUSDT")),
                    (SmolStr::new("period"), SmolStr::new("20")),
                    (SmolStr::new("std_dev"), SmolStr::new("2")),
                ],
            },
        ],
    }
}

/// T1710 — Audit-screen fixture row generator. Returns `count`
/// deterministic rows spanning multiple venues / symbols / kinds in
/// reverse-chronological order (newest-first). Row `n` sits at
/// `FIXED_EPOCH_SECS + n` and rotates through (Venue, Symbol, kind)
/// triples so the snapshot exercises every column shape.
#[must_use]
pub fn fake_journal_rows(count: usize) -> Vec<JournalRow> {
    let venues = [Venue::Binance, Venue::Coinbase, Venue::Kraken];
    let symbols = [
        Some(Symbol::new("BTCUSDT")),
        Some(Symbol::new("ETHUSDT")),
        Some(Symbol::new("SOLUSDT")),
        None,
    ];
    let kinds = [
        AuditKindLabel::Fill,
        AuditKindLabel::StrategyEvent,
        AuditKindLabel::Reconciliation,
    ];
    let strategies = [
        Some(StrategyId::new("btc_macd_trend")),
        Some(StrategyId::new("btc_rsi_reversion")),
        None,
    ];

    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        let n_i64 = i64::try_from(n).unwrap_or(0);
        let venue = venues[n % venues.len()];
        let symbol = symbols[n % symbols.len()].clone();
        let kind = kinds[n % kinds.len()];
        let strategy_id = strategies[n % strategies.len()].clone();
        // Fixed seconds-from-epoch so timestamps are stable across runs.
        let ts = fixed_ts(n_i64);
        let description = match kind {
            AuditKindLabel::Fill => format!(
                "buy 0.{:02} {} @ 50000",
                n % 100,
                symbol.as_ref().map_or("BTCUSDT", |s| s.0.as_str()),
            ),
            AuditKindLabel::StrategyEvent => "registry:StrategyLoaded:btc_macd_trend".to_string(),
            AuditKindLabel::Reconciliation => format!("reconcile delta={n}"),
        };
        out.push(JournalRow {
            tx_id: SmolStr::new(format!("fixture-row-{n:04}")),
            ts,
            venue,
            symbol,
            kind,
            description: SmolStr::new(description),
            strategy_id,
        });
    }
    out
}

// ── Phase 4 (T1805 / T1806 / T1811) — viewer + sparkline fixtures ───────────

// ── ui-gallery-bin v0.1 — new fixture helpers (design.md § State seeding contract) ──
//
// Four helpers required by the gallery route table (H-GAL-4 budget: ~70 LOC).
// All are pure additions; no existing helper signature is changed.

/// Gallery cell 21/22 — deterministic `VolumeBin` slice for the
/// `volume_histogram` showcase cells. Mixed buy/sell notionals so every
/// color path (`UP_500` / `DOWN_500`) is exercised.
#[must_use]
pub fn fake_volume_bins() -> Vec<crate::widgets::volume_histogram::VolumeBin> {
    use crate::widgets::volume_histogram::VolumeBin;
    vec![
        VolumeBin {
            buys_usdt: dec!(5_000),
            sells_usdt: dec!(3_000),
        },
        VolumeBin {
            buys_usdt: dec!(2_500),
            sells_usdt: dec!(7_000),
        },
        VolumeBin {
            buys_usdt: dec!(8_000),
            sells_usdt: dec!(1_000),
        },
        VolumeBin {
            buys_usdt: dec!(0),
            sells_usdt: dec!(4_500),
        },
        VolumeBin {
            buys_usdt: dec!(6_000),
            sells_usdt: dec!(6_000),
        },
        VolumeBin {
            buys_usdt: dec!(9_000),
            sells_usdt: dec!(500),
        },
        VolumeBin {
            buys_usdt: dec!(1_500),
            sells_usdt: dec!(8_500),
        },
        VolumeBin {
            buys_usdt: dec!(4_000),
            sells_usdt: dec!(4_000),
        },
    ]
}

/// Gallery cell 24 — a deterministic `SignalView` fixture for the
/// `chart_tooltip/signal_tooltip` cell. `n` offsets the timestamp so
/// multiple signals can sit at distinct positions on the chart x-axis.
#[must_use]
pub fn fake_signal_view(n: i64) -> trading_core::SignalView {
    use trading_core::{Quantity, Side, SignalView, StrategyId};
    SignalView {
        signal_id: smol_str::SmolStr::new(format!("fixture-signal-{n}")),
        symbol: trading_core::Symbol::new("BTCUSDT"),
        side: if n % 2 == 0 { Side::Buy } else { Side::Sell },
        intended_qty: Quantity::new(dec!(0.1)).unwrap_or_else(|_| unreachable!()),
        signal_ts: fixed_ts(n * 60),
        strategy_id: StrategyId::new("btc_macd_trend"),
        was_clamped: false,
        clamp_reason: None,
    }
}

/// Gallery cell 9 — `strategies/with_error_row`. Returns a cockpit
/// whose `strategies` panel has three rows: one `Ready`, one `Loading`,
/// and one `Error` — the same three-pill state set as `fake_strategy_rows`
/// but with `StrategyStatus::Error` on the third row. Reuses the existing
/// row builders + patches the first row to `Error` so the snapshot
/// shows all three status pills in one column.
///
/// In practice, `fake_cockpit_with_strategies()` already seeds an Error
/// row (via `fake_strategy_row_error()`) — this helper is a thin wrapper
/// that makes the cell's intent explicit by name without duplicating code.
#[must_use]
pub fn fake_strategy_row_error_in_v1_set() -> Cockpit {
    // `fake_cockpit_with_strategies` already seeds one Ready + one Loading
    // + one Error row. Re-use it directly — no code duplication.
    fake_cockpit_with_strategies()
}

/// Gallery cell 14 — `latency/degraded`. Returns a cockpit where the
/// Binance venue has `MarketHealthState::Stale`, representing a degraded
/// market-health state that the latency widget renders in its warn color.
#[must_use]
pub fn fake_market_health_degraded() -> Cockpit {
    let mut c = fake_cockpit_ready();
    c.market_health.insert(
        trading_core::Venue::Binance,
        crate::state::MarketHealthState::Stale,
    );
    c
}

/// Phase 4 (T1805) — deterministic `BacktestMetrics` matching the
/// RSI sample: Total return -57.80 %, Sharpe -55.4257, Max DD
/// 57.81 %, Trades 14118; CAGR + Win rate marked-absent.
#[must_use]
pub fn fake_backtest_metrics() -> BacktestMetrics {
    BacktestMetrics {
        total_return_pct: rust_decimal_macros::dec!(-57.80),
        cagr_pct: rust_decimal::Decimal::ZERO,
        cagr_present: false,
        sharpe: rust_decimal_macros::dec!(-55.4257),
        sharpe_present: true,
        max_drawdown_pct: rust_decimal_macros::dec!(57.81),
        win_rate_pct: rust_decimal::Decimal::ZERO,
        win_rate_present: false,
        trades: 14118,
    }
}

/// Phase 4 (T1806) — 60-point synthetic series matching the RSI
/// report shape: `peak = 100_000`, `trough = 42_195`,
/// `max-DD ≈ 0.5781`.
#[must_use]
pub fn fake_equity_series_for_viewer() -> EquitySeries {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    let mut pts = Vec::with_capacity(60);
    for i in 0..30i64 {
        pts.push((
            fixed_ts(i * 60),
            Money::<Usdt>::from_decimal(dec!(100000) - dec!(1000) * Decimal::from(i)),
        ));
    }
    for i in 0..30i64 {
        let v = dec!(70000) - dec!(1000) * Decimal::from(i);
        pts.push((
            fixed_ts((30 + i) * 60),
            Money::<Usdt>::from_decimal(v.max(dec!(42195))),
        ));
    }
    EquitySeries::from_points(pts).unwrap_or_else(|_| {
        EquitySeries::from_points(vec![(
            fixed_ts(0),
            Money::<Usdt>::from_decimal(dec!(100000)),
        )])
        .unwrap_or_else(|_| unreachable!())
    })
}

/// Phase 4 (T1811) — 120-point series for the cockpit-side
/// Strategies-detail sparkline baseline. Deterministic ramp-up then
/// ramp-down.
#[must_use]
pub fn fake_equity_series_for_sparkline() -> EquitySeries {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    let mut pts = Vec::with_capacity(120);
    for i in 0..60i64 {
        pts.push((
            fixed_ts(i * 60),
            Money::<Usdt>::from_decimal(dec!(1000) + Decimal::from(i) * dec!(10)),
        ));
    }
    for i in 0..60i64 {
        pts.push((
            fixed_ts((60 + i) * 60),
            Money::<Usdt>::from_decimal(dec!(1600) - Decimal::from(i) * dec!(5)),
        ));
    }
    EquitySeries::from_points(pts).unwrap_or_else(|_| unreachable!())
}

/// lab-polish-round-2 R1 — deterministic `(close_ts_millis, qty)` slice for
/// the `position_curve` gallery cell. Mimics a three-buy / two-sell cumulative
/// position sequence so both the rising and falling step are visible.
#[must_use]
pub fn fake_position_curve_points() -> Vec<(i64, rust_decimal::Decimal)> {
    use rust_decimal_macros::dec;
    // 6 bars spaced 1 hour apart starting from a fixed epoch.
    let base_ms = FIXED_EPOCH_SECS * 1_000;
    vec![
        (base_ms, dec!(0)),
        (base_ms + 3_600_000, dec!(0.5)),
        (base_ms + 7_200_000, dec!(1.0)),
        (base_ms + 10_800_000, dec!(1.5)),
        (base_ms + 14_400_000, dec!(1.0)),
        (base_ms + 18_000_000, dec!(0)),
    ]
}

/// Phase B (T-D-N13) — two deterministic `RunReportMirror` instances for
/// the `run_delta_badge` gallery cell and unit tests.
///
/// Returns `(last, prev)` where `last` has better P&L and lower drawdown than
/// `prev` — the badge should show UP/UP for P&L and DD columns.
#[must_use]
pub fn fake_run_report_mirror_pair() -> (
    crate::lab::runner::RunReportMirror,
    crate::lab::runner::RunReportMirror,
) {
    use crate::lab::equity_loader::LabTuple;
    use crate::lab::runner::RunReportMirror;
    use crate::lab::state::{DateRange, Preset};
    use backtest::engine::BacktestKpis;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    let tuple = LabTuple {
        strategy: SmolStr::new("v1.momentum"),
        symbol: SmolStr::new("XRPUSDT"),
        range: DateRange::Preset(Preset::H1_2024),
    };

    // Last run: +$3 200 P&L, 8% max DD, rising equity.
    // Timestamps are milliseconds since epoch (i64), not Timestamp.
    let last_series: Vec<(i64, Decimal)> = (0..10)
        .map(|i: i64| {
            (
                FIXED_EPOCH_SECS * 1000 + i * 3_600_000,
                dec!(100_000) + Decimal::from(i * 320),
            )
        })
        .collect();
    let last = RunReportMirror {
        tuple: tuple.clone(),
        equity_series: Arc::new(last_series),
        kpis: BacktestKpis {
            final_equity: Money::<Usdt>::from_decimal(dec!(103_200)),
            initial_equity: Money::<Usdt>::from_decimal(dec!(100_000)),
            max_drawdown: dec!(0.08),
            trade_count: 12,
            total_fees: Money::<Usdt>::from_decimal(dec!(4.80)),
            buys: 8,
            sells: 4,
            total_return_pct: dec!(0.032),
        },
        generated_at: OffsetDateTime::UNIX_EPOCH,
        bars: Arc::new(Vec::new()),
        position_curve: Arc::new(Vec::new()),
    };

    // Prev run: +$1 200 P&L, 14% max DD, slower rise.
    let prev_series: Vec<(i64, Decimal)> = (0..10)
        .map(|i: i64| {
            (
                FIXED_EPOCH_SECS * 1000 + i * 3_600_000,
                dec!(100_000) + Decimal::from(i * 120),
            )
        })
        .collect();
    let prev = RunReportMirror {
        tuple,
        equity_series: Arc::new(prev_series),
        kpis: BacktestKpis {
            final_equity: Money::<Usdt>::from_decimal(dec!(101_200)),
            initial_equity: Money::<Usdt>::from_decimal(dec!(100_000)),
            max_drawdown: dec!(0.14),
            trade_count: 8,
            total_fees: Money::<Usdt>::from_decimal(dec!(3.20)),
            buys: 5,
            sells: 3,
            total_return_pct: dec!(0.012),
        },
        generated_at: OffsetDateTime::UNIX_EPOCH,
        bars: Arc::new(Vec::new()),
        position_curve: Arc::new(Vec::new()),
    };

    (last, prev)
}

// ── advisor-leaderboard-screen v0.1.0 — bake-off leaderboard fixtures ─────────

/// A populated, deterministic `BakeoffReportMirror` for the Leaderboard screen.
///
/// The **full 13-arm advisor field** post-ADR-0067 (advisor-combination-search):
/// 4 single rule engines + 8 vote ensembles (the 2 F8 arms + the 6 new
/// combination arms) + the buy-and-hold benchmark, over BTCUSDT / 2024 H1, ranked
/// best-first. `v0.sma` is crowned (`ActiveWins`, robust); `v0.buyhold` is the
/// benchmark; several arms are Fragile (incl. multiple ensembles — exercising the
/// warn/badge tag), and the `tr_mr_sma_bb` `Unanimous{n:2}` decorrelation pair
/// traded ZERO times (its strict trend ∧ band-reversion consensus was never
/// reached — the OQ-3 "sat in cash" honesty case, rendered truthfully, NOT masked).
/// Realistic-ish numbers so the rendered table reads like a real 13-arm bake-off,
/// but fixed (no RNG) so the render guard is stable.
///
/// Insertion order = field order: the 4 singles, then `default_ensemble_field()`
/// (`majority`, `unanimous`, `trend_pair`, `tr_mr_macd_rsi`, `tr_mr_sma_bb`,
/// `any1of4`, `k2of4`, `k3of4`), then buy-and-hold appended by `run_bakeoff`.
///
/// Built directly as the mirror type — fixtures NEVER stand up the engine; the
/// mirror is the whole point of the `ui`-pure seam.
#[must_use]
#[allow(clippy::too_many_lines)] // a 13-row literal data table — splitting it hurts readability
pub fn fake_bakeoff_report_mirror() -> crate::leaderboard::BakeoffReportMirror {
    use crate::leaderboard::state::{
        BakeoffReportMirror, LeaderRow, OutcomeKind, ReasonLabel, RecommendationMirror,
        RobustnessLabel,
    };

    // Rows in INSERTION order (= field order). 13 arms: 4 singles, 8 ensembles,
    // benchmark. `v0.sma` is the crowned robust single; the ensembles carry an
    // honest mix of flags (most Fragile on real crypto — OQ-3), one trades 0×.
    let rows = vec![
        // ── 4 single rule engines ────────────────────────────────────────────
        LeaderRow {
            strategy: SmolStr::new("v0.sma"),
            is_benchmark: false,
            sharpe: 1.42,
            sortino: 1.95,
            calmar: 2.32,
            total_return_pct: dec!(0.1837),
            max_drawdown: dec!(0.0612),
            trade_count: 38,
            // The crowned arm — robust under resampling.
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.macd"),
            is_benchmark: false,
            sharpe: 0.88,
            sortino: 1.11,
            calmar: 0.84,
            total_return_pct: dec!(0.0921),
            max_drawdown: dec!(0.1043),
            trade_count: 64,
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.rsi"),
            is_benchmark: false,
            sharpe: -0.31,
            sortino: -0.43,
            calmar: -0.24,
            total_return_pct: dec!(-0.0457),
            max_drawdown: dec!(0.1872),
            trade_count: 112,
            // A fragile single loser — exercises the warn tag in the table.
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.bbands"),
            is_benchmark: false,
            sharpe: 0.54,
            sortino: 0.71,
            calmar: 0.42,
            total_return_pct: dec!(0.0388),
            max_drawdown: dec!(0.0921),
            trade_count: 47,
            robustness: Some(RobustnessLabel::Marginal),
        },
        // ── 8 vote ensembles (2 F8 + 6 combination-search, ADR-0067) ─────────
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.majority"),
            is_benchmark: false,
            // A high realized Sharpe but FRAGILE under resampling → ranked but
            // NOT crown-eligible (the F8 credibility lock; a Fragile ensemble
            // badge in the strategy column).
            sharpe: 1.61,
            sortino: 2.14,
            calmar: 2.04,
            total_return_pct: dec!(0.2104),
            max_drawdown: dec!(0.0788),
            trade_count: 29,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.unanimous"),
            is_benchmark: false,
            sharpe: 0.67,
            sortino: 0.83,
            calmar: 1.07,
            total_return_pct: dec!(0.0573),
            max_drawdown: dec!(0.0534),
            // Trades rarely (4-of-4 agreement is rare) but robust.
            trade_count: 9,
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.trend_pair"),
            is_benchmark: false,
            // The predicted-null control (both members trend) — little
            // decorrelation lift, Fragile under resampling (OQ-3 / the experiment
            // prediction). An honest near-zero edge.
            sharpe: 0.31,
            sortino: 0.39,
            calmar: 0.21,
            total_return_pct: dec!(0.0188),
            max_drawdown: dec!(0.0892),
            trade_count: 21,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.tr_mr_macd_rsi"),
            is_benchmark: false,
            // Trend ∧ mean-revert: simultaneous-Long is rare → sits MOSTLY FLAT,
            // low trade count, near-zero return, Fragile. HONEST (OQ-3), not a bug
            // — rendered as-is (the B1 "sat in cash" copy fires only at 0 trades;
            // this one traded a handful of times, so it's a bare low row).
            sharpe: 0.08,
            sortino: 0.10,
            calmar: 0.05,
            total_return_pct: dec!(0.0041),
            max_drawdown: dec!(0.0413),
            trade_count: 4,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.tr_mr_sma_bb"),
            is_benchmark: false,
            // The second trend ∧ band-reversion pair — its strict consensus was
            // NEVER reached on this window → ZERO trades, sat in cash the whole
            // time (the OQ-3 / U3 "sat in cash — consensus never reached" note).
            // A flat Sharpe-0 / 0-return / 0-DD row that is NOT a failure.
            sharpe: 0.0,
            sortino: 0.0,
            calmar: 0.0,
            total_return_pct: dec!(0.0),
            max_drawdown: dec!(0.0),
            trade_count: 0,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.any1of4"),
            is_benchmark: false,
            // The loosest k-ladder rung (long if ANY of 4 fires) — most exposure,
            // most trades, lowest decorrelation benefit → Fragile.
            sharpe: 0.44,
            sortino: 0.56,
            calmar: 0.33,
            total_return_pct: dec!(0.0312),
            max_drawdown: dec!(0.1207),
            trade_count: 88,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.k2of4"),
            is_benchmark: false,
            // The balanced quorum rung — a moderate edge, marginal under
            // resampling (not Fragile, not clearly Robust).
            sharpe: 0.61,
            sortino: 0.78,
            calmar: 0.49,
            total_return_pct: dec!(0.0466),
            max_drawdown: dec!(0.0954),
            trade_count: 33,
            robustness: Some(RobustnessLabel::Marginal),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.k3of4"),
            is_benchmark: false,
            // The strict rung (broad agreement) — tightest spread, fewest trades;
            // here it clears the gate (robust) but with a modest Sharpe.
            sharpe: 0.58,
            sortino: 0.72,
            calmar: 0.86,
            total_return_pct: dec!(0.0421),
            max_drawdown: dec!(0.0489),
            trade_count: 14,
            robustness: Some(RobustnessLabel::Robust),
        },
        // ── buy-and-hold benchmark (appended by run_bakeoff) ─────────────────
        LeaderRow {
            strategy: SmolStr::new("v0.buyhold"),
            is_benchmark: true,
            sharpe: 0.73,
            sortino: 0.89,
            calmar: 0.84,
            total_return_pct: dec!(0.1124),
            max_drawdown: dec!(0.1338),
            trade_count: 2,
            robustness: Some(RobustnessLabel::Robust),
        },
    ];

    // Best-first ranked order: CROWN-ELIGIBLE (non-Fragile) arms first by Sharpe,
    // then the Fragile arms ranked after (shown but ineligible-to-crown — they
    // cannot jump the crown even with a higher realized Sharpe, the F8 lock).
    //   indices: 0=sma 1=macd 2=rsi 3=bbands 4=majority 5=unanimous
    //            6=trend_pair 7=tr_mr_macd_rsi 8=tr_mr_sma_bb 9=any1of4
    //            10=k2of4 11=k3of4 12=buyhold
    // Eligible by Sharpe: sma(1.42) > buyhold(0.73) > macd(0.88?)… — order the
    // eligible set strictly by Sharpe: sma(1.42) > macd(0.88) > buyhold(0.73) >
    // unanimous(0.67) > k2of4(0.61) > k3of4(0.58) > bbands(0.54).
    // Fragile (ranked last, cannot be crowned), by Sharpe: majority(1.61) >
    // any1of4(0.44) > trend_pair(0.31) > tr_mr_macd_rsi(0.08) > tr_mr_sma_bb(0.0)
    // > rsi(-0.31).
    let ranked = vec![0, 1, 12, 5, 10, 11, 3, 4, 9, 6, 7, 8, 2];

    BakeoffReportMirror {
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        rows,
        ranked,
        crowned: Some(0),
        recommendation: RecommendationMirror {
            outcome: OutcomeKind::ActiveWins,
            winner: SmolStr::new("v0.sma"),
            winner_robustness: Some(RobustnessLabel::Robust),
            reasons: vec![
                ReasonLabel::HighestRobustSharpe,
                ReasonLabel::BeatBenchmarkSharpe,
            ],
        },
    }
}

/// A SHORT-AUGMENTED advisor field (advisor-short-selling, ADR-0068 § D9 —
/// T-U2): the 4 long-only single rule engines + buy-and-hold, PLUS the FIXED
/// 5-arm short slate (`sma_cross_ls`, `macd_ls`, `rsi_ls`, `bbands_ls`,
/// `always_short`). Drives the leaderboard render guard that proves the short
/// arms render with FRIENDLY directional labels (NOT raw `*_ls` ids) + the
/// `short` tag + a (likely) Fragile flag, plus the short field-note + the
/// unbounded-loss disclaimer.
///
/// The shorts are mostly FRAGILE (the honest null — the MN long/short precedent
/// was FAMILY-UNIFORM-FRAGILE; single-coin directional shorts inherit full
/// inverse beta + a funding cost), and `always_short` carries a BRUTAL drawdown
/// (a continuous short on an up-trending window — the honest signal). `v0.sma`
/// stays the crowned robust long single, so the short arms are an additive,
/// honestly-rendered field, never a crowned alpha claim. Built directly as the
/// mirror type — fixtures NEVER stand up the engine.
#[must_use]
#[allow(clippy::too_many_lines)] // a 10-row literal data table — splitting it hurts readability
pub fn fake_bakeoff_report_mirror_with_shorts() -> crate::leaderboard::BakeoffReportMirror {
    use crate::leaderboard::state::{
        BakeoffReportMirror, LeaderRow, OutcomeKind, ReasonLabel, RecommendationMirror,
        RobustnessLabel,
    };

    // Rows in INSERTION order: the 4 long-only singles, the 5 short arms, then
    // buy-and-hold. `v0.sma` is the crowned robust long single; the short arms
    // carry an honest mix (mostly Fragile; always_short brutally so).
    let rows = vec![
        // ── 4 long-only single rule engines (UNTOUCHED long-only arms) ────────
        LeaderRow {
            strategy: SmolStr::new("v0.sma"),
            is_benchmark: false,
            sharpe: 1.42,
            sortino: 1.95,
            calmar: 2.32,
            total_return_pct: dec!(0.1837),
            max_drawdown: dec!(0.0612),
            trade_count: 38,
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.macd"),
            is_benchmark: false,
            sharpe: 0.88,
            sortino: 1.11,
            calmar: 0.84,
            total_return_pct: dec!(0.0921),
            max_drawdown: dec!(0.1043),
            trade_count: 64,
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.rsi"),
            is_benchmark: false,
            sharpe: -0.31,
            sortino: -0.43,
            calmar: -0.24,
            total_return_pct: dec!(-0.0457),
            max_drawdown: dec!(0.1872),
            trade_count: 112,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.bbands"),
            is_benchmark: false,
            sharpe: 0.54,
            sortino: 0.71,
            calmar: 0.42,
            total_return_pct: dec!(0.0388),
            max_drawdown: dec!(0.0921),
            trade_count: 47,
            robustness: Some(RobustnessLabel::Marginal),
        },
        // ── the FIXED 5-arm short slate (ADR-0068 § D9) ──────────────────────
        LeaderRow {
            strategy: SmolStr::new("sma_cross_ls"),
            is_benchmark: false,
            // A directional short variant — modest realized edge but FRAGILE
            // under resampling (the honest null).
            sharpe: 0.36,
            sortino: 0.45,
            calmar: 0.28,
            total_return_pct: dec!(0.0241),
            max_drawdown: dec!(0.1418),
            trade_count: 52,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("macd_ls"),
            is_benchmark: false,
            sharpe: 0.22,
            sortino: 0.28,
            calmar: 0.16,
            total_return_pct: dec!(0.0153),
            max_drawdown: dec!(0.1622),
            trade_count: 78,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("rsi_ls"),
            is_benchmark: false,
            // A losing directional short on this window.
            sharpe: -0.48,
            sortino: -0.62,
            calmar: -0.34,
            total_return_pct: dec!(-0.0712),
            max_drawdown: dec!(0.2238),
            trade_count: 134,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("bbands_ls"),
            is_benchmark: false,
            sharpe: 0.18,
            sortino: 0.23,
            calmar: 0.12,
            total_return_pct: dec!(0.0108),
            max_drawdown: dec!(0.1534),
            trade_count: 61,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("always_short"),
            is_benchmark: false,
            // The always-short control — loses on this up-trending window by
            // construction, with a BRUTAL drawdown (the honest signal). A
            // continuous short on a rising asset is a guaranteed loser; the
            // leaderboard says so.
            sharpe: -1.12,
            sortino: -1.38,
            calmar: -0.71,
            total_return_pct: dec!(-0.2884),
            max_drawdown: dec!(0.4173),
            trade_count: 1,
            robustness: Some(RobustnessLabel::Fragile),
        },
        // ── buy-and-hold benchmark (appended by run_bakeoff) ─────────────────
        LeaderRow {
            strategy: SmolStr::new("v0.buyhold"),
            is_benchmark: true,
            sharpe: 0.73,
            sortino: 0.89,
            calmar: 0.84,
            total_return_pct: dec!(0.1124),
            max_drawdown: dec!(0.1338),
            trade_count: 2,
            robustness: Some(RobustnessLabel::Robust),
        },
    ];

    // Best-first ranked order. Crown-eligible (non-Fragile) first by Sharpe:
    //   0=sma(1.42) 1=macd(0.88) 9=buyhold(0.73) 3=bbands(0.54-marginal).
    // Fragile (ranked last, cannot be crowned), by Sharpe:
    //   4=sma_cross_ls(0.36) 5=macd_ls(0.22) 7=bbands_ls(0.18)
    //   2=rsi(-0.31) 6=rsi_ls(-0.48) 8=always_short(-1.12).
    let ranked = vec![0, 1, 9, 3, 4, 5, 7, 2, 6, 8];

    BakeoffReportMirror {
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        rows,
        ranked,
        crowned: Some(0),
        recommendation: RecommendationMirror {
            outcome: OutcomeKind::ActiveWins,
            winner: SmolStr::new("v0.sma"),
            winner_robustness: Some(RobustnessLabel::Robust),
            reasons: vec![
                ReasonLabel::HighestRobustSharpe,
                ReasonLabel::BeatBenchmarkSharpe,
            ],
        },
    }
}

/// The original **5-arm** advisor field (4 rule engines + buy-and-hold, ONE
/// Fragile single `v0.5.rsi`) — kept as the smaller baseline for the F8
/// anti-tautology discriminator (`leaderboard_f8_strictly_exceeds_five_arm_field`)
/// now that [`fake_bakeoff_report_mirror`] grew to the full 13-arm field
/// (ADR-0067). It carries no ensembles, so the F8 7-arm field (which adds two
/// ensemble rows incl. a second Fragile badge) provably paints strictly more
/// strategy-column Fragile clay + foreground than this. `v0.sma` crowned,
/// `ActiveWins`. Built directly as the mirror type — fixtures NEVER stand up the
/// engine.
#[must_use]
pub fn fake_bakeoff_report_mirror_five_arm() -> crate::leaderboard::BakeoffReportMirror {
    use crate::leaderboard::state::{
        BakeoffReportMirror, LeaderRow, OutcomeKind, ReasonLabel, RecommendationMirror,
        RobustnessLabel,
    };

    // Rows in INSERTION order (= field order: sma, macd, rsi, bbands, buyhold).
    let rows = vec![
        LeaderRow {
            strategy: SmolStr::new("v0.sma"),
            is_benchmark: false,
            sharpe: 1.42,
            sortino: 1.95,
            calmar: 2.32,
            total_return_pct: dec!(0.1837),
            max_drawdown: dec!(0.0612),
            trade_count: 38,
            robustness: None,
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.macd"),
            is_benchmark: false,
            sharpe: 0.88,
            sortino: 1.11,
            calmar: 0.84,
            total_return_pct: dec!(0.0921),
            max_drawdown: dec!(0.1043),
            trade_count: 64,
            robustness: None,
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.rsi"),
            is_benchmark: false,
            sharpe: -0.31,
            sortino: -0.43,
            calmar: -0.24,
            total_return_pct: dec!(-0.0457),
            max_drawdown: dec!(0.1872),
            trade_count: 112,
            // A fragile loser — exercises the warn tag in the table.
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.bbands"),
            is_benchmark: false,
            sharpe: 0.54,
            sortino: 0.71,
            calmar: 0.42,
            total_return_pct: dec!(0.0388),
            max_drawdown: dec!(0.0921),
            trade_count: 47,
            robustness: None,
        },
        LeaderRow {
            strategy: SmolStr::new("v0.buyhold"),
            is_benchmark: true,
            sharpe: 0.73,
            sortino: 0.89,
            calmar: 0.84,
            total_return_pct: dec!(0.1124),
            max_drawdown: dec!(0.1338),
            trade_count: 2,
            robustness: None,
        },
    ];

    // Best-first ranked order: sma(1.42) > macd(0.88) > buyhold(0.73) >
    // bbands(0.54) > rsi(-0.31). Indices into `rows`.
    let ranked = vec![0, 1, 4, 3, 2];

    BakeoffReportMirror {
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        rows,
        ranked,
        crowned: Some(0),
        recommendation: RecommendationMirror {
            outcome: OutcomeKind::ActiveWins,
            winner: SmolStr::new("v0.sma"),
            winner_robustness: None,
            reasons: vec![
                ReasonLabel::HighestRobustSharpe,
                ReasonLabel::BeatBenchmarkSharpe,
            ],
        },
    }
}

/// A `BakeoffReportMirror` where buy-and-hold won (`BenchmarkWins`) — exercises
/// the "Nothing beat simply holding BTCUSDT…" headline branch. Two rows kept
/// minimal; buy-and-hold crowned.
#[must_use]
pub fn fake_bakeoff_report_mirror_benchmark_wins() -> crate::leaderboard::BakeoffReportMirror {
    use crate::leaderboard::state::{
        BakeoffReportMirror, LeaderRow, OutcomeKind, ReasonLabel, RecommendationMirror,
    };

    let rows = vec![
        LeaderRow {
            strategy: SmolStr::new("v0.sma"),
            is_benchmark: false,
            sharpe: 0.21,
            sortino: 0.28,
            calmar: 0.14,
            total_return_pct: dec!(0.0143),
            max_drawdown: dec!(0.1521),
            trade_count: 41,
            robustness: None,
        },
        LeaderRow {
            strategy: SmolStr::new("v0.buyhold"),
            is_benchmark: true,
            sharpe: 0.69,
            sortino: 0.85,
            calmar: 0.84,
            total_return_pct: dec!(0.1124),
            max_drawdown: dec!(0.1338),
            trade_count: 2,
            robustness: None,
        },
    ];
    BakeoffReportMirror {
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        rows,
        ranked: vec![1, 0],
        crowned: Some(1),
        recommendation: RecommendationMirror {
            outcome: OutcomeKind::BenchmarkWins,
            winner: SmolStr::new("v0.buyhold"),
            winner_robustness: None,
            reasons: vec![ReasonLabel::BenchmarkUndefeated],
        },
    }
}

/// The HONEST real-crypto `BenchmarkWins` field (B1 / ADR-0066) — the full
/// 7-arm advisor field as it actually lands on a single volatile asset where no
/// active strategy clears the robustness bar:
///
/// - **every ACTIVE arm is Fragile** (the honest "measured robustness, not
///   asserted alpha" truth on real crypto) — so none is crown-eligible;
/// - the **buy-and-hold benchmark is the top-Sharpe arm and is CROWNED**
///   (`BenchmarkWins`, reason `BenchmarkUndefeated`) — the baseline won because
///   nothing active was robust, NOT because everything is broken;
/// - the benchmark is *itself* Fragile (p5-Sharpe < 0 under resampling on a
///   60-70%-vol single asset — the near-certain binding signal, ADR-0066 § D3)
///   yet still crowned — exercising the **informational** benchmark-robustness
///   note (the quiet "baseline is path-dependent" word), NOT the disqualifying
///   badge an active arm gets;
/// - the `v0.8.vote.unanimous` ensemble has **ZERO trades** (4-of-4 agreement
///   never reached on a single asset) — exercising the U3 "sat in cash —
///   consensus never reached" note instead of a bare Sharpe-0 row.
///
/// This is the fixture the `benchmark_wins_render` guard drives: it paints the
/// honest copy, the crowned baseline row, the informational benchmark-Fragile
/// note, and the "sat in cash" ensemble note together. Built directly as the
/// mirror type — fixtures NEVER stand up the engine.
#[must_use]
pub fn fake_bakeoff_report_mirror_benchmark_wins_full() -> crate::leaderboard::BakeoffReportMirror {
    use crate::leaderboard::state::{
        BakeoffReportMirror, LeaderRow, OutcomeKind, ReasonLabel, RecommendationMirror,
        RobustnessLabel,
    };

    // Rows in INSERTION order (field order: 4 singles, 2 ensembles, benchmark).
    // EVERY active arm is Fragile; buy-and-hold has the top Sharpe (0.69) and is
    // crowned. The unanimous ensemble trades 0 times (consensus never reached).
    let rows = vec![
        LeaderRow {
            strategy: SmolStr::new("v0.sma"),
            is_benchmark: false,
            sharpe: 0.34,
            sortino: 0.41,
            calmar: 0.22,
            total_return_pct: dec!(0.0218),
            max_drawdown: dec!(0.1731),
            trade_count: 44,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.macd"),
            is_benchmark: false,
            sharpe: 0.12,
            sortino: 0.15,
            calmar: 0.08,
            total_return_pct: dec!(0.0094),
            max_drawdown: dec!(0.1442),
            trade_count: 71,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.rsi"),
            is_benchmark: false,
            sharpe: -0.27,
            sortino: -0.36,
            calmar: -0.19,
            total_return_pct: dec!(-0.0381),
            max_drawdown: dec!(0.1903),
            trade_count: 118,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.bbands"),
            is_benchmark: false,
            sharpe: 0.19,
            sortino: 0.24,
            calmar: 0.13,
            total_return_pct: dec!(0.0152),
            max_drawdown: dec!(0.1288),
            trade_count: 52,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.majority"),
            is_benchmark: false,
            sharpe: 0.41,
            sortino: 0.52,
            calmar: 0.29,
            total_return_pct: dec!(0.0307),
            max_drawdown: dec!(0.1104),
            trade_count: 23,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.unanimous"),
            is_benchmark: false,
            // ZERO trades — 4-of-4 agreement never reached on a single asset, so
            // it sat in cash the whole window (the U3 "sat in cash" note). A flat
            // Sharpe-0 / 0-return / 0-drawdown row that is NOT a failure.
            sharpe: 0.0,
            sortino: 0.0,
            calmar: 0.0,
            total_return_pct: dec!(0.0),
            max_drawdown: dec!(0.0),
            trade_count: 0,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.buyhold"),
            is_benchmark: true,
            // The top Sharpe in the field → crowned. Itself Fragile (the baseline
            // is path-dependent on a single volatile asset) — exercises the
            // INFORMATIONAL benchmark-robustness note, not the disqualifying badge.
            sharpe: 0.69,
            sortino: 0.85,
            calmar: 0.84,
            total_return_pct: dec!(0.1124),
            max_drawdown: dec!(0.1338),
            trade_count: 2,
            robustness: Some(RobustnessLabel::Fragile),
        },
    ];

    // Best-first by Sharpe: buyhold(0.69) > majority(0.41) > sma(0.34) >
    // bbands(0.19) > macd(0.12) > unanimous(0.0) > rsi(-0.27). The benchmark
    // (index 6) is crown-eligible per ADR-0066 § D2 and tops the field.
    //   0=sma, 1=macd, 2=rsi, 3=bbands, 4=majority, 5=unanimous, 6=buyhold.
    let ranked = vec![6, 4, 0, 3, 1, 5, 2];

    BakeoffReportMirror {
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        rows,
        ranked,
        crowned: Some(6),
        recommendation: RecommendationMirror {
            outcome: OutcomeKind::BenchmarkWins,
            winner: SmolStr::new("v0.buyhold"),
            // The benchmark's own flag is shown on its row but the recommendation
            // does NOT echo a winner-robustness clause for the baseline (it is
            // exempt from the candidate verdict — ADR-0066 § D3); leave it None so
            // no "but it looked fragile" clause fires for the crowned baseline.
            winner_robustness: None,
            reasons: vec![ReasonLabel::BenchmarkUndefeated],
        },
    }
}

/// A populated 7-arm `BakeoffReportMirror` WITH the two F8 ensemble candidates
/// (ADR-0063) — the full advisor field: 4 singles + 2 vote ensembles +
/// buy-and-hold, with the robustness gate LIVE so flags are populated.
///
/// The arm set + outcome:
/// - `v0.sma` — crowned (`ActiveWins`), robust.
/// - `v0.8.vote.majority` — a 2-of-3 majority-vote ensemble; **FRAGILE** under
///   resampling, so it is shown ranked among the field but **cannot be
///   crowned** (the credibility lock — the first time the Fragile state is
///   non-inert). This is the render guard's load-bearing case: an ensemble row
///   AND a visible Fragile badge.
/// - `v0.8.vote.unanimous` — a 4-of-4 unanimous-vote ensemble; robust but
///   lower Sharpe (trades rarely).
/// - `v0.5.macd` / `v0.5.bbands` — robust singles.
/// - `v0.5.rsi` — a fragile single loser (also exercises the warn tag).
/// - `v0.buyhold` — the benchmark.
///
/// Built directly as the mirror type — fixtures NEVER stand up the engine.
#[must_use]
pub fn fake_bakeoff_report_mirror_with_ensembles() -> crate::leaderboard::BakeoffReportMirror {
    use crate::leaderboard::state::{
        BakeoffReportMirror, LeaderRow, OutcomeKind, ReasonLabel, RecommendationMirror,
        RobustnessLabel,
    };

    // Rows in INSERTION order (= field order: the 4 singles, the 2 ensembles,
    // then the buy-and-hold benchmark — `default_field() ∪
    // default_ensemble_field()` ∪ buyhold, ADR-0063 § D5).
    let rows = vec![
        LeaderRow {
            strategy: SmolStr::new("v0.sma"),
            is_benchmark: false,
            sharpe: 1.42,
            sortino: 1.95,
            calmar: 2.32,
            total_return_pct: dec!(0.1837),
            max_drawdown: dec!(0.0612),
            trade_count: 38,
            // The crowned arm — robust under resampling (the gate is LIVE).
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.macd"),
            is_benchmark: false,
            sharpe: 0.88,
            sortino: 1.11,
            calmar: 0.84,
            total_return_pct: dec!(0.0921),
            max_drawdown: dec!(0.1043),
            trade_count: 64,
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.rsi"),
            is_benchmark: false,
            sharpe: -0.31,
            sortino: -0.43,
            calmar: -0.24,
            total_return_pct: dec!(-0.0457),
            max_drawdown: dec!(0.1872),
            trade_count: 112,
            // A fragile single loser.
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.5.bbands"),
            is_benchmark: false,
            sharpe: 0.54,
            sortino: 0.71,
            calmar: 0.42,
            total_return_pct: dec!(0.0388),
            max_drawdown: dec!(0.0921),
            trade_count: 47,
            robustness: Some(RobustnessLabel::Marginal),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.majority"),
            is_benchmark: false,
            // A high realized Sharpe — but FRAGILE under resampling, so it
            // CANNOT be crowned (it would out-Sharpe v0.sma on the realized
            // path, which is exactly why the Fragile gate must bite + be
            // visible). This is the easiest-to-overfit candidate; the gate
            // rejecting it is the whole point of F8.
            sharpe: 1.61,
            sortino: 2.14,
            calmar: 2.04,
            total_return_pct: dec!(0.2104),
            max_drawdown: dec!(0.0788),
            trade_count: 29,
            robustness: Some(RobustnessLabel::Fragile),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.8.vote.unanimous"),
            is_benchmark: false,
            sharpe: 0.67,
            sortino: 0.83,
            calmar: 1.07,
            total_return_pct: dec!(0.0573),
            max_drawdown: dec!(0.0534),
            // Trades rarely (4-of-4 agreement is rare).
            trade_count: 9,
            robustness: Some(RobustnessLabel::Robust),
        },
        LeaderRow {
            strategy: SmolStr::new("v0.buyhold"),
            is_benchmark: true,
            sharpe: 0.73,
            sortino: 0.89,
            calmar: 0.84,
            total_return_pct: dec!(0.1124),
            max_drawdown: dec!(0.1338),
            trade_count: 2,
            robustness: Some(RobustnessLabel::Robust),
        },
    ];

    // Best-first ranked order among ELIGIBLE (non-fragile) arms first, then the
    // fragile arms ranked after (shown but ineligible-to-crown). Eligible by
    // Sharpe: sma(1.42) > macd(0.88) > buyhold(0.73) > unanimous(0.67) >
    // bbands(0.54). Fragile (ranked last, cannot be crowned): majority(1.61),
    // rsi(-0.31). Indices into `rows`:
    //   0=sma, 1=macd, 2=rsi, 3=bbands, 4=majority, 5=unanimous, 6=buyhold.
    let ranked = vec![0, 1, 6, 5, 3, 4, 2];

    BakeoffReportMirror {
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        rows,
        ranked,
        crowned: Some(0),
        recommendation: RecommendationMirror {
            outcome: OutcomeKind::ActiveWins,
            winner: SmolStr::new("v0.sma"),
            winner_robustness: Some(RobustnessLabel::Robust),
            reasons: vec![
                ReasonLabel::HighestRobustSharpe,
                ReasonLabel::BeatBenchmarkSharpe,
            ],
        },
    }
}

/// A `Cockpit` routed to `Screen::Leaderboard` with the supplied result state
/// installed. Synthetic — no engine, no I/O; the render guard drives this.
#[must_use]
pub fn fake_cockpit_leaderboard(
    result: PanelState<crate::leaderboard::BakeoffReportMirror>,
) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::Leaderboard;
    cockpit.leaderboard_screen_state = crate::leaderboard::LeaderboardScreenState {
        result,
        running: false,
        // F3 guided-input defaults (BTCUSDT / €200 / 2024 H1) — the default
        // cockpit start state, used by the negative-control render path.
        ..Default::default()
    };
    cockpit
}

/// A `Cockpit` routed to `Screen::Leaderboard` with an EXPLICIT F3 guided-input
/// selection (coin + budget + lookback) installed alongside the result state.
///
/// Drives the render guard that proves the guided-input controls + the
/// budget-context header paint with a NON-default selection (a chosen coin +
/// budget + lookback) — so the assertion is not satisfied by the defaults
/// alone. Synthetic — no engine, no I/O.
#[must_use]
pub fn fake_cockpit_leaderboard_with_input(
    result: PanelState<crate::leaderboard::BakeoffReportMirror>,
    coin: &str,
    budget_input: &str,
    lookback: crate::leaderboard::LeaderboardLookback,
) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::Leaderboard;
    cockpit.leaderboard_screen_state = crate::leaderboard::LeaderboardScreenState {
        result,
        running: false,
        coin: Symbol::new(coin),
        budget_input: budget_input.to_string(),
        lookback,
        ..Default::default()
    };
    cockpit
}

/// advisor-bakeoff-progress — a `Cockpit` routed to `Screen::Leaderboard` with a
/// bake-off IN FLIGHT and a candidate-level `BakeoffProgress` set, so the
/// DETERMINATE progress bar beneath the input panel renders "Running {id} —
/// {done+1} of {total}" filled `done / total`.
///
/// `result` is `Loading` (the in-flight result state) + `running = true` (the
/// bar's gate) + the supplied progress event. Drives the bake-off progress
/// render guard. Synthetic — no engine, no I/O, no channel (the progress is set
/// directly; the channel→state path is proved by `bakeoff_progress_relay.rs`).
#[must_use]
pub fn fake_cockpit_leaderboard_running_progress(
    done: u16,
    total: u16,
    current_id: &str,
) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::Leaderboard;
    cockpit.leaderboard_screen_state = crate::leaderboard::LeaderboardScreenState {
        result: PanelState::Loading,
        running: true,
        progress: Some(backtest::progress::BakeoffProgress {
            done,
            total,
            current_id: SmolStr::new(current_id),
        }),
        ..Default::default()
    };
    cockpit
}

// ── advisor-llm-narration F9 (ADR-0064) — the opt-in "why this one" narration ──

/// A FAITHFUL fixture narration prose for the `Ready` render state — a
/// plain-language summary of `fake_bakeoff_report_mirror()` (winner `v0.sma`,
/// `ActiveWins`, Sharpe 1.42 vs buy-and-hold 0.73). NO `llm`/network: this is
/// the canned text the render harness drops straight into
/// [`NarrationState::Ready`], standing in for what the agent's
/// `generate_narration` would return after passing the faithfulness post-check.
///
/// Deliberately faithful — it names the crowned strategy, states the outcome
/// correctly, references only KPIs visible in the table, and trips no
/// banned-phrase / prediction language — so the rendered PNG reads as the honest
/// summary the operator would actually see (not a fabrication).
pub const FAKE_NARRATION_READY_PROSE: &str = "SMA crossover came out on top here. Over this window it earned the strongest \
     risk-adjusted return of the field \u{2014} a Sharpe of 1.42 against 0.73 for simply \
     holding the coin \u{2014} while keeping its worst drawdown shallower than the other \
     strategies. The RSI strategy looked fragile under resampling, so it could not be \
     crowned even where its raw numbers tempted. This describes how the strategies \
     behaved on past data; it is not a forecast.";

/// A `Cockpit` routed to `Screen::Leaderboard` with a populated leaderboard AND
/// an explicit F9 [`NarrationState`] installed on the recommendation block.
///
/// Drives the F9 render guards: `Ready(prose)` paints the LLM prose card;
/// `NotRequested` / `FellBack` paint the templated reasons (the negative
/// control). Synthetic — no engine, no `llm`, no network; the narration is the
/// canned `ui` fixture text, exactly the seam the ADR § D5 fake-provider
/// pattern reserves for the render harness.
#[must_use]
pub fn fake_cockpit_leaderboard_with_narration(
    result: PanelState<crate::leaderboard::BakeoffReportMirror>,
    narration: crate::leaderboard::NarrationState,
) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::Leaderboard;
    cockpit.leaderboard_screen_state = crate::leaderboard::LeaderboardScreenState {
        result,
        running: false,
        narration,
        ..Default::default()
    };
    cockpit
}

// ── advisor-forward-plan v0.1.0 (roadmap F6) — forward-plan fixtures ───────────

/// A populated, deterministic ACTIVE-strategy [`ForwardPlanView`] for the
/// Forward-plan screen — an SMA pick currently FLAT, with the IF/THEN standing
/// rules and the €200 projected next-BUY sizing. This is the render guard's
/// POSITIVE case (a full conditional plan).
///
/// Built directly as the view type — fixtures NEVER stand up the engine; the
/// mirror is the whole point of the `ui`-pure seam (the
/// `fake_bakeoff_report_mirror` precedent).
#[must_use]
pub fn fake_forward_plan() -> crate::forward_plan::ForwardPlanView {
    use crate::forward_plan::state::{
        ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView,
    };
    ForwardPlanView {
        strategy: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        // Currently FLAT — waiting for the entry cross (exercises the next-BUY
        // sizing line, the most informative case).
        stance: PlanStanceView::Flat,
        latest_signal: Some(PlanSignalView::Hold),
        rule: PlanRuleView::SmaCross {
            fast_len: 12,
            slow_len: 26,
        },
        last_close: dec!(64000.00),
        as_of_label: SmolStr::new("Jun 19 14:00"),
        budget: dec!(200),
        // 200 / 64000 = 0.003125 units.
        projected_units: dec!(0.003125),
        sizing_capped: false,
        horizon_days: 7,
        horizon_through_label: SmolStr::new("Jun 26"),
    }
}

/// An RSI-reversion [`ForwardPlanView`] fixture — the `btc_rsi_reversion` strategy.
///
/// Entry: RSI(14) < 30 (oversold). Exit: RSI climbs back above 30 (flip-to-false).
/// There is no overbought threshold (no RSI-70). Used by the render guard to verify
/// the faithful RSI rule copy (entry + flip-to-false exit, no fabricated 70) actually
/// paints in the cockpit Forward-plan screen.
#[must_use]
pub fn fake_forward_plan_rsi() -> crate::forward_plan::ForwardPlanView {
    use crate::forward_plan::state::{
        ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView,
    };
    ForwardPlanView {
        strategy: SmolStr::new("btc_rsi_reversion"),
        symbol: SmolStr::new("BTCUSDT"),
        // FLAT — oversold condition not currently met; waiting for an entry.
        stance: PlanStanceView::Flat,
        latest_signal: Some(PlanSignalView::Hold),
        rule: PlanRuleView::RsiReversion { len: 14, lower: 30 },
        last_close: dec!(64000.00),
        as_of_label: SmolStr::new("Jun 21 14:00"),
        budget: dec!(200),
        // 200 / 64000 = 0.003125 units.
        projected_units: dec!(0.003125),
        sizing_capped: false,
        horizon_days: 7,
        horizon_through_label: SmolStr::new("Jun 28"),
    }
}

/// The buy-and-hold degenerate [`ForwardPlanView`] (the `BenchmarkWins`
/// honesty branch) — stance LONG after the first buy, NO sell trigger, "buy
/// the full €200 now and hold the horizon". This is the render guard's
/// NEGATIVE CONTROL: it must read as obviously the same KIND of object as the
/// active plan but visibly DIFFER (no sell-rule line, no re-evaluation
/// cadence), proving the populated guard is not a tautology.
#[must_use]
pub fn fake_forward_plan_buy_and_hold() -> crate::forward_plan::ForwardPlanView {
    use crate::forward_plan::state::{ForwardPlanView, PlanRuleView, PlanStanceView};
    ForwardPlanView {
        strategy: SmolStr::new("v0.buyhold"),
        symbol: SmolStr::new("BTCUSDT"),
        // LONG after the first buy; no latest signal (no re-evaluation, D5).
        stance: PlanStanceView::Long,
        latest_signal: None,
        rule: PlanRuleView::BuyAndHold,
        last_close: dec!(64000.00),
        as_of_label: SmolStr::new("Jun 19 14:00"),
        budget: dec!(200),
        projected_units: dec!(0.003125),
        sizing_capped: false,
        horizon_days: 7,
        horizon_through_label: SmolStr::new("Jun 26"),
    }
}

/// An ENSEMBLE (signal-vote) [`ForwardPlanView`] fixture (F8 / ADR-0063 § D3) —
/// the `v0.8.vote.majority` candidate: a 2-of-3 majority vote over the MACD /
/// RSI / Bollinger member rules, currently LONG (the quorum is met). This is
/// the render guard's POSITIVE ensemble case: the plan must describe the VOTE
/// faithfully (method + members + live tally), NOT fabricate a single rule.
///
/// Built directly as the view type — fixtures NEVER stand up the engine; the
/// mirror is the whole point of the `ui`-pure seam. The `members` carry each
/// member's OWN rule shape so the per-member list renders honestly.
#[must_use]
pub fn fake_forward_plan_ensemble() -> crate::forward_plan::ForwardPlanView {
    use crate::forward_plan::state::{
        ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView, PlanVoteMethodView,
    };
    ForwardPlanView {
        strategy: SmolStr::new("v0.8.vote.majority"),
        symbol: SmolStr::new("BTCUSDT"),
        // LONG — the majority quorum (≥ 2 of 3) is currently met, so the
        // ensemble holds. Exercises the LONG-stance tally branch.
        stance: PlanStanceView::Long,
        latest_signal: Some(PlanSignalView::Hold),
        // 2-of-3 majority vote. Carries `method` + `members` display labels
        // (Task 3: agent::config::PlanRuleKind::Ensemble now carries Vec<SmolStr>).
        rule: PlanRuleView::Ensemble {
            method: PlanVoteMethodView::Majority { k: 2, n: 3 },
            members: vec![
                SmolStr::new_static("MACD trend"),
                SmolStr::new_static("RSI reversion"),
                SmolStr::new_static("Bollinger reversion"),
            ],
        },
        last_close: dec!(64000.00),
        as_of_label: SmolStr::new("Jun 21 14:00"),
        budget: dec!(200),
        // 200 / 64000 = 0.003125 units.
        projected_units: dec!(0.003125),
        sizing_capped: false,
        horizon_days: 7,
        horizon_through_label: SmolStr::new("Jun 28"),
    }
}

/// A crowned-COMBINATION (advisor-combination-search, ADR-0067) ensemble
/// [`ForwardPlanView`] — the `v0.8.vote.tr_mr_macd_rsi` arm: a `Unanimous{n:2}`
/// vote over the MACD-trend + RSI-reversion member rules, currently LONG (the
/// strict 2-of-2 consensus is met). This is the forward-plan render proof that a
/// NEW combination arm, when crowned/forward-planned, draws its rule HONESTLY via
/// the EXISTING `PlanRuleView::Ensemble` path — naming its members as a brace-list
/// ("Holds only while ALL of {MACD trend, RSI reversion} agree…") with NO new
/// render code (the same `ensemble_vote_clause` the F8 majority arm uses, just the
/// Unanimous branch + the new member pair).
///
/// Built directly as the view type — fixtures NEVER stand up the engine; the
/// `members` carry the two member display labels (the F6 enrichment) so the
/// per-member naming renders truthfully.
#[must_use]
pub fn fake_forward_plan_combination() -> crate::forward_plan::ForwardPlanView {
    use crate::forward_plan::state::{
        ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView, PlanVoteMethodView,
    };
    ForwardPlanView {
        strategy: SmolStr::new("v0.8.vote.tr_mr_macd_rsi"),
        symbol: SmolStr::new("BTCUSDT"),
        // LONG — the strict 2-of-2 unanimous consensus is currently met (a trend-up
        // AND an oversold-bounce agree). Exercises the LONG-stance tally branch.
        stance: PlanStanceView::Long,
        latest_signal: Some(PlanSignalView::Hold),
        // 2-of-2 UNANIMOUS vote (the trend ∧ mean-revert decorrelation pair).
        // Carries the two member display labels so the named brace-list renders.
        rule: PlanRuleView::Ensemble {
            method: PlanVoteMethodView::Unanimous { n: 2 },
            members: vec![
                SmolStr::new_static("MACD trend"),
                SmolStr::new_static("RSI reversion"),
            ],
        },
        last_close: dec!(64000.00),
        as_of_label: SmolStr::new("Jun 21 14:00"),
        budget: dec!(200),
        // 200 / 64000 = 0.003125 units.
        projected_units: dec!(0.003125),
        sizing_capped: false,
        horizon_days: 7,
        horizon_through_label: SmolStr::new("Jun 28"),
    }
}

/// A SHORT-CAPABLE [`ForwardPlanView`] fixture (advisor-short-selling, ADR-0068
/// § D8 — T-U3) — the `sma_cross_ls` symmetric long/short arm. The plan renders
/// the LONG rules (the existing SMA IF/THEN path) PLUS the appended down-half:
/// sell-to-open on the bearish flip, buy-to-cover on the bullish flip, and the
/// maintenance-margin liquidation floor — with the unbounded-loss disclaimer at
/// the foot. The render guard asserts the short-rule copy + the disclaimer
/// paint; the long-only `fake_forward_plan` is the negative control.
///
/// Built directly as the view type — fixtures NEVER stand up the engine; the
/// short-capability is keyed on the `strategy` id (`is_short_capable`), so no
/// new engine field crosses the seam.
#[must_use]
pub fn fake_forward_plan_short() -> crate::forward_plan::ForwardPlanView {
    use crate::forward_plan::state::{
        ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView,
    };
    ForwardPlanView {
        // The `_ls` id is what `is_short_capable()` keys on (closed ui-side).
        strategy: SmolStr::new("sma_cross_ls"),
        symbol: SmolStr::new("BTCUSDT"),
        // FLAT — between flips; the plan shows both the long entry and the short
        // sell-to-open as standing rules.
        stance: PlanStanceView::Flat,
        latest_signal: Some(PlanSignalView::Hold),
        rule: PlanRuleView::SmaCross {
            fast_len: 12,
            slow_len: 26,
        },
        last_close: dec!(64000.00),
        as_of_label: SmolStr::new("Jun 21 14:00"),
        budget: dec!(200),
        projected_units: dec!(0.003125),
        sizing_capped: false,
        horizon_days: 7,
        horizon_through_label: SmolStr::new("Jun 28"),
    }
}

/// The ALWAYS-SHORT benchmark control [`ForwardPlanView`] (advisor-short-selling,
/// ADR-0068 § D9) — the down-side mirror of buy-and-hold. The plan renders the
/// single standing short rule ("open a short now and hold it the whole horizon …
/// loses on any up-trend by construction") + the liquidation floor + the
/// unbounded-loss disclaimer. Built directly as the view type.
#[must_use]
pub fn fake_forward_plan_always_short() -> crate::forward_plan::ForwardPlanView {
    use crate::forward_plan::state::{
        ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView,
    };
    ForwardPlanView {
        strategy: SmolStr::new("always_short"),
        symbol: SmolStr::new("BTCUSDT"),
        stance: PlanStanceView::Flat,
        latest_signal: Some(PlanSignalView::Hold),
        // The rule family is immaterial for always_short (the screen renders its
        // degenerate standing short rule keyed on the id); SmaCross is a benign
        // placeholder that never reaches the long IF/THEN path for this arm.
        rule: PlanRuleView::SmaCross {
            fast_len: 12,
            slow_len: 26,
        },
        last_close: dec!(64000.00),
        as_of_label: SmolStr::new("Jun 21 14:00"),
        budget: dec!(200),
        projected_units: dec!(0.003125),
        sizing_capped: false,
        horizon_days: 7,
        horizon_through_label: SmolStr::new("Jun 28"),
    }
}

/// A `Cockpit` routed to `Screen::ForwardPlan` with the supplied plan state
/// installed. Synthetic — no engine, no I/O; the render guard drives this.
#[must_use]
pub fn fake_cockpit_forward_plan(
    plan: PanelState<crate::forward_plan::ForwardPlanView>,
) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::ForwardPlan;
    cockpit.forward_plan_screen_state = crate::forward_plan::ForwardPlanScreenState { plan };
    cockpit
}

/// F7 EUR-FX — A `Cockpit` routed to `Screen::Leaderboard` with an explicit
/// `advisor_eur_usd_rate` and `budget_input`, so the bakeoff-input budget hint
/// renders the honest "€{eur} ≈ ${usdt} (at {rate} EUR/USD, config)" label.
///
/// Used by the `eur_fx_budget_render.rs` render guard to prove the FX
/// conversion label actually paints in the FORM band. Synthetic — no engine.
#[must_use]
pub fn fake_cockpit_leaderboard_with_fx_rate(budget_input: &str, eur_usd_rate: Decimal) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::Leaderboard;
    cockpit.advisor_eur_usd_rate = eur_usd_rate;
    cockpit.leaderboard_screen_state = crate::leaderboard::LeaderboardScreenState {
        result: PanelState::Empty,
        running: false,
        budget_input: budget_input.to_string(),
        ..Default::default()
    };
    cockpit
}

// ── advisor-param-tuning (ADR-0069) — Tune screen fixtures (T8) ───────────────
//
// A populated `SweepReportMirror` (a mix of Robust + Marginal + ≥1 FRAGILE cell,
// a shipped-baseline row, a buy-and-hold strip) for the render tests + the
// gallery. The mirror is a PURE `ui` type — built here WITHOUT the engine
// (`backtest::SweepReport` is never constructed), exactly the T8 contract: the
// fixtures are deterministic + engine-free + unit-constructible.

/// Build one swept-cell row from concrete params + a verdict + an in-sample
/// return + a Sharpe-spread, with the distribution shaped to match the verdict
/// (a FRAGILE cell has a NEGATIVE p5 Sharpe + a high P(loss) — the tail loses
/// money, the anti-overfit signal). Pure; deterministic.
fn fake_sweep_cell(
    fast: u32,
    slow: u32,
    verdict: crate::tune::SweepVerdictLabel,
    in_sample_return: Decimal,
    sharpe_p50: f64,
) -> crate::tune::SweepCellRow {
    use crate::tune::SweepVerdictLabel;
    let fragile = matches!(verdict, SweepVerdictLabel::Fragile);
    let promotable = !fragile;
    let distribution = crate::tune::SweepDistributionMirror {
        // FRAGILE: the tail loses money → negative p5 Sharpe + high P(loss).
        sharpe_p5: if fragile { -0.55 } else { 0.60 },
        sharpe_p50,
        sharpe_p95: sharpe_p50 + 0.55,
        prob_loss: if fragile { 0.52 } else { 0.14 },
        prob_sharpe_gt1: if fragile { 0.22 } else { 0.64 },
        maxdd_p95: if fragile { 0.62 } else { 0.31 },
    };
    crate::tune::SweepCellRow {
        params_label: SmolStr::new(format!("fast={fast}, slow={slow}")),
        verdict,
        promotable,
        in_sample_sharpe: sharpe_p50,
        in_sample_return,
        in_sample_maxdd: dec!(0.15),
        trade_count: 24,
        distribution,
    }
}

/// A synthetic MACD sweep cell — the `macd(fast,slow,signal)` params-label shape
/// the engine's `SweptParams::Macd::label` produces. Same FRAGILE distribution
/// shape as [`fake_sweep_cell`] (negative p5 Sharpe + high P(loss) when fragile).
fn fake_macd_cell(
    fast: u32,
    slow: u32,
    signal: u32,
    verdict: crate::tune::SweepVerdictLabel,
    in_sample_return: Decimal,
    sharpe_p50: f64,
) -> crate::tune::SweepCellRow {
    use crate::tune::SweepVerdictLabel;
    let fragile = matches!(verdict, SweepVerdictLabel::Fragile);
    let promotable = !fragile;
    let distribution = crate::tune::SweepDistributionMirror {
        sharpe_p5: if fragile { -0.50 } else { 0.58 },
        sharpe_p50,
        sharpe_p95: sharpe_p50 + 0.55,
        prob_loss: if fragile { 0.50 } else { 0.15 },
        prob_sharpe_gt1: if fragile { 0.24 } else { 0.62 },
        maxdd_p95: if fragile { 0.60 } else { 0.30 },
    };
    crate::tune::SweepCellRow {
        // Mirrors `SweptParams::Macd::label` → "macd(8,20,7)".
        params_label: SmolStr::new(format!("macd({fast},{slow},{signal})")),
        verdict,
        promotable,
        in_sample_sharpe: sharpe_p50,
        in_sample_return,
        in_sample_maxdd: dec!(0.15),
        trade_count: 31,
        distribution,
    }
}

/// A populated `SweepReportMirror` — an SMA grid with a mix of Robust / Marginal
/// / FRAGILE cells, the shipped-config baseline row, and a buy-and-hold strip.
///
/// Deliberately includes a FRAGILE cell with a GAUDY in-sample return (+9.1%)
/// but a NEGATIVE p5 Sharpe — the load-bearing anti-overfit case the render
/// guard checks (the verdict badge + the locked promotion affordance + the row
/// wash). Pure; engine-free; deterministic.
#[must_use]
pub fn fake_sweep_report_mirror() -> crate::tune::SweepReportMirror {
    use crate::tune::SweepVerdictLabel;
    let cells = vec![
        fake_sweep_cell(10, 20, SweepVerdictLabel::Robust, dec!(0.0738), 1.20),
        fake_sweep_cell(10, 30, SweepVerdictLabel::Marginal, dec!(0.0410), 0.80),
        fake_sweep_cell(15, 30, SweepVerdictLabel::Fragile, dec!(0.0910), 2.50),
        fake_sweep_cell(15, 40, SweepVerdictLabel::Robust, dec!(0.0612), 1.05),
        fake_sweep_cell(25, 60, SweepVerdictLabel::Marginal, dec!(0.0388), 0.72),
    ];
    let baseline = fake_sweep_cell(20, 50, SweepVerdictLabel::Marginal, dec!(0.0510), 0.90);
    crate::tune::SweepReportMirror {
        family_label: SmolStr::new("SMA crossover"),
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        grid_size: cells.len(),
        truncated: false,
        requested_count: cells.len(),
        cells,
        baseline,
        benchmark_kpis: crate::tune::SweepBenchmarkKpis {
            sharpe: 0.41,
            total_return_pct: dec!(0.0360),
            max_drawdown: dec!(0.0810),
        },
    }
}

/// A populated MACD `SweepReportMirror` — a MACD grid with `macd(f,s,sig)` param
/// labels, a mix of Robust / Marginal / FRAGILE cells, the shipped-config
/// baseline row (`macd(12,26,9)`), and a buy-and-hold strip.
///
/// Proves a COMPOSED family renders its result grid (the params labels are the
/// engine's `SweptParams::Macd` label shape) WITH a FRAGILE cell flagged — the
/// render guard's composed-family proof. The FRAGILE cell has a gaudy in-sample
/// return (+8.8%) but a negative p5 Sharpe (the anti-overfit case). Pure;
/// engine-free; deterministic.
#[must_use]
pub fn fake_sweep_report_mirror_macd() -> crate::tune::SweepReportMirror {
    use crate::tune::SweepVerdictLabel;
    let cells = vec![
        fake_macd_cell(8, 26, 9, SweepVerdictLabel::Robust, dec!(0.0702), 1.15),
        fake_macd_cell(12, 20, 9, SweepVerdictLabel::Marginal, dec!(0.0398), 0.78),
        fake_macd_cell(8, 20, 7, SweepVerdictLabel::Fragile, dec!(0.0880), 2.40),
        fake_macd_cell(16, 32, 11, SweepVerdictLabel::Robust, dec!(0.0590), 1.02),
        fake_macd_cell(12, 32, 7, SweepVerdictLabel::Marginal, dec!(0.0365), 0.70),
    ];
    let baseline = fake_macd_cell(12, 26, 9, SweepVerdictLabel::Marginal, dec!(0.0488), 0.88);
    crate::tune::SweepReportMirror {
        family_label: SmolStr::new("MACD"),
        coin: SmolStr::new("BTCUSDT"),
        range_label: SmolStr::new("2024 H1"),
        grid_size: cells.len(),
        truncated: false,
        requested_count: cells.len(),
        cells,
        baseline,
        benchmark_kpis: crate::tune::SweepBenchmarkKpis {
            sharpe: 0.41,
            total_return_pct: dec!(0.0360),
            max_drawdown: dec!(0.0810),
        },
    }
}

/// A TRUNCATED `SweepReportMirror` — the same populated grid but with
/// `truncated = true` + a `requested_count` above the cap, so the render guard
/// can prove the honest truncation banner paints.
#[must_use]
pub fn fake_sweep_report_mirror_truncated() -> crate::tune::SweepReportMirror {
    let mut mirror = fake_sweep_report_mirror();
    mirror.truncated = true;
    mirror.grid_size = mirror.cells.len();
    mirror.requested_count = 30;
    mirror
}

/// A `Cockpit` routed to `Screen::Tune` with the supplied result state installed.
/// Synthetic — no engine, no I/O; the render guard drives this. The form fields
/// stay at their defaults (SMA family, the shipped grid) so the form paints.
#[must_use]
pub fn fake_cockpit_tune(result: PanelState<crate::tune::SweepReportMirror>) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::Tune;
    cockpit.tune_coin = Symbol::new("BTCUSDT");
    cockpit.tune_screen_state = crate::tune::TuneScreenState {
        result,
        running: false,
        ..Default::default()
    };
    cockpit
}

/// A `Cockpit` routed to `Screen::Tune` with the supplied FAMILY selected in the
/// picker + the supplied result state. Used by the render guard to prove a
/// COMPOSED family (MACD / RSI / Bollinger) renders ITS axis form (not the SMA
/// axes). The selected family's sub-form stays at its default (shipped-centred)
/// so the axes paint with real values.
#[must_use]
pub fn fake_cockpit_tune_family(
    family: crate::tune::TuneFamily,
    result: PanelState<crate::tune::SweepReportMirror>,
) -> Cockpit {
    let mut cockpit = fake_cockpit_tune(result);
    cockpit.tune_screen_state.family = family;
    cockpit
}

/// A `Cockpit` routed to `Screen::Tune` with a sweep IN FLIGHT and a cell-level
/// `BakeoffProgress` set, so the DETERMINATE progress bar renders "Scoring {id}
/// — {done+1} of {total}" filled `done / total`. Drives the progress render
/// guard. Synthetic — no engine, no I/O, no channel (the progress is set
/// directly).
#[must_use]
pub fn fake_cockpit_tune_running_progress(done: u16, total: u16, current_id: &str) -> Cockpit {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = crate::state::Screen::Tune;
    cockpit.tune_coin = Symbol::new("BTCUSDT");
    cockpit.tune_screen_state = crate::tune::TuneScreenState {
        result: PanelState::Loading,
        running: true,
        progress: Some(backtest::progress::BakeoffProgress {
            done,
            total,
            current_id: SmolStr::new(current_id),
        }),
        ..Default::default()
    };
    cockpit
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// T1607 — two calls with the same args produce byte-equal `Vec<Bar>`.
    #[test]
    fn synthetic_candles_deterministic() {
        let symbol = Symbol::new("BTCUSDT");
        let seed = seed_for(Venue::Binance, &symbol);
        let a = synthetic_candles(seed, Venue::Binance, symbol.clone(), 60);
        let b = synthetic_candles(seed, Venue::Binance, symbol, 60);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.close.get(), y.close.get());
            assert_eq!(x.high.get(), y.high.get());
            assert_eq!(x.low.get(), y.low.get());
            assert_eq!(x.open.get(), y.open.get());
        }
    }

    /// T1607 — distinct symbols hash to distinct seeds → distinct walks.
    #[test]
    fn synthetic_candles_distinct_per_seed() {
        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");
        let seed_btc = seed_for(Venue::Binance, &btc);
        let seed_eth = seed_for(Venue::Binance, &eth);
        assert_ne!(seed_btc, seed_eth, "per-symbol seeds must differ");
        let a = synthetic_candles(seed_btc, Venue::Binance, btc, 30);
        let b = synthetic_candles(seed_eth, Venue::Binance, eth, 30);
        // Different symbols anchor at different starting prices, so the
        // first close already differs.
        assert_ne!(a[0].close.get(), b[0].close.get());
    }

    /// T1607 — `count = 4` returns ≥ 1 buy and ≥ 1 sell.
    #[test]
    fn synthetic_fills_for_has_buy_and_sell() {
        let fills = synthetic_fills_for(Venue::Binance, &Symbol::new("BTCUSDT"), 4);
        assert_eq!(fills.len(), 4);
        let buys = fills.iter().filter(|f| matches!(f.side, Side::Buy)).count();
        let sells = fills
            .iter()
            .filter(|f| matches!(f.side, Side::Sell))
            .count();
        assert!(buys >= 1, "expected at least one buy, got {buys}");
        assert!(sells >= 1, "expected at least one sell, got {sells}");
    }
}
