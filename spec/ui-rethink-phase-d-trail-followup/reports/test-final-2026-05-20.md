---
title: Test Report — M-FINAL Tester Sweep
feature: ui-rethink-phase-d-trail-followup
run_id: 2026-05-20-1600-UTC
commit: f5f180df0fe1ad0c5e7dbed88c975586858b8065
agent: tester
verdict: PASS-WITH-DEFERRED
---

# Test Report — ui-rethink-phase-d-trail-followup — 2026-05-20

## 1. Scope

- **Feature / change under test:** UI rethink Phase D+ — Trail follow-up (v0.1.1 patch).
  Closes 5 deferred items from Phase D v0.1.0:
  - R1 — iced `Subscription` bridge wiring `TrailMirrorTick` into `Cockpit::subscription`
    via `TrailMirrorRecipe` + `From<reflection::trail_mirror::TrailMirrorTick>` conversion;
    UI-local wrapper types (`TrailMirrorUiTick`, `TrailStageUi`, `ReconstructedTrailUi`);
    `Message::TrailMirrorTick(TrailMirrorUiTick)` payload upgrade + update arm; 2 new unit
    tests; `TrailScreenState.reconstructed_trail` + `pending_trail_audit_id` fields.
  - R2 — 3 new snapshot baselines (`trail__steady_state`, `trail__side_drawer_open`,
    `live__recent_activity_with_chevron`).
  - R3 — H5 backfill-latency bench (`crates/reflection/benches/trail_mirror.rs`; p99 < 50 ms).
  - R4 — `scripts/bench_idle_cpu.sh` tooling (H3 idle-CPU floor; execution deferred — see § 11).
  - R5 — K7 paper-mode `ForecastEmitted` counter probe (Q1=YES; execution deferred — see § 11).
- **Spec refs:** `spec/ui-rethink-phase-d-trail-followup/feature.md` (R1-R5),
  `spec/ui-rethink-phase-d-trail-followup/tasks.md` (T-D-N1..N19, M-FINAL T-F1..T-F9),
  `spec/ui-rethink-phase-d-trail-followup/decomp.md`
- **Commit SHA:** `f5f180df0fe1ad0c5e7dbed88c975586858b8065` (uncommitted working tree — orchestrator
  commits after PASS)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.4.0, arm64

---

## 2. Static Analysis (T-F1)

| Check               | Result | Notes                                                                      |
|---------------------|--------|----------------------------------------------------------------------------|
| `cargo fmt --check` | PASS   | Exit 0, no diff output                                                     |
| `cargo clippy --workspace -- -D warnings` | PASS | Exit 0, `Finished dev profile [unoptimized + debuginfo]` |
| `cargo clippy -p ui --features live -- -D warnings` | PRE-EXISTING FAIL | 13 errors in `crates/ui/src/live.rs:159-428` (see note) |
| `cargo audit`       | N/A    | `cargo-audit` not installed in this environment (pre-existing gap)        |
| `cargo deny`        | N/A    | Not in scope for this run                                                  |

**Invocations:**

```
cargo fmt --check                                    # EXIT:0
cargo clippy --workspace -- -D warnings              # EXIT:0  Finished dev profile [unoptimized + debuginfo] target(s) in 1.16s
```

**Pre-existing `--features live` lint clarification:**

`cargo clippy -p ui --features live -- -D warnings` produces 13 `needless_pass_by_value` errors
at `crates/ui/src/live.rs` lines 159, 182, 212, 232, 256, 280, 321, 342, 365, 396, 428, and one
`calls to push immediately after creation` error. These are all on pre-existing functions in the
file. The developer's diff (`git diff HEAD -- crates/ui/src/live.rs`) shows that the developer
only **appended** new code starting after line 511 (the Phase D+ `TrailMirrorRecipe` additions).
Lines 159–428 were not touched by this wave. These lints are pre-existing since Phase D v0.1.0
and are a v0.1.2 hygiene-patch candidate. They do NOT block v0.1.1 because the non-feature-gated
default-feature clippy gate (T-F1 contract) exits 0.

Routing: v0.1.2 hygiene patch — developer (lint fix for `crates/ui/src/live.rs`).

---

## 3. Unit & Integration Tests (T-F2)

**Command:** `cargo test --workspace --lib`
**Exit code:** 0

Per-crate summary (extracted from `cargo test --workspace --lib` run; matches developer-reported
profile from prior runs):

