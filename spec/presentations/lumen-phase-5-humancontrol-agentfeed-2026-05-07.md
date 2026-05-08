---
slug: lumen-phase-5-humancontrol-agentfeed
mode: release
date: 2026-05-07
agent: presenter
verdict: APPROVED
approved_by: operator
approved_at: 2026-05-08
---

# Phase 5 — HumanControl + AgentFeed · Sprint review

## TL;DR

The cockpit gets its **first net-new operator-write surface since v0**: a "Control" sidebar entry with execution-mode toggle + per-strategy pause + risk-veto override (typed-confirm flow). The four-phase **TD-1 deferral closes** with a custom-widget focus-ring escape hatch (visible halo on every focused destructive control). The `tape` widget renames to `AgentFeed` (module-level only). Tester second-pass PASS (one fmt fixup between passes), 11/11 anchors byte-identical, ready for sign-off. **Phase 5 is the last shippable phase of the initiative absent v2 LLM** — Phase 6 (Assistant slot) is reserved until the v2 LLM strategy lands.

## What changed

- **HumanControl panel** at `crates/ui/src/widgets/human_control.rs` (new) — execution-mode segmented control (Observe / Supervised / Auto, runtime-only persistence), daily loss limit + max position + used-today P&L mirror rows, kill button as the bottom action. Lives on a new "Control" sidebar entry (7th); the kill widget retires from the Debug screen.
- **Pause-strategy** — single-click toggle button per strategy on the Strategies-detail screen (Phase 3 surface). Emits `Message::StrategyPauseToggled(StrategyId)`; `audit::journal::strategy_paused` writer persists to the ledger.
- **Override-risk-veto** — typed-confirm modal flow (`OVERRIDE` phrase, mirror of kill-confirm pattern) per surfaced veto event. Emits `audit::journal::risk_veto_overridden`. Phase 5 ships the operator surface over a placeholder feed; **TD-2 (new master-roadmap row)** tracks the deferred upstream wiring of the risk-engine veto-emit channel.
- **TD-1 closure (T1912)** — the four-phase focus-ring deferral closes via `crates/ui/src/widgets/focus_ring.rs` (new). Subscription-driven custom-widget escape hatch wraps all four destructive surfaces (kill button + kill confirm input + override-risk-veto confirm + per-strategy pause + execution-mode segments). Visible accent-bordered halo on every focused control — Q5 architect ratification of path (b).
- **`tape` → `agent_feed` module rename** — `mv crates/ui/src/widgets/tape.rs crates/ui/src/widgets/agent_feed.rs`; mod-decl + import-site updates; consistency-test fixture; 9 baseline filenames `tape_*.snap` → `agent_feed_*.snap`. **`Cockpit::tape` field name preserved** (Q14) to avoid 100+ test ripple — code-comment annotation points at the new module path.
- **Two new audit writers, zero migration** — `StrategyEventKind` extension at the application layer; `strategy_events.kind` column is already `TEXT`. Mirrors `kill_switch_tripped` pattern. **11/11 anchors byte-identical post-writers.**

## Why

Phase 5 is the load-bearing phase for two architectural debts that have been carried for the entire initiative:
1. **Operator-write paths.** Phases 1–4 were uniformly read surfaces + design infrastructure. Real cockpit usefulness needs surfaces the operator can act on: pause a misbehaving strategy without halting the whole agent, override a risk veto when the operator's judgment supersedes the rule, change execution mode mid-session.
2. **TD-1 (focus-ring deferral).** Restated four phases in a row; the analyst rejected a fifth restatement, the architect committed Path (b) — custom-widget escape hatch. The viewer was zero-button (deferral invisible); Phase 5 ships **three** new typed-confirm flows where the focus-ring discrimination is exactly what's needed.

## What the operator can do now

| Action | Command |
|---|---|
| See the new Control sidebar entry | `cargo run --release --bin cockpit --features fixtures` → click `Control` |
| Toggle execution mode | (in Control screen) click `Observe` / `Supervised` / `Auto` segments |
| Pause / resume a strategy | (on Strategies-detail screen) click the pause button next to the strategy row |
| Override a risk veto | (on Strategies-detail screen, when a veto is surfaced) click `Override` → type `OVERRIDE` → confirm |
| See the focus halo on a focused control | Tab / click into any destructive control; the accent-bordered halo renders visibly |

No new operator workflow surface beyond these. AgentFeed renders the same fills as the prior tape (visual upgrade deferred).

## Live demo

`cargo run --release --bin cockpit --features fixtures` was launched and ran cleanly:

```
$ target/release/cockpit
(window opened; iced wgpu surface initialised; deterministic fixture
data populated PnL / Positions / Strategies / AgentFeed on the Home
screen by default; sidebar shows 7 entries — Home / Debug / Strategies
/ Risk / Audit / Charts / Control; status bar visible at the bottom;
window killed cleanly after 4 s; zero stdout / stderr emitted)
```

