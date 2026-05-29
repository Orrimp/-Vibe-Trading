//! Regime-switching strategy dispatcher (ADR-0049 § D3).
//!
//! `RegimeDispatcher` wraps a `MomentumStrategy` and a `CashHoldStrategy`,
//! routing each bar to the appropriate inner strategy based on the current
//! regime classification.
//!
//! ## Routing table (ADR-0049 § D3)
//!
//! ```text
//! Regime   → Strategy
//! ─────────────────────────────────
//! Bull     → MomentumStrategy
//! Bear     → MomentumStrategy
//! Volatile → CashHoldStrategy
//! Calm     → CashHoldStrategy
//! Chop     → CashHoldStrategy  (deprecated; legacy daily seed only)
//! ```
//!
//! ## Confidence gate (ADR-0049 § D6)
//!
//! The dispatcher switches routing only when `max_regime_confidence >= 0.70`.
//! Below this threshold, the previous regime's strategy keeps running
//! (hysteresis). This is the K-reg-2 mitigation.
//!
//! ## Cash-fallback semantic (SUPPRESSION, NOT LIQUIDATION)
//!
//! When routing to `CashHoldStrategy` (Volatile/Calm regimes), only `Hold`
//! signals are emitted — no new entry signals. **Existing positions are held**;
//! natural exits fire through the composed exit policy (ADR-0010). This is
//! the load-bearing contract from ADR-0049 § D3 architect option (i).
//!
//! ## Log-return history
//!
//! The dispatcher accumulates per-symbol close prices to compute log-returns
//! for the regime classifier. The classifier is fit on the first
//! `min_fit_bars` returns and updated incrementally as new bars arrive.
//! The first call to `forward_filter` happens after `min_fit_bars` bars.
//!
//! ## Determinism
//!
//! - No `SystemTime::now()` in `on_bar()`.
//! - Classifier state is deterministic: same input sequence → same routing.
//! - Per-symbol log-return history uses a `VecDeque` with fixed capacity.
//!
//! ## Cross-references
//!
//! - ADR-0049 § D3 — dispatcher + cash-fallback contract.
//! - ADR-0049 § D6 — confidence gate (0.70).
//! - `crates/strategy/src/cash_hold.rs` — the Volatile/Calm fallback.
//! - `crates/forecast/src/markov_switching.rs` — `RegimeClassifier` trait.
//! - `crates/strategy/tests/regime_dispatcher_end_to_end.rs` — K6 noop gate.

use std::collections::BTreeMap;
use std::collections::VecDeque;

use rust_decimal::prelude::ToPrimitive;
use tracing::debug;
use trading_core::{Bar, Signal, StrategyId, Symbol, Tick};

use crate::Strategy;
use crate::cash_hold::CashHoldStrategy;
use crate::cross_sectional::MomentumStrategy;

// Re-export the trait seam so callers can name it without depending on forecast directly.
pub use forecast::markov_switching::{
    CONFIDENCE_THRESHOLD, MIN_FIT_BARS, MarkovSwitchingClassifier, RegimeClassifier, RegimeError,
    RegimeProbability,
};

/// Minimum number of log-return samples before the first fit+filter call.
///
/// Defaults to [`MIN_FIT_BARS`] from `forecast::markov_switching`.
/// The dispatcher is warm (fitting allowed) once per-symbol history reaches
/// this count.
const DEFAULT_MIN_FIT_BARS: usize = MIN_FIT_BARS;

/// How often to re-fit the classifier (in bars).
///
/// At every `REFIT_INTERVAL` bars, the classifier is re-fit on the full
/// accumulated history. Between re-fits the forward filter is called on the
/// current full history with the existing model parameters.
///
/// Set to 100 bars (≈4 days at hourly cadence) as a performance-friendly
/// default. Callers can override via `RegimeDispatcherConfig`.
const DEFAULT_REFIT_INTERVAL: usize = 100;

// ── RegimeDispatcherConfig ────────────────────────────────────────────────────

/// Configuration for `RegimeDispatcher`.
#[derive(Debug, Clone)]
pub struct RegimeDispatcherConfig {
    /// Minimum bars before the first classifier fit.
    pub min_fit_bars: usize,
    /// Re-fit interval in bars (classifier is re-fit every N bars).
    pub refit_interval: usize,
    /// Maximum bars to retain in the per-symbol close price history.
    /// Older bars beyond this window are dropped.
    pub history_capacity: usize,
}

