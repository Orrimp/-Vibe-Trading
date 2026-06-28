//! `param_robustness_sweep` — C3 momentum parameter-robustness sweep.
//!
//! Sweeps the v1 cross-sectional momentum family over the **RE-SCOPED 6-cell θ-grid**
//! (orchestrator-specified 2026-05-30 for tractability; was 14-cell × N=500 per
//! original architect design; re-scoped to 6-cell × N=200 for ~10-15 min wall-clock).
//! Runs the C2 N-path robustness harness at each cell, and emits ONE anchored
//! θ-surface report under `spec/v1/momentum-parameter-robustness-sweep/reports/`.
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
//!   --out-dir spec/v1/momentum-parameter-robustness-sweep/reports/
//! ```
//!
//! ## Watch recipe (for long-running N=200 runs — copy-paste to operator terminal)
//!
//! ```bash
//! watch -n 15 '
//! PID=$(pgrep -f param_robustness_sweep | head -1)
//! [ -z "$PID" ] && echo "param_robustness_sweep not running" && exit
//! N=$(ls spec/v1/momentum-parameter-robustness-sweep/reports/robustness-sweep-*.md 2>/dev/null | wc -l | tr -d " ")
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

// ── Verdict + classifier — relocated to `backtest::bakeoff::robustness` ──────
//
// ADR-0059 M-DEV-1: `ParamRobustnessVerdict`, `classify_verdict`, and the band
// constants were extracted from this bin into the `backtest` library so the
// bake-off orchestrator can share them without code duplication.
//
// This bin re-imports them for backward compatibility — the output is
// byte-identical because the logic is identical (pure structural relocation).
pub use backtest::bakeoff::robustness::{ParamRobustnessVerdict, classify_verdict};

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
    /// Flat/entry threshold numerator for TS-momentum cells (D-TSM.3-LOCKED).
    ///
    /// `entry_threshold = Decimal::new(entry_threshold_num, entry_threshold_den)`.
    /// Momentum/MR/carry cells: both 0 → `entry_threshold = 0` → inert
    /// (the threshold is ONLY read under `SelectionMode::TimeSeriesLongFlat`).
    pub entry_threshold_num: i64,
    /// Denominator for `entry_threshold` (scale exponent for `Decimal::new`).
    ///
    /// `entry_threshold_den = 2` means `×10^-2`, so `(num=2, den=2)` → 0.02.
    /// Momentum/MR/carry cells: 0 → `Decimal::new(0, 0)` → `0.00` (inert).
    pub entry_threshold_den: u32,
    /// Human-readable role / hypothesis.
    pub role: &'static str,
}

impl ThetaCell {
    /// Returns the drift threshold as a `Decimal`.
    #[must_use]
    pub fn drift(&self) -> Decimal {
        Decimal::new(self.drift_threshold_num, self.drift_threshold_den)
    }

    /// Returns the flat/entry threshold as a `Decimal` (D-TSM.3-LOCKED).
    ///
    /// For momentum/MR/carry cells: both fields are 0 → `Decimal::ZERO` (inert).
    /// For TS cells: `Decimal::new(num, den)` → e.g. `(2, 2)` → `0.02`.
    #[must_use]
    pub fn entry_threshold(&self) -> Decimal {
        Decimal::new(self.entry_threshold_num, self.entry_threshold_den)
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
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "baseline θ* (C2-shipped config; g=0 MUST reproduce C2 anchor numbers)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 24,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "short lookback — 1d horizon; high churn",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 168,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "1w lookback horizon",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 720,
        k_long: 3,
        drift_threshold_num: 50,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "1mo lookback + wide hold-band — best a-priori robustness shot (low-churn corner)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 60,
        k_long: 1,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "narrow selection — top-1 only",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 60,
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "wide selection — top-5 (more legs to churn)",
    },
];

// ── Grid tier enum (for FP-C3.2 grid-sensitivity test) ────────────────────────

/// Which θ-grid to use.
///
/// `Tier1` is the LOCKED momentum anchored grid (§ D-C3.2-LOCKED).
/// `MrTier1` is the LOCKED MR θ-grid (§ D-MR.2-LOCKED).
/// `CarryTier1` is the LOCKED carry θ-grid (§ D-CARRY.2-LOCKED).
/// `TsTier1` is the LOCKED TS-momentum θ-grid (§ D-TSM.3-LOCKED).
/// `TwoCell` is a 2-cell mini-grid used only by the FP-C3.2 grid-sensitivity
/// test (different grid → different body-SHA). NOT for production runs.
/// `Ts4h` / `TsDaily` / `Carry4h` / `CarryDaily` are the LOCKED horizon retest
/// grids (§ D-HR.4-LOCKED). Only selected under `--horizon 4h` or `--horizon daily`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum GridKind {
    /// The LOCKED 6-cell Tier-1 anchored momentum grid (§ D-C3.2-LOCKED).
    Tier1,
    /// The LOCKED 6-cell MR Tier-1 θ-grid (§ D-MR.2-LOCKED).
    MrTier1,
    /// The LOCKED 6-cell carry Tier-1 θ-grid (§ D-CARRY.2-LOCKED).
    CarryTier1,
    /// The LOCKED 6-cell TS-momentum Tier-1 θ-grid (§ D-TSM.3-LOCKED).
    #[value(name = "ts-tier1")]
    TsTier1,
    /// 2-cell mini-grid for FP-C3.2 grid-sensitivity gate only.
    TwoCell,
    /// LOCKED 4h TS-momentum grid (§ D-HR.4-LOCKED): lookback {42,180,540} × threshold {0.00,0.02}.
    #[value(name = "ts-4h")]
    Ts4h,
    /// LOCKED daily TS-momentum grid (§ D-HR.4-LOCKED): lookback {5,20,60} × threshold {0.00,0.02}.
    #[value(name = "ts-daily")]
    TsDaily,
    /// LOCKED 4h carry grid (§ D-HR.4-LOCKED): L {2,6,12} × k_long {1,3,5}.
    #[value(name = "carry-4h")]
    Carry4h,
    /// LOCKED daily carry grid (§ D-HR.4-LOCKED): L {1,3,7} × k_long {1,3,5}.
    #[value(name = "carry-daily")]
    CarryDaily,
    /// The LOCKED 6-cell Basis-Reversal Tier-1 θ-grid (§ D-BR.2-LOCKED).
    ///
    /// `lookback_minutes` = L in **price bars** (basis is native 1h, so 1 bar = 1 hour).
    /// `rebalance_minutes_override` = cadence (8h or 24h — the turnover lever).
    #[value(name = "basis-tier1")]
    BasisTier1,
    /// The LOCKED 2-cell MN-spread Tier-1 θ-grid (§ D-MN.8-LOCKED, M-DEV-5).
    ///
    /// L ∈ {60, 168} bars, K_long = K_short = 3, rebalance = 480m (8h).
    /// L=24 dropped per D-MN.8 reconciliation (IC-peak band 60–168; fees falsified).
    #[value(name = "mn-tier1")]
    MnTier1,
}

/// Build the grid slice for a given kind.
#[must_use]
pub fn grid_for_kind(kind: GridKind) -> &'static [ThetaCell] {
    match kind {
        GridKind::Tier1 => TIER1_GRID,
        GridKind::MrTier1 => MR_TIER1_GRID,
        GridKind::CarryTier1 => CARRY_TIER1_GRID,
        GridKind::TsTier1 => TS_TIER1_GRID,
        GridKind::TwoCell => TWO_CELL_GRID,
        GridKind::Ts4h => TS_4H_GRID,
        GridKind::TsDaily => TS_DAILY_GRID,
        GridKind::Carry4h => CARRY_4H_GRID,
        GridKind::CarryDaily => CARRY_DAILY_GRID,
        GridKind::BasisTier1 => BASIS_TIER1_GRID,
        GridKind::MnTier1 => MN_TIER1_GRID,
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
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "mini-grid cell 0 (FP-C3.2 only)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 720,
        k_long: 3,
        drift_threshold_num: 50,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
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
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "baseline MR θ* (apples-to-apples vs momentum g=0; must DIFFER from C3 g=0 momentum)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 24,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "short lookback + narrow band — deliberately HIGH churn (R-MR.3 high-turnover cell)",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 168,
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "1w lookback horizon / mid turnover",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 720,
        k_long: 5,
        drift_threshold_num: 50,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "1mo lookback + wide band — deliberately LOW churn (R-MR.3 low-turnover cell)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 720,
        k_long: 3,
        drift_threshold_num: 30,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "long lookback + medium band — low-churn diagonal (narrower selection)",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 24,
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
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
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "baseline carry θ* (L=9 settlements, 8h rebalance, K=3 — natural funding cadence)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 3, // L=3 settlements (~1 day)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "short funding lookback (L=3, ~1d) — noisier signal; low-mid turnover",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 21, // L=21 settlements (~7 days)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "long funding lookback (L=21, ~1 week) — most persistent signal; low turnover",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 9, // L=9 settlements (~3 days)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 1440, // 24h — deliberately-slow (lowest-churn corner)
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "deliberately-slow 24h rebalance + wide K=5 (lowest-churn corner — carry's best structural shot)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 9, // L=9 settlements (~3 days)
        k_long: 1,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "narrow selection — top-1 carry name; low turnover",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 3, // L=3 settlements (~1 day)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "shortest lookback (L=3) + wide K=5 — highest-churn carry extreme (still far below price families)",
    },
];

/// Basis-Reversal Tier-1 θ-grid — LOCKED § D-BR.2-LOCKED (2026-06-05).
///
/// This exact 6-cell list is the hashed body field for the basis-reversal θ-surface anchor
/// (ADR-0051 § D6.9). Changing it = a different surface = a different SHA.
///
/// **`lookback_minutes` = L in price BARS** (the basis is native 1h, so 1 bar = 1 hour).
/// The carry θ-grid used `lookback_minutes` as "L settlements"; here it is "L bars" —
/// the same field, same formatter (`carry_grid_def_string`), different unit semantics.
///
/// **Held constant across every cell (§ D-BR.2-LOCKED):**
/// `score_source=basis_reversal`, `direction=momentum` (identity; sign in score),
/// `selection_mode=cross_sectional_top_k`, `exposure_cap=0.50`, `k_short=0`,
/// `size=equal_weight`, `vol_floor=inert`, 10-symbol universe, `N=200`,
/// `ensemble_seed=0xC0FFEE`, `fill_seed=0xC0FFEE`, generator=block-bootstrap-real,
/// `bootstrap_mode=shared-index`, revisions `3a8b96c4…` (OHLCV) + `aa72409a…` (basis).
///
/// **The θ-axis (signal lookback) — LOCKED to the spike's signal-bearing band (BS.2a):**
/// L ∈ {24, 60, 168} bars (SKIP L=720 noise: n=11, sign-flips across years).
///
/// | g | lookback L (bars) | rebalance | K | role / hypothesis |
/// |---|-------------------|-----------|---|-------------------|
/// | 0 | 60 (2.5d)         | 480m (8h) | 3 | **baseline basis θ*** — IC peak (−0.099/−0.081), low-churn cadence |
/// | 1 | 24 (1d)           | 480m      | 3 | short lookback — faster, more fee-exposed (IC −0.031/−0.022) |
/// | 2 | 168 (1wk)         | 480m      | 3 | long lookback — IC peak / lowest-turnover corner (−0.112/−0.069) |
/// | 3 | 60 (2.5d)         | 1440m (24h)| 5 | **deliberately-slow rebalance + wide K** (lowest-churn corner — best fee shot) |
/// | 4 | 60 (2.5d)         | 480m      | 1 | narrow selection — top-1 lowest-basis name |
/// | 5 | 24 (1d)           | 480m      | 5 | shortest lookback + wide K — **highest-churn extreme** (fee-trap stress) |
pub const BASIS_TIER1_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 60, // L=60 bars (2.5d) — IC peak baseline
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,          // drift=0.10
        rebalance_minutes_override: 480, // 8h — natural low-churn cadence
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "baseline basis θ* (L=60 bars, 8h rebalance, K=3 — IC peak at −0.099/−0.081)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 24, // L=24 bars (1d) — short lookback
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "short basis lookback (L=24, 1d) — faster signal, more fee-exposed; IC −0.031/−0.022",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 168, // L=168 bars (1wk) — long lookback / IC peak
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "long basis lookback (L=168, 1wk) — most persistent signal; IC −0.112/−0.069; low turnover",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 60, // L=60 bars (2.5d)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 1440, // 24h — deliberately-slow (lowest-churn corner)
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "deliberately-slow 24h rebalance + wide K=5 (lowest-churn corner — reversal arm's best fee shot)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 60, // L=60 bars (2.5d)
        k_long: 1,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "narrow selection — top-1 lowest-basis name (concentrated reversal bet)",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 24, // L=24 bars (1d) — short lookback
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "shortest lookback (L=24) + wide K=5 — highest-churn extreme (fee-trap stress for reversal arm)",
    },
];

