//! Gate-tied hyperparameter sweep — the Tune screen FORM state (ADR-0069 T6/T10).
//!
//! This is the pure, testable state half of the Tune screen: the range form
//! (family picker + per-axis `{min, max, step}` inputs + presets), the live
//! grid-size estimate, and the run lifecycle (`begin_run` / `set_progress` /
//! `finish_run`). It mirrors [`crate::leaderboard::LeaderboardScreenState`]
//! one-for-one: the same `PanelState<…>` + `running` + `progress` shape, the
//! same lifecycle method names, so the binary-side `Task` wiring is identical.
//!
//! ## Purity discipline (the layering seam)
//!
//! Every transition here is pure (no I/O, no `Task`, no engine call). The async
//! dispatch (`run_param_sweep` on the side-thread runtime) lives ONLY in the
//! binary (`bin/cockpit_live.rs`) + the `tune::runner` glue, exactly as the
//! leaderboard splits `BakeoffRunRequested` (pure: `begin_run`) from the binary's
//! `spawn_bakeoff`. The form holds *strings* the operator typed (round-tripped
//! verbatim) and parses them at render/dispatch time — never an engine type.
//!
//! ## All four families are runnable (T7b)
//!
//! The family picker presents all four families (SMA / MACD / RSI / Bollinger)
//! and every one is *runnable*: the engine's `run_param_sweep` enumerates the
//! `MacdGrid` / `RsiGrid` / `BollingerGrid` faithfully (proven by the engine's
//! identity guards), so each family carries its own `{min, max, step}` axis form
//! here, centred on the shipped params. Selecting a composed family renders ITS
//! axes (not the SMA ones); [`TuneFamily::is_runnable`] returns `true` for all.

use smol_str::SmolStr;

use crate::state::PanelState;
use crate::tune::state::SweepReportMirror;

/// The strategy family the operator is tuning. UI-side closed enum mirroring
/// `backtest::SweepFamily` — the picker never matches on the engine type.
///
/// All four variants are present so the picker shows the full menu; every one is
/// *runnable* (T7b) — the engine's `run_param_sweep` sweeps each family's grid
/// faithfully. [`TuneFamily::is_runnable`] returns `true` for all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TuneFamily {
    /// SMA crossover (`fast_len` / `slow_len`). The existing `ScenarioConfig`
    /// override seam.
    #[default]
    Sma,
    /// MACD (fast / slow / signal).
    Macd,
    /// RSI (period / oversold).
    Rsi,
    /// Bollinger bands (period / k).
    Bollinger,
}

impl TuneFamily {
    /// The families in picker order — SMA first (the runnable, default family).
    pub const ALL: &'static [TuneFamily] = &[
        TuneFamily::Sma,
        TuneFamily::Macd,
        TuneFamily::Rsi,
        TuneFamily::Bollinger,
    ];

    /// `true` when this family can actually be swept. All four families are
    /// runnable (T7b) — the engine's `run_param_sweep` enumerates each family's
    /// grid faithfully. The Run button reads this (alongside the grid estimate);
    /// the form is what actually gates a malformed/empty grid.
    #[must_use]
    pub fn is_runnable(self) -> bool {
        matches!(
            self,
            TuneFamily::Sma | TuneFamily::Macd | TuneFamily::Rsi | TuneFamily::Bollinger
        )
    }

    /// Map to the engine `backtest::SweepFamily`. Used only by the runner glue
    /// at the dispatch boundary (the ONE place an engine type is named).
    #[must_use]
    pub fn to_engine(self) -> backtest::SweepFamily {
        match self {
            TuneFamily::Sma => backtest::SweepFamily::Sma,
            TuneFamily::Macd => backtest::SweepFamily::Macd,
            TuneFamily::Rsi => backtest::SweepFamily::Rsi,
            TuneFamily::Bollinger => backtest::SweepFamily::Bollinger,
        }
    }
}

/// Which `{min, max, step}` axis an edit/preset targets — one closed enum across
/// EVERY family's integer axes (SMA fast/slow, MACD fast/slow/signal, RSI
/// period/oversold, Bollinger period). The Bollinger `k` multi-select is NOT a
/// `{min, max, step}` axis, so it is addressed by its own message, not here.
///
/// A single closed enum (rather than per-family sibling enums) keeps the message
/// payloads + the `update` arms uniform: one `SweepAxisEdit { axis, field, value }`
/// shape handles all 8 axes, and the form dispatches on the axis to the right
/// sub-form. Each variant knows its [`TuneFamily`] via [`TuneAxisKind::family`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuneAxisKind {
    /// SMA fast-window axis (shipped default 20).
    SmaFast,
    /// SMA slow-window axis (shipped default 50).
    SmaSlow,
    /// MACD fast-EMA-period axis (shipped default 12).
    MacdFast,
    /// MACD slow-EMA-period axis (shipped default 26).
    MacdSlow,
    /// MACD signal-period axis (shipped default 9).
    MacdSignal,
    /// RSI lookback-period axis (shipped default 14).
    RsiPeriod,
    /// RSI oversold-threshold axis (shipped default 30).
    RsiOversold,
    /// Bollinger lookback-period axis (shipped default 20).
    BollingerPeriod,
}

impl TuneAxisKind {
    /// The family this axis belongs to — lets the form route an edit to the
    /// correct sub-form without the caller knowing the mapping.
    #[must_use]
    pub fn family(self) -> TuneFamily {
        match self {
            TuneAxisKind::SmaFast | TuneAxisKind::SmaSlow => TuneFamily::Sma,
            TuneAxisKind::MacdFast | TuneAxisKind::MacdSlow | TuneAxisKind::MacdSignal => {
                TuneFamily::Macd
            }
            TuneAxisKind::RsiPeriod | TuneAxisKind::RsiOversold => TuneFamily::Rsi,
            TuneAxisKind::BollingerPeriod => TuneFamily::Bollinger,
        }
    }
}