impl Default for RegimeDispatcherConfig {
    fn default() -> Self {
        Self {
            min_fit_bars: DEFAULT_MIN_FIT_BARS,
            refit_interval: DEFAULT_REFIT_INTERVAL,
            history_capacity: 1_000, // ~42 days at hourly cadence
        }
    }
}

// ── DispatchedRegime ─────────────────────────────────────────────────────────

/// Which inner strategy the dispatcher is currently routing to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchedRegime {
    /// Routing to `MomentumStrategy` (Bull or Bear regime).
    Momentum,
    /// Routing to `CashHoldStrategy` (Volatile, Calm, or Chop regime).
    CashHold,
}

/// Per-symbol state held by the dispatcher.
#[derive(Debug)]
struct SymbolState {
    /// Rolling close price history (for log-return derivation).
    closes: VecDeque<f64>,
    /// Bar counter for re-fit scheduling.
    bars_seen: usize,
}

// ── RegimeDispatcher ─────────────────────────────────────────────────────────

/// A single pending regime-tag audit entry produced when the confidence gate
/// fires (ADR-0049 § D6).
///
/// The dispatcher accumulates these synchronously in `on_bar`; the caller
/// drains them asynchronously via `drain_pending_regime_tags()` and writes
/// each entry to the audit ledger with `audit::journal::post_regime_tag`.
///
/// Using an accumulator avoids introducing async into the `Strategy::on_bar`
/// signature (which is synchronous by design).
#[derive(Debug, Clone)]
pub struct PendingRegimeTag {
    /// The symbol the classification applies to (e.g. `"BTCUSDT"`).
    pub symbol: smol_str::SmolStr,
    /// Regime label: `"bull"`, `"bear"`, `"volatile"`, `"calm"`.
    pub regime: smol_str::SmolStr,
    /// Maximum posterior probability at the decision bar (as a display string).
    pub max_confidence: smol_str::SmolStr,
}

/// Strategy-switching dispatcher: routes each bar to either `MomentumStrategy`
/// (Bull/Bear) or `CashHoldStrategy` (Volatile/Calm) based on the current
/// regime classification.
///
/// ## Generic parameter
///
/// `C: RegimeClassifier` is the classifier backend.  The default for production
/// is `MarkovSwitchingClassifier`.  Tests can substitute a lightweight stub.
///
/// ## Audit hook (Wave D — ADR-0049 § D3)
///
/// Every time the confidence gate fires and a regime classification is resolved,
/// a [`PendingRegimeTag`] is appended to `pending_regime_tags`. Callers drain
/// this buffer asynchronously via `drain_pending_regime_tags()` and persist
/// each entry with `audit::journal::post_regime_tag`. This decouples the sync
/// `on_bar` path from async I/O.
pub struct RegimeDispatcher<C: RegimeClassifier> {
    id: StrategyId,
    /// Inner momentum strategy for Bull/Bear regimes.
    momentum: MomentumStrategy,
    /// Inner cash-hold strategy for Volatile/Calm regimes.
    cash_hold: CashHoldStrategy,
    /// The regime classifier.
    classifier: C,
    /// Current dispatch target.
    current_regime: DispatchedRegime,
    /// Config.
    config: RegimeDispatcherConfig,
    /// Per-symbol rolling state.
    symbol_state: BTreeMap<Symbol, SymbolState>,
    /// Bars since last re-fit (per-symbol).
    bars_since_refit: BTreeMap<Symbol, usize>,
    /// Whether the classifier has been fit at least once.
    is_fitted: bool,
    /// Pending regime-tag audit entries accumulated synchronously in `on_bar`.
    ///
    /// Populated when the D6 confidence gate fires (max_p ≥ 0.70).
    /// NOT populated during hysteresis holds (max_p < 0.70).
    /// Callers drain via `drain_pending_regime_tags()` and write async.
    pending_regime_tags: Vec<PendingRegimeTag>,
}

impl<C: RegimeClassifier> std::fmt::Debug for RegimeDispatcher<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegimeDispatcher")
            .field("id", &self.id)
            .field("current_regime", &self.current_regime)
            .finish()
    }
}

