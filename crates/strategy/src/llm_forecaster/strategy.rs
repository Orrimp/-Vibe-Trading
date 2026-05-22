//! `LlmForecasterStrategy: Strategy` — the `on_bar` consumer.
//!
//! ## Wave A scope
//!
//! This file contains the type skeleton only. `on_bar` returns
//! `Vec::new()` when the strategy is disabled (R9.3 default) or when
//! the carry-forward is active. The real LLM call path lands in Wave C
//! after `LlmForecasterImpl` (Wave B) is wired.
//!
//! ## Signal pipeline (T-AR-1)
//!
//! ```text
//! on_bar(bar):
//!   1. Push bar to per-symbol rolling OHLCV window (R2.1).
//!   2. If not yet at fire cadence → carry forward last LlmForecast.
//!   3. ForecastContext::test_fixture()/from_runtime() → request_hash().
//!   4. forecaster.forecast(ctx) → LlmForecast.
//!   5. Cache last_forecast.insert(symbol, forecast).
//!   6. Emit Signal per Rating::to_signal_kind() mapping (T-AR-1).
//!   7. Wave E: emit audit row + AuditTick (deferred).
//!   8. Return vec![signal].
//! ```
//!
//! ## Carry-forward (R5.4)
//!
//! Between fire ticks the strategy reuses the last `LlmForecast`. This
//! avoids calling the LLM on every bar (default: 24-bar cadence = once
//! per day on hourly bars). The `bars_since_last_fire` counter resets
//! on every fire.
//!
//! ## Cross-references
//!
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-1` — call sequence.
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-7 Wave C` — real impl.
//! - `crates/strategy/src/traits.rs` — `Strategy` trait.

use std::collections::HashMap;
use std::sync::Arc;

use trading_core::{Bar, Signal, SignalEvidence, StrategyId, Symbol, Tick};

use crate::Strategy;

use super::trait_def::LlmForecaster;
use super::types::{ForecastContext, LlmForecast, LlmForecasterConfig, LlmForecasterError, Rating};

/// Strategy ID registered in the registry under name `"llm_forecaster_v3"`.
pub const STRATEGY_ID: &str = "llm_forecaster_v3";

// ── Per-symbol state ──────────────────────────────────────────────────────────

/// Per-symbol rolling state maintained by `LlmForecasterStrategy`.
#[derive(Debug)]
struct SymbolState {
    /// Rolling OHLCV window (last N bars per `fire_every_n_bars`).
    recent_bars: Vec<Bar>,
    /// How many bars have elapsed since the last LLM fire.
    bars_since_last_fire: u32,
    /// Last forecast emitted for this symbol (carry-forward between fires).
    last_forecast: Option<LlmForecast>,
}

impl SymbolState {
    fn new() -> Self {
        Self {
            recent_bars: Vec::new(),
            bars_since_last_fire: 0,
            last_forecast: None,
        }
    }

    /// Push a new bar; keep at most `window_size` bars.
    fn push_bar(&mut self, bar: Bar, window_size: usize) {
        self.recent_bars.push(bar);
        if self.recent_bars.len() > window_size {
            self.recent_bars.remove(0);
        }
    }
}

// ── LlmForecasterStrategy ─────────────────────────────────────────────────────

/// LLM-based directional forecasting strategy.
///
/// Implements `Strategy` by calling `LlmForecaster::forecast()` once per
/// `config.fire_every_n_bars` bars, carrying forward the last forecast
/// between fires (R5.4).
///
/// ## Wave A note
///
/// In Wave A `on_bar` returns `Vec::new()` for all bars when
/// `config.enabled = false` (the default). When enabled with a
/// `StubForecaster`, it exercises the carry-forward / fire-cadence path
/// without making real LLM calls.
pub struct LlmForecasterStrategy {
    /// Configuration (model ID, fire cadence, cost caps, enabled flag).
    config: LlmForecasterConfig,
    /// The underlying LLM forecaster (stub at Wave A; real impl at Wave C).
    forecaster: Arc<dyn LlmForecaster>,
    /// Per-symbol rolling state.
    symbol_state: HashMap<Symbol, SymbolState>,
    /// Tokio runtime handle for `block_on` in the sync `on_bar` path.
    /// None in unit tests (stub forecaster is synchronous via async fn).
    rt: Option<tokio::runtime::Handle>,
}

impl std::fmt::Debug for LlmForecasterStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmForecasterStrategy")
            .field("config", &self.config)
            .finish()
    }
}

impl LlmForecasterStrategy {
    /// Construct a new strategy.
    ///
    /// `rt` is the tokio runtime handle for `block_on`. Pass `None` in
    /// unit tests when using a stub forecaster (the test runtime handles
    /// blocking automatically).
    #[must_use]
    pub fn new(
        config: LlmForecasterConfig,
        forecaster: Arc<dyn LlmForecaster>,
        rt: Option<tokio::runtime::Handle>,
    ) -> Self {
        Self {
            config,
            forecaster,
            symbol_state: HashMap::new(),
            rt,
        }
    }

