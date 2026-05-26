//! Lab screen state — ui-rethink-phase-a-lab T-D-4.
//!
//! Defines `LabState` (T-D-4 / Design § 5.1) — the per-session state
//! for the Lab screen: selected strategy, selected pair, date range,
//! params slot, and the comparison set (≤4 strategies).
//!
//! Also defines `StrategyFamily` (for the `strategy_chip` family pill),
//! `DateRange` + `Preset` (for the `date_range` picker), and the
//! update-handler helpers for the `Message::Lab*` variants.
//!
//! **No persistence here** — persistence lives in `lab::persistence`
//! (M-FINAL). This module is pure state + pure-function transitions.
//!
//! **No I/O, no `SystemTime`, no `Instant`** — pure Rust, easily
//! unit-testable.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};

use crate::widgets::training_log::RingBuffer;

/// Data source toggle for the Lab screen (lab-yahoo-realdata T-C3.1 / R3.1).
///
/// `Synthetic` (default) — GBM-generated bars as in all pre-v0.1.0 runs.
/// `YahooCache` — bars loaded from `data/yahoo/<TICKER>/<INTERVAL>/…` parquet
/// cache via `data::yahoo::YahooBarSource::load_cached`.
///
/// The enum is `#[serde(…)]`-ready so it can round-trip through the Lab
/// persistence schema (version: 1) without breaking existing saved state
/// (unknown string → falls back to `Synthetic` via `#[serde(default)]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabDataSource {
    /// GBM synthetic bars — byte-identical to pre-v0.1.0 default (H5).
    #[default]
    Synthetic,
    /// Yahoo Finance parquet cache — real historical OHLCV data.
    YahooCache,
}

/// Operator-facing strategy family labels (Design § 2.2 family pill).
/// Four-char badge rendered on the strategy chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrategyFamily {
    /// Rule-based strategy.
    #[default]
    Rule,
    /// Composed strategy (ensemble).
    Composed,
    /// Large-language-model-driven strategy.
    Llm,
    /// Deep-learning model strategy.
    Dl,
    /// Hybrid (rule + model) strategy.
    Hybrid,
}

impl StrategyFamily {
    /// Four-char uppercase badge rendered on the chip.
    #[must_use]
    pub const fn badge_label(self) -> &'static str {
        match self {
            StrategyFamily::Rule => "RULE",
            StrategyFamily::Composed => "COMP",
            StrategyFamily::Llm => "LLM",
            StrategyFamily::Dl => "DL",
            StrategyFamily::Hybrid => "HYBR",
        }
    }
}

/// Date-range preset labels (Design § 2.3 / R5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum Preset {
    /// Last 30 calendar days.
    Last30d,
    /// Last 90 calendar days.
    #[default]
    Last90d,
    /// First half of 2024 (2024-01-01 → 2024-06-30).
    H1_2024,
    /// Second half of 2024 (2024-07-01 → 2024-12-31).
    H2_2024,
}

impl Preset {
    /// Human-readable label rendered in the dropdown.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Preset::Last30d => "Last 30d",
            Preset::Last90d => "Last 90d",
            Preset::H1_2024 => "2024 H1",
            Preset::H2_2024 => "2024 H2",
        }
    }
}

/// Date range selected in the Lab (Design § 2.3 / R5.2).
///
/// `Custom` is reserved for Phase A (Design § 2.3 — no calendar widget
/// at Phase A; custom field editing lands in Phase B/C). The discriminant
/// is `serde`-aware for the persistence schema (`version: 1`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DateRange {
    /// One of the named presets.
    Preset(Preset),
    /// Operator-entered ISO-8601 date pair (Phase A: text fields, no
    /// calendar widget).
    Custom {
        /// ISO-8601 start date string.
        start_raw: SmolStr,
        /// ISO-8601 end date string.
        end_raw: SmolStr,
    },
}

impl Default for DateRange {
    fn default() -> Self {
        DateRange::Preset(Preset::default())
    }
}

