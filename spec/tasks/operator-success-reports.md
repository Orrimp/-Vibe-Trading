---
slug: operator-success-reports
status: shipped
owner: tester
updated: 2026-05-01
changelog:
  - 2026-05-01 (tester): FINAL gate PASS. T_FINAL_REPORTS ticked.
    Anchor gate `ANCHORS PASS (11 / 11)`; workspace tests
    580 PASS / 0 FAIL / 3 IGNORED; `reports` crate 143 PASS / 0 FAIL;
    cargo fmt + clippy clean (`-D warnings`, `--all-features`);
    `cargo build -p agent --features in_process_cron` clean; bin
    smoke exits 0 with full 12-field front-matter. T814 / T815 /
    T816 dev citations re-walked (minor 1–2 line drift on three
    constants/fns; all functions exist at adjacent locations);
    T817's orchestrator-verified anchor-gate citation reconfirmed.
    V1–V10 all VERIFIED. Status bumped `in-progress → shipped`.
    Test report:
    `spec/reports/test-2026-05-01-1828-operator-success-reports-final.md`.
  - 2026-05-01 (developer): T807, T808, T811, T812 ticked in Wave 2a;
    `crates/reports/` skeleton + reconcile engine + decay heuristic +
    MarkSource trait + ParquetMarkSource + FrozenMarkSource land.
  - 2026-05-01 (developer): T809, T810 ticked in Wave 2b; kill-switch
    audit dual-write + incident-report spawn seam (CommandIncidentSpawner /
    MockIncidentSpawner trait); optional in-process cron behind feature
    flag `in_process_cron` with reference systemd + launchd ops files.
    9-anchor regression-free.
  - 2026-05-01 (developer): T813 ticked in Wave 2c; render modules R2–R9
    + R11 reconciliation appendix + lib::generate orchestrator +
    csv_artifacts. 134 tests green; cargo fmt + clippy clean across
    workspace; `cargo run -p reports --bin report -- --period 7d ...`
    exits 0; 9-anchor regression-free.
  - 2026-05-01 (developer): T814 ticked in Wave 2d-1; three integration
    tests under `crates/reports/tests/` (determinism, body-no-volatile-
    metadata, reconciliation_mismatch) green. R10.3/R10.4/R11.4 gates
    enforced. Tests-only — no source touched. cargo fmt + clippy clean.
  - 2026-05-01 (developer): T815 ticked in Wave 2d-2; perf smoke test
    `crates/reports/tests/perf_smoke.rs` + 1-year fixture builder
    `crates/reports/tests/fixtures/build_ledger_1y.rs` land. R13.1
    wall-clock < 10s (observed 0.247s) and R13.3 RSS < 256 MiB
    (observed 34.6 MiB peak via libc::getrusage RUSAGE_SELF) both
    enforced. Test-only changes; one new dev-dep (`libc`) declared
    under `crates/reports/Cargo.toml`'s `[dev-dependencies]`. cargo
    fmt + clippy clean.
  - 2026-05-01 (developer): T816 ticked in Wave 2d-3; report-sample-7d
    + report-sample-90d scenarios + V10 cron-friendliness smoke
    (3×-parallel lib + 3×-parallel bin processes) shipped under
    `crates/reports/tests/report_scenarios.rs` against fixture
    builders `crates/reports/tests/fixtures/build_ledger_{7d,90d}.rs`.
    Two new anchors appended to `spec/anchors.toml` (report-sample-7d
    `ab06dbcb…`, report-sample-90d `2ef403f1…`). `verify_anchors.sh`
    extended with an additive `success-*-<scenario>.md` fallback glob;
    9-anchor flow unchanged. Anchor gate now `ANCHORS PASS (11 / 11)`.
    `cargo fmt --check` + `cargo clippy --workspace --tests --
    -D warnings` clean.
---

# Tasks — Operator success reports

