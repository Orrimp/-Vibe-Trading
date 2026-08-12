//! θ-sweep seam for `bin/param_robustness_sweep.rs` (review 1-15, M-DEV pattern).
//!
//! Behaviour-preserving extraction (the 1-14 `mc_harness` precedent): the
//! θ-grid definitions (`ThetaCell` + the LOCKED grids), the sweep-axis enums
//! (`SweepDirection` / `SweepScoreSource` / `SweepSelectionMode`), the
//! ADR-0051 § D6.1 seed derivation, the per-cell config injection
//! (`cell_config`), the grid-definition formatters, the θ-surface renderer
//! (`render_surface_report`), the family-verdict single source, and the
//! scenario-identity builder (`build_scenario_name`) are moved VERBATIM from
//! `bin/param_robustness_sweep.rs` so the FP-C3.2 / FP-C3.5 / M1 e2e gates
//! (`tests/param_sweep_e2e.rs`) exercise the REAL production chain instead of
//! local re-implementations (the #66 vacuous-test class). The bin now calls
//! this module; every anchored lane's arithmetic and byte-identity are
//! untouched.
//!
//! This is an internal test-seam module (like `mc_harness`), NOT a stable
//! public API surface.
//!
//! ## Anchor-safety invariants owned here (review 1-15)
//!
//! - **M2 scenario-identity discriminator**: [`GridKind::scenario_discriminator`]
//!   appends `-grid-twocell` to the never-anchored `--grid two-cell` lane so a
//!   two-cell probe run can no longer emit the SAME scenario identity as the
//!   anchored tier-1 surface (which shadowed anchor #86's report as "latest
//!   matching" and turned the anchors gate falsely RED). Every LOCKED
//!   production grid returns the EMPTY discriminator — all anchored
//!   `*-theta-surface-*` scenario names are byte-identical. Proven by
//!   `tests/param_sweep_e2e.rs::tier1_scenario_name_byte_unchanged_twocell_distinct`
//!   and the bin's `scenario_name_*` literal-string tests, both of which call
//!   [`build_scenario_name`] — the ONE production builder (review 1-18 H3: the
//!   bin used to carry an inline `format!` copy that never reached this seam,
//!   so both the M2 discriminator and the L3 `gbm-smoke` token were INERT in
//!   production while their tests passed against the library fn).
//! - **L3 honest gbm naming + VOID banner**: the gbm lane's scenario token is
//!   `gbm-smoke` (the old name embedded `block-bootstrap-gbm` although no
//!   bootstrap runs in that lane), and gbm-lane report bodies carry an explicit
//!   `VOID — not anchor-grade (frozen rule §4.1)` banner line. Both changes are
//!   gated to the never-anchored gbm lane; `block-bootstrap-real` bodies and
//!   names are byte-identical.
//! - **L6 single verdict source**: the renderer AND the bin's console summary
//!   both consume [`family_any_non_fragile`] / [`family_verdict_line`] — the
//!   hashed family line and the console line cannot desync.
//! - **D6.1 seed rule**: [`derive_path_seed`] delegates to the production
//!   [`crate::mc_harness::derive_path_seed`] (ADR-0051 D1) — one formula, one
//!   source, shared by the C2 ensemble and the C3 sweep.

#![allow(clippy::float_arithmetic)] // statistical metric layer uses f64 (lifted from the bin, which allows the same)
#![allow(clippy::cast_precision_loss)] // u64 bar counters -> f64 fractions in the renderer (display-only)
#![allow(clippy::doc_markdown, clippy::must_use_candidate)] // verbatim-lifted bin docs/API kept byte-close to the origin (R-NR.5 lift rule)

use rust_decimal::Decimal;

use crate::bakeoff::robustness::ParamRobustnessVerdict;

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
        role: "baseline θ* (C2-shipped config; disclosed DIRECTION-match vs the C2 anchor only — manual eyeball at N=200 vs C2's N=500, so the numbers do NOT reproduce; no automated compare exists)",
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

