// This must NOT compile: Order fields are private.
// Orders can only be constructed via Order::new(...) which enforces R2.4
// invariants at construction time.
use trading_core::{Order, Quantity};
use rust_decimal_macros::dec;

fn main() {
    // Attempting to read or write a private field should fail to compile.
    let q = Quantity::new(dec!(0.1)).expect("valid");
    // `qty` is a private field — this line must not compile.
    let order: Order = Order { qty: q, ..todo!() };
    let _ = order;
}
