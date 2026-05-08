---
slug: lumen-phase-1-foundation
mode: release
date: 2026-05-04
agent: presenter
verdict: APPROVED
approved_by: operator
approved_at: 2026-05-04
---

# Phase 1 — Foundation · Sprint review

## TL;DR

The cockpit's design system is rebuilt. New token palette, three-tier panel chrome, whisper shadows, focus ring, status bar at the bottom — same widgets, materially calmer surface. Tester PASS, 11/11 anchors clean, ready for your sign-off.

## What changed

- **Design tokens**: 12-token palette → full system with warm-paper light + cool-deep dark + muted teal accent. Both modes wired in code; dark is the cold-start default.
- **Panel chrome**: every panel now uses Tier 1 styling (hairline border + whisper shadow + tinted background). Inputs use sunken styling. Modals use Tier 3 (overlay + shadow_3).
- **Status bar**: new always-visible row at the bottom of the cockpit shell — connection state with coloured dot, latency, account, server time, CPU placeholder, version. No more chrome scattered across panels.

## Why

The shipped cockpit was operator-correct but design-system-thin: one flat panel surface, no light-mode hexes, no elevation language, no anchor for connection / latency / server-time. The Lumen design conversation produced a calm-fintech token system purpose-built for this project (not a third-party kit). Phase 1 closes that loop with a single one-merge swap so the visual diff is reviewable in one pass. Per master-roadmap rationale at [`features/lumen-design-adoption.md`](../features/lumen-design-adoption.md).

## What the operator can do now

| Action | Command |
|---|---|
| Look at the new cockpit (deterministic demo data) | `cargo run --release --bin cockpit --features fixtures` |
| Look at the new cockpit attached to the live agent | `cargo run --release --bin cockpit_live --features live -- --config config/agent.toml` |
| Click any tape row to open the audit modal (Tier 3 chrome) | (in the running cockpit, single-click a fill row) |
| Trip kill switch + see typed-confirm flow with focused-input ACCENT border | (in the running cockpit, click "Stop trading", type the safety phrase) |

No new operator workflow; same cockpit, same actions, refreshed surface.

## Live demo

`cargo run --release --bin cockpit --features fixtures` was launched and ran cleanly:

```
$ target/release/cockpit
(window opened; iced wgpu surface initialised; deterministic fixture
data populated PnL / Positions / Strategies / Tape; status bar
visible at bottom showing "Disconnected · ―" plus account /
server-time fields; cockpit was killed cleanly after 4s for the
screenshot-capture attempt; zero stdout / stderr emitted)
```

Stdout artifact (empty file confirms clean run): [`artifacts/lumen-phase-1-foundation-2026-05-04/cockpit-fixtures-stdout.txt`](artifacts/lumen-phase-1-foundation-2026-05-04/cockpit-fixtures-stdout.txt).

### Screenshots — operator capture pending

The Claude-Code sandbox does not have macOS screen-recording permission, so the `screencapture -x` call returned `could not create image from display`. The capture-screenshot skill's documented fallback is **operator-instruction blocks** — copy these into your terminal to produce the two screenshots referenced in this deck:

```bash
# 1. Fixtures bin (deterministic demo data — every panel + status bar)
cargo run --release --bin cockpit --features fixtures &
sleep 4
screencapture -W spec/reports/screenshots/lumen-phase-1-foundation/cockpit-fixtures.png
pkill -f "target/release/cockpit"

# 2. Live bin (real venues; needs config/agent.toml + network)
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8                      # iced + agent + venue handshake
screencapture -W spec/reports/screenshots/lumen-phase-1-foundation/cockpit-live.png
pkill -f "target/release/cockpit_live"
```

`screencapture -W` will prompt you to click the cockpit window. Save the resulting PNGs as named; the deck's reference paths line up. (If you want full-screen instead, swap `-W` for `-x`.)

| Screenshot | Path | Status |
|---|---|---|
| Fixtures bin · all panels + status bar | `spec/reports/screenshots/lumen-phase-1-foundation/cockpit-fixtures.png` | pending operator capture |
| Live bin · live agent attached | `spec/reports/screenshots/lumen-phase-1-foundation/cockpit-live.png` | pending operator capture |

(The fixtures bin is deterministic — re-running the capture command produces the same window content. The live bin is non-deterministic — content depends on what the venues are doing at capture time.)

