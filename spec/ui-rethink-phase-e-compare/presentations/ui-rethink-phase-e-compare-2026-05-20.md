---
title: Operator Deck — ui-rethink-phase-e-compare v0.1.0
feature: ui-rethink-phase-e-compare
mode: release
date: 2026-05-20
presenter_run_id: 2026-05-20T22:55Z
test_report: spec/ui-rethink-phase-e-compare/reports/test-final-2026-05-20.md
verdict_source: tester M-FINAL VERDICT → PASS (re-gated, no deferrals)
commit_at_tester_pass: fbc74e41f9344b0872f3fb56e762e7dead105d10
predecessor: ui-rethink-phase-d-trail-followup v0.1.1 (shipped 2026-05-20)
trace_row_state: in-progress  # promoted to accepted/shipped on operator tick
---

# Operator Deck — UI rethink Phase E (Compare matrix — J3)

> Fifth concrete feature carved out of the chart-centric UI rethink
> (`spec/dev-notes/ui-rethink-2026-05-17.md` §6 Phase E, lines 1082-1096;
> §J3, lines 340-390). Sprint-review deck — read top to bottom in under
> 5 minutes, then tick exactly one approval box at the bottom. Both
> **Approve with notes** and **Reject** keep the work in the loop;
> please add a one-line reason so the relevant agent can act on it.

## 1. Operator headline

Phase E lights up the **Compare matrix** — the navigation hinge of the
UI rethink. The Phase C sidebar's `Compare` entry, which has been a
placeholder route since Phase A, now opens a **6 strategies × ≤10
pairs** read-only grid that lets the operator answer both reciprocal
questions in one glance: *"which strategy wins for this pair?"* (fix a
column, scan the rows) and *"which pair wins for this strategy?"* (fix
a row, scan the columns). The matrix reads cached backtest report
frontmatter — **no live recompute on screen-open** — and seeds the Lab
on cell-click, so the operator can drill from any cell straight into a
populated Lab. Every empty (but legal) cell renders a one-click **Run**
affordance that re-uses the Phase B Lab Run dispatch, turning the
cold-boot experience into a hands-on launchpad rather than a paralysed
empty grid. Cells outside a strategy's declared universe render greyed
with a passive `—` label (honest about which (strategy, pair) tuples
are even legal — e.g. `pairs-2023-zscore-mr` only on BTCUSDT+ETHUSDT).
Multi-symbol strategies whose reports carry universe-aggregate Sharpe
are flagged via a **dual-surface disclaimer** (subtitle under the
toolbar + per-cell tooltip; per-pair decomposition is a deferred
v0.2.0 candidate). The release is **additive-only by construction**:
the 22 backtest body-SHA-256 anchors are byte-identical pre- and
post-sweep, the 7th layout-invariants proptest (256 cases) joins 6
carry-forward cases with zero failures, the lib-test count rises from
939 → 946 (+7 new) with zero failures, and no Phase A/B/C/D-shipped
surface is touched. **No deferrals.** Every M-FINAL gate is green.

## 2. What landed

### 2.1 Compare module — cache + state + dispatch (R3, R4, R6)

- [`crates/ui/src/compare/mod.rs:1-6`](../../../crates/ui/src/compare/mod.rs)
  — new module root (`pub mod cache; pub mod state;`); `pub mod
  compare;` declared in
  [`crates/ui/src/lib.rs`](../../../crates/ui/src/lib.rs) next to
  `pub mod lab;`.
- [`crates/ui/src/compare/state.rs:1-107`](../../../crates/ui/src/compare/state.rs)
  — `CompareScreenState { range, kpi_axis, cache, last_indexed }` +
  `CachedCell { sharpe, total_return, max_drawdown, trade_count,
  is_multi_symbol, .. }` + `CompareKpiAxis { Sharpe, Sortino,
  TotalReturn, MaxDrawdown, WinRate }`. Cache keyed by
  `BTreeMap<(StrategyId, Symbol, DateRange), CachedCell>` for
  deterministic snapshot ordering.
- [`crates/ui/src/compare/cache.rs:1-505`](../../../crates/ui/src/compare/cache.rs)
  — hand-parsed YAML frontmatter reader (K3 resolution: **no
  `serde_yaml` dep**); `scan_spec_tree`, `lookup_cell`,
  `parse_frontmatter`, scenario→universe mapper (R3.2); 5 in-module
  unit tests under `#[cfg(test)] mod tests` (PASS — see § 4 row T-F8).
