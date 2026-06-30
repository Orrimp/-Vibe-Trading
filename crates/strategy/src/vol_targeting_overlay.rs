//! Vol-targeting overlay — repositioned as a **risk tool** (v2 Phase 2C / P1-4).
//!
//! **Honest framing (v2 reposition):** this overlay is a *risk-shaping* tool,
//! not a Sharpe tool. Crypto's leverage effect is reversed (γ = −0.261, Brini–Lenz
//! 2024) relative to equities (+0.115), meaning the Sharpe-gain mechanism that
//! makes vol targeting attractive on equity factors is **mechanistically absent or
//! adverse** on crypto. What the overlay **does** deliver — consistently across 60+
//! assets regardless of leverage-effect sign (Harvey et al.) — is **drawdown and
//! tail reduction**: thinner left tail, shallower max drawdown, lower vol-of-vol.
//! The overlay never promises Sharpe improvement; it promises a more risk-shaped
//! equity curve.
//!
//! ## Key design changes (P1-4 reparam):
//!
//! 1. **EWMA σ̂ source (slow 126-day half-life).**  `VolTargetingConfig::vol_source`
//!    selects between the new EWMA path (`ewma_realized_vol` from `vol_estimator.rs`,
//!    with `LAMBDA_126D_HOURLY` as the default for hourly-bar cadence) and the legacy
//!    GARCH path.  The slow λ ≈ 0.999 771 is intentional: tight vol-target tracking
//!    costs 1105%/yr turnover vs 93% open-loop (Boyd–Candès–Hastie); the slow decay
//!    avoids chasing transient spikes.
//!
//! 2. **No-trade band (`no_trade_band`).** Only resize if the relative change from
//!    the current scale factor exceeds the band: `|new − old| / old > band`. Default
//!    0.05 (5%). Caps turnover without requiring tight target tracking.
//!
//! 3. **De-risk-only (`derisk_only`).** When `true` (default), the overlay may only
//!    *reduce* position size (scale ≤ current_scale), never upsize on a vol drop.
//!    This is mandatory for a long-only no-leverage account — upsizing into low-vol
//!    would lever up against a gap risk we cannot hedge. The clamp effectively becomes
//!    `scale_clamp_max = 1.0` when `derisk_only = true`.
//!
//! 4. **Per-symbol return-vol correlation (`ReturnVolCorrelation`).** After each bar
//!    the overlay accumulates (return, sigma_hat) pairs and computes ρ(returns, σ̂)
//!    incrementally. The operator reads this to see whether the leverage effect is
//!    even present (negative ρ = crypto's reversed effect; positive ρ = upward-vol-
//!    after-rally — FOMO). This does not gate anything; it is diagnostic telemetry.
//!
//! ## Vol source (`VolSource`)
//!
//! ```text
//! VolSource::Ewma  → ewma_realized_vol(&returns, lambda)  [default; P1-4]
//! VolSource::Garch → GarchParams::forecast_step(r_prev, sigma_prev)  [legacy]
//! ```
//!
//! The GARCH path is kept for backward compatibility: tests and scenarios that
//! supply `GarchParams` models and do not set `VolTargetingConfig::vol_source` to
//! `Ewma` continue to work exactly as before (the existing e2e uses the Garch path).
//!
//! ## Scale computation (P1-4 logic):
//!
//! ```text
//! raw_scale      = clamp(target_vol / sigma_hat, scale_clamp_min, scale_clamp_max)
//! effective_max  = if derisk_only { current_scale.min(1.0) } else { scale_clamp_max }
//! candidate      = raw_scale.min(effective_max)
//! final_scale    = if |candidate − current| / current <= no_trade_band
//!                  { current }     ← hold, within band
//!                  else
//!                  { candidate }   ← resize
//! ```
//!
//! Initial current_scale is 1.0 (neutral: no position yet → pass through at full
//! budget).
//!
//! ## Bar cadence choice (hourly)
//!
//! The overlay's inner `MomentumStrategy` is configured at 1-hour bars; we use
//! `LAMBDA_126D_HOURLY` (≈ 0.999 771) for the EWMA path. For daily-bar consumers,
//! override to `LAMBDA_126D_DAILY` (≈ 0.994 514).
//!
//! ## Strategy composition
//!
//! ```text
//! bar → MomentumStrategy::on_bar() → base signals
//!     → VolTargetingOverlay::scale_signals() → scaled signals
//! ```
//!
//! The overlay does NOT modify the strategy ID, symbols, or signal direction —
//! only the implied quantity scale factor (ADR-0038 § D5).
//!
//! ## Determinism
//!
//! - No `SystemTime::now()` in `on_bar()`.
//! - All EWMA/GARCH parameters deterministic given fixed lambda or checkpoint.
//! - Scale factor is pure `f64` arithmetic.
//!
//! ## Cross-references
//!
//! - ADR-0038 § D5 — strategy-side composition lock.
//! - `crates/strategy/src/vol_estimator.rs` — `ewma_realized_vol` (the new default σ̂).
//! - `crates/forecast/src/garch.rs` — `GarchModel::forecast_step`.
//! - `crates/strategy/src/vol_killswitch_overlay.rs` — sibling overlay (reference).
//! - `spec/v2/advisor-vol-overlay-reposition/feature.md` — P1-4 spec.
//! - Harvey et al. — risk-shaping universal; Brini–Lenz 2024 γ=−0.261 on crypto.
//! - Boyd–Candès–Hastie — open-loop + band is better than tight tracking.

#![allow(clippy::float_arithmetic)] // statistical computations — intentional

use std::collections::BTreeMap;

use rust_decimal::prelude::ToPrimitive;
use trading_core::{Bar, Signal, StrategyId, Symbol, Tick};

use crate::Strategy;
use crate::cross_sectional::MomentumStrategy;
use crate::vol_estimator::{LAMBDA_126D_HOURLY, ewma_realized_vol};

// ── VolSource ────────────────────────────────────────────────────────────────

/// Selects the volatility estimation source for `VolTargetingOverlay`.
///
/// | Variant | Source | Default in P1-4? |
/// |---------|--------|------------------|
/// | `Ewma`  | `vol_estimator::ewma_realized_vol` with the configured `lambda` | **Yes** |
/// | `Garch` | `GarchParams::forecast_step` (legacy, ADR-0038 D5)            | No      |
///
/// The `Garch` variant is preserved for backward compatibility: existing tests and
/// scenarios that supply `GarchParams` models continue to work unmodified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolSource {
    /// Slow EWMA σ̂ (P1-4 default). Uses `crate::vol_estimator::ewma_realized_vol`
    /// with the lambda from `VolTargetingConfig::ewma_lambda`.
    Ewma,
    /// Legacy GARCH(1,1) recurrence (ADR-0038 § D5). Used when `GarchParams` are
    /// supplied and `vol_source = Garch` (or when backward-compat is required).
    Garch,
}

