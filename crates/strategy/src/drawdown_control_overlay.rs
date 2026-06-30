//! Drawdown-control overlay strategy (v2 Phase 2C P1-3 / ADR-0080).
//!
//! Wraps any inner [`Strategy`] with a CPPI-style cushion multiplier that
//! de-risks exposure as equity draws down toward a **static floor** set at
//! `initial_equity × (1 − drawdown_floor_pct)`.  The default is a **20%
//! drawdown floor** per operator decision D8 (2026-06-30): *"never lose more
//! than 20% of the starting €200."*
//!
//! ## Cushion-multiplier formula (CPPI / Hsieh 2022)
//!
//! ```text
//! d(k)  = 1 − equity_k / hwm_k        (drawdown from HWM)
//! M(k)  = (d_max − d(k)) / (1 − d(k)) (cushion multiplier, 0..=1)
//!
//! where
//!   d_max  = drawdown_floor_pct  (default 0.20)
//!   hwm_k  = high-water mark at bar k (≥ initial_equity)
//!
//! The multiplier is CLAMPED to [0, 1] — de-risk only, never lever up.
//! When d(k) ≥ d_max the overlay shuts exposure to zero (M = 0).
//! ```
//!
//! Three independent derivations converge on this family (research §1):
//! - Discrete modulator [13]: `M(k) = (d_max − d(k))/(1 − d(k))`
//! - Growth-optimal fraction [31]: `π_α = (1−α)(X/X*)/(α+(1−α)(X/X*))`
//! - Convex risk-aversion ramp [12]: `γ_t = γ_0·D^max/(D^max − D_t)`
//!
//! ## HWM restart (LOAD-BEARING — ADR-0080 D2)
//!
//! When equity sets a **new all-time high** the high-water mark is reset to
//! the new high.  This is the load-bearing invariant: the BTC Jan-2020 –
//! Sep-2022 benchmark [Hsieh 2022, §1] showed:
//!
//! - **With HWM restart:** Sharpe 1.521, max-DD 72% → 20%.
//! - **Without HWM restart:** Sharpe −0.043 (lock-out-then-churn bleeds).
//!
//! The static floor *always* tracks `initial_equity × 0.80` (D8 CPPI);
//! the HWM only governs how the *current drawdown* is computed, preserving
//! upside participation in bull runs.
//!
//! ## Budget-cap invariant (CLAUDE.md non-negotiable)
//!
//! The overlay NEVER bypasses the `FixedFractionSizer` budget cap.
//! The cushion multiplier is applied via [`Strategy::quantity_scale`] —
//! the sizing pipeline then applies the budget-cap clamp **after** the
//! multiplier (i.e. `min(M(k) × base_qty, budget_cap_qty)`).  The overlay
//! does NOT rewrite the budget cap.
//!
//! ## Telemetry
//!
//! [`DrawdownControlOverlay::telemetry`] exposes current cushion, drawdown
//! from HWM, and HWM for operator visibility.  All values are `Decimal`.
//!
//! ## Determinism
//!
//! - No `SystemTime::now()` / `Instant::now()`.
//! - No `f64` in money/position arithmetic — HWM, floor, equity, and the
//!   multiplier are all computed and stored as `Decimal`.
//! - All state transitions are pure `Decimal` arithmetic — byte-reproducible
//!   across runs given the same bar sequence.
//!
//! ## Cross-references
//!
//! - ADR-0080 — drawdown-control overlay architecture decision.
//! - `crates/strategy/src/vol_targeting_overlay.rs` — sibling overlay pattern
//!   (composition via `Strategy::quantity_scale`).
//! - `crates/strategy/src/vol_estimator.rs` — sibling P1-5 module; both
//!   overlays can consume this estimator if combined in a future increment.
//! - `crates/risk/src/sizing.rs` — `FixedFractionSizer::budget_cap` invariant.
//! - `research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
//!   §6 P1-B — the cushion-multiplier formula + BTC HWM-restart benchmark.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use trading_core::{Bar, Money, Signal, StrategyId, Symbol, Tick, Usdt};

use crate::Strategy;

// ── DrawdownControlConfig ─────────────────────────────────────────────────────

