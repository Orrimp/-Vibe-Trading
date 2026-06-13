---
slug: simple-strategies-realdata
mode: release
status: draft
audience: human-operator
updated: 2026-06-13
generated: 2026-06-13T20:00:00Z
---

# Simple strategies on real Binance data — sma / macd / rsi / bbands in the Lab — release

## TL;DR

The four simple strategies (sma / macd / rsi / bbands) now run on the **real
pinned Binance hourly BTC/ETH corpus** straight from the cockpit Lab — pick a
strategy, a symbol, a 2023/2024 range, flip the new Source toggle to **Binance**,
and Run; results persist, repaint, KPI-compare, and overlay for free.

## What changed

- **The Lab Source toggle is now three-way: Synthetic / Yahoo / Binance.** The
  new **Binance** chip loads the project's canonical pinned Binance corpus
  (the same `3a8b96c4…` 1h BTC/ETH data the anchored scenarios use) instead of
  a random walk or Yahoo's separate dataset. Binance is *added alongside* Yahoo,
  not in place of it (operator decision, this feature's changelog).
- **A Binance run flows through the already-shipped persist / compare / overlay
  tooling untouched** — it writes a durable report to `lab-runs/`, repaints from
  disk in Lab history, and is KPI-diffable + two-run-overlayable in Compare. No
  new report template, no new compare math: the `lab-run-save-compare` chain
  (shipped 2026-06-12) auto-applies because a Binance run produces the exact
  same `RunReport` shape.
- **Two guards make a wrong-data run impossible** (see the dedicated beat below):
  a Binance run's equity must *measurably diverge* from synthetic, and a
  missing/un-fetched corpus raises a typed error with a re-fetch hint — it never
  silently falls back to synthetic.

## Why

The cockpit Lab is the project's strategy-checking surface, and the
`lab-run-save-compare` tooling (shipped 2026-06-12) just turned it into a real
characterization tool — but the four simple single-symbol strategies could only
be fed two data sources: synthetic GBM (a structureless random walk, useless for
evaluation) and the Yahoo cache (real, but a *different* dataset from the pinned
10-symbol Binance corpus every piece of anchored evidence is built on). The
operator wanted to check a basic strategy on *the same real BTC/ETH the canonical
scenarios use*, through the Lab, so the new compare tooling could characterize it.
This feature closes that gap by mirroring the existing Yahoo seam for a third
data source — one engine enum variant, one `preload_binance_bars` loader, one
toggle chip — and nothing else. It adds **no new strategy, no new sizing, no new
math**: only a new bar *source* feeding an already-shipped chain. (Source:
`spec/simple-strategies-realdata/feature.md` § Why.)

## What you can do now

| Action | Command |
|--------|---------|
| Run a simple strategy on real Binance data in the Lab | `cargo run -p ui --release --bin cockpit_live --features live` → Lab screen → Source = **Binance** → pick sma/macd/rsi/bbands + BTC or ETH + a 2023/2024 range → **Run** |
| Repaint a saved Binance run from disk | Lab → **History** → select the persisted `lab-runs/` entry (auto-loaded on boot) |
| KPI-compare / overlay two Binance runs | Lab → **Compare** → select two runs → KPI matrix + two-run equity overlay |
| Re-verify the headline divergence guard yourself | `cargo test -p backtest --test binance_cache_dispatch` |
| Confirm anchors untouched | `bash scripts/verify_anchors.sh` |

Notes the operator should know going in:
- The **Binance** chip enables only the four **single-symbol** strategies
  (`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`). Cross-sectional strategies
  (momentum / pairs / tcn / tcn-weights) are hidden under Binance and rejected
  at the engine with `UnsupportedDataSource` — single-symbol only at v0.1.0.
- The corpus is **hourly**, so strategy windows are bar-counts on the 1h series:
  the default **SMA 20/50 means 20h/50h**, a legitimate hourly strategy. Retune
  via the existing fast/slow length inputs (shown when `v0.sma` is active).
- The corpus is **gitignored + pinned** (`3a8b96c4…`). It must be present on
  disk; if it is missing, the run fails loudly with a re-fetch hint pointing at
  the offline fetch tool — it does **not** quietly run on synthetic.
- The `binance` cargo feature is **on by default** for the `ui` crate, so the
  everyday `cockpit_live` build ships the chip with no extra flags.

## Live demo

The most representative ground-truth run is the headline no-op-source guard suite
— it runs `v0.sma × BTCUSDT` on **real** `data/binance/BTCUSDT/2023/01.parquet`
hourly bars, proves it diverges from synthetic, and proves the four arms accept /
four reject correctly. Captured on this machine (corpus present, pin matched):

```
$ cargo test -p backtest --test binance_cache_dispatch
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.81s
     Running tests/binance_cache_dispatch.rs

running 9 tests
test binance_cache_rejected_by_momentum_arm ... ok
test binance_cache_rejected_by_pairs_arm ... ok
test binance_cache_rejected_by_tcn_arm ... ok
test binance_cache_rejected_by_tcn_weights_arm ... ok
test binance_cache_accepted_by_rsi_arm ... ok
test binance_cache_accepted_by_macd_arm ... ok
test binance_cache_accepted_by_sma_arm_label_is_binance ... ok
test binance_cache_accepted_by_bbands_arm ... ok
test binance_cache_real_bars_diverge_from_synthetic_baseline ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

The load-bearing line is `binance_cache_real_bars_diverge_from_synthetic_baseline
... ok` — that is real Binance hourly bytes reaching the v0.sma strategy and
producing an equity curve that differs from the synthetic GBM baseline by more
than the 1-USD floor. The four `accepted_by_*` lines are the single-symbol arms
taking Binance; the four `rejected_by_*` lines are the cross-sectional arms
refusing it. Full stdout:
`spec/simple-strategies-realdata/presentations/artifacts/simple-strategies-realdata-2026-06-13/binance_cache_dispatch.txt`.

## The no-silent-fallback guard (the project's signature lesson)

This feature is a *data-source* change, so the CLAUDE.md baseline-equity-divergence
gate is **N/A as written** (no overlay, no sizing modifier, no new decision
variable — the strategies are byte-unchanged). But the failure mode that gate
exists to catch — a value *computed but never applied* (the
`v3-volatility-forecaster-noop` burn) — has a precise analog here: a Binance
toggle that is *wired but silently feeds synthetic bars*, so the operator thinks
they are testing real BTC while watching a random walk. The feature ships the
purpose-built twin of that gate, in two halves:

1. **Live half — equity must diverge.** `binance_cache_real_bars_diverge_from_synthetic_baseline`
   runs `v0.sma × BTCUSDT × Jan 2023` on Binance bars and on synthetic bars with
   the **same** `(strategy, symbol, range, seed)`, and asserts the two final
   equities differ by **≥ 1 USD** (`Decimal::ONE`). The test PASSES — the real
   `3a8b96c4…` parquet bytes demonstrably reached the strategy. A silent
   synthetic fallback would make the two curves identical and fail this test.
2. **Design half — missing data errors loudly.** The `preload_binance_bars`
   loader returns a typed `Err` with a re-fetch hint on a missing / un-fetched /
   revision-mismatched corpus, and **never synthesizes bars**. Proven by
   `loader_missing_corpus_returns_typed_err_not_synthetic` (a `ZZZUSDT` request →
   typed cache-miss error, never `Ok(bars)`). PASS.

Net: a strategy can never quietly run on the wrong data. Either it runs on the
pinned Binance corpus, or it fails with a clear "run the fetch tool" message.

## Demo recipe (operator self-verification)

**Command**

```
cargo run -p ui --release --bin cockpit_live --features live
```

(The `binance` feature is on by default, so no extra `--features binance` is
needed. To exercise the *no-Binance* build instead, use
`--no-default-features --features live` — the toggle then shows two chips.)

**Steps**

1. Launch the cockpit with the command above. Wait for the window.
2. Open the **Lab** screen.
3. In the **Source** toggle row, click the **Binance** chip (third chip, right of
   Synthetic and Yahoo). The strategy list collapses to the four single-symbol
   strategies; cross-sectional strategies disappear.
4. Pick a strategy (e.g. `v0.sma`), a symbol (BTC or ETH), and a range inside
   2023 or 2024. For `v0.sma`, optionally set fast/slow lengths (remember: these
   are *hours* — 20/50 = 20h/50h).
5. Press **Run**. The equity curve repaints from the persisted `lab-runs/` report.
6. Run a second Binance configuration (different strategy or range). Open
   **Compare**, select the two runs, and confirm the KPI matrix + two-run equity
   overlay render.

**Timing** — cockpit cold build ≈ 2–4 min the first time, seconds thereafter; a
single hourly-corpus Lab run is sub-second to a few seconds (one parquet read +
a short backtest).

**Expected result** — the Binance chip highlights when active; only the four
single-symbol strategies are selectable under Binance; a run paints a real equity
curve (visibly different shape from the synthetic GBM curve for the same
settings); the run appears in History and is diffable/overlayable in Compare.

**Failure diagnosis** — if a run errors with a cache-miss / re-fetch message, the
gitignored Binance corpus is absent or stale on your machine — re-fetch it per
`data/binance/REVISION.toml` (pin `3a8b96c4…`), then retry. A *blank* curve (no
error) would indicate a render regression — capture it and route back to
ui-designer. If the Binance chip is missing entirely, you built without the
`binance` feature (you used `--no-default-features` without re-adding it).

**Cleanup** — none required. Binance Lab runs write only to `lab-runs/`
(gitignored, retention-purged keep-last-N-per-tuple); no git state, no
`spec/anchors.toml`, no committed report is touched. Close the cockpit when done.

## Screenshots

_Manual capture (sandbox is headless — no fresh screenshot can be generated
here, and no prior screenshot exists for this feature). The render layer is
machine-verified instead — see the three `lab_binance_render.rs` rows in the
Verification matrix below (the active-chip-marches-right test, the chip-highlight
test, and the equity-curve-rasterizes test all PASS, closing the "wired but
doesn't paint" gap). If a human-eye screenshot is wanted for the record, capture
it with this recipe:_

- **Command:** `cargo run -p ui --release --bin cockpit_live --features live`
- **Steps:** open Lab → click the **Binance** chip → pick `v0.sma` + BTC + a 2023
  range → Run.
- **Capture:** screenshot the Lab screen showing (a) the three-chip Source toggle
  with **Binance** highlighted and (b) a painted real-data equity curve.
- **Save to:** `spec/simple-strategies-realdata/reports/screenshots/lab-binance-toggle-2026-06-13.png`
- **Expected:** three chips visible, Binance active (accent token), four
  single-symbol strategies only, a non-flat equity polyline rendered.

## Verification

Built from the feature's acceptance criteria (`feature.md` R1–R6 / § Architecture
A1–A6) and the tester's AC matrix
(`spec/simple-strategies-realdata/reports/test-2026-06-13-simple-strategies-realdata.md`).
All evidence is a real test name + PASS from that report, re-confirmed live for
the AC4 suite and the anchor gate.

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| AC1 | `BinanceCache` accepted by the 4 single-symbol arms; report `data_source` label = `"binance"` | VERIFIED | `binance_cache_accepted_by_{sma,macd,rsi,bbands}_arm` PASS (re-run live, 9/9) |
| AC2 | The 4 cross-sectional arms reject `BinanceCache` → `UnsupportedDataSource` | VERIFIED | `binance_cache_rejected_by_{momentum,pairs,tcn,tcn_weights}_arm` PASS (live) |
| AC3 | Loader returns non-empty hourly bars + revision SHA; revision-mismatch → loud `Err` | VERIFIED | `loader_returns_nonempty_hourly_bars_with_revision_sha` PASS (ui/binance) |
| **AC4** | **No-op-source guard: Binance equity diverges ≥ 1 USD (Decimal) from synthetic baseline, same seed** | **VERIFIED** | `binance_cache_real_bars_diverge_from_synthetic_baseline` PASS (live; epsilon = 1 USD, real revision `3a8b96c4`); `binance_run_diverges_from_synthetic_baseline` PASS (ui/binance) |
| AC4-design | Loader NEVER synthesizes on miss → typed `Err` + re-fetch hint | VERIFIED | `loader_missing_corpus_returns_typed_err_not_synthetic` PASS (ZZZUSDT → typed cache-miss, never `Ok`) |
| AC5 | Persist + Compare round-trip: `.md` + CSV written; `EquityCache` element-by-element; `scan_spec_tree` CachedCell | VERIFIED | `binance_run_persists_and_round_trips_through_compare` PASS (ui/binance) |
| AC6 | Anchor tripwire: 119/119 unchanged (UN-ANCHORED by construction) | VERIFIED | `scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` (re-run live) |
| AC7 | Three-way toggle render-layer verified; Binance equity curve rasterizes | VERIFIED | `three_way_toggle_active_chip_marches_right`, `binance_chip_renders_visible_highlight`, `binance_sourced_equity_curve_rasterizes` PASS (ui/binance render layer) |
| **AC8** | **No-binance build shows exactly two chips (explicit invocation)** | **VERIFIED** | `cargo test -p ui --no-default-features --features live --test lab_source_toggle_no_binance` → `no_binance_feature_renders_two_chips` PASS |
| H3 | `spawn_preload_on_rt<S: LabBarSource>` generalization preserves in-memory == cached-disk equity round-trip | VERIFIED (no-regression) | `lab_run_engine::inner::h3_in_memory_equals_cached_disk` PASS |
| ADR-0050 | rt.spawn callthrough invariant intact after generalization (both the no-panic gate AND the direct-await-panics proof) | VERIFIED (no-regression) | `preload_callthrough_with_spawn_blocking_does_not_panic` PASS; `direct_await_without_rt_spawn_panics` PASS |
| Baseline-divergence (CLAUDE.md) | Strategy-overlay / sizing-modifier divergence gate | N/A | No overlay / sizing modifier / new decision variable — strategies byte-unchanged; AC4 is the purpose-built live analog |

## Numbers that matter

- **Headline divergence guard:** `binance_cache_real_bars_diverge_from_synthetic_baseline`
  PASS — real Binance equity vs synthetic, epsilon **= 1 USD (Decimal)**, on real
  corpus revision `3a8b96c4…`.
- **Tests:** all green. Suite totals from the tester report — backtest
  `binance_cache_dispatch` **9/9** (re-run live), ui lib **456/456**, ui fixtures
  **72** (2 ignored), ui render `live_equity_render` **15/15**, panel snapshots
  **103/103**, AC8 no-binance **1/1**, ADR-0050 callthrough **2/2**. 0 failures
  anywhere.
- **Anchors:** **119 / 119 PASS** (re-run live). UN-ANCHORED by construction — no
  `spec/anchors.toml` row added, no committed `spec/*/reports/` body mutated.
- **Visual baselines:** **11 Lab/Charts baselines regenerated** for the new chip.
  These are *mutable* baselines (not anchored report bodies) — orchestrator
  visually verified before the test run (three-way toggle present, screen intact).
- **Static checks:** `cargo fmt --check` clean; production-code clippy clean;
  spec-lint **70 violations, 0 new** (improved by 1 from the 71 baseline — all
  pre-existing dead-link / trace-path debt, none attributable to this feature).
- **Code surface:** no new dependencies; `binance` is a `ui` cargo feature
  re-exporting the existing `data` crate parquet reader; loader pins
  `Timeframe::OneHour`; money stays `Decimal` / `Money<Usdt>` (verified file:line
  in the tester's composition review).

## Open decisions

_No decisions pending — ready to ship._ The two genuinely decision-bearing
questions were resolved by the operator before build and are baked in:
**Q1-policy** = three-way toggle (Binance *added alongside* Yahoo, augmenting the
2026-05-24 Yahoo-only call — not reversing it); **Q-anchor** = UN-ANCHORED
(ad-hoc Lab runs persist to `lab-runs/` only, never into `anchors.toml`; 119/119
untouched by construction). Saying "yes" commits the operator to **no** follow-up
cost — no anchor re-lock, no manual capture is required to ship (the optional
screenshot above is for the record only).

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

<!-- operator fills this in -->

## Feedback log

_empty — no rejection routed._

## Changelog

- 2026-06-13 (presenter): initial release deck. Capstone of the post-live-removal
  Lab strategy-checking arc. Live-captured `binance_cache_dispatch` 9/9 (incl. the
  AC4 divergence guard) + `verify_anchors.sh` 119/119; verification matrix built
  from the tester AC matrix; approval block UN-ticked.
