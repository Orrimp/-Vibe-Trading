// This must NOT compile: AsOf<T> has no public constructor.
// A consumer cannot fabricate an AsOf with ts > query.
// Proving that look-ahead is unrepresentable (ADR-0058 D1, AC2).
use rust_decimal_macros::dec;
use trading_core::pit::{AsOf, TimestampMs};

fn main() {
    // Attempt to construct AsOf directly with struct literal (fields are private).
    let _: AsOf<rust_decimal::Decimal> = AsOf {
        as_of_ts: TimestampMs(99_999),
        value: dec!(0.5),
    };
}