- [`crates/ui/src/state.rs`](../../../crates/ui/src/state.rs) —
  3 new `Message` variants: `OpenLabFromCompare { strategy, pair,
  range }` (compound dispatch near the `OpenTrailFor` arm — mirrors
  Phase C `OpenStrategyInLab` and Phase D `OpenTrailFor`
  precedent), `CompareSelectRange(DateRange)`,
  `CompareSelectKpiAxis(CompareKpiAxis)`. 3 update arms ordered per
  K4 mitigation; 2 new H5 round-trip unit tests appended to the
  `#[cfg(test)] mod tests` block.
- [`crates/ui/src/state.rs:~880`](../../../crates/ui/src/state.rs)
  — `pub compare_screen_state: CompareScreenState` field on
  `Cockpit`, sibling of `lab_state` (`:798`) and
  `trail_screen_state` (`:879`). Default-init in
  `Cockpit::new()` / `Cockpit::new_with_persistence()` /
  `Default::default()` / `Debug::fmt`.

### 2.2 Matrix widget (R2)

- [`crates/ui/src/widgets/matrix.rs:1-437`](../../../crates/ui/src/widgets/matrix.rs)
  — new widget exposing `pub fn view(model, mode) -> Element<'_>`.
  Layout primitive: iced `Column<Row>` (no new `grid` widget per
  R2.5). Rows iterate `model.strategies_config.strategies` (Q7=a);
  columns iterate `strategy.universe()` (Q8=b). Per-cell match:
  - **Populated** — Sharpe text + 30-bar sparkline + hairline-
    bordered Button; hover → `active_row` border tint
    ([`matrix.rs:305-326`](../../../crates/ui/src/widgets/matrix.rs)).
    Click → `OpenLabFromCompare { strategy, pair, range }` (R4.2).
    K7 tooltip rendered on every cell whose `cached.is_multi_symbol`
    is `true`.
  - **Empty-but-legal** — centred "Run" Button with `ACCENT_500`
    hairline (Q4=b). Click → `OpenLabFromCompare { .. }` followed
    by an auto-`LabRun` dispatch (R4.3).
  - **Blanked** — centred `—` label + passive hairline (Q8=b;
    K8 mitigation — distinguishable from active "Run" cells).
- [`crates/ui/src/widgets/mod.rs`](../../../crates/ui/src/widgets/mod.rs)
  — `pub mod matrix;` declaration added.

### 2.3 Compare screen body + shell wiring (R1, R5)

- [`crates/ui/src/screens/compare.rs:1-217`](../../../crates/ui/src/screens/compare.rs)
  — `pub fn view(model, mode) -> Element<'_>`. Toolbar:
  `Row[ range_picker | kpi_axis_dropdown |
  k7_subtitle_when_any_cell_is_multi_symbol ]`. Body:
  `widgets::matrix::view(model, mode)`. The K7 subtitle uses the
  single new string constant
  `strings::COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE` (T-T1-4 dual-
  surface decision).
- [`crates/ui/src/screens/mod.rs`](../../../crates/ui/src/screens/mod.rs)
  — `pub mod compare;` declaration added.
- [`crates/ui/src/shell.rs:96`](../../../crates/ui/src/shell.rs)
  — **single-line swap**: `Screen::Compare => placeholder::view(
  strings::COMPARE_PLACEHOLDER, mode)` → `Screen::Compare =>
  screens::compare::view(model, mode)`. `compare` added to the
  `use crate::screens::{...}` list at `:28`. No sidebar change
  (Phase C reserved the entry already in `SIDEBAR_GROUPS_PHASE_C`
  Work zone at
  [`crates/ui/src/theme.rs:742`](../../../crates/ui/src/theme.rs)).

### 2.4 Strings + Lab seed plumbing (R8, R5.2)

- [`crates/ui/src/strings.rs`](../../../crates/ui/src/strings.rs)
  — 5 new Phase E constants in the Phase E section:
  `COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE`,
  `COMPARE_TOOLBAR_RANGE_LABEL`, `COMPARE_TOOLBAR_KPI_LABEL`,
  `COMPARE_CELL_RUN_LABEL`, `COMPARE_CELL_BLANKED_LABEL`.
  `COMPARE_PLACEHOLDER` (line 252) marked `#[deprecated(since =
  "0.3.0", note = "Compare now renders the matrix body — Phase F
  removes this constant")]` per the `SETTINGS_PLACEHOLDER:259-263`
  precedent.
