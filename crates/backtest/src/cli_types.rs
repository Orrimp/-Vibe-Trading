//! Shared CLI-scenario types used by `main.rs` and the extracted scenario
//! modules. These types are **not** part of the public `backtest` library API
//! (they live in the binary path), but are shared across `main.rs` and the
//! `scenarios::*` / `report::*` modules via `pub` visibility.
//!
//! Defined here so the scenario modules can take typed inputs without
//! depending on `main.rs`'s internal `Scenario` struct.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cost::SlippageModel;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use trading_core::Symbol;

// ── LatencySlippageSimConfig (v5-latency-slippage-sim R1 / ADR-0043 § D1) ─────

/// Configuration for deterministic latency and slippage simulation in backtest.
///
/// **Default is noop**: `latency_ms_min == latency_ms_max == 0` and
/// `slippage_model == SlippageModel::Linear { bps: 0 }` so no latency or
/// slippage is applied unless the operator explicitly sets non-zero values.
///
/// ## Latency model (Q1 = uniform jitter / ADR-0043 § D2)
///
/// When `latency_ms_min == latency_ms_max == 0`: timestamp unchanged (noop).
/// When `latency_ms_min == latency_ms_max > 0`: fixed delay.
/// When `latency_ms_min < latency_ms_max`: uniform sample from `[min, max]`.
///
/// The RNG is a seeded `ChaCha20` sub-stream keyed on `(scenario_seed, order_id)`
/// via blake3, making jitter deterministic across replay runs (D2).
///
/// ## Slippage model (ADR-0043 § D3 / v0.5.0 extension)
///
/// `SlippageModel::Linear { bps: 0 }`: fill price unchanged (noop).
/// `SlippageModel::Linear { bps > 0 }`: `fill_price = signal_price * (1 ± bps/10_000)`.
/// `SlippageModel::SquareRoot { alpha, volume_lookback_days }`: Almgren-Chriss form
///   `slippage_bps = α · √(Q/V) · 10_000` capped at `MAX_SLIPPAGE_BPS`.
///
/// ## Backward-compat serde (R-NR.2)
///
/// Legacy JSON/TOML payloads that use the old `slippage_bps: u16` field are
/// accepted and deserialized to `SlippageModel::Linear { bps }` so the 71
/// existing anchor SHAs under `v5-realdata-medium-2026-05` stay byte-identical.
///
/// ## Scope (Q4 = backtest-only / ADR-0043 § D5)
///
/// This config is consumed only by `crates/backtest`. The live-mode agent
/// (`crates/agent`) does not read it — live fills already carry real
/// latency and slippage from the venue.
/// Opt-in venue-filter exec-sim mode (ADR-0087, opt-in-forever — mirrors the
/// `SlippageModel::VolScaledSpread` ADR-0081 precedent). An enum (not a bool)
/// so a future mode (e.g. a maker-rebate simulation) stays additive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VenueFilterMode {
    /// Round qty down to the venue's `step_size` + reject sub-`min_notional`
    /// orders, using the checked-in static filter table
    /// (`cost::venue_filter::venue_filter_for`, ADR-0087 § D3).
    LotSizeAndMinNotional,
}

