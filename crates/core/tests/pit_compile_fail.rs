//! Compile-fail proof that `core::pit` prevents look-ahead at the type level
//! (M-TEST-2, AC2, ADR-0058 D1).
//!
//! The trybuild fixture `compile_fail/pit_no_public_constructor.rs` proves that:
//!   - `AsOf` has no public constructor (private fields `as_of_ts` / `value`).
//!   - A consumer CANNOT fabricate an `AsOf` with `ts > query`.
//!   - Look-ahead is a compile error, not a runtime bug.
//!
//! Removing the guard — by making `AsOf` fields `pub` — would make this test
//! FAIL, proving the guarantee is structural (a regression makes it visible).

#[test]
fn pit_look_ahead_is_a_compile_error() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/pit_no_public_constructor.rs");
}