impl GridKind {
    /// Scenario-name discriminator (review 1-15 M2).
    ///
    /// `--grid two-cell` previously emitted the SAME scenario identity as the
    /// anchored tier-1 run, so its (never-anchored) report shadowed anchor
    /// #86's report as "latest matching" and turned the anchors gate falsely
    /// RED. Non-production grids now append a discriminator to the scenario
    /// name; every LOCKED production grid returns the EMPTY string so all
    /// anchored scenario names stay byte-identical — proven by
    /// `tests/param_sweep_e2e.rs::tier1_scenario_name_byte_unchanged_twocell_distinct`
    /// and the bin's `scenario_name_anchored_*` literal-string tests (review
    /// 1-18 H3 corrected the old "`m2_*` unit tests" citation: no test by that
    /// name has ever existed).
    #[must_use]
    pub fn scenario_discriminator(self) -> &'static str {
        match self {
            Self::TwoCell => "-grid-twocell",
            Self::Tier1
            | Self::MrTier1
            | Self::CarryTier1
            | Self::TsTier1
            | Self::Ts4h
            | Self::TsDaily
            | Self::Carry4h
            | Self::CarryDaily
            | Self::BasisTier1
            | Self::MnTier1 => "",
        }
    }

    /// The strategy-family direction each grid was designed for (review 1-16).
    ///
    /// Read off each grid's LOCKED doc block:
    /// - [`Self::MrTier1`] is the ONLY grid designed for `--direction reversion`
    ///   (§ D-MR.2-LOCKED: "identical to C3 except direction=Reversion").
    /// - [`Self::Tier1`] is the momentum grid (§ D-C3.2-LOCKED).
    /// - The carry grids ([`Self::CarryTier1`]/[`Self::Carry4h`]/[`Self::CarryDaily`],
    ///   § D-CARRY.2-LOCKED / § D-HR.4-LOCKED), the TS grids
    ///   ([`Self::TsTier1`]/[`Self::Ts4h`]/[`Self::TsDaily`], § D-TSM.3-LOCKED /
    ///   § D-HR.4-LOCKED), the basis grid ([`Self::BasisTier1`], § D-BR.2-LOCKED)
    ///   and the MN grid ([`Self::MnTier1`], § D-MN.8-LOCKED) all hold
    ///   `direction=momentum` (identity — their signs live in the score fns).
    /// - [`Self::TwoCell`] is the FP-C3.2-only momentum probe grid.
    #[must_use]
    pub fn required_direction(self) -> SweepDirection {
        match self {
            Self::MrTier1 => SweepDirection::Reversion,
            Self::Tier1
            | Self::CarryTier1
            | Self::TsTier1
            | Self::TwoCell
            | Self::Ts4h
            | Self::TsDaily
            | Self::Carry4h
            | Self::CarryDaily
            | Self::BasisTier1
            | Self::MnTier1 => SweepDirection::Momentum,
        }
    }

    /// The CLI-level selection mode each grid was designed for (review 1-17).
    ///
    /// Read off each grid's LOCKED doc block + the anchored reports' recorded
    /// `held_constant` rows:
    /// - The TS grids ([`Self::TsTier1`]/[`Self::Ts4h`]/[`Self::TsDaily`],
    ///   § D-TSM.3-LOCKED / § D-HR.4-LOCKED) run
    ///   `selection_mode=time_series_long_flat` — anchors #90/#91 and the TS
    ///   horizon surfaces (#92+) record exactly that in their hashed
    ///   `held_constant` row.
    /// - EVERY other grid runs the default `cross-sectional-top-k` at the CLI
    ///   level. The MN grid's `LongShort` selection is applied per-cell INSIDE
    ///   [`cell_config`] from the MN score source (D-MN.5) — the anchored MN
    ///   invocations pass the default `--selection-mode`, so that is what this
    ///   guard requires.
    #[must_use]
    pub fn required_selection_mode(self) -> SweepSelectionMode {
        match self {
            Self::TsTier1 | Self::Ts4h | Self::TsDaily => SweepSelectionMode::TimeSeriesLongFlat,
            Self::Tier1
            | Self::MrTier1
            | Self::CarryTier1
            | Self::TwoCell
            | Self::Carry4h
            | Self::CarryDaily
            | Self::BasisTier1
            | Self::MnTier1 => SweepSelectionMode::CrossSectionalTopK,
        }
    }

    /// The score-source family (families) each grid was designed for (review 1-17).
    ///
    /// Read off each grid's LOCKED doc block + the anchored reports' recorded
    /// `held_constant` rows:
    /// - Price grids ([`Self::Tier1`]/[`Self::TwoCell`]/[`Self::MrTier1`]) and
    ///   the TS grids ([`Self::TsTier1`]/[`Self::Ts4h`]/[`Self::TsDaily`]) run
    ///   `score_source=vol_adjusted_return` (the TS surfaces #90/#91 hash that
    ///   exact `held_constant` field; the TS arm computes its own trend score
    ///   and must NOT load a carry/basis sidecar under the TS name).
    /// - Carry grids ([`Self::CarryTier1`]/[`Self::Carry4h`]/[`Self::CarryDaily`],
    ///   § D-CARRY.2-LOCKED / § D-HR.4-LOCKED) run `score_source=funding_carry`
    ///   (CLI `carry`).
    /// - [`Self::BasisTier1`] (§ D-BR.2-LOCKED) runs `score_source=basis_reversal`.
    /// - [`Self::MnTier1`] (§ D-MN.8-LOCKED) runs one of the three MN arms
    ///   (`mn-basis-spread` / `mn-funding-spread` / `mn-basis-funding-residual`).
    #[must_use]
    pub fn allowed_score_sources(self) -> &'static [SweepScoreSource] {
        match self {
            Self::Tier1
            | Self::TwoCell
            | Self::MrTier1
            | Self::TsTier1
            | Self::Ts4h
            | Self::TsDaily => &[SweepScoreSource::VolAdjustedReturn],
            Self::CarryTier1 | Self::Carry4h | Self::CarryDaily => &[SweepScoreSource::Carry],
            Self::BasisTier1 => &[SweepScoreSource::BasisReversal],
            Self::MnTier1 => &[
                SweepScoreSource::MnBasisSpread,
                SweepScoreSource::MnFundingSpread,
                SweepScoreSource::MnBasisFundingResidual,
            ],
        }
    }

    /// The decision cadence (`--horizon`) each grid was LOCKED at (review 1-18).
    ///
    /// The 1-17 guard closed direction × selection_mode × score_source but left
    /// the FOURTH forge axis — `--horizon` — wide open. Two live forgeries:
    ///
    /// - `--grid ts-4h` with the DEFAULT `--horizon 1h` runs the 4h-tuned
    ///   lookbacks {42,180,540} against 1h bars and emits
    ///   `v1-ts-momentum-theta-surface-{year}-…` — anchor #90/#91's identity —
    ///   into the frozen TS dir.
    /// - `--grid ts-tier1 --horizon 4h` is the converse: the 1h-tuned grid emits
    ///   `v1-ts-horizon-4h-theta-surface-{year}-…` — anchor #92/#93's identity.
    ///   The carry pair (`carry-4h`/`carry-daily`/`carry-tier1`) forges
    ///   #96..#99 the same way.
    ///
    /// The horizon is a HASHED body row (`| horizon |`, gated to coarse runs)
    /// AND a scenario-name segment, so a mismatched pair produces a
    /// wrong-cadence surface under a locked anchor's name; the forged report
    /// then shadows the real one as "latest matching" and the anchors gate goes
    /// falsely RED.
    ///
    /// Read off each grid's LOCKED doc block (§ D-HR.4-LOCKED for the horizon
    /// grids; every other grid is 1h by construction — its anchored reports
    /// carry NO `| horizon |` row, which is exactly what
    /// `Horizon::OneHour` renders).
    #[must_use]
    pub fn required_horizon(self) -> crate::resample::Horizon {
        match self {
            Self::Ts4h | Self::Carry4h => crate::resample::Horizon::FourHours,
            Self::TsDaily | Self::CarryDaily => crate::resample::Horizon::OneDay,
            Self::Tier1
            | Self::MrTier1
            | Self::CarryTier1
            | Self::TsTier1
            | Self::TwoCell
            | Self::BasisTier1
            | Self::MnTier1 => crate::resample::Horizon::OneHour,
        }
    }

    /// The `--taker-fee-bps` ladder each grid was LOCKED at (review 1-20).
    ///
    /// The FIFTH forge axis. `--taker-fee-bps` reaches `MatchConfig` (every fill
    /// is re-priced) but it enters the hashed body ONLY for the basis/MN
    /// families (`render_surface_report` gates the `| taker_fee_bps |` row on
    /// `is_basis_run || is_mn_run`) and the scenario NAME only for those same
    /// families (`build_scenario_name`). So on every OTHER grid the fee is
    /// invisible in both the identity and the body:
    ///
    /// - `--grid tier1 --score-source vol-adjusted-return --taker-fee-bps 20`
    ///   runs 20 bps fills and emits `v1-momentum-theta-surface-{year}-…` —
    ///   anchor #86's EXACT identity — into the frozen momentum dir with NO fee
    ///   row anywhere in the body. The forged report is indistinguishable from
    ///   the real one by inspection, shadows it as "latest matching", and turns
    ///   `scripts/verify_anchors.sh` falsely RED.
    ///
    /// Read off the anchored bodies' `| taker_fee_bps |` rows:
    /// - [`Self::BasisTier1`] — anchors #100..#107 record 0 / 2 / 5 / 10.
    /// - [`Self::MnTier1`] — anchors #108..#119 record 0 / 5. The full
    ///   `{0,2,5,10}` ladder is accepted (§ D-MN.8 shares the basis fee ladder);
    ///   the two un-anchored rungs mint their OWN `fee02`/`fee10` names, so they
    ///   cannot collide with an anchored identity.
    /// - EVERY other grid — no fee row exists in any of its anchored bodies
    ///   because each ran at the legacy hardcoded literal, which is exactly the
    ///   CLI default: **4**.
    #[must_use]
    pub fn allowed_taker_fee_bps(self) -> &'static [u32] {
        match self {
            // § D-BR.LOAD / § D-MN.8 fee ladder.
            Self::BasisTier1 | Self::MnTier1 => &[0, 2, 5, 10],
            // The legacy hardcoded literal that every pre-basis anchor ran at.
            Self::Tier1
            | Self::MrTier1
            | Self::CarryTier1
            | Self::TsTier1
            | Self::TwoCell
            | Self::Ts4h
            | Self::TsDaily
            | Self::Carry4h
            | Self::CarryDaily => &[LEGACY_TAKER_FEE_BPS],
        }
    }

    /// The `--slippage-bps` value each grid was LOCKED at (review 1-20).
    ///
    /// **Every** anchored surface in `evidence/anchors.toml` ran at slippage
    /// **2** — the basis/MN bodies record `| slippage_bps | 2 |` literally, and
    /// every other family ran at the same legacy hardcoded literal (no row).
    /// The § D-BR.LOAD fee sweep is explicitly the TAKER leg only.
    ///
    /// This is the in-lane half of the fifth forge axis: `--grid basis-tier1
    /// --taker-fee-bps 5 --slippage-bps 7` reproduces anchor #104's exact
    /// scenario name (`v1-basis-reversal-fee05bps-…`) and its exact
    /// `| taker_fee_bps | 5 |` row while filling at a different price — the
    /// only body difference is the `| slippage_bps |` row, which is precisely
    /// the row a reader trusts to be constant.
    #[must_use]
    pub fn required_slippage_bps(self) -> u32 {
        LEGACY_SLIPPAGE_BPS
    }
}