// Note: `Eq` is intentionally NOT derived. `SlippageModel::VolScaledSpread`
// contains `f64` fields (vol_multiplier, sigma_lambda) which do not implement
// `Eq`. Use `PartialEq` comparison or field-wise checks instead.
// ADR-0081 § Consequences records this.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LatencySlippageSimConfig {
    /// Minimum latency added to `order_ts_ms` in milliseconds.
    /// Default: 0 (noop).
    pub latency_ms_min: u64,
    /// Maximum latency added to `order_ts_ms` in milliseconds.
    /// When equal to `latency_ms_min`: fixed delay. Default: 0 (noop).
    pub latency_ms_max: u64,
    /// Slippage model applied to fill prices (v0.5.0 enum replaces legacy `slippage_bps: u32`).
    /// Default: `Linear { bps: 0 }` (noop).
    ///
    /// The legacy field name `slippage_bps` is accepted by the custom `Deserialize` impl
    /// for backward-compat with v0.1.0–v0.4.0 config files/tests.
    pub slippage_model: SlippageModel,
    /// Opt-in venue-filter realism (ADR-0087). `None` (the serde default) =
    /// no lot-size rounding, no min-notional reject — byte-identical to the
    /// pre-ADR-0087 fill path. `Some(VenueFilterMode::LotSizeAndMinNotional)`
    /// rounds fill qty down to the venue `step_size` and skips (no `Fill`)
    /// any order whose rounded notional is below `min_notional`, applied at
    /// the `PaperEngine::step` seam (opt-in-forever).
    #[serde(default)]
    pub venue_filter: Option<VenueFilterMode>,
    /// Pre-computed per-symbol daily volume proxy in USD (V term in α·√(Q/V)).
    ///
    /// Used ONLY by `SlippageModel::SquareRoot`. Populated by the scenario loader
    /// (main.rs) via `data::daily_volume_usd_trailing` before the bar loop; ignored
    /// for `SlippageModel::Linear`. `None` → `Decimal::ZERO` → no-impact (edge case).
    ///
    /// Arc because `LatencySlippageSimConfig` is `Clone`d into multiple scenarios
    /// and the map may be non-trivial. Excluded from serde (runtime-computed).
    #[serde(skip)]
    pub volume_usd_per_symbol: Option<Arc<HashMap<Symbol, Decimal>>>,
}

impl Default for LatencySlippageSimConfig {
    /// Noop default: latency=0, slippage=Linear{bps:0}, `venue_filter=None`.
    /// Produces byte-identical output to the pre-v0.5.0 default config.
    fn default() -> Self {
        Self {
            latency_ms_min: 0,
            latency_ms_max: 0,
            slippage_model: SlippageModel::Linear { bps: 0 },
            venue_filter: None,
            volume_usd_per_symbol: None,
        }
    }
}

impl LatencySlippageSimConfig {
    /// The ADVISOR-path default: identical to [`Self::default`] except that
    /// venue realism is ON (`venue_filter = Some(LotSizeAndMinNotional)`).
    ///
    /// # Why a separate constructor and not a changed `Default`
    ///
    /// PRD §13 Q5 (operator decision 2026-08-04, from the 2026-08-04 product
    /// review's finding 3): the advisor's headline promise is a specific small
    /// budget (€200) and it was being planned WITHOUT lot-size rounding or
    /// min-notional rejection — so the default path could emit a plan whose
    /// legs are unexecutable at the venue the plan names. The advisor path
    /// now opts in by construction.
    ///
    /// `Default` stays `venue_filter: None` deliberately: the anchored CLI
    /// scenarios (`main.rs`) and the frozen research lanes take that default,
    /// and ADR-0087 § D6 makes `None` the byte-identity arm — flipping the
    /// global default would move every anchored report body. The split is the
    /// point: **user-facing plans get realism; frozen evidence keeps its
    /// recorded arithmetic.**
    #[must_use]
    pub fn advisor_default() -> Self {
        Self {
            venue_filter: Some(VenueFilterMode::LotSizeAndMinNotional),
            ..Self::default()
        }
    }
}

impl LatencySlippageSimConfig {
    /// Returns `true` when the config is the noop default (all zeros, linear
    /// bps=0, no venue filter). Used by callers to skip RNG construction on
    /// the hot path.
    #[inline]
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.latency_ms_min == 0
            && self.latency_ms_max == 0
            && matches!(self.slippage_model, SlippageModel::Linear { bps: 0 })
            && self.venue_filter.is_none()
    }
}

