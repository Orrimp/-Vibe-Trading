---
title: Test Report
feature: <feature-slug>
run_id: 2026-05-06-2030-UTC
commit: 3efda6401e187db2a5bf9c21d83a0cbf862071f0
agent: tester
verdict: FAIL
---

# Test Report — <feature-slug> — 2026-05-06 20:30 UTC

> **R16.3 self-check note.** Per Brief R16.3, four brand-bleed
> tokens (the design-system name plus the three tier/elevation
> tokens listed in the gate-5 grep pattern) must not appear in
> `spec/reports/` test-/backtest- bodies. This report deliberately
> elides those four literals in prose. Task-list and feature-brief
> paths are referred to by the placeholder `<feature-slug>` (or
> `<phase-1-slug>` / `<phase-2-slug>` / `<phase-3-slug>` /
> `<master-roadmap>` as appropriate) wherever the literal slug
> would otherwise leak into report content. The four-token grep
> regex itself is never reproduced as a contiguous string in this
> report body — § 2 / § 7 / § 8 refer to it as "the Brief R16.3
> four-token grep" instead, the same elision pattern Phase 1 / 2 / 3
> testers used.

## 1. Scope

- **Feature / change under test:** Phase 4 Backtest panel — new
  `viewer` bin (KPI strip + equity curve + drawdown band +
  markdown body, CLI-arg-driven), the cross-phase
  `core::EquitySeries` + `BacktestMetrics` primitives, the
  additive `audit::query::equity_curve_for_strategy` sibling of
  `pnl_by_strategy`, the new `crates/reports/src/parse.rs`
  markdown summary parser, the four canvas widget modules
  (`widgets::canvas_chart` core + `kpi_strip` + `equity_curve` +
  `drawdown_band` + `sparkline`), and the cockpit Strategies-
  detail sparkline that closes the Phase 3 Q6 deferral.
- **Spec refs:** `spec/features/<feature-slug>.md`,
  `spec/tasks/<feature-slug>.md`,
  `spec/features/<master-roadmap>.md`.
- **Commit SHA:** `3efda6401e187db2a5bf9c21d83a0cbf862071f0`
  (worktree carries uncommitted Phase 4 edits per task-list
  T1801–T1815 + T1812 ui-designer attestation sub-block + T1815
  rustdoc/workspace addendum).
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`.
- **OS / arch:** `Darwin 25.4.0 arm64` (M-series).
- **Predecessor reports:**
  - Phase 1 third-pass PASS:
    `spec/reports/test-2026-05-04c-<phase-1-slug>.md`.
  - Phase 2 first-pass PASS:
    `spec/reports/test-2026-05-05-<phase-2-slug>.md`.
  - Phase 3 first-pass PASS:
    `spec/reports/test-2026-05-05-<phase-3-slug>.md`.
  - This is the **first** tester pass for Phase 4 — the
    orchestrator pre-cleared the rustdoc gate before tester spawn
    and the workspace-wide `cargo test` invocation, but Gate 3
    (rust-validate clippy) **does not converge** on tester re-run.

## 2. Static Analysis

| Check | Result | Notes |
|-------|--------|-------|
| `cargo fmt --all -- --check` | PASS | exit 0, zero diff. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **FAIL** | 1 hard error: `clippy::match_same_arms` fires on the new sparkline dispatch in `crates/ui/src/screens/strategies.rs:150` ↔ `:161` (both arms return `muted_body(STRATEGIES_SPARKLINE_LOADING)`). `error: could not compile \`ui\` (lib) due to 1 previous error`. Developer's T1815 tick block claims `cargo clippy ... → clean (zero warnings)`; tester's clean re-run from project root contradicts. See § 3.1 below for the verbatim excerpt and § 8 for verdict justification. |
| `cargo audit` | N/A (not installed) | `cargo audit` not on PATH (`error: no such command: 'audit'`); same handling as Phase 1 / 2 / 3 reports. Coverage gap is bridged by `cargo deny check` (`[advisories]` table v2 against the same RustSec DB) which PASSES. |
| `cargo deny check` | PASS | `advisories ok, bans ok, licenses ok, sources ok` — independent re-run from project root. |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS | Tester re-ran cleanly from project root after `rm -rf target/doc`: `Finished dev profile [unoptimized + debuginfo] target(s) in 15.76s`; `Generated … target/doc/agent/index.html and 16 other files`. Zero warnings, zero errors. (Independent verification of the orchestrator's pre-clear at `Finished dev profile … in 13.49s`.) |

### 2.1 Clippy failure excerpt (verbatim)

```
error: these match arms have identical bodies
   --> crates/ui/src/screens/strategies.rs:150:9
    |
150 |         (Some(_), Some(PanelState::Loading) | None) => muted_body(STRATEGIES_SPARKLINE_LOADING),
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
161 |         (None, _) => muted_body(STRATEGIES_SPARKLINE_LOADING),
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = help: if this is unintentional make the arms return different values
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.94.0/index.html#match_same_arms
    = note: `-D clippy::match-same-arms` implied by `-D warnings`
    = help: to override `-D warnings` add `#[allow(clippy::match_same_arms)]`
help: otherwise merge the patterns into a single arm
    |
150 ~         (Some(_), Some(PanelState::Loading) | None) | (None, _) => muted_body(STRATEGIES_SPARKLINE_LOADING),
151 |         (Some(_), Some(PanelState::Empty | PanelState::Ready(_))) => {
...
160 |         }
161 ~         };
    |

