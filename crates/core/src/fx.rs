//! EUR→USDT budget-conversion primitives (F7 — ADR-0065).
//!
//! A **configurable static FX rate** converts the operator's euro budget into
//! the USDT that the engine sizes against. This module is the single source of
//! truth for that one multiply — see the grep-guard test in
//! `crates/core/tests/eur_fx_conversion_applied.rs`.
//!
//! ## Design invariants (ADR-0065)
//!
//! - The ONLY EUR→USDT arithmetic in the codebase lives in
//!   [`FxRate::convert_eur_to_usdt`]. A second multiply elsewhere would
//!   reintroduce drift. The grep guard in the day-1 gate pins this.
//! - [`BudgetConversion`] computes `usdt` **once** at construction. The engine
//!   reads `conversion.usdt()`; the display reads `.usdt()` / `.eur()` /
//!   `.rate()`. They cannot drift because they share one converted value.
//! - `rate` is **private** with a checked constructor — a constructed [`FxRate`]
//!   is always valid (`rate > 0`).
//! - No `Timestamp::now()` / `SystemTime::now()` — `as_of` is a **label**
//!   (operator-supplied string), not a clock read. Determinism is preserved.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use thiserror::Error;

use crate::asset::Usdt;
use crate::money::Money;

// ── Default rate ─────────────────────────────────────────────────────────────

/// Default EUR/USD rate (USDT per 1 EUR). The operator can override via
/// `[advisor] eur_usd_rate = <value>` in `config/agent.toml`. This constant
/// is also the fallback the future v0.3 live-rate path (ADR-0065 § D6) would
/// use when the live fetch fails.
pub const DEFAULT_EUR_USD_RATE: Decimal = dec!(1.08);

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors from [`FxRate::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FxRateError {
    /// The rate must be strictly positive (> 0). A zero or negative rate
    /// would produce a zero or negative USDT budget, which is not sensible.
    #[error("FX rate must be > 0, got {0}")]
    NonPositiveRate(Decimal),
}

// ── FxRate ────────────────────────────────────────────────────────────────────

/// A validated EUR/USD exchange rate with provenance metadata.
///
/// The `rate` field is **private** — use the checked [`FxRate::new`] ctor or the
/// convenience [`FxRate::config`]. A constructed `FxRate` is always valid.
///
/// `source` and `as_of` are **labels** (e.g. `"config"`, `"2026-06-22"`) — they
/// are not clock reads, so determinism is fully preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxRate {
    /// USDT per 1 EUR (e.g. `1.08`). Private: use accessors.
    rate: Decimal,
    /// Provenance label, e.g. `"config"` or `"ecb"`.
    source: SmolStr,
    /// As-of label, e.g. `"2026-06-22"`. A string label, not a clock read.
    as_of: SmolStr,
}

impl FxRate {
    /// Checked constructor. Rejects `rate <= 0` with [`FxRateError::NonPositiveRate`].
    ///
    /// # Errors
    /// Returns [`FxRateError::NonPositiveRate`] if `rate <= 0`.
    pub fn new(
        rate: Decimal,
        source: impl Into<SmolStr>,
        as_of: impl Into<SmolStr>,
    ) -> Result<Self, FxRateError> {
        if rate <= Decimal::ZERO {
            return Err(FxRateError::NonPositiveRate(rate));
        }
        Ok(Self {
            rate,
            source: source.into(),
            as_of: as_of.into(),
        })
    }

    /// Infallible convenience ctor for a config-sourced rate.
    ///
    /// Stamps `source = "config"` and `as_of = ""`. The rate must be > 0.
    /// Callers always pass `DEFAULT_EUR_USD_RATE` or a config value validated
    /// at the config-load boundary; a zero/negative rate would be a programming
    /// error, not a runtime condition, so we use `debug_assert!` for that check.
    ///
    /// # Panics
    /// Only in debug builds when `rate <= 0` (programming error).
    #[must_use]
    pub fn config(rate: Decimal) -> Self {
        debug_assert!(
            rate > Decimal::ZERO,
            "FxRate::config called with non-positive rate {rate}"
        );
        // Construct directly to avoid `expect()` in library code.
        // The invariant (rate > 0) is a precondition, not a runtime check.
        Self {
            rate,
            source: SmolStr::new_static("config"),
            as_of: SmolStr::new_static(""),
        }
    }

    /// The EUR/USD rate (USDT per 1 EUR).
    #[must_use]
    pub fn rate(&self) -> Decimal {
        self.rate
    }

    /// Provenance label (e.g. `"config"`).
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// As-of label (e.g. `"2026-06-22"`). An empty string when not supplied.
    #[must_use]
    pub fn as_of(&self) -> &str {
        &self.as_of
    }

    /// Convert a euro `Decimal` budget to `Money<Usdt>`.
    ///
    /// **This is the ONLY EUR→USDT arithmetic in the codebase.**
    /// `usdt = eur × rate`. A grep guard in `crates/core/tests/
    /// eur_fx_conversion_applied.rs` pins this invariant.
    ///
    /// Both the engine (`ForwardRunConfig.budget`) and the display read from
    /// the same [`BudgetConversion`] — they share one converted value and
    /// cannot drift.
    #[must_use]
    pub fn convert_eur_to_usdt(&self, eur: Decimal) -> Money<Usdt> {
        Money::<Usdt>::from_decimal(eur * self.rate)
    }
}

