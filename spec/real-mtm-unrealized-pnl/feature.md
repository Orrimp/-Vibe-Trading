---
slug: real-mtm-unrealized-pnl
status: shipped
owner: tester
updated: 2026-05-02
version: 1.3.0
---

# Real mark-to-market unrealized P&L

## Why

[`operator-success-reports`](operator-success-reports.md) shipped at v1+
with a documented degeneracy: the orchestrator at
`crates/reports/src/lib.rs` lines 135–150 hardcodes
`let unrealized: Decimal = Decimal::ZERO;` because the `audit::query`
surface does not expose a typed open-positions slice. The accompanying
comment is honest about the gap — "open-position projection ships in
v2+" — and the v1+ feature spec encodes the same scoping note in the
Mark-to-market source (R11.1, R4.4) subsection.

The operational consequence is that **every operator-facing P&L number
that flows through `headline_return = realized + unrealized` is
under-reported by the value of currently open exposure**:

- **R2 headline** (`strategy_return_usdt`, `strategy_return_pct`) — the
  flagship "how did I do this week" number ignores any position the
  agent is still holding at `period_end`. If alpha is sitting on an
  open BTC long that has appreciated 8% intra-window, the headline
  reports zero of it.
- **R4 risk metrics** — `max_drawdown`, `sharpe`, `sortino`, `calmar`
  are all derived from the equity curve sampled in
  `sample_equity_curve(...)` (lib.rs:194), which currently uses
  `cash.amount()` flat across the window with no position contribution.
  Drawdowns inside open positions are invisible.
- **R11 reconciliation** — identity #1
  (`headline_return == realized + unrealized`) is satisfied trivially
  because both sides use 0 for unrealized; it provides no real cross-
  check until unrealized is sourced from a different code path than
  the headline aggregator.
- **CSV artifact `equity-<window>.csv`** — the
  `unrealized_pnl_usdt` column (already in the schema; see
  operator-success-reports Design "CSV artifact column schemas") is
  always emitted as `0`.

The operator-facing question this blocks is the obvious one: **"how
much of my equity is currently exposed to market moves?"** Today the
report cannot answer it.

The MarkSource trait (`crates/reports/src/marks.rs`, T812) and the
`assets:position:*` ledger accounts (journal.rs:82, 135) are already
shipped — the only missing piece is the typed reader that projects
fills into open-position rows + the orchestrator hookup.

## Requirements (R-items)

### R1 — Typed open-positions slice in `audit::query`

A new reader exposes open positions as of a point in time. Signature
shape is an architect call (Q1) but semantics are fixed:

- **Input:** `&Ledger`, `Timestamp` (the as-of moment, normally
  `period_end`).
- **Output:** `Result<Vec<OpenPosition>, LedgerError>`, sorted
  deterministically (by `symbol` ascending, then `strategy_id`
  ascending, `None` last) for byte-identical re-reads (R6).
- **Semantics:** sum (Buy_qty − Sell_qty) per `(symbol, strategy_id)`
  group from `journal_transactions` parsed via the existing
  `description` regex `"<side> <qty> <symbol> @ <price>"` (already
  used by `pnl_by_symbol` and `recent_fills`). Group keys with
  net qty == 0 are filtered out. Groups with net qty > 0 are
  emitted as a long open position; net qty < 0 is treated as a
  closed-out position misread (raise `LedgerError::Database` —
  v1+ is long-only per Q8).
- **Cost basis** is weighted average entry price across all Buy
  fills in the group up to `as_of`, less the proportional cost
  basis released on each Sell (FIFO or weighted; architect picks
  in Q7, but the recommended default is **weighted average across
  remaining qty** — same simplification `journal::post_fill`
  already makes at the realized-P&L computation in
  `journal.rs:111-118`).

### R2 — Orchestrator integration with the existing `MarkSource` trait

`crates/reports/src/lib.rs::generate(...)` replaces the hardcoded
`Decimal::ZERO` with:

```
unrealized = Σ over open_positions of:
    (qty * marks.close_at(symbol, period_end)? - cost_basis * qty)
```

- The mark lookup reuses the existing `MarkSource::close_at` already
  hit for the BTC baseline at lib.rs:148–150, so no new trait method
  is needed.
- Errors from `close_at` (out-of-range, unknown symbol) MUST NOT
  panic; the orchestrator's behavior on a mark-source miss is an
  architect decision (Q6) — recommended: surface a typed
  `MarkUnavailable { symbol, ts }` and propagate as a soft warning
  in front-matter `warnings:` while computing `unrealized` over the
  positions that DID resolve. **Do not** silently fall back to 0
  for the missing position — that re-introduces the gap this
  feature exists to close.

### R3 — Backwards compat: empty-positions ledgers stay byte-identical

For any ledger where `open_positions_at(period_end)` returns an empty
vec — e.g. backtest ledgers that close every position by EOD, or the
two existing fixtures `build_ledger_7d.rs` / `build_ledger_90d.rs`
which are fully symmetric (every Buy has a matching Sell within the
window; verified by inspection: 7d has 6 buy/sell pairs, 90d has 12
buy/sell pairs across 4 strategies) — `generate(...)` MUST produce
`unrealized = Decimal::ZERO`, byte-identical to today's body output.
This is the precondition for R4.

### R4 — Anchor regression — preferred outcome: byte-identical

The two v1+ anchors in `spec/anchors.toml` (`report-sample-7d`,
`report-sample-90d`) MUST EITHER:

- **(preferred) Stay byte-identical.** Justified above (R3): the
  existing fixtures have zero open positions at `period_end`, so
  `Σ (qty × mark − cost_basis) == 0`, and every body cell that
  derives from `unrealized` (R2 headline, R4 risk metrics, R11
  reconciliation row, equity-curve CSV column) is unchanged.
- **OR be re-locked with architect approval.** Precedent: v1.5a
  T717 re-locked anchors after a benign rendering change. If a
  fixture is extended (Q5) to include an open position, the
  re-lock follows the v1.5a procedure: tester captures the new
  SHA, architect approves, `spec/anchors.toml` row updates with
  a comment citing this feature.

The 9 v0/v0.5/v1/v1.5a anchors are unaffected (this feature touches
no strategy, no exec, no backtest code path).

### R5 — Reconciliation invariant `Σ debits == Σ credits` MUST still hold

Open-position tracking is **derived state**: it reads
`journal_transactions` and `journal_entries`, never writes. No new
journal lines, no new accounts, no schema change to the
double-entry tables. The `audit::verify_balance(...)` and
`global_debit_credit_sum(...)` invariants pass through unchanged.

### R6 — Determinism

Two reads of `open_positions_at(ledger, ts)` against the same audit
DB MUST return byte-identical `Vec<OpenPosition>`. This means:

- Sort order locked (per R1).
- No `HashMap` iteration on the hot path (use `BTreeMap`,
  matching the precedent at `query.rs::pnl_by_symbol` line 480).
- No `f64` (per AGENT.md determinism non-negotiables).
- Description-parse failures handled identically across runs
  (raise the same typed error; do not stash partial state).

### R7 — Performance

`open_positions_at` MUST complete in O(open positions × fills per
position), not O(all fills in ledger). Concretely: a SQL query
that filters by description prefix or by `account_id LIKE
'assets:position:%'` and aggregates server-side, NOT a Rust-side
fold over `recent_fills(usize::MAX, ...)`. Acceptance: V8 perf
gate (<100ms on 100 fills + 5 open positions).

If the existing indexes (`idx_entries_account`, `idx_entries_ts`)
are insufficient, an additional index is acceptable but ships as
a new migration `006_*.sql` — architect call (Q3).

### R8 — Existing invariants must continue to hold

- **T802** — `post_fill(strategy_id)` writes the optional
  `strategy_id` column. The new reader MUST consume this column
  (it is the `strategy_id` field on `OpenPosition`, propagated
  for per-strategy unrealized-P&L attribution in a future wave).
- **T805 / T806 / T809** — feed-reconnect, mean-reversion-stop,
  pair-short-observation event invariants are independent of this
  feature; the test harness must show them green on the new
  reader's path.

### R9 — R6 placeholder string MUST NOT change

`crates/reports/src/render/memory_highlights.rs::PLACEHOLDER` is
locked into the two v1+ anchors (precedent T811 forward-compat
note). This feature does not touch `memory_highlights.rs`. Reflection
memory is a separate queued backlog item.

### R10 — Hardcoded BTC-only account name in `journal.rs`

Out-of-band finding during reads: `journal::post_fill` writes
`"assets:position:BTC"` for **every** symbol (journal.rs:82,135).
This is benign for v0/v0.5/v1/v1.5a backtests (fees + realized P&L
unaffected) and for any single-symbol fixture, but it means the
account ledger does NOT actually carry per-symbol position
balances today — only a single mixed-symbol BTC-named bucket. The
new reader MUST therefore parse the **transaction description**
(not the account_id) for the symbol, exactly as `pnl_by_symbol`
already does (`query.rs::extract_symbol_from_description` line
512). Whether to fix the account-id naming is **out of scope for
this feature** but should be flagged to the architect (Q3 sub-bullet)
because a future per-symbol account refactor would let the
position reader read account balances directly — a cleaner
implementation. For now: parse the description.

## Verification (V-items)

- **V1 — Reader correctness.** Fixture ledger with 3 closed +
  2 open positions across 2 strategies returns exactly 2 rows
  with correct (qty, weighted-avg cost_basis, opened_at,
  strategy_id) tuples. Asserted byte-by-byte.
- **V2 — Orchestrator computes unrealized correctly.** Hand-
  computed expected value for a fixture with 2 open positions
  + frozen `MarkSource` matches `generate(...)`'s emitted
  `unrealized_pnl_usdt` in the equity-curve CSV row at
  `period_end` and in the R11 reconciliation table.
- **V3 — Empty-positions backwards compat.** `generate(...)`
  on `build_ledger_7d` and `build_ledger_90d` (both fully
  symmetric) emits `unrealized == Decimal::ZERO` and the
  rendered body is byte-identical to today's output.
- **V4 — Reconciliation invariant.** `audit::verify_balance`
  on every transaction in the post-feature fixture passes
  (i.e. no debit/credit imbalance introduced).
- **V5 — Anchor regression.** `bash scripts/verify_anchors.sh`
  → `ANCHORS PASS  (11 / 11)`. Architect's call (Q4) whether
  re-lock to (13 / 13) is required if a fixture is extended in
  Q5.
- **V6 — Existing event invariants.** T805 (feed-reconnect),
  T806 (mean-reversion-stop), T809 (pair-short-observation)
  unit + integration tests all green.
- **V7 — Determinism.** `open_positions_at(&ledger, ts)`
  called twice on the same opened ledger returns
  `Vec<OpenPosition>` slices that compare equal byte-for-byte
  via `assert_eq!`.