| Crate          | Passed | Failed | Ignored |
|----------------|-------:|-------:|--------:|
| `agent`        |     52 |      0 |       0 |
| `audit`        |     36 |      0 |       0 |
| `backtest`     |     13 |      0 |       1 |
| `cost`         |      9 |      0 |       0 |
| `data`         |     47 |      0 |       1 |
| `exec`         |      6 |      0 |       0 |
| `features`     |     55 |      0 |       0 |
| `forecast`     |     52 |      0 |       0 |
| `llm`          |     84 |      0 |       0 |
| `models`       |      0 |      0 |       0 |
| `reflection`   |     11 |      0 |       0 |
| `replay_cache` |      8 |      0 |       0 |
| `reports`      |    103 |      0 |       0 |
| `risk`         |     10 |      0 |       0 |
| `strategy`     |     85 |      0 |       0 |
| `trading_core` |     72 |      0 |       0 |
| `ui`           |    296 |      0 |       0 |
| **Total**      |**939** |  **0** |   **2** |

**Verified output (tail of run):**

```
test result: ok. 296 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
```

**Total workspace count:** `cargo test --workspace --lib | grep "^test result" | awk '{sum += $4} END {print sum}'` → **939**

Baseline was 937 (Phase D v0.1.0). Phase D+ adds 2 new unit tests (T-D-N3/N4):
- `ui::state::tests::open_trail_for_sets_pending_audit_id`
- `ui::state::tests::trail_mirror_tick_updated_clears_reconstructed_trail`

939 ≥ 937 baseline: NON-REGRESSION CONFIRMED.

**Failing Tests:** _none_

---

## 4. Property / Fuzz Tests

| Suite                              | Cases | Shrunk failures | Notes                                                   |
|------------------------------------|------:|----------------:|---------------------------------------------------------|
| `features::sma::proptests`         |   256 |               0 | carry-forward from Phase D; `cargo test --workspace --lib` |
| `features::ema::proptests`         |   256 |               0 | carry-forward                                           |
| `features::rsi::proptests`         |   512 |               0 | carry-forward                                           |
| `features::bbands::proptests`      |   512 |               0 | carry-forward                                           |
| `features::macd::proptests`        |   256 |               0 | carry-forward                                           |
| `strategy::composed::proptests`    |  1000 |               0 | carry-forward                                           |
| `strategy::lab::state::proptests`  |   256 |               0 | carry-forward                                           |
| `ui::layout_invariants (proptest)` |   256 |               0 | 6 widgets × 256 cases (see T-F5 §7 below)              |

No proptest failures under `cargo test --workspace --lib`. Phase D+ adds no new proptest suites.

---

## 5. Backtest Results

_n/a_ — Phase D+ adds a Subscription bridge, snapshot baselines, and a bench fixture. No strategy
logic was changed. The 22 body-SHA-256 anchor gate (T-F4) is the backtest regression gate for this
feature. See § 6 Anchor Gate.

---

## 6. Anchor Gate (T-F4) — NON-NEGOTIABLE

### Pre-sweep run (before any spec file edits)

**Command:** `bash scripts/verify_anchors.sh`
**Exit code:** 0

```
PASS  btc-2023-1m-sma-cross
PASS  btc-2023-1m-sma-baseline-refresh
PASS  btc-2023-1m-macd-trend
PASS  btc-2023-1m-rsi-reversion
PASS  btc-2023-1m-bbands-mean-revert
PASS  top10-2023-1h-momentum
PASS  top10-2024-h1-momentum
PASS  pairs-2023-zscore-mr
PASS  pairs-2024-h1-zscore-mr
PASS  report-sample-7d
PASS  report-sample-90d
PASS  top10-2023-fy-tcn-overlay
PASS  top10-2024-fy-tcn-overlay
PASS  top10-2023-fy-tcn-overlay-weights
PASS  top10-2024-fy-tcn-overlay-weights
PASS  top10-2023-fy-tcn-overlay-realdata
PASS  top10-2024-fy-tcn-overlay-realdata
PASS  top10-2023-fy-tcn-overlay-weights-realdata
PASS  top10-2024-fy-tcn-overlay-weights-realdata
PASS  forecast-distribution-bs1-realdata
PASS  forecast-distribution-bs2-realdata
PASS  sharpe-comparison-realdata
---
ANCHORS PASS  (22 / 22)
```

### Post-sweep run (after trace.toml + tasks.md tester edits)

**Command:** `bash scripts/verify_anchors.sh`
**Exit code:** 0

```
PASS  ...  (all 22 PASS — identical SHAs as above)
---
ANCHORS PASS  (22 / 22)
```

