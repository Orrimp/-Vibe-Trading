//! Companion CSV artifact writers (R3.3 / Q5).
//!
//! All amounts are plain `Decimal` strings (TEXT-form, no scientific
//! notation, no locale separators) — same encoding as the audit
//! ledger's TEXT columns.  Timestamps are RFC3339 microsecond
//! precision (matching `journal.rs::strategy_event` format).
//!
//! Each writer takes a fully-resolved input slice and a target path,
//! and produces a single CSV file with `csv::Writer` set to
//! `QuoteStyle::Necessary`.  No rendering decisions live here —
//! callers compute the rows once and hand them in.

use std::path::Path;

use audit::query::StrategyPnl;
use rust_decimal::Decimal;
use time::format_description::well_known::Rfc3339;
use trading_core::{FillView, FundingObs, JournalEntryView, StrategyEventView, Symbol, Timestamp};

/// Errors returned by the CSV writers.
#[derive(Debug, thiserror::Error)]
pub enum CsvError {
    /// File IO failure.
    #[error("io: {0}")]
    Io(String),
    /// CSV writer error (record-write / flush).
    #[error("csv: {0}")]
    Csv(String),
    /// Format conversion failure (timestamp / decimal).
    #[error("format: {0}")]
    Format(String),
}

fn fmt_ts(ts: Timestamp) -> Result<String, CsvError> {
    ts.inner()
        .format(&Rfc3339)
        .map_err(|e| CsvError::Format(e.to_string()))
}

fn writer(path: &Path) -> Result<csv::Writer<std::fs::File>, CsvError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CsvError::Io(e.to_string()))?;
    }
    let f = std::fs::File::create(path).map_err(|e| CsvError::Io(e.to_string()))?;
    Ok(csv::WriterBuilder::new()
        .quote_style(csv::QuoteStyle::Necessary)
        .from_writer(f))
}

/// One sample row of the equity CSV (matches R3.3 column schema).
#[derive(Debug, Clone)]
pub struct EquitySample {
    /// Sample timestamp.
    pub ts: Timestamp,
    /// `cash + Σ positions` at this sample (USDT).
    pub equity_total: Decimal,
    /// Realized P&L since inception (USDT).
    pub realized_pnl: Decimal,
    /// Unrealized P&L (mark-to-market) at this sample (USDT).
    pub unrealized_pnl: Decimal,
    /// `assets:cash:USDT` balance at this sample.
    pub cash_balance: Decimal,
}

/// Write the per-window equity curve CSV at `path`.
///
/// Columns:
/// `ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`.
///
/// # Errors
///
/// Returns [`CsvError::Io`] on IO failure, [`CsvError::Csv`] on write
/// failure, or [`CsvError::Format`] on timestamp formatting failure.
pub fn write_equity_csv(path: &Path, samples: &[EquitySample]) -> Result<(), CsvError> {
    let mut wtr = writer(path)?;
    wtr.write_record([
        "ts",
        "equity_total_usdt",
        "realized_pnl_usdt",
        "unrealized_pnl_usdt",
        "cash_balance_usdt",
    ])
    .map_err(|e| CsvError::Csv(e.to_string()))?;
    for s in samples {
        let ts = fmt_ts(s.ts)?;
        wtr.write_record([
            ts.as_str(),
            &s.equity_total.to_string(),
            &s.realized_pnl.to_string(),
            &s.unrealized_pnl.to_string(),
            &s.cash_balance.to_string(),
        ])
        .map_err(|e| CsvError::Csv(e.to_string()))?;
    }
    wtr.flush().map_err(|e| CsvError::Csv(e.to_string()))?;
    Ok(())
}

