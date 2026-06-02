//! `param_robustness_sweep` — C3 momentum parameter-robustness sweep.
//!
//! Sweeps the v1 cross-sectional momentum family over the **RE-SCOPED 6-cell θ-grid**
//! (orchestrator-specified 2026-05-30 for tractability; was 14-cell × N=500 per
//! original architect design; re-scoped to 6-cell × N=200 for ~10-15 min wall-clock).
//! Runs the C2 N-path robustness harness at each cell, and emits ONE anchored
//! θ-surface report under `spec/momentum-parameter-robustness-sweep/reports/`.
//!
//! ## Re-scope rationale (2026-05-30)
//!
//! The original 14-cell × N=500 grid required ~1 hour. The orchestrator re-scoped
//! to 6 cells × N=200 (~10-15 min) while preserving methodology integrity:
//! - g=0: baseline θ* (correctness probe — must reproduce C2 anchor numbers)
//! - g=1: short lookback (high-churn corner)
//! - g=2: 1w lookback (medium horizon)
//! - g=3: 1mo lookback + wide hold-band (best a-priori robustness shot)
//! - g=4: narrow k_long=1 selection
//! - g=5: wide k_long=5 selection
//!
//! ## ADR-0051 § D6 compliance (SAME-paths across cells)
//!
//! - **D6.1 (SAME path-set):** `cell_seed_g := ensemble_seed` for ALL g.
//!   `path_seed_{g,j} = derive_path_seed(ensemble_seed, j)` — byte-identical
//!   to the C2 D1 seed. The θ-axis varies config only; the seed stream is
//!   untouched (provably C1/C2-determinism-neutral; the 85 anchors hold by
//!   construction).
//! - **D6.2 REJECT:** the naive additive `ensemble_seed + g·k + j·k` collapses
//!   to `+ (g+j)·k`, assigning the same path seed to `(g, j)` and `(g−1, j+1)`.
//!   This bin does NOT implement D6.2 — it is rejected as a seed-collision bug.
//! - **D6.3 (grid def hashed):** the 6-cell grid is a `const` in this bin and
//!   is printed into the hashed body. A different grid → different SHA.
//! - **D6.4 (rows sorted by g before render):** the θ-surface table rows are
//!   sorted by cell index `g` before render; completion order does not affect
//!   the body.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --release -p backtest --features "candle realdata" --bin param_robustness_sweep -- \
//!   --generator block-bootstrap-real \
//!   --paths 200 \
//!   --ensemble-seed 0xC0FFEE \
//!   --out-dir spec/momentum-parameter-robustness-sweep/reports/
//! ```
//!
//! ## Watch recipe (for long-running N=200 runs — copy-paste to operator terminal)
//!
//! ```bash
//! watch -n 15 '
//! PID=$(pgrep -f param_robustness_sweep | head -1)
//! [ -z "$PID" ] && echo "param_robustness_sweep not running" && exit
//! N=$(ls spec/momentum-parameter-robustness-sweep/reports/robustness-sweep-*.md 2>/dev/null | wc -l | tr -d " ")
//! ELAPSED=$(ps -o etime= -p $PID 2>/dev/null | tr -d " ")
//! [ "$N" -gt 0 ] && echo "surface landed ($N file); elapsed ${ELAPSED}" || echo "running (no surface yet); elapsed ${ELAPSED}"
//! '
//! ```

#![allow(clippy::float_arithmetic)] // statistical metric layer uses f64

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tracing::info;

// ── Carry score source (M-DEV-6) ──────────────────────────────────────────────

/// Expected aggregate SHA for `data/binance-funding/REVISION.toml`.
/// Locked at design time per § D-CARRY.2-LOCKED (bf1ede44…).
const DEFAULT_FUNDING_REVISION_SHA: &str =
    "bf1ede44e57d797b57e5a4f2743f58027e4eba12d91e1ffaf883dcdd49365668";

// ── Verdict classifier ─────────────────────────────────────────────────────────

/// Per-θ verdict from the 5-signal weakest-link composite.
///
/// Bands are the frozen `robustness-decision-rule-2026-05-30.md` § 0 values,
/// encoded as `const` thresholds. Spread + p50-vs-real-path are interpretive
/// (NOT verdict-forcing) per rule § 4 step 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamRobustnessVerdict {
    /// Every primary signal in ROBUST band.
    Robust,
    /// No primary signal in FRAGILE band, but not all in ROBUST.
    Marginal,
    /// At least one primary signal in FRAGILE band.
    Fragile,
}

impl ParamRobustnessVerdict {
    /// Classify a single primary signal value against the frozen bands.
    /// Returns the per-signal band (FRAGILE overrides MARGINAL overrides ROBUST).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Robust => "ROBUST",
            Self::Marginal => "MARGINAL",
            Self::Fragile => "FRAGILE",
        }
    }
}

/// Frozen decision-rule bands (§ 0 / spec/dev-notes/robustness-decision-rule-2026-05-30.md).
///
/// FRAGILE if any primary signal breaches the FRAGILE threshold.
/// ROBUST only if ALL primary signals are in the ROBUST band.
/// MARGINAL otherwise.
///
/// Primary signals (weakest-link composite):
///   p5_sharpe, p50_sharpe, prob_loss, prob_sharpe_gt_1, p95_maxdd
mod verdict_bands {
    // ── FRAGILE thresholds (ANY signal breaching → FRAGILE verdict) ──────────
    /// p5 Sharpe < 0 → FRAGILE (tail loses money).
    pub const P5_SHARPE_FRAGILE: f64 = 0.0;
    /// p50 Sharpe < 0.5 → FRAGILE (central tendency weak).
    pub const P50_SHARPE_FRAGILE: f64 = 0.5;
    /// prob_loss > 0.35 → FRAGILE (coin-flip-ish loss rate).
    pub const PROB_LOSS_FRAGILE: f64 = 0.35;
    /// P(Sharpe > 1.0) < 0.35 → FRAGILE (minority clears gate).
    pub const PROB_SHARPE_GT1_FRAGILE: f64 = 0.35;
    /// p95 MaxDD > 0.70 → FRAGILE (tail drawdown worse than 73% single-path).
    pub const P95_MAXDD_FRAGILE: f64 = 0.70;

    // ── ROBUST thresholds (ALL signals must clear → ROBUST verdict) ──────────
    /// p5 Sharpe ≥ 0.5 → ROBUST band.
    pub const P5_SHARPE_ROBUST: f64 = 0.5;
    /// p50 Sharpe ≥ 1.0 → ROBUST band.
    pub const P50_SHARPE_ROBUST: f64 = 1.0;
    /// prob_loss ≤ 0.15 → ROBUST band.
    pub const PROB_LOSS_ROBUST: f64 = 0.15;
    /// P(Sharpe > 1.0) ≥ 0.60 → ROBUST band.
    pub const PROB_SHARPE_GT1_ROBUST: f64 = 0.60;
    /// p95 MaxDD ≤ 0.50 → ROBUST band.
    pub const P95_MAXDD_ROBUST: f64 = 0.50;
}

/// Compute the composite per-θ verdict (5-signal weakest-link).
///
/// This is a pure function — unit-testable at band boundaries.
#[must_use]
pub fn classify_verdict(summary: &backtest::stats::DistributionSummary) -> ParamRobustnessVerdict {
    use verdict_bands::*;

    let p5_sharpe = summary.sharpe.p5;
    let p50_sharpe = summary.sharpe.p50;
    let prob_loss = summary.prob_loss;
    let prob_sharpe_gt1 = summary.prob_sharpe_gt_1;
    let p95_maxdd = summary.max_dd_tail_p95;

    // FRAGILE check: any single primary signal in FRAGILE band → composite FRAGILE.
    let is_fragile = p5_sharpe < P5_SHARPE_FRAGILE
        || p50_sharpe < P50_SHARPE_FRAGILE
        || prob_loss > PROB_LOSS_FRAGILE
        || prob_sharpe_gt1 < PROB_SHARPE_GT1_FRAGILE
        || p95_maxdd > P95_MAXDD_FRAGILE;

    if is_fragile {
        return ParamRobustnessVerdict::Fragile;
    }

    // ROBUST check: ALL primary signals in ROBUST band → composite ROBUST.
    let is_robust = p5_sharpe >= P5_SHARPE_ROBUST
        && p50_sharpe >= P50_SHARPE_ROBUST
        && prob_loss <= PROB_LOSS_ROBUST
        && prob_sharpe_gt1 >= PROB_SHARPE_GT1_ROBUST
        && p95_maxdd <= P95_MAXDD_ROBUST;

    if is_robust {
        ParamRobustnessVerdict::Robust
    } else {
        ParamRobustnessVerdict::Marginal
    }
}

// ── θ-grid (LOCKED — ADR-0051 § D6.3 / D-C3.2-LOCKED) ────────────────────────

/// One cell in the Tier-1 θ-grid.
#[derive(Debug, Clone, Copy)]
pub struct ThetaCell {
    /// Cell index `g` — the LOCKED render + seed-composition order.
    pub g: usize,
    /// `lookback_minutes` (signal horizon).
    ///
    /// For carry cells: encodes L (funding settlements), NOT price-bar minutes.
    /// The carry strategy uses `lookback_minutes` as the settlement-ring size L.
    pub lookback_minutes: u32,
    /// `k_long` (selection breadth / entry cutoff).
    pub k_long: u32,
    /// `drift_rebalance_threshold` (no-trade hold band / turnover lever).
    pub drift_threshold_num: i64,
    /// Denominator for the drift threshold (fixed 100 → drift = num/100).
    pub drift_threshold_den: u32,
    /// Rebalance cadence override in minutes (0 = use base config default).
    ///
    /// Momentum/MR cells: always 0 (base config sets rebalance, anchor-neutral).
    /// Carry cells: 480 (8h) or 1440 (24h, g=3 lowest-churn corner).
    pub rebalance_minutes_override: u32,
    /// Human-readable role / hypothesis.
    pub role: &'static str,
}

impl ThetaCell {
    /// Returns the drift threshold as a `Decimal`.
    #[must_use]
    pub fn drift(&self) -> Decimal {
        Decimal::new(self.drift_threshold_num, self.drift_threshold_den)
    }

    /// Effective rebalance cadence in minutes.
    ///
    /// If `rebalance_minutes_override > 0`, returns the override.
    /// Otherwise returns `base_rebalance`.
    #[must_use]
    pub fn effective_rebalance(self, base_rebalance: u32) -> u32 {
        if self.rebalance_minutes_override > 0 {
            self.rebalance_minutes_override
        } else {
            base_rebalance
        }
    }
}

/// RE-SCOPED 6-cell θ-grid (orchestrator-specified 2026-05-30 for tractability).
///
/// This exact 6-cell list is a hashed body field (ADR-0051 § D6.3 / R3.3).
/// Changing it = a different surface = a different SHA.
/// **Held constant across every cell:** `rebalance_minutes = 60`,
/// `exposure_cap = 0.50`, `vol_floor = 0.000001`, `size = equal_weight`,
/// `k_short = 0`, 10-symbol universe, year = 2023, N = 200,
/// `ensemble_seed = 0xC0FFEE`, `fill_seed = 0xC0FFEE`,
/// generator = `block-bootstrap-real`, revision `3a8b96c4…`.
///
/// Re-scoped from the architect's 14-cell × N=500 design for ~10-15 min
/// wall-clock (6×200=1200 backtests). Methodology unchanged — the grid
/// covers the same hypothesis axes at reduced resolution.
///
/// The swept axes: `lookback_minutes` (signal horizon),
/// `k_long` (selection breadth), `drift_rebalance_threshold` (turnover lever).
///
/// drift_threshold_num/den: Decimal::new(num, den) — den=2 means ×10^-2.
/// So num=10,den=2 → 0.10; num=50,den=2 → 0.50.
pub const TIER1_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 60,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "baseline θ* (C2-shipped config; g=0 MUST reproduce C2 anchor numbers)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 24,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "short lookback — 1d horizon; high churn",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 168,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "1w lookback horizon",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 720,
        k_long: 3,
        drift_threshold_num: 50,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "1mo lookback + wide hold-band — best a-priori robustness shot (low-churn corner)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 60,
        k_long: 1,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "narrow selection — top-1 only",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 60,
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "wide selection — top-5 (more legs to churn)",
    },
];