/// The legacy hardcoded taker fee every pre-basis anchored surface ran at, and
/// the `--taker-fee-bps` CLI default (review 1-20).
pub const LEGACY_TAKER_FEE_BPS: u32 = 4;

/// The slippage EVERY anchored surface ran at, and the `--slippage-bps` CLI
/// default (review 1-20).
pub const LEGACY_SLIPPAGE_BPS: u32 = 2;

/// Validate the `--direction` × `--grid` pairing (review 1-16).
///
/// An unvalidated cross-product can FORGE the OTHER family's anchored scenario
/// NAME: `--direction momentum --grid mr-tier1` emits momentum's anchored
/// identity (anchor #86's name) over the MR grid, and `--direction reversion
/// --grid tier1` emits MR's anchored identity (anchor #87's name) over momentum
/// cells under a false LOCKED header — either way the forged report shadows the
/// real one as "latest matching" and turns the anchors gate falsely RED via a
/// single misinvocation. Every checked-in config/invocation uses a correctly
/// paired direction×grid, so bailing on mismatches rejects ONLY misinvocations
/// — correct pairs are byte-unchanged.
///
/// # Errors
///
/// On a mismatch, returns a message naming BOTH the requested direction and the
/// grid's required direction.
pub fn validate_direction_grid_pairing(
    grid: GridKind,
    direction: SweepDirection,
) -> Result<(), String> {
    let required = grid.required_direction();
    if direction == required {
        Ok(())
    } else {
        Err(format!(
            "--direction {direction:?} does not pair with --grid {grid:?}: that grid is the \
             {required:?}-family grid and requires --direction {required:?}. A mismatched pair \
             would forge the other family's anchored scenario name over the wrong cells \
             (anchors-gate false RED) — refusing to run."
        ))
    }
}