error: could not compile `ui` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
error: could not compile `ui` (lib test) due to 1 previous error
```

The lint suggests merging the two `STRATEGIES_SPARKLINE_LOADING`
arms into one combined pattern (or annotating with
`#[allow(clippy::match_same_arms)]` at the call-site if the
divergence is intentional). The five-arm dispatch at T1811's
narrative explicitly enumerates `(Some(_), Some(Loading) | None)`
**and** `(None, _)` separately for documentation reasons — the
two cases are semantically different (selected-strategy + still-
loading versus no-strategy-selected) but render to the same
muted-body line. Either merge is acceptable per the architect
contract; the developer needs to pick one.

## 3. Unit & Integration Tests

`cargo test --workspace --all-targets` — exit 0, all suites green.

| Metric | Value |
|---|---|
| Test binaries run | 108 |
| Tests passed | **850** |
| Tests failed | **0** |
| Tests ignored | 3 |
| Failing tests | _none_ |

Phase 3 → Phase 4 delta: **+40 tests / +4 binaries** (810/104 →
850/108) per the architect's Phase 4 net-new test budget.

### Spotlight tests (per Brief V-items + Phase 4 net-new tests)

| Test | Result | Output line |
|------|--------|-------------|
| `core::equity_series::tests::*` (T1801 / V1) | PASS | 8 tests inside `trading_core` lib pass — `from_points_computes_drawdown_correctly`, `from_points_monotone_up_returns_all_zero_drawdown`, `from_points_50_percent_drawdown_then_recovery`, `from_points_empty_returns_err`, `from_points_non_monotone_returns_err`, `downsample_below_target_is_noop`, `downsample_to_2000_preserves_peak_and_trough`, `downsample_preserves_first_and_last_point`. |
| `audit::query::tests::equity_curve_for_strategy_*` (4 unit tests, T1802 / V2) | PASS | `equity_curve_for_strategy_returns_window_samples`, `..._empty_window_returns_empty_window_err`, `..._until_none_includes_to_now`, `..._filters_by_strategy_id` — all 4 inside `audit` lib. |
| `audit::tests::equity_curve_for_strategy` (2 integration tests, T1802) | PASS | `equity_curve_for_strategy_multi_day_round_trip`, `..._strategy_isolation_multi_day` — `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| `reports::parse::tests::*` (≥ 5 tests, T1808 / V5) | PASS | 7 tests inside `reports` lib including `all_anchored_reports_parse_ok` (asserts every committed `backtest-*.md` parses Ok). |
| `widgets::canvas_chart::tests::*` (5 tests, T1804 / V4) | PASS | `gridlines_emit_5_horizontal_lines`, `inner_rect_applies_gutter`, `inner_rect_clamps_negative_dims_to_zero`, `polyline_with_fill_alpha_emits_filled_polygon`, `polyline_with_fill_zero_alpha_emits_stroke_only`. |
| `widgets::kpi_strip::tests::*` (2 snapshot tests, T1805 / V6) | PASS | `kpi_strip__sample_report`, `kpi_strip__metrics_unavailable`. |
| `widgets::equity_curve::tests::*` (2 snapshot tests, T1806 / V7) | PASS | `equity_curve__sample_report`, `equity_curve__no_equity_data`. |
| `widgets::drawdown_band::tests::*` (1 snapshot test, T1807 / V8) | PASS | `drawdown_band__sample_report`. |
| `widgets::sparkline::tests::*` (1 snapshot test, T1809 / V11) | PASS | `sparkline__120pt`. |
| `state::tests::strategy_equity_*` (T1801 / V3) | PASS | `strategy_equity_refresh_inserts_ready_panel_state`, `strategy_equity_refresh_err_inserts_error_panel_state`. |
| `viewer` bin unit tests (T1803 + T1810) | PASS | `cli_parser_accepts_report_path`, `cli_parser_rejects_no_args`, `cli_help_renders_without_<brand>` (test name carries the design-system literal verbatim — elided here per § R16.3 self-check; the test asserts the viewer's `--help` output does not contain the brand string per Constraint 1), `parse_front_matter_extracts_scenario` — `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| `viewer_read_only` integration (T1810 / V9) | PASS | `viewer_bin_is_read_only_on_spec_tree` — `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| `strategies_screen_sparkline_replaces_placeholder` integration (T1811 / V12) | PASS | binary built clean (5 unused-import warnings noted but non-fatal under `cargo test`); `running 0 tests` line in `--all-targets` log indicates the test cases live elsewhere; placeholder-replacement coverage is exercised by the panel snapshot `strategies_screen__sparkline_present` which passes. |
| `panel_snapshots` suite (55 tests, T1812 net-new + carry-over) | PASS | `test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s` (Phase 3 ended at 54; Phase 4 net = +1 `viewer__full_view__sample_report` + `strategies_screen__sparkline_present` -1 retired `…__sparkline_deferred`). |
| Widget-side snapshots (84 tests including 6 net-new) | PASS | `test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s` — `kpi_strip__sample_report` + `…__metrics_unavailable` + `equity_curve__sample_report` + `…__no_equity_data` + `drawdown_band__sample_report` + `widgets__sparkline__120pt`. |
| Cross-feature: `live_subscription_full_bus` (V14) | PASS | `t911_full_bus_drives_every_panel_out_of_loading`, `t911_kill_button_round_trip_via_mode_forwarder` — 2 passed. |
| Cross-feature: `cockpit_live_modal_metadata_chain` (V14) | PASS | 2 passed. |
| Cross-feature: `tape_row_click_opens_modal` (V14) | PASS | 8 passed. |
| Cross-feature: `recent_fills_filtered` (v1.5b multi-venue) | PASS | 4 passed. |

### Failing Tests

_none — `cargo test --workspace --all-targets` exit 0, 850 / 0 / 3._

## 4. Property / Fuzz Tests

`proptest` is present in `core` (positive-qty test). Default-feature
run already exercises it (`prop_positive_qty_accepted` PASS in
`trading_core` lib). Larger budget run (`PROPTEST_CASES=1024`) was
not re-executed in this gate pass — the workspace is read-only-by-
tester and the brief's 8-gate matrix did not request a budget bump
(same handling as Phase 1 / 2 / 3 testers).

| Suite | Cases | Shrunk failures | Seed |
|---|---:|---:|---|
| `core::prop_positive_qty_accepted` (default budget) | 256 | 0 | system-default |

## 5. Backtest Results

_n/a — Phase 4 is read-only over committed `backtest-*.md` bodies +
companion `*__equity.csv` artefacts + an additive read-only
`audit::query::equity_curve_for_strategy` sibling of `pnl_by_strategy`.
No `crates/strategy/`, `crates/exec/`, `crates/backtest/`,
`crates/cost/`, or `crates/reports/src/render/` rendering code
touched. The 11 body-SHA-256 backtest anchors are verified
byte-identical via Gate 4 below._

## 6. Benchmarks

_n/a — Phase 4 changes are screen-modules / additive query / new
markdown parser / new viewer bin only; no hot-path code touched (no
order-book, feature-calc, or inference changes). The
`equity_curve_for_strategy` SQL is bounded by the existing
`journal_entries(ts)` index path; no new index._

## 7. Environment / Infrastructure Issues

- `cargo audit` not installed on PATH. Per the rust-validate skill:
  "Install with `cargo install cargo-audit` if missing; ask the
  user before installing." Tester role does not auto-install.
  Coverage gap is bridged by `cargo deny check`
  (`[advisories]` v2 against the same RustSec DB) which PASSES.
- The Brief R16.3 four-token grep run against `spec/reports/`
  (untargeted) returns matches only in
  `spec/<slug>/reports/screenshots/<phase-1-slug>/README.md`,
  `spec/<slug>/reports/screenshots/<phase-2-slug>/README.md`,
  `spec/<slug>/reports/screenshots/<phase-3-slug>/README.md`, and
  `spec/<slug>/reports/screenshots/<phase-4-slug>/README.md` if present
  (screenshot-manifest titles carried forward as accepted state —
  filename context, not body content). The targeted grep against
  `--include='test-*.md' --include='backtest-*.md'` returns
  **zero matches** (exit 1) — i.e. no NEW report-body drift.
  Self-check on this report file: brand-bleed tokens absent (see
  prelude / R16.3 self-check note).
- No flaky tests observed; runtime is determinism-stable on the
  M-arm Darwin host.
- Five `unused_imports` warnings emitted by the
  `strategies_screen_sparkline_replaces_placeholder` integration
  test under `cargo test --workspace --all-targets`
  (`trading_core::StrategyId`, `fake_cockpit_v15a_pairs_steady_state`,
  `fake_equity_series_for_sparkline`, `ui::screens::strategies`,
  `PanelState`, `Screen`, `ui::theme::ThemeMode`). Test PASSES
  under `cargo test` (warnings are non-fatal there) but
  contributes to Gate 3 clippy noise — these will be promoted to
  errors under `cargo clippy --all-targets -- -D warnings` once
  the developer fixes the primary `match_same_arms` failure
  detailed in § 2.1, and should be addressed in the same fix pass.

## 8. Verdict

**`FAIL`**

7 of the 8 gates pass; **Gate 3 (`rust-validate`) FAILS** on
clippy under `-D warnings`. The new sparkline dispatch in
`crates/ui/src/screens/strategies.rs:140–162` (T1811) trips the
`clippy::match_same_arms` lint at lines 150 and 161 — both arms
return `muted_body(STRATEGIES_SPARKLINE_LOADING)`. Compilation of
`ui` (lib) and `ui` (lib test) targets fails under
`-D warnings`; the workspace cannot ship in this state.

The developer's T1815 tick block claims
`cargo clippy --workspace --all-targets --all-features -- -D
warnings` → "clean (zero warnings)". Tester's clean re-run from
project root contradicts that line — the lint is reproducible on
a fresh `cargo clippy` invocation against the current worktree.
This is a developer-side regression that must be fixed before
ship.

The remaining seven gates pass cleanly:

### Gate-by-gate summary

| # | Gate | Result | Note |
|---|------|--------|------|
| 1 | Honest-tick audit (T1801–T1815 + T1812 ui-designer attestation sub-block + T1815 rustdoc/workspace addendum) | PASS | All 15 task ticks at task-list lines 131 / 213 / 281 / 341 / 398 / 463 / 509 / 549 / 613 / 653 / 726 / 827 / 1185 / 1246 / 1278 carry file:line + test cmd + output. T1812 visual-diff attestation sub-block at task-list line 923 carries the `_ticked 2026-05-06 (ui-designer)._` signature and lists 8 sample-attested baselines + Phase 3 deferral closure verification + full-inventory verification (72 baselines visually scanned, 0 deviations) + `unknown`-color sweep (zero unmapped, one legitimate `Latency::Unknown` badge) + Q1 / Q2 / Q3 / Q6 / Q8 / Q9 / Q10 / Q11 evidence rollup. T1815 sub-bullet (task-list lines 1313–1351) documents the orchestrator-run rustdoc gate (`Finished dev profile … in 13.49s` after fixing 4 intra-doc links) AND the orchestrator-run workspace-wide test invocation (`108 test binaries / 850 tests passed / 0 failed / 3 ignored`). |
| 2 | `cargo test --workspace --all-targets` | PASS | All suites green: **850 passed, 0 failed, 3 ignored** across **108 test binaries**. Phase 4 net-new tests verified individually via spotlight table in § 3 — `equity_series::tests` 8/8, `state::tests::strategy_equity_*` 2/2, `audit::query::tests::equity_curve_for_strategy_*` 4/4, `audit::tests::equity_curve_for_strategy` 2/2, `reports::parse::tests` 7/7 (incl. `all_anchored_reports_parse_ok`), `widgets::canvas_chart::tests` 5/5, `widgets::kpi_strip` 2/2 snapshot, `widgets::equity_curve` 2/2 snapshot, `widgets::drawdown_band` 1/1 snapshot, `widgets::sparkline` 1/1 snapshot, `viewer` bin unit 4/4, `viewer_read_only` integration 1/1, `panel_snapshots` 55/55 (incl. net-new `viewer__full_view__sample_report` + `strategies_screen__sparkline_present`; deleted `…__sparkline_deferred`). Phase 1 / 2 / 3 invariants preserved. |
| 3 | `rust-validate` (fmt + clippy + cargo-deny + audit + docs) | **FAIL** | fmt PASS (exit 0, zero diff); **clippy FAIL** (`error: these match arms have identical bodies` at `crates/ui/src/screens/strategies.rs:150` ↔ `:161`; lint = `clippy::match_same_arms` implied by `-D warnings`; `error: could not compile \`ui\` (lib) due to 1 previous error`); deny PASS (`advisories ok, bans ok, licenses ok, sources ok`); audit N/A (not installed; deny advisories cover); rustdoc PASS (`Finished dev profile [unoptimized + debuginfo] target(s) in 15.76s` after `rm -rf target/doc`). |
| 4 | `bash scripts/verify_anchors.sh` | PASS | `ANCHORS PASS  (11 / 11)` — all 11 body-SHA-256s byte-identical to `spec/anchors.toml`. Phase 4 reads from committed bodies + companion CSVs read-only; the audit query addition is read-only over `journal_entries`; the `viewer` bin's read-only-on-spec-tree assertion (T1810 / T1814) holds. |
| 5 | R16.3 brand-bleed grep on `spec/reports/` | PASS | Targeted grep `grep -rni "<grep-pattern>" spec/reports/ --include='backtest-*.md' --include='test-*.md'` exit 1 (zero matches in test- and backtest- report bodies). Self-check on this file: zero matches in body text (verified per the prelude elision contract). |
| 6 | Cross-feature invariants 7/7 PASS | PASS | Tester independently re-ran each prior feature's named test: (1) `operator-success-reports` — `cargo test -p reports csv_artifacts::tests --lib` → `4 passed`; (2) `live-cockpit-unified` — `cargo test -p ui --features live --test live_subscription_full_bus` → `2 passed`; (3) `real-mtm-unrealized-pnl` — `cargo test -p ui --lib widgets::pnl` → `0 passed; 0 failed; 0 ignored; 84 filtered out` (P&L card has no unit tests in `widgets::pnl::tests`; surface unchanged confirmed via panel `pnl_*` baselines remaining byte-identical at Gate 7); (4) `per-symbol-position-accounts` — `cargo test -p audit --lib query::tests::position` → `0 passed; 13 filtered out` (sibling read path `recent_fills_filtered` 4/4 PASS); (5) `tape-row-audit-modal` — `cargo test -p ui --features fixtures --test tape_row_click_opens_modal` → `8 passed`; (6) `journal-tx-metadata` — `cargo test -p ui --features live --test cockpit_live_modal_metadata_chain` → `2 passed`; (7) `v1.5b-multi-venue` — `cargo test -p audit --lib query::tests::recent_fills_filtered` → `4 passed`. Identical evidence pattern to T1813's developer block. |
| 7 | Snapshot baselines clean | PASS | `find crates/ui/tests/snapshots crates/ui/src/widgets/snapshots -name '*.pending-snap' -o -name '*.snap.new'` returns empty (exit 0). Total `*.snap` baseline count: **72** (55 in `crates/ui/tests/snapshots/` panel-side + 17 in `crates/ui/src/widgets/snapshots/` widget-side) — matches the ui-designer attestation count exactly. Phase 3 → Phase 4 delta: 65 → 72 baselines (+7 net = 8 net-new − 1 deletion: 6 widget-side `kpi_strip__sample_report`, `…__metrics_unavailable`, `equity_curve__sample_report`, `…__no_equity_data`, `drawdown_band__sample_report`, `widgets__sparkline__120pt` + 2 panel-side `viewer__full_view__sample_report`, `strategies_screen__sparkline_present` − 1 panel-side `strategies_screen__sparkline_deferred` retired). |
| 8 | Visual-diff attestation by ui-designer | PASS | T1812 visual-diff attestation sub-block at task-list line 923 carries the `_ticked 2026-05-06 (ui-designer)._` signature and enumerates: (a) 8 sample-attested baselines (`…__kpi_strip__sample_report`, `…__kpi_strip__metrics_unavailable`, `…__equity_curve__sample_report`, `…__drawdown_band__sample_report`, `…__sparkline__120pt`, `…__strategies_screen__sparkline_present`, `…__viewer__full_view__sample_report`, Phase 2 carry-forward `chart__btc_with_two_buys_one_sell`); (b) **Phase 3 deferral closure verification** — `STRATEGIES_SPARKLINE_DEFERRED` constant fully removed from `crates/ui/src/strings.rs` (only a doc-comment reference inside the new `STRATEGIES_SPARKLINE_LOADING` block remains); `panel_snapshots__strategies_screen__sparkline_deferred.snap` deleted; (c) full-inventory verification — all 72 baselines visually scanned, the 64 carry-forward Phase 1/2/3 baselines confirmed shape-stable; (d) `unknown`-color sweep — one legitimate `Latency::Unknown` badge match, zero unmapped-token escapes, every Phase 4 token (ACCENT, UP_500, DOWN_500, FG_1/FG_2/FG_3, BORDER_1, the elevation tokens) maps cleanly; zero inline hex in the 8 net-new baselines; (e) Q1 / Q2 / Q3 / Q6 / Q8 / Q9 / Q10 / Q11 evidence rollup — Q1 via `points: 60` + `max_dd: 0.57805` matching byte-for-byte across curve / band / full-view baselines; Q2 via Phase 2's chart baseline byte-identical under shared `canvas_chart` core; Q3 via `cagr: —` + `win_rate: —` graceful-fallback cells; Q6 via solid `@ 0.18` fills (no gradient); Q8 via sparkline placement at top-right of chip row; Q9 via `points: 120` cap; Q11 via 5-net-new + 1-deletion baseline budget (8/1 expansion is architect-intended — 3 helper empty-state baselines for widget-internal coverage). Architect Phase 4 contract preserved end-to-end. |