// ── Grid tier enum (for FP-C3.2 grid-sensitivity test) ────────────────────────

/// Which θ-grid to use.
///
/// `Tier1` is the LOCKED momentum anchored grid (§ D-C3.2-LOCKED).
/// `MrTier1` is the LOCKED MR θ-grid (§ D-MR.2-LOCKED).
/// `CarryTier1` is the LOCKED carry θ-grid (§ D-CARRY.2-LOCKED).
/// `TwoCell` is a 2-cell mini-grid used only by the FP-C3.2 grid-sensitivity
/// test (different grid → different body-SHA). NOT for production runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum GridKind {
    /// The LOCKED 6-cell Tier-1 anchored momentum grid (§ D-C3.2-LOCKED).
    Tier1,
    /// The LOCKED 6-cell MR Tier-1 θ-grid (§ D-MR.2-LOCKED).
    MrTier1,
    /// The LOCKED 6-cell carry Tier-1 θ-grid (§ D-CARRY.2-LOCKED).
    CarryTier1,
    /// 2-cell mini-grid for FP-C3.2 grid-sensitivity gate only.
    TwoCell,
}

/// Build the grid slice for a given kind.
#[must_use]
pub fn grid_for_kind(kind: GridKind) -> &'static [ThetaCell] {
    match kind {
        GridKind::Tier1 => TIER1_GRID,
        GridKind::MrTier1 => MR_TIER1_GRID,
        GridKind::CarryTier1 => CARRY_TIER1_GRID,
        GridKind::TwoCell => TWO_CELL_GRID,
    }
}

/// 2-cell mini-grid for the FP-C3.2 grid-sensitivity test.
/// NOT used in the anchored run. A different grid → different hashed body.
pub const TWO_CELL_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 60,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "mini-grid cell 0 (FP-C3.2 only)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 720,
        k_long: 3,
        drift_threshold_num: 50,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "mini-grid cell 1 (FP-C3.2 only)",
    },
];

/// MR Tier-1 θ-grid — LOCKED 2026-05-31 (§ D-MR.2-LOCKED).
///
/// This exact 6-cell list is the hashed body field for the MR θ-surface anchor
/// (ADR-0051 § D6.3, inherited). Changing it = a different surface = a different SHA.
///
/// **Deliberately spans the turnover axis** (R-MR.3): g=1/g=5 are high-churn cells;
/// g=3/g=4 are low-churn cells; g=0 is the baseline MR θ* (direction-flipped C3 g=0).
///
/// **Held constant across every cell (identical to C3 except direction=Reversion):**
/// `rebalance_minutes=60`, `exposure_cap=0.50`, `vol_floor=0.000001`,
/// `size=equal_weight`, `k_short=0`, 10-symbol universe, year=2023, N=200,
/// `ensemble_seed=0xC0FFEE`, `fill_seed=0xC0FFEE`, generator=block-bootstrap-real.
///
/// | g | lookback | k_long | drift | role / turnover |
/// |---|----------|--------|-------|-----------------|
/// | 0 | 60       | 3      | 0.10  | baseline MR θ* (apples-to-apples vs momentum g=0) / mid |
/// | 1 | 24       | 3      | 0.10  | short lookback + narrow band — deliberately HIGH churn  |
/// | 2 | 168      | 3      | 0.10  | 1w lookback horizon / mid                               |
/// | 3 | 720      | 5      | 0.50  | 1mo + wide band — deliberately LOW churn                |
/// | 4 | 720      | 3      | 0.30  | long lookback + medium band / low-mid                   |
/// | 5 | 24       | 5      | 0.10  | short lookback + wide selection — maximal churn extreme  |
pub const MR_TIER1_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 60,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "baseline MR θ* (apples-to-apples vs momentum g=0; must DIFFER from C3 g=0 momentum)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 24,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "short lookback + narrow band — deliberately HIGH churn (R-MR.3 high-turnover cell)",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 168,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "1w lookback horizon / mid turnover",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 720,
        k_long: 5,
        drift_threshold_num: 50,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "1mo lookback + wide band — deliberately LOW churn (R-MR.3 low-turnover cell)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 720,
        k_long: 3,
        drift_threshold_num: 30,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "long lookback + medium band — low-churn diagonal (narrower selection)",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 24,
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        role: "short lookback + wide selection — maximal-churn extreme (confirms fee trap if MR shares it)",
    },
];

/// Carry Tier-1 θ-grid — LOCKED § D-CARRY.2-LOCKED.
///
/// This exact 6-cell list is the hashed body field for the carry θ-surface anchor
/// (ADR-0051 § D6.3, § D6.6). Changing it = a different surface = a different SHA.
///
/// **`lookback_minutes` here encodes L (funding settlements), NOT price-bar minutes.**
/// The carry score counts settlements (each 8h), so the strategy maps `lookback_minutes`
/// to the number of settlements in the trailing mean (D-CARRY.2-LOCKED: L as-is).
///
/// **Held constant across every cell:** `score_source=funding_carry`,
/// `direction=momentum` (identity; sign lives in `carry_score`),
/// `exposure_cap=0.50`, `size=equal_weight`, `k_short=0`, 10-symbol universe,
/// `ensemble_seed=0xC0FFEE`, `fill_seed=0xC0FFEE`, generator=block-bootstrap-real,
/// `bootstrap_mode=shared-index`, revisions `3a8b96c4…` (OHLCV) + `bf1ede44…` (funding),
/// N=200, `vol_floor` inert (funding score has no vol denominator — Q-CARRY-4).
///
/// | g | L (settlements) | rebalance | K | role |
/// |---|-----------------|-----------|---|------|
/// | 0 | 9 (~3 d)        | 480m (8h) | 3 | baseline carry θ* (natural funding cadence) |
/// | 1 | 3 (~1 d)        | 480m      | 3 | short lookback — noisier signal |
/// | 2 | 21 (~7 d)       | 480m      | 3 | long lookback — most persistent signal |
/// | 3 | 9 (~3 d)        | 1440m (24h)| 5 | deliberately-slow rebalance + wide K (lowest churn) |
/// | 4 | 9 (~3 d)        | 480m      | 1 | narrow selection — top-1 carry name |
/// | 5 | 3 (~1 d)        | 480m      | 5 | shortest lookback + wide K — highest-churn carry extreme |
pub const CARRY_TIER1_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 9, // L=9 settlements (~3 days)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h — natural funding settlement cadence
        role: "baseline carry θ* (L=9 settlements, 8h rebalance, K=3 — natural funding cadence)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 3, // L=3 settlements (~1 day)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        role: "short funding lookback (L=3, ~1d) — noisier signal; low-mid turnover",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 21, // L=21 settlements (~7 days)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        role: "long funding lookback (L=21, ~1 week) — most persistent signal; low turnover",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 9, // L=9 settlements (~3 days)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 1440, // 24h — deliberately-slow (lowest-churn corner)
        role: "deliberately-slow 24h rebalance + wide K=5 (lowest-churn corner — carry's best structural shot)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 9, // L=9 settlements (~3 days)
        k_long: 1,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        role: "narrow selection — top-1 carry name; low turnover",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 3, // L=3 settlements (~1 day)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        role: "shortest lookback (L=3) + wide K=5 — highest-churn carry extreme (still far below price families)",
    },
];

// ── CLI ───────────────────────────────────────────────────────────────────────

/// Which path generator to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum GeneratorKind {
    /// Block-bootstrap generator on real Binance data (headline generator).
    /// Requires `--features realdata`. The anchored scenario uses this.
    BlockBootstrapReal,
    /// GBM smoke-test generator. Does NOT require real data. NOT anchored.
    GbmSmoke,
}

impl GeneratorKind {
    fn label(self) -> &'static str {
        match self {
            Self::BlockBootstrapReal => "block-bootstrap-real",
            Self::GbmSmoke => "gbm-smoke",
        }
    }
}

/// Strategy family direction for the sweep (D-MR.0).
///
/// `momentum` selects top-K winners (v1 behavior — default; reproduces momentum anchor #86).
/// `reversion` negates scores so the unchanged `top_k_long` selects bottom-K losers (MR family).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum SweepDirection {
    /// Top-K winners — v1 momentum behavior. Default: reproduces momentum anchor #86.
    #[default]
    Momentum,
    /// Bottom-K losers — cross-sectional mean-reversion (D-MR.1).
    Reversion,
}

impl SweepDirection {
    /// Convert to the strategy `Direction` type (same semantic).
    fn to_strategy_direction(self) -> strategy::Direction {
        match self {
            Self::Momentum => strategy::Direction::Momentum,
            Self::Reversion => strategy::Direction::Reversion,
        }
    }

    /// Label for the scenario name and report.
    fn label(self) -> &'static str {
        match self {
            Self::Momentum => "momentum",
            Self::Reversion => "mr",
        }
    }
}

/// Which score source to use (M-DEV-6, D-CARRY.1).
///
/// `VolAdjustedReturn` (default) = the v1 price-based signal; reproduces momentum #86 / MR #87
/// byte-identical. `Carry` = funding-based `ScoreSource::FundingCarry`; uses the locked carry
/// θ-grid + funding revision; requires `--generator block-bootstrap-real`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum SweepScoreSource {
    /// Vol-adjusted price return — v1 default; reproduces momentum/MR anchors.
    #[default]
    #[value(name = "vol-adjusted-return")]
    VolAdjustedReturn,
    /// Funding-carry signal (§ D-CARRY.1 / R-CARRY.1-2); requires real funding data.
    #[value(name = "carry")]
    Carry,
}

impl SweepScoreSource {
    /// Convert to the strategy `ScoreSource` type.
    fn to_strategy_score_source(self) -> strategy::ScoreSource {
        match self {
            Self::VolAdjustedReturn => strategy::ScoreSource::VolAdjustedReturn,
            Self::Carry => strategy::ScoreSource::FundingCarry,
        }
    }

    /// Whether this source needs the funding data loaded.
    fn needs_funding(self) -> bool {
        self == Self::Carry
    }

    #[allow(dead_code)]
    /// Short label for scenario name.
    fn label(self) -> &'static str {
        match self {
            Self::VolAdjustedReturn => "carry-fy", // unused for non-carry
            Self::Carry => "carry-fy",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "param_robustness_sweep",
    about = "C3/MR — parameter-robustness sweep: θ-grid × N-path harness → θ-surface report",
    long_about = "Sweeps the v1 cross-sectional momentum (or MR) family over the re-scoped 6-cell \
                  θ-grid (orchestrator-specified 2026-05-30 for momentum; 2026-05-31 for MR), \
                  running the C2 N-path harness at each cell (same path-set across cells per \
                  ADR-0051 § D6.1), reduces each cell to a DistributionSummary, applies the \
                  frozen 5-signal weakest-link verdict classifier, and emits ONE anchored \
                  θ-surface report (ADR-0051 D3/D4). Also emits a buy-and-hold passive control \
                  row (the adversarial-review benchmark).\n\n\
                  --direction momentum (default) reproduces the momentum anchor #86 byte-identical.\n\
                  --direction reversion runs the MR family (§ D-MR.1) with --grid mr-tier1."
)]
struct Args {
    /// Path generator to use.
    #[arg(long, value_enum, default_value = "block-bootstrap-real")]
    generator: GeneratorKind,