- **V8 — Perf smoke.** A fixture with 100 fills (50 buy/sell
  pairs) + 5 open positions runs `open_positions_at(...)` in
  < 100ms wall-clock on the developer's box, asserted in
  `tests/perf_smoke.rs` (matches the v1+ R13 precedent).

## Backtest scenarios

_n/a — plumbing on existing fixtures; `report-sample-7d` and
`report-sample-90d` carry the regression load. No new scenario
unless Q5 extends a fixture, in which case the architect re-locks
the affected anchor — no new scenario row added._

## Open questions for architect

- **Q1 — Reader signature shape.** `open_positions_at(ledger,
  ts) -> Vec<OpenPosition>` (snapshot-style, matches `pnl_by_symbol`)
  vs `cash_balance(...) -> Money<USDT>`-style (single value,
  doesn't fit). Recommended: snapshot vec. Confirm.
- **Q2 — Typed `OpenPosition` struct fields.** Recommended:
  ```rust
  pub struct OpenPosition {
      pub symbol:           Symbol,
      pub qty:              Decimal,                  // > 0 (long-only, Q8)
      pub avg_cost_basis:   Decimal,                  // weighted avg entry price
      pub opened_at:        Timestamp,                // ts of first un-closed Buy
      pub strategy_id:      Option<StrategyId>,       // T802 column; None for pre-T802 rows
  }
  ```
  Architect picks final shape. Note: no `unrealized_pnl` field
  on the struct itself — that's a function of `MarkSource` and
  belongs in the orchestrator, not the reader (the reader has
  no business reaching into parquet).
- **Q3 — Index strategy.** Existing indexes:
  `idx_entries_account` (`journal_entries.account_id`),
  `idx_entries_ts` (`journal_entries.ts`),
  `idx_entries_txn` (`transaction_id`),
  `journal_transactions_sid_idx` (`strategy_id, ts` from migration
  004). Question: is a `journal_transactions` index on
  `(description)` or on a synthesized `symbol` column needed?
  R10 above flags that the symbol is parsed from the description,
  so a description-prefix LIKE may not be index-friendly. Two
  options: (a) accept a full-table-scan over
  `journal_transactions` (small enough at v1+ scale, already done
  by `pnl_by_symbol`), (b) add migration `006_*.sql` extracting
  symbol into a stored column. Architect picks; recommended (a)
  for now, defer (b) until perf budget bites.
- **Q4 — Anchor re-lock decision.** Empirical reading of
  `build_ledger_7d.rs` lines 189–204 and `build_ledger_90d.rs`
  lines 220–247 confirms ALL existing fixture fills are
  symmetric (every Buy has a matching Sell of the same `qty`
  within the window). Therefore `open_positions_at(period_end)
  == []` for both fixtures, `unrealized == 0`, body bytes
  unchanged, anchors stay byte-identical. **Architect to
  confirm this read** before tester locks PASS on (11 / 11)
  rather than (13 / 13). If architect chooses Q5 = "extend
  existing fixture" the anchors WILL drift and re-lock per
  v1.5a T717 precedent.
- **Q5 — Fixture choice for the new tests (V1, V2).** Two
  options:
  - **(a)** Reuse `build_ledger_7d.rs` / `build_ledger_90d.rs`
    by adding an open position (e.g. extra Buy at day 6 hour
    20 with no matching Sell). Pro: fewer fixtures to maintain.
    Con: shifts `unrealized` from 0 → non-zero, anchors re-lock,
    Q4 decision becomes mandatory re-lock.
  - **(b)** Add `build_ledger_with_open_positions.rs` —
    standalone fixture used only by V1 + V2 + V8. Pro: existing
    anchors stay byte-identical (Q4 = preferred outcome).
    Con: one more fixture file.
  Recommended: **(b)**. Keep the v1+ regression gate clean.
- **Q6 — Mark-source miss behavior.** What does
  `mark_source.close_at(symbol, period_end)` do when a fixture
  position references a symbol whose parquet root doesn't
  cover `period_end`? Today the BTC baseline at lib.rs:149–150
  uses `.ok()` to swallow errors. Same pattern, or surface a
  typed warning into front-matter? Recommended: surface as
  `warnings: ["mark unavailable: <symbol> at <ts>"]` in
  front-matter; compute `unrealized` over only the resolved
  positions; do NOT silently 0-out the missing one.
- **Q7 — Cost-basis semantics.** Weighted-average across all
  Buy fills in the open lot vs FIFO. Industry standard is
  weighted-average; the existing realized-P&L code at
  `journal.rs:111-118` already uses fill-price-as-cost-basis
  (a degenerate single-Buy case). Recommended: weighted
  average. Confirm — and if FIFO is chosen, V2's hand-computed
  expected value changes.
- **Q8 — Short positions.** v1+ is long-only. The pair short
  observation in T715/`pair_short_observation` is logged but
  is NOT a real short fill — it does not write to
  `assets:position:*`. v1.5a's pairs strategy trades the long
  leg as a real fill and the short leg as a memo-only
  observation. Therefore at v1+ scope: **open positions are
  long-only, qty > 0, and any net-negative qty is a malformed
  ledger**. Architect confirms or expands scope (the latter
  pulls in real short fills, a much bigger feature).

## Design

Architect resolutions for Q1–Q8 + R10 follow. Operator-aligned defaults
applied unless principled override; deviations called out explicitly.

### Q1 — Reader signature shape

**Decision:** new `pub async fn open_positions_at(ledger: &Ledger,
ts: Timestamp) -> Result<Vec<OpenPosition>, LedgerError>` in
`crates/audit/src/query.rs`. Snapshot-vec shape parallel to
`pnl_by_symbol` and `pnl_by_strategy`. Projects every fill in
`journal_transactions` whose `ts <= ts_str` into `(symbol,
strategy_id)` groups, sums `Buy_qty − Sell_qty`, filters net-zero
groups, returns surviving long-only positions sorted by `(symbol
ASC, strategy_id ASC, None last)` for byte-identical re-reads (R6).

**Rationale:** matches the analyst's recommended default. A scalar
`Money<C>`-style return doesn't fit (N rows). A streaming surface
is overkill at v1+ scale.

**Rejected:** borrow-style `&[OpenPosition]` (needs Ledger-owned
cache, no perf win); push the reader into `crates/reports/`
(would force every other consumer — cockpit positions widget,
future v2+ risk readers — to depend on `reports`).

### Q2 — `OpenPosition` struct fields and location

**Decision:** struct lives in `trading_core` (cross-crate
visibility). Final shape:

```rust
// crates/core/src/position.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPosition {
    pub symbol:         Symbol,
    pub qty:            Decimal,            // > 0 (long-only, Q8)
    pub avg_cost_basis: Money<Usdt>,        // PER-UNIT price, not notional
    pub opened_at:      Timestamp,          // ts of first un-closed Buy
    pub strategy_id:    Option<StrategyId>, // T802 column; None pre-T802
}
```

`avg_cost_basis` is the **per-unit** cost (USDT per unit of
`symbol`), so the orchestrator computes notional contribution as
`qty * avg_cost_basis` at mark time. Typed as `Money<Usdt>` rather
than raw `Decimal` to honour the no-bare-Decimal-money rule.

**Rationale:** `OpenPosition` will eventually be read by
`audit::query` (producer), `crates/reports/` (orchestrator), and
`crates/ui/` (positions widget — post-T903c the cockpit wants a
typed slice instead of synthesising one). Cross-crate types live
in `trading_core`.

**Rejected:** inline in `audit::query` (forces cockpit to depend
on `audit` for the type or duplicate it); add `unrealized_pnl`
field on the struct (mark-source-agnostic boundary belongs in
the orchestrator).

### Q3 — Index strategy + R10 (BTC hardcode)

**R10 verdict — DEFERRED.** Confirmed at
`crates/audit/src/journal.rs:82,135`: every Buy/Sell writes to the
literal `"assets:position:BTC"` regardless of `fill.symbol`. The
reader does NOT touch the account id — it parses the symbol from
`journal_transactions.description` (format `"<side> <qty> <symbol>
@ <price>"`) via the existing `extract_symbol_from_description`
helper at `query.rs:512`, which `pnl_by_symbol` and `recent_fills`
already rely on. The description IS symbol-faithful (verified
against `build_ledger_90d.rs` which writes BTCUSDT/ETHUSDT/SOLUSDT
descriptions despite the BTC-only account id). Fixing `post_fill`
is a chart-of-accounts migration with potential 9-anchor sensitivity
and is strictly larger than this feature's plumbing scope. Filed as
follow-up brief `spec/per-symbol-position-accounts/feature.md`
(analyst-owned; not authored here).

**Index decision:** **NO new SQL index** for v1+. The reader runs
`SELECT id, ts, description, strategy_id FROM journal_transactions
WHERE (description LIKE 'buy %' OR description LIKE 'sell %') AND
ts <= ?` then folds in Rust — same pattern `recent_fills`
(`query.rs:138-148`) uses against the non-indexed `description`
column. At v1+ scale the table is ≪ 100k rows; full-table scan
finishes well under the 100 ms V8 budget. **If V8 fails**, ship
follow-up migration `006_open_positions_index.sql` adding
`CREATE INDEX idx_journal_transactions_description_prefix ON
journal_transactions(substr(description, 1, 4), ts)`. Tester owns
the V8 measurement; if green the migration never lands.

**Rejected:** denormalized `symbol` column on
`journal_transactions` (bundle with R10 follow-up); composite
`(description, ts)` index (low selectivity on free-text).

### Q4 — Anchor regression: byte-identical (no re-lock)

**Decision:** STAY byte-identical. All 11 anchors in
`spec/anchors.toml` unchanged.

**Rationale (architect-confirmed by direct fixture read):**
- `build_ledger_7d.rs:188-201` — 12 fills in 6 perfectly symmetric
  (Buy, Sell) pairs.
- `build_ledger_90d.rs:218-247` — 24 fills in 12 symmetric pairs
  across 4 strategies.
- Both fixtures: net qty == 0 per `(symbol, strategy_id)` at
  `period_end` → `open_positions_at(period_end) = vec![]` →
  `Σ unrealized = 0`. Every body cell that depends on `unrealized`
  emits the SAME bytes as today's `Decimal::ZERO`-hardcoded path.
- The 9 v0/v0.5/v1/v1.5a anchors are independent (this feature
  touches no strategy/exec/backtest code path).

**Falsification gate (T_FINAL):** `bash
scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`. Any drift
routes `HANDOFF → architect` (most likely cause: render-module
serialization slip on `+0.00` vs `0.00`).

**Rejected:** re-lock the 2 v1+ anchors — only justified under
Q5 path (a); Q5 chooses (b) so re-lock is unnecessary.

### Q5 — Fixture choice: ADD a third (test-only, non-anchored)

**Decision:** add
`crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`
(seed `0xC0FFEE`, mirrors `build_ledger_7d` constants) for V1, V2,
V7, V8. The two anchored fixtures are NOT modified.

**Fixture content (developer-binding spec):**
- Same 6 (Buy, Sell) pairs as `build_ledger_7d`'s BTCUSDT +
  ETHUSDT plan, PLUS
- 2 EXTRA Buy fills at day 6 hour 20 with NO matching Sell:
  - `(strat_alpha, BTCUSDT, Side::Buy, qty=0.01, price=60_000)`
  - `(strat_beta, ETHUSDT, Side::Buy, qty=0.20, price=3_000)`
- Marks via `FrozenMarkSource::from_csv_str(...)` —
  `BTCUSDT @ period_end = 70_000`, `ETHUSDT @ period_end = 3_500`.
- V2 hand-computed expected: BTC `0.01 × (70_000 − 60_000) =
  +100.00`; ETH `0.20 × (3_500 − 3_000) = +100.00`; `Σ unrealized
  = +200.00 USDT`.

**Rejected:** extend `build_ledger_7d.rs` (drifts `report-sample-7d`
SHA → re-lock); inline open-position rows in `#[test]` bodies (V1 +
V2 + V8 share state — violates the fixture-builder pattern).

### Q6 — Mark-source miss: WARN + zero (architect override)

**Decision:** on `MarkError::OutOfRange` from
`MarkSource::close_at(symbol, period_end)` for an open position:
1. `tracing::warn!(symbol, ts, "mark unavailable for open
   position")`.
2. Contribute `Decimal::ZERO` for that position (NOT hard fail,
   NOT propagate).
3. Sum over all positions including the zero.
4. If ANY position fell back, append a Markdown footnote `*one
   or more open-position marks were unavailable at period_end;
   see logs*` to the R11.1 reconciliation row's `unrealized`
   cell. The footnote is deterministic (boolean of `mark_misses
   > 0`); byte-identical re-runs produce the same body.

**Override rationale:** the analyst's recommendation (front-matter
`warnings:` + skip the position) **conditionally changes the
arithmetic body** — same fixture, different `unrealized` depending
on parquet-root health, different SHA. That is a determinism
foot-gun. The chosen design keeps `unrealized` invariant under
mark-source health and surfaces the signal via logs + a body
footnote whose presence/absence depends only on the fixture's mark
coverage, not the parquet root's wall-clock state. Front-matter
`warnings:` is reserved for run-varying signals (load failures,
IO retries) per operator-success-reports Q7.

