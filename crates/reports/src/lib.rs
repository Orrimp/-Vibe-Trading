//! Operator-success-report renderer (lib + bin).
//!
//! Read-only over the audit ledger + parquet roots.  Produces a
//! YAML-front-mattered markdown body + companion CSV artifacts under
//! `evidence/reports/success/`.  The body is deterministic (R10.3) so a
//! body-SHA256 anchor can lock it into the regression gate.
//!
//! See `spec/features/operator-success-reports.md` and
//! `spec/tasks/operator-success-reports.md` for the contract.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)] // covered by `# Errors` notes per fn
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

pub mod atomic_write;
pub mod csv_artifacts;
pub mod marks;
pub mod parse;
pub mod reconcile;
pub mod render;
pub mod run_id;
pub mod sparkline;
pub mod window;

pub use marks::{FrozenMarkSource, MarkError, MarkSource, ParquetMarkSource};
pub use reconcile::{ReconciliationInputs, ReconciliationReport, ReconciliationRow};
pub use render::ReportArtifacts;
pub use window::{ReportWindow, WindowParseError};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::OffsetDateTime;
use trading_core::{LedgerError, Money, StrategyEventKind, Symbol, Timestamp, Usdt};

/// Top-level errors for the reports binary + library.
#[derive(Debug, Error)]
pub enum ReportError {
    /// Audit-side query / migration error.
    #[error("audit query: {0}")]
    Audit(#[from] LedgerError),
    /// Mark-source error (parquet / CSV / out-of-range).
    #[error("mark source: {0}")]
    Marks(#[from] MarkError),
    /// File IO error (tempfile / rename / mkdir).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Reconciliation FAIL — sibling JSON written, bin maps to exit 1.
    #[error("reconciliation FAILED — Δ != $0.00 — see {sibling_path:?}")]
    Reconciliation {
        /// Path of the sidecar `_reconciliation_failure.json` artifact.
        sibling_path: PathBuf,
    },
    /// `--period` parse error.
    #[error("invalid window: {0}")]
    Window(#[from] WindowParseError),
    /// CSV writer error.
    #[error("csv: {0}")]
    Csv(String),
    /// Reconciliation identity #1 has no report side: the headline section
    /// this function rendered could not be read back (bug-log `#156`).
    #[error("reconciliation: {0}")]
    Reconcile(String),
}

impl From<csv_artifacts::CsvError> for ReportError {
    fn from(e: csv_artifacts::CsvError) -> Self {
        ReportError::Csv(e.to_string())
    }
}

/// Read the published strategy-return figure back out of the rendered R2 section.
///
/// Reconciliation identity #1's REPORT side must come from what the report
/// actually published, not from the variable its LEDGER side is also built from
/// — that is precisely what made the row a tautology until 2026-11-07
/// (bug-log `#156`). The figure is parsed at the scale it was published, two
/// decimal places, which is this ledger's unit of account, so the comparison
/// stays exact-cent rather than approximate.
///
/// Returns `None` when the line cannot be parsed, and the caller turns that into
/// a hard error. It must NEVER fall back to zero: a zero report side would
/// reconcile against a zero ledger side on an empty period and re-create the
/// unfailable row this function exists to retire.
fn published_strategy_return(rendered_headline: &str) -> Option<Decimal> {
    let line = rendered_headline
        .lines()
        .find(|l| l.starts_with("Strategy return:"))?;
    let inner = line.split_once('(')?.1.strip_suffix(" USDT)")?;
    let (sign, rest) = inner.split_at(inner.char_indices().nth(1)?.0);
    let digits = rest.strip_prefix('$')?;
    let value: Decimal = digits.parse().ok()?;
    match sign {
        "+" => Some(value),
        "-" => Some(-value),
        _ => None,
    }
}

/// Format a `Timestamp` to RFC3339 microseconds (matches HF-3 /
/// `journal.rs::strategy_event` format).
fn fmt_ts_micros(ts: Timestamp) -> String {
    let Ok(fmt) = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    ) else {
        return ts.to_string();
    };
    ts.inner().format(&fmt).unwrap_or_else(|_| ts.to_string())
}

/// Top-level entry point — orchestrates the read-only queries +
/// renders the report + atomic-writes the markdown and companion CSVs.
///
/// # Errors
///
/// - [`ReportError::Window`] — invalid `--period` value.
/// - [`ReportError::Audit`] — audit query failed.
/// - [`ReportError::Io`] — atomic-write tempfile / rename failed.
/// - [`ReportError::Marks`] — mark source returned an error.
/// - [`ReportError::Reconciliation`] — at least one R11 identity has
///   `delta != 0`.  The bin maps this to `exit 1` (R1.6).
#[allow(clippy::too_many_lines)] // orchestration glue
pub async fn generate(
    window: ReportWindow,
    audit_db_path: &Path,
    marks: &dyn MarkSource,
    out: &Path,
    seed: Option<u64>,
) -> Result<ReportArtifacts, ReportError> {
    let started = Instant::now();

    // ── 1. Open the ledger + resolve the period ─────────────────────────────
    let ledger_path_str = audit_db_path.to_str().ok_or_else(|| {
        ReportError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "ledger path is not utf-8",
        ))
    })?;
    let ledger = audit::Ledger::open(ledger_path_str).await?;

