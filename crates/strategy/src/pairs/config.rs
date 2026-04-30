//! Config loader for `MeanReversionPairsStrategy` (T705 — v1.5a R7.6).
//!
//! Validates the TOML fields per the Design error-code table and rejects
//! USDC pairs with `unsupported_quote` per Q5.

use std::collections::BTreeSet;
use std::path::Path;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use trading_core::{Pair, PairError, PairKey, Symbol};

/// Error codes returned by the pairs config loader.
///
/// Matches the Design error-code table in the v1.5a spec.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PairsLoadError {
    #[error("[invalid_pairs] {0}")]
    InvalidPairs(String),
    #[error("[unknown_symbol] pair contains unknown symbol: {0}")]
    UnknownSymbol(String),
    #[error("[invalid_beta] {0}")]
    InvalidBeta(String),
    #[error("[unsupported_quote] USDC pairs require v1.5b multi-venue ingest")]
    UnsupportedQuote,
    #[error("[invalid_lookback] lookback_minutes must be >= 2, got {0}")]
    InvalidLookback(u32),
    #[error("[invalid_z_thresholds] {0}")]
    InvalidZThresholds(String),
    #[error("[invalid_exposure_cap] exposure_cap_per_pair must be in (0, 1], got {0}")]
    InvalidExposureCap(Decimal),
    #[error("[invalid_staleness] max_staleness_minutes must be >= 1, got {0}")]
    InvalidStaleness(u32),
    #[error("[unsupported_sizing] size must be 'binary_per_pair', got '{0}'")]
    UnsupportedSizing(String),
    #[error("[unsupported_kind] kind must be 'mean_reversion_pairs', got '{0}'")]
    UnsupportedKind(String),
    #[error("[toml_parse] TOML parse error: {0}")]
    TomlParse(String),
    #[error("[io_read] could not read file: {0}")]
    IoRead(String),
}

impl PairsLoadError {
    /// Machine-readable error code for `strategy_events.error_code`.
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidPairs(_) => "invalid_pairs",
            Self::UnknownSymbol(_) => "unknown_symbol",
            Self::InvalidBeta(_) => "invalid_beta",
            Self::UnsupportedQuote => "unsupported_quote",
            Self::InvalidLookback(_) => "invalid_lookback",
            Self::InvalidZThresholds(_) => "invalid_z_thresholds",
            Self::InvalidExposureCap(_) => "invalid_exposure_cap",
            Self::InvalidStaleness(_) => "invalid_staleness",
            Self::UnsupportedSizing(_) => "unsupported_sizing",
            Self::UnsupportedKind(_) => "unsupported_kind",
            Self::TomlParse(_) => "toml_parse",
            Self::IoRead(_) => "io_read",
        }
    }
}

// ── Validated config ──────────────────────────────────────────────────────────

/// Validated configuration for `MeanReversionPairsStrategy`.
///
/// All fields are pre-validated at load time — constructing this struct via
/// [`MeanReversionPairsConfig::from_str`] or [`MeanReversionPairsConfig::from_file`]
/// guarantees all invariants hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeanReversionPairsConfig {
    /// Strategy ID (filename stem, e.g. `"pairs_mr_h1"`).
    pub id: SmolStr,
    /// Strategy lifecycle stage (`"research"` or `"paper"`).
    pub stage: SmolStr,
    /// 1–16 pairs, each with a validated `beta`.
    pub pairs: Vec<Pair>,
    /// Lookback window in bars (≥ 2).
    pub lookback_minutes: u32,
    /// Cooldown after close, in minutes (≥ 0).
    pub cooldown_minutes: u32,
    /// Entry z-score threshold (> z_exit).
    pub z_entry: Decimal,
    /// Exit z-score threshold (> 0, |z| <= z_exit triggers close).
    pub z_exit: Decimal,
    /// Hard-stop z-score threshold (> z_entry).
    pub z_stop: Decimal,
    /// Vol floor for z-score denominator (prevents divide-by-zero).
    pub vol_floor: Decimal,
    /// Per-pair fraction of equity to allocate on entry (0, 1].
    pub exposure_cap_per_pair: Decimal,
    /// Maximum staleness of a cached leg before it is dropped (Q10, ≥ 1).
    pub max_staleness_minutes: u32,
}