/// Custom `Deserialize` for `LatencySlippageSimConfig`.
///
/// Accepts two shapes (R-NR.2 backward-compat):
/// 1. New shape: `{ latency_ms_min, latency_ms_max, slippage_model: { kind: "linear", bps: N } }`
/// 2. Legacy shape: `{ latency_ms_min, latency_ms_max, slippage_bps: N }` →
///    deserialized to `slippage_model: Linear { bps: N }`.
impl<'de> Deserialize<'de> for LatencySlippageSimConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ConfigVisitor;

        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = LatencySlippageSimConfig;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("LatencySlippageSimConfig (new slippage_model or legacy slippage_bps)")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut latency_ms_min: Option<u64> = None;
                let mut latency_ms_max: Option<u64> = None;
                // One of these must be present:
                let mut slippage_model: Option<SlippageModel> = None;
                let mut slippage_bps_legacy: Option<u32> = None;
                // ADR-0087 § D2: `#[serde(default)]` is inert under a custom
                // Deserialize impl — the `None` default is applied manually
                // via this initial binding (never overwritten unless the
                // "venue_filter" key is present).
                let mut venue_filter: Option<VenueFilterMode> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "latency_ms_min" => {
                            latency_ms_min = Some(map.next_value()?);
                        }
                        "latency_ms_max" => {
                            latency_ms_max = Some(map.next_value()?);
                        }
                        "slippage_model" => {
                            slippage_model = Some(map.next_value()?);
                        }
                        // Legacy field name (v0.1.0–v0.4.0): accept as u32 or u16.
                        "slippage_bps" => {
                            slippage_bps_legacy = Some(map.next_value::<u32>()?);
                        }
                        "venue_filter" => {
                            // `Option<VenueFilterMode>::deserialize` handles both
                            // `null` and a present `{ kind: ... }` value.
                            venue_filter = map.next_value()?;
                        }
                        _ => {
                            // Unknown fields: skip.
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                let model = match (slippage_model, slippage_bps_legacy) {
                    (Some(m), _) => m,
                    (None, Some(bps)) => SlippageModel::Linear { bps },
                    (None, None) => SlippageModel::Linear { bps: 0 },
                };

                Ok(LatencySlippageSimConfig {
                    latency_ms_min: latency_ms_min.unwrap_or(0),
                    latency_ms_max: latency_ms_max.unwrap_or(0),
                    slippage_model: model,
                    venue_filter,
                    volume_usd_per_symbol: None,
                })
            }
        }

        deserializer.deserialize_map(ConfigVisitor)
    }
}

// ── T-D-N1 unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
mod latency_slippage_config_tests {
    use super::*;

    /// T-D-N1: Default config is all zeros (noop / anchor-safe).
    #[test]
    fn latency_slippage_sim_config_default_is_noop() {
        let cfg = LatencySlippageSimConfig::default();
        assert_eq!(cfg.latency_ms_min, 0, "default latency_ms_min must be 0");
        assert_eq!(cfg.latency_ms_max, 0, "default latency_ms_max must be 0");
        assert_eq!(
            cfg.slippage_model,
            SlippageModel::Linear { bps: 0 },
            "default slippage_model must be Linear{{bps:0}}"
        );
        assert!(cfg.is_noop(), "default must be noop");
    }

    /// Default config has `PartialEq` with another default.
    #[test]
    fn default_equals_default() {
        let a = LatencySlippageSimConfig::default();
        let b = LatencySlippageSimConfig::default();
        assert_eq!(a, b);
    }