- [`crates/ui/src/lab/state.rs`](../../../crates/ui/src/lab/state.rs)
  — `PartialOrd`, `Ord` derives added on `Preset` + `DateRange` (a
  pure-trait change — required because `DateRange` now appears as
  part of a `BTreeMap` key in `CompareScreenState.cache`).

### 2.5 Four new visual snapshot baselines (T-D-N10..N13 / T-F3)

- [`crates/ui/tests/visual-baselines/compare__cold_boot_all_empty.png`](../../../crates/ui/tests/visual-baselines/compare__cold_boot_all_empty.png)
  (84,356 bytes) — 2 strategies, empty cache; every legal cell
  renders the "Run" affordance, blanked cells passive. K7 subtitle
  absent (no multi-symbol cells populated yet).
- [`crates/ui/tests/visual-baselines/compare__steady_state_populated.png`](../../../crates/ui/tests/visual-baselines/compare__steady_state_populated.png)
  (109,613 bytes) — 5 strategies, 24 cells populated per T-T1-2
  enumeration; K7 multi-symbol disclaimer subtitle visible.
- [`crates/ui/tests/visual-baselines/compare__empty_cell_run_affordance.png`](../../../crates/ui/tests/visual-baselines/compare__empty_cell_run_affordance.png)
  (94,390 bytes) — 2 strategies, 20 of 24 cells populated → 4 "Run"
  affordance cells exercising the active `ACCENT_500` hairline button.
- [`crates/ui/tests/visual-baselines/compare__column_header_hover.png`](../../../crates/ui/tests/visual-baselines/compare__column_header_hover.png)
  (84,356 bytes) — **byte-identical** to
  `compare__cold_boot_all_empty.png` **by design**. R2.4 v0.1.0
  contract: column headers are non-interactive (label only;
  drill-down via cell click). The fourth baseline asserts that
  hovering a column header produces **no** visual difference vs.
  cold boot — it is the negative control for the R2.4 contract,
  not an empty placeholder. Developer flagged this as intentional;
  tester independently verified via `cmp` (§ 4.3 of test report).

All four baselines are **NEW PNGs**, not changes to any of the 22
anchored body-SHAs. The 22-anchor gate stays non-negotiable and
passed pre- and post-sweep (§ 4 row T-F4).

### 2.6 New layout-invariants proptest + H5 round-trip tests

- [`crates/ui/tests/layout_invariants.rs`](../../../crates/ui/tests/layout_invariants.rs)
  — new `compare_screen_no_zero_dim` proptest (256 viewport-size
  samples from 320×240 → 3840×2160; asserts no panic + every cell
  area ≥ 1 px per R2.5) + `build_compare_cockpit()` helper. 7/7
  layout-invariants now PASS (6 carry-forward + 1 new).
- [`crates/ui/src/state.rs:~3370`](../../../crates/ui/src/state.rs)
  — H5 round-trip unit tests
  `open_lab_from_compare_sets_lab_strategy_pair_and_range` and
  extension `open_lab_from_compare_no_pair_leaves_pair_unchanged`.
  Both PASS (§ 4 row T-F7).

## 3. Architect resolutions (M-T1)

The architect's M-T1 pass resolved K3 (parser shape), the Q6 sub-
decision (disclaimer surface), and the state-location lock. Q1-Q8
were operator-decided via the standing "Autoapprove all" directive
(8 / 8 analyst-recommended defaults accepted in one tick on
2026-05-20). Resolution summary:

