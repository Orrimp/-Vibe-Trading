---
slug: tape-row-audit-modal
mode: release
status: approved
audience: human-operator
updated: 2026-05-03
generated: 2026-05-03T14:05:16Z
approved_by: vitaliy.schreibmann@senacor.com
approved_at: 2026-05-03
---

# Tape-row → audit modal — release

## TL;DR

Click any tape row to see the full audit ledger trail (debits, credits, transaction_id, source strategy) — the cockpit's first click-through-to-audit modal.

## What changed

- **New `journal_transaction_modal` widget** — the workspace's first `iced::widget::Stack` overlay. Pure iced, no new deps; modal renders only when `Cockpit.tape_audit_modal == Some(_)`, so the cockpit body's iced tree is byte-identical to today when the modal is closed.
- **3 new theme tokens** — `bg_overlay` (`#0B0D12`), `info` (`#7BC2FF`), `border_strong` (`#3A4456`) added to `crates/ui/src/theme.rs::color`. First concrete consumer of the design system from `spec/ui-design-principles.md`; the modal's backdrop, frame, and transaction-id text are the surfaces that exercise them.
- **New audit reader + plumbing** — `audit::query::journal_entries_for_transaction(&Ledger, &str) -> Result<Vec<JournalEntry>, LedgerError>`, a new `core::JournalEntry` struct (un-collapsed `(debit, credit)` pair for the 4-col view), `Fill::transaction_id: Option<SmolStr>` and `FillView::transaction_id: SmolStr` plumbed through the live runtime. `audit::journal::post_fill` now returns the generated `txn_id`.

## Why

The "Show the why" UI principle (`spec/ui-design-principles.md`) is a first-class rule: every order, signal, fill, risk veto, and strategy event must be click-through to its decision trail in the audit ledger. Today's tape was read-only in the literal sense — the operator saw fills scroll past with no drill-down, and answering "why did the agent post that fill?" required leaving the cockpit and writing SQL. This feature implements click-through-to-audit on the cockpit for the first time. It is the precedent every future drilldown (positions, strategy events) inherits.

## What you can do now

| Action | Command |
|--------|---------|
| Run the live cockpit and click a tape row | `cargo run --release --bin cockpit_live -- --config config/agent.toml` |
| Run the dev cockpit with fixtures (no exchange creds needed) | `cargo run --bin cockpit --features fixtures` |
| See the underlying journal_transaction for any fill | click any row in the live tape — the modal opens with the 4-col ledger view |
| Close the modal | `Esc`, click outside, or click `Close` |

## Live demo

```
$ cargo test -p ui --features live --test tape_row_click_opens_modal -- --nocapture
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.82s
     Running tests/tape_row_click_opens_modal.rs (target/debug/deps/tape_row_click_opens_modal-34561c196c2137fa)

running 8 tests
test t1208_v1_click_opens_modal_with_correct_tx_id ... ok
test t1208_v4_query_failure_renders_error_state ... ok
test t1208_determinism_two_runs_produce_identical_state_transitions ... ok
test t1208_v5c_agent_halt_closes_modal ... ok
test t1208_v3_empty_entries_renders_empty_state ... ok
test t1208_v5a_close_clears_modal ... ok
test t1208_v1_loaded_view_populates_ready_state ... ok
test t1208_v5b_open_new_tx_replaces_modal ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

This 8-test suite is the V1/V5 state-machine proof: click → loading → ready, error injection, empty state, three close paths (V5a explicit close, V5b replace-on-new-click, V5c agent-halted clears modal), and determinism across two runs.

## Screenshots

This is the first feature where rendered UI matters. Four panel snapshots already exist as `.snap` text files under `crates/ui/tests/snapshots/` — they describe the modal's logical state (title, header rows, columns, copy) and are what the snapshot suite asserts byte-identical:

- `panel_snapshots__tape_audit_modal_loading.snap`
- `panel_snapshots__tape_audit_modal_empty.snap`
- `panel_snapshots__tape_audit_modal_error.snap`
- `panel_snapshots__tape_audit_modal_ready_paper_fill.snap`

Excerpt from `tape_audit_modal_ready_paper_fill.snap` (the happy-path 4-entry fixture):

```
panel: tape_audit_modal
title: Journal transaction
state: ready
tx_id: 4f9a2c1e-aaaa-bbbb-cccc-000000000001
close_label: Close
header:
  Transaction ID: 4f9a2c1e-aaaa-bbbb-cccc-000000000001
  Time: 2026-05-03 14:32:18.0 +00:00:00
  Description: buy 0.04 BTCUSDT @ 50000
  Strategy: sma_crossover
columns: Account | Debit | Credit | Currency
rows:
  assets:cash:USDT | 0.00 USDT | 1,234.56 USDT | USDT
  assets:position:BTCUSDT | 0.04 USDT | 0.00 USDT | BTCUSDT
  assets:cash:USDT | 0.00 USDT | 1.23 USDT | USDT
  expenses:fees:exchange | 1.23 USDT | 0.00 USDT | USDT
```

For an actual rendered PNG (if the operator wants pixels rather than a text snapshot), capture manually:

```
# manual capture (sandbox is headless / no exchange creds in CI)
scripts/capture_screenshot.sh cockpit_live \
    "click a tape row, wait for modal Ready state" \
    spec/<slug>/reports/screenshots/tape-row-audit-modal/modal-ready.png