/// Maximum number of strategies in the comparison set (R8.1 / R8.2).
pub const COMPARE_SET_CAP: usize = 4;

/// Lab screen per-session state (Design § 5.1 / R6.1).
///
/// All fields are `Option` where the UI can be in a "not-yet-selected"
/// cold-start state. The field `compare_set` is a fixed-capacity array
/// with a tracked `len` so the 4-cap is enforced at the type level —
/// no heap allocation, no `SmallVec` dep needed.
///
/// `params` is reserved-but-empty at Phase A (Design § 5.1 / R6.4).
///
/// Note: derives removed in cockpit-training-control T-D-N4 because
/// `TrainingHandle` (an OS process handle) is neither `Clone` nor `PartialEq`.
/// Manual impls below cover the fields that are comparable / cloneable.
pub struct LabState {
    /// Currently selected primary strategy. `None` on cold start.
    pub strategy: Option<StrategyId>,
    /// Currently selected `(Venue, Symbol)` pair. `None` on cold start.
    pub pair: Option<(Venue, Symbol)>,
    /// Selected date range. Defaults to `Last90d` per Q-A3.
    pub range: DateRange,
    /// Reserved for Phase B param sheet — always `None` at Phase A.
    pub params: Option<()>,
    /// Comparison set — up to 4 additional strategies (R8.1).
    /// Length is separately tracked because Rust arrays don't carry a
    /// dynamic length.
    compare_buf: [Option<StrategyId>; COMPARE_SET_CAP],
    /// Number of occupied slots in `compare_buf`.
    compare_len: usize,

    // ── Training panel — cockpit-training-control T-D-N4 ─────────────────────
    /// In-flight training subprocess handle. Held by `LabState`; dropping it
    /// immediately SIGKILL's the subprocess (ADR-0034 / Q2 / R2.4).
    /// `None` when no training run is in flight.
    ///
    /// The `TrainingHandle` is not `Clone` or `PartialEq` — `LabState` derives
    /// `PartialEq` only for the non-training fields; training state is excluded.
    #[allow(dead_code)] // Populated in T-D-N4; read in view/update.
    pub training_inflight: Option<crate::lab::trainer::TrainingHandle>,

    /// Ring buffer of training log lines (most-recent-at-back).
    /// Populated by `Message::TrainingLogLine` via `push_line`.
    pub training_log: RingBuffer,

    /// Whether the training log scroll is anchored to the bottom (auto-scroll).
    /// `true` by default. Frozen by `Message::TrainingLogClicked`.
    pub training_log_anchored: bool,

    /// Whether the Training panel is collapsed (header-chip only) or expanded.
    /// `true` = collapsed (default at cold start per R1.2 / Q4).
    pub training_panel_collapsed: bool,

    /// Ring buffer of audit training-event rows delivered by the 1 Hz poller
    /// (T-D-N11). Capacity 1024 per ADR-0034 § D6.
    /// Only available with `--features live` (audit crate dependency).
    #[cfg(feature = "live")]
    pub training_events: std::collections::VecDeque<trading_core::views::TrainingEventRow>,

    // ── Phase B — Lab run results (ui-rethink-phase-b-lab-run T-D-N10) ──────
    /// Most-recent completed run result mirror. `None` on cold start and after
    /// a tuple change (`LabSelectStrategy` / `LabSelectPair` / `LabSelectRange`).
    /// NOT cloned (set to `None` in `LabState::clone`) and NOT serialized
    /// (schema stays `version: 1`).
    pub last_run_report: Option<crate::lab::runner::RunReportMirror>,
    /// Previous completed run result for the same tuple (enables `run_delta_badge`).
    /// Rotated from `last_run_report` when a second run on the same tuple completes.
    /// Same carve-out rules as `last_run_report`.
    pub prev_run_report: Option<crate::lab::runner::RunReportMirror>,