// ── VolTargetingConfig ────────────────────────────────────────────────────────

/// Configuration for `VolTargetingOverlay`.
///
/// ## P1-4 new fields
///
/// - `vol_source` — switches between EWMA (default) and GARCH (legacy).
/// - `ewma_lambda` — λ for the EWMA path (default: `LAMBDA_126D_HOURLY` ≈ 0.999771).
/// - `no_trade_band` — fractional dead-band: only resize when `|new−old|/old > band`.
/// - `derisk_only` — when `true`, never upsize (scale ≤ current_scale, capped at 1.0).
///
/// ## Backward-compatible defaults
///
/// The four original fields (`target_vol`, `scale_clamp_min`, `scale_clamp_max`,
/// `min_sigma_floor`) are unchanged.  The existing e2e test constructs
/// `VolTargetingConfig::default()` and drives `on_bar` with `GarchParams` models —
/// it uses `vol_source = Garch` and keeps `no_trade_band = 0.0` / `derisk_only = false`
/// to replicate pre-P1-4 behaviour exactly and keep the e2e green.
///
/// **For new consumers (P1-4 honest defaults):**
/// Call `VolTargetingConfig::p1_4_defaults()` which sets
/// `vol_source = Ewma`, `no_trade_band = 0.05`, `derisk_only = true`.
#[derive(Debug, Clone)]
pub struct VolTargetingConfig {
    /// Target per-bar σ (default 0.02, matching ADR-0038 § D5 and the GARCH σ̂ units).
    ///
    /// For the EWMA path with hourly bars the EWMA σ̂ is also expressed in per-bar
    /// (per-hour) units, so the ratio `target_vol / sigma_hat` is dimensionally correct.
    pub target_vol: f64,
    /// Lower clamp on the scale factor (default 0.5).
    ///
    /// For `derisk_only = true` the effective lower clamp is `scale_clamp_min`
    /// (unchanged), but the upper clamp is `1.0` (no upsizing).
    pub scale_clamp_min: f64,
    /// Upper clamp on the scale factor (default 2.0).
    ///
    /// Overridden to `1.0` when `derisk_only = true`.
    pub scale_clamp_max: f64,
    /// Floor on `sigma_hat` to prevent zero-division (default 1e-8).
    pub min_sigma_floor: f64,

    // ── P1-4 fields ──────────────────────────────────────────────────────────
    /// Vol estimation source.  Default: `VolSource::Garch` (backward-compat).
    /// Set to `VolSource::Ewma` for the P1-4 loose-and-slow recipe.
    pub vol_source: VolSource,
    /// EWMA smoothing parameter λ.  Only used when `vol_source = Ewma`.
    /// Default: `LAMBDA_126D_HOURLY` ≈ 0.999 771 (126-day half-life at hourly cadence).
    pub ewma_lambda: f64,
    /// No-trade band: fractional threshold below which the scale factor is NOT updated.
    ///
    /// If `|new_scale − current_scale| / current_scale <= no_trade_band`, the overlay
    /// holds its current scale (no turnover).  Default: `0.0` (no band; backward-compat).
    /// P1-4 honest default: `0.05` (5% band).
    pub no_trade_band: f64,
    /// De-risk-only flag.  When `true`, the overlay may only *reduce* position size
    /// (candidate scale capped at `current_scale.min(1.0)`), never upsize on a vol drop.
    ///
    /// Default: `false` (backward-compat; the GARCH e2e hits clamp_max = 2.0).
    /// P1-4 honest default: `true` (mandatory for a no-leverage long-only account).
    pub derisk_only: bool,
}

impl Default for VolTargetingConfig {
    /// Backward-compatible defaults: GARCH vol source, no no-trade band, no de-risk-only.
    ///
    /// These preserve the pre-P1-4 semantics so the existing
    /// `vol_targeting_overlay_end_to_end.rs` e2e stays green without modification.
    fn default() -> Self {
        Self {
            target_vol: 0.02,
            scale_clamp_min: 0.5,
            scale_clamp_max: 2.0,
            min_sigma_floor: 1e-8,
            // P1-4 new fields — backward-compatible defaults:
            vol_source: VolSource::Garch,
            ewma_lambda: LAMBDA_126D_HOURLY,
            no_trade_band: 0.0,
            derisk_only: false,
        }
    }
}

impl VolTargetingConfig {
    /// P1-4 honest defaults: EWMA vol source, 5% no-trade band, de-risk-only.
    ///
    /// Use these for new deployments where the overlay is sold as a risk tool, not
    /// a Sharpe tool. The slow EWMA (126-day half-life at hourly cadence) keeps
    /// turnover low; the 5% band further caps rebalancing; `derisk_only = true`
    /// ensures the overlay only ever *reduces* exposure on a vol spike.
    #[must_use]
    pub fn p1_4_defaults() -> Self {
        Self {
            target_vol: 0.02,
            scale_clamp_min: 0.5,
            scale_clamp_max: 2.0,
            min_sigma_floor: 1e-8,
            vol_source: VolSource::Ewma,
            ewma_lambda: LAMBDA_126D_HOURLY,
            no_trade_band: 0.05,
            derisk_only: true,
        }
    }
}

// ── PerSymbolGarchState ───────────────────────────────────────────────────────

/// Per-symbol GARCH recurrence state held by the overlay (legacy path).
#[derive(Debug, Clone)]
pub struct PerSymbolGarchState {
    /// Latest log-return `r_{t-1}` (from the prior bar's close / prev close).
    pub r_prev: f64,
    /// Previous σ prediction (initialised to `unconditional_var.sqrt()`).
    pub sigma_prev: f64,
    /// Previous bar's close price (for log-return derivation).
    pub prev_close: f64,
}

// ── PerSymbolEwmaState ────────────────────────────────────────────────────────

