---
slug: tape-row-audit-modal
status: shipped
owner: architect
updated: 2026-05-03
version: 1.6.0
---

# Tape-row → audit modal

## Why

The live tape is the operator's primary "what just happened" surface,
but today is read-only in the literal sense: the operator sees a row
("Buy 0.4 BTCUSDT @ 52,341.20") and cannot drill into the underlying
`journal_transaction` (debits, credits, transaction_id, source
strategy). Answering "why did the agent post that fill?" requires
leaving the cockpit and writing SQL.

The [UI design principles](../ui-design-principles.md) "Show the why"
section ([line ~336](../ui-design-principles.md#show-the-why)) makes
this a first-class rule: "every order, signal, fill, risk veto, and
strategy event is click-through to its decision trail in the audit
ledger". The same doc promotes this feature in
[Q4](../ui-design-principles.md#open-questions); operator approved on
**2026-05-03** ("YES. Promoted to backlog as `tape-row-audit-modal`
for v1.5+ scope").

This is the **first feature on the cockpit click-through-to-audit
path** (positions / strategies follow) and the **first feature to land
against the principles doc** — its decisions on theme tokens and
modal pattern seed answers future UI features inherit.

The architecture surface map already lists `recent_journal(&Ledger,
usize)` for a future "show the why" modal (see
[architecture.md "Cockpit ← `audit::query`"](../architecture.md#cockpit--auditquery));
this feature is its consumer. The specific reader needed —
`journal_entries_for_transaction(tx_id)` — is **not yet on the surface
map**; an open question is whether to add it or compose from existing
readers.

## Requirements (R-items)

### R1 — Tape rows clickable, emit `Message::TapeRowClicked(transaction_id)`

Each row in `crates/ui/src/widgets/tape.rs` (currently static cells in
`row_for(fill)`) becomes `Button`-wrapped, click-emitting. Click yields a
new `Message::TapeRowClicked(transaction_id: String)` on the `Message`
enum in `crates/ui/src/state.rs`. Transaction id is the
`journal_transactions.id` UUID string
([`migrations/001_chart_of_accounts.sql:13`](../../crates/audit/migrations/001_chart_of_accounts.sql)).

**Caveat.** `core::FillView`
([`crates/core/src/views.rs:14-22`](../../crates/core/src/views.rs))
does **not** carry `transaction_id` today. R1 is contingent on plumbing
it through (or the architect's preferred alternative — Q5).

### R2 — Cockpit handles click by querying the audit ledger

The `Message::TapeRowClicked(tx_id)` arm asynchronously fetches
`Vec<JournalEntry>` for `tx_id` via a **new** reader (proposed):

```rust
pub async fn journal_entries_for_transaction(
    ledger: &Ledger,
    tx_id: &str,
) -> Result<Vec<JournalEntry>, LedgerError>;
```

`JournalEntry` here is the row-level type (account_id, debit_amount,
credit_amount, currency, ts, memo) — distinct from existing
`JournalEntryView` which collapses debit/credit into a signed `amount`
([`crates/core/src/views.rs:26-31`](../../crates/core/src/views.rs)).
The modal needs the un-collapsed pair to render the 4-column table.
Architect picks the type's home (Q2).

The async query runs through the live subscription's tokio runtime
handle (same pattern as existing `audit::query` callers — see
[architecture.md "Cockpit ← Arc<KillSwitch>"](../architecture.md#cockpit--arckillswitch)).
Iced thread does not own a tokio runtime; architect picks the wiring
(`iced::Task::perform` + handle, or one-shot `Subscription`) — Q6.

### R3 — Modal renders journal entries as a 4-column table + header

Compact density (per
[principles density table](../ui-design-principles.md#density)).

**Header rows** (label + value, plain language): Transaction ID
(full UUID, monospaced), Time (RFC 3339, monospaced), Description
(verbatim from `journal_transactions.description`), Strategy
(`strategy_id` or `—` if NULL).

**Entry table — 4 columns: `Account` / `Debit` / `Credit` /
`Currency`.** Example paper-fill content (typical V8 fixture):

| Account             | Debit       | Credit      | Currency |
|---------------------|------------:|------------:|----------|
| `assets:cash:USDT`  | `0.00`      | `52,341.20` | USDT     |
| `assets:position:BTCUSDT` | `0.40` | `0.00`      | BTC      |
| `expense:fees:taker`| `5.23`      | `0.00`      | USDT     |
| `assets:cash:USDT`  | `0.00`      | `5.23`      | USDT     |

Numbers right-aligned, monospaced, locale-default thousands
separator, per principles "Numbers are scannable". Plain-language
column headers, not `account_id` / `debit_amount`.

### R4 — Modal close: Esc, click-outside, explicit Close button

Three close affordances per
[principles "Accessibility minimums"](../ui-design-principles.md#accessibility-minimums):

1. **`Esc`** — keyboard.
2. **Click outside** — backdrop click. Backdrop uses `bg_overlay`
   (proposed token, R6).
3. **`Close` text button** — top-right of modal, no icon (per
   principles "Iconography: no icons until needed").

All three emit `Message::TapeAuditModalClosed`.

### R5 — Read-only — no audit writes from this UI surface

No writes, no buttons that mutate, nothing that touches `EventBus`
write paths. Consistent with
[architecture "Cockpit ← `audit::query`"](../architecture.md#cockpit--auditquery)
hard constraint:

> The cockpit MUST NOT call `audit::ledger` writers. The bus is the
> only event-push surface from the operator to the audit ledger.

R5 is a constraint, not a behavior — "no new write surface".

### R6 — Theme: 9 shipped tokens + proposed `info` / `border_strong` / `bg_overlay`

The modal uses existing tokens for body/cells, plus three additions
from
[principles color palette](../ui-design-principles.md#color-palette)
(currently `propose` status):

- **`bg_overlay`** (`#0B0D12` dark) — modal backdrop.
- **`border_strong`** (`#3A4456` dark) — modal frame border + focus rings.
- **`info`** (`#7BC2FF` dark) — transaction-id text (informational,
  not interactive). Optional; `accent` is also defensible.

**Decision needed (Q3).** Land all three in this feature, or bump to
a follow-up "design system v2"? Recommend landing here — first concrete
consumer; bundling avoids two-trip "add token / use token" handoff.

### R7 — Strings: all modal copy in `ui::strings`, zero inline

Per
[principles "Strings ↔ widget"](../architecture.md#strings--widget)
+ existing build-break guard `no_inline_user_visible_strings_in_widgets`
([`crates/ui/tests/consistency.rs`](../../crates/ui/tests/consistency.rs)).
New constants (all `pub const &str`):

- `TAPE_AUDIT_MODAL_TITLE = "Journal transaction"`
- `TAPE_AUDIT_MODAL_TX_LABEL = "Transaction ID"`
- `TAPE_AUDIT_MODAL_TS_LABEL = "Time"`
- `TAPE_AUDIT_MODAL_DESC_LABEL = "Description"`
- `TAPE_AUDIT_MODAL_STRATEGY_LABEL = "Strategy"`
- `TAPE_AUDIT_MODAL_STRATEGY_NONE = "—"`
- `TAPE_AUDIT_MODAL_COL_{ACCOUNT,DEBIT,CREDIT,CURRENCY}` =
  `"Account"` / `"Debit"` / `"Credit"` / `"Currency"`
- `TAPE_AUDIT_MODAL_LOADING = "Loading journal entries…"`
- `TAPE_AUDIT_MODAL_EMPTY = "No entries for this transaction."`
- `TAPE_AUDIT_MODAL_ERROR_PREFIX = "Failed to load journal entries: "`
- `TAPE_AUDIT_MODAL_CLOSE_LABEL = "Close"`

Strings follow
[principles voice/copy](../ui-design-principles.md#voice-and-copy):
direct, terse, present-tense, sentence case, unicode `…`.

### R8 — States: loading / populated / empty / error all first-class

Per
[principles "No blank screens"](../ui-design-principles.md#no-blank-screens)
+ existing `state::PanelState<T>`:

- **Loading** — `TAPE_AUDIT_MODAL_LOADING` centered. Query is
  sub-ms IRL (indexed `journal_entries.transaction_id`) but state
  exists for resilience.
- **Populated** — happy path, table renders.
- **Empty** — `Vec<JournalEntry>` empty. Defensive only — by
  `audit::verify_balance` invariant every transaction has ≥ 2
  entries; if it triggers, it's a corruption signal.
- **Error** — `LedgerError` rendered as
  `TAPE_AUDIT_MODAL_ERROR_PREFIX + "{error}"`; modal stays open
  for operator read-then-dismiss.

`Cockpit` model gains `tape_audit_modal:
Option<PanelState<JournalTransactionView>>` (`None` = closed).
Architect confirms field shape (Q5/Q7).

### R9 — Keyboard: Esc closes; modal absorbs navigation while open

Per
[principles "Accessibility minimums"](../ui-design-principles.md#accessibility-minimums):

- `Esc` closes, returns focus to tape.
- `Tab` / `Shift-Tab` cycle within modal.
- Arrow / Page-Up / Page-Down do **NOT** scroll the underlying tape
  while modal is open.
- `Enter` on focused `Close` = click-`Close`.

Focus ring uses `border_strong`, NOT `accent` (principles
"keyboard-focused-vs-active distinguishable"). Architect picks the
iced 0.14 absorption mechanism (Q6).

### R10 — Density: modal honors compact (cockpit) density

Per
[principles density table](../ui-design-principles.md#density):

- Table row height: 24 px.
- Cell horizontal pad: 12 px.
- Modal inner pad: 24 px ("Dialog inner pad" — same in both modes).
- Modal outer width: ~480 px (architect picks final).

### R11 — Tape unchanged in shape — existing snapshots green

The tape change is **wrapping** existing row content in a `Button`,
**not** restructuring. Same columns, same alignment, same colors;
rows just gain hover affordance + click handler.

Existing tape snapshots — `panel_snapshots__tape_loading`,
`__tape_empty`, `__tape_error`, `__tape_ready_three_fills`
([`crates/ui/tests/panel_snapshots.rs:34-62`](../../crates/ui/tests/panel_snapshots.rs))
— assert text shape, not interactivity. They stay green by
construction.

### R12 — Anchor regression: 11/11 PASS

Pure UI + new audit reader. **Does not touch** `crates/strategy/`,
`crates/exec/`, `crates/backtest/`, `crates/reports/` rendering, or
existing `audit::query::*` reader signatures. All 11 anchored
reports (9 backtest + 2 v1+) byte-identical; **`verify-anchors`
11/11 PASS** preferred. If architect's R1 wiring ripples into a
report-rendering path (unlikely), re-lock per v1.5a T717 precedent
— but Q5 picks the path that avoids it.

### R13 — Operator-success-reports invariants must hold

T802 / T805 / T806 / T809 / T810 (precedent in
[per-symbol-position-accounts § R8](../per-symbol-position-accounts/feature.md)).
This feature touches none of those code paths; invariants hold by
inspection. Verification = existing tests stay green.

### R14 — Live-cockpit-unified invariants must hold

T901 / T903a-d / T905 / T906–T908 / T910 / T911 / T912. Tape gains
click affordance but reads stay read-only (T906–T908 ✓);
paper-engine `on_fill` order unchanged (T903a ✓); kill-switch /
mode forwarder untouched (T905, T911 ✓). Verification = V7.

### R15 — Consistency tests stay green

Per principles
[consistency enforcement](../ui-design-principles.md#consistency-enforcement)
+ existing build-break guards in
[`crates/ui/tests/consistency.rs`](../../crates/ui/tests/consistency.rs):

- `no_inline_hex_colors_in_widgets_or_state` — every modal color
  flows from `theme::color::*` (R6 token additions land in `theme.rs`
  if architect agrees Q3).
- `no_inline_user_visible_strings_in_widgets` — every string from
  `ui::strings` (R7).
- `Message::*` exhaustiveness — adding `TapeRowClicked`,
  `TapeAuditModalClosed`, and any intermediate
  `TapeAuditEntriesLoaded` arms (architect picks count, Q6) compiles
  only if every `update` arm is added. No `_ =>` catch-all.

## Verification (V-items)

### V1 — Click → modal opens with correct entries

Integration test (new file in `crates/ui/tests/`). Boot fixture
cockpit with synthetic ledger containing one fill, emit
`Message::TapeRowClicked(known_tx_id)`, assert
`model.tape_audit_modal == Some(PanelState::Ready(view))` AND
`view.entries.len() == 4` (V8 fixture) AND `view.transaction_id ==
known_tx_id`.

### V2 — Table renders debits / credits / currency / account_id

Panel snapshot (new) for the modal in compact density on the V8
fixture. Snapshot: `tape_audit_modal_ready_paper_fill.snap`.

### V3 — Empty entries → empty state, no crash

`JournalTransactionView` with `entries: vec![]` → rendered text
contains `TAPE_AUDIT_MODAL_EMPTY` AND **not** column headers.

### V4 — Query failure → error state, cockpit doesn't crash

Inject `LedgerError` into
`Message::TapeAuditEntriesLoaded(Err(e))`; assert
`model.tape_audit_modal == Some(PanelState::Error(_))` AND text
contains `TAPE_AUDIT_MODAL_ERROR_PREFIX` AND the rest of the
cockpit (P&L, positions, kill, …) renders without panic.

### V5 — Three close paths all close the modal

V5a: `Message::TapeAuditModalClosed` (funnel for click-outside /
Close-button / Esc per R4) → `tape_audit_modal == None`. V5b:
keyboard `Esc` may emit a separate `Message::EscPressed` translated
inside modal context (architect Q6) — separate test if so. V5c:
open → close → open new tx → second open replaces first cleanly
(no stale leak).

### V6 — Anchors 11/11 PASS

`bash scripts/verify_anchors.sh`; output `ANCHORS PASS  (11 / 11)`,
zero diffs vs `spec/anchors.toml`. Regression gate.

### V7 — Existing UI tests stay green

`cargo test -p ui` AND `cargo test -p ui --features live` AND
`cargo test --workspace` — zero failures. Four
`panel_snapshots__tape_*` stay green (R11). Two `consistency` stay
green (R15). `cockpit_live_kill_button_writes_audit` stays green
(R14 / T911).

### V8 — Modal snapshot in compact density on a 4-entry fixture

Single fixture transaction (paper-fill shape — see R3 example
table). Header: `tx_id = 4f9a2c1e-…`, `ts = 2026-05-03T14:32:18Z`,
`description = "Buy 0.4 BTCUSDT @ 52,341.20"`,
`strategy_id = "sma-cross-btc-1m"`. Snapshot
`tape_audit_modal_ready_paper_fill.snap`. Two consecutive runs →
byte-identical (determinism on number formatting).

### V9 / V10 — T802/T805/T806/T809/T810 + T901/T903a-d/T905/T906–T908/T910/T911/T912 invariants

Covered by existing `cargo test --workspace` + `cargo test -p ui
--features live` (R13 + R14).

### V11 — New audit reader unit-tested

`crates/audit/tests/journal_entries_for_transaction.rs` (new).
V11a: known transaction → correct `Vec<JournalEntry>` with
consistent ordering (architect picks `id` or insertion `ts`).
V11b: unknown `tx_id` → `Ok(vec![])` (empty success, not `Err`).
V11c: balance invariant `Σ debit == Σ credit` on the returned vec
(re-asserts `verify_balance` on reader output, guards against
partial-row leak).

## Backtest scenarios

_n/a — UI feature, no new backtest scenarios. Existing 9 backtest
anchors guard rendering / strategy / audit-write-path drift; this
feature touches none._

## Open questions for architect

### Q1 — iced 0.14 modal pattern

Shipped cockpit has **zero modals** today (kill-confirm UX inlines
typed-input via `KillState::Confirming` panel-content swap, see
[`crates/ui/src/widgets/kill.rs`](../../crates/ui/src/widgets/kill.rs)).
This feature is the cockpit's first true modal. Workspace grep
returns zero existing modal/overlay. Three plausible iced 0.14
patterns: (1) `iced::widget::Stack` — z-stack, hand-rolled
backdrop, iced-native; (2) `iced_aw::Modal` — third-party
drop-in, adds a workspace dep not currently present; (3)
hand-rolled overlay column — conditional render in top-level
`view()`, full-bleed `Container` with `bg_overlay`, modal centered.

**Analyst recommendation:** **(1) `Stack`** — pure iced, no new
deps, matches "no extra crates" implicit rule. (3) also fine if
simpler. Avoid (2) unless architect wants `iced_aw` for other
reasons.

### Q2 — Where does `JournalEntry` (un-collapsed) live?

`core::JournalEntryView` collapses `(debit, credit)` → signed
`amount`. Modal needs un-collapsed pair. (1) **new type**
`core::JournalEntry { account_id, debit, credit, currency, ts,
memo }` in `crates/core/src/views.rs`; used by new reader + modal;
(2) extend `JournalEntryView` with `debit` / `credit` alongside
`amount` (existing `Serialize` shape ripple risk); (3) reuse
`JournalEntryView` with sign-as-side rendering (lossy for "exact
debit/credit pair").

**Analyst recommendation:** **(1) new type** — surgically additive,
doesn't touch existing readers, more honest data shape for 4-col view.

### Q3 — Theme tokens — land in this feature?

Three additive tokens (`bg_overlay`, `info`, `border_strong`)
proposed in
[principles color palette](../ui-design-principles.md#color-palette);
hex values locked, status `propose`.

**Analyst recommendation:** land all three here. First concrete
consumer; bundling avoids a two-trip "add token / use token"
handoff. A separate tokens-only feature would be the smallest in
project history.

**Counter-argument:** if architect wants zero `theme.rs` churn,
bump to follow-up — modal falls back to `border` / `bg` at slightly
worse layering. V-items green either way.

### Q4 — Column order in the modal table

R3 proposes `(Account, Debit, Credit, Currency)`. Alternatives:
`(Account, Currency, Debit, Credit)` (currency next to account,
debit/credit adjacent), or add a 5th `Memo` column (DB has `memo
TEXT` per
[`migrations/001:28`](../../crates/audit/migrations/001_chart_of_accounts.sql)
but typically empty for paper fills).

**Analyst recommendation:** stick with R3 for v1; defer memo to v2.

### Q5 — How does the cockpit get `transaction_id` for a tape row?

`core::FillView` does NOT carry `transaction_id` today. Three
plumbings: (1) extend `FillView` with `transaction_id: String`
plumbed through `audit::query::recent_fills` +
`ui::live::fill_to_view` (touches `core`, `audit::query`,
`ui::live`; verify `FillView` `Serialize` ripples are safe);
(2) side-channel parallel `Vec<String>` keyed by row index (less
invasive but two-vec sync); (3) verify `core::Fill` already has a
1:1 transaction-keying field and pipe it through `FillView`.

**Analyst recommendation:** **(1) extend `FillView`** — cleanest.
Architect verifies `core::Fill` ↔ `journal_transactions.id` 1:1
mapping (should hold — `audit::post_fill` writes one transaction
per fill). Bundle in this feature; splitting would yield a feature
whose only output is "FillView has one new field".

### Q6 — Keyboard focus management

How does the modal "absorb" keyboard so arrow / tab / enter don't
leak to the cockpit beneath? (1) conditional `Subscription`
dispatching to modal arm when open; (2) single `Subscription`,
branching `update`; (3) iced built-in `focus_next` /
`focus_previous` plus a modal-only focus chain. Plus: does the
modal **grab focus on open** (auto-Tab cycle inside), or only
**intercept Esc**?

**Analyst recommendation:** architect picks; (2) is simplest. Grab
focus on open per principles "focus rings visible from
modal-appear".

### Q7 — Generic vs specific modal widget?

This feature ships a `journal_transaction_modal`. Future positions
+ strategy-events drilldowns will need similar modals.
**Analyst recommendation:** **specific** until a third consumer
materializes (per principles "three-uses rule"). Generic modals
underdeliver on each case (columns / header / copy differ);
refactor on the third.

### Q8 — Test plan

New tests: snapshot `tape_audit_modal_ready_paper_fill.snap` (V8);
integration for click → modal flow (V1, V3, V4, V5); audit unit
for `journal_entries_for_transaction` (V11). No new backtest. Run
order: `cargo test -p audit` → `cargo test -p ui` → `cargo test -p
ui --features live` → `cargo test --workspace` → `bash
scripts/verify_anchors.sh`. Architect confirms or splits.

### Q9 — Anything else?

- **Modal close on agent halt** — keep open (operator may be
  reading) or auto-close? Recommend keep-open; modal is read-only.
- **Multi-modal stacking** — clicking strategy-id header opens a
  strategy drilldown FROM the modal? Recommend "no" for v1
  (close-then-open); positions/strategy modals are future features.
- **Copy-to-clipboard** for transaction-id (SQL paste). Trivial
  via `iced::clipboard`. Defer unless architect bundles.

## Design

This feature is the cockpit's first true modal and the first feature
to land against [`spec/ui-design-principles.md`](../ui-design-principles.md).
It has three architectural surfaces:

1. **Backend additive surface in `core` + `audit`** — one new struct,
   one new field, one new reader.
2. **UI modal pattern (precedent-setting)** — `iced::widget::Stack`
   overlay; documents how every future "click-through-to-audit"
   modal will be built (`positions_modal`, `strategy_history_modal`,
   etc.).
3. **Three new theme tokens** — bundled per Q3 to avoid a two-trip
   "add token / use token" handoff. Tokens are additive; existing
   panels are byte-identical (V7).

Anchor risk is zero: backend touches are additive and the 11 anchored
reports never round-trip through `fill_to_view` / the live tape (R12).

### Q1 — iced 0.14 modal pattern: `iced::widget::Stack`

**Decision.** Use `iced::widget::Stack` (shipped with our pinned
`iced = "=0.14.0"`, verified via `Cargo.lock` `iced_widget = "0.14.2"`)
to z-stack the modal overlay on the cockpit body. The Stack's bottom
child is the existing cockpit `Column`; the top child is a full-bleed
`Container` styled with `bg_overlay` (captures backdrop clicks →
`Message::TapeAuditModalClosed`) wrapping a centered "modal card"
`Container` at ~480 px width framed by `border_strong`. Library
checklist: pure iced, no new workspace dep, no system C dep, no
edition issue.

**Rationale.** Pure iced + the Stack child only renders when
`Cockpit.tape_audit_modal == Some(_)`, so the cockpit body's iced
tree is byte-identical to today when the modal is closed (V7 stays
green by construction).

**Rejected.** `iced_aw::Modal` (adds a workspace dep historically
lagging iced minor bumps); hand-rolled overlay column (works but
Stack is the upstream-blessed pattern for absorbing pointer/keyboard
events at the right z-layer).

### Q2 — `JournalEntry` (un-collapsed) lives in `trading_core`

**Decision.** New `pub struct JournalEntry` in `crates/core/src/views.rs`:

```rust
pub struct JournalEntry {
    pub account: AccountId,
    pub debit:   Money<Usdt>,   // zero when this row is a credit
    pub credit:  Money<Usdt>,   // zero when this row is a debit
    pub currency: SmolStr,      // ISO/crypto ticker — "USDT", "BTC", …
    pub ts: Timestamp,
    pub memo: SmolStr,
}
```

`JournalEntryView` (signed-amount collapse) **stays** for its current
consumers (`recent_journal`, etc.).

**Rationale.** Surgically additive. The modal needs the un-collapsed
`(debit, credit)` pair to render a 4-col table; deriving sign from a
single signed `Decimal` is lossy for hand-typed fixture rows of `0`.
`core` is already the home of read-side view types — keeps `audit`
and `ui` from declaring their own view types in parallel.

**Currency type.** v0–v1.5a money math is `Money<Usdt>` only; the
new struct keeps `Money<Usdt>` for `debit`/`credit` and a separate
`SmolStr` `currency` for the display ticker ("BTC"/"USDT"). This
avoids dragging a generic `Currency` trait param through the view
layer.

**Rejected.** Extending `JournalEntryView` with parallel `debit`/`credit`
fields — its `Serialize` shape is reachable via `recent_journal` and
risks drift on operator-success-report bytes (anchored). Reusing
`JournalEntryView` with sign-as-side rendering — lossy for zero-amount
rows.

### Q3 — Theme tokens: land all three here

**Decision.** Add `bg_overlay = #0B0D12`, `info = #7BC2FF`,
`border_strong = #3A4456` to `crates/ui/src/theme.rs::color` (dark
hex from principles doc — concrete, not TBD). Land them in this
feature.

**Rationale.** The modal is the first concrete consumer of all
three; bundling avoids a two-trip "add token / use token" handoff.
Tokens are additive — `theme::color::*` is a closed-set namespace,
so adding a new `pub const` cannot drift any existing widget's
render (V7 holds; no existing snapshot inspects the new tokens).

**Light-mode hex** is documented in the principles doc but lands
with the broader light-mode feature; today's `theme.rs` is
dark-only by construction. Migration when light mode lands is
mechanical (`pub const` → `pub fn token(mode) -> Color`).

**Rejected.** Bump to follow-up "design system v2" — defers a
3-line `theme.rs` edit by one round-trip with zero net benefit.

### Q4 — Column order: `Account | Debit | Credit | Currency`

**Decision.** Analyst's R3 order. Number cells (debit, credit)
right-aligned, monospace digits, locale-default thousands separator
via the existing `widgets::num` formatter (per principles "Numbers
are scannable"). Account text left-aligned monospace (account-id
paths are colon-delimited, monospace aids scan); currency centered.

**Rationale.** Operators read `(account, dr, cr)` left-to-right;
currency is meta-data ("what unit are these in") and belongs at
the right edge. Matches double-entry bookkeeping mental model.
Memo deferred — empty for paper fills in v0–v1.5a; would force a
5th column for currently-zero info.

**Rejected.** `(Account, Currency, Debit, Credit)` — currency-first
feels form-y, not scannable. 5-col with `Memo` — horizontal pressure
for an empty column.

### Q5 — `transaction_id` plumbing path

**Decision.** Three additive surface changes:

1. **`core::FillView` gains `pub transaction_id: SmolStr`** —
   additive field; `SmolStr` matches existing conventions. No
   removed/reordered fields; `Serialize` round-trips cleanly.
2. **`core::Fill` gains `pub transaction_id: Option<SmolStr>`** —
   `None` at construction (backtests, paper engine pre-write);
   `Some(txn_id)` after the audit write succeeds. The bus carries
   the populated `Fill`; `ui::live::fill_to_view` reads the field
   (defaulting to empty `SmolStr` for the `None` case).
3. **`audit::journal::post_fill` return type bumped from
   `Result<(), LedgerError>` to `Result<SmolStr, LedgerError>`**
   — returns the generated `journal_transactions.id` UUID string.
   The `crates/agent/src/runtime.rs` live-mode glue stamps
   `fill.transaction_id = Some(txn_id)` between the audit write
   and `engine.on_fill` ([architecture "Cockpit ← EventBus"](../architecture.md#cockpit--eventbus)
   T802 ordering preserved). All existing call sites become
   `let _ = post_fill(...).await?;` (mechanical, ~7 sites in
   `crates/audit/tests/*`).

**Boot snapshot path** (`audit::query::recent_fills`) is even
simpler — `txn_id` is already SELECTed at line 138 and threaded
to `parse_fill_view_from_description` (line 162); the `FillView`
constructor at line 213 just adds the field. Zero new SQL cost.

**Anchor invariance.** Backtests construct `PaperEnginePublisher`
with `NullPublisher` — the live-mode `transaction_id` stamp never
fires on the backtest path. `crates/backtest/` and
`crates/reports/` never call `fill_to_view` and never render
`FillView` into report bodies. Independent grep over
`crates/reports/src/` for `FillView` confirms no consumer beyond
the ledger reader's own scope. **Zero anchor risk.**

**Snapshot invariance.** Existing `panel_snapshots__tape_*` are
produced by `tape_summary` (in `crates/ui/tests/panel_snapshots.rs:335-365`),
which renders `venue_ts | symbol | side | price | qty` — the new
`transaction_id` field is not inspected. V7 holds.

**Rejected.** Side-channel parallel `Vec<String>` keyed by row
index (two-vec sync, drifts under pause/resume buffer drains);
click-time `(symbol, side, venue_ts)` reverse lookup (fragile —
multiple fills can share a millisecond; ledger lacks the index).

### Q6 — Keyboard absorption: subscription on modal-open

**Decision.** When `Cockpit.tape_audit_modal == Some(_)`, the
cockpit's `subscription()` adds an `iced::keyboard::on_key_press`
listener:

- `Esc` → `Message::TapeAuditModalClosed`.
- `Tab` / `Shift-Tab` → iced built-in focus chain scoped to the
  modal `Container`.
- Arrow / Page-Up / Page-Down → consumed
  (`event::Status::Captured`) while modal is open. The tape has
  no keyboard binding today, so nothing regresses.
- `Enter` on focused `Close` → built-in `Button::on_press`.

When the modal closes, `subscription()` observes
`tape_audit_modal == None` and emits zero keyboard recipes;
focus returns to the tape's pause toggle (the only focusable
widget on the cockpit today). On open, focus auto-targets the
`Close` button via `iced::widget::focus_next` issued from the
`TapeRowClicked` arm (per principles "focus rings visible from
modal-appear").

**Rationale.** iced 0.14's `Subscription` is recomputed on every
state change; gating the recipe on `Option<JournalModalState>`
is the canonical pattern.

**Rejected.** Always-on subscription branching in `update` —
pollutes the cockpit handler with modal logic when no modal is
open; harder to extend for future modals.

### Q7 — Specific `JournalTransactionModal` widget

**Decision.** Specific. New file
`crates/ui/src/widgets/journal_transaction_modal.rs` exporting
`pub fn view<'a>(state: &'a JournalModalState) -> Element<'a, Message>`.

**Rationale.** Per the principles' three-uses rule, generic
before three concrete consumers underdelivers per case. Future
positions-drilldown and strategy-events-drilldown modals share
*structure* (Stack + `bg_overlay` + `border_strong` + Esc-close)
but differ in *content* (columns, header rows, copy). The third
consumer is the trigger to refactor shared structure into
`widgets::modal::overlay(content)`.

**Rejected.** Generic `widgets::modal::Modal<T>` — premature.

### Q8 — Test plan

Three new test files; existing snapshots stay byte-identical.

| Test | Location | Asserts | V-item |
|------|----------|---------|--------|
| Audit reader unit | `crates/audit/tests/journal_entries_for_transaction.rs` (NEW) | 4-entry fixture → `Vec<JournalEntry>` ordered by `journal_entries.id ASC`; unknown `tx_id` → `Ok(vec![])`; `Σ debit == Σ credit` | V11 |
| UI integration | `crates/ui/tests/tape_row_click_opens_modal.rs` (NEW) | `Message::TapeRowClicked` → `tape_audit_modal == Some(Loading)`; `TapeAuditEntriesLoaded(Ok(view))` → `Ready(view)` with `view.entries.len() == 4` | V1, V3, V4, V5 |
| Modal snapshot | `crates/ui/tests/panel_snapshots.rs` (extend) + `panel_snapshots__tape_audit_modal_ready_paper_fill.snap` (NEW) | Compact density, 4-entry fixture, byte-identical across re-runs | V2, V8 |
| Existing tape snapshots | `panel_snapshots__tape_*` | Byte-identical (`transaction_id` not in `tape_summary`) | V7, R11 |
| Existing consistency | `consistency.rs` | No inline hex / strings in widgets stays green | R15 |

**Run order**: `cargo test -p audit` → `cargo test -p ui` →
`cargo test -p ui --features live` → `cargo test --workspace`
→ `bash scripts/verify_anchors.sh`.

**Reader determinism**: ORDER BY `journal_entries.id ASC` —
UUIDs sort lexicographically; stored verbatim from
`Uuid::new_v4()`; stable across runs.

### Q9 — Adjacent concerns

**Modal close on agent halt.** `Message::AgentHaltedExternally`
also clears `tape_audit_modal` (the operator's attention belongs
on the halt banner, not stacked behind a read-only modal). Audit
data is still queryable post-halt via the same row click.

**Multi-modal stacking.** Only one modal at a time. A
`TapeRowClicked` while the modal is open replaces identity
unconditionally (`tape_audit_modal = Some(Loading)` for the new
tx_id, previous state discarded). No back-stack — the cockpit
is an instrument, not a browser.

**Clipboard (`Cmd-C`).** Deferred. The focus-chain "which row
is selected" semantics turn this into a 30+ minute effort;
revisit when the operator asks.

### Crate map delta

| Crate / file | Change | Public surface? |
|--------------|--------|-----------------|
| `crates/core/src/views.rs` | new `pub struct JournalEntry { account, debit, credit, currency, ts, memo }` | YES |
| `crates/core/src/views.rs` | `FillView` gains `pub transaction_id: SmolStr` | YES (additive field) |
| `crates/core/src/fill.rs` | `Fill` gains `pub transaction_id: Option<SmolStr>` | YES (additive field, `None` until populated by runtime) |
| `crates/core/src/lib.rs` | re-export `JournalEntry` | YES |
| `crates/audit/src/query.rs` | new `pub async fn journal_entries_for_transaction(&Ledger, &str) -> Result<Vec<JournalEntry>, LedgerError>` | YES |
| `crates/audit/src/query.rs` | `recent_fills` populates `transaction_id` from already-SELECTed `txn_id` | NO (signature unchanged) |
| `crates/audit/src/journal.rs` | `post_fill` returns `Result<SmolStr, LedgerError>` (was `Result<()>`) — the generated `txn_id` | YES (return-type change) |
| `crates/agent/src/runtime.rs` (live-mode glue ~line 429+) | populate `fill.transaction_id` from `post_fill`'s return before `engine.on_fill` | NO (internal sequencing only) |
| `crates/ui/src/theme.rs::color` | new `pub const BG_OVERLAY`, `INFO`, `BORDER_STRONG` | YES |
| `crates/ui/src/strings.rs` | new modal-copy constants (R7) — 13 entries; appended to `all()` | YES |
| `crates/ui/src/widgets/journal_transaction_modal.rs` (NEW) | `pub fn view<'a>(state: &'a JournalModalState) -> Element<'a, Message>` | YES |
| `crates/ui/src/widgets/mod.rs` | `pub mod journal_transaction_modal` | YES |
| `crates/ui/src/widgets/tape.rs` | wrap each row in `Button::on_press(Message::TapeRowClicked(fill.transaction_id.clone()))` | NO |
| `crates/ui/src/state.rs` | `Message::TapeRowClicked(SmolStr)`, `Message::TapeAuditModalClosed`, `Message::TapeAuditEntriesLoaded(Result<JournalTransactionView, SmolStr>)` (3 new variants) | YES |
| `crates/ui/src/state.rs` | `Cockpit.tape_audit_modal: Option<JournalModalState>` | YES |
| `crates/ui/src/state.rs` | new `pub struct JournalModalState { tx_id: SmolStr, entries: PanelState<JournalTransactionView> }` (and the `JournalTransactionView` carrying header rows + entries) | YES |
| `crates/ui/src/live.rs` | `fill_to_view` reads `fill.transaction_id` (defaulting to empty `SmolStr` when `None` — fixture-mode resilience) | NO (signature unchanged) |
| `crates/ui/src/fixtures.rs` | `fake_fill_view` populates a deterministic `transaction_id` (e.g. `format!("fixture-tx-{n}")`) for snapshot stability | NO (signature unchanged) |

### Public API additions

```rust
// trading_core
pub struct JournalEntry { pub account: AccountId, pub debit: Money<Usdt>,
    pub credit: Money<Usdt>, pub currency: SmolStr, pub ts: Timestamp,
    pub memo: SmolStr }
pub struct FillView { ..., pub transaction_id: SmolStr }
pub struct Fill { ..., pub transaction_id: Option<SmolStr> }

// audit
pub async fn journal_entries_for_transaction(
    ledger: &Ledger, tx_id: &str,
) -> Result<Vec<JournalEntry>, LedgerError>;
pub async fn post_fill(
    ledger: &Ledger, fill: &Fill, strategy_id: Option<&str>,
) -> Result<SmolStr, LedgerError>; // was Result<(), _>

// ui::theme::color
pub const BG_OVERLAY: Color = rgb(0x0B, 0x0D, 0x12);
pub const INFO: Color = rgb(0x7B, 0xC2, 0xFF);
pub const BORDER_STRONG: Color = rgb(0x3A, 0x44, 0x56);

// ui::state
pub struct JournalModalState { pub tx_id: SmolStr,
    pub entries: PanelState<JournalTransactionView> }
pub struct JournalTransactionView { pub tx_id: SmolStr, pub ts: Timestamp,
    pub description: SmolStr, pub strategy_id: Option<StrategyId>,
    pub entries: Vec<JournalEntry> }
pub enum Message { ..., TapeRowClicked(SmolStr), TapeAuditModalClosed,
    TapeAuditEntriesLoaded(Result<JournalTransactionView, SmolStr>) }

// ui::widgets::journal_transaction_modal
pub fn view<'a>(state: &'a JournalModalState) -> Element<'a, Message>;
```

### Modal state shape

```rust
/// Modal-only sub-state — the full PanelState<T> machinery applies
/// (loading | populated | empty | error) per R8.
#[derive(Debug, Clone)]
pub struct JournalModalState {
    /// The transaction id the modal is rendering. Populated at click
    /// time and carried as the modal's identity until close.
    pub tx_id: SmolStr,
    /// The entries panel state — first arrives as Loading, flips to
    /// Ready(view) on TapeAuditEntriesLoaded(Ok), Error on Err, Empty
    /// when entries.is_empty() (defensive — every transaction has
    /// >= 2 entries by audit invariant).
    pub entries: PanelState<JournalTransactionView>,
}
```

The `PanelState<JournalTransactionView>` re-uses the existing
`PanelState<T>` enum (R8 "no blank screens"). Click flow:

1. `Message::TapeRowClicked(tx_id)` → set `tape_audit_modal =
   Some(JournalModalState { tx_id, entries: PanelState::Loading })`,
   issue async `iced::Task::perform(...)` reading
   `audit::query::journal_entries_for_transaction`.
2. `Message::TapeAuditEntriesLoaded(Ok(view))` → if
   `view.entries.is_empty()` set `entries = PanelState::Empty`
   else `PanelState::Ready(view)`.
3. `Message::TapeAuditEntriesLoaded(Err(msg))` → `entries =
   PanelState::Error(msg)`.
4. `Message::TapeAuditModalClosed` → `tape_audit_modal = None`.
5. `Message::AgentHaltedExternally(_)` → in addition to the existing
   halt-state mutation, also set `tape_audit_modal = None`.

### Test strategy per V-item

| V | Test file | Fixture | Asserts |
|---|-----------|---------|---------|
| V1 | `crates/ui/tests/tape_row_click_opens_modal.rs` (NEW) | `ui::fixtures::fake_cockpit_ready_with_three_fills` (extend with deterministic `transaction_id`) + a synthetic `JournalTransactionView` builder | `Message::TapeRowClicked(known_tx_id)` → `model.tape_audit_modal == Some(_)`, `tx_id` matches |
| V2 | `crates/ui/tests/panel_snapshots.rs` (extend) | new fixture `fake_journal_modal_ready_paper_fill()` returning a `JournalModalState::Ready` with 4 entries | snapshot `tape_audit_modal_ready_paper_fill.snap` byte-identical across two runs (V8) |
| V3 | `tape_row_click_opens_modal.rs` | empty `Vec<JournalEntry>` | `entries == PanelState::Empty`, modal renders `TAPE_AUDIT_MODAL_EMPTY` copy, no column header text |
| V4 | `tape_row_click_opens_modal.rs` | injected `LedgerError` | `entries == PanelState::Error(_)`, error copy includes `TAPE_AUDIT_MODAL_ERROR_PREFIX`, P&L card / positions panel render without panic (smoke `view()`) |
| V5 | `tape_row_click_opens_modal.rs` (3 sub-tests) | open/close/reopen sequence | V5a: `TapeAuditModalClosed` → `None`. V5b: open new tx via `TapeRowClicked` while open → modal flips identity to new tx_id, no stale data leaks. V5c: open then `AgentHaltedExternally` → `None` |
| V6 | `bash scripts/verify_anchors.sh` | n/a | `ANCHORS PASS (11 / 11)` byte-identical |
| V7 | existing `panel_snapshots__tape_*`, `consistency.rs`, `cockpit_live_kill_button_writes_audit` | n/a | All green; no diffs |
| V8 | (same as V2) | 4-entry fixture: `assets:cash:USDT` Cr 52341.20, `assets:position:BTCUSDT` Dr 0.40, `expense:fees:taker` Dr 5.23, `assets:cash:USDT` Cr 5.23 | snapshot byte-identical across re-runs |
| V9 / V10 | `cargo test --workspace` (existing tests) | n/a | All green |
| V11 | `crates/audit/tests/journal_entries_for_transaction.rs` (NEW) | fresh in-memory ledger + post one paper Buy fill (4 entries) | V11a: returns 4 entries ordered by `journal_entries.id ASC`. V11b: unknown tx_id → `Ok(vec![])`. V11c: `Σ debit == Σ credit` |

### Risks & mitigations

1. **`iced::widget::Stack` ergonomics in 0.14.0 are unfamiliar.**
   Mitigation: Stack is documented and shipped with `iced 0.14.0`
   (verified via `Cargo.lock` `iced_widget = "0.14.2"`); a 30-line
   spike in the `cockpit --features fixtures` binary verifies the
   click-outside backdrop semantics before T1205 commits the widget.
   If Stack pointer-event capture is buggy in 0.14.0 specifically
   (no known issue, but iced's z-stack pointer order has historically
   been a sharp edge), fall back to the hand-rolled overlay-column
   pattern (Q1 alt 3) — same `bg_overlay` token, same widget API,
   different wrapper.

2. **`FillView::transaction_id` propagation breaks an existing
   consumer.** Mitigation: the field is **additive** (no removed
   fields, no reordered fields). `Serialize`/`Deserialize` derive
   on `FillView` continues to round-trip. The only consumers that
   shape-match are: `tape_summary` in `panel_snapshots.rs` (does
   not name `transaction_id`, so unaffected), the live tape's
   `row_for(fill)` (constructs cells positionally — unaffected),
   the `recent_fills` SQL projection (already SELECTs `txn_id`).
   A grep over `crates/` for `FillView {` is the verification.

3. **`Fill::transaction_id` becomes `Option<SmolStr>` and adds an
   `.unwrap_or_default()` in places that don't have a real
   `txn_id` yet.** Mitigation: type system enforces `Option`
   handling; the live runtime path stamps `Some(txn_id)` AFTER the
   audit write succeeds; the fixture path stamps a deterministic
   `Some("fixture-tx-{n}")` so snapshots are stable; backtests
   construct `Fill` with `transaction_id: None` and never call
   `fill_to_view` so the `None` is harmless. A `clippy::expect_used`
   audit confirms no `.unwrap()` on the field.

4. **`post_fill` return-type change ripples to every test caller.**
   Mitigation: the change is from `Result<(), E>` to `Result<SmolStr, E>`
   — every existing call site is `post_fill(...).await?`; under the
   new shape these become `let _txn_id = post_fill(...).await?;`
   (compiler emits an `unused_result` *warning*, not error; with
   `-D warnings` the developer adds `let _ = ...` two-char edit per
   call site). Mechanical, no behavioral change. Test count: 7 prod
   call sites in `crates/audit/tests/*` per the existing grep
   (`per_symbol_post_fill.rs`, `open_positions_at.rs`,
   `ledger_integration.rs`).

5. **Theme tokens used in widgets BEFORE light mode lands.**
   Mitigation: dark-mode hex values are concrete and locked in the
   principles doc (verified: `#0B0D12`, `#7BC2FF`, `#3A4456` —
   not TBD). The `theme::color::*` namespace is dark-only by
   construction today; the future light-mode feature adds a
   `ThemeMode` enum and a parallel `light::*` block. Adding three
   dark-only constants now does not block the light-mode landing
   later — the migration is `pub const BG_OVERLAY: Color = …` →
   `pub fn bg_overlay(mode: ThemeMode) -> Color { match mode { … } }`,
   mechanical and confined to `theme.rs`.

6. **Snapshot drift on existing panels from the 3 new theme
   tokens.** Mitigation: the existing `panel_snapshots__*` use
   `tape_summary` / `pnl_summary` / `positions_summary` /
   `kill_summary` / `latency_summary` / `strategies_summary` (all
   in `crates/ui/tests/panel_snapshots.rs:335+`); none of them
   inspect or include `bg_overlay` / `info` / `border_strong`.
   Confirmed by independent grep over the snapshot helper bodies.
   New theme constants are pure additions to a closed-set
   namespace — zero render-path side effect. Verified via
   reading the snapshot helpers; `default()` constructors do not
   touch the new tokens.

7. **Keyboard subscription leaks across modal open/close cycles.**
   Mitigation: subscription is recomputed on every `update` per
   iced's contract; gating the keyboard recipe on
   `Cockpit.tape_audit_modal.is_some()` is the canonical pattern.
   No persistent subscription handle to leak. Tested by V5c
   (open-then-halt → close, then open new modal → keyboard works
   again).

### Operator-success-reports + live-cockpit-unified invariants that must hold

This feature MUST preserve the following invariants from prior
features. Each is verified by an existing test that stays green:

| Invariant | Provenance | Verification |
|-----------|------------|--------------|
| T802 — `post_fill` writes journal-transaction + entries dual-write | operator-success-reports | `crates/audit/tests/ledger_integration.rs` (existing) — passes with the new return-type signature |
| T805 — feed-reconnect writes `strategy_events` row | operator-success-reports | `crates/audit/tests/feed_reconnect_test.rs` (existing) |
| T806 — agent-uptime open/heartbeat/close lifecycle | operator-success-reports | `crates/audit/tests/uptime_intervals_test.rs` (existing) |
| T809 — kill-switch dual-write (memo + strategy_event) | operator-success-reports | `crates/audit/tests/kill_switch_dual_write_test.rs` (existing) |
| T810 — `--features in_process_cron` builds clean | operator-success-reports | `cargo build -p agent --features in_process_cron` |
| T901 — agent runtime → bus event push | live-cockpit-unified | `cargo test -p agent` |
| T903a-d — paper engine / data feed / reconciler / forwarder bus wiring | live-cockpit-unified | `cargo test -p agent --test t903*` |
| T905 — mode-broadcast forwarder | live-cockpit-unified | existing test |
| T906 — kill button writes audit | live-cockpit-unified | `crates/ui/tests/cockpit_live_kill_button_writes_audit.rs` |
| T907–T908 — cockpit binaries gating | live-cockpit-unified | `cargo build` matrix |
| T910 / T912 — subprocess-launch tests | live-cockpit-unified | `cargo test -p ui --features live` |
| T911 — kill-switch ↔ cockpit observation | live-cockpit-unified | existing |

The `post_fill` signature change (Q5) is the only invariant-touching
delta — and it is **return-type-only**, not behavior-altering. T802's
journal shape is byte-identical (still writes one
`journal_transactions` row + N `journal_entries` rows in one SQL
transaction). T809's kill-switch dual-write does not call
`post_fill`. The rest of the invariants do not touch the audit
writer's return type.

### UI principles compliance

This is the **first feature against the principles doc**.
Documenting precedent that future features inherit:

- **Show the why** (principles ¶ "Show the why"): the entire
  feature exists to satisfy this principle. Every tape row is
  click-through to its `journal_transaction`. Precedent for
  positions / strategies follows.
- **No blank screens** (¶ "No blank screens"): modal carries all
  four `PanelState<T>` variants — `Loading` (sub-ms IRL but the
  state exists for resilience), `Empty` (defensive), `Error`
  (carries `LedgerError` text), `Ready` (happy path). R8 captures
  the rule, the modal honors it.
- **Plain language** (¶ "Plain language"): all strings in
  `ui::strings` (R7); column headers are `Account` / `Debit` /
  `Credit` / `Currency`, not `account_id` / `debit_amount`.
- **Numbers are scannable** (¶ "Numbers are scannable"): debit /
  credit cells right-aligned, monospace digits via `widgets::num`,
  locale-default thousands separator, no sign on absolute amounts
  (debit and credit are both naturally positive — sign-of-side
  is encoded in *which column* the number sits in, not its
  sign). Zero stays `fg_muted`.
- **Iconography: no icons until needed** (¶ "Iconography"): close
  button is text `"Close"`, not an `×` glyph. Backdrop is plain
  color, no chrome.
- **Accessibility minimums** (¶ "Accessibility minimums"): `Esc`
  closes (R4 + R9); focus ring uses `border_strong` (not
  `accent`); contrast ratio for `info` (`#7BC2FF`) on `bg_elev`
  (`#1A1F29`) is 6.8:1 — clears AA 4.5:1 (verified against the
  principles doc's contrast table).
- **Color is never the only signal** (¶ "Accessibility minimums"
  sub-bullet): debit / credit columns are *labeled*, not just
  *colored*. The numbers themselves stay `fg`; column headers
  carry the meaning.
- **Density** (¶ "Density"): modal honors compact density —
  table-row 24 px, cell pad 12 px, dialog inner pad 24 px, modal
  width ~480 px (R10).
- **Motion** (¶ "Motion"): modal open/close uses 180 ms
  ease-out (Q1 + R10 implicit). No idle animation, no flicker.
- **Confirm destructive actions**: N/A — modal is read-only (R5).
- **Voice and copy** (¶ "Voice and copy"): direct, terse,
  present-tense, sentence case, unicode `…` in
  `TAPE_AUDIT_MODAL_LOADING`. No "Please", no "Sorry".
- **Dark/light parity** (¶ "Dark / light mode parity"): dark
  hex shipped now; light hex documented in principles, lands
  with the broader light-mode feature.
- **Consistency enforcement** (¶ "Consistency enforcement"):
  - All colors via `ui::theme` — `no_inline_hex_colors_in_widgets_or_state`
    test stays green (R15 + V7).
  - All strings via `ui::strings` —
    `no_inline_user_visible_strings_in_widgets` stays green
    (R15 + V7).
  - Spacing scale closed (`12 / 16 / 24` only).
  - Type scale closed (`caption / body` for table cells, `title`
    for modal title — no fifth size).
  - Color tokens semantic (`info` / `border_strong` —
    not `blue` / `gray2`).
  - `Message::*` exhaustive — three new arms (`TapeRowClicked`,
    `TapeAuditModalClosed`, `TapeAuditEntriesLoaded`) all
    handled in `update`; no `_ => {}` catch-all.

**Precedent for future click-through-to-audit modals:** The
positions-drilldown and strategy-events-drilldown features will
re-use this feature's pattern: `Stack` overlay + `bg_overlay`
backdrop + `border_strong` frame + Esc-to-close subscription +
`Message::*Clicked(id)` + `*ModalClosed` + `*EntriesLoaded`
message triplet + a per-feature `widgets::*_modal.rs` until the
three-uses rule trips.

## Implementation
_developer fills this — left blank intentionally._

## Verification — links
_tester fills this — left blank intentionally._

## UI
_ui-designer fills this. First feature to land against
[ui-design-principles.md](../ui-design-principles.md). Principles
hooks: "Show the why" (Q4 promotion), color tokens `bg_overlay` /
`info` / `border_strong`, "No blank screens" (R8), "Plain language"
(R7), density compact (R10), keyboard / focus rules (R9, Q6)._

## Changelog

- 2026-05-03 (analyst): initial draft. Promoted from
  [`spec/backlog.md`](../backlog.md) "Queue → Active" per the
  operator's 2026-05-03 decision on UI principles Q4. 15 R-items,
  11 V-items, 9 open questions for the architect. Recommends
  bundling theme-token additions (`bg_overlay`, `info`,
  `border_strong`) into this feature (Q3), bundling the
  `FillView::transaction_id` plumbing into this feature (Q5),
  picking iced `Stack` for the modal pattern (Q1), and a specific
  (not generic) modal widget (Q7). Anchor risk: zero (R12) — pure
  UI + new audit reader, no backtest path touched. HANDOFF →
  architect.
- 2026-05-03 (architect): resolved Q1–Q9. Q1 `iced::widget::Stack`
  (no new dep — verified `iced 0.14.0` ships Stack via Cargo.lock
  `iced_widget = "0.14.2"`). Q2 new `JournalEntry` struct in
  `trading_core` (additive; `JournalEntryView` unchanged). Q3 land
  all three theme tokens here (concrete dark hex from principles
  doc). Q4 column order `Account | Debit | Credit | Currency`. Q5
  extend `FillView` with `transaction_id: SmolStr`, extend
  `core::Fill` with `transaction_id: Option<SmolStr>`, return
  `txn_id` from `audit::journal::post_fill`; the live runtime
  stamps the field after the audit write succeeds; `recent_fills`
  populates from already-SELECTed `txn_id`. Q6 modal-open-gated
  `iced::keyboard::on_key_press` subscription. Q7 specific
  `JournalTransactionModal` widget (refactor on third consumer).
  Q8 three new test files (audit unit, ui integration, ui
  snapshot) + existing tape snapshots stay byte-identical. Q9
  modal closes on `AgentHaltedExternally`; one modal at a time;
  clipboard deferred. **First feature against
  [ui-design-principles.md](../ui-design-principles.md)** —
  documents the "click-through-to-audit modal" pattern that
  positions-drilldown and strategy-events-drilldown will inherit.
  Anchor risk zero (R12) confirmed by independent verification:
  the 11 anchored reports never round-trip `FillView` /
  `fill_to_view`; backtests construct `PaperEnginePublisher` with
  `NullPublisher` (no live-mode side effect into report bodies).
  Tasks T1201–T1209 + `T_FINAL_TAPE_MODAL` filed at
  [tasks/tape-row-audit-modal.md](../tasks/tape-row-audit-modal.md).
  HANDOFF → orchestrator (spawn dev for T1201 → T1202 backend
  critical path; UI-designer for T1203–T1208 once T1201 lands).
- 2026-05-03 (tester): FINAL gate green; status bumped
  `in-progress → shipped`. Test report
  `spec/archive/test-2026-05-03-1351-tape-row-audit-modal-final.md (archived; see spec/archive/README.md)`.
  V1–V11 all VERIFIED. Anchors 11/11 PASS (V6). Operator-success-reports
  invariants (T802/T805/T806/T809/T810) and live-cockpit-unified
  invariants (T901/T903a-d/T905/T906–T908/T910/T911/T912) all hold;
  per-symbol-position-accounts (T1101–T1107) hold by inspection +
  test runs. 32 existing `panel_snapshots__*` byte-identical (R11/V7);
  4 new modal snapshots (Loading/Empty/Error/Ready paper-fill) captured
  deterministically. T_FINAL_TAPE_MODAL ticked. Workspace build/fmt/
  clippy/test sweep all clean. First feature shipped against
  [ui-design-principles.md](../ui-design-principles.md); precedent for
  the click-through-to-audit modal pattern (Stack overlay + bg_overlay
  backdrop + border_strong frame + Esc-close subscription) ready for
  positions-drilldown + strategy-events-drilldown reuse. HANDOFF →
  presenter (release-mode presentation + operator approval gate).