## Verification matrix

The Phase 1 brief carries 17 R-items + 9 V-items. Each row below cross-references the brief's V-item and the tester's report verdict.

| V-item | Subject | Status | Evidence |
|---|---|---|---|
| V1 | Both bins build clean | VERIFIED | `cargo build -p ui --bin cockpit --features fixtures` → `Finished release profile … in <build-time>`; same for `cockpit_live --features live`. T1514 sub-block. |
| V2 | All workspace tests pass | VERIFIED | `cargo test --workspace --all-targets` → 757 passed, 0 failed, 3 ignored across 96 binaries. T_FINAL gate 2. |
| V3 | rust-validate full skill PASS | VERIFIED | fmt / clippy `-D warnings` / cargo-deny / docs all PASS. T_FINAL gate 3, third-pass tester. |
| V4 | Anchor regression byte-identical | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` (re-run today: `2ef403f1…` last-line sample matches). |
| V5 | R16.3 brand-bleed grep returns zero in `spec/reports/` | VERIFIED | `grep -rni …` exit 1 (zero matches). T_FINAL gate 5. |
| V6 | Cross-feature invariants 7/7 | VERIFIED | T1512 sub-block in task list; tester third-pass confirmed. |
| V7 | Snapshot baselines clean | VERIFIED | 41 baselines on disk (36 refreshed + 5 net-new for T1506/T1508); zero `*.pending-snap`. |
| V8 | Visual-diff attestation by ui-designer | VERIFIED | T1511 sub-block records ui-designer attestation for all refreshed baselines. |
| V9 | Honest-tick audit on T1501–T1514 | VERIFIED | All 14 task rows + the T1514 fixup sub-block + the T1514 rustdoc gate addendum carry file:line + test command + test output. |

Tester report at [`reports/test-2026-05-04c-lumen-phase-1-foundation.md`](../reports/test-2026-05-04c-lumen-phase-1-foundation.md) (third pass; first two FAIL reports preserved at `…-04…md` and `…-04b…md` for audit).

## Numbers that matter

| Metric | Value | Source |
|---|---|---|
| Workspace tests | 757 passed / 0 failed / 3 ignored | tester report § 3 |
| Test binaries | 96 | tester report § 3 |
| Snapshot baselines | 41 (36 refreshed + 5 new) | T1511 sub-block |
| Tokens replaced | 12 (legacy semantic) → ~50 (Lumen palette + tiers + shadows + spacing + radii + typography + motion) | T1501 commit |
| Backtest anchors clean | 11 / 11 byte-identical | `verify_anchors.sh` |
| Net widget code change | ~+450 lines (status_bar.rs new) / ~−80 lines (consolidated token references) | T1502/T1508 commits |
| `rust-validate` steps PASS | 5 / 5 (fmt, clippy, deny, audit-N/A, docs) | tester report § 2 |
| R16.3 grep matches in `spec/reports/` | 0 | tester report § 8 gate 5 |
| Visual-diff baselines attested | 36 refreshed (sample-attested + full-inventory) | T1511 sub-block |

## Open decisions

One — the **Phase 2 promotion gate**.

Phase 2 (Shell IA + Charts) is queued and ready for analyst kickoff. Promotion is gated on operator approval of this Phase 1 deck per master-roadmap Constraint 3 (sequential phasing). The 2026-05-04 roadmap revision absorbed the operator's session feedback (sidebar IA + Home/Debug split + per-symbol price chart + buy/sell markers from a new filtered audit query) into the Phase 2 brief stub at [`features/lumen-phase-2-shell-ia-charts.md`](../features/lumen-phase-2-shell-ia-charts.md). Approve here = Phase 2 analyst spawns next.

There is one carried-forward technical-debt row from Phase 1: **TD-1 — true keyboard-focus ring** (iced 0.14 lacks `button::Status::Focused` and `text_input::Style.shadow`; Phase 1 ships hover-state ring + ACCENT input border-shift as a bounded approximation). Two named upgrade triggers (iced version bump, custom-widget escape hatch). Earliest re-evaluation at Phase 2 analyst kickoff. Documented in [`features/lumen-design-adoption.md` § Cross-phase technical-debt items](../features/lumen-design-adoption.md). No operator action needed today; surfacing for visibility.

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