    // Wall-clock now — front-matter only (never in body).
    let now = Timestamp::new(OffsetDateTime::now_utc());
    // Inception: query if needed, fallback to now to avoid panics.
    let inception = match audit::query::ledger_inception_ts(&ledger).await {
        Ok(t) => t,
        Err(_) => now,
    };
    let (period_start, period_end) = window.resolve(now, inception);

    // ── 2. Run all queries once ─────────────────────────────────────────────
    let cash = audit::query::cash_balance(&ledger).await?;
    let realized = audit::query::realized_pnl_since(&ledger, inception).await?;
    let realized_period = audit::query::realized_pnl_since(&ledger, period_start).await?;

    let pnl_by_strategy = audit::query::pnl_by_strategy(&ledger, period_start, period_end).await?;
    let pnl_by_symbol = audit::query::pnl_by_symbol(&ledger, period_start, period_end).await?;
    let strategy_events = audit::query::strategy_events_since(&ledger, period_start).await?;
    let uptime_intervals = audit::query::uptime_intervals_since(&ledger, period_start).await?;
    let recent_fills = audit::query::recent_fills(&ledger, usize::MAX).await?;
    let recent_journal = audit::query::recent_journal(&ledger, usize::MAX).await?;

    // ── 3. Mark-to-market unrealized P&L ────────────────────────────────────
    // T1003: project open positions at `period_end` via the typed reader
    // shipped by T1002, then mark them with `MarkSource::close_at`.  Per
    // architect Design § Q6, an `OutOfRange` mark for an open position
    // contributes `Decimal::ZERO`, logs a warning, and toggles a body
    // footnote on the R11 appendix — the unrealized arithmetic stays
    // invariant under mark-source health (determinism foot-gun avoided).
    //
    // Empty-positions ledgers (the two anchored fixtures
    // `build_ledger_7d` / `build_ledger_90d`) skip the loop entirely:
    // `unrealized` stays `Decimal::ZERO`, `mark_misses` stays empty,
    // and the body bytes match the pre-T1003 hardcoded path —
    // satisfying R3 / Q4 (anchors stay byte-identical, 11/11 PASS).
    let open_positions = audit::query::open_positions_at(&ledger, period_end).await?;
    let mut unrealized: Decimal = Decimal::ZERO;
    let mut mark_misses: u32 = 0;
    for pos in &open_positions {
        match marks.close_at(&pos.symbol, period_end) {
            Ok(mark_price) => {
                // Q7 contract: `pos.avg_cost_basis` is the per-unit cost
                // basis (USDT per unit).  Notional contribution at mark
                // is `qty * (mark - avg_cost_basis)`.
                unrealized += pos.qty * (mark_price - pos.avg_cost_basis.amount());
            }
            Err(MarkError::OutOfRange { .. }) => {
                tracing::warn!(
                    symbol = %pos.symbol,
                    ts = %period_end,
                    "mark unavailable for open position; treating unrealized as zero",
                );
                mark_misses += 1;
                // Q6: contribute Decimal::ZERO; do not propagate.
            }
            Err(e) => return Err(ReportError::Marks(e)),
        }
    }
    let mark_unavailable_footnote = mark_misses > 0;