    /// Non-zero config is NOT noop.
    #[test]
    fn non_zero_is_not_noop() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 50,
            latency_ms_max: 100,
            slippage_model: SlippageModel::Linear { bps: 10 },
            venue_filter: None,
            volume_usd_per_symbol: None,
        };
        assert!(!cfg.is_noop(), "non-zero config must not be noop");
    }

    /// Serialization round-trip (new `slippage_model` field).
    #[test]
    fn serde_round_trip() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 20,
            latency_ms_max: 80,
            slippage_model: SlippageModel::Linear { bps: 5 },
            venue_filter: None,
            volume_usd_per_symbol: None,
        };
        let json = serde_json::to_string(&cfg).expect("must serialize");
        let back: LatencySlippageSimConfig = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(cfg, back);
    }

    /// Backward-compat serde: old `slippage_bps: N` field deserializes to Linear{bps:N}.
    #[test]
    fn legacy_slippage_bps_deserializes_to_linear() {
        let legacy_json = r#"{"latency_ms_min":30,"latency_ms_max":80,"slippage_bps":8}"#;
        let cfg: LatencySlippageSimConfig =
            serde_json::from_str(legacy_json).expect("must deserialize legacy");
        assert_eq!(
            cfg.slippage_model,
            SlippageModel::Linear { bps: 8 },
            "legacy slippage_bps:8 must become Linear{{bps:8}}"
        );
        assert_eq!(cfg.latency_ms_min, 30);
        assert_eq!(cfg.latency_ms_max, 80);
    }

    /// Missing slippage field → Linear{bps:0} (noop).
    #[test]
    fn missing_slippage_field_defaults_to_noop() {
        let json = r#"{"latency_ms_min":10,"latency_ms_max":50}"#;
        let cfg: LatencySlippageSimConfig = serde_json::from_str(json).expect("must deserialize");
        assert_eq!(cfg.slippage_model, SlippageModel::Linear { bps: 0 });
    }

    /// `SquareRoot` model round-trips through serde.
    #[test]
    fn sqrt_model_serde_round_trip() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: rust_decimal_macros::dec!(1.0),
                volume_lookback_days: 90,
            },
            venue_filter: None,
            volume_usd_per_symbol: None,
        };
        let json = serde_json::to_string(&cfg).expect("must serialize");
        let back: LatencySlippageSimConfig = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(cfg, back, "SquareRoot model must survive serde round-trip");
    }

    /// ADR-0087 § D6 (`venue_filter_default_is_none` precedent): the config
    /// default carries no venue filter — the mode is opt-in-forever.
    /// PRD §13 Q5: the advisor path opts INTO venue realism; the plain
    /// `Default` (anchored CLI + frozen research lanes) must stay `None`.
    #[test]
    fn advisor_default_enables_venue_filter_while_plain_default_stays_none() {
        let advisor = LatencySlippageSimConfig::advisor_default();
        assert_eq!(
            advisor.venue_filter,
            Some(super::VenueFilterMode::LotSizeAndMinNotional),
            "the advisor path must plan with lot-size + min-notional realism"
        );
        let plain = LatencySlippageSimConfig::default();
        assert!(
            plain.venue_filter.is_none(),
            "the plain default is the ADR-0087 byte-identity arm — anchored \
             bodies depend on it staying None"
        );
        // Everything else must match, so the advisor arm changes exactly one thing.
        assert_eq!(advisor.latency_ms_min, plain.latency_ms_min);
        assert_eq!(advisor.latency_ms_max, plain.latency_ms_max);
        assert_eq!(advisor.slippage_model, plain.slippage_model);
    }

    #[test]
    fn venue_filter_defaults_to_none() {
        assert!(LatencySlippageSimConfig::default().venue_filter.is_none());
        assert!(LatencySlippageSimConfig::default().is_noop());
    }

    /// `venue_filter` round-trips through serde (new field, ADR-0087 § D2).
    #[test]
    fn venue_filter_serde_round_trip() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 0,
            latency_ms_max: 0,
            slippage_model: SlippageModel::Linear { bps: 0 },
            venue_filter: Some(super::VenueFilterMode::LotSizeAndMinNotional),
            volume_usd_per_symbol: None,
        };
        let json = serde_json::to_string(&cfg).expect("must serialize");
        assert!(
            json.contains("lot_size_and_min_notional"),
            "expected snake_case tag in JSON, got: {json}"
        );
        let back: LatencySlippageSimConfig = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(cfg, back);
        assert!(
            !back.is_noop(),
            "a config with venue_filter set is not noop"
        );
    }

    /// Legacy JSON payloads (pre-ADR-0087, no `venue_filter` key) still
    /// deserialize with `venue_filter: None` (R-NR.2 backward-compat).
    #[test]
    fn missing_venue_filter_field_defaults_to_none() {
        let json = r#"{"latency_ms_min":10,"latency_ms_max":50,"slippage_bps":8}"#;
        let cfg: LatencySlippageSimConfig = serde_json::from_str(json).expect("must deserialize");
        assert!(cfg.venue_filter.is_none());
    }

    // ── T-D-N4 plumbing tests: default-is-noop for each new struct ────────────

    /// T-D-N4a: `PairsScenarioInput.latency_slippage_sim` defaults to noop.
    #[test]
    fn pairs_scenario_input_default_sim_is_noop() {
        let input = super::PairsScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "pairs_mr_h1".to_string(),
            latency_slippage_sim: LatencySlippageSimConfig::default(),
        };
        assert!(
            input.latency_slippage_sim.is_noop(),
            "PairsScenarioInput: default sim must be noop"
        );
    }

    /// T-D-N4b: `TcnScenarioInput.latency_slippage_sim` defaults to noop.
    #[test]
    fn tcn_scenario_input_default_sim_is_noop() {
        let input = super::TcnScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: LatencySlippageSimConfig::default(),
            funding_override: None,
            basis_override: None,
        };
        assert!(
            input.latency_slippage_sim.is_noop(),
            "TcnScenarioInput: default sim must be noop"
        );
    }

    /// T-D-N4c: `SmaComposedRunInput.latency_slippage_sim` defaults to noop.
    #[test]
    fn sma_composed_run_input_default_sim_is_noop() {
        use trading_core::Symbol;
        let input = super::SmaComposedRunInput {
            strategy_id: "sma_crossover".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: LatencySlippageSimConfig::default(),
            short_enabled: false,
            composed_toml_override: None,
        };
        assert!(
            input.latency_slippage_sim.is_noop(),
            "SmaComposedRunInput: default sim must be noop"
        );
    }

    /// T-D-N4d: non-zero config flows through `PairsScenarioInput`.
    #[test]
    fn pairs_scenario_input_non_zero_sim_flows_through() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::Linear { bps: 8 },
            venue_filter: None,
            volume_usd_per_symbol: None,
        };
        let input = super::PairsScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "pairs_mr_h1".to_string(),
            latency_slippage_sim: cfg.clone(),
        };
        assert!(!input.latency_slippage_sim.is_noop());
        assert_eq!(
            input.latency_slippage_sim.slippage_model,
            SlippageModel::Linear { bps: 8 }
        );
    }

    /// T-D-N4e: non-zero config flows through `TcnScenarioInput`.
    #[test]
    fn tcn_scenario_input_non_zero_sim_flows_through() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::Linear { bps: 8 },
            venue_filter: None,
            volume_usd_per_symbol: None,
        };
        let input = super::TcnScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: cfg.clone(),
            funding_override: None,
            basis_override: None,
        };
        assert!(!input.latency_slippage_sim.is_noop());
        assert_eq!(
            input.latency_slippage_sim.slippage_model,
            SlippageModel::Linear { bps: 8 }
        );
    }

    /// T-D-N4f: non-zero config flows through `SmaComposedRunInput`.
    #[test]
    fn sma_composed_run_input_non_zero_sim_flows_through() {
        use trading_core::Symbol;
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::Linear { bps: 8 },
            venue_filter: None,
            volume_usd_per_symbol: None,
        };
        let input = super::SmaComposedRunInput {
            strategy_id: "sma_crossover".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: cfg.clone(),
            short_enabled: false,
            composed_toml_override: None,
        };
        assert!(!input.latency_slippage_sim.is_noop());
        assert_eq!(
            input.latency_slippage_sim.slippage_model,
            SlippageModel::Linear { bps: 8 }
        );
    }
}