**V2 path:** the new fixture ships a `FrozenMarkSource` covering
both open symbols at `period_end` → no-warning branch. V6
explicitly exercises the miss path against a frozen source that
omits one symbol.

**Rejected:** hard fail (operator loses the report for one missing
roll-up); analyst's front-matter `warnings:` pick (determinism
foot-gun above).

### Q7 — Cost basis: weighted average with proportional release

**Decision:** weighted-average across all Buy fills in the open
lot, proportional release on each Sell. Reader maintains
`(running_qty, running_notional)` per `(symbol, strategy_id)`:

- Buy `qty_b @ price_b`:
  `running_notional += qty_b * price_b; running_qty += qty_b`.
- Sell `qty_s @ price_s` (long-only, `qty_s <= running_qty`):
  `released = (running_notional / running_qty) * qty_s;
  running_notional -= released; running_qty -= qty_s`.
- End-of-scan: if `running_qty > 0` emit `OpenPosition { qty:
  running_qty, avg_cost_basis: Money(running_notional /
  running_qty), … }`; `== 0` skip; `< 0` raise per Q8.

**Rationale:** industry default; aligns with the existing
realized-P&L calculation at `journal.rs:111-118`. Decimal-only;
the division happens once per emitted position (precision
bounded). V7 asserts byte-identical re-reads.

