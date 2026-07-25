---
slug: live-equity-history-durable
mode: release
status: draft
audience: human-operator
updated: 2026-06-11
generated: 2026-06-11T21:10:00Z
trace: REQ-LIVE-EQUITY-HISTORY-001
---

# Durable live equity history — survive `cockpit_live` restart — release

## TL;DR

The cockpit Live equity curve no longer resets to blank on every restart — in
paper/live mode it now saves each bar's equity and redraws the full history on
boot — and this closes the last open cockpit Live-view item (TODO #3).

## What changed

- **Paper/live sessions now persist.** Every per-bar equity reading the agent
  computes is written to the audit ledger (new `equity_snapshots` table). When
  you relaunch `cockpit_live`, the Live equity curve comes up **pre-populated
  from the prior session** instead of empty, and the return caption reads
  **"Since inception"** (the figure spans the whole saved history, not just this
  session).
- **Research-replay mode persists nothing — on purpose.** Research replays the
  same 2023 data every boot, so saving it would stack duplicate, overlapping
  curves on top of each other. The write is gated off in research mode at the
  source. This is a design feature, not a gap.
- **No new screen, no new dependency, no schema risk.** The existing curve +
  KPI strip are reused verbatim; the only new storage is an additive
  `CREATE TABLE IF NOT EXISTS` migration that cannot touch the 119 locked
  backtest anchors.

## Why

The Live equity curve was **session-scoped**: the agent kept only a single
scalar equity number, and the UI accumulated an in-memory series that was never
saved and started empty in every cockpit constructor — quit and reopen, and the
curve was blank until the agent re-traded. This was a documented deferral: the
shipped `cockpit-live-dashboard-wiring` feature deliberately built the
session-scoped buffer as the right proportionate ship and named **this**
exec-side follow-on to make the series durable. This feature is that follow-on —
it writes the equity the agent already computes to the store it already runs
(the audit ledger), gated to paper/live so research replay can't pollute it, and
reads it back on boot. See
[`spec/live-equity-history-durable/feature.md`](../feature.md) and
[ADR-0052](../../../../_bmad-output/planning-artifacts/architecture/decisions/0052-durable-live-equity-series.md).

## What you can do now

| Action | Command |
|--------|---------|
| Run a paper session that **saves** its equity history (live Binance feed) | `cargo run -p ui --release --bin cockpit_live --features live` (with `config/agent.toml` `mode = "paper"`) |
| Run the headless agent that **saves** history with no UI attached | `cargo run -p agent --release --bin trading` (with `mode = "paper"`) |
| Inspect the saved rows directly | `sqlite3 ./data/audit/ledger.db 'SELECT bar_ts, total_equity, mode FROM equity_snapshots ORDER BY ts DESC LIMIT 10;'` |
| Verify nothing changed in research mode (current default) | `cargo run -p ui --release --bin cockpit --features fixtures` |

> **Mode reality check (important):** the operator's current
> `config/agent.toml` has `mode = "research"`. In research mode **this feature
> is invisible by design** — no save, no hydrate, session-scoped curve, caption
> stays "Session to date". The persistence-across-restart behavior is only
> reachable in **paper** mode (full recipe under
> [Verification recipe](#verification-recipe-operator)).

## Live demo

The most representative ground truth is not a console banner — it is the two
gates that prove the feature works: the **mode gate** (paper saves, research
saves nothing — the load-bearing correctness line) and the **render gate** (the
hydrated curve actually rasterizes pixels, per project law "verify UI at the
render layer"). Both run below verbatim at HEAD `9eef752`.

```
$ cargo test -p agent --test equity_store_integration

running 3 tests
test ac2_research_mode_writes_zero_rows ... ok
test ac1_paper_mode_persists_one_row_per_bar ... ok
test ac1_faked_store_tail_is_monotone ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

```
$ cargo test -p ui --features live --test live_equity_render

running 7 tests
test harness_catches_dropped_points_empty_curve ... ok
test hydrated_boot_curve_actually_renders ... ok
test live_equity_curve_actually_renders ... ok
test diag_accent_bounding_box ... ok
test healthy_curve_draws_far_more_than_broken ... ok
test flat_and_single_point_curves_render_without_panic ... ok
test live_append_after_hydrate_still_renders_and_grows ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
```

Notice `ac1_paper_mode_persists_one_row_per_bar` (paper writes a row) sitting
next to `ac2_research_mode_writes_zero_rows` (research writes nothing) — that
pair IS the duplication-prevention guarantee. And
`hydrated_boot_curve_actually_renders` proves the pre-populated curve draws real
ACCENT polyline pixels with **zero live bars delivered yet** — a
model-says-Ready-but-blank-canvas regression would fail here.
(Raw capture:
[`artifacts/live-equity-history-durable-2026-06-11/demo-run-2026-06-11.txt`](artifacts/live-equity-history-durable-2026-06-11/demo-run-2026-06-11.txt).)

## Screenshots

This is a UI feature; the windowed `cockpit_live` binary cannot be run in this
headless sandbox, so the one money shot — the **hydrated boot showing a
pre-populated curve + "Since inception" caption** — is a manual capture for the
operator. The behavior itself is already proven at the pixel layer by
`hydrated_boot_curve_actually_renders` above (it rasterizes the real Live screen
and counts the curve pixels); the screenshot is for the operator's own eyes, not
as the correctness gate.

```bash
# On your workstation, capture the hydrated-boot money shot.
# PRECONDITION: config/agent.toml has mode = "paper", and you have ALREADY
# run a paper session once (so equity_snapshots has prior rows to hydrate).
# Network: paper mode connects to live Binance WS — be online.

cargo run -p ui --release --bin cockpit_live --features live &
sleep 8   # window draw + boot hydrate query against ./data/audit/ledger.db
screencapture -W spec/live-equity-history-durable/reports/screenshots/hydrated-boot-since-inception.png   # macOS, click the cockpit window
# OR (Linux GNOME): gnome-screenshot -w -f spec/live-equity-history-durable/reports/screenshots/hydrated-boot-since-inception.png
pkill -f "target/release/cockpit_live"
```

Expected in the capture: the Live equity curve is **non-empty the instant the
window opens** (before any new bar arrives), and the return caption under the
KPI strip reads **"Since inception"** (not "Session to date"). If the table is
empty (fresh ledger), the curve stays in the honest `Loading` state — that is
correct, not a bug. Save the PNG under
`spec/live-equity-history-durable/reports/screenshots/` and note it in that
folder's `README.md`.

## Verification

V-ids map to the feature's nine acceptance criteria (AC1–AC9). Evidence is the
proving test + the suite it ran in; all green at HEAD `9eef752` per the tester
report
[`reports/test-2026-06-11-live-equity-history-durable.md`](../../../../evidence/v1/live-equity-history-durable/reports/test-2026-06-11-live-equity-history-durable.md).

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| AC1 | Paper mode persists one equity row per bar | VERIFIED | `agent::equity_store_integration::ac1_paper_mode_persists_one_row_per_bar` — PASS (re-run live by presenter) |
| AC2 | Research mode persists **zero** rows (the mode gate) | VERIFIED | `agent::equity_store_integration::ac2_research_mode_writes_zero_rows` — PASS (re-run live by presenter) |
| AC3 | Writer/reader round-trip lossless (Decimal-as-TEXT, monotone `bar_ts`) | VERIFIED | `audit::query::tests::equity_snapshot_round_trip_ac3` + `equity_snapshot_tail_limit_ac3` — PASS |
| AC4 | Hydration seeds buffer; curve + KPI strip `Ready` before any live tick | VERIFIED | `ui::state::tests::pnl_hydrated_seeds_buffer_curve_and_strip_ready` (+3 siblings) — PASS |
| AC5 | First live append after hydrate still lands (`as_of` guard reconciled) | VERIFIED | `ui::live_equity_render::live_append_after_hydrate_still_renders_and_grows` (pixel layer) — PASS (re-run live) |
| AC6 | **Hydrated boot rasterizes a non-empty curve (THE render gate)** | VERIFIED | `ui::live_equity_render::hydrated_boot_curve_actually_renders` — PASS (re-run live; ≥200 ACCENT px, x-span ≥400 px, 0 live ticks) |
| AC7 | Migration additive; 119 backtest anchors byte-unchanged | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` (re-run live by presenter) |
| AC8 | Retention bounded (purge removes rows past the 30-day horizon) | VERIFIED | `audit::query::tests::equity_snapshot_purge_ac8` — PASS |
| AC9 | Fixtures cockpit smoke unchanged; every I/O behind a trait; no new dep | VERIFIED | `cargo build -p ui --bin cockpit --features fixtures` PASS + smoke log clean + `LiveEquityStore` trait + `async-trait` already in workspace |

Cockpit-smoke gate (orchestrator pre-tick, mandatory after a UI brief's PASS):
[`reports/cockpit-smoke-2026-06-11T20-43Z.log`](../../../../evidence/v1/live-equity-history-durable/reports/cockpit-smoke-2026-06-11T20-43Z.log)
— the fixtures cockpit booted and ran (`Running target/debug/cockpit`) with
**no first-frame panic signature**, which is the gate's pass condition (absence
of the `iced_tiny_skia::engine.rs` zero-dim Quad panic).

## Numbers that matter

- **Tests:** 1,262 passed / **0 failed** across the new-feature crates (`audit`
  139, `agent` 119, `ui --lib` 447, `ui --features live --lib` 447,
  `live_equity_render` 7, `panel_snapshots` 103; 4 ignored — pre-existing doc /
  data-absent). Source: tester report § Full suite counts.
- **Anchors:** **119 / 119** byte-unchanged (re-run live by presenter). The
  additive migration adds **zero** rows to `spec/anchors.toml` and changes none
  of the SHAs.
- **Spec-lint:** `71 violations in 2 categories` (66 dead-link + 5
  trace-broken-path) — **all pre-existing, fewer than the audit-2026-06-08
  baseline of 94**; no new category, no count increase → no structural
  regression introduced since the tester's PASS.
- **Migration:** `013_equity_snapshots.sql` — `CREATE TABLE IF NOT EXISTS` + 2
  indexes, 9 columns; no `ALTER`, no backfill, no `UPDATE`.
- **New dependencies:** **0** (`async-trait` already a workspace dep).
- **New theme tokens:** **0**. New strings: **1** (`"Since inception"`).
- **Pre-existing reds (NOT this feature — verbatim from the tester report):**
  `ui::lab_run_engine::inner::h3_in_memory_equals_cached_disk` (deterministic;
  hardcodes `XRPUSDT`, data absent locally), plus two **flaky** Monte-Carlo
  backtest tests (`run_path_funding_none_is_anchor_neutral`,
  `run_path_k_short_zero_byte_identical_to_head`) that **passed clean on the
  second run** (76/76). None are attributable to this feature.

## Open decisions

1. **Nightly purge scheduling is deferred — approve the deferral, or ask for it
   now.** The retention purge (`purge_old_equity_snapshots`, 30-day horizon) is
   **wired as a function and unit-tested (AC8)**, but it is **not yet hooked
   into any nightly scheduler** — confirmed: no caller exists outside its own
   test. Per [ADR-0052 § D5](../../../../_bmad-output/planning-artifacts/architecture/decisions/0052-durable-live-equity-series.md)
   this scheduling is an explicit operator decision deferred out of v0.1.0.
   **Practical cost of the deferral:** at 1-min paper bars the table grows ~1,440
   rows/day; the boot hydrate is always `LIMIT`-capped at 2,880 rows, so the UI
   never reads more than it should regardless — the only consequence of "no
   nightly purge" is unbounded **disk** growth in `ledger.db` over a long-running
   paper deployment. Approving ships the feature with the purge dormant;
   rejecting/with-notes can request the nightly hook be added before ship.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback

2026-06-12 — operator: **Approved — ship** (captured via orchestrator decision dialog). The D5 purge-scheduling open question was resolved before approval: the nightly hook is WIRED (commit 2ec06c6, T6b), not dormant.

> On **Approved — ship**: the trace row `REQ-LIVE-EQUITY-HISTORY-001` flips to
> `shipped` only after this approval, and the purge stays dormant (scheduling
> remains a named follow-on per Open decision #1). No anchor re-lock is
> required (this feature locks no anchors). No manual capture is required to
> ship — the hydrated-boot behavior is already proven at the pixel layer; the
> screenshot recipe is optional, for your own eyes.

## Verification recipe (operator)

Two parts. **Part A is verifiable right now in your current research-mode
config — it proves zero regressions.** **Part B is the persistence-across-restart
proof and requires switching to paper mode** (a live Binance feed — read the
network note).

### Part A — Research mode: nothing changed (do this first, no config edit)

- **Command:** `cargo run -p ui --release --bin cockpit --features fixtures`
- **Steps:** Launch the fixtures cockpit. Open the Live screen. Observe the
  equity curve and the return caption.
- **Timing:** Window draws in < 8 s; no waiting on data.
- **Expected result:** Identical to before this feature — the curve is
  session-scoped, the caption reads **"Session to date"**, and nothing hydrates
  (fixtures has no `live` feature and no agent). The 1,262-test suite + the
  render gate + the cockpit-smoke log are the machine evidence that research-mode
  behavior is byte-unchanged.
- **Failure diagnosis:** If the curve or caption differs from prior fixtures
  behavior, or the window panics on first frame, that is a regression — capture
  stderr and route back to the developer/ui-designer.
- **Cleanup:** None — the fixtures binary writes nothing to any ledger.

### Part B — Paper mode: persistence survives restart (the headline proof)

> **NETWORK + DATA DEPENDENCY (read before running):** paper mode connects to
> the **live Binance WebSocket** (`crates/agent/src/runtime.rs:565-573` — "paper
> mode — multi-venue WS ingest"), NOT the 2023 replay. You must be **online**.
> The paper-mode equity reconciler emits + persists on a **60-second cadence**
> (`runtime.rs:677`, `reconciler_interval_ms = 60_000`), so the **first saved
> row appears ~1 minute after boot**, and you need **≥2 minutes of run time** to
> accumulate ≥2 rows (the minimum to make the curve + KPI strip come up `Ready`
> on the next boot). At this stage the paper trading loop seeds equity at the
> configured `initial_capital_usdt` and does not yet move it per-fill
> (`runtime.rs:660-674` — periodic-persist stub), so the saved series will be a
> flat-but-real inception line; that is expected for v0.1.0 and still proves the
> save/hydrate round-trip.

- **Command (one-time setup, then run twice):**
  ```bash
  # 1. Switch to paper mode.
  #    Edit config/agent.toml line 1:  mode = "paper"
  # 2. First session — let it save a few rows (≥2 min), then quit.
  cargo run -p ui --release --bin cockpit_live --features live
  # ... watch the Live screen for ~2-3 minutes, then close the window ...
  # 3. Relaunch — the curve should come up PRE-POPULATED.
  cargo run -p ui --release --bin cockpit_live --features live
  ```
- **Steps:**
  1. Set `mode = "paper"` in `config/agent.toml` (line 1).
  2. First launch: open the Live screen; let it run ≥ 2 minutes so the
     60-second reconciler writes ≥ 2 rows to `equity_snapshots`; then quit.
  3. Relaunch `cockpit_live`; open the Live screen immediately.
  4. (Optional) Confirm the rows directly:
     `sqlite3 ./data/audit/ledger.db 'SELECT bar_ts, total_equity, mode FROM equity_snapshots ORDER BY ts DESC LIMIT 5;'`
- **Timing:** First session ≥ 2 min (60 s/row, need ≥ 2 rows). Relaunch hydrate
  is instant on boot (a single `LIMIT 2880` query).
- **Expected result:** On the **relaunch**, the Live equity curve is **non-empty
  the moment the window opens — before any new bar arrives** — drawn from the
  prior session's saved rows, and the return caption reads **"Since inception"**.
  The `sqlite3` query shows `mode = paper` rows with monotone `bar_ts`.
- **Failure diagnosis:**
  - Curve blank on relaunch → check `equity_snapshots` actually has ≥ 1 row
    (`SELECT COUNT(*) FROM equity_snapshots;`); if 0, the first session ran < 60 s
    or never connected to Binance (check stderr for `Binance feed initialized`).
  - Caption still says "Session to date" on a hydrated boot → hydrate gate did
    not fire; confirm `mode = "paper"` (not `research`) and the `--features live`
    flag is present.
  - No rows ever appear → confirm network egress to Binance and that the agent
    logged `equity_reconciler_spawned (paper mode — periodic persist enabled)`.
- **Cleanup:** **This leaves rows in the ledger DB.** Running paper mode writes
  to `./data/audit/ledger.db` (the `equity_snapshots` table). To reset for a
  clean re-test: `sqlite3 ./data/audit/ledger.db 'DELETE FROM equity_snapshots;'`
  (drops only the equity rows; leaves the rest of the audit ledger intact). To
  return to the shipped default, set `config/agent.toml` line 1 back to
  `mode = "research"`.

## Changelog

- 2026-06-11 (presenter): initial release deck. Assembled after tester
  `VERDICT → PASS` + orchestrator cockpit-smoke gate PASS at HEAD `9eef752`.
  Re-ran live: mode-gate persistence proof (AC1/AC2, 3/3), render-layer pixel
  gate (AC5/AC6 + suite, 7/7), anchor gate (119/119), spec-lint (71 pre-existing,
  no regression vs the 94 baseline). Surfaced one open decision (nightly purge
  scheduling deferred per ADR-0052 D5). Approval block ships UN-ticked.