/// Phase 4 (T1810) — read the per-window equity curve CSV at `path`.
/// Inverse of [`write_equity_csv`]; consumes the same column schema
/// (`ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,
/// cash_balance_usdt`). Used by the viewer bin to populate the
/// `EquitySeries` that feeds the equity curve + drawdown band.
///
/// # Errors
///
/// Returns [`CsvError::Io`] on IO failure, [`CsvError::Csv`] on
/// parse failure, or [`CsvError::Format`] on timestamp parse failure.
pub fn read_equity_csv(path: &Path) -> Result<Vec<EquitySample>, CsvError> {
    use std::str::FromStr;
    use time::OffsetDateTime;

    let f = std::fs::File::open(path).map_err(|e| CsvError::Io(e.to_string()))?;
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(f);
    let mut out = Vec::new();
    for record in rdr.records() {
        let rec = record.map_err(|e| CsvError::Csv(e.to_string()))?;
        if rec.len() < 5 {
            return Err(CsvError::Csv(format!(
                "expected 5 columns, got {}",
                rec.len()
            )));
        }
        let ts = OffsetDateTime::parse(&rec[0], &Rfc3339)
            .map(Timestamp::new)
            .map_err(|e| CsvError::Format(e.to_string()))?;
        let equity_total = Decimal::from_str(rec[1].trim())
            .map_err(|e| CsvError::Csv(format!("equity_total parse: {e}")))?;
        let realized_pnl = Decimal::from_str(rec[2].trim())
            .map_err(|e| CsvError::Csv(format!("realized_pnl parse: {e}")))?;
        let unrealized_pnl = Decimal::from_str(rec[3].trim())
            .map_err(|e| CsvError::Csv(format!("unrealized_pnl parse: {e}")))?;
        let cash_balance = Decimal::from_str(rec[4].trim())
            .map_err(|e| CsvError::Csv(format!("cash_balance parse: {e}")))?;
        out.push(EquitySample {
            ts,
            equity_total,
            realized_pnl,
            unrealized_pnl,
            cash_balance,
        });
    }
    Ok(out)
}

/// Write the fills CSV at `path`.
///
/// Columns: `ts,symbol,side,qty,price,fee_usdt,fee_tier,strategy_id`.
///
/// `fills_with_strategy` carries the strategy id alongside each fill.
/// Use `None` for pre-migration fills (writes the empty string).
///
/// # Errors
///
/// Returns [`CsvError`] on IO / CSV / format failure.
pub fn write_fills_csv(
    path: &Path,
    fills_with_strategy: &[(FillView, Option<String>)],
) -> Result<(), CsvError> {
    let mut wtr = writer(path)?;
    wtr.write_record([
        "ts",
        "symbol",
        "side",
        "qty",
        "price",
        "fee_usdt",
        "fee_tier",
        "strategy_id",
    ])
    .map_err(|e| CsvError::Csv(e.to_string()))?;
    for (f, sid) in fills_with_strategy {
        let ts = fmt_ts(f.venue_ts)?;
        let fee_tier = match f.fee_tier {
            trading_core::FeeTier::Maker => "maker",
            trading_core::FeeTier::Taker => "taker",
        };
        let side = match f.side {
            trading_core::Side::Buy => "buy",
            trading_core::Side::Sell => "sell",
        };
        wtr.write_record([
            ts.as_str(),
            f.symbol.0.as_str(),
            side,
            &f.qty.get().to_string(),
            &f.price.get().to_string(),
            &f.fee.amount().to_string(),
            fee_tier,
            sid.as_deref().unwrap_or(""),
        ])
        .map_err(|e| CsvError::Csv(e.to_string()))?;
    }
    wtr.flush().map_err(|e| CsvError::Csv(e.to_string()))?;
    Ok(())
}

/// Write the per-strategy P&L CSV at `path`.
///
/// Columns:
/// `strategy_id,realized_usdt,closed_trade_count,winning_trade_count,win_rate,avg_trade_realized_usdt`.
///
/// `win_rate` is rendered as a percentage with two decimal places
/// (e.g. `75.00`).  When `closed_trade_count == 0` the win-rate cell
/// is the empty string.
///
/// # Errors
///
/// Returns [`CsvError`] on IO / CSV failure.
pub fn write_pnl_by_strategy_csv(path: &Path, rows: &[StrategyPnl]) -> Result<(), CsvError> {
    let mut wtr = writer(path)?;
    wtr.write_record([
        "strategy_id",
        "realized_usdt",
        "closed_trade_count",
        "winning_trade_count",
        "win_rate",
        "avg_trade_realized_usdt",
    ])
    .map_err(|e| CsvError::Csv(e.to_string()))?;
    for row in rows {
        let win_rate = if row.closed_trade_count == 0 {
            String::new()
        } else {
            let denom = Decimal::from(row.closed_trade_count);
            let num = Decimal::from(row.winning_trade_count);
            ((num / denom) * Decimal::from(100u32))
                .round_dp(2)
                .to_string()
        };
        wtr.write_record([
            row.strategy_id.0.as_str(),
            &row.realized.amount().to_string(),
            &row.closed_trade_count.to_string(),
            &row.winning_trade_count.to_string(),
            &win_rate,
            &row.avg_trade_realized.amount().to_string(),
        ])
        .map_err(|e| CsvError::Csv(e.to_string()))?;
    }
    wtr.flush().map_err(|e| CsvError::Csv(e.to_string()))?;
    Ok(())
}

