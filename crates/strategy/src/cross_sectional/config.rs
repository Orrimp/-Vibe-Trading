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

// ── Score source (D-CARRY.1, M-DEV-5) ─────────────────────────────────────────

/// Which signal source drives the cross-sectional ranking.
///
/// - `VolAdjustedReturn` (default): vol-adjusted price return, the v1 momentum/MR signal.
///   Every existing TOML and struct literal that omits this field keeps the v1 behavior
///   unchanged (serde `#[serde(default)]` → backward-compatible).
/// - `FundingCarry`: trailing-mean funding rate, negated (R-CARRY.2 sign convention).
///   `carry_score = −trailing_mean(funding)` so the most-negative-funding name floats
///   to the TOP of the unchanged descending `top_k_long` — the paid side earns.
/// - `BasisReversal`: trailing-mean basis, negated (R-BR.2 sign convention — LOAD-BEARING).
///   `basis_reversal_score = −trailing_mean(basis)` so the **lowest-basis** name floats
///   to the TOP of the unchanged descending `top_k_long` — the reversal-favored leg
///   (cheapest perp premium → outperforms). The minus is in ONE place (D-BR.1); a sign
///   flip turns the arm into a basis-MOMENTUM payer → RED on the sign-assertion falsifier.
///
/// **Anchor-neutrality:** `score_source` defaults `VolAdjustedReturn`; the 99 existing
/// momentum/MR/carry/TS/horizon anchors are byte-identical. Both carry and basis paths
/// are purely opt-in. The `BasisReversal` arm reuses the `funding_by_symbol` channel as
/// a generic sidecar carrier (D-BR.3) — the basis rides the same injection seam but is
/// consumed ONLY by `basis_reversal_score`, NEVER by the `run_path` accrual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScoreSource {
    /// Vol-adjusted price return (v1 behavior — default, anchor-neutral).
    #[default]
    VolAdjustedReturn,
    /// Funding-carry signal: −trailing_mean(funding) over L settlements (R-CARRY.1/2).
    FundingCarry,
    /// Basis-reversal signal: −trailing_mean(basis) over L bars (R-BR.1/2 — LOAD-BEARING SIGN).
    ///
    /// The perp-spot basis `(markPrice − indexPrice)/indexPrice` is used as a cross-sectional
    /// reversal signal. The SIGN (the leading minus) is the load-bearing convention (D-BR.1):
    /// - HIGH basis → HIGH crowd → subsequently UNDERPERFORMS → should be UNDERWEIGHTED.
    /// - LOW basis → LOW crowd → subsequently OUTPERFORMS → should be OVERWEIGHTED.
    /// - `−mean` makes the LOW-basis name score HIGHEST → it floats to the top of
    ///   the unchanged descending `top_k_long` → the arm longs the reversal-favored leg.
    ///
    /// The name `BasisReversal` (not `Basis`) documents the sign: there is no
    /// sign-neutral "basis" arm to confuse it with.
    BasisReversal,
    /// Basis-orthogonalized-to-funding rank-residual signal (D-MN.6, M-DEV-4).
    ///
    /// `residual_score[sym] = rank(basis_reversal_score[sym]) − rank(funding_carry_score[sym])`
    ///
    /// Both ranks are 1..N integers computed cross-sectionally at each rebalance over the
    /// warmed cross-section. The subtraction is EXACT (integer-valued `Decimal`) — NO
    /// division, NO rounding, NO f64 (D-MN.6 / ADR-0003 Decimal-exact requirement).
    ///
    /// The basis scores are read from `funding_map` (the same sidecar used by `BasisReversal`).
    /// The funding scores are read from `basis_score_map` (the second injected map, set via
    /// `with_basis_score`). The selection is `LongShort` — lowest residual → short (highest
    /// basis relative to its funding level), highest residual → long (lowest basis relative
    /// to its funding level).
    ///
    /// **Anchor-neutrality:** `BasisFundingResidual` is a NEW arm; `score_source` defaults
    /// `VolAdjustedReturn`; the 107 existing anchors are byte-identical (opt-in only).
    BasisFundingResidual,
}

// ── Selection mode (D-TSM.1, M-DEV-1) ─────────────────────────────────────────