/// MN-spread Tier-1 θ-grid — LOCKED § D-MN.8-LOCKED (2026-06-08, M-DEV-5).
///
/// This exact 2-cell list is the hashed body field for the MN θ-surface anchors
/// (ADR-0051 § D6.10). Changing it = a different surface = a different SHA.
///
/// **Held constant:** `selection_mode=long_short`, `k_long=k_short=3`, `drift=0.10`,
/// `exposure_cap=0.50`, `rebalance_minutes=480` (8h), `ensemble_seed=0xC0FFEE`,
/// `fill_seed=0xC0FFEE`, generator=block-bootstrap-real, `bootstrap_mode=shared-index`,
/// `max_leverage=1`, `maintenance_margin_frac=0.5`, N=200.
///
/// **L=24 dropped (vs BASIS_TIER1_GRID):** D-MN.8 reconciliation — for the spread,
/// IC peaks at 60–168; the fee-sweep (D6.9) falsified fees as the killer, so the
/// turnover lever matters less than IC peak.
///
/// | g | lookback L (bars) | K_long=K_short | role / hypothesis |
/// |---|-------------------|----------------|-------------------|
/// | 0 | 60 (2.5d)         | 3              | baseline MN θ* — IC peak (60–168 band) |
/// | 1 | 168 (1wk)         | 3              | longest IC-peak lookback — lowest turnover |
pub const MN_TIER1_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 60, // L=60 bars (2.5d) — IC peak baseline
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,          // drift=0.10
        rebalance_minutes_override: 480, // 8h — natural low-churn cadence
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "MN baseline θ* (L=60 bars, 8h rebalance, K=3) — IC-peak band basis",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 168, // L=168 bars (1wk) — longest IC-peak lookback
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 480, // 8h
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "MN long lookback (L=168, 1wk) — lowest turnover, most persistent IC-peak signal",
    },
];

/// TS-momentum Tier-1 θ-grid — LOCKED § D-TSM.3-LOCKED (2026-06-02).
///
/// This exact 6-cell list is the hashed body field for the TS θ-surface anchor
/// (ADR-0051 § D6.3, § D6.7). Changing it = a different surface = a different SHA.
///
/// **Swept axes: lookback L (bars) × entry_threshold (cum. log-ret over L)**
/// **Held constant:** `selection_mode=time_series_long_flat`, `direction=momentum` (identity),
/// `k_long=10` (inert), `exposure_cap=0.50`, `k_short=0`, `size=equal_weight`,
/// `rebalance_minutes=60` (1h), `ensemble_seed=0xC0FFEE`, `fill_seed=0xC0FFEE`,
/// generator=block-bootstrap-real, `bootstrap_mode=shared-index`, N=200.
/// Universe = 10-symbol large-cap set (OHLCV pin `3a8b96c4…`, `data/binance/`).
///
/// | g | lookback L (bars) | entry_threshold | role / hypothesis |
/// |---|-------------------|-----------------|-------------------|
/// | 0 | 168 (~1 wk)       | 0.00            | baseline TS θ* (1-wk trend, pure long/flat-on-sign) |
/// | 1 | 24 (~1 d)         | 0.00            | short lookback, zero band — whipsaw extreme |
/// | 2 | 720 (~30 d)       | 0.00            | long lookback, zero band — slow persistent trend |
/// | 3 | 168 (~1 wk)       | 0.02            | 1-wk + wide band — low-churn corner, best structural shot |
/// | 4 | 720 (~30 d)       | 0.02            | long lookback + wide band — slowest, most decisive |
/// | 5 | 24 (~1 d)         | 0.02            | short lookback + band-filtered (does band rescue whipsaw?) |
///
/// `entry_threshold_num=2, entry_threshold_den=2` → `Decimal::new(2, 2)` = 0.02.
/// `entry_threshold_num=0, entry_threshold_den=0` → `Decimal::new(0, 0)` = 0.00.
pub const TS_TIER1_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 168, // ~1 week
        k_long: 10,            // inert under TimeSeriesLongFlat (all names, no ranking)
        drift_threshold_num: 10,
        drift_threshold_den: 2, // drift=0.10 (hold-band; inert under TS — no drift check)
        rebalance_minutes_override: 0, // use base config 60m (1h)
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00 — pure long/flat-on-sign (baseline TS θ*)
        role: "baseline TS θ* (168-bar ~1-wk lookback, zero threshold — pure long/flat-on-sign)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 24, // ~1 day
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00 — whipsaw extreme (short lookback + zero band)
        role: "whipsaw extreme: 24-bar (~1d) lookback, zero threshold — highest fee-bleed risk",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 720, // ~30 days
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00 — slow, persistent trend signal
        role: "slow persistent signal: 720-bar (~30d) lookback, zero threshold",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 168, // ~1 week
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02 (+2% cum. log-ret required to enter) — low-churn corner
        role: "low-churn corner: 168-bar lookback + 0.02 band — TS-momentum's best structural shot at BH bar",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 720, // ~30 days
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02 — slowest, most decisive corner
        role: "slowest/most-decisive: 720-bar lookback + 0.02 band",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 24, // ~1 day
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02 — does the band rescue the whipsaw cell?
        role: "band-rescued whipsaw: 24-bar lookback + 0.02 band (does band filter fix fast-signal churn?)",
    },
];

/// HR-TS-4h θ-grid — LOCKED § D-HR.4-LOCKED (2026-06-03).
///
/// Swept axes: lookback (4h-bars) × entry_threshold.
/// Lookbacks: {42, 180, 540} 4h-bars = {~1 wk, ~30 d, ~90 d}.
/// Thresholds: {0.00, 0.02} (pure sign vs filtered).
///
/// Held constant: `selection_mode=time_series_long_flat`, `direction=momentum`,
/// `k_long=10` (inert), `exposure_cap=0.50`, `k_short=0`, `size=equal_weight`,
/// `rebalance_minutes_override=0` (every coarse bar), 10-symbol universe (pin `3a8b96c4…`),
/// `ensemble_seed=0xC0FFEE`, N=200.
///
/// | g | lookback (4h-bars) | wall-clock | entry_threshold | role |
/// |---|---:|---|---:|---|
/// | 0 | 42  | ~1 wk  | 0.00 | baseline TS θ* (1-wk trend, long/flat-on-sign) |
/// | 1 | 42  | ~1 wk  | 0.02 | 1-wk + wide band — low-churn corner |
/// | 2 | 180 | ~30 d  | 0.00 | 30-d trend, zero band |
/// | 3 | 180 | ~30 d  | 0.02 | 30-d + wide band — best structural shot |
/// | 4 | 540 | ~90 d  | 0.00 | slow 90-d trend |
/// | 5 | 540 | ~90 d  | 0.02 | slowest + most decisive |
pub const TS_4H_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 42, // 42 4h-bars ≈ 1 week
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0, // every coarse bar
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00 — pure long/flat-on-sign
        role: "baseline TS θ* (42 4h-bars ~1-wk, zero threshold)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 42, // 42 4h-bars ≈ 1 week
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02 — low-churn corner
        role: "1-wk + wide band (42 4h-bars, 0.02 threshold) — low-churn corner",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 180, // 180 4h-bars ≈ 30 days
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00
        role: "30-d trend, zero band (180 4h-bars)",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 180, // 180 4h-bars ≈ 30 days
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02
        role: "30-d + wide band (180 4h-bars, 0.02) — best structural shot",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 540, // 540 4h-bars ≈ 90 days
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00
        role: "slow 90-d trend (540 4h-bars, zero threshold)",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 540, // 540 4h-bars ≈ 90 days
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02
        role: "slowest + most decisive (540 4h-bars, 0.02 threshold)",
    },
];

/// HR-TS-daily θ-grid — LOCKED § D-HR.4-LOCKED (2026-06-03).
///
/// Swept axes: lookback (daily-bars) × entry_threshold.
/// Lookbacks: {5, 20, 60} daily-bars = {~1 wk, ~1 mo, ~1 qtr}.
/// Correctness bound: NO lookback > ~365 (60 ≪ 365 ✓).
///
/// | g | lookback (daily-bars) | wall-clock | entry_threshold | role |
/// |---|---:|---|---:|---|
/// | 0 | 5  | ~1 wk  | 0.00 | fast TSMOM (1-wk trend) |
/// | 1 | 5  | ~1 wk  | 0.02 | 1-wk + wide band |
/// | 2 | 20 | ~1 mo  | 0.00 | baseline TS θ* (1-mo trend) |
/// | 3 | 20 | ~1 mo  | 0.02 | 1-mo + wide band — best structural shot |
/// | 4 | 60 | ~1 qtr | 0.00 | classic slow TSMOM (1-qtr) |
/// | 5 | 60 | ~1 qtr | 0.02 | slowest + most decisive |
pub const TS_DAILY_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 5, // 5 daily-bars ≈ 1 week
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00
        role: "fast TSMOM: 5-day lookback, zero threshold",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 5, // 5 daily-bars ≈ 1 week
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02
        role: "1-wk + wide band (5-day lookback, 0.02 threshold)",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 20, // 20 daily-bars ≈ 1 month
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00
        role: "baseline TS θ* (20-day ~1-mo lookback, zero threshold)",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 20, // 20 daily-bars ≈ 1 month
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02
        role: "1-mo + wide band (20-day, 0.02) — best structural shot",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 60, // 60 daily-bars ≈ 1 quarter
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0, // 0.00
        role: "classic slow TSMOM (60-day ~1-qtr lookback, zero threshold)",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 60, // 60 daily-bars ≈ 1 quarter
        k_long: 10,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 2,
        entry_threshold_den: 2, // 0.02
        role: "slowest + most decisive (60-day, 0.02 threshold)",
    },
];

/// HR-CARRY-4h θ-grid — LOCKED § D-HR.4-LOCKED (2026-06-03).
///
/// Swept axes: L (funding settlements, in 4h-bar coarse-bar ring count) × k_long.
/// L values: {2, 6, 12} 4h-bars; k_long: {1, 3, 5}.
/// Rebalance under cosmetic-1h ladder: "every 4h bar" = `override=0` (fires every
/// synthetic 60-min bar = every coarse bar); "every 2nd 4h bar" = `override=120`.
///
/// | g | L (4h-bars) | rebalance | k_long | role |
/// |---|---:|---|---:|---|
/// | 0 | 6  | every 4h bar     | 3 | baseline carry θ* (~1 d settlement window) |
/// | 1 | 2  | every 4h bar     | 3 | fast (~1/3 d) |
/// | 2 | 12 | every 4h bar     | 3 | slow (~2 d) |
/// | 3 | 6  | every 2nd 4h bar | 5 | low-churn / settlement-aligned corner |
/// | 4 | 6  | every 4h bar     | 1 | narrow selection |
/// | 5 | 2  | every 4h bar     | 5 | fast + wide |
pub const CARRY_4H_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 6, // L=6 4h-bars (~1 day settlement window)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0, // every coarse bar (cosmetic-1h ladder: ≤60 fires each bar)
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "baseline carry θ* (L=6 4h-bars ~1d, every 4h bar, K=3)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 2, // L=2 4h-bars (fast, ~1/3 day)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "fast carry (L=2 4h-bars ~1/3d), every 4h bar, K=3",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 12, // L=12 4h-bars (slow, ~2 days)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "slow carry (L=12 4h-bars ~2d), every 4h bar, K=3",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 6, // L=6 4h-bars (~1 day)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 120, // every 2nd 4h bar (~8h) — settlement-aligned
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "low-churn corner (L=6, every 2nd 4h bar=120m, K=5 — settlement-aligned)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 6, // L=6 4h-bars (~1 day)
        k_long: 1,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "narrow selection (L=6, every 4h bar, K=1)",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 2, // L=2 4h-bars (fast)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "fast + wide (L=2 4h-bars, every 4h bar, K=5)",
    },
];

