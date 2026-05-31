//! Config loader for `CrossSectionalMomentumConfig` (T605 — v1 R7.5).
//!
//! Validates the TOML fields per the Design error-code table and rejects
//! `k_short > 0` with `unsupported_short_sizing` per Q3.

use std::path::Path;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

// ── Strategy family direction (D-MR.0) ────────────────────────────────────────

/// Which direction the cross-sectional strategy selects.
///
/// - `Momentum`: top-K symbols by vol-adjusted return (v1 behavior — the serde default).
/// - `Reversion`: bottom-K symbols (the biggest recent losers) — cross-sectional MR.
///
/// **Naming note (D-MR.1):** `core::forecast::Direction { Up, Down, Flat }` already
/// exists. This `cross_sectional::Direction { Momentum, Reversion }` is a distinct
/// type in the `strategy::cross_sectional` namespace — do NOT unify with the
/// forecast one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Top-K winners (v1 momentum behavior — default).
    #[default]
    Momentum,
    /// Bottom-K losers (cross-sectional mean-reversion).
    Reversion,
}

/// Error codes returned by the loader — matches the Design error-code table.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrossSectionalLoadError {
    #[error("[invalid_universe] universe is empty or missing")]
    InvalidUniverse,
    #[error("[unknown_symbol] universe contains unknown symbol: {0}")]
    UnknownSymbol(SmolStr),
    #[error("[invalid_lookback] lookback_minutes must be >= 1")]
    InvalidLookback,
    #[error("[invalid_rebalance] rebalance_minutes must be >= 1")]
    InvalidRebalance,
    #[error("[invalid_k_long] k_long must be >= 1")]
    InvalidKLong,
    #[error("[unsupported_short_sizing] k_short > 0 is not supported in v1 (spot-only long)")]
    UnsupportedShortSizing,
    #[error("[invalid_exposure_cap] exposure_cap must be in (0, 1]")]
    InvalidExposureCap,
    #[error("[invalid_drift_threshold] drift_rebalance_threshold must be in (0, 1)")]
    InvalidDriftThreshold,
    #[error("[unsupported_sizing] size must be 'equal_weight'")]
    UnsupportedSizing,
    #[error("[unsupported_kind] kind must be 'cross_sectional_momentum'")]
    UnsupportedKind,
    #[error("[toml_parse] TOML parse error: {0}")]
    TomlParse(String),
    #[error("[io_read] could not read file: {0}")]
    IoRead(String),
}

impl CrossSectionalLoadError {
    /// Machine-readable error code for `strategy_events.error_code`.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidUniverse => "invalid_universe",
            Self::UnknownSymbol(_) => "unknown_symbol",
            Self::InvalidLookback => "invalid_lookback",
            Self::InvalidRebalance => "invalid_rebalance",
            Self::InvalidKLong => "invalid_k_long",
            Self::UnsupportedShortSizing => "unsupported_short_sizing",
            Self::InvalidExposureCap => "invalid_exposure_cap",
            Self::InvalidDriftThreshold => "invalid_drift_threshold",
            Self::UnsupportedSizing => "unsupported_sizing",
            Self::UnsupportedKind => "unsupported_kind",
            Self::TomlParse(_) => "toml_parse",
            Self::IoRead(_) => "io_read",
        }
    }
}

/// Validated config for `MomentumStrategy`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSectionalMomentumConfig {
    /// Strategy ID (filename stem).
    pub id: SmolStr,
    /// Universe of symbols (validated non-empty).
    pub universe: Vec<SmolStr>,
    /// Lookback window in bars (≥ 1).
    pub lookback_minutes: u32,
    /// Rebalance cadence in bars (≥ 1).
    pub rebalance_minutes: u32,
    /// Number of longs to hold (≥ 1).
    pub k_long: u32,
    /// Number of shorts — must be 0 in v1.
    pub k_short: u32,
    /// Portfolio exposure cap (0, 1].
    pub exposure_cap: Decimal,
    /// Relative drift threshold for hold case (0, 1).
    pub drift_rebalance_threshold: Decimal,
    /// Vol floor for score denominator (> 0).
    pub vol_floor: Decimal,
    /// Stage: `"research"` or `"paper"`.
    pub stage: SmolStr,
    /// Strategy family direction (D-MR.0).
    /// Default = `Momentum` (v1 behavior) — serde `#[serde(default)]` means
    /// every existing TOML and struct literal that omits this field keeps
    /// the v1 `Momentum` behavior unchanged (no anchor or test breakage).
    #[serde(default)]
    pub direction: Direction,
}

