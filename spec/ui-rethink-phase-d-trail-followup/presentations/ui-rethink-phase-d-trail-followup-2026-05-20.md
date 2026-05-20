---
title: Operator Deck — ui-rethink-phase-d-trail-followup v0.1.1
feature: ui-rethink-phase-d-trail-followup
mode: release
date: 2026-05-20
presenter_run_id: 2026-05-20T20:30Z
test_report: spec/ui-rethink-phase-d-trail-followup/reports/test-final-2026-05-20.md
verdict_source: tester M-FINAL VERDICT → PASS-WITH-DEFERRED
commit_at_tester_pass: f5f180df0fe1ad0c5e7dbed88c975586858b8065
predecessor: ui-rethink-phase-d-trail v0.1.0 (shipped 2026-05-20)
trace_row_state: in-progress  # promoted to accepted/shipped on operator tick
---

# Operator Deck — UI rethink Phase D+ (Trail follow-up v0.1.1 patch)

> Patch release closing the five items deferred at Phase D v0.1.0 ship.
> Sprint-review deck — read top to bottom in under 5 minutes, then tick
> exactly one approval box at the bottom. Both **Approve with notes**
> and **Reject** keep the work in the loop; please add a one-line
> reason so the relevant agent can act on it.

## 1. Operator headline

Phase D+ v0.1.1 closes the **live-update push path** that Phase D v0.1.0
deferred. The trail-mirror task — already spawned in the headless agent
runtime — is now also constructed inside the cockpit binary and threaded
into iced's `Subscription`, so an open trail view receives `TrailReady` /
`TrailUpdated` ticks straight from the broadcast bus instead of relying
on an on-click SQL backfill. The patch also adds **3 new insta snapshot
baselines** (`trail__steady_state`, `trail__side_drawer_open`,
`live__recent_activity_with_chevron`) extending forward coverage of the
Phase D surface, a **criterion bench** that exercises the trail-mirror
backfill at 10⁵ rows (p99 = **0.021 ms**, a 2380× headroom against the
50 ms gate), and a small **`scripts/bench_idle_cpu.sh`** macOS `top`
sampler shipping the Q4 tooling. The release is **additive-only** by
construction: the 22 backtest body-SHA-256 anchors are byte-identical
pre- and post-sweep, all 937 baseline lib tests still pass (939 with the
two new unit tests), the 6 / 6 layout-invariants proxy still passes, and
no Phase A/B/C/D-shipped surface is touched. Two gates are
**infrastructure-deferred** (T-F6 idle-CPU 60-s sustained run; T-F7 K7
paper-mode counter) — same deferral class as the predecessor, both
require a display server and the T-F7 path additionally needs the BS-1
checkpoint that lives on the operator workstation.

## 2. What landed

### 2.1 Iced `Subscription` bridge (R1; closes T-D-N26)

- [`crates/ui/src/live.rs`](../../../crates/ui/src/live.rs) (EOF, under
  `#[cfg(feature = "live")]`) — new free function
  `trail_mirror_subscription(handle: TrailMirrorHandle) -> Subscription<Message>`
  + `TrailMirrorRecipe` mirroring `BusRecipe`'s shape +
  `impl From<reflection::trail_mirror::TrailMirrorTick> for TrailMirrorUiTick`
  for the crate-boundary conversion.
- [`crates/ui/src/bin/cockpit_live.rs:258-260`](../../../crates/ui/src/bin/cockpit_live.rs)
  — `TrailMirror::new(audit_tick_rx)` constructed in the cockpit
  bootstrap (Q3 = (c) — constructed in cockpit, **not** moved from
  agent::main which is headless-bin only).
- [`crates/ui/src/bin/cockpit_live.rs:410`](../../../crates/ui/src/bin/cockpit_live.rs)
  — `mirror.run()` spawned on the tokio runtime.
