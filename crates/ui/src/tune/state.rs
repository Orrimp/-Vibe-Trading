//! Gate-tied hyperparameter sweep result mirror (ADR-0069 T5).
//!
//! ## The ONE boundary (T5 contract)
//!
//! `SweepReportMirror::from_report` is the ONLY place a `backtest::SweepReport`
//! is read. Everything downstream (state, view, fixtures, render tests) works on
//! the mirror. This mirrors the `BakeoffReportMirror::from_report` precedent in
//! `leaderboard/state.rs:223` — that is the established pattern; follow it exactly.
//!
//! ## Mirror discipline (INVARIANT)
//!
//! `ui` MUST NOT import `strategy` / `exec` / `forecast` / `llm`. The sweep
//! result type (`backtest::SweepReport`) is consumed through the existing `backtest`
//! dep — the same sanctioned seam. The mirror carries only plain `SmolStr` /
//! `Decimal` / `f64` fields so the view is trivially pure and the mirror is
//! unit-constructible in fixtures without standing up the engine.
//!
//! ## `cargo tree -p ui` stays UNCHANGED
//!
//! No new crate edge is introduced. `backtest` is already a dep of `ui`.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use smol_str::SmolStr;

use crate::leaderboard::state::RobustnessLabel;

// ── Verdict label (UI-side closed enum) ──────────────────────────────────────

/// Robustness verdict for one sweep cell, mirrored from
/// `backtest::bakeoff::robustness::ParamRobustnessVerdict`.
///
/// UI-side closed enum — the screen/render code never matches on an engine type.
/// Reuses the existing `RobustnessLabel` from `leaderboard/state.rs` (same
/// display mapping).
pub type SweepVerdictLabel = RobustnessLabel;

/// Map a `backtest::bakeoff::robustness::ParamRobustnessVerdict` to the UI label.
fn verdict_label(v: backtest::bakeoff::robustness::ParamRobustnessVerdict) -> SweepVerdictLabel {
    use backtest::bakeoff::robustness::ParamRobustnessVerdict;
    match v {
        ParamRobustnessVerdict::Robust => RobustnessLabel::Robust,
        ParamRobustnessVerdict::Marginal => RobustnessLabel::Marginal,
        ParamRobustnessVerdict::Fragile => RobustnessLabel::Fragile,
    }
}

// ── Distribution summary mirror ───────────────────────────────────────────────

/// Bootstrap distribution summary for one sweep cell, mirrored as pure scalars.
///
/// These are the five gate signals surfaced to the operator (R3):
/// p5/p50/p95 Sharpe, P(loss), P(Sharpe>1), p95 `MaxDD`.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepDistributionMirror {
    /// p5 Sharpe (5th percentile of the bootstrap Sharpe distribution).
    pub sharpe_p5: f64,
    /// p50 Sharpe (median of the bootstrap Sharpe distribution).
    pub sharpe_p50: f64,
    /// p95 Sharpe (95th percentile of the bootstrap Sharpe distribution).
    pub sharpe_p95: f64,
    /// Probability the strategy loses money under resampling (P(`equity_final` < initial)).
    pub prob_loss: f64,
    /// Probability the bootstrap Sharpe exceeds 1.0.
    pub prob_sharpe_gt1: f64,
    /// p95 `MaxDrawdown` (95th percentile of bootstrap `MaxDD` — the tail risk).
    pub maxdd_p95: f64,
}

impl SweepDistributionMirror {
    /// Extract the five gate signals from a `backtest::DistributionSummary`.
    #[must_use]
    pub fn from_distribution(d: &backtest::DistributionSummary) -> Self {
        Self {
            sharpe_p5: d.sharpe.p5,
            sharpe_p50: d.sharpe.p50,
            sharpe_p95: d.sharpe.p95,
            prob_loss: d.prob_loss,
            prob_sharpe_gt1: d.prob_sharpe_gt_1,
            maxdd_p95: d.max_dd_tail_p95,
        }
    }
}

// ── Promote params (the UI-side structured carrier — ADR-0070 § D4) ───────────

