//! bug-log `#95`, class-level: a declared `portfolio_exposure_cap` must be ENFORCED.
//!
//! `#95` was not one lane's bug. Six scenario lanes declared
//! `portfolio_exposure_cap: Some(..)`; the cap was read on exactly ONE of them. The
//! declaration was inert on the rest — present in the struct, printed nowhere,
//! enforced nowhere. Fixing the five instances leaves the *class* open: the next lane
//! copied from one of these inherits the field and nothing says it must wire it.
//!
//! So this test does not enumerate lanes. It SCANS the scenarios directory, finds
//! every file that declares a cap, and requires each to reach one of the two
//! enforcers that exist:
//!
//! 1. **The batch enforcer** — `risk::size_portfolio_target`, all-or-nothing over a
//!    whole target vector (R5.2). Used by the montecarlo lane.
//! 2. **The per-order entry gate** — `risk::would_breach_gross_cap`, a predicate on
//!    RESULTING exposure asked before each order. Used by the four overlay lanes plus
//!    the theta sweep, which emit one order at a time and so have no vector to batch.
//!
//! A new lane that declares a cap and calls neither FAILS here, naming itself.
//!
//! ## Why this test is source-level, and what that does and does not buy
//!
//! Running these lanes needs `--features candle,realdata`, real checkpoints and
//! minutes of wall clock; CI passes neither feature. A test that only ran under those
//! features would be invisible to CI, which is `#170`'s lesson and the reason the
//! gate's unit tests moved to `crates/risk/`. So the split is deliberate:
//!
//! - the predicate's CORRECTNESS is pinned by two-sided unit tests in
//!   `risk::portfolio::gross_cap_tests` (including the one fixture that separates its
//!   `equity > 0` conjunct from its absence);
//! - the predicate's PRESENCE at each order site is pinned here;
//! - the predicate's RUNTIME EFFECT is pinned by the `Gross-cap refusals` row in the
//!   anchored report bodies, which is a measured count, not a claim.
//!
//! This test alone proves only that the call is written. That is exactly the half
//! `#95` was missing, and it is not the whole thing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Lines whose trimmed form starts with `//` are dropped before matching, so a
/// comment that merely NAMES an enforcer cannot satisfy this test. Several of these
/// files discuss `risk::size_portfolio_target` in prose at length; without this the
/// test would pass on the prose alone.
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The directory scanned. `GROSS_CAP_SCAN_DIR` overrides it so the gate can be pointed at a
/// tree other than this checkout's.
///
/// That override exists for ADR-0038 § D6.b step 3, which requires the would-have-caught test
/// to be *run against the pre-fix code* and its literal FAIL captured. Without it the proof
/// is a one-off I describe in prose; with it anyone can re-derive it:
///
/// ```text
/// mkdir -p /tmp/prefix && git archive f54cab2a crates/backtest/src/scenarios \
///   | tar -x -C /tmp/prefix --strip-components=4
/// GROSS_CAP_SCAN_DIR=/tmp/prefix cargo test -p backtest \
///   --test gross_cap_declared_is_gross_cap_enforced
/// ```
///
/// `f54cab2a` is the commit before the gate was wired. The override does not weaken the
/// gate: unset — which is how CI runs it — it scans this checkout, and the floor assertion
/// means a wrong path fails rather than passing vacuously.
fn scenarios_dir() -> PathBuf {
    match std::env::var("GROSS_CAP_SCAN_DIR") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("src/scenarios"),
    }
}

/// Measured on 2026-11-08: 6 scenario files declare a cap. The floor is a
/// non-vacuity guard, not a target — if a refactor moves these files or the literal
/// changes shape, the scan silently finding nothing would otherwise read as "all
/// clear", which is the failure this whole bug-log entry is about.
const DECLARING_FLOOR: usize = 6;

#[test]
fn every_lane_that_declares_a_portfolio_exposure_cap_also_enforces_it() {
    let dir = scenarios_dir();
    let mut declaring: BTreeMap<String, (bool, bool)> = BTreeMap::new();

    for entry in std::fs::read_dir(&dir).expect("scenarios dir must be readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).expect("scenario file must be readable");
        let code = code_only(&raw);
        if !code.contains("portfolio_exposure_cap: Some(") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 filename")
            .to_string();
        let batch = code.contains("size_portfolio_target(");
        let gate = code.contains("would_breach_gross_cap(");
        declaring.insert(name, (batch, gate));
    }

    assert!(
        declaring.len() >= DECLARING_FLOOR,
        "found only {} cap-declaring scenario file(s) under {} — expected at least {}. \
         The scan is looking in the wrong place or the declaration changed shape; an \
         empty scan must never read as compliance (bug-log #95, #164).",
        declaring.len(),
        dir.display(),
        DECLARING_FLOOR
    );

    let unenforced: Vec<&String> = declaring
        .iter()
        .filter(|(_, (batch, gate))| !batch && !gate)
        .map(|(name, _)| name)
        .collect();

    assert!(
        unenforced.is_empty(),
        "these scenario lanes declare `portfolio_exposure_cap: Some(..)` and enforce it \
         with NEITHER mechanism: {unenforced:?}\n\n\
         A declared cap that nothing reads is bug-log #95. Wire the lane to one of:\n  \
         - `risk::size_portfolio_target(..)` if it builds a whole target vector \
         (all-or-nothing, R5.2);\n  \
         - `risk::would_breach_gross_cap(..)` as a per-order entry gate on RESULTING \
         exposure if it emits one order at a time (see \
         `scenarios/garch_vol_target_overlay.rs`).\n\n\
         Do NOT satisfy this test by deleting the declaration unless the lane genuinely \
         has no portfolio cap — that trades a false claim for a missing control."
    );

    // State what was measured, so a PASS is not silent about its own scope (#160).
    let batch_n = declaring.values().filter(|(b, _)| *b).count();
    let gate_n = declaring.values().filter(|(_, g)| *g).count();
    println!(
        "[gross-cap] {} cap-declaring lane(s) [floor {}] · {} via the batch enforcer · \
         {} via the per-order entry gate",
        declaring.len(),
        DECLARING_FLOOR,
        batch_n,
        gate_n
    );
}

/// The entry gate must be asked BEFORE the order is built, not after.
///
/// A predicate evaluated after `Order::new` has already been constructed and routed
/// would be an observation, not a gate — and it would read identically in a diff. The
/// cheapest way to pin "gate, not observation" without running the lane is the
/// textual order of the two.
#[test]
fn the_entry_gate_precedes_the_order_it_gates() {
    let dir = scenarios_dir();
    let mut checked = 0usize;

    for entry in std::fs::read_dir(&dir).expect("scenarios dir must be readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let code = code_only(&std::fs::read_to_string(&path).expect("readable"));
        let Some(gate_at) = code.find("would_breach_gross_cap(") else {
            continue;
        };
        let Some(order_at) = code.find("Order::new(") else {
            continue;
        };
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        assert!(
            gate_at < order_at,
            "{name}: `would_breach_gross_cap` is called at byte {gate_at}, AFTER the \
             first `Order::new(` at byte {order_at}. A cap checked after the order is \
             built is an observation, not a gate."
        );
        checked += 1;
    }

    assert!(
        checked >= 5,
        "only {checked} lane(s) had both a gate call and an `Order::new(` to order them \
         against — expected at least 5 (garch, patchtst, tcn, regime, threshold_sweep). \
         A scan that checks nothing passes trivially."
    );
    println!("[gross-cap] entry-gate-before-order verified on {checked} lane(s)");
}
