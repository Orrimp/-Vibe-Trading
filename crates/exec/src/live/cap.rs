//! Exec-side notional cap mechanism (R8 / feature.md § A5 / AC-11).
//!
//! `check_notional_cap` is a **standalone pure fn** — the F1 half of the
//! AQ-4 seam.  F2's `check_armed` (F2-T3) *calls* this as condition (3) of
//! the 5-condition arming guard.
//!
//! **F1 builds + tests the cap arithmetic.  F1 NEVER decides "armed"** —
//! F1 never reads the arm-file, never checks mode, never composes the guard.
//!
//! Rule: `notional == cap` is ALLOWED (boundary inclusive); `notional > cap`
//! is REJECTED with `ExecError::CapExceeded`.  No `f64` anywhere (AC-9).

use rust_decimal::Decimal;

use crate::live::error::ExecError;

/// Check that an order notional does not exceed the configured cap.
///
/// `notional == cap` is **allowed** (boundary).
/// `notional > cap` is **rejected** with [`ExecError::CapExceeded`].
///
/// The rejected case never reaches the network — the faked transport records
/// zero requests for it (AC-11).
///
/// # Arguments
/// * `order_notional` — the order's notional in USDT (`price * qty`), `Decimal`.
/// * `cap`            — the `[live].max_notional_usdt` config value, `Decimal`.
///
/// # Errors
/// [`ExecError::CapExceeded`] when `order_notional > cap`.
pub fn check_notional_cap(order_notional: Decimal, cap: Decimal) -> Result<(), ExecError> {
    if order_notional > cap {
        Err(ExecError::CapExceeded {
            notional: order_notional,
            cap,
        })
    } else {
        Ok(()) // notional == cap is ALLOWED (boundary inclusive)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    /// AC-11 (adversarial): parametrized over (notional, cap):
    /// - `notional == cap` → ALLOWED
    /// - `notional > cap` → REJECTED (CapExceeded)
    /// - `notional < cap` → ALLOWED
    #[test]
    fn exec_side_cap_rejects_over_notional() {
        let cases: &[(Decimal, Decimal, bool)] = &[
            // (notional, cap, expected_ok)
            (dec!(100), dec!(100), true),        // boundary — allowed
            (dec!(100.01), dec!(100), false),    // just over — rejected
            (dec!(99.99), dec!(100), true),      // under — allowed
            (dec!(0), dec!(100), true),          // zero notional — allowed
            (dec!(1_000_000), dec!(100), false), // way over — rejected
        ];

        for (notional, cap, expected_ok) in cases {
            let result = check_notional_cap(*notional, *cap);
            if *expected_ok {
                assert!(
                    result.is_ok(),
                    "expected OK for notional={notional} cap={cap}, got {result:?}"
                );
            } else {
                assert!(
                    matches!(result, Err(ExecError::CapExceeded { .. })),
                    "expected CapExceeded for notional={notional} cap={cap}, got {result:?}"
                );
            }
        }
    }

    /// The CapExceeded error carries the exact notional and cap values.
    #[test]
    fn cap_exceeded_error_carries_values() {
        let result = check_notional_cap(dec!(500), dec!(200));
        match result {
            Err(ExecError::CapExceeded { notional, cap }) => {
                assert_eq!(notional, dec!(500));
                assert_eq!(cap, dec!(200));
            }
            other => panic!("expected CapExceeded, got {other:?}"),
        }
    }
}
