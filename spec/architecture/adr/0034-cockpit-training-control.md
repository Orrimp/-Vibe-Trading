---
adr: 0034
title: Cockpit training control — audit-DB-as-seam, subprocess lifecycle, R6 in-panel curves
status: accepted
date: 2026-05-19
supersedes: none
superseded-by: none
---

# ADR-0034: Cockpit training control — audit-DB-as-seam, subprocess lifecycle, R6 in-panel curves

## Context

`spec/cockpit-training-control/feature.md` v0.1.0 (analyst-locked 2026-05-19;
operator confirmed all four analyst defaults via orchestrator
AskUserQuestion same day) ships a Lab sub-panel that spawns the existing
`train_tcn` binary as an OS subprocess, tails its stdout, and (Tier 2)
plots per-epoch loss curves via a new SQLite-backed event stream. The
analyst's R1–R10 lock the **what**; this ADR locks the **how** along
four cross-cutting architectural axes that the developer cannot
re-litigate without operator approval:

1. **Where do training events surface to the cockpit?** Three options
   surveyed: (i) IPC over a Unix socket or named pipe, (ii) the existing
   `agent::EventBus` broadcast channels, (iii) writes to the audit
   SQLite DB read back by the cockpit. Option (i) introduces a new
   transport with no existing precedent in the workspace and a new
   serialization contract; (ii) couples training observability to the
   live-trading event bus that explicitly separates training and
   live-trade lifecycles (R7.5); (iii) reuses the existing
   `crates/audit` plumbing (ADR-0024 WAL-mode SQLite + raw `sqlx`) that
   already supports one writer + many readers concurrently and gives
   "free" historical persistence (R8.2 — the audit DB IS the
   training-history layer).

2. **Subprocess lifecycle ownership.** The cockpit's `lab::runner` shape
   (spawned per `ui-rethink-phase-a-lab` T-D-14) supports an in-process
   backtest future; trainer is a different beast (an OS process whose
   stdout pipe is the only signal). The shared cancellation-pair shape
   from `runner.rs` is reusable but the spawn primitive is not, so the
   trainer lives next to (not inside) `lab::runner`.

3. **R6 live-curve placement.** Two options: (a) extend the existing
   1537-LOC `widgets::chart` with a `ChartKind::TrainingCurves` variant
   wired to the main chart canvas; (b) reuse the same chart-overlay
   render path inside a panel-scoped sub-canvas. The analyst's R6.2
   already rejects (a) on chart-as-door grounds; this ADR locks the
   exact composition pattern for (b).

4. **K5 CLI-drift mitigation.** Two options: (a) cockpit calls
   `train_tcn --print-config-schema` at startup and validates its
   internal arg shape against the binary's clap definition;
   (b) a CI golden-diff against a snapshot of `train_tcn --help`.

The decision binds the developer's T-D-N decomposition (Phase M-T1 +
M-T2) and the migration file `010_training_events.sql`.

## Decision

### D1 — Audit-DB-as-seam (Q1 = (a))

Training events flow **`train_tcn` (writer) → `audit.sqlite training_events`
table → cockpit subscription (reader)**. New table created by additive
migration `010_training_events.sql`; zero ALTER on existing tables;
zero rows on existing DBs until `train_tcn --audit-db <path>` is
invoked. Anchor-byte-safe by construction (R10.3); the 19 locked
body-SHA-256 anchors (15 originals + 4 `-realdata`) stay byte-identical.