**Rejected:** FIFO (per-lot tracking; bigger reconciliation
surface; defer to v2+); last-price-as-cost-basis (always-zero on
open).

### Q8 — Long-only at v1+, short = malformed-ledger error

**Decision:** reader filters to `running_qty > 0`. On
`running_qty < 0` returns `LedgerError::Database(
"open_positions_at: net-negative qty for group ({symbol},
{strategy_id:?}) — short positions out of scope at v1+; check
ledger integrity")`.

**Rationale:** v1.5a pairs strategy logs the short leg as memo-only
`pair_short_observation` (no journal fill); v1+ has no real
shorts. Loud error surfaces unexpected Sells > Buys.

**Out of scope:** real shorts land in v2+ (perp executor / margin
adapter) — needs `Side::Short/Cover`, `liability:short:<asset>`
accounts, `OpenPosition.side: Side`. None ship here.

### R10 — `post_fill` BTC hardcode: explicitly DEFERRED

**Verdict:** out of scope. Follow-up brief (analyst-owned) at
`spec/per-symbol-position-accounts/feature.md` covers: (a) audit
the 9 anchors for account-rename sensitivity, (b) extend the chart
of accounts, (c) update `post_fill` to format
`"assets:position:{asset}"` from `fill.symbol`.

**This feature ships without R10** because the
description-parser path already handles per-symbol correctly —
verified against `pnl_by_symbol` (`query.rs:483-505`), which
produces correct per-symbol P&L on `build_ledger_90d` (4 symbols
all writing to the literal BTC account). `open_positions_at`
inherits the property.