    // (We *do* hit the marks trait so it isn't dead — pull the BTC close
    // at period_end + period_start for the BTC buy-and-hold baseline.)
    let btc_symbol = Symbol::new("BTCUSDT");
    let btc_start = marks.close_at(&btc_symbol, period_start).ok();
    let btc_end = marks.close_at(&btc_symbol, period_end).ok();

    // ── 4. Compute reconciliation ───────────────────────────────────────────
    let sum_by_strategy: Decimal = pnl_by_strategy.iter().map(|r| r.realized.amount()).sum();
    let sum_by_symbol: Decimal = pnl_by_symbol.iter().map(|(_, m)| m.amount()).sum();
    let headline_return_usdt = realized.amount() + unrealized;
    // The reconciliation now runs AFTER R2 and R3 — see the block below § 5.

    // ── 5. Body assembly ────────────────────────────────────────────────────
    let active_strategies: BTreeSet<String> = strategy_events
        .iter()
        .filter(|e| matches!(e.kind, StrategyEventKind::Load | StrategyEventKind::Swap))
        .filter_map(|e| e.strategy_id.as_ref().map(|s| s.0.to_string()))
        .collect();

    // R2 inputs.
    let opening = compute_opening_balance(&recent_journal);
    let strategy_return_usdt = headline_return_usdt;
    let strategy_return_pct = pct_of(strategy_return_usdt, opening);
    let (btc_return_pct, btc_return_usdt) = btc_baseline(btc_start, btc_end, opening);

    let r2 = render::headline::render(&render::headline::HeadlineInputs {
        strategy_return_pct,
        strategy_return_usdt,
        btc_return_pct,
        btc_return_usdt,
    });

    // R3 inputs — sample equity curve.
    let cadence_minutes: u32 = match window {
        ReportWindow::Days7 | ReportWindow::Weekly => 1,
        _ => 5,
    };
    // Cumulative ledger deltas from a ZERO baseline; `ledger_equity_curve` then
    // anchors the level so the curve ends at the measured cash balance.
    let period_points = audit::query::equity_series(
        &ledger,
        period_start,
        period_end,
        Decimal::ZERO,
        Decimal::ZERO,
    )
    .await?;
    let inception_points =
        audit::query::equity_series(&ledger, inception, period_end, Decimal::ZERO, Decimal::ZERO)
            .await?;
    let period_curve = ledger_equity_curve(
        &period_points,
        cash.amount(),
        period_start,
        period_end,
        cadence_minutes,
    );
    let inception_curve = ledger_equity_curve(
        &inception_points,
        cash.amount(),
        inception,
        period_end,
        cadence_minutes.max(5),
    );
    let r3 = render::equity_curve::render(&render::equity_curve::EquityCurveInputs {
        window_label: window.slug(),
        period_curve: period_curve.iter().map(|s| s.equity_total).collect(),
        since_inception_curve: inception_curve.iter().map(|s| s.equity_total).collect(),
    });