/// How signals are selected after scoring (D-TSM.1).
///
/// - `CrossSectionalTopK` (default): rank all warmed names descending, take the top K.
///   This is the v1 behavior (momentum/MR/carry). Every existing TOML and struct literal
///   that omits this field keeps the v1 `top_k_long` selection path unchanged
///   (`serde` `#[serde(default)]` → fully backward-compatible, anchor-neutral).
/// - `TimeSeriesLongFlat`: each warmed asset decides long/flat on its OWN trailing-return
///   sign vs `entry_threshold`. NO cross-sectional ranking, NO top-K. The portfolio is
///   the equal-weight set of all above-threshold names; cardinality is variable (0..N).
///
/// **Anchor-neutrality:** `selection_mode` defaults `CrossSectionalTopK`; the existing
/// 89 momentum/MR/carry anchors are byte-identical by construction. The TS path is opt-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    /// Cross-sectional top-K selection (v1 behavior — default, anchor-neutral).
    #[default]
    CrossSectionalTopK,
    /// Time-series long/flat per-asset selection (D-TSM.1 — no ranking).
    TimeSeriesLongFlat,
    /// Dollar-neutral long-low/short-high selection (D-MN.5, M-DEV-2).
    ///
    /// Selects the top-K by score (long book) AND the bottom-K by score
    /// (short book) — `k_short > 0` is ONLY permitted under this mode.
    /// Serde-default stays `CrossSectionalTopK` → all 107 anchors are
    /// byte-identical (the existing serialization path is unchanged).
    LongShort,
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
    #[error(
        "[inert_direction] direction = \"reversion\" is behaviorally INERT with \
         score_source = {score_source:?} / selection_mode = {selection_mode:?}: the D-MR.1 score \
         inversion applies ONLY to the vol_adjusted_return score under a cross-sectional \
         selection mode (cross_sectional_top_k / long_short); this combination would run \
         identity-direction (momentum-equivalent) behavior while still hashing as a distinct \
         \"reversion\" config — two K3 identities for one behavior. Drop `direction` or switch \
         to the inverting arm."
    )]
    InertDirection {
        score_source: ScoreSource,
        selection_mode: SelectionMode,
    },
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
            Self::InertDirection { .. } => "inert_direction",
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
    /// Score source (M-DEV-5, D-CARRY.1).
    /// Default = `VolAdjustedReturn` (v1 behavior) — serde `#[serde(default)]`
    /// keeps all existing TOMLs and struct literals anchor-neutral.
    /// Set to `FundingCarry` for the carry strategy.
    #[serde(default)]
    pub score_source: ScoreSource,
    /// Selection mode (M-DEV-1, D-TSM.1).
    /// Default = `CrossSectionalTopK` (v1 behavior) — serde `#[serde(default)]`
    /// keeps all existing TOMLs and struct literals anchor-neutral (89 anchors unchanged).
    /// Set to `TimeSeriesLongFlat` for time-series momentum.
    #[serde(default)]
    pub selection_mode: SelectionMode,
    /// Flat/entry threshold for `TimeSeriesLongFlat` selection (D-TSM.1).
    /// Default = `Decimal::ZERO` → inert for all existing momentum/MR/carry runs.
    /// Only read under `SelectionMode::TimeSeriesLongFlat`; ignored by `CrossSectionalTopK`.
    /// A negative value permits entry on a mild downtrend (wider-than-zero band).
    #[serde(default)]
    pub entry_threshold: Decimal,
}