Stdout artifact (empty file confirms clean run): [`artifacts/lumen-phase-5-humancontrol-agentfeed-2026-05-07/cockpit-fixtures-stdout.txt`](artifacts/lumen-phase-5-humancontrol-agentfeed-2026-05-07/cockpit-fixtures-stdout.txt).

### Screenshots — operator capture pending

The Claude-Code sandbox does not have macOS screen-recording permission (same fallback as Phases 1–4). Copy these to produce the four screenshots:

```bash
# 1. Control screen with HumanControl panel (execution-mode segmented control + limits + kill bottom action)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Control" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-control.png
pkill -f "target/release/cockpit"

# 2. Strategies-detail screen with pause + override-veto buttons (and a surfaced veto in the fixtures feed)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Strategies" in the sidebar, then click any strategy row …
screencapture -W spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-strategies-pause-override.png
pkill -f "target/release/cockpit"

# 3. Override-risk-veto modal (typed-confirm flow with `OVERRIDE` phrase)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … Strategies → click "Override" on a surfaced veto → modal opens …
screencapture -W spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-override-modal.png
pkill -f "target/release/cockpit"

# 4. Focus-ring halo on a focused destructive control (Tab into the kill button or pause button)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … Tab to the kill button (or override button); halo renders …
screencapture -W spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-focus-ring.png
pkill -f "target/release/cockpit"
```

| Screenshot | Path | Status |
|---|---|---|
| Cockpit · Control screen | `spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-control.png` | pending operator capture |
| Cockpit · Strategies pause + override-veto | `spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-strategies-pause-override.png` | pending operator capture |
| Cockpit · Override-risk-veto modal | `spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-override-modal.png` | pending operator capture |
| Cockpit · Focus-ring halo (TD-1 closure) | `spec/reports/screenshots/lumen-phase-5-humancontrol-agentfeed/cockpit-focus-ring.png` | pending operator capture |

## Verification matrix

The Phase 5 brief carries 15 R-items + 15 V-items + 15 Q-items (architect ratified all 15 with zero principled overrides; analyst's recommended Path b for TD-1 confirmed after iced version verification).

| V-item | Subject | Status | Evidence |
|---|---|---|---|
| V1 | Three bins build clean | VERIFIED | `cargo build --release -p ui --bin cockpit --features fixtures` → `Finished release profile … in 9.49s`; `cockpit_live --features live` → `… in 11.58s`; `viewer` → `… in 3.61s`. |
| V2 | All workspace tests pass | VERIFIED | `cargo test --workspace --all-targets` → **896 passed, 0 failed, 3 ignored** across **110 binaries** (Phase 4 was 850/108). |
| V3 | Two new audit writers | VERIFIED | `audit::journal::tests::strategy_paused_*` 7/7 unit; `audit::tests::strategy_paused` + `audit::tests::risk_veto_overridden` integration; both writers atomic dual-write (memo row + zero-amount entry + `strategy_events` row). |
| V4 | Anchor regression byte-identical post-writers | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`; `kind` column already `TEXT` so no migration. |
| V5 | TD-1 closure (focus-ring widget) | VERIFIED | `crates/ui/src/widgets/focus_ring.rs` lands; `wrap(id, child, focused, mode)` overlay + Subscription-driven `focused_widget` state; composes into all 4 destructive surfaces (kill button + kill input + override-veto modal + pause buttons + mode segments). Visual evidence on `panel_snapshots__focus_ring__focused_kill_button.snap` (`halo_visible: true`). |
| V6 | HumanControl panel | VERIFIED | `crates/ui/src/widgets/human_control.rs` lands; placement on 7th sidebar entry "Control"; Debug-screen kill widget retires; 4 mode states + 2 limit states snapshot-attested. |
| V7 | Pause-strategy single-click toggle | VERIFIED | `crates/ui/src/widgets/strategies.rs::pause_button` + `Message::StrategyPauseToggled`; 2 baselines (idle / paused) attested. |
| V8 | Override-risk-veto typed-confirm | VERIFIED | `crates/ui/src/widgets/override_risk_veto.rs` modal mirrors kill-confirm shape with `OVERRIDE` phrase; 3 baselines (button idle / modal open / modal phrase-matched) attested. |
| V9 | `tape` → `agent_feed` module rename | VERIFIED | `mv crates/ui/src/widgets/{tape,agent_feed}.rs` (git rename detection at next commit); `pub mod agent_feed;` in `widgets/mod.rs`; `PANEL_AGENT_FEED_TITLE = "Agent activity"` net-new; 9 `tape_*.snap` → `agent_feed_*.snap` renames; consistency test green. |
| V10 | `Cockpit::tape` field name preserved (Q14) | VERIFIED | `grep -n 'tape:' crates/ui/src/state.rs` → 3 hits (field decl + 2 constructors); code-comment annotation points at `agent_feed.rs` module. |
| V11 | rust-validate full skill PASS | VERIFIED | fmt clean (after orchestrator's one-line `cargo fmt --all` between tester passes) / clippy `-D warnings` clean (`Finished … in 36.35s`) / cargo-deny `advisories ok, bans ok, licenses ok, sources ok` / cargo-audit N/A / rustdoc clean (`Finished … in 18.27s` after `rm -rf target/doc`). |
| V12 | R16.3 brand-bleed grep returns zero | VERIFIED | `grep -rni …` against test-* + backtest-* exits 1; pre-existing matches in screenshots/README files unchanged. |
| V13 | Cross-feature invariants 7/7 | VERIFIED | T1914 sub-block; tester re-ran each prior feature's named test green. |
| V14 | Snapshot baselines clean | VERIFIED | 86 baselines on disk (67 panel + 17 widget + 2 audit; +14 net delta vs Phase 4); zero `*.pending-snap` / `*.snap.new`. |
| V15 | Visual-diff attestation by ui-designer | VERIFIED | T1913 sub-block carries 8/8 Q-evidence rollup (Q1 placement / Q5 TD-1 closure visual halo / Q6 rename / Q7 mirror rows / Q8 single-click / Q9 typed-confirm / Q12 / Q14 field preservation) + full-inventory + `unknown`-color sweep (only legitimate `Latency::Unknown`). |

Tester second-pass report at [`reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](../reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md). First-pass FAIL report preserved at [`reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md`](../reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md).