### Crate map delta

- **`trading_core`** — additive: new `OpenPosition` struct (Q2),
  inline at `crates/core/src/position.rs` re-exported via
  `pub use position::OpenPosition` in `lib.rs`.
- **`audit`** — additive `pub async fn open_positions_at` reader
  in `crates/audit/src/query.rs`. NO migration in this feature
  (Q3 no-index; conditional follow-up
  `006_open_positions_index.sql` only if V8 perf gate fires).
- **`reports`** — orchestrator change in
  `crates/reports/src/lib.rs::generate(...)` lines 135–150. NEW
  fixture `tests/fixtures/build_ledger_with_open_positions_7d.rs`
  (Q5). NEW perf test `tests/perf_smoke_open_positions.rs` (V8).
  Existing anchored `tests/report_scenarios.rs` UNCHANGED.
- **No other crates touched.** No new external dep. Workspace
  edition 2021 unchanged.

### Public API additions

```rust
// trading_core
pub struct OpenPosition {
    pub symbol:         Symbol,
    pub qty:            Decimal,            // > 0
    pub avg_cost_basis: Money<Usdt>,        // per-unit
    pub opened_at:      Timestamp,
    pub strategy_id:    Option<StrategyId>,
}

// audit::query
pub async fn open_positions_at(
    ledger: &Ledger,
    ts: Timestamp,
) -> Result<Vec<OpenPosition>, LedgerError>;
```

`extract_symbol_from_description` (`query.rs:512`) stays private.

### Orchestrator integration — the exact diff

`crates/reports/src/lib.rs::generate(...)` today (lines 135–150):

```rust
// ── 3. Mark-to-market unrealized P&L ────────────────────────────────────
let unrealized: Decimal = Decimal::ZERO;
let btc_symbol = Symbol::new("BTCUSDT");
let btc_start = marks.close_at(&btc_symbol, period_start).ok();
let btc_end = marks.close_at(&btc_symbol, period_end).ok();
```

Replacement:

```rust
// ── 3. Mark-to-market unrealized P&L ────────────────────────────────────
let open_positions = audit::query::open_positions_at(&ledger, period_end).await?;
let mut unrealized: Decimal = Decimal::ZERO;
let mut mark_misses: u32 = 0;
for pos in &open_positions {
    match marks.close_at(&pos.symbol, period_end) {
        Ok(mark) => {
            // unrealized contribution = qty * (mark - cost_basis_per_unit)
            unrealized += pos.qty * (mark - pos.avg_cost_basis.amount());
        }
        Err(MarkError::OutOfRange { .. }) => {
            tracing::warn!(
                symbol = %pos.symbol,
                ts = %period_end,
                "mark unavailable for open position"
            );
            mark_misses += 1;
            // Q6: contribute 0; do not propagate.
        }
        Err(e) => return Err(ReportError::Marks(e)),
    }
}
let mark_unavailable_footnote = mark_misses > 0;

// (The BTC baseline lookup keeps its existing pattern.)
let btc_symbol = Symbol::new("BTCUSDT");
let btc_start = marks.close_at(&btc_symbol, period_start).ok();
let btc_end = marks.close_at(&btc_symbol, period_end).ok();
```

Downstream changes (small):
- `recon_inputs.unrealized = unrealized;` (already wired).
- `recon_inputs.equity_check_sum = realized_period.amount() + unrealized;`
  (already wired; will now be non-zero on
  `build_ledger_with_open_positions_7d`).
- `render::reconciliation::render(...)` gets a new `mark_unavailable:
  bool` field on its inputs struct → renders the footnote when true
  (R11.1 row only). Existing renderer signatures are extended
  additively; the empty-positions code path (existing fixtures)
  passes `false` and emits the SAME bytes as today.
- The `equity-<window>.csv` writer's `unrealized_pnl_usdt` column
  now reflects the real value at `period_end` (zero on existing
  fixtures, non-zero on the new fixture).
- R3 equity-curve sampler (`sample_equity_curve` at
  `lib.rs:194`): SCOPE-OUT for v1+. The curve still uses
  `cash.amount()` flat across the window. R4 risk metrics
  derived from the curve therefore continue to ignore intra-
  window position-value swings. This feature closes the
  `period_end` snapshot of unrealized; the per-bar curve walk
  is a separate v2+ wave (intra-window MTM requires a series of
  `close_at` calls per bar, a bigger workload). Documented
  explicitly so the analyst doesn't expect drawdowns to start
  reflecting open-position swings.

### Test strategy (per V-item)