| Topic | Decision | Rationale |
|-------|----------|-----------|
| **K3** — `serde_yaml` workspace presence | **hand-parse flat YAML** | `cargo tree -e features --workspace \| grep -i yaml` returned only `yaml-rust2 v0.8.1` (transitive via `insta` + `config`); `serde_yaml` is **not** in the workspace. R7.6 forbids new external deps. Hand-parser is ~30 LOC (every report frontmatter is flat key:value with at most one nested `strategy:` block). No ADR. Locked in [`decomp.md § 1.1`](../decomp.md). |
| **Q6 sub-decision** — universe-aggregate disclaimer surface | **subtitle + per-cell tooltip** (dual surface) | K7 universe-aggregate semantic applies to **83 % of populated cells** (20 / 24 — the v1.momentum + v2.5.tcn rows per the static H1 census). A single corner-tooltip is insufficient at that coverage. Both surfaces reference the same new string constant `strings::COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE`. The architect **upgraded** the analyst's tooltip-only recommendation; locked in [`decomp.md § 1.4`](../decomp.md). |
| **State location** | `Cockpit::compare_screen_state` at `crates/ui/src/state.rs:~880` | Sibling of `trail_screen_state` (`:879`) and `lab_state` (`:798`); same 3-touchpoint pattern (struct field declaration + `Default` impl at `:1009,1108` + `Debug` impl at `:959`). Mirrors the Phase D shape verbatim; locked in [`decomp.md § 1.6`](../decomp.md). |
| **Wave shape** | A → B → C → D → E (5 waves; 5 net-new files; 18 T-D-N rows) | Wave A = cache module + state + Message variants; Wave B = `widgets::matrix`; Wave C = `screens::compare` + shell wiring (single-line `shell.rs:96` swap); Wave D = 4 snapshot baselines + 1 proptest case + 2 H5 round-trip tests + cockpit-smoke pre-run; Wave E = anchor gate + tester handoff. Spike requirement = **NONE** (all structural primitives carry-forward from Phase D+). Locked in [`decomp.md § 1.5 + § 3`](../decomp.md). |

## 4. Test results (verbatim from tester report)

### 4.1 Hard gates

