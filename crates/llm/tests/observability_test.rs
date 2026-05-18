//! T1909 acceptance — `emit_cache_event` integration test.
//!
//! Acceptance criterion: calling `emit_cache_event(&AgentRole::Trader,
//! 1000, 750)` increments `llm_cache_input_tokens_total{role="trader"}`
//! by 1000 and `llm_cache_hit_tokens_total{role="trader"}` by 750.
//!
//! Uses `metrics_util::debugging::DebuggingRecorder` to install a
//! process-local recorder and snapshot the counter pair afterwards. The
//! recorder is installed exactly once per process via `init_once()`.

use cost::AgentRole;
use llm::observability::emit_cache_event;
use metrics_util::debugging::{DebuggingRecorder, Snapshotter};

/// Install the recorder once per test process and return its snapshotter.
fn install_recorder() -> Snapshotter {
    let recorder = DebuggingRecorder::new();
    let snap = recorder.snapshotter();
    // `install_recorder` returns Err if a recorder is already installed.
    // For tests we tolerate either outcome because cargo runs each test
    // binary as its own process (each integration test file gets its own
    // process), so the first install per file wins cleanly.
    let _ = metrics::set_global_recorder(recorder);
    snap
}

/// Look up the cumulative value of `counter_name{role=role_label}` in the
/// snapshot. Returns 0 if the counter wasn't touched.
fn counter_value(snap: &Snapshotter, counter_name: &str, role_label: &str) -> u64 {
    use metrics_util::MetricKind;
    use metrics_util::debugging::DebugValue;

    let entries = snap.snapshot().into_vec();
    for (key, _unit, _desc, value) in entries {
        let (kind, key_ref) = key.into_parts();
        if kind != MetricKind::Counter {
            continue;
        }
        if key_ref.name() != counter_name {
            continue;
        }
        let has_role = key_ref
            .labels()
            .any(|l| l.key() == "role" && l.value() == role_label);
        if !has_role {
            continue;
        }
        match value {
            DebugValue::Counter(v) => return v,
            _ => continue,
        }
    }
    0
}

#[test]
fn t1909_emit_cache_event_increments_counter_pair() {
    let snap = install_recorder();

    emit_cache_event(&AgentRole::Trader, 1000, 750);

    let input = counter_value(&snap, "llm_cache_input_tokens_total", "trader");
    let hits = counter_value(&snap, "llm_cache_hit_tokens_total", "trader");

    assert_eq!(input, 1000, "input-tokens counter must read 1000");
    assert_eq!(hits, 750, "hit-tokens counter must read 750");
}
