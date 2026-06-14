//! Activity tape — the operator-facing list of in-flight background ops.
//! Source: subscribes to `agent::ActivityEvent` via `live::ActivityRecipe`.
//!
//! ## Design choices
//!
//! - `in_flight` is a `Vec<ActivityState>` (not `VecDeque`) capped at 32.
//!   The cap is small enough that O(32) linear scans and removals are
//!   negligible at the UI update rate. A `VecDeque` would buy nothing here.
//! - `ActivityTape::apply` is called from `Cockpit::update` which already
//!   runs on the iced thread (single-threaded); no synchronization needed.
//! - `red_hold_until: Option<Instant>` drives the Q5=(a) 3-second hold
//!   for failed activities. The `purge` method removes expired entries.
//!
//! cockpit-activity-status-bar v0.1.0 Wave B (T-D-N4).

use std::time::{Duration, Instant};

use agent::{ActivityEvent, ActivityId, ActivityKind, ActivityOutcome, ActivityPhase};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of simultaneous in-flight activities tracked.
/// Far exceeds the R2.2 visible cap of 3; the extra capacity absorbs bursts
/// + the Q5=(a) 3-second hold window for failed rows.
pub const ACTIVITY_TAPE_MAX_CAPACITY: usize = 32;

/// Q5=(a) hold window for failed activities: 3 seconds.
pub const ACTIVITY_FAILED_HOLD_SECS: u64 = 3;

// ── ActivityState ─────────────────────────────────────────────────────────────

/// UI-side mirror of one in-flight (or recently-failed) activity.
///
/// `Clone` required because `ActivityTape` is `Clone` (derives).
#[derive(Debug, Clone)]
pub struct ActivityState {
    /// Stable monotonic ID for correlating events. Matches `ActivityEvent::id`.
    pub id: ActivityId,
    /// Which subsystem produced this activity.
    pub kind: ActivityKind,
    /// Operator-facing label (≤ 64 chars per R1.2).
    pub label: String,
    /// Wall-clock when the `Start` event was applied (for elapsed display).
    pub started_at: Instant,
    /// Wall-clock of the most recent `Tick` or `Start` event.
    pub last_tick: Instant,
    /// Most-recent progress numerator (from `ActivityPhase::Tick`).
    pub current: Option<u64>,
    /// Most-recent progress denominator (from `ActivityPhase::Tick`).
    pub total: Option<u64>,
    /// `None` until `End` lands. `Some(outcome)` once the activity is done.
    pub outcome: Option<ActivityOutcome>,
    /// Q5=(a) — `Some(deadline)` while the row is in the 3-second red hold
    /// after a `Failed` outcome. `None` for in-flight and `Success` rows.
    pub red_hold_until: Option<Instant>,
}

impl ActivityState {
    /// `true` while the activity is still in-flight (no `End` event yet).
    #[must_use]
    pub fn is_in_flight(&self) -> bool {
        self.outcome.is_none()
    }

    /// `true` while the row is in the Q5=(a) red hold window.
    #[must_use]
    pub fn is_red_held(&self, now: Instant) -> bool {
        // clippy::map_unwrap_or fix 2026-05-26 — `is_some_and` is the idiomatic
        // shape for "Some(x) AND predicate(x)".
        self.red_hold_until.is_some_and(|deadline| now < deadline)
    }
}

// ── ActivityTape ──────────────────────────────────────────────────────────────

/// UI-side collection of in-flight and recently-failed activities.
///
/// Updated by `Cockpit::update` arms (`ActivityEventReceived`, `ActivityTapePurgeTick`).
/// Read by `widgets::activity_tape::view`.
///
/// `Clone` is derived so `Cockpit` (which derives `Clone`) can include this field.
#[derive(Debug, Default, Clone)]
pub struct ActivityTape {
    /// Ordered insertion; max `ACTIVITY_TAPE_MAX_CAPACITY` (32).
    in_flight: Vec<ActivityState>,
}