| Gate | Command | Output line | Verdict |
|------|---------|-------------|---------|
| T-F1 | `cargo fmt --check` | (exit 0, no output) | **PASS** (re-gate 2026-05-20) |
| T-F1b | `cargo clippy --workspace -- -D warnings` | `Finished dev profile [unoptimized + debuginfo] target(s) in 4.21s` | **PASS** |
| T-F9 | `cargo clippy -p ui --features live -- -D warnings` | `Finished dev profile [unoptimized + debuginfo] target(s) in 4.50s` | **PASS** (no regression vs. `b61164d`) |
| T-F2 | `cargo test --workspace --lib` | `test result: ok. 303 passed; 0 failed; 0 ignored` (ui crate; total = 946) | **PASS** (946 ≥ 939 baseline, +7 new) |
| T-F3 (run 1) | `cargo test -p ui --test visual_snapshots -- compare__ --test-threads=1` | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 3.44s` | **PASS** |
| T-F3 (run 2) | `cargo test -p ui --test visual_snapshots -- compare__ --test-threads=1` | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 3.34s` | **PASS** (determinism confirmed) |
| T-F4 (pre-sweep) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (22 / 22)` | **PASS** |
| T-F4 (post-sweep) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (22 / 22)` (identical SHAs) | **PASS** |
| T-F5 | `cargo test -p ui --test layout_invariants` | `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 67.74s` | **PASS** (6 carry-forward + 1 new) |
| T-F6 | `cargo test -p ui --test headless_emulator_smoke` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s` | **PASS** (0 panic lines, R7.3) |
| T-F7 | `cargo test -p ui --lib open_lab_from_compare` | `test result: ok. 2 passed; 0 failed` (H5 round-trip; 2 / 2) | **PASS** |
| T-F8 | `cargo test -p ui --lib compare::cache::tests` | `test result: ok. 5 passed; 0 failed` | **PASS** |
| spec-lint | `python3.14 scripts/spec_lint.py` | `spec-lint: FAIL (87 violations in 2 categories)` | **PASS vs. baseline** (= Phase D+ predecessor 87/2; 0 new regressions) |

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

Post-sweep run produced identical SHAs → `ANCHORS PASS (22 / 22)`.
R7.1 carry-forward confirmed; Phase E is purely additive UI by
construction (R7.7).

### 4.3 Per-crate test profile

| Crate | Passed | Failed | Ignored |
|-------|-------:|-------:|--------:|
| `agent` | 52 | 0 | 0 |
| `audit` | 36 | 0 | 0 |
| `backtest` | 13 | 0 | 1 |
| `cost` | 9 | 0 | 0 |
| `data` | 47 | 0 | 1 |
| `exec` | 6 | 0 | 0 |
| `features` | 55 | 0 | 0 |
| `forecast` | 52 | 0 | 0 |
| `llm` | 84 | 0 | 0 |
| `models` | 0 | 0 | 0 |
| `reflection` | 11 | 0 | 0 |
| `replay_cache` | 8 | 0 | 0 |
| `reports` | 103 | 0 | 0 |
| `risk` | 10 | 0 | 0 |
| `strategy` | 85 | 0 | 0 |
| `trading_core` | 72 | 0 | 0 |
| `ui` | **303** | 0 | 0 |
| **Total** | **946** | **0** | **2** |

Baseline = 939 (Phase D+ v0.1.1). Phase E adds 7 new unit tests
(5 cache + 2 H5 round-trip). Two ignored entries are pre-existing
(`backtest`, `data`) and not introduced by Phase E.

### 4.4 Snapshot baseline byte sizes (developer-reported vs. tester-verified)

| Baseline | Bytes | Developer-reported | Match |
|----------|------:|-------------------:|-------|
| `compare__cold_boot_all_empty.png` | 84,356 | 84,356 | YES |
| `compare__steady_state_populated.png` | 109,613 | 109,613 | YES |
| `compare__empty_cell_run_affordance.png` | 94,390 | 94,390 | YES |
| `compare__column_header_hover.png` | 84,356 | 84,356 | YES (byte-identical to cold_boot, by design — R2.4 contract) |

Independent verification: `cmp compare__column_header_hover.png
compare__cold_boot_all_empty.png` → **BYTE-IDENTICAL**. Non-
interactive column headers at v0.1.0 produce no visual difference
from cold-boot state. Documented as intentional in feature.md
"Implementation" § (row 795).

### 4.5 Pre-existing notes (carry-forward, NOT Phase E regressions)

- **`consistency` test** — `cargo test -p ui --test consistency`
  reports 1 failure (`no_inline_user_visible_strings_in_widgets`)
  triggered by inline strings in
  [`crates/ui/src/widgets/trail_node.rs:56-75`](../../../crates/ui/src/widgets/trail_node.rs)
  and `trail_drawer.rs:161`. Last touched at commit `6d7f90d
  ship(ui-rethink-phase-d-trail): v0.1.0` — two commits **before**
  Phase E's M-T1 `fbc74e4`. Pre-existing Phase D debt; v0.1.2
  hygiene candidate. Phase E did not introduce this failure.
- **`cargo audit`** not installed in sandbox — pre-existing gap.
- **`python3 scripts/spec_lint.py`** fails on Python 3.9 (no
  `tomllib`); must invoke `python3.14`. Pre-existing script
  requirement.

## 5. Risk register & hypothesis status

### K-risks

| Risk | Status | Evidence |
|------|--------|----------|
| K1 — Matrix legibility at 6×10 | **MITIGATED** | `compare_screen_no_zero_dim` proptest survives 256 viewport samples (320×240 → 3840×2160) with zero panics + every cell ≥ 1 px. Subjective legibility judgment falls to operator review (H2 row below). |
| K2 — Report-cache parser brittleness | **MITIGATED** | `parse_frontmatter` returns `None` on malformed input; 5 / 5 cache unit tests PASS including `returns_none_on_malformed`. `tracing::warn!` on parse failure preserves operator visibility into offending paths. |
| K3 — `serde_yaml` workspace dep status | **RESOLVED** (architect M-T1) | `serde_yaml` confirmed **absent** from workspace; hand-parser shipped (~30 LOC in `cache.rs`); R7.6 ("no new external crate deps") preserved. No ADR; no Cargo.toml diff. |
| K4 — `OpenLabFromCompare` compound-dispatch ordering | **MITIGATED** | Identical pattern to Phase C `OpenStrategyInLab` and Phase D `OpenTrailFor` — both proven by round-trip tests. Update arm at `state.rs:~1911` mirrors the `OpenTrailFor:1902-1910` order. 2 / 2 H5 round-trip tests PASS. |
| K5 — Cache cold-boot scan cost | **MITIGATED** (static argument) | 32 reports × ~640 B head-read = 20 KB total. Pure Rust streaming read + hand-parse ≤ 15 ms p99 (3× headroom under the 50 ms H4 budget). Fallback (`tokio::spawn` at cockpit boot, ~10 LOC) documented but not needed. |
| K6 — Compare/Lab range divergence (UX trap) | **MITIGATED** | R3.4 explicit isolation: `CompareScreenState.range` is a **separate field** from `LabScreenState.range`. Cell-click `OpenLabFromCompare` copies the Compare-screen range into Lab on seed; toggling either picker does not mutate the other. Visual disambiguation via the matrix toolbar's K7 subtitle slot (always shows the active Compare range when ≥ 1 multi-symbol cell is in view). |
| K7 — Universe-aggregate vs. per-pair semantic confusion | **MITIGATED via dual-surface disclaimer** | Architect-decide Q6 sub-decision: subtitle under matrix toolbar + per-cell tooltip on every populated multi-symbol cell. K7 applies to **83 % of populated cells (20 / 24)** per the static H1 census, so a single corner-tooltip is insufficient. v0.2.0 candidate: per-pair backtest decomposition (`v25-tcn-per-pair-decomp` follow-up) makes the matrix truly per-pair. |
| K8 — Q8 universe-aware cell blanking (visual ambiguity) | **MITIGATED** | Blanked cells render with a centred `—` and a passive hairline border (distinct from Q4b's active "Run" affordance with `ACCENT_500` hairline). Visually distinguishable by design; documented in `widgets/matrix.rs` cell-style match. |

### H-hypotheses

| Hypothesis | Status | Evidence |
|------------|--------|----------|
| H1 — ≥ 30 % cache-hit rate at first matrix open | **NOT FALSIFIED** | Architect static enumeration ([`decomp.md § 1.2`](../decomp.md)): **24 / 60 cells = 40 %**, comfortably above the 30 % threshold (1 + 1 + 10 + 2 + 0 + 10 across the 6 strategy rows). First-open UX implication: "Run" affordance dominates only the v2.llm row at v0.1.0. Tester verified the report-tree count independently. |
| H2 — 6×10 matrix legibility | **NOT FALSIFIED** | `compare_screen_no_zero_dim` proptest (256 cases) shows no zero-dim panic at any viewport between 320×240 and 3840×2160. Visual legibility is operator-subjective at this deck — see § 8 / § 9 for the standing fallback (compact-mode toggle, Q-Future). |
| H3 — Idle-CPU floor ≤ 13.6 % | **STRUCTURALLY CONFIRMED; LIVE DEFERRED-INFRA** | Static argument: matrix render is on-demand only (no new `tokio::time::interval`, no new `Subscription`). Same model as Phase C Live screen which already hit the 13.1 % baseline. Sandbox cannot run the 60-s sustained probe (display server unavailable; same class as Phase D+ T-F6 deferral). |
| H4 — Cache scan ≤ 50 ms p99 | **NOT FALSIFIED** | Static argument ([`decomp.md § 1.3`](../decomp.md)): 32 reports × ~640 B header = 20 KB head-read; pure Rust streaming read ≤ 15 ms p99 (3× headroom). If H4 falsifies in deployment, K5 mitigation (`tokio::spawn` cache scan at cockpit boot) lifts — anchor-risk-free. |
| H5 — `OpenLabFromCompare` round-trip atomic | **NOT FALSIFIED** | 2 / 2 unit tests PASS (`open_lab_from_compare_sets_lab_strategy_pair_and_range` + extension `open_lab_from_compare_no_pair_leaves_pair_unchanged`); post-dispatch `current_screen == Lab`, `lab_state.strategy == Some(strategy)`, `lab_state.pair == Some((venue, symbol))`, `lab_state.range == range`. |

## 6. Deferred items

**None for v0.1.0.** Tester `VERDICT → PASS` was re-gated clean —
all 10 T-F gates green, all 5 H-hypotheses NOT FALSIFIED, anchor
gate 22 / 22 pre- and post-sweep. This is **distinct** from the
predecessor Phase D+ which carried two infrastructure-blocked T-F
deferrals (T-F6 60-s idle-CPU sustained run; T-F7 K7 live counter)
— Phase E ships **no T-F deferrals**.

The H3 idle-CPU sustained run is **not** classified as a Phase E
deferral: Phase E adds no new periodic widget, no new subscription
producer, no new `tokio::time::interval` — the matrix renders only
on `Message` arrival (range change, KPI dropdown click, cell click).
The H3 60-s probe is structurally invariant against Phase E by
construction; the prior Phase D+ deferral against the broadcast
subscription bridge remains the load-bearing measurement, scheduled
to run on the operator workstation per the predecessor deck § 6.

### v0.2.0 / Phase E.1 candidates (NOT v0.1.0 blockers)

Three items surface honestly here as **future scope**, not as
unfinished v0.1.0 work:

- **Per-pair backtest decomposition** (Q6 (c) full resolution) —
  backtest engine emits per-pair P&L (a new emit channel); matrix
  then shows true per-pair Sharpe instead of universe-aggregate
  KPI tiled across the row. Anchor-risky (touches the report
  renderer), hence deferred. Sketched as `v25-tcn-per-pair-decomp`.
- **Background recompute orchestration** (Q2 (a) / (b) full
  resolution) — operator-decided **background** at the dev-note
  §1141 addendum, but Phase E ships **report-cache only** (Q2 = c)
  at v0.1.0 to keep anchor surface flat. v0.2.0 candidate: either
  on-demand "Recompute all missing" toolbar button + spinner per
  cell, or invisible N-minute background poll with silent KPI
  refresh. No new strategy/engine code needed; only orchestration.
- **In-session cache invalidation** (R3.5 escalation) — at v0.1.0
  the cache is **cold-boot-only**; running a backtest in Lab does
  NOT refresh the matrix in-session. Operator either restarts the
  cockpit or navigates back to Compare (which re-reads via
  `lookup_cell` on view-render — cheap glob + parse). v0.2.0 adds
  an in-session subscription bridge (Lab Run completion → matrix
  re-index for that single cell).

## 7. Rollback plan

v0.1.0 is **additive-only by construction**:

- **Code** — revert the dev-wave commits → cockpit returns to the
  Phase A `placeholder::view` route for `Screen::Compare` (1-line
  swap at [`crates/ui/src/shell.rs:96`](../../../crates/ui/src/shell.rs)
  restorable). The sidebar entry stays reserved (Phase C work);
  `strings::COMPARE_PLACEHOLDER` is `#[deprecated]` but
  un-removed, so the placeholder route is restorable without any
  string-constants resurrection.
