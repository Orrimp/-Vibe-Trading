//! Compile-fail tests using `trybuild` (T03 acceptance).
//!
//! These ensure that type-level invariants from R2.4 are enforced at
//! compile time:
//! - `Money<Usdt> + Money<Btc>` does not compile.
//! - `Quantity` cannot be constructed from a negative `Decimal` without
//!   going through `Quantity::new`, which returns `Result`.
//! - `Order` fields are private and cannot be accessed directly.

#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