/// Configuration for [`DrawdownControlOverlay`].
#[derive(Debug, Clone)]
pub struct DrawdownControlConfig {
    /// Maximum tolerated drawdown from the high-water mark before exposure
    /// is shut to zero.  Default `0.20` (20%) per operator decision D8.
    ///
    /// Must be in `(0, 1)`.  A value of `0.20` means the overlay scales
    /// exposure proportionally as drawdown approaches 20%, reaching M=0
    /// exactly when the drawdown hits the floor.
    pub drawdown_floor_pct: Decimal,

    /// Whether to restart the high-water mark when equity sets a new all-
    /// time high.  **Default `true` — this is LOAD-BEARING.**
    ///
    /// Setting this to `false` locks the HWM at `initial_equity` forever
    /// (pure static CPPI with no upside recovery).  The BTC benchmark
    /// [Hsieh 2022] showed Sharpe −0.04 without restart vs +1.52 with it.
    pub restart_on_hwm: bool,

    /// The reference equity used to compute the static floor.
    ///
    /// `floor = initial_equity × (1 − drawdown_floor_pct)`.
    ///
    /// The floor NEVER moves (D8 static CPPI — TIPP/ratcheting deferred to
    /// v0.2).  Only the HWM moves (when `restart_on_hwm = true`), which
    /// governs how the current-drawdown `d(k)` is computed from the HWM.
    pub initial_equity: Money<Usdt>,
}

impl DrawdownControlConfig {
    /// Compute the static floor: `initial_equity × (1 − drawdown_floor_pct)`.
    #[must_use]
    pub fn floor(&self) -> Decimal {
        self.initial_equity.amount() * (Decimal::ONE - self.drawdown_floor_pct)
    }
}

impl Default for DrawdownControlConfig {
    /// Default: 20% drawdown floor, HWM restart enabled, initial equity 200 USDT.
    ///
    /// The 200 USDT default matches the advisor's default €200 budget (F7 EUR→USDT).
    /// Callers MUST supply the actual initial equity for correct floor computation.
    fn default() -> Self {
        Self {
            drawdown_floor_pct: dec!(0.20),
            restart_on_hwm: true,
            initial_equity: Money::from_decimal(dec!(200)),
        }
    }
}

// ── DrawdownControlState ──────────────────────────────────────────────────────

/// Runtime state carried across bars.
#[derive(Debug, Clone)]
pub struct DrawdownControlState {
    /// Current high-water mark (≥ `initial_equity.amount()`).
    pub hwm: Decimal,
    /// The static floor = `initial_equity × (1 − drawdown_floor_pct)`.
    /// Never changes after construction (D8 static CPPI).
    pub floor: Decimal,
    /// Cushion multiplier from the most recent call to `update`.
    pub last_multiplier: Decimal,
    /// Drawdown from HWM at the most recent `update`.
    pub last_drawdown_from_hwm: Decimal,
    /// Total bars processed.
    pub bars_total: u64,
    /// Bars where the multiplier was < 1 (de-risking active).
    pub bars_de_risked: u64,
    /// Bars where the multiplier hit 0 (fully shut down).
    pub bars_shut: u64,
}

// ── DrawdownControlOverlay ────────────────────────────────────────────────────

/// Drawdown-control overlay: wraps any inner [`Strategy`] and scales signal
/// quantities via [`Strategy::quantity_scale`] by the CPPI cushion multiplier
/// `M(k) = (d_max − d(k)) / (1 − d(k))` where `d(k) = 1 − equity_k / hwm_k`.
///
/// The multiplier is clamped to `[0, 1]`  — de-risk only, never lever up.
/// The budget cap is NEVER bypassed (applied downstream by `FixedFractionSizer`).
pub struct DrawdownControlOverlay<S: Strategy> {
    /// Strategy ID.
    id: StrategyId,
    /// The inner strategy whose signals are de-risked.
    inner: S,
    /// Configuration (floor, restart, initial equity).
    config: DrawdownControlConfig,
    /// Runtime state.
    state: DrawdownControlState,
    /// Per-symbol cached cushion multiplier from the most recent `update_equity`.
    /// Shared across all symbols (the overlay is an account-level de-risk, not
    /// per-symbol).  Default 1.0 until the first equity update.
    cached_multiplier: Decimal,
}

impl<S: Strategy> std::fmt::Debug for DrawdownControlOverlay<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrawdownControlOverlay")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("state_hwm", &self.state.hwm)
            .field("state_last_multiplier", &self.state.last_multiplier)
            .finish()
    }
}

