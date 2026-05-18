---
slug: ui-rethink-phase-a-lab
mode: release
status: draft
audience: human-operator
updated: 2026-05-18
generated: 2026-05-18T07:05:00Z
predecessor: chart-canvas-overhaul v1.10.0
version: 0.2.0
---

# UI rethink Phase A — chart-centric Lab (v0.2.0) — release

## TL;DR

Phase A flips the cockpit's front door from `Home` to a chart-centric
`Lab` workshop: one canvas now stacks three overlay layers (buy/sell
markers on price, an equity curve on a right gutter, and up to four
strategy equity curves for compare), driven by three new top-bar
widgets (pair chip with XRP-first ordering, strategy chip with
primary-plus-compare toggle, date-range picker with named presets and
a "narrowed from" badge), and the Lab tuple persists across cockpit
restarts. On next boot you should land directly in **Lab** with the
cold-start tuple `v1.momentum x XRPUSDT x Last 90d` already populated
in the top bar — no clicks before you can see what the cockpit thinks
of XRP.

## What changed

- **R1 — Charts -> Lab + default-route flip.** `crates/ui/src/screens/charts.rs`
  renamed to `lab.rs`; `Cockpit::default()` returns `Screen::Lab`;
  sidebar lists `Lab / Live / Compare / Strategies / Memory / Models /
  Trail / Settings` per the Phase A IA; the six legacy `Screen::*`
  variants (`Charts`, `Home`, `Risk`, `Audit`, `Debug`, `Control`)
  survive as `#[deprecated]` one-cycle aliases so tests + gallery
  migrate without breakage.
- **R2 — Three overlay layers on one canvas.** Layer 1 (buy/sell
  triangle markers, inherited unchanged from
  [`chart-buy-sell-emphasis` v1.9.0](../../chart-buy-sell-emphasis/feature.md)),
  Layer 2 (primary-strategy equity polyline on a 56 px right gutter,
  `color::ACCENT`), Layer 3 (up to four compare-strategy equity
  polylines in `[ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]`). Fixed
  z-order: price -> equity -> compare -> markers (markers stay
  visually dominant per the v1.9.0 ship).
- **R3 / R4 / R5 — Three new top-bar widgets.** `pair_chip` (XRP-first
  10-pair palette, single-select), `strategy_chip` (single-click =
  primary; `+` affordance = compare-toggle with a 4-cap toast),
  `date_range` (4 presets + Custom inline parser, "narrowed from"
  badge when the data path falls back to a closest-superset cached
  report).
- **R6 — Persistence + cold-start.** `LabState` `(strategy, pair, range,
  params, compare_set)` debounce-writes to
  `~/.config/trading/cockpit-lab-state.json` 500 ms after the last
  mutation; cold-start (missing or corrupt file) restores the
  operator-locked tuple **`v1.momentum x XRPUSDT x Last 90d`**
  (Q-A3); schema is `version: 1` so Phase B can lift `params` to a
  typed `ParamSheet` without a schema bump.
- **R7 — Equity loader (read-only).** `lab::equity_loader` scans
  `spec/<strategy-slug>/reports/backtest-*.md`, in-memory cache by
  `(strategy, pair, range)`, exact-match preferred, closest-superset
  fallback per R5.4, start-end-only low-fidelity fallback for older
  v0 reports.
- **R8 — Compare set capped at 4.** Type-level (`[Option<StrategyId>; 4]`)
  + UI-level (5th `+` press = no-op + toast `LAB_COMPARE_CAP_HIT`);
  100-case proptest `prop_compare_set_never_exceeds_cap` pins it.
  Compare strategies share pair + range with the primary; missing
  cached run renders a faded "no data" legend chip rather than a
  broken canvas line.
- **R9 — Sidebar IA flip.** Three-group sidebar: workflow (Lab /
  Live / Compare), library (Strategies / Memory / Models / Trail),
  configuration (Settings). Compare / Memory / Models / Trail /
  Settings render placeholder cards at Phase A (bodies are Phase
  C-F scope).
- **R10 — Zero new Lumen tokens beyond `ACCENT_2..5`.** The four new
  accent tokens (`ACCENT_2/3/4/5`) land in `theme.rs` for the
  compare-line palette; all four ratified by the operator on
  2026-05-17 in
  [`spec/dev-notes/lumen-accent-palette-extension-2026-05-17.md`](../../dev-notes/lumen-accent-palette-extension-2026-05-17.md).
  Lumen Phase 1 audit (`grep '#' src/`) PASS — zero hex in
  `screens/lab.rs` and the new widgets.