impl MeanReversionPairsConfig {
    /// Parse and validate from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`PairsLoadError`] if any validation rule is violated.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, PairsLoadError> {
        let raw: RawConfig =
            toml::from_str(s).map_err(|e| PairsLoadError::TomlParse(e.to_string()))?;
        validate(raw)
    }

    /// Parse and validate from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns [`PairsLoadError`] on I/O error or any validation rule violation.
    pub fn from_file(path: &Path) -> Result<Self, PairsLoadError> {
        let s = std::fs::read_to_string(path).map_err(|e| PairsLoadError::IoRead(e.to_string()))?;
        Self::from_str(&s)
    }

    /// JSON schema stub (for `Strategy::config_schema`).
    ///
    /// Returns a minimal JSON object describing the config shape.
    #[must_use]
    pub fn json_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "kind": { "type": "string", "const": "mean_reversion_pairs" },
                "pairs": { "type": "array" },
                "lookback_minutes": { "type": "integer" },
                "cooldown_minutes": { "type": "integer" },
                "z_entry": { "type": "string" },
                "z_exit": { "type": "string" },
                "z_stop": { "type": "string" },
                "vol_floor": { "type": "string" },
                "exposure_cap_per_pair": { "type": "string" },
                "max_staleness_minutes": { "type": "integer" },
                "size": { "type": "string", "const": "binary_per_pair" }
            },
            "required": ["id", "kind", "pairs", "lookback_minutes", "cooldown_minutes",
                         "z_entry", "z_exit", "z_stop", "exposure_cap_per_pair", "size"]
        })
    }
}

// ── Raw deserializable form ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawConfig {
    pub id: SmolStr,
    pub kind: SmolStr,
    #[allow(dead_code)]
    pub stage: Option<SmolStr>,
    pub pairs: Vec<RawPairConfig>,
    pub lookback_minutes: u32,
    #[serde(default)]
    pub cooldown_minutes: u32,
    pub z_entry: SmolStr,
    pub z_exit: SmolStr,
    pub z_stop: SmolStr,
    #[serde(default = "default_vol_floor_str")]
    pub vol_floor: SmolStr,
    pub size: SmolStr,
    pub exposure_cap_per_pair: SmolStr,
    #[serde(default = "default_max_staleness")]
    pub max_staleness_minutes: u32,
}

#[derive(Debug, Deserialize)]
struct RawPairConfig {
    pub a: SmolStr,
    pub b: SmolStr,
    #[serde(default = "default_beta_str")]
    pub beta: SmolStr,
}

fn default_vol_floor_str() -> SmolStr {
    SmolStr::new("0.000001")
}

fn default_beta_str() -> SmolStr {
    SmolStr::new("1.0")
}

fn default_max_staleness() -> u32 {
    5
}

// ── Validation ─────────────────────────────────────────────────────────────────