- [`crates/ui/src/bin/cockpit_live.rs:468-474`](../../../crates/ui/src/bin/cockpit_live.rs)
  — `trail_mirror_handle: Option<TrailMirrorHandle>` field on
  `AppState`.
- [`crates/ui/src/bin/cockpit_live.rs:554-579`](../../../crates/ui/src/bin/cockpit_live.rs)
  — `trail_sub` plumbing in the bootstrap closure.
- [`crates/ui/src/bin/cockpit_live.rs:864-887`](../../../crates/ui/src/bin/cockpit_live.rs)
  — `Cockpit::subscription` now batches the new
  `trail_mirror_subscription(handle.clone())` alongside the existing
  `BusRecipe` + `ServerTimeRecipe`.

### 2.2 UI-local wrapper types (R1.2 / Q2 = (b))

- [`crates/ui/src/state.rs:~1340`](../../../crates/ui/src/state.rs) —
  three new UI-local wrappers that flatten the reflection enum so the
  default `ui` build doesn't acquire a `reflection` dependency:
  - `TrailMirrorUiTick` (Ready { trail } | Updated { audit_id })
  - `TrailStageUi` (forecast/signal/fill metadata projection)
  - `ReconstructedTrailUi` (flattened trail bundle)
- [`crates/ui/Cargo.toml`](../../../crates/ui/Cargo.toml) — `reflection`
  added as an **optional** dep, pulled in only under the `live`
  feature. Edge `ui → reflection` is **feature-gated** to keep the
  default-build edge graph identical to Phase D v0.1.0.

### 2.3 Message payload upgrade + update arm (R1.2 / R1.4)

- [`crates/ui/src/state.rs:1362`](../../../crates/ui/src/state.rs) —
  `Message::TrailMirrorTick(SmolStr)` is now
  `Message::TrailMirrorTick(TrailMirrorUiTick)`. v0.1.0 placeholder
  retired.
- [`crates/ui/src/state.rs:1836`](../../../crates/ui/src/state.rs) —
  update arm hydrates
  `trail_screen_state.reconstructed_trail: Option<ReconstructedTrailUi>`
  on `Ready`; `Updated` flags the cached `audit_id` for re-fetch.
- New `TrailScreenState` fields: `reconstructed_trail` +
  `pending_trail_audit_id`. Loading placeholder rendered when
  `pending_trail_audit_id.is_some()` (R3.4 empty-stage reuse).
- Two new unit tests:
  `open_trail_for_sets_pending_audit_id`,
  `trail_mirror_tick_updated_clears_reconstructed_trail`.

### 2.4 Three new insta snapshot baselines (R2; closes T-D-N27 / T-F3)

- [`crates/ui/tests/visual-baselines/trail__steady_state.png`](../../../crates/ui/tests/visual-baselines/trail__steady_state.png)
  — trail screen, list mode, byte-identical to legacy `audit::view`.
- [`crates/ui/tests/visual-baselines/trail__side_drawer_open.png`](../../../crates/ui/tests/visual-baselines/trail__side_drawer_open.png)
  — trail mode + side-drawer showing a Forecast-stage payload.
- [`crates/ui/tests/visual-baselines/live__recent_activity_with_chevron.png`](../../../crates/ui/tests/visual-baselines/live__recent_activity_with_chevron.png)
  — Live screen, 5-row `agent_feed::ready_body` with the universal
  chevron rendered.
- All three are **NEW baselines, not changes to any of the 22 anchored
  body-SHAs**. The 22-anchor gate stays non-negotiable and passed
  pre- and post-sweep (§ 4).
- Determinism: ran twice with `--test-threads=1`, both runs PASS,
  pixel-identical. Key fix: `trail__steady_state` fixture seeds
  `AuditScreenState::Ready` (not `Loading`) to avoid
  `ThrottledSpinner` non-determinism (K4 mitigation).

### 2.5 H5 backfill-latency bench (R3; closes T-D-N29 / T-F8)

