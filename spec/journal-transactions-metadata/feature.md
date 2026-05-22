---
slug: journal-transactions-metadata
status: shipped
owner: tester
updated: 2026-05-03
version: 1.6.1
---

# Journal-transactions metadata reader

## Why

The just-shipped [`tape-row-audit-modal`](../tape-row-audit-modal/feature.md)
renders journal entries correctly, but in **live mode** the modal's
header is empty: `description` displays as `""` and `strategy_id`
falls through to `TAPE_AUDIT_MODAL_STRATEGY_NONE` (`—`). The
4-column entries table shows the *what*; the parent
`journal_transactions` row's *why* — e.g.
`"paper fill: BTCUSDT buy 0.04 @ 60000.00 / strategy=sma_crossover"`
— is missing.

The gap was flagged at T1206 ship.
[`tape-row-audit-modal.md` Implementation § Async dispatch (lines
625–635)](../tape-row-audit-modal/feature.md):

> The view's header fields (`description`, `strategy_id`) default
> to empty / `None` until a follow-up adds the journal_transactions
> metadata reader; entries + ts + tx_id are populated.

This feature is that follow-up.

[`crates/audit/src/query.rs:297`](../../crates/audit/src/query.rs)
(`journal_entries_for_transaction`) is by design narrow per T1202's
"one reader, one job" pattern — it returns only the row-level
entries join. The metadata reader is its sibling.

The schema is ready:
[`crates/audit/migrations/001_chart_of_accounts.sql:12–17`](../../crates/audit/migrations/001_chart_of_accounts.sql)
defines `journal_transactions(id, ts, description, metadata)`;
[`migration 004`](../../crates/audit/migrations/004_journal_transactions_strategy_id.sql)
adds nullable `strategy_id`. All four columns the modal needs are
queryable today.

[`crates/ui/src/bin/cockpit_live.rs:496–528`](../../crates/ui/src/bin/cockpit_live.rs)
is the single wiring site: the `Task::perform` async closure
currently calls `journal_entries_for_transaction` then constructs a
partial `JournalTransactionView { description: SmolStr::default(),
strategy_id: None, … }`. After this feature, the closure chains the
new metadata reader and constructs a complete view.

Small, additive, read-only — no anchor risk, no write-path touch.

## Requirements (R-items)

### R1 — New `audit::query::journal_transaction_metadata` reader

```rust
pub async fn journal_transaction_metadata(
    ledger: &Ledger,
    tx_id: &str,
) -> Result<Option<JournalTransactionMetadata>, LedgerError>;
```

Returns the header row from `journal_transactions`, or `Ok(None)`
for an unknown `tx_id` (stale row, fixture-mode click). Read-only
`SELECT id, ts, description, strategy_id FROM journal_transactions
WHERE id = ?`. Lives next to `journal_entries_for_transaction` in
[`crates/audit/src/query.rs`](../../crates/audit/src/query.rs).

### R2 — New `core::JournalTransactionMetadata` struct

```rust
pub struct JournalTransactionMetadata {
    pub transaction_id: SmolStr,
    pub ts: Timestamp,
    pub description: String,
    pub strategy_id: Option<StrategyId>,
}
```

Lives in [`crates/core/src/views.rs`](../../crates/core/src/views.rs)
alongside `JournalEntry`; re-exported from `core::lib`.

**Analyst recommendation:** new struct, NOT a reuse of
`ui::state::JournalTransactionView`. The view carries the entries
vector — redundant for a header-only reader. The metadata struct is
stitched into the view at the cockpit_live site, not substituted
for it. Architect picks (Q1).

### R3 — Cockpit_live wires the reader BEFORE the entries reader

[`crates/ui/src/bin/cockpit_live.rs:496–528`](../../crates/ui/src/bin/cockpit_live.rs)
`Task::perform`: replace the partial view construction with a
chained read calling `journal_transaction_metadata(tx_id)` first,
then `journal_entries_for_transaction(tx_id)`, then constructing
the complete `JournalTransactionView` from both. Sequential vs
`tokio::join!` — architect picks (Q4).

The fixture-mode `cockpit` binary path is unchanged.

### R4 — Backwards compat with pre-T1102 ledgers

The reader queries `journal_transactions` directly — does NOT
derive from entries. Pre-T1102 ledgers have a populated
`description` column (the legacy description-parse path wrote it);
returned verbatim. `strategy_id` is nullable (added in migration
004) — pre-T802 rows surface as `None`, mirroring today's
`pnl_by_strategy` `(unattributed)` bucket. No backfill required.

### R5 — Anchor regression: 11/11 PASS

Read-only, additive. The 11 anchored reports never round-trip
through live `audit::query::*` readers; backtests use
`PaperEnginePublisher` with `NullPublisher`. The new reader is
not on any anchored path. Existing modal snapshots (4 in T1207)
stay byte-identical when re-rendered against fixtures whose
`description` is empty (current shape) — empty description rows
render identically to today.

`bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.

### R6 — T802 / T805 / T806 / T809 / T810 invariants hold

Operator-success-reports + live-cockpit-unified invariants. This
feature touches zero write paths, zero subscription wiring, zero
kill-switch logic. Verification = existing tests stay green.

### R7 — `journal_entries_for_transaction` (T1202) is unchanged

DO NOT modify the existing narrow reader. The new reader is
strictly additive. Per T1202's "one reader, one job" pattern —
entries reader stays row-level, metadata reader is its sibling,
the live cockpit composes both at the `Task::perform` site.

## Verification (V-items)

### V1 — Reader returns expected metadata for an existing transaction

Unit test in `crates/audit/tests/journal_transaction_metadata.rs`
(NEW). Boot fresh in-memory ledger, post one paper Buy fill (post_fill
returns the txn_id per T1206), call
`journal_transaction_metadata(&ledger, &txn_id)`. Assert
`Ok(Some(m))` with `transaction_id == txn_id`,
`description == "paper fill: …"`,
`strategy_id == Some(StrategyId::new("sma…"))`, `ts` matches.

### V2 — Reader returns `Ok(None)` for an unknown tx_id

Same test file. Synthetic UUID never written. Assert `Ok(None)` —
not `Err`, not `Ok(Some(default))`. Mirrors T1202's contract.

### V3 — Live cockpit modal shows full description + strategy_id

Smoke test against `cockpit_live` (existing
`crates/ui/tests/cockpit_live_*` family): boot ledger with one fill,
click a tape row, assert
`JournalTransactionView.description != ""` and
`strategy_id == Some(_)`. Specific shape: architect picks (Q5).

### V4 — Anchors 11/11 PASS

`bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`.
Regression gate per R5.

### V5 — Operator-success + live-cockpit invariants hold

`cargo test --workspace` + `cargo test -p ui --features live` all
green. R6 verification.

## Backtest scenarios

_n/a — additive read-only feature._

## Open questions for architect

### Q1 — Type home: new `core::JournalTransactionMetadata` vs reuse `JournalTransactionView`

R2 proposes a new struct. Alternative: reuse
`ui::state::JournalTransactionView` (carries
`tx_id / ts / description / strategy_id / entries`).

**Analyst recommendation:** **new struct in `core`.** The view's
entries vector is redundant for a header-only reader; reusing it
forces `entries: vec![]` placeholders or `Option<Vec<…>>` friction.
The view also lives in `ui::state`, not `core` — the audit reader
returning a `ui` type inverts the dependency direction.

### Q2 — Single fused reader vs two separate readers?

Should this be merged with `journal_entries_for_transaction` into
a single fetch returning `(Metadata, Vec<JournalEntry>)`?

**Tradeoff:** one async call (faster, simpler wiring) vs two
readers (cleaner testability — snapshot tests can build a modal
fixture from metadata alone; T1202's "one reader, one job"
preserved).

**Analyst recommendation:** **two separate readers.** Wall-clock
delta is microseconds for in-process SQLite; testability win is
real. Architect picks.

### Q3 — Exact field set on `JournalTransactionMetadata`

R2 proposes `transaction_id`, `ts`, `description`, `strategy_id`.
The schema also has `metadata: TEXT NOT NULL DEFAULT '{}'` (JSON
blob, currently unused by readers). Expose it?

**Analyst recommendation:** **omit `metadata` for v1.** The modal
header has no surface for a JSON blob; format choice (raw JSON?
key-value pairs?) has no operator-driven precedent. Add when a
concrete consumer materializes (three-uses rule applied to
schema fields).

### Q4 — `Task::perform` chain: sequential await vs `tokio::join!`?

R3 proposes sequential. Alternative: `tokio::join!` (~2× faster
wall-clock, both queries fire concurrently).

**Tradeoff:** sequential lets metadata `None` skip the entries
query (saves a round-trip on stale clicks); `join!` is faster on
the happy path but always pays for both.

**Analyst recommendation:** **sequential.** Wall-clock delta is
microseconds; the "skip entries on unknown tx_id" branch gives a
clean "unknown transaction" error path. Architect picks.

### Q5 — Snapshot test strategy: extend T1207 or add a new snapshot?

T1207 shipped 4 modal snapshots. The
`…_ready_paper_fill.snap` fixture already populates `description`
and `strategy_id`; it stays byte-identical regardless of this
wiring change.

Options: (1) re-verify T1207's existing 4 snapshots stay
byte-identical + add a small unit-level integration test on the
cockpit_live wiring; (2) add a NEW heavy `cockpit_live` snapshot
end-to-end via a synthetic ledger.

**Analyst recommendation:** **(1).** Byte-identical T1207 re-run
is the strongest "no regression" signal; a new heavyweight
snapshot duplicates T1207's coverage. Architect picks.

### Q6 — Error semantics on partial failure

What if metadata succeeds but entries fails (or vice versa)? Today
the modal shows entries-or-error; with two chained reads, the
failure space doubles.

Options: (a) any `Err` → modal shows
`TAPE_AUDIT_MODAL_ERROR_PREFIX + msg`; (b) metadata-error +
entries-OK → render entries with empty header (graceful degrade);
(c) entries-error + metadata-OK → render header alone.

**Analyst recommendation:** **(a) any `Err` → error state.**
Consistent with today; partial render is a UX trap (operator
can't tell whether an empty header means "no description" or
"couldn't load"). Architect picks.

## Design

Architect resolutions for Q1–Q6. Scope is small and fully additive: one
new struct in `core::views`, one new reader in `audit::query`, one
chained-fetch edit at the `cockpit_live` `Task::perform` site, plus
test scaffolding. No write-path touch, no migration, no new dep, no
new theme/string/widget surface, no anchor risk.

### Q1 — Type home: NEW `core::JournalTransactionMetadata`

**Decision:** Introduce
`pub struct JournalTransactionMetadata { transaction_id: SmolStr, ts:
Timestamp, description: SmolStr, strategy_id: Option<StrategyId> }`
in [`crates/core/src/views.rs`](../../crates/core/src/views.rs)
alongside the existing `JournalEntry` (T1201) and `FillView` view
types. Re-export from
[`crates/core/src/lib.rs:48`](../../crates/core/src/lib.rs) on the
same line as `FillView, JournalEntry, JournalEntryView, …`.
`#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]` to match
neighbours. Keep `ui::state::JournalTransactionView` (entries +
header combined) **unchanged**.

