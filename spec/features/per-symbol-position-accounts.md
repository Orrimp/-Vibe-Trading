---
slug: per-symbol-position-accounts
status: shipped
owner: architect
updated: 2026-05-03
---

# Per-symbol position accounts

## Why

Today the audit ledger uses **one** position account
(`"assets:position:BTC"`) for **every** fill, regardless of symbol.
`audit::journal::post_fill` hardcodes the account-id literal at
[`crates/audit/src/journal.rs:82`](../../crates/audit/src/journal.rs)
(Buy debit) and
[`crates/audit/src/journal.rs:135`](../../crates/audit/src/journal.rs)
(Sell credit). An ETHUSDT Buy and a BTCUSDT Buy both land on the same
`assets:position:BTC` row.

Symbol attribution is preserved only on the **string side** of
`journal_transactions.description` (format
`"<side> <qty> <symbol> @ <price>"`). The `real-mtm-unrealized-pnl`
reader (`audit::query::open_positions_at`) and `pnl_by_symbol` both
work around the BTC-only account by parsing the symbol out of that
free-text description via `extract_symbol_from_description`
([`crates/audit/src/query.rs:512`](../../crates/audit/src/query.rs)).

Two operational consequences:

1. **Fragility.** Any change to the description format (e.g. adding a
   strategy tag, a fee pretty-print, or refactoring `Display for Side`)
   silently breaks the symbol attribution for every reader that depends
   on it (`pnl_by_symbol`, `open_positions_at`, `recent_fills`).
2. **Wrong shape.** The chart of accounts already declares
   per-symbol accounts via `bootstrap::seed_universe_accounts`
   ([`crates/audit/src/bootstrap.rs:65`](../../crates/audit/src/bootstrap.rs)),
   which idempotently inserts `assets:position:<ASSET>` rows into the
   `accounts` table. The plumbing is half-done: the chart can carry
   per-symbol rows, but the writer never targets them.

Per-symbol position accounts (`assets:position:BTCUSDT`,
`assets:position:ETHUSDT`, …) make symbol attribution **structural**
(carried by `journal_entries.account_id`, an indexed FK column) rather
than **string-encoded** (parsed out of free-text). The chart of
accounts becomes the single source of truth for "which symbol does this
fill belong to"; description-parse becomes an internal optimization
that future readers may or may not use.

The `real-mtm-unrealized-pnl` architect explicitly punted this work
to a follow-up brief
([real-mtm-unrealized-pnl.md `## Design` § Q3 / R10 verdict, lines
386–401, 541–554](real-mtm-unrealized-pnl.md)) citing two risks:
chart-of-accounts migration risk + 9-anchor regression sensitivity.
This brief is that follow-up.

## Requirements (R-items)

### R1 — Schema migration `006_per_symbol_position_accounts.sql`

Migration extends the **chart of accounts** at the row level:
`INSERT OR IGNORE` an `assets:position:<SYMBOL>` row for every symbol
the agent will trade. The current schema (one `accounts` table with
`id TEXT PRIMARY KEY`, see
[`crates/audit/migrations/001_chart_of_accounts.sql:4-9`](../../crates/audit/migrations/001_chart_of_accounts.sql))
already supports per-symbol rows; the migration adds rows, not columns.

- **Cadence precedent:** matches operator-success-reports' additive
  pattern (`004_journal_transactions_strategy_id.sql` ALTER TABLE,
  `005_uptime_intervals.sql` CREATE TABLE — both purely additive). The
  new migration is `006_*.sql` and SHOULD ONLY contain idempotent
  `INSERT OR IGNORE` statements.
- **Account-id format:** `assets:position:<SYMBOL>` — `<SYMBOL>` is the
  full pair (e.g. `BTCUSDT`, `ETHUSDT`, `SOLUSDT`), matching the
  `Symbol::new(...)` instances already written into
  `journal_transactions.description`. **NOT** `assets:position:<ASSET>`
  (e.g. `BTC`) — that conflicts with the legacy account row and
  reintroduces the same ambiguity the migration is fixing.
- **Coexistence with `bootstrap::seed_universe_accounts`:** the
  migration MAY simply call into `bootstrap::seed_universe_accounts`'s
  insert pattern, OR `seed_universe_accounts` MAY be deleted in favour
  of the migration. Architect picks (Q1 sub-bullet).

### R2 — `audit::journal::post_fill` writes per-symbol account

`post_fill` selects `format!("assets:position:{}", fill.symbol)` for
both the Buy debit (line 82) and the Sell credit (line 135) instead of
the hardcoded `"assets:position:BTC"` literal. The function signature
is **unchanged** — `fill.symbol` is already in scope from `&Fill`.

- **Untouched call sites (V1 perimeter):** every existing caller of
  `post_fill` keeps compiling and passes its tests:
  - `crates/audit/tests/ledger_integration.rs` (T802 strategy_id tests),
  - `crates/audit/tests/open_positions_at.rs` (real-mtm V1 fixture),
  - `crates/audit/tests/pnl_by_strategy.rs`,
  - `crates/audit/tests/v15a_journal_test.rs`,
  - `crates/exec/src/paper.rs`'s call site (announce-second invariant),
  - any backtest-side journaling path (currently `crates/backtest/`
    does not call `post_fill` directly — verified by grep — so this
    requirement is a no-op for that crate).
- The new account row MUST exist before the first fill writes to it.
  The `journal_entries.account_id` FK reference to `accounts(id)` will
  fail if the migration didn't seed the row. Test fixtures and the
  agent boot path BOTH must run migration `006` before posting any
  fill.

### R3 — `audit::query::open_positions_at` may use account-id suffix

`open_positions_at` (real-mtm T1002) currently scans
`journal_transactions` and parses the symbol from the description.
Post-migration the same reader MAY (architect choice, Q4) switch to a
`SELECT ... FROM journal_entries WHERE account_id LIKE
'assets:position:%'`-style aggregation that strips the prefix. **Public
API is unchanged either way** — `pub async fn open_positions_at(ledger:
&Ledger, ts: Timestamp) -> Result<Vec<OpenPosition>, LedgerError>`
stays in place; semantics (sort order, weighted-avg cost basis, R6
determinism) stay in place.

This requirement is intentionally **soft** ("may"). The optimization
is internal and can land in this feature, defer to a future feature,
or never land. Architect picks the timing.

### R4 — Backwards compat: legacy rows still readable

The migration is **purely additive**. It does NOT rewrite any existing
`journal_entries` row. Every pre-migration row continues to reference
`assets:position:BTC` even when the underlying fill was for ETH/SOL/etc.

Readers that consume historical ledger data MUST continue to handle
this. Concretely:
- `audit::query::pnl_by_symbol` and `open_positions_at` keep using
  description-parse as the primary symbol source for legacy rows;
  if R3 lands, the account-id suffix is a fallback for new rows only.
- `audit::verify_balance` and the global debit/credit invariant pass
  through unchanged — both legacy and new rows remain balanced
  per-transaction at write time.

### R5 — Anchor regression: byte-identical (preferred)

