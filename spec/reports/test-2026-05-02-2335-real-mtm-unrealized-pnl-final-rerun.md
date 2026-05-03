---
slug: real-mtm-unrealized-pnl
report_type: test-final-rerun
verdict: PASS
date: 2026-05-02
prior_report: spec/reports/test-2026-05-02-2113-real-mtm-unrealized-pnl-final.md
---

# Real mark-to-market unrealized P&L — final tester re-run (PASS)

## Why this report exists

The first tester run on 2026-05-02 21:13 returned `VERDICT → FAIL` on the
V6 warn-capture flake — `tracing::subscriber::with_default(...)` is a
thread-local scope; under cargo's default parallel `#[tokio::test]`
execution, the sibling `t1006_v6_footnote_present_when_miss` raced with
the capture test, lazy-initing `tracing::Dispatch`'s cache to
`NoSubscriber` before the capture layer installed. 4-run sample: 2 PASS,
2 FAIL. Production code unaffected; only the test infra was unreliable.

V1, V2, V3, V4, V5, V7, V8 were already VERIFIED in the prior report.

## Stabilization

Orchestrator spawned a focused dev to apply **option 4** (separate test
binaries — recommended over `serial_test` dev-dep, full-test combination,
or global `dispatcher::set_default`):

- DELETED: `crates/reports/tests/mark_unavailable_warns.rs`.
- NEW: `crates/reports/tests/mark_unavailable_warns_capture.rs:146` —
  `t1006_v6_mark_miss_warns_and_zeroes`. Tests warn capture + footnote
  + Ok return.
- NEW: `crates/reports/tests/mark_unavailable_warns_footnote.rs:43` —
  `t1006_v6_footnote_present_when_miss`. Tests footnote literal verbatim.

Cargo runs each `tests/*.rs` binary in its own process; each new binary
contains exactly one test, so the capture test has zero parallel
siblings sharing a `Dispatch` cache. Race eliminated by construction.

## Re-run verification

Tester re-run + orchestrator-completed stages:

| Phase | Gate | Result |
|---|---|---|
| 1a | `cargo test -p reports --test mark_unavailable_warns_capture` | 1 passed (×5 consecutive runs) |
| 1b | `cargo test -p reports --test mark_unavailable_warns_footnote` | 1 passed (×5 consecutive runs) |
| 1c | both binaries combined | 2 passed (×5 consecutive runs) |
| 1d | `cargo test -p reports` (full crate) | 0 failures |
| 1e | `cargo test --workspace --all-targets` | 0 failures across ~80 binaries |
| 2 | T1006 tick block: original + stabilization sub-block intact | VERIFIED |
| 2 | Old `mark_unavailable_warns.rs` deleted | VERIFIED |
| 2 | New file:line citations resolve | VERIFIED |
| 3 | `bash scripts/verify_anchors.sh` | `ANCHORS PASS (11 / 11)` |
| 3 | `cargo fmt --all -- --check` | clean (orchestrator-completed; tester sandbox blocked) |
| 3 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean (orchestrator-completed; tester sandbox blocked) |

## Verification matrix — final

| V | Description | Status | Evidence |
|---|---|---|---|
| V1 | open-positions reader correctness | VERIFIED | `t1005_v1_reader_emits_two_open_positions` |
| V2 | orchestrator unrealized = +200 USDT (R11.1 Ledger cell) | VERIFIED | `t1006_v2_unrealized_equals_200_usdt` |
| V3 | empty-positions backwards compat (anchors stay) | VERIFIED | `cargo test -p reports --test report_scenarios --release` (T1008) |
| V4 | reconciliation invariant Σ debits == Σ credits | VERIFIED | `t1005_v4_balance_invariant_per_txn` |
| V5 | 11-anchor regression | VERIFIED | `bash scripts/verify_anchors.sh` PASS 11/11 |
| V6 | mark-miss → ZERO + footnote + warn (post-stabilization) | VERIFIED | `t1006_v6_mark_miss_warns_and_zeroes` (5/5 stable) + `t1006_v6_footnote_present_when_miss` |
| V7 | determinism: two reads byte-identical | VERIFIED | `t1005_v7_two_reads_byte_identical` |
| V8 | perf budget < 100ms | VERIFIED | `t1007_v8_perf_smoke` 0.287ms |

## Operator-success-reports invariants

| Invariant | Status |
|---|---|
| T802 `post_fill(strategy_id)` signature unchanged | VERIFIED |
| T805 `feed_reconnect` writer callable | VERIFIED |
| T806 `agent_uptime` open/heartbeat/close wired | VERIFIED |
| T809 `KillSwitch::trip` dual-write fires | VERIFIED |
| T810 `--features in_process_cron` builds clean | VERIFIED |

## Live-cockpit-unified invariants

| Invariant | Status |
|---|---|
| T901–T912 build + tests | VERIFIED |
| `cockpit_live` kill-button stitch (kill_switch_trip_writes_both) | VERIFIED |

## Verdict

```
VERDICT → PASS
Feature: real-mtm-unrealized-pnl
Status: shipped
Anchors: 11/11
V1–V8: all VERIFIED (V6 stable post-stabilization)
T_FINAL_REAL_MTM: ticked
```

## Files referenced

- Prior report: `spec/reports/test-2026-05-02-2113-real-mtm-unrealized-pnl-final.md`
- New test binaries: `crates/reports/tests/mark_unavailable_warns_capture.rs:146`, `crates/reports/tests/mark_unavailable_warns_footnote.rs:43`
- Anchor manifest: `spec/anchors.toml` (11 entries)
- Task file: `spec/tasks/real-mtm-unrealized-pnl.md` (T_FINAL_REAL_MTM ticked)
- Feature file: `spec/features/real-mtm-unrealized-pnl.md` (status `→ shipped`)

HANDOFF → presenter (release-mode presentation for operator approval)
