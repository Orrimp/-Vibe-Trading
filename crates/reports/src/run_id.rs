//! Run-id hash (R3.4).
//!
//! Deterministic 16-hex-char prefix of `sha256(period | 0x00 | ledger_sha
//! | 0x00 | seed_or_no_seed)`.  Same `(period, ledger_sha, seed)` →
//! same `run_id` on every machine, every locale, every wall-clock.

use sha2::{Digest, Sha256};

use crate::window::ReportWindow;

/// Compute a 16-hex-char run id for `(period, ledger_sha, seed)`.
///
/// The 64-bit prefix is sufficient for collision-resistance across a
/// single operator's report history (millions of distinct reports
/// before a 50% collision probability).  Operators see this short
/// string in front-matter and as the artifacts directory name.
///
/// # Determinism guarantee
///
/// - The same `(period, ledger_sha, seed)` tuple produces an identical
///   string on every invocation.  Asserted in
///   `crates/reports/tests/idempotent_run_id.rs` (T807 acceptance).
/// - Different `seed` values produce different run-ids.  Asserted by
///   the tests in this module.
#[must_use]
pub fn compute(period: &ReportWindow, ledger_sha: &[u8; 32], seed: Option<u64>) -> String {
    let mut h = Sha256::new();
    h.update(period.slug().as_bytes());
    h.update([0u8]); // separator
    h.update(ledger_sha);
    h.update([0u8]); // separator
    if let Some(s) = seed {
        h.update(s.to_le_bytes());
    } else {
        h.update(b"no-seed");
    }
    let out = h.finalize();
    hex::encode(&out[..8])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn fake_sha() -> [u8; 32] {
        let mut s = [0u8; 32];
        for (i, b) in s.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7);
        }
        s
    }

    #[test]
    fn t807_run_id_is_16_hex_chars() {
        let id = compute(&ReportWindow::Days7, &fake_sha(), None);
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn t807_run_id_idempotent_same_inputs() {
        let a = compute(&ReportWindow::Days7, &fake_sha(), Some(0x00C0_FFEE));
        let b = compute(&ReportWindow::Days7, &fake_sha(), Some(0x00C0_FFEE));
        assert_eq!(a, b);
    }

    #[test]
    fn t807_run_id_differs_for_different_seed() {
        let a = compute(&ReportWindow::Days7, &fake_sha(), Some(1));
        let b = compute(&ReportWindow::Days7, &fake_sha(), Some(2));
        assert_ne!(a, b);
    }

    #[test]
    fn t807_run_id_differs_for_different_period() {
        let a = compute(&ReportWindow::Days7, &fake_sha(), None);
        let b = compute(&ReportWindow::Days30, &fake_sha(), None);
        assert_ne!(a, b);
    }

    #[test]
    fn t807_run_id_differs_for_different_ledger_sha() {
        let mut sha2 = fake_sha();
        sha2[0] ^= 1;
        let a = compute(&ReportWindow::Days7, &fake_sha(), None);
        let b = compute(&ReportWindow::Days7, &sha2, None);
        assert_ne!(a, b);
    }

    #[test]
    fn t807_run_id_no_seed_distinct_from_zero_seed() {
        let none = compute(&ReportWindow::Days7, &fake_sha(), None);
        let zero = compute(&ReportWindow::Days7, &fake_sha(), Some(0));
        assert_ne!(none, zero);
    }
}
