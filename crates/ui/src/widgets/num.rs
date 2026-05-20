//! Numeric formatting helpers.
//!
//! All numbers rendered in the cockpit flow through these helpers so
//! alignment, thousands separators, and sign rendering are consistent.
//! Widgets never hand-format with `format!("{:.2}", x)` — it's a smell.

use iced::Color;
use rust_decimal::Decimal;

use crate::strings::{MINUS_SIGN_LITERAL, UNIT_BTC, UNIT_USDT};
use crate::theme::{ThemeMode, color};

/// Insert thousands separators into the integer part of a decimal string.
/// Works on a pre-stringified `Decimal` to avoid reintroducing `f64`.
///
/// `"12345.67"` → `"12,345.67"`.
/// `"-12345"` → `"-12,345"`.
#[must_use]
pub fn with_thousands_sep(s: &str) -> String {
    let (sign, rest) = if let Some(stripped) = s.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", s)
    };
    let (int_part, frac_part) = match rest.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };
    let mut out = String::with_capacity(int_part.len() + int_part.len() / 3);
    let bytes = int_part.as_bytes();
    for (i, c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c as char);
    }
    if let Some(f) = frac_part {
        out.push('.');
        out.push_str(f);
    }
    format!("{sign}{out}")
}

/// Pad a rounded decimal string out to exactly `places` fractional digits.
/// `"12"` with places=2 → `"12.00"`; `"12.5"` with places=2 → `"12.50"`.
fn pad_fractional(raw: &str, places: usize) -> String {
    match raw.split_once('.') {
        Some((int, frac)) => {
            if frac.len() >= places {
                format!("{int}.{}", &frac[..places])
            } else {
                format!("{int}.{frac}{}", "0".repeat(places - frac.len()))
            }
        }
        None => format!("{raw}.{}", "0".repeat(places)),
    }
}

/// Format a USDT amount to two decimal places with thousands separators,
/// suffixed with `" USDT"`.
#[must_use]
pub fn fmt_usdt(d: Decimal) -> String {
    let rounded = d.round_dp(2);
    let padded = pad_fractional(&rounded.to_string(), 2);
    format!("{} {}", with_thousands_sep(&padded), UNIT_USDT)
}

/// Format a price (Decimal) to two decimal places with thousands separators.
/// No currency suffix — prices render next to symbol columns that already
/// imply the quote asset.
#[must_use]
pub fn fmt_price(d: Decimal) -> String {
    let rounded = d.round_dp(2);
    let padded = pad_fractional(&rounded.to_string(), 2);
    with_thousands_sep(&padded)
}

/// Format a base-asset quantity (e.g. BTC) to eight decimal places stripped
/// of trailing zeros. Thousand separators are not applied — fractional
/// quantities never need them and integer-sized crypto is rare enough.
#[must_use]
pub fn fmt_qty(d: Decimal) -> String {
    let rounded = d.round_dp(8);
    let raw = rounded.to_string();
    if raw.contains('.') {
        let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
        if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        raw
    }
}

/// Format a qty with a BTC suffix — used in the P&L card and exposure line.
#[must_use]
pub fn fmt_qty_btc(d: Decimal) -> String {
    format!("{} {}", fmt_qty(d), UNIT_BTC)
}

/// Format a percentage (input already in percent, i.e. `12.5` = 12.5%) to
/// two decimal places with a trailing `%`.
#[must_use]
pub fn fmt_pct(d: Decimal) -> String {
    let rounded = d.round_dp(2);
    let padded = pad_fractional(&rounded.to_string(), 2);
    format!("{padded}%")
}

/// Phase 4 (T1805) — format a percentage with sentiment colouring.
/// Returns the formatted string + the colour the caller paints. The
/// minus-sign prefix is the unicode `MINUS_SIGN_LITERAL` so the
/// rendering matches the Lumen reference component (R2.4).
///
/// `> 0` → `UP_500`; `< 0` → `DOWN_500`; `0` → `FG_1`. The string
/// always carries an explicit `MINUS_SIGN_LITERAL` prefix on
/// negatives.
#[must_use]
pub fn format_pct_sentiment(value: Decimal, mode: ThemeMode) -> (String, Color) {
    let rounded = value.round_dp(2);
    let abs_padded = pad_fractional(&rounded.abs().to_string(), 2);
    let (text, c) = if value.is_zero() {
        (format!("{abs_padded}%"), color::FG_1.current(mode))
    } else if value.is_sign_positive() {
        (format!("{abs_padded}%"), color::UP_500.current(mode))
    } else {
        (
            format!("{MINUS_SIGN_LITERAL}{abs_padded}%"),
            color::DOWN_500.current(mode),
        )
    };
    (text, c)
}