/// Structured tuned params for one swept cell — the **UI-side closed enum** that
/// crosses into `Message::PromoteSweptConfig` when the operator clicks
/// "Use this config" on a promotable row (advisor-param-promotion, ADR-0070).
///
/// Mirrors the four sweep families one-for-one, carrying ONLY plain scalars (no
/// engine type) so it lives happily on `SweepCellRow` and in a `Message`. `k` is
/// encoded as **tenths** (`k_tenths = 20` → 2.0σ) — the existing
/// `PlanRuleKind::BollingerReversion` convention — so the enum stays `Clone + Eq`
/// with no `Decimal` field, and the binary layer converts `k_tenths → Decimal`
/// exactly where it builds the `agent::ForwardParamOverride`.
///
/// Populated at the ONE engine→UI boundary ([`cell_to_row`], from
/// `backtest::SweptParams`) so `from_report` stays the ONLY place a
/// `backtest::SweptParams` is read — no engine type ever reaches `view`/`update`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromoteParams {
    /// SMA crossover with tuned window lengths.
    Sma {
        /// Fast window (periods).
        fast_len: u32,
        /// Slow window (periods).
        slow_len: u32,
    },
    /// MACD trend with tuned EMA periods.
    Macd {
        /// Fast EMA period.
        fast: u32,
        /// Slow EMA period.
        slow: u32,
        /// Signal-line smoothing period.
        signal: u32,
    },
    /// RSI reversion with tuned period and oversold threshold.
    Rsi {
        /// RSI lookback period.
        period: u32,
        /// Oversold entry threshold.
        oversold: u32,
    },
    /// Bollinger reversion with tuned period and k (encoded as tenths; 20 → 2.0σ).
    Bollinger {
        /// Bollinger lookback period.
        period: u32,
        /// Band multiplier × 10 (`k_tenths = 20` → 2.0σ).
        k_tenths: u32,
    },
}

impl PromoteParams {
    /// Map a `backtest::SweptParams` to the UI-side carrier. The ONLY place a
    /// `backtest::SweptParams` is read for promotion (called from [`cell_to_row`],
    /// itself reachable only through `from_report` — the ONE boundary). Pure +
    /// total: the Bollinger `k: Decimal` is quantised to tenths via `× 10` (the
    /// sweep grid's `k` presets are tenths-exact — `{1.5, 2.0, 2.5, 3.0}` — so the
    /// round is lossless for every config the gate scored).
    #[must_use]
    fn from_swept(params: &backtest::SweptParams) -> Self {
        use backtest::SweptParams;
        match params {
            SweptParams::Sma { fast_len, slow_len } => Self::Sma {
                fast_len: *fast_len,
                slow_len: *slow_len,
            },
            SweptParams::Macd { fast, slow, signal } => Self::Macd {
                fast: *fast,
                slow: *slow,
                signal: *signal,
            },
            SweptParams::Rsi { period, oversold } => Self::Rsi {
                period: *period,
                oversold: *oversold,
            },
            SweptParams::Bollinger { period, k } => Self::Bollinger {
                period: *period,
                // k → tenths (2.0 → 20). `to_u32` after the ×10 quantise; defends
                // against an exotic non-finite k with a 0 fallback (never expected
                // — the grid presets are tenths-exact).
                k_tenths: (k * Decimal::from(10)).round().to_u32().unwrap_or(0),
            },
        }
    }

    /// A short human-readable label for the params, for the honesty copy
    /// (e.g. `"fast 10 / slow 20"`, `"MACD 8 / 20 / 5"`). Distinct from the
    /// grid's terse `params_label` — this reads in a sentence.
    #[must_use]
    pub fn label(&self) -> SmolStr {
        match self {
            Self::Sma { fast_len, slow_len } => {
                SmolStr::new(format!("fast {fast_len} / slow {slow_len}"))
            }
            Self::Macd { fast, slow, signal } => {
                SmolStr::new(format!("MACD {fast} / {slow} / {signal}"))
            }
            Self::Rsi { period, oversold } => {
                SmolStr::new(format!("RSI {period}, oversold {oversold}"))
            }
            Self::Bollinger { period, k_tenths } => SmolStr::new(format!(
                "Bollinger {period}, k {}.{}",
                k_tenths / 10,
                k_tenths % 10
            )),
        }
    }
}

// ── Per-cell row ──────────────────────────────────────────────────────────────

