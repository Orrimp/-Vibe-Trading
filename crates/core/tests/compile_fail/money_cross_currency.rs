// This must NOT compile: adding Money<Usdt> to Money<Btc> is a type error.
use trading_core::{Btc, Money, Usdt};
use rust_decimal_macros::dec;

fn main() {
    let a: Money<Usdt> = Money::from_decimal(dec!(100.0));
    let b: Money<Btc> = Money::from_decimal(dec!(1.0));
    let _ = a + b; // should not compile
}
