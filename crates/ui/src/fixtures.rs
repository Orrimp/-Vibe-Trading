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
    Bar, FeeTier, FillView, Money, PnlSnapshot, PositionView, Price, Quantity, Side,
    StrategyEventKind, StrategyEventView, StrategyId, Symbol, Tick, Timeframe, Timestamp,
};

use crate::state::{AgentMode, Cockpit, PanelState, StrategyRow, StrategyStatus};

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
    }
}

/// A tick with an explicit skew (venue - local in ms) so the latency
/// badge can be exercised deterministically.
#[must_use]
pub fn fake_tick_with_skew_ms(skew_ms: i64) -> Tick {
    let venue = fixed_ts(0);
    let local = {
        let dt = OffsetDateTime::from_unix_timestamp_nanos(
            i128::from(FIXED_EPOCH_SECS) * 1_000_000_000 + i128::from(skew_ms) * 1_000_000,
        )
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        Timestamp::new(dt)
    };
    Tick {
        symbol: Symbol::new("BTCUSDT"),
        venue_ts: venue,
        local_recv_ts: local,
        price: Price::new(dec!(40050.00)).unwrap_or_else(|_| unreachable!()),
        qty: Quantity::new(dec!(0.05)).unwrap_or_else(|_| unreachable!()),
        side: Side::Buy,
        trade_id: 1,
    }
}

/// A single deterministic fill view. `n` controls minor price drift so a
/// sequence of fills renders with distinct rows.
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
    }
}

/// A P&L snapshot with a losing day (exercises `color::NEG`).
#[must_use]
pub fn fake_pnl_negative() -> PnlSnapshot {
    PnlSnapshot {
        cash: Money::from_decimal(dec!(99_500.00)),
        unrealized: Money::from_decimal(dec!(-75.00)),
        realized: Money::from_decimal(dec!(-325.00)),
        total_equity: Money::from_decimal(dec!(99_100.00)),
        daily_return: Money::from_decimal(dec!(-900.00)),
        as_of: fixed_ts(0),
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

/// A cockpit booted fully ready — for manual smoke tests with
/// `cargo run --bin cockpit --features fixtures`.
#[must_use]
pub fn fake_cockpit_ready() -> Cockpit {
    let mut c = Cockpit::ready(fake_fill_feed(12), fake_positions(), fake_pnl_positive());
    c.mode = AgentMode::Paper;
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
