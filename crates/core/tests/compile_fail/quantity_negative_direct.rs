// This must NOT compile: Quantity cannot be constructed directly with a negative
// value — the inner Decimal field is private, so the only construction path is
// Quantity::new(d) which returns Result<Quantity, QtyError>.
use rust_decimal_macros::dec;
use trading_core::Quantity;

fn main() {
    // Attempting to use the tuple-struct literal syntax is a compile error
    // because the inner field is private.
    let _q = Quantity(dec!(-1));
}
