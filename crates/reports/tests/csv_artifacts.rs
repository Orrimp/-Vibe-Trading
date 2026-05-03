#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — companion CSV artifact integration test.
//!
//! Verifies the column schemas in the Design's "CSV artifact column
//! schemas" subsection: header strings + value formatting.

use audit::query::StrategyPnl;
use reports::csv_artifacts::{
    write_equity_csv, write_fills_csv, write_pnl_by_strategy_csv, write_pnl_by_symbol_csv,
    write_strategy_events_csv, EquitySample,
};
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tempfile::TempDir;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, FillView, Money, Price, Quantity, Side, StrategyEventKind, StrategyEventView,
    StrategyId, Symbol, Timestamp, Usdt,
};

#[test]
fn t813_csv_equity_header_and_row() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("equity.csv");
    let samples = vec![EquitySample {
        ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
        equity_total: dec!(1000.00),
        realized_pnl: dec!(50.00),
        unrealized_pnl: dec!(10.00),
        cash_balance: dec!(940.00),
    }];
    write_equity_csv(&path, &samples).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.starts_with(
        "ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt"
    ));
    assert!(body.contains("1970-01-01T00:00:00Z,1000.00,50.00,10.00,940.00"));
}

#[test]
fn t813_csv_fills_header_and_row() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fills.csv");
    let f = FillView {
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        price: Price::new(dec!(70000)).unwrap(),
        qty: Quantity::new(dec!(0.01)).unwrap(),
        fee: Money::<Usdt>::from_decimal(dec!(0.70)),
        fee_tier: FeeTier::Taker,
        venue_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
        transaction_id: smol_str::SmolStr::default(),
    };
    let rows = vec![(f, Some("alpha".to_string()))];
    write_fills_csv(&path, &rows).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.starts_with("ts,symbol,side,qty,price,fee_usdt,fee_tier,strategy_id"));
    assert!(body.contains("BTCUSDT,buy,0.01,70000,0.70,taker,alpha"));
}

#[test]
fn t813_csv_pnl_by_strategy_header_and_row() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("pnl_by_strategy.csv");
    let rows = vec![StrategyPnl {
        strategy_id: StrategyId::new("alpha"),
        realized: Money::<Usdt>::from_decimal(dec!(100.00)),
        closed_trade_count: 4,
        winning_trade_count: 3,
        avg_trade_realized: Money::<Usdt>::from_decimal(dec!(25.00)),
    }];
    write_pnl_by_strategy_csv(&path, &rows).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.starts_with(
        "strategy_id,realized_usdt,closed_trade_count,winning_trade_count,win_rate,avg_trade_realized_usdt"
    ));
    assert!(body.contains("alpha,100.00,4,3,75.00,25.00"));
}

#[test]
fn t813_csv_pnl_by_symbol_header_and_row() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("pnl_by_symbol.csv");
    let rows = vec![(
        Symbol::new("BTCUSDT"),
        Money::<Usdt>::from_decimal(dec!(150.00)),
    )];
    write_pnl_by_symbol_csv(&path, &rows).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.starts_with("symbol,realized_usdt"));
    assert!(body.contains("BTCUSDT,150.00"));
}

#[test]
fn t813_csv_strategy_events_header_and_row() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("strategy_events.csv");
    let events = vec![StrategyEventView {
        id: SmolStr::new("evt-1"),
        ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
        kind: StrategyEventKind::Load,
        strategy_id: Some(StrategyId::new("alpha")),
        old_hash: None,
        new_hash: Some(SmolStr::new("abcdef")),
        source_path: Some(SmolStr::new("config/alpha.toml")),
        operator: SmolStr::new("system"),
        error_code: None,
        error_summary: None,
    }];
    write_strategy_events_csv(&path, &events).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.starts_with(
        "ts,kind,strategy_id,old_hash,new_hash,source_path,operator,error_code,error_summary"
    ));
    assert!(body.contains("Load,alpha,,abcdef,config/alpha.toml,system,,"));
}