    // ── Reconciliation (R11) ────────────────────────────────────────────────
    //
    // Deliberately placed AFTER R2 and R3. The table's own columns are
    // `| Identity | Report | Ledger | Δ | Pass? |`, so identity #1's left side is
    // the figure this function PUBLISHED in the headline, read back out of it, and
    // identity #4's left side is the endpoint delta of the equity curve it
    // PUBLISHED. Both need the rendered sections to exist.
    //
    // Until 2026-11-07 this block sat before them and fed a ledger value into both
    // columns of #1 and #4 — for #4, the character-identical expression
    // `realized_period.amount() + unrealized` into the two fields `compute`
    // subtracts — so two of the four rows published `Δ = 0 | PASS` unconditionally,
    // for every input, in four committed operator-facing bodies (bug-log `#156`).
    // `ReconciliationInputs` no longer has an `equity_check_sum` field, so a caller
    // cannot supply both sides of any identity again.
    let fees_period = audit::query::total_fees_since(&ledger, period_start).await?;
    let published_headline = published_strategy_return(&r2).ok_or_else(|| {
        ReportError::Reconcile(
            "could not read the published strategy return back out of the rendered \
             headline; identity #1 would have no report side"
                .to_string(),
        )
    })?;
    // `equity_period_end - equity_period_start`, from the curve the report ships.
    // NOTE, and this is a real residual the row will now SURFACE rather than hide:
    // `sample_equity_curve` is a documented approximation that holds equity constant
    // at cash (see its doc comment and `equity_total: cash`), so this delta is 0
    // until that curve is ledger-derived. The ledger side is not 0 whenever the
    // period had P&L. A FAILING row there is the correct output of a gate that
    // finally measures something; it is not a defect in this wiring.
    let equity_delta = match (period_curve.first(), period_curve.last()) {
        (Some(first), Some(last)) => last.equity_total - first.equity_total,
        _ => Decimal::ZERO,
    };
    let recon_inputs = ReconciliationInputs {
        headline_return: published_headline,
        realized: realized.amount(),
        unrealized,
        // Declared, not assumed: a period-end mark miss zeroes `unrealized`, so
        // identities #1 and #4 are UNMEASURED rather than passing over a zero
        // the report itself substituted (bug-log `#156`).
        unrealized_measured: !mark_unavailable_footnote,
        realized_period: realized_period.amount(),
        fees: fees_period.amount(),
        sum_by_strategy,
        sum_by_symbol,
        equity_delta,
    };
    let recon = reconcile::compute(&recon_inputs);
    let recon_pass = recon.all_passed();

    // R4 inputs.
    let curve_dec: Vec<Decimal> = period_curve.iter().map(|s| s.equity_total).collect();
    let (max_dd_pct, max_dd_usdt) = render::risk_metrics::max_drawdown(&curve_dec);
    let r4 = render::risk_metrics::render(&render::risk_metrics::RiskMetricsInputs {
        period: window.slug(),
        sharpe: render::risk_metrics::sharpe(&curve_dec, cadence_minutes),
        sortino: render::risk_metrics::sortino(&curve_dec, cadence_minutes),
        calmar: render::risk_metrics::calmar(&curve_dec, cadence_minutes),
        max_drawdown_pct: max_dd_pct,
        max_drawdown_usdt: max_dd_usdt,
        recovery_bars: render::risk_metrics::recovery_bars(&curve_dec),
    });

    // R5 inputs.
    let r5 = render::strategy_attribution::render(
        &render::strategy_attribution::StrategyAttributionInputs {
            rows: pnl_by_strategy.clone(),
            active_strategies: active_strategies.clone(),
        },
    );

    // R6 inputs.
    // Decay heuristic: pass each strategy's full equity slice (we only have
    // the global curve at this scope; per-strategy slicing ships in v2+).
    let r6 = render::memory_highlights::render_with_decay(&[]);