| V-item | Test location | Fixture | Assertion |
|--------|---------------|---------|-----------|
| **V1** Reader correctness | `crates/audit/tests/open_positions_at.rs` (NEW) | `build_ledger_with_open_positions_7d.rs` | `open_positions_at(ledger, period_end)` returns exactly 2 rows; assert `(symbol, qty, avg_cost_basis, opened_at, strategy_id)` byte-identical to the hand-coded expected vec. |
| **V2** Orchestrator unrealized | `crates/reports/tests/unrealized_orchestrator.rs` (NEW) | same + `FrozenMarkSource` with BTCUSDT@70k, ETHUSDT@3.5k | `generate(...)` body's R11.1 reconciliation row reports `unrealized = +200.00 USDT`; `equity-<window>.csv`'s `unrealized_pnl_usdt` column at the final row equals `+200.00`. |
| **V3** Empty-positions backwards compat | `crates/reports/tests/report_scenarios.rs` (EXISTING) | `build_ledger_7d.rs` + `build_ledger_90d.rs` | Existing 11/11 anchor PASS unchanged. The hash-match itself proves bodies are byte-identical to today. |
| **V4** Reconciliation invariant | `crates/audit/tests/open_positions_at.rs` (NEW; same file as V1) | `build_ledger_with_open_positions_7d.rs` | `audit::verify_balance(...)` on every transaction id in the fixture returns Ok; `global_debit_credit_sum() == Decimal::ZERO`. |
| **V5** Anchor regression | `bash scripts/verify_anchors.sh` | (no fixture — gate over committed `spec/operator-success-reports/reports/*.md`) | `ANCHORS PASS  (11 / 11)`. |
| **V6** Existing event invariants | unchanged existing tests `crates/audit/tests/{feed_reconnect_smoke,strategy_events_kind_round_trip,kill_switch_dual_write}.rs` (already shipped) | their existing fixtures | All green; no change needed. **PLUS** new `crates/reports/tests/mark_unavailable_warns.rs`: a fixture with one open position whose symbol is NOT in the `FrozenMarkSource` → asserts `tracing` warn fires once + body contains the footnote string + run does not error. |
| **V7** Determinism | `crates/audit/tests/open_positions_at.rs` (NEW; same file) | `build_ledger_with_open_positions_7d.rs` | Two consecutive `open_positions_at(...)` calls produce `assert_eq!`-equal `Vec<OpenPosition>`. |
| **V8** Perf smoke | `crates/reports/tests/perf_smoke_open_positions.rs` (NEW) | new fixture variant: 100 fills (50 buy/sell pairs) + 5 unmatched Buys | `let t = Instant::now(); open_positions_at(...).await?; assert!(t.elapsed() < Duration::from_millis(100));` |

### Risks + mitigations

| # | Risk | Mitigation |
|---|------|------------|
| **R-1** | Anchor drift if Q5 fixture choice goes wrong (hidden mutation of `build_ledger_7d.rs`) | T_FINAL anchor gate is the regression catch. Q5 explicitly forbids modifying the existing fixtures; reviewer rejects any PR that touches `build_ledger_7d.rs` or `build_ledger_90d.rs` lines 188–247. |
| **R-2** | Perf regression: `open_positions_at` scans all fills | V8 perf gate (<100ms on 100 fills + 5 open positions). If gate fails, ship migration `006_open_positions_index.sql` as a follow-up; do not relax the gate. |
| **R-3** | `MarkSource::OutOfRange` in production with no marks loaded | Q6 contract: warn + zero + body footnote (deterministic). Operator runbook entry added to `spec/runbooks/operator-success-reports.md` (developer ticks the runbook update under T_FINAL). |
| **R-4** | Weighted-avg cost-basis precision (`Decimal` division loss) | One division per emitted `OpenPosition`, not per fill. `Decimal` default precision (28 digits) is well above any realistic `notional / qty` quotient. Property test (V7) re-runs the same arithmetic and asserts byte-equality across two reader invocations. No `f64` anywhere. |
| **R-5** | R10's BTC hardcode making the description-parser brittle | The parser already handles ETH/SOL/BNB/etc. (verified against `build_ledger_90d.rs` in `pnl_by_symbol`'s integration tests). The hardcode is at the account_id, NOT the description. We do not consume the account_id. New test V1 explicitly exercises BTCUSDT + ETHUSDT in the same fixture. |
| **R-6** | `OpenPosition` cross-crate type churn (added to `trading_core`, blocks recompilation of every dependent crate) | `trading_core` is at the bottom of the dep graph; an additive struct triggers a full workspace rebuild. Mitigated by sequencing T1001 (the type addition) first, then fanning out T1002 + T1003 in parallel. |
| **R-7** | Q6 footnote-vs-no-footnote determinism slip (a flaky `tracing` subscriber order causing the body to flicker) | The footnote is computed from a `mark_misses: u32` counter, NOT from log output. The body is independent of `tracing` configuration. |

### Operator-success-reports invariants that must hold