**WAL-mode confirmation:** `Ledger::open` (`crates/audit/src/ledger.rs:20`)
opens with `?mode=rwc` but does NOT issue an explicit `PRAGMA
journal_mode = WAL` — this ADR observes a latent gap against ADR-0024's
claim that "SQLite WAL mode permits one writer + many readers
concurrently". The gap is non-blocking for this feature because the
cockpit's read pattern (1 Hz indexed SELECT over a half-open `[since,
until)` window) and `train_tcn`'s write pattern (≤ 1 INSERT per epoch =
1 every 5–30 min) overlap so rarely that even the SQLite default rollback
journal handles the contention without observable latency. **Follow-up
backlog item:** add `PRAGMA journal_mode = WAL;` to `Ledger::open` and
verify across the 19 anchors. Tracked separately; not blocking on this
feature.

### D2 — `training_events` schema

The migration `crates/audit/migrations/010_training_events.sql` ships
the analyst's R4.2 schema verbatim with two refinements driven by the
read-pattern analysis (D6 below):

```sql
CREATE TABLE IF NOT EXISTS training_events (
    id              TEXT PRIMARY KEY,         -- UUID v4 (one per event row)
    ts              TEXT NOT NULL,            -- RFC3339 6-digit microsecond per ADR-0004
    run_id          TEXT NOT NULL,            -- UUID v4 (groups all events from one train_tcn invocation)
    kind            TEXT NOT NULL,            -- 'start' | 'epoch' | 'finish' | 'failed'
    epoch           INTEGER,                  -- NULL for start/failed; populated for epoch/finish
    total_epochs    INTEGER,                  -- NULL for start/failed; populated for epoch/finish
    train_loss      TEXT,                     -- f32-as-TEXT (no float arithmetic in audit per ADR-0003)
    val_loss        TEXT,
    wall_clock_ms   INTEGER,                  -- ms within current epoch (or full run for finish)
    model_revision  TEXT,                     -- NULL until finish; canonical SHA from CheckpointMetadata
    scenario        TEXT NOT NULL,            -- 'bs1' | 'bs2' | 'default' | operator label
    seed            INTEGER NOT NULL,
    pid             INTEGER,                  -- D6 — process PID captured at 'start'; NULL otherwise. Powers orphan-detect.
    error_message   TEXT                      -- NULL except kind='failed'
);

CREATE INDEX IF NOT EXISTS training_events_ts_idx
    ON training_events(ts);
CREATE INDEX IF NOT EXISTS training_events_run_id_idx
    ON training_events(run_id, ts);
CREATE INDEX IF NOT EXISTS training_events_kind_idx
    ON training_events(kind, ts);
```

Refinements vs. analyst R4.2:

- **`scenario` is `NOT NULL`** (analyst left nullability implicit). The
  binary always knows its scenario label (default `"default"`); making it
  non-null avoids a permanent `Option<SmolStr>` in the value type.
- **New `pid INTEGER` column** captures the OS PID at the `start`
  emission edge so the orphan-detect query (D7) can do
  `kill(pid, 0)`-based liveness checks without round-tripping through
  any external store. PID alone is best-effort (PID reuse possible on
  long-running boxes); the combination `(start_ts, pid)` is robust
  enough for the operator-visibility surface this feature ships.

**Primary key choice:** `id TEXT PRIMARY KEY` (UUID v4) — matches every
other audit row's identity strategy (`journal_transactions.id`,
`strategy_signals.id`). Composite `(run_id, epoch)` would be tighter
but breaks the per-row uniqueness for `kind='start' | 'failed'` rows
that have no epoch. Sticking with UUID keeps the writer simple.

### D3 — Module placement: NEW `crates/ui/src/lab/trainer.rs`

The analyst's R2.1 default — a sibling to `lab::runner` — is confirmed
verbatim. Rationale:

- The Lab is the operator's cockpit-as-default workshop (per
  `ui-rethink-phase-a-lab` R1.2); training-control IS Lab work — the
  meta-step of producing the model the operator then backtest-runs.
  Placing trainer as a sibling of `lab::runner` keeps "Lab" as the
  single ownership boundary.
- A new `crates/ui/src/training/` sibling-to-lab was considered and
  rejected because the operator's mental model has training as a Lab
  sub-task, not a parallel-route concept.
- Shared cancellation-pair shape (`RunCancelHandle` /
  `RunCancelReceiver` from `runner.rs:64`) is **reused**, not copied —
  the trainer imports those types directly. The spawn primitive
  (`tokio::process::Command` vs. `rt_handle.spawn(...)`) and the
  cancellation semantic (SIGKILL via `child.kill()` vs. `try_recv` at
  next bar boundary) differ; the trainer module exposes a parallel
  `spawn_training_run` function and a new `TrainingHandle` type.

### D4 — `train_tcn` instrumentation contract

CLI signature, added at the **end of the existing arg list** (after
`scenario`):

```
--audit-db <PATH>          Path to audit.sqlite. When provided, train_tcn
                           emits start/epoch/finish/failed rows to the
                           training_events table. Default: omitted →
                           no audit writes.