/// Raw deserializable form before validation.
#[derive(Debug, Deserialize)]
struct RawConfig {
    pub id: SmolStr,
    pub kind: SmolStr,
    pub stage: SmolStr,
    pub universe: Vec<SmolStr>,
    #[serde(default = "default_lookback")]
    pub lookback_minutes: u32,
    #[serde(default = "default_rebalance")]
    pub rebalance_minutes: u32,
    #[serde(default = "default_k_long")]
    pub k_long: u32,
    #[serde(default)]
    pub k_short: u32,
    #[serde(default = "default_exposure_cap")]
    pub exposure_cap: Decimal,
    #[serde(default = "default_drift")]
    pub drift_rebalance_threshold: Decimal,
    #[serde(default = "default_vol_floor")]
    pub vol_floor: Decimal,
    #[serde(default = "default_size")]
    pub size: SmolStr,
    /// Strategy family direction — default = `Momentum` so existing TOMLs
    /// without this field keep the v1 behavior unchanged.
    #[serde(default)]
    pub direction: Direction,
}

fn default_lookback() -> u32 {
    60
}
fn default_rebalance() -> u32 {
    60
}
fn default_k_long() -> u32 {
    3
}
fn default_exposure_cap() -> Decimal {
    Decimal::new(50, 2)
}
fn default_drift() -> Decimal {
    Decimal::new(10, 2)
}
fn default_vol_floor() -> Decimal {
    Decimal::new(1, 6)
}
fn default_size() -> SmolStr {
    SmolStr::new("equal_weight")
}

impl CrossSectionalMomentumConfig {
    /// Load and validate a config from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns [`CrossSectionalLoadError`] on any parse or validation failure.
    pub fn from_file(path: &Path) -> Result<Self, CrossSectionalLoadError> {
        let bytes =
            std::fs::read(path).map_err(|e| CrossSectionalLoadError::IoRead(e.to_string()))?;
        let toml_str = std::str::from_utf8(&bytes)
            .map_err(|e| CrossSectionalLoadError::TomlParse(format!("non-UTF8: {e}")))?;
        Self::from_str(toml_str)
    }

    /// Parse and validate TOML content.
    ///
    /// # Errors
    ///
    /// Returns [`CrossSectionalLoadError`] on any parse or validation failure.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(toml_str: &str) -> Result<Self, CrossSectionalLoadError> {
        let raw: RawConfig = toml::from_str(toml_str)
            .map_err(|e| CrossSectionalLoadError::TomlParse(e.to_string()))?;

        // Kind
        if raw.kind.as_str() != "cross_sectional_momentum" {
            return Err(CrossSectionalLoadError::UnsupportedKind);
        }

        // Universe
        if raw.universe.is_empty() {
            return Err(CrossSectionalLoadError::InvalidUniverse);
        }

        // Lookback
        if raw.lookback_minutes < 1 {
            return Err(CrossSectionalLoadError::InvalidLookback);
        }

        // Rebalance
        if raw.rebalance_minutes < 1 {
            return Err(CrossSectionalLoadError::InvalidRebalance);
        }

        // k_long
        if raw.k_long < 1 {
            return Err(CrossSectionalLoadError::InvalidKLong);
        }

        // Q3 — k_short must be 0 in v1
        if raw.k_short > 0 {
            return Err(CrossSectionalLoadError::UnsupportedShortSizing);
        }

        // exposure_cap
        if raw.exposure_cap <= Decimal::ZERO || raw.exposure_cap > Decimal::ONE {
            return Err(CrossSectionalLoadError::InvalidExposureCap);
        }

        // drift_rebalance_threshold
        if raw.drift_rebalance_threshold <= Decimal::ZERO
            || raw.drift_rebalance_threshold >= Decimal::ONE
        {
            return Err(CrossSectionalLoadError::InvalidDriftThreshold);
        }

        // sizing
        if raw.size.as_str() != "equal_weight" {
            return Err(CrossSectionalLoadError::UnsupportedSizing);
        }