All **11 anchors** in [`spec/anchors.toml`](../anchors.toml) — the 9
backtest body anchors (v0/v0.5/v1/v1.5a) plus the 2 v1+ success-report
anchors (`report-sample-7d`, `report-sample-90d`) — MUST stay
byte-identical.

Justification:
- Backtest reports DO NOT reference `account_id` strings in their body
  (verified by grep over `spec/reports/backtest-*.md`: zero hits for
  `assets:position` / `assets:cash`). They surface symbol via
  `Symbol::Display` and strategy-level rollups from `crates/backtest/`,
  which does not call `audit::journal::post_fill` (verified by grep).
- Success-report bodies (verified by grep over
  `spec/reports/success/success-*.md`: zero hits) derive from
  `pnl_by_symbol` / `pnl_by_strategy`, which parse symbol from the
  transaction description. Descriptions carry the correct symbol
  pre- and post-migration; body bytes are independent of which
  `account_id` row the entry sits on.
- The migration only adds chart-of-accounts rows; no money moves.
  `Σ debits == Σ credits` per transaction is preserved (R6).

A future rendering change that surfaces `account_id` would re-lock
the affected anchor per v1.5a T717 precedent — out of scope here.

### R6 — Reconciliation invariant: `Σ debits == Σ credits` holds

No money moves. The migration adds account rows only; existing
`journal_entries` rows are untouched. `audit::verify_balance(...)` on
every existing transaction id passes pre- and post-migration with the
same result. `global_debit_credit_sum() == Decimal::ZERO` holds across
the migration boundary.

### R7 — Description-parse path remains working (deprecated, not removed)

`extract_symbol_from_description`
([`crates/audit/src/query.rs:512`](../../crates/audit/src/query.rs))
and its consumers (`pnl_by_symbol`, `recent_fills`, `open_positions_at`)
MUST keep working post-migration — R4 requires it for legacy rows.
The function MAY be marked `#[deprecated]`; the body stays. Removal
is out of scope (Q5).

### R8 — operator-success-reports invariants preserved

The following T-task invariants MUST stay green:
- **T802** — `post_fill(strategy_id: Option<&str>)` signature
  unchanged; the `strategy_id` column on `journal_transactions` is
  written verbatim. Per-symbol account rewrite is orthogonal.
- **T805** — feed-reconnect event invariants (independent code path).
- **T806** — agent uptime intervals (`agent_uptime` table; independent).
- **T809** — kill-switch dual-write (`audit_memo` + `strategy_events`).
  Independent of `post_fill`.
- **T810** — `--features in_process_cron`. Independent.
- The 2 v1+ anchors stay byte-identical (R5).

### R9 — live-cockpit-unified invariants preserved

T901 (Prometheus toggle), T903a (`paper::on_fill` announce-second after
`audit::post_fill`), T903b (bar/tick taps), T903c (reconciler
`PnlSnapshot`), T905 (kill-switch/mode forwarder), T906–T908 (UI),
T910 (uptime smoke), T911 (kill-button trips kill-switch), T912
(Prometheus toggle test) all stay green.

T903a is the only T9XX that touches `post_fill`'s call site;
satisfied because R2 holds the signature constant. T903c reads
`audit::query`; public surface is unchanged (R3).

### R10 — Determinism

Two reads of `audit::query::open_positions_at(ledger, ts)` against the
same audit DB MUST return byte-identical `Vec<OpenPosition>`,
pre- and post-migration. (Inherits from real-mtm R6.)

The migration's `INSERT OR IGNORE` is order-independent at the row
level. Account-id strings are deterministic functions of `Symbol`.

### R11 — Universe coverage

The migration MUST seed `assets:position:<SYMBOL>` rows for every
symbol the agent currently trades or backtests. Source of truth:
- `bootstrap::seed_universe_accounts`'s caller list (currently
  uncalled; needs to be wired or its symbol set inlined into the
  migration — architect picks),
- the symbols hardcoded in
  `crates/reports/tests/fixtures/build_ledger_{7d,90d,1y,with_open_positions_7d}.rs`,
- the symbols in the v1+ universe config (`config/universe.toml` or
  similar — architect to confirm path).

Missing a symbol means the first fill for that symbol fails on the FK
reference to `accounts(id)`. The agent boot path SHOULD have a
defensive `seed_universe_accounts(symbols)` call AFTER the
migration runs, so production rollout cannot silently miss a symbol.

## Verification (V-items)

### V1 — Post-migration `post_fill` writes to per-symbol account

Test fixture: empty audit DB, run migrations through `006`, post a
single ETHUSDT Buy fill. Assert:
- exactly one `journal_entries` row exists with
  `account_id = 'assets:position:ETHUSDT'`,
- zero rows with `account_id = 'assets:position:BTC'`.

Symmetric assertion for BTCUSDT, SOLUSDT.

### V2 — Pre-migration legacy rows still readable

Test fixture: hand-crafted audit DB containing the legacy shape (one
ETHUSDT fill that wrote to `'assets:position:BTC'`). Run migration
`006` on top of it. Assert:
- the legacy row is unchanged (`SELECT account_id FROM journal_entries
  WHERE id = ?` still returns `'assets:position:BTC'`),
- `audit::verify_balance(transaction_id)` returns `Ok(())`,
- `audit::query::pnl_by_symbol(...)` correctly attributes the row to
  `Symbol::new("ETHUSDT")` (via description-parse, R7).

### V3 — `open_positions_at` correct on mixed pre/post ledgers

Test fixture: a single audit DB containing BOTH:
- pre-migration shape: 1 BTCUSDT Buy + 1 ETHUSDT Buy, both writing to
  the legacy `'assets:position:BTC'` account, descriptions correct,
- post-migration shape: 1 SOLUSDT Buy writing to
  `'assets:position:SOLUSDT'`, description correct.

Run `audit::query::open_positions_at(&ledger, period_end)`. Assert the
returned `Vec<OpenPosition>` contains exactly 3 rows
(BTCUSDT, ETHUSDT, SOLUSDT) with correct (qty, avg_cost_basis,
strategy_id) tuples. Architect's R3 decision (description-parse vs
account-id-suffix vs hybrid) determines the exact implementation; the
**test contract is shape-equal regardless**.

### V4 — Anchor regression: 11/11 PASS

Run `bash scripts/verify_anchors.sh`. Expect:
`ANCHORS PASS  (11 / 11)`. No re-lock; no `spec/anchors.toml` mutation.

### V5 — Reconciliation invariant

Across the migration boundary on the `build_ledger_with_open_positions_7d`
fixture:
- pre-migration: `audit::verify_balance` returns `Ok(())` for every
  transaction id; `global_debit_credit_sum() == Decimal::ZERO`.
- post-migration (same fixture re-run): identical results.

### V6 — operator-success-reports + live-cockpit-unified invariants

All of the following remain green (no code changes outside `audit/`
should affect them; this V-item is the explicit gate):
- T802, T805, T806, T809, T810,
- T901, T902, T903a–d, T904, T905, T906, T907, T908, T909, T910,
  T911, T912.

Tester runs the existing test binaries unchanged; expects no
regressions.