// ── BudgetConversion ──────────────────────────────────────────────────────────

/// The single source of truth for one budget-conversion boundary.
///
/// Computes `usdt = rate.convert_eur_to_usdt(eur)` **once** in the constructor.
/// The **engine** reads `conversion.usdt()` (→ `ForwardRunConfig.budget`);
/// the **display** reads `conversion.usdt()` / `conversion.eur()` /
/// `conversion.rate()`. One converted value, no drift
/// (the ADR-0062 / F6 anti-drift discipline).
///
/// The display figure the operator reads is definitionally the `Money<Usdt>`
/// F4 [`crate::risk`] caps against.
#[derive(Debug, Clone)]
pub struct BudgetConversion {
    /// The operator's euro budget (labelled input scalar, not a `Money<Eur>`).
    eur: Decimal,
    /// The FX rate used for this conversion (carries provenance).
    rate: FxRate,
    /// The converted USDT budget — computed once, shared by engine + display.
    usdt: Money<Usdt>,
}

impl BudgetConversion {
    /// Construct a conversion, computing `usdt` once via
    /// [`FxRate::convert_eur_to_usdt`].
    #[must_use]
    pub fn new(eur: Decimal, rate: FxRate) -> Self {
        let usdt = rate.convert_eur_to_usdt(eur);
        Self { eur, rate, usdt }
    }

    /// The operator's euro budget (the raw input scalar).
    #[must_use]
    pub fn eur(&self) -> Decimal {
        self.eur
    }

    /// The converted USDT budget — the value the engine sizes against.
    ///
    /// Both the engine and the display call this method on the same
    /// `BudgetConversion`; they cannot diverge.
    #[must_use]
    pub fn usdt(&self) -> Money<Usdt> {
        self.usdt
    }

    /// The FX rate used for this conversion (gives access to `.rate()`,
    /// `.source()`, `.as_of()` for the display layer).
    #[must_use]
    pub fn rate(&self) -> &FxRate {
        &self.rate
    }
}

// ── FxNote ────────────────────────────────────────────────────────────────────

/// A lightweight display carrier for the FX provenance metadata.
///
/// Extracted from a [`BudgetConversion`] so the Live screen and Forward-plan
/// can display "€X ≈ $Y (at R EUR/USD, source as-of)" without holding a full
/// [`BudgetConversion`]. This is a **`core`-typed** value so it crosses the
/// UI boundary without adding a new `ui` dependency edge.
///
/// Built via [`BudgetConversion::fx_note`].
#[derive(Debug, Clone, PartialEq)]
pub struct FxNote {
    /// The operator's euro budget (label input scalar).
    pub eur: Decimal,
    /// The converted USDT amount (what F4 caps against).
    pub usdt: Decimal,
    /// The EUR/USD rate used.
    pub rate: Decimal,
    /// Provenance label (e.g. `"config"`).
    pub source: SmolStr,
    /// As-of label (e.g. `"2026-06-22"`).
    pub as_of: SmolStr,
}

impl BudgetConversion {
    /// Extract a lightweight [`FxNote`] for use in the display layer.
    ///
    /// The Live screen and Forward-plan use this to render the "€X ≈ $Y
    /// (at R EUR/USD, source as-of)" FX note without holding the full
    /// `BudgetConversion`.
    #[must_use]
    pub fn fx_note(&self) -> FxNote {
        FxNote {
            eur: self.eur,
            usdt: self.usdt.amount(),
            rate: self.rate.rate(),
            source: SmolStr::new(self.rate.source()),
            as_of: SmolStr::new(self.rate.as_of()),
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn default_rate_is_positive() {
        assert!(DEFAULT_EUR_USD_RATE > Decimal::ZERO);
    }

    #[test]
    fn fx_rate_new_rejects_zero() {
        assert!(FxRate::new(Decimal::ZERO, "test", "").is_err());
    }

    #[test]
    fn fx_rate_new_rejects_negative() {
        assert!(FxRate::new(dec!(-1), "test", "").is_err());
    }

    #[test]
    fn fx_rate_new_accepts_positive() {
        let r = FxRate::new(dec!(1.08), "config", "2026-06-22").unwrap();
        assert_eq!(r.rate(), dec!(1.08));
        assert_eq!(r.source(), "config");
        assert_eq!(r.as_of(), "2026-06-22");
    }

    #[test]
    fn budget_conversion_computes_once() {
        let rate = FxRate::config(dec!(1.08));
        let conv = BudgetConversion::new(dec!(200), rate);
        assert_eq!(conv.eur(), dec!(200));
        assert_eq!(conv.usdt().amount(), dec!(216));
        assert_eq!(conv.rate().rate(), dec!(1.08));
    }

    #[test]
    fn unit_rate_is_identity() {
        let rate = FxRate::config(dec!(1.0));
        let conv = BudgetConversion::new(dec!(200), rate);
        assert_eq!(conv.usdt().amount(), dec!(200));
    }
}
