//! `SmaCrossover` + Composed scenario execution — Phase B T-D-N2.
//!
//! Extracted from `main.rs` inline bar loop @3206-3305 and `write_report`
//! @2488. Behaviour-preserving: same seed, same RNG draws, same loop order,
//! same fill/equity/KPI compute as the original.
//!
//! # Cancel poll (D6 / K3 mitigation)
//!
//! The bar loop checks `cancel.is_cancelled()` at every 128-bar boundary
//! (`bar_idx & 0x7F == 0`). On cancellation returns `Err(RunError::Cancelled)`.

use rust_decimal::Decimal;

// ── Shared compute helper (T-D-N12 / K8) ─────────────────────────────────────

/// Annualised Sharpe ratio from a minute-resolution equity curve.
///
/// Re-exported from `crates/backtest/src/lib.rs` per ADR-0035 § Decision 8 (T-D-N12).
/// This is the single source of truth; `main.rs`'s call site re-points here.
/// Signature locked: `pub fn compute_sharpe(equity_curve: &[Decimal]) -> f64`.
// Float arithmetic is required for Sharpe + drawdown stats per ADR-0003
// (Decimal for money/price/qty; f64 for statistics).
#[allow(clippy::float_arithmetic)]
// `n` is bounded by equity_curve.len() which is at most a year of minute bars
// (525 600); the precision loss is acceptable for statistical computation.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn compute_sharpe(equity_curve: &[Decimal]) -> f64 {
    if equity_curve.len() < 2 {
        return 0.0;
    }
    let mut returns: Vec<f64> = Vec::with_capacity(equity_curve.len() - 1);
    for w in equity_curve.windows(2) {
        if w[0] > Decimal::ZERO {
            let r = (w[1] - w[0]) / w[0];
            // Safe f64 conversion for stat computation
            if let Ok(rf) = f64::try_from(r) {
                returns.push(rf);
            }
        }
    }
    if returns.is_empty() {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean) * (r - mean)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    if std_dev < 1e-12 {
        return 0.0;
    }
    // Annualise: 525_600 minutes/year → multiply mean and std by sqrt(525_600).
    let ann_factor = (525_600.0_f64).sqrt();
    let ann_mean = mean * 525_600.0_f64;
    let ann_std = std_dev * ann_factor;
    ann_mean / ann_std
}

// ── Report-body-scenario label for the SMA/Composed strategy kind ─────────────

/// Map a `ScenarioStrategy`-like enum to a strategy-notes string fragment.
/// Used by `report::sma::write` to populate the `## Notes` section.
/// Mirrors the `strategy_notes` computation in `main.rs::write_report` @2538.
#[derive(Debug, Clone)]
pub enum SmaStrategyKind {
    /// Compiled-in SMA crossover.
    SmaCrossover { fast_len: usize, slow_len: usize },
    /// Composed TOML strategy.
    Composed { id: String },
}

impl SmaStrategyKind {
    #[must_use]
    pub fn notes_fragment(&self) -> String {
        match self {
            Self::SmaCrossover { fast_len, slow_len } => {
                format!("v0 SMA crossover: fast={fast_len}, slow={slow_len}")
            }
            Self::Composed { id } => format!("Composed strategy: {id}"),
        }
    }
}