### V7 — Determinism

Two consecutive `open_positions_at(...)` invocations on the
mixed pre/post fixture (V3) return `Vec<OpenPosition>` slices that
compare equal byte-for-byte via `assert_eq!`. (Real-mtm V7 widened to
include a mixed-shape fixture.)

### V8 — Universe coverage smoke

For every symbol in `config/universe.toml` (or the architect-confirmed
universe-source-of-truth, Q1 sub-bullet), the post-migration `accounts`
table contains a row with `id = 'assets:position:<SYMBOL>'`. Asserted
via a unit test that loads the universe config and queries the audit
DB.

## Backtest scenarios

_n/a — chart-of-accounts plumbing; no new strategy or render._

## Open questions for architect

### Q1 — Migration shape

The current schema (`001_chart_of_accounts.sql`) has a separate
`accounts` table with `id TEXT PRIMARY KEY` and an FK from
`journal_entries.account_id`. The chart-of-accounts is **explicit**,
not implicit in the entries table. Therefore migration `006_*.sql`
should:

- **(a) recommended** — pure data migration, idempotent
  `INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES
  ('assets:position:<SYMBOL>', 'asset', '<BASE_ASSET>')` for every
  symbol in the universe. No schema change. The migration body is a
  list of `INSERT OR IGNORE` lines.
- **(b)** — schema change adding a `symbol` column on
  `journal_entries` extracted from the account_id at write time.
  Reads simplify; storage doubles.
- **(c)** — both: data migration + index on
  `account_id LIKE 'assets:position:%'` for the R3 reader optimization.

**Analyst recommendation:** (a). It's the smallest change that
satisfies R1–R8. (c) is a strict superset of (a) and can be added if
R3 lands in the same wave; the index is cheap to add later.

**Sub-question Q1.x:** is `bootstrap::seed_universe_accounts`
([`crates/audit/src/bootstrap.rs:65`](../../crates/audit/src/bootstrap.rs))
the right place to call from the agent boot path (currently it's
unwired), or should the migration replace it? Recommended: keep
`seed_universe_accounts` (defensive boot-time idempotent seed) AND
ship the migration (audit-DB-bring-up-time seed). Both call the same
`INSERT OR IGNORE` pattern.

### Q2 — account_id format

Two candidates:

- **(a) recommended** — `assets:position:<SYMBOL>` (the analyst's pick;
  matches the `seed_universe_accounts` pattern at
  `bootstrap.rs:70-71`). The strategy is already attributed via the
  T802 `strategy_id` column on `journal_transactions`.
- **(b)** — `assets:position:<SYMBOL>:<STRATEGY>` (one account per
  `(symbol, strategy)` pair). Symmetrical with the per-strategy P&L
  rollup but doubles or triples the `accounts`-table row count.
  Strategy attribution is already structural (T802); duplicating it
  into the account-id is overkill.

**Analyst recommendation:** (a). Strategy stays in the column;
symbol moves to the account-id.

### Q3 — Pre-migration legacy rows: backfill or leave?

- **(a) recommended — purely additive.** The migration adds account
  rows; legacy `journal_entries` rows continue to reference the legacy
  `assets:position:BTC` account regardless of which symbol they were
  for. R7's description-parse path handles legacy reads. **Anchor
  risk: zero**, because legacy rows are unchanged at the row level
  and report bodies don't render account-ids (R5).
- **(b)** — one-time backfill: rewrite legacy
  `assets:position:BTC`-targeted entries to
  `assets:position:<actual_symbol>` based on description-parse.
  **Anchor risk: non-zero**: any reader that aggregates by account_id
  would shift its output, and even though current report renderers
  don't, this opens a future foot-gun (a rendering change that surfaces
  account-ids would silently re-write the historical interpretation).

**Analyst recommendation:** (a). Backfill is irreversible and
violates the additive-migration cadence operator-success-reports set
in `004` and `005`.

### Q4 — `open_positions_at` reader: switch to account-id suffix?

R3 leaves the option open. Three paths:

- **(a)** — switch in this feature. Pros: structural symbol attribution;
  removes one consumer of `extract_symbol_from_description`. Cons:
  hybrid logic (legacy rows: parse description; new rows: parse
  account-id) — both paths must be tested.
- **(b) recommended** — defer to a follow-up feature. This feature's
  scope is the writer + chart-of-accounts; the reader optimization is
  separable. Pros: smaller blast radius; tester gates one thing at a
  time. Cons: description-parse stays in the hot path indefinitely.
- **(c)** — switch + drop legacy support. Out of scope; the audit DB is
  append-only history.

**Analyst recommendation:** (b). Land the writer change first; let
operator confirm anchors stayed byte-identical; THEN refactor the
reader in a small follow-up.

### Q5 — Description-parse deprecation timeline

`extract_symbol_from_description` has two states post-feature:
- **(a) recommended** — keep indefinitely with a `#[deprecated]`
  attribute. Cheap; legacy rows always need it.
- **(b)** — schedule removal in a future feature once Q3 is revisited
  (e.g. v3+ ledger compaction). Out of scope here.

**Analyst recommendation:** (a). The function is 8 lines; it stays.

### Q6 — Testing approach: new fixture or extend `build_ledger_with_open_positions_7d`?

The real-mtm fixture `build_ledger_with_open_positions_7d.rs`
([`crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`](../../crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs))
already mixes BTCUSDT + ETHUSDT fills. Two paths for V3:

- **(a)** — extend the existing fixture with a pre-migration row
  (insert a row directly via SQL bypassing `post_fill`). Pros: one
  fixture; matches the operator-success-reports anchor-test path.
  Cons: the fixture currently builds a deterministic ledger via
  `post_fill` only; bypassing the writer for one row makes the
  fixture's invariants (qty bookkeeping, fee accounting) harder to
  reason about.
- **(b) recommended** — new dedicated fixture
  `build_ledger_mixed_pre_post_migration.rs` (test-only, non-anchored).
  Pros: keeps the existing fixture's properties stable; the new
  fixture's only purpose is to exercise V2 + V3 + V5 + V7 across the
  migration boundary.

**Analyst recommendation:** (b). Fixture isolation matches the Q5
resolution from real-mtm (where the architect added a dedicated
`build_ledger_with_open_positions_7d.rs` rather than extending
`build_ledger_7d.rs`).

### Q7 — Anchor risk audit

Analyst-confirmed by grep over committed reports:
- `spec/reports/backtest-*.md`: zero matches for
  `assets:position` / `assets:cash` / `account_id`.
- `spec/reports/success/success-*.md`: zero matches for the same.

The 9 backtest anchors render strategy-P&L tables that derive from
`crates/backtest/` (a separate code path that does not call
`audit::journal::post_fill` — verified by grep), so they are
**doubly insulated** from this feature: their inputs come from the
backtest engine, not the audit DB.

The 2 v1+ anchors render success reports over the audit DB. Their body
bytes derive from `pnl_by_symbol`, `pnl_by_strategy`, etc. — all of
which read symbol from description, not account-id. R5 holds.

