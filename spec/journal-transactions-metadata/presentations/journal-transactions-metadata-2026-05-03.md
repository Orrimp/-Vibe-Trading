---
slug: journal-transactions-metadata
mode: release
status: approved
audience: human-operator
updated: 2026-05-03
generated: 2026-05-03T16:14:34Z
approved_by: vitaliy.schreibmann@senacor.com
approved_at: 2026-05-03
---

# Journal-transactions metadata reader — release

## TL;DR

The tape-row audit modal now shows full transaction context (description + `strategy_id`) — closing the partial-view gap from the `tape-row-audit-modal` ship.

## What changed

- **New `core::JournalTransactionMetadata` struct** — header-only DTO (`transaction_id`, `ts`, `description`, `strategy_id`) at `crates/core/src/views.rs:62-83`, re-exported from `core::lib` alphabetically alongside `JournalEntry`. Lives in `core` (not `ui`) so the audit reader doesn't depend on UI types.
- **New `audit::query::journal_transaction_metadata` reader** — sibling of T1202's `journal_entries_for_transaction` at `crates/audit/src/query.rs:347-403`. Single-row `SELECT id, ts, description, strategy_id FROM journal_transactions WHERE id = ?`; returns `Ok(None)` for stale clicks (never `Err` for missing rows), mirroring T1202's empty-result contract.
- **Cockpit_live's modal-fetch path now chains metadata→entries** — `crates/ui/src/bin/cockpit_live.rs:496-552` replaces T1206's partial-view construction (which defaulted `description: SmolStr::default()` and `strategy_id: None`) with a sequential await chain. Any `Err` (or metadata `Ok(None)`) collapses to the modal `Error` state via the existing `TAPE_AUDIT_MODAL_ERROR_PREFIX` copy.

## Why

The just-shipped `tape-row-audit-modal` had a known partial-view gap: live mode rendered the 4-column entries table correctly, but the modal header's `description` was empty and `strategy_id` fell through to `—`. The T1206 dev's deviation note flagged it explicitly. This feature is that follow-up: the schema already had the columns (migration 001 + 004), so the work is purely a new read path + a closure rewire — additive, read-only, off every anchored path. (See `spec/journal-transactions-metadata/feature.md` § Why.)

## What you can do now

| Action | Command |
|--------|---------|
| Run the live cockpit and see full transaction context on click | `cargo run --release --bin cockpit_live -- --config config/agent.toml` |
| Click any tape row | modal opens with populated header (`description` + `strategy_id`) and the 4-col ledger view below |
| Run the dev cockpit with fixtures (no exchange creds needed) | `cargo run --bin cockpit --features fixtures` |

## Live demo

```
$ cargo test -p ui --test cockpit_live_modal_metadata_chain -- --nocapture
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.79s
     Running tests/cockpit_live_modal_metadata_chain.rs (target/debug/deps/cockpit_live_modal_metadata_chain-f7151c25e4357caa)

running 2 tests
test t1304_v3b_unknown_tx_short_circuits_to_error ... ok
test t1304_v3_chained_fetch_populates_view_header ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

V3 wiring proof — happy path asserts the resulting `JournalTransactionView` carries `description == "buy 0.4 BTCUSDT @ 52341.20"` and `strategy_id == Some(StrategyId::new("sma-cross-btc-1m"))` (T1206 defaults gone); defensive case asserts a bogus UUID short-circuits to `Err("{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction")` (Q6 None-arm).

## Screenshots

_n/a — modal renders identically; provenance-blind by design (Q5 architect call). Smoke test (T1304) is the V3 evidence._

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | Reader returns metadata for an existing transaction | VERIFIED | `crates/audit/tests/journal_transaction_metadata.rs::t1302_v1_returns_metadata_for_existing_transaction ... ok` (suite 3/0/0) |
| V2 | Reader returns `Ok(None)` for an unknown tx_id | VERIFIED | same file `t1302_v2_returns_none_for_unknown_tx_id ... ok` |
| V3 | Chained fetch populates the `JournalTransactionView` header | VERIFIED | `crates/ui/tests/cockpit_live_modal_metadata_chain.rs::t1304_v3_chained_fetch_populates_view_header ... ok` + `t1304_v3b_unknown_tx_short_circuits_to_error ... ok` (2/0/0) |
| V4 | Anchors 11/11 PASS | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` |
| V5 | Operator-success-reports + live-cockpit invariants hold | VERIFIED | `cargo test --workspace --all-targets` 0 failed; `cargo test -p ui --features live` clean (panel_snapshots 36/36 byte-identical) |

## Numbers that matter

- **Tests:** +7 new (T1301 +2 round-trip serde; T1302 +3 reader; T1304 +2 wiring smoke). Workspace: ~89 test suites, **0 failures**.
- **Anchors:** **11/11 PASS** (`bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`).
- **Panel snapshots:** **36/36 byte-identical** (32 cockpit + 4 modal — all `tape_audit_modal_{loading,empty,error,ready_paper_fill}` snaps unchanged; Q5 invariant held).
- **Cross-feature invariants:** all 4 prior features verified GREEN — operator-success-reports (T802/T805/T806/T809/T810); live-cockpit-unified (T901–T912); per-symbol-position-accounts (T1101–T1107); tape-row-audit-modal (T1201–T1209).
- **Scope:** 1 new struct, 1 new reader, ~30-line closure rewire. Zero new deps, zero migrations, zero new strings/theme tokens/widget files.

## UI principles compliance

The chain-fetch path honors the design principles documented in `spec/ui-design-principles.md`:
- **No blank screens** — any `Err` (or metadata `Ok(None)`) collapses to `PanelState::Error` (Q6); operator never sees an empty header that could mean "no description" or "couldn't load".
- **Show the why** — the modal now actually shows `description` + `strategy_id` (the whole point of this fire); was `""` / `None` in T1206.
- **Determinism** — sequential await is deterministic and short-circuits on stale clicks; `tokio::join!` was rejected on ergonomics + no-short-circuit grounds (Q4).
- **Consistency** — no inline error strings; reuses `TAPE_AUDIT_MODAL_ERROR_PREFIX` from `crates/ui/src/strings.rs`.

## Open decisions

_no decisions pending — ready to ship_

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_(empty until operator fills)_

## Changelog

- 2026-05-03 (presenter): initial draft. Release-mode presentation for the `journal-transactions-metadata` feature; tester PASS at `spec/archive/test-2026-05-03-1608-journal-transactions-metadata-final.md (archived; see spec/archive/README.md)`. Live demo: `cargo test -p ui --test cockpit_live_modal_metadata_chain -- --nocapture` 2/2 PASS. Anchor gate: `ANCHORS PASS (11 / 11)`. V1–V5 all VERIFIED. Approval block UN-ticked per mechanical pre-tick gate.