/// One swept cell — the sweep analogue of `LeaderRow`.
///
/// Free of every engine type: `params_label` is a plain `SmolStr`, KPIs are
/// `Decimal` / `f64` / `usize`, the verdict is the closed UI-side `SweepVerdictLabel`.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepCellRow {
    /// Short label for the params, e.g. `"fast=10, slow=20"`.
    pub params_label: SmolStr,
    /// Robustness verdict label (Robust/Marginal/Fragile).
    pub verdict: SweepVerdictLabel,
    /// Whether this config can be promoted (false iff `verdict == Fragile`).
    ///
    /// The "Use this config →" affordance is DISABLED + GREYED on FRAGILE rows
    /// (the honesty lock mirrors the leaderboard "Fragile cannot be crowned").
    /// Promotion wiring itself is OUT OF SCOPE for v0.1 — but the disabled-on-
    /// fragile flag ships from day 1 so the honesty is visible immediately.
    pub promotable: bool,
    /// In-sample Sharpe (hourly, annualised). De-emphasised — "the distribution
    /// is what the gate judges".
    pub in_sample_sharpe: f64,
    /// In-sample total return fraction (`0.1` = +10%). De-emphasised.
    pub in_sample_return: Decimal,
    /// In-sample max drawdown fraction. De-emphasised.
    pub in_sample_maxdd: Decimal,
    /// Number of executed trades (in-sample).
    pub trade_count: usize,
    /// Bootstrap distribution summary (the ANTI-OVERFITTING affordance, R3).
    pub distribution: SweepDistributionMirror,
    /// advisor-param-promotion (ADR-0070 § D4) — the structured tuned params,
    /// carried so the "Use this config" affordance can reconstruct the promote
    /// target without re-reading any engine type. Present on EVERY cell (incl.
    /// fragile — the data is here; only the *affordance* is gated behind
    /// `promotable` in the view). Populated at the ONE boundary ([`cell_to_row`]).
    pub promote_params: PromoteParams,
}

// ── KPIs mirror ───────────────────────────────────────────────────────────────

/// Mirror of `backtest::CandidateKpis` for the benchmark arm.
///
/// Used for the buy-and-hold header strip ("vs just holding {coin}: …").
#[derive(Debug, Clone, PartialEq)]
pub struct SweepBenchmarkKpis {
    /// Annualised Sharpe.
    pub sharpe: f64,
    /// Total return fraction.
    pub total_return_pct: Decimal,
    /// Max drawdown fraction.
    pub max_drawdown: Decimal,
}

// ── The report mirror (the ONE boundary) ─────────────────────────────────────

/// Mirror of `backtest::SweepReport` — the pure-`ui` shape the Tune screen
/// renders from.
///
/// `from_report` is the ONLY place a `backtest::SweepReport` is consumed.
/// Downstream code (state, view, fixtures, render tests) NEVER touches the
/// engine type — only this mirror.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepReportMirror {
    /// Strategy family label (e.g. `"SMA crossover"`).
    pub family_label: SmolStr,
    /// Coin (e.g. `"BTCUSDT"`).
    pub coin: SmolStr,
    /// Human-readable range label (e.g. `"2024 H1"`).
    pub range_label: SmolStr,
    /// Number of cells in the result (= the capped, valid count).
    pub grid_size: usize,
    /// Whether the operator's requested grid was truncated to `MAX_SWEEP_CONFIGS`.
    pub truncated: bool,
    /// The total requested count (before truncation + validity filtering).
    pub requested_count: usize,
    /// Per-cell rows, in insertion order (= grid axis-major order).
    pub cells: Vec<SweepCellRow>,
    /// The shipped-config baseline row (the divergence anchor).
    pub baseline: SweepCellRow,
    /// Buy-and-hold benchmark KPIs (always shown).
    pub benchmark_kpis: SweepBenchmarkKpis,
}