/// Which `{min, max, step}` field of an axis an edit targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisField {
    /// The inclusive minimum.
    Min,
    /// The inclusive maximum.
    Max,
    /// The step (≥ 1).
    Step,
}

/// One axis preset — a narrow / shipped / wide one-click range so the common
/// case is one tap and the cap is rarely hit (ADR-0069 § 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPreset {
    /// A tight band around the shipped value (few cells).
    Narrow,
    /// The shipped default band (the centre the bake-off already uses).
    Shipped,
    /// A wide exploratory band.
    Wide,
}

impl AxisPreset {
    /// The presets in chip order.
    pub const ALL: &'static [AxisPreset] =
        &[AxisPreset::Narrow, AxisPreset::Shipped, AxisPreset::Wide];
}

/// The raw, round-tripped text for one `{min, max, step}` axis. Strings (not
/// `u32`) so the operator's keystrokes survive verbatim and a blank/partial
/// field never snaps to a default mid-edit (the leaderboard `budget_input`
/// discipline). Parsed at render/dispatch time via [`AxisInput::parsed`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisInput {
    /// Raw min text.
    pub min: String,
    /// Raw max text.
    pub max: String,
    /// Raw step text.
    pub step: String,
}

impl AxisInput {
    /// Build from concrete `{min, max, step}` integers (used to seed presets).
    #[must_use]
    pub fn from_values(min: u32, max: u32, step: u32) -> Self {
        Self {
            min: min.to_string(),
            max: max.to_string(),
            step: step.to_string(),
        }
    }

    /// Parse the three fields to `(min, max, step)`. `None` for any field that
    /// is blank or not a non-negative integer. Pure; total.
    #[must_use]
    pub fn parsed(&self) -> (Option<u32>, Option<u32>, Option<u32>) {
        (
            self.min.trim().parse::<u32>().ok(),
            self.max.trim().parse::<u32>().ok(),
            self.step.trim().parse::<u32>().ok(),
        )
    }

    /// Set one field's raw text.
    pub fn set(&mut self, field: AxisField, value: String) {
        match field {
            AxisField::Min => self.min = value,
            AxisField::Max => self.max = value,
            AxisField::Step => self.step = value,
        }
    }
}

/// The SMA grid form — the two `{min, max, step}` axes the operator edits.
///
/// `fast_len` shipped default centred on 20, `slow_len` on 50 (the divergence
/// anchor). Presets seed these; manual edits round-trip through [`AxisInput`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmaGridForm {
    /// The fast-window axis input.
    pub fast: AxisInput,
    /// The slow-window axis input.
    pub slow: AxisInput,
}

impl Default for SmaGridForm {
    /// The shipped default grid — `fast 10..30 step 5`, `slow 30..70 step 10`
    /// (matching the engine's `SweepAxis::sma_fast_default` / `sma_slow_default`
    /// so the form's default mirrors the engine's default grid).
    fn default() -> Self {
        Self {
            fast: AxisInput::from_values(10, 30, 5),
            slow: AxisInput::from_values(30, 70, 10),
        }
    }
}

impl SmaGridForm {
    /// Apply a preset to one axis. The presets are deliberately conservative —
    /// even `Wide` stays inside the engine's `1 ≤ fast < slow ≤ 400` guard and
    /// keeps the cell count near the cap. A non-SMA axis is a no-op (the caller
    /// routes by [`TuneAxisKind::family`], so this never fires off-family).
    pub fn apply_preset(&mut self, axis: TuneAxisKind, preset: AxisPreset) {
        let input = match (axis, preset) {
            // Fast axis presets — centred on the shipped 20.
            (TuneAxisKind::SmaFast, AxisPreset::Narrow) => AxisInput::from_values(15, 25, 5),
            (TuneAxisKind::SmaFast, AxisPreset::Shipped) => AxisInput::from_values(10, 30, 5),
            (TuneAxisKind::SmaFast, AxisPreset::Wide) => AxisInput::from_values(5, 40, 5),
            // Slow axis presets — centred on the shipped 50.
            (TuneAxisKind::SmaSlow, AxisPreset::Narrow) => AxisInput::from_values(40, 60, 10),
            (TuneAxisKind::SmaSlow, AxisPreset::Shipped) => AxisInput::from_values(30, 70, 10),
            (TuneAxisKind::SmaSlow, AxisPreset::Wide) => AxisInput::from_values(30, 100, 10),
            // Off-family axis — no-op (defensive; the router never sends these here).
            _ => return,
        };
        match axis {
            TuneAxisKind::SmaFast => self.fast = input,
            TuneAxisKind::SmaSlow => self.slow = input,
            _ => {}
        }
    }

    /// Edit one field of one axis (round-tripped verbatim). A non-SMA axis is a
    /// no-op (the caller routes by family).
    pub fn edit(&mut self, axis: TuneAxisKind, field: AxisField, value: String) {
        match axis {
            TuneAxisKind::SmaFast => self.fast.set(field, value),
            TuneAxisKind::SmaSlow => self.slow.set(field, value),
            _ => {}
        }
    }
}

/// A pure estimate of the grid the current SMA form would produce, computed
/// without the engine so the form's "N configs → ~M runs" readout updates live
/// and the Run button can disable on an empty/invalid grid.
///
/// Enumerates the same axis-major cartesian product the engine's
/// `SmaGrid::enumerate_valid` does, applies the same `1 ≤ fast < slow ≤ 400`
/// validity guard, and caps at `backtest::MAX_SWEEP_CONFIGS` — so the readout
/// matches what `run_param_sweep` will actually run (no surprise truncation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridEstimate {
    /// Valid cells AFTER the cap (= what the sweep runs).
    pub runnable: usize,
    /// Valid cells BEFORE the cap (the cartesian product minus invalid cells).
    pub valid_total: usize,
    /// Cells dropped because `fast >= slow` (or out of `[1, 400]`).
    pub invalid: usize,
    /// `true` when `valid_total > cap` — the form will show the truncation note.
    pub truncated: bool,
    /// `true` when at least one axis field is blank / unparseable — Run disables
    /// with a "fill the ranges" prompt rather than running a malformed grid.
    pub has_blank_field: bool,
}