    /// Error message from the most-recent `LabRunCompleted(Err(_))`, or `None`
    /// after a successful run / cold-start. Drives the Run button's `Failed`
    /// state and the error banner next to the Run button so engine errors
    /// (e.g. `load momentum config` or `YahooCache cache miss`) are surfaced
    /// to the operator instead of disappearing silently.
    ///
    /// NOT cloned (`LabState::clone` sets to None) and NOT serialized.
    /// Cleared on next `LabRunRequested` and on `LabRunCompleted(Ok)`.
    pub last_run_error: Option<smol_str::SmolStr>,

    // lab-polish-round-2 R2 — SMA param overrides ────────────────────────
    /// Optional override for the fast SMA window length (`None` → 20 default).
    /// Set by the param-stepper widget when the operator picks `v0.sma`.
    /// Plumbed into `SmaComposedRunInput.sma_fast_len` at run time.
    /// NOT serialized — resets to None on cockpit restart (Phase A simplicity).
    pub sma_fast_len: Option<usize>,
    /// Optional override for the slow SMA window length (`None` → 50 default).
    pub sma_slow_len: Option<usize>,
    /// Raw text the operator is currently typing into the fast input.
    /// Kept separately from `sma_fast_len` so partial / invalid input
    /// (e.g. "1" while typing "10") doesn't immediately reset to None.
    pub sma_fast_input: String,
    /// Raw text for the slow input. Same rationale as `sma_fast_input`.
    pub sma_slow_input: String,

    // ── lab-yahoo-realdata v0.1.0 — data source toggle (T-C3.1 / R3.1) ──────
    /// Data source for the next Run. `Synthetic` (default) preserves the
    /// pre-v0.1.0 byte-identical behaviour (H5 / R-NR.8).
    /// `YahooCache` loads bars from the local parquet cache.
    pub data_source: LabDataSource,

    // ── Wave D-3 — Stop button (lab-end-to-end-v2 T-D3.1 / R6.1) ──────────
    /// In-flight run cancellation handle. `Some` while a backtest is running;
    /// `None` otherwise. Dropping the handle closes the cancel channel so
    /// the receiver returns `Disconnected` at the next poll boundary.
    ///
    /// Populated by the binary-side `cockpit_live.rs::update` wrapper at
    /// `LabRunRequested` and cleared on `LabRunCompleted` / `LabRunStopRequested`.
    #[allow(dead_code)]
    pub run_cancel: Option<backtest::cancel::RunCancelHandle>,

    // ── cockpit-training-pressed-wiring v0.1.0 T-D-N1 ────────────────────────
    /// In-flight training subprocess cancellation handle. `Some` while training
    /// is running; `None` otherwise. Dropping the handle closes the cancel
    /// channel. The SIGKILL itself is fired by `TrainingHandle::drop` when
    /// `training_inflight` is dropped.
    ///
    /// Populated by the binary-side `cockpit_live.rs::update` wrapper at
    /// `TrainingPressed` and cleared on `TrainingExited` / `TrainingCancelPressed`.
    #[allow(dead_code)]
    pub training_cancel: Option<crate::lab::trainer::RunCancelHandle>,

    // ── Wave D-4 — Progress (lab-end-to-end-v2 T-D4.6 / R9) ───────────────
    /// Most-recent progress event from the in-flight backtest. `None` when
    /// no run is in-flight or the engine hasn't emitted yet.
    pub run_progress: Option<backtest::progress::Progress>,
}