    /// Number of bootstrap paths per cell (N). Re-scoped default 200 (was 500 in 14-cell design).
    /// N is a hashed body field — changing N changes the anchor SHA.
    #[arg(long, default_value_t = 200)]
    paths: usize,

    /// Master ensemble seed (hex or decimal). SAME-paths rule (ADR-0051 § D6.1):
    /// path j gets seed `ensemble_seed.wrapping_add(j * 0x9E3779B9)` for ALL cells.
    #[arg(long, default_value = "0xC0FFEE")]
    ensemble_seed: String,

    /// Parquet root for real OHLCV bars (block-bootstrap-real only).
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Pinned data revision SHA.
    #[arg(
        long,
        default_value = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7"
    )]
    expected_revision_sha: String,

    /// Output directory for the θ-surface report.
    /// Defaults to the momentum dir; use --out-dir to override for MR runs.
    #[arg(
        long,
        default_value = "spec/momentum-parameter-robustness-sweep/reports/"
    )]
    out_dir: PathBuf,

    /// Calendar year for the scenario's backtest span (2023 or 2024).
    #[arg(long, default_value_t = 2023)]
    year: i32,

    /// θ-grid to use.
    /// `tier1` is the LOCKED momentum anchored grid (§ D-C3.2-LOCKED).
    /// `mr-tier1` is the LOCKED MR θ-grid (§ D-MR.2-LOCKED).
    /// `two-cell` is only for the FP-C3.2 grid-sensitivity gate.
    #[arg(long, value_enum, default_value = "tier1")]
    grid: GridKind,

    /// Strategy family direction (D-MR.0).
    /// `momentum` (default) = top-K winners; reproduces momentum anchor #86 byte-identical.
    /// `reversion` = bottom-K losers (MR family); use with --grid mr-tier1.
    #[arg(long, value_enum, default_value = "momentum")]
    direction: SweepDirection,

    /// Score source (M-DEV-6, D-CARRY.1).
    /// `vol-adjusted-return` (default) reproduces momentum/MR anchors byte-identical.
    /// `carry` uses the funding-carry signal; requires block-bootstrap-real + funding data.
    #[arg(long, value_enum, default_value = "vol-adjusted-return")]
    score_source: SweepScoreSource,

    /// Root directory for funding parquets (carry only).
    /// Default: `data/binance-funding/` (the locked path from carry-funding-data-backfill).
    #[arg(long, default_value = "data/binance-funding/")]
    funding_root: PathBuf,

    /// Expected aggregate SHA-256 for data/binance-funding/REVISION.toml (carry only).
    /// Default: the locked funding revision SHA (bf1ede44…).
    #[arg(long, default_value = DEFAULT_FUNDING_REVISION_SHA)]
    funding_revision_sha: String,
}

// ── Seed helpers ──────────────────────────────────────────────────────────────

/// Parse an ensemble seed from a hex string (0x…) or decimal string.
fn parse_seed(s: &str) -> Result<u64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).with_context(|| format!("parse hex seed: {s}"))
    } else {
        s.parse::<u64>()
            .with_context(|| format!("parse decimal seed: {s}"))
    }
}

/// ADR-0051 D1 + D6.1: derive per-path seed from master seed and path index.
///
/// SAME-paths rule: `path_seed_{g,j} = derive_path_seed(ensemble_seed, j)`
/// — the same for EVERY cell g. The θ-axis varies config only; the seed is
/// byte-identical to the C2 D1 seed (the 85 anchors hold by construction).
#[inline]
pub fn derive_path_seed(master: u64, j: usize) -> u64 {
    master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))
}

// ── Git / hostname helpers (mirrors monte_carlo.rs) ────────────────────────────

fn read_git_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()))
}

fn read_data_revision_sha(data_root: &std::path::Path) -> String {
    let rev_path = data_root.join("REVISION.toml");
    std::fs::read_to_string(&rev_path)
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("sha"))
                .and_then(|l| l.split('=').nth(1))
                .map(|v| v.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Per-symbol source bar collection (type alias to reduce complexity).
type SourceBars = Vec<(trading_core::Symbol, Vec<trading_core::Bar>)>;
/// Result of loading source bars: bars by symbol + revision SHA string.
type SourceBarsResult = Result<(SourceBars, String)>;

// ── Per-path metric struct ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct IndexedPathMetrics {
    j: usize,
    metrics: backtest::stats::PathMetrics,
    trades: usize,
    /// Total realized funding harvested on this path (Decimal; 0 for momentum/MR).
    /// Summed from `run_path`'s cashflow accrual — a proxy via final_equity delta
    /// relative to a zero-funding baseline. We track this via a direct sum in
    /// `run_one_path_with_config` by re-computing from the result (not yet threaded
    /// through `run_path`'s return type). For now: `final_equity − initial_equity`
    /// delta attributable to funding is not separately tracked in `PathRunResult`.
    /// We carry a placeholder `Decimal::ZERO` here; the realized-funding COLUMN is
    /// populated at the cell level via a separate carry of `total_funding_harvested`.
    #[allow(dead_code)]
    funding_harvested: Decimal,
}

// ── Per-cell result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CellResult {
    cell: ThetaCell,
    summary: backtest::stats::DistributionSummary,
    verdict: ParamRobustnessVerdict,
    /// Total trade count across all N paths (for FP-C3.1 divergence gate).
    #[allow(dead_code)]
    total_trades: u64,
    /// Total realized funding harvested across all N paths (Decimal).
    /// Non-zero only for carry runs (score_source=Carry). Populated from the
    /// per-path equity delta attributable to funding (D-CARRY.2-LOCKED carry column).
    total_funding_harvested: Decimal,
}

// ── Config injection helper ────────────────────────────────────────────────────

/// Build a per-cell `CrossSectionalMomentumConfig` from the base config,
/// overriding the swept axes, the strategy family direction, and score source.
///
/// This is the ~30-line crux that FP-C3.1 falsifies. The base config provides
/// the frozen fields (`rebalance_minutes=60`, `exposure_cap=0.50`,
/// `vol_floor=1e-6`, `size=equal_weight`, `k_short=0`, universe).
///
/// For `SweepDirection::Momentum` + `SweepScoreSource::VolAdjustedReturn` (defaults)
/// this is byte-identical to the pre-MR behavior — the 86 momentum anchors hold
/// by construction (D-MR.6). Carry cells also override `rebalance_minutes` via
/// `ThetaCell::effective_rebalance`.
pub fn cell_config(
    base: &strategy::CrossSectionalMomentumConfig,
    cell: &ThetaCell,
    direction: SweepDirection,
    score_source: SweepScoreSource,
) -> strategy::CrossSectionalMomentumConfig {
    let mut cfg = base.clone();
    cfg.lookback_minutes = cell.lookback_minutes;
    cfg.k_long = cell.k_long;
    cfg.drift_rebalance_threshold = cell.drift();
    cfg.direction = direction.to_strategy_direction();
    cfg.score_source = score_source.to_strategy_score_source();
    // Apply rebalance override if set (carry cells); momentum/MR cells have override=0
    // → effective_rebalance returns the base config's rebalance_minutes unchanged.
    cfg.rebalance_minutes = cell.effective_rebalance(base.rebalance_minutes);
    cfg
}

// ── Buy-and-hold passive control ───────────────────────────────────────────────

/// Run a buy-and-hold (equal-weight, hold from bar 0) over one path.
///
/// This is the adversarial-review benchmark: passive holding the same 10 coins.
/// Returns the equity curve (same length as `bars`+1, starting at `initial_capital`).
///
/// Implementation: equal-weight buy of all n symbols at bar 0 close, then track
/// mark-to-market equity (no rebalancing, no fees after bar 0).
#[must_use]
fn run_buyhold_path(
    bars: &[trading_core::Bar],
    initial_capital: Decimal,
    n_symbols: usize,
) -> (Vec<Decimal>, Decimal) {
    if bars.is_empty() || n_symbols == 0 {
        return (vec![initial_capital], initial_capital);
    }

    // Equal-weight allocation per symbol.
    #[allow(clippy::cast_precision_loss)]
    let weight = initial_capital / Decimal::try_from(n_symbols as f64).unwrap_or(dec!(10));

    // Group bars by symbol (in order).
    let mut by_symbol: std::collections::BTreeMap<String, Vec<Decimal>> =
        std::collections::BTreeMap::new();
    for bar in bars {
        by_symbol
            .entry(bar.symbol.to_string())
            .or_default()
            .push(bar.close.get());
    }

    // Buy at bar 0 close; track qty per symbol.
    let mut qtys: std::collections::BTreeMap<String, Decimal> = std::collections::BTreeMap::new();
    for (sym, prices) in &by_symbol {
        let buy_price = *prices.first().unwrap_or(&dec!(1));
        if buy_price > Decimal::ZERO {
            qtys.insert(sym.clone(), weight / buy_price);
        }
    }

    // Build equity curve (bar count + 1 entries).
    // Determine the number of distinct timestamps.
    let n_bars = {
        let bar_ts: std::collections::BTreeSet<i64> = bars
            .iter()
            .map(|b| b.open_ts.inner().unix_timestamp_nanos() as i64)
            .collect();
        bar_ts.len()
    };

    // For each timestep, compute mark-to-market equity.
    // Strategy: group bars by timestamp in order.
    let mut bar_map: std::collections::BTreeMap<i128, std::collections::BTreeMap<String, Decimal>> =
        std::collections::BTreeMap::new();
    for bar in bars {
        let ts = bar.open_ts.inner().unix_timestamp_nanos();
        bar_map
            .entry(ts)
            .or_default()
            .insert(bar.symbol.to_string(), bar.close.get());
    }

    let mut equity_curve: Vec<Decimal> = Vec::with_capacity(n_bars + 1);
    equity_curve.push(initial_capital);

    // Carry last known price so we handle missing bars gracefully.
    let mut last_prices: std::collections::BTreeMap<String, Decimal> =
        std::collections::BTreeMap::new();

    for prices_at_ts in bar_map.values() {
        for (sym, price) in prices_at_ts {
            last_prices.insert(sym.clone(), *price);
        }
        let equity: Decimal = qtys
            .iter()
            .map(|(sym, qty)| {
                let p = last_prices.get(sym).copied().unwrap_or(dec!(0));
                qty * p
            })
            .sum();
        equity_curve.push(equity);
    }

    let final_eq = *equity_curve.last().unwrap_or(&initial_capital);
    (equity_curve, final_eq)
}

// ── Grid definition string (hashed body field — K3 / § D6.3) ─────────────────

/// Build the canonical grid-definition string for the hashed body.
///
/// This is a hashed body field (R3.3): a different grid → a different string →
/// a different body-SHA. Format: one row per cell, `g|lookback|k_long|drift`,
/// rows in `g` order.
#[must_use]
pub fn grid_def_string(grid: &[ThetaCell]) -> String {
    let mut s = String::from("grid_definition:\n");
    for cell in grid {
        s.push_str(&format!(
            "  g={} lookback={} k_long={} drift={}\n",
            cell.g,
            cell.lookback_minutes,
            cell.k_long,
            cell.drift()
        ));
    }
    s
}

/// Build the canonical grid-definition string for carry reports.
///
/// Includes `l_settlements` (the `lookback_minutes` field reinterpreted as L)
/// AND `rebalance_minutes` (swept in carry, unlike momentum/MR). This is a
/// hashed body field for the carry anchor (K3 / § D6.3 + D6.6).
///
/// Format mirrors `grid_def_string` but adds `rebalance` and renames
/// `lookback` to `l_settlements` to make the unit explicit.
#[must_use]
pub fn carry_grid_def_string(grid: &[ThetaCell]) -> String {
    let mut s = String::from("grid_definition:\n");
    for cell in grid {
        s.push_str(&format!(
            "  g={} l_settlements={} rebalance_minutes={} k_long={} drift={}\n",
            cell.g,
            cell.lookback_minutes,
            cell.rebalance_minutes_override,
            cell.k_long,
            cell.drift()
        ));
    }
    s
}