impl GridEstimate {
    /// `true` when there is at least one runnable cell AND no blank field — the
    /// precondition for enabling the Run button.
    #[must_use]
    pub fn is_runnable(self) -> bool {
        self.runnable > 0 && !self.has_blank_field
    }
}

/// Compute the [`GridEstimate`] for an SMA form. Pure; total; no engine call.
///
/// `cap` is `backtest::MAX_SWEEP_CONFIGS` (passed in so the function is unit-
/// testable without the engine and there is a single source of truth at the
/// call site). Caps the runnable count at `cap`; mirrors the engine's
/// axis-major enumeration + `1 ≤ fast < slow ≤ 400` guard exactly.
#[must_use]
pub fn estimate_sma_grid(form: &SmaGridForm, cap: usize) -> GridEstimate {
    let (fmin, fmax, fstep) = form.fast.parsed();
    let (smin, smax, sstep) = form.slow.parsed();

    let has_blank_field = fmin.is_none()
        || fmax.is_none()
        || fstep.is_none()
        || smin.is_none()
        || smax.is_none()
        || sstep.is_none();

    // Enumerate each axis with the engine's `step.max(1)` + `<= max` rule.
    let fast_vals = axis_values(fmin, fmax, fstep);
    let slow_vals = axis_values(smin, smax, sstep);

    let cartesian = fast_vals.len().saturating_mul(slow_vals.len());
    let mut valid_total = 0usize;
    for &f in &fast_vals {
        for &s in &slow_vals {
            if f >= 1 && f < s && s <= 400 {
                valid_total += 1;
            }
        }
    }
    grid_estimate(cartesian, valid_total, cap, has_blank_field)
}

/// Enumerate one axis `min, min+step, … ≤ max`. Empty when any field is blank
/// or `min > max`. Bounded (`take(512)`) so a pathological `1..400 step 1` form
/// can't blow up the live readout — well above the cap, so the truncation note
/// still fires correctly.
fn axis_values(min: Option<u32>, max: Option<u32>, step: Option<u32>) -> Vec<u32> {
    let (Some(min), Some(max), Some(step)) = (min, max, step) else {
        return Vec::new();
    };
    let step = step.max(1);
    let mut v = Vec::new();
    let mut x = min;
    while x <= max && v.len() < 512 {
        v.push(x);
        x = x.saturating_add(step);
    }
    v
}

// ── MACD form ─────────────────────────────────────────────────────────────────

/// The MACD grid form — three `{min, max, step}` axes (fast / slow / signal).
///
/// Centred on the shipped MACD config (fast=12, slow=26, signal=9), mirroring
/// the engine's [`backtest::MacdGrid::default`] grid so the form's default is the
/// engine's default. Manual edits round-trip through [`AxisInput`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacdGridForm {
    /// Fast EMA period axis.
    pub fast: AxisInput,
    /// Slow EMA period axis.
    pub slow: AxisInput,
    /// Signal smoothing period axis.
    pub signal: AxisInput,
}

impl Default for MacdGridForm {
    /// The shipped default grid — `fast 8..16 step 4`, `slow 20..32 step 6`,
    /// `signal 7..11 step 2` (matching `backtest::MacdGrid::default`).
    fn default() -> Self {
        Self {
            fast: AxisInput::from_values(8, 16, 4),
            slow: AxisInput::from_values(20, 32, 6),
            signal: AxisInput::from_values(7, 11, 2),
        }
    }
}

impl MacdGridForm {
    /// Edit one field of one axis (round-tripped verbatim). A non-MACD axis is a
    /// no-op (the caller routes by family).
    pub fn edit(&mut self, axis: TuneAxisKind, field: AxisField, value: String) {
        match axis {
            TuneAxisKind::MacdFast => self.fast.set(field, value),
            TuneAxisKind::MacdSlow => self.slow.set(field, value),
            TuneAxisKind::MacdSignal => self.signal.set(field, value),
            _ => {}
        }
    }

    /// Apply a narrow/shipped/wide preset to one axis (centred on the shipped
    /// fast=12 / slow=26 / signal=9). A non-MACD axis is a no-op.
    pub fn apply_preset(&mut self, axis: TuneAxisKind, preset: AxisPreset) {
        let input = match (axis, preset) {
            (TuneAxisKind::MacdFast, AxisPreset::Narrow) => AxisInput::from_values(10, 14, 2),
            (TuneAxisKind::MacdFast, AxisPreset::Shipped) => AxisInput::from_values(8, 16, 4),
            (TuneAxisKind::MacdFast, AxisPreset::Wide) => AxisInput::from_values(6, 18, 3),
            (TuneAxisKind::MacdSlow, AxisPreset::Narrow) => AxisInput::from_values(22, 30, 4),
            (TuneAxisKind::MacdSlow, AxisPreset::Shipped) => AxisInput::from_values(20, 32, 6),
            (TuneAxisKind::MacdSlow, AxisPreset::Wide) => AxisInput::from_values(18, 40, 4),
            (TuneAxisKind::MacdSignal, AxisPreset::Narrow) => AxisInput::from_values(8, 10, 2),
            (TuneAxisKind::MacdSignal, AxisPreset::Shipped) => AxisInput::from_values(7, 11, 2),
            (TuneAxisKind::MacdSignal, AxisPreset::Wide) => AxisInput::from_values(5, 13, 2),
            _ => return,
        };
        match axis {
            TuneAxisKind::MacdFast => self.fast = input,
            TuneAxisKind::MacdSlow => self.slow = input,
            TuneAxisKind::MacdSignal => self.signal = input,
            _ => {}
        }
    }
}

