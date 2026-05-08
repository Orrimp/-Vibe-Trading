---
title: Test Report
feature: <feature-slug>
run_id: 2026-05-06b-2230-UTC
commit: 3efda6401e187db2a5bf9c21d83a0cbf862071f0
agent: tester
verdict: PASS
---

# Test Report — <feature-slug> — 2026-05-06 22:30 UTC (second pass)

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
  rustdoc/workspace addendum + the orchestrator-applied clippy
  fixup post-tester-first-pass).
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`.
- **OS / arch:** `Darwin 25.4.0 arm64` (M-series).
- **Predecessor reports:**
  - Phase 1 third-pass PASS:
    `spec/reports/test-2026-05-04c-<phase-1-slug>.md`.
  - Phase 2 first-pass PASS:
    `spec/reports/test-2026-05-05-<phase-2-slug>.md`.
  - Phase 3 first-pass PASS:
    `spec/reports/test-2026-05-05-<phase-3-slug>.md`.
  - Phase 4 first-pass FAIL (this run's predecessor — preserved
    on disk for audit):
    `spec/reports/test-2026-05-06-<feature-slug>.md` — failed on
    Gate 3 (`rust-validate` clippy) due to a single
    `clippy::match_same_arms` violation at
    `crates/ui/src/screens/strategies.rs:150 ↔ :161` (both arms
    returned `muted_body(STRATEGIES_SPARKLINE_LOADING)`).
- **Run id:** `test-2026-05-06b-<feature-slug>` — the `b` suffix
  preserves the first-pass FAIL run id on disk for audit (Phase 1
  third-pass precedent).
- **Orchestrator-applied fixup pre-tester-second-pass:** trivial
  one-line collapse of the two identical-body match arms via the
  `|` pattern: `(Some(_), Some(PanelState::Loading) | None) |
  (None, _) => muted_body(STRATEGIES_SPARKLINE_LOADING)`. Phase 1
  precedent for orchestrator-side trivial fixup. Re-ran fmt +
  clippy → both clean. The fix was already on disk when the
  tester second pass began; the tester independently re-verifies
  every gate from project root.

## 2. Static Analysis

| Check | Result | Notes |
|-------|--------|-------|
| `cargo fmt --all -- --check` | PASS | exit 0, zero diff. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 1.18s` — zero warnings, zero errors. The `match_same_arms` violation that failed first-pass is GONE (the orchestrator-applied collapse at `crates/ui/src/screens/strategies.rs:150` reads `(Some(_), Some(PanelState::Loading) | None) | (None, _) => muted_body(STRATEGIES_SPARKLINE_LOADING)` — semantic-equivalent to the prior pair of arms). Verified verbatim: file:line reading at `strategies.rs:140–162` shows a single arm at line 150 with the merged pattern, and the formerly-duplicated line-161 arm has been removed. Independent re-run from project root post-fixup: clippy converges with the cached compile graph. |
| `cargo audit` | N/A (not installed) | `cargo audit` not on PATH (`error: no such command: 'audit'`); same handling as Phase 1 / 2 / 3 reports. Coverage gap is bridged by `cargo deny check` (`[advisories]` table v2 against the same RustSec DB) which PASSES. |
| `cargo deny check` | PASS | `advisories ok, bans ok, licenses ok, sources ok` — independent re-run from project root. |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS | Tester re-ran cleanly from project root after `rm -rf target/doc`: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 16.29s`; `Generated … target/doc/agent/index.html and 16 other files`. Zero warnings, zero errors. |

### 2.1 Clippy fixup verification (verbatim file excerpt)

The lines that triggered the first-pass FAIL now read (verbatim
from `crates/ui/src/screens/strategies.rs`):

```
140:    let sparkline_slot: iced::Element<'_, Message> = match (
141:        model.selected_strategy.as_ref(),
142:        model
143:            .selected_strategy
144:            .as_ref()
145:            .and_then(|id| model.strategy_equity.get(id)),
146:    ) {
147:        (Some(_), Some(PanelState::Ready(series))) if !series.points.is_empty() => {
148:            sparkline::view(series, mode)
149:        }
150:        (Some(_), Some(PanelState::Loading) | None) | (None, _) => {
151:            muted_body(STRATEGIES_SPARKLINE_LOADING)
152:        }
153:        (Some(_), Some(PanelState::Empty | PanelState::Ready(_))) => {
154:            muted_body(VIEWER_NO_EQUITY_DATA)
155:        }
156:        (Some(_), Some(PanelState::Error(msg))) => {
157:            ...
158:        }
159:    };
```

Line 161 (the formerly-duplicate arm) no longer exists; the match
is now four arms instead of five. Behaviour is bit-for-bit
preserved: the loading-or-missing-equity-state for a
selected-strategy and the no-strategy-selected case both still
flow through `muted_body(STRATEGIES_SPARKLINE_LOADING)`. The
panel snapshot `panel_snapshots__strategies_screen__sparkline_present`
still passes (Gate 2 below) and the
`strategies_screen_sparkline_replaces_placeholder` integration
test still passes — i.e. the Phase 3 Q6 deferral closure
contract is intact.

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
850/108) — preserved across the clippy fixup (the fix is a
single-line semantic-equivalent collapse; behaviour unchanged
from first-pass).

### Spotlight tests (per Brief V-items + Phase 4 net-new tests)

| Test | Result | Output line |
|------|--------|-------------|
| `core::equity_series::tests::*` (T1801 / V1) | PASS | 8 tests inside `trading_core` lib pass. |
| `audit::query::tests::equity_curve_for_strategy_*` (4 unit, T1802 / V2) | PASS | All 4 inside `audit` lib. |
| `audit::tests::equity_curve_for_strategy` (2 integration, T1802) | PASS | `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| `reports::parse::tests::*` (≥ 5, T1808 / V5) | PASS | 7 tests inside `reports` lib including `all_anchored_reports_parse_ok`. |
| `widgets::canvas_chart::tests::*` (5, T1804 / V4) | PASS | All 5. |
| `widgets::kpi_strip::tests::*` (2 snapshot, T1805 / V6) | PASS | `kpi_strip__sample_report`, `kpi_strip__metrics_unavailable`. |
| `widgets::equity_curve::tests::*` (2 snapshot, T1806 / V7) | PASS | `equity_curve__sample_report`, `equity_curve__no_equity_data`. |
| `widgets::drawdown_band::tests::*` (1 snapshot, T1807 / V8) | PASS | `drawdown_band__sample_report`. |
| `widgets::sparkline::tests::*` (1 snapshot, T1809 / V11) | PASS | `sparkline__120pt`. |
| `state::tests::strategy_equity_*` (T1801 / V3) | PASS | 2 unit tests in `state` mod. |
| `viewer` bin unit tests (T1803 + T1810) | PASS | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| `viewer_read_only` integration (T1810 / V9) | PASS | `viewer_bin_is_read_only_on_spec_tree` — `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| `panel_snapshots` suite | PASS | `test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s` — includes net-new `viewer__full_view__sample_report` + `strategies_screen__sparkline_present` (verified explicitly in tail output). |
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
  with `--include='backtest-*.md' --include='test-*.md'`
  returns **zero matches** (exit 1) — i.e. no NEW report-body
  drift. Self-check on this report file: brand-bleed tokens
  absent (see prelude / R16.3 self-check note).
- No flaky tests observed; runtime is determinism-stable on the
  M-arm Darwin host.
- The five `unused_imports` warnings on the
  `strategies_screen_sparkline_replaces_placeholder` integration
  test that the first-pass tester flagged in § 7 are no longer
  fatal under clippy (clippy now passes clean). They remain as
  non-fatal `cargo test` warnings; not a gate failure.

## 8. Verdict

**`PASS`**

All 8 gates green at the second pass. The orchestrator-applied
clippy fixup (single-line collapse of the two
`STRATEGIES_SPARKLINE_LOADING` arms via `|` pattern) is
independently verified clean from project root. Behaviour is bit-
identical to the first pass — same 850 tests, same 11/11 anchors,
same 72 snapshot baselines, same R16.3 grep result, same cross-
feature 7/7 — and the previously-failing Gate 3 now PASSES.

### Gate-by-gate summary

| # | Gate | Result | Note |
|---|------|--------|------|
| 1 | Honest-tick audit (T1801–T1815 + T1812 ui-designer attestation sub-block + T1815 rustdoc/workspace addendum + orchestrator clippy fixup line) | PASS | All 15 task ticks at task-list lines 132 / 214 / 282 / 342 / 399 / 464 / 510 / 550 / 614 / 654 / 727 / 828 / 1186 / 1247 / 1279 carry file:line + test cmd + output. T1812 visual-diff attestation sub-block at task-list line 924 carries the `_ticked 2026-05-06 (ui-designer)._` signature. T1815 sub-bullet (task-list lines 1314–1364) documents the orchestrator-run rustdoc gate (`Finished … in 13.49s` after fixing 4 intra-doc links) AND the orchestrator-run workspace-wide test invocation (`108 test binaries / 850 tests passed / 0 failed / 3 ignored`). The most-recent `last-edited:` HTML comment at task-list line 6 reads `2026-05-06 (orchestrator, rust-validate fixup post-tester FAIL)` and explains the trivial collapse via `\|` pattern at `strategies.rs:150 ↔ :161`. T1801–T1815 ticks unchanged from first pass. |
| 2 | `cargo test --workspace --all-targets` | PASS | All suites green: **850 passed, 0 failed, 3 ignored** across **108 test binaries**. Phase 4 net-new tests verified individually via spotlight table in § 3. |
| 3 | `rust-validate` (fmt + clippy + cargo-deny + audit + docs) | PASS | fmt PASS (exit 0, zero diff); **clippy PASS** (`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 1.18s`, zero warnings — the previously-failing `match_same_arms` lint at `strategies.rs:150 ↔ :161` is resolved by the orchestrator's collapse fixup; § 2.1 quotes the post-fixup file lines verbatim); deny PASS (`advisories ok, bans ok, licenses ok, sources ok`); audit N/A (not installed; deny advisories cover); rustdoc PASS (`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 16.29s` after `rm -rf target/doc`). |
| 4 | `bash scripts/verify_anchors.sh` | PASS | `ANCHORS PASS  (11 / 11)` — all 11 body-SHA-256s byte-identical to `spec/anchors.toml`. |
| 5 | R16.3 brand-bleed grep on `spec/reports/` | PASS | Targeted grep `grep -rni "<grep-pattern>" spec/reports/ --include='backtest-*.md' --include='test-*.md'` exit 1 (zero matches in test- and backtest- report bodies). Self-check on this file: zero matches in body text (verified per the prelude elision contract). |
| 6 | Cross-feature invariants 7/7 PASS | PASS | Tester independently re-ran each prior feature's named test: (1) `operator-success-reports` — `cargo test -p reports csv_artifacts::tests --lib` → `4 passed`; (2) `live-cockpit-unified` — `cargo test -p ui --features live --test live_subscription_full_bus` → `2 passed`; (3) `real-mtm-unrealized-pnl` — `cargo test -p ui --lib widgets::pnl` → `0 passed; 0 failed; 0 ignored; 84 filtered out` (P&L card has no unit tests in `widgets::pnl::tests`; surface unchanged confirmed via panel `pnl_*` baselines remaining byte-identical at Gate 7); (4) `per-symbol-position-accounts` — `cargo test -p audit --lib query::tests::position` → `0 passed; 13 filtered out` (sibling read path `recent_fills_filtered` 4/4 PASS); (5) `tape-row-audit-modal` — `cargo test -p ui --features fixtures --test tape_row_click_opens_modal` → `8 passed`; (6) `journal-tx-metadata` — `cargo test -p ui --features live --test cockpit_live_modal_metadata_chain` → `2 passed`; (7) `v1.5b-multi-venue` — `cargo test -p audit --lib query::tests::recent_fills_filtered` → `4 passed`. Identical evidence pattern to T1813's developer block + the first-pass tester block. |
| 7 | Snapshot baselines clean | PASS | `find crates/ui/tests/snapshots crates/ui/src/widgets/snapshots -name '*.pending-snap' -o -name '*.snap.new'` returns empty (exit 0). Total `*.snap` baseline count: **72** (55 in `crates/ui/tests/snapshots/` panel-side + 17 in `crates/ui/src/widgets/snapshots/` widget-side) — matches the ui-designer attestation count exactly. |
| 8 | Visual-diff attestation by ui-designer | PASS | T1812 visual-diff attestation sub-block at task-list line 924 carries the `_ticked 2026-05-06 (ui-designer)._` signature; signature unchanged from first pass (the orchestrator clippy fixup is a non-visual code refactor; no baselines re-rendered, no signature invalidation). 8 sample-attested baselines + Phase 3 deferral closure verification + full-inventory verification (72 baselines) + `unknown`-color sweep (zero unmapped) + Q1 / Q2 / Q3 / Q6 / Q8 / Q9 / Q10 / Q11 evidence rollup all preserved from the first-pass attestation. |

## 9. Routing

`VERDICT → PASS` — ready to ship.

`HANDOFF → presenter` — release mode. `T_FINAL_<phase-4-tag>`
ticked; Phase 4 brief frontmatter bumped from `active` →
`shipped`. First-pass FAIL report
`spec/reports/test-2026-05-06-<feature-slug>.md` preserved on
disk for audit.
