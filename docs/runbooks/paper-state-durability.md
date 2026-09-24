# Runbook — What survives a restart (paper state durability)

**Created:** 2026-09-24 (story 3-21 AC1). The facts below existed already, spread
across ADR-0052, the `journal` module docs and a schema comment in
`persistence.rs`. Story 3-21's complaint was that there was nowhere to *read*
them: every durable artifact in this repo has a formal contract — anchors are
byte-frozen and gated, trace rows are lint-enforced, evidence reports are
SHA-locked — except the one artifact the operator is personally invested in.

Scope: **paper / simulation only.** This repo executes no live orders.

## The contract, in one table

| What | Where on disk | Survives a kill? | Written when |
|---|---|---|---|
| Cash, positions, fills (double-entry) | `./data/audit/ledger.db` | ✅ atomic per fill | on every fill, one balanced transaction |
| Live equity series | same `ledger.db` (ADR-0052) | ✅ | per bar, in paper/live mode |
| Reflection / lesson cards | `./data/audit/reflection.db` | ✅ | per lesson |
| Lab session selection | `$XDG_CONFIG_HOME/trading/cockpit-lab-state.json`<br>(default `~/.config/trading/…`) | ⚠️ up to ~500 ms may be lost — see below | debounced 500 ms after a change |
| Lab run results, equity curves, KPI strip | — | ❌ session-scoped, recomputed | never |
| Data-source toggle, SMA params | — | ❌ not in the schema | never |
| Activity tape, toasts, panel states | — | ❌ session-scoped | never |
| Research-mode equity | — | ❌ deliberately not hydrated on boot | never |

`Money` is `rust_decimal::Decimal` behind the `Money<C>` newtype everywhere on
the money side — never `f64` (AD-9).

## The money side

Every fill writes one balanced double-entry transaction through
`audit::journal::post_fill`, which opens a SQL transaction, inserts the header
and both legs, and commits. Debits == credits is enforced *per transaction*, so
a kill between two fills leaves a coherent ledger, and a kill *inside* one
leaves the transaction uncommitted — the fill is absent, never half-applied.
The live equity series is written through the same ledger
(`audit::equity_store`, ADR-0052), so it cannot disagree with the cash it
describes.

Proof, not assertion: `crates/audit/tests/crash_consistency.rs` (story 3-21 AC2)
SIGKILLs a child process mid-transaction, with 1.38 MB of uncommitted pages
already forced onto the disk, reopens the file and asserts the reload is
coherent — exact-`Decimal` balance globally and per transaction, no orphan
header or entry, every fill carrying both its legs. Three negative controls
commit the shapes the crash must not leave behind and prove the contract catches
them, and the whole guard was RED-proven by temporarily committing the half-fill.

### Two operational facts that follow

**The ledger runs SQLite in `delete` (rollback-journal) mode, not WAL.**
`Ledger::open` builds `sqlite://{path}?mode=rwc`
(`crates/audit/src/ledger.rs:40-49`) and sets no `journal_mode` pragma. During a
transaction the *modified* pages go into the main `.db` file and their
pre-images into `<db>-journal`; recovery is an undo. So:

> **A copy of `ledger.db` taken while the cockpit is running is not a consistent
> snapshot unless `ledger.db-journal` is copied with it.** Back up the whole
> directory, or back up with the cockpit closed.

**`journal::verify_balance` is tolerance-based, not exact.** It compares
`|Σdebit − Σcredit| > dec!(0.00000001)` (`crates/audit/src/journal.rs:2099`).
That is 1e-8 — six orders of magnitude below a cent, so it is effectively exact
for money, but AD-9's "exact-cent reconciliation" is an epsilon in the code, not
an equality. The AC2 contract asserts exact `Decimal` equality, which is
strictly stronger; the production function was not changed.

**Not claimed at all:** media-level torn writes or a lying `fsync`, a kill inside
SQLite's own commit-marker `fsync`, filesystem loss, or concurrent multi-process
writers. The paper session has exactly one writer.

## The session side, and its one known gap

The Lab selection — schema version, strategy, pair, date range, compare set,
training-panel state — is JSON at
`$XDG_CONFIG_HOME/trading/cockpit-lab-state.json`. `Cockpit::boot` reads it at
launch; `ui::state::update` writes it 500 ms after the last change.

**Gap, measured and accepted:** `iced::application(..).run()` consumes the
application state, so there is no correct place to force a final flush at window
close. A selection changed in the last ~500 ms before the window closes is lost.
Nothing else is.

**If the file cannot be read** — a schema version this cockpit does not
understand, or bytes that do not parse — the cockpit does three things, in this
order:

1. **moves the file aside** to `cockpit-lab-state.json.unreadable-<unix-seconds>`,
   *before* the debounced writer can overwrite it with cold-start defaults. This
   is the point of the whole mechanism: without it a failed load would DESTROY
   the saved session rather than ignore it;
2. starts from cold-start defaults (`v0.sma × BTCUSDT × Last 90d`);
3. says so on the Lab screen, in the failure colour, naming the reason and the
   path the old file was kept at.

To recover by hand, inspect the kept-aside file and copy anything you still want
back into place:

```bash
ls -l ~/.config/trading/cockpit-lab-state.json*
```

There is **no migration** between schema versions: version 1 is the only version
there has ever been, so every mismatch is a fail-loud. A future version 2
migrates in `lab::persistence::decode`, and only then does the fail-loud arm
narrow.

## History

This contract is younger than it looks. Until 2026-09-24 the cockpit called
`Cockpit::new()` in both binaries, so it restored nothing and saved nothing —
the whole persistence module was reachable only from its own unit tests
(bug-log #102). Every launch was a cold start, which is why nobody noticed: a
cold start and a faithfully restored session looked identical on screen. The
notice described above is what makes them different, and
`crates/ui/tests/lab_session_restore_render.rs` proves at the pixels that it
does.