/// Compute the [`GridEstimate`] for a MACD form. Pure; total; no engine call.
///
/// Mirrors `backtest::MacdGrid::enumerate_valid` exactly: the cartesian product
/// of the three axes, dropping triples where NOT (`fast >= 1 && fast < slow &&
/// slow <= 400 && signal >= 1`), capped at `cap`.
#[must_use]
pub fn estimate_macd_grid(form: &MacdGridForm, cap: usize) -> GridEstimate {
    let (fmin, fmax, fstep) = form.fast.parsed();
    let (smin, smax, sstep) = form.slow.parsed();
    let (gmin, gmax, gstep) = form.signal.parsed();

    let has_blank_field = [fmin, fmax, fstep, smin, smax, sstep, gmin, gmax, gstep]
        .iter()
        .any(Option::is_none);

    let fast_vals = axis_values(fmin, fmax, fstep);
    let slow_vals = axis_values(smin, smax, sstep);
    let sig_vals = axis_values(gmin, gmax, gstep);

    let cartesian = fast_vals
        .len()
        .saturating_mul(slow_vals.len())
        .saturating_mul(sig_vals.len());
    let mut valid_total = 0usize;
    for &f in &fast_vals {
        for &s in &slow_vals {
            for &g in &sig_vals {
                if f >= 1 && f < s && s <= 400 && g >= 1 {
                    valid_total += 1;
                }
            }
        }
    }
    grid_estimate(cartesian, valid_total, cap, has_blank_field)
}

// ── RSI form ──────────────────────────────────────────────────────────────────

/// The RSI grid form — a `{min, max, step}` period axis × an oversold-threshold
/// axis. Mirrors the engine's [`backtest::RsiGrid`] (oversold is swept as a
/// discrete integer threshold, NOT a single value), centred on the shipped
/// config (period=14, oversold=30).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsiGridForm {
    /// RSI lookback period axis.
    pub period: AxisInput,
    /// Oversold threshold axis (integer; `rsi < oversold` fires entry).
    pub oversold: AxisInput,
}

impl Default for RsiGridForm {
    /// The shipped default grid — `period 10..18 step 4`, `oversold 25..35 step 5`
    /// (matching `backtest::RsiGrid::default`).
    fn default() -> Self {
        Self {
            period: AxisInput::from_values(10, 18, 4),
            oversold: AxisInput::from_values(25, 35, 5),
        }
    }
}

impl RsiGridForm {
    /// Edit one field of one axis (round-tripped). A non-RSI axis is a no-op.
    pub fn edit(&mut self, axis: TuneAxisKind, field: AxisField, value: String) {
        match axis {
            TuneAxisKind::RsiPeriod => self.period.set(field, value),
            TuneAxisKind::RsiOversold => self.oversold.set(field, value),
            _ => {}
        }
    }

    /// Apply a narrow/shipped/wide preset to one axis. A non-RSI axis is a no-op.
    pub fn apply_preset(&mut self, axis: TuneAxisKind, preset: AxisPreset) {
        let input = match (axis, preset) {
            (TuneAxisKind::RsiPeriod, AxisPreset::Narrow) => AxisInput::from_values(12, 16, 2),
            (TuneAxisKind::RsiPeriod, AxisPreset::Shipped) => AxisInput::from_values(10, 18, 4),
            (TuneAxisKind::RsiPeriod, AxisPreset::Wide) => AxisInput::from_values(6, 22, 4),
            (TuneAxisKind::RsiOversold, AxisPreset::Narrow) => AxisInput::from_values(28, 32, 2),
            (TuneAxisKind::RsiOversold, AxisPreset::Shipped) => AxisInput::from_values(25, 35, 5),
            (TuneAxisKind::RsiOversold, AxisPreset::Wide) => AxisInput::from_values(20, 40, 5),
            _ => return,
        };
        match axis {
            TuneAxisKind::RsiPeriod => self.period = input,
            TuneAxisKind::RsiOversold => self.oversold = input,
            _ => {}
        }
    }
}

/// Compute the [`GridEstimate`] for an RSI form. Pure; total; no engine call.
///
/// Mirrors `backtest::RsiGrid::enumerate_valid`: the cartesian product of the
/// period × oversold axes, dropping pairs where NOT (`period >= 2 && 1 <=
/// oversold <= 49`), capped at `cap`.
#[must_use]
pub fn estimate_rsi_grid(form: &RsiGridForm, cap: usize) -> GridEstimate {
    let (pmin, pmax, pstep) = form.period.parsed();
    let (omin, omax, ostep) = form.oversold.parsed();

    let has_blank_field = [pmin, pmax, pstep, omin, omax, ostep]
        .iter()
        .any(Option::is_none);

    let period_vals = axis_values(pmin, pmax, pstep);
    let os_vals = axis_values(omin, omax, ostep);

    let cartesian = period_vals.len().saturating_mul(os_vals.len());
    let mut valid_total = 0usize;
    for &p in &period_vals {
        for &os in &os_vals {
            if p >= 2 && (1..=49).contains(&os) {
                valid_total += 1;
            }
        }
    }
    grid_estimate(cartesian, valid_total, cap, has_blank_field)
}

// ── Bollinger form ────────────────────────────────────────────────────────────

/// The four discrete `k` band-multiplier presets the Bollinger grid sweeps —
/// MIRRORS `backtest::BollingerGrid::default().k_presets` (`{1.5, 2.0, 2.5, 3.0}`,
/// Decimal-exact, no float-step drift). Indexed by [`BollingerGridForm.k_selected`].
pub const BOLLINGER_K_PRESETS: [&str; 4] = ["1.5", "2.0", "2.5", "3.0"];

/// The index of the shipped `k = 2.0` preset (selected by default).
const SHIPPED_BBANDS_K_INDEX: usize = 1;

/// The Bollinger grid form — a `{min, max, step}` period axis × a MULTI-SELECT
/// over the four `k` presets (the engine's `k_presets` list, not a `{min, max,
/// step}` axis — `k` is a Decimal preset list to avoid float-step drift).
///
/// Centred on the shipped config (period=20, k=2.0). `k_selected[i]` toggles the
/// i-th [`BOLLINGER_K_PRESETS`] entry; at least one must stay selected to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BollingerGridForm {
    /// Lookback period axis.
    pub period: AxisInput,
    /// Which of the four `k` presets are selected (parallel to
    /// [`BOLLINGER_K_PRESETS`]). Default: only the shipped `k = 2.0`.
    pub k_selected: [bool; 4],
}