/// Per-symbol EWMA recurrence state held by the overlay (P1-4 path).
///
/// The EWMA recurrence (`vol_estimator::ewma_realized_vol`) needs the full
/// return history to compute the series. Since `on_bar` receives one bar at a
/// time, we maintain a rolling buffer of log-returns per symbol and call the
/// stateless `ewma_realized_vol` on the accumulated buffer, then cache the
/// last value as `sigma_hat`.
///
/// Memory: we cap the buffer at `MAX_EWMA_HISTORY` bars to bound per-symbol
/// memory. With a 126-day half-life λ ≈ 0.9998, weights at bar t−N contribute
/// `λ^N` of influence; at N = 10 000 hourly bars (≈ 417 days) a shock at bar 0
/// retains `0.9998^10000 ≈ e^{−1.93} ≈ 14%` weight — sufficient for the slow
/// smoother. We cap at this length so memory stays bounded.
#[derive(Debug, Clone)]
pub struct PerSymbolEwmaState {
    /// Rolling buffer of log-returns (oldest first).  Capped at `MAX_EWMA_HISTORY`.
    pub returns: Vec<f64>,
    /// Previous bar's close price (for log-return derivation).
    pub prev_close: f64,
    /// Cached σ̂ from the most recent EWMA computation.  `None` until the first bar.
    pub sigma_hat: Option<f64>,
}

/// Maximum log-return history kept per symbol for the EWMA path.
///
/// 10 000 hourly bars ≈ 417 days.  At λ ≈ 0.9998 a shock at bar 0 retains
/// `λ^10000 ≈ 14%` influence — acceptable for the slow smoother.
const MAX_EWMA_HISTORY: usize = 10_000;

impl PerSymbolEwmaState {
    fn new() -> Self {
        Self {
            returns: Vec::with_capacity(64),
            prev_close: 0.0,
            sigma_hat: None,
        }
    }

    /// Push a new close price, compute its log-return, and update the EWMA σ̂.
    /// Returns the current `sigma_hat` (or `None` before any price pair is seen).
    fn update(&mut self, close: f64, lambda: f64) -> Option<f64> {
        if self.prev_close > 0.0 && close > 0.0 {
            let r = (close / self.prev_close).ln();
            // Append and cap the buffer.
            self.returns.push(r);
            if self.returns.len() > MAX_EWMA_HISTORY {
                self.returns.remove(0);
            }
            // Recompute EWMA on the full buffer; take the last value.
            let sigmas = ewma_realized_vol(&self.returns, lambda);
            self.sigma_hat = sigmas.last().copied();
        }
        self.prev_close = close;
        self.sigma_hat
    }
}

// ── ReturnVolCorrelation ──────────────────────────────────────────────────────

/// Per-symbol return-vol correlation telemetry.
///
/// Accumulates (return, sigma_hat) pairs over the overlay's lifetime and computes
/// Pearson ρ(returns, σ̂). This is **operator-readable diagnostic data** only —
/// it never feeds the scale computation or the gate.
///
/// **Interpretation:**
/// - ρ < 0 (e.g. equity's typical −0.1 to −0.4): the leverage effect is present →
///   vol targeting may improve Sharpe (vol rises *after* down moves).
/// - ρ > 0 (crypto's typical +0.26): the leverage effect is *reversed* — vol rises
///   *after* up moves (FOMO effect). Vol targeting de-risks *after* the rally, not
///   after the crash. Expect only risk-shaping benefit; no Sharpe gain. This is the
///   honest message: crypto γ ≈ −0.261 (Brini–Lenz 2024) means ρ(ret,vol) > 0.
/// - |ρ| ≈ 0: no consistent relationship — vol targeting is decorrelated from returns.
#[derive(Debug, Default, Clone)]
pub struct ReturnVolCorrelation {
    /// Accumulated returns (paired with sigma_hat observations).
    returns: Vec<f64>,
    /// Accumulated sigma_hat values (paired with returns).
    sigmas: Vec<f64>,
    /// Cached Pearson ρ from the most recent computation.  `None` until ≥ 2 pairs.
    pub rho: Option<f64>,
}

impl ReturnVolCorrelation {
    /// Push a new (return, sigma_hat) observation and recompute ρ.
    ///
    /// Returns the updated ρ.
    pub fn push(&mut self, ret: f64, sigma_hat: f64) -> Option<f64> {
        self.returns.push(ret);
        self.sigmas.push(sigma_hat);
        self.rho = pearson_correlation(&self.returns, &self.sigmas);
        self.rho
    }

    /// Number of (return, sigma_hat) observations accumulated.
    #[must_use]
    pub fn n_obs(&self) -> usize {
        self.returns.len()
    }
}

/// Pearson correlation ρ(x, y).  Returns `None` when n < 2 or either series has
/// zero variance (constant series → correlation undefined).
fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len();
    if n < 2 || n != y.len() {
        return None;
    }
    let nf = n as f64;
    let mean_x = x.iter().sum::<f64>() / nf;
    let mean_y = y.iter().sum::<f64>() / nf;
    let mut cov = 0.0_f64;
    let mut var_x = 0.0_f64;
    let mut var_y = 0.0_f64;
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    if var_x < 1e-30 || var_y < 1e-30 {
        return None; // constant series → ρ undefined
    }
    Some(cov / (var_x * var_y).sqrt())
}

// ── GARCH model inline (avoids cross-crate dep in strategy for tests) ──────

/// Minimal GARCH(1,1) parameters — mirrors `forecast::garch::GarchModel`.
///
/// Stored inline so `crates/strategy` does not need `forecast` as a dependency
/// for non-`#[cfg(feature = "forecast")]` builds.
#[derive(Debug, Clone)]
pub struct GarchParams {
    pub omega: f64,
    pub alpha: f64,
    pub beta: f64,
    pub unconditional_var: f64,
}

impl GarchParams {
    /// One GARCH(1,1) recurrence step: `σ²_t = ω + α·r²_{t-1} + β·σ²_{t-1}`.
    /// Returns predicted σ (floored at √ω).
    #[inline]
    #[must_use]
    pub fn forecast_step(&self, r_prev: f64, sigma_prev: f64) -> f64 {
        let sigma2 =
            self.omega + self.alpha * r_prev * r_prev + self.beta * sigma_prev * sigma_prev;
        sigma2.max(self.omega).sqrt()
    }

    /// Initial sigma from unconditional variance.
    #[must_use]
    pub fn init_sigma(&self) -> f64 {
        self.unconditional_var.sqrt().max(self.omega.sqrt())
    }
}

// ── VolTargetingOverlay ───────────────────────────────────────────────────────