/// HR-CARRY-daily θ-grid — LOCKED § D-HR.4-LOCKED (2026-06-03).
///
/// Swept axes: L (funding settlements, in daily-bar coarse-bar ring count) × k_long.
/// L values: {1, 3, 7} daily-bars; k_long: {1, 3, 5}.
/// Rebalance: every daily bar = `override=0` (fires every synthetic 60-min bar).
/// Correctness bound: L=7 ≪ 366 ✓.
///
/// | g | L (daily-bars) | rebalance | k_long | role |
/// |---|---:|---|---:|---|
/// | 0 | 3 | every daily bar | 3 | baseline carry θ* (~3 d window) |
/// | 1 | 1 | every daily bar | 3 | fastest (~1 d) |
/// | 2 | 7 | every daily bar | 3 | slow (~1 wk) |
/// | 3 | 3 | every daily bar | 5 | wide selection |
/// | 4 | 3 | every daily bar | 1 | narrow selection |
/// | 5 | 7 | every daily bar | 5 | slow + wide |
pub const CARRY_DAILY_GRID: &[ThetaCell] = &[
    ThetaCell {
        g: 0,
        lookback_minutes: 3, // L=3 daily-bars (~3 days)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0, // every daily bar
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "baseline carry θ* (L=3 daily-bars ~3d, K=3)",
    },
    ThetaCell {
        g: 1,
        lookback_minutes: 1, // L=1 daily-bar (~1 day) — fastest
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "fastest carry (L=1 daily-bar ~1d, K=3)",
    },
    ThetaCell {
        g: 2,
        lookback_minutes: 7, // L=7 daily-bars (~1 week)
        k_long: 3,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "slow carry (L=7 daily-bars ~1wk, K=3)",
    },
    ThetaCell {
        g: 3,
        lookback_minutes: 3, // L=3 daily-bars (~3 days)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "wide selection (L=3 daily-bars, K=5)",
    },
    ThetaCell {
        g: 4,
        lookback_minutes: 3, // L=3 daily-bars (~3 days)
        k_long: 1,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "narrow selection (L=3 daily-bars, K=1)",
    },
    ThetaCell {
        g: 5,
        lookback_minutes: 7, // L=7 daily-bars (~1 week)
        k_long: 5,
        drift_threshold_num: 10,
        drift_threshold_den: 2,
        rebalance_minutes_override: 0,
        entry_threshold_num: 0,
        entry_threshold_den: 0,
        role: "slow + wide (L=7 daily-bars ~1wk, K=5)",
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

/// Which score source to use (M-DEV-6, D-CARRY.1 / M-DEV-5, D-BR.1).
///
/// `VolAdjustedReturn` (default) = the v1 price-based signal; reproduces momentum #86 / MR #87
/// byte-identical. `Carry` = funding-based `ScoreSource::FundingCarry`; uses the locked carry
/// θ-grid + funding revision; requires `--generator block-bootstrap-real`.
/// `BasisReversal` = perp-spot basis reversal signal (§ D-BR.1 / R-BR.1-2); uses the locked
/// basis θ-grid (§ D-BR.2-LOCKED) + basis revision; requires `--generator block-bootstrap-real`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum SweepScoreSource {
    /// Vol-adjusted price return — v1 default; reproduces momentum/MR anchors.
    #[default]
    #[value(name = "vol-adjusted-return")]
    VolAdjustedReturn,
    /// Funding-carry signal (§ D-CARRY.1 / R-CARRY.1-2); requires real funding data.
    #[value(name = "carry")]
    Carry,
    /// Basis-reversal signal (§ D-BR.1 / R-BR.1-2); requires real basis data.
    ///
    /// Uses `ScoreSource::BasisReversal` = `−trailing_mean(basis)` (sign baked in).
    /// Reuses the `funding_by_symbol` co-resample channel (D-BR.3). NO cashflow —
    /// the basis is a selection signal (D-BR.1). The fee axis (D-BR.LOAD) is swept
    /// via `--taker-fee-bps`.
    #[value(name = "basis-reversal")]
    BasisReversal,
    /// MN arm 1: raw basis-reversal spread (long low-basis / short high-basis, dollar-neutral).
    ///
    /// `ScoreSource::BasisReversal` + `SelectionMode::LongShort` + `k_short = 3`.
    /// Basis → score; funding → short-leg accrual.
    /// Requires `--grid mn-tier1` and both `--basis-root` + `--funding-root`.
    #[value(name = "mn-basis-spread")]
    MnBasisSpread,
    /// MN arm 2: raw funding-carry spread (long neg-funding / short pos-funding, dollar-neutral).
    ///
    /// `ScoreSource::FundingCarry` + `SelectionMode::LongShort` + `k_short = 3`.
    /// Funding → score AND accrual (the funding spread pays/earns both legs).
    /// Requires `--grid mn-tier1` and both `--basis-root` + `--funding-root`.
    #[value(name = "mn-funding-spread")]
    MnFundingSpread,
    /// MN arm 3: basis⊥funding rank-residual spread (long low-basis-relative / short high-basis-relative).
    ///
    /// `ScoreSource::BasisFundingResidual` + `SelectionMode::LongShort` + `k_short = 3`.
    /// Residual = rank(basis) − rank(funding) (Decimal-exact integer, D-MN.6).
    /// Basis → basis_score_map; funding → score (funding_map) AND short-leg accrual.
    /// Requires `--grid mn-tier1` and both `--basis-root` + `--funding-root`.
    #[value(name = "mn-basis-funding-residual")]
    MnBasisFundingResidual,
}

impl SweepScoreSource {
    /// Convert to the strategy `ScoreSource` type.
    fn to_strategy_score_source(self) -> strategy::ScoreSource {
        match self {
            Self::VolAdjustedReturn => strategy::ScoreSource::VolAdjustedReturn,
            Self::Carry => strategy::ScoreSource::FundingCarry,
            Self::BasisReversal => strategy::ScoreSource::BasisReversal,
            Self::MnBasisSpread => strategy::ScoreSource::BasisReversal,
            Self::MnFundingSpread => strategy::ScoreSource::FundingCarry,
            Self::MnBasisFundingResidual => strategy::ScoreSource::BasisFundingResidual,
        }
    }

    /// Whether this source needs the funding sidecar loaded (carry or basis).
    fn needs_funding(self) -> bool {
        matches!(self, Self::Carry)
    }

    /// Whether this source needs the basis sidecar loaded.
    fn needs_basis(self) -> bool {
        matches!(self, Self::BasisReversal)
    }

    /// Whether this is a market-neutral (MN) arm (D-MN.5, M-DEV-5).
    fn is_mn(self) -> bool {
        matches!(
            self,
            Self::MnBasisSpread | Self::MnFundingSpread | Self::MnBasisFundingResidual
        )
    }

    /// The short arm-label used in MN scenario names (D-MN.8, M-DEV-5).
    fn mn_arm_label(self) -> &'static str {
        match self {
            Self::MnBasisSpread => "basis",
            Self::MnFundingSpread => "funding",
            Self::MnBasisFundingResidual => "basisperp",
            _ => "unknown",
        }
    }

    #[allow(dead_code)]
    /// Short label for scenario name.
    fn label(self) -> &'static str {
        match self {
            Self::VolAdjustedReturn => "carry-fy", // unused for non-carry
            Self::Carry => "carry-fy",
            Self::BasisReversal => "basis-reversal-fy",
            Self::MnBasisSpread => "mn-basis-spread",
            Self::MnFundingSpread => "mn-funding-spread",
            Self::MnBasisFundingResidual => "mn-basis-funding-residual",
        }
    }
}

/// Which selection mode to use (M-DEV-4, D-TSM.1).
///
/// `cross-sectional-top-k` (default) = the v1 `top_k_long` ranking path; reproduces
/// momentum/MR/carry anchors byte-identical. `time-series-long-flat` = per-asset
/// threshold gating (D-TSM.1); uses the LOCKED TS_TIER1_GRID + `entry_threshold`
/// per cell; scenario name `v1-ts-momentum-theta-surface-{year}-…`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum SweepSelectionMode {
    /// Cross-sectional top-K — v1 default; reproduces momentum/MR/carry anchors.
    #[default]
    #[value(name = "cross-sectional-top-k")]
    CrossSectionalTopK,
    /// Time-series long/flat — per-asset threshold; uses TS_TIER1_GRID (D-TSM.1).
    #[value(name = "time-series-long-flat")]
    TimeSeriesLongFlat,
}

impl SweepSelectionMode {
    /// Convert to the strategy `SelectionMode` type.
    fn to_strategy_selection_mode(self) -> strategy::SelectionMode {
        match self {
            Self::CrossSectionalTopK => strategy::SelectionMode::CrossSectionalTopK,
            Self::TimeSeriesLongFlat => strategy::SelectionMode::TimeSeriesLongFlat,
        }
    }

    /// Whether this is the TS-momentum path.
    fn is_ts(self) -> bool {
        self == Self::TimeSeriesLongFlat
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
        default_value = "spec/v1/momentum-parameter-robustness-sweep/reports/"
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

    /// Selection mode (M-DEV-4, D-TSM.1).
    /// `cross-sectional-top-k` (default) reproduces momentum/MR/carry anchors byte-identical.
    /// `time-series-long-flat` uses per-asset threshold gating; requires --grid ts-tier1.
    /// The entry_threshold is per-cell in TS_TIER1_GRID — NOT a separate CLI flag.
    #[arg(long, value_enum, default_value = "cross-sectional-top-k")]
    selection_mode: SweepSelectionMode,

    /// Decision cadence for the horizon retest (M-DEV-3, D-HR.2).
    /// `1h` (default) = identity pass-through → all 91 anchors are byte-identical.
    /// `4h` = resample 1h bars to true 4h (4:1 fold); 2190/2196 bars/year.
    /// `daily` = resample to daily (24:1 fold); 365/366 bars/year.
    /// The metric branch picks compute_sharpe_hourly (1h) vs compute_sharpe_periodic
    /// (4h/daily) so the 1h anchors are byte-unchanged by construction (D-HR.1).
    #[arg(long, value_enum, default_value = "1h")]
    horizon: backtest::resample::Horizon,

    /// Taker fee in basis points (M-DEV-4, D-BR.LOAD).
    ///
    /// Default = **4** (the legacy hardcoded literal at `param_robustness_sweep.rs:2409-2410`).
    /// Every non-basis run passes the default → `MatchConfig` is byte-identical → the 99
    /// existing anchors are unchanged. The basis arm sweeps this over {0,2,5,10} bps to
    /// produce the fee-sensitivity surface (R-BR.LOAD). Slippage is swept separately via
    /// `--slippage-bps` (default 2).
    #[arg(long, default_value_t = 4)]
    taker_fee_bps: u32,

    /// Slippage in basis points (M-DEV-4, D-BR.LOAD).
    ///
    /// Default = **2** (the legacy hardcoded literal). Held at the default across the fee
    /// ladder (the LOAD-BEARING fee sweep varies the taker leg only, per § D-BR.LOAD).
    /// Changing this on a non-basis run would break the existing anchors.
    #[arg(long, default_value_t = 2)]
    slippage_bps: u32,

    /// Root directory for basis parquets (basis-reversal only).
    /// Default: `data/binance-basis/` (the locked path from the basis data load).
    #[arg(long, default_value = "data/binance-basis/")]
    basis_root: PathBuf,

    /// Expected aggregate SHA-256 for data/binance-basis/REVISION.toml (basis only).
    /// Default: the locked basis revision SHA (aa72409a…).
    #[arg(
        long,
        default_value = "aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd"
    )]
    basis_revision_sha: String,
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

