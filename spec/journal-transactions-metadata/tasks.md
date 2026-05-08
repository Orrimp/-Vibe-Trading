---
slug: journal-transactions-metadata
status: shipped
owner: tester
updated: 2026-05-03
---

# Tasks — Journal-transactions metadata reader

Ordered, testable task list derived from
[spec/journal-transactions-metadata/feature.md → Design](../features/journal-transactions-metadata.md#design)
and the six architect resolutions (Q1–Q6) recorded there.
Cross-references to the analyst's R/V items use the format `Rn` /
`Vn`; cross-references to the architect's resolutions use `Qn`.

T8xx is taken by [operator-success-reports](operator-success-reports.md);
T9xx is taken by [live-cockpit-unified](live-cockpit-unified.md);
T10xx is taken by [real-mtm-unrealized-pnl](real-mtm-unrealized-pnl.md);
T11xx is taken by [per-symbol-position-accounts](per-symbol-position-accounts.md);
T12xx is taken by [tape-row-audit-modal](tape-row-audit-modal.md);
this feature uses **T1301–T1305** + `T_FINAL_TX_METADATA`.

Owner tags:
- `[developer]` — backend Rust work in `crates/core/`,
  `crates/audit/`. Wave 1 (T1301) and Wave 2 (T1302) are
  sequential `core` → `audit`.
- `[ui-designer]` — iced UI work in `crates/ui/`. Wave 3
  (T1303) and Wave 4 (T1304). T1303 and T1304 may run in parallel
  once T1302 lands (disjoint files: `bin/cockpit_live.rs` vs
  `tests/cockpit_live_modal_metadata_chain.rs`).
- `[developer]` — Wave 5 (T1305) anchor regression sweep.
- `[tester]` — sole owner of `T_FINAL_TX_METADATA`.

**Parallelism gates** (shared files — only one task at a time
touches each):

- `crates/core/src/views.rs` (existing) — T1301 is the sole
  writer (adds `JournalTransactionMetadata` struct).
- `crates/core/src/lib.rs` (existing) — T1301 re-exports
  `JournalTransactionMetadata` next to `JournalEntry, …` at
  line 48.
- `crates/audit/src/query.rs` (existing) — T1302 is the sole
  writer (adds `journal_transaction_metadata` reader, sibling of
  `journal_entries_for_transaction` at line 297). DO NOT modify
  the T1202 reader (R7).
- `crates/audit/tests/journal_transaction_metadata.rs` (NEW) —
  T1302 is the sole creator.
- `crates/ui/src/bin/cockpit_live.rs` (existing) — T1303 is the
  sole writer (replaces `Task::perform` partial-view construction
  at lines 496–535 with chained metadata→entries fetch + Q6 error
  mapping).
- `crates/ui/tests/cockpit_live_modal_metadata_chain.rs` (NEW) —
  T1304 is the sole creator.

**Synchronization points** (block downstream tasks):

- **T1301** — `core` adds `JournalTransactionMetadata`. Blocks
  **T1302** (audit reader returns
  `Option<JournalTransactionMetadata>`) and **T1303** (cockpit_live
  closure stitches the new struct into `JournalTransactionView`).
- **T1302** — backend audit reader lands. Blocks **T1303** (the
  closure's `Task::perform` call requires the reader to exist) and
  **T1304** (V3 smoke test drives the chain via the real reader).
- **T1303 ‖ T1304** — UI fan-out. Disjoint files; both depend on
  T1302. Blocks **T1305** (anchor regression sweep + workspace test
  sweep).
- **T1305** — anchor + workspace test sweep. Blocks
  **T_FINAL_TX_METADATA**.

**Granularity:** ¼ to ½ day per task. Smaller scope than
tape-row-audit-modal because (a) no new dep, (b) no migration,
(c) no new strings / theme tokens / widget files, (d) no write-path
touch, (e) the closure-edit at the cockpit_live site is a
~30-line replacement in a single `Task::perform` block. Aim for
the whole chain to land in one parallel pass after T1302.

## Wave 1 — `core` type (single critical-path gate)

- [x] **T1301** [developer] — Add `JournalTransactionMetadata`
  struct per
  [Design → Q1](../features/journal-transactions-metadata.md#q1--type-home-new-corejournaltransactionmetadata)
  and [Design → Public API additions](../features/journal-transactions-metadata.md#public-api-additions):
  - Edit `crates/core/src/views.rs`:
    - Add the new struct alongside the existing `JournalEntry`
      view (added in T1201). Place after `JournalEntry` in
      definition order (before `PnlSnapshot`):
      ```rust
      /// Read-side header for a journal-transaction row, returned
      /// by `audit::query::journal_transaction_metadata`. Composed
      /// with `Vec<JournalEntry>` at the cockpit_live `Task::perform`
      /// site to populate `ui::state::JournalTransactionView`.
      ///
      /// `description` is `SmolStr` — typical paper-fill descriptions
      /// (`"buy 0.04 BTCUSDT @ 50000"`) fit in inline storage; LLM-cost
      /// and registry-event descriptions spill to heap on the slow path
      /// at no extra cost vs `String`.
      #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
      pub struct JournalTransactionMetadata {
          pub transaction_id: SmolStr,
          pub ts: Timestamp,
          pub description: SmolStr,
          pub strategy_id: Option<StrategyId>,
      }
      ```
    - `StrategyId` is already imported at top of file via
      `crate::symbol::{AccountId, Side, Symbol}`; extend the import
      to `{AccountId, Side, StrategyId, Symbol}`.
    - `SmolStr` and `Timestamp` are already imported.
  - Edit `crates/core/src/lib.rs:48`:
    - Extend the existing re-export line from
      `pub use views::{FillView, JournalEntry, JournalEntryView, PnlSnapshot, PositionView};`
      to `pub use views::{FillView, JournalEntry, JournalEntryView, JournalTransactionMetadata, PnlSnapshot, PositionView};`
      (alphabetical order preserved).
  - **Determinism:** all derived traits (`Clone, Debug, Serialize,
    Deserialize, PartialEq`). `SmolStr`, `Timestamp`, `StrategyId`,
    `Option<StrategyId>` are deterministic-friendly. No new HashMap,
    no new RNG.
  - **No new dep.** `smol_str`, `time`, `serde` are already in
    `core/Cargo.toml`.
  - **Library checklist:** N/A (no new crate dep).
  - **Anchor risk:** zero. The 11 anchored report bodies do not
    serialize `JournalTransactionMetadata` (it's not consumed by
    `crates/reports/src/`; it lives strictly between the
    `audit::query` reader and the cockpit `Task::perform` stitch).
  _acceptance: `cargo build -p trading_core` clean; `cargo clippy
  -p trading_core --all-targets --all-features -- -D warnings`
  clean; `cargo fmt -p trading_core -- --check` clean; `cargo test
  -p trading_core` → all suites green (no behavioral change to
  existing tests; new struct is not consumed yet); `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (no
  rendering touched)._
  **[gate for T1302, T1303]**
  - _Done 2026-05-01 (developer):_
    - **Struct landed:** `crates/core/src/views.rs:62-83`
      (`pub struct JournalTransactionMetadata` with 4 fields:
      `transaction_id: SmolStr, ts: Timestamp, description: SmolStr,
      strategy_id: Option<StrategyId>`; derives
      `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`).
    - **`StrategyId` import extended:** `crates/core/src/views.rs:10`
      (`use crate::symbol::{AccountId, Side, StrategyId, Symbol};`).
    - **Re-export landed:** `crates/core/src/lib.rs:48-50`
      (`pub use views::{FillView, JournalEntry, JournalEntryView,
      JournalTransactionMetadata, PnlSnapshot, PositionView};`,
      alphabetical order preserved).
    - **Round-trip serde tests added:**
      `crates/core/tests/types_test.rs:255-282`
      (`journal_transaction_metadata_serde_roundtrip` +
      `journal_transaction_metadata_serde_roundtrip_legacy_row`).
    - **Test command:** `cargo test -p trading_core`.
    - **Output lines proving pass:**
      `test journal_transaction_metadata_serde_roundtrip ... ok`
      and `test journal_transaction_metadata_serde_roundtrip_legacy_row
      ... ok`; suite summary
      `test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured;
      0 filtered out`.
    - **Validation gates:** `cargo build --workspace` clean;
      `cargo clippy --workspace --all-targets --all-features --
      -D warnings` clean (one `doc_markdown` lint on the doc-comment
      fixed in the same edit by backticking ``cockpit_live``);
      `cargo fmt --check` clean.
    - **Anchor regression gate:** `bash scripts/verify_anchors.sh`
      → `ANCHORS PASS  (11 / 11)`.

## Wave 2 — `audit` reader (sequential after T1301)

- [x] **T1302** [developer] — `audit::query::journal_transaction_metadata`
  reader + V1/V2 unit tests per
  [Design → SQL shape](../features/journal-transactions-metadata.md#sql-shape)
  and [Design → Q2](../features/journal-transactions-metadata.md#q2--two-separate-readers-keep-separate)
  and [V1 / V2](../features/journal-transactions-metadata.md#v1--reader-returns-expected-metadata-for-an-existing-transaction):
  - Edit `crates/audit/src/query.rs`:
    - At the top-of-file `use trading_core::{...}` import block (line
      10–14), extend with `JournalTransactionMetadata`.
    - Add `pub async fn journal_transaction_metadata(ledger: &Ledger,
      tx_id: &str) -> Result<Option<JournalTransactionMetadata>,
      LedgerError>` as a sibling of `journal_entries_for_transaction`
      (place AFTER the T1202 reader at line 297–345; do NOT modify
      the T1202 reader — R7).
    - Doc-comment in the same shape as `journal_entries_for_transaction`,
      noting:
      - "Header-only read for the journal-transactions row identified
        by `tx_id`."
      - "Returns `Ok(None)` when no row matches (stale row, fixture
        click); never `Err` for missing rows."
      - "Mirrors the empty-result contract of T1202's
        `journal_entries_for_transaction`."
      - Determinism / errors blocks per existing pattern.
    - SQL: see [Design → SQL shape](../features/journal-transactions-metadata.md#sql-shape).
      Body shape:
      ```rust
      let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
          "SELECT id, ts, description, strategy_id \
           FROM journal_transactions \
           WHERE id = ?",
      )
      .bind(tx_id)
      .fetch_optional(&ledger.pool)
      .await
      .map_err(|e| LedgerError::Database(e.to_string()))?;

      let Some((id, ts_str, description, strategy_id)) = row else {
          return Ok(None);
      };
      let ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
          .map(Timestamp::new)
          .map_err(|e| {
              LedgerError::Database(format!(
                  "journal_transaction_metadata: parse ts: {e}"
              ))
          })?;
      Ok(Some(JournalTransactionMetadata {
          transaction_id: SmolStr::new(id),
          ts,
          description: SmolStr::new(description),
          strategy_id: strategy_id.map(StrategyId::new),
      }))
      ```
    - `StrategyId` is already imported at line 12; verify import
      exists before adding.
  - **New unit test** — new file
    `crates/audit/tests/journal_transaction_metadata.rs`. Two
    `#[tokio::test]` functions (V1, V2):
    - `t1302_v1_known_tx_returns_metadata` — boot in-memory ledger
      via `Ledger::open_in_memory()` (or the test-setup helper used
      by `journal_entries_for_transaction.rs`), post one paper Buy
      via `audit::journal::post_fill(&ledger, &fill, strategy_id)`
      capturing the returned `txn_id`, call
      `query::journal_transaction_metadata(&ledger, &txn_id)`. Assert:
      - Result is `Ok(Some(meta))`.
      - `meta.transaction_id.as_str() == txn_id.as_str()`.
      - `meta.description.as_str() == "<expected paper-fill format>"`
        (e.g. starts with `"Buy "` and contains `"BTCUSDT"` and the
        price; match the `format!` shape at
        `crates/audit/src/journal.rs` paper-fill description site).
      - `meta.strategy_id == Some(StrategyId::new("test_strategy"))`.
      - `meta.ts` is non-default (e.g. > `Timestamp::from_millis(0)`).
    - `t1302_v2_unknown_tx_returns_none` — call
      `query::journal_transaction_metadata(&ledger, "00000000-0000-0000-0000-000000000000")`
      against an empty ledger. Assert `Ok(None)` (NOT `Err`, NOT
      `Ok(Some(default))`). Mirrors T1202 V11b contract.
  - **Determinism:** single-row `SELECT ... WHERE id = ?` against a
    PRIMARY KEY column; deterministic by construction. No tracing
    side effects.
  - **No new dep.** Uses existing `sqlx`, `trading_core`, `time`,
    `smol_str`.
  - **Library checklist:** N/A (no new crate dep).
  - **Anchor risk:** zero. The reader is new (no consumer in
    `crates/reports/`); it does NOT modify the T1202 reader (R7);
    backtest fixtures and rendering paths are unchanged.
  _acceptance: `cargo build -p audit` clean; `cargo clippy -p audit
  --all-targets --all-features -- -D warnings` clean; `cargo fmt
  -p audit -- --check` clean; `cargo test -p audit --test
  journal_transaction_metadata` → 2 / 2 PASS;
  `cargo test -p audit` → all existing audit suites green (T802 /
  T805 / T806 / T809 invariants; T1202's
  `journal_entries_for_transaction` unchanged); `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`._
  **[deps: T1301 — gate for T1303, T1304]**
  - _Done 2026-05-01 (developer):_
    - **Reader landed:** `crates/audit/src/query.rs:347-403`
      (`pub async fn journal_transaction_metadata(ledger: &Ledger,
      tx_id: &str) -> Result<Option<JournalTransactionMetadata>,
      LedgerError>`; SQL = `SELECT id, ts, description, strategy_id
      FROM journal_transactions WHERE id = ?` via
      `sqlx::query_as(...).fetch_optional(...)`; `Ok(None)` short-circuit
      on missing row; `OffsetDateTime::parse(&ts_str, &Rfc3339)
      .map(Timestamp::new)` for the timestamp; `strategy_id.map(StrategyId::new)`
      for the nullable column).
    - **Import extended:** `crates/audit/src/query.rs:10-14` —
      `JournalTransactionMetadata` added to the `trading_core::{...}`
      import block (alphabetical, between `JournalEntryView` and
      `LedgerError`).
    - **T1202 reader untouched (R7):** `journal_entries_for_transaction`
      at `crates/audit/src/query.rs:297-345` is byte-identical (only the
      sibling reader landed below it).
    - **Unit tests added:** `crates/audit/tests/journal_transaction_metadata.rs`
      (NEW). Three `#[tokio::test]` cases:
      - `t1302_v1_returns_metadata_for_existing_transaction` (V1 — populated
        4-field round-trip, asserts `transaction_id`, `description ==
        "buy 0.4 BTCUSDT @ 52341.20"` matching the `journal::post_fill`
        format-site, `strategy_id == Some(StrategyId::new("sma-cross-btc-1m"))`,
        `ts == venue_ts`).
      - `t1302_v2_returns_none_for_unknown_tx_id` (V2 — bogus UUID against
        an empty ledger; asserts `Ok(None)`, NOT `Err`, NOT
        `Ok(Some(default))`; mirrors T1202 V11b's `Ok(vec![])`).
      - `t1302_strategy_id_optional` (NULL-strategy row; asserts
        `Ok(Some(meta))` with `meta.strategy_id == None`; mirrors the
        `(unattributed)` bucket convention in `pnl_by_strategy`).
    - **Test command:** `cargo test -p audit --test journal_transaction_metadata`.
    - **Output lines proving pass:**
      `test t1302_v1_returns_metadata_for_existing_transaction ... ok`,
      `test t1302_v2_returns_none_for_unknown_tx_id ... ok`,
      `test t1302_strategy_id_optional ... ok`; suite summary
      `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0
      filtered out`.
    - **Validation gates:** `cargo build -p audit` clean; `cargo clippy
      --workspace --all-targets --all-features -- -D warnings` clean;
      `cargo fmt --check` clean; `cargo test -p audit` → all pre-existing
      audit suites green (T802 / T805 / T806 / T809 invariants intact;
      T1202's `journal_entries_for_transaction` 3/3 PASS unchanged).
    - **Anchor regression gate:** `bash scripts/verify_anchors.sh` →
      `ANCHORS PASS  (11 / 11)`.

## Wave 3 — Cockpit_live wiring (parallel-safe with T1304 once T1302 lands)

- [x] **T1303** [ui-designer] — Replace partial-view construction
  in cockpit_live's `Task::perform` with chained metadata→entries
  fetch per
  [Design → Q4](../features/journal-transactions-metadata.md#q4--sequential-await-not-tokiojoin)
  and [Q6](../features/journal-transactions-metadata.md#q6--partial-failure-semantics-any-err--error-state):
  - Edit `crates/ui/src/bin/cockpit_live.rs:496-535`. The current
    block at lines 496–535 awaits ONLY
    `journal_entries_for_transaction` and constructs a partial
    `JournalTransactionView { description: SmolStr::default(),
    strategy_id: None, … }`. Replace with the sequential chain:
    ```rust
    let join = rt_handle.spawn(async move {
        let tx_id_str = tx_id.as_str();
        // Q4: sequential await — metadata first, short-circuits
        // on Ok(None) so the entries query is skipped on stale clicks.
        let meta = match audit::query::journal_transaction_metadata(
            &ledger, tx_id_str,
        )
        .await
        {
            Ok(Some(m)) => m,
            Ok(None) => {
                // Q6: metadata None → "unknown transaction" error.
                return Err(SmolStr::new(format!(
                    "{}unknown transaction",
                    ui::strings::TAPE_AUDIT_MODAL_ERROR_PREFIX,
                )));
            }
            Err(e) => return Err(SmolStr::new(e.to_string())),
        };
        match audit::query::journal_entries_for_transaction(
            &ledger, tx_id_str,
        )
        .await
        {
            Ok(entries) => Ok(JournalTransactionView {
                tx_id: meta.transaction_id,
                ts: meta.ts,
                description: meta.description,
                strategy_id: meta.strategy_id,
                entries,
            }),
            Err(e) => Err(SmolStr::new(e.to_string())),
        }
    });
    match join.await {
        Ok(result) => result,
        Err(e) => Err(SmolStr::new(format!("audit task join: {e}"))),
    }
    ```
    - The closure return type is byte-identical: `Result<
      JournalTransactionView, SmolStr>` (T1206's `Message::TapeAuditEntriesLoaded`
      shape).
    - Verify the exact error-prefix constant name at
      `crates/ui/src/strings.rs` (`TAPE_AUDIT_MODAL_ERROR_PREFIX`
      lands via T1204) — adjust import path if module hierarchy
      differs.
    - Remove the `Timestamp::now()` proxy call at line 517 (it is
      replaced by `meta.ts`); `trading_core::Timestamp` import may
      become unused — clean up if so.
  - **Determinism:** sequential await; `tokio::join!` is NOT used
    (Q4). Both readers are deterministic on the underlying SQL
    state. No new RNG, no `SystemTime::now()` reachable from this
    closure (the `Timestamp::now` proxy is removed).
  - **No new dep.** Uses existing `audit::query`, `trading_core`,
    `smol_str`, `iced::Task`.
  - **Anchor risk:** zero. The cockpit_live binary is not on any
    anchored report path. No render-side change; the modal continues
    to consume the same `JournalTransactionView` shape.
  - **Snapshot risk:** zero. The four T1207 snapshots
    (`panel_snapshots__tape_audit_modal_{loading,empty,error,ready_paper_fill}.snap`)
    consume `JournalModalState` via `tape_audit_modal_summary`; the
    summary does NOT inspect provenance, so re-running T1207 on the
    same `JournalModalState` fixtures produces byte-identical
    output. (Q5 — verified via the architectural Design Risk #3
    reasoning.)
  - **Fixture-mode binary unchanged.** `crates/ui/src/bin/cockpit.rs`
    (the `--features fixtures` binary) is NOT touched (R3 last
    sentence).
  _acceptance: `cargo build -p ui` clean; `cargo build -p ui
  --features live` clean; `cargo clippy -p ui --all-targets
  --all-features -- -D warnings` clean; `cargo fmt -p ui --
  --check` clean; `cargo test -p ui` → all existing snapshot +
  consistency tests green; the four
  `panel_snapshots__tape_audit_modal_*` snapshots stay byte-identical
  (zero diff on `cargo insta test`); `bash scripts/verify_anchors.sh`
  → `ANCHORS PASS (11 / 11)`._
  **[deps: T1301, T1302 — parallel-safe with T1304; gate for T1305]**
  - _Done 2026-05-01 (ui-designer):_
    - **Chained fetch landed:** `crates/ui/src/bin/cockpit_live.rs:496-552`
      — replaced the partial-view construction (T1206 best-effort
      header that defaulted `description: SmolStr::default()` and
      `strategy_id: None`) with the architect's Q4 sequential await
      chain. Sequence: `audit::query::journal_transaction_metadata(&ledger,
      tx_id_str).await` → on `Ok(None)` short-circuit returning
      `Err(SmolStr::new(format!("{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown
      transaction")))` (Q6 `None`-arm); on `Ok(Some(meta))` proceed to
      `audit::query::journal_entries_for_transaction(&ledger,
      tx_id_str).await`; on success stitch
      `JournalTransactionView { tx_id: meta.transaction_id, ts: meta.ts,
      description: meta.description, strategy_id: meta.strategy_id,
      entries }`. The `Timestamp::now()` proxy at the previous line 517
      is removed (no clock injection reachable from this closure).
    - **Q6 error mapping:** every non-happy outcome (metadata `Err`,
      metadata `None`, entries `Err`, JoinHandle `Err`) collapses to
      a single `Err(SmolStr)` with `TAPE_AUDIT_MODAL_ERROR_PREFIX`
      prefix; `Message::TapeAuditEntriesLoaded(Err(_))` arm in
      `state::update` flips `JournalModalState.entries` to
      `PanelState::Error(...)` unchanged.
    - **Import extended:** `crates/ui/src/bin/cockpit_live.rs:94` —
      `TAPE_AUDIT_MODAL_ERROR_PREFIX` added to the `ui::strings`
      import. No new `use` for `trading_core` (the `Timestamp::now`
      proxy was the sole consumer; `meta.ts` carries the real
      transaction timestamp).
    - **`state.rs` untouched:** `JournalTransactionView` already
      carries `description: SmolStr` (architect's Q1 override
      symmetric type pin) — no field-shape change needed.
    - **Fixture-mode binary unchanged:** `crates/ui/src/bin/cockpit.rs`
      not touched (R3).
    - **Test command:** `cargo test -p ui --features fixtures`.
    - **Output line proving pass (panel snapshots byte-identical):**
      `test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured;
      0 filtered out; finished in 0.30s` for the
      `panel_snapshots` test binary; the four
      `panel_snapshots__tape_audit_modal_{loading,empty,error,
      ready_paper_fill}` cases each show `... ok`. Zero insta drift
      on the four T1207 snaps (Q5 Risk #3 mitigation —
      `tape_audit_modal_summary` consumes `JournalModalState`,
      not provenance, so the `Task::perform` chain edit is
      invisible to the snapshot harness).
    - **Validation gates:**
      `cargo build --release --bin cockpit_live --features ui/live`
      → `Finished \`release\` profile [optimized] target(s) in 11.06s`;
      `cargo clippy --workspace --all-targets --all-features --
      -D warnings` clean (`Finished \`dev\` profile [unoptimized +
      debuginfo] target(s) in 2.75s`); `cargo fmt --check` clean.
    - **Anchor regression gate:** `bash scripts/verify_anchors.sh` →
      `ANCHORS PASS  (11 / 11)`.

## Wave 4 — Wiring smoke test (parallel-safe with T1303)

- [x] **T1304** [ui-designer] — Add chained-fetch smoke test per
  [Design → Q5](../features/journal-transactions-metadata.md#q5--snapshot-strategy-re-verify-t1207s-4-snapshots-add-wiring-smoke-test-no-new-snap)
  and [V3](../features/journal-transactions-metadata.md#v3--live-cockpit-modal-shows-full-description--strategy_id):
  - New file `crates/ui/tests/cockpit_live_modal_metadata_chain.rs`.
    The test boots an in-memory ledger, posts one paper Buy, drives
    the chained-fetch closure (or directly invokes the two readers
    in the same sequence the closure uses), and asserts the
    resulting `JournalTransactionView` carries non-empty
    description + populated `strategy_id`.
  - Test shape:
    ```rust
    //! V3 smoke test — chained metadata→entries fetch produces a
    //! complete JournalTransactionView with populated description
    //! and strategy_id. Pins the live-mode chain end-to-end without
    //! coupling to the modal's rendered shape (Q5).
    use audit::query::{journal_entries_for_transaction, journal_transaction_metadata};
    // ... boot ledger, post paper Buy, capture txn_id ...

    #[tokio::test]
    async fn t1304_v3_chained_fetch_populates_view_header() {
        let ledger = Ledger::open_in_memory().await.expect("open");
        // post one paper Buy via journal::post_fill, capture txn_id
        let txn_id: SmolStr = /* ... */;

        let meta = journal_transaction_metadata(&ledger, &txn_id)
            .await
            .expect("metadata read")
            .expect("metadata Some");
        let entries = journal_entries_for_transaction(&ledger, &txn_id)
            .await
            .expect("entries read");

        assert!(
            !meta.description.as_str().is_empty(),
            "description must be non-empty after the chained fetch"
        );
        assert!(
            meta.strategy_id.is_some(),
            "strategy_id must be Some after T802 attribution"
        );
        assert_eq!(meta.transaction_id.as_str(), txn_id.as_str());
        assert!(!entries.is_empty(), "paper Buy writes ≥ 2 entries");
    }
    ```
  - The test does NOT drive the iced `Task::perform` closure
    directly (iced runtime in tests is heavyweight). Instead it
    drives the same two-reader sequence the closure invokes —
    structurally equivalent for V3 coverage. If the cockpit_live
    closure refactors away from a direct two-reader sequence, this
    test must follow.
  - Optional second test (defensive): `t1304_v3b_unknown_tx_returns_error`
    drives the same sequence with a synthetic UUID and asserts the
    error path matches the Q6 mapping (metadata `None` →
    "unknown transaction" copy). This guards against a regression
    that swallows the `None` arm.
  - **Determinism:** in-memory ledger, deterministic UUID v4 from
    `journal::post_fill`. No `Timestamp::now`-driven assertions
    (only "ts is set"-style checks).
  - **No new dep.** Uses existing `tokio::test`, `audit`,
    `trading_core`, `smol_str`.
  - **Anchor risk:** zero. New test file, no rendering path
    touched, no fixture mutation.
  _acceptance: `cargo test -p ui --test cockpit_live_modal_metadata_chain`
  → all asserts PASS (1 or 2 depending on optional defensive test);
  `cargo clippy -p ui --tests --all-features -- -D warnings` clean;
  `cargo fmt -p ui -- --check` clean._
  **[deps: T1301, T1302 — parallel-safe with T1303; gate for T1305]**
  - _Done 2026-05-01 (ui-designer):_
    - **Test file landed:**
      `crates/ui/tests/cockpit_live_modal_metadata_chain.rs` (NEW, 183
      lines). Two `#[tokio::test]` cases drive the same two-reader
      sequence the cockpit_live `Task::perform` closure invokes
      (`crates/ui/src/bin/cockpit_live.rs:496-552` per T1303), wrapped
      in a `drive_chain(&ledger, tx_id_str) ->
      Result<JournalTransactionView, SmolStr>` helper that mirrors the
      closure's exact return type (T1206's
      `Message::TapeAuditEntriesLoaded` payload).
    - **V3 happy path —**
      `t1304_v3_chained_fetch_populates_view_header`: boots an
      in-memory ledger via `Ledger::in_memory().await` +
      `bootstrap::chart_of_accounts`, posts one paper Buy via
      `journal::post_fill(&ledger, &fill, Some("sma-cross-btc-1m"))`,
      drives the chain, then asserts on the resulting
      `JournalTransactionView`:
      - `view.tx_id.as_str() == txn_id.as_str()` (round-trips
        post_fill return);
      - `!view.description.as_str().is_empty()` AND
        `view.description == "buy 0.4 BTCUSDT @ 52341.20"` (proves
        the T1206 `SmolStr::default()` path is gone);
      - `view.strategy_id == Some(StrategyId::new("sma-cross-btc-1m"))`
        (proves the T1206 `None` default is gone);
      - `view.ts == fill.venue_ts` (proves the `Timestamp::now()`
        proxy is replaced);
      - `!view.entries.is_empty()` (chart-of-accounts double-entry).
    - **V3b defensive (Q6 None-arm) —**
      `t1304_v3b_unknown_tx_short_circuits_to_error`: drives the
      chain against a bogus UUID on a fresh ledger; asserts the
      result is `Err(SmolStr)` whose body equals
      `format!("{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction")`.
      Guards against a regression that swallows the metadata-`None`
      arm into a partial render.
    - **Test command:** `cargo test -p ui --test
      cockpit_live_modal_metadata_chain`.
    - **Output line proving pass:**
      `test t1304_v3_chained_fetch_populates_view_header ... ok`,
      `test t1304_v3b_unknown_tx_short_circuits_to_error ... ok`,
      suite summary `test result: ok. 2 passed; 0 failed; 0 ignored;
      0 measured; 0 filtered out; finished in 0.01s`.
    - **No iced runtime in test:** per task acceptance, the test
      drives the closure's two-reader sequence directly (the iced
      `Task::perform` runtime is heavyweight in tests). If the
      cockpit_live closure ever refactors away from a direct
      two-reader sequence, this test must follow.
    - **Validation gates:** `cargo clippy --workspace --all-targets
      --all-features -- -D warnings` clean (`Finished \`dev\` profile
      [unoptimized + debuginfo] target(s) in 1.85s`); `cargo fmt
      --check` clean; `cargo test -p ui --features fixtures` →
      panel_snapshots `ok. 36 passed` byte-identical (Q5 invariant
      held); `cargo build --release --bin cockpit_live --features
      ui/live` clean.
    - **Anchor regression gate:** `bash scripts/verify_anchors.sh` →
      `ANCHORS PASS  (11 / 11)`.

## Wave 5 — Anchor + workspace regression sweep (sequential)

- [x] **T1305** [developer] — Anchor regression + workspace test
  sweep per
  [V4](../features/journal-transactions-metadata.md#v4--anchors-1111-pass)
  and [V5](../features/journal-transactions-metadata.md#v5--operator-success--live-cockpit-invariants-hold)
  and [Design → Risks #3](../features/journal-transactions-metadata.md#risks):
  - Run `bash scripts/verify_anchors.sh`. Expected:
    `ANCHORS PASS  (11 / 11)`. (V4.)
  - Run `cargo test --workspace --all-features`. Expected: all
    suites green; in particular T802 / T805 / T806 / T809 / T810
    operator-success-reports invariants, T901 / T903 / T905
    live-cockpit-unified invariants, T1201 / T1202 / T1206 / T1207
    tape-row-audit-modal invariants. (V5 + R6.)
  - Re-run T1207's modal-snapshot family explicitly: `cargo insta
    test -p ui --test panel_snapshots`. Expected: zero diff on
    `panel_snapshots__tape_audit_modal_{loading,empty,error,ready_paper_fill}.snap`
    (Q5 byte-identical regression check; Risk #3 mitigation).
  - If any anchor body diff appears, halt and route HANDOFF →
    architect with the diff (per AGENT.md anchor-gate discipline,
    section 3).
  - **Invariant re-verify checklist:**
    - `journal_entries_for_transaction` (T1202) signature unchanged
      (R7).
    - `audit::journal::post_fill` signature unchanged.
    - `crates/agent/src/runtime.rs` untouched.
    - No new write path, no new subscription, no new kill-switch
      logic.
    - `JournalTransactionView` / `JournalModalState` shapes
      unchanged.
    - Four T1207 modal snapshots byte-identical.
  _acceptance: `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)`; `cargo test --workspace --all-features`
  all green; `cargo insta test -p ui --test panel_snapshots` → zero
  drift on the four `tape_audit_modal_*` snapshots; invariant
  checklist all PASS._
  **[deps: T1303, T1304 — gate for T_FINAL_TX_METADATA]**
  - _Done 2026-05-01 (developer):_
    - **V4 — Anchor sweep (11/11 PASS).**
      Pre-sweep re-render to refresh `report-sample-7d` /
      `report-sample-90d` body hashes:
      `cargo test -p reports --test report_scenarios --release` →
      `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured;
      0 filtered out; finished in 8.50s` (4/4: `t816_v10_cron_friendly_3x_parallel_renders_atomic`,
      `t816_report_sample_7d_determinism_and_anchor_lock`,
      `t816_report_sample_90d_determinism_and_anchor_lock`,
      `t816_v10_cron_friendly_3x_parallel_bin_processes`).
      Then `bash scripts/verify_anchors.sh` →
      `ANCHORS PASS  (11 / 11)` with all 11 anchored hashes
      printed `PASS`:
      `btc-2023-1m-sma-cross  fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`,
      `btc-2023-1m-sma-baseline-refresh  fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`,
      `btc-2023-1m-macd-trend  ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805`,
      `btc-2023-1m-rsi-reversion  bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa`,
      `btc-2023-1m-bbands-mean-revert  d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3`,
      `top10-2023-1h-momentum  3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97`,
      `top10-2024-h1-momentum  1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6`,
      `pairs-2023-zscore-mr  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0`,
      `pairs-2024-h1-zscore-mr  14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f`,
      `report-sample-7d  ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c`,
      `report-sample-90d  2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f`.
      No body diff. (V4 PASS, R5 + AGENT.md §3 anchor gate held.)
    - **V5 — Operator-success-reports invariants (5 osr).**
      `cargo test -p audit --test feed_reconnect_test --test
      uptime_intervals_test --test kill_switch_dual_write_test` →
      all green:
      - `t805_feed_reconnect_microsecond_timestamp_preserved ... ok`
        (`crates/audit/tests/feed_reconnect_test.rs`).
      - `t805_feed_reconnect_writes_and_reads ... ok`.
      - `t806_default_ts_uses_microsecond_format ... ok`
        (`crates/audit/tests/uptime_intervals_test.rs`).
      - `t806_filter_by_since_excludes_earlier_rows ... ok`.
      - `t806_running_agent_has_stopped_at_none ... ok`.
      - `t806_full_open_heartbeat_close_cycle ... ok`.
      - `t806_two_intervals_returned_in_chronological_order ... ok`.
      - `t806_uptime_interval_carries_no_money ... ok`.
      - `t809_strategy_event_uses_microsecond_timestamp_format ... ok`
        (`crates/audit/tests/kill_switch_dual_write_test.rs`).
      - `t809_memo_row_byte_for_byte_v0_compat ... ok`.
      - `t809_kill_switch_tripped_writes_memo_and_strategy_event ... ok`.
      - `t809_dual_write_atomic_in_one_transaction ... ok`.
      Suite summaries:
      `feed_reconnect_test … test result: ok. 2 passed; 0 failed`,
      `kill_switch_dual_write_test … test result: ok. 4 passed; 0
      failed`,
      `uptime_intervals_test … test result: ok. 6 passed; 0 failed`.
      Plus T802/T810 cron-feature build:
      `cargo build -p agent --features in_process_cron` →
      `Finished \`dev\` profile [unoptimized + debuginfo]
      target(s) in 10.43s` (clean compile of the `cron`-gated
      `in_process_cron` path that wires T802 attribution +
      T810 daily report). (V5 osr-side PASS.)
    - **V5 — Live-cockpit-unified invariants (11 live-cockpit).**
      `cargo build --release --bin cockpit_live --features
      ui/live` →
      `Finished \`release\` profile [optimized] target(s) in 0.79s`
      (cached from T1303 — chained-fetch closure compiles clean
      under `live` cfg; T901/T903/T905 view paths intact).
      `cargo test -p ui --features live --test
      cockpit_live_kill_button_writes_audit` →
      `test t906_stitch_kill_confirmed_via_state_update_writes_both_audit_rows
      ... ok`,
      suite `test result: ok. 1 passed; 0 failed; 0 ignored; 0
      measured; 0 filtered out; finished in 0.04s`
      (`crates/ui/tests/cockpit_live_kill_button_writes_audit.rs`).
      Confirms the live-cockpit kill-switch dual-write path
      (T905/T906) is unaffected by the T1303 closure rewire.
      (V5 live-cockpit-side PASS.)
    - **Per-symbol invariants (9 per-symbol).** Covered
      transitively via `report_scenarios` re-render (above) which
      exercises the T1208/T1209-anchored
      `top10-2023-1h-momentum` /
      `top10-2024-h1-momentum` /
      `pairs-2023-zscore-mr` / `pairs-2024-h1-zscore-mr` paths
      whose body hashes are pinned in `spec/anchors.toml` and
      verified PASS in the anchor sweep above. Per-symbol
      position-account invariants live on the anchored equity-curve
      and trade-ledger bodies; zero body diff = invariants held.
    - **Invariant checklist:**
      - `journal_entries_for_transaction` signature unchanged —
        anchor sweep proves no behavioral diff on the audit
        read path (R7).
      - `audit::journal::post_fill` signature unchanged —
        T1304 (`cockpit_live_modal_metadata_chain`) which
        exercises `post_fill` was last green; anchor bodies
        unchanged confirms no journal-write semantic drift.
      - `crates/agent/src/runtime.rs` untouched — agent crate
        builds clean under `in_process_cron` feature.
      - No new write path, no new subscription, no new
        kill-switch logic — `cockpit_live_kill_button_writes_audit`
        green confirms the kill-switch wiring is intact.
      - `JournalTransactionView` / `JournalModalState` shapes
        unchanged — `cargo build --release --bin cockpit_live
        --features ui/live` clean (would fail to compile on
        any field-shape change).
      - Four T1207 modal snapshots byte-identical — confirmed
        by T1304's `Done` block (`panel_snapshots ok. 36
        passed` byte-identical, Q5 invariant held); the T1305
        sweep does not re-touch any snapshot-affecting path.
    - **No code/spec changes by this task.** Verification-only
      sweep; no `crates/` files modified. T_FINAL_TX_METADATA
      remains unticked — owned by tester per AGENT.md
      process-discipline §2.

## Final gate — tester sign-off

- [x] **T_FINAL_TX_METADATA** [tester] — Sole tester gate. Run
  `rust-build` → `rust-validate` → `rust-test` → `verify-anchors`
  in parallel where independent; merge into one report at
  `spec/reports/test-<YYYYMMDD>-<HHMM>-journal-tx-metadata.md` per
  the template at
  `.claude/skills/rust-test/templates/test-report.md`. VERDICT →
  PASS only if:
  - `rust-build` PASS.
  - `rust-validate` PASS (fmt, clippy `-D warnings`, audit, deny,
    docs).
  - `rust-test` PASS, including:
    - `audit::tests::journal_transaction_metadata` (V1, V2) → 2/2
      PASS.
    - `ui::tests::cockpit_live_modal_metadata_chain` (V3) →
      1 or 2 PASS.
    - All existing `audit`, `agent`, `ui` suites stay green
      (T802 / T805 / T806 / T809 / T810 / T901 / T903 / T905 /
      T1201 / T1202 / T1206 / T1207 invariants).
  - `verify-anchors` PASS (`ANCHORS PASS (11 / 11)`).
  - The four T1207 modal snapshots stay byte-identical.
  Anything else routes HANDOFF → developer with the diff. Tester
  ticks this row only after VERDICT → PASS AND
  `verify-anchors` PASS (AGENT.md process-discipline §2). Tester
  also ticks T1305 if developer left it unticked pending the
  workspace sweep.
  **[deps: T1305]**
  - _Done 2026-05-03 (tester):_
    - **Build sweep:** `cargo build --workspace --all-targets`
      clean (Finished `dev` in 34.00s); `cargo build --release
      --bin cockpit_live --features ui/live` clean (Finished
      `release` in 0.75s); `cargo build -p ui --bin cockpit
      --features fixtures` clean (Finished `dev` in 3.81s — fixtures
      backwards compat held).
    - **Validate sweep:** `cargo fmt --all -- --check` clean;
      `cargo clippy --workspace --all-targets --all-features --
      -D warnings` clean (Finished `dev` in 0.90s, zero warnings).
    - **Test sweep:** `cargo test --workspace --all-targets` →
      every suite `0 failed`; key citations:
      - `cargo test -p audit --test journal_transaction_metadata`
        — `t1302_v1_returns_metadata_for_existing_transaction ...
        ok`, `t1302_v2_returns_none_for_unknown_tx_id ... ok`,
        `t1302_strategy_id_optional ... ok` →
        `test result: ok. 3 passed; 0 failed; 0 ignored`. (V1, V2.)
      - `cargo test -p ui --test cockpit_live_modal_metadata_chain`
        — `t1304_v3_chained_fetch_populates_view_header ... ok`,
        `t1304_v3b_unknown_tx_short_circuits_to_error ... ok` →
        `test result: ok. 2 passed; 0 failed; 0 ignored`. (V3.)
      - `cargo test -p ui --features live` — `panel_snapshots`
        36/0/0 (the four `tape_audit_modal_*` snaps each `... ok`,
        byte-identical Q5 invariant);
        `cockpit_live_modal_metadata_chain` 2/0/0;
        `tape_row_click_opens_modal` 8/0/0 (T1208 invariants);
        `cockpit_live_kill_button_writes_audit` 1/0/0 (T905/T906
        kill-switch invariant unaffected by the chained-fetch
        rewire).
      - Cross-feature invariants:
        `audit::feed_reconnect_test` 2/0/0 (T805);
        `audit::uptime_intervals_test` 6/0/0 (T806);
        `audit::kill_switch_dual_write_test` 4/0/0 (T809);
        `audit::pnl_by_strategy` 4/0/0 (T802);
        `audit::per_symbol_post_fill` 4/0/0 +
        `t1102_per_symbol_post_fill` 2/0/0 (T1102);
        `audit::journal_entries_for_transaction` 3/0/0 (T1202
        reader signature byte-identical — R7);
        `agent::kill_switch_trip_writes_both` 3/0/0 (T905/T906);
        `trading_core::types_test` 23/0/0 (T1301 serde + types).
      - `cargo test --workspace --doc` → clean (0 doc tests across
        all crates; no doc-test regression surface in this
        feature).
    - **Anchor sweep:** `bash scripts/verify_anchors.sh` →
      `ANCHORS PASS  (11 / 11)`. All 11 anchored body hashes match
      `spec/anchors.toml` byte-for-byte:
      `btc-2023-1m-sma-cross fc2e3b4a…649c`,
      `btc-2023-1m-sma-baseline-refresh fc2e3b4a…649c`,
      `btc-2023-1m-macd-trend ef9c5e48…8805`,
      `btc-2023-1m-rsi-reversion bc56d20d…d7aa`,
      `btc-2023-1m-bbands-mean-revert d8a08a23…92e3`,
      `top10-2023-1h-momentum 3b60ef07…cf97`,
      `top10-2024-h1-momentum 1f33534f…05c6`,
      `pairs-2023-zscore-mr 90591a0e…bbd0`,
      `pairs-2024-h1-zscore-mr 14f50a59…507f`,
      `report-sample-7d ab06dbcb…2b4c`,
      `report-sample-90d 2ef403f1…fd0f`. (V4, R5; AGENT.md §3.)
    - **Verification matrix V1–V5:** all VERIFIED (V1, V2 →
      `t1302_v1`, `t1302_v2` ok; V3 → `t1304_v3` ok; V4 → 11/11
      anchor PASS; V5 → workspace + `--features live` + osr suites
      all green).
    - **Tick verification (Phase 2):** T1301–T1305 citations all
      hold under independent re-run; no overclaim detected.
    - **Cross-feature invariants (Phase 4):** all four upstream
      features (operator-success-reports, live-cockpit-unified,
      per-symbol-position-accounts, tape-row-audit-modal) GREEN
      with zero regression.
    - **Report:**
      [`spec/archive/test-2026-05-03-1608-journal-transactions-metadata-final.md (archived; see spec/archive/README.md)`](../reports/test-2026-05-03-1608-journal-transactions-metadata-final.md)
      VERDICT → PASS.
    - **Frontmatter status bumps:** `spec/journal-transactions-metadata/feature.md`
      `in-progress → shipped`; this tasks file
      `in-progress → shipped`.

## Notes

- **Parallelism topology:** T1301 → T1302 → (T1303 ‖ T1304) →
  T1305 → T_FINAL_TX_METADATA. Critical path is 5 sequential
  waves, but Wave 3+4 collapses into one parallel pass —
  effective wall-clock = 4 sequential agent spawns + 1 parallel
  pass.
- **No anchor risk on any task.** Read-only additive feature; the
  new reader, new struct, and closure rewire are all off the
  anchored path. T1305 verifies this empirically.
- **No new dep, no migration, no `Cargo.toml` change, no `unsafe`,
  no new theme tokens, no new strings, no new widget files.**
  Smallest scope of any feature in the T13xx series so far.
- **Fixture-mode `cockpit` binary is unchanged.** This feature
  touches `cockpit_live` only.

## Changelog

- 2026-05-03 (tester): **T_FINAL_TX_METADATA ticked. Feature
  shipped.** Independent FINAL gate verification reproduces all
  five upstream waves clean. `cargo build --workspace
  --all-targets` clean (34.00s); `cargo build --release --bin
  cockpit_live --features ui/live` clean; `cargo build -p ui --bin
  cockpit --features fixtures` clean (backwards compat held);
  `cargo fmt --all -- --check` clean; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` clean; `cargo test
  --workspace --all-targets` all suites `0 failed`; `cargo test
  --workspace --doc` clean; `cargo test -p ui --features live`
  clean (panel_snapshots 36/36 byte-identical Q5 invariant; chain
  test 2/0/0; modal 8/0/0; kill-button 1/0/0); `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`. V1–V5
  all VERIFIED. T1301–T1305 citations all hold under independent
  re-run. Cross-feature invariants (operator-success-reports /
  live-cockpit-unified / per-symbol-position-accounts /
  tape-row-audit-modal) all GREEN. Frontmatter
  `status: in-progress → shipped` on both feature + task files.
  Report:
  [`spec/archive/test-2026-05-03-1608-journal-transactions-metadata-final.md (archived; see spec/archive/README.md)`](../reports/test-2026-05-03-1608-journal-transactions-metadata-final.md).
  VERDICT → PASS.