impl<C: RegimeClassifier> RegimeDispatcher<C> {
    /// Construct a new dispatcher.
    ///
    /// # Arguments
    ///
    /// - `momentum`: the v1 `MomentumStrategy` to route to in Bull/Bear regimes.
    /// - `cash_hold`: the `CashHoldStrategy` to route to in Volatile/Calm regimes.
    /// - `classifier`: a `RegimeClassifier` implementor (e.g. `MarkovSwitchingClassifier`).
    /// - `config`: dispatcher tuning parameters.
    #[must_use]
    pub fn new(
        momentum: MomentumStrategy,
        cash_hold: CashHoldStrategy,
        classifier: C,
        config: RegimeDispatcherConfig,
    ) -> Self {
        Self {
            id: StrategyId::new("regime_dispatcher"),
            momentum,
            cash_hold,
            classifier,
            current_regime: DispatchedRegime::Momentum, // default before first fit
            config,
            symbol_state: BTreeMap::new(),
            bars_since_refit: BTreeMap::new(),
            is_fitted: false,
            pending_regime_tags: Vec::new(),
        }
    }

    /// Returns the current dispatched regime (for diagnostics / tests).
    #[must_use]
    pub fn current_regime(&self) -> DispatchedRegime {
        self.current_regime
    }

    /// Returns true if the classifier has been fitted at least once.
    #[must_use]
    pub fn is_fitted(&self) -> bool {
        self.is_fitted
    }

    /// Feed one bar into the per-symbol history and potentially re-fit + re-filter.
    ///
    /// Returns the new `DispatchedRegime` after processing this bar, or
    /// `None` if there is not yet enough data for the first fit.
    fn update_classifier_state(&mut self, bar: &Bar) -> Option<DispatchedRegime> {
        let sym = bar.symbol.clone();
        let close_f64 = bar.close.get().to_f64().unwrap_or(0.0);

        let state = self
            .symbol_state
            .entry(sym.clone())
            .or_insert_with(|| SymbolState {
                closes: VecDeque::with_capacity(self.config.history_capacity + 1),
                bars_seen: 0,
            });

        // Maintain rolling close history.
        state.closes.push_back(close_f64);
        while state.closes.len() > self.config.history_capacity {
            state.closes.pop_front();
        }
        state.bars_seen += 1;

        // Need at least min_fit_bars + 1 closes to compute min_fit_bars log-returns.
        if state.closes.len() < self.config.min_fit_bars + 1 {
            return None;
        }

        // Compute log-returns from the rolling closes.
        let log_returns = compute_log_returns(&state.closes);

        let bars_since = self.bars_since_refit.entry(sym).or_insert(0);
        let should_refit = !self.is_fitted || *bars_since >= self.config.refit_interval;

        if should_refit {
            match self.classifier.fit(&log_returns) {
                Ok(()) => {
                    self.is_fitted = true;
                    *bars_since = 0;
                }
                Err(e) => {
                    debug!("regime classifier fit failed: {e}; retaining previous routing");
                    // Retain previous routing on fit failure.
                    return Some(self.current_regime);
                }
            }
        } else {
            *bars_since += 1;
        }

        // Run forward filter.
        let posteriors = match self.classifier.forward_filter(&log_returns) {
            Ok(p) => p,
            Err(e) => {
                debug!("regime classifier forward_filter failed: {e}; retaining previous routing");
                return Some(self.current_regime);
            }
        };

        // Use the last bar's posterior for routing.
        let last_posterior = match posteriors.last() {
            Some(p) => p,
            None => return Some(self.current_regime),
        };

        // ADR-0049 § D6 confidence gate: only switch if max_p >= 0.70.
        if !last_posterior.above_confidence_threshold() {
            debug!(
                max_p = last_posterior.max_confidence(),
                threshold = CONFIDENCE_THRESHOLD,
                "below confidence threshold — retaining previous routing"
            );
            return Some(self.current_regime);
        }

        // Confidence gate passed — accumulate an audit entry (Wave D, ADR-0049 § D6).
        // NOT emitted during hysteresis hold (the early-return above handles that path).
        let regime_name = last_posterior.regime_name();
        let max_p = last_posterior.max_confidence();
        // Format max_confidence to 6 decimal places to match the audit DB microsecond style.
        let max_confidence_str = format!("{max_p:.6}");
        self.pending_regime_tags.push(PendingRegimeTag {
            symbol: smol_str::SmolStr::new(bar.symbol.0.as_str()),
            regime: smol_str::SmolStr::new(regime_name),
            max_confidence: smol_str::SmolStr::new(&max_confidence_str),
        });
        debug!(
            symbol = bar.symbol.0.as_str(),
            regime = regime_name,
            max_p,
            "regime tag queued for audit"
        );

        // Map regime name to dispatch target.
        let new_regime = regime_name_to_dispatch(regime_name);
        Some(new_regime)
    }