Ordered, testable task list derived from
[spec/features/operator-success-reports.md → Design](../features/operator-success-reports.md#design)
and the nine architect resolutions (Q1–Q9) recorded in the same Design
section. Cross-references to the analyst's R/V items use the format
`Rn` / `Vn`; cross-references to the analyst's open questions use
`Qn`.

Owner tags: `[developer]` for backend Rust work across `trading_core`
/ `audit` / `agent` / `reports`. **No `[ui-designer]` tasks** — v1+
operator-success-reports is a non-UI feature (the cockpit's `viewer`
binary already renders markdown reports inline; no widget or fixture
change is required).

**Parallelism gates** (shared files — only one developer touches each):

- `crates/core/**` (`trading_core` package) — single owner per task.
  T801 is the critical-path gate; everything downstream blocks on it.
- `crates/audit/**` — T802 (schema + writer change) blocks T803, T805.
- `crates/reports/**` — owned by the reports-feature developer; can
  fan out across the `src/render/*` modules in parallel after T807
  lands the lib skeleton.
- `crates/agent/**` — T809 (kill-switch wire) and T810 (cron flag)
  touch the same file (`agent::main`); sequence them.

**Synchronization points** (block downstream tasks):

- **T801** — `trading_core::StrategyEventKind` variant additions
  (`KillSwitchTripped`, `FeedReconnect`). Once merged, T802 / T805
  can write rows of those kinds.
- **T802** — `004_journal_transactions_strategy_id.sql` migration +
  `post_fill` signature change. Blocks T803 (`pnl_by_strategy`
  query) and T813 (R5 attribution renderer).
- **T807** — `crates/reports/` lib skeleton + atomic-write +
  front-matter writer. Blocks all `tests/render/*` tasks (T813).

**Granularity:** ~½ day per task. Tasks numbered T8xx so v0 T0xx,
v0.5 T5xx, v1 T6xx, v1.5a T7xx namespaces stay intact. v1+ builds
heavily on existing `audit::query` + the v1.5a `strategy_events`
extension pattern, so v1+'s task count is comparable to v1.5a's.

## Week 1 — types, schema, audit query, kill-switch wiring

- [x] **T801** [developer] — `trading_core` v1+ `StrategyEventKind`
  variant additions per
  [Design → Q-resolution Q8](../features/operator-success-reports.md#q-resolution-summary):
  - `StrategyEventKind::KillSwitchTripped` variant.
  - `StrategyEventKind::FeedReconnect` variant.
  - `Display` arms for both.
  - PascalCase serde rename arms (already covered by the enum's
    existing `#[serde(rename_all = "PascalCase")]`).
  - Round-trip serde tests in `crates/core/src/strategy_events.rs`
    mirror the v1.5a `MeanReversionStop` / `PairShortObservation`
    test pattern.
  All `Serialize` + `Deserialize` + `Clone` + `Debug` + `PartialEq`
  + `Eq`. Additive — every existing match site uses an exhaustive
  default and compiles unchanged. No edges reverse; `trading_core`
  is upstream. —
  _acceptance: `cargo test -p trading_core` clean; both new variants
  serde-round-trip; `cargo clippy -p trading_core -- -D warnings`
  clean; existing v0/v0.5/v1/v1.5a `StrategyEventKind` consumers
  compile unchanged._
  **[gate for T802, T803, T805]**

  **Notes (developer, 2026-05-01):**
  - Variants added at `crates/core/src/strategy_events.rs:111` (`KillSwitchTripped`)
    and `crates/core/src/strategy_events.rs:113` (`FeedReconnect`); Display arms
    at `crates/core/src/strategy_events.rs:126-127`.
  - Round-trip serde tests at `crates/core/src/strategy_events.rs:277-296`
    (`t801_strategy_event_kind_v1plus_variants` +
    `t801_strategy_event_kind_v1plus_serde_roundtrip`).
  - Test cmd: `cargo test -p trading_core --lib`.
  - Output line: `test strategy_events::tests::t801_strategy_event_kind_v1plus_variants ... ok`,
    `test strategy_events::tests::t801_strategy_event_kind_v1plus_serde_roundtrip ... ok`
    (42 passed total).
  - UI match-site updates were required because UI does NOT use a wildcard
    default — explicit arms in `crates/ui/src/widgets/strategies.rs:147-153`
    and `:178-188`; test fixture in `crates/ui/tests/panel_snapshots.rs:557-562`
    and `:595-600`. Cockpit renders new kinds as informational events
    (same styling as v1.5a `MeanReversionStop`/`PairShortObservation`).
    `cargo test -p ui` → 32 passed.
  - `cargo clippy --workspace -- -D warnings` clean (after merging the
    new informational arms with `Load` in `event_kind_label` to satisfy
    `clippy::match_same_arms` — labels are textually identical so the
    rendered output is unchanged).

- [x] **T802** [developer] — Audit schema migration
  `004_journal_transactions_strategy_id.sql` + `post_fill` signature
  change per
  [Design → Q2 — `pnl_by_strategy` query design + schema migration](../features/operator-success-reports.md#q2--pnl_by_strategy-query-design--schema-migration):
  - New file `crates/audit/migrations/004_journal_transactions_strategy_id.sql`
    with the `ALTER TABLE journal_transactions ADD COLUMN strategy_id TEXT;`
    and the `journal_transactions_sid_idx` index.
  - `audit::journal::post_fill` gains `strategy_id: Option<&str>`
    parameter; writes the new column verbatim.
  - All in-tree call sites updated:
    - `crates/exec/src/paper.rs::PaperEngine` (passes the active
      strategy id from the signal it processes).
    - `crates/agent/src/strategy_driver` (passes the strategy id of
      the strategy that emitted the signal).
    - `crates/backtest/src/main.rs` (passes the scenario's strategy
      id; this is **not** a change to backtest report shape — see
      below).
  - **Critical guardrail:** the backtest binary must NOT change its
    written report bytes. The `strategy_id` column is purely
    storage-side; the report renderer reads aggregates that compose
    over fills regardless of whether `strategy_id` is present. Verify
    by re-running all 9 anchor scenarios post-T802 — body SHA256s
    must remain byte-identical (V6).
  - Pre-migration ledgers are valid (NULL is the column default for
    every existing row). —
  _acceptance: `cargo test -p audit` clean; new migration applies
  to a `:memory:` ledger; `post_fill(&ledger, &fill, Some("sma_crossover"))`
  populates the column; `post_fill(&ledger, &fill, None)` leaves
  NULL; **`cargo test -p backtest --test anchor_regression` (or
  equivalent local re-run of the 9 anchor scenarios) shows all 9
  body SHA256s unchanged**; `cargo clippy --workspace -- -D warnings`
  clean._
  **[deps: T801]**
  **[gate for T803, T813]**

  **Notes (developer, 2026-05-01):**
  - Migration: `crates/audit/migrations/004_journal_transactions_strategy_id.sql`
    (ALTER TABLE + index `journal_transactions_sid_idx`).
  - `post_fill` signature change at `crates/audit/src/journal.rs:35-38`
    (gains `strategy_id: Option<&str>`); INSERT statement at
    `crates/audit/src/journal.rs:55-65` writes the new column verbatim.
  - In-tree call sites:
    - `crates/audit/tests/ledger_integration.rs:104,135,163` (existing
      tests pass `None` — migration is backwards-compatible).
    - `crates/exec/src/paper.rs` and `crates/agent/src/strategy_driver*`
      do NOT exist in this state of the codebase (the architect spec
      lists them as planned). When they land, they MUST pass the
      strategy id from the signal.
    - `crates/backtest/src/main.rs` does NOT call `audit::journal::post_fill`
      (it maintains its own in-memory ledger for backtests). No change
      needed there for V6 — confirmed by `grep` and by the anchor
      verification below.
  - New tests at `crates/audit/tests/ledger_integration.rs:178-256`
    (`t802_post_fill_populates_strategy_id_when_some`,
    `t802_post_fill_leaves_strategy_id_null_when_none`,
    `t802_migration_004_creates_index`).
  - Test cmd: `cargo test -p audit --test ledger_integration`.
  - Output: `test t802_migration_004_creates_index ... ok`,
    `test t802_post_fill_leaves_strategy_id_null_when_none ... ok`,
    `test t802_post_fill_populates_strategy_id_when_some ... ok`
    (8 passed total).
  - Anchor regression: `bash scripts/verify_anchors.sh` reports
    `ANCHORS PASS  (9 / 9)`. Backtest binary does not call
    `audit::journal::post_fill` (the journal write path is for the
    paper/live executor only — backtest uses its own in-memory ledger),
    so the column-only migration cannot leak into report bytes.
  - `cargo clippy --workspace -- -D warnings` clean.

- [x] **T803** [developer] — `audit::query::pnl_by_strategy` reader
  per
  [Design → Q2](../features/operator-success-reports.md#q2--pnl_by_strategy-query-design--schema-migration).
  Lives in `crates/audit/src/query.rs` (preferred per Q1 — keeps
  the query API in one place). Returns
  `Vec<StrategyPnl>` (struct, not tuple). Sorted by `realized` DESC,
  ties broken by `strategy_id` ASC (R5.5). Pre-migration NULL
  rows bucket into `StrategyId::new("(unattributed)")`. Closed-trade
  count = `COUNT(DISTINCT jt.id)` over income:realized_pnl rows in
  the window; winning_count = trades with `(credit - debit) > 0`. —
  _acceptance: integration test
  `crates/audit/tests/pnl_by_strategy.rs` posts 12 fills across 4
  strategies (3 trades each) with deliberate mix of wins/losses;
  asserts (a) 4 rows returned in `realized DESC` order, (b)
  `Σ rows.realized == realized_pnl_since(period_start)` to the
  satoshi, (c) win-rate computed correctly, (d) one extra fill
  posted with `strategy_id = None` produces a 5th `(unattributed)`
  row, (e) running with `until = far_past` produces an empty `Vec`._
  **[deps: T801, T802]**
  **[gate for T813 R5 renderer]**

  **Notes (developer, 2026-05-01):**
  - `StrategyPnl` struct at `crates/audit/src/query.rs:524-538`.
  - `pnl_by_strategy` reader at `crates/audit/src/query.rs:557-657`,
    sorted by realized DESC with strategy_id ASC tie-break
    (`crates/audit/src/query.rs:649-655`).
  - NULL → `StrategyId::new("(unattributed)")` bucket at
    `crates/audit/src/query.rs:610`.
  - Closed-trade count via per-`(strategy_id, transaction_id)`
    accumulator at `crates/audit/src/query.rs:617,628-637`. (Spec
    text says "COUNT(DISTINCT jt.id)"; multi-realized-pnl-row
    transactions still tally as one closed trade. With the v0
    `post_fill` shape — one realized_pnl row per sell transaction
    — the two definitions coincide.)
  - 4 integration tests at
    `crates/audit/tests/pnl_by_strategy.rs:111-242`:
    `t803_12_fills_4_strategies_sorted_with_correct_stats`,
    `t803_unattributed_bucket_when_strategy_id_null`,
    `t803_empty_when_window_excludes_all_rows`,
    `t803_tie_break_by_strategy_id_asc`.
  - Test cmd: `cargo test -p audit --test pnl_by_strategy`.
  - Output: 4 passed including
    `test t803_12_fills_4_strategies_sorted_with_correct_stats ... ok`.
  - `cargo clippy --workspace --tests -- -D warnings` clean.

- [x] **T804** [developer] — `audit::query::ledger_snapshot_sha` +
  `audit::query::ledger_inception_ts` helpers per
  [Design → Public lib API + ReportWindow resolves](../features/operator-success-reports.md#reportwindow-resolves-to-since-until):
  - `ledger_snapshot_sha(db_path: &Path) -> Result<[u8; 32], LedgerError>`
    streams the SQLite file via `sha2::Sha256` (no full-file load
    in memory).
  - `ledger_inception_ts(ledger: &Ledger) -> Result<Timestamp, LedgerError>`
    returns the `MIN(ts)` across `journal_transactions`.
  Both additive, no new deps (sha2 already in workspace per
  `Cargo.toml`). —
  _acceptance: unit tests in `crates/audit/tests/snapshot_sha.rs`
  and `crates/audit/tests/inception_ts.rs`; `ledger_snapshot_sha`
  is byte-stable across two reads of the same file; flipping a single
  byte changes the digest; `ledger_inception_ts` returns the earliest
  `ts` from a fixture with three transactions._
  **[deps: T801]**

  **Notes (developer, 2026-05-01):**
  - `ledger_snapshot_sha` at `crates/audit/src/query.rs:813-833`
    (chunked 64 KiB read via `sha2::Sha256`).
  - `ledger_inception_ts` at `crates/audit/src/query.rs:846-862`
    (`MIN(ts)` over `journal_transactions`).
  - `sha2` added to `crates/audit/Cargo.toml:20`; `tempfile` added
    as dev-dep at `crates/audit/Cargo.toml:24`.
  - 3 snapshot tests at `crates/audit/tests/snapshot_sha.rs:14-67`:
    `t804_snapshot_sha_byte_stable_across_two_reads`,
    `t804_snapshot_sha_flipping_one_byte_changes_digest`,
    `t804_snapshot_sha_known_vector_empty_file`.
  - 2 inception tests at `crates/audit/tests/inception_ts.rs:33-66`:
    `t804_inception_ts_returns_earliest_of_three`,
    `t804_inception_ts_errors_on_empty_ledger`.
  - Test cmd: `cargo test -p audit --test snapshot_sha --test inception_ts`.
  - Output: 5 passed (`test t804_snapshot_sha_byte_stable_across_two_reads ... ok`,
    `test t804_inception_ts_returns_earliest_of_three ... ok`, et al.).
  - `cargo clippy --workspace --tests -- -D warnings` clean.

- [x] **T805** [developer] — `audit::journal::feed_reconnect` writer
  + `kind = "FeedReconnect"` parser arm per
  [Design → R7.1 — uptime / clock-skew / feed-reconnect provenance](../features/operator-success-reports.md#r71--uptime--clock-skew--feed-reconnect-provenance).
  - `pub async fn feed_reconnect(ledger, symbol, ts: Option<&str>) -> Result<(), LedgerError>`
    writes a `strategy_events` row with `kind = "FeedReconnect"`,
    `strategy_id = None`, `error_summary = "<symbol>"`.
  - `parse_strategy_event_view` in `crates/audit/src/query.rs`
    gains an arm for `"FeedReconnect"` mapping to
    `StrategyEventKind::FeedReconnect`.
  - The Binance reconnect handler in `crates/data/src/binance.rs`
    adds a single call to this writer when the WebSocket re-establishes
    a connection (small, additive — does not change the reconnect
    semantics). —
  _acceptance: integration test writes one `FeedReconnect` event;
  `strategy_events_since(early_ts)` returns it with the correct
  `kind` and `error_summary`; reconciler `Σ debits == Σ credits`
  unchanged (no money columns)._
  **[deps: T801]**

  **Notes (developer, 2026-05-01):**
  - Writer at `crates/audit/src/journal.rs:498-528`
    (`feed_reconnect(ledger, symbol, ts: Option<&str>)` —
    `kind = "FeedReconnect"`, `error_summary = symbol`).
  - Parser arm at `crates/audit/src/query.rs:411-412` (`"FeedReconnect"`
    plus snake-case alias).
  - `BinanceFeed` extended at `crates/data/src/binance.rs:88-126`
    (additive: optional `Arc<audit::Ledger>` + `with_ledger()` builder).
  - Reconnect handler emits a `FeedReconnect` event after
    re-establishment in both `subscribe_bars`
    (`crates/data/src/binance.rs:285-296`) and `subscribe_trades`
    (`crates/data/src/binance.rs:411-422`). The first connect is
    suppressed via an `is_reconnect` flag — only re-establishments
    emit, and audit-write failure is warn-logged (never breaks the
    stream).
  - `audit` promoted from dev-dep to runtime dep on `data` in
    `crates/data/Cargo.toml:9` — this is a NEW edge `data → audit`
    not previously listed in architecture.md but covered by
    operator-success-reports.md design (R7.1, T805).
  - Integration tests at
    `crates/audit/tests/feed_reconnect_test.rs:25-93`:
    `t805_feed_reconnect_writes_and_reads` (kind / error_summary /
    no-money invariant) and
    `t805_feed_reconnect_microsecond_timestamp_preserved`.
  - Test cmd: `cargo test -p audit --test feed_reconnect_test`.
  - Output: `test t805_feed_reconnect_writes_and_reads ... ok`,
    `test t805_feed_reconnect_microsecond_timestamp_preserved ... ok`
    (2 passed).
  - `cargo clippy --workspace --tests -- -D warnings` clean.

- [x] **T806** [developer] — Audit schema migration
  `005_uptime_intervals.sql` + `agent_uptime` table per
  [Design → R7.1](../features/operator-success-reports.md#r71--uptime--clock-skew--feed-reconnect-provenance):
  - New table `agent_uptime` (boot_id TEXT PRIMARY KEY, started_at
    TEXT NOT NULL, last_heartbeat_at TEXT NOT NULL, stopped_at TEXT
    NULL).
  - `audit::journal::open_uptime_interval(boot_id, ts) -> Result<(), LedgerError>`
    inserts the row.
  - `audit::journal::heartbeat_uptime(boot_id, ts) -> Result<(), LedgerError>`
    updates `last_heartbeat_at`.
  - `audit::journal::close_uptime_interval(boot_id, ts) -> Result<(), LedgerError>`
    sets `stopped_at`.
  - `audit::query::uptime_intervals_since(since) -> Result<Vec<UptimeInterval>, LedgerError>`
    reader.
  - The agent's `main.rs` wires: on boot, generate a UUID `boot_id`
    and call `open_uptime_interval`; spawn a 30s tokio interval
    that calls `heartbeat_uptime`; on graceful shutdown call
    `close_uptime_interval`. Failures are warn-logged, never fatal.
  —
  _acceptance: integration test exercises a full open/heartbeat/close
  cycle and asserts `uptime_intervals_since` returns the correct row
  shape; agent boots cleanly with the new schema in place; clippy
  + fmt clean._
  **[deps: T801]**

  **Notes (developer, 2026-05-01):**
  - Migration: `crates/audit/migrations/005_uptime_intervals.sql`
    (table `agent_uptime` + index `agent_uptime_started_idx`).
  - Three writers in `crates/audit/src/journal.rs`:
    - `open_uptime_interval` at line 624.
    - `heartbeat_uptime` at line 652.
    - `close_uptime_interval` at line 677.
    All three use the 6-digit fractional-second format (helper
    `uptime_ts_string` at line 601) — same HF-3 microsecond format
    the v1.5a `strategy_event` writer uses, so ORDER BY ts is
    stable.
  - Reader `uptime_intervals_since` at
    `crates/audit/src/query.rs:802` (returns `Vec<UptimeInterval>`
    chronologically, NULL `stopped_at` surfaces as `None`).
  - Agent main wiring:
    - Boot row write at `crates/agent/src/main.rs:107` (UUID v4
      boot id generated immediately above).
    - 30s heartbeat task at `crates/agent/src/main.rs:113-134`
      (cancellation token + `tokio::time::interval`; failures
      warn-logged, never fatal).
    - Close on graceful shutdown at
      `crates/agent/src/main.rs:284-288`. The cancel token cancels
      the heartbeat task before the close write, so heartbeats
      cannot race with the close.
  - `uuid` added to `crates/agent/Cargo.toml:45`.
  - 6 integration tests at
    `crates/audit/tests/uptime_intervals_test.rs:24-228`:
    `t806_full_open_heartbeat_close_cycle`,
    `t806_running_agent_has_stopped_at_none`,
    `t806_two_intervals_returned_in_chronological_order`,
    `t806_filter_by_since_excludes_earlier_rows`,
    `t806_default_ts_uses_microsecond_format`,
    `t806_uptime_interval_carries_no_money`.
  - Test cmd: `cargo test -p audit --test uptime_intervals_test`.
  - Output: 6 passed (`test t806_full_open_heartbeat_close_cycle ... ok`,
    et al.).
  - `cargo build -p agent` clean; `cargo clippy --workspace --tests
    -- -D warnings` clean.

## Week 2 — reports crate skeleton, render modules, atomic write

- [x] **T807** [developer] — `crates/reports/` skeleton per
  [Design → `crates/reports/` layout](../features/operator-success-reports.md#cratesreports-layout):
  - `Cargo.toml` (lib + bin); workspace member added to root
    `Cargo.toml`.
  - `src/lib.rs` exporting `generate`, `ReportWindow`,
    `ReportArtifacts`, `MarkSource`, `ReportError`.
  - `src/window.rs` with `ReportWindow` enum + parser (R1.2). All
    7 accepted shapes parse; 4 rejected shapes return
    `WindowParseError::Malformed`.
  - `src/atomic_write.rs` per
    [Design → Atomic write](../features/operator-success-reports.md#atomic-write-r122--q3).
  - `src/run_id.rs` per
    [Design → Run-id hash](../features/operator-success-reports.md#run-id-hash-r34).
  - `src/sparkline.rs` per
    [Design → Sparkline encoding](../features/operator-success-reports.md#sparkline-encoding-r32--q4).
  - `src/render/front_matter.rs` writes the 12-field front-matter
    in the fixed order per
    [Design → Front-matter schema](../features/operator-success-reports.md#front-matter-schema-r101--q7).
  - Stub render modules (`headline`, `equity_curve`, `risk_metrics`,
    `strategy_attribution`, `memory_highlights`, `system_health`,
    `what_changed`, `open_risks`, `reconciliation`) each
    `pub fn render(…) -> String { String::new() }` so the lib
    compiles; bodies fill in T813. —
  _acceptance: `cargo build -p reports` clean; `cargo clippy -p
  reports -- -D warnings` clean; unit tests for `ReportWindow::parse`
  + `atomic_write` + `run_id` + `sparkline` + `front_matter`
  pass; bin `cargo run -p reports --bin report -- --period 7d
  --ledger /dev/null` exits cleanly (with a stub-empty render —
  bodies populate at T813)._
  **[deps: T804]**
  **[gate for T813, T814]**

  **Notes (developer, 2026-05-01):**
  - Workspace member added at `Cargo.toml:16` (root).
  - Crate Cargo.toml at `crates/reports/Cargo.toml:1-37`
    (lib + bin name `report`).
  - `src/lib.rs` exports at `crates/reports/src/lib.rs:25-29`
    (`generate`, `ReportWindow`, `ReportArtifacts`, `MarkSource`,
    `ReportError`).
  - `src/window.rs` parser at `crates/reports/src/window.rs:60-77`;
    7 accepted shapes parse + 4 rejected shapes fail
    (tests `t807_parses_*`, `t807_rejects_*` lines 119-187).
  - `src/atomic_write.rs` writer at
    `crates/reports/src/atomic_write.rs:38-58` (tempfile +
    `<stem>.tmp.<pid>.<counter>` + fsync + rename).  Three-thread
    concurrent test at line 84-110.
  - `src/run_id.rs` at `crates/reports/src/run_id.rs:25-39`
    (16-hex-char SHA-256 prefix; idempotence test at line 60).
  - `src/sparkline.rs` encoder at `crates/reports/src/sparkline.rs:38-72`
    with the `▁▂▃▄▅▆▇█` palette + 60-char `DEFAULT_WIDTH`.
  - `src/render/front_matter.rs` 12-field writer at
    `crates/reports/src/render/front_matter.rs:64-93`; locked field
    order test at line 121-150.
  - Stub render modules under
    `crates/reports/src/render/{headline,equity_curve,risk_metrics,
    strategy_attribution,memory_highlights,system_health,
    what_changed,open_risks,reconciliation}.rs` (each
    `pub fn render(...) -> String` with `String::new()` placeholder
    or — for `memory_highlights` — the locked-in placeholder).
  - `src/bin/report.rs` clap CLI at `crates/reports/src/bin/report.rs:24-49`
    (`--period`, `--ledger`, `--output`, `--seed`).
  - Test cmd: `cargo test -p reports --lib`.
  - Output lines:
    `test window::tests::t807_parses_7d ... ok`,
    `test window::tests::t807_rejects_bogus ... ok`,
    `test atomic_write::tests::t807_atomic_write_no_partial_file_at_canonical_path ... ok`,
    `test run_id::tests::t807_run_id_idempotent_same_inputs ... ok`,
    `test sparkline::tests::t807_encode_one_to_eight_eight_cells ... ok`,
    `test render::front_matter::tests::t807_front_matter_renders_all_12_fields_in_order ... ok`
    (58 passed total).
  - Bin smoke: `cargo run -p reports --bin report -- --period 7d
    --ledger /dev/null --output /tmp/test-report.md` →
    `wrote /tmp/test-report.md (run_id=7a8021c21d97f155)` exit 0.
  - `cargo clippy -p reports --tests -- -D warnings` clean.
  - `cargo doc -p reports --no-deps` clean.
  - Workspace clippy: `cargo clippy --workspace --tests -- -D warnings`
    clean (no regressions in other crates).
  - Anchor regression: `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (9 / 9)`.

- [x] **T808** [developer] — Reconciliation engine
  `crates/reports/src/reconcile.rs` per
  [Design → Reconciliation engine](../features/operator-success-reports.md#reconciliation-engine-r11--q6):
  - `ReconciliationReport`, `ReconciliationRow` structs.
  - `compute(...)` function takes the four identity inputs and
    returns the populated report.
  - `passed` field is `delta == Decimal::ZERO` (exact-cent — Q6).
  - `to_appendix_table()` method returns the R11.3 markdown table
    string with `PASS` / `FAIL` cells.
  - `to_failure_json(run_id, ledger_sha, period, period_start, period_end)`
    serializes the failing rows per the schema in
    [Design → Reconciliation engine](../features/operator-success-reports.md#reconciliation-engine-r11--q6).
  - On any `Δ != 0`, the renderer (T813) prepends the FAIL banner
    line above R9. —
  _acceptance: unit test `crates/reports/tests/reconciliation.rs`
  asserts (a) all-zero deltas → `all_passed == true`, (b) one
  injected one-cent delta → `all_passed == false` and only that
  row's `passed` is false, (c) `to_failure_json` round-trips
  through `serde_json::Value`._
  **[deps: T807]**

  **Notes (developer, 2026-05-01):**
  - `ReconciliationRow` + `ReconciliationReport` structs at
    `crates/reports/src/reconcile.rs:24-67`.
  - `ReconciliationInputs` struct at `crates/reports/src/reconcile.rs:75-97`
    (the 7 Decimal inputs the orchestrator hands to the engine).
  - `compute(...)` function at `crates/reports/src/reconcile.rs:104-127`
    (pure over inputs).
  - `passed` field uses `delta == Decimal::ZERO` exact-cent equality at
    `crates/reports/src/reconcile.rs:43-50`.
  - `to_appendix_table()` markdown writer at
    `crates/reports/src/reconcile.rs:142-156` (PASS/FAIL cells uppercase).
  - `to_failure_json(run_id, ledger_sha, period, period_start, period_end)`
    at `crates/reports/src/reconcile.rs:165-198` — schema_version=1, 4
    rows, TEXT-form Decimal values.
  - 3-case integration test at
    `crates/reports/tests/reconciliation.rs:13-87`:
    `t808_case_1_all_zero_deltas_all_passed_true`,
    `t808_case_2_one_cent_imbalance_only_that_row_fails`,
    `t808_case_3_to_failure_json_round_trips_through_serde_value`.
  - Plus 7 in-module unit tests at
    `crates/reports/src/reconcile.rs:201-291`.
  - Test cmd: `cargo test -p reports --test reconciliation`.
  - Output lines:
    `test t808_case_1_all_zero_deltas_all_passed_true ... ok`,
    `test t808_case_2_one_cent_imbalance_only_that_row_fails ... ok`,
    `test t808_case_3_to_failure_json_round_trips_through_serde_value ... ok`
    (3 passed).
  - In-module: `cargo test -p reports --lib reconcile::tests` →
    `test reconcile::tests::t808_balanced_inputs_all_pass ... ok`
    et al. (7 passed).
  - `cargo clippy -p reports --tests -- -D warnings` clean.

- [x] **T809** [developer] — Kill-switch trip → audit + incident
  report wiring per
  [Design → Q8 (cont.) — Kill-switch incident report wiring](../features/operator-success-reports.md#q8-cont--kill-switch-incident-report-wiring-r121c):
  - **Audit-side change:** rewrite `audit::journal::kill_switch_tripped`
    so it writes BOTH the existing zero-amount memo journal row
    (v0 backwards compat) AND a `strategy_events` row with
    `kind = "KillSwitchTripped"`. Both writes inside the same
    `sqlx::Transaction` so they're atomic.
  - **Agent-side change:** `KillSwitch::trip` in
    `crates/agent/src/kill_switch.rs` gains an `Arc<Ledger>`
    member (passed at `KillSwitch::new` time) and a
    `tokio::spawn` that calls the writer. Failure to write is
    warn-logged, never fatal.
  - **Incident-report spawn:** after the audit write, the trip
    handler spawns the reports binary out-of-process via
    `std::process::Command::new("target/release/report")` (or the
    debug-build `cargo run` fallback) with `--period since:<ts>`
    and `--output spec/reports/success/incident-<ts>.md`. Spawn
    is fire-and-forget (no await); failure is warn-logged.
  - All four tests in
    [Design → Test strategy](../features/operator-success-reports.md#test-strategy)
    that touch this path use a mocked spawn helper so the test
    suite does not actually compile-and-launch the reports binary.
  —
  _acceptance: integration test
  `crates/agent/tests/kill_switch_trip_writes_both.rs` triggers
  `KillSwitch::trip(HaltReason::Test)` and asserts (a) one new memo
  journal row, (b) one new `strategy_events` row with the
  `KillSwitchTripped` kind, (c) `Σ debits == Σ credits` unchanged,
  (d) the spawn helper was called with the expected `--period
  since:<ts>` argument._
  **[deps: T801, T807]**

  **Notes (developer, 2026-05-01):**
  - Audit-side rewrite of `audit::journal::kill_switch_tripped` at
    `crates/audit/src/journal.rs:297-405` — the function now opens a
    single `sqlx::Transaction`, writes the v0 zero-amount memo row
    (preserved byte-for-byte: same description format
    `registry:KillSwitchTripped:<reason>` and same JSON metadata
    payload `{event,reason,operator}` that `registry_event` produced),
    AND a new `strategy_events` row with `kind = "KillSwitchTripped"`,
    `strategy_id = NULL`, `error_summary = <reason>`. The memo row
    keeps RFC-3339 second precision (v0 byte-for-byte); the
    `strategy_events` row uses the 6-digit microsecond format the
    rest of v0.5+/v1+ writers use (HF-3 / determinism gate).
  - Agent-side wiring at `crates/agent/src/kill_switch.rs`:
    - `IncidentSpawnArgs` struct at line 79 — `halt_ts_rfc3339`
      + `reason` carried into the spawner.
    - `IncidentSpawner` trait at line 96 — production /
      test seam.
    - `CommandIncidentSpawner` at line 106 — production impl;
      tries `target/release/report`, falls back to
      `target/debug/report`; spawn is fire-and-forget (no `.wait()`);
      missing binary is warn-logged.
    - `MockIncidentSpawner` at line 161 — test recorder.
    - `KillSwitch::with_audit` at line 249 — new constructor that
      stores `Arc<audit::Ledger>` + `Arc<dyn IncidentSpawner>`.
    - `KillSwitch::trip` at line 279 — after the v0 broadcast, when
      `with_audit` was used, fire-and-forget tokio task that calls
      `audit::journal::kill_switch_tripped(&ledger, &reason_str,
      "kill_switch")`; then `spawner.spawn(IncidentSpawnArgs {
      halt_ts_rfc3339, reason })`. v0 `KillSwitch::new` callers
      retain v0 behavior unchanged (no audit write, no spawn).
  - `agent::main` rewires `KillSwitch` construction at
    `crates/agent/src/main.rs:96-101`: opens the audit ledger first,
    then constructs the kill switch with
    `Arc::new(CommandIncidentSpawner)` as the spawner.
  - Re-exports at `crates/agent/src/lib.rs:13-16`
    (`CommandIncidentSpawner`, `IncidentSpawnArgs`, `IncidentSpawner`,
    `MockIncidentSpawner`).
  - Audit-side dual-write tests at
    `crates/audit/tests/kill_switch_dual_write_test.rs:32-243`:
    `t809_kill_switch_tripped_writes_memo_and_strategy_event`,
    `t809_memo_row_byte_for_byte_v0_compat`,
    `t809_strategy_event_uses_microsecond_timestamp_format`,
    `t809_dual_write_atomic_in_one_transaction`.
    - Test cmd: `cargo test -p audit --test kill_switch_dual_write_test`.
    - Output: `test t809_kill_switch_tripped_writes_memo_and_strategy_event ... ok`,
      `test t809_memo_row_byte_for_byte_v0_compat ... ok`,
      `test t809_strategy_event_uses_microsecond_timestamp_format ... ok`,
      `test t809_dual_write_atomic_in_one_transaction ... ok`
      (4 passed).
  - Agent-side integration tests at
    `crates/agent/tests/kill_switch_trip_writes_both.rs:50-187`:
    `t809_trip_writes_audit_dual_and_calls_spawn_helper` (acceptance
    a/b/c/d), `t809_trip_is_idempotent_only_first_call_dual_writes`,
    `t809_trip_without_audit_wire_is_v0_compat`.
    - Test cmd: `cargo test -p agent --test kill_switch_trip_writes_both`.
    - Output: `test t809_trip_writes_audit_dual_and_calls_spawn_helper ... ok`,
      `test t809_trip_is_idempotent_only_first_call_dual_writes ... ok`,
      `test t809_trip_without_audit_wire_is_v0_compat ... ok`
      (3 passed).
  - `cargo clippy --workspace --tests -- -D warnings` clean (no new warnings).
  - Anchor regression: `bash scripts/verify_anchors.sh` reports
    `ANCHORS PASS  (9 / 9)` — neither the audit rewrite nor the
    agent kill-switch path is reachable from the backtest binary.

- [x] **T810** [developer] — Optional in-process cron under feature
  flag `--features in_process_cron` per
  [Design → Q7 — Cron trigger](../features/operator-success-reports.md#q7--cron-trigger-r121b):
  - Add `tokio_cron_scheduler` as an **optional** dep under the
    new feature flag (NOT enabled by default).
  - Behind the flag, `agent::main` spawns a scheduler that runs
    Mondays 09:00 (configurable via `cfg.reports.cron_expression`)
    and invokes `reports::generate(ReportWindow::Weekly, …)`
    in-process.
  - Default build (no flag) ships unchanged behavior.
  - Reference systemd timer + launchd plist files written under
    `ops/reports.timer.example`, `ops/reports.service.example`,
    `ops/com.trading.reports.plist.example` so operators can copy
    them. None of these files are wired into the build. —
  _acceptance: `cargo build --features in_process_cron` clean;
  `cargo build` (default features) unchanged; example ops files
  present and parseable by `systemd-analyze verify` (Linux) /
  `plutil -lint` (macOS) — verification optional, manually run by
  the developer._
  **[deps: T807]**

  **Notes (developer, 2026-05-01):**
  - Feature flag declaration at `crates/agent/Cargo.toml:15-20`
    (`in_process_cron = ["dep:tokio-cron-scheduler", "dep:reports"]`).
  - Optional deps at `crates/agent/Cargo.toml:31` (`reports = { path
    = "../reports", optional = true }`) and `crates/agent/Cargo.toml:56`
    (`tokio-cron-scheduler = { version = "0.15", optional = true }`).
    Default build pulls in **neither** — confirmed by
    `cargo build -p agent` succeeding without the new deps.
  - Cron module at `crates/agent/src/cron.rs:1-119`, gated
    `#![cfg(feature = "in_process_cron")]`.  `CronConfig` carries
    cron expression (default `"0 0 9 * * Mon"`), ledger DB path,
    parquet root, output dir.  `start(cfg)` creates a tokio
    `JobScheduler`, registers a `Job::new_async` that calls
    `reports::generate(ReportWindow::Weekly, ledger, marks, out, None)`
    on the cron schedule, starts the scheduler, returns the handle so
    the caller can hold it.  Failures inside fired jobs are
    warn-logged via `tracing::warn!` — never fatal.
  - Module export at `crates/agent/src/lib.rs:6-7` (cfg-gated).
  - `agent::main` cron startup at
    `crates/agent/src/main.rs:148-164`: behind `#[cfg(feature =
    "in_process_cron")]`, builds `CronConfig` from `cfg.audit.ledger_db_path`
    + `cfg.data.historical.parquet_root` + defaults, calls
    `agent::cron::start(cron_cfg).await`, holds the scheduler in a
    `_cron_scheduler` binding for the agent's lifetime.  Default
    builds skip the entire block.
  - Reference operator files (no build wiring):
    - `ops/reports.timer.example` — systemd timer (Mondays 09:00).
    - `ops/reports.service.example` — companion systemd unit, calls
      pre-built `target/release/report` (NOT `cargo run`, R-7
      mitigation).
    - `ops/com.trading.reports.plist.example` — launchd plist for
      macOS dev.  Validated with `plutil -lint
      ops/com.trading.reports.plist.example` → `OK`.
      `systemd-analyze verify` not available on this dev box; files
      validated by inspection.
  - Test cmds:
    - Default: `cargo build -p agent` →
      `Finished \`dev\` profile [unoptimized + debuginfo] target(s)`
      (no new deps pulled).
    - Feature: `cargo build -p agent --features in_process_cron` →
      `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 31.75s`
      (pulls in `tokio-cron-scheduler v0.15.1`, `reports`).
    - Lint feature: `cargo clippy -p agent --tests --features in_process_cron
      -- -D warnings` →
      `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 19.63s`
      (clean).
    - Default agent tests still green:
      `cargo test -p agent` → 23 + 3 + 1 + 3 + 2 + 3 + 3 + 4 + 3 + 4 + 3 = all
      passed (zero failures).

## Week 2 — render modules + integration

- [x] **T811** [developer] — Strategy decay heuristic + R7
  reflection-memory placeholder lifecycle note per
  [Design → R9 — Open risks (5 thresholds)](../features/operator-success-reports.md#r9--open-risks-5-thresholds)
  and Q9 carry-forward:
  - Implement the per-strategy decay computation: for each strategy
    in the active set, compose the equity curve restricted to that
    strategy's fills, compute since-inception Sharpe and last-7-day
    Sharpe via the R4 risk-metrics module, fire the risk if
    `last7d_sharpe < 0 && inception_sharpe > 0`.
  - Add a one-paragraph **forward-compatibility note** in
    `crates/reports/src/render/memory_highlights.rs` rustdoc that
    explains the placeholder string is locked into the v1+ anchor
    SHAs and will require a re-lock when the reflection-memory
    feature ships. The note explicitly references task T717 of
    `spec/tasks/v15a-mean-reversion-pairs.md` as the precedent for
    re-locking anchors.
  - **Optional but recommended:** open a stub TODO file
    `spec/reports/memory-anchor-relock-TBD.md` (a plain note, NOT
    a feature brief) that the eventual reflection-memory architect
    grep'd-for-marker. —
  _acceptance: decay computation produces correct fire/no-fire on
  a synthetic 2-strategy fixture; `memory_highlights.rs` rustdoc
  contains the note; `cargo doc -p reports` clean._
  **[deps: T807]**

  **Notes (developer, 2026-05-01):**
  - Decay heuristic + `StrategyEquitySlice` at
    `crates/reports/src/render/memory_highlights.rs:46-99` (`decay_fired`
    + `decayed_strategies`); both pure over their inputs (no I/O).
  - Forward-compatibility note in module rustdoc at
    `crates/reports/src/render/memory_highlights.rs:7-17`, references
    task T717 of `spec/tasks/v15a-mean-reversion-pairs.md` as the
    re-lock precedent.
  - `SharpeFn` injection point at
    `crates/reports/src/render/risk_metrics.rs:23` so T811 unit tests
    fire without depending on the eventual real R4 implementation.
  - Locked placeholder constant `PLACEHOLDER` at
    `crates/reports/src/render/memory_highlights.rs:35` (byte-stable
    across runs).
  - Stub note file at `spec/reports/memory-anchor-relock-TBD.md`
    documents the relock contract (not a feature brief — a TODO marker
    for the eventual reflection-memory architect).
  - 7 unit tests at
    `crates/reports/src/render/memory_highlights.rs:104-243`:
    `t811_render_returns_placeholder_byte_stable`,
    `t811_placeholder_contains_no_run_varying_fields`,
    `t811_decay_fires_when_inception_pos_and_last7d_neg`,
    `t811_decay_does_not_fire_when_both_positive`,
    `t811_decay_does_not_fire_when_inception_negative`,
    `t811_decay_two_strategy_fixture`,
    `t811_decayed_strategies_returns_sorted_ids`,
    `t811_decay_pure_two_calls_equal`.
  - Test cmd: `cargo test -p reports --lib render::memory_highlights`.
  - Output lines:
    `test render::memory_highlights::tests::t811_decay_two_strategy_fixture ... ok`,
    `test render::memory_highlights::tests::t811_render_returns_placeholder_byte_stable ... ok`,
    `test render::memory_highlights::tests::t811_decay_fires_when_inception_pos_and_last7d_neg ... ok`
    (8 passed).
  - `cargo doc -p reports --no-deps` clean.

- [x] **T812** [developer] — `MarkSource` trait + `ParquetMarkSource`
  + `FrozenMarkSource` (test) per
  [Design → Mark-to-market source](../features/operator-success-reports.md#mark-to-market-source-r111-r44):
  - Trait + trait method signatures in `crates/reports/src/marks.rs`.
  - `ParquetMarkSource` reads `data/binance/<symbol>/<year>/*.parquet`
    via Polars LazyFrame; LRU caches the last 4096 (symbol, ts)
    lookups.
  - `FrozenMarkSource` constructs from a checked-in CSV under
    `crates/reports/tests/fixtures/snapshot_marks.csv` (BTCUSDT
    + the v1.5a 4-symbol universe at 1m cadence over the test
    windows).
  - Both implementations return `MarkError::OutOfRange` on
    requests outside the loaded range. —
  _acceptance: unit test `crates/reports/tests/marks.rs` asserts
  (a) `ParquetMarkSource::close_at` returns the expected close at
  a known ts within the v1 / v1.5a parquet fixtures, (b)
  `FrozenMarkSource::close_at` round-trips against the checked-in
  CSV, (c) `close_series(BTCUSDT, t0, t1, 1)` returns `(t1-t0)/60`
  rows._
  **[deps: T807]**

  **Notes (developer, 2026-05-01):**
  - `MarkSource` trait at `crates/reports/src/marks.rs:36-72`
    (methods `close_at`, `close_series`).
  - `MarkError` at `crates/reports/src/marks.rs:18-29`
    (`OutOfRange`, `Io`, `Parse`).
  - `ParquetMarkSource` impl at `crates/reports/src/marks.rs:131-318`
    — Polars `LazyFrame::scan_parquet`, deterministic file walk
    (`year ASC, filename ASC`), 4096-entry LRU cache.
  - `FrozenMarkSource` impl at `crates/reports/src/marks.rs:323-444`
    — CSV (`symbol,close_time,close`), sorted by `(symbol,
    close_time)`, used by every `crates/reports/tests/` integration
    test that needs marks without parquet I/O.
  - Test fixture at `crates/reports/tests/fixtures/snapshot_marks.csv`
    (BTCUSDT + ETH/SOL/XRP USDT; 15 rows × 4 symbols at 1m cadence).
  - 7 integration tests at `crates/reports/tests/marks.rs:34-181`:
    `t812_frozen_close_at_round_trips_from_csv_fixture`,
    `t812_frozen_close_at_returns_out_of_range_below_first_bar`,
    `t812_frozen_close_series_btc_1m_cadence_row_count`,
    `t812_frozen_close_series_4_symbol_universe_round_trip`,
    `t812_parquet_close_at_returns_expected_close_via_tempdir_fixture`,
    `t812_parquet_close_at_out_of_range_below_first_bar`,
    `t812_parquet_close_series_returns_one_row_per_cadence`.
  - Plus 6 in-module unit tests at `crates/reports/src/marks.rs:447-565`.
  - Test cmd: `cargo test -p reports --test marks`.
  - Output lines:
    `test t812_frozen_close_at_round_trips_from_csv_fixture ... ok`,
    `test t812_parquet_close_at_returns_expected_close_via_tempdir_fixture ... ok`,
    `test t812_frozen_close_series_btc_1m_cadence_row_count ... ok`
    (7 passed in tests/marks.rs; 6 more in unit lib tests).
  - The acceptance text mentions parquet "fixtures" — there are no
    pre-shipped operator-success-report parquet fixtures, so the
    parquet tests build a tiny ad-hoc parquet file via Polars
    `ParquetWriter` in a `tempfile::tempdir()` for each test
    (deterministic + self-contained).
  - `cargo clippy -p reports --tests -- -D warnings` clean.

- [x] **T813** [developer] — Render modules R2–R9 per the brief +
  per
  [Design → `crates/reports/` layout](../features/operator-success-reports.md#cratesreports-layout):
  - **R2** `headline.rs`: strategy return + BTC buy-and-hold
    baseline; format per R2.3 to two decimal places of percent +
    cents-precise USDT.
  - **R3** `equity_curve.rs`: sample equity curve at 1m cadence
    for windows ≤ 7d, 5m cadence for > 7d; emit sparkline (R3.2)
    + write companion CSV (R3.3 columns).
  - **R4** `risk_metrics.rs`: Sharpe / Sortino / Calmar / max-DD
    / recovery-time as a 5-row markdown table (R4.3); reuse v1
    annualization constant `525_600`.
  - **R5** `strategy_attribution.rs`: render the per-strategy table
    using `pnl_by_strategy` (T803) + the strategy-active-set from
    `strategy_events_since(period_start)`. Strategies with zero
    trades render `(no activity)` (R5.2).
  - **R6** `memory_highlights.rs`: emit the literal placeholder
    string per R6.1 — byte-stable across runs (R6.3).
  - **R7** `system_health.rs`: 6-row table per R7.1; sources per
    [Design → R7.1](../features/operator-success-reports.md#r71--uptime--clock-skew--feed-reconnect-provenance).
  - **R8** `what_changed.rs`: chronological bullet list of
    `Load`/`Swap`/`Unload`/`Reject` events; `_no strategy
    lifecycle events in this period._` if empty (R8.3).
  - **R9** `open_risks.rs`: five threshold checks per R9.1; pinned
    above the equity curve in the body (R9.1); `_no open risks._`
    sentinel if all green; `unknown — see logs` per-risk on inner
    `Result::Err` (R9.3).
  - All renderers pure over their inputs (no I/O inside the render
    function); the `lib::generate` orchestrator does the I/O once
    per query and hands `Decimal` / `Vec<…>` slices to the renderers.
  - All companion CSVs written via `csv_artifacts.rs` per
    [Design → CSV artifact column schemas](../features/operator-success-reports.md#csv-artifact-column-schemas-r33--q5). —
  _acceptance: unit tests under `crates/reports/tests/` (one per
  R-item) assert exact-string match on hand-computed fixture
  ledgers; all CSVs produced with the documented columns; cargo
  test -p reports green._
  **[deps: T802, T803, T807, T808, T811, T812]**
  - Honest-tick citations:
    - R2 headline — `crates/reports/src/render/headline.rs:41` —
      `cargo test -p reports --test headline_render` →
      `test t813_r2_headline_exact_string_match ... ok`
    - R3 equity_curve — `crates/reports/src/render/equity_curve.rs:31` —
      `cargo test -p reports --lib render::equity_curve` →
      `test render::equity_curve::tests::t813_equity_curve_section_renders_both_sparklines ... ok`
    - R4 risk_metrics — `crates/reports/src/render/risk_metrics.rs:62` —
      `cargo test -p reports --test risk_metrics` →
      `test t813_r4_render_table_contains_period_and_5_metric_rows ... ok`
    - R5 strategy_attribution —
      `crates/reports/src/render/strategy_attribution.rs:38` —
      `cargo test -p reports --test strategy_attribution` →
      `test t813_r5_two_strategy_table_renders_pnl_and_win_rate ... ok`
    - R6 memory_highlights — `crates/reports/src/render/memory_highlights.rs:57` —
      `cargo test -p reports --test memory_highlights` →
      `test t813_r6_render_with_decay_emits_footer_for_decayed_strategies ... ok`
    - R7 system_health — `crates/reports/src/render/system_health.rs:39` —
      `cargo test -p reports --test system_health` →
      `test t813_r7_renders_six_rows_with_known_values ... ok`
    - R8 what_changed — `crates/reports/src/render/what_changed.rs:26` —
      `cargo test -p reports --test what_changed` →
      `test t813_r8_load_swap_chronological_order_with_strategy_id ... ok`
    - R9 open_risks — `crates/reports/src/render/open_risks.rs:49` —
      `cargo test -p reports --test open_risks` →
      `test t813_r9_drawdown_fired_renders_threshold_and_observed ... ok`
    - R11 reconciliation appendix —
      `crates/reports/src/render/reconciliation.rs:20` —
      `cargo test -p reports --lib render::reconciliation` →
      `test render::reconciliation::tests::t813_reconciliation_section_contains_table_and_pass_cells ... ok`
    - csv_artifacts — `crates/reports/src/csv_artifacts.rs:74` —
      `cargo test -p reports --test csv_artifacts` →
      `test t813_csv_equity_header_and_row ... ok`
    - lib::generate orchestrator — `crates/reports/src/lib.rs:96` —
      `cargo test -p reports --test generate_smoke` →
      `test t813_generate_writes_markdown_and_csvs ... ok`

- [x] **T814** [developer] — Determinism + body-no-volatile-metadata +
  reconciliation FAIL integration tests per
  [Design → Test strategy](../features/operator-success-reports.md#test-strategy):
  - `crates/reports/tests/determinism.rs` runs `report-sample-7d`
    twice 10s apart against the same fixture at seed `0xC0FFEE`;
    asserts front-matter `generated:` differs but body-SHA256
    byte-identical (R10.3 / V4).
  - `crates/reports/tests/body_no_volatile_metadata.rs` asserts the
    8 forbidden substrings are absent from body bytes (R10.4).
  - `crates/reports/tests/reconciliation_mismatch.rs` injects a
    one-cent imbalance between two query reads, runs the binary,
    asserts (a) FAIL banner present in body, (b) `FAIL` cell in
    R11 table, (c) sibling `_reconciliation_failure.json` written
    with the expected schema, (d) bin exits 1 (R11.4 / V5). —
  _acceptance: all three tests green under
  `cargo test -p reports`; the determinism test passes a SHA-256
  byte-identity assertion across the two runs; the FAIL test
  surfaces the JSON sibling at the predicted path._
  **[deps: T813]**
  - Wave 2d-1 honest tick (developer, 2026-05-01):
    - determinism — `crates/reports/tests/determinism.rs:84` —
      `cargo test -p reports --test determinism` →
      `test t814_determinism_two_runs_same_seed_byte_identical_body ... ok`
    - body-no-volatile-metadata —
      `crates/reports/tests/body_no_volatile_metadata.rs:61` —
      `cargo test -p reports --test body_no_volatile_metadata` →
      `test t814_body_does_not_contain_any_volatile_substring ... ok`
    - reconciliation FAIL (lib path + JSON sidecar) —
      `crates/reports/tests/reconciliation_mismatch.rs:90` —
      `cargo test -p reports --test reconciliation_mismatch` →
      `test t814_reconciliation_fail_writes_banner_table_and_sibling_json ... ok`
    - reconciliation FAIL (bin exits 1) —
      `crates/reports/tests/reconciliation_mismatch.rs:171` —
      `cargo test -p reports --test reconciliation_mismatch` →
      `test t814_reconciliation_fail_bin_exits_one ... ok`
    - `cargo fmt --all -- --check` clean;
      `cargo clippy --workspace --tests -- -D warnings` clean.
    - No `crates/reports/src/` source edits — tests-only.

- [x] **T815** [developer] — Performance smoke test
  `crates/reports/tests/perf_smoke.rs` per R13:
  - Build a 1-year-history fixture ledger via
    `tests/fixtures/build_ledger_1y.rs` (composes
    `build_ledger_7d` + `build_ledger_90d` with synthetic fills
    at deterministic seed).
  - Run the binary with `--period 90d --ledger fixture.db --seed 0xC0FFEE`.
  - Assert wall-clock `< 10s` (R13.1) measured via
    `std::time::Instant::now`.
  - Assert RSS `< 256 MiB` (R13.3) measured via `getrusage` (or
    `peak_alloc` crate if `getrusage` is non-portable). —
  _acceptance: test green; fixture ledger ships under
  `crates/reports/tests/fixtures/`; perf budget enforced._
  **[deps: T813]**
  - file:line: `crates/reports/tests/perf_smoke.rs:106-176` (test fn
    `t815_perf_smoke_90d_under_10s_and_under_256mib`); fixture builder
    at `crates/reports/tests/fixtures/build_ledger_1y.rs:54-220`.
  - test cmd: `cargo test -p reports --test perf_smoke --release`.
  - output line: `test t815_perf_smoke_90d_under_10s_and_under_256mib ... ok`
    (`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0
    filtered out; finished in 3.17s`). With `--nocapture`: wall-clock
    0.247s (budget < 10s) and peak RSS 34.6 MiB (budget < 256 MiB) —
    both well under R13.1 / R13.3 ceilings. RSS measured via
    `libc::getrusage(RUSAGE_SELF, &mut u)` reading `ru_maxrss`, with a
    `cfg(target_os = "macos")` branch (bytes vs. KB). `libc` added as
    `[dev-dependencies]` only.

- [x] **T816** [developer] — Report scenarios `report-sample-7d` +
  `report-sample-90d` per
  [feature → Backtest Scenarios](../features/operator-success-reports.md#backtest-scenarios):
  - `crates/reports/tests/fixtures/build_ledger_7d.rs` constructs
    the deterministic 7-day SQLite snapshot per the brief
    Scenario `report-sample-7d` description (≥1 strategy `Load`,
    ≥3 closed trades across at least two strategies, ≥1
    `RebalanceRejected` event, ≥1 funding-rate observation row).
  - `crates/reports/tests/fixtures/build_ledger_90d.rs` for the
    90-day scenario (4 strategies, ≥1 strategy swap, ≥1
    `MeanReversionStop`, ≥1 deliberate drawdown excursion >
    11.25%).
  - Both scenarios run end-to-end via the bin; tester captures
    the body-SHA256 at first successful run; the two new SHAs
    extend the regression gate from 9 → 11 anchors.
  - **Cron-friendliness smoke (V10):** the test script runs the
    binary 3× in parallel from the same CWD against the same
    fixture; verifies all three exit 0 + produce byte-identical
    bodies + no partial files appear at any canonical path during
    the run. —
  _acceptance: both scenarios produce reports under
  `spec/reports/success/`; the artifacts directory contains the
  expected CSVs (`equity-since-inception.csv`, `equity-7d.csv`
  / `equity-90d.csv`, `fills.csv`, `pnl_by_strategy.csv`,
  `pnl_by_symbol.csv`, `journal.csv`, `strategy_events.csv`); body
  SHA256 byte-identical across two sequential runs._
  **[deps: T813, T814]**

  **Honest-tick citations (developer, 2026-05-01):**
  - **Fixture (7d):** `crates/reports/tests/fixtures/build_ledger_7d.rs`
    (266 lines; constants `PERIOD_START_RFC3339` L60, `PERIOD_END_RFC3339`
    L65, `FAR_FUTURE_RFC3339` L70; entry point `build_ledger_7d` L83;
    fill plan L139–L177).
  - **Fixture (90d):** `crates/reports/tests/fixtures/build_ledger_90d.rs`
    (entry point `build_ledger_90d`; fill plan covering 4 strategies
    incl. `pairs_zeta`; `Swap` event + `MeanReversionStop` event).
  - **Test file:** `crates/reports/tests/report_scenarios.rs` —
    `t816_report_sample_7d_determinism_and_anchor_lock` L168 (asserts
    body-SHA matches `EXPECTED_SHA_7D` L79
    `ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c`),
    `t816_report_sample_90d_determinism_and_anchor_lock` L226 (asserts
    body-SHA matches `EXPECTED_SHA_90D` L83
    `2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f`),
    V10 lib-level smoke `t816_v10_cron_friendly_3x_parallel_renders_atomic`
    L276 (3× concurrent renders + canonical-path partial-file poller),
    V10 bin-level smoke `t816_v10_cron_friendly_3x_parallel_bin_processes`
    L388 (3× `cargo run -p reports --bin report` processes).
  - **Anchor gate update:** `scripts/verify_anchors.sh` extended with
    an additive fallback to `spec/reports/success/success-*-<scenario>.md`
    when the original `backtest-*-<scenario>.md` glob misses (script
    L29–L42).
  - **Anchors appended:** `spec/anchors.toml` L60–L73 (two new
    `[[anchors]]` entries — report-sample-7d + report-sample-90d).
  - **Test cmd:** `cargo test -p reports --test report_scenarios`
  - **Output proving each test PASS:**
    `test t816_v10_cron_friendly_3x_parallel_renders_atomic ... ok`
    `test t816_report_sample_7d_determinism_and_anchor_lock ... ok`
    `test t816_report_sample_90d_determinism_and_anchor_lock ... ok`
    `test t816_v10_cron_friendly_3x_parallel_bin_processes ... ok`
    `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;`
    Re-run identical → green twice in a row.
  - **Anchor gate cmd:** `bash scripts/verify_anchors.sh`
  - **Anchor gate output:** `ANCHORS PASS  (11 / 11)` — every prior
    9 anchor SHA-256 unchanged byte-for-byte; new entries
    `report-sample-7d` and `report-sample-90d` PASS.
  - **fmt:** `cargo fmt --all -- --check` clean.
  - **clippy:** `cargo clippy --workspace --tests -- -D warnings` clean.

- [x] **T817** [developer] — v0 + v0.5 + v1 + v1.5a regression gate
  re-run per V6:
  - Re-run all 9 v0/v0.5/v1/v1.5a backtest scenarios through the
    v1+-extended workspace.
  - Body-SHA256s must match the locked anchors per V6 byte-identical:
    `btc-2023-1m-sma-cross` =
    `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`,
    `btc-2023-1m-sma-baseline-refresh` = same,
    `btc-2023-1m-macd-trend` = `ef9c5e48…`,
    `btc-2023-1m-rsi-reversion` = `bc56d20d…`,
    `btc-2023-1m-bbands-mean-revert` = `d8a08a23…`,
    `top10-2023-1h-momentum` =
    `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97`,
    `top10-2024-h1-momentum` =
    `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6`,
    `pairs-2023-zscore-mr` =
    `90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0`,
    `pairs-2024-h1-zscore-mr` =
    `14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f`.
  - **Critical:** the T802 schema migration + `post_fill` signature
    change must NOT shift any backtest body bytes. If any anchor
    drifts, route to architect — likely a determinism leak in the
    backtest binary's call to `post_fill` (e.g. the strategy id
    leaking into the report body). —
  _acceptance: all 9 anchor reports byte-identical;
  `cargo test --workspace` clean; `cargo clippy --workspace --
  -D warnings` clean._
  **[deps: T802, T813]**

  **Notes (orchestrator-verified, 2026-05-01):** background dev agent
  re-ran all 9 scenarios but its sandbox blocked `scripts/verify_anchors.sh`
  execution; agent honored the honest-tick rule and refused to tick.
  Orchestrator re-ran the gate from `/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/`:
  - 9 fresh reports on disk: `spec/reports/backtest-20260501-163{242,246,251,256,302,309,315,319,324}-<scenario>.md`.
  - Test cmd: `bash scripts/verify_anchors.sh`.
  - Output: `ANCHORS PASS  (9 / 9)` — every anchor SHA-256 matches
    `spec/anchors.toml` byte-for-byte (`fc2e3b4a…`, `fc2e3b4a…`,
    `ef9c5e48…`, `bc56d20d…`, `d8a08a23…`, `3b60ef07…`, `1f33534f…`,
    `90591a0e…`, `14f50a59…`).
  - Build proof: `cargo build --release --bin backtest` clean (per
    background agent's stdout); 9 scenario runs all completed without
    crashes (per background agent).
  - The T802 `post_fill` signature change + Wave 2c renderer additions
    did NOT shift any backtest report body bytes — V6 holds.

## Final

- [x] **T_FINAL_REPORTS** [tester] — End-to-end gate:
  - Both report scenarios (T816) green with deterministic body
    SHA256s captured + locked into the regression gate (9 → 11).
  - Determinism + reconciliation FAIL integration tests (T814)
    green.
  - Perf smoke (T815) under budget.
  - 9-anchor regression-free (T817) — every v0/v0.5/v1/v1.5a anchor
    SHA byte-identical.
  - Reconciler invariant `Σ debits == Σ credits` holds across both
    fixture ledgers + the 1-year fixture.
  - `audit::query::pnl_by_strategy` sum-equals-scalar invariant
    proven in T803's integration test.
  - `cargo run --bin report -- --period 7d --ledger
    target/test-ledgers/sample-7d.db` boots cleanly and exits 0.
  - `cargo run --bin trading -- --config config/agent.toml --mode
    research` boots cleanly with the v1+ kill-switch wire active
    (T809) — agent does NOT spawn a report on a clean boot. —
  _acceptance: tester's report template populated with both v1+
  report scenarios + the 9 v0/v0.5/v1/v1.5a regression reports;
  V1–V10 from the feature's Verification section pass._
  **[deps: T813, T814, T815, T816, T817]**

  **Tester honest-tick (2026-05-01, FINAL gate):**
  - **Test report:** `spec/reports/test-2026-05-01-1828-operator-success-reports-final.md`
    (V1–V10 matrix, all 10 VERIFIED).
  - **Anchor gate cmd:** `bash scripts/verify_anchors.sh`
  - **Anchor gate output:** `ANCHORS PASS  (11 / 11)` — every prior
    9 anchor SHA byte-identical; new T816 entries
    `report-sample-7d` (`ab06dbcb…`) + `report-sample-90d`
    (`2ef403f1…`) PASS.
  - **Workspace test cmd:** `cargo test --workspace --all-targets`
  - **Workspace test output:** exit 0; aggregate
    `PASS: 580  FAIL: 0  IGNORED: 3`. `reports` crate alone:
    143 PASS / 0 FAIL / 0 IGNORED.
  - **Bin smoke cmd:** `cargo run -p reports --bin report --
    --period 7d --ledger /tmp/audit.db --output /tmp/wave2d-smoke.md`
  - **Bin smoke output:** `wrote /tmp/wave2d-smoke.md (run_id=ccddc74afcca4f86)`,
    exit 0, 1899 B; full 12-field front-matter present.
  - **Static-analysis cmds:** `cargo fmt --all -- --check` exit 0;
    `cargo clippy --workspace --all-targets --all-features --
    -D warnings` exit 0;
    `cargo build -p agent --features in_process_cron` clean.
  - **Tick verifications:** T814 / T815 / T816 dev citations
    re-walked file:line + cmd + output; all VERIFIED (with
    minor 1–2 line drift on three constants/fns — function/test
    exists at adjacent location, per tester contract). T817's
    orchestrator-verified anchor-gate citation re-confirmed.
  - **V1–V10 verdict:** all VERIFIED. Mapping in test report §8.

## Parallelism map

```
Week 1 (types, schema, audit query, kill-switch wiring):
  developer:
    T801 ──► T802 ──► T803
       │       │
       ├──► T804
       ├──► T805
       └──► T806

Week 2 (reports crate, render, integration, e2e):
  developer:
    T804 ──► T807 ──► T808
                │
                ├──► T811
                ├──► T812
                ├──► T809  (touches agent::main + audit::journal::kill_switch_tripped)
                └──► T810  (touches agent::main; sequence after T809)

    T802, T803, T807, T808, T811, T812
                │
                ▼
              T813 (render modules R2–R9 + CSV artifacts)
                │
        ┌───────┼───────┐
        ▼       ▼       ▼
      T814    T815    T816 ──► T_FINAL_REPORTS
                              ▲
              T817 ──────────┘
```

**Handoff contract — no UI involvement:**

- v1+ ships zero new screens, zero widgets, zero new strings in
  `ui::strings`. The cockpit's `viewer` binary already renders
  `spec/reports/success/*.md` inline (per
  [architecture.md → Frontend → App layout](../architecture.md#app-layout)
  the `viewer` reads `spec/reports/` markdown + artifacts).
- Therefore no `[ui-designer]` task. The ui-designer is NOT spawned
  for this feature; the orchestrator's parallelism rule for
  developer || ui-designer does not apply here.
- If a future iteration adds a dashboard widget that reads a
  rendered report (e.g. a "this-week's-headline-return" badge in
  the cockpit), that widget gets its own feature brief and the
  ui-designer is brought in then.

## Notes

- Every task that writes spec files uses the `spec-update` skill.
- **T801** is the critical-path gate — it unblocks T802–T806 and
  the audit-side Q8 wiring. Do it first.
- **T802** is the load-bearing migration. The `post_fill` signature
  change touches every in-tree call site. The backtest-anchor
  re-run in T817 is the V6 gate; if any of the 9 anchors drift,
  route to architect — the most likely cause is the strategy id
  leaking into the backtest report body via a careless format
  string in `crates/backtest/src/main.rs::write_report` (the
  `strategy_id` is column-only at the SQL layer; it must NOT
  surface in the rendered body).
- **T803** is the heart of R5 strategy attribution. Sum-equals-scalar
  invariant + `(unattributed)` bucket are non-negotiable.
- **T807** is the lib skeleton — the gate for every render-module
  task (T813) and every integration test task (T814).
- **T809** is the audit-side rewrite of `kill_switch_tripped` — the
  load-bearing Q8 change. The v0 memo journal row stays (backwards
  compat); the new `strategy_events` row is the operator-success-
  report's source of truth for R7's "kill-switch trips" count.
- **T811** is forward-compat scaffolding for the future reflection-
  memory feature. The eventual reflection-memory architect must
  re-lock the two new operator-success-report anchors when R6
  gains real content; the precedent is v1.5a T717's top10 momentum
  re-lock.
- **T816** captures the two new anchor SHAs. Until the tester
  captures them, the regression gate is **9-anchor + new-scenario
  byte-identical-across-two-runs determinism check** for the two
  new scenarios.
- The 9 v0/v0.5/v1/v1.5a anchor hashes are **non-negotiable** post-
  T802. If any drift, route to **architect**.
- No new runtime crate dependency in default builds. Optional
  `tokio_cron_scheduler` only under the `in_process_cron` feature
  flag (T810).
- The `reports` binary is **read-only** over the audit DB. SQLite
  is opened with `PRAGMA query_only = 1` after `Ledger::open`.
  Concurrent reads while the agent holds the WAL write lock are
  safe.
- Determinism is non-negotiable: every render must run byte-
  identically across two invocations against the same ledger
  snapshot at the same `--seed`. The body-SHA256 anchors lock at
  the tester's first successful run.

## Changelog

- 2026-05-01 (architect): initial task breakdown — 17 tasks
  (T801–T817) + `T_FINAL_REPORTS`. Covers `StrategyEventKind`
  variant additions, two additive audit migrations
  (`004_journal_transactions_strategy_id.sql`,
  `005_uptime_intervals.sql`), `post_fill` signature change,
  `pnl_by_strategy` query, kill-switch trip Q8 dual-write,
  feed-reconnect writer, optional in-process cron flag, reports
  crate skeleton + render modules R2–R9, atomic write,
  reconciliation engine + sibling JSON, mark-source trait + parquet
  impl, strategy-decay heuristic, two report scenarios with
  body-SHA256 capture, 9-anchor regression gate. No `[ui-designer]`
  tasks — feature is non-UI per the cockpit/viewer split. Parallelism
  map + Q-resolution cross-references included.