```

Lifecycle:

- **Connection model: short-lived, opened-per-run.** `train_tcn` opens
  a fresh `Ledger` at the `start` emission edge (after config parsing,
  before training loop), holds it for the duration of the run, and
  drops it at `finish` / `failed`. Per-emission reconnection is
  rejected — adds 4 × N SQLite open/close cycles per run for no
  observability benefit. Long-lived shared with the cockpit is
  rejected — `train_tcn` is its own process with no shared handle.
- **Missing-file behaviour: hard-fail.** If `--audit-db <PATH>` points
  at a non-existent file, `train_tcn` exits non-zero with a clear
  error message before any training. **Rationale:** the cockpit
  creates the file (via its own `Ledger::open` at boot — see ADR-0024).
  A hard-fail catches the case where the operator typo'd the path;
  auto-creating would silently swallow that and produce an orphaned
  DB the cockpit will never see.
- **Panic / SIGKILL survivability for `kind='failed'`:** wrap `main()`'s
  `Result<()>` body in a `catch_unwind` boundary that, on either an
  `Err(_)` return OR a panic unwind, opens (if not already open) the
  Ledger and writes the `failed` row before re-raising. SIGKILL is
  inherently unrecoverable — no `kind='failed'` row is written on
  SIGKILL because the process has no chance to run user code. The
  cockpit detects SIGKILL via its own `Child::wait()` ExitStatus
  (`R9.4`) and renders "Failed: process killed (exit 137)" without
  needing a DB-side row.

Emit edges (R5.2 confirmed):

| Edge      | Site                                              | Fields populated                                                                       |
|-----------|---------------------------------------------------|----------------------------------------------------------------------------------------|
| `start`   | after config parsing, before training loop        | `id, ts, run_id, kind='start', scenario, seed, pid`                                    |
| `epoch`   | `info!("epoch complete", ...)` site (`train_tcn.rs:523`) | `id, ts, run_id, kind='epoch', epoch, total_epochs, train_loss, val_loss, wall_clock_ms, scenario, seed` |
| `finish`  | end of `write_checkpoint`                         | `id, ts, run_id, kind='finish', epoch=total_epochs, total_epochs, train_loss=final, val_loss=final, wall_clock_ms=run_total, model_revision, scenario, seed` |
| `failed`  | top-level `catch_unwind`/`Err(_)` boundary in main | `id, ts, run_id, kind='failed', error_message, scenario, seed`                         |

The `tokio::runtime::Runtime` for the audit-write path is built per-run
(`Runtime::new()` at the `start` edge inside the otherwise sync `fn main()`).
Audit writes are `block_on`'d. Training-loop performance is unaffected;
the writes happen at edge points outside the inner training loop.

### D5 — Audit writers + readers

New writers in `crates/audit/src/journal.rs`:

```rust
pub async fn post_training_start(
    ledger: &Ledger,
    run_id: &str,
    scenario: &str,
    seed: i64,
    pid: i32,
) -> Result<SmolStr, LedgerError>;

pub async fn post_training_epoch(
    ledger: &Ledger,
    run_id: &str,
    epoch: i32,
    total_epochs: i32,
    train_loss: f32,
    val_loss: f32,
    wall_clock_ms: i64,
    scenario: &str,
    seed: i64,
) -> Result<SmolStr, LedgerError>;

pub async fn post_training_finish(
    ledger: &Ledger,
    run_id: &str,
    epoch: i32,
    total_epochs: i32,
    final_train_loss: f32,
    final_val_loss: f32,
    total_wall_clock_ms: i64,
    model_revision: &str,
    scenario: &str,
    seed: i64,
) -> Result<SmolStr, LedgerError>;

pub async fn post_training_failed(
    ledger: &Ledger,
    run_id: &str,
    error_message: &str,
    scenario: &str,
    seed: i64,
) -> Result<SmolStr, LedgerError>;
```

Same `#[instrument]` + `LedgerError` shape as `post_strategy_signal`
(precedent in `journal.rs:266`). f32 values bound via `format!("{val}")`
to TEXT — Decimal-as-TEXT contract per ADR-0003 (Decimal newtype is
overkill for loss-value observability; the TEXT-bound `f32` Display
round-trips losslessly enough for plotting).

New readers in `crates/audit/src/query.rs`:

```rust
pub async fn recent_training_events(
    ledger: &Ledger,
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<TrainingEventRow>, LedgerError>;

pub async fn latest_training_run(
    ledger: &Ledger,
) -> Result<Option<TrainingRunSummary>, LedgerError>;

pub async fn orphan_training_runs(
    ledger: &Ledger,
    fresh_window: time::Duration,  // typically Duration::hours(24)
) -> Result<Vec<OrphanTrainingRun>, LedgerError>;
```