**Architect to confirm before tester locks PASS on 11/11**: any cell
in any anchored report that encodes an `account_id` string is
unfindable by grep. If architect finds one we missed, the migration
strategy may need to be revisited (most likely outcome: stays additive,
but the affected report's renderer skips the account-id cell or
synthesizes it from the symbol).

### Q8 — `seed_universe_accounts` wiring

`bootstrap::seed_universe_accounts` exists but is currently uncalled
(verified by grep across the workspace). Two paths:

- **(a) recommended** — wire it from the agent boot path
  ([`crates/agent/src/runtime.rs`](../../crates/agent/src/runtime.rs))
  with the live universe symbol set, AND ship the migration. Defence
  in depth.
- **(b)** — delete `seed_universe_accounts` and rely on the migration
  alone. Cleaner but loses the boot-time defensive seed (a new symbol
  added to the universe between migrations would fail at first fill).

**Analyst recommendation:** (a).

## Design

Author: architect, 2026-05-03. Resolves Q1–Q8 from the analyst brief
above. This is a **plumbing-only** feature riding on top of an
already-implemented-but-uncalled pattern
(`audit::bootstrap::seed_universe_accounts` at
[`crates/audit/src/bootstrap.rs:65`](../../crates/audit/src/bootstrap.rs)).
No new strategy code, no new render, no new UI surface. Design
length budget: ≤ 350 lines. Anchor budget: 11 / 11 byte-identical
(R5).

### Independent verification of analyst findings (2026-05-03, architect)

Re-grepped before designing on top:

| Claim | Result |
|---|---|
| `crates/audit/src/journal.rs:82,135` BTC hardcode | CONFIRMED — line 82 (`Buy` debit) and line 135 (`Sell` credit) both pass the literal `"assets:position:BTC"`. `fill.symbol` is in scope on both lines (used in the `description` `format!` at line 50). |
| `crates/audit/src/query.rs:512` `extract_symbol_from_description` | CONFIRMED — exists at line 512, parses `parts[2]` from `"<side> <qty> <symbol> @ <price>"`. |
| `crates/audit/src/bootstrap.rs:65` `seed_universe_accounts` | CONFIRMED **with one critical correction**: signature is `seed_universe_accounts(ledger, base_assets: &[&str])` — takes **base assets** (e.g. `"BTC"`, `"ETH"`), NOT pair symbols (e.g. `"BTCUSDT"`). Inserts BOTH `assets:position:<ASSET>` AND `assets:position_mark:<ASSET>` rows. **This shape does NOT match the Q2 decision** (per-symbol-pair). See § Q8 for the wiring consequence. |
| `crates/audit/migrations/001_chart_of_accounts.sql:4-9` `accounts` separate table | CONFIRMED — `accounts(id TEXT PRIMARY KEY, kind, currency)` plus FK from `journal_entries.account_id`. Pure data migration is sufficient (no schema change needed). |
| `grep "assets:position\|assets:cash" spec/reports/backtest-*.md spec/reports/success/success-*.md` | ZERO hits across all committed report bodies. Anchor risk by construction is zero (Q7). |
| `grep "audit::journal::post_fill" crates/backtest/` | ZERO hits — `crates/backtest/` does NOT call `post_fill`. The per-symbol writer change cannot regress any backtest report. |
| `seed_universe_accounts` callers | ZERO callers across the whole workspace (`grep -rn "seed_universe_accounts" crates/`). |
| `chart_of_accounts` callers | 28 call sites: agent boot path + 25 test bootstraps + cockpit_live bin + cost::sink. **This is the canonical seed point.** |
| Universe of symbols actually traded | `config/agent.toml:62-65` `[funding].universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT"]` (10 symbols). The active strategies (`config/strategies/*.toml`) reference a subset: `top10_momentum_h1` covers BTCUSDT, ETHUSDT, SOLUSDT (+ pairs strategies add BNBUSDT). The migration MUST seed the union (10 symbols) so adding a strategy never trips an FK. |

All analyst claims hold. The one substantive correction is
`seed_universe_accounts`'s asset-vs-symbol mismatch — it does NOT
implement the per-symbol pattern Q2 picks; it implements a per-base-asset
pattern that is the WRONG SHAPE for this feature. Q8 resolution updated
accordingly (see below).

### Q-resolutions

#### Q1 — Migration shape: option (a), purely additive

**Decision.** New migration `006_per_symbol_position_accounts.sql` is a
pure data migration: a list of `INSERT OR IGNORE INTO accounts (id,
kind, currency) VALUES (...)` statements, one per pair-symbol in the
agent's universe. No schema change. No backfill. Matches the
operator-success-reports cadence (`004` = additive ALTER TABLE; `005`
= additive CREATE TABLE; `006` = additive INSERT-only).

**Rationale.** The `accounts` table already has `id TEXT PRIMARY KEY`;
adding rows is the smallest possible change that satisfies R1–R8 and
preserves R6 (no money moves). Pure SQL means the migration is
auditable end-to-end without reading Rust source — operators can `cat`
the file and see exactly which account-ids the chart now contains.

**Alternatives rejected.**
- Option (b) (add `symbol` column on `journal_entries`): doubles
  per-row storage, requires a writer-side schema migration, requires
  rewriting every legacy row to populate the new column → violates the
  "additive" cadence and reintroduces the backfill question Q3 closed.
- Option (c) (data + index on `account_id LIKE 'assets:position:%'`):
  premature optimization. The R3 reader stays description-parse (Q4),
  so no consumer needs the prefix index yet. Adding it later is one
  additional migration file — cheap.
- Calling `seed_universe_accounts` from the migration: not possible — the
  migration is pure SQL (`sqlx::migrate!` runs `.sql` files); Rust code
  cannot run inside it. Even if it could, the `seed_universe_accounts`
  asset-vs-symbol mismatch (see independent verification above) means
  it produces the wrong rows.

#### Q2 — account_id format: `assets:position:<SYMBOL>` (full pair)

**Decision.** Account-id format is `assets:position:<SYMBOL>` where
`<SYMBOL>` is the full Binance pair (e.g. `assets:position:BTCUSDT`,
`assets:position:ETHUSDT`). Strategy attribution is NOT encoded in the
account-id — it stays in the T802 `journal_transactions.strategy_id`
column.

**Rationale.** Symbol moves to the structural account-id; strategy
stays in the column. Two orthogonal axes; encoding both in the
account-id would explode the row count of `accounts` quadratically
(`|symbols| × |strategies|`) and re-encode information already
structurally tracked. The full pair (not the base asset) matches the
existing `journal_transactions.description` symbol token (which the
description-parse path already extracts via
`extract_symbol_from_description`), so the reader-side consistency
check (Q4) is a string equality.

**Alternatives rejected.**
- `assets:position:<BASE>` (e.g. `assets:position:BTC`) — collides with
  the pre-migration legacy account `assets:position:BTC` and
  reintroduces the same ambiguity (was this BTCUSDT? BTCUSD? BTCBUSD?).
  Also breaks the description-parse cross-check, since descriptions
  carry the full pair.
- `assets:position:<SYMBOL>:<STRATEGY>` — duplicates T802; rejected per
  analyst recommendation.