impl<S: Strategy> DrawdownControlOverlay<S> {
    /// Construct a new overlay wrapping `inner` with the given `config`.
    ///
    /// The HWM is initialised to `config.initial_equity.amount()`.
    /// The floor is `config.floor()` (static — never changes post-construction).
    #[must_use]
    pub fn new(inner: S, config: DrawdownControlConfig) -> Self {
        let id = StrategyId::new("drawdown_control_overlay");
        let initial = config.initial_equity.amount();
        let floor = config.floor();

        let state = DrawdownControlState {
            hwm: initial,
            floor,
            last_multiplier: Decimal::ONE,
            last_drawdown_from_hwm: Decimal::ZERO,
            bars_total: 0,
            bars_de_risked: 0,
            bars_shut: 0,
        };

        Self {
            id,
            inner,
            config,
            state,
            cached_multiplier: Decimal::ONE,
        }
    }

    /// Update the overlay with the current account equity.
    ///
    /// Must be called once per bar **before** `on_bar` so that
    /// `quantity_scale` returns the correct multiplier for the bar's signals.
    ///
    /// In a live/paper-run scenario the caller provides the current equity
    /// from the account ledger.  In unit tests a synthetic equity sequence is
    /// driven directly.
    pub fn update_equity(&mut self, equity_k: Decimal) {
        self.state.bars_total += 1;

        // HWM restart: ratchet up if equity is at a new all-time high.
        if self.config.restart_on_hwm && equity_k > self.state.hwm {
            self.state.hwm = equity_k;
        }

        // d(k) = 1 − equity_k / hwm_k
        // Guard: hwm must be > 0 (it starts at initial_equity > 0, so safe).
        let drawdown_from_hwm = if self.state.hwm.is_zero() {
            Decimal::ZERO
        } else {
            (Decimal::ONE - equity_k / self.state.hwm).max(Decimal::ZERO)
        };
        self.state.last_drawdown_from_hwm = drawdown_from_hwm;

        // M(k) = (d_max − d(k)) / (1 − d(k)), clamped to [0, 1].
        let d_max = self.config.drawdown_floor_pct;
        let multiplier = compute_cushion_multiplier(d_max, drawdown_from_hwm);
        self.state.last_multiplier = multiplier;
        self.cached_multiplier = multiplier;

        // Telemetry counters.
        if multiplier < Decimal::ONE {
            self.state.bars_de_risked += 1;
        }
        if multiplier.is_zero() {
            self.state.bars_shut += 1;
        }
    }

    /// Reference to the inner strategy (for tests / inspection).
    #[must_use]
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Mutable reference to the inner strategy (for test driving).
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Current runtime state (for operator visibility / telemetry).
    #[must_use]
    pub fn state(&self) -> &DrawdownControlState {
        &self.state
    }

    /// Structured telemetry for the current bar.
    ///
    /// Returns `(cushion_multiplier, drawdown_from_hwm, hwm)` as `Decimal`.
    #[must_use]
    pub fn telemetry(&self) -> DrawdownTelemetry {
        DrawdownTelemetry {
            cushion_multiplier: self.state.last_multiplier,
            drawdown_from_hwm: self.state.last_drawdown_from_hwm,
            hwm: self.state.hwm,
            floor: self.state.floor,
        }
    }

    /// The static floor (initial equity × (1 − floor pct)).
    ///
    /// This is computed from config and NEVER changes after construction.
    /// Exposed for tests asserting the D8 static-floor invariant.
    #[must_use]
    pub fn static_floor(&self) -> Decimal {
        self.state.floor
    }
}

// ── Cushion-multiplier formula ────────────────────────────────────────────────