/// Raw deserializable form before validation.
///
/// Review 1-16: `deny_unknown_fields` — a typo'd KEY (e.g. `direcion = "reversion"`)
/// previously deserialized cleanly (the real field fell back to its serde default)
/// and silently ran Momentum: unknown VALUES failed loudly, unknown KEYS did not.
/// No checked-in config carries unknown keys — the one production TOML of this
/// kind, `config/strategies/top10_momentum_h1.toml`, uses only declared fields
/// (as do all `crates/strategy/tests/fixtures/bad_v1_strategies/*.toml` fixtures).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Score source — default = `VolAdjustedReturn` so existing TOMLs keep v1 behavior.
    #[serde(default)]
    pub score_source: ScoreSource,
    /// Selection mode — default = `CrossSectionalTopK` so existing TOMLs keep v1 behavior.
    #[serde(default)]
    pub selection_mode: SelectionMode,
    /// Entry threshold — default = `Decimal::ZERO` → inert for all existing runs.
    #[serde(default)]
    pub entry_threshold: Decimal,
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

        // Q3 / M-DEV-2: k_short > 0 is permitted ONLY under LongShort mode.
        // Under CrossSectionalTopK and TimeSeriesLongFlat, shorts have no semantics
        // and k_short > 0 is still rejected (existing error preserved for those modes).
        if raw.k_short > 0 && raw.selection_mode != SelectionMode::LongShort {
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

        // Review 1-16: `direction = "reversion"` requires the inverting arm.
        // The D-MR.1 negation lives ONLY on the VolAdjustedReturn score path
        // under a cross-sectional selection mode (`on_bar` inverts at the score
        // cache boundary for CrossSectionalTopK/LongShort; carry/basis/residual
        // signs live inside their score fns and ignore `direction`;
        // TimeSeriesLongFlat ignores `direction` entirely). Any other
        // combination is behaviorally INERT yet hash-distinguishing (two K3
        // identities for one behavior — e.g. "carry reversion" silently runs
        // identity-direction carry) — reject loudly instead.
        if raw.direction == Direction::Reversion
            && (raw.score_source != ScoreSource::VolAdjustedReturn
                || raw.selection_mode == SelectionMode::TimeSeriesLongFlat)
        {
            return Err(CrossSectionalLoadError::InertDirection {
                score_source: raw.score_source,
                selection_mode: raw.selection_mode,
            });
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
            score_source: raw.score_source,
            selection_mode: raw.selection_mode,
            entry_threshold: raw.entry_threshold,
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

    // ── M-DEV-5: ScoreSource field tests ─────────────────────────────────────

    /// M-DEV-5 (a): TOML with no `score_source` field → `ScoreSource::VolAdjustedReturn`
    /// (backward-compat — all 87 existing anchors are unaffected).
    #[test]
    fn m_dev5_no_score_source_defaults_to_vol_adjusted_return() {
        let cfg = CrossSectionalMomentumConfig::from_str(VALID_TOML).unwrap();
        assert_eq!(
            cfg.score_source,
            ScoreSource::VolAdjustedReturn,
            "omitting `score_source` must default to VolAdjustedReturn (backward compat)"
        );
    }

    /// M-DEV-5 (b): `score_source = "funding_carry"` parses correctly.
    #[test]
    fn m_dev5_score_source_funding_carry_parses() {
        let toml = r#"
id    = "test_carry"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
score_source = "funding_carry"
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml).unwrap();
        assert_eq!(
            cfg.score_source,
            ScoreSource::FundingCarry,
            "`score_source = \"funding_carry\"` must parse to ScoreSource::FundingCarry"
        );
    }

    /// M-DEV-3 (a): `score_source = "basis_reversal"` parses correctly.
    #[test]
    fn m_dev3_score_source_basis_reversal_parses() {
        let toml = r#"
id    = "test_basis"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
score_source = "basis_reversal"
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml).unwrap();
        assert_eq!(
            cfg.score_source,
            ScoreSource::BasisReversal,
            "`score_source = \"basis_reversal\"` must parse to ScoreSource::BasisReversal"
        );
    }

    /// M-DEV-3 (b): Config hash differs between VolAdjustedReturn and BasisReversal
    /// at identical θ (K3 — basis-vs-momentum hash discriminator).
    #[test]
    fn m_dev3_config_hash_differs_by_basis_reversal() {
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
        let mut cfg_var = CrossSectionalMomentumConfig::from_str(toml_base).unwrap();
        let mut cfg_basis = cfg_var.clone();
        cfg_basis.score_source = ScoreSource::BasisReversal;

        cfg_var.id = SmolStr::new("test_hash");
        cfg_basis.id = SmolStr::new("test_hash");

        let strat_var = MomentumStrategy::from_config(cfg_var, SmolStr::new("test"));
        let strat_basis = MomentumStrategy::from_config(cfg_basis, SmolStr::new("test"));

        assert_ne!(
            strat_var.hash, strat_basis.hash,
            "VolAdjustedReturn and BasisReversal configs at identical θ MUST produce different hashes (K3)"
        );
    }

    /// M-DEV-3 (c): Config hash differs between FundingCarry and BasisReversal
    /// at identical θ (K3 — carry-vs-basis hash discriminator).
    #[test]
    fn m_dev3_config_hash_differs_carry_vs_basis_reversal() {
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
        let mut cfg_carry = CrossSectionalMomentumConfig::from_str(toml_base).unwrap();
        let mut cfg_basis = cfg_carry.clone();
        cfg_carry.score_source = ScoreSource::FundingCarry;
        cfg_basis.score_source = ScoreSource::BasisReversal;

        cfg_carry.id = SmolStr::new("test_hash");
        cfg_basis.id = SmolStr::new("test_hash");

        let strat_carry = MomentumStrategy::from_config(cfg_carry, SmolStr::new("test"));
        let strat_basis = MomentumStrategy::from_config(cfg_basis, SmolStr::new("test"));

        assert_ne!(
            strat_carry.hash, strat_basis.hash,
            "FundingCarry and BasisReversal configs at identical θ MUST produce different hashes (K3)"
        );
    }

    /// M-DEV-5 (c): Config hash differs between VolAdjustedReturn and FundingCarry at
    /// identical θ (K3 — carry-vs-momentum hash discriminator).
    #[test]
    fn m_dev5_config_hash_differs_by_score_source() {
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
        let mut cfg_var = CrossSectionalMomentumConfig::from_str(toml_base).unwrap();
        let mut cfg_carry = cfg_var.clone();
        cfg_carry.score_source = ScoreSource::FundingCarry;

        cfg_var.id = SmolStr::new("test_hash");
        cfg_carry.id = SmolStr::new("test_hash");

        let strat_var = MomentumStrategy::from_config(cfg_var, SmolStr::new("test"));
        let strat_carry = MomentumStrategy::from_config(cfg_carry, SmolStr::new("test"));

        assert_ne!(
            strat_var.hash, strat_carry.hash,
            "VolAdjustedReturn and FundingCarry configs at identical θ MUST produce different hashes (K3)"
        );
    }

    // ── M-DEV-1: SelectionMode field tests ────────────────────────────────────

    /// M-DEV-1 (a): TOML with no `selection_mode` field → `SelectionMode::CrossSectionalTopK`
    /// (backward-compat — all 89 existing anchors are unaffected).
    #[test]
    fn m_dev1_no_selection_mode_defaults_to_cross_sectional_top_k() {
        let cfg = CrossSectionalMomentumConfig::from_str(VALID_TOML).unwrap();
        assert_eq!(
            cfg.selection_mode,
            SelectionMode::CrossSectionalTopK,
            "omitting `selection_mode` must default to CrossSectionalTopK (backward compat)"
        );
    }

    /// M-DEV-1 (b): TOML with no `entry_threshold` field → `Decimal::ZERO` (backward-compat).
    #[test]
    fn m_dev1_no_entry_threshold_defaults_to_zero() {
        let cfg = CrossSectionalMomentumConfig::from_str(VALID_TOML).unwrap();
        assert_eq!(
            cfg.entry_threshold,
            Decimal::ZERO,
            "omitting `entry_threshold` must default to Decimal::ZERO (backward compat)"
        );
    }

    /// M-DEV-1 (c): `selection_mode = "time_series_long_flat"` parses correctly.
    #[test]
    fn m_dev1_selection_mode_time_series_long_flat_parses() {
        let toml = r#"
id    = "test_ts"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
selection_mode = "time_series_long_flat"
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml).unwrap();
        assert_eq!(
            cfg.selection_mode,
            SelectionMode::TimeSeriesLongFlat,
            "`selection_mode = \"time_series_long_flat\"` must parse to SelectionMode::TimeSeriesLongFlat"
        );
    }

    /// M-DEV-1 (d): Config hash differs between CrossSectionalTopK and TimeSeriesLongFlat at
    /// identical θ (K3 — TS-vs-momentum hash discriminator).
    #[test]
    fn m_dev1_config_hash_differs_by_selection_mode() {
        use super::super::momentum::MomentumStrategy;
        use smol_str::SmolStr;

        let toml_base = r#"
id    = "test_hash"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = 168
rebalance_minutes = 60
k_long = 2
"#;
        let mut cfg_cs = CrossSectionalMomentumConfig::from_str(toml_base).unwrap();
        let mut cfg_ts = cfg_cs.clone();
        cfg_ts.selection_mode = SelectionMode::TimeSeriesLongFlat;

        cfg_cs.id = SmolStr::new("test_hash");
        cfg_ts.id = SmolStr::new("test_hash");

        let strat_cs = MomentumStrategy::from_config(cfg_cs, SmolStr::new("test"));
        let strat_ts = MomentumStrategy::from_config(cfg_ts, SmolStr::new("test"));

        assert_ne!(
            strat_cs.hash, strat_ts.hash,
            "CrossSectionalTopK and TimeSeriesLongFlat configs at identical θ MUST produce different hashes (K3)"
        );
    }

    /// M-DEV-1 (e): Config hash differs when entry_threshold differs at identical other θ (K3).
    #[test]
    fn m_dev1_config_hash_differs_by_entry_threshold() {
        use super::super::momentum::MomentumStrategy;
        use smol_str::SmolStr;

        let toml_base = r#"
id    = "test_hash"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = 168
rebalance_minutes = 60
k_long = 2
selection_mode = "time_series_long_flat"
"#;
        let mut cfg_zero = CrossSectionalMomentumConfig::from_str(toml_base).unwrap();
        let mut cfg_two_pct = cfg_zero.clone();
        cfg_two_pct.entry_threshold = Decimal::new(2, 2); // 0.02

        cfg_zero.id = SmolStr::new("test_hash");
        cfg_two_pct.id = SmolStr::new("test_hash");

        let strat_zero = MomentumStrategy::from_config(cfg_zero, SmolStr::new("test"));
        let strat_two_pct = MomentumStrategy::from_config(cfg_two_pct, SmolStr::new("test"));

        assert_ne!(
            strat_zero.hash, strat_two_pct.hash,
            "entry_threshold=0.00 and entry_threshold=0.02 configs MUST produce different hashes (K3)"
        );
    }

    // ── M-DEV-2: SelectionMode::LongShort + k_short tests ────────────────────

    /// M-DEV-2 (a): `selection_mode = "long_short"` parses correctly.
    #[test]
    fn m_dev2_selection_mode_long_short_parses() {
        let toml = r#"
id    = "test_ls"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
selection_mode = "long_short"
k_long = 1
k_short = 1
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml).unwrap();
        assert_eq!(
            cfg.selection_mode,
            SelectionMode::LongShort,
            "`selection_mode = \"long_short\"` must parse to SelectionMode::LongShort"
        );
        assert_eq!(
            cfg.k_short, 1,
            "k_short = 1 under LongShort must be accepted"
        );
    }

    /// M-DEV-2 (b): `k_short > 0` under LongShort is ACCEPTED (the gate lifts).
    #[test]
    fn m_dev2_k_short_positive_accepted_under_long_short() {
        let toml = r#"
id    = "test_ls"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
selection_mode = "long_short"
k_long = 1
k_short = 1
"#;
        let result = CrossSectionalMomentumConfig::from_str(toml);
        assert!(
            result.is_ok(),
            "k_short > 0 under LongShort must be accepted; got err: {:?}",
            result.err()
        );
    }

    /// M-DEV-2 (c): `k_short > 0` under CrossSectionalTopK is still REJECTED.
    #[test]
    fn m_dev2_k_short_positive_still_rejected_under_cross_sectional_top_k() {
        let toml = r#"
id    = "test_ls"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
k_long = 1
k_short = 1
"#;
        let result = CrossSectionalMomentumConfig::from_str(toml);
        assert!(
            matches!(result, Err(CrossSectionalLoadError::UnsupportedShortSizing)),
            "k_short > 0 under CrossSectionalTopK must still be rejected"
        );
    }

    /// M-DEV-2 (d): LongShort config hashes differently from CrossSectionalTopK (K3).
    #[test]
    fn m_dev_mn_config_hash_differs_by_long_short() {
        use super::super::momentum::MomentumStrategy;
        use smol_str::SmolStr;

        let toml_cs = r#"
id    = "test_hash"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = 60
rebalance_minutes = 480
k_long = 3
"#;
        let toml_ls = r#"
id    = "test_hash"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = 60
rebalance_minutes = 480
k_long = 3
k_short = 3
selection_mode = "long_short"
"#;
        let cfg_cs = CrossSectionalMomentumConfig::from_str(toml_cs).unwrap();
        let cfg_ls = CrossSectionalMomentumConfig::from_str(toml_ls).unwrap();

        let strat_cs = MomentumStrategy::from_config(cfg_cs, SmolStr::new("test"));
        let strat_ls = MomentumStrategy::from_config(cfg_ls, SmolStr::new("test"));

        assert_ne!(
            strat_cs.hash, strat_ls.hash,
            "CrossSectionalTopK (k_short=0) and LongShort (k_short=3) configs MUST produce \
             different hashes (K3 — the config hash distinguishes strategy variants)"
        );
    }

    // ── Review 1-16: deny_unknown_fields + inert-direction cross-field checks ──

    /// Review 1-16: a typo'd KEY must fail loudly instead of silently running
    /// the field's default. `direcion` (sic) previously deserialized cleanly
    /// and ran Momentum.
    #[test]
    fn review_1_16_unknown_key_rejected() {
        let toml = r#"
id    = "test_typo"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
direcion = "reversion"
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "toml_parse",
            "an unknown key must be rejected at parse (deny_unknown_fields), got: {err}"
        );
        assert!(
            err.to_string().contains("direcion"),
            "the parse error must name the offending key: {err}"
        );
    }

    /// Review 1-16: the one checked-in production TOML of this kind
    /// (`config/strategies/top10_momentum_h1.toml`) still parses under
    /// `deny_unknown_fields` — its field set is mirrored by VALID_TOML plus the
    /// two commented production-only fields it does not carry.
    #[test]
    fn review_1_16_production_toml_still_parses() {
        // Resolve the workspace root from this crate's manifest dir (crates/strategy).
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/strategies/top10_momentum_h1.toml");
        let cfg = CrossSectionalMomentumConfig::from_file(&path)
            .expect("checked-in production TOML must keep parsing under deny_unknown_fields");
        assert_eq!(cfg.id.as_str(), "top10_momentum_h1");
        assert_eq!(cfg.universe.len(), 10);
        assert_eq!(cfg.direction, Direction::Momentum);
    }

    /// Review 1-16 (REQUIRED): reversion + funding_carry is behaviorally inert
    /// (identity-direction carry) — must be rejected with `inert_direction`.
    #[test]
    fn review_1_16_reversion_plus_funding_carry_rejected() {
        let toml = r#"
id    = "test_inert"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
direction = "reversion"
score_source = "funding_carry"
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "inert_direction",
            "reversion + funding_carry must be rejected as inert, got: {err}"
        );
        assert!(
            err.to_string().contains("FundingCarry"),
            "the error must name the inert combination: {err}"
        );
    }

    /// Review 1-16: reversion + time_series_long_flat is inert too (`direction`
    /// is ignored entirely on the TS path) — must be rejected.
    #[test]
    fn review_1_16_reversion_plus_time_series_rejected() {
        let toml = r#"
id    = "test_inert_ts"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
direction = "reversion"
selection_mode = "time_series_long_flat"
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "inert_direction",
            "reversion + time_series_long_flat must be rejected as inert, got: {err}"
        );
    }

    /// Review 1-16 boundary: the INVERTING arm still parses — reversion +
    /// vol_adjusted_return + cross_sectional_top_k is the MR family (anchored
    /// #87 lane) and must remain accepted.
    #[test]
    fn review_1_16_reversion_inverting_arm_still_accepted() {
        let toml = r#"
id    = "test_mr_ok"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
direction = "reversion"
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml)
            .expect("the MR inverting arm must remain accepted");
        assert_eq!(cfg.direction, Direction::Reversion);
        assert_eq!(cfg.score_source, ScoreSource::VolAdjustedReturn);
        assert_eq!(cfg.selection_mode, SelectionMode::CrossSectionalTopK);
    }

    /// Review 1-16 boundary: reversion under LongShort × vol_adjusted_return is
    /// NOT inert (the negation applies on that path and swaps the books) — it
    /// stays accepted; only the inert combinations are rejected.
    #[test]
    fn review_1_16_reversion_long_short_not_rejected() {
        let toml = r#"
id    = "test_ls_rev"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
direction = "reversion"
selection_mode = "long_short"
k_long = 1
k_short = 1
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml)
            .expect("reversion under LongShort inverts (not inert) and must stay accepted");
        assert_eq!(cfg.direction, Direction::Reversion);
        assert_eq!(cfg.selection_mode, SelectionMode::LongShort);
    }
}