impl ActivityTape {
    /// Create a new, empty tape.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply an `ActivityEvent` to the tape.
    ///
    /// - `Start` — push a new `ActivityState`.
    /// - `Tick`  — update `current` / `last_tick` in-place (linear scan ≤ 32).
    /// - `End { Success / Cancelled }` — remove the row immediately.
    /// - `End { Failed }` — flip state to red-hold; row removed by `purge`.
    pub fn apply(&mut self, event: ActivityEvent) {
        let now = Instant::now();
        match &event.phase {
            ActivityPhase::Start { .. } => {
                // F4 mitigation: if same ID already exists, overwrite in-place
                // (defensive; should never happen in practice).
                if let Some(existing) = self.in_flight.iter_mut().find(|s| s.id == event.id) {
                    *existing = ActivityState {
                        id: event.id,
                        kind: event.kind,
                        label: event.label.clone(),
                        started_at: now,
                        last_tick: now,
                        current: None,
                        total: None,
                        outcome: None,
                        red_hold_until: None,
                    };
                    return;
                }
                // Capacity cap: if at max, discard the oldest in-flight row
                // (no red-held rows) to make room.
                if self.in_flight.len() >= ACTIVITY_TAPE_MAX_CAPACITY {
                    // Remove the oldest row that is NOT in the red-hold window.
                    if let Some(pos) = self
                        .in_flight
                        .iter()
                        .position(|s| s.red_hold_until.is_none())
                    {
                        self.in_flight.remove(pos);
                    } else {
                        // All slots are red-held — just drop the oldest.
                        self.in_flight.remove(0);
                    }
                }
                self.in_flight.push(ActivityState {
                    id: event.id,
                    kind: event.kind,
                    label: event.label,
                    started_at: now,
                    last_tick: now,
                    current: None,
                    total: None,
                    outcome: None,
                    red_hold_until: None,
                });
            }
            ActivityPhase::Tick { current, .. } => {
                if let Some(state) = self.in_flight.iter_mut().find(|s| s.id == event.id) {
                    state.current = Some(*current);
                    state.last_tick = now;
                }
            }
            ActivityPhase::End(outcome) => {
                match outcome {
                    ActivityOutcome::Success | ActivityOutcome::Cancelled => {
                        // Remove immediately (success rows are silent; cancelled
                        // rows dim but we still remove per Q5=(a) default).
                        self.in_flight.retain(|s| s.id != event.id);
                    }
                    ActivityOutcome::Failed(_) => {
                        // Q5=(a) — red hold for 3 seconds.
                        if let Some(state) = self.in_flight.iter_mut().find(|s| s.id == event.id) {
                            let hold = Duration::from_secs(ACTIVITY_FAILED_HOLD_SECS);
                            state.outcome = Some(outcome.clone());
                            state.red_hold_until = Some(state.started_at + hold);
                        }
                    }
                }
            }
        }
    }

    /// Remove rows whose red-hold window has expired.
    ///
    /// Called at ~1 Hz by the `ActivityTapePurgeTick` message arm.
    /// O(32) scan — negligible.
    pub fn purge(&mut self, now: Instant) {
        self.in_flight.retain(|s| {
            // Keep if in-flight (no end outcome yet)
            if s.outcome.is_none() {
                return true;
            }
            // Keep if still within the red-hold window (clippy::map_unwrap_or fix).
            s.red_hold_until.is_some_and(|d| now < d)
        });
    }