// ── SMA / Composed scenario input ─────────────────────────────────────────────

/// Inputs for the SMA crossover + Composed scenario backtest and report writer.
///
/// Extracted from `main.rs::Scenario` and the surrounding `main()` context.
/// Used by `scenarios::sma_composed::run` and `report::sma::write`.
#[derive(Debug, Clone)]
pub struct SmaScenarioInput {
    /// Canonical scenario name (written into the YAML front-matter `scenario:` field).
    pub scenario_name: String,
    /// Canonical name written into the **report body** (may differ for alias scenarios
    /// like `btc-2023-1m-sma-baseline-refresh` which use `btc-2023-1m-sma-cross`).
    pub body_name: String,
    /// Override for the `Wall-clock time` row in the body.
    /// `Some(0.2)` for v0-anchor scenarios to preserve the locked SHA-256.
    /// `None` means use the actual elapsed time.
    pub body_elapsed_override: Option<f64>,
    pub symbol: Symbol,
    pub start_year: i32,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Path to the SMA-baseline report (for comparative scenarios).
    pub baseline_report: Option<String>,
}

// ── Momentum scenario input ────────────────────────────────────────────────────

/// Inputs for the v1 cross-sectional momentum backtest and report writer.
#[derive(Debug, Clone)]
pub struct MomentumScenarioInput {
    /// CLI scenario name (written into the YAML `scenario:` field).
    pub scenario_name: String,
    pub start_year: i32,
    pub bar_count: usize,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Strategy config ID, e.g. `"top10_momentum_h1"`.
    pub config_id: String,
    /// Pre-loaded real bars (`RealData` path). `None` → generate synthetic bars.
    ///
    /// Added for `v3.0.0-volatility-rebaseline` (`top10-2023-fy-momentum-realdata`)
    /// to support real Binance data as the un-targeted v1 momentum baseline.
    pub bars_override: Option<Vec<trading_core::Bar>>,
    /// Dataset revision SHA (from `data/binance/REVISION.toml`) for the
    /// `data_revision_sha:` frontmatter field. `None` for synthetic scenarios.
    pub data_revision_sha: Option<String>,
    /// v5-latency-slippage-sim R1 / ADR-0043 § D1 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    ///
    /// **Anchor contract**: CLI paths that construct `MomentumScenarioInput`
    /// without this field use `..Default::default()` or `LatencySlippageSimConfig::default()`
    /// explicitly, ensuring byte-identical output for all 34 anchored scenarios.
    pub latency_slippage_sim: LatencySlippageSimConfig,
}