```

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1   | Click → modal opens with correct entries | VERIFIED | `t1208_v1_click_opens_modal_with_correct_tx_id` + `t1208_v1_loaded_view_populates_ready_state` PASS |
| V2   | Modal renders entries correctly (panel snapshot) | VERIFIED | `panel_snapshots__tape_audit_modal_ready_paper_fill.snap` byte-stable |
| V3   | Empty transaction → empty state | VERIFIED | `t1208_v3_empty_entries_renders_empty_state` PASS |
| V4   | Query failure → error state, cockpit doesn't crash | VERIFIED | `t1208_v4_query_failure_renders_error_state` PASS |
| V5   | Three close paths + replace-on-reopen | VERIFIED | `t1208_v5a_close_clears_modal` / `t1208_v5b_open_new_tx_replaces_modal` / `t1208_v5c_agent_halt_closes_modal` PASS |
| V6   | Anchors 11/11 PASS | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` |
| V7   | Existing UI tests stay green | VERIFIED | `cargo test -p ui` + `cargo test -p ui --features live` + `cargo test --workspace --all-targets` — 0 failures; 32 pre-existing `panel_snapshots__*` byte-identical |
| V8   | Modal snapshot in compact density on 4-entry fixture | VERIFIED | `panel_snapshots__tape_audit_modal_ready_paper_fill.snap` byte-identical across two runs |
| V9   | T802 / T805 / T806 / T809 / T810 invariants | VERIFIED | audit-suite tests 8/2/6/4 PASS + workspace build clean |
| V10  | `cockpit --features fixtures` backwards compat | VERIFIED | `cargo build -p ui --bin cockpit --features fixtures` Finished clean |
| V11  | T901–T912 live-cockpit-unified invariants | VERIFIED | `cargo test -p agent` (33 + 31 PASS) + `cockpit_live_kill_button_writes_audit` PASS + release-bin build clean |

## Numbers that matter

- **Tests added: +26 new tests** — T1201 +1 round-trip, T1202 +3 reader, T1203 +5 token, T1205 +5 widget, T1206 wires 5 `Message` variants exhaustively, T1207 +4 modal snapshots, T1208 +8 state-machine.
- **Anchors: 11/11 PASS** — body-SHA-256 byte-identical to `spec/anchors.toml`.
- **Workspace test count: ~100 live-feature tests, 0 failures.**
- **Pre-existing panel snapshots: 32/32 byte-identical** — T1206 view-branching invariant held; the modal's view branch only fires when `tape_audit_modal == Some(_)`.
- **First UI feature against `spec/ui-design-principles.md`** — establishes the precedent for every future click-through-to-audit drilldown.

## UI principles compliance

This is the **first feature shipped against `spec/ui-design-principles.md`**; it documents the precedent every future UI feature presentation will reference.

| Principle | How this feature exercises it |
|-----------|-------------------------------|
| Show the why | Entire feature exists to satisfy this principle — every tape row is now click-through to its `journal_transaction`. |
| No blank screens | Modal carries all four `PanelState<T>` variants — Loading / Empty / Error / Ready (R8). Snapshots cover all four. |
| Plain language | Column headers are `Account` / `Debit` / `Credit` / `Currency`, not `account_id` / `debit_amount`. All copy in `ui::strings`. |
| Numbers are scannable | Debit / credit cells right-aligned, monospace digits via `widgets::num`, locale-default thousands separator; sign-of-side encoded by *which column* the number sits in, not by sign. |
| Iconography | Close button is text `"Close"`, not a glyph. Backdrop is plain color, no chrome. |
| Confirm destructive actions | N/A — modal is read-only (R5). |
| Accessibility | `Esc` closes; focus ring uses `border_strong` (not `accent`); `info` (`#7BC2FF`) on `bg_elev` clears WCAG-AA 4.5:1 contrast. |
| Color is never the only signal | Debit / credit columns are *labeled*, not just *colored* — column headers carry the meaning. |
| Density | Modal honors compact density — table-row 24 px, cell pad 12 px, dialog inner pad 24 px. |
| Voice and copy | Direct, terse, present-tense, sentence case, unicode `…` in `TAPE_AUDIT_MODAL_LOADING`. No "Please", no "Sorry". |
| Consistency | Zero inline hex (`no_inline_hex_colors_in_widgets_or_state` green), zero inline strings (`no_inline_user_visible_strings_in_widgets` green), `Message::*` exhaustive — three new arms all handled, no `_ => {}` catch-all. |

Future click-through-to-audit features (positions-drilldown, strategy-events-drilldown) will re-use this feature's pattern: `Stack` overlay + `bg_overlay` backdrop + `border_strong` frame + Esc-close subscription + `*Clicked` / `*ModalClosed` / `*EntriesLoaded` message triplet.

## Open decisions

_no decisions pending — ready to ship_

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

## Changelog

- 2026-05-03 (presenter): initial draft. Release-mode presentation built on tester's PASS report (`spec/archive/test-2026-05-03-1351-tape-row-audit-modal-final.md (archived; see spec/archive/README.md)`). Anchors 11/11 PASS. Live demo: `cargo test -p ui --features live --test tape_row_click_opens_modal -- --nocapture` 8/8 PASS. First UI feature through the presenter pipeline; UI principles compliance section establishes the precedent every future UI feature will reference.
- 2026-05-03 (operator approval): vitaliy.schreibmann@senacor.com approved ship — ticked `[x] Approved — ship` (line 146). Status `draft → approved`. Mechanical pre-tick gate held: presenter shipped UN-TICKED (4th presenter fire, 2nd clean ship via the script gate after sandbox-denial-then-orchestrator-verifies pattern). Feature is fully complete; first UI feature through the presenter pipeline + first against `spec/ui-design-principles.md`.