    /// Return the full slice of tracked activities (in-flight + red-held).
    ///
    /// The widget layer reads this and applies the R2.3 200 ms render-floor,
    /// R3.1 max-3-visible cap, and Q5=(a) colour mapping.
    #[must_use]
    pub fn visible(&self) -> &[ActivityState] {
        &self.in_flight
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::time::{Duration, Instant};

    use agent::{ActivityEvent, ActivityId, ActivityKind, ActivityOutcome, ActivityPhase};

    use super::ActivityTape;

    fn make_start(id: u64) -> ActivityEvent {
        ActivityEvent {
            id: ActivityId(id),
            kind: ActivityKind::YahooPreload,
            label: format!("label {id}"),
            phase: ActivityPhase::Start { total_units: None },
            ts_ms: 0,
        }
    }

    fn make_tick(id: u64, current: u64) -> ActivityEvent {
        ActivityEvent {
            id: ActivityId(id),
            kind: ActivityKind::YahooPreload,
            label: format!("label {id}"),
            phase: ActivityPhase::Tick {
                current,
                elapsed_ms: 100,
            },
            ts_ms: 0,
        }
    }

    fn make_end_success(id: u64) -> ActivityEvent {
        ActivityEvent {
            id: ActivityId(id),
            kind: ActivityKind::YahooPreload,
            label: format!("label {id}"),
            phase: ActivityPhase::End(ActivityOutcome::Success),
            ts_ms: 0,
        }
    }

    fn make_end_failed(id: u64) -> ActivityEvent {
        ActivityEvent {
            id: ActivityId(id),
            kind: ActivityKind::YahooPreload,
            label: format!("label {id}"),
            phase: ActivityPhase::End(ActivityOutcome::Failed("boom".to_owned())),
            ts_ms: 0,
        }
    }

    /// T-D-N4 test 1 — `Start` event appends a new row.
    #[test]
    fn applies_start_event_appends_to_in_flight() {
        let mut tape = ActivityTape::new();
        assert!(tape.visible().is_empty());

        tape.apply(make_start(1));

        let visible = tape.visible();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, ActivityId(1));
        assert!(visible[0].is_in_flight());
    }

    /// T-D-N4 test 2 — `Tick` event updates the existing row in-place (no
    /// new allocation).
    #[test]
    fn applies_tick_event_updates_in_place_no_realloc() {
        let mut tape = ActivityTape::new();
        tape.apply(make_start(2));

        let len_before = tape.visible().len();
        tape.apply(make_tick(2, 50));

        assert_eq!(tape.visible().len(), len_before, "no new row added");
        assert_eq!(tape.visible()[0].current, Some(50));
    }

    /// T-D-N4 test 3 — `End { Success }` removes the row immediately.
    #[test]
    fn applies_end_success_event_removes_immediately() {
        let mut tape = ActivityTape::new();
        tape.apply(make_start(3));
        assert_eq!(tape.visible().len(), 1);

        tape.apply(make_end_success(3));
        assert!(
            tape.visible().is_empty(),
            "success row must be removed immediately"
        );
    }

    /// T-D-N4 test 4 — `End { Failed }` puts the row into a 3-second red hold.
    #[test]
    fn applies_end_failed_event_holds_red_for_3s() {
        let mut tape = ActivityTape::new();
        tape.apply(make_start(4));
        tape.apply(make_end_failed(4));

        let state = &tape.visible()[0];
        assert!(state.outcome.is_some(), "outcome must be set");
        assert!(state.red_hold_until.is_some(), "red_hold_until must be set");

        let hold = state.red_hold_until.unwrap();
        // The hold is relative to `started_at`; check it is ≥ started_at + 3s.
        let expected_min = state.started_at + Duration::from_secs(3);
        assert!(
            hold >= expected_min,
            "hold deadline must be ≥ started_at + 3s (got {hold:?}, expected ≥ {expected_min:?})"
        );
        // Not yet expired — we are well within the 3-second window.
        assert!(
            state.is_red_held(Instant::now()),
            "row should still be red-held"
        );
    }

    /// T-D-N4 test 5 — `purge` removes rows whose hold window has passed.
    #[test]
    fn purge_removes_expired_red_holds() {
        let mut tape = ActivityTape::new();
        tape.apply(make_start(5));
        tape.apply(make_end_failed(5));
        assert_eq!(tape.visible().len(), 1, "row present during hold");

        // Simulate time well past the 3-second hold.
        let far_future = Instant::now() + Duration::from_secs(10);
        tape.purge(far_future);

        assert!(
            tape.visible().is_empty(),
            "expired red-hold row must be removed by purge"
        );
    }
}