impl SweepReportMirror {
    /// Build the mirror from a `backtest::SweepReport`.
    ///
    /// This is the ONLY place an engine `SweepReport` is read. Everything
    /// downstream (state, view, fixtures, render tests) works on the mirror.
    /// Pure + total (no I/O, no panic).
    #[must_use]
    pub fn from_report(report: &backtest::SweepReport) -> Self {
        let cells = report.cells.iter().map(cell_to_row).collect();
        let baseline = cell_to_row(&report.baseline);
        let benchmark_kpis = SweepBenchmarkKpis {
            sharpe: report.benchmark.sharpe,
            total_return_pct: report.benchmark.total_return_pct,
            max_drawdown: report.benchmark.max_drawdown,
        };
        Self {
            family_label: report.config_echo.family_label.clone(),
            coin: report.config_echo.coin.clone(),
            range_label: report.config_echo.range_label.clone(),
            grid_size: report.config_echo.grid_size,
            truncated: report.config_echo.truncated,
            requested_count: report.config_echo.requested_count,
            cells,
            baseline,
            benchmark_kpis,
        }
    }
}

/// Map one `backtest::SweepCellResult` → `SweepCellRow`.
fn cell_to_row(cell: &backtest::SweepCellResult) -> SweepCellRow {
    let verdict = verdict_label(cell.verdict);
    let promotable = !matches!(verdict, RobustnessLabel::Fragile);
    SweepCellRow {
        params_label: cell.params.label(),
        verdict,
        promotable,
        in_sample_sharpe: cell.kpis.sharpe,
        in_sample_return: cell.kpis.total_return_pct,
        in_sample_maxdd: cell.kpis.max_drawdown,
        trade_count: cell.kpis.trade_count,
        distribution: SweepDistributionMirror::from_distribution(&cell.distribution),
        // ADR-0070 § D4 — map the engine `SweptParams` to the UI carrier HERE (the
        // ONE boundary). Every cell carries it; the view gates the affordance.
        promote_params: PromoteParams::from_swept(&cell.params),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::float_arithmetic,
    clippy::similar_names,   // sharpe_p5 / sharpe_p95 are intentionally paired
    clippy::float_cmp,       // exact f64 round-trip from fixture is fine in tests
)]
mod tests {
    use super::*;
    use backtest::{
        CandidateKpis, SweepCellResult, SweepReport, SweepRequestEcho, SweptParams,
        bakeoff::robustness::ParamRobustnessVerdict,
        stats::{DistributionSummary, MetricDistribution},
    };
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;

    // ── Fixture helpers ───────────────────────────────────────────────────────

    fn zero_metric() -> MetricDistribution {
        MetricDistribution {
            mean: 0.0,
            std: 0.0,
            p5: 0.0,
            p25: 0.0,
            p50: 0.0,
            p75: 0.0,
            p95: 0.0,
            min: 0.0,
            max: 0.0,
        }
    }

    fn make_dist(
        sharpe_p5: f64,
        sharpe_p50: f64,
        sharpe_p95: f64,
        prob_loss: f64,
        prob_sharpe_gt1: f64,
        maxdd_p95: f64,
    ) -> DistributionSummary {
        DistributionSummary {
            sharpe: MetricDistribution {
                mean: sharpe_p50,
                std: 0.1,
                p5: sharpe_p5,
                p25: sharpe_p5,
                p50: sharpe_p50,
                p75: sharpe_p50,
                p95: sharpe_p95,
                min: sharpe_p5,
                max: sharpe_p95,
            },
            sortino: zero_metric(),
            calmar: zero_metric(),
            max_drawdown: MetricDistribution {
                mean: 0.3,
                std: 0.05,
                p5: 0.1,
                p25: 0.2,
                p50: 0.3,
                p75: 0.4,
                p95: maxdd_p95,
                min: 0.05,
                max: 0.7,
            },
            total_return: zero_metric(),
            prob_loss,
            prob_sharpe_gt_0: 0.8,
            prob_sharpe_gt_1: prob_sharpe_gt1,
            max_dd_tail_p50: 0.3,
            max_dd_tail_p95: maxdd_p95,
            // P1-2 fields: zero for test helper (gate tests don't read these).
            cvar_95: 0.0,
            cvar_99: 0.0,
            median_terminal_wealth: 0.0,
            skew: 0.0,
        }
    }

    fn make_kpis(sharpe: f64, ret_pct: rust_decimal::Decimal, trades: usize) -> CandidateKpis {
        CandidateKpis {
            sharpe,
            sortino: sharpe * 1.1,
            calmar: sharpe * 0.9,
            total_return_pct: ret_pct,
            max_drawdown: dec!(0.15),
            trade_count: trades,
            turnover: rust_decimal::Decimal::ZERO,
        }
    }

