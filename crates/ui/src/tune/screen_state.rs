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
//! ## The grid is SMA-first
//!
//! The family picker presents all four families (SMA / MACD / RSI / Bollinger)
//! because the picker IS the affordance; but only SMA is *runnable* until the
//! engine's T7 string-generation builder lands. The non-SMA families render as
//! present-but-pending chips (the picker shows them, Run is disabled with an
//! honest "coming soon" note) so the UI is honest about what works today.

use smol_str::SmolStr;

use crate::state::PanelState;
use crate::tune::state::SweepReportMirror;

/// The strategy family the operator is tuning. UI-side closed enum mirroring
/// `backtest::SweepFamily` — the picker never matches on the engine type.
///
/// All four variants are present so the picker shows the full menu; only
/// [`TuneFamily::Sma`] is *runnable* in v0.1 (the others are the engine's T7
/// string-generation gap). [`TuneFamily::is_runnable`] is the honest gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TuneFamily {
    /// SMA crossover (`fast_len` / `slow_len`). Runnable now — the existing
    /// `ScenarioConfig` override seam.
    #[default]
    Sma,
    /// MACD (fast / slow / signal). Pending the engine's T7 builder.
    Macd,
    /// RSI (period / oversold). Pending the engine's T7 builder.
    Rsi,
    /// Bollinger bands (period / k). Pending the engine's T7 builder.
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

    /// `true` when this family can actually be swept today. Only SMA is runnable
    /// in v0.1 — the composed families need the engine's T7 string-generation
    /// builder (`build_swept_strategy`), which is not yet wired. The Run button
    /// reads this; a non-runnable family disables Run with an honest note.
    #[must_use]
    pub fn is_runnable(self) -> bool {
        matches!(self, TuneFamily::Sma)
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

/// Which axis of an SMA grid an edit targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmaAxisKind {
    /// The fast-window axis (shipped default 20).
    Fast,
    /// The slow-window axis (shipped default 50).
    Slow,
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
    /// keeps the cell count near the cap.
    pub fn apply_preset(&mut self, axis: SmaAxisKind, preset: AxisPreset) {
        let input = match (axis, preset) {
            // Fast axis presets — centred on the shipped 20.
            (SmaAxisKind::Fast, AxisPreset::Narrow) => AxisInput::from_values(15, 25, 5),
            (SmaAxisKind::Fast, AxisPreset::Shipped) => AxisInput::from_values(10, 30, 5),
            (SmaAxisKind::Fast, AxisPreset::Wide) => AxisInput::from_values(5, 40, 5),
            // Slow axis presets — centred on the shipped 50.
            (SmaAxisKind::Slow, AxisPreset::Narrow) => AxisInput::from_values(40, 60, 10),
            (SmaAxisKind::Slow, AxisPreset::Shipped) => AxisInput::from_values(30, 70, 10),
            (SmaAxisKind::Slow, AxisPreset::Wide) => AxisInput::from_values(30, 100, 10),
        };
        match axis {
            SmaAxisKind::Fast => self.fast = input,
            SmaAxisKind::Slow => self.slow = input,
        }
    }

    /// Edit one field of one axis (round-tripped verbatim).
    pub fn edit(&mut self, axis: SmaAxisKind, field: AxisField, value: String) {
        match axis {
            SmaAxisKind::Fast => self.fast.set(field, value),
            SmaAxisKind::Slow => self.slow.set(field, value),
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
    /// The family being tuned (default SMA — the runnable one).
    pub family: TuneFamily,
    /// The SMA `{min, max, step}` axis form (the only runnable family in v0.1).
    pub sma_grid: SmaGridForm,
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
        }
    }
}

impl TuneScreenState {
    /// The grid estimate for the current form (SMA only in v0.1). Drives the
    /// live readout + the Run-enabled gate. Pure.
    #[must_use]
    pub fn grid_estimate(&self) -> GridEstimate {
        estimate_sma_grid(&self.sma_grid, backtest::MAX_SWEEP_CONFIGS)
    }

    /// `true` when Run should be enabled: the family is runnable, the grid has
    /// ≥ 1 runnable cell with no blank field, and no sweep is already running.
    #[must_use]
    pub fn can_run(&self) -> bool {
        self.family.is_runnable() && !self.running && self.grid_estimate().is_runnable()
    }

    /// Select a family (the picker). Does NOT clear the existing result — the
    /// operator may inspect a prior SMA result while eyeing another family.
    pub fn select_family(&mut self, family: TuneFamily) {
        self.family = family;
    }

    /// Edit one `{min, max, step}` field of one SMA axis (round-tripped).
    pub fn edit_sma_axis(&mut self, axis: SmaAxisKind, field: AxisField, value: String) {
        self.sma_grid.edit(axis, field, value);
    }

    /// Apply a narrow/shipped/wide preset to one SMA axis.
    pub fn apply_sma_preset(&mut self, axis: SmaAxisKind, preset: AxisPreset) {
        self.sma_grid.apply_preset(axis, preset);
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
    fn non_sma_families_are_not_runnable_yet() {
        for f in [TuneFamily::Macd, TuneFamily::Rsi, TuneFamily::Bollinger] {
            assert!(!f.is_runnable(), "{f:?} is the T7 gap — not runnable yet");
        }
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
        form.apply_preset(SmaAxisKind::Fast, AxisPreset::Wide);
        assert_eq!(form.fast.min, "5");
        assert_eq!(form.fast.max, "40");
        // Wide fast + shipped slow stays runnable.
        let est = estimate_sma_grid(&form, CAP);
        assert!(est.is_runnable());
    }

    #[test]
    fn edit_round_trips_verbatim() {
        let mut form = SmaGridForm::default();
        form.edit(SmaAxisKind::Slow, AxisField::Max, "123".to_string());
        assert_eq!(form.slow.max, "123");
        let (_, smax, _) = form.slow.parsed();
        assert_eq!(smax, Some(123));
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
        // A non-runnable family must disable Run even with a prior result on screen.
        assert!(!st.can_run());
        assert!(
            matches!(st.result, PanelState::Ready(_)),
            "switching family must NOT clear a prior result"
        );
    }
}