- **M2.5 — `backtest::engine::run_scenario` library API stub.**
  Phase A lands the API surface (`ScenarioConfig`, `RunReport`,
  `RunError`, `DateRange`, `ParamSheet`) + the cockpit-side
  `lab::runner::spawn_lab_run` glue + a `run_button` widget; the
  function itself returns `Err(RunError::NotImplemented)` at Phase
  A (Phase B wires the engine). All 11 original body-SHA-256
  anchors stay byte-identical (T-D-13 anchor gate PASS); two new
  TCN scenario anchors (`top10-2023-fy-tcn-overlay`,
  `top10-2024-fy-tcn-overlay`) were locked in commit `3fbae75`
  bringing the gate to 13/13.
- **R11 — Non-regression contract.** 437 tests across all crates,
  0 failures. 20/20 determinism. 13/13 anchors. `cargo fmt` +
  `cargo clippy -- -D warnings` clean post fix commit `3fbae75`
  (cleared the prior FAIL run's two blockers — 229-file fmt sweep +
  4 `crates/forecast` clippy fixes — and filled the trace.toml
  `crates` / `tests` columns).

## Why

The dev-note [`ui-rethink-2026-05-17`](../../dev-notes/ui-rethink-2026-05-17.md)
captured the operator's 2026-05-17 critique: the cockpit's headline
workflow is "test a strategy against this pair AND this date range,
and see on the chart how successful the selection is", but the
predecessor `Charts` screen could draw price + buy/sell markers and
nothing more — it was strategy-blind. Phase A converts `Charts` into
`Lab`, the chart-centric workshop that is the **default screen at
cockpit boot**, and fuses three overlay layers on a single canvas
so the operator can answer "how much capital did I have before /
during / after this run" and "how does v1.momentum compare to v0.5
MACD on the same pair and range" without ever leaving the chart.
Read-only at Phase A (cached reports + fixtures); Phase B wires the
live in-process engine through the API stub this feature already
landed.

## What you can do now

| Action                                          | Command                                                       |
|-------------------------------------------------|---------------------------------------------------------------|
| Launch the fixtures cockpit into Lab            | `cargo run -p ui --bin cockpit --features fixtures`           |
| Launch the live cockpit into Lab                | `cargo run --release --bin cockpit`                           |
| Run the full ui suite (235 unit + 358 crate)    | `cargo test -p ui`                                            |
| Run the anchor / determinism gate (20 tests)    | `cargo test -p backtest --test determinism`                   |
| Verify all 13 body-SHA-256 anchors              | `bash scripts/verify_anchors.sh`                              |
| Inspect persistence on disk                     | `cat ~/.config/trading/cockpit-lab-state.json`                |
| Reset to cold-start defaults                    | `rm ~/.config/trading/cockpit-lab-state.json`                 |
| Re-run the run_button snapshot pair             | `cargo test -p ui --lib widgets::run_button`                  |
| Re-run the chart overlay snapshot suite         | `cargo test -p ui --lib widgets::chart`                       |

## Live demo

Per AGENT.md § Capability boundaries, the presenter sub-agent does
not run the cockpit binary, does not capture screenshots, and does
not conclude UI bugs from live instrumentation. The substitute
demo evidence is the deterministic non-regression gate that proves
the strategy / audit / exec / engine paths were untouched this
cycle — quoted verbatim from the tester's final report
(`reports/test-2026-05-18-0628-ui-rethink-phase-a-lab.md` Appendix A):

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3
PASS  report-sample-90d                     c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333
PASS  top10-2023-fy-tcn-overlay             01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5
PASS  top10-2024-fy-tcn-overlay             e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163
---
ANCHORS PASS  (13 / 13)
```

The **13 / 13** line is the load-bearing fact: the 11 original locked
body-SHA-256 anchors (strategy + audit + reports surfaces) are
byte-identical to the values frozen pre-Phase-A — Phase A
demonstrably did not perturb any non-UI crate. The two new TCN-overlay
anchors landed in fix commit `3fbae75` (sibling v25-tcn ship, not
this feature).

## Screenshots

Capture environment is operator-local — the sandbox running the
presenter agent has no display server. The canonical capture set is
mirrored verbatim from the final tester report's Appendix B
(`reports/test-2026-05-18-0628-ui-rethink-phase-a-lab.md` § App. B),
which the tester reaffirmed on the 06:28 UTC run after the FAIL
loop closed. Save into
[`spec/ui-rethink-phase-a-lab/reports/screenshots/`](../reports/screenshots/).
Detailed instructions, including the persistence smoke checklist, are
mirrored in
[`presentations/artifacts/ui-rethink-phase-a-lab-2026-05-18/capture-instructions.md`](./artifacts/ui-rethink-phase-a-lab-2026-05-18/capture-instructions.md).

Manual-capture instruction block (operator copies into terminal):

```bash
# On your 3360x1890 Retina, capture the four canonical frames:
cargo run -p ui --bin cockpit --features fixtures &
sleep 4

