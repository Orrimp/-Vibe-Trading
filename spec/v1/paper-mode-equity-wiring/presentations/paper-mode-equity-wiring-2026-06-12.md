---
slug: paper-mode-equity-wiring
mode: release
status: draft
audience: human-operator
updated: 2026-06-12
generated: 2026-06-12T00:00:00Z
trace: REQ-LIVE-EQUITY-PAPER-001
---

# Paper-mode equity wiring — make the cockpit equity curve REAL in paper mode — release

## TL;DR

Paper mode now runs the real trading strategy against the live Binance feed, so the cockpit equity curve moves with actual paper fills instead of sitting flat at your starting capital — and it is structurally gated, at two layers, against ever silently going flat again.

## What changed

- **Paper mode actually trades now.** Yesterday's `live-equity-history-durable` deck exposed that paper-mode equity was a flat line at starting capital — the paper arm connected the live feed but ran no strategy loop (its update channel was thrown away), so equity was `cash + 0 × 0 = starting capital`, forever. This feature runs the same per-bar pipeline research already uses — your configured SMA crossover → risk sizer → in-process paper matching engine — against live Binance bar closes, publishing real fills, positions, and equity.
- **One trading loop for the whole agent.** Research and paper now share a single `spawn_trading_loop` (ADR-0053). Research is proven byte-identical; the structural payoff is that the eventual live-money path inherits this exact topology instead of a third copy.
- **The flat curve is now structurally impossible to ship.** A regression back to a flat line is caught at two layers — the data layer (the persisted equity values must not all be equal) and the render layer (the drawn curve must have real vertical extent). See the governance section below — this is the load-bearing point.

## Why

The cockpit's durable equity history (shipped the day before, ADR-0052) was faithfully persisting and re-hydrating a constant — the persistence rails were real but the value flowing through them never moved, because nothing in paper mode ran a strategy or produced a fill. Paper mode was therefore a rehearsal of "an account that never trades," which defeats the entire point of paper trading. This feature makes paper-mode equity reflect the real mark-to-market of a real (paper) position book driven by the strategy you actually configured, so paper mode becomes a true rehearsal of live. (Source: `spec/paper-mode-equity-wiring/feature.md` § Why; `_bmad-output/planning-artifacts/architecture/decisions/0053-unified-per-bar-trading-loop.md`.)

## What you can do now

| Action | Command |
|--------|---------|
| Run paper mode with a real, moving equity curve | Set `mode = "paper"` in `config/agent.toml`, then `cargo run -p ui --release --bin cockpit_live --features live` (full recipe below) |
| Prove the equity actually moves (data layer) | `cargo test -p agent --test equity_store_integration paper_loop_produces_moving_equity -- --exact` |
| Prove the curve renders non-flat (render layer) | `cargo test -p ui --test live_equity_render y_variation_gate_moving_passes_flat_fails -- --exact` |
| Confirm research + backtests are unchanged | `bash scripts/verify_anchors.sh` (expect `ANCHORS PASS (119 / 119)`) |

## Live demo

The product change is a paper-mode loop that needs a live Binance WebSocket and minutes of wallclock to show fills — that belongs in the operator recipe below, not a captured embed. The ground-truth evidence captured here is the two halves of the divergence gate (the proof the flat-line bug is fixed and stays fixed) plus the live anchor gate, all run against the on-disk implementation.

```
$ cargo test -p agent --test equity_store_integration paper_loop_produces_moving_equity -- --exact --nocapture
running 1 test
test paper_loop_produces_moving_equity ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.66s

$ cargo test -p ui --test live_equity_render y_variation_gate_moving_passes_flat_fails -- --exact --nocapture
running 1 test
test y_variation_gate_moving_passes_flat_fails ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.40s

$ bash scripts/verify_anchors.sh
...
ANCHORS PASS  (119 / 119)
```

The first run drives 80 moving bars through the paper loop and asserts the persisted equity values are **not all equal** (a flat series fails it — that is the exact bug being fixed). The second renders a moving curve and a flat curve and proves only the moving one clears the vertical-extent threshold. The third proves the research/backtest world is byte-for-byte untouched. Full captured output (including the structural `drop(state_tx)`-deleted grep): `artifacts/paper-mode-equity-wiring-2026-06-12/divergence-gates.txt`.

## Governance highlight — the silent-no-op class is now gated at two layers

This is the **first feature since the v3 vol-overlay no-op fix (2026-05-22)** where the CLAUDE.md baseline-equity-divergence non-negotiable **APPLIES** rather than being N/A. The durable-history feature was a read-only monitor with no strategy decision, so it was a genuine N/A. This feature introduces a real decision into paper mode — strategy signal → position sizing → paper execution — and the bug it fixes (an update channel computed-then-thrown-away, so equity never moved) is precisely the "value computed but never applied" class that cost the project the v3 round-trips.

The gate did not get rubber-stamped. It shipped satisfied in **both** halves:

- **Data layer** — `paper_loop_produces_moving_equity` (`crates/agent/tests/equity_store_integration.rs:214`) fails if the persisted equity series is constant. A flat curve makes its `!all_equal` assertion (line 298) panic. This is a genuine sentinel for the bug, not a happy-path test.
- **Render layer** — `y_variation_gate_moving_passes_flat_fails` (`crates/ui/tests/live_equity_render.rs:655`) is the load-bearing proof. The existing pixel checks (accent-pixel count, x-axis span) **cannot** catch a flat curve: the flat-line guard draws a degenerate series as a centered full-width horizontal line that passes both. The flat half of this test **proves that** — it shows a flat series clearing count ≥ 200 and x-span ≥ 400 while failing the new check. The only discriminator is the drawn curve's vertical extent: `CURVE_Y_VAR_MIN = 30px` (line 635), versus ~168px for a healthy session and ~1–2px for a flat line — a ~2-orders-of-magnitude gap, so the threshold is stable across themes and layout shifts.

Plain version: the exact failure mode that previously slipped through unit tests and anchored reports now cannot reach you without tripping a test at the data layer AND a test at the pixel layer. (Source: ADR-0053 § D5; tester report § 8, § 13.)

## Paper-mode demo recipe

A self-contained recipe to see the fix with your own eyes. Honest network/timing caveats are in **Expected result** and **Failure diagnosis** — read those before concluding anything.

**Command**

```bash
# 1. Edit config/agent.toml — change the first line:
#       mode = "research"   →   mode = "paper"
# 2. Launch the live cockpit:
cargo run -p ui --release --bin cockpit_live --features live
# 3. In the cockpit, open the Live view (the equity-curve screen).
```

**Steps**

1. Set `mode = "paper"` in `config/agent.toml` (it ships `mode = "research"`). Leave everything else as-is — paper trades the already-configured `SmaCrossover` (`fast_len = 20`, `slow_len = 50` per `[strategies.sma_crossover]`). Paper mode does not silently substitute a different strategy.
2. Launch with `--features live` (the `live` feature is what wires the Binance feed + agent; without it you get the fixtures cockpit, which never runs the paper loop).
3. Watch the Live view over a few minutes: the equity curve should show **real movement** once a position opens, the fills tape should populate as the crossover signals, and the Trades counter should climb.
4. Quit the cockpit, then relaunch the same command. The curve should **hydrate from durable history** on boot (the persisted real series, ADR-0052 rails) and show the "Since inception" caption — proving the movement was saved, not just drawn live.

**Timing**

- The feed is the live Binance WebSocket on **1-minute bar closes** (`BTCUSDT/1m`). Equity marks once per closed bar, so movement appears on a **minute cadence**, not instantly — give it several minutes.
- The SMA crossover warms up over its slow window (50 bars). Fills only occur when the crossover **actually signals**; in a quiet market that can be infrequent. The equity **mark** still moves with price every bar once a position is open — you do not need a fresh fill on every bar to see the curve move.

**Expected result**

- Live view: a **non-flat** equity curve that tracks BTC price once a position is open; an active fills tape; a climbing Trades counter; KPI Total-return and Max-DD reflecting real movement (Sharpe / CAGR / Win cards stay `—` — live KPI math is still out of scope, unchanged from the durable feature).
- After relaunch: the curve hydrates with prior history and the "Since inception" caption is present.

**Failure diagnosis — the one distinction that matters**

- **Flat-at-capital BEFORE the first fill is CORRECT, not the bug.** Until the strategy opens its first position, equity is genuinely `cash` with no position to mark, so the curve is a flat line at starting capital. That is now correct behavior.
- **Flat-at-capital FOREVER (across many bars, fills tape stays empty for a long time, never any movement even after positions should have opened) is the old bug** — and it is what the two divergence gates above are built to catch before it ever reaches you. If you genuinely see forever-flat with the strategy warmed up and the market moving, that is a real regression: capture the Live view and route it back (Reject below).
- No movement at all + empty fills tape + a stale-feed warning → the Binance WS is not delivering bars (network / connectivity), not a trading-loop bug. The stale-data watchdog surfaces this.

**Cleanup**

- Revert `config/agent.toml` back to `mode = "research"` when done.
- The paper session writes equity rows to the durable store (`./data/audit/ledger.db` per `[audit]`). If you want a clean slate for the next demo, clear the paper equity history out-of-band before relaunching; otherwise the next launch hydrates this session's curve (which is the intended re-hydration behavior).

## Verification matrix