- **T802** — `post_fill(ledger, fill, strategy_id: Option<&str>)`
  signature unchanged. The new reader CONSUMES the `strategy_id`
  column on `journal_transactions` (T802's migration 004 column);
  it does not write to the journal at all.
- **T805** (feed-reconnect) and **T806** (mean-reversion-stop, in
  v1.5a vocabulary; see `spec/operator-success-reports/tasks.md`
  for the v1+ vocabulary mapping) — independent code paths;
  unchanged.
- **T809** dual-write (`kill_switch_tripped` writes the v0 memo
  journal row PLUS the new `strategy_events` row) — unchanged.
- **T810** in-process cron flag (`--features in_process_cron`) —
  unchanged.
- **2 v1+ anchors** (`report-sample-7d`, `report-sample-90d`) —
  byte-identical, NO re-lock (Q4 resolution).

### Determinism guardrails (architect checklist re-confirmed)

- No `SystemTime::now()` reachable from the new reader. Confirmed:
  the reader's only inputs are `&Ledger` and a caller-supplied
  `Timestamp`.
- No `f64`. Confirmed: arithmetic is `Decimal` + `Money<Usdt>`
  throughout.
- 6-digit microsecond `ts` format unchanged. Confirmed: the new
  reader does NOT emit any new `ts` strings; it reads existing
  rows. The `OpenPosition.opened_at` field is populated from the
  first un-closed Buy's `journal_transactions.ts` column verbatim.
- No `HashMap` on the hot path. Use `BTreeMap<(Symbol,
  Option<StrategyId>), (Decimal, Decimal)>` (qty, notional)
  matching the precedent at `query.rs::pnl_by_symbol` line 480.
- No new RNG. Reader is pure SQL + fold.

### Library / crate compatibility checklist

No new external crate dep — checklist N/A. Workspace edition 2021
unchanged. No stdlib-name shadow.

## Implementation

- **T1006 — V2 + V6 orchestrator tests (2026-05-01, developer):**
  V2 positive-path test at
  `crates/reports/tests/unrealized_orchestrator.rs:92`
  (`t1006_v2_unrealized_equals_200_usdt`) — drives
  `reports::generate(...)` against the T1004 fixture under a
  `FrozenMarkSource` carrying both BTCUSDT and ETHUSDT marks at
  `period_end`, asserts the R11 reconciliation appendix's
  Ledger-side cell == `+200 USDT` (BTC `0.01 × (70_000 − 60_000) =
  +100` plus ETH `0.20 × (3_500 − 3_000) = +100`) and
  `MARK_UNAVAILABLE_FOOTNOTE` absent. V6 negative-path tests
  exercise the architect Design § Q6 mark-miss contract under a
  `FrozenMarkSource` that omits ETHUSDT — asserting `tracing::warn!`
  fires once for the missed symbol, the position contributes
  `Decimal::ZERO`, the body footnote literal renders verbatim, and
  `generate(...)` returns `Ok(_)` (no `Err(ReportError::Marks)`
  propagation).

- **T1006 stabilization — V6 warn-capture test split (2026-05-02,
  developer, orchestrator-spawned):** the tester's first re-run of
  T1006 surfaced a `tracing::Dispatch` thread-local cache race in
  the original `crates/reports/tests/mark_unavailable_warns.rs`
  binary (the no-subscriber `t1006_v6_footnote_present_when_miss`
  ran in parallel with the `with_default(...)`-scoped
  `t1006_v6_mark_miss_warns_and_zeroes`, intermittently binding the
  orchestrator's `tracing::warn!` call site to NoSubscriber before
  the capture layer installed → 0-event capture in 2/4 runs).
  Production code is correct; only the test infra was unreliable.
  Stabilization split the file into two single-test integration
  binaries
  (`crates/reports/tests/mark_unavailable_warns_capture.rs:146` for
  the warn-capture test;
  `crates/reports/tests/mark_unavailable_warns_footnote.rs:43` for
  the body-footnote literal test) so cargo's per-binary process
  isolation guarantees a clean dispatcher-cache state on the
  capture thread. No production code, `Cargo.toml`, or
  `spec/anchors.toml` was touched. Verified with 5 consecutive
  `cargo test -p reports --test mark_unavailable_warns_capture
  --test mark_unavailable_warns_footnote` runs — 5/5 PASS.
  Anchor gate `bash scripts/verify_anchors.sh` → 11/11 PASS.

## Verification — links
_tester fills this — left blank intentionally_

## UI
_no new UI surface expected. The cockpit's PNL panel reads the bus's
`pnl` channel (T903c reconciler); once `generate(...)` computes real
unrealized, the cockpit picks it up automatically. ui-designer review
optional but not required at v1+ scope. Architect confirms in design
phase._

## Changelog

- 2026-05-02 (analyst): initial draft. Promotes "Real
  mark-to-market unrealized P&L" from `spec/backlog.md` Queue.
  Builds on `MarkSource` (T812) + `assets:position:*` accounts
  + T802 `strategy_id` column. 10 R-items, 8 V-items, 8 open
  questions. Anchor risk: preferred outcome
  byte-identical (R3/R4); fallback re-lock per v1.5a T717
  precedent if Q5 extends a fixture. HANDOFF → architect.
- 2026-05-02 (architect): appended Design section resolving
  Q1–Q8. **Q1** snapshot vec `open_positions_at(ledger, ts) ->
  Vec<OpenPosition>` in `audit::query`. **Q2** `OpenPosition`
  lives in `trading_core` (`{symbol, qty: Decimal, avg_cost_basis:
  Money<Usdt>, opened_at, strategy_id: Option<StrategyId>}`).
  **Q3** no new SQL index for v1+ (deferred to follow-up
  migration `006_*.sql` only if V8 perf fails); **R10**
  (post_fill BTC hardcode) DEFERRED to a follow-up brief
  `per-symbol-position-accounts.md`. **Q4** anchors stay
  byte-identical (architect-confirmed by reading both
  fixtures: 6 + 12 perfectly symmetric Buy/Sell pairs;
  `unrealized = 0` on both → 11/11 PASS holds). **Q5**
  ADD `build_ledger_with_open_positions_7d.rs` (test-only,
  non-anchored). **Q6** mark miss → warn + zero + body
  footnote (architect override of analyst's
  surface-as-front-matter recommendation; determinism
  rationale documented). **Q7** weighted-avg cost basis
  with proportional release on Sells. **Q8** long-only,
  net-negative qty raises `LedgerError::Database`. Crate
  map delta + public-API additions + exact orchestrator
  diff + 7 risks + V1–V8 test strategy + 5
  operator-success-reports invariants documented. No new
  external dep. HANDOFF → developer (tasks at
  `spec/real-mtm-unrealized-pnl/tasks.md`).
