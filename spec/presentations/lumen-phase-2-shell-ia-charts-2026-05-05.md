---
slug: lumen-phase-2-shell-ia-charts
mode: release
date: 2026-05-05
agent: presenter
verdict: APPROVED
approved_by: operator
approved_at: 2026-05-05
---

# Phase 2 — Shell IA + Charts · Sprint review

## TL;DR

The cockpit is now a **left-sidebar shell** with three screens (Home / Debug / Charts). The Charts screen plots one symbol's price with buy/sell triangles overlaid from the audit ledger — the cross-check the operator asked for. Tester first-pass PASS, 11/11 anchors clean, ready for sign-off.

## What changed

- **Sidebar nav** at the left of the cockpit (180 px, fixed, text-only, no icons). Selected entry uses the Phase 1 active-row pattern (2 px ACCENT left rule).
- **Three screens** routed via `Cockpit::current_screen`. **Home** carries the trading view (PnL · Positions · Strategies · Tape). **Debug** carries the operations chrome (kill switch · latency · per-venue market health · server time · version · placeholder logs). **Charts** carries the new symbol-plot.
- **Charts screen**: chip-row symbol selector at the top (active chip rule on the bottom edge — Q5 ratified), line-series price plot rendered on iced canvas (gridlines `BORDER_1`, line `ACCENT`), buy/sell triangles overlaid in `UP_500` / `DOWN_500` from the audit ledger. Empty state reads "No data" in `FG_3` — never blank.
- **`audit::query::recent_fills_filtered(venue, symbol, since, until)`** — additive, read-only over the existing journal-transactions table. Phase 2 venue-handling caveat documented inline (`PHASE 3 NOTE`): only Binance fills exist on disk today; non-Binance returns `Ok(vec![])`. The argument is forward-compat scaffolding for Phase 3's audit-screen migration.
- **Right-rail track reservation** for Phase 6's Assistant slot — column exists in the shell grid at `Length::Fixed(0.0)` width (zero-cost when v2 LLM isn't shipped, no `cfg!` gate).

## Why

The Phase 1 cockpit was operator-correct but conflated three operator modes — "is the system trading sensibly" / "is the system healthy" / "what just happened" — onto one scan. Splitting into Home + Debug puts trading data and operations chrome on separate scans. The Charts screen closes the cross-check gap the operator surfaced at the 2026-05-04 session — "did my strategy buy at the low or the high of this candle?" The audit ledger has had every fill all along; Phase 2 lays a visual path to it.

## What the operator can do now

| Action | Command |
|---|---|
| Look at the new shell with deterministic demo data | `cargo run --release --bin cockpit --features fixtures` |
| Look at the new shell against the live agent | `cargo run --release --bin cockpit_live --features live -- --config config/agent.toml` |
| Switch screens | (in the running cockpit) click `Home` / `Debug` / `Charts` in the left sidebar |
| Plot a symbol | (on Charts screen) click a symbol chip at the top of the screen |
| See your fills as triangles on the chart | nothing — markers populate from the audit ledger automatically when a symbol is selected |

No new operator workflow beyond the navigation. Same actions, new IA, new Charts surface.

## Live demo

`cargo run --release --bin cockpit --features fixtures` was launched and ran cleanly:

```
$ target/release/cockpit
(window opened; iced wgpu surface initialised; deterministic fixture
data populated PnL / Positions / Strategies / Tape on the Home screen
by default; sidebar visible at the left with Home / Debug / Charts;
status bar visible at the bottom; window killed cleanly after 4 s for
the screenshot-capture attempt; zero stdout / stderr emitted)
```

Stdout artifact (empty file confirms clean run): [`artifacts/lumen-phase-2-shell-ia-charts-2026-05-05/cockpit-fixtures-stdout.txt`](artifacts/lumen-phase-2-shell-ia-charts-2026-05-05/cockpit-fixtures-stdout.txt).

### Screenshots — operator capture pending

The Claude-Code sandbox does not have macOS screen-recording permission, so the `screencapture -x` call returned `could not create image from display` (same fallback as Phase 1). The capture-screenshot skill's documented fallback is **operator-instruction blocks** — copy these to produce the four screenshots referenced below:

```bash
# 1. Fixtures bin · Home screen (default landing — PnL / Positions / Strategies / Tape under the new shell)
cargo run --release --bin cockpit --features fixtures &
sleep 4
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-home.png
pkill -f "target/release/cockpit"

# 2. Fixtures bin · Debug screen (click "Debug" in the sidebar before capture)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Debug" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-debug.png
pkill -f "target/release/cockpit"

# 3. Fixtures bin · Charts screen with a symbol selected (click "Charts" → click any symbol chip)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Charts", then click a symbol chip …
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-charts-with-markers.png
pkill -f "target/release/cockpit"

# 4. Live bin · Home screen (real venues)
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-live-home.png
pkill -f "target/release/cockpit_live"
```

| Screenshot | Path | Status |
|---|---|---|
| Fixtures · Home (default landing) | `spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-home.png` | pending operator capture |
| Fixtures · Debug screen | `spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-debug.png` | pending operator capture |
| Fixtures · Charts with markers | `spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-charts-with-markers.png` | pending operator capture |
| Live · Home | `spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-live-home.png` | pending operator capture |

## Verification matrix

The Phase 2 brief carries 15 R-items + 12 V-items + 11 Q-items (architect ratified all 11 with zero deviations from analyst recommendation). Each row below is one V-item from the tester's eight-gate audit.

