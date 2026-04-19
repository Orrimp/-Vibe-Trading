//! `ComposedStrategyConfig` — serde struct + loader (T506).
//!
//! Deserializes one `config/strategies/<id>.toml` file and validates:
//! - `id` matches filename stem.
//! - `stage` is `research` or `paper`.
//! - `signal` is non-empty.
//! - `size` is `fixed_fraction(<f>)`.

use std::collections::BTreeMap;
use std::path::Path;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::error::StrategyLoadError;
use super::hash::compute_config_hash;
use super::parser::parse_signal;

/// Stage value for a composed strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Research,
    Paper,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Research => write!(f, "research"),
            Self::Paper => write!(f, "paper"),
        }
    }
}

/// Sizing expression — v0.5 supports only `fixed_fraction`.
#[derive(Debug, Clone, PartialEq)]
pub enum Sizing {
    FixedFraction(Decimal),
}

/// Parsed, validated TOML config for a composed strategy.
///
/// The raw serde struct is `RawConfig`; `ComposedStrategyConfig` is the
/// post-validation form that also carries the parsed `RuleAst` and content hash.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    pub id: SmolStr,
    pub kind: SmolStr,
    pub symbol: SmolStr,
    pub stage: SmolStr,
    pub signal: SmolStr,
    pub size: SmolStr,
    #[serde(default)]
    pub params: BTreeMap<SmolStr, Decimal>,
}

/// Fully validated composed strategy configuration.
#[derive(Debug, Clone)]
pub struct ComposedStrategyConfig {
    pub id: SmolStr,
    pub symbol: SmolStr,
    pub stage: Stage,
    pub signal_raw: SmolStr,
    pub sizing: Sizing,
    pub params: BTreeMap<SmolStr, Decimal>,
    /// SHA-256 content hash (32 bytes) of the canonicalized AST.
    pub hash: [u8; 32],
}

impl ComposedStrategyConfig {
    /// Parse and validate a TOML file at `path`.
    ///
    /// `stem` is the filename without extension (the canonical `StrategyId`).
    ///
    /// # Errors
    ///
    /// Returns [`StrategyLoadError`] on any parse or validation failure.
    pub fn from_file(path: &Path) -> Result<Self, StrategyLoadError> {
        let bytes = std::fs::read(path)
            .map_err(|e| StrategyLoadError::IoRead(e.to_string()))?;
        let toml_str = std::str::from_utf8(&bytes)
            .map_err(|e| StrategyLoadError::TomlParse(format!("non-UTF8: {e}")))?;

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        Self::from_str(toml_str, stem)
    }

    /// Parse and validate TOML content with the given `stem` as the expected id.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyLoadError`] on parse or validation failure.
    pub fn from_str(toml_str: &str, stem: &str) -> Result<Self, StrategyLoadError> {
        let raw: RawConfig = toml::from_str(toml_str).map_err(|e| {
            // toml crate's error message can mention unknown fields
            let msg = e.to_string();
            if msg.contains("unknown field") || msg.contains("extra key") {
                StrategyLoadError::UnknownField(msg)
            } else {
                StrategyLoadError::TomlParse(msg)
            }
        })?;

        // Id-filename mismatch.
        if raw.id.as_str() != stem {
            return Err(StrategyLoadError::IdFilenameMismatch {
                stem: SmolStr::new(stem),
                id: raw.id.clone(),
            });
        }

        // Kind — only "composed" supported in v0.5.
        if raw.kind.as_str() != "composed" {
            return Err(StrategyLoadError::UnknownField(format!(
                "kind '{}' is not supported (only 'composed' in v0.5)",
                raw.kind
            )));
        }

        // Stage.
        let stage = match raw.stage.as_str() {
            "research" => Stage::Research,
            "paper" => Stage::Paper,
            other => return Err(StrategyLoadError::InvalidStage(SmolStr::new(other))),
        };

        // Signal — non-empty check.
        if raw.signal.trim().is_empty() {
            return Err(StrategyLoadError::EmptySignal);
        }

        // Parse the signal DSL.
        let ast = parse_signal(raw.signal.as_str(), &raw.params)?;

        // Typecheck the AST.
        super::typecheck::typecheck(&ast)?;

        // Sizing.
        let sizing = parse_sizing(raw.size.as_str())?;

        // Content hash.
        let hash = compute_config_hash(&raw.id, &ast, &raw.params);

        Ok(Self {
            id: raw.id,
            symbol: raw.symbol,
            stage,
            signal_raw: raw.signal,
            sizing,
            params: raw.params,
            hash,
        })
    }
}