/// Write the per-symbol P&L CSV at `path`.
///
/// Columns: `symbol,realized_usdt`.
///
/// # Errors
///
/// Returns [`CsvError`] on IO / CSV failure.
pub fn write_pnl_by_symbol_csv(
    path: &Path,
    rows: &[(Symbol, trading_core::Money<trading_core::Usdt>)],
) -> Result<(), CsvError> {
    let mut wtr = writer(path)?;
    wtr.write_record(["symbol", "realized_usdt"])
        .map_err(|e| CsvError::Csv(e.to_string()))?;
    for (sym, pnl) in rows {
        wtr.write_record([sym.0.as_str(), &pnl.amount().to_string()])
            .map_err(|e| CsvError::Csv(e.to_string()))?;
    }
    wtr.flush().map_err(|e| CsvError::Csv(e.to_string()))?;
    Ok(())
}

/// Write the journal-entries CSV at `path`.
///
/// Columns: `ts,account,debit_usdt,credit_usdt,memo,transaction_id`.
///
/// `JournalEntryView` carries an `amount` (positive when credit
/// dominates).  We split that signed amount back into debit/credit so
/// the CSV column shape matches the audit's storage contract.
///
/// # Errors
///
/// Returns [`CsvError`] on IO / CSV / format failure.
pub fn write_journal_csv(
    path: &Path,
    entries: &[(JournalEntryView, String)],
) -> Result<(), CsvError> {
    let mut wtr = writer(path)?;
    wtr.write_record([
        "ts",
        "account",
        "debit_usdt",
        "credit_usdt",
        "memo",
        "transaction_id",
    ])
    .map_err(|e| CsvError::Csv(e.to_string()))?;
    for (entry, txn_id) in entries {
        let ts = fmt_ts(entry.ts)?;
        let (debit, credit) = if entry.amount >= Decimal::ZERO {
            (Decimal::ZERO, entry.amount)
        } else {
            (entry.amount.abs(), Decimal::ZERO)
        };
        wtr.write_record([
            ts.as_str(),
            entry.account.0.as_str(),
            &debit.to_string(),
            &credit.to_string(),
            entry.memo.as_str(),
            txn_id.as_str(),
        ])
        .map_err(|e| CsvError::Csv(e.to_string()))?;
    }
    wtr.flush().map_err(|e| CsvError::Csv(e.to_string()))?;
    Ok(())
}

/// Write the strategy-events CSV at `path`.
///
/// Columns:
/// `ts,kind,strategy_id,old_hash,new_hash,source_path,operator,error_code,error_summary`.
///
/// # Errors
///
/// Returns [`CsvError`] on IO / CSV / format failure.
pub fn write_strategy_events_csv(
    path: &Path,
    events: &[StrategyEventView],
) -> Result<(), CsvError> {
    let mut wtr = writer(path)?;
    wtr.write_record([
        "ts",
        "kind",
        "strategy_id",
        "old_hash",
        "new_hash",
        "source_path",
        "operator",
        "error_code",
        "error_summary",
    ])
    .map_err(|e| CsvError::Csv(e.to_string()))?;
    for ev in events {
        let ts = fmt_ts(ev.ts)?;
        wtr.write_record([
            ts.as_str(),
            &ev.kind.to_string(),
            ev.strategy_id.as_ref().map_or("", |s| s.0.as_str()),
            ev.old_hash.as_ref().map_or("", smol_str::SmolStr::as_str),
            ev.new_hash.as_ref().map_or("", smol_str::SmolStr::as_str),
            ev.source_path
                .as_ref()
                .map_or("", smol_str::SmolStr::as_str),
            ev.operator.as_str(),
            ev.error_code.as_ref().map_or("", smol_str::SmolStr::as_str),
            ev.error_summary
                .as_ref()
                .map_or("", smol_str::SmolStr::as_str),
        ])
        .map_err(|e| CsvError::Csv(e.to_string()))?;
    }
    wtr.flush().map_err(|e| CsvError::Csv(e.to_string()))?;
    Ok(())
}