// ── Pairs scenario input ───────────────────────────────────────────────────────

/// Inputs for the v1.5a mean-reversion pairs backtest and report writer.
#[derive(Debug, Clone)]
pub struct PairsScenarioInput {
    /// CLI scenario name (written into the YAML `scenario:` field).
    pub scenario_name: String,
    pub start_year: i32,
    pub bar_count: usize,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Strategy config ID, e.g. `"pairs_mr_h1"`.
    pub config_id: String,
    /// v5-latency-slippage-sim R1 / ADR-0047 D2 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    pub latency_slippage_sim: LatencySlippageSimConfig,
}

// ── TCN overlay scenario input ─────────────────────────────────────────────────

/// Inputs for the v2.5 TCN overlay momentum backtest and report writer.
/// Shared by both the passthrough and real-weights variants.
/// Also used by `patchtst_overlay_weights`, `garch_vol_target_overlay`, and
/// `threshold_sweep` (all share this struct — field added once, auto-propagates).
#[derive(Debug, Clone)]
pub struct TcnScenarioInput {
    /// CLI scenario name (written into the YAML `scenario:` field).
    pub scenario_name: String,
    pub start_year: i32,
    pub bar_count: usize,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Strategy config ID, e.g. `"tcn_overlay_momentum"`.
    pub config_id: String,
    /// Forecaster ID, e.g. `"passthrough"` or `"tcn-bs1"`.
    pub forecaster_id: String,
    /// Pre-loaded real bars (`RealData` path). `None` → generate synthetic bars.
    pub bars_override: Option<Vec<trading_core::Bar>>,
    /// Optional output path for the equity-curve text file (`--emit-equity-bin`).
    pub emit_equity_bin: Option<PathBuf>,
    /// v5-latency-slippage-sim R1 / ADR-0047 D2 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    ///
    /// NOTE: `threshold_sweep::run_cell` consumes this field but the sim is
    /// structurally noop for analysis sweeps (no equity surface). Deferred per
    /// ADR-0047 D2.
    pub latency_slippage_sim: LatencySlippageSimConfig,
    /// Carry-strategy funding lookup (ADR-0051 § D6.6, M-DEV-4, Stage 2).
    ///
    /// When `Some`, maps `(Symbol, Timestamp)` → the co-resampled funding rate
    /// for that bar. Built from `GeneratedPath.funding_by_symbol` + the
    /// synthetic `open_ts` of each bar in `bars_by_symbol[s][k]`.
    ///
    /// `None` for every momentum/MR/buy-and-hold run → `run_path` behaviour is
    /// byte-identical to the pre-carry code. The accrual block is never entered;
    /// the 87 existing anchors are byte-unchanged.
    ///
    /// At Stage 2 (this commit): `run_path` RECEIVES the field but does NOT yet
    /// use it for signal/cashflow — that is Stage 3 (M-DEV-5 + M-DEV-4 signal).
    /// Threading it now keeps the seam additive and anchor-neutral.
    pub funding_override:
        Option<std::collections::BTreeMap<(Symbol, trading_core::Timestamp), Decimal>>,
    /// MN-spread basis lookup (ADR-0051 § D6.10, M-DEV-1).
    ///
    /// When `Some`, maps `(Symbol, Timestamp)` → the co-resampled basis value
    /// for that bar. Built from `GeneratedPath.basis_by_symbol` + synthetic
    /// `open_ts`. Used by the MN-spread arm for basis-score injection AND as
    /// the second sidecar alongside `funding_override`.
    ///
    /// `None` for every non-MN run → byte-identical to pre-M-DEV-1 code.
    pub basis_override:
        Option<std::collections::BTreeMap<(Symbol, trading_core::Timestamp), Decimal>>,
}

