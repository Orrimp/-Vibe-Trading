//! `StrategyLoadError` — parse, typecheck, and runtime errors for composed strategies (T503, T504).
//!
//! Every error carries a machine-readable `error_code` string matching the
//! table in `spec/features/v05-composed-strategies.md#design`.

use smol_str::SmolStr;
use thiserror::Error;

/// Error returned when a composed-strategy TOML file fails to load.
///
/// Each variant maps to one entry in the error-code table (see feature spec).
/// The `error_code()` method returns the short machine-readable identifier
/// carried in `StrategyLoadError` broadcast events and `strategy_events` rows.
#[derive(Debug, Error)]
pub enum StrategyLoadError {
    /// File read failed (removed or permission denied during reload).
    #[error("io_read: {0}")]
    IoRead(String),

    /// Malformed TOML syntax.
    #[error("toml_parse: {0}")]
    TomlParse(String),

    /// `deny_unknown_fields` triggered by serde.
    #[error("unknown_field: {0}")]
    UnknownField(String),

    /// The `id` field does not equal the filename stem.
    #[error("id_filename_mismatch: file stem '{stem}' != id '{id}'")]
    IdFilenameMismatch { stem: SmolStr, id: SmolStr },

    /// Rule-DSL syntax error.
    #[error("grammar_parse: {0}")]
    GrammarParse(String),

    /// Indicator name not in the supported set.
    #[error("unknown_indicator: '{0}'")]
    UnknownIndicator(SmolStr),

    /// Indicator called with wrong number of arguments.
    #[error("arity_mismatch: '{name}' expects {expected} args, got {got}")]
    ArityMismatch {
        name: SmolStr,
        expected: usize,
        got: usize,
    },

    /// Parameter reference not declared in `[params]`.
    #[error("unknown_param: '{0}'")]
    UnknownParam(SmolStr),

    /// Numeric-range violation (e.g. `fast >= slow` for MACD, RSI period < 2).
    #[error("invalid_range: {0}")]
    InvalidRange(String),

    /// `stage` is not `research` or `paper`.
    #[error("invalid_stage: '{0}'")]
    InvalidStage(SmolStr),

    /// Sizing expression is not `fixed_fraction(<f>)`.
    #[error("unsupported_sizing: '{0}'")]
    UnsupportedSizing(SmolStr),

    /// Signal string is empty or whitespace-only.
    #[error("empty_signal")]
    EmptySignal,
}

impl StrategyLoadError {
    /// Machine-readable error code matching the spec table.
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::IoRead(_) => "io_read",
            Self::TomlParse(_) => "toml_parse",
            Self::UnknownField(_) => "unknown_field",
            Self::IdFilenameMismatch { .. } => "id_filename_mismatch",
            Self::GrammarParse(_) => "grammar_parse",
            Self::UnknownIndicator(_) => "unknown_indicator",
            Self::ArityMismatch { .. } => "arity_mismatch",
            Self::UnknownParam(_) => "unknown_param",
            Self::InvalidRange(_) => "invalid_range",
            Self::InvalidStage(_) => "invalid_stage",
            Self::UnsupportedSizing(_) => "unsupported_sizing",
            Self::EmptySignal => "empty_signal",
        }
    }

    /// Short human-readable summary for the cockpit error badge.
    #[must_use]
    pub fn summary(&self) -> String {
        // Use Display via thiserror; strip the leading "code: " prefix for brevity.
        self.to_string()
    }
}