## 9. Routing

`HANDOFF → developer (gate FAIL — see report § 2.1 / § 8 Gate 3)`
— the new sparkline dispatch in `crates/ui/src/screens/strategies.rs`
trips `clippy::match_same_arms` under `-D warnings`. Two
acceptable fix paths per the lint's own help text:

1. **Merge the two arms** that both return
   `muted_body(STRATEGIES_SPARKLINE_LOADING)` into a combined
   pattern: `(Some(_), Some(PanelState::Loading) | None) | (None, _)
   => muted_body(STRATEGIES_SPARKLINE_LOADING)` — clippy's own
   `help: otherwise merge the patterns into a single arm` block
   spells out the exact mechanical rewrite at lines 150 / 161.
2. **Annotate with `#[allow(clippy::match_same_arms)]`** at the
   call-site if the developer wants to preserve the documentation
   intent of two distinct semantic arms (selected-strategy +
   loading-or-missing-equity-state versus no-strategy-selected)
   collapsing to the same UX line.

Either is consistent with the architect's T1811 contract — the
contract names a five-arm dispatch but does not pin the
clippy-collapse choice. While in the file, the developer should
also clean up the five `unused_imports` warnings in
`crates/ui/tests/strategies_screen_sparkline_replaces_placeholder.rs:11–15`
flagged by `cargo test --workspace --all-targets` (see § 7) —
those will fire under `-D warnings` once the primary error is
fixed.

After the fix, re-run `cargo clippy --workspace --all-targets
--all-features -- -D warnings` to confirm zero errors / zero
warnings, then route back to tester for ratification.

``T_FINAL_<phase-4-tag>`` LEFT UN-TICKED. Phase 4 brief frontmatter
NOT bumped to `shipped`. Presenter NOT spawned.

`HANDOFF → developer` — gate FAIL (Gate 3 / `rust-validate`
clippy).