`recent_training_events` sibling-of-`recent_signals`/`recent_fills_filtered`:
same RFC3339 binding, same half-open `[since, until)` window. Value
types `TrainingEventRow`, `TrainingRunSummary`, `OrphanTrainingRun` land
in `crates/core/src/lib.rs` next to `SignalView` / `FillView` /
`StrategyEventView` — consistent with the public-types-in-core
convention.

**`latest_training_run` reader query** (the orphan-detect status-strip
note, D7):

```sql
SELECT * FROM training_events
WHERE kind = 'start'
ORDER BY ts DESC
LIMIT 1;
-- Then in the same call, a second indexed lookup against
-- (run_id, kind IN ('finish','failed')) to decide complete vs orphan.
```

### D6 — Cockpit subscription: 1 Hz audit-DB poller

New `crates/ui/src/lab/training_subscription.rs` module wrapping an
`iced::Subscription` recipe. The recipe is built only while
`LabState::training_inflight.is_some()` AND on cockpit boot for
orphan-detect (D7). Recipe shape:

```rust
// Pseudocode — mirrors crates/ui/src/cockpit_live.rs subscription_for shape.
pub fn training_events_subscription(
    rt_handle: tokio::runtime::Handle,
    ledger: Ledger,
    run_id: SmolStr,
) -> iced::Subscription<Message> {
    iced::Subscription::run_with_id(
        ("training_events", run_id.clone()),  // hash identity → re-spawned only on run_id change
        async_stream::stream! {
            let mut last_seen_ts = Timestamp::epoch();  // 1970 — picks up all rows on first tick
            let mut ticker = tokio::time::interval(Duration::from_millis(1000));
            loop {
                ticker.tick().await;
                let now = Timestamp::now_utc();  // wall-clock OK — observability only
                match recent_training_events(&ledger, last_seen_ts, now).await {
                    Ok(rows) if !rows.is_empty() => {
                        last_seen_ts = rows.last().unwrap().ts;
                        yield Message::TrainingEventsRefreshed(rows);
                    }
                    Ok(_) => { /* no new rows */ }
                    Err(e) => {
                        tracing::warn!(err = %e, "training_events poll error");
                    }
                }
            }
        },
    )
}
```

- **Identity:** `(module_id, run_id)` tuple. iced cancels the prior
  stream when `run_id` changes; this gives the analyst's R7.4
  "subscription stops automatically when `training_inflight = None`"
  for free — the cockpit changes `training_inflight = None` and on the
  next view rebuild a different (or empty) `Subscription::batch`
  passes through `with_id`, and iced reaps the old recipe's stream.
- **Polling debounce:** the 1 Hz `tokio::time::interval` self-throttles.
  No backpressure problem at 1 Hz against the audit DB's WAL-mode
  reads; the read query is indexed (`training_events_ts_idx`).
- **Shutdown:** when iced exits, the side-thread tokio runtime drops,
  which drops the stream, which drops the SQLite pool handle (the
  `Ledger` clone). Clean.
- **Boot-time orphan-detect:** on Lab module init the cockpit calls
  `query::orphan_training_runs(&ledger, Duration::hours(24))` ONCE
  (not via subscription); see D7. If an orphan exists AND its PID is
  alive, the cockpit additionally spawns the subscription against that
  `run_id` so the operator sees live event flow without re-clicking
  Train.

### D7 — Orphan-detect status-strip annotation (Q4 = (a))

Per operator-confirmed Q4, the cockpit does NOT auto-route into the
Train panel. The boot-time flow:

1. Lab module init calls
   `query::orphan_training_runs(&ledger, Duration::hours(24))`.
2. The query returns rows where `kind='start'` exists AND no
   corresponding `kind IN ('finish','failed')` row exists for the same
   `run_id` AND the `start` ts is within the last 24h.
3. For each candidate, the cockpit calls a `pid_alive(pid: i32) -> bool`
   helper (uses `libc::kill(pid, 0)` on Unix; analogous
   `OpenProcess`/`GetExitCodeProcess` on Windows — gate via `cfg`).
   PID-alive AND PID matches → live orphan (training still running).
   PID-dead → dead orphan (cockpit crashed mid-run; subprocess
   subsequently died too).