impl Default for BollingerGridForm {
    /// The shipped default grid — `period 14..26 step 6` (matching
    /// `backtest::BollingerGrid::default`) and only the shipped `k = 2.0` preset
    /// checked (a one-cell-per-period default the operator widens by ticking more
    /// `k` presets).
    fn default() -> Self {
        let mut k_selected = [false; 4];
        k_selected[SHIPPED_BBANDS_K_INDEX] = true;
        Self {
            period: AxisInput::from_values(14, 26, 6),
            k_selected,
        }
    }
}

impl BollingerGridForm {
    /// Edit one field of the period axis (round-tripped). Only the period axis
    /// exists for Bollinger; a non-Bollinger axis is a no-op.
    pub fn edit(&mut self, axis: TuneAxisKind, field: AxisField, value: String) {
        if matches!(axis, TuneAxisKind::BollingerPeriod) {
            self.period.set(field, value);
        }
    }

    /// Apply a narrow/shipped/wide preset to the period axis. A non-Bollinger
    /// axis is a no-op.
    pub fn apply_preset(&mut self, axis: TuneAxisKind, preset: AxisPreset) {
        if !matches!(axis, TuneAxisKind::BollingerPeriod) {
            return;
        }
        self.period = match preset {
            AxisPreset::Narrow => AxisInput::from_values(16, 24, 4),
            AxisPreset::Shipped => AxisInput::from_values(14, 26, 6),
            AxisPreset::Wide => AxisInput::from_values(10, 30, 4),
        };
    }

    /// Toggle the i-th `k` preset. Out-of-range indices are ignored (defensive).
    pub fn toggle_k(&mut self, index: usize) {
        if let Some(slot) = self.k_selected.get_mut(index) {
            *slot = !*slot;
        }
    }

    /// The selected `k` presets as Decimals (the engine's `k_presets` list).
    #[must_use]
    pub fn selected_k_decimals(&self) -> Vec<rust_decimal::Decimal> {
        use std::str::FromStr;
        self.k_selected
            .iter()
            .enumerate()
            .filter(|&(_, &on)| on)
            .filter_map(|(i, _)| rust_decimal::Decimal::from_str(BOLLINGER_K_PRESETS[i]).ok())
            .collect()
    }

    /// How many `k` presets are currently selected.
    #[must_use]
    pub fn k_count(&self) -> usize {
        self.k_selected.iter().filter(|&&on| on).count()
    }
}

/// Compute the [`GridEstimate`] for a Bollinger form. Pure; total; no engine call.
///
/// Mirrors `backtest::BollingerGrid::enumerate_valid`: the cartesian product of
/// the period axis times the SELECTED `k` presets, dropping pairs that fail the
/// `period >= 2` guard (every preset `k` is already `> 0`), capped at `cap`.
/// Zero selected `k` presets yields zero cells (Run disables).
#[must_use]
pub fn estimate_bollinger_grid(form: &BollingerGridForm, cap: usize) -> GridEstimate {
    let (pmin, pmax, pstep) = form.period.parsed();

    // A blank period field OR zero selected k presets blocks the run (the latter
    // is treated as "has_blank_field" so the readout prompts to pick a k).
    let has_blank_field =
        pmin.is_none() || pmax.is_none() || pstep.is_none() || form.k_count() == 0;

    let period_vals = axis_values(pmin, pmax, pstep);
    let k_count = form.k_count();

    let cartesian = period_vals.len().saturating_mul(k_count);
    let mut valid_total = 0usize;
    for &p in &period_vals {
        if p >= 2 {
            valid_total += k_count;
        }
    }
    grid_estimate(cartesian, valid_total, cap, has_blank_field)
}

/// Assemble a [`GridEstimate`] from the raw cartesian + valid counts (the common
/// tail of every per-family estimate: invalid = cartesian − valid, cap the
/// runnable, set the truncation flag). Pure.
fn grid_estimate(
    cartesian: usize,
    valid_total: usize,
    cap: usize,
    has_blank_field: bool,
) -> GridEstimate {
    let invalid = cartesian.saturating_sub(valid_total);
    let runnable = valid_total.min(cap);
    let truncated = valid_total > cap;
    GridEstimate {
        runnable,
        valid_total,
        invalid,
        truncated,
        has_blank_field,
    }
}

/// The Tune screen state — the form selection + the run lifecycle.
///
/// Mirrors [`crate::leaderboard::LeaderboardScreenState`]: a `result`
/// `PanelState`, a `running` token (disables Run mid-sweep), and the latest
/// `progress` (drives the determinate bar). The form fields (`family`,
/// `sma_grid`) round-trip the operator's selection.
#[derive(Debug, Clone)]
pub struct TuneScreenState {
    /// The sweep result.
    ///
    /// - `Empty` — cold start, no run yet (the "set ranges and press Run sweep"
    ///   prompt).
    /// - `Loading` — a sweep is in flight (the determinate bar + calm body).
    /// - `Ready(mirror)` — the result grid (cells + baseline + benchmark).
    /// - `Error(msg)` — the run failed (operator-friendly reason).
    pub result: PanelState<SweepReportMirror>,
    /// Whether a sweep is currently in flight. Guards double-dispatch — Run is
    /// disabled while `true` (the leaderboard `running` token).
    pub running: bool,
    /// The latest cell-level sweep progress, mirrored as the raw `backtest`
    /// `BakeoffProgress` (the sweep reuses the bake-off progress wire type).
    /// `Some` once the first event lands; drives the determinate bar; cleared in
    /// `finish_run`.
    pub progress: Option<backtest::progress::BakeoffProgress>,