/// Vol-targeting overlay (P1-4 repositioned as a **risk tool**).
///
/// Wraps `MomentumStrategy` and scales signals by the clamped ratio
/// `target_vol / sigma_hat` with:
/// - Optional **no-trade band** (skip resize within band threshold).
/// - Optional **de-risk-only** (never upsize on a vol drop).
/// - Per-symbol **return-vol correlation** reporting (diagnostic telemetry for the
///   operator: "is the leverage-effect mechanism even present on this coin?").
///
/// The overlay implements `Strategy` by delegating `on_bar()` to the inner
/// `MomentumStrategy`, then applying the vol-targeting scale to each signal's
/// implied quantity via `Strategy::quantity_scale` at order-construction time
/// (ADR-0038 § D5 strategy-side composition).
pub struct VolTargetingOverlay {
    /// Strategy ID.
    id: StrategyId,
    /// Inner v1 momentum strategy.
    inner: MomentumStrategy,
    /// Per-symbol GARCH models (omega, alpha, beta, unconditional_var).
    /// Used only when `config.vol_source = Garch`.
    models: BTreeMap<Symbol, GarchParams>,
    /// Per-symbol GARCH recurrence state (legacy path).
    garch_state: BTreeMap<Symbol, PerSymbolGarchState>,
    /// Per-symbol EWMA state (P1-4 path).
    ewma_state: BTreeMap<Symbol, PerSymbolEwmaState>,
    /// Vol-targeting config.
    config: VolTargetingConfig,
    /// Scaling statistics (for diagnostics).
    pub stats: VolTargetingStats,
    /// Per-symbol cached scale factor from the most recent `on_bar`.
    /// Default 1.0 for symbols not yet seen (accessed via `quantity_scale`).
    scale_cache: BTreeMap<Symbol, f64>,
    /// Per-symbol return-vol correlation accumulator (P1-4 telemetry).
    pub return_vol_correlation: BTreeMap<Symbol, ReturnVolCorrelation>,
}

/// Running diagnostics for the vol-targeting scaler.
#[derive(Debug, Default, Clone)]
pub struct VolTargetingStats {
    /// Bars processed.
    pub bars_total: u64,
    /// Signals scaled (scale ≠ current_scale, i.e. a resize occurred).
    pub signals_scaled: u64,
    /// Signals passed through unchanged (no resize: within no-trade band or de-risk-only).
    pub signals_passthrough: u64,
    /// Bars with no GARCH model (symbol not in checkpoint; GARCH path only).
    pub bars_no_model: u64,
    /// Resize events suppressed by the no-trade band.
    pub band_suppressed: u64,
    /// Upsize attempts suppressed by `derisk_only`.
    pub derisk_suppressed: u64,
}

impl std::fmt::Debug for VolTargetingOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolTargetingOverlay")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("stats", &self.stats)
            .finish()
    }
}

impl VolTargetingOverlay {
    /// Construct from an inner momentum strategy and a set of GARCH models.
    ///
    /// `models` maps symbol names (e.g. `"BTCUSDT"`) to their fitted GARCH(1,1)
    /// parameters.  Symbols not in `models` receive pass-through (scale = 1.0) on
    /// the GARCH path.  On the EWMA path, `models` may be empty.
    #[must_use]
    pub fn new(
        inner: MomentumStrategy,
        models: BTreeMap<String, GarchParams>,
        config: VolTargetingConfig,
    ) -> Self {
        let id = StrategyId::new("vol_targeting_overlay_momentum");

        // Initialise per-symbol GARCH state from unconditional variance (legacy path).
        let garch_state: BTreeMap<Symbol, PerSymbolGarchState> = models
            .iter()
            .map(|(sym, m)| {
                (
                    Symbol::new(sym.as_str()),
                    PerSymbolGarchState {
                        r_prev: 0.0,
                        sigma_prev: m.init_sigma(),
                        prev_close: 0.0,
                    },
                )
            })
            .collect();

        let models_by_sym: BTreeMap<Symbol, GarchParams> = models
            .into_iter()
            .map(|(k, v)| (Symbol::new(k.as_str()), v))
            .collect();

        Self {
            id,
            inner,
            models: models_by_sym,
            garch_state,
            ewma_state: BTreeMap::new(),
            config,
            stats: VolTargetingStats::default(),
            scale_cache: BTreeMap::new(),
            return_vol_correlation: BTreeMap::new(),
        }
    }

    /// Compute the raw (unclamped-to-band, unclamped-to-derisk) vol-targeting scale.
    ///
    /// Returns a scale in `[scale_clamp_min, scale_clamp_max]`.
    /// If `sigma_hat` is below `min_sigma_floor`, the scale is clamped to max.
    #[must_use]
    pub fn compute_scale(&self, sigma_hat: f64) -> f64 {
        let sigma_safe = sigma_hat.max(self.config.min_sigma_floor);
        let raw = self.config.target_vol / sigma_safe;
        raw.max(self.config.scale_clamp_min)
            .min(self.config.scale_clamp_max)
    }

    /// Apply the no-trade band + de-risk-only policy to produce the final scale.
    ///
    /// Given:
    /// - `raw_scale`: output of `compute_scale(sigma_hat)`.
    /// - `current_scale`: the cached scale for this symbol (defaults to 1.0).
    ///
    /// Returns the `(final_scale, was_band_suppressed, was_derisk_suppressed)` triple.
    #[must_use]
    fn apply_policy(&self, raw_scale: f64, current_scale: f64) -> (f64, bool, bool) {
        // Step 1: de-risk-only cap — candidate may never exceed current_scale or 1.0.
        let (candidate, derisk_suppressed) =
            if self.config.derisk_only && raw_scale > current_scale.min(1.0) {
                (current_scale.min(1.0), true)
            } else {
                (raw_scale, false)
            };

        // Step 2: no-trade band — suppress resize if the relative change is within the band.
        let band_suppressed = if self.config.no_trade_band > 0.0 && current_scale > 0.0 {
            let relative_change = (candidate - current_scale).abs() / current_scale;
            relative_change <= self.config.no_trade_band
        } else {
            false
        };

        let final_scale = if band_suppressed {
            current_scale // hold — within band
        } else {
            candidate
        };

        (final_scale, band_suppressed, derisk_suppressed)
    }

    /// Inner momentum strategy reference (for tests).
    #[must_use]
    pub fn inner(&self) -> &MomentumStrategy {
        &self.inner
    }

    /// GARCH models reference (for tests).
    #[must_use]
    pub fn models(&self) -> &BTreeMap<Symbol, GarchParams> {
        &self.models
    }

    /// Current GARCH state for a symbol (for tests).
    #[must_use]
    pub fn garch_state(&self, symbol: &Symbol) -> Option<&PerSymbolGarchState> {
        self.garch_state.get(symbol)
    }

    /// Deprecated alias for `garch_state()` — kept for backward compatibility.
    #[must_use]
    pub fn state(&self, symbol: &Symbol) -> Option<&PerSymbolGarchState> {
        self.garch_state.get(symbol)
    }

