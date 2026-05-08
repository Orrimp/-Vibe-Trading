---
slug: lumen-phase-3-detail-screens
mode: release
date: 2026-05-05
agent: presenter
verdict: APPROVED
approved_by: operator
approved_at: 2026-05-06
---

# Phase 3 — Detail screens · Sprint review

## TL;DR

Three new sidebar entries — **Strategies**, **Risk**, **Audit** — surface backend data the cockpit didn't have a UI for. Per-strategy params + signal events; per-venue exposure + kill-threshold proximity gauge with tri-band colour ramp; full ledger browser with venue / symbol / kind / time-range filters and 250-row pagination. Tester first-pass PASS, 11/11 anchors byte-identical post-migration, ready for sign-off.

## What changed

- **Sidebar nav** extended from 3 entries to **6**: Home → Debug → **Strategies → Risk → Audit** → Charts. Phase 2's widget API parameterisation absorbed the extension without touching the widget body.
- **Strategies-detail screen**: per-strategy view with read-only `[[strategy]]` params from `config/agent.toml` + filtered signal-event history from the existing `strategies_recent_events` buffer. Click a row in Home → Strategies summary to jump straight to its detail; Cockpit persists the selection across screen switches.
- **Risk / Limits screen**: per-venue exposure cells + daily loss limit consumed + kill-threshold proximity gauge as a horizontal bar (`UP_500` ≤ 70% → `WARN_500` > 70% → `DOWN_500` > 90%). Reads via a new `RiskTelemetry` channel on `agent::EventBus` (mirror of the existing `MarketHealth` publisher pattern).
- **Audit / Journal screen**: full ledger browser with filter chip-row (venue · symbol · kind · time-range) + fixed 250-row pagination + per-row click reuses the existing T1208 `journal_transaction_modal`. Powered by a new sibling audit query method `recent_journal_filtered` (additive to `recent_fills_filtered`).
- **`008_journal_transactions_venue.sql` migration**: additive column with `DEFAULT NULL` + backfill `'binance'` for existing rows. New rows post-migration carry the actual venue from the `post_fill` writer's new `venue: Venue` parameter (~25 call-sites updated). **11/11 anchors byte-identical post-migration.**

## Why

Phase 2 closed the IA gap (sidebar + Home/Debug split) and the chart cross-check gap. Phase 3 closes the **last operator-visible data gap at v1.5b**: three pieces of backend data — per-strategy detail, risk exposure-vs-caps, and the full ledger browser — that already existed in `crates/strategy`, `crates/agent`, and `crates/audit` but had no cockpit UI surface. Phase 3 is read-only over existing data; **no new audit writers** (Phase 5 HumanControl introduces the first new operator-write paths). The migration is the only schema change, and it's strictly additive.

## What the operator can do now

| Action | Command |
|---|---|
| Look at the new shell with deterministic demo data | `cargo run --release --bin cockpit --features fixtures` |
| Look at the new shell against the live agent | `cargo run --release --bin cockpit_live --features live -- --config config/agent.toml` |
| Inspect a strategy's params + recent signals | (in the running cockpit) click `Strategies` in the sidebar, OR click a strategy row on Home → Strategies summary |
| See current risk exposure + kill-threshold proximity | (in the running cockpit) click `Risk` in the sidebar |
| Browse the full ledger with filters | (in the running cockpit) click `Audit` in the sidebar; pick venue / symbol / kind / time-range chips; page through |

No new operator workflow surface beyond the navigation. Same agent, three new read surfaces.

## Live demo

`cargo run --release --bin cockpit --features fixtures` was launched and ran cleanly:

```
$ target/release/cockpit
(window opened; iced wgpu surface initialised; deterministic fixture
data populated PnL / Positions / Strategies / Tape on the Home screen
by default; sidebar shows 6 entries; status bar visible at the bottom;
window killed cleanly after 4 s for the screenshot-capture attempt;
zero stdout / stderr emitted)
```