H2 anchor-preservation claim confirmed. Phase D+ additive construction (Subscription bridge +
3 NEW snapshot baselines + 1 NEW bench) preserved all 22 anchored bodies byte-identical.

---

## 7. Cockpit Smoke / Layout Invariants (T-F5)

**Command:** `cargo test -p ui --test layout_invariants`
**Exit code:** 0

```
running 6 tests
test focus_ring_layout_never_zero_dim ... ok
test journal_transaction_modal_layout_never_zero_dim ... ok
test kpi_strip_layout_never_zero_dim ... ok
test chart_view_layout_never_zero_dim ... ok
test strategies_id_cell_layout_never_zero_dim ... ok
test positions_view_layout_never_zero_dim ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 59.01s
```

6/6 PASS. M1-C cockpit-smoke proxy satisfied. R7.3 ("0 panic lines") confirmed: 6 widget layout
invariants pass under 256 proptest cases each, no zero-dimension panics.

---

## 8. Snapshot Baseline Tests (T-F3)

**Determinism requirement:** run twice with `--test-threads=1`; both runs PASS and pixel-identical.

### Run 1

**Command:** `cargo test -p ui --test visual_snapshots -- --test-threads=1`
**Exit code:** 0

```
running 7 tests
test charts_screen_dark_floor ... ok
test charts_screen_dark_operator ... ok
test charts_screen_dark_typical ... ok
test live__recent_activity_with_chevron ... ok
test trail__side_drawer_open ... ok
test trail__steady_state ... ok
test visual_diff_helper_writes_diff_png_on_mismatch ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.43s
```

### Run 2

**Command:** `cargo test -p ui --test visual_snapshots -- --test-threads=1`
**Exit code:** 0

```
running 7 tests
test charts_screen_dark_floor ... ok
test charts_screen_dark_operator ... ok
test charts_screen_dark_typical ... ok
test live__recent_activity_with_chevron ... ok
test trail__side_drawer_open ... ok
test trail__steady_state ... ok
test visual_diff_helper_writes_diff_png_on_mismatch ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.84s
```

Both runs PASS. All 3 new baselines (`trail__steady_state`, `trail__side_drawer_open`,
`live__recent_activity_with_chevron`) deterministic across 2 consecutive runs. Confirmed at
`crates/ui/tests/visual-baselines/` (3 new PNG files present on disk).

H4 (snapshot determinism N≥2) holds. Key determinism note from developer: `trail__steady_state`
fixture seeds `AuditScreenState::Ready` (not `Loading`) to prevent `ThrottledSpinner`
non-determinism — correct mitigation per K4.

---

## 9. Benchmarks (T-F8 — H5 backfill-latency)

**Command:** `cargo bench -p reflection --bench trail_mirror`
**Exit code:** 0

```
Benchmarking trail_mirror/trail_mirror_open
Benchmarking trail_mirror/trail_mirror_open: Warming up for 3.0000 s
Benchmarking trail_mirror/trail_mirror_open: Collecting 50 samples in estimated 10.008 s (1.1M iterations)
trail_mirror/trail_mirror_open
                        time:   [9.0291 µs 9.1934 µs 9.3605 µs]
                        change: [-0.1118% +2.3454% +4.7076%] (p = 0.06 > 0.05)
                        No change in performance detected.

trail_mirror_open p99 = 0.021 ms
```

**Result:** p99 = **0.021 ms** << 50 ms gate (H5 acceptance threshold).

H5 (backfill p99 < 50 ms at 10⁵ rows) NOT FALSIFIED. The criterion harness using 10⁵ synthetic
rows seeded `ChaCha20Rng::seed_from_u64(0xD005_D5C0_FFEE_BC01)`, 100 random `Open` requests,
LRU cleared between iterations to force the SQL path.

Previous baseline: none (first bench run for this path). Developer reported 0.020 ms; tester
independently observed 0.021 ms — within run-to-run noise.

K3 note: `:memory:` SQLite is faster than on-disk WAL-mode. The p99 = 0.021 ms is the in-memory
fixture value. If on-disk production performance is slower, that is a v0.1.2 follow-up scope
(R3.5 / R6.3 pre-fetch redesign). No action required for v0.1.1.

---

## 10. Idle-CPU Bench Script Self-Test (T-D-N11 verification)

**Command:** `bash scripts/bench_idle_cpu.sh $$ 3`
**Exit code:** 0

```
0 0.0
1 0.0
2 0.0
```