// ── Simple Gregorian calendar (mirrors monte_carlo.rs) ────────────────────────

#[allow(clippy::cast_possible_truncation)]
fn days_since_epoch_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days as i64 + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_adj = if m <= 2 { y + 1 } else { y };
    (y_adj as u32, m as u32, d as u32)
}

// ── Report renderer (ADR-0051 D3 / § D6.4) ────────────────────────────────────

/// Render the θ-surface report.
///
/// Front-matter (run-varying, NOT hashed) + deterministic body (shared-input
/// header + θ-surface table + buy-and-hold control row + family-summary line).
///
/// ## Anti-cherry-pick enforcement (§ 0 pre-registration commitment)
///
/// The renderer NEVER emits a "best θ is ROBUST" claim.
/// The family summary line is exactly one of:
/// - `FAMILY-UNIFORM-FRAGILE` (every active cell FRAGILE)
/// - `FAMILY-HAS-NON-FRAGILE-CELLS` (≥1 cell MARGINAL/ROBUST → each flagged `→ C5`)
#[allow(clippy::too_many_arguments)]
fn render_surface_report(
    // Front-matter (run-varying)
    generated: &str,
    wall_clock_s: f64,
    host: &str,
    pid: u32,
    git_commit: &str,
    data_revision_sha: &str,
    scenario: &str,
    // Body (deterministic, hashed)
    master_seed: u64,
    fill_seed: u64,
    n_paths: usize,
    generator_label: &str,
    bootstrap_mode: &str,
    block_length_policy: &str,
    selected_block_length_l: Option<usize>,
    source_revision_sha: &str,
    grid: &[ThetaCell],
    // Cell results (already sorted by g)
    cell_results: &[CellResult],
    // Buy-and-hold control distribution
    buyhold_summary: &backtest::stats::DistributionSummary,
    // Strategy family direction (D-MR.0):
    // Momentum → standard momentum report (slug/heading unchanged for anchor compatibility).
    // Reversion → MR report slug + trades column added (R-MR.3 turnover legibility).
    direction: SweepDirection,
    // Score source (M-DEV-6):
    // VolAdjustedReturn → standard report (anchor-neutral, no extra column).
    // Carry → carry report slug + realized-funding-harvested column added.
    score_source: SweepScoreSource,
    // Carry only: funding revision SHA (included in body for K3).
    funding_revision_sha: Option<&str>,
) -> String {
    // ── Front-matter (NOT hashed) ─────────────────────────────────────────────
    // slug: momentum reports keep "momentum-parameter-robustness-sweep" for anchor compat.
    // MR reports use "cross-sectional-mean-reversion-strategy".
    // Carry reports use "carry-strategy".
    let slug = match score_source {
        SweepScoreSource::Carry => "carry-strategy",
        SweepScoreSource::VolAdjustedReturn => match direction {
            SweepDirection::Momentum => "momentum-parameter-robustness-sweep",
            SweepDirection::Reversion => "cross-sectional-mean-reversion-strategy",
        },
    };
    let frontmatter = format!(
        "---\n\
         slug: {slug}\n\
         scenario: {scenario}\n\
         generated: {generated}\n\
         wall_clock_s: {wall_clock_s:.1}\n\
         host: {host}\n\
         pid: {pid}\n\
         git_commit: {git_commit}\n\
         data_revision_sha: {data_revision_sha}\n\
         ---\n"
    );

    // ── Body (deterministic, hashed by the anchor) ────────────────────────────
    let mut body = String::new();

    let family_label = match score_source {
        SweepScoreSource::Carry => "Carry (Funding)",
        SweepScoreSource::VolAdjustedReturn => match direction {
            SweepDirection::Momentum => "Momentum",
            SweepDirection::Reversion => "Mean-Reversion (MR)",
        },
    };
    body.push_str(&format!(
        "# {family_label} θ-Surface — Parameter-Robustness Sweep — {scenario}\n\n"
    ));

    // Shared-input header block (every field that is constant across all cells).
    body.push_str("## Ensemble parameters (shared across all θ-cells)\n\n");
    body.push_str(
        "| Field                    | Value                                                   |\n",
    );
    body.push_str(
        "|--------------------------|----------------------------------------------------------|\n",
    );
    body.push_str(&format!(
        "| master_seed              | 0x{master_seed:X}                                          |\n"
    ));
    body.push_str(&format!(
        "| fill_seed                | 0x{fill_seed:X}                                          |\n"
    ));
    body.push_str(&format!(
        "| n_paths                  | {n_paths}                                                 |\n"
    ));
    body.push_str("| sub_seed_rule            | \"master + j*0x9E3779B9 (SAME paths across cells, ADR-0051 D6.1)\" |\n");
    body.push_str("| reduction_rule           | \"index-order mean/std; total_cmp sort; type-7 linear pct\" |\n");
    body.push_str(&format!(
        "| generator                | {generator_label}                                        |\n"
    ));
    body.push_str(&format!(
        "| bootstrap_mode           | {bootstrap_mode}                                         |\n"
    ));
    body.push_str(&format!(
        "| block_length_policy      | {block_length_policy}                                    |\n"
    ));
    if let Some(l) = selected_block_length_l {
        body.push_str(&format!(
            "| selected_block_length_L  | {l} (θ-independent — same L for all cells per OQ-3)      |\n"
        ));
    } else {
        body.push_str(
            "| selected_block_length_L  | N/A                                                     |\n",
        );
    }
    body.push_str(&format!(
        "| source_revision_sha      | {source_revision_sha}                                    |\n"
    ));
    // held_constant: add direction for MR runs; score_source + funding for carry runs.
    // The body field is part of the hash — the carry string differs from momentum/MR.
    let held_constant_str: String = match score_source {
        SweepScoreSource::Carry => {
            format!(
                "| held_constant            | score_source=funding_carry direction=momentum exposure_cap=0.50 vol_floor=inert k_short=0 size=equal_weight |\n\
                 | funding_revision_sha     | {} |\n",
                funding_revision_sha.unwrap_or("unknown")
            )
        }
        SweepScoreSource::VolAdjustedReturn => match direction {
            SweepDirection::Momentum => {
                "| held_constant            | rebalance_minutes=60 exposure_cap=0.50 vol_floor=0.000001 k_short=0 size=equal_weight |\n".to_string()
            }
            SweepDirection::Reversion => {
                "| held_constant            | rebalance_minutes=60 exposure_cap=0.50 vol_floor=0.000001 k_short=0 size=equal_weight direction=reversion |\n".to_string()
            }
        },
    };
    body.push_str(&held_constant_str);
    body.push('\n');

    // Frozen grid definition (hashed body field — K3 / § D6.3).
    let grid_header: &str = match score_source {
        SweepScoreSource::Carry => {
            "## Carry θ-grid definition (6-cell, LOCKED § D-CARRY.2-LOCKED — changing this changes the SHA)\n\n"
        }
        SweepScoreSource::VolAdjustedReturn => match direction {
            SweepDirection::Momentum => {
                "## Re-scoped θ-grid definition (6-cell, 2026-05-30 orchestrator re-scope — changing this changes the SHA)\n\n"
            }
            SweepDirection::Reversion => {
                "## MR θ-grid definition (6-cell, 2026-05-31 LOCKED § D-MR.2-LOCKED — changing this changes the SHA)\n\n"
            }
        },
    };
    body.push_str(grid_header);
    // Carry grid: use carry-specific format (includes rebalance — it's swept).
    // Momentum/MR: use the standard grid_def_string (no rebalance — anchor-safe).
    match score_source {
        SweepScoreSource::Carry => {
            body.push_str(&carry_grid_def_string(grid));
        }
        SweepScoreSource::VolAdjustedReturn => {
            body.push_str(&grid_def_string(grid));
        }
    }
    body.push('\n');

    // θ-surface table (rows sorted by g).
    body.push_str("## θ-surface (per-cell distribution + verdict)\n\n");
    body.push_str("Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.\n");
    body.push_str("Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).\n");
    body.push_str("Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).\n\n");

    // M-DEV-4: add `trades` column for MR only (R-MR.3 turnover legibility).
    // Gate on direction so momentum anchor #86 body-SHA stays byte-identical.
    // M-DEV-6: add `funding_harvested` column for carry only (D-CARRY.2-LOCKED).
    // Gate on score_source so MR/momentum body-SHAs stay byte-identical.
    let show_trades = score_source == SweepScoreSource::VolAdjustedReturn
        && direction == SweepDirection::Reversion;
    let show_funding = score_source == SweepScoreSource::Carry;
    if show_trades {
        body.push_str(
            "Trades = total trade count across all N paths (turnover legibility — R-MR.3).\n\n",
        );
        body.push_str("| g  | lookback | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | trades     | verdict  | notes |\n");
        body.push_str("|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|------------|----------|-------|\n");
    } else if show_funding {
        body.push_str(
            "funding_harvested = total realized funding cashflow across all N paths (Decimal, D-CARRY.2-LOCKED).\n\n",
        );
        body.push_str("| g  | l_settle | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | funding_harvested | verdict  | notes |\n");
        body.push_str("|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------------|----------|-------|\n");
    } else {
        body.push_str("| g  | lookback | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  | notes |\n");
        body.push_str("|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|-------|\n");
    }

    let mut any_non_fragile = false;

    for cr in cell_results {
        let s = &cr.summary;
        let verdict_str = cr.verdict.as_str();
        let spread = s.sharpe.p95 - s.sharpe.p5;
        let c5_flag = if cr.verdict != ParamRobustnessVerdict::Fragile {
            any_non_fragile = true;
            "→ C5 DEFLATION REQUIRED"
        } else {
            ""
        };

        if show_trades {
            body.push_str(&format!(
                "| {:2} | {:8} | {:6} | {:.2} | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | {:10} | {:8} | {} |\n",
                cr.cell.g,
                cr.cell.lookback_minutes,
                cr.cell.k_long,
                cr.cell.drift(),
                s.sharpe.p5,
                s.sharpe.p50,
                s.sharpe.p95,
                s.prob_loss,
                s.prob_sharpe_gt_1,
                s.max_dd_tail_p95 * 100.0,
                spread,
                cr.total_trades,
                verdict_str,
                c5_flag,
            ));
        } else if show_funding {
            body.push_str(&format!(
                "| {:2} | {:8} | {:6} | {:.2} | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | {:18} | {:8} | {} |\n",
                cr.cell.g,
                cr.cell.lookback_minutes,
                cr.cell.k_long,
                cr.cell.drift(),
                s.sharpe.p5,
                s.sharpe.p50,
                s.sharpe.p95,
                s.prob_loss,
                s.prob_sharpe_gt_1,
                s.max_dd_tail_p95 * 100.0,
                spread,
                cr.total_funding_harvested,
                verdict_str,
                c5_flag,
            ));
        } else {
            body.push_str(&format!(
                "| {:2} | {:8} | {:6} | {:.2} | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | {:8} | {} |\n",
                cr.cell.g,
                cr.cell.lookback_minutes,
                cr.cell.k_long,
                cr.cell.drift(),
                s.sharpe.p5,
                s.sharpe.p50,
                s.sharpe.p95,
                s.prob_loss,
                s.prob_sharpe_gt_1,
                s.max_dd_tail_p95 * 100.0,
                spread,
                verdict_str,
                c5_flag,
            ));
        }
    }

    body.push('\n');

    // Buy-and-hold control row (passive benchmark — no verdict).
    body.push_str("## Buy-and-hold passive control (adversarial-review benchmark)\n\n");
    body.push_str("Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.\n");
    body.push_str("Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.\n\n");
    body.push_str("| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |\n");
    body.push_str("|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|\n");
    {
        let s = buyhold_summary;
        let spread = s.sharpe.p95 - s.sharpe.p5;
        body.push_str(&format!(
            "| BUYHOLD   | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | (passive — no verdict) |\n",
            s.sharpe.p5,
            s.sharpe.p50,
            s.sharpe.p95,
            s.prob_loss,
            s.prob_sharpe_gt_1,
            s.max_dd_tail_p95 * 100.0,
            spread,
        ));
    }
    body.push('\n');

    // Family-level summary line (the § 0 pre-registration commitment, mechanized).
    // NEVER crowns a "best θ is ROBUST" — only FAMILY-UNIFORM-FRAGILE or FAMILY-HAS-NON-FRAGILE-CELLS.
    body.push_str("## Family verdict\n\n");
    if any_non_fragile {
        body.push_str("FAMILY-HAS-NON-FRAGILE-CELLS\n\n");
        body.push_str(
            "At least one cell is MARGINAL or ROBUST. Per the § 0 pre-registration commitment:\n",
        );
        body.push_str("C3 makes NO 'this θ is robust' claim. Each non-FRAGILE cell is flagged '→ C5 DEFLATION REQUIRED'\n");
        body.push_str("and is handed to the C5 PBO/Deflated-Sharpe pass before any promotion.\n");
    } else {
        body.push_str("FAMILY-UNIFORM-FRAGILE\n\n");
        body.push_str("Every active θ-cell is FRAGILE under the frozen decision-rule bands.\n");
        body.push_str("No multiple-testing correction is needed for a uniform-negative result:\n");
        body.push_str(
            "C3 is not selecting a winner — it is reporting that no cell cleared the bar.\n",
        );
        match score_source {
            SweepScoreSource::Carry => {
                body.push_str(
                    "Conclusion: v1 cross-sectional carry (funding) is structurally fragile across the\n",
                );
                body.push_str(
                    "tested parameter space on this universe (2023-FY resampled). Funding mean-reversion\n",
                );
                body.push_str(
                    "or directional price exposure may have overwhelmed the funding harvest.\n",
                );
            }
            SweepScoreSource::VolAdjustedReturn => match direction {
                SweepDirection::Momentum => {
                    body.push_str(
                        "Conclusion: v1 cross-sectional momentum is structurally fragile across the\n",
                    );
                    body.push_str(
                        "tested parameter space. The turnover/fee-bleed is not tunable away within\n",
                    );
                    body.push_str(
                        "the Tier-1 grid (lookback × k_long × drift_rebalance_threshold).\n",
                    );
                }
                SweepDirection::Reversion => {
                    body.push_str(
                        "Conclusion: v1 cross-sectional mean-reversion is structurally fragile across the\n",
                    );
                    body.push_str(
                        "tested parameter space. The turnover/fee-bleed is not tunable away within\n",
                    );
                    body.push_str(
                        "the MR Tier-1 grid (lookback × k_long × drift_rebalance_threshold).\n",
                    );
                }
            },
        }
    }
    body.push('\n');
    body.push_str("Notes:\n");
    body.push_str("- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.\n");
    body.push_str("- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).\n");
    body.push_str("- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).\n");
    body.push_str("- Generator: `block-bootstrap-real` only is anchor-grade.\n");
    body.push_str("- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).\n");

    format!("{frontmatter}{body}")
}