    /// Current EWMA state for a symbol (for tests).
    #[must_use]
    pub fn ewma_state(&self, symbol: &Symbol) -> Option<&PerSymbolEwmaState> {
        self.ewma_state.get(symbol)
    }

    /// Return-vol correlation for a symbol (P1-4 telemetry).
    ///
    /// Returns `None` if no observations have been accumulated yet.
    #[must_use]
    pub fn return_vol_rho(&self, symbol: &Symbol) -> Option<f64> {
        self.return_vol_correlation.get(symbol).and_then(|c| c.rho)
    }

    /// Compute the sigma_hat for a bar using the GARCH path (legacy).
    fn sigma_from_garch(&mut self, bar: &Bar) -> f64 {
        let close_f64 = bar.close.get().to_f64().unwrap_or(0.0);

        if let Some(model) = self.models.get(&bar.symbol).cloned() {
            let state = self
                .garch_state
                .entry(bar.symbol.clone())
                .or_insert_with(|| PerSymbolGarchState {
                    r_prev: 0.0,
                    sigma_prev: model.init_sigma(),
                    prev_close: 0.0,
                });

            let r_curr = if state.prev_close > 0.0 && close_f64 > 0.0 {
                (close_f64 / state.prev_close).ln()
            } else {
                0.0
            };

            let sh = model.forecast_step(state.r_prev, state.sigma_prev);
            state.sigma_prev = sh;
            state.r_prev = r_curr;
            state.prev_close = close_f64;
            sh
        } else {
            self.stats.bars_no_model += 1;
            self.config.target_vol // scale = target_vol / target_vol = 1.0
        }
    }

    /// Compute the sigma_hat for a bar using the EWMA path (P1-4).
    ///
    /// Returns `None` when fewer than 2 bars have been seen for this symbol
    /// (first bar provides the initial close; second bar produces the first return).
    /// In that case the caller uses `target_vol` as a fallback (scale = 1.0).
    fn sigma_from_ewma(&mut self, bar: &Bar) -> f64 {
        let close_f64 = bar.close.get().to_f64().unwrap_or(0.0);
        let lambda = self.config.ewma_lambda;
        let target_vol = self.config.target_vol;

        let state = self
            .ewma_state
            .entry(bar.symbol.clone())
            .or_insert_with(PerSymbolEwmaState::new);

        state.update(close_f64, lambda).unwrap_or(target_vol)
    }
}

impl Strategy for VolTargetingOverlay {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        self.stats.bars_total += 1;

        // 1. Compute sigma_hat from the configured source.
        let sigma_hat = match self.config.vol_source {
            VolSource::Garch => self.sigma_from_garch(bar),
            VolSource::Ewma => self.sigma_from_ewma(bar),
        };

        // 2. Accumulate return-vol correlation telemetry (P1-4).
        //    The "return" here is the log-return for this bar derived from the
        //    EWMA state (if EWMA) or from the GARCH prev_close (if GARCH).
        //    We use a lightweight approximation: the last pushed log-return from
        //    whichever state is active; for simplicity we read from the EWMA
        //    buffer (which always tracks close prices) regardless of vol_source.
        let ret_for_corr: f64 = match self.config.vol_source {
            VolSource::Ewma => {
                // The EWMA state's returns buffer contains the last push — use it.
                self.ewma_state
                    .get(&bar.symbol)
                    .and_then(|s| s.returns.last().copied())
                    .unwrap_or(0.0)
            }
            VolSource::Garch => {
                // For the GARCH path track returns separately via garch_state.r_prev.
                self.garch_state
                    .get(&bar.symbol)
                    .map(|s| s.r_prev)
                    .unwrap_or(0.0)
            }
        };
        // Only push observations where sigma_hat is non-trivially above the floor
        // (avoids polluting the correlation with warm-up zeroes).
        if sigma_hat > self.config.min_sigma_floor * 10.0 && ret_for_corr != 0.0 {
            self.return_vol_correlation
                .entry(bar.symbol.clone())
                .or_default()
                .push(ret_for_corr, sigma_hat);
        }

        // 3. Compute raw scale, then apply no-trade-band + de-risk-only policy.
        let raw_scale = self.compute_scale(sigma_hat);
        let current_scale = self.scale_cache.get(&bar.symbol).copied().unwrap_or(1.0);
        let (final_scale, band_suppressed, derisk_suppressed) =
            self.apply_policy(raw_scale, current_scale);

        if band_suppressed {
            self.stats.band_suppressed += 1;
        }
        if derisk_suppressed {
            self.stats.derisk_suppressed += 1;
        }

        // 4. Cache the final scale (before the inner strategy call so the cache is
        //    always up-to-date even when the inner emits no signals this bar).
        self.scale_cache.insert(bar.symbol.clone(), final_scale);

        // 5. Delegate to inner momentum strategy.
        let base_signals = self.inner.on_bar(bar);

        if base_signals.is_empty() {
            return base_signals;
        }

        // 6. Update stats counters.
        let tol = 1e-6;
        if (final_scale - 1.0).abs() < tol {
            self.stats.signals_passthrough += base_signals.len() as u64;
        } else {
            self.stats.signals_scaled += base_signals.len() as u64;
        }
        base_signals
    }

    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal> {
        self.inner.on_tick(tick)
    }

    fn quantity_scale(&self, symbol: &Symbol) -> f64 {
        self.scale_cache.get(symbol).copied().unwrap_or(1.0)
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "target_vol": { "type": "number", "default": 0.02 },
            "scale_clamp_min": { "type": "number", "default": 0.5 },
            "scale_clamp_max": { "type": "number", "default": 2.0 },
            "min_sigma_floor": { "type": "number", "default": 1e-8 },
            "vol_source": { "type": "string", "enum": ["ewma", "garch"], "default": "garch" },
            "ewma_lambda": { "type": "number", "default": 0.999771 },
            "no_trade_band": { "type": "number", "default": 0.0 },
            "derisk_only": { "type": "boolean", "default": false },
            "momentum_config_id": { "type": "string", "default": "top10_momentum" },
            "forecaster_id": { "type": "string", "default": "garch-bs1" }
        })
    }
}

// ── Load from checkpoint ──────────────────────────────────────────────────────

/// Deserialisation types for the GARCH JSON checkpoint.
/// Mirrors `train_garch.rs::SymbolParams`.
#[cfg(feature = "forecast")]
#[allow(dead_code)]
mod checkpoint_loader {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::GarchParams;

    #[derive(serde::Deserialize)]
    struct SymbolEntry {
        omega: f64,
        alpha: f64,
        beta: f64,
        unconditional_var: f64,
    }

    #[derive(serde::Deserialize)]
    struct Checkpoint {
        params: BTreeMap<String, SymbolEntry>,
    }