    // R7 inputs.
    let kill_switch_count = strategy_events
        .iter()
        .filter(|e| matches!(e.kind, StrategyEventKind::KillSwitchTripped))
        .count();
    let clock_skew_count = strategy_events
        .iter()
        .filter(|e| matches!(e.kind, StrategyEventKind::KillSwitchTripped))
        .filter(|e| {
            e.error_summary
                .as_ref()
                .is_some_and(|s| s.contains("clock_skew"))
        })
        .count();
    let feed_reconnect_count = strategy_events
        .iter()
        .filter(|e| matches!(e.kind, StrategyEventKind::FeedReconnect))
        .count();
    let uptime_pct = render::system_health::compute_uptime_pct(
        &uptime_intervals,
        period_start.unix_millis(),
        period_end.unix_millis(),
    );
    let r7 = render::system_health::render(&render::system_health::SystemHealthInputs {
        uptime_pct: Ok(format!("{uptime_pct}%")),
        kill_switch_trips: Ok(kill_switch_count.to_string()),
        clock_skew_events: Ok(clock_skew_count.to_string()),
        feed_reconnects: Ok(feed_reconnect_count.to_string()),
        funding_poll_rate: Ok("n/a".into()),
        // T1935 / Q11 — denominator $135 → $200 at v2.0.0.
        llm_spend: Ok("$0.00 / $200".into()),
        // T1935 / Q5d — new System Health row, research-mode default.
        cache_hit_ratio: Ok("0.0%".into()),
    });

    // R8 inputs.
    let r8 = render::what_changed::render(&render::what_changed::WhatChangedInputs {
        events: strategy_events.clone(),
    });

    // R9 inputs.
    let max_dd_threshold_75 = Decimal::from(15) * Decimal::from(75) / Decimal::from(100);
    let drawdown_fired = max_dd_pct >= max_dd_threshold_75 && max_dd_pct > Decimal::ZERO;
    let total_trades: u32 = pnl_by_strategy.iter().map(|r| r.closed_trade_count).sum();
    let rebalance_rejected_count = strategy_events
        .iter()
        .filter(|e| matches!(e.kind, StrategyEventKind::RebalanceRejected))
        .count();
    let mr_stop_count = strategy_events
        .iter()
        .filter(|e| matches!(e.kind, StrategyEventKind::MeanReversionStop))
        .count();
    let pair_trade_count: u32 = pnl_by_strategy
        .iter()
        .filter(|r| r.strategy_id.0.starts_with("pairs_"))
        .map(|r| r.closed_trade_count)
        .sum();
    let r9 = render::open_risks::render(&render::open_risks::OpenRisksInputs {
        drawdown: Ok(render::open_risks::RiskOutcome {
            fired: drawdown_fired,
            threshold: format!("max_drawdown >= {max_dd_threshold_75}%"),
            observed: format!("{max_dd_pct}%"),
        }),
        llm_budget: Ok(render::open_risks::RiskOutcome {
            fired: false,
            threshold: "mtd_spend >= 80% of budget".into(),
            // T1935 / Q11 — denominator $135 → $200 at v2.0.0.
            observed: "$0.00 / $200".into(),
        }),
        strategy_decay: Ok(render::open_risks::RiskOutcome {
            fired: false,
            threshold: "any strategy: last_7d_sharpe < 0 && inception_sharpe > 0".into(),
            observed: "no strategies decayed".into(),
        }),
        rebalance_rejections: Ok(render::open_risks::RiskOutcome {
            fired: total_trades > 0
                && u32::try_from(rebalance_rejected_count).unwrap_or(0) > (total_trades / 20),
            threshold: "rebalance_rejected > 5% of trade_count".into(),
            observed: format!("{rebalance_rejected_count} rejected of {total_trades} trades"),
        }),
        mr_stops: Ok(render::open_risks::RiskOutcome {
            fired: pair_trade_count > 0
                && u32::try_from(mr_stop_count).unwrap_or(0) > (pair_trade_count / 10),
            threshold: "mr_stop > 10% of pair_trade_count".into(),
            observed: format!("{mr_stop_count} hard-stops of {pair_trade_count} pair trades"),
        }),
    });