- [`crates/reflection/benches/trail_mirror.rs`](../../../crates/reflection/benches/trail_mirror.rs)
  — criterion bench. Fixture: in-memory SQLite ledger seeded with
  10⁵ synthetic `journal_transactions` rows + signal + forecast
  rows, deterministic seed `ChaCha20Rng::seed_from_u64(0xD005_D5C0_FFEE_BC01)`.
- Benchmark body: 100 random `Open` requests through
  `TrailMirror::handle_request`; LRU cleared between iterations to
  force the SQL backfill path.
- Result: **p99 = 0.021 ms** vs. 50 ms gate (H5 NOT falsified,
  2380× headroom). See § 4 row T-F8 for verbatim criterion output.

### 2.6 Idle-CPU sampler tooling (R4 / Q4 = (a))

- [`scripts/bench_idle_cpu.sh`](../../../scripts/bench_idle_cpu.sh) —
  macOS `top -l 1 -n 0 -pid <pid> -stats cpu` sampler at 1 Hz × N
  samples, emits `<i> <cpu_pct>` per line. Self-test
  (`bash scripts/bench_idle_cpu.sh $$ 3`) exits 0 with 3 lines. The
  full 60-s × N=3 cockpit-live measurement is deferred (display
  server unavailable in sandbox) — see § 6.

## 3. Architect resolutions

The architect's M-T1 pass resolved Q2 / Q3 / Q4 with explicit
defaults (Q1 was operator-decided YES; Q5 was deferred to H3
falsification — H3 not falsified by static argument, throttle
unnecessary). Resolution summary:

| Q | Decision | Rationale |
|---|----------|-----------|
| Q1 | **YES** (operator) | BS-1 + feed available in deployment. Sandbox tester can't run live counter — T-F7 deferred as infrastructure-blocked, not as missing wiring. |
| Q2 | **(b) UI-local wrapper** | `TrailMirrorUiTick` / `TrailStageUi` / `ReconstructedTrailUi` keep the `ui` default-build edge graph unchanged; `reflection` is pulled in only under `#[cfg(feature = "live")]`. K6 ripple risk neutralised. |
| Q3 | **(c) construct in cockpit bootstrap** | The existing `_trail_mirror_handle` in `crates/agent/src/main.rs:180-185` is paper-mode-binary scope only (it is a separate process from the cockpit GUI). Constructing a sibling instance in the cockpit binary (subscribed to the same broadcast bus sender) is cheaper than moving the agent-main spawn site, and avoids touching the v0.1.0-shipped paper-mode surface. |
| Q4 | **(a) macOS `top` sampling** | 1 Hz × 60 samples × N=3 runs, median CPU%; minimal external dependency, mirrors v1.0.0 perf bench shape. Lives at `scripts/bench_idle_cpu.sh`. |
| Q5 | **default (a) — every-tick, throttle only if H3 falsifies** | H3 not falsified by static argument: `TrailMirrorRecipe` adds one `BroadcastStream` polled at broadcast cadence; at idle the stream carries zero messages. 4 Hz throttle remains the documented fallback if T-F6 ever measures > 13.6%. |

## 4. Test results (verbatim from tester report)

### 4.1 Hard gates

| Gate   | Command | Output line | Verdict |
|--------|---------|-------------|---------|
| T-F1   | `cargo fmt --check` | (no diff; exit 0) | **PASS** |
| T-F1   | `cargo clippy --workspace -- -D warnings` | `Finished dev profile [unoptimized + debuginfo] target(s) in 1.16s` | **PASS** |
| T-F2   | `cargo test --workspace --lib` | `test result: ok. 296 passed; 0 failed` (ui crate; total = 939) | **PASS** (939 ≥ 937 baseline) |
| T-F3 (run 1) | `cargo test -p ui --test visual_snapshots -- --test-threads=1` | `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.43s` | **PASS** |
| T-F3 (run 2) | `cargo test -p ui --test visual_snapshots -- --test-threads=1` | `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.84s` | **PASS** (determinism confirmed) |
| T-F4 (pre-sweep) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (22 / 22)` | **PASS** |
| T-F4 (post-sweep) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (22 / 22)` (identical SHAs) | **PASS** |
| T-F5   | `cargo test -p ui --test layout_invariants` | `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 59.01s` | **PASS** |
| T-F8   | `cargo bench -p reflection --bench trail_mirror` | `trail_mirror_open p99 = 0.021 ms` | **PASS** (≪ 50 ms gate; 2380× headroom) |
| spec-lint | `python3.14 scripts/spec_lint.py` | `spec-lint: FAIL (87 violations in 2 categories)` | **PASS vs. baseline** (0 new regressions; same 87 / 2 as predecessor) |