    /// Load GARCH params from a JSON checkpoint file.
    ///
    /// # Errors
    ///
    /// Returns an error string if the file is not found or JSON is malformed.
    pub fn load_params(path: &Path) -> Result<BTreeMap<String, GarchParams>, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("read GARCH checkpoint {}: {e}", path.display()))?;
        let ck: Checkpoint = serde_json::from_str(&json)
            .map_err(|e| format!("parse GARCH checkpoint {}: {e}", path.display()))?;
        Ok(ck
            .params
            .into_iter()
            .map(|(sym, p)| {
                (
                    sym,
                    GarchParams {
                        omega: p.omega,
                        alpha: p.alpha,
                        beta: p.beta,
                        unconditional_var: p.unconditional_var,
                    },
                )
            })
            .collect())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use time::OffsetDateTime;
    use trading_core::symbol::Symbol;
    use trading_core::{Bar, Price, Quantity, Timeframe, Timestamp, Venue};

    use super::*;
    use crate::Strategy;
    use crate::cross_sectional::CrossSectionalMomentumConfig;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn stub_model() -> GarchParams {
        GarchParams {
            omega: 1e-6,
            alpha: 0.10,
            beta: 0.85,
            unconditional_var: 1e-6 / (1.0 - 0.10 - 0.85),
        }
    }

    fn strategy_stub_config() -> CrossSectionalMomentumConfig {
        let toml = r#"
id    = "top10_momentum_h1"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
            "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT"]