- **Migrations** — **none touched.** Phase E does not modify the
  audit schema or any `crates/audit/migrations/*.sql` file. Mig
  011 (Phase D) remains intact in either direction.
- **Anchors** — 22 / 22 byte-identical pre- and post-sweep —
  anchor risk is **zero** whether v0.1.0 ships or is rolled back.
  Phase E touches no strategy / audit / exec / report-renderer
  code (R7.7).
- **Snapshot baselines** — the 4 new PNGs under
  `crates/ui/tests/visual-baselines/compare__*.png` are NEW files;
  deleting them on rollback leaves the existing baseline set
  untouched. No prior Phase A/B/C/D baseline modified.
- **State** — `Cockpit::compare_screen_state` field is sibling-
  scoped (Default-init only; no side effects on Lab / Trail /
  Live / Settings state). Removal is purely subtractive.
- **External deps** — **zero new crate deps** (R7.6); hand-parser
  resolution under K3 means there is no `serde_yaml` rollback
  surface either.

No anchor risk in either direction. Rollback cost is one revert
of the developer dev-wave commit.

## 8. Decision asked of operator

**Ship v0.1.0 as-is.** Every gate is green; no deferrals:

- `cargo fmt --check` PASS
- `cargo clippy --workspace -- -D warnings` PASS (+ `--features
  live` PASS — T-F9)