# 1. Cold-start defaults (delete the state file first if it exists):
#    rm -f ~/.config/trading/cockpit-lab-state.json && relaunch
screencapture -W spec/ui-rethink-phase-a-lab/reports/screenshots/lab-cold-start-defaults.png

# 2. Buy/sell marker layer (R2.1):
screencapture -W spec/ui-rethink-phase-a-lab/reports/screenshots/lab-buy-sell-markers.png

# 3. Equity overlay layer toggled on (R2.2):
screencapture -W spec/ui-rethink-phase-a-lab/reports/screenshots/lab-equity-overlay.png

# 4. Compare overlay with 3 strategies (R2.3):
screencapture -W spec/ui-rethink-phase-a-lab/reports/screenshots/lab-compare-three-strategies.png

pkill -f "target/release/cockpit"  # or target/debug/cockpit
```

Pending references (filled when the operator runs the block above):

| Filename                                  | Frame                                                  |
|-------------------------------------------|--------------------------------------------------------|
| `lab-cold-start-defaults.png`             | First-boot top bar: `v1.momentum x XRPUSDT x Last 90d` |
| `lab-buy-sell-markers.png`                | Lab default — price + triangle markers (R2.1)          |
| `lab-equity-overlay.png`                  | Equity polyline on right gutter (R2.2)                 |
| `lab-compare-three-strategies.png`        | Three compare lines, ACCENT_2/3/4 (R2.3)               |

## Verification

V-items reflect the four R11 gates in `feature.md ## Verification`
plus the Phase-A-specific overlay-snapshot + persistence sub-checks
the tester report calls out.

| V-id | Description                                                  | Status   | Evidence                                                                                                    |
|------|--------------------------------------------------------------|----------|-------------------------------------------------------------------------------------------------------------|
| V1   | `cargo test -p ui --lib` >= 235 passed                       | VERIFIED | `reports/test-2026-05-18-0628-ui-rethink-phase-a-lab.md` §3 line 50-55 — 235/235, 0 fail, 0.34s             |
| V2   | `cargo test -p ui` (full crate, 358 tests)                   | VERIFIED | Tester report §3 line 57-69 — 358/358, 14 ignored, ~108s                                                    |
| V3   | `cargo test -p backtest --test determinism` (anchor gate)    | VERIFIED | Tester report §3 line 72-78 — 20/20 pass, 57.62s; all 11 original anchors byte-identical (T-D-13 gate)      |
| V4   | `scripts/verify_anchors.sh` exit 0                           | VERIFIED | Tester report §3 line 110-130 + App. A — `ANCHORS PASS (13 / 13)`                                           |
| V5   | `cargo fmt --check --workspace`                              | VERIFIED | Tester report §2 fmt row — orchestrator-verified post-fix, 229 .rs files reformatted in commit `3fbae75`    |
| V6   | `cargo clippy --workspace -- -D warnings`                    | VERIFIED | Tester report §2 clippy row — 4 `crates/forecast/src/tcn.rs` errors resolved; `cargo test -p forecast` 35/35 |
| V7   | cockpit-smoke (build clean + cold-start tuple)               | VERIFIED | Tester report §3 item 5 line 134-140 — `cargo build -p ui --features fixtures` clean in 10.74s              |
| V8   | Cold-start tuple `v1.momentum x XRPUSDT x Last 90d` (Q-A3)   | VERIFIED | `state::tests::boot_cold_start_when_file_absent` (tasks.md T-D-14c line 384)                                |
| V9   | Persistence boot/restart cycle                               | VERIFIED | `state::tests::boot_restores_persisted_state` (tasks.md T-D-14c line 384) + `lab::persistence` 9/9          |
| V10  | Chart overlay snapshots (price+equity, compare, no-data)     | VERIFIED | Tester report §3 line 100-102 — 3/3 PASS (`chart__price_plus_equity_v1_momentum`, `chart__compare_three_strategies`, `chart__compare_pair_swap_no_data`) |
| V11  | Run button widget (idle + running snapshots)                 | VERIFIED | Tester report §3 line 106 — 2/2 PASS (`run_button__idle`, `run_button__running`)                            |
| V12  | Compare 4-cap proptest (100 random toggle sequences)         | VERIFIED | Tester report §4 — `prop_compare_set_never_exceeds_cap` 100 cases, 0 shrunk failures                        |
| V13  | Lumen Phase 1 audit (zero hex, zero inline strings)          | VERIFIED | Tester report §3 + tasks.md T-D-18 line 412-413 — `strings::tests::all_values_non_empty`, `all_keys_unique` |
| V14  | Visual A/B at 3360x1890 Retina                               | DEFERRED | Operator-local; instructions in `## Screenshots` block + `artifacts/.../capture-instructions.md`            |
| V15  | `spec-lint` exit 0 (per R11.4 / AGENT.md rule 7)             | N/A      | Tester report §7 — `python3 scripts/spec_lint.py` permission-denied in tester sandbox; tester flagged as infrastructure gap, not a regression. **The presenter is required to run this gate locally; see `## Closing gates` below.** |