    /// Get or insert the per-symbol state.
    fn state_mut(&mut self, symbol: &Symbol) -> &mut SymbolState {
        self.symbol_state
            .entry(symbol.clone())
            .or_insert_with(SymbolState::new)
    }

    /// Emit a `Signal` from a `Rating` + bar context.
    fn rating_to_signal(&self, rating: Rating, bar: &Bar) -> Signal {
        Signal {
            strategy_id: StrategyId(smol_str::SmolStr::new(STRATEGY_ID)),
            symbol: bar.symbol.clone(),
            ts: bar.close_ts,
            kind: rating.to_signal_kind(),
            evidence: SignalEvidence {
                fast_ma: None,
                slow_ma: None,
                extra: Default::default(),
            },
            pair_data: None,
        }
    }

    /// Drive the forecast call via `block_on` if a tokio handle is available,
    /// or via `futures::executor::block_on` otherwise (test path).
    fn call_forecast(&self, ctx: ForecastContext) -> Result<LlmForecast, LlmForecasterError> {
        let forecaster = Arc::clone(&self.forecaster);
        if let Some(handle) = &self.rt {
            handle.block_on(forecaster.forecast(ctx))
        } else {
            // Test path: no tokio runtime handle; use pollster for sync dispatch.
            pollster::block_on(forecaster.forecast(ctx))
        }
    }
}

impl Strategy for LlmForecasterStrategy {
    fn id(&self) -> StrategyId {
        StrategyId(smol_str::SmolStr::new(STRATEGY_ID))
    }

    /// Process one bar.
    ///
    /// ## Wave A behaviour
    ///
    /// - Returns `Vec::new()` when `config.enabled = false` (default).
    /// - When enabled with a stub forecaster:
    ///   - Increments the per-symbol fire counter.
    ///   - On the first bar (or every `fire_every_n_bars` bars): calls
    ///     `forecaster.forecast()` and caches the result.
    ///   - Between fire ticks: carry-forwards the last cached forecast.
    ///   - Returns a `Vec<Signal>` with one signal per bar.
    ///
    /// ## Wave C behaviour (after real LlmForecasterImpl is wired)
    ///
    /// Same cadence; `forecast()` calls the real LLM via `BudgetedProvider`
    /// wrapping `RecordingProvider` / `ReplayProvider` for determinism.
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // R9.3: default-disabled guard. Phase F byte-identity preserved.
        if !self.config.enabled {
            return Vec::new();
        }

        let sym = bar.symbol.clone();
        let window_size = self.config.fire_every_n_bars as usize;

        // Step 1: push bar to rolling window.
        {
            let state = self.state_mut(&sym);
            state.push_bar(bar.clone(), window_size);
            state.bars_since_last_fire += 1;
        }

        // Step 2: check if it's time to fire.
        let should_fire = {
            let state = self.symbol_state.get(&sym).expect("just inserted");
            state.last_forecast.is_none()
                || state.bars_since_last_fire >= self.config.fire_every_n_bars
        };

