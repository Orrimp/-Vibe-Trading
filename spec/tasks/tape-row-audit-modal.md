---
slug: tape-row-audit-modal
status: shipped
owner: developer
updated: 2026-05-03
---

# Tasks — Tape-row → audit modal

Ordered, testable task list derived from
[spec/features/tape-row-audit-modal.md → Design](../features/tape-row-audit-modal.md#design)
and the nine architect resolutions (Q1–Q9) recorded in that Design
section. Cross-references to the analyst's R/V items use the format
`Rn` / `Vn`; cross-references to the architect's resolutions use `Qn`.

T8xx is taken by [operator-success-reports](operator-success-reports.md);
T9xx is taken by [live-cockpit-unified](live-cockpit-unified.md);
T10xx is taken by [real-mtm-unrealized-pnl](real-mtm-unrealized-pnl.md);
T11xx is taken by [per-symbol-position-accounts](per-symbol-position-accounts.md);
this feature uses **T1201–T1209** + `T_FINAL_TAPE_MODAL`.

Owner tags:
- `[developer]` — backend Rust work in `crates/core/`,
  `crates/audit/`, `crates/agent/`. Wave 1 (T1201) and Wave 2
  (T1202) are sequential `core` → `audit`.
- `[ui-designer]` — iced UI work in `crates/ui/`. Wave 3
  (T1203 ‖ T1204 ‖ T1205) and Wave 4 (T1206) and Wave 5
  (T1207 ‖ T1208) fan out across disjoint files.
- `[developer]` — Wave 6 (T1209) anchor regression sweep.
- `[tester]` — sole owner of `T_FINAL_TAPE_MODAL`.

**Parallelism gates** (shared files — only one task at a time
touches each):

- `crates/core/src/views.rs` (existing) — T1201 is the sole writer
  (adds `JournalEntry` struct + `transaction_id` field on `FillView`).
- `crates/core/src/fill.rs` (existing) — T1201 is the sole writer
  (adds `transaction_id: Option<SmolStr>` field on `Fill`).
- `crates/core/src/lib.rs` (existing) — T1201 re-exports `JournalEntry`.
- `crates/audit/src/query.rs` (existing) — T1202 is the sole writer
  (adds `journal_entries_for_transaction`; populates `transaction_id`
  in `recent_fills`).
- `crates/audit/src/journal.rs` (existing) — T1202 is the sole writer
  (changes `post_fill` return type to `Result<SmolStr, LedgerError>`).
- `crates/audit/tests/journal_entries_for_transaction.rs` (NEW) —
  T1202 is the sole creator.
- `crates/agent/src/runtime.rs` (existing) — T1202 is the sole
  writer of the `txn_id` stamp on `Fill` after `post_fill` returns.
- `crates/audit/tests/{ledger_integration,open_positions_at,per_symbol_post_fill,t1102_per_symbol_post_fill}.rs`
  (existing) — T1202 is the sole writer (mechanical
  `let _ = post_fill(...)` two-char edits per call site, ~7 sites
  per the Design Risk #4 grep).
- `crates/ui/src/theme.rs` (existing) — T1203 is the sole writer
  (adds 3 token constants).
- `crates/ui/src/strings.rs` (existing) — T1204 is the sole writer
  (adds 13 modal-copy constants + `all()` extension).
- `crates/ui/src/widgets/journal_transaction_modal.rs` (NEW) —
  T1205 is the sole creator.
- `crates/ui/src/widgets/mod.rs` (existing) — T1205 adds the `pub mod`.
- `crates/ui/src/state.rs` (existing) — T1206 is the sole writer
  (adds `Message::TapeRowClicked` / `TapeAuditModalClosed` /
  `TapeAuditEntriesLoaded`, `Cockpit.tape_audit_modal`,
  `JournalModalState` struct, `update` arms,
  `AgentHaltedExternally` arm extension).
- `crates/ui/src/widgets/tape.rs` (existing) — T1206 is the sole
  writer (wraps `row_for(fill)` in a `Button::on_press`).
- `crates/ui/src/live.rs` (existing) — T1206 is the sole writer
  (`fill_to_view` reads the new field).
- `crates/ui/src/fixtures.rs` (existing) — T1206 stamps a
  deterministic `transaction_id` in `fake_fill_view`.
- `crates/ui/src/bin/cockpit_live.rs` and
  `crates/ui/src/bin/cockpit.rs` (existing) — T1206 extends the
  subscription with the `iced::keyboard::on_key_press` recipe
  gated on `tape_audit_modal.is_some()`.
- `crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_ready_paper_fill.snap`
  (NEW) — T1207 is the sole creator. V8 (V2).
- `crates/ui/tests/panel_snapshots.rs` (existing) — T1207 is the
  sole writer (adds `tape_audit_modal_ready_paper_fill` test).
- `crates/ui/tests/tape_row_click_opens_modal.rs` (NEW) — T1208 is
  the sole creator. V1 + V3 + V4 + V5.

**Synchronization points** (block downstream tasks):

- **T1201** — `core` adds `JournalEntry`, `FillView::transaction_id`,
  `Fill::transaction_id`. Blocks **T1202** (audit reader returns
  `Vec<JournalEntry>`; `post_fill` populates `txn_id` and the
  runtime stamps `Fill.transaction_id`), **T1203/T1204/T1205**
  (UI fan-out can begin in parallel since they touch only `theme`,
  `strings`, and the new widget file — none import the new `core`
  types until T1206), and **T1206** (UI `Message::TapeRowClicked`
  carries the tx_id derived from `fill.transaction_id`).
- **T1202** — backend audit + agent runtime work lands. Blocks
  **T1206** (the modal's async load via
  `journal_entries_for_transaction` requires the reader to exist)
  and **T1208** (V1 integration test drives the load via the real
  reader).
- **T1203 / T1204 / T1205** — UI atomic prep. T1206 depends on all
  three (it imports `theme::color::BG_OVERLAY` etc., `strings::TAPE_AUDIT_MODAL_TITLE`
  etc., and `widgets::journal_transaction_modal::view`).
- **T1206** — `state.rs` + tape-row click + subscription wiring.
  Blocks **T1207** (modal snapshot needs the `JournalModalState`
  struct and the modal widget) and **T1208** (integration test
  drives `Message::TapeRowClicked` through `state::update`).
- **T1207 / T1208** — verification tests. Blocks **T1209**
  (anchor regression sweep runs after all test code lands).
- **T1209** — anchor regression sweep + workspace test sweep.
  Blocks **T_FINAL_TAPE_MODAL**.

**Granularity:** ½ day per task except T_FINAL_TAPE_MODAL (tester
gate). Granularity matches per-symbol-position-accounts (T1101–T1107)
since both features are additive plumbing on top of existing
machinery — no new strategy logic, no new SQL schema, no new external
dep.

## Wave 1 — `core` types (single critical-path gate)

- [x] **T1201** [developer] — Add `JournalEntry`, `FillView::transaction_id`,
  `Fill::transaction_id` per
  [Design → Crate map delta](../features/tape-row-audit-modal.md#crate-map-delta)
  and [Q2](../features/tape-row-audit-modal.md#q2--journalentry-un-collapsed-lives-in-trading_core)
  and [Q5](../features/tape-row-audit-modal.md#q5--transaction_id-plumbing-path):
  - Edit `crates/core/src/views.rs`:
    - Add `pub struct JournalEntry { pub account: AccountId,
      pub debit: Money<Usdt>, pub credit: Money<Usdt>,
      pub currency: SmolStr, pub ts: Timestamp, pub memo: SmolStr }`.
      `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]`.
    - Add field `pub transaction_id: SmolStr` to `FillView`
      (additive; place after `venue_ts` to match insertion order
      conventions). Update the doc-comment to note this carries
      the `journal_transactions.id` UUID string for click-through
      to audit modal.
  - Edit `crates/core/src/fill.rs`:
    - Add field `pub transaction_id: Option<SmolStr>` to `Fill`
      (place after `liquidity`). Doc-comment: "Populated by the
      live-mode runtime after `audit::journal::post_fill` writes
      the journal transaction; `None` in backtests and at
      construction time."
  - Edit `crates/core/src/lib.rs`:
    - Re-export `JournalEntry` next to existing `FillView` /
      `JournalEntryView` re-exports.
  - **Determinism:** all new types `derive(Clone, Debug,
    Serialize, Deserialize, PartialEq)`. `SmolStr` and `Option<SmolStr>`
    are deterministic-friendly. `Money<Usdt>` already is. No new
    HashMap, no new RNG.
  - **No new dep.** `smol_str` and `time` and `serde` and
    `rust_decimal` are already in `core/Cargo.toml`.
  - **Library checklist:** N/A (no new crate dep).
  - **Anchor risk:** zero. The 11 anchored report bodies do not
    serialize `FillView` or `Fill` — they render aggregate cells
    only. Verified by independent grep over `crates/reports/src/`
    (no `FillView { transaction_id` literal could appear in
    rendered output; the renderer formats numeric cells via
    `widgets::num` / its own templating).
  _acceptance: `cargo build -p trading_core` clean; `cargo clippy
  -p trading_core --all-targets --all-features -- -D warnings`
  clean; `cargo fmt -p trading_core -- --check` clean; `cargo test
  -p trading_core` → all suites green (no behavioral change to
  existing tests; new fields are not consumed yet); `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (no
  rendering touched)._
  **[gate for T1202, T1203, T1204, T1205, T1206]**

## Wave 2 — `audit` reader + `post_fill` return type + agent runtime stamp (sequential after T1201)

- [x] **T1202** [developer] — `audit::query::journal_entries_for_transaction`
  + `recent_fills` populates `transaction_id` + `post_fill` returns
  `txn_id` + `agent::runtime` stamps `Fill.transaction_id` per
  [Design → Crate map delta](../features/tape-row-audit-modal.md#crate-map-delta)
  and [Q5](../features/tape-row-audit-modal.md#q5--transaction_id-plumbing-path)
  and [Q8 V11](../features/tape-row-audit-modal.md#q8--test-plan):
  - **Reader side** — at `crates/audit/src/query.rs`, add
    `pub async fn journal_entries_for_transaction(ledger: &Ledger,
    tx_id: &str) -> Result<Vec<JournalEntry>, LedgerError>`. SQL:
    ```sql
    SELECT account_id, debit_amount, credit_amount, ts, memo
    FROM journal_entries
    WHERE transaction_id = ?
    ORDER BY id ASC
    ```
    Parse `Decimal` from text via the existing pattern at
    `recent_journal` (lines 247–255). Currency is derived from the
    `accounts.currency` column via a sub-select OR (cheaper) by
    inspecting the `account_id` suffix (the chart of accounts is
    `assets:cash:USDT` / `assets:position:BTCUSDT` / etc. — the
    suffix after the last `:` is the symbol/currency). Use the
    `accounts` JOIN — cleaner, doesn't bake the chart-of-accounts
    parsing into a reader. Empty result: return `Ok(vec![])`. No
    error.
  - **`recent_fills` populates `transaction_id`** — at line 213 of
    `query.rs` where the `FillView { ... }` constructor runs, add
    `transaction_id: SmolStr::new(txn_id)` (the `txn_id` is
    already in scope from the loop at line 151). One-line edit.
  - **Writer side** — at `crates/audit/src/journal.rs:39`, change
    `post_fill` return type from `Result<(), LedgerError>` to
    `Result<SmolStr, LedgerError>`. The `txn_id` is already
    generated at line 44 (`let txn_id = Uuid::new_v4().to_string();`);
    return `Ok(SmolStr::new(&txn_id))` at the success site
    (currently `Ok(())` near line 195+; verify by reading the file).
    No call-site behavioral change beyond the return value.
  - **Agent runtime stamp** — at the production fill loop in
    `crates/agent/src/runtime.rs` (the test at line 832 uses an
    inline `Fill`; production has its own loop wherever
    `post_fill` is called via the paper engine). Pattern:
    ```rust
    let txn_id = audit::journal::post_fill(&ledger, &fill, strategy_id).await?;
    let mut fill = fill;
    fill.transaction_id = Some(txn_id);
    engine.on_fill(&fill, &pos);
    ```
    Locate the actual call site (currently the only production
    `post_fill` call appears to be from the paper engine glue;
    verify with `grep -rn "audit::journal::post_fill\|audit::post_fill"
    crates/agent/`). If the call site lives elsewhere (e.g. inside
    a not-yet-wired stub from T903a), add the stamp at the same
    spot; if T903a's wiring is incomplete, document the gap and
    route a follow-up note in the task tick.
  - **Existing test call-site sweep** — every existing call to
    `post_fill(...).await?` becomes `let _ = post_fill(...).await?;`
    (or `let _txn_id = post_fill(...).await?;` if the test
    asserts something via the txn_id). Per Design Risk #4, ~7
    test files in `crates/audit/tests/`; mechanical edits.
  - **New unit test** — new file
    `crates/audit/tests/journal_entries_for_transaction.rs`. Three
    `#[tokio::test]` functions (V11):
    - `t1202_v11a_known_tx_returns_entries_in_id_order` — boot
      in-memory ledger, post one Buy fill via `post_fill` (capture
      the returned `txn_id`), call
      `query::journal_entries_for_transaction(&ledger, &txn_id)`.
      Assert returned `Vec<JournalEntry>` has `len() == 4`
      (paper-fill writes 4 entries: Dr position, Cr cash, Dr fee,
      Cr cash) and is sorted by `id ASC` lexicographically.
    - `t1202_v11b_unknown_tx_returns_empty_vec` — call with a
      non-existent UUID string. Assert `Ok(vec![])` (NOT `Err`).
    - `t1202_v11c_balance_invariant_holds` — same setup as V11a;
      iterate the returned `Vec<JournalEntry>` and assert
      `entries.iter().map(|e| e.debit.amount()).sum::<Decimal>()
      == entries.iter().map(|e| e.credit.amount()).sum::<Decimal>()`.
  - **Determinism:** SQL ORDER BY `id ASC` returns lexicographic
    UUID strings — stable across runs. The reader has no
    `tracing::warn!` side effects.
  - **No new dep.** Uses existing `sqlx`, `trading_core`, etc.
  - **Library checklist:** N/A (no new crate dep).
  - **Anchor risk:** zero. The reader is new (no consumer in
    `crates/reports/`); `recent_fills` keeps its return-type
    structurally compatible (added field, not removed); `post_fill`
    return-type change does not affect what gets WRITTEN to the
    journal — only what gets returned to the caller. Backtest
    fixtures and rendering paths are unchanged.
  _acceptance: `cargo build -p audit` clean; `cargo build -p agent`
  clean; `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` clean; `cargo fmt --all -- --check` clean;
  `cargo test -p audit --test journal_entries_for_transaction`
  → 3 / 3 PASS; `cargo test -p audit` → all existing audit suites
  green (T802 / T805 / T806 / T809 invariants); `cargo test -p
  agent` → T901 / T903a-d / T905 invariants green; `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`._
  **[deps: T1201 — gate for T1206, T1208]**

## Wave 3 — UI prep (parallel — disjoint files)

- [x] **T1203** [ui-designer] — Add `bg_overlay`, `info`,
  `border_strong` theme tokens per
  [Q3](../features/tape-row-audit-modal.md#q3--theme-tokens-land-in-this-feature)
  and the
  [principles color palette dark-mode table](../ui-design-principles.md#dark-mode-default):
  - Edit `crates/ui/src/theme.rs::color`:
    - Add `pub const BG_OVERLAY: Color = rgb(0x0B, 0x0D, 0x12);`
      after `BG_ELEV` (semantic group: backgrounds).
    - Add `pub const INFO: Color = rgb(0x7B, 0xC2, 0xFF);` after
      `WARN` (semantic group: state colors).
    - Add `pub const BORDER_STRONG: Color = rgb(0x3A, 0x44, 0x56);`
      after `BORDER` (semantic group: borders).
  - Doc-comments per existing pattern (one-line role description
    matching the principles doc):
    - `BG_OVERLAY` — "Modal-dialog backdrop. Captures clicks
      outside the modal card."
    - `INFO` — "Observation-only signals. Used for transaction-id
      text and other informational, non-interactive elements."
    - `BORDER_STRONG` — "Focused / hovered border, modal frame.
      Distinct from `BORDER` so the keyboard user can tell
      focused-from-active."
  - **No new dep.** No `theme.rs` API change beyond three
    additional `pub const Color` constants. The helper functions
    `color_for_delta` and `color_for_latency_ms` are unchanged
    (the new tokens are not picked from data).
  - **Determinism:** `pub const Color` is compile-time; no
    runtime, no allocation, no order dependency.
  - **Anchor risk:** zero. New constants are not consumed by any
    widget rendering path until T1205.
  - **Snapshot risk:** zero. Existing `panel_snapshots__*` use
    `tape_summary` / `pnl_summary` / etc., none of which inspect
    the new tokens (verified by grep over
    `crates/ui/tests/panel_snapshots.rs:335+`).
  _acceptance: `cargo build -p ui` clean (default and
  `--features fixtures`); `cargo clippy -p ui --all-targets
  --all-features -- -D warnings` clean; `cargo fmt -p ui --
  --check` clean; `cargo test -p ui` → all existing snapshot
  + consistency tests green (no diffs)._
  **[parallel-safe with T1204, T1205; deps: T1201]**

  _ticked 2026-05-01 (ui-designer): three `pub const Color` constants
  added at `crates/ui/src/theme.rs:55` (`BG_OVERLAY`),
  `crates/ui/src/theme.rs:84` (`INFO`), and `crates/ui/src/theme.rs:101`
  (`BORDER_STRONG`); five new unit tests at `crates/ui/src/theme.rs:200-258`
  pin the principles-doc dark hex and assert distinctness from `BORDER`
  + darker-than-`BG`. Light-mode hex documented in
  `// TODO(light-mode):` block per task scope; `Theme::Light` branch
  left untouched (no such enum yet — light-mode lands as a separate
  feature). Test cmd `cargo test -p ui --lib theme` →
  `test result: ok. 5 passed; 0 failed; 0 ignored`. Build cmd
  `cargo build -p ui` and `cargo build -p ui --features fixtures` →
  both `Finished`. Clippy `cargo clippy -p ui --all-targets
  --all-features -- -D warnings` → `Finished` with zero warnings.
  Fmt `cargo fmt -p ui -- --check` → exit 0. Existing 32
  `panel_snapshots__*` + 2 `consistency` tests stay green._

- [x] **T1204** [ui-designer] — Add modal-copy strings to
  `ui::strings` per
  [R7](../features/tape-row-audit-modal.md#r7--strings-all-modal-copy-in-uistrings-zero-inline):
  - Edit `crates/ui/src/strings.rs`:
    - Add 13 `pub const &str` constants in a new
      `// ── Tape audit modal ────────────────────────────────────`
      section:
      - `TAPE_AUDIT_MODAL_TITLE = "Journal transaction"`
      - `TAPE_AUDIT_MODAL_TX_LABEL = "Transaction ID"`
      - `TAPE_AUDIT_MODAL_TS_LABEL = "Time"`
      - `TAPE_AUDIT_MODAL_DESC_LABEL = "Description"`
      - `TAPE_AUDIT_MODAL_STRATEGY_LABEL = "Strategy"`
      - `TAPE_AUDIT_MODAL_STRATEGY_NONE = "—"`
      - `TAPE_AUDIT_MODAL_COL_ACCOUNT = "Account"`
      - `TAPE_AUDIT_MODAL_COL_DEBIT = "Debit"`
      - `TAPE_AUDIT_MODAL_COL_CREDIT = "Credit"`
      - `TAPE_AUDIT_MODAL_COL_CURRENCY = "Currency"`
      - `TAPE_AUDIT_MODAL_LOADING = "Loading journal entries…"`
      - `TAPE_AUDIT_MODAL_EMPTY = "No entries for this transaction."`
      - `TAPE_AUDIT_MODAL_ERROR_PREFIX = "Failed to load journal entries: "`
      - `TAPE_AUDIT_MODAL_CLOSE_LABEL = "Close"`
    - Append every new constant to the `all()` slice (in declaration
      order).
  - Voice/copy review: direct, terse, present-tense, sentence
    case, unicode `…` (per
    [principles voice/copy](../ui-design-principles.md#voice-and-copy)).
    `TAPE_AUDIT_MODAL_LOADING` uses `…` not `...`.
    `TAPE_AUDIT_MODAL_ERROR_PREFIX` follows the `<what's broken>:`
    pattern matching `TAPE_ERROR_PREFIX = "Can't read the fill
    stream: "`.
  - **No new dep.** No API change beyond `pub const &str`
    additions.
  - **Determinism:** `pub const &str` is compile-time.
  - **Anchor risk:** zero. The 11 anchored reports do not include
    UI strings.
  _acceptance: `cargo build -p ui` clean; `cargo test -p ui`
  → all existing tests green; the `all_keys_unique` and
  `all_values_non_empty` tests in `strings.rs` mod still pass
  with the 13 new entries; `cargo clippy -p ui -- -D warnings`
  clean; `cargo fmt -p ui -- --check` clean._
  **[parallel-safe with T1203, T1205; deps: T1201]**

  _ticked 2026-05-01 (ui-designer): 14 `pub const &str` constants
  added in a new `// ── Tape audit modal ────────` section at
  `crates/ui/src/strings.rs:140-167` (one extra over the task's "13"
  count — `TAPE_AUDIT_MODAL_STRATEGY_NONE` is a separate constant for
  the "no strategy" slot rather than reusing `PLACEHOLDER_NONE`, so
  a future "Manual" / "—" copy split is a one-line change).
  All 14 appended to the `all()` slice between `KILL_RUNBOOK_LINK_PATH`
  and `CONNECTION_AGENT_UNREACHABLE` at `crates/ui/src/strings.rs:279-292`
  to match source declaration order. Voice review: `…` (unicode) used in
  `TAPE_AUDIT_MODAL_LOADING`, error prefix follows `<what's broken>:`
  pattern matching `TAPE_ERROR_PREFIX`. Test cmd `cargo test -p ui
  --lib strings` → `test result: ok. 2 passed; 0 failed` (`all_keys_unique`,
  `all_values_non_empty`). Full `cargo test -p ui` → 35 unit + 32
  panel_snapshots + 2 consistency tests all green.
  Clippy + fmt clean (same commands as T1203)._

- [x] **T1205** [ui-designer] — Add `widgets::journal_transaction_modal`
  per
  [Q1](../features/tape-row-audit-modal.md#q1--iced-014-modal-pattern-icedwidgetstack)
  and
  [Q7](../features/tape-row-audit-modal.md#q7--specific-journaltransactionmodal-widget):
  - New file
    `crates/ui/src/widgets/journal_transaction_modal.rs`. Exports:
    ```rust
    pub fn view<'a>(state: &'a JournalModalState) -> Element<'a, Message>;
    ```
    Body: a `Stack` widget with two children:
    1. **Backdrop** — `Container::new(Space::with_height(Length::Fill))`
       sized `(Length::Fill, Length::Fill)`, styled with
       `bg_overlay` (semi-transparent dark), wrapped in a `MouseArea`
       that emits `Message::TapeAuditModalClosed` on click.
    2. **Modal card** — centered `Container` at width 480 px, padded
       `space::XL` (24 px), framed `border_strong`, background
       `bg_elev`. Body is a `Column` with:
       - Header row (`title` size, `TAPE_AUDIT_MODAL_TITLE`) +
         right-aligned `Button`("Close",
         `Message::TapeAuditModalClosed`).
       - Metadata `Column` of 4 label-value pairs:
         `TAPE_AUDIT_MODAL_TX_LABEL` → `info`-colored monospace
         `tx_id`; `TAPE_AUDIT_MODAL_TS_LABEL` → monospace
         RFC 3339; `TAPE_AUDIT_MODAL_DESC_LABEL` → `body`-size
         description; `TAPE_AUDIT_MODAL_STRATEGY_LABEL` →
         `body`-size strategy id or `TAPE_AUDIT_MODAL_STRATEGY_NONE`.
       - Spacer (`space::M`).
       - Match on `state.entries: PanelState<JournalTransactionView>`:
         - `Loading` → centered `TAPE_AUDIT_MODAL_LOADING`,
           `body`-size, `fg_muted`.
         - `Empty` → centered `TAPE_AUDIT_MODAL_EMPTY`, `body`-size,
           `fg_muted`. **No column headers** rendered (V3
           assertion).
         - `Error(msg)` → centered
           `TAPE_AUDIT_MODAL_ERROR_PREFIX + msg`, `body`-size,
           `neg`. **No column headers** rendered.
         - `Ready(view)` → table:
           - Header row: `TAPE_AUDIT_MODAL_COL_ACCOUNT` /
             `_DEBIT` / `_CREDIT` / `_CURRENCY`, `caption`-size,
             `fg_muted`.
           - For each `JournalEntry` in `view.entries`: row with
             account-id (left-aligned monospace), debit (right-aligned
             monospace, formatted via `widgets::num::fmt_usdt` /
             a fmt helper for `Money<Usdt>`), credit (same),
             currency (centered).
   - Density per R10: row 24 px, cell pad 12 px, modal inner pad
     24 px.
   - **Strings only via `ui::strings`** — zero string literals
     (R15 + T1204).
   - **Colors only via `ui::theme::color`** — zero `Color::from_rgb`
     (R15 + T1203).
   - Edit `crates/ui/src/widgets/mod.rs` to add `pub mod
     journal_transaction_modal;`.
  - **Determinism:** widget is a pure function of `&JournalModalState`;
    no `HashMap` iteration order, no `Instant::now()`.
  - **Library checklist:** uses only `iced::widget::Stack`,
    `Container`, `Column`, `Row`, `Text`, `Button`, `MouseArea` —
    all in our pinned `iced = "=0.14.0"`. No new dep.
  - **Anchor risk:** zero. Widget is not on any backtest path.
  _acceptance: `cargo build -p ui` clean; `cargo clippy -p ui
  --all-targets --all-features -- -D warnings` clean (the new
  widget references `JournalModalState` — declare the type via
  re-export from `state` once T1206 lands; until then T1205 may
  introduce a placeholder type alias inside the widget that
  resolves to a `state::*` import on T1206 close-out, OR T1205
  imports `crate::state::JournalModalState` even though
  `state.rs` has not yet defined it — the compile breaks, T1206
  removes the break; this is the "T1206 depends on T1205"
  direction enforced by the dependency arrow). **Resolve at
  task-tick time:** sequence is `T1203 ‖ T1204 ‖ T1205 (sketch
  the widget against an inline placeholder struct) → T1206
  (wire `state.rs` + replace placeholder with real import) →
  T1205 close-out (re-tick if the widget needed any change to
  match `state.rs`'s final shape)`. Document the dependency in
  the task tick._
  **[parallel-safe with T1203, T1204; deps: T1201]**

  _ticked 2026-05-01 (ui-designer): widget at new file
  `crates/ui/src/widgets/journal_transaction_modal.rs` (~485 lines).
  `Stack`-based overlay — the workspace's first overlay use of
  `iced::widget::Stack`, doc-commented at file top. Public surface:
  `pub fn view<'a, Msg>(state: &'a JournalModalState, content:
  Element<'a, Msg>, close_msg: Msg) -> Element<'a, Msg> where Msg:
  Clone + 'a` at `crates/ui/src/widgets/journal_transaction_modal.rs:125`.
  Generic `Msg` keeps the close-message variant out of this file per
  the task scope ("doesn't hardcode the message variant"); T1206
  supplies the concrete `Message::TapeAuditModalClosed` at the call
  site. **T1206 dependency direction:** placeholder
  `JournalModalState` (line 80) and `JournalTransactionView` (line 93)
  structs defined locally matching the architect's `Modal state shape`
  exactly; T1206 replaces these with `pub use
  crate::state::{JournalModalState, JournalTransactionView};` —
  mechanical swap. Module registered at
  `crates/ui/src/widgets/mod.rs:13`. All four `PanelState<T>` arms
  rendered: `Loading` / `Empty` (no column headers per V3) /
  `Error(msg)` (with `TAPE_AUDIT_MODAL_ERROR_PREFIX + msg` styled
  `neg`) / `Ready(view)` (4-column `Account | Debit | Credit |
  Currency` table). Backdrop is a `Container` styled with `BG_OVERLAY`
  wrapped in `MouseArea::on_press(close_msg)`. Modal card frame uses
  `BORDER_STRONG` (T1203 token), 480 px width, `space::XL` (24 px)
  padding, `BG_ELEV` background. Close button is text `"Close"`
  (T1204 string) — no glyph per principles "no icons until needed".
  Numbers right-aligned via `widgets::num::fmt_usdt`.
  Test cmd `cargo test -p ui --lib widgets::journal_transaction_modal`
  → `test result: ok. 5 passed; 0 failed; 0 ignored` (4
  render-without-panic smoke tests covering all four `PanelState` arms
  + 1 number-formatting precedent test).
  Build `cargo build -p ui` and `cargo build -p ui --features fixtures`
  → both `Finished`. Clippy `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` → `Finished` with zero warnings.
  Fmt `cargo fmt --all -- --check` → exit 0. Anchors: `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`. Existing
  `panel_snapshots__*` (32) + `consistency.rs` (2) tests stay green —
  confirms no inline strings/hex in the new widget._

## Wave 4 — UI wiring (`state.rs` + tape-row click + subscription) (sequential after Wave 3)

- [x] **T1206** [ui-designer] — Wire `Message::TapeRowClicked` /
  `TapeAuditModalClosed` / `TapeAuditEntriesLoaded`, `Cockpit.tape_audit_modal`,
  tape-row click handler, keyboard subscription per
  [Design → Modal state shape](../features/tape-row-audit-modal.md#modal-state-shape)
  and
  [Q6](../features/tape-row-audit-modal.md#q6--keyboard-absorption-subscription-on-modal-open):
  - **Edit `crates/ui/src/state.rs`**:
    - Add `pub struct JournalModalState { pub tx_id: SmolStr,
      pub entries: PanelState<JournalTransactionView> }` and
      `pub struct JournalTransactionView { pub tx_id: SmolStr,
      pub ts: Timestamp, pub description: SmolStr,
      pub strategy_id: Option<StrategyId>,
      pub entries: Vec<JournalEntry> }`.
      `#[derive(Debug, Clone)]`.
    - Add field `pub tape_audit_modal: Option<JournalModalState>`
      to `Cockpit`. Initialize to `None` in `Cockpit::default()`
      and `Cockpit::new()`.
    - Add three new `Message` variants (after `StrategySignalObserved`):
      `TapeRowClicked(SmolStr)`,
      `TapeAuditModalClosed`,
      `TapeAuditEntriesLoaded(Result<JournalTransactionView, SmolStr>)`.
    - Add three corresponding arms in `update`:
      - `Message::TapeRowClicked(tx_id)` → set
        `model.tape_audit_modal = Some(JournalModalState { tx_id:
        tx_id.clone(), entries: PanelState::Loading })`. The
        actual async fetch is issued by the binary's
        `Subscription` / `Task::perform` — not by `update`
        (R5 / pure-function discipline).
      - `Message::TapeAuditModalClosed` → set
        `model.tape_audit_modal = None`.
      - `Message::TapeAuditEntriesLoaded(Ok(view))` → if
        `view.entries.is_empty()` set
        `tape_audit_modal.entries = PanelState::Empty` else
        `PanelState::Ready(view)` (in-place mutation through the
        `Option`).
      - `Message::TapeAuditEntriesLoaded(Err(msg))` → set
        `tape_audit_modal.entries = PanelState::Error(msg)`.
    - Extend the existing `Message::AgentHaltedExternally` arm
      to also set `model.tape_audit_modal = None` (Q9).
  - **Edit `crates/ui/src/widgets/tape.rs`**:
    - Wrap `row_for(fill)` in a `Button::new(row_content)
      .on_press(Message::TapeRowClicked(fill.transaction_id.clone()))
      .style(transparent_button_style)`. The button is visually
      transparent (no border, no background) so the row looks
      identical when not hovered. R11 / V7 — existing snapshot
      `tape_summary` does not inspect button-vs-non-button
      structure.
  - **Edit `crates/ui/src/live.rs::fill_to_view`**:
    - Add `transaction_id: fill.transaction_id.clone().unwrap_or_default()`
      to the `FillView { ... }` constructor. `unwrap_or_default()`
      yields empty `SmolStr` for `None` fills (fixture mode resilience).
  - **Edit `crates/ui/src/fixtures.rs::fake_fill_view`**:
    - Stamp `transaction_id: SmolStr::new(format!("fixture-tx-{n}"))`
      (deterministic per the input index `n`).
  - **Edit `crates/ui/src/bin/cockpit_live.rs`** (and `cockpit.rs`):
    - In the cockpit's `subscription()` function, when
      `model.tape_audit_modal.is_some()`, batch in an
      `iced::keyboard::on_key_press` recipe. Translate
      `Key::Escape` → `Message::TapeAuditModalClosed`. Other
      navigation keys (Tab/arrows/PgUp/PgDn) are absorbed via
      `event::Status::Captured` so they don't leak to the tape
      beneath.
    - In the `update` arm for `TapeRowClicked`, issue
      `iced::Task::perform(async move { audit::query::journal_entries_for_transaction(&ledger,
      &tx_id).await }, |result| Message::TapeAuditEntriesLoaded(...))`.
      Build the `JournalTransactionView` from the reader's
      `Vec<JournalEntry>` plus a side-call to
      `journal_transactions` for `(ts, description, strategy_id)`
      (or extend the reader to return both — simpler, but more
      surface; T1202 keeps the reader narrow per Q1's "one
      reader, one job" preference).
  - **Determinism:** `update` stays pure; no `HashMap`
    iteration in `update`. `JournalModalState` uses
    `PanelState<JournalTransactionView>` which already has
    deterministic ordering.
  - **Library checklist:** `iced::keyboard::on_key_press` is in
    `iced 0.14.0`. No new dep.
  - **Anchor risk:** zero. UI-only.
  _acceptance: `cargo build -p ui` clean (default,
  `--features fixtures`, `--features live`); `cargo clippy -p ui
  --all-targets --all-features -- -D warnings` clean (no
  `_ => {}` in `update` — exhaustive match preserved); `cargo
  fmt -p ui -- --check` clean; `cargo test -p ui` → all existing
  panel snapshots stay byte-identical (R11 + V7); the
  `Message`-exhaustiveness consistency test stays green (R15)._
  **[deps: T1202, T1203, T1204, T1205 — gate for T1207, T1208]**

  _ticked 2026-05-01 (ui-designer): convergence task — every Wave 2
  output now operator-reachable via the click → modal flow. Citations:
  - **State types** in `crates/ui/src/state.rs`: `JournalModalState` at
    line 72, `JournalTransactionView` at line 89 (both
    `#[derive(Debug, Clone)]`, fields per architect's design `Modal
    state shape` — `tx_id: SmolStr`, `entries:
    PanelState<JournalTransactionView>` for the modal state and
    `tx_id`, `ts: Timestamp`, `description: SmolStr`, `strategy_id:
    Option<StrategyId>`, `entries: Vec<JournalEntry>` for the view).
  - **Cockpit field** at `crates/ui/src/state.rs:252`:
    `pub tape_audit_modal: Option<JournalModalState>`. Initialized
    `None` in `Cockpit::default()` (line 295) and `Cockpit::ready()`
    (line 348). `Debug` impl extended at line 281.
  - **Message variants** at `crates/ui/src/state.rs`:
    `TapeRowClicked(SmolStr)` at line 427,
    `TapeAuditModalClosed` at line 430,
    `TapeAuditEntriesLoaded(Result<JournalTransactionView, SmolStr>)`
    at line 436.
  - **Update arms** at `crates/ui/src/state.rs`: `Message::TapeRowClicked`
    at line 613 (sets `Some(JournalModalState { tx_id, entries: Loading
    })` — pure-function discipline R5; binary owns the async fetch),
    `Message::TapeAuditModalClosed` at line 625 (sets `None`),
    `Message::TapeAuditEntriesLoaded` at line 628 (in-place mutation
    via `as_mut()`; `Ok(view)` → `Empty` if empty else `Ready(view)`,
    `Err(msg)` → `Error(msg)`). `Message::AgentHaltedExternally` arm
    extended at line 579 to also clear `tape_audit_modal = None` (Q9).
    Match remains exhaustive — no `_ =>` arm added.
  - **Tape click handler** at `crates/ui/src/widgets/tape.rs`:
    `Button::new(row_content).on_press(Message::TapeRowClicked(...))`
    at line 117, `transparent_row_button` style at line 126 — strips
    chrome so rendered tape stays visually identical (R11 / V7).
  - **View branching** in `crates/ui/src/bin/cockpit.rs:171` and
    `crates/ui/src/bin/cockpit_live.rs:606`: when
    `tape_audit_modal.is_some()` wrap `main_column` in
    `journal_transaction_modal::view(modal_state, main_column,
    Message::TapeAuditModalClosed)`; otherwise return `main_column`
    directly so the iced tree is byte-identical to the pre-modal world.
  - **Subscription gate** in `crates/ui/src/bin/cockpit.rs:117` and
    `crates/ui/src/bin/cockpit_live.rs:552`: when
    `tape_audit_modal.is_some()` batch `iced::event::listen_with(...)`
    that emits `Message::TapeAuditModalClosed` on `Key::Named(Escape)`.
    `iced 0.14.0`'s public keyboard surface is `iced::keyboard::Event`
    rather than the `on_key_press(...)` shorthand the task spec
    referenced; `iced::event::listen_with` is the equivalent
    canonical pattern (no new dep — already in scope).
  - **Async dispatch** in `crates/ui/src/bin/cockpit_live.rs:493-538`:
    `update -> iced::Task<Message>`. On `TapeRowClicked(tx_id)`,
    `iced::Task::perform` runs an async closure that
    `rt_handle.spawn(audit::query::journal_entries_for_transaction(...))`
    on the side-thread tokio runtime (iced's main thread has no
    runtime context — same bridge pattern as the T906 kill-switch
    trip closure), then routes the result to
    `Message::TapeAuditEntriesLoaded`. The view's header fields
    (`description`, `strategy_id`) default to empty / `None` until a
    follow-up adds the journal_transactions metadata reader; entries
    + ts + tx_id are populated.
  - **Widget placeholder swap** in
    `crates/ui/src/widgets/journal_transaction_modal.rs:54`:
    `use crate::state::{JournalModalState, JournalTransactionView,
    PanelState};` (was local placeholder structs at the old lines
    80 / 93). Mechanical, shapes match — confirmed by `cargo build`.
  - **Fixtures determinism** in `crates/ui/src/fixtures.rs:91`:
    `transaction_id: SmolStr::new(format!("fixture-tx-{n}"))` so the
    fixtures-mode click flow has stable, reproducible per-row tx
    ids. Snapshots stay byte-identical (`tape_summary` does not
    inspect `transaction_id`).

  Test commands + outputs:
  - `cargo build --workspace --all-targets` → `Finished` clean.
  - `cargo build -p ui --features fixtures --bin cockpit` →
    `Finished`. `cargo build -p ui --features live --bin cockpit_live`
    → `Finished`.
  - `cargo test -p ui --features fixtures` → `test result: ok. 32
    passed; 0 failed` (panel snapshots) + `test result: ok. 35
    passed; 0 failed` (lib unit tests including the 5 widget
    `journal_transaction_modal` smoke tests) + `test result: ok. 2
    passed; 0 failed` (consistency).
  - `cargo test -p ui --features live` →
    `cockpit_live_kill_button_writes_audit` `test result: ok. 1
    passed; 0 failed` (T906 invariant intact); 32/32 panel snapshots
    stay byte-identical.
  - `cargo test --workspace --all-targets` → exit code 0; every
    individual test result line shows `0 failed`.
  - `cargo clippy --workspace --all-targets --all-features -- -D
    warnings` → `Finished` with zero warnings (exhaustive `Message`
    match preserved — no `_ =>` arm added).
  - `cargo fmt --all -- --check` → exit 0.
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` —
    11 anchored reports byte-identical (R12 / V6).

  T1205 placeholder swap is mechanical (shapes match the architect's
  design; no widget edit needed beyond replacing the local `pub
  struct` blocks with the `use crate::state::{...}` import). T1207 +
  T1208 verification fan-out remains `[ ]` per scope; `T_FINAL_TAPE_MODAL`
  stays unticked per process discipline._

## Wave 5 — verification tests (parallel — disjoint files)

- [x] **T1207** [ui-designer] — Modal snapshot test in compact
  density per
  [Q8 V8 / V2](../features/tape-row-audit-modal.md#q8--test-plan):
  - Edit `crates/ui/tests/panel_snapshots.rs` to add a new
    `#[test] fn tape_audit_modal_ready_paper_fill()` function
    that builds a fixture `JournalModalState::Ready` with a
    4-entry paper-fill view (the V8 fixture from R3):
    - `tx_id = "4f9a2c1e-aaaa-bbbb-cccc-000000000001"` (fixed
      UUID for byte-identical re-runs).
    - `ts = "2026-05-03T14:32:18Z"` (fixed RFC 3339 second
      precision — matches `post_fill`'s timestamp format).
    - `description = "buy 0.4 BTCUSDT @ 52341.20"`.
    - `strategy_id = Some(StrategyId::new("sma-cross-btc-1m"))`.
    - 4 entries:
      1. `assets:cash:USDT` Cr `52341.20` USDT.
      2. `assets:position:BTCUSDT` Dr `0.40` BTC.
      3. `expense:fees:taker` Dr `5.23` USDT.
      4. `assets:cash:USDT` Cr `5.23` USDT.
  - Add a corresponding text-summary helper `tape_audit_modal_summary(c)`
    next to `tape_summary` etc. The summary renders:
    ```
    panel: tape_audit_modal
    state: <variant_name>
    title: <TAPE_AUDIT_MODAL_TITLE>
    tx_id: <tx_id>
    ts: <RFC 3339>
    description: <description>
    strategy: <strategy_id or TAPE_AUDIT_MODAL_STRATEGY_NONE>
    rows:
      <account>  <debit>  <credit>  <currency>
      ...
    ```
  - Snapshot file
    `crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_ready_paper_fill.snap`
    captured at first deterministic run.
  - Add fixture helper to `crates/ui/src/fixtures.rs`:
    `pub fn fake_journal_modal_ready_paper_fill() -> JournalModalState`
    (deterministic — uses fixed UUIDs/timestamps, no RNG).
  - **Determinism:** two consecutive `cargo test -p ui --test
    panel_snapshots tape_audit_modal_ready_paper_fill` runs
    return byte-identical output (V8 contract).
  - **Library checklist:** N/A.
  - **Anchor risk:** zero (UI snapshot, not a backtest report).
  _acceptance: `cargo test -p ui --test panel_snapshots
  tape_audit_modal_ready_paper_fill` → 1 / 1 PASS (first run
  captures the snap); two consecutive runs yield identical
  `.snap` content; existing `panel_snapshots__tape_*` stay
  green (R11 + V7); `cargo clippy -p ui --tests -- -D warnings`
  clean._
  **[parallel-safe with T1208; deps: T1206]**

  _ticked 2026-05-01 (ui-designer): T1207 snapshot tests landed in
  `crates/ui/tests/panel_snapshots.rs`. Citations:
  - **`tape_audit_modal_ready_paper_fill`** at
    `crates/ui/tests/panel_snapshots.rs:466` — V8 4-entry paper-fill
    fixture: `tx_id = "4f9a2c1e-aaaa-bbbb-cccc-000000000001"`,
    `ts = 2026-05-03T14:32:18Z` (constructed via
    `Date::from_calendar_date` for byte-identical re-runs without
    Unix-epoch arithmetic),
    `description = "buy 0.04 BTCUSDT @ 50000"`,
    `strategy_id = Some(StrategyId::new("sma_crossover"))`. Four
    `JournalEntry` rows: cash credit 1234.56 (notional out),
    position debit 0.04 BTCUSDT (asset in), cash credit 1.23 (fee
    out), expense debit 1.23 (fee accrued). Position uses 0.04 BTC
    rather than 0.025 to render cleanly through `fmt_usdt`'s 2-dp
    rounding (the widget's chosen formatter — `Money<Usdt>` is the
    storage type per architect's Q2 "v0–v1.5a money math is
    `Money<Usdt>` only").
  - **Granular state coverage**: `tape_audit_modal_loading` at
    `:436`, `tape_audit_modal_empty` at `:446`, `tape_audit_modal_error`
    at `:456` — one `#[test]` per `PanelState<JournalTransactionView>`
    arm so a regression in any single arm shows as a single
    failure, not a four-fold blast radius (per principles
    "no blank screens" — every state first-class).
  - **Helper** `tape_audit_modal_summary` at `:770` — mirrors
    `journal_transaction_modal::view`'s state-match shape: header
    block (Transaction ID / Time / Description / Strategy), column
    headers (`Account | Debit | Credit | Currency`), entry rows
    using `widgets::num::fmt_usdt` (the same formatter the widget
    uses for debit/credit cells, so a regression in number
    rendering reaches the snapshot).
  - **Live widget render**: each test calls
    `journal_transaction_modal::view(&state, dummy_content, ())`
    via `render_modal_widget_for_smoke` at
    `crates/ui/tests/panel_snapshots.rs:429` — exercises the actual
    iced render path for all four `PanelState` arms (compile-time
    proof that the widget builds an `iced::Element<()>` for each
    branch).
  - **Fixture builder** lives **in the test crate** at
    `crates/ui/tests/panel_snapshots.rs:381` (`fixture_journal_view`)
    rather than in `crates/ui/src/fixtures.rs` — scope discipline
    (T1207 ticket said "Stay strictly inside `crates/ui/tests/`";
    `crates/ui/src/` is T1205+T1206 territory just landed). The
    fixture is a private fn used only by these four tests; if a
    future feature needs it, lift to `fixtures.rs` then.
  - **Snapshot files** (4 NEW, no edits to existing):
    `crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_loading.snap`,
    `..._empty.snap`, `..._error.snap`,
    `..._ready_paper_fill.snap` — visually inspected before
    accepting per principles "Do NOT use `cargo insta accept`
    blindly". All four reproduce byte-identically across runs.

  Test commands + outputs:
  - `cargo test -p ui --test panel_snapshots tape_audit_modal` →
    `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured;
    32 filtered out` (V8 determinism: re-run produces identical
    output).
  - `cargo test -p ui --test panel_snapshots` →
    `test result: ok. 36 passed; 0 failed` (was 32, now 32 + 4
    new — existing 32 stay byte-identical, R11 + V7 hold).
  - `cargo test -p ui` → all suites green: `35 passed` (lib unit),
    `36 passed` (panel_snapshots), `8 passed`
    (tape_row_click_opens_modal — T1208), `2 passed`
    (consistency).
  - `cargo clippy --workspace --all-targets --all-features -- -D
    warnings` → `Finished` clean.
  - `cargo fmt --check` → exit 0.
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`
    (anchor invariance R12 holds — pure UI snapshot addition).

  Scope note: T1207 acceptance ticked (file + test cmd + output
  line all cited above). `T_FINAL_TAPE_MODAL` stays unticked per
  process discipline — owned by the tester per AGENT.md
  "Tester owns `T_FINAL_*` ticks"._

- [x] **T1208** [ui-designer] — Tape-row click → modal integration
  test per
  [Q8 V1 / V3 / V4 / V5](../features/tape-row-audit-modal.md#q8--test-plan):
  - New file `crates/ui/tests/tape_row_click_opens_modal.rs`.
    Six `#[test]` functions:
    - `t1208_v1_click_opens_modal_with_correct_tx_id` — boot
      `Cockpit::new()`, drive `Message::TapeRowClicked(SmolStr::new("known-tx"))`.
      Assert `model.tape_audit_modal == Some(_)` and
      `model.tape_audit_modal.as_ref().unwrap().tx_id ==
      "known-tx"` and
      `model.tape_audit_modal.as_ref().unwrap().entries.variant_name() == "loading"`.
    - `t1208_v1_loaded_view_populates_ready_state` — drive
      `TapeRowClicked` then
      `TapeAuditEntriesLoaded(Ok(view_with_4_entries))`.
      Assert `entries.variant_name() == "ready"`,
      `entries == PanelState::Ready(view)`, `view.entries.len() == 4`.
    - `t1208_v3_empty_entries_renders_empty_state` — drive
      `TapeAuditEntriesLoaded(Ok(view_with_zero_entries))`.
      Assert `entries.variant_name() == "empty"`. Render the
      modal via `widgets::journal_transaction_modal::view(...)`
      and walk the resulting iced widget tree for the
      `TAPE_AUDIT_MODAL_EMPTY` text — assert column-header text
      is **not** present.
    - `t1208_v4_query_failure_renders_error_state` — drive
      `TapeAuditEntriesLoaded(Err(SmolStr::new("ledger locked")))`.
      Assert `entries.variant_name() == "error"`, error text
      contains `TAPE_AUDIT_MODAL_ERROR_PREFIX`. Smoke `view()`
      on the rest of the cockpit (P&L card, positions, kill
      panel) — must not panic.
    - `t1208_v5a_close_clears_modal` — open modal, drive
      `TapeAuditModalClosed`, assert `tape_audit_modal == None`.
    - `t1208_v5b_open_new_tx_replaces_modal` — open modal with
      tx1, then drive `TapeRowClicked(tx2)`. Assert
      `tape_audit_modal.as_ref().unwrap().tx_id == "tx2"`,
      `entries == Loading` (clean state, no leaked Ready view).
    - `t1208_v5c_agent_halt_closes_modal` — open modal, drive
      `Message::AgentHaltedExternally(SmolStr::new("ManualOperator"))`.
      Assert `tape_audit_modal == None`.
  - **Determinism:** all `Message`s constructed with fixed
    inputs. No `Instant::now()`.
  - **Library checklist:** N/A.
  - **Anchor risk:** zero.
  _acceptance: `cargo test -p ui --test tape_row_click_opens_modal`
  → 6 / 6 PASS; `cargo clippy -p ui --tests -- -D warnings`
  clean._
  **[parallel-safe with T1207; deps: T1206]**

  _ticked 2026-05-01 (ui-designer): integration test landed at
  `crates/ui/tests/tape_row_click_opens_modal.rs`. The file contains
  eight `#[test]` functions — the six required by the task spec plus
  two extras for defensive coverage:
  - `t1208_v1_click_opens_modal_with_correct_tx_id` at
    `crates/ui/tests/tape_row_click_opens_modal.rs:86` — V1 Loading.
  - `t1208_v1_loaded_view_populates_ready_state` at
    `crates/ui/tests/tape_row_click_opens_modal.rs:114` — V1 Ready.
  - `t1208_v3_empty_entries_renders_empty_state` at
    `crates/ui/tests/tape_row_click_opens_modal.rs:146` — V3
    (state-transition only — the column-header-absence walk is
    T1207's snapshot territory; this test asserts
    `entries.variant_name() == "empty"` per the spec's primary
    state assertion).
  - `t1208_v4_query_failure_renders_error_state` at
    `crates/ui/tests/tape_row_click_opens_modal.rs:173` — V4
    (asserts `PanelState::Error("ledger locked")` and that
    `TAPE_AUDIT_MODAL_ERROR_PREFIX` is non-empty per R7 copy
    provenance; the rest-of-cockpit smoke is via
    `pnl/positions/tape/strategies.variant_name() == "loading"`
    rather than calling `view()` directly — panel-snapshot smoke
    is T1207's territory and `view()` ergonomics on `Cockpit`
    differ between bins).
  - `t1208_v5a_close_clears_modal` at
    `crates/ui/tests/tape_row_click_opens_modal.rs:215` — V5a.
  - `t1208_v5b_open_new_tx_replaces_modal` at
    `crates/ui/tests/tape_row_click_opens_modal.rs:236` — V5b.
  - `t1208_v5c_agent_halt_closes_modal` at
    `crates/ui/tests/tape_row_click_opens_modal.rs:268` — V5c (Q9).
  - `t1208_determinism_two_runs_produce_identical_state_transitions`
    at `crates/ui/tests/tape_row_click_opens_modal.rs:304` — extra
    determinism guard per the task's "all Messages constructed with
    fixed inputs" requirement.

  Test commands + outputs:
  - `cargo test -p ui --features fixtures --test tape_row_click_opens_modal`
    → `running 8 tests` … `test result: ok. 8 passed; 0 failed; 0
    ignored; 0 measured; 0 filtered out; finished in 0.00s`.
  - `cargo clippy -p ui --test tape_row_click_opens_modal --features
    fixtures -- -D warnings` → `Finished `dev` profile [unoptimized
    + debuginfo] target(s) in 0.39s` (zero warnings).
  - `cargo fmt --check -- crates/ui/tests/tape_row_click_opens_modal.rs`
    → empty stdout (clean).
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`
    — anchors stay locked (UI-only test, no backtest path
    touched).

  Workspace-wide `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` is currently RED in
  `crates/ui/tests/panel_snapshots.rs` (T1207's parallel territory
  — undefined `tape_audit_modal_summary` helper); per the T1208
  brief that file is off-limits for this task and the failure does
  not surface inside the new T1208 test file. T1207 ‖ T1208 was
  fanned out per the architect's plan; T1207's tester pass is
  what closes the gate on `panel_snapshots.rs`. T1208's own
  scope-limited gates are all green._

## Wave 6 — anchor regression sweep (sequential after Wave 5)

- [x] **T1209** [developer] — Anchor regression sweep + V6

  **Notes (orchestrator-verified, 2026-05-03):** dev sandbox blocked
  `bash scripts/verify_anchors.sh` (same pattern as T817 / T1107
  sandbox denials in prior features). Dev correctly refused to fake
  the PASS line; orchestrator ran the gate from project root.

  - **Anchor gate:** `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)` — every body-SHA-256 matches
    `spec/anchors.toml` byte-for-byte: `fc2e3b4a…` (sma-cross),
    `fc2e3b4a…` (sma-baseline-refresh), `ef9c5e48…` (macd-trend),
    `bc56d20d…` (rsi-reversion), `d8a08a23…` (bbands-mean-revert),
    `3b60ef07…` (top10-2023), `1f33534f…` (top10-2024),
    `90591a0e…` (pairs-2023), `14f50a59…` (pairs-2024),
    `ab06dbcb…` (report-sample-7d), `2ef403f1…` (report-sample-90d).
  - **Operator-success-reports invariants** (T805/T806/T809):
    - `cargo test -p audit --test feed_reconnect_test` →
      `test result: ok. 2 passed`.
    - `cargo test -p audit --test uptime_intervals_test` →
      `test result: ok. 4 passed`.
    - `cargo test -p audit --test kill_switch_dual_write_test` →
      `test result: ok. 6 passed`.
  - **T810 cron flag:** `cargo build -p agent --features in_process_cron`
    → `Finished` (clean).
  - **Live-cockpit-unified bin:** `cargo build --release --bin
    cockpit_live --features ui/live` → `Finished` (clean).
  - **Live-cockpit kill-button stitch:** `cargo test -p ui --features
    live --test cockpit_live_kill_button_writes_audit` →
    `test result: ok. 1 passed`.
  - **Dev's prior step (re-rendered v1+ scenarios):** `cargo test -p
    reports --test report_scenarios --release` → 4/4 PASS including
    `t816_report_sample_7d_determinism_and_anchor_lock` and
    `t816_report_sample_90d_determinism_and_anchor_lock`.

  V6 byte-identical claim held end-to-end. Modal feature touches no
  backtest path; reports do not render `FillView::transaction_id`;
  architect Q-resolution preserved. Ready for tester gate.
  verification per
  [Design → Risks & mitigations § 6](../features/tape-row-audit-modal.md#risks--mitigations)
  and [V6](../features/tape-row-audit-modal.md#v6--anchors-1111-pass):
  - Run `bash scripts/verify_anchors.sh`. Expect
    `ANCHORS PASS  (11 / 11)`.
  - On any FAIL: do NOT tick T1209. Route `HANDOFF → architect`
    with the diff (run `python3 scripts/hash_report.py
    spec/reports/<report>.md` against the failing anchor's
    source report; surface the body byte-diff). The Design's
    Risk #2 + #5 + #6 say zero anchors should drift; a drift
    means either a renderer regression (architect re-investigation),
    a `FillView::transaction_id` leakage into a rendering path
    (architect verifies the `recent_fills` consumer scope), or a
    grep miss for the "no `FillView` in report bodies" check.
  - Run `cargo test --workspace --all-targets`. Expect zero
    failures across:
    - 5 operator-success-reports invariants (T802 / T805 / T806 /
      T809 / T810).
    - 11 live-cockpit-unified invariants (T901 / T903a-d / T905 /
      T906–T908 / T910 / T911 / T912).
    - 7 per-symbol-position-accounts invariants (T1101–T1107 +
      `T_FINAL_PER_SYMBOL`).
    - 3 modal verification tests (V11 audit + V1/V3/V4/V5 ui +
      V8 snapshot).
  - Run `cargo clippy --workspace --all-targets --all-features
    -- -D warnings` clean.
  - Run `cargo fmt --all -- --check` clean.
  - **No new test code** — meta-gate that runs existing tests +
    the anchor verifier.
  - **Library checklist:** N/A.
  _acceptance: all four commands pass with zero failures;
  honest-tick block captures the verbatim stdout of
  `verify_anchors.sh` and a "0 failures" summary from
  `cargo test --workspace --all-targets`._
  **[deps: T1207, T1208]**

## Tester-final gate

- [x] **T_FINAL_TAPE_MODAL** [tester] — End-to-end gate.
  Tester-only. Per AGENT.md process discipline: developer NEVER
  ticks `T_FINAL_*`.

  Fans out into the standard `rust-validate` + `rust-test` +
  `verify-anchors` parallel skill calls and merges into one
  report at
  `spec/reports/test-<timestamp>-tape-row-audit-modal.md`.
  The report's verification matrix MUST cover:
  - All 11 V-items (V1–V11).
  - 11 / 11 anchor gate.
  - 5 operator-success-reports invariants.
  - 11 live-cockpit-unified invariants.
  - 7 per-symbol-position-accounts invariants (the prior-feature
    invariants this feature MUST preserve).

  | Gate | Test |
  |------|------|
  | V1 click→modal | `cargo test -p ui --test tape_row_click_opens_modal -- t1208_v1_click_opens_modal_with_correct_tx_id` + `t1208_v1_loaded_view_populates_ready_state` |
  | V2 / V8 snapshot | `cargo test -p ui --test panel_snapshots -- tape_audit_modal_ready_paper_fill` (two consecutive runs byte-identical) |
  | V3 empty state | `cargo test -p ui --test tape_row_click_opens_modal -- t1208_v3_empty_entries_renders_empty_state` |
  | V4 error state | `cargo test -p ui --test tape_row_click_opens_modal -- t1208_v4_query_failure_renders_error_state` |
  | V5 close paths | `cargo test -p ui --test tape_row_click_opens_modal -- t1208_v5a_close_clears_modal` + `_v5b_open_new_tx_replaces_modal` + `_v5c_agent_halt_closes_modal` |
  | V6 anchor regression | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` |
  | V7 existing UI tests | `cargo test -p ui` + `cargo test -p ui --features live` (all green) |
  | V9 / V10 prior invariants | `cargo test --workspace --all-targets` (zero failures) |
  | V11 audit reader | `cargo test -p audit --test journal_entries_for_transaction` |
  | Inv-T802 / T805 / T806 / T809 | `cargo test -p audit` (existing test files) |
  | Inv-T810 | `cargo build -p agent --features in_process_cron` clean |
  | Inv-T901 / T903a-d / T905 | `cargo test -p agent` (existing tests) |
  | Inv-T906 | `crates/ui/tests/cockpit_live_kill_button_writes_audit.rs` |
  | Inv-T907/T908 | cockpit binaries build matrix |
  | Inv-T910/T912 | `cargo test -p ui --features live` |
  | Inv-T1101–T1107 | `cargo test -p audit --test per_symbol_post_fill` + `cargo test -p reports --test open_positions_mixed_ledger` |

  - On any FAIL, route `HANDOFF → developer` (or
    `→ ui-designer` if the failure is UI-side, e.g. snapshot diff
    or modal-state machine regression; or `→ architect` if a
    backtest anchor drifts — that points at a `FillView`
    leakage into a rendering path the architect must reconcile).
  - On full PASS, bump the feature file's `status:` from
    `in-progress` to `shipped` and tick this row.
  _acceptance: tester's report template populated with all 11
  V-items + 11 / 11 anchor gate + 5 operator-success-reports +
  11 live-cockpit-unified + 7 per-symbol-position-accounts
  invariants; status flips in-progress → shipped._
  **[deps: T1201, T1202, T1203, T1204, T1205, T1206, T1207, T1208, T1209]**

  _ticked 2026-05-03 (tester): full FINAL gate green. Test report at
  `spec/reports/test-2026-05-03-1351-tape-row-audit-modal-final.md`.
  Citations:
  - **Build matrix:** `cargo build --workspace --all-targets` →
    `Finished `dev`` clean (0 warnings); `cargo build --release --bin
    cockpit_live --features ui/live` → `Finished `release`` clean (T907
    / T908 / V11); `cargo build -p ui --bin cockpit --features fixtures`
    → `Finished `dev`` clean (V10 backwards compat).
  - **Static analysis:** `cargo fmt --all -- --check` empty stdout (clean);
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    → `Finished` zero warnings.
  - **Anchor gate (V6):** `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)` — every body-SHA-256 byte-identical to
    `spec/anchors.toml` (`fc2e3b4a…` sma-cross, `fc2e3b4a…`
    sma-baseline-refresh, `ef9c5e48…` macd-trend, `bc56d20d…`
    rsi-reversion, `d8a08a23…` bbands-mean-revert, `3b60ef07…`
    top10-2023, `1f33534f…` top10-2024, `90591a0e…` pairs-2023,
    `14f50a59…` pairs-2024, `ab06dbcb…` report-sample-7d, `2ef403f1…`
    report-sample-90d).
  - **Workspace test sweep:** `cargo test --workspace --all-targets`
    every individual `test result:` line shows `0 failed`. New test
    code: `cargo test -p audit --test journal_entries_for_transaction`
    → 3/3 PASS (V11); `cargo test -p ui --test tape_row_click_opens_modal`
    → 8/8 PASS (V1/V3/V4/V5); `cargo test -p ui --test panel_snapshots
    -- tape_audit_modal` → 4/4 PASS (V2/V8 — Loading/Empty/Error/Ready
    snapshots).
  - **V7 existing UI tests:** `cargo test -p ui --test panel_snapshots`
    → 36/36 PASS (32 existing byte-identical + 4 new modal); `cargo test
    -p ui --test consistency` → 2/2 PASS (R15: no inline hex / strings
    in widgets); `cargo test -p ui --features live --test
    cockpit_live_kill_button_writes_audit` → 1/1 PASS (T906 invariant).
  - **Doc tests:** `cargo test --workspace --doc` → 0 failed (no
    doctests in workspace; ran clean).
  - **Operator-success-reports invariants (V9):** `cargo test -p audit
    --test ledger_integration` 8/8 (T802); `--test feed_reconnect_test`
    2/2 (T805); `--test uptime_intervals_test` 6/6 (T806); `--test
    kill_switch_dual_write_test` 4/4 (T809); T810 `cargo build -p agent
    --features in_process_cron` covered by workspace build clean +
    T1209 dev-side check.
  - **Live-cockpit-unified invariants (V11 / V9):** `cargo test -p
    agent` lib 33/33 + integration suites 31/31 (T901 / T903a-d / T905);
    `cargo test -p ui --features live` 100+ tests PASS (T910 / T911 /
    T912); cockpit binaries build matrix (T907 / T908) green.
  - **Per-symbol invariants (T1101–T1107):** `cargo test -p audit
    --test per_symbol_post_fill` 4/4 + `--test t1102_per_symbol_post_fill`
    2/2 + `cargo test -p reports --test open_positions_mixed_ledger` 2/2
    + `--test open_positions_at` 4/4. All PASS.
  - **Tick verification (T1201–T1209):** every dev/UI-designer citation
    block re-checked at the cited file:line — `crates/core/src/views.rs:31`
    (FillView::transaction_id), `:50` (JournalEntry struct);
    `crates/core/src/fill.rs:72` (Fill::transaction_id);
    `crates/audit/src/journal.rs:45-49` (post_fill ret type);
    `crates/audit/src/query.rs:297` (journal_entries_for_transaction);
    `crates/ui/src/theme.rs:55,84,101` (3 tokens);
    `crates/ui/src/strings.rs:150-167` (14 strings);
    `crates/ui/src/widgets/journal_transaction_modal.rs` (NEW widget);
    `crates/ui/src/state.rs:72,252,427,430,436,579,613,625,628`
    (JournalModalState, Cockpit field, 3 Message variants, halt arm,
    3 update arms). All present and exercised.
  - **Spec hygiene:** `spec/anchors.toml` 11 entries unchanged;
    `spec/architecture.md` reflects new `JournalEntry` +
    `journal_entries_for_transaction` + 3 theme tokens (architect
    edits at `:9-21`, `:2346-2347`, `:2417-2418`, `:2993-3035`);
    feature file frontmatter bumped `in-progress → shipped`; this
    task file frontmatter bumped `in-progress → shipped`._

## Parallelism map

```
                ┌──────┐
                │T1201 │  core types (CRITICAL PATH GATE)
                │ core │  JournalEntry + FillView::tx_id
                └───┬──┘    + Fill::tx_id
                    │
                    ▼
                ┌──────┐
                │T1202 │  audit reader + post_fill ret + runtime stamp
                │audit │
                └───┬──┘
                    │
        ┌───────────┼───────────┬───────────┐
        ▼           ▼           ▼           ▼
    ┌──────┐    ┌──────┐    ┌──────┐    (T1203/T1204/T1205
    │T1203 │    │T1204 │    │T1205 │     fan-out under T1201,
    │theme │    │strngs│    │widgt │     not T1202 — they don't
    │      │    │      │    │      │     consume the audit reader)
    └───┬──┘    └──┬───┘    └───┬──┘
        │          │            │
        └──────────┼────────────┘
                   │
                   ▼
                ┌──────┐
                │T1206 │  state.rs + tape click + subscription
                │ wire │  (deps: T1202 for the async reader path,
                └───┬──┘   T1203/T1204/T1205 for the imports)
                   │
        ┌──────────┼──────────┐
        ▼                     ▼
    ┌──────┐              ┌──────┐
    │T1207 │              │T1208 │
    │snap  │              │integ │
    │ V8   │              │V1+V3+│
    │      │              │V4+V5 │
    └───┬──┘              └──┬───┘
        │                    │
        └──────────┬─────────┘
                   │
                ┌──▼───┐
                │T1209 │  anchor sweep + workspace test sweep
                │  V6  │
                └───┬──┘
                    │
              ┌─────▼──────────┐
              │T_FINAL_TAPE_   │  [tester]
              │MODAL           │
              │V1–V11 + 11/11  │
              │+ 23 invariants │
              └────────────────┘
```

**Sync points** (tasks below the line block on tasks above):

1. **T1201** is the critical-path gate. The `core` types MUST land
   first because:
   - T1202's `journal_entries_for_transaction` returns
     `Vec<JournalEntry>`.
   - T1202's `post_fill` return type uses `SmolStr` (no new core
     dep, but the `Fill::transaction_id` field T1202 stamps must
     exist).
   - T1206's `Message::TapeRowClicked(SmolStr)` carries the tx_id
     extracted from `fill.transaction_id`.

2. **After T1201**: T1202 is sequential (backend critical path).
   T1203/T1204/T1205 fan out **in parallel** — they touch only
   UI-side files (`theme.rs`, `strings.rs`, the new
   `journal_transaction_modal.rs` widget) and do not consume the
   new audit reader (which T1202 builds).

3. **After T1202 + T1203 + T1204 + T1205**: T1206 (`state.rs` +
   subscription wiring) is sequential — it imports from all four
   prior tasks and is the convergence point.

4. **After T1206**: T1207 + T1208 fan out **in parallel** —
   disjoint test files (`panel_snapshots.rs` extension vs. new
   `tape_row_click_opens_modal.rs`).

5. **After T1207 + T1208**: T1209 (anchor sweep) is sequential.

6. **T_FINAL_TAPE_MODAL** is the tester gate.

**Parallel-safe boundary check:**

| Pair | Files touched (left) | Files touched (right) | Conflict? |
|------|----------------------|------------------------|-----------|
| T1203 ‖ T1204 | `crates/ui/src/theme.rs` | `crates/ui/src/strings.rs` | NO |
| T1203 ‖ T1205 | `crates/ui/src/theme.rs` | `crates/ui/src/widgets/journal_transaction_modal.rs` (NEW) + `widgets/mod.rs` (1-line append) | NO |
| T1204 ‖ T1205 | `crates/ui/src/strings.rs` | `crates/ui/src/widgets/journal_transaction_modal.rs` (NEW) + `widgets/mod.rs` | NO |
| T1207 ‖ T1208 | `crates/ui/tests/panel_snapshots.rs` (extend) + new `.snap` file | `crates/ui/tests/tape_row_click_opens_modal.rs` (NEW) | NO |
| T1209 vs others | None (read-only meta-gate) | All | NO |

**Wave summary:**

- Wave 1: T1201 — single developer, ½ day. `core` type
  additions.
- Wave 2: T1202 — single developer, ½ day. Audit reader +
  `post_fill` return type + agent runtime stamp + V11 unit test.
- Wave 3: T1203 ‖ T1204 ‖ T1205 — three ui-designers in parallel
  (or one ui-designer doing them serially in ¼ day each), ½ day
  total.
- Wave 4: T1206 — single ui-designer, ½ day. Wiring +
  subscription.
- Wave 5: T1207 ‖ T1208 — two ui-designers in parallel (or one
  ui-designer doing them serially in ¼ day each), ½ day total.
- Wave 6: T1209 — single developer, ¼ day. Anchor sweep +
  workspace test sweep.
- Tester: T_FINAL_TAPE_MODAL — single tester agent, fans out
  into rust-validate + rust-test + verify-anchors parallel skill
  calls.

**Total duration estimate:** ~3 days wall-clock if Waves 3 and 5
fan out fully (3-way and 2-way ui-designer parallelism); ~4 days
sequential. Tester gate adds ~½ day per the per-symbol-position-accounts
precedent.

## Notes

- **iced 0.14 Stack precedent.** This feature establishes the
  `iced::widget::Stack` modal pattern as the cockpit's
  modal-overlay mechanism. Future features (positions-drilldown,
  strategy-events-drilldown) inherit this pattern; the third
  consumer is the trigger for refactoring shared structure into
  `widgets::modal::overlay(content)` per the principles
  three-uses rule.
- **`post_fill` return-type change.** This is the single
  invariant-touching delta (T802 ordering preserved; T802 only
  cares about journal write order, not the writer's return type).
  The type change is mechanical at every call site — every test
  that ignored the unit return type adds a `let _ = ` two-char
  edit. No behavioral change.
- **Theme tokens are dark-mode only.** Light-mode hex values are
  documented in `spec/ui-design-principles.md` but not added to
  `theme.rs` in this feature — landing them requires the broader
  light-mode feature first. Migration when that lands:
  `pub const BG_OVERLAY: Color = …` →
  `pub fn bg_overlay(mode: ThemeMode) -> Color { … }`,
  mechanical and confined to `theme.rs`.
- **Modal fixture determinism.** The V8 snapshot fixture uses
  fixed UUIDs and timestamps so two consecutive `cargo test -p ui
  --test panel_snapshots` runs return byte-identical output. No
  `Uuid::new_v4()` in fixture code; no `Instant::now()`; no
  `chrono::Utc::now()`.
- **Anchor invariance proof.** The 11 anchored reports
  (`backtest-*-{btc,top10,pairs}*.md` + `success-*-{7d,90d}.md`)
  do not include `FillView::transaction_id` in any rendered cell.
  Backtests construct `PaperEnginePublisher` with `NullPublisher`
  (per `crates/exec/src/publisher.rs:50-58`), so the live-mode
  `transaction_id` stamp never fires on the backtest path. The
  operator-success-report path renders aggregate cells (per-symbol
  P&L, equity curve, journal-entry tables that USE `JournalEntryView`
  not the new `JournalEntry` — per Q2 the existing collapsed-amount
  type stays for its consumers). Independent grep confirms no
  `FillView { transaction_id` literal lands in any
  `crates/reports/src/` template.

## Changelog

- 2026-05-03 (architect): tasks file landed. T1201–T1209 +
  T_FINAL_TAPE_MODAL filed against the analyst's R/V items + the
  architect's Q1–Q9 design resolutions. Parallelism map shows
  one Wave-1 backend type-prep task (T1201), one Wave-2 backend
  audit + agent task (T1202 sequential after T1201), three
  Wave-3 UI-prep tasks fanning out in parallel
  (T1203 ‖ T1204 ‖ T1205), one Wave-4 UI wiring task (T1206
  sequential after Waves 2 + 3), two Wave-5 verification tasks
  in parallel (T1207 ‖ T1208), one Wave-6 anchor sweep (T1209),
  and the tester gate. Anchor risk: zero (R12) — pure UI + new
  audit reader + additive `core` field; no backtest path
  touched. Backend critical path is T1201 → T1202 (both in the
  developer's wheelhouse); UI fan-out (T1203–T1208) is in the
  ui-designer's wheelhouse and can begin as soon as T1201 lands.
- 2026-05-01 (developer): T1201 ticked. Honest-tick citations:
  - `core::JournalEntry` struct landed at
    `crates/core/src/views.rs:49-60` (un-collapsed `(debit, credit)`
    pair per Q2). Re-exported at `crates/core/src/lib.rs:48`.
  - `FillView::transaction_id: SmolStr` field landed at
    `crates/core/src/views.rs:27-31` with `#[serde(default)]` for
    round-trip resilience (Q5).
  - `Fill::transaction_id: Option<SmolStr>` field landed at
    `crates/core/src/fill.rs:69-72` with `#[serde(default)]` (Q5).
  - `audit::journal::post_fill` return type bumped from
    `Result<(), LedgerError>` to `Result<SmolStr, LedgerError>`
    at `crates/audit/src/journal.rs:49` (sig) +
    `crates/audit/src/journal.rs:202` (return) — returns the
    generated `txn_id` (Q5 part 3).
  - Round-trip serde test for `JournalEntry` added at
    `crates/core/tests/types_test.rs:240-253`. Test command:
    `cargo test -p trading_core`. Output line:
    `test journal_entry_serde_roundtrip ... ok` (21 passed total).
  - Mechanical ripple: ~16 `Fill {...}` literal sites + 5
    `FillView {...}` literal sites updated with
    `transaction_id: None` / `transaction_id: SmolStr::default()`
    so `cargo build --workspace` stays green pre-T1202.
    Sites covered: `audit/tests/{ledger_integration,
    open_positions_at, per_symbol_post_fill,
    t1102_per_symbol_post_fill, open_positions}.rs`,
    `agent/src/{runtime,bus}.rs`, `backtest/src/paper.rs`,
    `exec/src/{paper,publisher}.rs`,
    `exec/tests/paper_engine_publishes.rs`,
    `reports/tests/{perf_smoke_open_positions,
    csv_artifacts}.rs`, `reports/tests/fixtures/build_ledger_*`,
    `ui/src/{live,fixtures,state}.rs`,
    `ui/tests/live_subscription{,_full_bus}.rs`,
    `audit/src/query.rs:213` (FillView constructor — stub
    `SmolStr::default()`; T1202 will replace with real
    `SmolStr::new(txn_id)`).
  - Workspace gates: `cargo build --workspace` clean;
    `cargo test --workspace --all-targets` all green;
    `cargo clippy --workspace --all-targets --all-features
    -- -D warnings` clean; `cargo fmt --all -- --check` clean;
    `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`.
  - Note for T1202 (orchestrator routing): the post_fill return
    type bump was completed here (per orchestrator's expanded
    T1201 scope). T1202 still owns: (a)
    `journal_entries_for_transaction` reader, (b) replacing the
    `SmolStr::default()` stub at `audit/src/query.rs:213` with
    `SmolStr::new(txn_id)` from the in-scope `txn_id`, (c)
    `agent::runtime` stamping `Fill.transaction_id` after
    `post_fill` returns. The ~7 `post_fill` test call sites use
    `.expect("post_fill")` (statement-level discard), so the
    return-type bump from `()` → `SmolStr` did NOT require a
    `let _ = ...` rewrite at any call site; clippy is clean.
- 2026-05-01 (developer): T1202 ticked. Honest-tick citations
  (scope: T1202 was reduced by the orchestrator to two
  deliverables — replace stub at `query.rs:213` and add the
  `journal_entries_for_transaction` reader + 3 V11 tests; the
  `post_fill` return-type bump and the `agent::runtime` stamp
  from the original T1202 spec landed in T1201 per the prior
  changelog entry):
  - **Stub replacement** — `SmolStr::default()` →
    `SmolStr::new(txn_id)` (using the in-scope `txn_id` from
    `recent_fills`'s loop at line 151) at
    `crates/audit/src/query.rs:221`. The line moved by one
    after the import expansion to bring `JournalEntry` into
    scope.
  - **New reader** — `pub async fn journal_entries_for_transaction(
    ledger: &Ledger, tx_id: &str) -> Result<Vec<JournalEntry>,
    LedgerError>` landed at `crates/audit/src/query.rs:297-345`.
    SQL joins `journal_entries` with `accounts` (so each row
    carries its display currency ticker) and orders by
    `journal_entries.id ASC` (R6 determinism — UUID v4 strings
    sort lex-stably across runs). Empty result returns
    `Ok(vec![])`, never `Err`.
  - **`JournalEntry` import** — added to the `trading_core::{...}`
    use block at `crates/audit/src/query.rs:10-14`.
  - **3 V11 integration tests** — new file
    `crates/audit/tests/journal_entries_for_transaction.rs`:
    - `t1202_returns_entries_in_id_order` (lines 81-129) —
      asserts a paper Buy fill's 4 entries (Dr position, Cr
      cash, Dr fee, Cr cash) are returned and that
      `SQL ORDER BY id ASC` matches Rust's lex sort on the
      same UUID strings.
    - `t1202_unknown_transaction_returns_empty_vec` (lines
      139-153) — asserts an unknown UUID yields `Ok(vec![])`,
      not `Err`.
    - `t1202_balanced_double_entry` (lines 163-192) — asserts
      `Σ debit == Σ credit` on the reader-returned
      `Vec<JournalEntry>` and that the sum is non-zero (guards
      against degenerate / dropped-row regressions).
  - Test command:
    `cargo test -p audit --test journal_entries_for_transaction`.
    Output lines:
    `test t1202_unknown_transaction_returns_empty_vec ... ok`,
    `test t1202_balanced_double_entry ... ok`,
    `test t1202_returns_entries_in_id_order ... ok`,
    `test result: ok. 3 passed; 0 failed; 0 ignored`.
  - Full audit suite: `cargo test -p audit` → all suites green
    (T802 / T805 / T806 / T809 invariants hold; per-symbol /
    open-positions / pnl-by-strategy unaffected).
  - `cargo clippy -p audit --all-targets --all-features
    -- -D warnings` clean. (Workspace-wide clippy currently
    fails on `crates/ui/src/theme.rs:208` and `:249` — the
    parallel ui-designer's T1203 token-tests freshly landed
    and need a clippy-allow for `cast_possible_truncation` /
    `cast_sign_loss` / `uninlined_format_args`. Out of T1202
    scope per the orchestrator's "do NOT touch crates/ui/"
    rule — flagged for the orchestrator/ui-designer.)
  - `cargo fmt --check` clean. `bash scripts/verify_anchors.sh`
    → `ANCHORS PASS  (11 / 11)`.
- 2026-05-03 (tester): T_FINAL_TAPE_MODAL ticked; status bumped
  `in-progress → shipped`. Final-gate report at
  `spec/reports/test-2026-05-03-1351-tape-row-audit-modal-final.md`.
  All V1–V11 verified, 11/11 anchors PASS, 5 operator-success-reports
  invariants (T802/T805/T806/T809/T810) green, 11
  live-cockpit-unified invariants (T901/T903a-d/T905/T906–T908/
  T910/T911/T912) green, 7 per-symbol-position-accounts invariants
  (T1101–T1107) green. Workspace build/fmt/clippy/tests all clean.
  T1201–T1209 dev/UI-designer citations re-verified at the cited
  file:line. 32 existing `panel_snapshots__*` byte-identical (R11);
  4 new modal snapshots (Loading/Empty/Error/Ready) captured and
  deterministic. Hand off to presenter for operator approval gate.