fn validate(raw: RawConfig) -> Result<MeanReversionPairsConfig, PairsLoadError> {
    // kind discriminator
    if raw.kind.as_str() != "mean_reversion_pairs" {
        return Err(PairsLoadError::UnsupportedKind(raw.kind.to_string()));
    }

    // size discriminator
    if raw.size.as_str() != "binary_per_pair" {
        return Err(PairsLoadError::UnsupportedSizing(raw.size.to_string()));
    }

    // pairs count
    if raw.pairs.is_empty() {
        return Err(PairsLoadError::InvalidPairs(
            "at least 1 pair required".to_string(),
        ));
    }
    if raw.pairs.len() > 16 {
        return Err(PairsLoadError::InvalidPairs(format!(
            "max 16 pairs, got {}",
            raw.pairs.len()
        )));
    }

    // Validate pairs and check for USDC, duplicates, degeneracy, invalid beta
    let mut seen_keys: BTreeSet<PairKey> = BTreeSet::new();
    let mut validated_pairs = Vec::with_capacity(raw.pairs.len());
    for rp in &raw.pairs {
        // USDC check (Q5)
        if rp.a.as_str().ends_with("USDC") || rp.b.as_str().ends_with("USDC") {
            return Err(PairsLoadError::UnsupportedQuote);
        }

        let sym_a = Symbol::new(rp.a.as_str());
        let sym_b = Symbol::new(rp.b.as_str());

        let beta: Decimal = rp
            .beta
            .parse()
            .map_err(|_| PairsLoadError::InvalidBeta(format!("cannot parse: {}", rp.beta)))?;

        let pair = Pair::new(sym_a, sym_b, beta).map_err(|e| match e {
            PairError::DegeneratePair => PairsLoadError::InvalidPairs("a == b".to_string()),
            PairError::InvalidBeta { beta } => {
                PairsLoadError::InvalidBeta(format!("beta={beta} must be > 0"))
            }
            PairError::BetaOutOfRange { beta } => {
                PairsLoadError::InvalidBeta(format!("beta={beta} out of range [0.1, 10]"))
            }
            PairError::UnsupportedQuote => PairsLoadError::UnsupportedQuote,
        })?;

        if !seen_keys.insert(pair.key.clone()) {
            return Err(PairsLoadError::InvalidPairs(format!(
                "duplicate pair ({}, {})",
                pair.key.a, pair.key.b
            )));
        }

        validated_pairs.push(pair);
    }

    // lookback_minutes >= 2
    if raw.lookback_minutes < 2 {
        return Err(PairsLoadError::InvalidLookback(raw.lookback_minutes));
    }

    // Decimal z thresholds
    let z_entry: Decimal = raw
        .z_entry
        .parse()
        .map_err(|_| PairsLoadError::InvalidZThresholds("z_entry not a decimal".to_string()))?;
    let z_exit: Decimal = raw
        .z_exit
        .parse()
        .map_err(|_| PairsLoadError::InvalidZThresholds("z_exit not a decimal".to_string()))?;
    let z_stop: Decimal = raw
        .z_stop
        .parse()
        .map_err(|_| PairsLoadError::InvalidZThresholds("z_stop not a decimal".to_string()))?;

    // z_exit > 0
    if z_exit <= Decimal::ZERO {
        return Err(PairsLoadError::InvalidZThresholds(format!(
            "z_exit={z_exit} must be > 0"
        )));
    }
    // z_entry > z_exit
    if z_entry <= z_exit {
        return Err(PairsLoadError::InvalidZThresholds(format!(
            "z_entry={z_entry} must be > z_exit={z_exit}"
        )));
    }
    // z_stop > z_entry
    if z_stop <= z_entry {
        return Err(PairsLoadError::InvalidZThresholds(format!(
            "z_stop={z_stop} must be > z_entry={z_entry}"
        )));
    }

    // vol_floor
    let vol_floor: Decimal = raw
        .vol_floor
        .parse()
        .map_err(|_| PairsLoadError::InvalidZThresholds("vol_floor not a decimal".to_string()))?;

    // exposure_cap_per_pair
    let exposure_cap_per_pair: Decimal = raw.exposure_cap_per_pair.parse().map_err(|_| {
        PairsLoadError::InvalidExposureCap(dec!(0)) // placeholder; real parse error
    })?;
    if exposure_cap_per_pair <= Decimal::ZERO || exposure_cap_per_pair > Decimal::ONE {
        return Err(PairsLoadError::InvalidExposureCap(exposure_cap_per_pair));
    }

    // max_staleness_minutes >= 1
    if raw.max_staleness_minutes < 1 {
        return Err(PairsLoadError::InvalidStaleness(raw.max_staleness_minutes));
    }

    Ok(MeanReversionPairsConfig {
        id: raw.id,
        stage: raw.stage.unwrap_or_else(|| SmolStr::new("research")),
        pairs: validated_pairs,
        lookback_minutes: raw.lookback_minutes,
        cooldown_minutes: raw.cooldown_minutes,
        z_entry,
        z_exit,
        z_stop,
        vol_floor,
        exposure_cap_per_pair,
        max_staleness_minutes: raw.max_staleness_minutes,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn canonical_toml() -> &'static str {
        r#"
id = "pairs_mr_h1"
kind = "mean_reversion_pairs"
stage = "research"

pairs = [
    { a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },
    { a = "ETHUSDT", b = "SOLUSDT", beta = "1.0" },
    { a = "BNBUSDT", b = "BTCUSDT", beta = "1.0" },
]

lookback_minutes      = 60
cooldown_minutes      = 60
z_entry               = "2.0"
z_exit                = "0.5"
z_stop                = "4.0"
vol_floor             = "0.000001"
size                  = "binary_per_pair"
exposure_cap_per_pair = "0.25"
max_staleness_minutes = 5
"#
    }

    #[test]
    fn t705_canonical_toml_parses() {
        let cfg = MeanReversionPairsConfig::from_str(canonical_toml()).unwrap();
        assert_eq!(cfg.id.as_str(), "pairs_mr_h1");
        assert_eq!(cfg.pairs.len(), 3);
        assert_eq!(cfg.lookback_minutes, 60);
        assert_eq!(cfg.cooldown_minutes, 60);
    }

    #[test]
    fn t705_wrong_kind() {
        let toml = canonical_toml().replace("mean_reversion_pairs", "cross_sectional_momentum");
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "unsupported_kind");
    }

    #[test]
    fn t705_wrong_size() {
        let toml = canonical_toml().replace("binary_per_pair", "equal_weight");
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "unsupported_sizing");
    }

    #[test]
    fn t705_empty_pairs() {
        let toml = canonical_toml().replace(
            r#"pairs = [
    { a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },
    { a = "ETHUSDT", b = "SOLUSDT", beta = "1.0" },
    { a = "BNBUSDT", b = "BTCUSDT", beta = "1.0" },
]"#,
            "pairs = []",
        );
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_pairs");
    }

    #[test]
    fn t705_degenerate_pair() {
        let toml = canonical_toml().replace(
            r#"{ a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },"#,
            r#"{ a = "BTCUSDT", b = "BTCUSDT", beta = "1.0" },"#,
        );
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_pairs");
    }

    #[test]
    fn t705_duplicate_pair() {
        let toml = canonical_toml().replace(
            r#"{ a = "ETHUSDT", b = "SOLUSDT", beta = "1.0" },"#,
            r#"{ a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },"#,
        );
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_pairs");
    }

    #[test]
    fn t705_usdc_pair_rejected() {
        let toml = canonical_toml().replace(
            r#"{ a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },"#,
            r#"{ a = "BTCUSDC", b = "ETHUSDC", beta = "1.0" },"#,
        );
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "unsupported_quote");
    }

    #[test]
    fn t705_invalid_beta_zero() {
        let toml = canonical_toml().replace(
            r#"{ a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },"#,
            r#"{ a = "BTCUSDT", b = "ETHUSDT", beta = "0.0" },"#,
        );
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_beta");
    }

    #[test]
    fn t705_invalid_beta_out_of_range() {
        let toml = canonical_toml().replace(
            r#"{ a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },"#,
            r#"{ a = "BTCUSDT", b = "ETHUSDT", beta = "11.0" },"#,
        );
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_beta");
    }

    #[test]
    fn t705_invalid_lookback() {
        let toml = canonical_toml().replace("lookback_minutes      = 60", "lookback_minutes = 1");
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_lookback");
    }

    #[test]
    fn t705_invalid_z_thresholds_entry_not_gt_exit() {
        let toml =
            canonical_toml().replace(r#"z_entry               = "2.0""#, r#"z_entry = "0.3""#);
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_z_thresholds");
    }

    #[test]
    fn t705_invalid_z_thresholds_stop_not_gt_entry() {
        let toml =
            canonical_toml().replace(r#"z_stop                = "4.0""#, r#"z_stop = "1.5""#);
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_z_thresholds");
    }

    #[test]
    fn t705_invalid_z_exit_zero() {
        let toml =
            canonical_toml().replace(r#"z_exit                = "0.5""#, r#"z_exit = "0.0""#);
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_z_thresholds");
    }

    #[test]
    fn t705_invalid_exposure_cap() {
        let toml = canonical_toml().replace(
            r#"exposure_cap_per_pair = "0.25""#,
            r#"exposure_cap_per_pair = "1.5""#,
        );
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_exposure_cap");
    }

    #[test]
    fn t705_invalid_staleness() {
        let toml =
            canonical_toml().replace("max_staleness_minutes = 5", "max_staleness_minutes = 0");
        let err = MeanReversionPairsConfig::from_str(&toml).unwrap_err();
        assert_eq!(err.error_code(), "invalid_staleness");
    }
}
