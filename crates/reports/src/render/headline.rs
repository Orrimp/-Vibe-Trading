//! R2 — Headline (strategy return + BTC buy-and-hold baseline).
//!
//! Pure renderer: takes pre-computed dollar / percentage values as
//! `Decimal` and emits the two-line headline section.  No I/O, no
//! clock access — the orchestrator pulls cash + opening balance + BTC
//! close prices and hands the resulting `Decimal`s here.

use std::fmt::Write;

use rust_decimal::Decimal;

/// Inputs for the R2 headline section.
///
/// All amounts are in USDT.  Percentages are unitless `Decimal`s
/// (e.g. `dec!(12.34)` for 12.34 %).
#[derive(Debug, Clone)]
pub struct HeadlineInputs {
    /// Strategy return as a percentage (`+12.34` for +12.34 %).
    pub strategy_return_pct: Decimal,
    /// Strategy return absolute (USDT).
    pub strategy_return_usdt: Decimal,
    /// BTC buy-and-hold return as a percentage.
    pub btc_return_pct: Decimal,
    /// BTC buy-and-hold return absolute (USDT).
    pub btc_return_usdt: Decimal,
}

/// Render the R2 headline section per R2.3.
///
/// Format:
///
/// ```text
/// ## Headline
///
/// Strategy return: +12.34% (+$12345.67 USDT)
/// BTC buy-and-hold: +8.91% (+$8910.42 USDT)
/// ```
///
/// Pure over `inputs` — same inputs produce byte-identical output.
#[must_use]
pub fn render(inputs: &HeadlineInputs) -> String {
    let mut out = String::with_capacity(192);
    out.push_str("## Headline\n\n");
    let _ = writeln!(
        out,
        "Strategy return: {} ({} USDT)",
        format_pct(inputs.strategy_return_pct),
        format_usdt(inputs.strategy_return_usdt),
    );
    let _ = writeln!(
        out,
        "BTC buy-and-hold: {} ({} USDT)",
        format_pct(inputs.btc_return_pct),
        format_usdt(inputs.btc_return_usdt),
    );
    out
}

/// Format a `Decimal` to exactly two decimal places (preserves trailing
/// zeros so `dec!(-3.5)` renders as `-3.50`).
fn fmt_2dp(d: Decimal) -> String {
    // `Decimal::round_dp(2)` does not pad trailing zeros — it returns
    // a `Decimal` whose `Display` uses the underlying scale.  We
    // explicitly normalise to scale 2 by re-multiplying by 1.00.
    let two_scale = (d.round_dp(2) * rust_decimal_macros::dec!(1.00)).round_dp(2);
    format!("{two_scale:.2}")
}

/// Format a percentage as `+12.34%` / `-5.67%` to two decimal places.
fn format_pct(pct: Decimal) -> String {
    let s = fmt_2dp(pct);
    if pct >= Decimal::ZERO {
        format!("+{s}%")
    } else {
        // `s` already carries the `-` sign.
        format!("{s}%")
    }
}

/// Format a USDT amount as `+$12345.67` / `-$5.00` (cents-precise).
fn format_usdt(amount: Decimal) -> String {
    let s = fmt_2dp(amount);
    if amount >= Decimal::ZERO {
        format!("+${s}")
    } else {
        // Strip the leading `-` so the sign sits before the dollar
        // glyph: `-$500.00` instead of `$-500.00`.
        let abs = s.strip_prefix('-').unwrap_or(&s);
        format!("-${abs}")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t813_headline_two_lines_per_r23() {
        let inp = HeadlineInputs {
            strategy_return_pct: dec!(12.34),
            strategy_return_usdt: dec!(12345.67),
            btc_return_pct: dec!(8.91),
            btc_return_usdt: dec!(8910.42),
        };
        let body = render(&inp);
        assert!(body.contains("## Headline"));
        assert!(body.contains("Strategy return: +12.34% (+$12345.67 USDT)"));
        assert!(body.contains("BTC buy-and-hold: +8.91% (+$8910.42 USDT)"));
    }

    #[test]
    fn t813_headline_negative_returns_format_correctly() {
        let inp = HeadlineInputs {
            strategy_return_pct: dec!(-3.5),
            strategy_return_usdt: dec!(-500.00),
            btc_return_pct: dec!(-1.0),
            btc_return_usdt: dec!(-100.00),
        };
        let body = render(&inp);
        assert!(body.contains("Strategy return: -3.50% (-$500.00 USDT)"));
        assert!(body.contains("BTC buy-and-hold: -1.00% (-$100.00 USDT)"));
    }

    #[test]
    fn t813_headline_byte_stable_across_runs() {
        let inp = HeadlineInputs {
            strategy_return_pct: dec!(0.5),
            strategy_return_usdt: dec!(50.00),
            btc_return_pct: dec!(0.25),
            btc_return_usdt: dec!(25.00),
        };
        let a = render(&inp);
        let b = render(&inp);
        assert_eq!(a, b);
    }
}
