---
slug: lumen-phase-4-backtest-panel
mode: release
date: 2026-05-06
agent: presenter
verdict: APPROVED
approved_by: operator
approved_at: 2026-05-06
---

# Phase 4 — Backtest panel · Sprint review

## TL;DR

A new **`viewer`** binary renders backtest reports with a **KPI strip + equity curve + drawdown band** above the existing markdown body — the offline review surface that's been missing since v0. Phase 3's deferred per-strategy equity sparkline lands too, on the shared `core::EquitySeries` primitive. Tester second-pass PASS (one trivial clippy fix between passes), 11/11 anchors byte-identical, ready for sign-off.

## What changed

- **New `viewer` binary** at `crates/ui/src/bin/viewer.rs` — CLI-arg-driven (`viewer <report-path>`), zero-button surface (no kill, no order entry, no file-picker UI). Sibling of `cockpit` and `cockpit_live`; the workspace now ships **three bins**.
- **KPI strip** — six metric cards (Total return / CAGR / Sharpe / Max DD / Win rate / Trades) at the top of the viewer. Source: parsed from the report's existing markdown summary table — no new artefact format, no body-content rewrite. Missing fields render as `—` in `FG_3` (the 11 anchored sample reports omit CAGR + Win rate by design — those cards always render dashes until upstream report generators surface those metrics).
- **Equity curve + drawdown band** — line plot in `ACCENT` with filled area in `UP_500` at low alpha; drawdown band beneath, `DOWN_500` at low alpha. Five horizontal `BORDER_1` gridlines per the Lumen `Backtest.jsx` reference. Both share a refactored `widgets::canvas_chart` core that Phase 2's price chart also consumes — single source of truth for canvas drawing.
- **`core::EquitySeries`** — the cross-phase primitive (rich struct: `points: Vec<EquityPoint>` where each point carries `(ts, equity, drawdown_pct)`, plus `peak / trough / max_drawdown_pct / inception_ts / as_of_ts`). Same shape consumed by two surfaces from two different sources.
- **`audit::query::equity_curve_for_strategy(strategy_id, since, until)`** — additive sibling of `recent_fills_filtered` and `recent_journal_filtered`. Read-only over the same description-prefixed-rows pattern.
- **Phase 3 deferral closure** — the `STRATEGIES_SPARKLINE_DEFERRED` placeholder retires. The cockpit Strategies-detail screen now renders a real `widgets::sparkline` (~120-point cap with downsampling) fed by the new audit query.
- **Markdown body preservation** — the existing rendered markdown stays below the structured strip. No body content rewrite. **11/11 backtest anchors byte-identical.**

## Why

The `viewer` row was reserved in `architecture.md` since v0 but the file never landed — the cockpit-first ship priorities pushed it down. Phase 4 closes that gap, and bundles the Phase-3-deferred per-strategy sparkline because both surfaces need the same equity-history primitive. Designing the primitive once (now) prevents Phase 5/6 from re-litigating it.

## What the operator can do now

| Action | Command |
|---|---|
| Render any committed backtest report with the structured strip | `cargo run --release --bin viewer spec/reports/backtest-<scenario>.md` |
| See the per-strategy equity sparkline on the cockpit | `cargo run --release --bin cockpit --features fixtures` → click `Strategies` → click any strategy row |
| Inherit Phase 3's all-six-screen sidebar | (in the running cockpit) Home / Debug / Strategies / Risk / Audit / Charts |

The `viewer` window title reads `"Backtest report — {scenario}"`. CLI usage error (no path / non-existent path / non-markdown extension) returns a non-zero exit code with a one-line stderr message.

## Live demo

`cargo run --release --bin viewer spec/reports/backtest-20260420-151944-btc-2023-1m-sma-baseline-refresh.md` was launched and ran cleanly:

```
$ target/release/viewer spec/reports/backtest-20260420-151944-btc-2023-1m-sma-baseline-refresh.md
(window opened; iced wgpu surface initialised; KPI strip parsed
total_return / sharpe / max_dd / trades from the report's metadata
table; CAGR + win_rate cards rendered as `—` per Q3 graceful-fallback;
equity curve + drawdown band rendered from the report's equity-CSV
sidecar; markdown body rendered verbatim below; window killed cleanly
after 4 s; zero stdout / stderr emitted)
```

Stdout artifact (empty file confirms clean run): [`artifacts/lumen-phase-4-backtest-panel-2026-05-06/viewer-stdout.txt`](artifacts/lumen-phase-4-backtest-panel-2026-05-06/viewer-stdout.txt).

### Screenshots — operator capture pending