/// Phase 4 (T1805) — format a max-DD percentage. Always `DOWN_500`
/// with the unicode minus prefix (R2.4 — drawdown is always negative
/// or zero by construction).
#[must_use]
pub fn format_pct_max_dd(value: Decimal, mode: ThemeMode) -> (String, Color) {
    let abs = value.abs().round_dp(2);
    let padded = pad_fractional(&abs.to_string(), 2);
    let text = if abs.is_zero() {
        format!("{padded}%")
    } else {
        format!("{MINUS_SIGN_LITERAL}{padded}%")
    };
    (text, color::DOWN_500.current(mode))
}

/// Phase 4 (T1805) — format the Sharpe ratio to 4 decimal places.
#[must_use]
pub fn format_sharpe(value: Decimal) -> String {
    let rounded = value.round_dp(4);
    let raw = rounded.to_string();
    let padded = pad_fractional(&raw, 4);
    // Replace ASCII '-' with the unicode minus literal so all
    // negative-sign rendering is consistent with the strip's other
    // cards.
    if let Some(stripped) = padded.strip_prefix('-') {
        format!("{MINUS_SIGN_LITERAL}{stripped}")
    } else {
        padded
    }
}

/// Phase 4 (T1805) — format a `u64` count with thousands-separators.
#[must_use]
pub fn format_count(value: u64) -> String {
    with_thousands_sep(&value.to_string())
}

/// Format a signed percentage delta with explicit `+` / `-` sign.
///
/// Used by `run_delta_badge` for max-drawdown and other pct deltas (T-D-N13).
/// Input is a decimal fraction (e.g. `0.024` = 2.4 %). Output: `"+2.40%"` / `"-2.40%"`.
#[must_use]
pub fn fmt_pct_signed(d: Decimal) -> String {
    // Convert fraction to percentage points then round.
    let pct = d * Decimal::ONE_HUNDRED;
    let rounded = pct.round_dp(2);
    let padded = pad_fractional(&rounded.abs().to_string(), 2);
    if d.is_zero() {
        format!("{padded}%")
    } else if d.is_sign_positive() {
        format!("+{padded}%")
    } else {
        format!("-{padded}%")
    }
}

/// Format a signed delta in USDT with explicit `+` / `-` sign.
#[must_use]
pub fn fmt_usdt_signed(d: Decimal) -> String {
    let base = fmt_usdt(d.abs());
    if d.is_zero() {
        base
    } else if d.is_sign_positive() {
        format!("+{base}")
    } else {
        format!("-{base}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn thousands_sep_basic() {
        assert_eq!(with_thousands_sep("1234567"), "1,234,567");
        assert_eq!(with_thousands_sep("12345.67"), "12,345.67");
        assert_eq!(with_thousands_sep("-12345.67"), "-12,345.67");
        assert_eq!(with_thousands_sep("999"), "999");
        assert_eq!(with_thousands_sep("1000"), "1,000");
    }

    #[test]
    fn usdt_format_two_decimals() {
        assert_eq!(fmt_usdt(dec!(100000)), "100,000.00 USDT");
        assert_eq!(fmt_usdt(dec!(1.5)), "1.50 USDT");
        assert_eq!(fmt_usdt(dec!(-1234.567)), "-1,234.57 USDT");
    }

    #[test]
    fn qty_strips_trailing_zeros() {
        assert_eq!(fmt_qty(dec!(0.10000000)), "0.1");
        assert_eq!(fmt_qty(dec!(1)), "1");
        assert_eq!(fmt_qty(dec!(0.00000001)), "0.00000001");
    }

    #[test]
    fn pct_formats_cleanly() {
        assert_eq!(fmt_pct(dec!(11.10)), "11.10%");
        assert_eq!(fmt_pct(dec!(0)), "0.00%");
        assert_eq!(fmt_pct(dec!(-5.5)), "-5.50%");
    }

    #[test]
    fn signed_usdt_explicit_sign() {
        assert_eq!(fmt_usdt_signed(dec!(100)), "+100.00 USDT");
        assert_eq!(fmt_usdt_signed(dec!(-100)), "-100.00 USDT");
        assert_eq!(fmt_usdt_signed(dec!(0)), "0.00 USDT");
    }
}