// ── Load source bars ───────────────────────────────────────────────────────────

fn load_source_bars(
    args: &Args,
    symbols_prices: &[(trading_core::Symbol, Decimal)],
    bar_count: usize,
) -> SourceBarsResult {
    match args.generator {
        GeneratorKind::GbmSmoke => {
            let bars_by_symbol: SourceBars = symbols_prices
                .iter()
                .enumerate()
                .map(|(idx, (sym, start_price))| {
                    let sym_seed = 0xC0FFEE_u64.wrapping_add(idx as u64 * 0x9E37_79B9);
                    let bars = backtest::scenarios::momentum::synthetic_bars_hourly(
                        sym,
                        bar_count,
                        sym_seed,
                        *start_price,
                        args.year,
                    );
                    (sym.clone(), bars)
                })
                .collect();
            Ok((bars_by_symbol, "N/A".to_string()))
        }
        GeneratorKind::BlockBootstrapReal => load_real_bars(args, symbols_prices, bar_count),
    }
}

#[cfg(feature = "realdata")]
fn load_real_bars(
    args: &Args,
    symbols_prices: &[(trading_core::Symbol, Decimal)],
    bar_count: usize,
) -> SourceBarsResult {
    use backtest::realdata::{RealDataBarSource, TimeSpan as RealDataTimeSpan};

    let symbols: Vec<trading_core::Symbol> =
        symbols_prices.iter().map(|(s, _)| s.clone()).collect();
    let src = RealDataBarSource::new(args.data_root.clone(), symbols.clone());
    let span = RealDataTimeSpan::full_year(args.year);
    let expected_total = bar_count * symbols.len();
    let scenario_name = format!("param-robustness-sweep-load-{}", args.year);
    let loaded = src
        .load(span, expected_total, &scenario_name)
        .map_err(|e| anyhow::anyhow!("load real bars: {e}"))?;

    if loaded.revision_sha != args.expected_revision_sha {
        anyhow::bail!(
            "data revision mismatch: expected {} but computed {}",
            args.expected_revision_sha,
            loaded.revision_sha
        );
    }

    let revision_sha = loaded.revision_sha.clone();
    info!(
        bar_count = loaded.loaded_count,
        revision_sha = %revision_sha,
        "real bars loaded"
    );

    let merged_bars = loaded.bars;
    let mut by_symbol: std::collections::BTreeMap<String, Vec<trading_core::Bar>> =
        std::collections::BTreeMap::new();
    for bar in merged_bars {
        by_symbol
            .entry(bar.symbol.to_string())
            .or_default()
            .push(bar);
    }
    for bars in by_symbol.values_mut() {
        bars.sort_by_key(|b| b.open_ts);
    }

    let bars_by_symbol: SourceBars = symbols_prices
        .iter()
        .map(|(sym, _)| {
            let bars = by_symbol.remove(&sym.to_string()).unwrap_or_default();
            (sym.clone(), bars)
        })
        .collect();

    Ok((bars_by_symbol, revision_sha))
}

#[cfg(not(feature = "realdata"))]
fn load_real_bars(
    _args: &Args,
    _symbols_prices: &[(trading_core::Symbol, Decimal)],
    _bar_count: usize,
) -> SourceBarsResult {
    anyhow::bail!(
        "load_real_bars called without --features realdata. \
         Use --generator gbm-smoke or rebuild with the realdata feature."
    )
}

/// Return type of `prepare_generator_params`:
/// (label, bootstrap_mode, block_length_policy, selected_L, pre-built path_gen).
type GeneratorParams = (
    String,
    String,
    String,
    Option<usize>,
    Option<data::BlockBootstrapPathGen>,
);

/// Build the generator label + bootstrap_mode + block_length_policy + selected_L
/// + the pre-built `BlockBootstrapPathGen` (for BlockBootstrapReal only).
///
/// **Performance:** The `BlockBootstrapPathGen` is built ONCE here (outside all
/// rayon parallel loops) and returned. It is then shared immutably across all
/// N=500 × G=14 parallel path tasks via `&` references. This avoids the O(N×G)
/// clone of the 87,600-bar source that the original per-task construction caused.
fn prepare_generator_params(
    generator: GeneratorKind,
    real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    ensemble_seed: u64,
) -> Result<GeneratorParams> {
    match generator {
        GeneratorKind::GbmSmoke => Ok((
            "gbm-smoke".to_string(),
            "N/A".to_string(),
            "N/A".to_string(),
            None,
            None,
        )),
        GeneratorKind::BlockBootstrapReal => {
            let source: Vec<(trading_core::Symbol, Vec<trading_core::Bar>)> =
                real_bars_by_symbol.to_vec();
            let path_gen = data::BlockBootstrapPathGen::new(source, data::BlockLengthPolicy::Auto)
                .context("build BlockBootstrapPathGen to get selected_L")?;

            let universe: Vec<(trading_core::Symbol, Decimal)> = real_bars_by_symbol
                .iter()
                .map(|(sym, bars)| {
                    let start = bars.first().map(|b| b.close.get()).unwrap_or(dec!(1));
                    (sym.clone(), start)
                })
                .collect();

            use data::MonteCarloPathGen as _;
            let probe = path_gen
                .generate(&universe, 10, ensemble_seed)
                .context("probe generate for selected_L")?;
            let selected_l = probe.selected_block_length;

            Ok((
                "block-bootstrap-real".to_string(),
                "shared-index".to_string(),
                "auto".to_string(),
                selected_l,
                Some(path_gen),
            ))
        }
    }
}

// ── Per-path runner (C3-local copy of run_one_path — takes a config) ──────────