/// Validate the FULL `--direction` × `--selection-mode` × `--score-source` ×
/// `--horizon` × `--grid` tuple (review 1-17, extended to the horizon axis by
/// review 1-18 — itself extending the 1-16 direction-only guard).
///
/// The 1-16 guard closed the direction axis but left the selection_mode and
/// score_source axes open:
/// - `--grid ts-tier1` + the default `cross-sectional-top-k` passes the
///   direction check yet forges anchor #86's momentum identity over the TS
///   grid (and the converse, `--grid tier1 --selection-mode
///   time-series-long-flat`, forges the TS anchors' names #90/#91 over
///   momentum cells);
/// - `--grid ts-tier1 --score-source carry` loads the funding sidecar and runs
///   behaviorally-different equity under the TS anchored name.
///
/// Review 1-18 adds the FOURTH axis, `--horizon` (see
/// [`GridKind::required_horizon`]): `--grid ts-4h` at the default `--horizon
/// 1h` forges the 1h TS anchors' names (#90/#91) with 4h-tuned lookbacks, and
/// `--grid ts-tier1 --horizon 4h` forges the horizon anchors' names
/// (#92/#93) — the carry pair forges #96..#99 identically.
///
/// Review 1-20 adds the FIFTH and SIXTH axes, `--taker-fee-bps` and
/// `--slippage-bps` (see [`GridKind::allowed_taker_fee_bps`] and
/// [`GridKind::required_slippage_bps`]). Both reach `MatchConfig` and re-price
/// every fill, but neither reaches the scenario NAME or the hashed body outside
/// the basis/MN families — so they were the most invisible forge axis of all:
/// - `--grid tier1 --score-source vol-adjusted-return --taker-fee-bps 20`
///   passed all four earlier legs, ran 20 bps fills, and emitted anchor #86's
///   EXACT identity into the frozen momentum dir with no fee row in the body.
/// - `--grid basis-tier1 --taker-fee-bps 5 --slippage-bps 7` is the in-lane
///   variant: anchor #104's exact name and exact `taker_fee_bps` row, filled at
///   a different price.
///
/// Every checked-in invocation pairs all six axes exactly as its grid's LOCKED
/// doc block and anchored `held_constant` / `taker_fee_bps` / `slippage_bps`
/// rows record (see [`GridKind::required_direction`],
/// [`GridKind::required_selection_mode`], [`GridKind::allowed_score_sources`],
/// [`GridKind::required_horizon`], [`GridKind::allowed_taker_fee_bps`],
/// [`GridKind::required_slippage_bps`]), so bailing on mismatches rejects ONLY
/// misinvocations — correct tuples are byte-unchanged.
///
/// # Errors
///
/// On any mismatch, returns a message naming ALL SIX requested axes (direction,
/// selection mode, score source, horizon, taker fee, slippage) and the grid's
/// required tuple, plus an explicit `offending axis: …` line naming the first
/// axis that failed and the value the grid requires there.
#[allow(clippy::too_many_arguments)] // the guard IS the full CLI axis tuple; splitting it re-opens a forge axis
pub fn validate_grid_axis_pairing(
    grid: GridKind,
    direction: SweepDirection,
    selection_mode: SweepSelectionMode,
    score_source: SweepScoreSource,
    horizon: crate::resample::Horizon,
    taker_fee_bps: u32,
    slippage_bps: u32,
) -> Result<(), String> {
    let required_direction = grid.required_direction();
    let required_mode = grid.required_selection_mode();
    let allowed_sources = grid.allowed_score_sources();
    let required_horizon = grid.required_horizon();
    let allowed_fees = grid.allowed_taker_fee_bps();
    let required_slippage = grid.required_slippage_bps();
    let direction_ok = direction == required_direction;
    let mode_ok = selection_mode == required_mode;
    let source_ok = allowed_sources.contains(&score_source);
    let horizon_ok = horizon == required_horizon;
    let fee_ok = allowed_fees.contains(&taker_fee_bps);
    let slippage_ok = slippage_bps == required_slippage;
    if direction_ok && mode_ok && source_ok && horizon_ok && fee_ok && slippage_ok {
        return Ok(());
    }
    // Name the FIRST offending axis explicitly (review 1-18): the full-tuple
    // dump alone made a horizon-only mismatch hard to read at the console.
    let offending = if !direction_ok {
        format!("--direction (requested {direction:?}, required {required_direction:?})")
    } else if !mode_ok {
        format!("--selection-mode (requested {selection_mode:?}, required {required_mode:?})")
    } else if !source_ok {
        format!("--score-source (requested {score_source:?}, required one of {allowed_sources:?})")
    } else if !horizon_ok {
        format!("--horizon (requested {horizon}, required {required_horizon})")
    } else if !fee_ok {
        format!("--taker-fee-bps (requested {taker_fee_bps}, required one of {allowed_fees:?})")
    } else {
        format!("--slippage-bps (requested {slippage_bps}, required {required_slippage})")
    };
    Err(format!(
        "requested axis tuple (--direction {direction:?}, --selection-mode \
         {selection_mode:?}, --score-source {score_source:?}, --horizon {horizon}, \
         --taker-fee-bps {taker_fee_bps}, --slippage-bps {slippage_bps}) does not \
         pair with --grid {grid:?}: that grid requires (direction={required_direction:?}, \
         selection_mode={required_mode:?}, score_source ∈ {allowed_sources:?}, \
         horizon={required_horizon}, taker_fee_bps ∈ {allowed_fees:?}, \
         slippage_bps={required_slippage}). offending axis: {offending}. A mismatched tuple \
         would run one family's behavior — or one fee regime's fills — under another \
         family's anchored scenario name; the forged report shadows the real one as \
         \"latest matching\" and turns the anchors gate falsely RED — refusing to run."
    ))
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
/// Units gloss (review 1-16): the "1w"/"1mo" role glosses in this table (and the
/// momentum Tier-1 table it mirrors) read `lookback_minutes` as BARS under the
/// native 1-bar=1-hour ladder (168 bars ≈ 1 week, 720 bars ≈ 1 month), while the
/// rebalance cadence is real wall-minutes — this doc-level assumption is stated
/// here only and never enters the hashed `grid_def_string`.
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
    pub fn to_strategy_direction(self) -> strategy::Direction {
        match self {
            Self::Momentum => strategy::Direction::Momentum,
            Self::Reversion => strategy::Direction::Reversion,
        }
    }

    /// Label for the scenario name and report.
    pub fn label(self) -> &'static str {
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
    pub fn to_strategy_score_source(self) -> strategy::ScoreSource {
        match self {
            Self::VolAdjustedReturn => strategy::ScoreSource::VolAdjustedReturn,
            Self::Carry | Self::MnFundingSpread => strategy::ScoreSource::FundingCarry,
            Self::BasisReversal | Self::MnBasisSpread => strategy::ScoreSource::BasisReversal,
            Self::MnBasisFundingResidual => strategy::ScoreSource::BasisFundingResidual,
        }
    }

    /// Whether this source needs the funding sidecar loaded (carry or basis).
    pub fn needs_funding(self) -> bool {
        matches!(self, Self::Carry)
    }

    /// Whether this source needs the basis sidecar loaded.
    pub fn needs_basis(self) -> bool {
        matches!(self, Self::BasisReversal)
    }

    /// Whether this is a market-neutral (MN) arm (D-MN.5, M-DEV-5).
    pub fn is_mn(self) -> bool {
        matches!(
            self,
            Self::MnBasisSpread | Self::MnFundingSpread | Self::MnBasisFundingResidual
        )
    }

    /// The short arm-label used in MN scenario names (D-MN.8, M-DEV-5).
    pub fn mn_arm_label(self) -> &'static str {
        match self {
            Self::MnBasisSpread => "basis",
            Self::MnFundingSpread => "funding",
            Self::MnBasisFundingResidual => "basisperp",
            _ => "unknown",
        }
    }

    // Review 1-20 L: a `label()` arm used to live here. It was DEAD — no caller
    // anywhere in the workspace — and it was also WRONG in a dangerous way: it
    // mapped `VolAdjustedReturn | Carry => "carry-fy"`, i.e. it claimed the
    // momentum/MR default source was the carry family. Scenario identity comes
    // from ONE seam, [`build_scenario_name`] (review 1-18 H3), which never
    // called it. Deleted rather than wired: a second, disagreeing naming source
    // is exactly the 1-15 M2 / 1-18 H3 failure mode.
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
    pub fn to_strategy_selection_mode(self) -> strategy::SelectionMode {
        match self {
            Self::CrossSectionalTopK => strategy::SelectionMode::CrossSectionalTopK,
            Self::TimeSeriesLongFlat => strategy::SelectionMode::TimeSeriesLongFlat,
        }
    }

    /// Whether this is the TS-momentum path.
    pub fn is_ts(self) -> bool {
        self == Self::TimeSeriesLongFlat
    }
}