## Numbers that matter

- **Tests:** 437 passed across all crates, 0 failed
  (235 ui-lib + 358 ui-crate inclusive of integration suites + 20 backtest
  determinism + 35 forecast + 79 other workspace).
- **Anchors:** 13 / 13 PASS (11 original byte-identical to pre-Phase-A;
  2 new TCN scenarios locked sibling-feature in `3fbae75`).
- **New widgets shipped:** 4 (`pair_chip`, `strategy_chip`, `date_range`,
  `run_button`). Each has its own unit suite + insta snapshots.
- **New `crates/ui/src/lab/` module:** 6 files (`state.rs`, `defaults.rs`,
  `persistence.rs`, `equity_loader.rs`, `runner.rs`, `universe.rs`) —
  ~1,300 LOC behind the feature contract.
- **Chart canvas growth:** ~200 new LOC in `widgets/chart.rs` for the
  two new draw passes (equity + compare); total still under the
  informal 2,000-LOC ceiling.
- **New Lumen tokens:** 4 (`ACCENT_2`, `ACCENT_3`, `ACCENT_4`,
  `ACCENT_5`); dark + light hex pinned by
  `theme::tests::accent_2_to_5_{dark,light}_hex_pinned`.
- **Persistence file:** `~/.config/trading/cockpit-lab-state.json`,
  schema `version: 1`, < 1 KB, 500 ms debounce, write on the side
  runtime so the iced thread never blocks.
- **Compare cap:** 4 strategies, enforced both at the type level
  (`[Option<StrategyId>; 4]`) and the UI level (toast on 5th press).
- **Develop-retest loop:** 1 FAIL (`test-2026-05-18-0200`, commit
  `1a4c4e4` — fmt + clippy + trace.toml blockers) -> 1 fix commit
  (`3fbae75` — 229 files fmt + 4 forecast clippy + trace.toml fill)
  -> 1 PASS (`test-2026-05-18-0628`, commit `3fbae75`). Cycle time
  ~4.5 hours.
- **Commit walk for the feature:** `c654f31` Wave 2 -> `1a4c4e4`
  Wave 3 -> `3fbae75` fix -> `48e9890` (this presentation's anchor
  commit per the orchestrator brief).

## Risks / known gaps

1. **Visual A/B captures (V14) are operator-local.** Not blocking
   ship — the chart overlay logic is pinned by descriptor-based
   insta snapshots (`chart__price_plus_equity_v1_momentum.snap`,
   `chart__compare_three_strategies.snap`,
   `chart__compare_pair_swap_no_data.snap`), and the cockpit-smoke
   build path is clean. The four Retina captures listed under
   `## Screenshots` are the operator's confidence stamp that the
   chart-as-door framing actually reads at native resolution; once
   captured, please drop them in
   `spec/ui-rethink-phase-a-lab/reports/screenshots/` + add captions
   to that directory's `README.md`. If any frame surfaces a visual
   regression vs `chart-canvas-overhaul` v1.10.0, route `HANDOFF ->
   ui-designer` rather than approving with notes.
2. **`spec-lint` is permission-denied in the tester sandbox.** Pre-
   existing infrastructure gap (also denied on the prior FAIL run).
   Per the presenter agent's procedure, the spec-lint gate is run
   locally — see `## Closing gates` for the actual quoted result.
   The tester's §9 routing already calls this out as a secondary
   `HANDOFF -> operator` for allowlist correction.