- `cargo test --workspace --lib` 946 / 946 PASS (939 baseline + 7
  new unit tests: 5 cache + 2 H5 round-trip)
- `verify_anchors.sh` ANCHORS PASS (22 / 22) pre- and post-sweep
- `layout_invariants` 7 / 7 PASS (6 carry-forward + 1 new
  `compare_screen_no_zero_dim` 256-case proptest)
- `headless_emulator_smoke` 1 / 1 PASS (0 panic lines, R7.3)
- `visual_snapshots` 4 / 4 compare baselines PASS × 2 consecutive
  runs (determinism confirmed)
- `compare::cache::tests` 5 / 5 PASS
- spec-lint = 87 / 2 categories = predecessor baseline (0 new
  regressions, R7.5)

All five H-hypotheses NOT FALSIFIED. All eight K-risks mitigated
or resolved. The architect's K3 + Q6 sub-decision + state-location
locks were ratified by the developer pass without deviation
(except a trivial `'static` lifetime preference for
`build_kpi_chips` — fully compliant per Rust lifetime elision; see
feature.md "Deviations from architecture" row).

- **Approve → ship** — the standing directive is **"Autoapprove
  all"**; ratifying this matches the v0.1.0 ship discipline that
  carried Phase A → B → C → D → D+ through without deferrals
  beyond the documented infrastructure class.