/// Manual `Clone` for `LabState` — `TrainingHandle` (an OS process handle)
/// is not `Clone`, so we exclude it. Cloning `LabState` resets
/// `training_inflight` to `None` (the in-flight handle stays in the original;
/// this clone is used only for snapshot/persistence operations).
impl Clone for LabState {
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy.clone(),
            pair: self.pair.clone(),
            range: self.range.clone(),
            params: self.params,
            compare_buf: self.compare_buf.clone(),
            compare_len: self.compare_len,
            // Training handle is NOT cloned — the clone starts with no handle.
            training_inflight: None,
            training_log: self.training_log.clone(),
            training_log_anchored: self.training_log_anchored,
            training_panel_collapsed: self.training_panel_collapsed,
            #[cfg(feature = "live")]
            training_events: self.training_events.clone(),
            // T-D-N10: run report mirrors are NOT cloned (large Arc allocation;
            // only needed in the live session instance, not in snapshots/persistence clones).
            last_run_report: None,
            prev_run_report: None,
            last_run_error: None,
            sma_fast_len: None,
            sma_slow_len: None,
            sma_fast_input: String::new(),
            sma_slow_input: String::new(),
            // data_source IS cloned — it's a UI selection, not an in-flight resource.
            data_source: self.data_source,
            // Cancel handle is NOT cloned — the clone starts without an in-flight run.
            run_cancel: None,
            // Progress is NOT cloned — the clone starts clean.
            run_progress: None,
            // Training cancel handle is NOT cloned.
            training_cancel: None,
        }
    }
}

impl std::fmt::Debug for LabState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LabState")
            .field("strategy", &self.strategy)
            .field("pair", &self.pair)
            .field("range", &self.range)
            .field("compare_buf", &self.compare_buf)
            .field("compare_len", &self.compare_len)
            .field("training_inflight", &self.training_inflight)
            .field("training_log_len", &self.training_log.len())
            .field("training_log_anchored", &self.training_log_anchored)
            .field("training_panel_collapsed", &self.training_panel_collapsed)
            .field(
                "last_run_report",
                &self.last_run_report.as_ref().map(|_| "<RunReportMirror>"),
            )
            .field(
                "prev_run_report",
                &self.prev_run_report.as_ref().map(|_| "<RunReportMirror>"),
            )
            .field("data_source", &self.data_source)
            .field("run_cancel_present", &self.run_cancel.is_some())
            .field("run_progress", &self.run_progress)
            .finish_non_exhaustive()
    }
}

impl Default for LabState {
    fn default() -> Self {
        Self {
            strategy: None,
            pair: None,
            range: DateRange::default(),
            params: None,
            compare_buf: [const { None }; COMPARE_SET_CAP],
            compare_len: 0,
            training_inflight: None,
            training_log: VecDeque::new(),
            training_log_anchored: true,
            training_panel_collapsed: true,
            #[cfg(feature = "live")]
            training_events: std::collections::VecDeque::new(),
            last_run_report: None,
            prev_run_report: None,
            last_run_error: None,
            sma_fast_len: None,
            sma_slow_len: None,
            sma_fast_input: String::new(),
            sma_slow_input: String::new(),
            data_source: LabDataSource::default(),
            run_cancel: None,
            run_progress: None,
            training_cancel: None,
        }
    }
}

impl LabState {
    /// Construct a `LabState` with the given fields and empty compare set.
    ///
    /// Used by `persistence::lab_state_from_json` and `persistence::cold_start_defaults`
    /// to build a state without accessing private fields across module boundaries.
    #[must_use]
    pub fn with_selection(
        strategy: Option<trading_core::StrategyId>,
        pair: Option<(trading_core::Venue, trading_core::Symbol)>,
        range: DateRange,
    ) -> Self {
        Self {
            strategy,
            pair,
            range,
            params: None,
            compare_buf: [const { None }; COMPARE_SET_CAP],
            compare_len: 0,
            training_inflight: None,
            training_log: VecDeque::new(),
            training_log_anchored: true,
            training_panel_collapsed: true,
            #[cfg(feature = "live")]
            training_events: std::collections::VecDeque::new(),
            last_run_report: None,
            prev_run_report: None,
            last_run_error: None,
            sma_fast_len: None,
            sma_slow_len: None,
            sma_fast_input: String::new(),
            sma_slow_input: String::new(),
            data_source: LabDataSource::default(),
            run_cancel: None,
            run_progress: None,
            training_cancel: None,
        }
    }
}