/// ADR-0051 D1 + D6.1: derive per-path seed from master seed and path index.
///
/// SAME-paths rule: `path_seed_{g,j} = derive_path_seed(ensemble_seed, j)`
/// — the same for EVERY cell g. The θ-axis varies config only; the seed is
/// byte-identical to the C2 D1 seed (the 85 anchors hold by construction).
#[inline]
#[must_use]
pub fn derive_path_seed(master: u64, j: usize) -> u64 {
    // Review 1-15 (H3): ONE production formula — delegate to the ADR-0051 D1
    // implementation in `mc_harness` instead of keeping a second copy.
    crate::mc_harness::derive_path_seed(master, j)
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
        let _ = std::fmt::Write::write_fmt(
            &mut s,
            format_args!(
                "  g={} lookback={} k_long={} drift={}\n",
                cell.g,
                cell.lookback_minutes,
                cell.k_long,
                cell.drift()
            ),
        );
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
        let _ = std::fmt::Write::write_fmt(
            &mut s,
            format_args!(
                "  g={} l_settlements={} rebalance_minutes={} k_long={} drift={}\n",
                cell.g,
                cell.lookback_minutes,
                cell.rebalance_minutes_override,
                cell.k_long,
                cell.drift()
            ),
        );
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
        let _ = std::fmt::Write::write_fmt(
            &mut s,
            format_args!(
                "  g={} lookback_bars={} rebalance_minutes={} k_long={} drift={}\n",
                cell.g,
                cell.lookback_minutes,
                cell.rebalance_minutes_override,
                cell.k_long,
                cell.drift()
            ),
        );
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
    use crate::scenarios::montecarlo::{MAINTENANCE_MARGIN_FRAC, MAX_LEVERAGE};
    let mut s = String::from("grid_definition:\n");
    for cell in grid {
        let _ = std::fmt::Write::write_fmt(
            &mut s,
            format_args!(
                "  g={} lookback_bars={} rebalance_minutes={} k_long={} k_short={} drift={} max_leverage={} maintenance_margin_frac={}\n",
                cell.g,
                cell.lookback_minutes,
                cell.rebalance_minutes_override,
                cell.k_long,
                cell.k_long, // k_short = k_long for the MN symmetric split
                cell.drift(),
                MAX_LEVERAGE,
                MAINTENANCE_MARGIN_FRAC,
            ),
        );
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
        let _ = std::fmt::Write::write_fmt(
            &mut s,
            format_args!(
                "  g={} lookback={} entry_threshold={} k_long={} drift={}\n",
                cell.g,
                cell.lookback_minutes,
                cell.entry_threshold(),
                cell.k_long,
                cell.drift()
            ),
        );
    }
    s
}