Script writes 3 lines of `<i> <cpu_pct>` to stdout and exits 0. Script functional.
Full T-F6 idle-CPU floor gate (60 s sustained cockpit_live run, N=3 runs, median ≤ 13.6%) is
deferred — see § 12 Deferred.

---

## 11. Spec-Lint Gate

**Command:** `python3.14 scripts/spec_lint.py`
**Exit code:** 2 (non-zero — pre-existing baseline violations only)

**Result:** `spec-lint: FAIL (87 violations in 2 categories)`

| Category            | Baseline (Phase D tester report 2026-05-20) | Current (Phase D+ sweep 2026-05-20) | Delta         |
|---------------------|---------------------------------------------|-------------------------------------|---------------|
| dead-link           | 81                                          | 81                                  | 0 (unchanged) |
| trace-broken-path   | 6                                           | 6                                   | 0 (unchanged) |
| **TOTAL**           | **87**                                      | **87**                              | **0**         |

**spec-lint: PASS** (no new regressions vs predecessor tester report baseline).

**Tester correction applied:** The developer filled the `REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001`
`tests` array in `spec/trace.toml` using `::` module-path notation (e.g.
`crates/ui/src/state.rs::tests::open_trail_for_sets_pending_audit_id`). The spec-lint parser
treats `tests` entries as file paths, so `::test::name` suffixes caused 6 spurious
trace-broken-path violations (93 total initially). The tester corrected these to valid file paths
(`crates/ui/src/state.rs`, `crates/ui/tests/visual_snapshots.rs`,
`crates/reflection/benches/trail_mirror.rs`) with inline comments naming the test functions.
Count returned to baseline 87. This is a format hygiene correction, not a spec modification.

**Pre-existing spec debt (6 trace-broken-path violations, NOT blocking — same as Phase D baseline):**
- `REQ-V25A-PATCHTST-001`: anchors `top10-2023-fy-patchtst-overlay`, `top10-2024-fy-patchtst-overlay`
  not in `anchors.toml` — future model not yet built.
- `REQ-V25B-TRANSFORMER-001`: anchors `top10-2023-fy-transformer-overlay`,
  `top10-2024-fy-transformer-overlay` not in `anchors.toml` — future model.
- `REQ-V26-BAKEOFF-001`: anchors `top10-2023-fy-bakeoff-winner`, `top10-2024-fy-bakeoff-winner`
  not in `anchors.toml` — future bake-off.

Routing for pre-existing debt: architect (trace.toml cleanup when those features land).

---

## 12. Environment / Infrastructure Issues

- `cargo audit` not installed — pre-existing gap, not introduced by Phase D+.
- `cockpit_live` requires a display server + live data feed for T-F6 and T-F7 (see § 13 Deferred).
- Gnuplot not found (criterion used plotters backend for bench charts) — cosmetic only; bench
  numbers unaffected. Exit code was 0.

---

## 13. Deferred to Follow-up

### T-F6 — Idle-CPU floor (H3 gate) — DEFERRED (sandbox display server constraint)

**Gate:** `scripts/bench_idle_cpu.sh <cockpit_live_pid> 60` × N=3 runs; median ≤ 13.6%.

**Rationale:** Running `cockpit_live` with the Phase D+ Subscription bridge armed requires a
macOS display server. This sandbox environment does not have a window display available for the
iced binary to open. The `scripts/bench_idle_cpu.sh` script itself is verified functional
(self-test with `$$ 3` exits 0, writes 3 lines). The architectural argument for H3 holds:
the `TrailMirrorRecipe` adds one `BroadcastStream` polled at broadcast cadence alongside the
existing `BusRecipe` + `ServerTimeRecipe`; idle-CPU impact is structurally O(broadcast rate),
which is near-zero at idle (no trail ticks arrive when the cockpit is idle). The Phase D baseline
was 13.1%; the 0.5% headroom is conservative given the stream will have zero messages at idle.

**Falsification condition if run:** median(N=3 60s samples) > 13.6% → Q5 fallback (a) 4 Hz
throttle per R1.5 operator-decided default.

**Routing if deferred reaches next operator cycle:** developer (run on operator workstation with
cockpit_live binary + display, measure and verify H3).

### T-F7 — K7 paper-mode ForecastEmitted counter probe — DEFERRED (infrastructure + display server)

**Gate (Q1=YES):** Run `cockpit_live` binary with paper-mode feed + BS-1 checkpoint; assert
`reflection_audit_tick_seen_total{variant="ForecastEmitted"} ≥ 1` after 60 s.