Stdout artifact (empty file confirms clean run): [`artifacts/lumen-phase-3-detail-screens-2026-05-05/cockpit-fixtures-stdout.txt`](artifacts/lumen-phase-3-detail-screens-2026-05-05/cockpit-fixtures-stdout.txt).

### Screenshots — operator capture pending

The Claude-Code sandbox does not have macOS screen-recording permission, so the `screencapture -x` call returned `could not create image from display` (same fallback as Phase 1 and Phase 2). Copy these to produce the four screenshots:

```bash
# 1. Strategies-detail screen (click "Strategies" in the sidebar before capture)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Strategies" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-strategies.png
pkill -f "target/release/cockpit"

# 2. Risk / Limits screen with kill-threshold proximity gauge
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Risk" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-risk.png
pkill -f "target/release/cockpit"

# 3. Audit screen with filter chips + pagination
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Audit" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-audit.png
pkill -f "target/release/cockpit"

# 4. Live bin · sidebar with all 6 entries (start on any screen)
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-live-six-entries.png
pkill -f "target/release/cockpit_live"
```

| Screenshot | Path | Status |
|---|---|---|
| Fixtures · Strategies-detail | `spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-strategies.png` | pending operator capture |
| Fixtures · Risk / Limits | `spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-risk.png` | pending operator capture |
| Fixtures · Audit / Journal | `spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-audit.png` | pending operator capture |
| Live · 6-entry sidebar | `spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-live-six-entries.png` | pending operator capture |

## Verification matrix