    fn make_cell(
        fast: u32,
        slow: u32,
        verdict: ParamRobustnessVerdict,
        sharpe: f64,
    ) -> SweepCellResult {
        let dist = make_dist(
            if matches!(verdict, ParamRobustnessVerdict::Fragile) {
                -0.5
            } else {
                0.6
            },
            sharpe,
            sharpe + 0.5,
            if matches!(verdict, ParamRobustnessVerdict::Fragile) {
                0.5
            } else {
                0.1
            },
            if matches!(verdict, ParamRobustnessVerdict::Fragile) {
                0.2
            } else {
                0.65
            },
            if matches!(verdict, ParamRobustnessVerdict::Fragile) {
                0.8
            } else {
                0.3
            },
        );
        SweepCellResult {
            params: SweptParams::Sma {
                fast_len: fast,
                slow_len: slow,
            },
            kpis: make_kpis(sharpe, dec!(0.12), 5),
            verdict,
            distribution: dist,
            equity_curve: vec![],
        }
    }

    fn make_echo(grid_size: usize, truncated: bool) -> SweepRequestEcho {
        SweepRequestEcho {
            family_label: SmolStr::new_static("SMA crossover"),
            coin: SmolStr::new_static("BTCUSDT"),
            range_label: SmolStr::new_static("2024 H1"),
            grid_size,
            truncated,
            requested_count: if truncated { grid_size + 10 } else { grid_size },
            invalid_count: 2,
        }
    }

    fn make_report() -> SweepReport {
        SweepReport {
            config_echo: make_echo(3, false),
            cells: vec![
                make_cell(10, 20, ParamRobustnessVerdict::Robust, 1.2),
                make_cell(10, 30, ParamRobustnessVerdict::Marginal, 0.8),
                make_cell(15, 30, ParamRobustnessVerdict::Fragile, 2.5),
            ],
            baseline: make_cell(20, 50, ParamRobustnessVerdict::Marginal, 0.9),
            benchmark: make_kpis(0.5, dec!(0.08), 0),
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// T5.1 — `from_report` maps the correct number of cells.
    #[test]
    fn from_report_maps_correct_cell_count() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);
        assert_eq!(mirror.cells.len(), 3, "expected 3 cells");
        assert_eq!(mirror.grid_size, 3);
    }