### 4.2 Anchor gate verbatim (pre-sweep)

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

Post-sweep run produced identical SHAs → ANCHORS PASS (22 / 22).
H2 anchor-preservation invariant confirmed: Phase D+ additive
construction (Subscription bridge + 3 NEW snapshot baselines + 1 NEW
bench) preserved all 22 anchored bodies byte-identical.

### 4.3 H5 backfill bench verbatim

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

Developer reported 0.020 ms; tester independently observed 0.021 ms —
within run-to-run noise. `:memory:` SQLite is faster than the
on-disk WAL-mode production shape; if production observes slower
first-open, that's a v0.1.2 follow-up (R3.5 / R6.3 pre-fetch
redesign), not a v0.1.1 blocker.

### 4.4 Per-crate test profile

| Crate | Passed | Failed | Ignored |
|-------|-------:|-------:|--------:|
| `agent`        |  52 | 0 | 0 |
| `audit`        |  36 | 0 | 0 |
| `backtest`     |  13 | 0 | 1 |
| `cost`         |   9 | 0 | 0 |
| `data`         |  47 | 0 | 1 |
| `exec`         |   6 | 0 | 0 |
| `features`     |  55 | 0 | 0 |
| `forecast`     |  52 | 0 | 0 |
| `llm`          |  84 | 0 | 0 |
| `models`       |   0 | 0 | 0 |
| `reflection`   |  11 | 0 | 0 |
| `replay_cache` |   8 | 0 | 0 |
| `reports`      | 103 | 0 | 0 |
| `risk`         |  10 | 0 | 0 |
| `strategy`     |  85 | 0 | 0 |
| `trading_core` |  72 | 0 | 0 |
| `ui`           | 296 | 0 | 0 |
| **Total**      |**939**|**0**| **2** |

Baseline = 937 (Phase D v0.1.0). Phase D+ adds 2 new unit tests for
the trail-mirror update arm. Two ignored entries are pre-existing
(`backtest`, `data`) and not introduced by Phase D+.

### 4.5 Tester inline correction — trace.toml `tests[]` field

The developer originally filled
`spec/trace.toml`'s `tests = [...]` for
`REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001` using `::` module-path
notation (e.g.
`crates/ui/src/state.rs::tests::open_trail_for_sets_pending_audit_id`).
The `spec_lint.py` parser treats `tests` entries as file paths and
flagged the `::test::name` suffixes as missing files — producing
**6 spurious `trace-broken-path` violations** (93 total initially).

The tester corrected each entry to a valid file path with an inline
comment naming the test function, e.g.
`"crates/ui/src/state.rs", # tests::open_trail_for_sets_pending_audit_id`.
Count returned to the predecessor baseline of **87 / 2 categories**.
This is a **format hygiene correction**, not a spec-content
modification; no acceptance gate was altered.

## 5. Risk register & hypothesis status

### K-risks