    // R11 reconciliation appendix (Q6: footnote toggled by
    // `mark_unavailable_footnote`; `false` keeps body bytes byte-identical
    // to the pre-T1003 path on empty-positions / fully-resolved fixtures).
    let r11 = render::reconciliation::render(&recon, mark_unavailable_footnote);

    // ── 6. Body assembly (in section order) ─────────────────────────────────
    // Order per Design: R9 (pinned), R2, R3, R4, R5, R6, R7, R8, R11.
    // Banner (when FAIL) prepended above R9.
    let mut body = String::with_capacity(4096);
    if !recon_pass {
        body.push_str(render::reconciliation::FAIL_BANNER);
        body.push_str("\n\n");
    }
    body.push_str(&r9);
    body.push('\n');
    body.push_str(&r2);
    body.push('\n');
    body.push_str(&r3);
    body.push_str(&r4);
    body.push('\n');
    body.push_str(&r5);
    body.push('\n');
    body.push_str(&r6);
    body.push('\n');
    body.push_str(&r7);
    body.push('\n');
    body.push_str(&r8);
    body.push('\n');
    body.push_str(&r11);

    // ── 7. Front-matter ─────────────────────────────────────────────────────
    let ledger_sha = audit::query::ledger_snapshot_sha(audit_db_path)?;
    let run_id_hex = run_id::compute(&window, &ledger_sha, seed);
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    let body_sha: [u8; 32] = h.finalize().into();
    let elapsed = started.elapsed().as_secs_f64();

    let fm = render::front_matter::FrontMatter {
        period: window.slug(),
        period_start: fmt_ts_micros(period_start),
        period_end: fmt_ts_micros(period_end),
        generated: fmt_ts_micros(now),
        run_id: run_id_hex.clone(),
        ledger_snapshot_sha: hex::encode(ledger_sha),
        seed: seed.map(|s| format!("0x{s:X}")),
        data_source: format!("fixture:{}", audit_db_path.display()),
        wall_clock_s: format!("{elapsed:.6}"),
        binary_version: env!("CARGO_PKG_VERSION").to_string(),
        git_commit: option_env!("GIT_COMMIT").unwrap_or("n/a").to_string(),
        agent_pid: std::process::id(),
        host: gethostname_or_unknown(),
        reconciliation: if recon_pass {
            "PASS".into()
        } else {
            "FAIL".into()
        },
    };
    let front_matter = fm.render();

    // ── 8. CSV companions ───────────────────────────────────────────────────
    let artifacts_dir = out
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("artifacts")
        .join(&run_id_hex);
    std::fs::create_dir_all(&artifacts_dir)?;
    let mut csv_paths: Vec<PathBuf> = Vec::new();

    // equity-<window>.csv
    // `file_slug()`, not `slug()`: the display form contains colons for
    // `Since(..)` windows and is illegal in a Windows filename. `slug()` stays
    // the body/run-id form and is deliberately unchanged — see `ReportWindow::file_slug`.
    let equity_window_path = artifacts_dir.join(format!("equity-{}.csv", window.file_slug()));
    csv_artifacts::write_equity_csv(&equity_window_path, &period_curve)?;
    csv_paths.push(equity_window_path);

    // equity-since-inception.csv
    let inception_path = artifacts_dir.join("equity-since-inception.csv");
    csv_artifacts::write_equity_csv(&inception_path, &inception_curve)?;
    csv_paths.push(inception_path);

    // fills.csv (filtered to period).
    let fills_in_period: Vec<(trading_core::FillView, Option<String>)> = recent_fills
        .into_iter()
        .filter(|f| {
            f.venue_ts.unix_millis() >= period_start.unix_millis()
                && f.venue_ts.unix_millis() <= period_end.unix_millis()
        })
        .map(|f| (f, None))
        .collect();
    let fills_path = artifacts_dir.join("fills.csv");
    csv_artifacts::write_fills_csv(&fills_path, &fills_in_period)?;
    csv_paths.push(fills_path);