// ── BacktestState (SMA / Composed run state) ──────────────────────────────────

/// Single-symbol backtest accumulator state.
/// Mirrors `main.rs::BacktestState` @2380.
/// Lives here so `report::sma::write` can reference it.
pub struct BacktestState {
    pub cash: rust_decimal::Decimal,
    pub position_qty: rust_decimal::Decimal,
    pub position_cost: rust_decimal::Decimal,
    pub trades: usize,
    pub buys: usize,
    pub sells: usize,
    pub total_fees: rust_decimal::Decimal,
    pub peak_equity: rust_decimal::Decimal,
    pub max_drawdown: rust_decimal::Decimal,
    pub ledger_imbalance_events: usize,
    pub equity_curve: Vec<rust_decimal::Decimal>,
}

impl BacktestState {
    #[must_use]
    pub fn new(initial_capital: rust_decimal::Decimal) -> Self {
        Self {
            cash: initial_capital,
            position_qty: rust_decimal::Decimal::ZERO,
            position_cost: rust_decimal::Decimal::ZERO,
            trades: 0,
            buys: 0,
            sells: 0,
            total_fees: rust_decimal::Decimal::ZERO,
            peak_equity: initial_capital,
            max_drawdown: rust_decimal::Decimal::ZERO,
            ledger_imbalance_events: 0,
            equity_curve: vec![initial_capital],
        }
    }

    #[must_use]
    pub fn equity(&self, mark: rust_decimal::Decimal) -> rust_decimal::Decimal {
        self.cash + self.position_qty * mark
    }

    pub fn update_drawdown(&mut self, equity: rust_decimal::Decimal) {
        if equity > self.peak_equity {
            self.peak_equity = equity;
        }
        if self.peak_equity > rust_decimal::Decimal::ZERO {
            let dd = (self.peak_equity - equity) / self.peak_equity;
            if dd > self.max_drawdown {
                self.max_drawdown = dd;
            }
        }
    }

    pub fn apply_buy(
        &mut self,
        qty: rust_decimal::Decimal,
        fill_price: rust_decimal::Decimal,
        fee: rust_decimal::Decimal,
    ) {
        let notional = qty * fill_price;
        self.cash -= notional + fee;
        self.position_qty += qty;
        self.position_cost += notional;
        self.total_fees += fee;
        self.trades += 1;
        self.buys += 1;
    }

    /// Apply a sell fill to the state.
    ///
    /// When `short_enabled = false` (the long-only default), the position is
    /// clamped to zero — byte-identical to HEAD's code.
    ///
    /// When `short_enabled = true` (ADR-0068 D1/D3), the position is allowed to
    /// go negative (a short position). The `position_cost` tracking carries the
    /// signed open-proceeds basis rather than zeroing it.
    pub fn apply_sell(
        &mut self,
        qty: rust_decimal::Decimal,
        fill_price: rust_decimal::Decimal,
        fee: rust_decimal::Decimal,
        short_enabled: bool,
    ) {
        let notional = qty * fill_price;
        self.cash += notional - fee;
        self.position_qty -= qty;
        if !short_enabled && self.position_qty < rust_decimal::Decimal::ZERO {
            // Long-only clamp #3 (ADR-0068 D1 site list):
            // `cli_types.rs:632-635` `apply_sell` clamp-to-zero.
            // GATED: active only when short_enabled=false → byte-identical to HEAD.
            self.position_qty = rust_decimal::Decimal::ZERO;
            self.position_cost = rust_decimal::Decimal::ZERO;
        }
        self.total_fees += fee;
        self.trades += 1;
        self.sells += 1;
    }
}

