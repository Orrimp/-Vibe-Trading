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
    #[error(
        "[inert_score_source] score_source = {score_source:?} is behaviorally INERT with \
         selection_mode = \"time_series_long_flat\": the TS arm computes its own trailing \
         log-return trend score and NEVER reads score_source — this combination would silently \
         run price-TS behavior while still hashing as a distinct {score_source:?} config (two \
         K3 identities for one behavior), and any carry/basis sidecar loaded for it would be \
         dead weight under a misleading identity. Drop `score_source` (the default \
         vol_adjusted_return) or switch to a cross-sectional selection mode."
    )]
    InertScoreSource { score_source: ScoreSource },
    #[error(
        "[inert_threshold] entry_threshold = {entry_threshold} is behaviorally INERT with \
         selection_mode = {selection_mode:?}: the threshold is read ONLY under \
         \"time_series_long_flat\" — a nonzero value here is ignored at runtime yet still \
         hashes as a distinct config (two K3 identities for one behavior). Drop \
         `entry_threshold` or switch selection_mode to \"time_series_long_flat\"."
    )]
    InertThreshold {
        entry_threshold: Decimal,
        selection_mode: SelectionMode,
    },
    #[error(
        "[degenerate_residual_arm] score_source = \"basis_funding_residual\" DEGENERATES with \
         selection_mode = {selection_mode:?}: the rank-residual is computed ONLY under \
         \"long_short\" (`build_rebalance_signals` derives `effective_scores` behind \
         `selection_mode == LongShort && score_source == BasisFundingResidual`). Under any \
         other mode the cached `self.scores` are used instead — and for this arm those hold \
         the plain trailing basis mean, i.e. a BasisReversal-shaped score. So the config \
         would SILENTLY run a different arm than the one it names, while hashing as a \
         distinct \"basis_funding_residual\" identity: two K3 identities for one behavior, \
         and a θ-surface labelled `score_source=basis_funding_residual` whose numbers came \
         from the basis-reversal signal. Switch selection_mode to \"long_short\" or pick the \
         score_source you actually want."
    )]
    DegenerateResidualArm { selection_mode: SelectionMode },
    #[error(
        "[invalid_entry_threshold] entry_threshold = {0} is out of range: must be <= 1.0. The \
         threshold is compared against a trailing LOG-return, so 1.0 already demands a ~172% \
         price rise over the lookback before entering — values above it are almost certainly a \
         units mistake (every shipped TS cell uses 0.00 or 0.02). Negative values ARE allowed: \
         a negative threshold widens the entry band (enter on a mild downtrend), and falsifier \
         fixtures use deeply-negative thresholds deliberately to force always-long behavior."
    )]
    InvalidEntryThreshold(Decimal),
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
            Self::InertScoreSource { .. } => "inert_score_source",
            Self::InertThreshold { .. } => "inert_threshold",
            Self::DegenerateResidualArm { .. } => "degenerate_residual_arm",
            Self::InvalidEntryThreshold(_) => "invalid_entry_threshold",
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
    /// Only read under `SelectionMode::TimeSeriesLongFlat`; the loader rejects a
    /// nonzero value under any other mode (`inert_threshold`, review 1-17) and
    /// values > 1.0 (`invalid_entry_threshold` — a 100% log-return entry bar is
    /// a units mistake). A negative value permits entry on a mild downtrend
    /// (wider-than-zero band) and stays allowed — falsifier fixtures use
    /// deeply-negative thresholds deliberately to force always-long behavior.
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

        // Review 1-17: entry_threshold bounds. The threshold is compared against
        // a trailing LOG-return, so 1.0 (~172% price rise over the lookback) is
        // the generous upper sanity bound — every sibling numeric field is
        // range-checked and this was the one unvalidated numeric in the loader.
        // NEGATIVE values stay ALLOWED deliberately: a negative threshold widens
        // the entry band (enter on a mild downtrend, documented on the field),
        // and falsifier fixtures use deeply-negative thresholds to force
        // always-long behavior (F-TSM.2's degenerate control).
        if raw.entry_threshold > Decimal::ONE {
            return Err(CrossSectionalLoadError::InvalidEntryThreshold(
                raw.entry_threshold,
            ));
        }

        // Review 1-17: score_source is NEVER read under TimeSeriesLongFlat (the
        // TS arm computes its own trailing log-return trend score) — any non-
        // default score_source there is behaviorally inert yet hash-distinct
        // ("TS carry" would silently run price-TS under a carry identity).
        // Mirrors the 1-16 InertDirection guard one field over.
        if raw.selection_mode == SelectionMode::TimeSeriesLongFlat
            && raw.score_source != ScoreSource::VolAdjustedReturn
        {
            return Err(CrossSectionalLoadError::InertScoreSource {
                score_source: raw.score_source,
            });
        }

        // Review 1-17: entry_threshold is read ONLY under TimeSeriesLongFlat —
        // a nonzero value under CrossSectionalTopK/LongShort is behaviorally
        // inert yet hash-distinct (two K3 identities for one behavior).
        if raw.entry_threshold != Decimal::ZERO
            && raw.selection_mode != SelectionMode::TimeSeriesLongFlat
        {
            return Err(CrossSectionalLoadError::InertThreshold {
                entry_threshold: raw.entry_threshold,
                selection_mode: raw.selection_mode,
            });
        }

        // Review 1-21: `basis_funding_residual` is COMPUTED only under LongShort.
        //
        // `build_rebalance_signals` builds `effective_scores` behind
        // `selection_mode == LongShort && score_source == BasisFundingResidual`; under
        // any other selection mode it falls back to `self.scores`, which for this arm
        // holds `basis_trailing_mean_for_residual` — a plain −mean(basis), i.e. the
        // BasisReversal signal. A config naming the residual arm would therefore run the
        // basis arm and still hash as a distinct identity. Same failure class, and the
        // same remedy, as the 1-16 `InertDirection` and 1-17 `InertScoreSource` guards
        // sitting either side of this one.
        //
        // Accept-set impact: NONE. No checked-in TOML sets `score_source` at all (the
        // sweep driver mutates the field on a loaded struct), and every anchored MN
        // residual surface (#116-#119) ran `selection_mode = long_short`. This rejects
        // only combinations nothing uses — proven by
        // `tests::degenerate_residual_arm_rejects_only_non_long_short`.
        if raw.score_source == ScoreSource::BasisFundingResidual
            && raw.selection_mode != SelectionMode::LongShort
        {
            return Err(CrossSectionalLoadError::DegenerateResidualArm {
                selection_mode: raw.selection_mode,
            });
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
    ///
    /// # Kept in step with [`Self::from_str`] (review 1-21)
    ///
    /// This schema had gone stale: it still declared `"k_short": {"minimum": 0,
    /// "maximum": 0}` — the v1 spot-only rule — although M-DEV-2 lifted that gate and the
    /// loader has accepted `k_short > 0` under `selection_mode = "long_short"` ever since
    /// (every anchored MN surface, #108-#119, ran `k_long = k_short = 3`). A schema that
    /// forbids the axis the shipped configs use is worse than no schema: it is a
    /// confident, wrong answer to "what may I write here?".
    ///
    /// The three axes the loader validates (`direction`, `score_source`,
    /// `selection_mode`) were missing entirely and are declared here now. The
    /// CROSS-FIELD rules — `k_short > 0` ⇒ `long_short`, `entry_threshold ≠ 0` ⇒
    /// `time_series_long_flat`, `basis_funding_residual` ⇒ `long_short`,
    /// `reversion` ⇒ `vol_adjusted_return` + a cross-sectional mode — are NOT expressible
    /// in this flat property list; they live in `from_str`, which is the authority.
    /// `tests::json_schema_matches_the_loader_on_k_short` pins the one that regressed.
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
                // M-DEV-2: > 0 requires selection_mode = "long_short" (enforced in from_str).
                "k_short": { "type": "integer", "minimum": 0, "default": 0 },
                "exposure_cap": { "type": "number", "exclusiveMinimum": 0, "maximum": 1, "default": 0.5 },
                "drift_rebalance_threshold": { "type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1, "default": 0.1 },
                "vol_floor": { "type": "number", "exclusiveMinimum": 0, "default": 0.000001 },
                "size": { "type": "string", "const": "equal_weight" },
                "direction": {
                    "type": "string",
                    "enum": ["momentum", "reversion"],
                    "default": "momentum"
                },
                "score_source": {
                    "type": "string",
                    "enum": [
                        "vol_adjusted_return",
                        "funding_carry",
                        "basis_reversal",
                        "basis_funding_residual"
                    ],
                    "default": "vol_adjusted_return"
                },
                "selection_mode": {
                    "type": "string",
                    "enum": ["cross_sectional_top_k", "time_series_long_flat", "long_short"],
                    "default": "cross_sectional_top_k"
                },
                // Read ONLY under time_series_long_flat (enforced in from_str).
                "entry_threshold": { "type": "number", "maximum": 1.0, "default": 0 }
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

    // ── Review 1-17: InertScoreSource / InertThreshold / entry_threshold bounds ─

    /// Review 1-17: TS × funding_carry is behaviorally inert (the TS arm never
    /// reads score_source — it would silently run price-TS under a carry
    /// identity) — must be rejected with `inert_score_source`.
    #[test]
    fn review_1_17_ts_plus_funding_carry_rejected() {
        let toml = r#"
id    = "test_ts_carry"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
selection_mode = "time_series_long_flat"
score_source = "funding_carry"
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "inert_score_source",
            "TS × funding_carry must be rejected as inert, got: {err}"
        );
        assert!(
            err.to_string().contains("FundingCarry"),
            "the error must name the inert score source: {err}"
        );
    }

    /// Review 1-17: TS × basis_reversal is inert for the same reason.
    #[test]
    fn review_1_17_ts_plus_basis_reversal_rejected() {
        let toml = r#"
id    = "test_ts_basis"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
selection_mode = "time_series_long_flat"
score_source = "basis_reversal"
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "inert_score_source",
            "TS × basis_reversal must be rejected as inert, got: {err}"
        );
    }

    /// Review 1-17 boundary: the shipped TS lane (TS × default
    /// vol_adjusted_return) must remain accepted.
    #[test]
    fn review_1_17_ts_with_default_score_source_still_accepted() {
        let toml = r#"
id    = "test_ts_ok"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
selection_mode = "time_series_long_flat"
entry_threshold = 0.02
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml)
            .expect("the shipped TS lane must remain accepted");
        assert_eq!(cfg.selection_mode, SelectionMode::TimeSeriesLongFlat);
        assert_eq!(cfg.score_source, ScoreSource::VolAdjustedReturn);
        assert_eq!(cfg.entry_threshold, Decimal::new(2, 2));
    }

    /// Review 1-17: a nonzero entry_threshold under CrossSectionalTopK is
    /// ignored at runtime yet hash-distinct — rejected with `inert_threshold`.
    #[test]
    fn review_1_17_nonzero_threshold_under_top_k_rejected() {
        let toml = r#"
id    = "test_thr_topk"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
entry_threshold = 0.02
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "inert_threshold",
            "nonzero entry_threshold under CrossSectionalTopK must be rejected, got: {err}"
        );
        assert!(
            err.to_string().contains("CrossSectionalTopK"),
            "the error must name the mode the threshold is inert under: {err}"
        );
    }

    /// Review 1-17: the threshold is inert under LongShort too — same rejection.
    #[test]
    fn review_1_17_nonzero_threshold_under_long_short_rejected() {
        let toml = r#"
id    = "test_thr_ls"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
selection_mode = "long_short"
k_long = 1
k_short = 1
entry_threshold = 0.01
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "inert_threshold",
            "nonzero entry_threshold under LongShort must be rejected, got: {err}"
        );
    }

    /// Review 1-17: entry_threshold > 1.0 (a 100% log-return entry bar) is a
    /// units mistake — rejected with `invalid_entry_threshold`.
    #[test]
    fn review_1_17_entry_threshold_above_one_rejected() {
        let toml = r#"
id    = "test_thr_500pct"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
selection_mode = "time_series_long_flat"
entry_threshold = 5.0
"#;
        let err = CrossSectionalMomentumConfig::from_str(toml).unwrap_err();
        assert_eq!(
            err.error_code(),
            "invalid_entry_threshold",
            "entry_threshold = 5.0 (500%) must be rejected, got: {err}"
        );
    }

    /// Review 1-17 boundary: exactly 1.0 is the inclusive upper bound (still
    /// accepted); negative thresholds stay allowed (falsifier fixtures rely on
    /// deeply-negative thresholds to force always-long behavior).
    #[test]
    fn review_1_17_entry_threshold_boundary_and_negative_accepted() {
        let toml_one = r#"
id    = "test_thr_one"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
selection_mode = "time_series_long_flat"
entry_threshold = 1.0
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml_one)
            .expect("entry_threshold = 1.0 is the inclusive bound and must parse");
        assert_eq!(cfg.entry_threshold, Decimal::new(10, 1));

        let toml_neg = r#"