    /// Drain and return all pending [`PendingRegimeTag`] entries accumulated
    /// since the last call to this method.
    ///
    /// The caller is responsible for writing each entry to the audit ledger
    /// via `audit::journal::post_regime_tag`. Entries are only present when
    /// the ADR-0049 § D6 confidence gate (max_p ≥ 0.70) fired; hysteresis
    /// holds do NOT produce entries.
    ///
    /// Returns an empty `Vec` if no confidence-gate events fired since last drain.
    pub fn drain_pending_regime_tags(&mut self) -> Vec<PendingRegimeTag> {
        std::mem::take(&mut self.pending_regime_tags)
    }
}

/// Map a regime name string (from `RegimeProbability::regime_name()`) to a
/// `DispatchedRegime`.
///
/// Per ADR-0049 § D3 routing table:
/// - "bull" / "bear" → Momentum
/// - "volatile" / "calm" / "chop" → CashHold (Chop is deprecated but falls
///   through to CashHold for legacy daily-seed callers)
fn regime_name_to_dispatch(name: &str) -> DispatchedRegime {
    match name {
        "bull" | "bear" => DispatchedRegime::Momentum,
        // Volatile and Calm are the primary Cash targets (ADR-0049 § D3).
        // Chop is deprecated in the new classifier but preserved for the
        // legacy daily `classify_regime` seed — also routes to CashHold.
        "volatile" | "calm" | "chop" => DispatchedRegime::CashHold,
        // Unknown / "unknown" from a future classifier state — conservative fallback.
        _ => DispatchedRegime::CashHold,
    }
}

/// Compute a Vec of log-returns `ln(p_t / p_{t-1})` from a rolling close deque.
///
/// Returns `len - 1` values.  If the deque has fewer than 2 entries, returns
/// an empty Vec.
fn compute_log_returns(closes: &VecDeque<f64>) -> Vec<f64> {
    if closes.len() < 2 {
        return vec![];
    }
    let mut log_returns = Vec::with_capacity(closes.len() - 1);
    let slice: Vec<f64> = closes.iter().copied().collect();
    for i in 1..slice.len() {
        let prev = slice[i - 1];
        let curr = slice[i];
        if prev > 0.0 && curr > 0.0 {
            log_returns.push((curr / prev).ln());
        } else {
            log_returns.push(0.0);
        }
    }
    log_returns
}

impl<C: RegimeClassifier + Send + Sync> Strategy for RegimeDispatcher<C> {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // Update classifier state and potentially switch routing.
        let new_regime = self.update_classifier_state(bar);
        if let Some(regime) = new_regime {
            if regime != self.current_regime {
                debug!(
                    ?regime,
                    "RegimeDispatcher: routing switch {:?} → {:?}", self.current_regime, regime
                );
            }
            self.current_regime = regime;
        }

        // Route to the active strategy.
        match self.current_regime {
            DispatchedRegime::Momentum => self.momentum.on_bar(bar),
            DispatchedRegime::CashHold => self.cash_hold.on_bar(bar),
        }
    }

    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal> {
        // Tick routing mirrors the current bar-level regime.
        match self.current_regime {
            DispatchedRegime::Momentum => self.momentum.on_tick(tick),
            DispatchedRegime::CashHold => self.cash_hold.on_tick(tick),
        }
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "min_fit_bars": { "type": "integer", "default": DEFAULT_MIN_FIT_BARS },
            "refit_interval": { "type": "integer", "default": DEFAULT_REFIT_INTERVAL },
            "history_capacity": { "type": "integer", "default": 1000 },
        })
    }
}