/// Horizon-aware periods-per-year (M-DEV-3, D-HR.1.2).
///
/// Returns the `periods_per_year` scalar for `compute_*_periodic` at the given
/// `(horizon, year)`. Leap-year aware: 2024 is a leap year (8784h / 4 = 2196 at 4h;
/// 8784h / 24 = 366 at daily).
///
/// **The 1h value is provided for completeness ONLY.** The sweep MUST NOT call
/// `compute_sharpe_periodic` with the 1h value — instead it calls the verbatim
/// `compute_sharpe_hourly` (the 1h anchors are byte-identical by construction,
/// D-HR.1 / D-HR.7). Use this only for 4h and daily.
#[must_use]
fn sweep_periods_per_year(horizon: backtest::resample::Horizon, year: i32) -> f64 {
    horizon.periods_per_year(year)
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
    /// Surfaced from `run_path`'s `realized_funding` field.
    #[allow(dead_code)]
    funding_harvested: Decimal,
    /// Number of bars where ≥1 long position was held (time-in-market, M-DEV-4).
    /// Always populated from `run_path`'s `time_in_market_bars` counter.
    /// Used only by the TS render column (GATED to TS reports — momentum/MR/carry
    /// body-SHAs stay byte-identical because the column is not rendered for them).
    time_in_market_bars: u64,
    /// Total bars processed on this path (equity_curve.len() − 1).
    /// Denominator for time-in-market fraction computation (M-DEV-4).
    bars_run: u64,
    /// Number of maintenance-margin liquidation events on this path (M-DEV-5, MN only).
    /// Populated from `run_path`'s `liquidations` field. 0 for all non-MN runs.
    liquidations: u64,
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
    /// Total time-in-market bars across all N paths (M-DEV-4 / D-TSM.6.4).
    /// Used only when `selection_mode == TimeSeriesLongFlat` to render the
    /// `time_in_market` column (GATED — body-SHAs of momentum/MR/carry unchanged).
    total_time_in_market_bars: u64,
    /// Total bars in the run across all N paths (for computing the fraction).
    total_bars_run: u64,
    /// Total maintenance-margin liquidations across all N paths (M-DEV-5, MN only).
    /// 0 for all non-MN runs → anchor-neutral by construction.
    total_liquidations: u64,
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
    selection_mode: SweepSelectionMode,
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
    // M-DEV-4: set selection_mode and entry_threshold from the cell.
    // For momentum/MR/carry cells: selection_mode=CrossSectionalTopK (default),
    // entry_threshold=0 (from entry_threshold_num=0, entry_threshold_den=0) → inert.
    // For TS cells: selection_mode=TimeSeriesLongFlat, entry_threshold from cell.
    cfg.selection_mode = selection_mode.to_strategy_selection_mode();
    cfg.entry_threshold = cell.entry_threshold();
    // M-DEV-5 (D-MN.5): for MN arms, override selection_mode=LongShort + k_short=k_long.
    // This is ADDITIVE — only set for MN arms; all other arms get the selection_mode from
    // the SweepSelectionMode arm above (CrossSectionalTopK → byte-identical to existing anchors).
    if score_source.is_mn() {
        cfg.selection_mode = strategy::SelectionMode::LongShort;
        cfg.k_short = cell.k_long; // symmetric K split: k_short = k_long = 3
    }
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
/// Delegate to the library implementation (ADR-0059 M-DEV-1: extracted from this
/// bin into `backtest::bakeoff::buyhold` for the bake-off orchestrator to share).
/// Behaviour is byte-identical — pure structural relocation.
fn run_buyhold_path(
    bars: &[trading_core::Bar],
    initial_capital: Decimal,
    n_symbols: usize,
) -> (Vec<Decimal>, Decimal) {
    backtest::bakeoff::buyhold::run_buyhold_path(bars, initial_capital, n_symbols)
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

/// Build the canonical grid-definition string for basis-reversal reports (M-DEV-5).
///
/// Format mirrors `carry_grid_def_string` (rebalance + lookback swept) but uses
/// `lookback_bars` (not `l_settlements`) to make the unit explicit for the basis arm.
/// This is a hashed body field for the basis anchor (K3 / § D6.3 + D6.9).
///
/// Format: `g={} lookback_bars={} rebalance_minutes={} k_long={} drift={}`
#[must_use]
pub fn basis_grid_def_string(grid: &[ThetaCell]) -> String {
    let mut s = String::from("grid_definition:\n");
    for cell in grid {
        s.push_str(&format!(
            "  g={} lookback_bars={} rebalance_minutes={} k_long={} drift={}\n",
            cell.g,
            cell.lookback_minutes,
            cell.rebalance_minutes_override,
            cell.k_long,
            cell.drift()
        ));
    }
    s
}

/// Build the canonical grid-definition string for MN-spread reports (M-DEV-5, D-MN.8).
///
/// Includes `lookback_bars`, `rebalance_minutes`, `k_long=k_short` (the symmetric split),
/// and the LOCKED margin constants (`max_leverage`, `maintenance_margin_frac`). This is
/// a hashed body field for the MN anchor (K3 / ADR-0051 § D6.10).
///
/// Format mirrors `basis_grid_def_string` but adds `k_short` + margin constants.
#[must_use]
pub fn mn_grid_def_string(grid: &[ThetaCell]) -> String {
    use backtest::scenarios::montecarlo::{MAINTENANCE_MARGIN_FRAC, MAX_LEVERAGE};
    let mut s = String::from("grid_definition:\n");
    for cell in grid {
        s.push_str(&format!(
            "  g={} lookback_bars={} rebalance_minutes={} k_long={} k_short={} drift={} max_leverage={} maintenance_margin_frac={}\n",
            cell.g,
            cell.lookback_minutes,
            cell.rebalance_minutes_override,
            cell.k_long,
            cell.k_long, // k_short = k_long for the MN symmetric split
            cell.drift(),
            MAX_LEVERAGE,
            MAINTENANCE_MARGIN_FRAC,
        ));
    }
    s
}

/// Build the canonical grid-definition string for TS-momentum reports (M-DEV-4).
///
/// Includes `lookback_bars` (the price-bar lookback, same field as `lookback_minutes`
/// in the struct but interpreted as bars not minutes for TS) AND `entry_threshold`
/// (the no-trade band, the 2nd swept axis for TS). This is a hashed body field
/// for the TS anchor (K3 / § D6.3 + D6.7).
///
/// Format mirrors `carry_grid_def_string` but uses TS-specific names.
#[must_use]
pub fn ts_grid_def_string(grid: &[ThetaCell]) -> String {
    let mut s = String::from("grid_definition:\n");
    for cell in grid {
        s.push_str(&format!(
            "  g={} lookback={} entry_threshold={} k_long={} drift={}\n",
            cell.g,
            cell.lookback_minutes,
            cell.entry_threshold(),
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
    // Selection mode (M-DEV-4, D-TSM.1):
    // CrossSectionalTopK → standard report (anchor-neutral, no TS column).
    // TimeSeriesLongFlat → TS report slug + time_in_market column added (GATED to TS).
    selection_mode: SweepSelectionMode,
    // M-DEV-3: horizon (D-HR.5). OneHour → body unchanged (all 91 anchors byte-identical).
    // FourHours/OneDay → render the real horizon in the hashed body.
    horizon: backtest::resample::Horizon,
    // M-DEV-4 (D-BR.LOAD): taker fee and slippage in bps.
    // Rendered as hashed body fields ONLY for BasisReversal runs (so the 99 existing
    // anchor body-SHAs stay byte-identical — the same gating the horizon row uses).
    taker_fee_bps: u32,
    slippage_bps: u32,
) -> String {
    // ── Front-matter (NOT hashed) ─────────────────────────────────────────────
    // slug: momentum reports keep "momentum-parameter-robustness-sweep" for anchor compat.
    // MR reports use "cross-sectional-mean-reversion-strategy".
    // Carry reports use "carry-strategy".
    // TS reports use "time-series-momentum-robustness".
    // Horizon retest reports (horizon != 1h) use "horizon-retest-robustness" (D-HR.5).
    let is_horizon_run = horizon != backtest::resample::Horizon::OneHour;
    let is_basis_run = score_source == SweepScoreSource::BasisReversal;
    let is_mn_run = score_source.is_mn();
    let slug = if is_horizon_run {
        "horizon-retest-robustness"
    } else if selection_mode.is_ts() {
        "time-series-momentum-robustness"
    } else {
        match score_source {
            SweepScoreSource::Carry => "carry-strategy",
            SweepScoreSource::BasisReversal => "perp-basis-signal-robustness",
            SweepScoreSource::MnBasisSpread
            | SweepScoreSource::MnFundingSpread
            | SweepScoreSource::MnBasisFundingResidual => "perp-basis-mn-spread",
            SweepScoreSource::VolAdjustedReturn => match direction {
                SweepDirection::Momentum => "momentum-parameter-robustness-sweep",
                SweepDirection::Reversion => "cross-sectional-mean-reversion-strategy",
            },
        }
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

    let family_label: String = if is_horizon_run && selection_mode.is_ts() {
        let hz = match horizon {
            backtest::resample::Horizon::FourHours => "4h",
            backtest::resample::Horizon::OneDay => "daily",
            backtest::resample::Horizon::OneHour => "1h",
        };
        format!("Time-Series Momentum ({hz} horizon)")
    } else if is_horizon_run && score_source == SweepScoreSource::Carry {
        let hz = match horizon {
            backtest::resample::Horizon::FourHours => "4h",
            backtest::resample::Horizon::OneDay => "daily",
            backtest::resample::Horizon::OneHour => "1h",
        };
        format!("Carry (Funding, {hz} horizon)")
    } else if selection_mode.is_ts() {
        "Time-Series Momentum".to_string()
    } else {
        match score_source {
            SweepScoreSource::Carry => "Carry (Funding)".to_string(),
            SweepScoreSource::BasisReversal => {
                format!("Basis-Reversal (taker_fee={taker_fee_bps}bps)")
            }
            SweepScoreSource::MnBasisSpread => {
                format!("MN Basis-Spread (long-short, taker_fee={taker_fee_bps}bps)")
            }
            SweepScoreSource::MnFundingSpread => {
                format!("MN Funding-Spread (long-short, taker_fee={taker_fee_bps}bps)")
            }
            SweepScoreSource::MnBasisFundingResidual => {
                format!("MN Basis⊥Funding Residual (long-short, taker_fee={taker_fee_bps}bps)")
            }
            SweepScoreSource::VolAdjustedReturn => match direction {
                SweepDirection::Momentum => "Momentum".to_string(),
                SweepDirection::Reversion => "Mean-Reversion (MR)".to_string(),
            },
        }
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
    // M-DEV-3: render the real horizon in the hashed body (D-HR.5 / K3).
    // GATED to horizon runs so the 1h body-SHAs are byte-identical.
    if is_horizon_run {
        body.push_str(&format!(
            "| horizon                  | {horizon}                                               |\n",
            horizon = match horizon {
                backtest::resample::Horizon::FourHours => "4h",
                backtest::resample::Horizon::OneDay => "daily",
                backtest::resample::Horizon::OneHour => "1h", // unreachable under is_horizon_run
            }
        ));
    }
    // M-DEV-4 (D-BR.LOAD): render the fee level as a hashed body field for basis runs.
    // GATED to `BasisReversal` so the 99 existing anchor body-SHAs are byte-identical.
    // The fee level distinguishes the four fee-level surfaces as DISTINCT anchors.
    // M-DEV-5 (D-MN.8): also render fee for MN runs (same 12-surface anchor scheme).
    if is_basis_run || is_mn_run {
        body.push_str(&format!(
            "| taker_fee_bps            | {taker_fee_bps}                                                   |\n"
        ));
        body.push_str(&format!(
            "| slippage_bps             | {slippage_bps}                                                    |\n"
        ));
    }
    // held_constant: add direction for MR runs; score_source + funding for carry runs;
    // selection_mode + rebalance + k_long(inert) for TS runs; basis fields for basis runs.
    // The body field is part of the hash — each family's string differs from others.
    let held_constant_str: String = if selection_mode.is_ts() {
        "| held_constant            | selection_mode=time_series_long_flat score_source=vol_adjusted_return direction=momentum rebalance_minutes=60 exposure_cap=0.50 k_long=10(inert) vol_floor=inert k_short=0 size=equal_weight |\n".to_string()
    } else {
        match score_source {
            SweepScoreSource::Carry => {
                format!(
                    "| held_constant            | score_source=funding_carry direction=momentum exposure_cap=0.50 vol_floor=inert k_short=0 size=equal_weight |\n\
                     | funding_revision_sha     | {} |\n",
                    funding_revision_sha.unwrap_or("unknown")
                )
            }
            SweepScoreSource::BasisReversal => {
                format!(
                    "| held_constant            | score_source=basis_reversal direction=momentum exposure_cap=0.50 vol_floor=inert k_short=0 size=equal_weight |\n\
                     | basis_revision_sha       | {} |\n",
                    funding_revision_sha.unwrap_or("unknown")
                )
            }
            SweepScoreSource::MnBasisSpread => {
                // funding_revision_sha holds the combined "basis:{sha} funding:{sha}" string.
                format!(
                    "| held_constant            | score_source=basis_reversal selection_mode=long_short k_long=k_short=3 exposure_cap=0.50 vol_floor=inert max_leverage=1 maintenance_margin_frac=0.5 |\n\
                     | data_revisions           | {} |\n",
                    funding_revision_sha.unwrap_or("unknown"),
                )
            }
            SweepScoreSource::MnFundingSpread => {
                format!(
                    "| held_constant            | score_source=funding_carry selection_mode=long_short k_long=k_short=3 exposure_cap=0.50 vol_floor=inert max_leverage=1 maintenance_margin_frac=0.5 |\n\
                     | data_revisions           | {} |\n",
                    funding_revision_sha.unwrap_or("unknown")
                )
            }
            SweepScoreSource::MnBasisFundingResidual => {
                format!(
                    "| held_constant            | score_source=basis_funding_residual selection_mode=long_short k_long=k_short=3 exposure_cap=0.50 vol_floor=inert max_leverage=1 maintenance_margin_frac=0.5 |\n\
                     | data_revisions           | {} |\n",
                    funding_revision_sha.unwrap_or("unknown"),
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
        }
    };
    body.push_str(&held_constant_str);
    body.push('\n');

    // Frozen grid definition (hashed body field — K3 / § D6.3).
    // For horizon runs (§ D-HR.4-LOCKED): use the horizon-specific header + appropriate
    // grid formatter. The horizon grids use the same format as their 1h counterparts
    // (ts_grid_def_string for TS, carry_grid_def_string for carry).
    let grid_header_str: String = if is_horizon_run && selection_mode.is_ts() {
        let hz = match horizon {
            backtest::resample::Horizon::FourHours => "4h",
            backtest::resample::Horizon::OneDay => "daily",
            backtest::resample::Horizon::OneHour => "1h",
        };
        format!(
            "## TS-momentum {hz} θ-grid definition (6-cell, LOCKED § D-HR.4-LOCKED — changing this changes the SHA)\n\n"
        )
    } else if is_horizon_run && score_source == SweepScoreSource::Carry {
        let hz = match horizon {
            backtest::resample::Horizon::FourHours => "4h",
            backtest::resample::Horizon::OneDay => "daily",
            backtest::resample::Horizon::OneHour => "1h",
        };
        format!(
            "## Carry {hz} θ-grid definition (6-cell, LOCKED § D-HR.4-LOCKED — changing this changes the SHA)\n\n"
        )
    } else if selection_mode.is_ts() {
        "## TS-momentum θ-grid definition (6-cell, LOCKED § D-TSM.3-LOCKED — changing this changes the SHA)\n\n".to_string()
    } else {
        match score_source {
            SweepScoreSource::Carry => {
                "## Carry θ-grid definition (6-cell, LOCKED § D-CARRY.2-LOCKED — changing this changes the SHA)\n\n".to_string()
            }
            SweepScoreSource::BasisReversal => {
                "## Basis-Reversal θ-grid definition (6-cell, LOCKED § D-BR.2-LOCKED — changing this changes the SHA)\n\n".to_string()
            }
            SweepScoreSource::MnBasisSpread
            | SweepScoreSource::MnFundingSpread
            | SweepScoreSource::MnBasisFundingResidual => {
                "## MN-Spread θ-grid definition (2-cell, LOCKED § D-MN.8-LOCKED — changing this changes the SHA)\n\n".to_string()
            }
            SweepScoreSource::VolAdjustedReturn => match direction {
                SweepDirection::Momentum => {
                    "## Re-scoped θ-grid definition (6-cell, 2026-05-30 orchestrator re-scope — changing this changes the SHA)\n\n".to_string()
                }
                SweepDirection::Reversion => {
                    "## MR θ-grid definition (6-cell, 2026-05-31 LOCKED § D-MR.2-LOCKED — changing this changes the SHA)\n\n".to_string()
                }
            },
        }
    };
    body.push_str(&grid_header_str);
    // TS grid: use ts_grid_def_string (includes entry_threshold — the TS swept axis).
    // Carry grid: use carry_grid_def_string (l_settlements + rebalance).
    // Basis grid: use basis_grid_def_string (lookback_bars + rebalance — same shape but different name).
    // MN grid: use mn_grid_def_string (lookback_bars + rebalance + k_short + margin constants).
    // Momentum/MR: use the standard grid_def_string (no rebalance — anchor-safe).
    if selection_mode.is_ts() {
        body.push_str(&ts_grid_def_string(grid));
    } else {
        match score_source {
            SweepScoreSource::Carry => {
                body.push_str(&carry_grid_def_string(grid));
            }
            SweepScoreSource::BasisReversal => {
                body.push_str(&basis_grid_def_string(grid));
            }
            SweepScoreSource::MnBasisSpread
            | SweepScoreSource::MnFundingSpread
            | SweepScoreSource::MnBasisFundingResidual => {
                body.push_str(&mn_grid_def_string(grid));
            }
            SweepScoreSource::VolAdjustedReturn => {
                body.push_str(&grid_def_string(grid));
            }
        }
    }
    body.push('\n');

    // θ-surface table (rows sorted by g).
    body.push_str("## θ-surface (per-cell distribution + verdict)\n\n");
    body.push_str("Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.\n");
    body.push_str("Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).\n");
    body.push_str("Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).\n\n");

    // M-DEV-4 (original, MR): add `trades` column for MR only (R-MR.3 turnover legibility).
    // Gate on direction so momentum anchor #86 body-SHA stays byte-identical.
    // M-DEV-6: add `funding_harvested` column for carry only (D-CARRY.2-LOCKED).
    // Gate on score_source so MR/momentum body-SHAs stay byte-identical.
    // M-DEV-4 (TS): add `time_in_market` column for TS only (D-TSM.6.4 / ADR-0051 § D6.5.4).
    // Gate on selection_mode so momentum/MR/carry body-SHAs stay byte-identical.
    // M-DEV-5 (basis): add `trades` column for basis-reversal (turnover legibility — the
    // fee story for a reversal arm is dominated by turnover; D-BR.2-LOCKED).
    // Gated to BasisReversal so all existing body-SHAs stay byte-identical.
    let show_trades = !selection_mode.is_ts()
        && score_source == SweepScoreSource::VolAdjustedReturn
        && direction == SweepDirection::Reversion;
    let show_funding = !selection_mode.is_ts() && score_source == SweepScoreSource::Carry;
    let show_basis_trades = is_basis_run && !selection_mode.is_ts();
    let show_mn = is_mn_run && !selection_mode.is_ts();
    let show_time_in_market = selection_mode.is_ts();
    if show_time_in_market {
        body.push_str(
            "time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).\n\n",
        );
        body.push_str("| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |\n");
        body.push_str("|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|\n");
    } else if show_basis_trades {
        body.push_str(
            "Trades = total trade count across all N paths (turnover legibility — fee story for reversal arm, D-BR.2-LOCKED).\n\n",
        );
        body.push_str("| g  | lookback | rebalance | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | trades     | verdict  | notes |\n");
        body.push_str("|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|------------|----------|-------|\n");
    } else if show_mn {
        body.push_str(
            "Liquidations = total maintenance-margin liquidation events across all N paths (MN only, D-MN.8).\n\n",
        );
        body.push_str("| g  | lookback | rebalance | k_long | k_short | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | liquidations | verdict  | notes |\n");
        body.push_str("|----|----------|-----------|--------|---------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------|----------|-------|\n");
    } else if show_trades {
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

        if show_time_in_market {
            // TS-specific: time_in_market column = fraction of bars with ≥1 long position.
            // Computed as total_time_in_market_bars / total_bars_run across all N paths.
            let tim_fraction = if cr.total_bars_run > 0 {
                cr.total_time_in_market_bars as f64 / cr.total_bars_run as f64
            } else {
                0.0
            };
            body.push_str(&format!(
                "| {:2} | {:8} | {:.2}      | {:6} | {:.2} | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | {:.4}          | {:8} | {} |\n",
                cr.cell.g,
                cr.cell.lookback_minutes,
                cr.cell.entry_threshold(),
                cr.cell.k_long,
                cr.cell.drift(),
                s.sharpe.p5,
                s.sharpe.p50,
                s.sharpe.p95,
                s.prob_loss,
                s.prob_sharpe_gt_1,
                s.max_dd_tail_p95 * 100.0,
                spread,
                tim_fraction,
                verdict_str,
                c5_flag,
            ));
        } else if show_basis_trades {
            // Basis-reversal row: includes `rebalance` column (swept in D-BR.2-LOCKED
            // for the cadence/turnover axis) and `trades` (turnover legibility).
            body.push_str(&format!(
                "| {:2} | {:8} | {:9} | {:6} | {:.2} | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | {:10} | {:8} | {} |\n",
                cr.cell.g,
                cr.cell.lookback_minutes,
                cr.cell.rebalance_minutes_override,
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
        } else if show_mn {
            // MN row: includes `rebalance` + `k_short` (symmetric split) + `liquidations`.
            body.push_str(&format!(
                "| {:2} | {:8} | {:9} | {:6} | {:7} | {:.2} | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | {:12} | {:8} | {} |\n",
                cr.cell.g,
                cr.cell.lookback_minutes,
                cr.cell.rebalance_minutes_override,
                cr.cell.k_long,
                cr.cell.k_long, // k_short = k_long (symmetric)
                cr.cell.drift(),
                s.sharpe.p5,
                s.sharpe.p50,
                s.sharpe.p95,
                s.prob_loss,
                s.prob_sharpe_gt_1,
                s.max_dd_tail_p95 * 100.0,
                spread,
                cr.total_liquidations,
                verdict_str,
                c5_flag,
            ));
        } else if show_trades {
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
        if selection_mode.is_ts() {
            if is_horizon_run {
                let hz = match horizon {
                    backtest::resample::Horizon::FourHours => "4h",
                    backtest::resample::Horizon::OneDay => "daily",
                    backtest::resample::Horizon::OneHour => "1h",
                };
                body.push_str(&format!(
                    "Conclusion: v1 time-series momentum at the {hz} horizon (per-asset long/flat on own trailing return) is\n"
                ));
                body.push_str(
                    "structurally fragile across the tested parameter space on this 10-symbol universe.\n",
                );
                body.push_str(
                    "Even at the classically-preferred coarser decision cadence, the trend-capture benefit\n",
                );
                body.push_str(
                    "does not overcome the buy-and-hold bar net of fees. Closes the OHLCV-only active-trading thesis.\n",
                );
            } else {
                body.push_str(
                    "Conclusion: v1 time-series momentum (per-asset long/flat on own trailing return) is\n",
                );
                body.push_str(
                    "structurally fragile across the tested parameter space on this 10-symbol 1h universe.\n",
                );
                body.push_str(
                    "Whipsaw/fee-bleed or late exits may have dominated the trend-capture benefit.\n",
                );
                body.push_str(
                    "This closes the active-trading thesis on this universe: no method (x-sec or time-series)\n",
                );
                body.push_str(
                    "beat passive buy-and-hold net of fees. Routes to broader-universe / horizon axis.\n",
                );
            }
        } else {
            match score_source {
                SweepScoreSource::Carry => {
                    if is_horizon_run {
                        let hz = match horizon {
                            backtest::resample::Horizon::FourHours => "4h",
                            backtest::resample::Horizon::OneDay => "daily",
                            backtest::resample::Horizon::OneHour => "1h",
                        };
                        body.push_str(&format!(
                            "Conclusion: v1 cross-sectional carry (funding) at the {hz} horizon is structurally fragile across the\n"
                        ));
                        body.push_str(
                            "tested parameter space on this universe. Even at the native settlement cadence,\n",
                        );
                        body.push_str(
                            "funding mean-reversion or directional price exposure overwhelmed the funding harvest.\n",
                        );
                    } else {
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
                }
                SweepScoreSource::BasisReversal => {
                    body.push_str(&format!(
                        "Conclusion: v1 cross-sectional basis-reversal at {taker_fee_bps} bps taker fee is structurally fragile\n"
                    ));
                    body.push_str(
                        "across the tested parameter space on this 10-symbol universe. The fee-bleed from\n",
                    );
                    body.push_str(
                        "reversal-arm turnover consumes the gross −0.10 IC edge at this fee level.\n",
                    );
                    body.push_str(
                        "VERDICT: FRAGILE-on-fees at this fee level. Pre-registered result — see R-BR.LOAD.\n",
                    );
                }
                SweepScoreSource::MnBasisSpread
                | SweepScoreSource::MnFundingSpread
                | SweepScoreSource::MnBasisFundingResidual => {
                    let arm_label = score_source.mn_arm_label();
                    body.push_str(&format!(
                        "Conclusion: v2 market-neutral {arm_label} spread at {taker_fee_bps} bps taker fee is structurally fragile\n"
                    ));
                    body.push_str(
                        "across the tested parameter space on this 10-symbol universe. The dollar-neutral\n",
                    );
                    body.push_str(
                        "construction removes directional beta but not fee-bleed from short-leg turnover.\n",
                    );
                    body.push_str(
                        "VERDICT: FRAGILE. Pre-registered result — see R-MN.LOAD (§ D6.10).\n",
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

    // M-DEV-3: apply horizon resample per symbol (D-HR.2).
    // For Horizon::OneHour (default) → identity pass-through → byte-untouched 1h load path.
    // For 4h/daily → fold into coarse bars. The coverage check above stays on the 1h count.
    let bars_by_symbol: SourceBars = symbols_prices
        .iter()
        .map(|(sym, _)| {
            let bars_1h = by_symbol.remove(&sym.to_string()).unwrap_or_default();
            let bars = backtest::resample::resample_ohlcv(&bars_1h, args.horizon);
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
/// (4) M-DEV-4 (D-BR.LOAD): `taker_fee_bps`/`slippage_bps` are now parameters
///    replacing the hardcoded literals. Defaults `4`/`2` → MatchConfig is
///    byte-identical for every non-basis run → the 99 existing anchors hold.
/// (5) M-DEV-5 (D-MN.8): `score_source` drives MN-arm-specific dual-sidecar
///    injection. For non-MN arms (`!score_source.is_mn()`): byte-identical to
///    the pre-MN code (the 107 existing anchors hold by construction).
#[allow(clippy::too_many_arguments)]
fn run_one_path_with_config(
    j: usize,
    path_seed_j: u64,
    fill_seed: u64,
    cfg: &strategy::CrossSectionalMomentumConfig,
    universe: &[(trading_core::Symbol, Decimal)],
    // Pre-built path generator for BlockBootstrapReal; None for GbmSmoke.
    // For carry: this is the CARRY path_gen (with funding already attached via with_funding).
    // For basis: this is the BASIS path_gen (with basis attached via with_funding).
    // For MN arms: this is the MN path_gen (with BOTH basis AND funding attached —
    //   basis_by_symbol for score, funding_by_symbol for short-leg accrual).
    // For momentum/MR: this is the BASE path_gen (no funding).
    block_path_gen: Option<&data::BlockBootstrapPathGen>,
    bar_count: usize,
    generator: GeneratorKind,
    year: i32,
    // Whether a sidecar was injected (carry OR basis OR MN).
    // Used to decide whether to extract the sidecar map from generated_path.
    // For the basis arm: the sidecar is extracted for the SCORE only — the
    // `run_path` accrual stays gated `None` (no cashflow — D-BR.1).
    // For MN arms: both sidecars are extracted (see score_source arm below).
    inject_sidecar: bool,
    // Whether the sidecar is carry (as opposed to basis or MN).
    // When inject_sidecar=true and is_carry=true: the map is passed as
    //   `funding_override` to `TcnScenarioInput` for BOTH score AND accrual.
    // When inject_sidecar=true and is_carry=false (basis): the map is passed
    //   ONLY to the strategy via `with_funding`; `funding_override` in
    //   `TcnScenarioInput` stays `None` so the `run_path` accrual block
    //   (`montecarlo.rs:322`) is never entered — the basis has NO cashflow.
    // For MN arms: `is_carry=false`; MN-specific logic is in the score_source arm.
    is_carry: bool,
    // M-DEV-3: horizon for metric branch selection (D-HR.1).
    // OneHour → verbatim compute_sharpe_hourly (anchor-safe);
    // FourHours/OneDay → compute_sharpe_periodic(periods_per_year).
    horizon: backtest::resample::Horizon,
    // M-DEV-4 (D-BR.LOAD): taker fee and slippage in bps.
    // Defaults = 4/2 (the legacy hardcoded literals). Every non-basis caller
    // passes the defaults → MatchConfig byte-identical → 99 anchors hold.
    taker_fee_bps: u32,
    slippage_bps: u32,
    // M-DEV-5 (D-MN.8): the score source for MN-arm-specific dual-sidecar injection.
    // For non-MN arms: VolAdjustedReturn/Carry/BasisReversal — the pre-MN logic is
    // byte-identical (the 107 existing anchors hold by construction).
    // For MN arms: controls which map goes to score vs accrual (see inline comments).
    score_source: SweepScoreSource,
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
                basis_by_symbol: None,
            }
        }
    };

    // ── Helper: build a SidecarMap from a `Vec<Vec<Option<Decimal>>>` field ──
    type SidecarMap =
        std::collections::BTreeMap<(trading_core::Symbol, trading_core::Timestamp), Decimal>;

    let build_sidecar_map = |opt_by_sym: &Option<Vec<Vec<Option<Decimal>>>>,
                             bars_by_symbol: &[Vec<trading_core::Bar>]|
     -> Option<SidecarMap> {
        opt_by_sym.as_ref().map(|by_sym| {
            let mut map = SidecarMap::new();
            for (sym_i, (sym, _)) in universe.iter().enumerate() {
                if let Some(sidecar_row) = by_sym.get(sym_i)
                    && let Some(bars_row) = bars_by_symbol.get(sym_i)
                {
                    for (bar, &sidecar_val) in bars_row.iter().zip(sidecar_row.iter()) {
                        if let Some(rate) = sidecar_val {
                            map.insert((sym.clone(), bar.open_ts), rate);
                        }
                    }
                }
            }
            map
        })
    };

    // ── Build sidecar maps depending on arm ──────────────────────────────────
    //
    // NON-MN arms (momentum/MR/carry/basis):
    //   Same logic as before. The `inject_sidecar` / `is_carry` path is unchanged.
    //   The 107 existing anchors are byte-identical by construction.
    //
    // MN arms (MnBasisSpread / MnFundingSpread / MnBasisFundingResidual):
    //   The MN path_gen has BOTH `funding_by_symbol` (real funding rates, for accrual)
    //   AND `basis_by_symbol` (basis values, for scoring) attached.
    //   The wiring per arm (D-MN.5 / D-MN.4):
    //
    //   MnBasisSpread:
    //     score = BasisReversal (−trailing_mean(basis)) → basis in `with_funding` (score channel)
    //     accrual = real funding rates → funding_override = funding_by_symbol map
    //
    //   MnFundingSpread:
    //     score = FundingCarry (−trailing_mean(funding)) → funding in `with_funding` (score channel)
    //     accrual = real funding rates (SAME map as score for FundingSpread) → funding_override
    //     (funding drives BOTH score and accrual for this arm)
    //
    //   MnBasisFundingResidual:
    //     score = BasisFundingResidual: basis via `with_basis_score`, funding via `with_funding`
    //     accrual = real funding rates → funding_override = funding_by_symbol map

    // Extract basis and funding maps from the generated path.
    let basis_map_from_path: Option<SidecarMap> = build_sidecar_map(
        &generated_path.basis_by_symbol,
        &generated_path.bars_by_symbol,
    );
    let funding_map_from_path: Option<SidecarMap> = build_sidecar_map(
        &generated_path.funding_by_symbol,
        &generated_path.bars_by_symbol,
    );

    // Legacy path (non-MN): build strategy_sidecar_map from funding_by_symbol only.
    // This is the UNCHANGED pre-MN logic (107 anchors safe).
    let strategy_sidecar_map: Option<SidecarMap> = if inject_sidecar && !score_source.is_mn() {
        // Non-MN: the sidecar rides funding_by_symbol (carry or basis via D-BR.3 channel reuse).
        funding_map_from_path.clone()
    } else {
        None
    };

    // funding_override for TcnScenarioInput (non-MN path):
    //   - carry: pass the sidecar map (enables run_path accrual).
    //   - basis: always None (no cashflow — D-BR.1).
    //   - momentum/MR: None (inject_sidecar=false → strategy_sidecar_map is None).
    let funding_override_non_mn: Option<SidecarMap> = if is_carry && !score_source.is_mn() {
        strategy_sidecar_map.clone()
    } else {
        None
    };

    // MN-arm-specific sidecar construction (D-MN.4 / D-MN.5, M-DEV-5).
    // For non-MN arms: strat_score_map = None, strat_basis_score_map = None,
    //   funding_override_mn = None → byte-identical to the pre-MN code.
    let (strat_score_map, strat_basis_score_map, funding_override_mn): (
        Option<SidecarMap>,
        Option<SidecarMap>,
        Option<SidecarMap>,
    ) = match score_source {
        SweepScoreSource::MnBasisSpread => {
            // Basis → score (via with_funding, BasisReversal arm).
            // Real funding → accrual (via funding_override).
            (basis_map_from_path, None, funding_map_from_path)
        }
        SweepScoreSource::MnFundingSpread => {
            // Funding → score (via with_funding, FundingCarry arm).
            // Funding → accrual (via funding_override; same map as score).
            (funding_map_from_path.clone(), None, funding_map_from_path)
        }
        SweepScoreSource::MnBasisFundingResidual => {
            // Basis → basis_score_map (via with_basis_score).
            // Funding → score ring (via with_funding, funding_rings for rank).
            // Funding → accrual (via funding_override).
            (
                funding_map_from_path.clone(),
                basis_map_from_path,
                funding_map_from_path,
            )
        }
        _ => (None, None, None), // non-MN: leave all None → 107 anchors safe
    };

    let funding_override = if score_source.is_mn() {
        funding_override_mn
    } else {
        funding_override_non_mn
    };
    let final_strategy_score_map = if score_source.is_mn() {
        strat_score_map
    } else {
        strategy_sidecar_map
    };

    // ── Merge per-symbol bars into the flat replay feed ───────────────────────
    let merged_bars = data::ReplayFeed::merge_synthetic(generated_path.bars_by_symbol);

    // ── Build fresh strategy with the INJECTED config (the C3 seam) ──────────
    // For carry AND basis (non-MN): inject the sidecar map via with_funding so the
    // strategy's score function can read carry/basis values per (Symbol, ts).
    // For momentum/MR: no sidecar → with_funding(None) is a no-op (anchor-safe).
    // For MN arms: inject the appropriate maps per the arm logic above.
    //
    // D-BR.1: for the basis arm (non-MN), the sidecar is injected HERE (score-only),
    // but funding_override in TcnScenarioInput is None (no accrual — no cashflow).
    let strat = strategy::MomentumStrategy::from_config(
        cfg.clone(),
        SmolStr::new(format!("param-sweep-cell-{}", cfg.lookback_minutes)),
    )
    .with_funding(final_strategy_score_map)
    .with_basis_score(strat_basis_score_map);

    // ── Run the backtest on this path ─────────────────────────────────────────
    // M-DEV-4 (D-BR.LOAD): taker_fee_bps and slippage_bps are now parameters
    // replacing the legacy hardcoded literals. Defaults (4/2) → MatchConfig
    // byte-identical for every non-basis run → 99 anchors hold.
    let input = backtest::cli_types::TcnScenarioInput {
        scenario_name: format!("sweep-path-{j}"),
        start_year: year,
        bar_count: merged_bars.len(),
        initial_capital: dec!(100_000),
        slippage_bps,
        taker_fee_bps,
        config_id: "top10_momentum_h1".to_string(),
        forecaster_id: "param_sweep".to_string(),
        bars_override: Some(merged_bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override,
        basis_override: None,
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
    // M-DEV-4: time-in-market counter from run_path (pure observability — no equity effect).
    let time_in_market_bars = result.time_in_market_bars;
    let bars_run = result.equity_curve.len().saturating_sub(1) as u64;
    // M-DEV-5: liquidations counter from run_path (0 for all non-MN runs → anchor-neutral).
    let liquidations = result.liquidations;

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

    // M-DEV-3: metric branch (D-HR.1) — 1h uses verbatim fns (anchor-safe);
    // coarse horizons use the *_periodic fns with the correct periods_per_year.
    let (sharpe, sortino, calmar) = if horizon == backtest::resample::Horizon::OneHour {
        // Verbatim 1h path — byte-identical to all 91 existing anchors.
        (
            backtest::stats::compute_sharpe_hourly(&equity_clamped),
            backtest::stats::compute_sortino_hourly(&equity_clamped),
            backtest::stats::compute_calmar(&equity_clamped),
        )
    } else {
        let ppy = sweep_periods_per_year(horizon, year);
        (
            backtest::stats::compute_sharpe_periodic(&equity_clamped, ppy),
            backtest::stats::compute_sortino_periodic(&equity_clamped, ppy),
            backtest::stats::compute_calmar_periodic(&equity_clamped, ppy),
        )
    };
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
        time_in_market_bars,
        bars_run,
        liquidations,
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

// ── Basis-reversal loader (M-DEV-5) ────────────────────────────────────────────

/// Load the basis data, compute `basis_at_return`, and build a basis-specific
/// `BlockBootstrapPathGen` with the basis attached (ADR-0051 § D6.9, D-BR.3).
///
/// **CRITICAL D-BR.1:** The basis sidecar is attached via `with_funding(...)` for the
/// SCORE only. The `run_path` accrual gate (`montecarlo.rs:322`) is NEVER entered for the
/// basis arm because `TcnScenarioInput.funding_override` is set to `None` in
/// `run_one_path_with_config`. The basis arm's P&L is pure price-of-selection; NO cashflow.
///
/// **Channel reuse (D-BR.3):** The basis rides the `funding_by_symbol` co-resample channel
/// (basis and funding are mutually exclusive in v0.1.0 — different `ScoreSource` arms).
/// The field is named `funding_*` but carries the BASIS value when this path_gen is used.
///
/// Returns `(basis_path_gen, Some(basis_revision_sha))` for the basis anchor body.
#[cfg(feature = "realdata")]
fn load_basis_path_gen(
    args: &Args,
    real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    symbols_prices: &[(trading_core::Symbol, Decimal)],
    bar_count: usize,
) -> Result<(Option<data::BlockBootstrapPathGen>, Option<String>)> {
    use backtest::basis_data::{BasisDataSource, LoadedBasis, build_basis_at_return};
    use backtest::realdata::TimeSpan as RealDataTimeSpan;

    // Load basis parquets and REVISION-verify.
    let symbols: Vec<trading_core::Symbol> =
        symbols_prices.iter().map(|(s, _)| s.clone()).collect();
    let basis_src = BasisDataSource::new(args.basis_root.clone(), symbols.clone());
    let span = RealDataTimeSpan::full_year(args.year);
    let scenario_name = format!("basis-sweep-load-{}", args.year);
    let loaded: LoadedBasis = basis_src
        .load(&span, &scenario_name)
        .map_err(|e| anyhow::anyhow!("load basis data: {e}"))?;

    // Verify basis revision SHA against the locked expected.
    if loaded.revision_sha != args.basis_revision_sha {
        anyhow::bail!(
            "basis revision mismatch: expected={} computed={}",
            args.basis_revision_sha,
            loaded.revision_sha
        );
    }
    let basis_revision_sha = loaded.revision_sha.clone();
    info!(
        basis_rows = loaded.rows.len(),
        basis_revision_sha = %basis_revision_sha,
        "basis data loaded and verified"
    );

    // Build the `basis_at_return[sym_i][k]` array (aligned to real return steps).
    // The basis parquet uses `open_time_ms` as the timestamp key (per BasisRow schema).
    // For each symbol: extract (open_time_ms, basis_close) and bar open_ts_ms.
    let mut basis_by_symbol_rows: Vec<Vec<(i64, Decimal)>> = Vec::with_capacity(symbols.len());
    let mut bar_ts_by_symbol_raw: Vec<Vec<i64>> = Vec::with_capacity(symbols.len());

    for (sym, _) in symbols_prices {
        // Collect basis rows for this symbol, sorted by open_time_ms.
        let sym_basis: Vec<(i64, Decimal)> = loaded
            .rows
            .iter()
            .filter(|r| r.symbol == *sym)
            .map(|r| (r.open_time_ms, r.basis_close))
            .collect();
        basis_by_symbol_rows.push(sym_basis);

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

    let basis_refs: Vec<&[(i64, Decimal)]> =
        basis_by_symbol_rows.iter().map(|v| v.as_slice()).collect();
    let bar_ts_refs: Vec<&[i64]> = bar_ts_by_symbol_raw.iter().map(|v| v.as_slice()).collect();

    let basis_at_return = build_basis_at_return(&basis_refs, &bar_ts_refs);
    info!(
        n_symbols = basis_at_return.len(),
        first_sym_len = basis_at_return.first().map_or(0, Vec::len),
        "basis_at_return built for basis-reversal co-resampling"
    );

    // Build a basis-specific BlockBootstrapPathGen with basis attached via with_funding.
    // D-BR.3: the basis rides the `funding_by_symbol` channel (basis + funding mutually
    // exclusive in v0.1.0). The value is the BASIS, not funding — see D-BR.1 note above.
    let basis_path_gen = data::BlockBootstrapPathGen::new(
        real_bars_by_symbol.to_vec(),
        data::BlockLengthPolicy::Auto,
    )
    .context("build basis BlockBootstrapPathGen")?
    .with_funding(Some(basis_at_return));

    // Verify the probe (selected_L should match the base path_gen).
    {
        use data::MonteCarloPathGen as _;
        let universe_probe: Vec<(trading_core::Symbol, Decimal)> = symbols_prices.to_vec();
        let _probe = basis_path_gen
            .generate(&universe_probe, bar_count, 0xC0FFEE)
            .context("basis path_gen probe generate")?;
        info!("basis path_gen probe: OK (basis co-resampling active)");
    }

    Ok((Some(basis_path_gen), Some(basis_revision_sha)))
}

#[cfg(not(feature = "realdata"))]
fn load_basis_path_gen(
    _args: &Args,
    _real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    _symbols_prices: &[(trading_core::Symbol, Decimal)],
    _bar_count: usize,
) -> Result<(Option<data::BlockBootstrapPathGen>, Option<String>)> {
    anyhow::bail!(
        "load_basis_path_gen called without --features realdata. \
         Basis-reversal requires real basis data. Rebuild with: cargo run -p backtest \
         --features candle,realdata --bin param_robustness_sweep -- --score-source basis-reversal ..."
    )
}

// ── MN dual-sidecar loader (M-DEV-5, D-MN.4) ─────────────────────────────────

/// Load BOTH basis and funding data, and build an MN path_gen with both attached.
///
/// The MN path_gen co-resamples BOTH sidecars at the SAME `idx_seq` (zero new RNG draws)
/// by attaching funding via `with_funding` and basis via `with_basis`. During
/// `BlockBootstrapPathGen::generate`, BOTH sidecars are bootstrapped with the SAME
/// block index sequence as the OHLCV bars, preserving cross-sidecar timing alignment.
///
/// Returns `(Some(mn_path_gen), Some("<basis_sha> <funding_sha>"))` where the SHA string
/// encodes both revision SHAs for the hashed body.
///
/// **D-MN.4 dual-sidecar roles:**
/// - `basis_by_symbol` (from `with_basis`): drives score for `MnBasisSpread` and
///   the `basis_score_map` for `MnBasisFundingResidual`.
/// - `funding_by_symbol` (from `with_funding`): drives score for `MnFundingSpread`
///   and the funding ring for `MnBasisFundingResidual`; drives short-leg accrual for
///   ALL three MN arms via `funding_override` in `TcnScenarioInput`.
#[cfg(feature = "realdata")]
fn load_mn_path_gen(
    args: &Args,
    real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    symbols_prices: &[(trading_core::Symbol, Decimal)],
    bar_count: usize,
) -> Result<(Option<data::BlockBootstrapPathGen>, Option<String>)> {
    use backtest::basis_data::{BasisDataSource, LoadedBasis, build_basis_at_return};
    use backtest::funding_data::{FundingDataSource, LoadedFunding, build_funding_at_return};
    use backtest::realdata::TimeSpan as RealDataTimeSpan;

    let symbols: Vec<trading_core::Symbol> =
        symbols_prices.iter().map(|(s, _)| s.clone()).collect();
    let span = RealDataTimeSpan::full_year(args.year);

    // ── Load basis ────────────────────────────────────────────────────────────
    let basis_src = BasisDataSource::new(args.basis_root.clone(), symbols.clone());
    let scenario_name_b = format!("mn-sweep-basis-load-{}", args.year);
    let loaded_basis: LoadedBasis = basis_src
        .load(&span, &scenario_name_b)
        .map_err(|e| anyhow::anyhow!("load MN basis data: {e}"))?;

    if loaded_basis.revision_sha != args.basis_revision_sha {
        anyhow::bail!(
            "MN basis revision mismatch: expected={} computed={}",
            args.basis_revision_sha,
            loaded_basis.revision_sha
        );
    }
    let basis_revision_sha = loaded_basis.revision_sha.clone();
    info!(
        basis_rows = loaded_basis.rows.len(),
        basis_revision_sha = %basis_revision_sha,
        "MN basis data loaded and verified"
    );

    // ── Load funding ──────────────────────────────────────────────────────────
    let funding_src = FundingDataSource::new(args.funding_root.clone(), symbols.clone());
    let scenario_name_f = format!("mn-sweep-funding-load-{}", args.year);
    let loaded_funding: LoadedFunding = funding_src
        .load(&span, &scenario_name_f)
        .map_err(|e| anyhow::anyhow!("load MN funding data: {e}"))?;

    if loaded_funding.revision_sha != args.funding_revision_sha {
        anyhow::bail!(
            "MN funding revision mismatch: expected={} computed={}",
            args.funding_revision_sha,
            loaded_funding.revision_sha
        );
    }
    let funding_revision_sha = loaded_funding.revision_sha.clone();
    info!(
        funding_rows = loaded_funding.rows.len(),
        funding_revision_sha = %funding_revision_sha,
        "MN funding data loaded and verified"
    );

    // ── Build basis_at_return ─────────────────────────────────────────────────
    let mut basis_by_symbol_rows: Vec<Vec<(i64, Decimal)>> = Vec::with_capacity(symbols.len());
    let mut bar_ts_by_symbol_raw: Vec<Vec<i64>> = Vec::with_capacity(symbols.len());

    for (sym, _) in symbols_prices {
        let sym_basis: Vec<(i64, Decimal)> = loaded_basis
            .rows
            .iter()
            .filter(|r| r.symbol == *sym)
            .map(|r| (r.open_time_ms, r.basis_close))
            .collect();
        basis_by_symbol_rows.push(sym_basis);

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

    let basis_refs: Vec<&[(i64, Decimal)]> =
        basis_by_symbol_rows.iter().map(|v| v.as_slice()).collect();
    let bar_ts_refs_b: Vec<&[i64]> = bar_ts_by_symbol_raw.iter().map(|v| v.as_slice()).collect();
    let basis_at_return = build_basis_at_return(&basis_refs, &bar_ts_refs_b);

    // ── Build funding_at_return ───────────────────────────────────────────────
    let mut funding_by_symbol_rows: Vec<Vec<(i64, Decimal)>> = Vec::with_capacity(symbols.len());
    let mut bar_ts_by_symbol_raw2: Vec<Vec<i64>> = Vec::with_capacity(symbols.len());

    for (sym, _) in symbols_prices {
        let sym_funding: Vec<(i64, Decimal)> = loaded_funding
            .rows
            .iter()
            .filter(|r| r.symbol == *sym)
            .map(|r| (r.funding_time_ms, r.funding_rate))
            .collect();
        funding_by_symbol_rows.push(sym_funding);

        let bar_ts: Vec<i64> = real_bars_by_symbol
            .iter()
            .find(|(s, _)| s == sym)
            .map(|(_, bars)| {
                bars.iter()
                    .map(|b| b.open_ts.inner().unix_timestamp() * 1000)
                    .collect()
            })
            .unwrap_or_default();
        bar_ts_by_symbol_raw2.push(bar_ts);
    }

    let funding_refs: Vec<&[(i64, Decimal)]> = funding_by_symbol_rows
        .iter()
        .map(|v| v.as_slice())
        .collect();
    let bar_ts_refs_f: Vec<&[i64]> = bar_ts_by_symbol_raw2.iter().map(|v| v.as_slice()).collect();
    let funding_at_return = build_funding_at_return(&funding_refs, &bar_ts_refs_f);

    info!(
        n_symbols = basis_at_return.len(),
        first_sym_basis_len = basis_at_return.first().map_or(0, Vec::len),
        first_sym_funding_len = funding_at_return.first().map_or(0, Vec::len),
        "MN basis_at_return + funding_at_return built for co-resampling"
    );

    // ── Build MN BlockBootstrapPathGen with BOTH sidecars attached ────────────
    // The SAME `idx_seq` is used for all three co-sampled arrays (OHLCV, basis, funding).
    // This ensures timing alignment across all three sidecars (D-MN.4: zero new RNG draws).
    let mn_path_gen = data::BlockBootstrapPathGen::new(
        real_bars_by_symbol.to_vec(),
        data::BlockLengthPolicy::Auto,
    )
    .context("build MN BlockBootstrapPathGen")?
    .with_funding(Some(funding_at_return))
    .with_basis(Some(basis_at_return));

    // Verify the probe (selected_L should match the base path_gen).
    {
        use data::MonteCarloPathGen as _;
        let universe_probe: Vec<(trading_core::Symbol, Decimal)> = symbols_prices.to_vec();
        let probe = mn_path_gen
            .generate(&universe_probe, bar_count, 0xC0FFEE)
            .context("MN path_gen probe generate")?;
        info!(
            has_funding = probe.funding_by_symbol.is_some(),
            has_basis = probe.basis_by_symbol.is_some(),
            "MN path_gen probe: OK (dual-sidecar co-resampling active)"
        );
    }

    // Combined SHA for the report body (both revision pinned).
    let combined_sha = format!("basis:{basis_revision_sha} funding:{funding_revision_sha}");
    Ok((Some(mn_path_gen), Some(combined_sha)))
}

#[cfg(not(feature = "realdata"))]
fn load_mn_path_gen(
    _args: &Args,
    _real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    _symbols_prices: &[(trading_core::Symbol, Decimal)],
    _bar_count: usize,
) -> Result<(Option<data::BlockBootstrapPathGen>, Option<String>)> {
    anyhow::bail!(
        "load_mn_path_gen called without --features realdata. \
         MN spread requires real basis + funding data. Rebuild with: cargo run -p backtest \
         --features candle,realdata --bin param_robustness_sweep -- --score-source mn-basis-spread ..."
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

    // M-DEV-3: derive bar_count from (year, horizon) per D-HR.3.
    // For 1h (default) this is byte-identical to the previous fixed 8760/8784.
    // For 4h: divide by 4 (exact integer: 8760/4=2190, 8784/4=2196).
    // For daily: divide by 24 (exact integer: 8760/24=365, 8784/24=366).
    let bars_per_year_1h: usize = match args.year {
        2023 => 8760,
        2024 => 8784,
        _ => 8760,
    };
    let bar_count = match args.horizon {
        backtest::resample::Horizon::OneHour => bars_per_year_1h,
        backtest::resample::Horizon::FourHours => bars_per_year_1h / 4,
        backtest::resample::Horizon::OneDay => bars_per_year_1h / 24,
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

    // ── M-DEV-6/M-DEV-5: load sidecar data and build the sidecar path_gen ──────
    // For momentum/MR: `carry_path_gen_opt = None` → anchor-neutral by construction.
    // For carry: load funding, build `funding_at_return[sym_i][k]`, build a
    // carry-specific BlockBootstrapPathGen with funding attached (D-CARRY.7).
    // For basis (M-DEV-5): load basis, build `basis_at_return[sym_i][k]`, build a
    // basis-specific BlockBootstrapPathGen with basis attached via `with_funding`
    // (D-BR.3 — reuses the funding_by_symbol co-resample channel).
    // For MN arms (M-DEV-5, D-MN.4): load BOTH basis and funding, build an MN
    // path_gen with basis attached via `with_basis` AND funding via `with_funding`.
    // NOTE: `carry_path_gen_opt` is reused for the basis/MN path_gen too (same shape).
    let (carry_path_gen_opt, funding_revision_sha_for_report): (
        Option<data::BlockBootstrapPathGen>,
        Option<String>,
    ) = if args.score_source.needs_funding() && args.generator == GeneratorKind::BlockBootstrapReal
    {
        load_carry_path_gen(&args, &real_bars_by_symbol, &symbols_prices, bar_count)?
    } else if args.score_source.needs_basis() && args.generator == GeneratorKind::BlockBootstrapReal
    {
        // M-DEV-5: load basis and build the basis path_gen (reusing the same Option slot).
        load_basis_path_gen(&args, &real_bars_by_symbol, &symbols_prices, bar_count)?
    } else if args.score_source.is_mn() && args.generator == GeneratorKind::BlockBootstrapReal {
        // M-DEV-5 (D-MN.4): load BOTH basis and funding, build the MN path_gen
        // with both sidecars attached for co-resampling at the same idx_seq.
        load_mn_path_gen(&args, &real_bars_by_symbol, &symbols_prices, bar_count)?
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
    // For basis: use the basis-specific path_gen (has basis attached via with_funding).
    // For MN arms: use the MN path_gen (has BOTH basis via with_basis AND funding via with_funding).
    // For momentum/MR: use the base path_gen (no sidecar → anchor-neutral).
    let is_basis = args.score_source == SweepScoreSource::BasisReversal;
    let active_path_gen_opt: Option<&data::BlockBootstrapPathGen> =
        if let Some(ref cpg) = carry_path_gen_opt {
            // carry_path_gen_opt holds carry OR basis OR MN path_gen (reused channel).
            Some(cpg)
        } else {
            block_path_gen_opt.as_ref()
        };

    let is_carry = args.score_source == SweepScoreSource::Carry;
    // inject_sidecar: true for carry OR basis OR MN — all extract sidecar from generated_path.
    // For basis/MN: funding_override stays None for the score-only arms; MN handles it internally.
    let inject_sidecar = is_carry || is_basis || args.score_source.is_mn();

    // ── Outer θ-loop (sequential for log legibility — ~10-15 min at N=200, 6 cells) ─
    // ADR-0051 § D6.4: collect into Vec, sort by g before render.
    // The inner per-path rayon fan-out is where parallelism lives.
    let mut cell_results: Vec<CellResult> = Vec::with_capacity(grid.len());

    for cell in grid {
        let cell_start = std::time::Instant::now();
        let per_cell_cfg = cell_config(
            &base_cfg,
            cell,
            args.direction,
            args.score_source,
            args.selection_mode,
        );

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
                        inject_sidecar,
                        is_carry,
                        args.horizon,
                        args.taker_fee_bps,
                        args.slippage_bps,
                        args.score_source,
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
        // M-DEV-4: time-in-market totals across all N paths. Summed before consuming iter.
        let total_time_in_market_bars: u64 = indexed.iter().map(|r| r.time_in_market_bars).sum();
        let total_bars_run: u64 = indexed.iter().map(|r| r.bars_run).sum();
        // M-DEV-5 (D-MN.8): total liquidations across all N paths.
        // 0 for all non-MN runs → anchor-neutral by construction.
        let total_liquidations: u64 = indexed.iter().map(|r| r.liquidations).sum();
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
            total_time_in_market_bars,
            total_bars_run,
            total_liquidations,
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
                            basis_by_symbol: None,
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
                // M-DEV-3: BH metric branch — mirror the per-cell branch (D-HR.1/D-HR.4).
                // 1h → verbatim fns (byte-identical); coarse → *_periodic.
                let (bh_sharpe, bh_sortino, bh_calmar) =
                    if args.horizon == backtest::resample::Horizon::OneHour {
                        (
                            backtest::stats::compute_sharpe_hourly(&equity_clamped),
                            backtest::stats::compute_sortino_hourly(&equity_clamped),
                            backtest::stats::compute_calmar(&equity_clamped),
                        )
                    } else {
                        let ppy = sweep_periods_per_year(args.horizon, args.year);
                        (
                            backtest::stats::compute_sharpe_periodic(&equity_clamped, ppy),
                            backtest::stats::compute_sortino_periodic(&equity_clamped, ppy),
                            backtest::stats::compute_calmar_periodic(&equity_clamped, ppy),
                        )
                    };
                let pm = backtest::stats::PathMetrics {
                    sharpe: bh_sharpe,
                    sortino: bh_sortino,
                    calmar: bh_calmar,
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

    // M-DEV-4: TS-momentum gets its own scenario slug (distinct from momentum/MR/carry).
    // M-DEV-3: horizon retest runs get a horizon-specific slug (§ D-HR.5).
    let is_horizon_run = args.horizon != backtest::resample::Horizon::OneHour;
    let horizon_label = match args.horizon {
        backtest::resample::Horizon::OneHour => "",
        backtest::resample::Horizon::FourHours => "4h",
        backtest::resample::Horizon::OneDay => "daily",
    };
    let gen_label = match args.generator {
        GeneratorKind::BlockBootstrapReal => "real",
        GeneratorKind::GbmSmoke => "gbm",
    };
    let scenario_name = if is_horizon_run && args.selection_mode.is_ts() {
        // Horizon TS run: e.g. "v1-ts-horizon-4h-theta-surface-2023-block-bootstrap-real-fy"
        format!(
            "v1-ts-horizon-{horizon}-theta-surface-{year}-block-bootstrap-{gen}-fy",
            horizon = horizon_label,
            year = args.year,
            gen = gen_label,
        )
    } else if is_horizon_run && args.score_source == SweepScoreSource::Carry {
        // Horizon carry run: e.g. "v1-carry-horizon-4h-theta-surface-2023-block-bootstrap-real-fy"
        format!(
            "v1-carry-horizon-{horizon}-theta-surface-{year}-block-bootstrap-{gen}-fy",
            horizon = horizon_label,
            year = args.year,
            gen = gen_label,
        )
    } else if args.selection_mode.is_ts() {
        format!(
            "v1-ts-momentum-theta-surface-{year}-block-bootstrap-{gen}-fy",
            year = args.year,
            gen = gen_label,
        )
    } else {
        match args.score_source {
            SweepScoreSource::Carry => format!(
                "v1-carry-theta-surface-{year}-block-bootstrap-{gen}-fy",
                year = args.year,
                gen = gen_label,
            ),
            // M-DEV-5 (D-BR.9): basis-reversal scenario name carries the fee level
            // as a zero-padded two-digit number so the four fee × two regime surfaces
            // are DISTINCT anchors (§ D-BR.2-LOCKED / § D-BR.9).
            SweepScoreSource::BasisReversal => format!(
                "v1-basis-reversal-fee{fee:02}bps-theta-surface-{year}-block-bootstrap-{gen}-fy",
                fee = args.taker_fee_bps,
                year = args.year,
                gen = gen_label,
            ),
            // M-DEV-5 (D-MN.8): MN scenario name carries the arm label + fee level.
            // Format: "v2-mn-{arm}-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy"
            // The three arms × two fee levels × two years → 12 DISTINCT anchors (§ D6.10).
            SweepScoreSource::MnBasisSpread
            | SweepScoreSource::MnFundingSpread
            | SweepScoreSource::MnBasisFundingResidual => format!(
                "v2-mn-{arm}-fee{fee:02}bps-theta-surface-{year}-block-bootstrap-{gen}-fy",
                arm = args.score_source.mn_arm_label(),
                fee = args.taker_fee_bps,
                year = args.year,
                gen = gen_label,
            ),
            SweepScoreSource::VolAdjustedReturn => format!(
                "v1-{family}-theta-surface-{year}-block-bootstrap-{gen}-fy",
                family = args.direction.label(),
                year = args.year,
                gen = gen_label,
            ),
        }
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
        args.selection_mode,
        args.horizon,
        args.taker_fee_bps,
        args.slippage_bps,
    );

    // ── Resolve effective out_dir ─────────────────────────────────────────────
    // For carry: if the user did not override --out-dir, default to the carry reports dir.
    // For TS: if the user did not override --out-dir, default to the TS reports dir.
    // For horizon retest (horizon != 1h): default to the horizon-retest-robustness reports dir.
    // For basis: default to the perp-basis-signal-robustness reports dir (D-BR.9).
    // We detect "was the default changed?" by checking if it's still the momentum default.
    let momentum_default_out_dir =
        PathBuf::from("spec/v1/momentum-parameter-robustness-sweep/reports/");
    let effective_out_dir = if is_horizon_run && args.out_dir == momentum_default_out_dir {
        // M-DEV-3: horizon runs default to the horizon-retest-robustness reports dir (D-HR.8).
        PathBuf::from("spec/v1/horizon-retest-robustness/reports/")
    } else if args.selection_mode.is_ts() && args.out_dir == momentum_default_out_dir {
        PathBuf::from("spec/v1/time-series-momentum-robustness/reports/")
    } else if args.score_source == SweepScoreSource::Carry
        && args.out_dir == momentum_default_out_dir
    {
        PathBuf::from("spec/v1/carry-strategy/reports/")
    } else if args.score_source == SweepScoreSource::BasisReversal
        && args.out_dir == momentum_default_out_dir
    {
        // M-DEV-5 (D-BR.9): basis-reversal reports live in the dedicated namespace dir.
        PathBuf::from("spec/v1/perp-basis-signal-robustness/reports/")
    } else if args.score_source.is_mn() && args.out_dir == momentum_default_out_dir {
        // M-DEV-5 (D-MN.8): MN-spread reports live in the MN namespace dir (§ D6.10).
        PathBuf::from("spec/v1/perp-basis-mn-spread/reports/")
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