/// Parse `"fixed_fraction(<f>)"` into `Sizing::FixedFraction`.
fn parse_sizing(s: &str) -> Result<Sizing, StrategyLoadError> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("fixed_fraction(").and_then(|r| r.strip_suffix(')')) {
        let f: Decimal = inner.trim().parse().map_err(|_| {
            StrategyLoadError::UnsupportedSizing(SmolStr::new(s))
        })?;
        Ok(Sizing::FixedFraction(f))
    } else {
        Err(StrategyLoadError::UnsupportedSizing(SmolStr::new(s)))
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t506_parse_valid_macd_trend() {
        let toml = r#"
id     = "btc_macd_trend"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
size   = "fixed_fraction(0.1)"
"#;
        let cfg = ComposedStrategyConfig::from_str(toml, "btc_macd_trend")
            .expect("should parse valid config");
        assert_eq!(cfg.id.as_str(), "btc_macd_trend");
        assert_eq!(cfg.stage, Stage::Research);
        assert!(matches!(cfg.sizing, Sizing::FixedFraction(_)));
    }

    #[test]
    fn t506_parse_valid_rsi_reversion() {
        let toml = r#"
id     = "btc_rsi_reversion"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "rsi(14) < 30 AND close > min(low, 20)"
size   = "fixed_fraction(0.1)"
"#;
        let cfg = ComposedStrategyConfig::from_str(toml, "btc_rsi_reversion")
            .expect("should parse valid rsi reversion config");
        assert_eq!(cfg.id.as_str(), "btc_rsi_reversion");
    }

    #[test]
    fn t506_parse_valid_bbands_mean_revert() {
        let toml = r#"
id     = "btc_bbands_mean_revert"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "close < bollinger_lower(20,2) AND volume > 1.5 * avg(volume, 20)"
size   = "fixed_fraction(0.1)"
"#;
        let cfg = ComposedStrategyConfig::from_str(toml, "btc_bbands_mean_revert")
            .expect("should parse valid bbands config");
        assert_eq!(cfg.id.as_str(), "btc_bbands_mean_revert");
    }

    #[test]
    fn t506_hash_is_deterministic() {
        let toml = r#"
id     = "btc_macd_trend"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
size   = "fixed_fraction(0.1)"
"#;
        let cfg1 = ComposedStrategyConfig::from_str(toml, "btc_macd_trend").unwrap();
        let cfg2 = ComposedStrategyConfig::from_str(toml, "btc_macd_trend").unwrap();
        assert_eq!(cfg1.hash, cfg2.hash, "content hash must be deterministic");
    }

    #[test]
    fn t506_hash_differs_for_different_signals() {
        let toml1 = r#"
id = "test_strategy"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 30"
size = "fixed_fraction(0.1)"
"#;
        let toml2 = r#"
id = "test_strategy"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 35"
size = "fixed_fraction(0.1)"
"#;
        let cfg1 = ComposedStrategyConfig::from_str(toml1, "test_strategy").unwrap();
        let cfg2 = ComposedStrategyConfig::from_str(toml2, "test_strategy").unwrap();
        assert_ne!(cfg1.hash, cfg2.hash, "different signals should have different hashes");
    }

    #[test]
    fn t506_with_params() {
        let toml = r#"
id     = "test_with_params"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "rsi(14) < rsi_floor"
size   = "fixed_fraction(0.1)"

[params]
rsi_floor = 35
"#;
        let cfg = ComposedStrategyConfig::from_str(toml, "test_with_params")
            .expect("should parse config with params");
        assert_eq!(cfg.params.len(), 1);
    }
}