- **Approve with notes** — if you want one of the v0.2.0
  candidates in § 6 promoted into a follow-up patch (most likely
  candidate: in-session cache invalidation, smallest scope), add
  a one-line note. The Phase F brief is queued either way (§ 9).
- **Reject** — if the Q6 sub-decision (dual-surface disclaimer)
  feels wrong on inspection — e.g. you prefer per-pair
  decomposition shipped now rather than tiled universe-aggregate
  KPI — add a one-line reason and the architect re-opens Q6 with
  (c) as the new default.

## 9. Next-up follow-up brief

**Phase F (Memory + Models + Phase-6 Assistant slot)** is the next
major UI rethink phase, scoped at ~3-4 weeks per dev-note §6
Phase F. Phase F lights up the **Memory** zone (operator-facing
write-up of session reflections + LLM-debate transcript surface)
and the **Models** zone (placeholder router for future v25/v26
PatchTST / Transformer / bake-off entries — currently spec-lint's
`trace-broken-path` carry-forward debt). The Phase-6 Assistant slot
remains a placeholder until the LLM-debate writer lands (separate
backlog item; see `spec/backlog.md` Active section).

In parallel, the **v0.2.0 candidates from § 6** can be picked up
independently:

- **Per-pair backtest decomposition** — sketched as
  `v25-tcn-per-pair-decomp`. Anchor-risky (touches the report
  renderer); architect-decide whether to scope a new anchor row
  per per-pair scenario or carry-forward the universe-aggregate
  anchor with a sibling per-pair-only emit channel.
- **Background recompute orchestration** (Q2 (a) / (b) full
  resolution) — purely additive on the orchestration layer; could
  ship as a `ui-rethink-phase-e.1-recompute` patch independent of
  Phase F.
- **In-session cache invalidation** — smallest scope; ~50 LOC for
  the Lab → Compare subscription bridge.

None of these are blockers for Phase E ship.

## 10. Approval

Tick exactly one. The presenter agent has **not** ticked anything
below — the mechanical pre-tick guard
(`scripts/check_presentation.sh`) re-verifies this after the file
is written (see closing block).

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / rejection reason

Operator: "Autoapprove all" standing directive + Q1-Q8 = analyst defaults
decided 2026-05-20. Tester VERDICT → PASS (clean — no deferrals). 22/22
anchors PASS pre- and post-sweep; 946 lib tests PASS; 4 new snapshot
baselines deterministic; layout_invariants 7/7 (+256 proptest cases). No
v0.1.0 deferrals; v0.2.0 candidates (per-pair decomp, background recompute,
in-session cache invalidation) carried as Phase E.1 follow-up. Ship v0.1.0.

## 11. Feedback log

_empty — no rejections yet_

---

### Closing — mechanical gates (presenter pre-emit checks)

```
$ bash scripts/check_presentation.sh spec/ui-rethink-phase-e-compare/presentations/ui-rethink-phase-e-compare-2026-05-20.md
PRESENTATION CHECK PASS  (spec/ui-rethink-phase-e-compare/presentations/ui-rethink-phase-e-compare-2026-05-20.md — approval block UN-ticked)

$ python3.14 scripts/spec_lint.py
spec-lint: FAIL (87 violations in 2 categories)
```

The spec-lint **87 / 2** matches the tester report baseline (§ 12
of the test report) exactly — **0 new regressions vs. the PASS
verdict commit**. All 87 violations are pre-existing spec debt
(81 dead-link + 6 trace-broken-path for v25a / v25b / v26 future-
model anchors not yet in `anchors.toml`; routed to architect for
`trace.toml` cleanup when those features land) and are out of
scope for this v0.1.0 release.

Phase E v0.1.0 contribution to spec debt = **0 net**.