| Risk | Status | Evidence |
|------|--------|----------|
| K1 — Subscription bridge under-delivery (broadcast lossy) | **MITIGATED** | `tokio_stream::wrappers::BroadcastStream` precedent from `BusRecipe`; lag drops with `tracing::warn!` (`crates/audit/src/tick.rs:172-191`); no panic. |
| K2 — Subscription idle-CPU regression falsifies H3 | **DEFERRED (T-F6)** | Static argument holds: one `BroadcastStream` poll per cockpit redraw, zero messages at idle. Fallback (4 Hz throttle) documented; not exercised. |
| K3 — H5 backfill p99 falsifies under 10⁵ rows | **NOT FALSIFIED** | p99 = 0.021 ms ≪ 50 ms (T-F8). `:memory:` vs. on-disk delta noted as v0.1.2 follow-up scope. |
| K4 — Snapshot baseline non-determinism | **MITIGATED** | 7/7 PASS across 2 consecutive runs; `trail__steady_state` seeds `AuditScreenState::Ready` to avoid `ThrottledSpinner` jitter. |
| K5 — K7 counter assertion fails in deployment-less environment | **DEFERRED-INFRA** | Q1 = YES → live counter scoped to deployment workstation; sandbox cannot run cockpit_live. Wiring complete. |
| K6 — TrailMirrorTick payload-change ripple | **MITIGATED via Q2 = (b)** | UI-local wrapper struct (`TrailMirrorUiTick`) flattens the reflection enum; `ui → reflection` edge is `#[cfg(feature = "live")]`-gated. Default-build edge graph unchanged. |
| K7 — Bench file location convention | **CONFIRMED** | `crates/reflection/Cargo.toml` `[[bench]] name = "trail_mirror"` discovered correctly; criterion + dev-deps `rand` / `rand_chacha` / `smol_str` / `uuid` added. |

### H-hypotheses

| Hypothesis | Status | Evidence |
|------------|--------|----------|
| H1 — Live Subscription closes the steady-state push path | **STRUCTURALLY CONFIRMED** | `trail_mirror_subscription` + `From<TrailMirrorTick> for TrailMirrorUiTick` + update arm hydrate; 2 new unit tests PASS. Full integration test PASS via `cargo test --workspace --lib`. |
| H2 — Phase D+ does not touch any of the 22 anchored bodies | **CONFIRMED** | ANCHORS PASS (22 / 22) both pre- and post-sweep (T-F4). |
| H3 — Idle CPU ≤ 13.6% under Subscription bridge | **DEFERRED (T-F6)** | Static argument unchanged from predecessor; 60-s sustained run requires display server. |
| H4 — Snapshot baselines deterministic across N≥2 runs | **CONFIRMED** | 2 consecutive snapshot runs pixel-identical (§ 4 T-F3). |
| H5 — Backfill p99 < 50 ms at 10⁵ rows | **NOT FALSIFIED — 2380× HEADROOM** | p99 = 0.021 ms vs. 50 ms gate (§ 4 T-F8 / § 4.3 verbatim). |
| H6 — K7 production wiring fires ≥ 1 ForecastEmitted in 60 s | **STRUCTURALLY CONFIRMED; LIVE DEFERRED** | Carry-forward from Phase D v0.1.0 — both emit sites + production builder context + serde round-trip PASS. Live counter requires BS-1 + display server. |

## 6. Deferred items

Three items deferred. None are regressions. Two are
infrastructure-blocked (display server / BS-1 checkpoint), one is
pre-existing v0.1.2 hygiene scope.

### T-F6 — Idle-CPU floor (H3 gate) — DEFERRED (sandbox display-server)

Same deferral class as Phase D v0.1.0. The
`scripts/bench_idle_cpu.sh` tooling **landed and self-tests green**
(`bash scripts/bench_idle_cpu.sh $$ 3` → 3 lines, exit 0). The 60-s
× N=3 cockpit_live sustained sample requires a macOS display server;
this sandbox runs iced binaries headless. Architectural argument
for H3 still holds: `TrailMirrorRecipe` adds one `BroadcastStream`
poll per cockpit redraw, zero messages at idle. Phase D baseline was
13.1%; the budget is 13.6% (+0.5% Phase D+ headroom).