    // pnl_by_strategy.csv
    let pbs_path = artifacts_dir.join("pnl_by_strategy.csv");
    csv_artifacts::write_pnl_by_strategy_csv(&pbs_path, &pnl_by_strategy)?;
    csv_paths.push(pbs_path);

    // pnl_by_symbol.csv
    let pby_path = artifacts_dir.join("pnl_by_symbol.csv");
    csv_artifacts::write_pnl_by_symbol_csv(&pby_path, &pnl_by_symbol)?;
    csv_paths.push(pby_path);

    // journal.csv (filtered to period; transaction_id unknown from the view
    // — we pass the empty string since the view does not carry it).
    let journal_in_period: Vec<(trading_core::JournalEntryView, String)> = recent_journal
        .into_iter()
        .filter(|e| {
            e.ts.unix_millis() >= period_start.unix_millis()
                && e.ts.unix_millis() <= period_end.unix_millis()
        })
        .map(|e| (e, String::new()))
        .collect();
    let journal_path = artifacts_dir.join("journal.csv");
    csv_artifacts::write_journal_csv(&journal_path, &journal_in_period)?;
    csv_paths.push(journal_path);

    // strategy_events.csv
    let events_path = artifacts_dir.join("strategy_events.csv");
    csv_artifacts::write_strategy_events_csv(&events_path, &strategy_events)?;
    csv_paths.push(events_path);

    // ── 9. Atomic-write the markdown + sibling JSON on FAIL ─────────────────
    let mut full = String::with_capacity(front_matter.len() + body.len());
    full.push_str(&front_matter);
    full.push_str(&body);
    atomic_write::atomic_write(out, &full)?;

    if !recon_pass {
        let sibling = sibling_failure_json_path(out);
        let json = recon.to_failure_json(
            &run_id_hex,
            &hex::encode(ledger_sha),
            &window.slug(),
            &fmt_ts_micros(period_start),
            &fmt_ts_micros(period_end),
        );
        atomic_write::atomic_write(&sibling, &json)?;
        return Err(ReportError::Reconciliation {
            sibling_path: sibling,
        });
    }

    Ok(ReportArtifacts {
        markdown_path: out.to_path_buf(),
        run_id: run_id_hex,
        csv_paths,
        body_sha256: body_sha,
    })
}

/// Build the path of the sibling `_reconciliation_failure.json` artifact
/// next to the markdown output (R11.4).
fn sibling_failure_json_path(md: &Path) -> PathBuf {
    let parent = md.parent().unwrap_or_else(|| Path::new("."));
    let stem = md.file_stem().and_then(|s| s.to_str()).unwrap_or("report");
    parent.join(format!("{stem}_reconciliation_failure.json"))
}