    // ── The range form ───────────────────────────────────────────────────────
    /// The family being tuned (default SMA).
    pub family: TuneFamily,
    /// The SMA `{min, max, step}` axis form.
    pub sma_grid: SmaGridForm,
    /// The MACD `{fast, slow, signal}` axis form.
    pub macd_grid: MacdGridForm,
    /// The RSI `{period, oversold}` axis form.
    pub rsi_grid: RsiGridForm,
    /// The Bollinger `{period}` axis + `k`-preset multi-select form.
    pub bollinger_grid: BollingerGridForm,
}

impl Default for TuneScreenState {
    fn default() -> Self {
        Self {
            // Cold start is the honest Empty prompt (no run yet), NOT Loading.
            result: PanelState::Empty,
            running: false,
            progress: None,
            family: TuneFamily::default(),
            sma_grid: SmaGridForm::default(),
            macd_grid: MacdGridForm::default(),
            rsi_grid: RsiGridForm::default(),
            bollinger_grid: BollingerGridForm::default(),
        }
    }
}

impl TuneScreenState {
    /// The grid estimate for the CURRENTLY-SELECTED family's form. Drives the
    /// live readout + the Run-enabled gate. Pure; dispatches on `self.family`.
    #[must_use]
    pub fn grid_estimate(&self) -> GridEstimate {
        let cap = backtest::MAX_SWEEP_CONFIGS;
        match self.family {
            TuneFamily::Sma => estimate_sma_grid(&self.sma_grid, cap),
            TuneFamily::Macd => estimate_macd_grid(&self.macd_grid, cap),
            TuneFamily::Rsi => estimate_rsi_grid(&self.rsi_grid, cap),
            TuneFamily::Bollinger => estimate_bollinger_grid(&self.bollinger_grid, cap),
        }
    }

    /// `true` when Run should be enabled: the family is runnable, the grid has
    /// ≥ 1 runnable cell with no blank field, and no sweep is already running.
    #[must_use]
    pub fn can_run(&self) -> bool {
        self.family.is_runnable() && !self.running && self.grid_estimate().is_runnable()
    }

    /// advisor-param-promotion (ADR-0070 § D6) — the window label the current
    /// sweep result scored over, for the "robust on THIS window" promotion
    /// honesty copy. Reads the `Ready` mirror's `range_label` (the exact window
    /// the gate scored); falls back to a neutral phrase when no result is on
    /// screen (defensive — promotion is only reachable from a `Ready` grid row).
    #[must_use]
    pub fn range_label_or_default(&self) -> SmolStr {
        match &self.result {
            PanelState::Ready(mirror) => mirror.range_label.clone(),
            _ => SmolStr::new(crate::strings::TUNE_PROMOTE_WINDOW_FALLBACK),
        }
    }

    /// Select a family (the picker). Does NOT clear the existing result — the
    /// operator may inspect a prior result while eyeing another family.
    pub fn select_family(&mut self, family: TuneFamily) {
        self.family = family;
    }

    /// Edit one `{min, max, step}` field of one axis — ROUTED to the owning
    /// family's sub-form via [`TuneAxisKind::family`] (round-tripped verbatim).
    pub fn edit_axis(&mut self, axis: TuneAxisKind, field: AxisField, value: String) {
        match axis.family() {
            TuneFamily::Sma => self.sma_grid.edit(axis, field, value),
            TuneFamily::Macd => self.macd_grid.edit(axis, field, value),
            TuneFamily::Rsi => self.rsi_grid.edit(axis, field, value),
            TuneFamily::Bollinger => self.bollinger_grid.edit(axis, field, value),
        }
    }

    /// Apply a narrow/shipped/wide preset to one axis — ROUTED to the owning
    /// family's sub-form via [`TuneAxisKind::family`].
    pub fn apply_preset(&mut self, axis: TuneAxisKind, preset: AxisPreset) {
        match axis.family() {
            TuneFamily::Sma => self.sma_grid.apply_preset(axis, preset),
            TuneFamily::Macd => self.macd_grid.apply_preset(axis, preset),
            TuneFamily::Rsi => self.rsi_grid.apply_preset(axis, preset),
            TuneFamily::Bollinger => self.bollinger_grid.apply_preset(axis, preset),
        }
    }

    /// Toggle the i-th Bollinger `k`-preset (the multi-select). Pure.
    pub fn toggle_bollinger_k(&mut self, index: usize) {
        self.bollinger_grid.toggle_k(index);
    }

    /// Mark a sweep as started — `result` → `Loading`, `running` → `true`,
    /// `progress` cleared (a stale "{n} of {total}" must never linger). Called
    /// from the `Message::SweepRunRequested` update arm. The binary does the
    /// I/O half (`spawn_sweep`).
    pub fn begin_run(&mut self) {
        self.result = PanelState::Loading;
        self.running = true;
        self.progress = None;
    }

    /// Land a cell-level progress update — drives the determinate bar. Called
    /// from the `Message::SweepProgress` update arm.
    pub fn set_progress(&mut self, progress: backtest::progress::BakeoffProgress) {
        self.progress = Some(progress);
    }