        Ok(Self {
            id: raw.id,
            universe: raw.universe,
            lookback_minutes: raw.lookback_minutes,
            rebalance_minutes: raw.rebalance_minutes,
            k_long: raw.k_long,
            k_short: raw.k_short,
            exposure_cap: raw.exposure_cap,
            drift_rebalance_threshold: raw.drift_rebalance_threshold,
            vol_floor: raw.vol_floor,
            stage: raw.stage,
            direction: raw.direction,
        })
    }

    /// JSON schema for `Strategy::config_schema()`.
    pub fn json_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["id", "kind", "stage", "universe"],
            "properties": {
                "id": { "type": "string" },
                "kind": { "type": "string", "const": "cross_sectional_momentum" },
                "stage": { "type": "string", "enum": ["research", "paper"] },
                "universe": { "type": "array", "items": { "type": "string" }, "minItems": 1 },
                "lookback_minutes": { "type": "integer", "minimum": 1, "default": 60 },
                "rebalance_minutes": { "type": "integer", "minimum": 1, "default": 60 },
                "k_long": { "type": "integer", "minimum": 1, "default": 3 },
                "k_short": { "type": "integer", "minimum": 0, "maximum": 0, "default": 0 },
                "exposure_cap": { "type": "number", "exclusiveMinimum": 0, "maximum": 1, "default": 0.5 },
                "drift_rebalance_threshold": { "type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1, "default": 0.1 },
                "vol_floor": { "type": "number", "exclusiveMinimum": 0, "default": 0.000001 },
                "size": { "type": "string", "const": "equal_weight" }
            }
        })
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
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

    #[test]
    fn t605_valid_toml_parses() {
        let cfg = CrossSectionalMomentumConfig::from_str(VALID_TOML).unwrap();
        assert_eq!(cfg.id.as_str(), "top10_momentum_h1");
        assert_eq!(cfg.universe.len(), 10);
        assert_eq!(cfg.k_long, 3);
        assert_eq!(cfg.k_short, 0);
    }

    #[test]
    fn t605_wrong_kind_rejected() {
        let toml = r#"
id = "foo"
kind = "composed"
stage = "research"
universe = ["BTCUSDT"]
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "unsupported_kind");
    }

    #[test]
    fn t605_empty_universe_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = []
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_universe");
    }

    #[test]
    fn t605_k_short_nonzero_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT"]
k_long = 1
k_short = 1
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "unsupported_short_sizing");
    }

    #[test]
    fn t605_invalid_exposure_cap_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT"]
exposure_cap = 1.5
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_exposure_cap");
    }

    #[test]
    fn t605_unsupported_sizing_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT"]
size = "fixed_fraction(0.1)"
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "unsupported_sizing");
    }

    #[test]
    fn t605_invalid_lookback_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT"]
lookback_minutes = 0
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_lookback");
    }

    #[test]
    fn t605_invalid_k_long_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT"]
k_long = 0
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_k_long");
    }

    #[test]
    fn t605_invalid_drift_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT"]
drift_rebalance_threshold = 0.0
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_drift_threshold");
    }

    #[test]
    fn t605_invalid_rebalance_rejected() {
        let toml = r#"
id = "foo"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT"]
rebalance_minutes = 0
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_rebalance");
    }

    #[test]
    fn t605_toml_parse_error() {
        let err = CrossSectionalMomentumConfig::from_str("not valid toml ]][").unwrap_err();
        assert_eq!(err.error_code(), "toml_parse");
    }

    // ── M-DEV-1: Direction field tests ────────────────────────────────────────

    /// M-DEV-1 (a): TOML with no `direction` field → `Direction::Momentum` (backward compat).
    #[test]
    fn mr_dev1_no_direction_defaults_to_momentum() {
        let cfg = CrossSectionalMomentumConfig::from_str(VALID_TOML).unwrap();
        assert_eq!(
            cfg.direction,
            Direction::Momentum,
            "omitting `direction` must default to Momentum (backward compat)"
        );
    }

    /// M-DEV-1 (b): `direction = "reversion"` → `Direction::Reversion`.
    #[test]
    fn mr_dev1_direction_reversion_parses() {
        let toml = r#"
id    = "test_mr"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
direction = "reversion"
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml).unwrap();
        assert_eq!(
            cfg.direction,
            Direction::Reversion,
            "`direction = \"reversion\"` must parse to Direction::Reversion"
        );
    }

    /// M-DEV-1 (c): Config hash differs between Momentum and Reversion at identical θ (K3).
    #[test]
    fn mr_dev1_config_hash_differs_by_direction() {
        use super::super::momentum::MomentumStrategy;
        use smol_str::SmolStr;

        let toml_base = r#"
id    = "test_hash"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = 60
rebalance_minutes = 60
k_long = 2
"#;
        let mut cfg_mom = CrossSectionalMomentumConfig::from_str(toml_base).unwrap();
        let mut cfg_rev = cfg_mom.clone();
        cfg_rev.direction = Direction::Reversion;

        // Make the IDs the same so only direction differs.
        cfg_mom.id = SmolStr::new("test_hash");
        cfg_rev.id = SmolStr::new("test_hash");

        let strat_mom = MomentumStrategy::from_config(cfg_mom, SmolStr::new("test"));
        let strat_rev = MomentumStrategy::from_config(cfg_rev, SmolStr::new("test"));

        assert_ne!(
            strat_mom.hash, strat_rev.hash,
            "Momentum and Reversion configs at identical θ MUST produce different hashes (K3)"
        );
    }
}