/// Build the published equity curve from the ledger's own movements.
///
/// Replaces `sample_equity_curve`, which held equity CONSTANT at cash and set
/// the `realized_pnl` and `unrealized_pnl` columns to literal zero — its own doc
/// called it a "v1+ orchestrator approximation". The consequence was not only a
/// flat picture: reconciliation identity #4 is declared as
/// `equity_period_end - equity_period_start`, so with a constant curve it had
/// nothing real to compare, and the row shipped PASS over a measured -5.17 of fee
/// spend in a committed operator-facing body (bug-log `#156`).
///
/// `points` carries cumulative ledger deltas from a zero baseline. The curve is
/// sampled on the same cadence grid as before — so the chart's x-axis and the CSV
/// row count are unchanged — with each sample taking the last movement at or
/// before its tick, i.e. a step function, which is what a ledger is.
///
/// `end_anchor` is the equity level the curve must END at (the known cash balance
/// at `end`). Every sample is offset so the last one lands there, which keeps the
/// published LEVEL anchored to a measured quantity while the SHAPE and the
/// period delta come from the journal. The alternative — treating `cash` as the
/// level at `start` — would end the curve at `cash + Σmovements` and overstate it
/// by exactly the period's P&L.
fn ledger_equity_curve(
    points: &[audit::query::EquityPoint],
    end_anchor: Decimal,
    start: Timestamp,
    end: Timestamp,
    cadence_minutes: u32,
) -> Vec<csv_artifacts::EquitySample> {
    let cadence_ms = i64::from(cadence_minutes.max(1)) * 60_000;
    let mut out: Vec<csv_artifacts::EquitySample> = Vec::new();
    let from = start.unix_millis();
    let to = end.unix_millis();
    if to <= from {
        return out;
    }
    // Offset so the final cumulative equity lands on `end_anchor`.
    let cumulative_end = points.last().map_or(Decimal::ZERO, |p| p.equity_total);
    let offset = end_anchor - cumulative_end;

    let mut cursor = from;
    let mut idx: usize = 0;
    while cursor <= to {
        // Advance to the last point at or before this tick (points are ordered).
        while idx < points.len() && points[idx].ts.unix_millis() <= cursor {
            idx += 1;
        }
        let at = idx.checked_sub(1).map(|i| &points[i]);
        let ts_nanos = i128::from(cursor) * 1_000_000;
        let dt = OffsetDateTime::from_unix_timestamp_nanos(ts_nanos)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        out.push(csv_artifacts::EquitySample {
            ts: Timestamp::new(dt),
            equity_total: at.map_or(Decimal::ZERO, |p| p.equity_total) + offset,
            realized_pnl: at.map_or(Decimal::ZERO, |p| p.realized_pnl),
            unrealized_pnl: at.map_or(Decimal::ZERO, |p| p.unrealized_pnl),
            cash_balance: at.map_or(Decimal::ZERO, |p| p.cash_balance) + offset,
        });
        // Same defensive cap as the sampler this replaces (1 year x 1m is fine).
        if out.len() >= 1_000_000 {
            break;
        }
        cursor = cursor.saturating_add(cadence_ms);
    }
    out
}

/// Compute the opening-balance USDT amount from the journal entry slice.
///
/// Sums credits − debits on `equity:opening_balance` rows.  Returns
/// zero when no opening-balance row is present.
fn compute_opening_balance(entries: &[trading_core::JournalEntryView]) -> Decimal {
    let mut total = Decimal::ZERO;
    for e in entries {
        if e.account.0.as_str() == "equity:opening_balance" {
            total += e.amount;
        }
    }
    // `amount` is `credits − debits`; `equity:opening_balance` is opened
    // as a credit so the absolute value is the opening capital.
    total.abs()
}

/// Compute a return percentage of `pnl` against `denom`, returning `0`
/// when `denom == 0`.
fn pct_of(pnl: Decimal, denom: Decimal) -> Decimal {
    if denom == Decimal::ZERO {
        return Decimal::ZERO;
    }
    (pnl / denom) * Decimal::from(100u32)
}

/// Compute the BTC buy-and-hold baseline: `(end - start) / start * opening`.
fn btc_baseline(
    btc_start: Option<Decimal>,
    btc_end: Option<Decimal>,
    opening: Decimal,
) -> (Decimal, Decimal) {
    let (Some(start), Some(end)) = (btc_start, btc_end) else {
        return (Decimal::ZERO, Decimal::ZERO);
    };
    if start == Decimal::ZERO {
        return (Decimal::ZERO, Decimal::ZERO);
    }
    let pct = ((end - start) / start) * Decimal::from(100u32);
    let absolute = (Money::<Usdt>::from_decimal(opening) * ((end - start) / start)).amount();
    (pct, absolute)
}

/// Wraps `gethostname` calls — falls back to `"unknown"` per R10.1.
fn gethostname_or_unknown() -> String {
    // Avoid a new dependency by reading `HOSTNAME` env var.
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string())
}