V-ids map to the feature's acceptance criteria (AC1–AC8). Evidence is the proving test + its result, verified live by the presenter against the on-disk tree on 2026-06-12.

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| AC1 | Paper loop produces moving equity (data-layer divergence) | VERIFIED | `paper_loop_produces_moving_equity` PASS (`equity_store_integration.rs:214`; `!all_equal` assert at :298) — presenter ran live, 1 passed |
| AC2 | One writer, one series (row count == bar count, no double-mint) | VERIFIED | same test, `assert_eq!(rows.len(), bar_count)` at :313 — PASS; `build_snapshot_row` is the sole writer |
| AC3 | No real orders (paper invariant — `PaperEngine` only, seed `0x00C0_FFEE`) | VERIFIED | structural: `spawn_trading_loop` constructs no exchange client; mode-label assert at :324 — PASS |
| AC4 | `drop(state_tx)` stub deleted | VERIFIED | `grep "drop(state_tx)" runtime.rs` → only a comment at :651, zero live calls (presenter ran live) |
| AC5 | Fills + positions populate the Live panels | VERIFIED | same test, `fill_count > 0` (:272) + `pos_count > 0` (:281) — PASS |
| AC6 | Non-flat curve renders at the pixel layer (render-layer divergence, THE gate) | VERIFIED | `y_variation_gate_moving_passes_flat_fails` PASS (`live_equity_render.rs:655`, `CURVE_Y_VAR_MIN = 30`) — presenter ran live, 1 passed |
| AC7 | Research + backtests byte-unchanged | VERIFIED | `verify_anchors.sh` → `ANCHORS PASS (119 / 119)` (presenter ran live); paced-replay + research tests green |
| AC8 | Fixtures smoke + no-`live`-feature build unchanged | VERIFIED | tester: `cargo build -p ui` / `--features live` / `--bin cockpit --features fixtures` all PASS; no new deps (tester report § 7) |

## Numbers that matter

- **Anchors: 119 / 119 PASS** — verified live by the presenter (`bash scripts/verify_anchors.sh`). The file's inline milestone comments lag the count; the script is ground truth.
- **Tests** (all run live by the presenter where noted):
  - `crates/ui` render harness `live_equity_render`: **8 passed, 0 failed** (presenter ran live).
  - `crates/agent` `equity_store_integration`: **5 passed, 0 failed** (presenter ran live).
  - `cargo test -p ui --lib`: **447 passed** on both feature sets (no-feature and `--features live`) (tester report § 3a).
  - `cargo test -p ui --test panel_snapshots`: **103 passed** (tester report § 3a).
  - `cargo test -p agent` (all suites) + `cargo test -p audit`: all pass, 0 failed (tester report § 3a).
- **Render-layer threshold:** `CURVE_Y_VAR_MIN = 30px` — moving curve ~168px, flat line ~1–2px (`live_equity_render.rs:635`).
- **Research byte-stability guards (AC7):** `paced_replay_late_subscriber_receives_fills_positions_pnl`, `paper_loop_equity_store_research_none_zero_rows` (research `None` → 0 rows), existing research integration tests — all PASS (tester report § 10).
- **Cockpit-smoke:** orchestrator-run, PASSED — `spec/paper-mode-equity-wiring/reports/cockpit-smoke-2026-06-11T21-37Z.log` (build green; windowed Live view operator-verified out-of-band per the parked-view note).
- **Spec-lint:** `spec-lint: FAIL (71 violations in 2 categories)` — all pre-existing, **decreased** from the `audit-2026-06-08` baseline of 94 (−23). No new category, no count grew; does not block (tester report § 11). Presenter re-confirmed live below.
- **Perf:** _n/a — no hot-path change; trading loop and equity curve are not criterion-benchmarked (tester report § 6)._

## Screenshots

_n/a — the only UI-surface change is a render-TEST assertion (`CURVE_Y_VAR_MIN`), not a widget, layout, or theme change; there is no new visual surface to capture. The render harness rasterizes the real Live screen and asserts on pixels (`y_variation_gate_moving_passes_flat_fails`), which is the pixel-level proof in lieu of a static screenshot. A live windowed view is the operator recipe above._

## Open decisions

1. **Ship paper-mode equity wiring?** Everything verifies green (8 ACs, 119/119 anchors, both divergence gates live-confirmed). The only thing standing between this and shipped is your approval and the queued commit unlock. A "yes" carries one follow-up cost: the implementation is currently **uncommitted on disk** by your directive — on approval it commits + pushes at your morning unlock (no anchor re-lock needed; this feature is anchor-additive with zero new rows).

_Carried-forward item (not a decision for this feature — unchanged by it): the ADR-0052 nightly equity-purge scheduling is still deferred. This feature persists forward through the same rails and does not touch that deferral._

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

2026-06-12 — operator: **Approved — ship** (captured via orchestrator decision dialog). Operator runs the paper-mode hands-on demo post-approval; approval ratifies the gated evidence (8/8 ACs, divergence gate at both layers).

## Feedback log

_no feedback yet_

## Changelog

- 2026-06-12 (presenter): initial release deck. Evidence verified live against the uncommitted on-disk tree (anchors 119/119, render harness 8/8, equity_store_integration 5/5, both divergence-gate halves, AC4 structural grep). Governance section + paper-mode recipe authored.
- 2026-06-12 (presenter, re-verify): the lone ship blocker — test names in the `REQ-LIVE-EQUITY-PAPER-001` `anchors` field (namespace misuse, spec-lint 71→73) — was fixed by the orchestrator (`anchors = []` per the anchor-additive contract; tests stay cited in `tests`). Re-verified: spec-lint back to **71** (trace-broken-path 5, all pre-existing, none referencing this req); trace row confirms `anchors = []` + `state = "tester-done"` (VERDICT → PASS); pre-tick guard PASS (approval block UN-ticked). Gate cleared → `PRESENTATION → READY`.