    /// T5.2 — `promotable == false` iff `verdict == Fragile`.
    #[test]
    fn from_report_promotable_false_iff_fragile() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);

        for row in &mirror.cells {
            let is_fragile = matches!(row.verdict, RobustnessLabel::Fragile);
            assert_eq!(
                row.promotable, !is_fragile,
                "promotable must be false iff verdict == Fragile (params_label={})",
                row.params_label
            );
        }
        // Also check the baseline.
        let baseline_fragile = matches!(mirror.baseline.verdict, RobustnessLabel::Fragile);
        assert_eq!(
            mirror.baseline.promotable, !baseline_fragile,
            "baseline: promotable must be false iff Fragile"
        );
    }

    /// T5.3 — Baseline is mapped from `report.baseline` (the shipped config).
    #[test]
    fn from_report_baseline_is_shipped_config() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);
        assert_eq!(
            mirror.baseline.params_label.as_str(),
            "fast=20, slow=50",
            "baseline must map from the shipped config (fast=20, slow=50)"
        );
    }

    /// advisor-param-promotion (ADR-0070 § D4) — `from_report` populates a
    /// structured `promote_params` for EVERY cell (incl. the fragile one — the
    /// data is present; only the affordance is gated in the view). FAILS-before
    /// (the field did not exist).
    #[test]
    fn from_report_populates_promote_params_for_every_cell() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);

        // make_cell builds SMA cells (10/20, 10/30, 15/30); the carrier must
        // round-trip the exact tuned lens.
        assert_eq!(
            mirror.cells[0].promote_params,
            PromoteParams::Sma {
                fast_len: 10,
                slow_len: 20
            },
            "cell 0 promote_params must carry the tuned SMA lens"
        );
        // The FRAGILE cell (15/30) STILL carries its params — the lock is on the
        // affordance, not the data.
        assert_eq!(
            mirror.cells[2].promote_params,
            PromoteParams::Sma {
                fast_len: 15,
                slow_len: 30
            },
            "the FRAGILE cell must still carry promote_params (data present, \
             affordance gated)"
        );
        assert!(
            !mirror.cells[2].promotable,
            "sanity: cell 2 is the fragile one"
        );
        // The baseline carries the shipped lens.
        assert_eq!(
            mirror.baseline.promote_params,
            PromoteParams::Sma {
                fast_len: 20,
                slow_len: 50
            },
        );
    }

    /// ADR-0070 § D4 — `PromoteParams::from_swept` maps each family one-for-one,
    /// quantising the Bollinger `k: Decimal` to tenths (2.5 → 25) losslessly.
    #[test]
    fn promote_params_from_swept_maps_every_family() {
        assert_eq!(
            PromoteParams::from_swept(&SweptParams::Sma {
                fast_len: 7,
                slow_len: 14
            }),
            PromoteParams::Sma {
                fast_len: 7,
                slow_len: 14
            }
        );
        assert_eq!(
            PromoteParams::from_swept(&SweptParams::Macd {
                fast: 8,
                slow: 20,
                signal: 5
            }),
            PromoteParams::Macd {
                fast: 8,
                slow: 20,
                signal: 5
            }
        );
        assert_eq!(
            PromoteParams::from_swept(&SweptParams::Rsi {
                period: 10,
                oversold: 25
            }),
            PromoteParams::Rsi {
                period: 10,
                oversold: 25
            }
        );
        // k = 2.5σ → tenths 25 (the grid presets {1.5,2.0,2.5,3.0} are tenths-exact).
        assert_eq!(
            PromoteParams::from_swept(&SweptParams::Bollinger {
                period: 20,
                k: dec!(2.5)
            }),
            PromoteParams::Bollinger {
                period: 20,
                k_tenths: 25
            }
        );
    }

    /// T5.4 — `coin`, `range_label`, `family_label` are echoed correctly.
    #[test]
    fn from_report_echoes_request_metadata() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);
        assert_eq!(mirror.coin.as_str(), "BTCUSDT");
        assert_eq!(mirror.range_label.as_str(), "2024 H1");
        assert_eq!(mirror.family_label.as_str(), "SMA crossover");
    }

    /// T5.5 — Verdict labels are correctly mapped.
    #[test]
    fn from_report_verdict_labels_correct() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);
        assert_eq!(mirror.cells[0].verdict, RobustnessLabel::Robust);
        assert_eq!(mirror.cells[1].verdict, RobustnessLabel::Marginal);
        assert_eq!(mirror.cells[2].verdict, RobustnessLabel::Fragile);
    }

    /// T5.6 — Truncation flag is echoed.
    #[test]
    fn from_report_truncation_flag_echoed() {
        let mut report = make_report();
        report.config_echo.truncated = true;
        report.config_echo.requested_count = 34;
        let mirror = SweepReportMirror::from_report(&report);
        assert!(mirror.truncated, "truncated flag must be echoed");
        assert_eq!(mirror.requested_count, 34);
    }

    /// T5.7 — Benchmark KPIs are echoed from report.benchmark.
    #[test]
    fn from_report_benchmark_kpis_echoed() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);
        assert_eq!(mirror.benchmark_kpis.sharpe, 0.5_f64);
        assert_eq!(mirror.benchmark_kpis.total_return_pct, dec!(0.08));
    }

    /// T5.8 — Distribution mirror fields are mapped correctly.
    #[test]
    fn from_report_distribution_fields_mapped() {
        let report = make_report();
        let mirror = SweepReportMirror::from_report(&report);
        let first = &mirror.cells[0]; // Robust cell
        // Robust cell: sharpe_p5 = 0.6, prob_loss = 0.1, maxdd_p95 = 0.3
        assert!(
            first.distribution.sharpe_p5 > 0.0,
            "Robust: sharpe_p5 must be > 0"
        );
        assert!(
            first.distribution.prob_loss < 0.5,
            "Robust: prob_loss must be low"
        );

        let fragile = &mirror.cells[2]; // Fragile cell
        assert!(
            fragile.distribution.sharpe_p5 < 0.0,
            "Fragile: sharpe_p5 must be < 0"
        );
    }
}