        if should_fire {
            // Step 3: build ForecastContext.
            // TODO Wave C: retrieve top-K from reflection-memory.
            let recent_bars = {
                let state = self.symbol_state.get(&sym).expect("just inserted");
                state.recent_bars.clone()
            };
            let ctx = ForecastContext::test_fixture(sym.clone(), bar.open_ts, recent_bars);

            // Step 4: call forecaster.
            match self.call_forecast(ctx) {
                Ok(forecast) => {
                    // Step 5: cache forecast + reset counter.
                    let signal = self.rating_to_signal(forecast.rating, bar);
                    let state = self.state_mut(&sym);
                    state.last_forecast = Some(forecast);
                    state.bars_since_last_fire = 0;
                    // Step 7: Wave E adds audit row + AuditTick emission here.
                    vec![signal]
                }
                Err(e) => {
                    if e.is_backtest_fatal() {
                        // In backtest mode: propagate fatally. Wave D wires
                        // this to a non-zero exit; for now, log + panic to
                        // surface the issue immediately.
                        panic!("fatal LLM forecaster error (backtest): {e}");
                    }
                    tracing::error!(
                        target: "llm_forecaster::strategy",
                        symbol = %sym,
                        error = %e,
                        "LLM forecast failed; carrying forward last signal"
                    );
                    // Fall through to carry-forward path.
                    let state = self.symbol_state.get(&sym).expect("just inserted");
                    if let Some(forecast) = &state.last_forecast {
                        let signal = self.rating_to_signal(forecast.rating, bar);
                        vec![signal]
                    } else {
                        // No prior forecast; emit Hold.
                        vec![self.rating_to_signal(Rating::Hold, bar)]
                    }
                }
            }
        } else {
            // Step 2 carry-forward: reuse last forecast.
            let state = self.symbol_state.get(&sym).expect("just inserted");
            if let Some(forecast) = &state.last_forecast {
                let signal = self.rating_to_signal(forecast.rating, bar);
                vec![signal]
            } else {
                // Warmup: no forecast yet. Emit Hold.
                vec![self.rating_to_signal(Rating::Hold, bar)]
            }
        }
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        // The LLM forecaster is bar-driven only; ticks are ignored.
        Vec::new()
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "type": "object",
            "properties": {
                "kind": { "type": "string", "const": "llm_forecaster_v3" },
                "enabled": { "type": "boolean", "default": false },
                "model_id": { "type": "string", "default": super::types::DEFAULT_MODEL_ID },
                "fire_every_n_bars": { "type": "integer", "default": super::types::DEFAULT_FIRE_EVERY_N_BARS },
                "timeout_ms": { "type": "integer", "default": super::types::DEFAULT_TIMEOUT_MS },
                "cost_cap_usd_per_backtest": { "type": "number", "default": 100.0 },
                "cost_cap_usd_per_call": { "type": "number", "default": 0.05 }
            },
            "required": ["kind"]
        })
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Bar, Price, Quantity, SignalKind, Symbol, Timeframe, Timestamp, Venue};

    use super::super::types::{LlmForecasterConfig, Rating, StubForecaster};

    fn make_symbol(s: &str) -> Symbol {
        Symbol::new(s)
    }

    fn make_ts(epoch_s: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
    }

    fn make_bar(symbol: &str, open_ts_s: i64) -> Bar {
        let sym = make_symbol(symbol);
        let ts = make_ts(open_ts_s);
        Bar {
            symbol: sym,
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: make_ts(open_ts_s + 3600),
            open: Price::new(dec!(45000)).expect("positive price"),
            high: Price::new(dec!(45100)).expect("positive price"),
            low: Price::new(dec!(44900)).expect("positive price"),
            close: Price::new(dec!(45050)).expect("positive price"),
            volume: Quantity::new(dec!(1000)).expect("positive qty"),
            trade_count: 100,
            local_recv_ts: ts,
            venue: Venue::Binance,
        }
    }

    fn enabled_config(fire_every: u32) -> LlmForecasterConfig {
        LlmForecasterConfig {
            enabled: true,
            fire_every_n_bars: fire_every,
            ..LlmForecasterConfig::default()
        }
    }

    /// Strategy with `enabled = false` returns empty Vec on every bar.
    #[test]
    fn disabled_strategy_returns_no_signals() {
        let cfg = LlmForecasterConfig::default(); // enabled = false
        let forecaster = Arc::new(StubForecaster::default());
        let mut strat = LlmForecasterStrategy::new(cfg, forecaster, None);
        let bar = make_bar("BTCUSDT", 1_700_000_000);
        let signals = strat.on_bar(&bar);
        assert!(
            signals.is_empty(),
            "disabled strategy must return no signals"
        );
    }

    /// First bar always fires the forecaster (no prior forecast).
    #[test]
    fn first_bar_fires_forecaster() {
        let cfg = enabled_config(24);
        let stub = Arc::new(StubForecaster::with_rating(Rating::Buy));
        let mut strat = LlmForecasterStrategy::new(cfg, stub, None);
        let bar = make_bar("BTCUSDT", 1_700_000_000);
        let signals = strat.on_bar(&bar);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].kind, SignalKind::Buy);
    }

    /// Bars 2..N use carry-forward; bar N+1 fires again.
    #[test]
    fn carry_forward_between_fire_ticks() {
        let fire_every = 3u32;
        let cfg = enabled_config(fire_every);
        let stub = Arc::new(StubForecaster::with_rating(Rating::Sell));
        let mut strat = LlmForecasterStrategy::new(cfg, stub, None);
        let sym = "BTCUSDT";

        // Bar 0 (ts=0): fires → Sell
        let sig0 = strat.on_bar(&make_bar(sym, 0));
        assert_eq!(sig0[0].kind, SignalKind::Sell, "bar 0 should fire");

        // Bars 1, 2: carry-forward → still Sell
        for i in 1..fire_every {
            let sig = strat.on_bar(&make_bar(sym, i as i64 * 3600));
            assert_eq!(
                sig[0].kind,
                SignalKind::Sell,
                "bar {i} should carry forward"
            );
        }

        // Bar 3 (== fire_every): fires again → still Sell (stub is fixed)
        let sig_refired = strat.on_bar(&make_bar(sym, fire_every as i64 * 3600));
        assert_eq!(
            sig_refired[0].kind,
            SignalKind::Sell,
            "bar fire_every should re-fire"
        );

        // Confirm strategy id is correct.
        assert_eq!(strat.id().0.as_str(), STRATEGY_ID);
    }

    /// STRONG_BUY stub → Buy signal; STRONG_SELL stub → Sell signal.
    #[test]
    fn strong_ratings_collapse_to_buy_sell_signals() {
        let sym = "ETHUSDT";
        for (rating, expected_kind) in [
            (Rating::StrongBuy, SignalKind::Buy),
            (Rating::StrongSell, SignalKind::Sell),
        ] {
            let cfg = enabled_config(1);
            let stub = Arc::new(StubForecaster::with_rating(rating));
            let mut strat = LlmForecasterStrategy::new(cfg, stub, None);
            let signals = strat.on_bar(&make_bar(sym, 1_700_000_000));
            assert_eq!(
                signals[0].kind, expected_kind,
                "rating {rating:?} should produce {expected_kind:?}"
            );
        }
    }
}