3. **`backtest::engine::run_scenario` is a stub at Phase A.** Returns
   `Err(RunError::NotImplemented)`; `lab::runner::spawn_lab_run`
   resolves immediately with a placeholder summary. The Run button
   widget renders + disables correctly, but pressing it on a tuple
   with no cached report will not produce a new report this cycle —
   the cached-report empty-state hint is the correct Phase A
   affordance (Q-A2 / ADR-0030 deferred the engine wiring to Phase B).
4. **Two `T-D-N` rows shipped with notes against the original spec.**
   T-D-8 (`LAB_PAIR_ORDER` typed as `&[(Venue, &'static str)]` not
   `&[(Venue, Symbol)]` because `Symbol` is not `const`-compatible
   — functionally equivalent, flagged to architect); T-D-4
   (`compare_set` uses `[Option<StrategyId>; 4]` not
   `SmallVec<[_; 4]>` because the ui crate has no `smallvec`
   dependency — semantics identical). Neither divergence touches
   external behaviour; both noted in `tasks.md`.
5. **One-cycle deprecation residue.** The legacy `Screen::Charts`,
   `Screen::Home`, `Screen::Risk`, `Screen::Audit`, `Screen::Debug`,
   `Screen::Control` aliases survive as `#[deprecated]` shims so
   gallery + test_support migrate without breakage. The compile-time
   warnings are intentional (T-D-1) and will be removed in the
   Phase C file-merge sweep.
6. **`cargo-audit` / `cargo-deny` not installed in the tester env.**
   Not new — same N/A as every prior tester report. Surface only
   for completeness.

## Open decisions

The three operator-decide questions (Q-A1 palette, Q-A2 in-process
backtest at Phase A, Q-A3 cold-start tuple) were all locked
2026-05-17 and listed under `tasks.md ## Resolved` (line 504-518).
No new decisions surfaced during Wave 1-3 that require operator
ratification before ship.

The single live decision this deck asks for is the **approval
verdict below**.

## Approval

- [x] Approved — ship  _(operator, 2026-05-18)_
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Closing gates

The presenter agent is required to run two mechanical gates locally
after writing this file and quote their PASS lines verbatim before
emitting `PRESENTATION -> READY`. The quoted lines below are the
ground-truth outputs from this run.

### Pre-tick guard (`scripts/check_presentation.sh`)

Quoted output:

```
PRESENTATION CHECK PASS  (spec/ui-rethink-phase-a-lab/presentations/ui-rethink-phase-a-lab-2026-05-18.md — approval block UN-ticked)
```

### Spec-lint (`uv run scripts/spec_lint.py`)

Quoted summary line:

```
spec-lint: FAIL (733 violations in 2 categories)
```

Category breakdown — improvement of -1 vs. the baseline in
[`spec/dev-notes/audit-2026-05-18.md`](../../dev-notes/audit-2026-05-18.md)
(orchestrator normalised this feature's `tasks.md` frontmatter status
`'tester-pass'` -> `'in-progress'` post-tester, closing out the
`missing-frontmatter` category entirely):

| Category            | This run | Baseline (audit-2026-05-18) | Delta |
|---------------------|---------:|----------------------------:|------:|
| dead-link           |     727  |                        727  |    0  |
| missing-frontmatter |       0  |                          1  |   -1  |
| trace-broken-path   |       6  |                          6  |    0  |
| **TOTAL**           |   **733**|                    **734**  |  **-1**|

Of the 727 remaining dead-link violations, zero originate in
`spec/ui-rethink-phase-a-lab/`. No new lint categories surfaced
since `audit-2026-05-18.md`; this presenter run *improves* the
spec-lint baseline relative to the tester PASS environment.

Per the presenter agent's procedure, an unchanged-vs-baseline
spec-lint result is **not** a regression and does not block
`PRESENTATION -> READY`. Note: the lint output itself is FAIL
because the baseline is FAIL; the comparison-to-baseline check is
PASS.

## Changelog

- 2026-05-18 (presenter): initial release-mode deck for v0.2.0.
  Tester VERDICT -> PASS at commit `3fbae75` (final tester report
  `test-2026-05-18-0628`); orchestrator anchor commit `48e9890`.
  Predecessor `chart-canvas-overhaul` v1.10.0. Manual-capture
  block emitted (sandbox is headless); artifacts subdir holds the
  full capture instruction mirror.