**Rationale:**
- Domain-level metadata is a `core` concern (the audit reader returns
  it, the live runtime stitches it, the UI renders it). Putting it in
  `ui::state` would invert the dependency direction (audit depending
  on ui).
- A header-only metadata reader does not own an `entries: Vec<…>`
  field; reusing `JournalTransactionView` would force `entries:
  vec![]` placeholders or `Option<Vec<…>>` friction at the audit
  boundary.
- `core::views` is the established home for read-side DTOs (precedent:
  `FillView`, `JournalEntry`, `JournalEntryView`, `PnlSnapshot`,
  `PositionView`).

**Principled override on the brief default:** the brief proposes
`description: String`. I pin **`description: SmolStr`** for symmetry
with `JournalTransactionView.description: SmolStr` (the consumer
struct), `JournalEntry.memo: SmolStr`, and the audit-write side that
formats descriptions as short `format!("{} {} {} @ {}", side, qty,
sym, px)` strings — well below the 23-byte `SmolStr` inline-storage
threshold for typical paper-fill descriptions. Heap fallback for the
LLM-cost / registry-event description shapes is fine; cost is identical
to `String` on the slow path. Keeps the cockpit-side stitch a
move (no `String→SmolStr` re-allocation).

**Alternatives rejected:**
- *Reuse `ui::state::JournalTransactionView`* — inverts crate
  dependency direction; forces empty-entries placeholders at the
  audit boundary.
- *Land in `crates/core/src/lib.rs` directly* — `lib.rs` is the
  re-export hub, not the type-definition site; `views.rs` is the
  established home (5 DTOs there already).
- *`description: String`* — see principled override above.

### Q2 — Two separate readers (KEEP SEPARATE)

**Decision:** Keep `journal_transaction_metadata` and the existing
`journal_entries_for_transaction` (T1202) as **two distinct async
readers**. Cockpit_live's `Task::perform` closure sequences both. No
fused `(Metadata, Vec<JournalEntry>)` reader.