// ── SmaComposedRun scenario input (Wave D-2 / T-AR-4) ────────────────────────

/// Input for the single-symbol SMA/Composed strategy bar-loop extracted into
/// `scenarios::sma_composed_run`. Carries the strategy id and the Scenario
/// parameters needed by both the engine dispatch path and the CLI call path.
///
/// `strategy_id` selects which composed TOML is loaded
/// (`config/strategies/<strategy_id>.toml`) — or `"sma_crossover"` for the
/// compiled-in crossover.
#[derive(Debug, Clone)]
pub struct SmaComposedRunInput {
    /// Strategy id, e.g. `"sma_crossover"`, `"btc_macd_trend"`,
    /// `"btc_rsi_reversion"`, `"btc_bbands_mean_revert"`.
    pub strategy_id: String,
    /// Symbol to trade, e.g. `Symbol::new("BTCUSDT")`.
    pub symbol: trading_core::Symbol,
    /// Calendar year for synthetic-bar epoch base (2023 or 2024).
    pub start_year: i32,
    /// Total bars to replay (e.g. `525_600` for a full 2023 minute run).
    pub bar_count: usize,
    /// Starting portfolio capital in USDT.
    pub initial_capital: rust_decimal::Decimal,
    /// Matching engine slippage in basis points.
    pub slippage_bps: u32,
    /// Matching engine taker fee in basis points.
    pub taker_fee_bps: u32,

    // ── lab-polish-round-2 R2 — SMA param overrides ──────────────────────
    //
    // ANCHOR-PRESERVING CONTRACT: when both fields are None the run uses
    // the legacy (20, 50) hardcoded defaults so anchored CLI scenarios stay
    // byte-identical. The Lab UI sets these to user-chosen values to enable
    // in-cockpit A/B testing without editing TOML.
    //
    /// Optional override for the fast SMA window length (default 20).
    /// Only honoured when `strategy_id == "sma_crossover"`. `None` → 20.
    pub sma_fast_len: Option<usize>,
    /// Optional override for the slow SMA window length (default 50).
    /// Only honoured when `strategy_id == "sma_crossover"`. `None` → 50.
    pub sma_slow_len: Option<usize>,
    /// v5-latency-slippage-sim R1 / ADR-0047 D2 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    pub latency_slippage_sim: LatencySlippageSimConfig,

    /// ADR-0068 D1/D2 — enable the single-coin directional short-selling path.
    ///
    /// `false` (default) → long-only clamps active; path is byte-identical to HEAD.
    /// `true` → gates off the four long-only clamps; `Sell`-when-flat-or-short
    /// opens/extends a short via `backtest::short_exec`; `Buy`-when-short covers.
    /// Set `true` ONLY for the new `_ls` / `always_short` arms (ADR-0068 D9).
    pub short_enabled: bool,

    // ── ADR-0069 T7 — in-memory composed TOML override ───────────────────
    //
    // When `Some(toml_str)`, the strategy is loaded from this string via
    // `ComposedStrategyConfig::from_str` instead of from disk. The `strategy_id`
    // field still identifies the strategy id/stem used for parsing.
    //
    // Only populated by `backtest::bakeoff::sweep::build_swept_config` for
    // the MACD / RSI / Bollinger sweep families. All anchored CLI paths set
    // this to `None` → byte-identical behaviour preserved.
    //
    // ANCHOR-PRESERVING CONTRACT: the `None` branch is the only code path
    // that runs during anchored scenarios. `Some(...)` is sweep-only and
    // `write_report = false` for every sweep cell (ADR-0069 D9).
    pub composed_toml_override: Option<String>,
}

// ── Strategy metadata (SMA / Composed report) ─────────────────────────────────

/// Strategy metadata for the SMA/Composed report.
/// Mirrors `main.rs::StrategyMeta` @538 plus `strategy_notes` from `write_report` @2538.
#[derive(Debug, Clone)]
pub struct StrategyMeta {
    pub id: String,
    pub kind: String,
    pub hash_hex: String,
    pub source_path: String,
    pub signal: String,
    /// Fragment for the `## Notes` section, e.g. "v0 SMA crossover: fast=20, slow=50".
    /// Mirrors `main.rs::write_report::strategy_notes` @2538.
    pub notes: String,
}