    /// Land a completed sweep. `Ok(mirror)` → `Ready` (or `Empty` if the grid
    /// produced zero cells); `Err(msg)` → `Error`. Always clears `running` +
    /// `progress`. Called from the `Message::SweepRunCompleted` update arm.
    pub fn finish_run(&mut self, outcome: Result<SweepReportMirror, SmolStr>) {
        self.running = false;
        self.progress = None;
        self.result = match outcome {
            Ok(mirror) if mirror.cells.is_empty() => PanelState::Empty,
            Ok(mirror) => PanelState::Ready(mirror),
            Err(msg) => PanelState::Error(msg),
        };
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const CAP: usize = backtest::MAX_SWEEP_CONFIGS;

    #[test]
    fn default_family_is_sma_and_runnable() {
        let st = TuneScreenState::default();
        assert_eq!(st.family, TuneFamily::Sma);
        assert!(st.family.is_runnable(), "SMA must be runnable in v0.1");
    }

    #[test]
    fn all_families_are_runnable() {
        for f in TuneFamily::ALL {
            assert!(
                f.is_runnable(),
                "{f:?} must be runnable (T7b flipped them on)"
            );
        }
    }

    #[test]
    fn axis_kind_maps_to_owning_family() {
        assert_eq!(TuneAxisKind::SmaFast.family(), TuneFamily::Sma);
        assert_eq!(TuneAxisKind::SmaSlow.family(), TuneFamily::Sma);
        assert_eq!(TuneAxisKind::MacdFast.family(), TuneFamily::Macd);
        assert_eq!(TuneAxisKind::MacdSlow.family(), TuneFamily::Macd);
        assert_eq!(TuneAxisKind::MacdSignal.family(), TuneFamily::Macd);
        assert_eq!(TuneAxisKind::RsiPeriod.family(), TuneFamily::Rsi);
        assert_eq!(TuneAxisKind::RsiOversold.family(), TuneFamily::Rsi);
        assert_eq!(
            TuneAxisKind::BollingerPeriod.family(),
            TuneFamily::Bollinger
        );
    }

    #[test]
    fn default_grid_is_runnable_and_under_cap() {
        let st = TuneScreenState::default();
        let est = st.grid_estimate();
        // Default: fast {10,15,20,25,30} × slow {30,40,50,60,70} = 25 cartesian;
        // drop fast>=slow cells; the valid count is < cap, so not truncated.
        assert!(est.runnable > 0, "default grid must run something");
        assert!(!est.has_blank_field, "default fields are all filled");
        assert!(est.is_runnable());
        assert!(st.can_run(), "default SMA form must be runnable");
    }

    #[test]
    fn invalid_cells_dropped_when_fast_ge_slow() {
        // fast {40,50} × slow {30,45}: only (40,45) is valid (fast<slow<=400).
        let form = SmaGridForm {
            fast: AxisInput::from_values(40, 50, 10),
            slow: AxisInput::from_values(30, 45, 15),
        };
        let est = estimate_sma_grid(&form, CAP);
        // cartesian = 2*2 = 4; valid = (40,45) only = 1; invalid = 3.
        assert_eq!(est.valid_total, 1, "only (40,45) satisfies fast<slow");
        assert_eq!(est.invalid, 3);
        assert_eq!(est.runnable, 1);
        assert!(!est.truncated);
    }

    #[test]
    fn over_cap_grid_truncates() {
        // fast 1..30 step 1 (30 vals) × slow 31..60 step 1 (30 vals): all valid
        // (fast<slow always) = 900 cells >> cap → truncated, runnable == cap.
        let form = SmaGridForm {
            fast: AxisInput::from_values(1, 30, 1),
            slow: AxisInput::from_values(31, 60, 1),
        };
        let est = estimate_sma_grid(&form, CAP);
        assert!(est.valid_total > CAP, "the grid must exceed the cap");
        assert!(est.truncated, "over-cap grid must set truncated");
        assert_eq!(est.runnable, CAP, "runnable is capped");
        assert!(est.is_runnable());
    }

    #[test]
    fn blank_field_blocks_run() {
        let mut form = SmaGridForm::default();
        form.fast.min = String::new(); // operator cleared the field mid-edit
        let est = estimate_sma_grid(&form, CAP);
        assert!(est.has_blank_field, "a blank field must be flagged");
        assert!(!est.is_runnable(), "Run must disable on a blank field");
    }

    #[test]
    fn empty_grid_blocks_run() {
        // min > max on the fast axis → zero fast values → zero cells.
        let form = SmaGridForm {
            fast: AxisInput::from_values(50, 10, 5),
            slow: AxisInput::from_values(30, 70, 10),
        };
        let est = estimate_sma_grid(&form, CAP);
        assert_eq!(est.runnable, 0, "min>max yields no cells");
        assert!(!est.is_runnable(), "Run must disable on an empty grid");
    }

    #[test]
    fn preset_apply_seeds_axis() {
        let mut form = SmaGridForm::default();
        form.apply_preset(TuneAxisKind::SmaFast, AxisPreset::Wide);
        assert_eq!(form.fast.min, "5");
        assert_eq!(form.fast.max, "40");
        // Wide fast + shipped slow stays runnable.
        let est = estimate_sma_grid(&form, CAP);
        assert!(est.is_runnable());
    }

    #[test]
    fn edit_round_trips_verbatim() {
        let mut form = SmaGridForm::default();
        form.edit(TuneAxisKind::SmaSlow, AxisField::Max, "123".to_string());
        assert_eq!(form.slow.max, "123");
        let (_, smax, _) = form.slow.parsed();
        assert_eq!(smax, Some(123));
    }

    // ── Per-family form estimates (T7b) ─────────────────────────────────────

    #[test]
    fn macd_default_form_matches_engine_default_and_runs() {
        // The form default must enumerate the SAME valid-triple count as the
        // engine's `MacdGrid::default` (the form mirrors the engine grid).
        let est = estimate_macd_grid(&MacdGridForm::default(), CAP);
        let (_, engine_valid) = backtest::MacdGrid::default().enumerate_valid();
        assert_eq!(
            est.valid_total,
            engine_valid.len(),
            "MACD form default must match the engine default grid's valid count"
        );
        assert!(est.is_runnable(), "default MACD grid must run something");
        assert!(!est.has_blank_field);
    }

    #[test]
    fn macd_drops_fast_ge_slow_like_engine() {
        // fast [20,30] × slow [20] × signal [9]: both (20,20),(30,20) invalid.
        let form = MacdGridForm {
            fast: AxisInput::from_values(20, 30, 10),
            slow: AxisInput::from_values(20, 20, 1),
            signal: AxisInput::from_values(9, 9, 1),
        };
        let est = estimate_macd_grid(&form, CAP);
        assert_eq!(est.valid_total, 0, "no fast<slow triple is valid");
        assert!(!est.is_runnable());
    }

    #[test]
    fn macd_selecting_family_drives_estimate() {
        let mut st = TuneScreenState::default();
        st.select_family(TuneFamily::Macd);
        // The state estimate must now reflect the MACD form, not SMA.
        let est = st.grid_estimate();
        let direct = estimate_macd_grid(&MacdGridForm::default(), CAP);
        assert_eq!(est, direct, "state estimate must dispatch to the MACD form");
        assert!(st.can_run(), "default MACD form must be runnable");
    }

    #[test]
    fn macd_edit_routes_by_family_via_state() {
        let mut st = TuneScreenState::default();
        st.select_family(TuneFamily::Macd);
        st.edit_axis(TuneAxisKind::MacdSignal, AxisField::Max, "13".to_string());
        assert_eq!(st.macd_grid.signal.max, "13");
        // The SMA form is untouched (the router addressed MACD only).
        assert_eq!(st.sma_grid, SmaGridForm::default());
    }

    #[test]
    fn rsi_default_form_matches_engine_default() {
        let est = estimate_rsi_grid(&RsiGridForm::default(), CAP);
        let (_, engine_valid) = backtest::RsiGrid::default().enumerate_valid();
        assert_eq!(est.valid_total, engine_valid.len());
        assert!(est.is_runnable());
    }

    #[test]
    fn rsi_drops_oversold_ge_50() {
        // oversold 48..52 step 2 → {48,50,52}: only 48 is <= 49.
        let form = RsiGridForm {
            period: AxisInput::from_values(14, 14, 1),
            oversold: AxisInput::from_values(48, 52, 2),
        };
        let est = estimate_rsi_grid(&form, CAP);
        assert_eq!(est.valid_total, 1, "only oversold=48 is valid");
    }

    #[test]
    fn bollinger_default_form_matches_engine_default() {
        // Engine default uses ALL FOUR k presets; the form default selects only
        // the shipped k=2.0, so the form's valid count is the engine's / 4. Match
        // the form against a single-k engine grid to prove the period axis is right.
        let form = BollingerGridForm::default();
        let est = estimate_bollinger_grid(&form, CAP);
        // period {14,20,26} = 3 valid periods × 1 selected k = 3 cells.
        assert_eq!(
            est.valid_total, 3,
            "default form: 3 periods × 1 k = 3 cells"
        );
        assert!(est.is_runnable());
        // Ticking all four k presets quadruples to the engine default's 12 cells.
        let wide = BollingerGridForm {
            k_selected: [true; 4],
            ..BollingerGridForm::default()
        };
        let (_, engine_valid) = backtest::BollingerGrid::default().enumerate_valid();
        assert_eq!(
            estimate_bollinger_grid(&wide, CAP).valid_total,
            engine_valid.len(),
            "all-k form must match the engine default grid"
        );
    }

    #[test]
    fn bollinger_zero_k_blocks_run() {
        let form = BollingerGridForm {
            k_selected: [false; 4],
            ..BollingerGridForm::default()
        };
        let est = estimate_bollinger_grid(&form, CAP);
        assert!(est.has_blank_field, "zero k presets must block run");
        assert!(!est.is_runnable());
    }

    #[test]
    fn bollinger_selected_k_decimals_match_presets() {
        let form = BollingerGridForm {
            k_selected: [true, false, true, false], // 1.5 + 2.5
            ..BollingerGridForm::default()
        };
        let ks = form.selected_k_decimals();
        assert_eq!(ks.len(), 2);
        assert_eq!(ks[0], rust_decimal_macros::dec!(1.5));
        assert_eq!(ks[1], rust_decimal_macros::dec!(2.5));
    }

    #[test]
    fn bollinger_toggle_k_via_state() {
        let mut st = TuneScreenState::default();
        st.select_family(TuneFamily::Bollinger);
        assert_eq!(
            st.bollinger_grid.k_count(),
            1,
            "default: shipped k=2.0 only"
        );
        st.toggle_bollinger_k(3); // add 3.0
        assert_eq!(st.bollinger_grid.k_count(), 2);
        st.toggle_bollinger_k(1); // remove the shipped 2.0
        assert_eq!(st.bollinger_grid.k_count(), 1);
    }

    #[test]
    fn begin_run_sets_loading_and_running() {
        let mut st = TuneScreenState::default();
        st.begin_run();
        assert!(st.running);
        assert!(matches!(st.result, PanelState::Loading));
        assert!(st.progress.is_none());
        assert!(!st.can_run(), "can_run is false while a sweep runs");
    }

    #[test]
    fn finish_run_ok_lands_ready() {
        let mut st = TuneScreenState::default();
        st.begin_run();
        let mirror = crate::fixtures::fake_sweep_report_mirror();
        st.finish_run(Ok(mirror));
        assert!(!st.running);
        assert!(matches!(st.result, PanelState::Ready(_)));
        assert!(st.progress.is_none());
    }

    #[test]
    fn finish_run_err_lands_error() {
        let mut st = TuneScreenState::default();
        st.begin_run();
        st.finish_run(Err(SmolStr::new("boom")));
        assert!(!st.running);
        assert!(matches!(st.result, PanelState::Error(_)));
    }

    #[test]
    fn finish_run_empty_cells_lands_empty() {
        let mut st = TuneScreenState::default();
        st.begin_run();
        let mut mirror = crate::fixtures::fake_sweep_report_mirror();
        mirror.cells.clear();
        st.finish_run(Ok(mirror));
        assert!(matches!(st.result, PanelState::Empty));
    }

    #[test]
    fn select_family_keeps_result() {
        let mut st = TuneScreenState::default();
        let mirror = crate::fixtures::fake_sweep_report_mirror();
        st.finish_run(Ok(mirror));
        st.select_family(TuneFamily::Macd);
        assert_eq!(st.family, TuneFamily::Macd);
        // Switching family must NOT clear a prior result on screen.
        assert!(
            matches!(st.result, PanelState::Ready(_)),
            "switching family must NOT clear a prior result"
        );
        // MACD is now runnable (T7b) with its default form — Run is enabled.
        assert!(st.can_run(), "MACD default form is runnable");
    }
}