**Rationale:**
- Preserves T1202's "one reader, one job" pattern documented in
  [tape-row-audit-modal.md → Open questions for architect Q2](../tape-row-audit-modal/feature.md#q2--journalentry-un-collapsed-lives-in-trading_core).
  Each reader is a single SQL statement against a single table —
  trivially auditable.
- Snapshot tests can construct a header-only metadata fixture
  without dragging in entries (small future win — every modal-state
  variant doesn't need a 4-row entries fixture).
- The cockpit_live closure is short and composable (await metadata;
  if `Some`, await entries).

**Alternatives rejected:**
- *Fused reader returning `(Metadata, Vec<JournalEntry>)`* — buys
  microseconds, costs the "one reader, one job" pattern, costs
  testability granularity, doubles the cardinality of test fixtures
  per failure mode.

### Q3 — Field set: 4 fields, omit `metadata: TEXT` JSON column

**Decision:** Pin the four fields proposed in R2:
- `transaction_id: SmolStr`
- `ts: Timestamp`
- `description: SmolStr`
- `strategy_id: Option<StrategyId>`

Omit the schema's `metadata: TEXT NOT NULL DEFAULT '{}'` JSON-blob
column.

**Rationale:**
- The four chosen fields are the exact set the modal header consumes
  today. No speculative columns.
- The `metadata` JSON blob has zero consumers in the rendering path
  (modal header has no surface for a free-form blob; format choice
  — raw JSON, key-value pairs, lazy disclosure — has no
  operator-driven precedent). Three-uses rule: revisit when a
  concrete consumer materializes (e.g. a "raw metadata JSON"
  disclosure section in a future modal expansion).

**Alternatives rejected:**
- *Add `metadata: Option<String>` (or `Option<serde_json::Value>`)*
  — no consumer, no operator-aligned format choice, deferred safely.

### Q4 — Sequential `await` (NOT `tokio::join!`)

**Decision:** Sequential await chain at the
[`crates/ui/src/bin/cockpit_live.rs:518–524`](../../crates/ui/src/bin/cockpit_live.rs)
`Task::perform` closure:

1. Await `journal_transaction_metadata(&ledger, tx_id)`.
2. On `Ok(None)` — return `Err(SmolStr::new(TAPE_AUDIT_MODAL_ERROR_PREFIX
   + "unknown transaction"))`. Skip the entries query (Q4 short-circuit
   benefit).
3. On `Ok(Some(meta))` — await
   `journal_entries_for_transaction(&ledger, tx_id)`.
4. Stitch metadata + entries into a complete `JournalTransactionView`.

**Rationale:**
- Both queries hit the same in-process SQLite cache; combined wall
  time is sub-millisecond on a hot path. `tokio::join!` saves
  microseconds at the cost of:
  - More complex error handling (two `Result`s to merge).
  - No short-circuit on the metadata-`None` branch — the entries
    query always fires even when we're going to discard the result.
- Sequential composition reads top-to-bottom; the `metadata.is_none()
  → error` branch is explicit (mirrors today's "unknown tx_id"
  signal channel).

**Alternatives rejected:**
- *`tokio::join!`* — microsecond win, ergonomic + error-handling
  loss, no short-circuit on stale-click.
- *`tokio::try_join!`* — same problem; loses the metadata-`None`
  vs entries-error distinction.

### Q5 — Snapshot strategy: re-verify T1207's 4 snapshots, ADD wiring smoke test (no new snap)

**Decision:** Apply the analyst's recommendation
([Q5 option 1](#q5--snapshot-test-strategy-extend-t1207-or-add-a-new-snapshot)).
The four T1207 modal snapshots stay byte-identical:

- `panel_snapshots__tape_audit_modal_loading.snap`
- `panel_snapshots__tape_audit_modal_empty.snap`
- `panel_snapshots__tape_audit_modal_error.snap`
- `panel_snapshots__tape_audit_modal_ready_paper_fill.snap`

The `…_ready_paper_fill` fixture already pre-stamps `description:
"buy 0.04 BTCUSDT @ 50000"` and `strategy_id: Some("sma_crossover")`
(verified at
[`crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_ready_paper_fill.snap:14-15`](../../crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_ready_paper_fill.snap)).
The snapshot summarizes `JournalModalState` shape — it does not
inspect the *source* of the populated fields. Re-running T1207 after
this feature lands MUST produce byte-identical output.

Add ONE new integration test exercising the chained-fetch path:
`crates/ui/tests/cockpit_live_modal_metadata_chain.rs` (NEW). Boot
in-memory ledger, post one paper Buy via `journal::post_fill`,
synthesize a `Message::TapeRowClicked(txn_id)`, drive the
`Task::perform` chain, and assert the resulting
`JournalTransactionView` has `description.as_str() != ""` and
`strategy_id == Some(_)`. This is a wiring smoke test, NOT a
snapshot — it pins the live-mode chain end-to-end without coupling
to the modal's rendered shape.

**Principled override on the brief default:** the brief proposes
adding `panel_snapshots__tape_audit_modal_ready_with_metadata.snap`.
I reject this override: a duplicate snapshot adds no signal because
the snapshot harness consumes a `JournalModalState`, not a
provenance-tagged source. The existing `…_ready_paper_fill.snap`
*already* covers the populated header render; a new snapshot would
be byte-identical noise.

**Rationale:**
- Byte-identical T1207 re-run is the strongest "no regression"
  signal: existing rendering is provably untouched.
- The wiring smoke test pins the new chained-fetch path at a layer
  the existing snapshot never reaches (Task::perform composition).
- Net snapshot count unchanged at 4; net integration-test count +1.

**Alternatives rejected:**
- *New populated-metadata snapshot (brief default)* — duplicates
  existing `…_ready_paper_fill.snap`; the `JournalModalState` shape
  doesn't carry provenance, so the snapshot would be
  byte-identical.
- *No new test at all* — leaves the chained-fetch path uncovered;
  unit tests on `journal_transaction_metadata` (V1, V2) cover the
  reader but not the cockpit_live closure composition.

### Q6 — Partial-failure semantics: any-`Err` → `Error` state

**Decision:** Map every non-happy outcome of the chained fetch to
`JournalModalState.entries: PanelState::Error(SmolStr)`:

| Metadata reader   | Entries reader | Modal state                                  |
|-------------------|----------------|----------------------------------------------|
| `Ok(Some(m))`     | `Ok(vec![])`   | `Empty`                                      |
| `Ok(Some(m))`     | `Ok(non-empty)`| `Ready(view{description=m.description, …})`  |
| `Ok(Some(m))`     | `Err(e)`       | `Error(TAPE_AUDIT_MODAL_ERROR_PREFIX + e)`   |
| `Ok(None)`        | _skipped_      | `Error(TAPE_AUDIT_MODAL_ERROR_PREFIX + "unknown transaction")` |
| `Err(e)`          | _skipped_      | `Error(TAPE_AUDIT_MODAL_ERROR_PREFIX + e)`   |

**Rationale:**
- Consistent with today's modal: any error path renders the existing
  `TAPE_AUDIT_MODAL_ERROR_PREFIX` copy. No new strings, no new
  widget changes.
- Partial render (header-only or entries-only) is a UX trap: an
  empty header could mean "no description on this tx" *or* "couldn't
  load metadata"; the operator can't tell. A consistent `Error`
  state is honest.
- Operator retry: clicking the same row again retriggers the chain
  (T1206 `Message::TapeRowClicked` is idempotent on identity —
  re-clicking just re-fires the load).

**Alternatives rejected:**
- *Graceful-degrade: render entries even when metadata fails* — UX
  trap (see above).
- *Distinct error states (`MetadataError` / `EntriesError`)* — adds
  state-machine surface for zero operator value (operator action is
  the same: retry, or move on).

### Crate-map delta

| Crate / file                                          | Change                              |
|-------------------------------------------------------|-------------------------------------|
| `crates/core/src/views.rs`                            | NEW `pub struct JournalTransactionMetadata` |
| `crates/core/src/lib.rs:48`                           | Re-export `JournalTransactionMetadata` next to `JournalEntry, …` |
| `crates/audit/src/query.rs`                           | NEW `pub async fn journal_transaction_metadata` (sibling of `journal_entries_for_transaction` at line 297) |
| `crates/audit/tests/journal_transaction_metadata.rs`  | NEW — V1, V2 unit tests             |
| `crates/ui/src/bin/cockpit_live.rs:496–528`           | Replace partial-view construction with chained metadata→entries fetch (Q4) + Q6 error mapping |
| `crates/ui/tests/cockpit_live_modal_metadata_chain.rs`| NEW — V3 chained-fetch smoke test   |

**Unchanged on purpose** (R7 + R5):
- `crates/audit/src/query.rs:297-345` (`journal_entries_for_transaction`).
- `crates/audit/src/journal.rs` (write path; `post_fill` signature).
- `crates/audit/migrations/*` (no migration; schema covers all four
  fields already).
- `crates/ui/src/state.rs` (`JournalTransactionView` /
  `JournalModalState` shapes — the new metadata struct is
  consumed at the cockpit_live stitch site, not substituted for
  either).
- `crates/ui/src/widgets/journal_transaction_modal.rs` (renders
  the same `JournalTransactionView` it does today).
- `crates/ui/src/strings.rs` (no new copy — existing
  `TAPE_AUDIT_MODAL_ERROR_PREFIX` covers Q6).
- `crates/ui/src/theme.rs` (no new tokens).
- The four T1207 modal snapshots (Q5).

### Public API additions

```rust
// crates/core/src/views.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JournalTransactionMetadata {
    /// `journal_transactions.id` UUID string.
    pub transaction_id: SmolStr,
    /// Transaction timestamp (microsecond precision).
    pub ts: Timestamp,
    /// Free-form description (e.g. `"buy 0.04 BTCUSDT @ 50000"`).
    /// Empty string for legacy rows without a description.
    pub description: SmolStr,
    /// Attribution to the strategy that emitted the signal.
    /// `None` for pre-T802 rows or non-strategy transactions.
    pub strategy_id: Option<StrategyId>,
}

// crates/audit/src/query.rs (additive)
pub async fn journal_transaction_metadata(
    ledger: &Ledger,
    tx_id: &str,
) -> Result<Option<JournalTransactionMetadata>, LedgerError>;
```

Consumed by:
- `crates/ui/src/bin/cockpit_live.rs` (sole consumer for v1).

### SQL shape

```sql
SELECT id, ts, description, strategy_id
FROM journal_transactions
WHERE id = ?
```

- Single-row read; binds `tx_id` once.
- `id` is the UUID string written by `journal::post_fill` at
  `crates/audit/src/journal.rs:82` (`INSERT INTO journal_transactions
  (id, ts, description, strategy_id) …`); confirmed identical to
  the `transaction_id` field stamped on `Fill` after T1202's
  `post_fill` return-type bump.
- `description` is `TEXT NOT NULL DEFAULT ''` (migration 001) —
  always present, possibly empty for legacy rows.
- `strategy_id` is `TEXT` (nullable, migration 004) — `None` for
  pre-T802 rows mirroring the `(unattributed)` bucket convention
  in `pnl_by_strategy`.
- `Ok(None)` when no row matches (stale row, fixture-mode click,
  unknown UUID); never `Err` for missing rows.
- `Err(LedgerError::Database(...))` for SQL / `Decimal` / timestamp
  parse errors (mirroring `journal_entries_for_transaction`).

Async via `sqlx::query_as`; row tuple `(String, String, String,
Option<String>)`. Parse `ts` via `OffsetDateTime::parse(&ts_str,
&Rfc3339).map(Timestamp::new)` (same pattern as
`journal_entries_for_transaction:330`).

### Test strategy (V1–V5 mapping)

| V    | Test                                                        | File                                                                    | Owner        |
|------|-------------------------------------------------------------|-------------------------------------------------------------------------|--------------|
| V1   | Reader returns `Some(meta)` for known tx_id                 | `crates/audit/tests/journal_transaction_metadata.rs` (NEW)              | developer    |
| V2   | Reader returns `Ok(None)` for unknown tx_id                 | same file                                                               | developer    |
| V3   | Cockpit_live modal shows full description + strategy_id     | `crates/ui/tests/cockpit_live_modal_metadata_chain.rs` (NEW)            | ui-designer  |
| V4   | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11/11)`   | tester gate                                                             | tester       |
| V5   | `cargo test --workspace` + `cargo test -p ui --features live` green | tester gate                                                             | tester       |

Plus: regression assertion that all four T1207 modal snapshots
remain byte-identical (Q5).

### Risks

1. **Partial-failure UX confusion** — operator sees a generic
   `TAPE_AUDIT_MODAL_ERROR_PREFIX + msg` whether metadata or entries
   failed; can't tell which (Q6 collapses both into one state).
   **Mitigation:** the error message body carries the originating
   reader's error string (e.g. "unknown transaction" vs an SQL parse
   error), which gives the operator a hint. Operator retry =
   re-click the same row. Future work could distinguish with
   per-cause copy if a real incident motivates it.

2. **New reader race against in-flight writes** — two near-identical
   tx clicks during a high-throughput period could theoretically race
   the writer. **Mitigation:** the reader is a `SELECT ... WHERE id
   = ?` against a UUID PRIMARY KEY column; SQLite read-after-write
   semantics are linearizable for this access pattern. The reader is
   read-only — no race against itself. The writer holds the journal
   transaction internally (already the case for T1202's `post_fill`).

3. **Snapshot-test churn for the four T1207 snaps** — if any
   serializer-side change (e.g. `tape_audit_modal_summary`)
   accidentally renders metadata provenance, snapshots drift.
   **Mitigation:** the four T1207 snaps are unchanged structurally —
   `tape_audit_modal_summary` consumes `JournalModalState`, which
   does not carry the new `JournalTransactionMetadata` type (the
   stitch happens upstream at the `Task::perform` site, before the
   `JournalTransactionView` reaches the modal). T1209-style anchor
   regression sweep proves it.

4. **Cockpit_live binary panic on metadata `None`** — the existing
   T1206 closure constructs a partial view unconditionally; the new
   chain has an explicit `None` arm. **Mitigation:** Q6 maps `None`
   to `JournalModalState.entries: PanelState::Error(...)`; T1206's
   `update` arm for `Message::TapeAuditEntriesLoaded(Err(_))` already
   handles the error variant via `PanelState::Error` rendering —
   verified at
   [`crates/ui/src/state.rs`](../../crates/ui/src/state.rs)
   `update` handler for `TapeAuditEntriesLoaded`.

5. **Pre-T802 ledgers surface `strategy_id: None`** — operator sees
   `TAPE_AUDIT_MODAL_STRATEGY_NONE` (`—`) on legacy rows. **Not a
   bug** — mirrors the `(unattributed)` bucket in
   `pnl_by_strategy`. R4 contract; no mitigation needed beyond
   documentation.

### Operator-success-reports + live-cockpit-unified + tape-row-audit-modal invariants that must hold

- **T802 / T805 / T806 / T809 / T810 (operator-success-reports):**
  the new reader does NOT touch any rendering path. No change to
  `crates/reports/src/`. No change to `audit::query::pnl_by_strategy`
  (the `(unattributed)` bucket convention is unchanged). 11/11
  anchored bodies stay byte-identical. R5 / V4.
- **T901 / T903 / T905 (live-cockpit-unified):** no change to the
  bin-shared agent runtime, no change to the bus channels, no change
  to subscription wiring. The cockpit_live closure edit is wholly
  contained in the existing `Task::perform` block (~30 lines at
  `cockpit_live.rs:496-535`). R6 / V5.
- **T1201 / T1202 / T1206 / T1207 (tape-row-audit-modal):** the new
  reader is strictly additive next to T1202's
  `journal_entries_for_transaction`. T1202's signature is byte-identical.
  T1206's `Message` variants are byte-identical (the `Result<
  JournalTransactionView, SmolStr>` shape is preserved — the closure
  composes two reads and emits the same `Message::TapeAuditEntriesLoaded`
  variant). R7 / Q5 / Q6.

## Implementation

_developer fills this — left blank intentionally._

## Verification — links

- 2026-05-03 (tester) FINAL gate:
  [`spec/archive/test-2026-05-03-1608-journal-transactions-metadata-final.md (archived; see spec/archive/README.md)`](../reports/test-2026-05-03-1608-journal-transactions-metadata-final.md)
  — VERDICT → PASS. All five V-items VERIFIED:
  - **V1** `t1302_v1_returns_metadata_for_existing_transaction ... ok`
    (`crates/audit/tests/journal_transaction_metadata.rs`).
  - **V2** `t1302_v2_returns_none_for_unknown_tx_id ... ok` (same file).
  - **V3** `t1304_v3_chained_fetch_populates_view_header ... ok`
    + `t1304_v3b_unknown_tx_short_circuits_to_error ... ok`
    (`crates/ui/tests/cockpit_live_modal_metadata_chain.rs`).
  - **V4** `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`.
  - **V5** `cargo test --workspace --all-targets` clean +
    `cargo test -p ui --features live` clean (panel_snapshots 36/36
    byte-identical; T802/T805/T806/T809/T810 + T901/T903/T905/T906 +
    T1101–T1107 + T1201–T1209 invariants all GREEN).

## UI

_ui-designer fills this if Q5 picks T1207 extension. Visible
change: modal header rows show real description + strategy_id in
live mode. No new strings, no new tokens, no new widgets._

## Changelog

- 2026-05-03 (analyst): initial draft. Promoted from the T1206
  deviation note in
  [`tape-row-audit-modal.md` Implementation § Async dispatch
  (lines 625–635)](../tape-row-audit-modal/feature.md). 7 R-items, 5
  V-items, 6 open questions for the architect. Anchor risk:
  zero — additive read-only feature; no write path, no rendering
  path, no anchored code path consumed by the new reader.
  Recommends: new `core::JournalTransactionMetadata` struct (Q1),
  two separate readers per T1202's "one reader, one job" (Q2),
  omit `metadata` JSON blob for v1 (Q3), sequential await chain
  (Q4), re-use T1207's 4 snapshots for byte-identical regression
  check + small wiring smoke test (Q5), any-`Err` → error-state
  semantics (Q6). HANDOFF → architect.
- 2026-05-03 (architect): resolved the six open questions and
  appended the `## Design` section. **Q1** new
  `pub struct JournalTransactionMetadata` in
  [`crates/core/src/views.rs`](../../crates/core/src/views.rs)
  alongside `JournalEntry` (analyst recommendation accepted; one
  principled override: `description: SmolStr` not `String`, for
  symmetry with `JournalTransactionView.description: SmolStr` and
  `JournalEntry.memo: SmolStr` — keeps the cockpit-side stitch a
  move). **Q2** two separate readers per T1202's "one reader, one
  job" pattern; cockpit_live `Task::perform` sequences both.
  **Q3** four fields (`transaction_id, ts, description,
  strategy_id`); skip `metadata: TEXT NOT NULL DEFAULT '{}'` JSON
  blob (no consumer, no operator-aligned format). **Q4** sequential
  await with metadata-`None` short-circuit; entries query skipped
  on stale-click. **Q5** override of brief default — re-verify
  T1207's existing 4 snapshots stay byte-identical (analyst
  recommendation 1) + add ONE new wiring smoke test
  `crates/ui/tests/cockpit_live_modal_metadata_chain.rs` (NEW); a
  duplicate populated-metadata snapshot would be byte-identical
  noise because `JournalModalState` does not carry provenance.
  **Q6** any-`Err` collapses to
  `PanelState::Error(TAPE_AUDIT_MODAL_ERROR_PREFIX + msg)`;
  metadata-`None` → "unknown transaction" error; consistent with
  today's modal error UX. **Crate-map delta:** new
  `core::JournalTransactionMetadata` struct + re-export; new
  `audit::query::journal_transaction_metadata` reader; new
  `audit/tests/journal_transaction_metadata.rs` test file;
  cockpit_live `Task::perform` chain edit (lines 496–535) replacing
  partial-view construction with chained metadata→entries fetch +
  Q6 error mapping; new `ui/tests/cockpit_live_modal_metadata_chain.rs`
  smoke test. No new dep, no migration, no `Cargo.toml` change, no
  new theme/string/widget surface, no `unsafe`. Anchor budget
  unchanged (11/11 byte-identical) — read-only additive feature
  off the anchored path. **Risks** (5): partial-failure UX
  confusion (mitigation: error msg carries originating cause);
  reader race against writer (mitigation: read-only on UUID PRIMARY
  KEY); snapshot churn for T1207 4 snaps (mitigation: serializer
  consumes `JournalModalState`, not the new metadata type);
  cockpit_live panic on metadata `None` (mitigation: Q6 maps to
  `PanelState::Error`); pre-T802 ledgers surface `strategy_id:
  None` (not a bug, mirrors `(unattributed)` bucket). Tasks
  T1301–T1305 + `T_FINAL_TX_METADATA` filed at
  [tasks/journal-transactions-metadata.md](tasks.md).
  HANDOFF → orchestrator.
- 2026-05-01 (developer): **T1301 done.** Added
  `pub struct JournalTransactionMetadata` at
  [`crates/core/src/views.rs:62-83`](../../crates/core/src/views.rs)
  with the four architect-pinned fields (`transaction_id: SmolStr,
  ts: Timestamp, description: SmolStr, strategy_id:
  Option<StrategyId>`); derives `Debug, Clone, PartialEq, Eq,
  Serialize, Deserialize`. Extended the `crate::symbol` import on
  `views.rs:10` with `StrategyId`. Re-exported next to `JournalEntry,
  …` at [`crates/core/src/lib.rs:48-50`](../../crates/core/src/lib.rs)
  (alphabetical). Added two serde round-trip tests in
  [`crates/core/tests/types_test.rs:255-282`](../../crates/core/tests/types_test.rs)
  (populated row + legacy empty-description / `None` strategy row).
  Verification gates: `cargo test -p trading_core` →
  `test result: ok. 23 passed; 0 failed`; `cargo build --workspace`
  clean; `cargo clippy --workspace --all-targets --all-features --
  -D warnings` clean (one `doc_markdown` lint surfaced and fixed
  inline by backticking ``cockpit_live``); `cargo fmt --check`
  clean; `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)`. T1302 unblocked.
  HANDOFF → orchestrator (T1301 done; T1302 unblocked).
- 2026-05-01 (developer): **T1302 done.** Added
  `pub async fn journal_transaction_metadata(ledger: &Ledger, tx_id:
  &str) -> Result<Option<JournalTransactionMetadata>, LedgerError>`
  at [`crates/audit/src/query.rs:347-403`](../../crates/audit/src/query.rs)
  as a sibling of `journal_entries_for_transaction` (T1202 reader at
  297-345 untouched — R7). SQL: `SELECT id, ts, description,
  strategy_id FROM journal_transactions WHERE id = ?` via
  `sqlx::query_as(...).fetch_optional(...)`; `Ok(None)` short-circuit
  on missing row (Q6 stale-click contract); `OffsetDateTime::parse(...,
  &Rfc3339).map(Timestamp::new)` for the ts column; nullable
  `strategy_id` mapped via `.map(StrategyId::new)`. Extended the
  `trading_core::{...}` import block at
  [`crates/audit/src/query.rs:10-14`](../../crates/audit/src/query.rs)
  with `JournalTransactionMetadata`. Added three `#[tokio::test]` cases
  in [`crates/audit/tests/journal_transaction_metadata.rs`](../../crates/audit/tests/journal_transaction_metadata.rs)
  (NEW): `t1302_v1_returns_metadata_for_existing_transaction` (V1 —
  populated 4-field round-trip; asserts `description ==
  "buy 0.4 BTCUSDT @ 52341.20"` matching the
  `crates/audit/src/journal.rs:58` format-site exactly),
  `t1302_v2_returns_none_for_unknown_tx_id` (V2 — bogus UUID against an
  empty ledger; asserts `Ok(None)` not `Err`, mirrors T1202 V11b),
  `t1302_strategy_id_optional` (NULL-strategy-id row surfaces as
  `Some(meta)` with `strategy_id: None`, mirrors the
  `(unattributed)` bucket convention). Verification gates:
  `cargo test -p audit --test journal_transaction_metadata` →
  `test result: ok. 3 passed; 0 failed; 0 ignored`; `cargo test -p
  audit` → all pre-existing audit suites green (T802 / T805 / T806 /
  T809 invariants intact; T1202's `journal_entries_for_transaction`
  3/3 PASS unchanged); `cargo build -p audit` clean; `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` clean;
  `cargo fmt --check` clean; `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)`. T1303 ‖ T1304 unblocked.
  HANDOFF → orchestrator (T1302 done; T1303 ‖ T1304 unblocked).
- 2026-05-01 (ui-designer): **T1303 + T1304 done.** **T1303** —
  replaced the partial-view construction at
  [`crates/ui/src/bin/cockpit_live.rs:496-552`](../../crates/ui/src/bin/cockpit_live.rs)
  with the architect's Q4 sequential await chain
  (`journal_transaction_metadata` → `Ok(None)` short-circuit to
  `Err("{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction")` →
  `Ok(Some(meta))` proceed to `journal_entries_for_transaction` →
  stitch the complete `JournalTransactionView` from `meta.tx_id /
  meta.ts / meta.description / meta.strategy_id / entries`). Q6
  error mapping: every non-happy outcome (metadata `Err`, metadata
  `None`, entries `Err`, JoinHandle `Err`) collapses to a single
  `Err(SmolStr)` carrying `TAPE_AUDIT_MODAL_ERROR_PREFIX +` originating
  cause; the existing
  `Message::TapeAuditEntriesLoaded(Err(_))` arm in `state::update`
  flips `JournalModalState.entries` to `PanelState::Error(...)`
  unchanged. Removed the `Timestamp::now()` proxy (no clock
  injection reachable from this closure now). `state.rs` untouched —
  `JournalTransactionView` already carries `description: SmolStr`
  per architect's Q1 override. Fixture-mode binary
  `crates/ui/src/bin/cockpit.rs` untouched (R3). Import added at
  [`crates/ui/src/bin/cockpit_live.rs:94`](../../crates/ui/src/bin/cockpit_live.rs):
  `TAPE_AUDIT_MODAL_ERROR_PREFIX` next to `APP_TITLE`. **T1304** — added
  [`crates/ui/tests/cockpit_live_modal_metadata_chain.rs`](../../crates/ui/tests/cockpit_live_modal_metadata_chain.rs)
  (NEW, 183 lines) with two `#[tokio::test]` cases: (1)
  `t1304_v3_chained_fetch_populates_view_header` — happy path;
  asserts populated `description == "buy 0.4 BTCUSDT @ 52341.20"`
  and `strategy_id == Some(StrategyId::new("sma-cross-btc-1m"))` on
  the chain output; replaces the T1206 `SmolStr::default()` /
  `None` defaults; verifies `view.ts == fill.venue_ts` (no clock
  proxy). (2) `t1304_v3b_unknown_tx_short_circuits_to_error` —
  defensive Q6 None-arm; asserts a bogus UUID short-circuits to
  `Err("{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction")`. The
  test drives the same two-reader sequence the closure invokes
  (iced `Task::perform` runtime is heavyweight in tests); a
  `drive_chain` helper mirrors the closure body byte-for-byte. **No
  new snapshot** per Q5 — the four T1207 modal snaps stay
  byte-identical (`tape_audit_modal_summary` consumes
  `JournalModalState`, not provenance, so the chain edit is
  invisible to the snapshot harness). **Verification gates:**
  `cargo test -p ui --test cockpit_live_modal_metadata_chain` →
  `test result: ok. 2 passed; 0 failed; 0 ignored`; `cargo test -p
  ui --features fixtures` → `panel_snapshots` 36 / 36 PASS
  byte-identical (the four `tape_audit_modal_*` snaps unchanged);
  `cargo build --release --bin cockpit_live --features ui/live`
  clean; `cargo clippy --workspace --all-targets --all-features --
  -D warnings` clean; `cargo fmt --check` clean. **Anchor regression
  gate:** `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 /
  11)`. T1305 unblocked.
  HANDOFF → orchestrator (T1303 + T1304 done; T1305 unblocked).
- 2026-05-03 (tester): **FINAL gate PASS — feature shipped.**
  Independent verification reproduces all five upstream waves clean.
  Static analysis: `cargo fmt --all -- --check` clean, `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` clean.
  Build sweep: `cargo build --workspace --all-targets` clean (34.00s),
  `cargo build --release --bin cockpit_live --features ui/live` clean
  (0.75s), `cargo build -p ui --bin cockpit --features fixtures`
  clean (3.81s, fixtures backwards compat held). Test sweep: `cargo
  test --workspace --all-targets` zero failures across all suites;
  `cargo test --workspace --doc` clean; `cargo test -p ui --features
  live` clean. **V1/V2** `cargo test -p audit --test
  journal_transaction_metadata` 3/0/0 (`t1302_v1_returns_metadata_for_existing_transaction`,
  `t1302_v2_returns_none_for_unknown_tx_id`, `t1302_strategy_id_optional`
  all `... ok`). **V3** `cargo test -p ui --test
  cockpit_live_modal_metadata_chain` 2/0/0
  (`t1304_v3_chained_fetch_populates_view_header`,
  `t1304_v3b_unknown_tx_short_circuits_to_error` both `... ok`).
  **V4** `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)` byte-identical with `spec/anchors.toml`.
  **V5** osr suites green (T802/T805/T806/T809: `pnl_by_strategy`
  4/0/0, `feed_reconnect_test` 2/0/0, `uptime_intervals_test` 6/0/0,
  `kill_switch_dual_write_test` 4/0/0); live-cockpit-unified suites
  green (`agent` lib 33/0/0, `kill_switch_trip_writes_both` 3/0/0,
  `cockpit_live_kill_button_writes_audit` 1/0/0); per-symbol-position-accounts
  suites green (`per_symbol_post_fill` 4/0/0,
  `t1102_per_symbol_post_fill` 2/0/0, `open_positions` 8/0/0);
  tape-row-audit-modal suites green (`tape_row_click_opens_modal`
  8/0/0, `journal_entries_for_transaction` 3/0/0 — T1202 reader
  byte-identical per R7); **panel_snapshots 36/36 byte-identical**
  including the four `tape_audit_modal_*` snaps (Q5 invariant held).
  Tick verification (Phase 2): T1301–T1305 citations all hold under
  independent re-run; no overclaim. T_FINAL_TX_METADATA ticked.
  Frontmatter `status: in-progress → shipped` on both feature + task
  files. Report:
  [`spec/archive/test-2026-05-03-1608-journal-transactions-metadata-final.md (archived; see spec/archive/README.md)`](../reports/test-2026-05-03-1608-journal-transactions-metadata-final.md).
  VERDICT → PASS. Feature ready for presenter.
