//! Replay every committed `.ice` session in
//! `crates/ui/tests/recorded-sessions/` against the cockpit program
//! and assert each session's expectations hold.
//!
//! ui-session-journal-iced-tester v0.1 (T04). See
//! [`spec/v1/ui-session-journal-iced-tester/feature.md`](../../spec/v1/ui-session-journal-iced-tester/feature.md).
//!
//! ## How sessions are produced
//!
//! Operator records via:
//!
//! ```bash
//! cargo run -p ui --features live,record-tests --bin cockpit_live
//! # The recorder overlay appears automatically (compile-time auto-attach
//! # — see iced-0.14.0/src/application.rs:198). Operator clicks
//! # record / interacts / stop / export. Native file dialog (rfd) saves
//! # to `crates/ui/tests/recorded-sessions/<name>.ice`.
//! ```
//!
//! ## Replay
//!
//! Walks the directory; on each `.ice`, parses + runs against a cockpit
//! seeded by [`ui::test_support::program_from_cockpit`] (the same
//! factory the bootstrap's `visual_snapshots.rs` uses, honors H5).
//!
//! ## v0.1 status
//!
//! Ships an empty `recorded-sessions/` directory (only `.gitkeep`). The
//! test passes vacuously — exits 0 with "replayed 0 recorded
//! session(s)". Operators add `.ice` files post-ship; the test
//! automatically picks them up on next run.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use ui::fixtures::fake_cockpit_v15a_pairs_steady_state;
use ui::test_support::program_from_cockpit;

#[test]
fn replay_all_recorded_sessions() {
    let dir = format!("{}/tests/recorded-sessions", env!("CARGO_MANIFEST_DIR"));

    // Count .ice files first so the test output is meaningful even
    // when iced_test::run silently skips an empty directory.
    let entries = std::fs::read_dir(&dir).expect("recorded-sessions dir must exist");
    let ice_count = entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("ice"))
        .count();

    let cockpit = fake_cockpit_v15a_pairs_steady_state();
    let program = program_from_cockpit(cockpit);

    iced_test::run(program, &dir).unwrap_or_else(|err| {
        panic!(
            "iced_test::run failed for `{dir}` ({ice_count} .ice file(s) found):\n{err}\n\n\
             Inspect the failing session, then either:\n  \
             (a) accept the change: re-record + replace the .ice file, or\n  \
             (b) reject the change: fix the producing widget code."
        )
    });

    // Test always exits 0; eprintln! makes the no-coverage case visible
    // when running with `--nocapture`.
    eprintln!("ui-session-journal-iced-tester: replayed {ice_count} recorded session(s)");
}