// ── Per-cell result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CellResult {
    pub cell: ThetaCell,
    pub summary: crate::stats::DistributionSummary,
    pub verdict: ParamRobustnessVerdict,
    /// Total trade count across all N paths (for FP-C3.1 divergence gate).
    pub total_trades: u64,
    /// Total realized funding harvested across all N paths (Decimal).
    /// Non-zero only for carry runs (score_source=Carry). Populated from the
    /// per-path equity delta attributable to funding (D-CARRY.2-LOCKED carry column).
    pub total_funding_harvested: Decimal,
    /// Total time-in-market bars across all N paths (M-DEV-4 / D-TSM.6.4).
    /// Used only when `selection_mode == TimeSeriesLongFlat` to render the
    /// `time_in_market` column (GATED — body-SHAs of momentum/MR/carry unchanged).
    pub total_time_in_market_bars: u64,
    /// Total bars in the run across all N paths (for computing the fraction).
    pub total_bars_run: u64,
    /// Total maintenance-margin liquidations across all N paths (M-DEV-5, MN only).
    /// 0 for all non-MN runs → anchor-neutral by construction.
    pub total_liquidations: u64,
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
#[must_use]
#[allow(clippy::too_many_arguments)]
// Verbatim seam extraction from `bin/param_robustness_sweep.rs` (review 1-15):
// splitting the renderer would diverge the lifted code from its origin,
// defeating the byte-parity argument — mirrors the 1-14 mc_harness R-NR.5
// verbatim-lift rule.
#[allow(clippy::too_many_lines)]
pub fn render_surface_report(
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
    buyhold_summary: &crate::stats::DistributionSummary,
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
    horizon: crate::resample::Horizon,
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
    let is_horizon_run = horizon != crate::resample::Horizon::OneHour;
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
            crate::resample::Horizon::FourHours => "4h",
            crate::resample::Horizon::OneDay => "daily",
            crate::resample::Horizon::OneHour => "1h",
        };
        format!("Time-Series Momentum ({hz} horizon)")
    } else if is_horizon_run && score_source == SweepScoreSource::Carry {
        let hz = match horizon {
            crate::resample::Horizon::FourHours => "4h",
            crate::resample::Horizon::OneDay => "daily",
            crate::resample::Horizon::OneHour => "1h",
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
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!("# {family_label} θ-Surface — Parameter-Robustness Sweep — {scenario}\n\n"),
    );

    // Review 1-15 L3: the gbm-smoke lane is declared VOID by the frozen
    // decision rule (§ 4.1 — only `block-bootstrap-real` output is
    // anchor-grade / decision-grade). Print it as an explicit banner line at
    // the top of the body, not just a trailing Notes bullet. Gated on the gbm
    // label so every real-lane body stays byte-identical.
    if generator_label == "gbm-smoke" {
        body.push_str(
            "**VOID — not anchor-grade (frozen rule §4.1).** This surface was produced by the \
             `gbm-smoke` generator: smoke-test-only output; no decision may be read from it.\n\n",
        );
    }

    // Shared-input header block (every field that is constant across all cells).
    body.push_str("## Ensemble parameters (shared across all θ-cells)\n\n");
    body.push_str(
        "| Field                    | Value                                                   |\n",
    );
    body.push_str(
        "|--------------------------|----------------------------------------------------------|\n",
    );
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!(
            "| master_seed              | 0x{master_seed:X}                                          |\n"
        ),
    );
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!(
            "| fill_seed                | 0x{fill_seed:X}                                          |\n"
        ),
    );
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!(
            "| n_paths                  | {n_paths}                                                 |\n"
        ),
    );
    body.push_str("| sub_seed_rule            | \"master + j*0x9E3779B9 (SAME paths across cells, ADR-0051 D6.1)\" |\n");
    body.push_str("| reduction_rule           | \"index-order mean/std; total_cmp sort; type-7 linear pct\" |\n");
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!(
            "| generator                | {generator_label}                                        |\n"
        ),
    );
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!(
            "| bootstrap_mode           | {bootstrap_mode}                                         |\n"
        ),
    );
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!(
            "| block_length_policy      | {block_length_policy}                                    |\n"
        ),
    );
    if let Some(l) = selected_block_length_l {
        let _ = std::fmt::Write::write_fmt(
            &mut body,
            format_args!(
                "| selected_block_length_L  | {l} (θ-independent — same L for all cells per OQ-3)      |\n"
            ),
        );
    } else {
        body.push_str(
            "| selected_block_length_L  | N/A                                                     |\n",
        );
    }
    let _ = std::fmt::Write::write_fmt(
        &mut body,
        format_args!(
            "| source_revision_sha      | {source_revision_sha}                                    |\n"
        ),
    );
    // M-DEV-3: render the real horizon in the hashed body (D-HR.5 / K3).
    // GATED to horizon runs so the 1h body-SHAs are byte-identical.
    //
    // ⚠ DO NOT "FIX" THE RAGGED COLUMN ALIGNMENT BELOW (review 1-18).
    // The literal padding is INSIDE the hashed body: "4h" (2 chars) and "daily"
    // (5 chars) are followed by a FIXED-width run of spaces, so the closing `|`
    // does not line up with the neighbouring rows and the daily row is 3 columns
    // narrower than the 4h row. That raggedness is baked into the locked
    // body-SHA-256 of FOUR anchors — #92/#93 (ts-horizon-4h 2023/2024) and
    // #94/#95 (ts-horizon-daily) — plus #96..#99 on the carry side. Re-padding
    // to a tidy table changes those bytes and turns `scripts/verify_anchors.sh`
    // RED with no way to re-lock short of the ADR-0038 § D6.b re-emission
    // protocol. Cosmetics are not worth eight anchors.
    if is_horizon_run {
        let _ = std::fmt::Write::write_fmt(
            &mut body,
            format_args!(
                "| horizon                  | {horizon}                                               |\n",
                horizon = match horizon {
                    crate::resample::Horizon::FourHours => "4h",
                    crate::resample::Horizon::OneDay => "daily",
                    crate::resample::Horizon::OneHour => "1h", // unreachable under is_horizon_run
                }
            ),
        );
    }
    // M-DEV-4 (D-BR.LOAD): render the fee level as a hashed body field for basis runs.
    // GATED to `BasisReversal` so the 99 existing anchor body-SHAs are byte-identical.
    // The fee level distinguishes the four fee-level surfaces as DISTINCT anchors.
    // M-DEV-5 (D-MN.8): also render fee for MN runs (same 12-surface anchor scheme).
    if is_basis_run || is_mn_run {
        let _ = std::fmt::Write::write_fmt(
            &mut body,
            format_args!(
                "| taker_fee_bps            | {taker_fee_bps}                                                   |\n"
            ),
        );
        let _ = std::fmt::Write::write_fmt(
            &mut body,
            format_args!(
                "| slippage_bps             | {slippage_bps}                                                    |\n"
            ),
        );
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
            crate::resample::Horizon::FourHours => "4h",
            crate::resample::Horizon::OneDay => "daily",
            crate::resample::Horizon::OneHour => "1h",
        };
        format!(
            "## TS-momentum {hz} θ-grid definition (6-cell, LOCKED § D-HR.4-LOCKED — changing this changes the SHA)\n\n"
        )
    } else if is_horizon_run && score_source == SweepScoreSource::Carry {
        let hz = match horizon {
            crate::resample::Horizon::FourHours => "4h",
            crate::resample::Horizon::OneDay => "daily",
            crate::resample::Horizon::OneHour => "1h",
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

    // Review 1-15 L6: single source — the family flag is computed ONCE by
    // `family_any_non_fragile` and consumed by BOTH this renderer's hashed
    // family line and the bin's console summary (desync is structurally
    // impossible). Byte-identical to the old fold: the flag is true iff any
    // rendered row is non-FRAGILE.
    let any_non_fragile = family_any_non_fragile(cell_results);

    for cr in cell_results {
        let s = &cr.summary;
        let verdict_str = cr.verdict.as_str();
        let spread = s.sharpe.p95 - s.sharpe.p5;
        let c5_flag = if cr.verdict == ParamRobustnessVerdict::Fragile {
            ""
        } else {
            "→ C5 DEFLATION REQUIRED"
        };

        if show_time_in_market {
            // TS-specific: time_in_market column = fraction of bars with ≥1 long position.
            // Computed as total_time_in_market_bars / total_bars_run across all N paths.
            let tim_fraction = if cr.total_bars_run > 0 {
                cr.total_time_in_market_bars as f64 / cr.total_bars_run as f64
            } else {
                0.0
            };
            let _ = std::fmt::Write::write_fmt(
                &mut body,
                format_args!(
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
                ),
            );
        } else if show_basis_trades {
            // Basis-reversal row: includes `rebalance` column (swept in D-BR.2-LOCKED
            // for the cadence/turnover axis) and `trades` (turnover legibility).
            let _ = std::fmt::Write::write_fmt(
                &mut body,
                format_args!(
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
                ),
            );
        } else if show_mn {
            // MN row: includes `rebalance` + `k_short` (symmetric split) + `liquidations`.
            let _ = std::fmt::Write::write_fmt(
                &mut body,
                format_args!(
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
                ),
            );
        } else if show_trades {
            let _ = std::fmt::Write::write_fmt(
                &mut body,
                format_args!(
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
                ),
            );
        } else if show_funding {
            let _ = std::fmt::Write::write_fmt(
                &mut body,
                format_args!(
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
                ),
            );
        } else {
            let _ = std::fmt::Write::write_fmt(
                &mut body,
                format_args!(
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
                ),
            );
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
        let _ = std::fmt::Write::write_fmt(
            &mut body,
            format_args!(
                "| BUYHOLD   | {:.6} | {:.6}  | {:.6}  | {:.6} | {:.6}    | {:.2}%   | {:.6} | (passive — no verdict) |\n",
                s.sharpe.p5,
                s.sharpe.p50,
                s.sharpe.p95,
                s.prob_loss,
                s.prob_sharpe_gt_1,
                s.max_dd_tail_p95 * 100.0,
                spread,
            ),
        );
    }
    body.push('\n');

    // Family-level summary line (the § 0 pre-registration commitment, mechanized).
    // NEVER crowns a "best θ is ROBUST" — only FAMILY-UNIFORM-FRAGILE or FAMILY-HAS-NON-FRAGILE-CELLS.
    // Review 1-15 L6: the line itself comes from `family_verdict_line` — the
    // same single source main's console summary prints (byte-identical output:
    // "<LINE>\n\n" exactly as before).
    body.push_str("## Family verdict\n\n");
    body.push_str(family_verdict_line(any_non_fragile));
    body.push_str("\n\n");
    if any_non_fragile {
        body.push_str(
            "At least one cell is MARGINAL or ROBUST. Per the § 0 pre-registration commitment:\n",
        );
        body.push_str("C3 makes NO 'this θ is robust' claim. Each non-FRAGILE cell is flagged '→ C5 DEFLATION REQUIRED'\n");
        body.push_str("and is handed to the C5 PBO/Deflated-Sharpe pass before any promotion.\n");
    } else {
        body.push_str("Every active θ-cell is FRAGILE under the frozen decision-rule bands.\n");
        body.push_str("No multiple-testing correction is needed for a uniform-negative result:\n");
        body.push_str(
            "C3 is not selecting a winner — it is reporting that no cell cleared the bar.\n",
        );
        if selection_mode.is_ts() {
            if is_horizon_run {
                let hz = match horizon {
                    crate::resample::Horizon::FourHours => "4h",
                    crate::resample::Horizon::OneDay => "daily",
                    crate::resample::Horizon::OneHour => "1h",
                };
                let _ = std::fmt::Write::write_fmt(
                    &mut body,
                    format_args!(
                        "Conclusion: v1 time-series momentum at the {hz} horizon (per-asset long/flat on own trailing return) is\n"
                    ),
                );
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
                            crate::resample::Horizon::FourHours => "4h",
                            crate::resample::Horizon::OneDay => "daily",
                            crate::resample::Horizon::OneHour => "1h",
                        };
                        let _ = std::fmt::Write::write_fmt(
                            &mut body,
                            format_args!(
                                "Conclusion: v1 cross-sectional carry (funding) at the {hz} horizon is structurally fragile across the\n"
                            ),
                        );
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
                    let _ = std::fmt::Write::write_fmt(
                        &mut body,
                        format_args!(
                            "Conclusion: v1 cross-sectional basis-reversal at {taker_fee_bps} bps taker fee is structurally fragile\n"
                        ),
                    );
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
                    let _ = std::fmt::Write::write_fmt(
                        &mut body,
                        format_args!(
                            "Conclusion: v2 market-neutral {arm_label} spread at {taker_fee_bps} bps taker fee is structurally fragile\n"
                        ),
                    );
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

// ── Family verdict — single source (review 1-15 L6) ──────────────────────────────

/// Whether ANY cell in the surface is non-FRAGILE (MARGINAL or ROBUST).
///
/// Review 1-15 L6: this is THE single source for the family verdict. The
/// renderer's hashed family line AND the bin's console summary both consume it
/// (previously each recomputed the flag independently — a future desync risk
/// between the console and the hashed body).
#[must_use]
pub fn family_any_non_fragile(cell_results: &[CellResult]) -> bool {
    cell_results
        .iter()
        .any(|cr| cr.verdict != ParamRobustnessVerdict::Fragile)
}

/// The § R2.3 family-verdict line for a given `family_any_non_fragile` flag.
///
/// The renderer NEVER emits a "best θ is ROBUST" claim — the family line is
/// always exactly one of these two pre-registered values (FP-C3.5).
#[must_use]
pub fn family_verdict_line(any_non_fragile: bool) -> &'static str {
    if any_non_fragile {
        "FAMILY-HAS-NON-FRAGILE-CELLS"
    } else {
        "FAMILY-UNIFORM-FRAGILE"
    }
}

// ── Scenario identity (review 1-15 M2 + L3) ────────────────────────────────────

/// Build the sweep's scenario identity string (lifted verbatim from the bin's
/// `main`, review 1-15; the arm structure and every anchored template are
/// byte-identical).
///
/// `generator_token` is the honest generator segment of the name:
/// - `"block-bootstrap-real"` for the anchored real lane (BYTE-IDENTICAL to
///   every locked anchor name in `evidence/anchors.toml`), and
/// - `"gbm-smoke"` for the smoke lane (review 1-15 L3 — the old name embedded
///   a false `block-bootstrap-gbm` token although the gbm lane runs no
///   bootstrap; the lane has never been anchored, so the rename is anchor-safe).
///
/// The grid-kind discriminator (review 1-15 M2) is appended LAST: `""` for
/// every LOCKED production grid (names byte-unchanged), `-grid-twocell` for
/// the FP-C3.2-only two-cell mini-grid (so a probe run can never shadow the
/// anchored tier-1 report as "latest matching").
#[must_use]
#[allow(clippy::too_many_arguments)] // scenario identity is a pure function of the full CLI axis tuple
pub fn build_scenario_name(
    grid: GridKind,
    direction: SweepDirection,
    score_source: SweepScoreSource,
    selection_mode: SweepSelectionMode,
    horizon: crate::resample::Horizon,
    year: i32,
    generator_token: &str,
    taker_fee_bps: u32,
) -> String {
    let is_horizon_run = horizon != crate::resample::Horizon::OneHour;
    let horizon_label = match horizon {
        crate::resample::Horizon::OneHour => "",
        crate::resample::Horizon::FourHours => "4h",
        crate::resample::Horizon::OneDay => "daily",
    };
    let base = if is_horizon_run && selection_mode.is_ts() {
        // Horizon TS run: e.g. "v1-ts-horizon-4h-theta-surface-2023-block-bootstrap-real-fy"
        format!("v1-ts-horizon-{horizon_label}-theta-surface-{year}-{generator_token}-fy")
    } else if is_horizon_run && score_source == SweepScoreSource::Carry {
        // Horizon carry run: e.g. "v1-carry-horizon-4h-theta-surface-2023-block-bootstrap-real-fy"
        format!("v1-carry-horizon-{horizon_label}-theta-surface-{year}-{generator_token}-fy")
    } else if selection_mode.is_ts() {
        format!("v1-ts-momentum-theta-surface-{year}-{generator_token}-fy")
    } else {
        match score_source {
            SweepScoreSource::Carry => {
                format!("v1-carry-theta-surface-{year}-{generator_token}-fy")
            }
            // M-DEV-5 (D-BR.9): basis-reversal scenario name carries the fee level
            // as a zero-padded two-digit number so the fee surfaces are DISTINCT anchors.
            SweepScoreSource::BasisReversal => format!(
                "v1-basis-reversal-fee{taker_fee_bps:02}bps-theta-surface-{year}-{generator_token}-fy"
            ),
            // M-DEV-5 (D-MN.8): MN scenario name carries the arm label + fee level.
            SweepScoreSource::MnBasisSpread
            | SweepScoreSource::MnFundingSpread
            | SweepScoreSource::MnBasisFundingResidual => format!(
                "v2-mn-{arm}-fee{taker_fee_bps:02}bps-theta-surface-{year}-{generator_token}-fy",
                arm = score_source.mn_arm_label(),
            ),
            SweepScoreSource::VolAdjustedReturn => format!(
                "v1-{family}-theta-surface-{year}-{generator_token}-fy",
                family = direction.label(),
            ),
        }
    };
    // Review 1-15 M2: the grid-kind discriminator — empty for every LOCKED
    // production grid (anchored names byte-identical), non-empty for the
    // never-anchored probe grids.
    format!("{base}{disc}", disc = grid.scenario_discriminator())
}
