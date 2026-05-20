---
slug: per-symbol-position-accounts
mode: release
status: approved
audience: human-operator
updated: 2026-05-03
generated: 2026-05-03T08:08:27Z
approved_by: vitaliy.schreibmann@senacor.com
approved_at: 2026-05-03
---

# Per-symbol position accounts — release

## TL;DR

The audit ledger now uses per-symbol position accounts
(`assets:position:BTCUSDT`, `assets:position:ETHUSDT`, …) instead of one
hardcoded `assets:position:BTC` for every fill — symbol attribution is
now structural, not parsed from a description string.

## What changed

- Migration `006_per_symbol_position_accounts.sql` adds 10
  `assets:position:<SYMBOL>` rows to the `accounts` table (one per
  pair-symbol in `config/agent.toml [funding].universe`).
- `audit::journal::post_fill` writes both the Buy debit and Sell credit
  to the per-pair account derived from `fill.symbol`
  (`format!("assets:position:{}", fill.symbol)`); function signature
  unchanged.
- `audit::query::open_positions_at` keeps description-parse as the
  primary symbol source (handles legacy + new rows uniformly); the
  account-id is a defensive warn-only cross-check (Q4 design choice).

## Why

Pre-feature, every fill — BTCUSDT, ETHUSDT, SOLUSDT, all of them —
landed on the single `assets:position:BTC` row. Symbol attribution
survived only on the **string side** of `journal_transactions.description`
(`"buy 1.0 ETHUSDT @ 2000"`), so any change to the description format
(adding a strategy tag, refactoring `Display for Side`, prettifying
fees) would have silently broken every reader that depended on it
(`pnl_by_symbol`, `open_positions_at`, `recent_fills`). Per-symbol
accounts make symbol attribution structural — carried by an indexed FK
column — and turn description-parse into an internal optimization the
chart of accounts no longer relies on. See
[spec/per-symbol-position-accounts/feature.md § Why](../feature.md#why).

## What you can do now

The cleanup is mostly invisible at the operator level — a string in
the audit DB changed; no new UI, no new command, no new metric. Two
concrete spot-checks:

| Action | Command |
|--------|---------|
| Render a 7d success report against a working ledger | `cargo run -p reports --bin report -- --period 7d --ledger /tmp/audit.db --output /tmp/per-symbol-demo.md` |
| Inspect per-pair `account_id`s after new fills land | `sqlite3 /tmp/audit.db "SELECT account_id, COUNT(*) FROM journal_entries WHERE account_id LIKE 'assets:position:%' GROUP BY account_id;"` |

The first command exercises the full reader path against any ledger.
The second is the structural proof — post-deployment fills will land
on per-pair `account_id`s rather than the legacy `assets:position:BTC`
bucket.

## Live demo

V1 + V2 + V5 + V8 all run through the same test binary
(`crates/audit/tests/per_symbol_post_fill.rs`). Verbatim stdout:

```
$ cargo test -p audit --test per_symbol_post_fill -- --nocapture
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.42s
     Running tests/per_symbol_post_fill.rs (target/debug/deps/per_symbol_post_fill-23e5ec97c62e2acc)

running 4 tests
test t1105_v2_legacy_row_readable_after_migration ... ok
test t1105_v1_post_fill_writes_per_symbol_account ... ok
test t1105_v8_universe_coverage ... ok
test t1105_v5_balance_invariant_pre_and_post_migration ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

`t1105_v1_*` proves the writer flip; `t1105_v2_*` proves legacy rows
on `assets:position:BTC` remain readable; `t1105_v5_*` proves
`Σ debits == Σ credits` across the migration boundary; `t1105_v8_*`
proves every universe symbol has a chart-of-accounts row.

## Screenshots

_n/a — backend-plumbing feature; no UI surface._

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1   | `post_fill` writes per-symbol account-id | VERIFIED | `crates/audit/tests/per_symbol_post_fill.rs::t1105_v1_post_fill_writes_per_symbol_account` PASS (live demo above) |
| V2   | Pre-migration legacy rows still readable | VERIFIED | `t1105_v2_legacy_row_readable_after_migration` PASS — hand-crafted legacy ETHUSDT-on-BTC row survives migration 006 |
| V3   | `open_positions_at` correct on mixed ledger | VERIFIED | `crates/reports/tests/open_positions_mixed_ledger.rs::t1106_v3_mixed_ledger_correct_open_positions` PASS — 3 rows (BTCUSDT, ETHUSDT, SOLUSDT) sorted, correct (qty, avg_cost_basis, strategy_id) |
| V4   | Anchor regression 11/11 | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (Q7 zero-anchor-risk prediction held) |
| V5   | Reconciliation invariant Σ Dr == Σ Cr | VERIFIED | `t1105_v5_balance_invariant_pre_and_post_migration` PASS |
| V6   | operator-success-reports + live-cockpit-unified invariants | VERIFIED | `cargo test --workspace --all-targets` 0 failures; T802/T805/T806/T809/T810/T901/T903a–d/T905/T906–T908/T910/T911/T912 all green |
| V7   | Determinism: two reads byte-identical | VERIFIED | `t1106_v7_two_reads_byte_identical` PASS |
| V8   | Universe coverage | VERIFIED | `t1105_v8_universe_coverage` PASS — every `[funding].universe` symbol has `assets:position:<SYMBOL>` row |

Anchor gate, verbatim:

```
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c
PASS  report-sample-90d                     2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f
---
ANCHORS PASS  (11 / 11)
```

## Numbers that matter

- **New tests:** +10 across the feature — T1101 (+2 migration smoke),
  T1102 (+2 writer/reader), T1105 (+4: V1/V2/V5/V8), T1106 (+2: V3/V7).
  T1107 verifies the 11 anchors.
- **Anchors:** **11 / 11 PASS** — Q7's zero-anchor-risk prediction
  (no committed report body cell renders an `account_id` string) held
  empirically across all 9 backtest scenarios + the 2 v1+
  operator-success-report scenarios.
- **Workspace test sweep:** ~641 tests across 85 binaries, 0 failed,
  0 ignored (per tester report § 3).
- **Build / static:** fmt clean (zero diff), clippy clean, build clean
  (23.92s, 0 warnings), `cargo build -p agent --features
  in_process_cron` clean (Inv-T810).
- **Process notes worth flagging:**
  - One spec deviation caught at fixture extension: T1104 spec wrote
    `"Buy"` with a capital B; runtime `Side::Display` emits lowercase
    `"buy"`, which is what `open_positions_at`'s
    `description LIKE 'buy %'` filter requires. Developer caught and
    corrected mid-task.
  - One architect prediction empirically false but recoverable: T1101
    expected the migration to drop in clean; the dev had to bump 2
    hardcoded `13`→`23` count assertions in
    `crates/audit/tests/ledger_integration.rs` (10 new account rows).
    Test-side update only.
  - One stalled agent re-spawn: T1101's first attempt hit the
    watchdog; a tighter brief on the second attempt succeeded.

## Open decisions

_no decisions pending — ready to ship_

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_(empty — operator fills)_

## Changelog

- 2026-05-03 (presenter): initial draft. V1–V8 all VERIFIED in tester
  report `spec/archive/test-2026-05-03-0803-per-symbol-position-accounts-final.md (archived; see spec/archive/README.md)`;
  anchors 11/11 PASS re-verified live in this presentation; live demo
  re-runs `cargo test -p audit --test per_symbol_post_fill -- --nocapture`
  (4/4 PASS); approval block ships UN-TICKED per the mechanical
  pre-tick gate (`scripts/check_presentation.sh`).
- 2026-05-03 (operator approval): vitaliy.schreibmann@senacor.com
  approved ship — ticked `[x] Approved — ship` (line 155). Status
  `draft → approved`. Mechanical pre-tick gate held: presenter shipped
  the file UN-TICKED (third presenter fire, first clean ship since
  the script-based enforcement landed); operator ticked the box; this
  is the authoritative approval. Feature is fully complete: dev
  pipeline T1101–T1107, tester T_FINAL_PER_SYMBOL, status `shipped`
  in both task and feature files since 2026-05-03 08:03.