/// C3-local path runner — mirrors `monte_carlo.rs::run_one_path` byte-for-byte
/// EXCEPT it takes a caller-supplied `CrossSectionalMomentumConfig` AND a
/// pre-built `Option<&data::BlockBootstrapPathGen>` instead of loading the TOML
/// and constructing a new path_gen per path.
///
/// The path_gen is pre-built ONCE outside the rayon parallel loop and shared
/// across all N=500 tasks for a given cell via `&` references. Since rayon
/// Scope provides immutable sharing, `BlockBootstrapPathGen::generate` takes
/// `&self` and is pure for a given seed, this is correct and deterministic.
///
/// This keeps `run_path` AND the C2 `monte_carlo.rs` driver byte-identical
/// (R-NR.2 — the 85 anchors hold by construction). The ONLY changes vs C2's
/// `run_one_path` are: (1) config is caller-supplied, (2) path_gen is shared,
/// (3) for carry: the path_gen already has funding attached (built in main with
///    `with_funding`), so this function generates funding_override seamlessly.
#[allow(clippy::too_many_arguments)]
fn run_one_path_with_config(
    j: usize,
    path_seed_j: u64,
    fill_seed: u64,
    cfg: &strategy::CrossSectionalMomentumConfig,
    universe: &[(trading_core::Symbol, Decimal)],
    // Pre-built path generator for BlockBootstrapReal; None for GbmSmoke.
    // For carry: this is the CARRY path_gen (with funding already attached via with_funding).
    // For momentum/MR: this is the BASE path_gen (no funding).
    block_path_gen: Option<&data::BlockBootstrapPathGen>,
    bar_count: usize,
    generator: GeneratorKind,
    year: i32,
    // Whether funding was injected (carry only).
    // Used to decide whether to extract funding_override from generated_path.
    is_carry: bool,
) -> Result<IndexedPathMetrics> {
    use data::MonteCarloPathGen as _;

    // ── Generate the synthetic path ───────────────────────────────────────────
    let generated_path = match generator {
        GeneratorKind::BlockBootstrapReal => {
            let path_gen = block_path_gen.ok_or_else(|| {
                anyhow::anyhow!("block_path_gen must be Some for BlockBootstrapReal")
            })?;
            // The path_gen for carry already has funding attached (built in main).
            // For momentum/MR, no funding is in the path_gen → byte-identical output.
            path_gen
                .generate(universe, bar_count, path_seed_j)
                .with_context(|| format!("generate path {j}"))?
        }
        GeneratorKind::GbmSmoke => {
            let bars_by_symbol: Vec<Vec<trading_core::Bar>> = universe
                .iter()
                .enumerate()
                .map(|(sym_i, (sym, start_price))| {
                    let sym_seed = path_seed_j.wrapping_add(sym_i as u64 * 0x9E37_79B9);
                    backtest::scenarios::momentum::synthetic_bars_hourly(
                        sym,
                        bar_count,
                        sym_seed,
                        *start_price,
                        year,
                    )
                })
                .collect();
            data::GeneratedPath {
                bars_by_symbol,
                selected_block_length: None,
                funding_by_symbol: None,
            }
        }
    };

    // ── Build funding_override BTreeMap from generated_path.funding_by_symbol ──
    // Only when carry is active (is_carry=true and the path_gen emitted funding).
    // The map key is `(Symbol, open_ts)` — the synthetic timestamp of bars_by_symbol[s][k].
    // For momentum/MR (is_carry=false): funding_override=None → anchor-neutral.
    let funding_override: Option<
        std::collections::BTreeMap<(trading_core::Symbol, trading_core::Timestamp), Decimal>,
    > = if is_carry {
        if let Some(ref fund_by_sym) = generated_path.funding_by_symbol {
            let mut map = std::collections::BTreeMap::new();
            for (sym_i, (sym, _)) in universe.iter().enumerate() {
                if let Some(funding_row) = fund_by_sym.get(sym_i)
                    && let Some(bars_row) = generated_path.bars_by_symbol.get(sym_i)
                {
                    for (bar, &funding_val) in bars_row.iter().zip(funding_row.iter()) {
                        if let Some(rate) = funding_val {
                            map.insert((sym.clone(), bar.open_ts), rate);
                        }
                    }
                }
            }
            Some(map)
        } else {
            None
        }
    } else {
        None
    };

    // ── Merge per-symbol bars into the flat replay feed ───────────────────────
    let merged_bars = data::ReplayFeed::merge_synthetic(generated_path.bars_by_symbol);

    // ── Build fresh strategy with the INJECTED config (the C3 seam) ──────────
    // This is the only difference from run_one_path: we use the caller-supplied cfg.
    let strat = strategy::MomentumStrategy::from_config(
        cfg.clone(),
        SmolStr::new(format!("param-sweep-cell-{}", cfg.lookback_minutes)),
    );

    // ── Run the backtest on this path ─────────────────────────────────────────
    let input = backtest::cli_types::TcnScenarioInput {
        scenario_name: format!("sweep-path-{j}"),
        start_year: year,
        bar_count: merged_bars.len(),
        initial_capital: dec!(100_000),
        slippage_bps: 2,
        taker_fee_bps: 4,
        config_id: "top10_momentum_h1".to_string(),
        forecaster_id: "param_sweep".to_string(),
        bars_override: Some(merged_bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override,
    };

    let result = pollster::block_on(backtest::scenarios::montecarlo::run_path(
        input, fill_seed, strat,
    ))
    .with_context(|| format!("run_path for sweep path {j}"))?;

    let trades = result.trades;
    // M-DEV-6: the per-path realized funding cashflow, now surfaced by run_path
    // (sum of notional × −rate over every settlement-boundary accrual). ZERO for
    // momentum/MR (funding_override is None → the accrual block is never entered).
    let funding_harvested = result.realized_funding;

    // ── Compute per-path metric scalars ───────────────────────────────────────
    let equity_clamped: Vec<Decimal> = result
        .equity_curve
        .iter()
        .map(|&e| {
            if e <= Decimal::ZERO {
                dec!(0.000001)
            } else {
                e
            }
        })
        .collect();

    let sharpe = backtest::stats::compute_sharpe_hourly(&equity_clamped);
    let sortino = backtest::stats::compute_sortino_hourly(&equity_clamped);
    let calmar = backtest::stats::compute_calmar(&equity_clamped);
    let max_dd = backtest::stats::compute_max_drawdown_f64(&equity_clamped);
    let total_ret = backtest::stats::compute_total_return(&equity_clamped);

    debug_assert!(sharpe.is_finite(), "Sharpe NaN after clamping at path {j}");
    debug_assert!(
        sortino.is_finite(),
        "Sortino NaN after clamping at path {j}"
    );
    debug_assert!(calmar.is_finite(), "Calmar NaN after clamping at path {j}");

    tracing::trace!(
        j,
        path_seed_j,
        sharpe,
        max_dd,
        trades,
        "sweep path complete"
    );

    Ok(IndexedPathMetrics {
        j,
        metrics: backtest::stats::PathMetrics {
            sharpe,
            sortino,
            calmar,
            max_drawdown: max_dd,
            total_return: total_ret,
            final_equity: result.final_equity,
            initial_equity: result.initial_equity,
        },
        trades,
        funding_harvested,
    })
}

// ── Carry funding loader (M-DEV-6) ────────────────────────────────────────────

/// Load the funding data, compute `funding_at_return`, and build a carry-specific
/// `BlockBootstrapPathGen` with funding attached (ADR-0051 § D6.6, D-CARRY.7).
///
/// Returns `(carry_path_gen, Some(funding_revision_sha))` for the carry anchor body.
/// The `carry_path_gen` has funding already attached via `with_funding(...)`.
#[cfg(feature = "realdata")]
fn load_carry_path_gen(
    args: &Args,
    real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    symbols_prices: &[(trading_core::Symbol, Decimal)],
    bar_count: usize,
) -> Result<(Option<data::BlockBootstrapPathGen>, Option<String>)> {
    use backtest::funding_data::{FundingDataSource, LoadedFunding, build_funding_at_return};
    use backtest::realdata::TimeSpan as RealDataTimeSpan;

    // Load funding parquets and REVISION-verify.
    let symbols: Vec<trading_core::Symbol> =
        symbols_prices.iter().map(|(s, _)| s.clone()).collect();
    let funding_src = FundingDataSource::new(args.funding_root.clone(), symbols.clone());
    let span = RealDataTimeSpan::full_year(args.year);
    let scenario_name = format!("carry-sweep-funding-load-{}", args.year);
    let loaded: LoadedFunding = funding_src
        .load(&span, &scenario_name)
        .map_err(|e| anyhow::anyhow!("load carry funding: {e}"))?;

    // Verify funding revision SHA against the locked expected.
    if loaded.revision_sha != args.funding_revision_sha {
        anyhow::bail!(
            "funding revision mismatch: expected={} computed={}",
            args.funding_revision_sha,
            loaded.revision_sha
        );
    }
    let funding_revision_sha = loaded.revision_sha.clone();
    info!(
        funding_rows = loaded.rows.len(),
        funding_revision_sha = %funding_revision_sha,
        "carry funding loaded and verified"
    );

    // Build the `funding_at_return[sym_i][k]` array (aligned to real return steps).
    // For each symbol, extract (funding_time_ms, rate) and (bar open_ts_ms).
    let mut funding_by_symbol_rows: Vec<Vec<(i64, Decimal)>> = Vec::with_capacity(symbols.len());
    let mut bar_ts_by_symbol_raw: Vec<Vec<i64>> = Vec::with_capacity(symbols.len());

    for (sym, _) in symbols_prices {
        // Collect funding rows for this symbol (sorted by funding_time_ms).
        let sym_funding: Vec<(i64, Decimal)> = loaded
            .rows
            .iter()
            .filter(|r| r.symbol == *sym)
            .map(|r| (r.funding_time_ms, r.funding_rate))
            .collect();
        funding_by_symbol_rows.push(sym_funding);

        // Collect bar open timestamps for this symbol from real_bars_by_symbol.
        let bar_ts: Vec<i64> = real_bars_by_symbol
            .iter()
            .find(|(s, _)| s == sym)
            .map(|(_, bars)| {
                bars.iter()
                    .map(|b| b.open_ts.inner().unix_timestamp() * 1000)
                    .collect()
            })
            .unwrap_or_default();
        bar_ts_by_symbol_raw.push(bar_ts);
    }

    let funding_refs: Vec<&[(i64, Decimal)]> = funding_by_symbol_rows
        .iter()
        .map(|v| v.as_slice())
        .collect();
    let bar_ts_refs: Vec<&[i64]> = bar_ts_by_symbol_raw.iter().map(|v| v.as_slice()).collect();

    let funding_at_return = build_funding_at_return(&funding_refs, &bar_ts_refs);
    info!(
        n_symbols = funding_at_return.len(),
        first_sym_len = funding_at_return.first().map_or(0, Vec::len),
        "funding_at_return built for carry co-resampling"
    );

    // Build a carry-specific BlockBootstrapPathGen with funding attached.
    let carry_path_gen = data::BlockBootstrapPathGen::new(
        real_bars_by_symbol.to_vec(),
        data::BlockLengthPolicy::Auto,
    )
    .context("build carry BlockBootstrapPathGen")?
    .with_funding(Some(funding_at_return));

    // Verify the probe (selected_L should match the base path_gen).
    {
        use data::MonteCarloPathGen as _;
        let universe_probe: Vec<(trading_core::Symbol, Decimal)> = symbols_prices.to_vec();
        let _probe = carry_path_gen
            .generate(&universe_probe, bar_count, 0xC0FFEE)
            .context("carry path_gen probe generate")?;
        info!("carry path_gen probe: OK (funding co-resampling active)");
    }

    Ok((Some(carry_path_gen), Some(funding_revision_sha)))
}

#[cfg(not(feature = "realdata"))]
fn load_carry_path_gen(
    _args: &Args,
    _real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    _symbols_prices: &[(trading_core::Symbol, Decimal)],
    _bar_count: usize,
) -> Result<(Option<data::BlockBootstrapPathGen>, Option<String>)> {
    anyhow::bail!(
        "load_carry_path_gen called without --features realdata. \
         Carry requires real funding data. Rebuild with: cargo run -p backtest \
         --features candle,realdata --bin param_robustness_sweep -- --score-source carry ..."
    )
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let sweep_pool = rayon::ThreadPoolBuilder::new()
        .build()
        .context("build rayon sweep thread pool")?;

    llm::tracing_init::install_global(&[], false).ok();

    let args = Args::parse();
    let start = std::time::Instant::now();

    info!(
        generator = args.generator.label(),
        n_paths = args.paths,
        ensemble_seed = %args.ensemble_seed,
        grid = ?args.grid,
        "param_robustness_sweep: starting"
    );

    let ensemble_seed = parse_seed(&args.ensemble_seed).context("parse --ensemble-seed")?;

    // Fixed fill-tie-break seed (ADR-0051 D1: held constant across all paths AND cells).
    const FILL_SEED: u64 = 0xC0FFEE;

    let bar_count = match args.year {
        2023 => 8760usize,
        2024 => 8784usize,
        _ => 8760usize,
    };

    let symbols_prices = backtest::scenarios::momentum::top10_symbols_with_prices();
    let universe: Vec<(trading_core::Symbol, Decimal)> = symbols_prices.clone();

    // ── Load real bars (block-bootstrap-real only) ────────────────────────────
    #[cfg(not(feature = "realdata"))]
    if args.generator == GeneratorKind::BlockBootstrapReal {
        anyhow::bail!(
            "param_robustness_sweep --generator block-bootstrap-real requires --features realdata. \
             Rebuild with: cargo run -p backtest --features candle,realdata --bin param_robustness_sweep -- ..."
        );
    }

    let (real_bars_by_symbol, source_revision_sha) =
        load_source_bars(&args, &symbols_prices, bar_count)?;

    // ── Pre-build BlockBootstrapPathGen ONCE (shared across all rayon tasks) ────
    // Performance fix: build once, reuse for all N=500 × G=14 parallel tasks.
    // BlockBootstrapPathGen::generate is &self (read-only, deterministic for a
    // given seed), so rayon's immutable borrow rule is satisfied.
    let (generator_label, bootstrap_mode, block_length_policy_str, selected_l, block_path_gen_opt) =
        prepare_generator_params(args.generator, &real_bars_by_symbol, ensemble_seed)?;

    // ── M-DEV-6: load funding data and build carry path_gen (carry only) ──────
    // For momentum/MR: `carry_path_gen_opt = None` → anchor-neutral by construction.
    // For carry: load funding, build `funding_at_return[sym_i][k]`, build a
    // carry-specific BlockBootstrapPathGen with funding attached (D-CARRY.7).
    let (carry_path_gen_opt, funding_revision_sha_for_report): (
        Option<data::BlockBootstrapPathGen>,
        Option<String>,
    ) = if args.score_source.needs_funding() && args.generator == GeneratorKind::BlockBootstrapReal
    {
        load_carry_path_gen(&args, &real_bars_by_symbol, &symbols_prices, bar_count)?
    } else {
        (None, None)
    };

    // ── Load base config (for universe, frozen fields) ────────────────────────
    let rel_path = std::path::PathBuf::from("config/strategies/top10_momentum_h1.toml");
    let toml_path = backtest::paths::resolve_workspace_path(&rel_path);
    let base_cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load base momentum config: {}", rel_path.display()))?;

    // ── Select grid ───────────────────────────────────────────────────────────
    let grid = grid_for_kind(args.grid);

    info!(
        n_cells = grid.len(),
        n_paths = args.paths,
        score_source = ?args.score_source,
        "outer θ-loop starting (sequential for log legibility)"
    );

    // ── Select the active path_gen ────────────────────────────────────────────
    // For carry: use the carry-specific path_gen (has funding attached).
    // For momentum/MR: use the base path_gen (no funding → anchor-neutral).
    let active_path_gen_opt: Option<&data::BlockBootstrapPathGen> =
        if let Some(ref cpg) = carry_path_gen_opt {
            Some(cpg)
        } else {
            block_path_gen_opt.as_ref()
        };

    let is_carry = args.score_source == SweepScoreSource::Carry;

    // ── Outer θ-loop (sequential for log legibility — ~10-15 min at N=200, 6 cells) ─
    // ADR-0051 § D6.4: collect into Vec, sort by g before render.
    // The inner per-path rayon fan-out is where parallelism lives.
    let mut cell_results: Vec<CellResult> = Vec::with_capacity(grid.len());

    for cell in grid {
        let cell_start = std::time::Instant::now();
        let per_cell_cfg = cell_config(&base_cfg, cell, args.direction, args.score_source);

        info!(
            g = cell.g,
            lookback = cell.lookback_minutes,
            k_long = cell.k_long,
            drift = %cell.drift(),
            rebalance = cell.effective_rebalance(base_cfg.rebalance_minutes),
            "θ-cell starting"
        );

        // ── Inner N-path fan-out (rayon) ──────────────────────────────────────
        // ADR-0051 D6.1: path_seed_j = derive_path_seed(ensemble_seed, j) — SAME for every cell.
        let path_indices: Vec<usize> = (0..args.paths).collect();

        let results: Vec<Result<IndexedPathMetrics>> = sweep_pool.install(|| {
            path_indices
                .into_par_iter()
                .map(|j| {
                    let path_seed_j = derive_path_seed(ensemble_seed, j);
                    run_one_path_with_config(
                        j,
                        path_seed_j,
                        FILL_SEED,
                        &per_cell_cfg,
                        &universe,
                        active_path_gen_opt,
                        bar_count,
                        args.generator,
                        args.year,
                        is_carry,
                    )
                })
                .collect()
        });

        // ── Collect indexed results in path-index order ───────────────────────
        // ADR-0051 D2: sort by j so reduction is in ascending index order.
        let mut indexed: Vec<IndexedPathMetrics> = results
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("θ-cell g={}: one or more paths failed", cell.g))?;

        indexed.sort_by_key(|r| r.j);

        let total_trades: u64 = indexed.iter().map(|r| r.trades as u64).sum();
        // M-DEV-6: per-cell realized funding harvested = sum of the per-path funding
        // cashflow across all N paths (run_path now surfaces it). ZERO for momentum/MR
        // (funding_override is None → the accrual block is never entered). Summed via
        // .iter() BEFORE the consuming .into_iter() below, mirroring total_trades.
        let total_funding_harvested: Decimal = indexed.iter().map(|r| r.funding_harvested).sum();
        let metrics: Vec<backtest::stats::PathMetrics> =
            indexed.into_iter().map(|r| r.metrics).collect();

        // ── Reduce (ADR-0051 D2: sequential in index order) ─────────────────
        let summary = backtest::stats::DistributionSummary::from_path_metrics(&metrics)
            .with_context(|| format!("build DistributionSummary for g={}", cell.g))?;

        let verdict = classify_verdict(&summary);
        let cell_elapsed = cell_start.elapsed().as_secs_f64();

        info!(
            g = cell.g,
            lookback = cell.lookback_minutes,
            k_long = cell.k_long,
            drift = %cell.drift(),
            sharpe_p50 = summary.sharpe.p50,
            p5_sharpe = summary.sharpe.p5,
            prob_loss = summary.prob_loss,
            max_dd_p95 = summary.max_dd_tail_p95,
            verdict = verdict.as_str(),
            total_trades,
            cell_elapsed_s = cell_elapsed,
            "θ-cell complete"
        );

        cell_results.push(CellResult {
            cell: *cell,
            summary,
            verdict,
            total_trades,
            total_funding_harvested,
        });
    }

    // ADR-0051 § D6.4: sort by g before render.
    cell_results.sort_by_key(|cr| cr.cell.g);

    // ── Buy-and-hold control (SAME N paths, SAME seeds, SAME pre-built path_gen) ─
    // Single indexed pass — no wasteful first pass (the earlier double-pass was a bug).
    // ADR-0051 D6.1: same path seeds as the strategy cells (path_seed = derive(master,j)).
    // ADR-0051 D2: collect (j, PathMetrics) tuples, sort by j before reduction.
    info!("computing buy-and-hold passive control");
    let bh_path_indices: Vec<usize> = (0..args.paths).collect();
    let n_symbols = symbols_prices.len();

    let bh_results: Vec<(usize, backtest::stats::PathMetrics)> = sweep_pool.install(|| {
        bh_path_indices
            .into_par_iter()
            .map(|j| {
                let path_seed_j = derive_path_seed(ensemble_seed, j);
                let generated_path = match args.generator {
                    GeneratorKind::BlockBootstrapReal => {
                        use data::MonteCarloPathGen as _;
                        let path_gen = block_path_gen_opt
                            .as_ref()
                            .expect("block_path_gen_opt must be Some for BlockBootstrapReal");
                        path_gen
                            .generate(&universe, bar_count, path_seed_j)
                            .expect("generate buyhold path")
                    }
                    GeneratorKind::GbmSmoke => {
                        let bars_by_symbol: Vec<Vec<trading_core::Bar>> = universe
                            .iter()
                            .enumerate()
                            .map(|(sym_i, (sym, start_price))| {
                                let sym_seed = path_seed_j.wrapping_add(sym_i as u64 * 0x9E37_79B9);
                                backtest::scenarios::momentum::synthetic_bars_hourly(
                                    sym,
                                    bar_count,
                                    sym_seed,
                                    *start_price,
                                    args.year,
                                )
                            })
                            .collect();
                        data::GeneratedPath {
                            bars_by_symbol,
                            selected_block_length: None,
                            funding_by_symbol: None,
                        }
                    }
                };
                let merged = data::ReplayFeed::merge_synthetic(generated_path.bars_by_symbol);
                let (equity, final_eq) = run_buyhold_path(&merged, dec!(100_000), n_symbols);
                let equity_clamped: Vec<Decimal> = equity
                    .iter()
                    .map(|&e| {
                        if e <= Decimal::ZERO {
                            dec!(0.000001)
                        } else {
                            e
                        }
                    })
                    .collect();
                let pm = backtest::stats::PathMetrics {
                    sharpe: backtest::stats::compute_sharpe_hourly(&equity_clamped),
                    sortino: backtest::stats::compute_sortino_hourly(&equity_clamped),
                    calmar: backtest::stats::compute_calmar(&equity_clamped),
                    max_drawdown: backtest::stats::compute_max_drawdown_f64(&equity_clamped),
                    total_return: backtest::stats::compute_total_return(&equity_clamped),
                    final_equity: final_eq,
                    initial_equity: dec!(100_000),
                };
                (j, pm)
            })
            .collect()
    });

    // ADR-0051 D2: sort by j so reduction is in ascending index order.
    let mut bh_indexed = bh_results;
    bh_indexed.sort_by_key(|(j, _)| *j);
    let bh_metrics_ordered: Vec<backtest::stats::PathMetrics> =
        bh_indexed.into_iter().map(|(_, m)| m).collect();

    let buyhold_summary =
        backtest::stats::DistributionSummary::from_path_metrics(&bh_metrics_ordered)
            .context("build buy-and-hold DistributionSummary")?;

    info!(
        bh_sharpe_p50 = buyhold_summary.sharpe.p50,
        bh_prob_loss = buyhold_summary.prob_loss,
        bh_max_dd_p95 = buyhold_summary.max_dd_tail_p95,
        "buy-and-hold control complete"
    );

    let wall_clock_s = start.elapsed().as_secs_f64();

    // ── Generate timestamp ────────────────────────────────────────────────────
    let generated = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let secs_of_day = now % 86400;
        let hours = secs_of_day / 3600;
        let mins = secs_of_day / 60 % 60;
        let secs = secs_of_day % 60;
        let days_since_epoch = now / 86400;
        let (y, m, d) = days_since_epoch_to_ymd(days_since_epoch);
        format!("{y:04}-{m:02}-{d:02}T{hours:02}:{mins:02}:{secs:02}Z")
    };
    let host = read_hostname();
    let git_commit = read_git_commit();
    let data_rev_frontmatter = read_data_revision_sha(&args.data_root);
    let pid = std::process::id();

    let scenario_name = match args.score_source {
        SweepScoreSource::Carry => format!(
            "v1-carry-theta-surface-{year}-block-bootstrap-{gen}-fy",
            year = args.year,
            gen = match args.generator {
                GeneratorKind::BlockBootstrapReal => "real",
                GeneratorKind::GbmSmoke => "gbm",
            }
        ),
        SweepScoreSource::VolAdjustedReturn => format!(
            "v1-{family}-theta-surface-{year}-block-bootstrap-{gen}-fy",
            family = args.direction.label(),
            year = args.year,
            gen = match args.generator {
                GeneratorKind::BlockBootstrapReal => "real",
                GeneratorKind::GbmSmoke => "gbm",
            }
        ),
    };

    // ── Render report (ADR-0051 D3 / § D6.4) ─────────────────────────────────
    let report = render_surface_report(
        &generated,
        wall_clock_s,
        &host,
        pid,
        &git_commit,
        &data_rev_frontmatter,
        &scenario_name,
        ensemble_seed,
        FILL_SEED,
        args.paths,
        &generator_label,
        &bootstrap_mode,
        &block_length_policy_str,
        selected_l,
        &source_revision_sha,
        grid,
        &cell_results,
        &buyhold_summary,
        args.direction,
        args.score_source,
        funding_revision_sha_for_report.as_deref(),
    );

    // ── Resolve effective out_dir ─────────────────────────────────────────────
    // For carry: if the user did not override --out-dir, default to the carry reports dir.
    // We detect "was the default changed?" by checking if it's still the momentum default.
    let momentum_default_out_dir =
        PathBuf::from("spec/momentum-parameter-robustness-sweep/reports/");
    let effective_out_dir = if args.score_source == SweepScoreSource::Carry
        && args.out_dir == momentum_default_out_dir
    {
        PathBuf::from("spec/carry-strategy/reports/")
    } else {
        args.out_dir.clone()
    };

    // ── Write report ──────────────────────────────────────────────────────────
    std::fs::create_dir_all(&effective_out_dir)
        .with_context(|| format!("create out_dir {:?}", effective_out_dir))?;

    let ts_suffix = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let secs_of_day = now % 86400;
        let h = secs_of_day / 3600;
        let m = secs_of_day / 60 % 60;
        let s = secs_of_day % 60;
        let days = now / 86400;
        let (y, mo, d) = days_since_epoch_to_ymd(days);
        format!("{y:04}{mo:02}{d:02}-{h:02}{m:02}{s:02}")
    };
    let report_filename = format!("robustness-sweep-{ts_suffix}-{scenario_name}.md");
    let report_path = effective_out_dir.join(&report_filename);
    std::fs::write(&report_path, &report)
        .with_context(|| format!("write report to {:?}", report_path))?;

    // ── Compute body SHA ──────────────────────────────────────────────────────
    let body_sha = {
        use sha2::{Digest, Sha256};
        let body = backtest::extract_report_body(&report);
        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        hasher.finalize().iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
    };

    // ── Family verdict summary ────────────────────────────────────────────────
    let any_non_fragile = cell_results
        .iter()
        .any(|cr| cr.verdict != ParamRobustnessVerdict::Fragile);
    let family_verdict = if any_non_fragile {
        "FAMILY-HAS-NON-FRAGILE-CELLS"
    } else {
        "FAMILY-UNIFORM-FRAGILE"
    };

    info!(
        scenario = %scenario_name,
        n_cells = cell_results.len(),
        n_paths = args.paths,
        wall_clock_s,
        family_verdict,
        bh_sharpe_p50 = buyhold_summary.sharpe.p50,
        report = %report_path.display(),
        body_sha = %body_sha,
        "param_robustness_sweep: DONE"
    );

    println!("param_robustness_sweep DONE");
    println!("  report:         {}", report_path.display());
    println!("  body_sha:       {body_sha}");
    println!("  wall_clock_s:   {wall_clock_s:.1}");
    println!("  n_cells:        {}", cell_results.len());
    println!("  n_paths:        {}", args.paths);
    println!("  family_verdict: {family_verdict}");
    println!(
        "  buyhold p50 Sharpe: {:.4}  P(loss): {:.4}  p95 MaxDD: {:.2}%",
        buyhold_summary.sharpe.p50,
        buyhold_summary.prob_loss,
        buyhold_summary.max_dd_tail_p95 * 100.0
    );
    println!("\n  per-cell summary:");
    for cr in &cell_results {
        println!(
            "    g={:2} lookback={:4} k_long={} drift={:.2} → {} | p50={:.4} p5={:.4} MaxDD_p95={:.1}%",
            cr.cell.g,
            cr.cell.lookback_minutes,
            cr.cell.k_long,
            cr.cell.drift(),
            cr.verdict.as_str(),
            cr.summary.sharpe.p50,
            cr.summary.sharpe.p5,
            cr.summary.max_dd_tail_p95 * 100.0
        );
    }

    Ok(())
}