id    = "test_thr_neg"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
selection_mode = "time_series_long_flat"
entry_threshold = -999999.0
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml_neg)
            .expect("negative entry_threshold must stay accepted (falsifier fixture pattern)");
        assert!(cfg.entry_threshold < Decimal::ZERO);
    }

    // ── Review 1-21 ───────────────────────────────────────────────────────────

    /// Review 1-21 MEDIUM: `basis_funding_residual` must be LOUD outside `long_short`,
    /// and the rejection must be NARROW.
    ///
    /// The degeneration: `build_rebalance_signals` computes the rank residual only when
    /// `selection_mode == LongShort`; otherwise the cached `self.scores` are used, which
    /// for this arm hold the plain trailing basis mean. A config saying
    /// `basis_funding_residual` under `cross_sectional_top_k` silently ran the
    /// basis-reversal signal under a residual identity.
    ///
    /// The test enumerates the FULL accept-set of the new guard, per the patch-pass
    /// contract: `long_short` is accepted, and the two other modes are the only things
    /// rejected. `time_series_long_flat` is rejected by the OLDER `InertScoreSource`
    /// guard, which fires first — that is fine, both are loud; this asserts the
    /// specific code for the mode that reaches the new one.
    #[test]
    fn degenerate_residual_arm_rejects_only_non_long_short() {
        let toml_for = |mode: &str, extra: &str| {
            format!(
                r#"
id    = "test_residual_mode"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
score_source = "basis_funding_residual"
selection_mode = "{mode}"
{extra}
"#
            )
        };

        // ACCEPTED — the one mode that actually computes the residual.
        let ok = CrossSectionalMomentumConfig::from_str(&toml_for("long_short", "k_short = 3"))
            .expect("basis_funding_residual under long_short is the shipped MN residual arm");
        assert_eq!(ok.score_source, ScoreSource::BasisFundingResidual);
        assert_eq!(ok.selection_mode, SelectionMode::LongShort);
        // …and k_short = 0 under long_short stays legal (the falsifiers use it).
        CrossSectionalMomentumConfig::from_str(&toml_for("long_short", "k_short = 0"))
            .expect("long_short with k_short = 0 is the un-shorted control arm");

        // REJECTED — the mode that silently ran a different arm.
        let err = CrossSectionalMomentumConfig::from_str(&toml_for("cross_sectional_top_k", ""))
            .expect_err("basis_funding_residual under cross_sectional_top_k must be rejected");
        assert_eq!(err.error_code(), "degenerate_residual_arm", "got: {err}");
        assert!(
            err.to_string().contains("DEGENERATES"),
            "the message must say WHAT degenerates, not just that something is invalid. \
             Got: {err}"
        );

        // REJECTED by the older sibling guard (also loud) — recorded so a future edit
        // that reorders the checks does not mistake this for an acceptance.
        let err_ts = CrossSectionalMomentumConfig::from_str(&toml_for("time_series_long_flat", ""))
            .expect_err("basis_funding_residual under time_series_long_flat must be rejected");
        assert!(
            matches!(
                err_ts.error_code(),
                "inert_score_source" | "degenerate_residual_arm"
            ),
            "expected one of the two loud guards, got: {err_ts}"
        );
    }

    /// Review 1-21 LOW: the JSON schema must not forbid the axis the loader accepts.
    ///
    /// `"k_short": {"minimum": 0, "maximum": 0}` outlived the v1 spot-only rule by the
    /// whole MN story: every anchored MN surface ran `k_long = k_short = 3`. This pins
    /// the schema against the loader so the two cannot drift apart again silently.
    #[test]
    fn json_schema_matches_the_loader_on_k_short() {
        let schema = CrossSectionalMomentumConfig::json_schema();
        let k_short = &schema["properties"]["k_short"];
        assert_eq!(k_short["minimum"], serde_json::json!(0));
        assert!(
            k_short.get("maximum").is_none(),
            "the schema must not cap k_short at 0 — the loader accepts k_short > 0 under \
             selection_mode = \"long_short\" (M-DEV-2), which is what every anchored MN \
             surface ran. Schema said: {k_short}"
        );

        // The loader really does accept the value the schema now permits (the vacuity
        // check on this very fix: asserting a JSON literal proves nothing about the
        // parser).
        let cfg = CrossSectionalMomentumConfig::from_str(
            r#"
id    = "test_schema_k_short"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
selection_mode = "long_short"
k_short = 3
"#,
        )
        .expect("k_short = 3 under long_short must parse");
        assert_eq!(cfg.k_short, 3);

        // The three axes the loader validates are now declared.
        for axis in ["direction", "score_source", "selection_mode"] {
            assert!(
                schema["properties"][axis].is_object(),
                "the schema must declare `{axis}` — it is a validated, hash-bearing field"
            );
        }
        assert!(
            schema["properties"]["score_source"]["enum"]
                .as_array()
                .is_some_and(|v| v.iter().any(|s| s == "basis_funding_residual")),
            "every ScoreSource variant must appear in the schema enum"
        );
    }
}