#### Q3 — Backfill: NO backfill, purely additive

**Decision.** The migration adds account rows only. No `UPDATE` of any
existing `journal_entries` row. Legacy rows (every pre-migration fill,
which all reference `assets:position:BTC` regardless of underlying
symbol) stay byte-identical at the row level. Reader handles legacy
via the description-parse path (Q4).

**Rationale.** Backfill is irreversible — once a legacy `BTC` row is
rewritten to `ETHUSDT`, there is no audit trail of "this was once
incorrectly attributed". The audit DB is append-only history; the
correct semantic is "from migration `006` onward, all new fills carry
the structural symbol on the account-id; legacy rows preserve their
original (incorrect) account-id and are re-attributed via the
description-parse fallback at read time". This is exactly the same
pattern T802 used for the nullable `strategy_id` column — pre-migration
rows surface as `(unattributed)`.

**Alternatives rejected.**
- One-time backfill (option b in the brief) — anchor risk non-zero
  (any future renderer that surfaces account-ids would re-interpret
  history); irreversible; violates the additive cadence.

#### Q4 — Reader: keep description-parse as primary; account-id as defensive cross-check

**Decision.** `audit::query::open_positions_at` (and `pnl_by_symbol`,
`recent_fills`) keep description-parse as the **primary** symbol source.
After parsing the symbol from the description, the reader MAY (T1106
acceptance: MUST) cross-check that the row's `account_id` is one of:
- the legacy `"assets:position:BTC"` (pre-migration row), OR
- `format!("assets:position:{}", parsed_symbol)` (post-migration row).

If the cross-check fails (i.e. `account_id` starts with
`"assets:position:"` but its suffix doesn't match either expected
form), emit `tracing::warn!` and fall back to the description-parsed
symbol — never raise an error. The defensive check catches a future
renderer or writer-side bug; it is a safety net, not a primary code
path.

**Rationale.** This is the smallest reader-side change that satisfies
R3 (soft "may" requirement) without rewriting the reader's hot path.
Description-parse is cheap (one `splitn(5, ' ')` per row) and works
against pre- AND post-migration rows uniformly. A pure account-id
reader would need branching (legacy `BTC` row → parse description;
post-migration row → parse account-id), which is hybrid logic with two
test paths. By keeping description-parse primary, we get:
- one code path for both row shapes;
- no consumer-side migration (`pnl_by_symbol`, `recent_fills` keep
  working unchanged);
- a defensive consistency check that catches future writer bugs at
  observation time, not at production-issue time.

**Alternatives rejected.**
- Switch to account-id-suffix as primary (analyst Q4 option a): forces
  hybrid logic for legacy compatibility; doubles the test surface; gains
  nothing measurable since description-parse is already deterministic
  and fast.
- Defer reader changes entirely (analyst Q4 option b): leaves no
  cross-check; a future bug in the writer would only surface when the
  description format changes (the original fragility R1 calls out).

#### Q5 — Description-parse deprecation: indefinite, soft-deprecated

**Decision.** `extract_symbol_from_description` is NOT removed and NOT
marked `#[deprecated]`. A doc-comment on the function notes "primary
symbol source for both pre- and post-migration rows; the
account-id-suffix path is a defensive cross-check (Q4) only. New code
that needs structural symbol attribution SHOULD use
`open_positions_at` or `pnl_by_symbol` rather than parsing
description directly."

**Rationale.** The `#[deprecated]` attribute would emit a compiler
warning every time the function is called from the (currently 3) call
sites that legitimately need it (Q4 keeps it as the primary path).
Better to leave the symbol unmarked and document the intent in the
doc-comment. Removal is out of scope; the function is 8 lines and
stays.

**Alternatives rejected.**
- `#[deprecated(note = "...")]` — generates noise on every legitimate
  call; the function is the primary path per Q4.
- Schedule removal (option b) — the function is needed for legacy-row
  reads as long as the audit DB carries pre-migration rows (forever,
  per R3 / R4).

#### Q6 — Fixture: extend `build_ledger_with_open_positions_7d`

**Decision.** Extend the existing T1004 fixture
`crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`
with mixed legacy/new rows for V3. Do NOT create a new fixture file.

**Rationale.** The operator-aligned default (per the architect-task
brief) overrides the analyst's Q6 recommendation (b). Reasons:
1. The existing fixture is already non-anchored (T1004 created it
   precisely so anchor-locked fixtures stay stable). Extending it does
   not risk anchor drift.
2. One fixture for "open-positions reader correctness" + "mixed
   legacy/new ledger" keeps the V3/V5/V7 invariants in a single source
   of ledger truth. Two fixtures would mean two parallel "do these
   match" gates.
3. The fixture's existing posts use `post_fill`. Adding mixed rows
   means: (a) post the pre-migration legacy rows via direct SQL
   `INSERT` to bypass the post-T1102 writer (forcing the legacy
   account-id), (b) post the post-migration rows via the updated
   `post_fill`. This is a 30-line addition to the existing fixture —
   smaller than a whole new file.