4. **Live orphan** → status strip renders: `"Orphan training run still
   writing: <scenario> (epoch N/M, pid <PID>) — click Train panel to
   tail"`. Click-target: the existing "Train" header chip in the Lab
   sub-panel (R1.2). Clicking expands the panel + spawns the
   `training_events_subscription` against the orphan's `run_id`.
5. **Dead orphan** → status strip renders: `"Last training run did not
   complete: <scenario> (last seen epoch N/M, <Ts ago>)"`. Click-target:
   same chip; clicking just expands the panel (no subscription — the
   audit DB has the last-known state already loaded into
   `LabState::last_training_summary`).

The annotation lives in the cockpit chrome status strip (NOT the Lab
sub-panel's own status strip) so it survives panel-collapsed state.
The exact strings are factored into `crate::strings` per the no-string-
literals rule; cross-reference T-D-N16 in tasks.md.

### D8 — R6 live curve plot integration

The plot lives **inside the Train panel** (not on the main chart
canvas) via a dedicated `widgets::training_plot` module that reuses
the same canvas + tiny-skia rendering primitives as `widgets::chart`
but exposes a narrower API:

```rust
pub struct TrainingPlot<'a> {
    series: &'a [TrainingPlotPoint],   // (epoch: u32, train_loss: f32, val_loss: f32)
    height_px: f32,                    // fixed ~160 px
    theme: ThemeMode,
}

pub struct TrainingPlotPoint {
    pub epoch: u32,
    pub train_loss: f32,
    pub val_loss: f32,
}
```

Composition: data assembled by the Lab screen from
`LabState::training_events: VecDeque<TrainingEventRow>` (filtered to
`kind='epoch'`), passed into `TrainingPlot::view(...)` which returns a
`Canvas<'_>` that renders inside the Train panel's column layout.

- **Y-axis scaling:** auto-linear scaled to
  `[0, max(train_loss, val_loss) * 1.1]` per R6.3. No log axis at
  Tier 2 (Huber losses are already in a reasonable range; log
  obscures the early-epoch progress operators want to watch). If the
  operator asks for log later, it's a one-flag addition.
- **Empty state:** centered Lumen `text::SMALL` placeholder "No
  training run in flight" rendered into the same canvas area when
  `series.is_empty()`. **Pre-first-epoch state** (run is in flight but
  no `kind='epoch'` row yet — the operator's just clicked Train):
  centered Lumen `text::SMALL` "Warming up — first epoch landing
  shortly" plus a `iced_aw::spinner` (precedent: ADR per
  REQ-ICED-AW-002). The status strip independently shows
  "Training (warming up, t=Ts)" with the elapsed-since-start wall
  clock — this gives the operator confidence that the subprocess is
  alive before any data lands.

The decision NOT to reuse `widgets::chart` directly is driven by the
shape mismatch: the main chart's data model is OHLCV bars + overlay
polylines indexed by `Timestamp`; the training-plot needs an integer
x-axis (epoch number) and per-line y-series with no time semantics.
Cross-loading the integer epoch axis into the chart's timestamp axis
would either need a fake timestamp synthesised per epoch (ugly + fragile
when epoch durations vary 5-30 min) or a `ChartKind::TrainingCurves`
variant (rejected by R6.1). The new `training_plot` module shares
~30 LOC of axis-rendering helpers with `chart.rs` via a shared
`widgets::axis` helper module (NOT a duplication — the helpers are
already extracted in `chart.rs` and become `pub(crate)` for the new
plot's reuse).

### D9 — K5 mitigation: golden-CLI CI diff (analyst suggestion (b))

The cockpit hard-codes the `train_tcn` arg shape via the trainer's
`Command::args(["--config", …, "--output-dir", …, …])` invocation. CLI
drift on the `train_tcn` side would silently break the cockpit. The
mitigation is the cheaper of the two options surfaced by analyst K5:

- **Chosen: CI golden-diff snapshot.** A new test
  `crates/forecast/tests/train_tcn_cli_snapshot.rs` runs
  `train_tcn --help` and asserts the output matches a checked-in
  `crates/forecast/tests/golden/train_tcn_help.txt` (snapshot via
  `insta::assert_snapshot!`). Any change to the clap definition forces
  a snapshot review where the developer also updates the cockpit's
  trainer call site. The snapshot is human-readable + small (≤ 2 KB).
- Rejected: runtime `--print-config-schema` validation. Requires
  adding a new clap subcommand to `train_tcn`, a deserialization step
  on cockpit boot, and a versioning contract for the schema format —
  ~150 LOC of moving pieces for the same drift-detection guarantee
  the snapshot test gives in ~30 LOC.

### D10 — Path resolution for the `train_tcn` binary

Resolution order (R2.2 specifies `current_exe()`-relative with `cargo
run` fallback — this ADR locks the precedence):

1. Same-directory-as-cockpit-binary lookup: `current_exe().parent() /
   "train_tcn"` (handles the cargo-bundle + release path).
2. Workspace-relative dev fallback: if cockpit's
   `current_exe()` lives inside `target/debug/` or `target/release/`,
   try `<workspace_root> / "target" / "<profile>" / "train_tcn"`.
3. `cargo run` fallback (dev only, behind a `#[cfg(debug_assertions)]`
   gate): construct `cargo run -p forecast --bin train_tcn --release --
   <args>`. This path exists for the operator who's running the
   cockpit via `cargo run` itself and hasn't built the train_tcn
   binary yet; the cargo invocation transparently rebuilds + runs.

If all three fail, `spawn_training_run` returns `Err("train_tcn
binary not found at <paths tried>")` synchronously (R9.1); the
status strip displays the error; no subprocess spawned; cockpit does
not panic.

## Alternatives considered

- **(D1.alt) Named-pipe IPC instead of audit-DB-as-seam.** Rejected:
  introduces a new transport; serialization contract is bespoke;
  cockpit must own pipe-cleanup on subprocess crash; no historical
  persistence (operator restarts the cockpit, training events lost).
  The audit DB already does ALL of this and the WAL-mode contention
  is non-existent at the cadence training events occur.
- **(D1.alt) `agent::EventBus` broadcast channel for training events.**
  Rejected per analyst R7.5: the bus is the live-trading event stream;
  intermixing training observability couples two unrelated lifecycles
  (training can run while the agent is paused; the agent shouldn't
  care about training events). Future feature can add a bus channel
  if it needs one.
- **(D3.alt) New `crates/ui/src/training/` sibling-to-lab.** Rejected:
  splits the operator's mental model. Training is Lab work
  (`ui-rethink-phase-a-lab` R1.2). One Lab.
- **(D4.alt) Long-lived audit DB connection shared with the cockpit's
  Ledger handle.** Rejected: `train_tcn` is its own process; the
  cockpit's `Ledger` handle is in-process. Sharing would require IPC.
  The short-lived per-run connection is fine — SQLite open is ~1 ms.
- **(D4.alt) Auto-create the audit DB file if `--audit-db <PATH>`
  doesn't exist.** Rejected: silently swallows typos. The cockpit
  creates the file at boot; `train_tcn` finding it missing is
  diagnostic, not fixup-worthy.
- **(D5.alt) Composite primary key `(run_id, epoch)`.** Rejected:
  `kind='start'`/`'failed'` rows have NULL `epoch`. UUID PK keeps the
  writer simple and matches every other audit table.
- **(D8.alt) Reuse `widgets::chart` with a `ChartKind::TrainingCurves`
  variant.** Already rejected by analyst R6.1. This ADR confirms the
  rejection in code-shape detail: integer-epoch axis vs.
  timestamp-bar axis is a mismatch that contorts the chart's data
  model.
- **(D8.alt) Log-y axis.** Deferred. Huber losses sit in a range that
  linear-y resolves well; log obscures the early-epoch progress the
  operator wants to see. One-flag addition if needed later.
- **(D9.alt) Runtime `--print-config-schema` validation.** Rejected:
  ~5× the LOC for the same K5-mitigation guarantee.

## Consequences

**Positive:**

- The 19 locked body-SHA-256 anchors (15 originals + 4 `-realdata`)
  stay byte-identical. Migration 010 is purely additive; `train_tcn`'s
  `<sha>.metadata.json` bytes are unaffected by the `--audit-db` opt-in.
- One new ADR captures all cross-cutting decisions; the developer's
  T-D-N work is mechanical from here.
- Training-history persistence is FREE — the audit DB IS the layer.
  The cockpit's "latest training run on boot" surface works across
  restarts with no new persistence shape.
- The pattern (subprocess + audit-DB-as-seam + 1 Hz poller
  subscription) generalises to v2.5a PatchTST and v2.5b Transformer
  training rounds without re-architecting. The `BinarySpec`
  abstraction noted in analyst Out-of-scope can plug into
  `lab::trainer` later as a one-arg switch.

**Negative:**

- One new SQLite table; one new audit migration; ~250 LOC of
  writer/reader plumbing in `crates/audit`. Cost is bounded but real.
- The audit DB grows monotonically — every epoch of every training
  run writes a row forever. At BS-1/BS-2 cadence (5-30 epochs per
  run, ≤10 runs/week typical) the growth is ~50–300 rows/week ≈ tens
  of KB/year. Non-issue. A future retention job can trim
  `training_events` rows older than N months without touching any
  other table.
- The PID-based liveness check is best-effort. PID reuse on a
  long-running operator box is possible (PID 12345 reused by some
  unrelated process after the cockpit + train_tcn died). Mitigation:
  the orphan-detect window is bounded to 24h (D7), and the operator
  always has the audit DB's `kind='start'` row's timestamp + scenario
  label to disambiguate manually. PID-reuse false-positives are a
  cosmetic problem, not a correctness one.
- The new `widgets::training_plot` module duplicates ~30 LOC of
  axis-rendering helpers with `widgets::chart`. Mitigation: the
  helpers are extracted into a `widgets::axis` shared module (the
  cleanup IS part of this feature's T-D-N decomposition — see
  tasks.md T-D-N17). Net LOC delta is small and the shared module
  is value-add for future plot widgets.
- The latent WAL-mode gap in `Ledger::open` (D1) — already a
  workspace-wide issue per ADR-0024 — surfaces here but does not
  block. Filed as backlog item.

**Determinism:**

- `train_tcn`'s `<sha>.metadata.json` body bytes are byte-identical
  with `--audit-db` enabled vs. disabled. The audit emit edges are
  side-effects after metadata canonicalization; they do NOT mutate
  the metadata generator's input. Verified by the new integration
  test `train_tcn_audit_emits.rs` (T-D-N15 / R5 acceptance) which
  runs the same `--dry-run --epochs 1 --symbols BTCUSDT` with and
  without `--audit-db` and asserts byte-equal `<sha>.metadata.json`
  outputs.
- `training_events` row bytes (the `ts`, `wall_clock_ms`, `id` UUID)
  are inherently non-deterministic. They are observability data, not
  replay inputs — R10.5 spells this out. Zero new anchors.

**Risk register impact:**

- K1 (stdout/cockpit-shutdown race): unchanged. Accepted.
- K2 (audit DB lock contention): unchanged. WAL-mode contention is
  non-existent at this cadence; the latent WAL-mode gap (D1) is a
  separate backlog item.
- K3 (orphan PIDs): D7 is the primary mitigation. PID column in the
  schema (D2) makes orphan detection robust against the cockpit
  having lost its in-memory handle.
- K4 (live curve redraw at high frequency): unchanged. 1 Hz cap.
- K5 (CLI flag drift): mitigated by D9 (golden-CLI snapshot test).
- K6 (panel obscures chart): unchanged. Collapsed default per R1.2.

## Cross-references

- Predecessor: `ui-rethink-phase-a-lab` v0.2.0 — Lab module, ADR-0030
  for cockpit ↔ backtest in-process API, `lab::runner::spawn_lab_run`
  precedent.
- Cross-feature: `v25-tcn-alpha-investigation` — F4 verdict feeds the
  first real consumer of this UI handle.
- Audit invariant: ADR-0024 (raw `sqlx` + SQLite WAL; latent WAL-mode
  gap noted in D1 for backlog).
- Money math invariant: ADR-0003 (Decimal-as-TEXT precedent reused for
  f32 loss values as TEXT).
- Timestamp invariant: ADR-0004 (6-digit fractional-second RFC3339).
- Checkpoint provenance: ADR-0029 (`model_revision` SHA — populated
  on `kind='finish'` rows).
- Realdata path: ADR-0032 (`REVISION.toml` data pin — orthogonal; the
  data the v2.5 retraining cycle will consume).
- Migration precedent: `crates/audit/migrations/009_strategy_signals.sql`
  + ADR-0020 (additive `CREATE TABLE IF NOT EXISTS`, anchor-byte-safe
  by construction).
- Subscription precedent: `crates/ui/src/cockpit_live.rs::subscription_for`
  (recipe identity via `with_id`, side-thread tokio runtime bridge).