lookback_minutes = 60
rebalance_minutes = 60
k_long = 3
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#;
        CrossSectionalMomentumConfig::from_str(toml).expect("valid stub config")
    }

    fn make_inner() -> MomentumStrategy {
        MomentumStrategy::from_config(strategy_stub_config(), SmolStr::new("stub"))
    }

    fn make_bar(symbol: &str, minute: i64, close: rust_decimal::Decimal) -> Bar {
        let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(minute));
        Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneHour,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(1)).unwrap(),
            trade_count: 1,
            open_ts: ts,
            close_ts: ts,
            local_recv_ts: ts,
            venue: Venue::Binance,
        }
    }

    // ── Original GARCH-path tests (backward-compat) ───────────────────────────

    #[test]
    fn compute_scale_at_target_vol() {
        // When sigma_hat == target_vol, scale should be 1.0.
        let config = VolTargetingConfig::default();
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config.clone());
        let scale = overlay.compute_scale(config.target_vol);
        assert!(
            (scale - 1.0).abs() < 1e-9,
            "scale at target_vol should be 1.0, got {scale}"
        );
    }

    #[test]
    fn compute_scale_clamp_max() {
        let config = VolTargetingConfig::default();
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config.clone());
        let scale = overlay.compute_scale(1e-12);
        assert_eq!(
            scale, config.scale_clamp_max,
            "scale should be clamped to max"
        );
    }

    #[test]
    fn compute_scale_clamp_min() {
        let config = VolTargetingConfig::default();
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config.clone());
        let scale = overlay.compute_scale(100.0);
        assert_eq!(
            scale, config.scale_clamp_min,
            "scale should be clamped to min"
        );
    }

    #[test]
    fn garch_forecast_step_positive() {
        let m = stub_model();
        let sigma = m.forecast_step(0.01, 0.005);
        assert!(
            sigma > 0.0,
            "GARCH forecast_step must be positive, got {sigma}"
        );
    }

    #[test]
    fn garch_forecast_step_floored_at_sqrt_omega() {
        let m = stub_model();
        // With r_prev=0 and sigma_prev=0, sigma2 = omega → sigma = sqrt(omega).
        let sigma = m.forecast_step(0.0, 0.0);
        let expected = m.omega.sqrt();
        assert!(
            (sigma - expected).abs() < 1e-12,
            "forecast_step floored at sqrt(omega): expected {expected}, got {sigma}"
        );
    }

    #[test]
    fn vol_targeting_overlay_new_initialises_garch_state() {
        let mut models = BTreeMap::new();
        models.insert("BTCUSDT".to_string(), stub_model());
        let overlay = VolTargetingOverlay::new(make_inner(), models, VolTargetingConfig::default());
        let sym = Symbol::new("BTCUSDT");
        let state = overlay
            .garch_state(&sym)
            .expect("BTCUSDT GARCH state must be initialised");
        assert!(state.sigma_prev > 0.0, "sigma_prev should be > 0 on init");
        assert_eq!(state.r_prev, 0.0);
    }

    // ── P1-4 new tests: no-trade band ─────────────────────────────────────────

    #[test]
    fn no_trade_band_suppresses_small_change() {
        // If the candidate scale is within 5% of the current scale, the overlay
        // should hold — no resize.
        let config = VolTargetingConfig {
            no_trade_band: 0.05, // 5% band
            vol_source: VolSource::Garch,
            derisk_only: false,
            ..VolTargetingConfig::default()
        };
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let current_scale = 1.0_f64;
        // Candidate at 1.03 → relative change = 3% < 5% → suppressed.
        let candidate = 1.03;
        let (final_scale, band_suppressed, _) = overlay.apply_policy(candidate, current_scale);
        assert!(
            band_suppressed,
            "3% change within 5% band should be suppressed"
        );
        assert!(
            (final_scale - current_scale).abs() < 1e-9,
            "suppressed: final scale should stay at current {current_scale}, got {final_scale}"
        );
    }

    #[test]
    fn no_trade_band_allows_large_change() {
        // A 10% change with a 5% band should NOT be suppressed.
        let config = VolTargetingConfig {
            no_trade_band: 0.05,
            vol_source: VolSource::Garch,
            derisk_only: false,
            ..VolTargetingConfig::default()
        };
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let current_scale = 1.0_f64;
        let candidate = 0.88; // 12% change → outside 5% band
        let (final_scale, band_suppressed, _) = overlay.apply_policy(candidate, current_scale);
        assert!(
            !band_suppressed,
            "12% change outside 5% band should not be suppressed"
        );
        assert!(
            (final_scale - candidate).abs() < 1e-9,
            "not suppressed: final scale should be candidate {candidate}, got {final_scale}"
        );
    }

    #[test]
    fn no_trade_band_zero_is_passthrough() {
        // With no_trade_band = 0.0 every change (even tiny) should pass through.
        let config = VolTargetingConfig {
            no_trade_band: 0.0,
            vol_source: VolSource::Garch,
            derisk_only: false,
            ..VolTargetingConfig::default()
        };
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let current_scale = 1.0_f64;
        let candidate = 1.001; // tiny change
        let (final_scale, band_suppressed, _) = overlay.apply_policy(candidate, current_scale);
        assert!(!band_suppressed, "band=0 → no suppression");
        assert!(
            (final_scale - candidate).abs() < 1e-9,
            "band=0: final scale should equal candidate, got {final_scale}"
        );
    }

    // ── P1-4 new tests: de-risk-only ──────────────────────────────────────────

    #[test]
    fn derisk_only_blocks_upsize() {
        // When derisk_only=true and vol drops (raw_scale > current), the overlay
        // must NOT upsize — it holds at current_scale (capped at 1.0).
        let config = VolTargetingConfig {
            derisk_only: true,
            no_trade_band: 0.0, // no band interference
            vol_source: VolSource::Garch,
            ..VolTargetingConfig::default()
        };
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let current_scale = 0.8_f64;
        let raw_scale = 1.5; // vol dropped → raw wants to upsize above current 0.8
        let (final_scale, _, derisk_suppressed) = overlay.apply_policy(raw_scale, current_scale);
        assert!(
            derisk_suppressed,
            "vol drop (raw > current) should be de-risk-suppressed"
        );
        // final_scale = current_scale.min(1.0) = 0.8.min(1.0) = 0.8
        assert!(
            (final_scale - 0.8).abs() < 1e-9,
            "de-risk-only: scale should stay at current 0.8, got {final_scale}"
        );
    }

    #[test]
    fn derisk_only_allows_derisking() {
        // When derisk_only=true and vol spikes (raw_scale < current), the overlay
        // SHOULD cut — this is the intended de-risking direction.
        let config = VolTargetingConfig {
            derisk_only: true,
            no_trade_band: 0.0,
            vol_source: VolSource::Garch,
            ..VolTargetingConfig::default()
        };
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let current_scale = 1.0_f64;
        let raw_scale = 0.7; // vol spiked → wants to cut below current 1.0
        let (final_scale, band_suppressed, derisk_suppressed) =
            overlay.apply_policy(raw_scale, current_scale);
        assert!(
            !derisk_suppressed,
            "de-risking (raw < current) should not be suppressed"
        );
        assert!(!band_suppressed, "not in band: 30% change, band=0");
        assert!(
            (final_scale - 0.7).abs() < 1e-9,
            "de-risk-only: vol spike → cut to {raw_scale}, got {final_scale}"
        );
    }

    #[test]
    fn derisk_only_caps_at_one_not_current_when_current_above_one() {
        // When current_scale > 1.0 (shouldn't happen in practice with derisk_only,
        // but corner-case: initialized at 1.0 and the GARCH path could have produced >1).
        // The cap is min(current_scale, 1.0) = 1.0.
        let config = VolTargetingConfig {
            derisk_only: true,
            no_trade_band: 0.0,
            vol_source: VolSource::Garch,
            ..VolTargetingConfig::default()
        };
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let current_scale = 1.8_f64; // hypothetically above 1.0
        let raw_scale = 2.0; // vol dropped further — wants to upsize
        let (final_scale, _, derisk_suppressed) = overlay.apply_policy(raw_scale, current_scale);
        assert!(derisk_suppressed, "raw > current.min(1.0) → suppressed");
        // effective cap = current_scale.min(1.0) = 1.0
        assert!(
            (final_scale - 1.0).abs() < 1e-9,
            "cap at 1.0 when current=1.8: got {final_scale}"
        );
    }

    #[test]
    fn no_trade_band_and_derisk_interact_correctly() {
        // De-risk-only is applied FIRST, then the band.
        // Scenario: current=1.0, vol spikes → raw=0.92.  Band=5%.
        // Step1: derisk check: 0.92 < 1.0.min(1.0) → not suppressed, candidate=0.92.
        // Step2: band check: |0.92−1.0|/1.0 = 8% > 5% → not suppressed.
        // Result: resize to 0.92.
        let config = VolTargetingConfig {
            derisk_only: true,
            no_trade_band: 0.05,
            vol_source: VolSource::Garch,
            ..VolTargetingConfig::default()
        };
        let overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let (final_scale, band_suppressed, derisk_suppressed) = overlay.apply_policy(0.92, 1.0);
        assert!(!derisk_suppressed);
        assert!(!band_suppressed);
        assert!((final_scale - 0.92).abs() < 1e-9);
    }

    // ── P1-4 new tests: return-vol correlation ────────────────────────────────

    #[test]
    fn return_vol_correlation_positive_series() {
        // Construct pairs where returns and vol move together → ρ > 0.
        // Simulated: when return is high, sigma is also high.
        let mut corr = ReturnVolCorrelation::default();
        // Perfectly correlated series: (0.01, 0.01), (0.02, 0.02), (0.03, 0.03), ...
        for i in 1..=20 {
            let v = i as f64 * 0.01;
            corr.push(v, v);
        }
        let rho = corr.rho.expect("should have ρ after 20 observations");
        assert!(
            rho > 0.99,
            "perfectly correlated series → ρ ≈ 1.0, got {rho}"
        );
    }

    #[test]
    fn return_vol_correlation_negative_series() {
        // Negative correlation: high return → low vol (equity leverage effect).
        let mut corr = ReturnVolCorrelation::default();
        for i in 1..=20 {
            let ret = i as f64 * 0.01; // increasing returns
            let sig = 0.20 - i as f64 * 0.008; // decreasing vol
            corr.push(ret, sig);
        }
        let rho = corr.rho.expect("should have ρ after 20 observations");
        assert!(
            rho < -0.99,
            "perfectly anti-correlated series → ρ ≈ −1.0, got {rho}"
        );
    }

    #[test]
    fn return_vol_correlation_zero_corr_series() {
        // Alternating: ρ should be close to 0.
        let mut corr = ReturnVolCorrelation::default();
        // Returns alternate sign, vol is constant → ρ = 0 (constant vol has zero variance).
        // Use varying vol to avoid the constant-series branch.
        // Interleave: (0.01, 0.10), (−0.01, 0.12), (0.01, 0.10), (−0.01, 0.12), ...
        // ρ(ret, vol) over long series → 0 (no trend).
        for i in 0..40 {
            let ret = if i % 2 == 0 { 0.01 } else { -0.01 };
            let sig = if i % 2 == 0 { 0.10 } else { 0.12 };
            corr.push(ret, sig);
        }
        let rho = corr.rho.expect("should have ρ");
        // The series has ret ∈ {+0.01, −0.01} perfectly anti-correlated with sig ∈ {0.10, 0.12}
        // → ρ = −1.0 (high positive ret correlates with low vol).
        // Actually: when ret > 0, sig is lower; when ret < 0, sig is higher → negative ρ.
        assert!(
            rho < -0.99,
            "alternating: high-ret↔low-vol series → ρ ≈ −1.0, got {rho}"
        );
    }

    #[test]
    fn return_vol_correlation_near_zero_for_uncorrelated() {
        // Build truly uncorrelated series using a simple deterministic pseudo-mix.
        // Returns: a sine wave (no trend); sigmas: a cosine wave shifted by π/2.
        // sin and cos are orthogonal → ρ → 0 over a full period.
        let mut corr = ReturnVolCorrelation::default();
        let n = 100_usize;
        for i in 0..n {
            let t = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
            let ret = t.sin() * 0.05;
            let sig = (t + std::f64::consts::FRAC_PI_2).sin().abs() * 0.10 + 0.01;
            corr.push(ret, sig);
        }
        let rho = corr.rho.expect("should have ρ after 100 observations");
        // Pearson(sin(t), |cos(t)|) over a full period is near 0 (not exactly 0 due to
        // the abs() shifting the mean, but should be |rho| < 0.2 for a full cycle).
        assert!(rho.abs() < 0.2, "sin/|cos| series → ρ ≈ 0 (got {rho})");
    }

    #[test]
    fn return_vol_correlation_none_with_single_observation() {
        let mut corr = ReturnVolCorrelation::default();
        corr.push(0.01, 0.02);
        assert!(corr.rho.is_none(), "single observation → ρ = None (n < 2)");
    }

    #[test]
    fn return_vol_correlation_n_obs_counter() {
        let mut corr = ReturnVolCorrelation::default();
        assert_eq!(corr.n_obs(), 0);
        corr.push(0.01, 0.02);
        assert_eq!(corr.n_obs(), 1);
        corr.push(0.02, 0.03);
        assert_eq!(corr.n_obs(), 2);
    }

    // ── P1-4 new tests: EWMA vol source ──────────────────────────────────────

    #[test]
    fn ewma_vol_source_computes_sigma_after_warmup() {
        // With the EWMA vol source, after 10 bars with price variation the
        // sigma_hat should be > 0 and != target_vol (i.e. the overlay is actually
        // computing and using the EWMA, not falling back to the target_vol).
        let config = VolTargetingConfig::p1_4_defaults();
        let mut overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let btc = Symbol::new("BTCUSDT");

        // Drive bars with varying prices to generate log-returns.
        let prices = [
            dec!(50_000),
            dec!(51_000),
            dec!(49_500),
            dec!(52_000),
            dec!(50_800),
            dec!(53_000),
            dec!(51_200),
            dec!(54_000),
            dec!(52_500),
            dec!(55_000),
        ];
        for (i, &p) in prices.iter().enumerate() {
            overlay.on_bar(&make_bar("BTCUSDT", i as i64 * 60, p));
        }

        // After 10 bars (9 returns), the EWMA state should hold a sigma.
        let ewma = overlay
            .ewma_state(&btc)
            .expect("EWMA state must exist after on_bar");
        assert!(
            ewma.returns.len() >= 9,
            "should have 9 returns from 10 bars"
        );
        assert!(
            ewma.sigma_hat.is_some(),
            "sigma_hat should be computed after warmup"
        );
        let sigma = ewma.sigma_hat.unwrap();
        assert!(sigma > 0.0, "EWMA sigma must be positive, got {sigma}");
    }

    #[test]
    fn ewma_derisk_only_scale_never_exceeds_one() {
        // With derisk_only=true, after any sequence of bars the cached scale for
        // any symbol must be <= 1.0.
        let config = VolTargetingConfig::p1_4_defaults(); // derisk_only=true
        let mut overlay = VolTargetingOverlay::new(make_inner(), BTreeMap::new(), config);
        let btc = Symbol::new("BTCUSDT");

        // Drive with slowly rising prices (low log-returns → low EWMA sigma).
        // Low sigma → high raw_scale; but derisk_only clips at 1.0.
        let prices = [
            dec!(50_000),
            dec!(50_001),
            dec!(50_002),
            dec!(50_003),
            dec!(50_004),
            dec!(50_005),
            dec!(50_006),
            dec!(50_007),
        ];
        for (i, &p) in prices.iter().enumerate() {
            overlay.on_bar(&make_bar("BTCUSDT", i as i64 * 60, p));
        }

        let scale = overlay.quantity_scale(&btc);
        assert!(
            scale <= 1.0 + 1e-9,
            "derisk_only=true: scale must never exceed 1.0, got {scale}"
        );
    }

    #[test]
    fn p1_4_defaults_sets_expected_fields() {
        let config = VolTargetingConfig::p1_4_defaults();
        assert_eq!(
            config.vol_source,
            VolSource::Ewma,
            "p1_4_defaults → EWMA source"
        );
        assert!(
            (config.no_trade_band - 0.05).abs() < 1e-9,
            "p1_4_defaults → 5% band"
        );
        assert!(config.derisk_only, "p1_4_defaults → derisk_only=true");
        assert!(
            (config.ewma_lambda - LAMBDA_126D_HOURLY).abs() < 1e-12,
            "p1_4_defaults → 126d-hourly λ"
        );
    }

    // ── Pearson correlation helper unit tests ─────────────────────────────────

    #[test]
    fn pearson_empty_returns_none() {
        assert!(pearson_correlation(&[], &[]).is_none());
    }

    #[test]
    fn pearson_single_returns_none() {
        assert!(pearson_correlation(&[1.0], &[1.0]).is_none());
    }

    #[test]
    fn pearson_constant_x_returns_none() {
        // Zero variance in x → ρ undefined.
        let x = vec![1.0; 10];
        let y: Vec<f64> = (0..10).map(|i| i as f64).collect();
        assert!(
            pearson_correlation(&x, &y).is_none(),
            "constant x → ρ undefined"
        );
    }

    #[test]
    fn pearson_identity_gives_one() {
        let v: Vec<f64> = (0..20).map(|i| i as f64 * 0.01).collect();
        let rho = pearson_correlation(&v, &v).expect("identical series → ρ = 1.0");
        assert!((rho - 1.0).abs() < 1e-9, "x=y → ρ=1.0, got {rho}");
    }

    #[test]
    fn pearson_negated_gives_minus_one() {
        let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.01).collect();
        let y: Vec<f64> = x.iter().map(|&xi| -xi).collect();
        let rho = pearson_correlation(&x, &y).expect("negated → ρ = −1.0");
        assert!((rho + 1.0).abs() < 1e-9, "x=−y → ρ=−1.0, got {rho}");
    }
}