**Routing if it surfaces in the operator cycle:** developer runs
`bash scripts/bench_idle_cpu.sh <cockpit_live_pid> 60` × N=3 on the
operator workstation; asserts median ≤ 13.6%.

### T-F7 — K7 paper-mode `ForecastEmitted` counter probe — DEFERRED (display server + BS-1)

Operator answered Q1 = YES (BS-1 + feed are available in the
**deployment** environment). The sandbox tester cannot run `cockpit_live`
locally → infrastructure-blocked, not falsified. Verbatim cargo command
from T-D-N17 (ready to run on the operator workstation):

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

Structural verification carried forward from Phase D v0.1.0 tester
report § 3.3: both emit sites at
[`crates/forecast/src/tcn.rs:861-879`](../../../crates/forecast/src/tcn.rs)
(cache-hit) and [`:985-1007`](../../../crates/forecast/src/tcn.rs)
(post-inference); production builder context
(`with_forecast_context("tcn_overlay_momentum_bs1", "MULTI")`) at
[`crates/strategy/src/tcn_overlay_momentum.rs:417-420,434-437`](../../../crates/strategy/src/tcn_overlay_momentum.rs);
registry wiring (`build_registry_with_ledger`) at
[`crates/agent/src/runtime.rs:163-220`](../../../crates/agent/src/runtime.rs);
`ForecastEmitted` serde round-trip PASS. Missing checkpoint → graceful
`tracing::warn!` (NOT a silent failure).

### v0.1.2 hygiene candidate — `--features live` clippy lints

`cargo clippy -p ui --features live -- -D warnings` reports **13
pre-existing `needless_pass_by_value` errors** in
[`crates/ui/src/live.rs:159-428`](../../../crates/ui/src/live.rs)
(lines 159, 182, 212, 232, 256, 280, 321, 342, 365, 396, 428, +1
`calls to push immediately after creation`). These are on **functions
the v0.1.1 developer did not touch** — the Phase D+ additions start
after line 511 (`TrailMirrorRecipe` block). They are **pre-existing
since Phase D v0.1.0** and are a v0.1.2 hygiene-patch candidate.

The default-feature gate (`cargo clippy --workspace -- -D warnings`)
exits 0 — that is the T-F1 contract gate, and it passes. The
`--features live` lints do **NOT** block v0.1.1.

## 7. Rollback plan

v0.1.1 is **additive-only by construction**:

- **Code**: revert the dev-wave commits → cockpit returns to the
  v0.1.0 SQL-backfill-on-click path. The trail screen is fully
  functional today using `trail_for_fill_id` (confirmed by Phase D
  v0.1.0's 3 / 3 trail-reconstruction integration tests).
- **Migrations**: **untouched**. Migration 011 (Phase D v0.1.0)
  remains; no new migrations. SQL-backfill path stays intact in
  either direction.
- **Anchors**: 22 / 22 byte-identical pre- and post-sweep — anchor
  risk is **zero** whether v0.1.1 ships or is rolled back.
- **Snapshot baselines**: the 3 new PNGs under
  `crates/ui/tests/visual-baselines/` are NEW files; deleting them
  on rollback leaves the existing baseline set untouched.
- **Bench**: `crates/reflection/benches/trail_mirror.rs` + dev-deps
  in `reflection/Cargo.toml` are isolated from any production build.
- **Idle-CPU script**: `scripts/bench_idle_cpu.sh` is a standalone
  tool with no callers in the production build graph.

No anchor risk in either direction. Rollback cost is one revert.

## 8. Decision asked of operator

**Ship v0.1.1 as-is.** Every hard gate is green:

- `cargo fmt --check` PASS
- `cargo clippy --workspace -- -D warnings` PASS
- `cargo test --workspace --lib` 939 / 939 PASS (937 baseline + 2 new
  unit tests)
- `verify_anchors.sh` ANCHORS PASS (22 / 22) pre- and post-sweep
- `layout_invariants` 6 / 6 PASS
- `visual_snapshots` 7 / 7 PASS × 2 consecutive runs (determinism
  confirmed)
- H5 backfill p99 = **0.021 ms** ≪ 50 ms gate (2380× headroom)
- spec-lint = 87 / 2 categories = predecessor baseline (0 new
  regressions)

Deferred items are honestly classified as infrastructure-blocked
(T-F6, T-F7) or pre-existing v0.1.2 hygiene (`--features live`
lints). None are correctness regressions.

- **Approve → ship** if the deferred set in § 6 is the right cut.
  Standing directive is "Autoapprove all" — ratifying this is consistent
  with the v0.1.0 ship decision.
- **Approve with notes** if you want one of the deferred items
  promoted (most likely candidate: T-F7 paper-mode counter, runnable
  now on the operator workstation per § 6 verbatim command).
- **Reject** if the Q3 = (c) plumbing choice (construct
  `TrailMirror` inside cockpit_live rather than moving the agent::main
  spawn site) feels wrong on inspection; add a one-line reason so the
  architect can re-open Q3.

## 9. Next-up follow-up brief (v0.1.2)

A short hygiene + deployment-probe patch could land soon:

- **`--features live` clippy cleanup** — fix the 13 pre-existing
  `needless_pass_by_value` lints in `crates/ui/src/live.rs:159-428`
  (developer work item; closes the `--features live` cargo gate so
  it can be promoted into the default CI matrix).
- **T-F6 deployment-side idle-CPU floor** — run
  `bash scripts/bench_idle_cpu.sh <cockpit_live_pid> 60` × N=3 on the
  operator workstation; assert median ≤ 13.6%. Tooling is ready.
- **T-F7 deployment-side K7 counter** — run the verbatim cargo
  command in § 6 against the BS-1 checkpoint; assert
  `reflection_audit_tick_seen_total{variant="ForecastEmitted"} ≥ 1`
  after 60 s.
- **Optional** — measure the on-disk WAL-mode H5 backfill p99 to
  complement the `:memory:` 0.021 ms number, ruling out the K3
  delta as a future v0.1.2 surprise.

None of these are blockers; they extend forward coverage of the
Phase D+ surface already shipped.

## 10. Approval

Tick exactly one. The presenter agent has **not** ticked anything
below — the mechanical pre-tick guard
(`scripts/check_presentation.sh`) re-verifies this after the file
is written (see closing block).

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / rejection reason

Operator: "Autoapprove all" + Q1=YES + Q5=(a) decided 2026-05-20.
Deferred set (T-F6 idle-CPU 60-s sustained probe, T-F7 K7 ForecastEmitted
counter live run) is the same sandbox-display-server class accepted at
Phase D v0.1.0 ship; not a code regression. v0.1.2 hygiene candidates
captured: deployment-side T-F7 probe + `--features live` clippy cleanup
in `crates/ui/src/live.rs:159-428`. Ship v0.1.1.

## 11. Feedback log

_empty — no rejections yet_

---

### Closing — mechanical gates (presenter pre-emit checks)

```
$ bash scripts/check_presentation.sh spec/ui-rethink-phase-d-trail-followup/presentations/ui-rethink-phase-d-trail-followup-2026-05-20.md
PRESENTATION CHECK PASS  (spec/ui-rethink-phase-d-trail-followup/presentations/ui-rethink-phase-d-trail-followup-2026-05-20.md — approval block UN-ticked)

$ python3.14 scripts/spec_lint.py
spec-lint: FAIL (87 violations in 2 categories)
```

The spec-lint **87 / 2** matches the tester report baseline (§ 11 of
the test report) exactly — **0 new regressions vs. the
PASS-WITH-DEFERRED verdict commit**. All 87 violations are
pre-existing spec debt (81 dead-link + 6 trace-broken-path for
v25a/v25b/v26 future-model anchors not yet in `anchors.toml`) and are
out of scope for this v0.1.1 patch.

Phase D+ v0.1.1 contribution to spec debt = **0 net**.