The Claude-Code sandbox does not have macOS screen-recording permission (same fallback as Phases 1–3). Copy these to produce the four screenshots:

```bash
# 1. Viewer · full backtest report (KPI strip + equity curve + drawdown band + body)
cargo run --release --bin viewer \
    spec/reports/backtest-20260420-151944-btc-2023-1m-sma-baseline-refresh.md &
sleep 4
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/viewer-full-report.png
pkill -f "target/release/viewer"

# 2. Viewer · drawdown-rich report (RSI reversion shows max-DD ~57%)
cargo run --release --bin viewer \
    spec/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md &
sleep 4
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/viewer-drawdown.png
pkill -f "target/release/viewer"

# 3. Cockpit · Strategies-detail screen with the new sparkline
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Strategies" in the sidebar, then click any strategy row …
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/cockpit-strategies-sparkline.png
pkill -f "target/release/cockpit"

# 4. Live cockpit · same sparkline against the live agent (real audit-ledger data)
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8
# … click "Strategies", then click a strategy row …
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/cockpit-live-strategies-sparkline.png
pkill -f "target/release/cockpit_live"
```

| Screenshot | Path | Status |
|---|---|---|
| Viewer · full backtest report | `spec/reports/screenshots/lumen-phase-4-backtest-panel/viewer-full-report.png` | pending operator capture |
| Viewer · drawdown-rich report | `spec/reports/screenshots/lumen-phase-4-backtest-panel/viewer-drawdown.png` | pending operator capture |
| Cockpit · Strategies-detail sparkline (fixtures) | `spec/reports/screenshots/lumen-phase-4-backtest-panel/cockpit-strategies-sparkline.png` | pending operator capture |
| Live · Strategies-detail sparkline | `spec/reports/screenshots/lumen-phase-4-backtest-panel/cockpit-live-strategies-sparkline.png` | pending operator capture |

## Verification matrix

The Phase 4 brief carries 17 R-items + 14 V-items + 12 Q-items (architect ratified all 12 with zero principled overrides; one shape refinement on Q1 — drawdown_pct nested inside `EquityPoint` rather than parallel `Vec<Decimal>` — eliminates length-coupling).

