//! Unit and serde round-trip tests for `core` types (T02 acceptance).
//!
//! Acceptance criteria:
//! - every type round-trips through `serde_json`
//! - `Quantity::new(-1)` returns `Err`
//! - `Price::new(-1)` returns `Err`
//! - `Price::new(0)` returns `Err`
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    Asset, Bar, FeeTier, Fill, Liquidity, Money, Position, Price, Quantity, Side, Signal,
    SignalEvidence, SignalKind, StrategyId, Symbol, Tick, Timeframe, Timestamp, Usdt,
};

fn ts() -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap())
}

fn price(d: Decimal) -> Price {
    Price::new(d).unwrap()
}

fn qty(d: Decimal) -> Quantity {
    Quantity::new(d).unwrap()
}

// ── Quantity ─────────────────────────────────────────────────────────────────

#[test]
fn quantity_new_zero_ok() {
    assert!(Quantity::new(dec!(0)).is_ok());
}

#[test]
fn quantity_new_positive_ok() {
    assert!(Quantity::new(dec!(1.5)).is_ok());
}

#[test]
fn quantity_new_negative_err() {
    assert!(Quantity::new(dec!(-1)).is_err());
}

#[test]
fn quantity_serde_roundtrip() {
    let q = qty(dec!(0.12345));
    let json = serde_json::to_string(&q).unwrap();
    let q2: Quantity = serde_json::from_str(&json).unwrap();
    assert_eq!(q, q2);
}

// ── Price ─────────────────────────────────────────────────────────────────────

#[test]
fn price_new_positive_ok() {
    assert!(Price::new(dec!(40000.00)).is_ok());
}

#[test]
fn price_new_zero_err() {
    assert!(Price::new(dec!(0)).is_err());
}

#[test]
fn price_new_negative_err() {
    assert!(Price::new(dec!(-1)).is_err());
}

#[test]
fn price_serde_roundtrip() {
    let p = price(dec!(39999.99));
    let json = serde_json::to_string(&p).unwrap();
    let p2: Price = serde_json::from_str(&json).unwrap();
    assert_eq!(p, p2);
}

// ── Money ─────────────────────────────────────────────────────────────────────

#[test]
fn money_serde_roundtrip() {
    let m: Money<Usdt> = Money::from_decimal(dec!(12345.67));
    let json = serde_json::to_string(&m).unwrap();
    let m2: Money<Usdt> = serde_json::from_str(&json).unwrap();
    assert_eq!(m, m2);
}

#[test]
fn money_add_same_currency() {
    let a: Money<Usdt> = Money::from_decimal(dec!(100.0));
    let b: Money<Usdt> = Money::from_decimal(dec!(50.0));
    let c = a + b;
    assert_eq!(c.amount(), dec!(150.0));
}

// ── Symbol ────────────────────────────────────────────────────────────────────

#[test]
fn symbol_serde_roundtrip() {
    let s = Symbol::new("BTCUSDT");
    let json = serde_json::to_string(&s).unwrap();
    let s2: Symbol = serde_json::from_str(&json).unwrap();
    assert_eq!(s, s2);
}

// ── StrategyId ────────────────────────────────────────────────────────────────

#[test]
fn strategy_id_serde_roundtrip() {
    let id = StrategyId::new("sma_crossover");
    let json = serde_json::to_string(&id).unwrap();
    let id2: StrategyId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, id2);
}

// ── Asset ─────────────────────────────────────────────────────────────────────

#[test]
fn asset_serde_roundtrip() {
    let a = Asset::Btc;
    let json = serde_json::to_string(&a).unwrap();
    let a2: Asset = serde_json::from_str(&json).unwrap();
    assert_eq!(a, a2);
}

// ── Timestamp ─────────────────────────────────────────────────────────────────

#[test]
fn timestamp_serde_roundtrip() {
    let t = ts();
    let json = serde_json::to_string(&t).unwrap();
    let t2: Timestamp = serde_json::from_str(&json).unwrap();
    assert_eq!(t, t2);
}

#[test]
fn timestamp_unix_millis() {
    let t = ts();
    assert_eq!(t.unix_millis(), 1_700_000_000_000_i64);
}

// ── Bar ───────────────────────────────────────────────────────────────────────

#[test]
fn bar_serde_roundtrip() {
    let bar = Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneMinute,
        open_ts: ts(),
        close_ts: ts(),
        open: price(dec!(40000.00)),
        high: price(dec!(40100.00)),
        low: price(dec!(39900.00)),
        close: price(dec!(40050.00)),
        volume: qty(dec!(12.5)),
        trade_count: 100,
        local_recv_ts: ts(),
    };
    let json = serde_json::to_string(&bar).unwrap();
    let bar2: Bar = serde_json::from_str(&json).unwrap();
    assert_eq!(bar.symbol, bar2.symbol);
    assert_eq!(bar.close, bar2.close);
}

// ── Tick ──────────────────────────────────────────────────────────────────────

#[test]
fn tick_serde_roundtrip() {
    let tick = Tick {
        symbol: Symbol::new("BTCUSDT"),
        venue_ts: ts(),
        local_recv_ts: ts(),
        price: price(dec!(40050.00)),
        qty: qty(dec!(0.01)),
        side: Side::Buy,
        trade_id: 42,
    };
    let json = serde_json::to_string(&tick).unwrap();
    let tick2: Tick = serde_json::from_str(&json).unwrap();
    assert_eq!(tick.trade_id, tick2.trade_id);
    assert_eq!(tick.price, tick2.price);
}

// ── Signal ────────────────────────────────────────────────────────────────────

#[test]
fn signal_serde_roundtrip() {
    let sig = Signal {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol: Symbol::new("BTCUSDT"),
        ts: ts(),
        kind: SignalKind::Buy,
        evidence: SignalEvidence::sma(dec!(200.0), dec!(195.0)),
        pair_data: None,
    };
    let json = serde_json::to_string(&sig).unwrap();
    let sig2: Signal = serde_json::from_str(&json).unwrap();
    assert_eq!(sig.kind, sig2.kind);
}

// ── Position ──────────────────────────────────────────────────────────────────

#[test]
fn position_serde_roundtrip() {
    let pos = Position::empty(Symbol::new("BTCUSDT"));
    let json = serde_json::to_string(&pos).unwrap();
    let pos2: Position = serde_json::from_str(&json).unwrap();
    assert_eq!(pos.symbol, pos2.symbol);
    assert!(pos2.is_flat());
}

// ── Fill ──────────────────────────────────────────────────────────────────────

#[test]
fn fill_serde_roundtrip() {
    use trading_core::{FillId, OrderId};
    let fill = Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: qty(dec!(0.25)),
        price: price(dec!(40008.00)),
        fee: Money::from_decimal(dec!(1.60032)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts(),
        local_ts: ts(),
        liquidity: Liquidity::Taker,
    };
    let json = serde_json::to_string(&fill).unwrap();
    let fill2: Fill = serde_json::from_str(&json).unwrap();
    assert_eq!(fill.id, fill2.id);
    assert_eq!(fill.price, fill2.price);
}