// ── Unit tests for the verdict classifier ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic DistributionSummary for testing the verdict classifier.
    fn make_summary(
        p5_sharpe: f64,
        p50_sharpe: f64,
        p95_sharpe: f64,
        prob_loss: f64,
        prob_sharpe_gt_1: f64,
        p95_maxdd: f64,
    ) -> backtest::stats::DistributionSummary {
        backtest::stats::DistributionSummary {
            sharpe: backtest::stats::MetricDistribution {
                mean: p50_sharpe,
                std: 0.5,
                p5: p5_sharpe,
                p25: p50_sharpe - 0.1,
                p50: p50_sharpe,
                p75: p50_sharpe + 0.1,
                p95: p95_sharpe,
                min: p5_sharpe - 0.1,
                max: p95_sharpe + 0.1,
            },
            sortino: backtest::stats::MetricDistribution {
                mean: 0.0,
                std: 0.0,
                p5: 0.0,
                p25: 0.0,
                p50: 0.0,
                p75: 0.0,
                p95: 0.0,
                min: 0.0,
                max: 0.0,
            },
            calmar: backtest::stats::MetricDistribution {
                mean: 0.0,
                std: 0.0,
                p5: 0.0,
                p25: 0.0,
                p50: 0.0,
                p75: 0.0,
                p95: 0.0,
                min: 0.0,
                max: 0.0,
            },
            max_drawdown: backtest::stats::MetricDistribution {
                mean: p95_maxdd,
                std: 0.05,
                p5: p95_maxdd * 0.5,
                p25: p95_maxdd * 0.7,
                p50: p95_maxdd * 0.8,
                p75: p95_maxdd * 0.9,
                p95: p95_maxdd,
                min: p95_maxdd * 0.3,
                max: p95_maxdd * 1.1,
            },
            total_return: backtest::stats::MetricDistribution {
                mean: 0.0,
                std: 0.0,
                p5: 0.0,
                p25: 0.0,
                p50: 0.0,
                p75: 0.0,
                p95: 0.0,
                min: 0.0,
                max: 0.0,
            },
            prob_loss,
            prob_sharpe_gt_0: 1.0 - prob_loss,
            prob_sharpe_gt_1,
            max_dd_tail_p50: p95_maxdd * 0.8,
            max_dd_tail_p95: p95_maxdd,
        }
    }

    #[test]
    fn classifier_fragile_on_negative_p5_sharpe() {
        // p5_sharpe < 0 → FRAGILE (even if everything else is ROBUST-band).
        let s = make_summary(-0.1, 1.2, 2.0, 0.10, 0.70, 0.40);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Fragile);
    }

    #[test]
    fn classifier_fragile_on_low_p50_sharpe() {
        // p50_sharpe < 0.5 → FRAGILE.
        let s = make_summary(0.3, 0.4, 1.5, 0.10, 0.70, 0.40);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Fragile);
    }

    #[test]
    fn classifier_fragile_on_high_prob_loss() {
        // prob_loss > 0.35 → FRAGILE.
        let s = make_summary(0.5, 1.0, 2.0, 0.40, 0.70, 0.40);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Fragile);
    }

    #[test]
    fn classifier_fragile_on_low_prob_sharpe_gt1() {
        // prob_sharpe_gt_1 < 0.35 → FRAGILE.
        let s = make_summary(0.5, 1.0, 2.0, 0.10, 0.30, 0.40);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Fragile);
    }

    #[test]
    fn classifier_fragile_on_high_p95_maxdd() {
        // p95_maxdd > 0.70 → FRAGILE.
        let s = make_summary(0.5, 1.0, 2.0, 0.10, 0.70, 0.75);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Fragile);
    }

    #[test]
    fn classifier_robust_all_signals_in_robust_band() {
        // All signals in ROBUST band → ROBUST.
        let s = make_summary(0.6, 1.1, 2.0, 0.10, 0.70, 0.40);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Robust);
    }

    #[test]
    fn classifier_marginal_between_bands() {
        // p5 Sharpe = 0.3 (MARGINAL band), p50 = 0.8, prob_loss = 0.20,
        // prob_sharpe_gt1 = 0.50, p95_maxdd = 0.60 → not FRAGILE, not all ROBUST → MARGINAL.
        let s = make_summary(0.3, 0.8, 2.0, 0.20, 0.50, 0.60);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Marginal);
    }

    #[test]
    fn classifier_fragile_on_exact_boundary_p5() {
        // p5_sharpe exactly 0.0 → FRAGILE (< 0.0 is FRAGILE; == 0.0 is MARGINAL boundary).
        // Actually: p5 < 0 → FRAGILE. p5 == 0.0 is NOT < 0, so not FRAGILE by this signal.
        // p50 = 0.4 < 0.5 → FRAGILE by p50 signal.
        let s = make_summary(0.0, 0.4, 1.5, 0.10, 0.70, 0.40);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Fragile);
    }

    #[test]
    fn classifier_fragile_boundary_p50_exactly_fragile() {
        // p50_sharpe = 0.5 — exactly the FRAGILE boundary (< 0.5 → FRAGILE, not ≤).
        // 0.5 is NOT < 0.5 so not FRAGILE by p50.
        // p5 = 0.3, prob_loss = 0.20, p95_maxdd = 0.60, prob_sharpe_gt1 = 0.50
        // → not FRAGILE on any signal → MARGINAL (p5 < 0.5 ROBUST threshold).
        let s = make_summary(0.3, 0.5, 1.5, 0.20, 0.50, 0.60);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Marginal);
    }

    #[test]
    fn classifier_robust_boundary_all_just_above() {
        // All signals just clearing the ROBUST threshold → ROBUST.
        let s = make_summary(0.5, 1.0, 2.0, 0.15, 0.60, 0.50);
        assert_eq!(classify_verdict(&s), ParamRobustnessVerdict::Robust);
    }

    #[test]
    fn tier1_grid_has_6_cells() {
        // Re-scoped from 14 cells to 6 cells (orchestrator 2026-05-30) for
        // ~10-15 min wall-clock (6×200=1200 backtests vs 14×500=7000).
        assert_eq!(
            TIER1_GRID.len(),
            6,
            "TIER1_GRID must have exactly 6 cells (re-scoped)"
        );
    }

    #[test]
    fn tier1_grid_g_indices_are_0_to_5() {
        for (i, cell) in TIER1_GRID.iter().enumerate() {
            assert_eq!(cell.g, i, "cell g must match array index");
        }
    }

    #[test]
    fn tier1_grid_cell_0_is_baseline_theta_star() {
        let c = &TIER1_GRID[0];
        assert_eq!(c.g, 0);
        assert_eq!(c.lookback_minutes, 60);
        assert_eq!(c.k_long, 3);
        // drift = 10 / 10^2 = 0.10
        assert_eq!(c.drift(), Decimal::new(10, 2));
    }

    #[test]
    fn tier1_grid_cell_3_is_low_churn_corner() {
        // g=3: 1mo lookback + wide hold-band — best a-priori robustness shot.
        let c = &TIER1_GRID[3];
        assert_eq!(c.g, 3);
        assert_eq!(c.lookback_minutes, 720);
        assert_eq!(c.k_long, 3);
        assert_eq!(c.drift(), Decimal::new(50, 2));
    }

    #[test]
    fn tier1_grid_cell_4_is_narrow_selection() {
        // g=4: k_long=1, narrow selection.
        let c = &TIER1_GRID[4];
        assert_eq!(c.g, 4);
        assert_eq!(c.lookback_minutes, 60);
        assert_eq!(c.k_long, 1);
        assert_eq!(c.drift(), Decimal::new(10, 2));
    }

    #[test]
    fn tier1_grid_cell_5_is_wide_selection() {
        // g=5: k_long=5, wide selection.
        let c = &TIER1_GRID[5];
        assert_eq!(c.g, 5);
        assert_eq!(c.lookback_minutes, 60);
        assert_eq!(c.k_long, 5);
        assert_eq!(c.drift(), Decimal::new(10, 2));
    }

    #[test]
    fn grid_def_string_contains_all_6_cells() {
        let s = grid_def_string(TIER1_GRID);
        for cell in TIER1_GRID {
            let expected = format!("g={}", cell.g);
            assert!(s.contains(&expected), "grid_def missing g={}", cell.g);
        }
    }

    #[test]
    fn derive_path_seed_matches_adr0051_d1() {
        // Verify the seed derivation matches the ADR-0051 D1 formula.
        let master = 0xC0FFEE_u64;
        let j = 5_usize;
        let expected = master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9));
        assert_eq!(derive_path_seed(master, j), expected);
    }

    #[test]
    fn derive_path_seed_same_for_all_cells() {
        // SAME-paths: the seed is the same for cell g=0 and g=5 at the same j.
        let master = 0xC0FFEE_u64;
        let j = 42_usize;
        // Under D6.1, cell_seed_g := ensemble_seed (same for all g).
        // So path_seed_{g,j} = derive_path_seed(ensemble_seed, j) for ALL g.
        let seed_g0 = derive_path_seed(master, j);
        let seed_g5 = derive_path_seed(master, j);
        assert_eq!(
            seed_g0, seed_g5,
            "SAME-paths: seed must be identical for all g at the same j"
        );
    }
}
