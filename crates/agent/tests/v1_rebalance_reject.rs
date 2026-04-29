//! T620 — v1 rebalance-reject integration test (Q6).
//!
//! Verifies that when `size_portfolio_target` returns a `PortfolioExposureBreach`
//! error, a `RebalanceRejected` strategy event is written to the audit ledger
//! via `audit::journal::rebalance_rejected`.
//!
//! This test exercises the Q6 resolution end-to-end:
//! 1. Portfolio sizer returns `PortfolioExposureBreach`.
//! 2. Caller writes `RebalanceRejected` to the strategy_events table.
//! 3. Query confirms the event appears in `strategy_history` with kind=Reject-like.
//!
//! NOTE: This test does NOT require the agent's full loop — it calls the journal
//! function directly to verify the audit contract independently of the agent runtime.
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::Arc;

use audit::{bootstrap, journal, ledger::Ledger, query};
use risk::{PortfolioSizeError, TargetLeg};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Money, Price, RiskLimits, StrategyId, Symbol, Timestamp, Usdt};

async fn open_ledger() -> Arc<Ledger> {
    let ledger = Ledger::in_memory().await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();
    Arc::new(ledger)
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s)
}

fn mark(p: f64) -> Price {
    Price::new(Decimal::try_from(p).unwrap()).unwrap()
}

fn ts() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

// ── T620-A: portfolio breach → rebalance_rejected audit entry ────────────────

#[tokio::test]
async fn t620_portfolio_breach_writes_rebalance_rejected() {
    let ledger = open_ledger().await;
    let strategy_id = StrategyId::new("test_momentum_v1");

    // Construct targets that breach the 50% portfolio cap: 3 × 20% = 60%.
    let equity = Money::<Usdt>::from_decimal(dec!(100_000));
    let targets: BTreeMap<Symbol, TargetLeg> = [
        (
            sym("BTCUSDT"),
            TargetLeg {
                symbol: sym("BTCUSDT"),
                target_weight: dec!(0.20),
                mark_price: mark(40_000.0),
            },
        ),
        (
            sym("ETHUSDT"),
            TargetLeg {
                symbol: sym("ETHUSDT"),
                target_weight: dec!(0.20),
                mark_price: mark(2_000.0),
            },
        ),
        (
            sym("BNBUSDT"),
            TargetLeg {
                symbol: sym("BNBUSDT"),
                target_weight: dec!(0.20),
                mark_price: mark(300.0),
            },
        ),
    ]
    .into_iter()
    .collect();

    let limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.50)),
    };

    let result = risk::size_portfolio_target(
        &targets,
        equity,
        &BTreeMap::new(),
        dec!(0.10),
        &limits,
        &strategy_id,
        ts(),
    );

    assert!(
        matches!(
            result,
            Err(PortfolioSizeError::PortfolioExposureBreach { .. })
        ),
        "expected PortfolioExposureBreach, got {result:?}"
    );

    // Write the rebalance_rejected event.
    let error_summary = match &result {
        Err(e) => e.to_string(),
        Ok(_) => unreachable!(),
    };

    journal::rebalance_rejected(
        &ledger,
        strategy_id.0.as_str(),
        "portfolio_exposure_breach",
        &error_summary,
        Some("1970-01-01T00:00:00Z"),
    )
    .await
    .unwrap();

    // Verify the audit trail: strategy_history must contain RebalanceRejected.
    let history = query::strategy_history(&ledger, strategy_id.clone())
        .await
        .unwrap();

    assert!(!history.is_empty(), "strategy_history must not be empty");

    let last = history.last().unwrap();
    assert_eq!(
        last.kind,
        trading_core::StrategyEventKind::RebalanceRejected,
        "last event must be RebalanceRejected, got {:?}",
        last.kind
    );
    assert!(
        last.error_code.as_deref() == Some("portfolio_exposure_breach"),
        "error_code must be portfolio_exposure_breach, got {:?}",
        last.error_code
    );
}

// ── T620-B: per-symbol breach also triggers rebalance_rejected ────────────────

#[tokio::test]
async fn t620_per_symbol_breach_writes_rebalance_rejected() {
    let ledger = open_ledger().await;
    let strategy_id = StrategyId::new("test_momentum_per_sym");

    // K=1 degenerate: 50% weight > 40% per-symbol cap.
    let equity = Money::<Usdt>::from_decimal(dec!(100_000));
    let targets: BTreeMap<Symbol, TargetLeg> = [(
        sym("BTCUSDT"),
        TargetLeg {
            symbol: sym("BTCUSDT"),
            target_weight: dec!(0.50),
            mark_price: mark(40_000.0),
        },
    )]
    .into_iter()
    .collect();

    let limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.60)),
    };

    let result = risk::size_portfolio_target(
        &targets,
        equity,
        &BTreeMap::new(),
        dec!(0.10),
        &limits,
        &strategy_id,
        ts(),
    );

    assert!(
        matches!(result, Err(PortfolioSizeError::SymbolExposureBreach { .. })),
        "expected SymbolExposureBreach, got {result:?}"
    );

    let error_summary = match &result {
        Err(e) => e.to_string(),
        Ok(_) => unreachable!(),
    };

    journal::rebalance_rejected(
        &ledger,
        strategy_id.0.as_str(),
        "per_symbol_exposure_breach",
        &error_summary,
        Some("1970-01-01T00:00:00Z"),
    )
    .await
    .unwrap();

    let history = query::strategy_history(&ledger, strategy_id.clone())
        .await
        .unwrap();
    assert!(!history.is_empty());
    let last = history.last().unwrap();
    assert_eq!(
        last.kind,
        trading_core::StrategyEventKind::RebalanceRejected
    );
    assert!(last.error_code.as_deref() == Some("per_symbol_exposure_breach"));
}

// ── T620-C: valid portfolio does NOT produce RebalanceRejected ────────────────

#[tokio::test]
async fn t620_valid_portfolio_does_not_write_rebalance_rejected() {
    let ledger = open_ledger().await;
    let strategy_id = StrategyId::new("test_momentum_valid");

    // 3 × 15% = 45% < 50% cap — valid.
    let equity = Money::<Usdt>::from_decimal(dec!(100_000));
    let targets: BTreeMap<Symbol, TargetLeg> = [
        (
            sym("BTCUSDT"),
            TargetLeg {
                symbol: sym("BTCUSDT"),
                target_weight: dec!(0.15),
                mark_price: mark(40_000.0),
            },
        ),
        (
            sym("ETHUSDT"),
            TargetLeg {
                symbol: sym("ETHUSDT"),
                target_weight: dec!(0.15),
                mark_price: mark(2_000.0),
            },
        ),
        (
            sym("BNBUSDT"),
            TargetLeg {
                symbol: sym("BNBUSDT"),
                target_weight: dec!(0.15),
                mark_price: mark(300.0),
            },
        ),
    ]
    .into_iter()
    .collect();

    let limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.50)),
    };

    let result = risk::size_portfolio_target(
        &targets,
        equity,
        &BTreeMap::new(),
        dec!(0.10),
        &limits,
        &strategy_id,
        ts(),
    );

    assert!(
        result.is_ok(),
        "expected Ok for valid portfolio: {result:?}"
    );

    // No RebalanceRejected event should be written.
    let history = query::strategy_history(&ledger, strategy_id.clone())
        .await
        .unwrap();
    assert!(
        history.is_empty(),
        "no strategy events should exist for a valid portfolio rebalance"
    );
}