| V-item | Subject | Status | Evidence |
|---|---|---|---|
| V1 | Three bins build clean | VERIFIED | `cargo build --release -p ui --bin viewer` → `Finished release profile … in 51.08s`; `cockpit --features fixtures` → `… in 3.11s`; `cockpit_live --features live` → `… in 12.05s`. |
| V2 | All workspace tests pass | VERIFIED | `cargo test --workspace --all-targets` → **850 passed, 0 failed, 3 ignored** across **108 binaries** (Phase 3 ended at 810/104; Phase 4 net-new = 40 tests / 4 binaries). |
| V3 | `core::EquitySeries` primitive | VERIFIED | `core::equity_series::tests::*` 8/8 PASS. |
| V4 | `audit::query::equity_curve_for_strategy` | VERIFIED | `audit::query::tests::equity_curve_for_strategy_*` 4/4 unit + `audit::tests::equity_curve_for_strategy` 2/2 integration PASS. |
| V5 | Markdown summary parser | VERIFIED | `reports::parse::tests::*` 7/7 PASS — covers present-fields-only and graceful-fallback paths. |
| V6 | `viewer` bin read-only on `spec/` tree | VERIFIED | `ui::tests::viewer_read_only` 1/1 PASS — confirms no spec-tree writes. |
| V7 | KPI strip + equity curve + drawdown band widgets | VERIFIED | `kpi_strip` 2/2 + `equity_curve` 2/2 + `drawdown_band` 1/1 + `canvas_chart` 5/5 + `sparkline` 1/1 PASS. |
| V8 | Phase 3 sparkline deferral closure | VERIFIED | `ui::tests::strategies_screen_sparkline_replaces_placeholder` PASS; `STRATEGIES_SPARKLINE_DEFERRED` constant removed from strings.rs; `panel_snapshots__strategies_screen__sparkline_deferred.snap` deleted; new `__sparkline_present.snap` baseline lands. |
| V9 | rust-validate full skill PASS | VERIFIED | fmt clean / clippy `-D warnings` clean (`Finished … in 1.18s` second pass after orchestrator's `match_same_arms` fix at `screens/strategies.rs:150`) / cargo-deny `advisories ok, bans ok, licenses ok, sources ok` / cargo-audit N/A / rustdoc clean (`Finished … in 16.29s` after `rm -rf target/doc`; 4 intra-doc-link fixes applied during the dev pass). |
| V10 | Anchor regression byte-identical | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` — verified post the audit-query addition + the migration-free Phase 4 changes. |
| V11 | R16.3 brand-bleed grep returns zero | VERIFIED | `grep -rni …` against test-* + backtest-* exits 1; pre-existing matches in screenshots/README files unchanged from prior phases. |
| V12 | Cross-feature invariants 7/7 | VERIFIED | T1814 sub-block; tester re-ran each prior feature's named test green. |
| V13 | Snapshot baselines clean | VERIFIED | 72 baselines on disk (55 panel + 17 widget; +7 net delta vs Phase 3); zero `*.pending-snap` / `*.snap.new`. |
| V14 | Visual-diff attestation by ui-designer | VERIFIED | T1812 sub-block carries 8 sample-attested + Phase 3 deferral closure verification + full-inventory + `unknown`-color sweep + Q-evidence rollup (Q1 X-position alignment / Q2 shared canvas-chart core / Q3 KPI graceful fallback / Q6 / Q8 / Q9 / Q10 / Q11). |

Tester second-pass report at [`reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md`](../reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md). First-pass FAIL report preserved at [`reports/test-2026-05-06-lumen-phase-4-backtest-panel.md`](../reports/test-2026-05-06-lumen-phase-4-backtest-panel.md).

## Numbers that matter

| Metric | Value | Source |
|---|---|---|
| Workspace tests | 850 passed / 0 failed / 3 ignored | tester second-pass report § 3 (Phase 3: 810) |
| Test binaries | 108 (Phase 3: 104; net-new: 4 — viewer / strategies-sparkline-replaces-placeholder / equity_curve_for_strategy / equity-csv tests) | tester report § 3 |
| Snapshot baselines | 72 (55 panel + 17 widget; +7 net + 1 deletion vs Phase 3) | T1812 sub-block + ui-designer attestation |
| Phase 4 R-items | 17 | feature brief |
| Phase 4 Q-items | 12 / 12 ratified, zero principled overrides | architect Design § Q-resolutions |
| Phase 4 net-new strings | ~12 (`VIEWER_*`, `KPI_*`, `STRATEGIES_SPARKLINE_LOADING`, `STRATEGIES_EQUITY_HISTORY_UNAVAILABLE_PREFIX`) | T1801–T1809 commits |
| Phase 4 net-new audit query | `equity_curve_for_strategy` (sibling to Phase 2's `recent_fills_filtered` and Phase 3's `recent_journal_filtered`) | T1802 |
| Bins in workspace | 3 (`viewer` is greenfield; `cockpit`, `cockpit_live` carry-forward) | T1803 |
| Backtest anchors clean | 11 / 11 byte-identical | `verify_anchors.sh` |
| Net code change | ~+1,900 lines (5 new widgets + viewer.rs + EquitySeries + audit query + parser + sparkline replacement) | T1801–T1815 |
| `rust-validate` steps PASS | 5 / 5 (after orchestrator's clippy fixup) | tester second-pass report § 2 |
| Tester passes to ratification | 2 (first-pass FAIL on clippy `match_same_arms`; second-pass PASS) | tester report set |

## Open decisions

One — the **Phase 5 promotion gate**.

Phase 5 (HumanControl + AgentFeed rename) is queued. Its stub at [`features/lumen-phase-5-humancontrol-agentfeed.md`](../features/lumen-phase-5-humancontrol-agentfeed.md) inherits Phase 1–4 design contracts. Phase 5 is **the first phase to introduce net-new operator-write paths** — pause-strategy, override-risk-veto, execution-mode toggle (Observe / Supervised / Auto). The `tape` widget renames to `AgentFeed` (module-level only; no visual change). Anchor risk: zero by default; if pause/override audit writers land they are additive. Approve = Phase 5 analyst spawns next.

**TD-1 tightening point.** Phase 4's design-pass restated the focus-ring deferral (iced still pins `=0.14.0`). The viewer is a zero-button surface so the deferral was operationally invisible. **Phase 5 is the load-bearing phase** for the focus-ring decision because it ships net-new operator-write controls (typed-confirm flows for pause/override). The architect should expect to either fold the iced 0.15+ upgrade into Phase 5 OR commit to the custom-widget escape hatch. The master roadmap's TD-1 row is updated through 2026-05-06.

## Approval

Pick exactly one. Add notes below the chosen line if useful.

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes

_(operator fills in)_

## Feedback log

_(presenter appends rejection notes here on re-spawn; empty on first ship)_

## Closing

Mechanical pre-tick gate is run after this file is written; the result is quoted in the orchestrator's reply. No section above is pre-ticked; the operator is the only one who ticks.

The presenter's verdict line is emitted as the last line of the orchestrator's reply, not embedded in this file.