| V-item | Subject | Status | Evidence |
|---|---|---|---|
| V1 | Both bins build clean | VERIFIED | `cargo build --release -p ui --bin cockpit --features fixtures` → `Finished release profile … in 5.94s`; `cargo build --release -p ui --bin cockpit_live --features live` → `Finished release profile … in 11.76s`. |
| V2 | All workspace tests pass | VERIFIED | `cargo test --workspace --all-targets` → **781 passed, 0 failed, 3 ignored** across **98 binaries**. |
| V3 | Phase 2 net-new tests pass | VERIFIED | `audit::query::recent_fills_filtered_*` 3/3 PASS; `state::tests` (`switch_screen_is_pure`, `chart_buffer_evicts_at_capacity`, `chart_buffer_keys_distinct_per_pair`, `select_symbol_persists_across_screen_switch`) PASS; `chart_markers_from_audit_query` 1/1 PASS; `shell_grid` 3/3 PASS; `synthetic_candles` deterministic 3/3 PASS. |
| V4 | rust-validate full skill PASS | VERIFIED | fmt clean / clippy `-D warnings` clean (`Finished … in 1.28s`) / cargo-deny `advisories ok, bans ok, licenses ok, sources ok` / cargo-audit N/A (deny advisories cover) / rustdoc clean (`Finished … in 9.03s` after `rm -rf target/doc`). |
| V5 | Anchor regression byte-identical | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`. |
| V6 | R16.3 brand-bleed grep returns zero in `spec/reports/` body content | VERIFIED | `grep -rni …` against test-* + backtest-* exits 1; pre-existing screenshots-README references are unchanged from Phase 1 third-pass and are not body content. |
| V7 | Cross-feature invariants 7/7 | VERIFIED | T1614 sub-block; tester re-ran each prior feature's named test green. |
| V8 | Snapshot baselines clean | VERIFIED | 53 baselines on disk (45 panel + 8 widget); zero `*.pending-snap` / `*.snap.new`; `unknown`-color sweep returns zero unmapped escapes (only the legitimate `Latency::Unknown` badge state). |
| V9 | Visual-diff attestation by ui-designer | VERIFIED | T1613 sub-block carries 6 sample-attested + 1 bonus + full-inventory verification + Q1/Q5/Q6/Q7 evidence (line-series default; chip-row bottom rule; per-symbol seed determinism; right-rail zero-width). |
| V10 | Honest-tick audit on T1601–T1616 | VERIFIED | All 16 task rows + the orchestrator's rustdoc fill-in + the ui-designer's attestation sub-block carry file:line + test command + test output. |
| V11 | Phase 2 audit-query is additive (zero anchor risk) | VERIFIED | `recent_fills_filtered` is a generalisation of `recent_fills`; same description-prefixed-rows scan; does not write the ledger; 11/11 anchors stay byte-identical. |
| V12 | Phase 6 right-rail reservation in place (zero shipped UI) | VERIFIED | `shell_grid` integration test asserts the column-track exists at `Length::Fixed(0.0)`; no widget renders in it; no token references it. |

Tester report at [`reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](../reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md).

## Numbers that matter

| Metric | Value | Source |
|---|---|---|
| Workspace tests | 781 passed / 0 failed / 3 ignored | tester report § 3 |
| Test binaries | 98 (Phase 1: 96; net-new: 2 — `chart_markers_from_audit_query` + `shell_grid`) | tester report § 3 |
| Snapshot baselines | 53 (45 panel + 8 widget; ~12 net-new on top of Phase 1's 41) | T1613 sub-block + ui-designer attestation |
| Phase 2 R-items | 15 | feature brief |
| Phase 2 Q-items | 11 / 11 ratified, zero deviations | architect Design § Q-resolutions |
| Phase 2 net-new strings | 12 (`SIDEBAR_NAV_*`, `CHART_*`, `DEBUG_*`, `SCREEN_NOT_YET`) | T1604/T1605/T1607 commits |
| Backtest anchors clean | 11 / 11 byte-identical | `verify_anchors.sh` |
| Net code change | ~+1,650 lines (3 net-new widgets + 4 net-new screens + audit query method + state-shape additions); ~−40 (consolidations) | T1601–T1616 task ticks |
| `rust-validate` steps PASS | 5 / 5 | tester report § 2 |
| `cargo doc -D warnings` time | 9.03 s (after `rm -rf target/doc` for clean baseline) | tester report § 2 |
| Phase 2 carry-forward TD rows | 1 (TD-1 — iced 0.14 focus-ring API gap; deferral restated, next re-eval Phase 3) | master roadmap |

## Open decisions

One — the **Phase 3 promotion gate**.

Phase 3 (Detail screens — Strategies / Risk / Audit) is queued. Its stub at [`features/lumen-phase-3-detail-screens.md`](../features/lumen-phase-3-detail-screens.md) inherits Phase 2's sidebar contract and the operator-locked Q11–Q14 decisions; its analyst kickoff happens after operator approval of this Phase 2 deck per master-roadmap Constraint 3 (sequential phasing). Phase 3 is read-only over existing backend data — no new audit writers; the audit-query method extensions are read-only and additive.

**One Phase 3 prerequisite worth surfacing now**: the `recent_fills_filtered` venue argument returns `Ok(vec![])` for non-Binance venues today because `journal_transactions` doesn't carry a `venue` column. Phase 3's Audit screen will need a `journal_transactions.venue` migration to surface multi-venue fills. The migration is **additive** (new column, default-null backfill), no anchor risk, but it's a prerequisite analyst-and-architect decision Phase 3 inherits as a known item rather than discovers cold.

There is also one carried-forward technical-debt row: **TD-1 — true keyboard-focus ring** (iced 0.14 lacks the focus-ring API; verified again at Phase 2 design-pass — `crates/ui/Cargo.toml:50` still pins `iced = "=0.14.0"`). Hover-state ring + ACCENT input border-shift continue as the bounded approximation. Next re-evaluation: Phase 3 analyst kickoff. Documented in master roadmap → Cross-phase technical-debt items.

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