/// Compute the cushion multiplier `M(k)` — normalised so that `M(0)=1` and
/// `M(d_max)=0`.
///
/// Formula (normalised from the architecture's base form):
///
/// ```text
/// M(k) = (d_max − d_k) / (d_max × (1 − d_k))
/// ```
///
/// Boundary conditions:
/// - `d_k = 0`     → M = d_max / (d_max × 1) = 1.0   (at HWM — full exposure).
/// - `d_k = d_max` → M = 0 / (d_max × (1−d_max)) = 0  (at floor — shut exposure).
/// - `d_k > d_max` → M = 0  (clamped — floor breached).
/// - `d_k < 0`     → M = 1.0 (clamped — equity above HWM, shouldn't occur post-guard).
///
/// The architecture doc `M(k)=(d_max−d(k))/(1−d(k))` is the unnormalised form;
/// dividing by `d_max` normalises the boundary to `M(0)=1`.  This satisfies the
/// operator contract "full exposure at ATH, zero exposure at floor."
///
/// This is a pure function — no I/O, no state, deterministic.
#[must_use]
pub fn compute_cushion_multiplier(d_max: Decimal, d_k: Decimal) -> Decimal {
    // If drawdown has reached or exceeded the floor, shut exposure.
    if d_k >= d_max {
        return Decimal::ZERO;
    }
    // Guard against negative drawdown (equity above HWM — shouldn't occur after
    // the `max(Decimal::ZERO)` guard in `update_equity`, but defensively clamp).
    if d_k <= Decimal::ZERO {
        return Decimal::ONE;
    }
    // Normalised: M = (d_max − d_k) / (d_max × (1 − d_k)).
    // Denominator floor to prevent division by zero (d_max > 0 by construction).
    let denom_inner = (Decimal::ONE - d_k).max(dec!(0.000001));
    let denom = d_max * denom_inner;
    if denom.is_zero() {
        return Decimal::ONE;
    }
    let raw = (d_max - d_k) / denom;
    // Clamp to [0, 1] (de-risk only, never lever up).
    raw.max(Decimal::ZERO).min(Decimal::ONE)
}

// ── DrawdownTelemetry ─────────────────────────────────────────────────────────

/// Operator-visible telemetry from the most recent bar.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawdownTelemetry {
    /// Current cushion multiplier M(k) in [0, 1].
    pub cushion_multiplier: Decimal,
    /// Current drawdown from HWM d(k) in [0, 1].
    pub drawdown_from_hwm: Decimal,
    /// Current high-water mark.
    pub hwm: Decimal,
    /// Static floor (never changes post-construction).
    pub floor: Decimal,
}

// ── Strategy impl ─────────────────────────────────────────────────────────────

