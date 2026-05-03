#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — R2 headline render integration test.
//!
//! Asserts the rendered headline body matches a hand-computed string
//! exactly, with both percentages + dollar figures at fixed precision.

use reports::render::headline::{render, HeadlineInputs};
use rust_decimal_macros::dec;

#[test]
fn t813_r2_headline_exact_string_match() {
    let body = render(&HeadlineInputs {
        strategy_return_pct: dec!(12.34),
        strategy_return_usdt: dec!(12345.67),
        btc_return_pct: dec!(8.91),
        btc_return_usdt: dec!(8910.42),
    });
    let expected = "## Headline\n\n\
                    Strategy return: +12.34% (+$12345.67 USDT)\n\
                    BTC buy-and-hold: +8.91% (+$8910.42 USDT)\n";
    assert_eq!(body, expected);
}

#[test]
fn t813_r2_headline_zero_returns_render_with_plus_sign() {
    let body = render(&HeadlineInputs {
        strategy_return_pct: dec!(0),
        strategy_return_usdt: dec!(0),
        btc_return_pct: dec!(0),
        btc_return_usdt: dec!(0),
    });
    assert!(body.contains("Strategy return: +0.00% (+$0.00 USDT)"));
    assert!(body.contains("BTC buy-and-hold: +0.00% (+$0.00 USDT)"));
}