**Verbatim cargo command (from T-D-N17):**

```
RUST_LOG=info,reflection=debug \
  cargo run --features live,forecast-audit-tick --bin cockpit_live -- \
    --config config/agent.toml --mode paper &
COCKPIT_PID=$!
sleep 60
curl -s localhost:9100/metrics \
  | grep '^reflection_audit_tick_seen_total{variant="ForecastEmitted"}'
kill $COCKPIT_PID
```

**Rationale:** Two constraints block execution:
1. `cockpit_live` requires a display server (iced GUI binary).
2. The BS-1 checkpoint file (`config/agent.toml` → `bs1_checkpoint_path`) must be present on the
   deployment workstation. The anchor `forecast-distribution-bs1-realdata` was produced on the
   operator workstation where the checkpoint exists; this sandbox cannot access that artifact.

**Structural verification (carrying forward from Phase D v0.1.0 tester report § 3.3 T-F7):**
- Both emit sites confirmed: `crates/forecast/src/tcn.rs:861-879` (cache-hit) and `:985-1007`
  (post-inference).
- Production builder context: `crates/strategy/src/tcn_overlay_momentum.rs:417-420,434-437`
  (`with_forecast_context("tcn_overlay_momentum_bs1", "MULTI")`).
- `build_registry_with_ledger` at `crates/agent/src/runtime.rs:163-220` wires the ledger through
  at paper-mode startup.
- ForecastEmitted serde round-trip passes (carry-forward from Phase D:
  `cargo test -p audit --test tick_serde_roundtrip` → `forecast_emitted_roundtrip ... ok`).
- Missing checkpoint → graceful skip via `tracing::warn!` (R5.4 confirmed).

**Classification:** Infrastructure-blocked (same class as Phase D v0.1.0 deferred T-F7).
If Q1 was YES (operator-decided), this is now a deployment-cycle gate, not a code-correctness
gate. The wiring is complete and correct.

---

## 14. Trace.toml Anchors Column

Per tester ownership rules: the `anchors` column for `REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001` is
`[]` — Phase D+ adds zero new anchor scenarios (H2 carry-forward). The 22 existing anchors serve
as the non-regression gate. Tester notes this as verified with `ANCHORS PASS (22/22)` pre- and
post-sweep.

The `tests` field was corrected from `::` notation to valid file paths (see § 11). State updated
from `in-progress (developer)` to `in-progress (tester sweep PASS-WITH-DEFERRED 2026-05-20)`.

---

## 15. Verdict

**`PASS-WITH-DEFERRED`**

All hard gates passed independently:

| Gate   | Command / Evidence                                                         | Result                  |
|--------|----------------------------------------------------------------------------|-------------------------|
| T-F1   | `cargo fmt --check` / `cargo clippy --workspace -- -D warnings`            | PASS (EXIT:0 / EXIT:0)  |
| T-F2   | `cargo test --workspace --lib`                                             | 939 passed, 0 failed    |
| T-F3   | `cargo test -p ui --test visual_snapshots -- --test-threads=1` (×2 runs)  | 7/7 PASS both runs      |
| T-F4   | `bash scripts/verify_anchors.sh` (pre- AND post-sweep)                    | ANCHORS PASS (22/22)    |
| T-F5   | `cargo test -p ui --test layout_invariants`                                | 6/6 PASS                |
| T-F8   | `cargo bench -p reflection --bench trail_mirror`                           | p99 = 0.021 ms < 50 ms  |
| spec-lint | `python3.14 scripts/spec_lint.py`                                       | 87 violations, 0 new regressions vs predecessor baseline |

Deferred gates (T-F6, T-F7) are infrastructure-blocked (require display server + cockpit_live
binary + BS-1 checkpoint). They are not code-correctness regressions. Deferred is the honest
classification per the gate spec: "if your sandbox lacks the checkpoint files or display server
for cockpit_live, deferral is honest."

The pre-existing `--features live` clippy lint (`live.rs:159-428`, 13 `needless_pass_by_value`
errors) is a v0.1.2 hygiene-patch candidate, not a v0.1.1 regression. The developer did not
introduce these lints (they were added in lines the developer did not touch).

---

## 16. Routing

`VERDICT → PASS-WITH-DEFERRED` — all hard gates green; T-F6/T-F7 deferred with rationale.
Feature is ready for the presenter step and operator approval. Deferred items (idle-CPU floor,
paper-mode K7 counter) are infrastructure-blocked deployment-cycle gates.