The Phase 3 brief carries 15 R-items + 13 V-items + 11 Q-items (architect ratified all 11 with one deferral — Q6 sparkline pushed to Phase 4 since the cheap path doesn't exist on the current state shape).

| V-item | Subject | Status | Evidence |
|---|---|---|---|
| V1 | Both bins build clean | VERIFIED | `cargo build --release -p ui --bin cockpit --features fixtures` → `Finished release profile … in 4.27s`; `cargo build --release -p ui --bin cockpit_live --features live` → `Finished release profile … in 12.55s`. |
| V2 | All workspace tests pass | VERIFIED | `cargo test --workspace --all-targets` → **810 passed, 0 failed, 3 ignored** across **104 binaries**. |
| V3 | Migration applies cleanly | VERIFIED | `audit::tests::migration_008_*` 3/3 PASS — column added with `'binance'` backfill, post-migration writes persist actual venue (Coinbase/Kraken roundtrip). |
| V4 | `recent_journal_filtered` query | VERIFIED | `audit::query::tests::recent_journal_filtered_*` 5/5 unit + `audit::tests::recent_journal_filtered` 2/2 integration PASS. |
| V5 | Risk Telemetry channel | VERIFIED | `ui::tests::risk_telemetry_subscription` 1/1 PASS; mirror of `MarketHealth` publisher shape. |
| V6 | Cross-link Home → Strategies-detail | VERIFIED | `ui::tests::home_strategies_row_cross_link` 3/3 PASS. |
| V7 | Audit filter UX + modal trigger | VERIFIED | `ui::tests::audit_filter_chip_emits_filter_changed` 3/3 + `ui::tests::audit_row_opens_modal` 2/2 PASS. |
| V8 | rust-validate full skill PASS | VERIFIED | fmt clean / clippy `-D warnings` clean (`Finished … in 1.25s`) / cargo-deny `advisories ok, bans ok, licenses ok, sources ok` / cargo-audit N/A / rustdoc clean (`Finished … in 10.70s` after `rm -rf target/doc`). |
| V9 | Anchor regression byte-identical post-migration | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` — verified twice during the developer pass (post-migration + post-query). The migration is strictly additive; no row body content changed. |
| V10 | R16.3 brand-bleed grep returns zero | VERIFIED | `grep -rni …` against test-* + backtest-* exits 1; pre-existing screenshots-README references unchanged from prior phases. |
| V11 | Cross-feature invariants 7/7 | VERIFIED | T1714 sub-block; tester re-ran each prior feature's named test green. |
| V12 | Snapshot baselines clean | VERIFIED | 65 baselines on disk (54 panel + 11 widget); zero `*.pending-snap` / `*.snap.new`. |
| V13 | Visual-diff attestation by ui-designer | VERIFIED | T1713 sub-block carries 7 sample-attested + full-inventory verification + `unknown`-color sweep + Q-evidence rollup (Q1 migration / Q2 signal-history / Q3 RiskTelemetry / Q4 fixed-250 pagination / Q5 in-session filter / Q9 tri-band threshold-bar / Q10 read-only params / Q11 budget). |

Tester report at [`reports/test-2026-05-05-lumen-phase-3-detail-screens.md`](../reports/test-2026-05-05-lumen-phase-3-detail-screens.md).

## Numbers that matter

| Metric | Value | Source |
|---|---|---|
| Workspace tests | 810 passed / 0 failed / 3 ignored | tester report § 3 (Phase 2: 781) |
| Test binaries | 104 (Phase 2: 98; net-new: 6) | tester report § 3 |
| Snapshot baselines | 65 (54 panel + 11 widget; ~12 net-new on top of Phase 2's 53) | T1713 sub-block + ui-designer attestation |
| Phase 3 R-items | 15 | feature brief |
| Phase 3 Q-items | 11 / 11 ratified, 1 deferral (Q6 sparkline → Phase 4) | architect Design § Q-resolutions |
| Phase 3 net-new strings | 30 (Strategies/Risk/Audit screen labels + Phase 3 sidebar entries) | T1701 commit |
| Phase 3 net-new audit query | `recent_journal_filtered` (sibling to Phase 2's `recent_fills_filtered`) | T1712 |
| SQL migration | `008_journal_transactions_venue.sql` (additive ALTER + backfill) | T1702 |
| `post_fill` writer call-sites updated | ~25 (audit/reports/ui test fixtures) | T1702 |
| Backtest anchors clean | 11 / 11 byte-identical post-migration | `verify_anchors.sh` (verified twice) |
| Net code change | ~+1,400 lines (3 screen modules, RiskTelemetry channel, recent_journal_filtered, migration, fixtures, integration tests) | T1701–T1716 task ticks |
| `rust-validate` steps PASS | 5 / 5 | tester report § 2 |
| Phase 3 carry-forward TD rows | 1 (TD-1 — iced 0.14 focus-ring API gap, deferral re-stated, next re-eval Phase 4) | master roadmap |

## Open decisions

One — the **Phase 4 promotion gate**.

Phase 4 (Backtest panel — `viewer` bin: KPI strip + equity curve + drawdown band) is queued. Its stub at [`features/lumen-phase-4-backtest-panel.md`](../features/lumen-phase-4-backtest-panel.md) inherits Phase 1/2/3 design contracts. Phase 4 is a single-binary scope (`viewer` only); no `cockpit` / `cockpit_live` changes. Anchor risk: zero (viewer reads existing committed reports).

**Phase 4 will absorb the deferred Q6 sparkline** from the Phase 3 architect ratification. The cheap path didn't exist on Phase 3's state shape (`Cockpit::pnl: PanelState<PnlSnapshot>` is a single snapshot, not a historical buffer). Phase 4 needs the same equity-history primitive for the viewer's equity curve, so building it in Phase 4 hits two birds. Phase 3 ships placeholder copy on the Strategies-detail screen + a snapshot baseline `strategies_screen__sparkline_deferred.snap` that locks the deferral seam.

There is also one carried-forward technical-debt row: **TD-1 — true keyboard-focus ring** (iced 0.14 lacks the focus-ring API; verified again at Phase 3 design-pass — `crates/ui/Cargo.toml:52` still pins `iced = "=0.14.0"`). Hover-state ring + ACCENT input border-shift continue. Next re-evaluation: Phase 4 analyst kickoff. Documented in master roadmap → Cross-phase technical-debt items.

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