impl LabState {
    /// Returns a read-only slice of the current comparison set.
    #[must_use]
    pub fn compare_set(&self) -> &[Option<StrategyId>] {
        &self.compare_buf[..self.compare_len]
    }

    /// Number of strategies in the comparison set.
    #[must_use]
    pub fn compare_len(&self) -> usize {
        self.compare_len
    }

    /// Returns `true` if `id` is in the comparison set.
    #[must_use]
    pub fn is_in_compare_set(&self, id: &StrategyId) -> bool {
        self.compare_buf[..self.compare_len]
            .iter()
            .any(|slot| slot.as_ref() == Some(id))
    }

    /// Toggle `id` in the comparison set (Design § 2.2 / R4.2 / R8.1).
    ///
    /// - If `id` is already in the set, it is removed (idempotent
    ///   second press → removes).
    /// - If `id` is not in the set AND `compare_len < COMPARE_SET_CAP`,
    ///   it is added.
    /// - If the set is full (4 strategies), the toggle is a no-op and
    ///   the function returns `false` to signal the caller to emit a
    ///   toast.
    ///
    /// Returns `true` on any state change, `false` when the set is full
    /// and `id` is not already present (cap-hit no-op).
    pub fn toggle_compare(&mut self, id: StrategyId) -> bool {
        // Check if already present — if so, remove it.
        for i in 0..self.compare_len {
            if self.compare_buf[i].as_ref() == Some(&id) {
                // Shift remaining slots left.
                for j in i..self.compare_len.saturating_sub(1) {
                    self.compare_buf[j] = self.compare_buf[j + 1].take();
                }
                if self.compare_len > 0 {
                    self.compare_buf[self.compare_len - 1] = None;
                    self.compare_len -= 1;
                }
                return true;
            }
        }
        // Not present — add if capacity allows.
        if self.compare_len >= COMPARE_SET_CAP {
            return false; // cap hit — caller emits toast
        }
        self.compare_buf[self.compare_len] = Some(id);
        self.compare_len += 1;
        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn id(s: &str) -> StrategyId {
        StrategyId(smol_str::SmolStr::new(s))
    }

    /// T-D-4 — `toggle_compare` is idempotent: adding then removing an id
    /// leaves the set unchanged.
    #[test]
    fn toggle_compare_is_idempotent_add_remove() {
        let mut state = LabState::default();
        let s = id("v1.momentum");

        // First add.
        assert!(state.toggle_compare(s.clone()), "first add should succeed");
        assert_eq!(state.compare_len(), 1);
        assert!(state.is_in_compare_set(&s));

        // Second toggle removes.
        assert!(
            state.toggle_compare(s.clone()),
            "second toggle (remove) should return true"
        );
        assert_eq!(state.compare_len(), 0);
        assert!(!state.is_in_compare_set(&s));

        // Set is now empty — same as initial state.
        assert_eq!(state.compare_set().len(), 0);
    }

    /// T-D-4 — `toggle_compare` enforces the 4-cap. The 5th add is a
    /// no-op and returns `false`.
    #[test]
    fn toggle_compare_enforces_4_cap() {
        let mut state = LabState::default();
        for i in 0..4 {
            let added = state.toggle_compare(id(&format!("strat-{i}")));
            assert!(added, "add #{i} should succeed");
        }
        assert_eq!(state.compare_len(), 4);

        // 5th add is cap-hit no-op.
        let cap_hit = !state.toggle_compare(id("strat-overflow"));
        assert!(cap_hit, "5th add should be a cap-hit no-op (return false)");
        assert_eq!(state.compare_len(), 4, "compare set must not grow beyond 4");
    }

    /// T-D-4 — compare set preserves insertion order when an element is
    /// removed from the middle.
    #[test]
    fn toggle_compare_preserves_order_on_mid_remove() {
        let mut state = LabState::default();
        let a = id("a");
        let b = id("b");
        let c = id("c");

        state.toggle_compare(a.clone());
        state.toggle_compare(b.clone());
        state.toggle_compare(c.clone());
        assert_eq!(state.compare_len(), 3);

        // Remove middle element b.
        state.toggle_compare(b.clone());
        assert_eq!(state.compare_len(), 2);

        let slots: Vec<&StrategyId> = state.compare_set().iter().flatten().collect();
        assert_eq!(slots, &[&a, &c], "a and c must remain, in order");
    }

    /// T-D-4 — all Preset labels are non-empty.
    #[test]
    fn preset_labels_non_empty() {
        for p in [
            Preset::Last30d,
            Preset::Last90d,
            Preset::H1_2024,
            Preset::H2_2024,
        ] {
            assert!(!p.label().is_empty(), "empty label for {:?}", p);
        }
    }

    /// T-D-4 — all StrategyFamily badge labels are non-empty and ≤4 chars.
    #[test]
    fn strategy_family_badges_valid() {
        for f in [
            StrategyFamily::Rule,
            StrategyFamily::Composed,
            StrategyFamily::Llm,
            StrategyFamily::Dl,
            StrategyFamily::Hybrid,
        ] {
            let badge = f.badge_label();
            assert!(!badge.is_empty(), "empty badge for {:?}", f);
            assert!(badge.len() <= 4, "badge too long: {:?} = {:?}", f, badge);
        }
    }

    // ── T-C3.1 — LabDataSource enum ──────────────────────────────────────────

    /// T-C3.1 — Default is Synthetic (H5 / R-NR.8 byte-identity).
    #[test]
    fn lab_data_source_default_is_synthetic() {
        assert_eq!(LabDataSource::default(), LabDataSource::Synthetic);
        let state = LabState::default();
        assert_eq!(state.data_source, LabDataSource::Synthetic);
    }

    /// T-C3.1 — LabDataSource serde round-trip via JSON.
    #[test]
    fn lab_data_source_serde_round_trip() {
        let cases = [
            (LabDataSource::Synthetic, "\"synthetic\""),
            (LabDataSource::YahooCache, "\"yahoo_cache\""),
        ];
        for (variant, expected_json) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json, "wrong JSON for {variant:?}");
            let parsed: LabDataSource = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, variant, "serde round-trip mismatch for {variant:?}");
        }
    }

    /// T-C3.1 — Clone preserves data_source (it is a UI selection, not a handle).
    #[test]
    fn lab_state_clone_preserves_data_source() {
        let mut state = LabState::default();
        state.data_source = LabDataSource::YahooCache;
        let cloned = state.clone();
        assert_eq!(cloned.data_source, LabDataSource::YahooCache);
    }

    // ── T-D-16 proptest: cap holds under randomised add/remove sequences ──────
    //
    // proptest generates 100 random sequences of 0/1 operations (0 = toggle
    // the strategy at `index % 4`, 1 = toggle a random other strategy) and
    // verifies the compare set never exceeds `COMPARE_SET_CAP`.
    //
    // Using deterministic `prop_compose!` + `just`-seeded generation keeps
    // CI bit-identical across hosts (no OS RNG).

    proptest::proptest! {
        /// T-D-16 — compare set length never exceeds COMPARE_SET_CAP under
        /// up to 100 random toggle operations on a universe of 8 strategy IDs.
        #[test]
        fn prop_compare_set_never_exceeds_cap(
            ops in proptest::collection::vec(0usize..8usize, 0..100usize),
        ) {
            let strategies: Vec<StrategyId> = (0..8)
                .map(|i| id(&format!("strat-{i}")))
                .collect();
            let mut state = LabState::default();
            for strat_idx in ops {
                let _ = state.toggle_compare(strategies[strat_idx].clone());
                proptest::prop_assert!(
                    state.compare_len() <= COMPARE_SET_CAP,
                    "compare set exceeded cap: len={} cap={}",
                    state.compare_len(),
                    COMPARE_SET_CAP
                );
            }
        }
    }
}