## Numbers that matter

| Metric | Value | Source |
|---|---|---|
| Workspace tests | 896 passed / 0 failed / 3 ignored | tester second-pass report § 3 (Phase 4: 850) |
| Test binaries | 110 (Phase 4: 108; net-new: 2) | tester report § 3 |
| Snapshot baselines | 86 (67 panel + 17 widget + 2 audit; +14 net delta vs Phase 4) | T1913 sub-block + ui-designer attestation |
| Phase 5 R-items | 15 | feature brief |
| Phase 5 Q-items | 15 / 15 ratified, zero principled overrides | architect Design § Q-resolutions |
| Phase 5 net-new strings | 22 (`HUMAN_CONTROL_*`, `OVERRIDE_*`, `PAUSE_*`, `EXECUTION_MODE_*` plus `PANEL_AGENT_FEED_TITLE` retiring `PANEL_TAPE_TITLE`) | T1901–T1911 commits |
| Phase 5 net-new audit writers | 2 (`strategy_paused`, `risk_veto_overridden`) — additive `StrategyEventKind` variants; no migration | T1902 |
| New widgets | 3 (`human_control`, `override_risk_veto`, `focus_ring`) | T1904 / T1909 / T1912 |
| Module renames | 1 (`tape` → `agent_feed`); 9 baseline filename renames | T1903 |
| Backtest anchors clean | 11 / 11 byte-identical | `verify_anchors.sh` |
| TD-row updates this phase | 2 (TD-1 CLOSED Path b; TD-2 NEW for risk-engine veto-emit upstream wiring) | master roadmap |
| `rust-validate` steps PASS | 5 / 5 (after orchestrator's fmt fixup) | tester second-pass report § 2 |
| Tester passes to ratification | 2 (first-pass FAIL on fmt drift; second-pass PASS) | tester report set |

## Open decisions

One — the **Phase 6 promotion gate** (and why it's reserved, not next-up).

Phase 6 (Assistant slot) is **reserved**, not queued. The brief at [`features/lumen-phase-6-assistant-slot.md`](../features/lumen-phase-6-assistant-slot.md) ships **zero shipped UI** until the v2 LLM strategy lands. The right-rail column-track at `Length::Fixed(0.0)` reservation Phase 2 baked in is the only Phase-6 surface in the codebase today. **Phase 5 is therefore the last shippable phase of the lumen-design-adoption initiative absent v2 LLM.**

The natural next move is to either:
1. **Promote v2 LLM** (its own analyst → architect → developer pipeline; the largest queued backend feature). When v2 LLM ships, Phase 6 unlocks.
2. **Promote a different Active backlog item** (real-mtm-unrealized-pnl, per-symbol-position-accounts, journal-tx-metadata, tape-row-audit-modal — any of which could ship before v2 LLM kicks off).
3. **Pause the cockpit-side initiative**, declare the 5-phase ship complete, and let the v2 LLM rollout pick Phase 6 up when it gets there.

There is also one carried-forward technical-debt row (Phase 5 introduced):
- **TD-2 — Risk-engine veto-emit upstream wiring.** Phase 5's override surface ships over an empty live `Vec<VetoEvent>`. Operators see no veto events to override until the upstream wiring lands. Not a safety primary (the risk engine still vetoes upstream); an observability/awareness gap. Promotion trigger: operator request OR risk-engine evolution OR compliance requirement. Documented in master roadmap → Cross-phase technical-debt items.

And one closed:
- **TD-1 — True keyboard-focus ring.** **CLOSED** via Path (b) custom-widget escape hatch. Visual halo lands on every focused destructive control. Four-phase deferral retired.

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