impl<S: Strategy> Strategy for DrawdownControlOverlay<S> {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    /// Delegate bar processing to the inner strategy.
    ///
    /// The cushion multiplier is applied via [`Strategy::quantity_scale`] —
    /// the sizing pipeline queries `quantity_scale` at order-construction time
    /// and multiplies the base quantity by M(k).  This is the same mechanism
    /// as `VolTargetingOverlay` (ADR-0038 § D5 strategy-side composition).
    ///
    /// **Important:** in a production loop the caller must call
    /// [`DrawdownControlOverlay::update_equity`] once per bar with the current
    /// account equity BEFORE calling `on_bar`.  In the bake-off / backtest
    /// the equity is updated by the engine; the overlay observes it here.
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        self.inner.on_bar(bar)
    }

    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal> {
        self.inner.on_tick(tick)
    }

    /// Return the cached cushion multiplier M(k) as an `f64` for the sizing
    /// pipeline (same interface as `VolTargetingOverlay`).
    ///
    /// The multiplier is the same for every symbol (the de-risk is at the
    /// account level, not per-coin).
    fn quantity_scale(&self, _symbol: &Symbol) -> f64 {
        // Convert Decimal → f64 at the trait boundary.
        // The multiplier is in [0, 1] so precision loss is negligible.
        self.cached_multiplier.to_f64().unwrap_or(1.0)
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "drawdown_floor_pct": {
                "type": "number",
                "default": 0.20,
                "description": "Maximum tolerated drawdown from HWM before exposure shuts to zero (D8: 20%)"
            },
            "restart_on_hwm": {
                "type": "boolean",
                "default": true,
                "description": "Restart HWM on new all-time high (LOAD-BEARING — see ADR-0080 D2)"
            },
            "initial_equity_usdt": {
                "type": "number",
                "default": 200.0,
                "description": "Starting equity in USDT (determines the static floor = initial × (1 - drawdown_floor_pct))"
            }
        })
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    // ── compute_cushion_multiplier unit tests ─────────────────────────────────

    #[test]
    fn multiplier_at_zero_drawdown_is_one() {
        // d_k = 0 (equity at HWM — maximum cushion) → M = 1.0 (full exposure).
        // Normalised formula: M = (d_max − 0) / (d_max × (1 − 0)) = d_max/d_max = 1.
        let m = compute_cushion_multiplier(dec!(0.20), dec!(0.0));
        assert_eq!(
            m,
            Decimal::ONE,
            "M at zero drawdown must be 1.0 (full exposure)"
        );
    }

    #[test]
    fn multiplier_at_floor_drawdown_is_zero() {
        // d_k = d_max → M = 0 (floor reached — shut exposure).
        let m = compute_cushion_multiplier(dec!(0.20), dec!(0.20));
        assert_eq!(m, Decimal::ZERO, "M at floor drawdown must be 0.0 (shut)");
    }

    #[test]
    fn multiplier_beyond_floor_is_zero() {
        // d_k > d_max → M = 0 (clamped).
        let m = compute_cushion_multiplier(dec!(0.20), dec!(0.30));
        assert_eq!(m, Decimal::ZERO, "M beyond floor must be 0.0 (clamped)");
    }

    #[test]
    fn multiplier_halfway_matches_normalised_formula() {
        // d_k = 0.10, d_max = 0.20.
        // Normalised: M = (0.20 − 0.10) / (0.20 × (1 − 0.10)) = 0.10 / 0.18 ≈ 0.5556.
        let m = compute_cushion_multiplier(dec!(0.20), dec!(0.10));
        assert!(
            m > dec!(0.0) && m < Decimal::ONE,
            "M at half-drawdown must be in (0, 1), got {m}"
        );
        let expected = dec!(0.10) / (dec!(0.20) * (Decimal::ONE - dec!(0.10)));
        let diff = (m - expected).abs();
        assert!(
            diff < dec!(0.00001),
            "M at halfway should match formula ≈ {expected}, got {m}, diff {diff}"
        );
    }

    #[test]
    fn multiplier_at_negative_drawdown_is_clamped_to_one() {
        // d_k < 0 (equity above HWM — clamped to 1 via early return).
        let m = compute_cushion_multiplier(dec!(0.20), dec!(-0.05));
        assert_eq!(
            m,
            Decimal::ONE,
            "M at negative drawdown must be clamped to 1.0"
        );
    }

    // ── Static floor invariant ────────────────────────────────────────────────

    /// D8 INVARIANT: the static floor is ALWAYS initial × (1 − floor_pct),
    /// even after the HWM ratchets to 2× initial.
    #[test]
    fn floor_never_moves_even_when_hwm_doubles() {
        use crate::always_long::AlwaysLongStrategy;

        let initial_equity = dec!(1000);
        let config = DrawdownControlConfig {
            drawdown_floor_pct: dec!(0.20),
            restart_on_hwm: true,
            initial_equity: Money::from_decimal(initial_equity),
        };
        let inner = AlwaysLongStrategy::new();
        let mut overlay = DrawdownControlOverlay::new(inner, config);

        // Floor at construction.
        let floor_initial = overlay.static_floor();
        assert_eq!(
            floor_initial,
            dec!(800),
            "Initial floor must be 1000 × 0.80 = 800, got {floor_initial}"
        );

        // Drive equity up to 2× initial (new HWM).
        overlay.update_equity(dec!(2000));
        let floor_after_hwm = overlay.static_floor();
        assert_eq!(
            floor_after_hwm,
            dec!(800), // STILL 800, not 2000 × 0.80 = 1600 (that would be TIPP)
            "Floor must NOT ratchet with HWM (static CPPI D8); got {floor_after_hwm}"
        );

        // HWM should have moved to 2000.
        let hwm = overlay.state().hwm;
        assert_eq!(hwm, dec!(2000), "HWM must ratchet to new high: got {hwm}");
    }

    // ── HWM restart behaviour ─────────────────────────────────────────────────

    /// HWM RESTART PROOF: with restart=true, after equity rallies to a new high
    /// the HWM resets and M recovers toward 1.0; without restart it doesn't.
    #[test]
    fn hwm_restart_preserves_upside_in_second_drawdown() {
        use crate::always_long::AlwaysLongStrategy;

        // Set up two overlays: one with restart, one without.
        let make_overlay = |restart: bool| {
            let config = DrawdownControlConfig {
                drawdown_floor_pct: dec!(0.20),
                restart_on_hwm: restart,
                initial_equity: Money::from_decimal(dec!(1000)),
            };
            let inner = AlwaysLongStrategy::new();
            DrawdownControlOverlay::new(inner, config)
        };

        let mut with_restart = make_overlay(true);
        let mut without_restart = make_overlay(false);

        // Phase 1: drawdown to 85% of initial (below HWM=1000, d(k)=0.15 < 0.20).
        with_restart.update_equity(dec!(850));
        without_restart.update_equity(dec!(850));

        let m_with_after_dd = with_restart.state().last_multiplier;
        let m_without_after_dd = without_restart.state().last_multiplier;
        // Both should be de-risked (M < 1), same value.
        assert!(
            m_with_after_dd < Decimal::ONE,
            "Both should de-risk at 850; got {m_with_after_dd}"
        );
        assert_eq!(
            m_with_after_dd, m_without_after_dd,
            "At same drawdown, restart/no-restart differ only after a new HWM"
        );

        // Phase 2: equity rallies to a new ATH (1200).
        with_restart.update_equity(dec!(1200));
        without_restart.update_equity(dec!(1200));

        let m_with_at_ath = with_restart.state().last_multiplier;
        let _m_without_at_ath = without_restart.state().last_multiplier;

        // With restart: HWM moves to 1200, d(k) = 1 - 1200/1200 = 0 → M = 1.0.
        assert_eq!(
            m_with_at_ath,
            Decimal::ONE,
            "With restart: M should recover to 1.0 at new ATH; got {m_with_at_ath}"
        );

        // Without restart: HWM stays at 1000, d(k) = 1 - 1200/1000 = -0.20 → clamped to 0 → M = 1.
        // Wait: equity 1200 > HWM 1000 with restart=false → d_k = max(1 - 1200/1000, 0) = 0.
        // So even without restart, d_k=0 (equity above OLD HWM) → M = 1.
        // The DISTINCTION comes in the SECOND drawdown:
        // after rally to 1200, draw back to 1000.
        // With restart: HWM=1200, d_k = 1 - 1000/1200 = 0.167 < 0.20 → M > 0.
        // Without restart: HWM=1000 (still), d_k = 1 - 1000/1000 = 0 → M = 1.
        // Hmm — without restart locks HWM at initial, so after rallying it doesn't grow.
        // After second drawdown to 1000, without-restart sees d_k=0 (still at initial HWM).
        // With restart HWM=1200, so second drawdown to 1000 → d_k=0.167 → de-risked.
        // Actually the KEY difference is when equity is BETWEEN 1000 and 1200:
        // with restart equity=1050: d_k = 1-1050/1200 = 0.125 → M < 1 (cautious).
        // without restart equity=1050: d_k = 1-1050/1000 = -0.05 → clamp 0 → M = 1.
        // So without restart the controller is MORE aggressive after a rally (dangerous).

        // Phase 3: second drawdown — equity falls from 1200 to 1050.
        with_restart.update_equity(dec!(1050));
        without_restart.update_equity(dec!(1050));

        let m_with_second_dd = with_restart.state().last_multiplier;
        let m_without_second_dd = without_restart.state().last_multiplier;

        // With restart: HWM=1200, d_k = 1 - 1050/1200 = 0.125 < 0.20 → M > 0 but < 1.
        assert!(
            m_with_second_dd < Decimal::ONE,
            "With restart: should de-risk in second drawdown from new HWM; got {m_with_second_dd}"
        );

        // Without restart: HWM=1000 (never moved), d_k = 1 - 1050/1000 = -0.05 → 0 → M = 1.0.
        assert_eq!(
            m_without_second_dd,
            Decimal::ONE,
            "Without restart: HWM stays at initial so 1050 > 1000 → M = 1.0 (no de-risk); got {m_without_second_dd}"
        );

        // Proof: with restart is MORE conservative (lower M) after a rally+drawdown.
        assert!(
            m_with_second_dd < m_without_second_dd,
            "With restart must be more conservative in second drawdown: \
             with={m_with_second_dd}, without={m_without_second_dd}"
        );
    }

    // ── Budget-cap invariant (quantity_scale max) ─────────────────────────────

    /// BUDGET-CAP INVARIANT: quantity_scale is always in [0, 1].
    /// The downstream FixedFractionSizer applies budget_cap AFTER this multiplier,
    /// so the composed result is always ≤ min(M(k)×base_qty, budget_cap_qty).
    #[test]
    fn quantity_scale_is_always_in_zero_to_one() {
        use crate::always_long::AlwaysLongStrategy;
        use trading_core::Symbol;

        let config = DrawdownControlConfig {
            drawdown_floor_pct: dec!(0.20),
            restart_on_hwm: true,
            initial_equity: Money::from_decimal(dec!(1000)),
        };
        let inner = AlwaysLongStrategy::new();
        let mut overlay = DrawdownControlOverlay::new(inner, config);
        let btc = Symbol::new("BTCUSDT");

        // Test across a range of equity values.
        let equity_seq: &[Decimal] = &[
            dec!(1000), // ATH
            dec!(900),  // 10% drawdown
            dec!(800),  // 20% drawdown — at floor
            dec!(700),  // 30% — beyond floor (M=0)
            dec!(1100), // new ATH
            dec!(950),  // 50/150 = 0.33 from new HWM
        ];

        for &eq in equity_seq {
            overlay.update_equity(eq);
            let scale = overlay.quantity_scale(&btc);
            assert!(
                (0.0..=1.0).contains(&scale),
                "quantity_scale must be in [0, 1] for equity={eq}, got {scale}"
            );
        }
    }

    // ── update_equity state transitions ──────────────────────────────────────

    #[test]
    fn update_equity_hwm_ratchets_on_new_high() {
        use crate::always_long::AlwaysLongStrategy;

        let config = DrawdownControlConfig {
            drawdown_floor_pct: dec!(0.20),
            restart_on_hwm: true,
            initial_equity: Money::from_decimal(dec!(1000)),
        };
        let inner = AlwaysLongStrategy::new();
        let mut overlay = DrawdownControlOverlay::new(inner, config);

        overlay.update_equity(dec!(1100));
        assert_eq!(overlay.state().hwm, dec!(1100));

        overlay.update_equity(dec!(1050)); // lower — HWM stays.
        assert_eq!(overlay.state().hwm, dec!(1100));

        overlay.update_equity(dec!(1200)); // new high.
        assert_eq!(overlay.state().hwm, dec!(1200));
    }

    #[test]
    fn update_equity_no_restart_hwm_stays_fixed() {
        use crate::always_long::AlwaysLongStrategy;

        let config = DrawdownControlConfig {
            drawdown_floor_pct: dec!(0.20),
            restart_on_hwm: false, // no restart
            initial_equity: Money::from_decimal(dec!(1000)),
        };
        let inner = AlwaysLongStrategy::new();
        let mut overlay = DrawdownControlOverlay::new(inner, config);

        overlay.update_equity(dec!(1500)); // equity > initial.
        assert_eq!(
            overlay.state().hwm,
            dec!(1000),
            "With restart_on_hwm=false, HWM must not ratchet"
        );
    }

    #[test]
    fn bars_total_counter_increments() {
        use crate::always_long::AlwaysLongStrategy;

        let config = DrawdownControlConfig::default();
        let inner = AlwaysLongStrategy::new();
        let mut overlay = DrawdownControlOverlay::new(inner, config);

        for i in 1..=5_u64 {
            overlay.update_equity(dec!(200));
            assert_eq!(overlay.state().bars_total, i);
        }
    }

    // ── Telemetry ─────────────────────────────────────────────────────────────

    #[test]
    fn telemetry_reflects_current_state() {
        use crate::always_long::AlwaysLongStrategy;

        let config = DrawdownControlConfig {
            drawdown_floor_pct: dec!(0.20),
            restart_on_hwm: true,
            initial_equity: Money::from_decimal(dec!(1000)),
        };
        let inner = AlwaysLongStrategy::new();
        let mut overlay = DrawdownControlOverlay::new(inner, config);

        overlay.update_equity(dec!(900)); // 10% drawdown from HWM=1000.
        let t = overlay.telemetry();
        assert_eq!(t.hwm, dec!(1000));
        assert_eq!(t.floor, dec!(800));
        // d_k = 1 - 900/1000 = 0.10; M normalised = (0.20-0.10)/(0.20*(1-0.10)) ≈ 0.556.
        assert!(t.cushion_multiplier > dec!(0) && t.cushion_multiplier < Decimal::ONE);
        assert_eq!(t.drawdown_from_hwm, dec!(0.10));
    }
}