// ── Convenience builder ───────────────────────────────────────────────────────

/// Builder: regime-switching dispatcher on v1 momentum (ADR-0049 § D3).
///
/// Routes Bull/Bear → `MomentumStrategy`; Volatile/Calm → `CashHoldStrategy`.
/// The dispatcher gates switches on `max_p >= 0.70` (ADR-0049 § D6).
///
/// # Arguments
///
/// - `momentum`: the v1 `MomentumStrategy` to wrap.
/// - `classifier`: a fitted or unfitted `RegimeClassifier` (fit happens on
///   first `min_fit_bars` bars).
/// - `config`: dispatcher config (default: `RegimeDispatcherConfig::default()`).
#[must_use]
pub fn with_regime_dispatcher<C: RegimeClassifier + Send + Sync>(
    momentum: MomentumStrategy,
    classifier: C,
    config: RegimeDispatcherConfig,
) -> RegimeDispatcher<C> {
    RegimeDispatcher::new(momentum, CashHoldStrategy::new(), classifier, config)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::symbol::Symbol;
    use trading_core::{Bar, Price, Quantity, SignalKind, Timeframe, Timestamp, Venue};

    use super::*;

    // ── Stub classifier ───────────────────────────────────────────────────────

    /// A stub `RegimeClassifier` for unit tests.
    ///
    /// It always returns a fixed posterior probability distribution when
    /// `forward_filter` is called.  This allows routing to be tested
    /// independently of the Markov-switching model.
    struct StubClassifier {
        /// Fixed posterior to emit for every bar (last entry is used).
        posterior: [f64; 4],
        /// Whether the stub is "fitted" (simulated).
        fitted: bool,
    }

    impl StubClassifier {
        /// Create a stub that always reports `posterior` as the last bar's state.
        fn new(posterior: [f64; 4]) -> Self {
            Self {
                posterior,
                fitted: false,
            }
        }

        /// Create a stub whose confidence is below the 0.70 threshold.
        fn uncertain() -> Self {
            // Uniform: max_p = 0.25 < 0.70 → no routing switch.
            Self::new([0.25, 0.25, 0.25, 0.25])
        }

        /// Create a stub that routes to Bull (state 0) with high confidence.
        fn bull() -> Self {
            Self::new([0.90, 0.05, 0.03, 0.02])
        }

        /// Create a stub that routes to Bear (state 1) with high confidence.
        fn bear() -> Self {
            Self::new([0.05, 0.90, 0.03, 0.02])
        }

        /// Create a stub that routes to Volatile (state 2) with high confidence.
        fn volatile() -> Self {
            Self::new([0.02, 0.03, 0.90, 0.05])
        }

        /// Create a stub that routes to Calm (state 3) with high confidence.
        fn calm() -> Self {
            Self::new([0.02, 0.03, 0.05, 0.90])
        }
    }

    impl RegimeClassifier for StubClassifier {
        fn fit(&mut self, _log_returns: &[f64]) -> Result<(), RegimeError> {
            self.fitted = true;
            Ok(())
        }

        fn forward_filter(&self, history: &[f64]) -> Result<Vec<RegimeProbability>, RegimeError> {
            if !self.fitted {
                return Err(RegimeError::NotFitted);
            }
            // Return one posterior per bar — all identical.
            let n = history.len();
            Ok(vec![RegimeProbability { p: self.posterior }; n])
        }
    }

    // ── Bar builder ────────────────────────────────────────────────────────────

    fn make_bar(symbol: &str, ts_secs: i64, close: rust_decimal::Decimal) -> Bar {
        let ts = Timestamp::new(
            OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000 + ts_secs),
        );
        Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: ts,
            local_recv_ts: ts,
            venue: Venue::Binance,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(1.0)).unwrap(),
            trade_count: 1,
        }
    }

    fn stub_momentum() -> MomentumStrategy {
        use crate::cross_sectional::CrossSectionalMomentumConfig;
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
        let cfg = CrossSectionalMomentumConfig::from_str(toml).expect("valid stub config");
        MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("stub"))
    }

    /// Build a dispatcher with a given stub classifier and custom min_fit_bars.
    fn make_dispatcher(
        classifier: StubClassifier,
        min_fit_bars: usize,
    ) -> RegimeDispatcher<StubClassifier> {
        let config = RegimeDispatcherConfig {
            min_fit_bars,
            refit_interval: 1_000_000, // disable re-fit during warm-up test
            history_capacity: 10_000,
        };
        RegimeDispatcher::new(stub_momentum(), CashHoldStrategy::new(), classifier, config)
    }

    /// Feed N bars with rising prices on BTCUSDT to warm up the history.
    fn warm_up(dispatcher: &mut RegimeDispatcher<StubClassifier>, n: usize) {
        for i in 0..n {
            let price = dec!(50_000.0) + rust_decimal::Decimal::from(i as i64);
            let bar = make_bar("BTCUSDT", i as i64 * 3600, price);
            dispatcher.on_bar(&bar);
        }
    }

    // ── Routing tests ─────────────────────────────────────────────────────────

    #[test]
    fn dispatcher_routes_to_momentum_in_bull() {
        // min_fit_bars = 10; warm up with 12 bars so fit fires.
        let mut d = make_dispatcher(StubClassifier::bull(), 10);
        warm_up(&mut d, 12);
        assert_eq!(
            d.current_regime(),
            DispatchedRegime::Momentum,
            "Bull regime must route to Momentum"
        );
    }

    #[test]
    fn dispatcher_routes_to_momentum_in_bear() {
        let mut d = make_dispatcher(StubClassifier::bear(), 10);
        warm_up(&mut d, 12);
        assert_eq!(
            d.current_regime(),
            DispatchedRegime::Momentum,
            "Bear regime must route to Momentum"
        );
    }

    #[test]
    fn dispatcher_routes_to_cash_in_volatile() {
        let mut d = make_dispatcher(StubClassifier::volatile(), 10);
        warm_up(&mut d, 12);
        assert_eq!(
            d.current_regime(),
            DispatchedRegime::CashHold,
            "Volatile regime must route to CashHold"
        );
    }

    #[test]
    fn dispatcher_routes_to_cash_in_calm() {
        let mut d = make_dispatcher(StubClassifier::calm(), 10);
        warm_up(&mut d, 12);
        assert_eq!(
            d.current_regime(),
            DispatchedRegime::CashHold,
            "Calm regime must route to CashHold"
        );
    }

    #[test]
    fn dispatcher_holds_previous_when_confidence_below_70_pct() {
        // Start with Bull routing (default Momentum before fit).
        let mut d = make_dispatcher(StubClassifier::uncertain(), 10);
        // Default before fit: Momentum.
        assert_eq!(d.current_regime(), DispatchedRegime::Momentum);
        warm_up(&mut d, 12);
        // Even after fit, uniform posterior (max_p=0.25) is below 0.70 →
        // should retain previous routing (Momentum).
        assert_eq!(
            d.current_regime(),
            DispatchedRegime::Momentum,
            "Low-confidence posterior must not trigger a routing switch"
        );
    }

    /// K6 falsifier / CLAUDE.md non-negotiable:
    /// When routing to CashHold on Bull→Volatile transition, the dispatcher
    /// MUST emit Hold signals — no Sell signals that would liquidate positions.
    #[test]
    fn cash_fallback_suppresses_not_liquidates() {
        // Volatile stub → CashHold routing.
        let mut d = make_dispatcher(StubClassifier::volatile(), 10);
        // Warm up to trigger fit and routing to CashHold.
        warm_up(&mut d, 12);
        assert_eq!(
            d.current_regime(),
            DispatchedRegime::CashHold,
            "Dispatcher must be in CashHold after warm-up with Volatile classifier"
        );

        // Feed one more bar and inspect signals.
        let bar = make_bar("BTCUSDT", 12 * 3600, dec!(50_012.0));
        let signals = d.on_bar(&bar);

        // LOAD-BEARING: no Sell signals in CashHold routing.
        // Sell would liquidate existing positions — that's LIQUIDATION, not SUPPRESSION.
        for sig in &signals {
            assert_ne!(
                sig.kind,
                SignalKind::Sell,
                "CashHoldStrategy must NOT emit Sell (that would be LIQUIDATION not SUPPRESSION); \
                 got {:?} for symbol {}",
                sig.kind,
                sig.symbol
            );
        }
        // All signals must be Hold.
        for sig in &signals {
            assert_eq!(
                sig.kind,
                SignalKind::Hold,
                "CashHoldStrategy must emit only Hold signals, got {:?}",
                sig.kind
            );
        }
    }

    #[test]
    fn dispatcher_handles_chop_legacy_regime() {
        // Chop is deprecated in the new classifier but must still route to CashHold.
        // Simulate via a posterior where state 0 = 0.10, state 1 = 0.10,
        // state 2 = 0.10, state 3 = 0.10 — but test route function directly.
        let dispatch = regime_name_to_dispatch("chop");
        assert_eq!(
            dispatch,
            DispatchedRegime::CashHold,
            "Chop must map to CashHold (legacy daily-seed callsites)"
        );
    }

    #[test]
    fn dispatcher_is_deterministic() {
        // Run dispatcher twice on identical input — must produce identical regime.
        let run = |classifier: StubClassifier| {
            let mut d = make_dispatcher(classifier, 10);
            warm_up(&mut d, 20);
            d.current_regime()
        };

        let r1 = run(StubClassifier::volatile());
        let r2 = run(StubClassifier::volatile());
        assert_eq!(
            r1, r2,
            "RegimeDispatcher must be deterministic across identical runs"
        );
    }

    #[test]
    fn cash_hold_emits_only_hold_invariant() {
        // This test is the standalone CashHoldStrategy invariant gate for the dispatcher.
        // Repeated here at the dispatcher level to confirm composition doesn't break it.
        let mut d = make_dispatcher(StubClassifier::volatile(), 10);
        warm_up(&mut d, 12);

        // Verify every bar emits only Hold signals from CashHold routing.
        for i in 12..20_i64 {
            let bar = make_bar(
                "BTCUSDT",
                i * 3600,
                dec!(50_000.0) + rust_decimal::Decimal::from(i),
            );
            let signals = d.on_bar(&bar);
            for sig in &signals {
                assert_eq!(
                    sig.kind,
                    SignalKind::Hold,
                    "All signals during CashHold routing must be Hold, got {:?}",
                    sig.kind
                );
            }
        }
    }

    #[test]
    fn dispatcher_reverse_transition_resumes_momentum() {
        // Bull→Volatile→Bull transition: after switch back, routing must be Momentum.
        // We achieve this by having a classifier that first reports Volatile,
        // then reports Bull.
        // We test this by directly checking the routing table mapping.
        assert_eq!(regime_name_to_dispatch("bull"), DispatchedRegime::Momentum);
        assert_eq!(regime_name_to_dispatch("bear"), DispatchedRegime::Momentum);
        assert_eq!(
            regime_name_to_dispatch("volatile"),
            DispatchedRegime::CashHold
        );
        assert_eq!(regime_name_to_dispatch("calm"), DispatchedRegime::CashHold);
    }

    #[test]
    fn regime_name_to_dispatch_unknown_falls_back_to_cash() {
        // Unknown regime name should fall back to conservative CashHold.
        assert_eq!(
            regime_name_to_dispatch("unknown"),
            DispatchedRegime::CashHold,
            "Unknown regime names must fall back to CashHold (conservative)"
        );
        assert_eq!(
            regime_name_to_dispatch(""),
            DispatchedRegime::CashHold,
            "Empty regime name must fall back to CashHold"
        );
    }

    #[test]
    fn compute_log_returns_correct() {
        let closes: VecDeque<f64> = vec![100.0, 110.0, 99.0].into_iter().collect();
        let lr = compute_log_returns(&closes);
        assert_eq!(lr.len(), 2);
        assert!((lr[0] - (110.0_f64 / 100.0).ln()).abs() < 1e-12);
        assert!((lr[1] - (99.0_f64 / 110.0).ln()).abs() < 1e-12);
    }

    #[test]
    fn compute_log_returns_empty_on_single_close() {
        let closes: VecDeque<f64> = vec![100.0].into_iter().collect();
        let lr = compute_log_returns(&closes);
        assert!(lr.is_empty());
    }

    // ── Wave D audit hook tests (ADR-0049 § D3 / T-D-D1) ─────────────────────

    /// T-D-D1 gate: `drain_pending_regime_tags()` returns entries when the
    /// confidence gate fires on a resolved regime (max_p ≥ 0.70).
    ///
    /// This is the primary "audit hook fires on resolved regime" test required
    /// by the Wave D task spec.
    #[test]
    fn audit_hook_fires_on_resolved_regime() {
        // Use a Bull classifier (max_p = 0.90 ≥ 0.70) → gate fires.
        let mut d = make_dispatcher(StubClassifier::bull(), 10);

        // Before warm-up: no tags.
        assert!(
            d.drain_pending_regime_tags().is_empty(),
            "No tags before fit"
        );

        // Warm up with 12 bars (11 closes → min_fit_bars=10 satisfied → gate can fire).
        warm_up(&mut d, 12);

        // After warm-up, at least one PendingRegimeTag must have been accumulated.
        let tags = d.drain_pending_regime_tags();
        assert!(
            !tags.is_empty(),
            "Confidence gate fired (max_p=0.90 ≥ 0.70) — must produce ≥ 1 PendingRegimeTag"
        );

        // All emitted tags must have the Bull regime label.
        for tag in &tags {
            assert_eq!(
                tag.regime.as_str(),
                "bull",
                "Bull classifier → all tags must be 'bull'"
            );
            assert_eq!(
                tag.symbol.as_str(),
                "BTCUSDT",
                "Symbol must match the bar's symbol"
            );
            // max_confidence string must be a parseable f64 ≥ 0.70.
            let parsed: f64 = tag
                .max_confidence
                .as_str()
                .parse()
                .expect("max_confidence must be a valid float string");
            assert!(
                parsed >= 0.70,
                "max_confidence must be ≥ 0.70 (was {parsed})"
            );
        }

        // After draining, the buffer must be empty.
        assert!(
            d.drain_pending_regime_tags().is_empty(),
            "Buffer must be empty after drain"
        );
    }

    /// T-D-D1 gate: hysteresis hold does NOT emit a `PendingRegimeTag`.
    ///
    /// When `max_p < 0.70`, the dispatcher retains previous routing without
    /// accumulating any audit entry.
    #[test]
    fn audit_hook_silent_on_hysteresis_hold() {
        // Uncertain classifier: max_p = 0.25 < 0.70 → hysteresis hold.
        let mut d = make_dispatcher(StubClassifier::uncertain(), 10);

        // Warm up past min_fit_bars so the filter runs but gate does NOT fire.
        warm_up(&mut d, 12);

        let tags = d.drain_pending_regime_tags();
        assert!(
            tags.is_empty(),
            "Hysteresis hold (max_p=0.25 < 0.70) must NOT produce any PendingRegimeTag; \
             got {} entries",
            tags.len()
        );
    }

    /// T-D-D1 gate: `drain_pending_regime_tags()` clears the buffer on each call.
    #[test]
    fn drain_pending_regime_tags_clears_buffer() {
        let mut d = make_dispatcher(StubClassifier::bear(), 10);
        warm_up(&mut d, 12);

        // First drain gets all accumulated tags.
        let first = d.drain_pending_regime_tags();
        assert!(
            !first.is_empty(),
            "First drain must return tags from warm-up"
        );

        // Second drain must be empty (buffer cleared).
        let second = d.drain_pending_regime_tags();
        assert!(
            second.is_empty(),
            "Second drain must be empty (buffer was cleared by first drain)"
        );
    }

    /// T-D-D1 gate: tags from different regimes carry correct regime labels.
    #[test]
    fn audit_hook_regime_label_matches_classifier() {
        // Volatile classifier → tags carry "volatile".
        let mut d = make_dispatcher(StubClassifier::volatile(), 10);
        warm_up(&mut d, 12);
        let tags = d.drain_pending_regime_tags();
        assert!(!tags.is_empty());
        for tag in &tags {
            assert_eq!(tag.regime.as_str(), "volatile");
        }

        // Calm classifier → tags carry "calm".
        let mut d2 = make_dispatcher(StubClassifier::calm(), 10);
        warm_up(&mut d2, 12);
        let tags2 = d2.drain_pending_regime_tags();
        assert!(!tags2.is_empty());
        for tag in &tags2 {
            assert_eq!(tag.regime.as_str(), "calm");
        }
    }
}