/// Write the funding-observations CSV at `path`.
///
/// Columns: `symbol,funding_ts,funding_rate,next_funding_ts,poll_ts`.
///
/// # Errors
///
/// Returns [`CsvError`] on IO / CSV / format failure.
pub fn write_funding_obs_csv(path: &Path, rows: &[FundingObs]) -> Result<(), CsvError> {
    let mut wtr = writer(path)?;
    wtr.write_record([
        "symbol",
        "funding_ts",
        "funding_rate",
        "next_funding_ts",
        "poll_ts",
    ])
    .map_err(|e| CsvError::Csv(e.to_string()))?;
    for r in rows {
        let funding_ts = fmt_ts(r.funding_ts)?;
        let next_ts = fmt_ts(r.next_funding_ts)?;
        let poll_ts = fmt_ts(r.poll_ts)?;
        wtr.write_record([
            r.symbol.0.as_str(),
            funding_ts.as_str(),
            &r.funding_rate.to_string(),
            next_ts.as_str(),
            poll_ts.as_str(),
        ])
        .map_err(|e| CsvError::Csv(e.to_string()))?;
    }
    wtr.flush().map_err(|e| CsvError::Csv(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use tempfile::TempDir;
    use trading_core::{Money, Usdt};

    #[test]
    fn t813_equity_csv_roundtrips_via_string() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("equity.csv");
        let samples = vec![EquitySample {
            ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
            equity_total: dec!(1000.00),
            realized_pnl: dec!(50.00),
            unrealized_pnl: dec!(10.00),
            cash_balance: dec!(940.00),
        }];
        write_equity_csv(&path, &samples).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains(
            "ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt"
        ));
        assert!(body.contains("1000.00,50.00,10.00,940.00"));
    }

    #[test]
    fn t813_pnl_by_strategy_csv_columns() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pnl_by_strategy.csv");
        let rows = vec![StrategyPnl {
            strategy_id: trading_core::StrategyId::new("alpha"),
            realized: Money::<Usdt>::from_decimal(dec!(100.00)),
            closed_trade_count: 4,
            winning_trade_count: 3,
            avg_trade_realized: Money::<Usdt>::from_decimal(dec!(25.00)),
        }];
        write_pnl_by_strategy_csv(&path, &rows).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains(
            "strategy_id,realized_usdt,closed_trade_count,winning_trade_count,win_rate,avg_trade_realized_usdt"
        ));
        assert!(body.contains("alpha,100.00,4,3,75.00,25.00"));
    }

    #[test]
    fn t813_pnl_by_symbol_csv_columns() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pnl_by_symbol.csv");
        let rows = vec![(
            Symbol::new("BTCUSDT"),
            Money::<Usdt>::from_decimal(dec!(150.00)),
        )];
        write_pnl_by_symbol_csv(&path, &rows).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("symbol,realized_usdt"));
        assert!(body.contains("BTCUSDT,150.00"));
    }

    #[test]
    fn t813_strategy_events_csv_columns() {
        use smol_str::SmolStr;
        use trading_core::{StrategyEventKind, StrategyId};
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("strategy_events.csv");
        let events = vec![StrategyEventView {
            id: SmolStr::new("evt-1"),
            ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
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
        assert!(body.contains(
            "ts,kind,strategy_id,old_hash,new_hash,source_path,operator,error_code,error_summary"
        ));
        assert!(body.contains("Load,alpha,,abcdef,config/alpha.toml,system,,"));
    }
}