**Alternatives rejected.**
- New fixture `build_ledger_mixed_pre_post_migration.rs` (analyst's b):
  duplicates the open-positions setup; introduces a second
  `non-anchored` fixture to track in `crates/reports/tests/fixtures/`.

#### Q7 — Anchor risk: zero, by construction (re-verified)

**Decision.** All 11 anchors in `spec/anchors.toml` stay byte-identical.
No `spec/anchors.toml` mutation. No re-lock.

**Rationale (independently re-verified 2026-05-03).** Two greps
confirm:
- `grep "assets:position\|assets:cash" spec/reports/backtest-*.md
  spec/reports/success/success-*.md` → ZERO hits.
- `grep "audit::journal::post_fill" crates/backtest/` → ZERO hits (the
  9 backtest anchors render from `crates/backtest/`'s engine, which
  bypasses the audit ledger writer entirely).

The 2 v1+ success-report anchors render from `pnl_by_symbol` /
`pnl_by_strategy` over the audit DB. Per Q4 those readers keep
description-parse as the primary symbol source — output is
byte-identical pre- and post-migration on the same fixture. Per R6 no
money moves. Per Q3 no row is rewritten. Verification matrix confirms
11 / 11 stay locked.

**Alternatives rejected.** None — the empirical evidence is conclusive.

#### Q8 — `seed_universe_accounts`: deprecate, do not delete; migration is the source of truth

**Decision.** Migration `006_per_symbol_position_accounts.sql` is the
canonical seed of the per-pair `assets:position:<SYMBOL>` rows. The
existing `bootstrap::seed_universe_accounts(ledger, base_assets)` is
**marked `#[deprecated(note = "...")]`** but kept in tree:
- it has zero callers across the workspace (verified by grep), so the
  `#[deprecated]` warning is silent in normal builds;
- it implements the **WRONG SHAPE** (per-base-asset, not per-pair-symbol)
  for the post-migration world — calling it would produce dead
  `assets:position:BTC` (without the `USDT` suffix) rows that nothing
  references;
- removing it is a separate clean-up task (T1103). Deprecation marks
  the intent; deletion needs a one-task PR with the `git rm` and the
  unit-test removal at `bootstrap.rs::tests::*` (if any).

**Rationale.** `seed_universe_accounts` is a misshape: it predates the
Q2 decision and seeds rows by **base asset** (e.g. `BTC`) rather than
**pair symbol** (e.g. `BTCUSDT`). The migration cannot call into it
(SQL only), and even a Rust-level rewrite (`seed_universe_accounts(["BTCUSDT", …])`
called from `agent::runtime::run` after migrations) would be redundant
with the migration's INSERT statements. `#[deprecated]` lets us
ship migration `006` without forcing the symbol delete in the same
wave; T1103 lands the removal once the migration is shipped.

**Alternatives rejected.**
- Delete `seed_universe_accounts` in T1102 (same task as the writer
  switch): conflates two concerns; if the deletion uncovers a
  forgotten test caller, T1102's anchor-gate failure mode is muddied.
  Keep the deletion separable.
- Keep `seed_universe_accounts` undeprecated as a "defensive boot-time
  seed" (analyst Q8 option a): it inserts the WRONG account-ids
  (`assets:position:BTC` for the BTC base asset, not
  `assets:position:BTCUSDT`). Calling it from `agent::runtime::run`
  would actively pollute the chart of accounts with dead rows.
- Rewrite `seed_universe_accounts` to take symbols (`&[Symbol]`) and
  call it from the agent boot path: in-scope but redundant with the
  migration. Migration `006` runs at `Ledger::open` — every binary
  (agent, cockpit_live, every test) hits it. There is no gap to
  defend in depth against.

### Crate map delta

Touch list (architect-spec; developer fills T1101–T1107 against this):

| Crate | File | Change shape | Anchor impact |
|---|---|---|---|
| `audit` | `crates/audit/migrations/006_per_symbol_position_accounts.sql` (NEW) | Pure SQL: 10 `INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES (?, ?, ?)` lines, one per universe pair-symbol. | None — no body cell renders account-ids (Q7). |
| `audit` | `crates/audit/src/journal.rs:82,135` | Replace literal `"assets:position:BTC"` (both call sites) with a `format!("assets:position:{}", fill.symbol)` (or hoist to `let position_account_id = format!(...)` once at function top and reuse twice). `fill.symbol` is already in scope (used at line 50 `description` format). | None — pre-T1102 rows unchanged; post-T1102 rows write to per-pair account-id; description string unchanged. |
| `audit` | `crates/audit/src/bootstrap.rs:65 seed_universe_accounts` | Add `#[deprecated(since = "...", note = "shape mismatch — use migration 006_per_symbol_position_accounts.sql")]` attribute. Body unchanged. T1103. | None — no callers. |
| `audit` | `crates/audit/src/query.rs::open_positions_at` (and optionally `pnl_by_symbol`, `recent_fills`) | Add a defensive cross-check after `extract_symbol_from_description` (Q4). Description-parse stays primary. | None — primary code path unchanged; cross-check is observation-only. |
| `reports` | `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs` | Extend with mixed legacy/new rows: post N rows via `post_fill` (uses new per-pair writer), pre-pend M rows via direct SQL `INSERT` to the legacy `assets:position:BTC` account. T1104. | None — fixture is already non-anchored. |
| `audit` | `crates/audit/tests/per_symbol_post_fill.rs` (NEW) | V1 + V2 + V8 tests. T1105. | None. |
| `reports` | `crates/reports/tests/open_positions_mixed_ledger.rs` (NEW) | V3 mixed-fixture test. T1106. | None. |

**Untouched crates** (verified by grep): `crates/backtest/` (no
`post_fill` calls), `crates/strategy/`, `crates/risk/`,
`crates/exec/` (the `paper.rs` call site passes `&fill` whose
`fill.symbol` is the source of truth — call signature unchanged),
`crates/cost/`, `crates/data/`, `crates/agent/runtime.rs` (the agent
boot path runs `Ledger::open` which auto-applies migrations, so the
006 INSERTs land at boot without an explicit Rust-side call), `crates/ui/`.

### Migration shape — exact SQL

`crates/audit/migrations/006_per_symbol_position_accounts.sql`:

```sql
-- T1101 — Per-symbol position accounts (per-symbol-position-accounts R1).
--
-- Adds one `assets:position:<SYMBOL>` row to the `accounts` table per
-- pair-symbol in the agent's universe (config/agent.toml [funding].universe).
-- Pre-migration the chart of accounts had a single `assets:position:BTC`
-- row; post-T1102 every fill targets the per-pair account-id, and that row
-- must exist before the first fill writes to it (FK from
-- journal_entries.account_id → accounts.id, see 001_chart_of_accounts.sql:23).
--
-- Purely additive (R3): legacy rows on `assets:position:BTC` stay; readers
-- handle legacy via description-parse (Q4). No money moves (R6).
-- Anchor regression: 11/11 byte-identical (R5, Q7) — body cells do not
-- render account-ids; verified by grep.

INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:BTCUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:ETHUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:BNBUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:SOLUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:XRPUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:ADAUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:DOGEUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:AVAXUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:DOTUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:LINKUSDT', 'asset', 'USDT');
```

Source of truth for the universe: `config/agent.toml:62-65`
`[funding].universe` (10 symbols). The migration enumerates the union
of every symbol any active strategy may trade. Adding a new symbol to
the universe in the future requires a follow-up migration (e.g.
`007_universe_extension.sql`) — this is intentional friction so a
strategy hot-load cannot silently introduce an FK-failing symbol.

`kind = 'asset'`, `currency = 'USDT'` matches the row shape of the
existing `assets:position:BTC` row at `bootstrap.rs:19`. The currency
is the **quote** asset (every Binance pair in the universe is `*USDT`);
this is consistent with the chart-of-accounts pattern where
`assets:cash:USDT` carries `currency = 'USDT'`.

### Reader compatibility table

For every combination of `journal_entries.account_id` × ledger shape:

| Ledger contains | Reader does | Result | Cross-check (Q4) |
|---|---|---|---|
| Only legacy `assets:position:BTC` (any symbol) | `extract_symbol_from_description(desc)` | Correct symbol from description | account-id == `"assets:position:BTC"` (legacy form) → cross-check PASSES (legacy whitelist) |
| Only post-T1102 `assets:position:<SYMBOL>` rows | `extract_symbol_from_description(desc)` | Correct symbol | account-id == `format!("assets:position:{}", parsed_symbol)` → cross-check PASSES |
| Mixed (some legacy BTC + some per-pair) | `extract_symbol_from_description(desc)` per row | Each row gets correct symbol via description | Each row's cross-check passes against its respective whitelist (legacy `BTC` OR per-pair) |
| Future renderer/writer bug: `assets:position:UNKNOWN` | `extract_symbol_from_description(desc)` | Correct symbol from description | Cross-check FAILS → `tracing::warn!("account_id={}, parsed_symbol={}, mismatch", ...)` + fall through to description-parsed symbol |

In all cases the reader returns the correct symbol; legacy rows are
not rewritten; future bugs are observable via `warn!` rather than
silent. R10 (determinism) holds: two reads of the same ledger return
byte-identical `Vec<OpenPosition>`.

### Test strategy (per V-item)

| V-item | Test file | Fixture | Asserts |
|---|---|---|---|
| V1 — `post_fill` writes per-symbol account | `crates/audit/tests/per_symbol_post_fill.rs::t1105_v1_post_fill_writes_per_symbol_account` | Empty audit DB; run all 6 migrations; post one ETHUSDT Buy + one BTCUSDT Buy + one SOLUSDT Buy via `post_fill`. | `SELECT account_id, COUNT(*) FROM journal_entries GROUP BY account_id` returns exactly 3 rows (ETHUSDT, BTCUSDT, SOLUSDT) on the position side; zero rows reference the legacy `assets:position:BTC` account. |
| V2 — Pre-migration legacy rows still readable | `crates/audit/tests/per_symbol_post_fill.rs::t1105_v2_legacy_row_readable_after_migration` | Hand-craft an audit DB pre-006: `INSERT INTO accounts ('assets:position:BTC', ...)` (from migration 001), `INSERT INTO journal_entries (account_id='assets:position:BTC', description='Buy 1.0 ETHUSDT @ 2000', ...)`. Then run migration 006 on top. | `SELECT account_id FROM journal_entries WHERE id = ?` still returns `assets:position:BTC` (unchanged). `audit::verify_balance(transaction_id)` returns `Ok(())`. `pnl_by_symbol` correctly buckets the row under `Symbol::new("ETHUSDT")` (via description-parse). |
| V3 — `open_positions_at` correct on mixed ledger | `crates/reports/tests/open_positions_mixed_ledger.rs::t1106_v3_mixed_ledger_correct_open_positions` | Extended `build_ledger_with_open_positions_7d.rs` (T1104): pre-pend 1 BTCUSDT Buy + 1 ETHUSDT Buy via direct SQL INSERT to the legacy `assets:position:BTC` account; append 1 SOLUSDT Buy via `post_fill` (per-pair). | `open_positions_at(period_end)` returns `Vec` of length 3, sorted `(BTCUSDT, ETHUSDT, SOLUSDT)`, each with correct `(qty, avg_cost_basis, strategy_id)`. |
| V4 — Anchor regression 11/11 PASS | `bash scripts/verify_anchors.sh` (no test code) | All 11 anchored reports unchanged. | Output line `ANCHORS PASS  (11 / 11)`. No diff against `spec/anchors.toml`. |
| V5 — Reconciliation invariant | `crates/audit/tests/per_symbol_post_fill.rs::t1105_v5_balance_invariant_pre_and_post_migration` | Same fixture as V3. | For every transaction id in the fixture, `audit::verify_balance(txn_id) == Ok(())`. `Σ debits == Σ credits` per transaction holds across the migration boundary. |
| V6 — operator-success-reports + live-cockpit-unified invariants | T802 / T805 / T806 / T809 / T810 existing tests + T901 / T903a-d / T905 / T906–T908 / T910 / T911 / T912 existing tests, all run unchanged. | Existing fixtures. | All green. T1107 (anchor sweep) acts as the meta-gate for the 11 anchors; the T-task tests run as part of `cargo test --workspace`. |
| V7 — Determinism | `crates/reports/tests/open_positions_mixed_ledger.rs::t1106_v7_two_reads_byte_identical` | Same fixture as V3. | Two consecutive `open_positions_at(...)` calls return `Vec<OpenPosition>` slices that compare equal via `assert_eq!`. |
| V8 — Universe coverage | `crates/audit/tests/per_symbol_post_fill.rs::t1105_v8_universe_coverage` | Empty audit DB; run all 6 migrations. | For every symbol in `config/agent.toml [funding].universe`, `SELECT 1 FROM accounts WHERE id = 'assets:position:<SYMBOL>'` returns one row. Loads the universe via the existing `Config::default()` reader. |

### Risks & mitigations

1. **Universe drift over time.** A new symbol added to the active
   strategy universe (e.g. a new `config/strategies/foo.toml` with
   `symbol = "BCHUSDT"`) without an accompanying migration `007`
   would fail at first fill on the FK reference to `accounts.id`.
   *Mitigation:* V8 universe-coverage test loads
   `config/agent.toml [funding].universe` and asserts every symbol has
   a chart-of-accounts row. T1107's anchor sweep runs the test under
   `cargo test --workspace`. A future strategy-add PR that introduces
   a symbol outside the universe will fail the test gate at PR time.

2. **Reader divergence between description-parse and account-id.** The
   defensive cross-check (Q4) would silently divergence-warn rather
   than fail. If the writer regresses (e.g. someone reverts T1102) the
   description would still carry the correct symbol but the account-id
   would be wrong; readers keep working but the structural attribution
   is broken. *Mitigation:* T1106 V3 fixture exercises the cross-check
   explicitly. A future tester run sees `tracing::warn!` lines. Long
   term: a metrics counter on the warn-emit path would give continuous
   observability — out of scope here.

3. **Migration ordering vs deprecated `seed_universe_accounts`.** If
   T1103 deletes `seed_universe_accounts` in a separate wave but the
   delete lands before the migration (e.g. someone reverts T1101 but
   keeps T1103), no boot-time defensive seed exists at all.
   *Mitigation:* T1103 tasked AFTER T1101 in the parallelism map; the
   migration is the source of truth. The `#[deprecated]` attribute in
   T1103 lands BEFORE deletion in any future wave, so the deletion is
   gated by "no callers anywhere" which is already true today.

4. **Reconciliation invariant on the migration boundary.** The
   migration adds account rows; `accounts` is referenced by `FK` from
   `journal_entries.account_id`. If a row in `accounts` is somehow
   removed mid-flight (it won't be — the migration only INSERTs), the
   FK would break. *Mitigation:* the migration uses `INSERT OR IGNORE`
   only — no DELETE, no UPDATE. R6 holds by inspection. V5 explicitly
   re-runs `verify_balance` per transaction across the migration
   boundary on the V3 fixture.

5. **Anchor drift if Q7 was wrong.** If a committed report body
   somewhere DOES carry an `account_id` string and the grep missed it
   (e.g. an embedded fence with non-printable characters), migration
   `006` would shift the byte-hash. *Mitigation:* T1107 runs
   `verify-anchors` immediately after T1101+T1102+T1104 land. A single
   anchor FAIL routes back to architect for re-investigation. The
   re-grep in § Independent verification confirms zero hits with high
   confidence (two greps, two corpora, zero hits). Risk: low.

6. **Fixture extension introduces a load-bearing direct-SQL INSERT
   path.** The T1104 fixture extension uses raw SQL to bypass the
   updated `post_fill` writer (so we get legacy-shape rows for V3).
   That direct-SQL path could rot — if migration `001`'s
   `journal_entries` schema changes the INSERT shape changes too.
   *Mitigation:* document the fixture's raw-SQL `INSERT` as
   "deliberately matches the pre-006 `assets:position:BTC` shape".
   Schema changes to `journal_entries` are vanishingly rare (the
   table has been stable since `001_chart_of_accounts.sql`). If it
   ever happens, V3 would fail loudly at compile time (sqlx) or at
   test run, not silently.

### Operator-success-reports invariants that must hold

(Mirrors R8 in the analyst brief; unchanged.)
- **T802** — `post_fill(strategy_id: Option<&str>)` signature stays
  byte-identical. The per-symbol switch is line-edit-only inside the
  function body.
- **T805** — feed-reconnect events via
  `audit::journal::feed_reconnect`. Independent code path.
- **T806** — agent uptime intervals (`agent_uptime` table).
  Independent.
- **T809** — kill-switch dual-write (`audit_memo` + `strategy_events`).
  Independent.
- **T810** — `--features in_process_cron`. Independent.
- The 2 v1+ anchors (`report-sample-7d` `ab06dbcb…`, `report-sample-90d`
  `2ef403f1…`) stay byte-identical (R5 + Q7).

### live-cockpit-unified invariants that must hold

(Mirrors R9 in the analyst brief; unchanged.)
- **T901** — Prometheus toggle. Independent.
- **T903a** — `paper::on_fill` announce-second invariant: `bus.publish_fill`
  fires AFTER `audit::post_fill` returns. Per-symbol writer change is
  line-edit-only inside `post_fill`'s body — return path unchanged. Holds.
- **T903b** — bar/tick taps. Independent.
- **T903c** — reconciler `PnlSnapshot` reads `audit::query::*`. Public
  surface unchanged (Q4 keeps existing readers' signatures byte-identical).
  Holds.
- **T905** — kill-switch / mode forwarder. Independent.
- **T906–T908** — UI panels. Read-only over `audit::query`. Holds.
- **T910** — uptime smoke. Independent.
- **T911** — kill-button trips kill-switch. Independent.
- **T912** — Prometheus toggle test. Independent.

### Parallelism summary (forward-pointer to spec/tasks)

Wave 1 (after migration shape locked):
- T1101 (migration `006` SQL) → blocks T1102 (writer can't write to
  per-pair without the row existing) and T1104 (fixture needs
  migration's seeded rows).

Wave 2 (parallel after T1101):
- T1102 (writer switch) — sole edit to `journal.rs:82,135`.
- T1103 (`#[deprecated]` on `seed_universe_accounts`) — sole edit to
  `bootstrap.rs:65`.
- T1104 (fixture extension) — sole edit to
  `build_ledger_with_open_positions_7d.rs`.

Wave 3 (parallel after T1102 + T1104):
- T1105 (V1 + V2 + V5 + V8 tests in `crates/audit/tests/`).
- T1106 (V3 + V7 tests in `crates/reports/tests/`).

Wave 4 (sequential, after T1105 + T1106):
- T1107 (anchor regression sweep) — read-only.

Tester-final:
- T_FINAL_PER_SYMBOL — gated by all of the above + `verify-anchors`.

## Implementation
_developer fills this — left blank intentionally_

## Verification — links
_tester fills this — left blank intentionally_

## UI
_no new UI surface. The cockpit's positions widget reads
`audit::query::open_positions_at` (real-mtm public API); R3 leaves
that signature unchanged. No ui-designer involvement._

## Changelog

- 2026-05-03 (tester): final gate PASS. T_FINAL_PER_SYMBOL ticked
  in `spec/tasks/per-symbol-position-accounts.md`. All eight V-items
  VERIFIED: V1 (per-symbol writer) via `t1105_v1_post_fill_writes_per_symbol_account`;
  V2 (legacy readability) via `t1105_v2_legacy_row_readable_after_migration`;
  V3 (mixed-ledger reader) via `t1106_v3_mixed_ledger_correct_open_positions`;
  V4 (anchor regression) via `verify_anchors.sh` 11/11 PASS; V5
  (reconciliation invariant) via
  `t1105_v5_balance_invariant_pre_and_post_migration`; V6
  (operator-success-reports + live-cockpit-unified invariants) via
  `cargo test --workspace --all-targets` zero failures; V7
  (determinism) via `t1106_v7_two_reads_byte_identical`; V8 (universe
  coverage) via `t1105_v8_universe_coverage`. 16 operator-success-reports
  + live-cockpit-unified invariants all VERIFIED. Static analysis
  clean (fmt / clippy / build / build with `--features in_process_cron`).
  Anchors held byte-identical pre- and post-tests. Status flipped
  in-progress → shipped on both the feature and task files. Test
  report at `spec/reports/test-2026-05-03-0803-per-symbol-position-accounts-final.md`.
  `VERDICT → PASS`.
- 2026-05-03 (architect): Design section landed. Resolves Q1–Q8.
  **Q1** purely additive migration `006_per_symbol_position_accounts.sql`
  (10 `INSERT OR IGNORE` lines for the universe at
  `config/agent.toml:62-65`). **Q2** account-id format
  `assets:position:<SYMBOL>` (full pair). **Q3** NO backfill — legacy
  rows untouched. **Q4** description-parse stays primary in
  `open_positions_at`; account-id-suffix is a defensive cross-check
  only. **Q5** `extract_symbol_from_description` indefinitely
  retained, NOT marked `#[deprecated]` (it's still primary path);
  doc-comment notes new code SHOULD use `open_positions_at`. **Q6**
  EXTEND `build_ledger_with_open_positions_7d.rs` (override of
  analyst's b — the existing fixture is non-anchored, so extension
  is anchor-safe). **Q7** anchor risk zero by independent re-grep
  (zero hits for `assets:position` in committed report bodies; zero
  `post_fill` calls from `crates/backtest/`); 11 / 11 byte-identical.
  **Q8 (corrected)** `seed_universe_accounts` shape MISMATCH —
  takes base assets (`"BTC"`) not pair symbols (`"BTCUSDT"`), so it
  CANNOT be reused; mark `#[deprecated]` in T1103 with a deletion
  follow-up. The migration is the canonical seed (no Rust-side
  defensive seed needed; `Ledger::open` runs migrations on every
  binary boot). Tasks T1101–T1107 + T_FINAL_PER_SYMBOL filed at
  [spec/tasks/per-symbol-position-accounts.md](../tasks/per-symbol-position-accounts.md).
  Status flips draft → in-progress; owner architect → developer
  on T1101 spawn.
- 2026-05-02 (analyst): initial draft. Promotes "R10 follow-up:
  per-symbol-position-accounts" from the implicit Queue (deferral
  noted in
  [`real-mtm-unrealized-pnl.md` Design § Q3 / R10 verdict](real-mtm-unrealized-pnl.md))
  into Active. Plumbing-only feature: chart-of-accounts migration +
  `post_fill` writer change + (optional) `open_positions_at` reader
  optimization. 11 R-items, 8 V-items, 8 open questions for
  architect. Anchor risk: 9 backtest + 2 v1+ — preferred outcome
  byte-identical (R5); architect must confirm the migration stays
  purely additive at the chart-of-accounts level (Q3, Q7) so report
  bodies stay byte-identical. HANDOFF → architect.
