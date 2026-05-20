//! H5 — `AuditEvent` enum size must stay ≤ 256 bytes (4 cache lines).
//!
//! If `std::mem::size_of::<AuditEvent>() > 256`, the developer must box the
//! offending variant (likely `Fill { fill: Box<Fill>, fees: Decimal }`).
//! This test exists so CI catches size regressions before they ship.

use audit::tick::AuditEvent;
use static_assertions::const_assert;

// H5 — AuditEvent must stay ≤ 256 bytes so broadcasting across N subscribers
// is memcpy-bound, not allocation-bound.
const_assert!(std::mem::size_of::<AuditEvent>() <= 256);

#[test]
fn audit_event_size_within_budget() {
    let sz = std::mem::size_of::<AuditEvent>();
    assert!(
        sz <= 256,
        "AuditEvent is {sz} bytes — exceeds 256-byte budget (H5). \
         Box the largest variant (likely Fill {{ fill: Box<Fill> }})."
    );
}
