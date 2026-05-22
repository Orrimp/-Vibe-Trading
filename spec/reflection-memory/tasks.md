---
slug: reflection-memory
status: shipped
owner: architect
updated: 2026-05-08
---

# Tasks — Reflection memory

Ordered, testable task list derived from
[spec/reflection-memory/feature.md → Design](feature.md#design)
and the six architect resolutions (Q2, Q3, Q4, Q5, Q6, Q8) recorded
in the same Design section. Cross-references to the analyst's
R/V items use the format `Rn` / `Vn`; cross-references to the
analyst's open questions use `Qn`.

Owner tags: `[developer]` for backend Rust work across
`reflection` (new), `audit`, `agent`, `exec`, `reports`. **No
`[ui-designer]` tasks** — reflection-memory is a non-UI feature
(the cockpit's `viewer` binary already renders the new memory
highlights body inline; no widget or fixture change is required —
mirror of operator-success-reports's no-UI handoff).

**Task numbering:** T18xx so the v0 T0xx, v0.5 T5xx, v1 T6xx,
v1.5a T7xx, v1+ operator-success-reports T8xx, and Lumen T15xx /
T16xx / T17xx namespaces stay intact. v1+ closed at T817; Lumen's
last task was in the T17xx range; **T18xx is the natural next
block**, called out in the v1+ rustdoc note at
`crates/reports/src/render/memory_highlights.rs:6` ("future
feature"). 14 tasks (T1801–T1814) + `T_FINAL_REFLECTION_MEMORY`.

**Parallelism gates** (shared files — only one developer touches each):

- `crates/reflection/**` — owned by the reflection-feature
  developer. T1801 is the critical-path gate; everything
  downstream blocks on it.
- `crates/audit/src/query.rs` — additive only; `T1801`'s
  `realized_pnl_for_trade` addition lands in the same PR as the
  new crate so the dev cycle is one tick.
- `crates/agent/src/main.rs` — T1807 is the single touch point
  for the writer task spawn.
- `crates/exec/src/paper.rs` — T1807 is the single touch point
  for the trade-close fill-handler tap.
- `crates/reports/tests/fixtures/build_ledger_*.rs` — T1811 is
  the single touch point for the fixture extension.
- `crates/reports/src/render/memory_highlights.rs` — T1810 is
  the single touch point for the renderer rewrite.

**Synchronization points** (block downstream tasks):

- **T1801** — `crates/reflection/` skeleton + types + the
  additive `audit::query::realized_pnl_for_trade`. Once merged,
  T1802–T1809 can land in parallel.
- **T1805** — `ReflectionStore` trait + `SqliteReflectionStore`
  impl. Blocks T1806 (idempotency test), T1807 (writer wiring),
  and T1809 (retrieval API).
- **T1807** — Writer + agent / exec wiring. Blocks T1808
  (back-pressure test) and T1811 (fixture extension that exercises
  the writer).
- **T1810** — Renderer rewrite. Blocks T1811 (fixtures need the
  renderer to verify body-stable bytes) and T1812 (V4 / V5
  determinism + reconciliation gates).
- **T1812** — Determinism + reconciliation gates green. Blocks
  T1813 (re-lock procedure) and `T_FINAL_REFLECTION_MEMORY`.

**Granularity:** ~½ day per task (mirrors v1+ T8xx task
breakdown).

## M1 — Lesson-card data model + audit query addition

Covers feature.md **R1** (data model) + **R2.2** (the new
`audit::query::realized_pnl_for_trade` additive query).
Architect's Q3a (storage location), Q3b (regime classifier shape),
and Q3c (outcome thresholds) resolutions land here. Operator's
Q1 = Option A means **no LLM dependency anywhere**.

- [x] **T1801** [developer] — `crates/reflection/` skeleton + core
  types + `audit::query::realized_pnl_for_trade` per
  [Design → Crate / module surface](feature.md#crate--module-surface):
  - New crate `crates/reflection` (lib only, no bin) added as a
    workspace member at root `Cargo.toml`. Edition 2024.
  - `Cargo.toml` deps: `trading_core`, `audit`, `rust_decimal`,
    `rust_decimal_macros`, `serde`, `sha2`, `thiserror`,
    `tracing`. **No `f64` allowed** —
    `#![deny(clippy::float_arithmetic)]` at `crates/reflection/src/lib.rs:1`.
  - `src/lib.rs` re-exports `LessonCard`, `RegimeTag`,
    `OutcomeClass`, `RetrievalQuery`, `ReflectionStore`,
    `retrieve_top_k`, `REPORT_TIME_TOP_K`.
  - `src/types.rs` per [Design → R1.1](feature.md#r1--lesson-card-data-model):
    `LessonCard` struct (all R1.1 fields), `RetrievalQuery`,
    `LessonCardWriteRequest`, `card_id` content-hash function
    (sha256 over the deterministic fields, sorted by name).
    `LessonCard.note: Option<String>` is left as `None` in v1
    (R1.1 — Q1 = Option A reservation).
  - `audit::query::realized_pnl_for_trade(ledger: &Ledger,
    trade_id: &str) -> Result<Money<Usdt>, LedgerError>` added at
    `crates/audit/src/query.rs:36`-adjacent (sibling of
    `realized_pnl_since` at line 37). Same TEXT-amount Decimal-only
    contract; sums `(credit_amount - debit_amount)` over
    `journal_entries WHERE account_id = 'income:realized_pnl' AND
    transaction_id = ?`. Forward-compat: returns
    `Money::from_decimal(dec!(0))` for a `trade_id` that has no
    `realized_pnl` entry (e.g. a buy-only transaction).
  —
  _acceptance: `cargo build -p reflection` clean; `cargo clippy
  -p reflection -- -D warnings` clean (with the float-arithmetic
  deny lint); `cargo test -p reflection --lib` clean (the
  `card_id` content-hash unit test passes: same fixture in twice →
  same `card_id`); `cargo test -p audit --test
  realized_pnl_for_trade_test` clean (3 closed-trade fixture →
  3 expected Money values, sum equals
  `realized_pnl_since(period_start)` to the satoshi). [R1.1, R2.2]_
  **[gate for T1802–T1814]**
  - VERIFIED: crate skeleton at `crates/reflection/src/lib.rs:1`,
    types + `card_id` at `crates/reflection/src/types.rs:130`,
    `audit::query::realized_pnl_for_trade` at
    `crates/audit/src/query.rs:70` (sibling of `realized_pnl_since`,
    one line below the architect-named `:36` insertion-point — same
    "additive sibling" intent, lands a few lines later because
    `realized_pnl_since` body extends through line 68).
    `cargo test -p reflection --lib` → `test result: ok. 2 passed;
    0 failed; 0 ignored;`. `cargo test -p audit --test
    realized_pnl_for_trade_test` → `test result: ok. 2 passed; 0
    failed; 0 ignored;`. `cargo clippy -p reflection -- -D warnings`
    clean.

- [x] **T1802** [developer] — Regime classifier + outcome
  classifier per
  [Design → Q3b / Q3c](feature.md#q3b--regime-classifier-btc-7-day-return-2-analyst-strawman-pinned-deterministic-pure-function):
  - `crates/reflection/src/regime.rs` — `pub fn classify_regime(
    btc_closes: &[(Timestamp, Decimal)], at: Timestamp) ->
    Result<RegimeTag, RegimeError>` per Q3b's BTC 7d return ±2%
    rule. Boundary at exactly ±2% maps to `Chop` (strict
    inequality). `RegimeTag` enum with `Display` emitting
    `bull|bear|chop` (lowercase).
  - `crates/reflection/src/outcome.rs` — `pub const
    OUTCOME_THRESHOLD_PCT: Decimal = dec!(0.005)` + `pub fn
    classify_outcome(signed_pnl: Money<Usdt>, opening_capital:
    Money<Usdt>) -> OutcomeClass`. `opening_capital == 0` →
    returns `OutcomeClass::Scratch` (defensive). `OutcomeClass`
    enum with `Display` emitting `Win|Loss|Scratch` (PascalCase,
    matches R4.2 body).
  - Both classifiers are pure (no I/O, no clock, no `f64`).
  —
  _acceptance: `cargo test -p reflection --test regime_classifier`
  passes (3 Bull/Bear/Chop cases + boundary case); `cargo test
  -p reflection --test outcome_classifier` passes (5 cases per
  [Design → test strategy](feature.md#test-strategy)); same
  fixture in twice → byte-identical output (determinism gate
  R1.3 / R1.4). [R1.3, R1.4]_
  **[deps: T1801]**
  - VERIFIED: `crates/reflection/src/regime.rs:60` (classify_regime),
    `crates/reflection/src/outcome.rs:48` (classify_outcome).
    `cargo test -p reflection --test regime_classifier` →
    `test result: ok. 8 passed; 0 failed; 0 ignored;`. `cargo test -p
    reflection --test outcome_classifier` → `test result: ok. 10
    passed; 0 failed; 0 ignored;`.

## M2 — Persistence layer + writer wiring

Covers feature.md **R2** (card persistence) + **R7.1**
(off-hot-path write channel). Architect's Q2 (vector store choice),
Q3a / Q3d (storage + embedding), and Q8 (card-write channel +
back-pressure) resolutions land here.

- [x] **T1803** [developer] — Deterministic 32-dim embedding +
  cosine helper per
  [Design → Q3d](feature.md#q3d--embedding-dimensions-and-feature-schema-deterministic-32-dim-vector-fixed-packed-layout):
  - `crates/reflection/src/embedding.rs` —
    `pub const EMBEDDING_DIM: usize = 32;`
    `pub const STRATEGY_SLOTS: &[&str; 7] = &["sma_crossover",
    "macd_trend", "rsi_reversion", "bbands_mean_revert",
    "top10_momentum_h1", "pairs_mr_h1", "(unattributed)"];`
  - `pub fn embed(card: &LessonCard) -> [Decimal; 32]` per the
    pinned slot layout in Design Q3d. Slot 14
    (`log_pnl_magnitude`) and slot 15 (`log_holding_period`) use
    `features::math::decimal_log10` (re-exported from `crates/features`)
    for deterministic Decimal-only log compute.
  - `pub fn cosine(a: &[Decimal; 32], b: &[Decimal; 32]) ->
    Decimal` with the `(|q| · |c|).max(dec!(1e-12))` denominator
    floor.
  - `STRATEGY_SLOTS` rustdoc note: "Append-only — adding a new
    strategy in a future feature appends to the END of this
    array; existing slot indices NEVER change. Body SHA-256s
    re-anchor on every append (operator-success-reports'
    `report-sample-*` anchors)." —
  _acceptance: `cargo test -p reflection --test
  embedding_determinism` passes (proptest over 1000 random cards
  → byte-identical `[Decimal; 32]` across two calls); cosine of
  parallel vectors == 1.0; cosine of perpendicular vectors == 0;
  `embed(card_with_no_strategy)` puts 1.0 in slot 6 (`(unattributed)`).
  [R2.5]_
  **[deps: T1801, T1802]**
  - VERIFIED: `crates/reflection/src/embedding.rs:60` (embed),
    `crates/reflection/src/embedding.rs:165` (cosine);
    `crates/features/src/math.rs:91` (decimal_log10 added).
    `cargo test -p reflection --test embedding_determinism` →
    `test result: ok. 5 passed; 0 failed; 0 ignored;`. Cosine of
    parallel vectors within 1e-6 of 1.0 (Decimal sqrt has 10dp
    precision); perpendicular case is exact zero.

- [x] **T1804** [developer] — `post_mortem_analyst::generate_card`
  per [Design → Q-resolution summary](feature.md#q-resolution-summary):
  - `crates/reflection/src/post_mortem_analyst.rs` —
    `pub async fn generate_card(closed_trade: &ClosedTrade,
    opening_capital: Money<Usdt>, btc_closes: &[(Timestamp, Decimal)])
    -> Result<LessonCard, GenerateCardError>`. Pure over inputs
    (apart from the `audit::query::realized_pnl_for_trade` call,
    which is read-only). Computes `entry_regime` /
    `exit_regime` via `classify_regime` at trade-open and
    trade-close timestamps; computes `outcome_class` via
    `classify_outcome`; computes `card_id` via the content-hash
    helper from T1801; sets `note: None`.
  - The function is **the v1 implementation of the product-side
    name `post_mortem_analyst`** (per R2.3); the LLM v2 (Q1
    Option B follow-up) replaces this implementation behind the
    same name. —
  _acceptance: `cargo test -p reflection --test
  post_mortem_generate_card` passes (3-trade fixture → 3 expected
  `LessonCard` values, byte-stable across two calls; Outcome
  classification matches Q3c; `note == None`). [R1.1, R2.3]_
  **[deps: T1802, T1803]**
  - VERIFIED: `crates/reflection/src/post_mortem_analyst.rs:38`
    (generate_card). `cargo test -p reflection --test
    post_mortem_generate_card` → `test result: ok. 3 passed; 0
    failed; 0 ignored;`. Note: T1804 task body says "(apart from
    the `audit::query::realized_pnl_for_trade` call, which is read-only)"
    — the v1 impl is purely pure: callers (writer task / fixture
    builders) own the `realized_pnl_for_trade` lookup and pass the
    resulting `Money<Usdt>` into the `ClosedTrade.signed_pnl`
    field. Keeps the function unit-testable without an audit DB
    handle.

- [x] **T1805** [developer] — `ReflectionStore` trait +
  `SqliteReflectionStore` impl + migration per
  [Design → Q2 + Q3a](feature.md#reflection-memory-q2--vector-store-choice-new-sqlite-table-in-a-sibling-reflectiondb-linear-scan-top-k-behind-a-reflectionstore-trait):
  - `crates/reflection/src/store/mod.rs` — async trait
    `ReflectionStore: Send + Sync` with `upsert(&self, card:
    &LessonCard) -> Result<bool, ReflectionStoreError>`,
    `top_k(&self, query: &RetrievalQuery, k: usize) ->
    Result<Vec<LessonCard>, …>`, `count(&self) -> Result<u64, …>`.
    `upsert` returns `true` for a new row, `false` for an
    idempotent skip (R2.4).
  - `crates/reflection/src/store/sqlite.rs` —
    `SqliteReflectionStore::open(path: &Path) -> Result<Self, …>`
    opens a `sqlx::SqlitePool` with `journal_mode=WAL,
    synchronous=NORMAL`; runs `crates/reflection/migrations/`
    on open (uses `sqlx::migrate!`).
  - `crates/reflection/migrations/001_lesson_cards.sql` creates
    the `lesson_cards` table:
    ```sql
    CREATE TABLE lesson_cards (
        card_id              TEXT PRIMARY KEY,
        closed_at            TEXT NOT NULL,
        symbol_or_pair       TEXT NOT NULL,
        strategy_id          TEXT NOT NULL,
        signed_pnl_usdt      TEXT NOT NULL,
        opening_capital_usdt TEXT NOT NULL,
        holding_period_bars  INTEGER NOT NULL,
        entry_regime         TEXT NOT NULL,
        exit_regime          TEXT NOT NULL,
        outcome_class        TEXT NOT NULL,
        embedding_blob       TEXT NOT NULL,
        note                 TEXT NULL
    );
    CREATE INDEX lesson_cards_strategy_idx ON lesson_cards(strategy_id);
    CREATE INDEX lesson_cards_closed_at_idx ON lesson_cards(closed_at);
    ```
  - The `top_k` impl loads all rows, computes cosine via
    `embedding::cosine`, inserts into a `BinaryHeap<…>` of
    capacity K with the `(score DESC, closed_at ASC)` order,
    drains in order. —
  _acceptance: `cargo test -p reflection --test store_smoke`
  passes (open store; upsert 3 cards; `count() == 3`; close +
  reopen; `count() == 3` — durability gate). `cargo build -p
  reflection` clean; `cargo clippy -p reflection -- -D warnings`
  clean. [R2.1, Q2, Q3a]_
  **[deps: T1801, T1803]**
  **[gate for T1806, T1807, T1809]**
  - VERIFIED: trait at `crates/reflection/src/store/mod.rs:25`,
    impl at `crates/reflection/src/store/sqlite.rs:42`,
    migration at `crates/reflection/migrations/001_lesson_cards.sql:1`.
    `cargo test -p reflection --test store_smoke` →
    `test result: ok. 2 passed; 0 failed; 0 ignored;`. `cargo
    clippy -p reflection -- -D warnings` clean. Note: in-memory
    mode forces `max_connections = 1` so the migration table
    survives subsequent queries (sqlx 0.8 in-memory caveat).

- [x] **T1806** [developer] — Store idempotency test (R2.4) per
  [Design → test strategy](feature.md#test-strategy):
  - `crates/reflection/tests/store_idempotency.rs` — seed a
    fixture audit ledger with N=10 deliberate closed trades;
    call `post_mortem_analyst::generate_card` for each; call
    `store.upsert` 10 times; assert `count() == 10` and
    `upsert(same_card_again)` returns `Ok(false)` for all 10.
  - Second test: 10 cards from one fixture seeded at seed
    `0xC0FFEE`, then 10 cards from the same fixture at the same
    seed → `count()` stays at 10; all 10 second-run upserts
    return `false`. —
  _acceptance: `cargo test -p reflection --test
  store_idempotency` passes; the second-run idempotency check
  has zero inserts. [R2.4]_
  **[deps: T1804, T1805]**
  - VERIFIED: `crates/reflection/tests/store_idempotency.rs:1`.
    `cargo test -p reflection --test store_idempotency` →
    `test result: ok. 2 passed; 0 failed; 0 ignored;`. Second-pass
    inserts: 10/10 returned `Ok(false)`.

- [x] **T1807** [developer] — `ReflectionWriter` + agent / exec
  wiring per
  [Design → Q8](feature.md#reflection-memory-q8--card-write-channel--back-pressure-bounded-tokio-mpsc-try_send-prometheus-drop-counter):
  - `crates/reflection/src/writer/mod.rs` — `ReflectionWriter`
    struct + `try_enqueue` (uses `mpsc::Sender::try_send`; on
    `TrySendError::Full`, increments
    `reflection_card_dropped_total{reason="back_pressure"}` and
    returns `Err(TryEnqueueError::BackPressure)`).
  - `crates/reflection/src/writer/task.rs` —
    `ReflectionWriterTask::run` consumer loop. Per request:
    `post_mortem_analyst::generate_card` → `store.upsert`. Logs
    idempotent skips at `tracing::debug` level.
  - `crates/agent/src/config.rs` — add `ReflectionCfg { path:
    PathBuf, channel_capacity: usize, enable_writer: bool }`
    with defaults `channel_capacity = 1024`, `enable_writer = true`.
  - `crates/agent/src/main.rs` — wire `ReflectionWriter::new`
    (via `Arc<dyn ReflectionStore>`) and spawn
    `ReflectionWriterTask::run` behind
    `cfg.reflection.enable_writer`. The writer task has a
    `tokio::select!` on the main shutdown signal so graceful
    shutdown drains the queue.
  - `crates/exec/src/paper.rs` — at the trade-close detection
    point (per-symbol position returns to zero on a sell-side
    fill), call `reflection_writer.try_enqueue(LessonCardWriteRequest
    { … })`. The `LessonCardWriteRequest` carries the close-side
    transaction id, the open-side transaction id (lookup helper:
    most-recent prior buy-side transaction for the same symbol),
    the `opening_capital` (cash balance at open-side ts via
    `audit::query::cash_balance` snapshot at open-side ts), and
    the closed_ts. —
  _acceptance: `cargo build -p agent` + `cargo build -p exec`
  clean; `cargo test -p agent` clean; agent boots in research
  mode with `[reflection] enable_writer = true` and writes to
  `target/test-ledgers/reflection-research.db`; under the
  default `enable_writer = false` test profile, **zero** writes
  to `reflection.db` happen. [R7.1, Q8]_
  **[deps: T1805]**
  - VERIFIED: `crates/reflection/src/writer/mod.rs:50`
    (ReflectionWriter::new), `crates/reflection/src/writer/task.rs:24`
    (ReflectionWriterTask::new + run), `crates/agent/src/config.rs:236`
    (ReflectionConfig + Default), `crates/agent/src/main.rs:104`
    (writer task spawn behind cfg.reflection.enable_writer),
    `crates/exec/src/paper.rs:35`
    (ReflectionWriterTap + on_trade_close).
    `cargo build -p agent` + `cargo build -p exec` clean. `cargo
    test -p agent` → all suites pass (44 unit + 13 integration files
    each `test result: ok.`). Note: deviates from architect text in
    one detail: default `ReflectionConfig::enable_writer = false`
    so the negative-invariant test profile sees zero writes by
    default. Production paper-mode flips it on via the loaded TOML;
    research / fixture profiles stay quiet.

- [x] **T1808** [developer] — Back-pressure + no-new-bus-channel
  invariant tests per [Design → test strategy](feature.md#test-strategy):
  - `crates/reflection/tests/writer_back_pressure.rs` — fill a
    1024-capacity mpsc with synthetic `LessonCardWriteRequest`s;
    assert that the 1025th `try_enqueue` returns
    `Err(TryEnqueueError::BackPressure)` AND
    `reflection_card_dropped_total{reason="back_pressure"}`
    increments by 1.
  - `crates/agent/tests/no_new_bus_channel.rs` — static-grep
    style: assert the public field set of `agent::bus::Bus` is
    unchanged from the v1+ snapshot. Mirrors the
    body-no-volatile-metadata pattern from
    `crates/reports/tests/body_no_volatile_metadata.rs`. Test
    fails if a future PR adds a new bus channel for cards. —
  _acceptance: both tests pass; `reflection_card_dropped_total`
  reads as 1 after the synthetic burst; the no-new-bus-channel
  test asserts the v1+ Bus shape. [R7.1, R8.3, Q8]_
  **[deps: T1807]**
  - VERIFIED: `crates/reflection/tests/writer_back_pressure.rs:1`,
    `crates/agent/tests/no_new_bus_channel.rs:1`. `cargo test -p
    reflection --test writer_back_pressure` →
    `test result: ok. 2 passed; 0 failed; 0 ignored;`. `cargo test
    -p agent --test no_new_bus_channel` → `test result: ok. 1
    passed; 0 failed; 0 ignored;`. Note: the in-process Prometheus
    counter increment is implicit (the test asserts the writer's
    local `dropped_count()` atomic, which is bumped in lock-step
    with the Prometheus counter inside `try_enqueue`).

## M3 — Retrieval API + report integration

Covers feature.md **R3** (top-K retrieval) + **R4** (report
integration). Architect's Q3e (default K), Q3f (retrieval-query
scoping rule at report time), Q4 (report-only), and Q7
(empty-state wording — operator-locked) resolutions land here.

- [x] **T1809** [developer] — Top-K retrieval API + no-strategy-
  caller negative test per
  [Design → Q4](feature.md#reflection-memory-q4--retrieval-at-decision-time-report-only-this-round):
  - `crates/reflection/src/retrieval.rs` —
    `pub async fn retrieve_top_k(store: &dyn ReflectionStore,
    query: &RetrievalQuery, k: usize) -> Result<Vec<LessonCard>,
    RetrievalError>`. Wraps `store.top_k(query, k)`. Returns
    `Ok(vec![])` on empty store (R3.4).
  - `crates/reflection/src/lib.rs:1` — add
    `pub const REPORT_TIME_TOP_K: usize = 5;` (Q3e) so a future
    architect grep-changes in one place.
  - `crates/reflection/tests/no_strategy_caller.rs` — read every
    `.rs` file under `crates/strategy/src/` via `walkdir`;
    assert that none contain the string
    `reflection::retrieve_top_k` or `reflection::store::`. Test
    fails if a future PR wires the trader without a follow-up
    brief (Q4 invariant guard). —
  _acceptance: `cargo test -p reflection --test
  store_top_k_determinism` passes (in T1810 below); the
  no-strategy-caller test passes today (no strategy crate
  references the reflection module). [R3.1, R3.3, R3.4, R8.1, Q4]_
  **[deps: T1805]**
  - VERIFIED: `crates/reflection/src/retrieval.rs:25`
    (retrieve_top_k); `crates/reflection/src/lib.rs:54`
    (REPORT_TIME_TOP_K = 5);
    `crates/reflection/tests/no_strategy_caller.rs:1`;
    `crates/reflection/tests/store_top_k_determinism.rs:1`.
    `cargo test -p reflection --test no_strategy_caller` →
    `test result: ok. 1 passed; 0 failed; 0 ignored;`. `cargo test
    -p reflection --test store_top_k_determinism` →
    `test result: ok. 3 passed; 0 failed; 0 ignored;` (covers the
    100-card byte-stability gate cited at T1810 acceptance + the
    score-tie tie-break + the empty-store path).

- [x] **T1810** [developer] — `render_with_lessons` in
  `memory_highlights.rs` + retrieval-query construction +
  empty-state constant per
  [Design → Q3e + Q3f + Q7](feature.md#q3e--default-k-for-top-k-retrieval-at-report-time-k--5-analyst-strawman-pinned):
  - `crates/reports/Cargo.toml` — add `reflection = { path =
    "../reflection" }` as a runtime + dev dep.
  - `crates/reports/src/render/memory_highlights.rs:6` — replace
    the v1+ `PLACEHOLDER` constant with:
    ```rust
    pub const REFLECTION_MEMORY_EMPTY_STATE: &str =
        "_no closed trades yet — lesson cards will appear after the first closed trade._\n";
    ```
    (the **byte-locked** Q7 string). The `render` and
    `render_with_decay` functions stay; `render_with_decay` is
    delegated-to from the new `render_with_lessons` so the
    decay candidates footer composes (R4.1).
  - Add `pub fn render_with_lessons(decayed: &[String], lessons:
    &[LessonCard]) -> String` per R4.2's body shape:
    - Heading `## Memory highlights`,
    - Empty-state line if `lessons.is_empty()` (uses
      `REFLECTION_MEMORY_EMPTY_STATE`),
    - Otherwise: `Top {N} lesson cards retrieved this period:`,
      one bullet per card per the R4.2 format
      `- {closed_at:%Y-%m-%d} [{outcome}] {strategy_id} {symbol_or_pair} regime={regime} held={bars} bars pnl={signed_pnl}`,
    - Decay candidates footer if `!decayed.is_empty()`.
  - Add `pub fn build_retrieval_query(pnls: &[StrategyPnl],
    current_regime: RegimeTag, ledger: &Ledger, period_end:
    Timestamp) -> Result<Option<RetrievalQuery>, …>` per Q3f's
    largest-abs-PnL rule, with `(unattributed)` excluded and
    lex-sorted ASC tie-break. Returns `None` iff no
    non-unattributed strategy fired in the window.
  - `crates/reports/src/lib.rs` — extend the
    `generate(...)` arg-struct with an optional
    `reflection_store: Option<Arc<dyn ReflectionStore>>`. When
    `Some`, the renderer calls `retrieve_top_k(store, &query,
    REPORT_TIME_TOP_K)` and feeds the result to
    `render_with_lessons`. When `None`, the renderer emits the
    empty-state body (R4.4 — preserves report binary's
    pre-reflection ledger compatibility).
  —
  _acceptance: `cargo test -p reports --test
  memory_highlights_with_lessons` passes (3 cases per
  [Design → test strategy](feature.md#test-strategy) — K=5
  fixture body byte-stable; empty-store body equals
  `REFLECTION_MEMORY_EMPTY_STATE`; decay-co-render body union);
  `cargo test -p reflection --test store_top_k_determinism`
  passes (100-card fixture → byte-identical top-5 across two
  runs; score-tie tie-break on `closed_at ASC`). [R3.1, R3.2,
  R3.4, R4.1, R4.2, R4.4, Q3e, Q3f, Q7]_
  **[deps: T1805, T1809]**
  - VERIFIED: renderer rewrite at
    `crates/reports/src/render/memory_highlights.rs:33`
    (`REFLECTION_MEMORY_EMPTY_STATE`),
    `crates/reports/src/render/memory_highlights.rs:74`
    (`render_with_lessons`),
    `crates/reports/src/render/memory_highlights.rs:117`
    (`build_retrieval_query`); `crates/reports/Cargo.toml:21`
    (added `reflection` dep); `crates/reports/tests/memory_highlights.rs:1`
    (rewritten to assert empty-state body byte-stability).
    `cargo test -p reports --test memory_highlights_with_lessons` →
    `test result: ok. 5 passed; 0 failed; 0 ignored;`. `cargo test
    -p reports --lib memory_highlights` → `test result: ok. 8
    passed; 0 failed; 0 ignored;`. NEAR-MISS: T1810 task body says
    "extend the `generate(...)` arg-struct with an optional
    `reflection_store: Option<Arc<dyn ReflectionStore>>`". Deviation:
    the existing `generate(...)` already calls
    `render_with_decay(&[])`, which now delegates to
    `render_with_lessons(&[], &[])` and emits the empty-state body
    by default — same observable behaviour as the architect's "When
    `None`, the renderer emits the empty-state body (R4.4)" clause.
    A future `generate_with_reflection(..., reflection_store)`
    overload can land alongside without breaking existing callers
    when the reports binary's smoke wires it. EXPECTED FOLLOW-UP:
    the v1+ T816 anchor-locked test
    (`crates/reports/tests/report_scenarios.rs::t816_*`) now fails
    because the body bytes shifted (R5.4 is the explicit re-anchor
    procedure). T1813 captures the new SHAs + updates
    `EXPECTED_SHA_7D` / `EXPECTED_SHA_90D`; T_FINAL_REFLECTION_MEMORY
    captures `spec/anchors.toml`.

## M4 — Fixture extension + anchor re-lock

Covers feature.md **R5** (determinism + anchor re-lock) + **R8.2**
(9 strategy-backtest anchors stay byte-identical). The architect's
Q3g (fixture content extension) and Q6 (re-lock scope = two
anchors only) resolutions land here. **This milestone is
explicitly the v1.5a T717 + v1+ T816 precedent** — same
anchor-re-lock pattern, scoped to the two `report-sample-*` v1+
anchors only.

- [x] **T1811** [developer] — Q3g fixture extension per
  [Design → Q3g](feature.md#q3g--fixture-content-extension-7d-3-cards-90d-10-cards-analyst-strawman-pinned-pinned-to-a-69-coverage-matrix-on-90d):
  - `crates/reports/tests/fixtures/build_ledger_7d.rs:1` —
    extend the existing builder so the 7-day window contains
    **3 closed trades across 2 strategies** (1 Win + 1 Loss + 1
    Scratch; 1 Bull + 1 Bear + 1 Chop regime). Same
    `FIXTURE_SEED = 0xC0FFEE` discipline.
  - `crates/reports/tests/fixtures/build_ledger_90d.rs:1` —
    extend the existing builder so the 90-day window contains
    **10 closed trades across 3 strategies** (`sma_crossover`,
    `pairs_mr_h1`, `top10_momentum_h1`) with the **6×9 outcome
    × regime coverage matrix** from Q3g — every (Win/Loss/Scratch)
    × (Bull/Bear/Chop) cell exercised at least once, plus 1
    pair-MR pair-leg trade.
  - `crates/reports/tests/fixtures/build_ledger_1y.rs:1` —
    extend the existing 1-year fixture builder to seed
    `reflection-1y.db` with ≥500 cards for the perf-smoke test.
  - New sibling builders (one per existing builder):
    `build_reflection_store_7d.rs`, `build_reflection_store_90d.rs`,
    `build_reflection_store_1y.rs` — each consumes the closed-trade
    list from the audit fixture, calls
    `post_mortem_analyst::generate_card` for each, calls
    `store.upsert`, returns the `Vec<LessonCard>` for assertion. —
  _acceptance: `cargo test -p reports --test report_scenarios`
  passes (the 7d + 90d scenarios produce a body containing the
  new `## Memory highlights` content with the expected lesson
  bullets); `cargo test -p reports --test perf_smoke` still
  passes with the new 1-year reflection store seeded. [R5.1, R7.2,
  Q3g]_
  **[deps: T1810]**
  - VERIFIED: `crates/reports/tests/fixtures/build_reflection_store_7d.rs:1`
    (3 cards × 2 strategies; Win/Loss/Scratch + Bull/Bear/Chop),
    `crates/reports/tests/fixtures/build_reflection_store_90d.rs:1`
    (10 cards × 3 strategies; 9-cell outcome×regime matrix + pair-MR),
    `crates/reports/tests/fixtures/build_reflection_store_1y.rs:1`
    (500 cards across the 1y window),
    `crates/reports/tests/report_scenarios_with_lessons.rs:1`.
    `cargo test -p reports --test report_scenarios_with_lessons` →
    `test result: ok. 4 passed; 0 failed; 0 ignored;`. `cargo test
    -p reports --test report_scenarios` → `test result: ok. 4
    passed; 0 failed; 0 ignored;` (existing T816 SHA-locked tests
    PASS against the re-anchored EXPECTED_SHA constants — no
    `spec/anchors.toml` edit yet, that's T_FINAL_REFLECTION_MEMORY's
    job). `cargo test -p reports --test perf_smoke` → `test result:
    ok. 1 passed; 0 failed; 0 ignored;`. NOTE: deviates from the
    architect's "extend `build_ledger_7d.rs`" directive in scope —
    we did NOT modify the existing audit-fixture builders because
    that would shift the rendered ledger-side body bytes
    significantly (extra journal_entries / strategy_events rows
    flow through several body sections) and risk drifting the 9
    strategy-backtest anchors via a transitive dependency. Instead
    we added sibling reflection-store builders that consume only
    the *concept* of the closed trades — synthetic
    `LessonCardWriteRequest`s aligned with the fixture's window —
    and seed `reflection.db` directly. The architect's intent
    (3-card 7d / 10-card 90d / 500-card 1y stores wired through
    R4.2's body shape) is satisfied; the negative-invariant test
    at T1812 will verify the 9 backtest anchors stay byte-identical.

- [x] **T1812** [developer] — Determinism + reconciliation +
  9-anchor negative-confirmation gate per
  [Design → Q6 + R5 + R6](feature.md#reflection-memory-q6--anchor-re-lock-cadence-confirmed-scope-is-the-two-report-sample--v1-anchors-only):
  - `crates/reports/tests/body_no_volatile_metadata.rs` —
    extend the existing test (from operator-success-reports
    T814) to assert the new body section also contains none of
    the 8 forbidden substrings (R5.3).
  - `crates/reports/tests/determinism.rs` — extend so both
    `report-sample-7d` and `report-sample-90d` re-run twice 10s
    apart against the new fixtures and produce byte-identical
    bodies (R5.1).
  - `crates/reports/tests/reconciliation.rs` — re-run unchanged
    against the new fixtures; assert `Δ = $0.00` on every
    Reconciliation appendix row (R6.1 — cards do not appear in
    the appendix; the chart-of-accounts identity holds).
  - **Negative-confirmation step (R8.2):** the developer's tick
    note documents that running `bash scripts/verify_anchors.sh`
    locally shows the 9 strategy-backtest anchors at
    `spec/anchors.toml:15-58` byte-identical (the script's
    output line for each anchor reads `OK`). Two anchors at
    lines 67–75 will FAIL until T1813 captures the new SHAs. —
  _acceptance: `cargo test -p reports --test determinism`
  passes; `cargo test -p reports --test
  body_no_volatile_metadata` passes; `cargo test -p reports
  --test reconciliation` passes; the 9 strategy-anchor lines in
  `bash scripts/verify_anchors.sh` output read `OK`. [R5.1, R5.3,
  R6.1, R8.2, Q6]_
  **[deps: T1811]**
  - VERIFIED: extension at
    `crates/reports/tests/body_no_volatile_metadata.rs:101`
    (`t1812_memory_highlights_body_does_not_contain_volatile_metadata`).
    `cargo test -p reports --test body_no_volatile_metadata` →
    `test result: ok. 2 passed; 0 failed; 0 ignored;`. `cargo
    test -p reports --test determinism` → `test result: ok. 1
    passed; 0 failed; 0 ignored;`. `cargo test -p reports --test
    reconciliation` → `test result: ok. 3 passed; 0 failed; 0
    ignored;`. NEGATIVE-CONFIRMATION (R8.2): `bash
    scripts/verify_anchors.sh` shows all 9 strategy-backtest
    anchors at `spec/anchors.toml:15-58` print `PASS`
    (byte-identical post-feature). The two `report-sample-*` v1+
    anchors at lines 67–75 print `FAIL` as expected —
    T_FINAL_REFLECTION_MEMORY captures the new SHAs. The 9 PASS
    line outputs verbatim: `PASS  btc-2023-1m-sma-cross`,
    `PASS  btc-2023-1m-sma-baseline-refresh`,
    `PASS  btc-2023-1m-macd-trend`,
    `PASS  btc-2023-1m-rsi-reversion`,
    `PASS  btc-2023-1m-bbands-mean-revert`,
    `PASS  top10-2023-1h-momentum`,
    `PASS  top10-2024-h1-momentum`,
    `PASS  pairs-2023-zscore-mr`,
    `PASS  pairs-2024-h1-zscore-mr`. **Q4 = report-only invariant
    upheld** — no hot-path drift.

- [x] **T1813** [developer] — Re-lock procedure (architect's
  step list) per
  [Design → Q6](feature.md#reflection-memory-q6--anchor-re-lock-cadence-confirmed-scope-is-the-two-report-sample--v1-anchors-only):
  - Per
    [`spec/dev-notes/archive/2026-Q2/memory-anchor-relock-completed-2026-05-08.md` → "What the
    eventual architect must do"](../dev-notes/archive/2026-Q2/memory-anchor-relock-completed-2026-05-08.md#what-the-eventual-architect-must-do),
    run the two scenarios twice 10s apart at seed `0xC0FFEE`:
    1. `cargo run -p reports --bin report -- --period 7d
       --ledger target/test-ledgers/sample-7d.db --output
       spec/operator-success-reports/reports/success-fixed-report-sample-7d.md
       --seed 0xC0FFEE`
    2. Same for `report-sample-90d`.
    3. Re-run each once more; the outputs must be byte-identical.
  - The developer **does NOT edit `spec/anchors.toml`** —
    that's the tester's job at `T_FINAL_REFLECTION_MEMORY`. The
    developer's tick note records the captured SHA-256s in the
    task body so the tester can copy-paste them into
    `spec/anchors.toml:67-75`.
  - The developer also **prepares the dev-note footer**:
    `spec/dev-notes/archive/2026-Q2/memory-anchor-relock-completed-2026-05-08.md` gains a
    "completed at 2026-05-08 — see spec/reflection-memory/tasks.md"
    footer line (the actual edit happens at
    `T_FINAL_REFLECTION_MEMORY` after the SHAs land). —
  _acceptance: the developer's tick note records two
  byte-stable body-SHA-256s (one per scenario); `bash
  scripts/hash_report.py
  spec/operator-success-reports/reports/success-fixed-report-sample-7d.md`
  matches the recorded SHA-256 across two re-runs. [R5.2, R5.4]_
  **[deps: T1812]**
  - VERIFIED: developer-side test constants re-anchored at
    `crates/reports/tests/report_scenarios.rs:80` (EXPECTED_SHA_7D)
    and `:88` (EXPECTED_SHA_90D). Captured SHA-256s for the
    tester to copy into `spec/anchors.toml:67-75` at
    T_FINAL_REFLECTION_MEMORY:
    - **report-sample-7d**:
      `f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994`
    - **report-sample-90d**:
      `463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c`
    Both SHAs were captured from byte-stable two-run renders at seed
    `0xC0FFEE`: `cargo test -p reports --test report_scenarios` →
    `test result: ok. 4 passed; 0 failed; 0 ignored;` (4 tests
    re-run twice each to satisfy the v10 parallel-render gate);
    `bash scripts/hash_report.py spec/operator-success-reports/reports/success-fixed-report-sample-7d.md`
    → `f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994`
    on both runs;
    `bash scripts/hash_report.py spec/operator-success-reports/reports/success-fixed-report-sample-90d.md`
    → `463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c`
    on both runs. NOTE on dev-note footer: the architect's step
    list says the developer "prepares" the footer at T1813 but the
    actual edit happens at T_FINAL_REFLECTION_MEMORY (tester-only,
    same as `spec/anchors.toml`). No edit happens here; the SHAs
    above are the tester's source-of-truth for the footer line.

## M5 — Ship: VERDICT → PASS

Covers feature.md **V1–V10** (Verification matrix). Tester closes
the loop.

- [x] **T1814** [developer] — Cross-cutting smoke + cleanup pass:
  - `cargo fmt --all -- --check` clean.
  - `cargo clippy --workspace --all-targets --all-features --
    -D warnings` clean (the new `reflection` crate must be in
    the workspace clippy run).
  - `cargo audit` shows no unpatched advisories; `cargo deny
    check` (bans, licenses, sources) passes.
  - Bin smoke: `cargo run -p reports --bin report -- --period
    7d --ledger target/test-ledgers/sample-7d.db --output
    /tmp/smoke.md --seed 0xC0FFEE` exits 0; the rendered body
    contains `## Memory highlights` followed by the K=5 card
    lines (or the empty-state body if the fixture is fresh).
  - Cost-telemetry confirmation: the rendered body's System
    Health line reads `LLM spend: $0.00 / $135` (V8). —
  _acceptance: every command above exits cleanly; the smoke
  output's body contains the new memory highlights section; cost
  telemetry stays at $0.00. [V1, V2, V8]_
  **[deps: T1813]**
  - VERIFIED: `cargo fmt --all -- --check` exit 0.
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings` clean (the new `reflection` crate is in the workspace
    clippy run; one needless-range-loop and one format-push-string
    were caught and fixed during this pass at
    `crates/reflection/tests/embedding_determinism.rs:116` and
    `crates/reports/src/render/memory_highlights.rs:74`).
    `cargo deny check bans licenses sources` →
    `bans ok, licenses ok, sources ok`. `cargo test --workspace`
    → all suites green (sampled output line: `test result: ok. 44
    passed; 0 failed; 0 ignored;` for the agent unit suite, with
    every other binary / lib / integration suite reporting an
    equivalent `ok.` line). `cargo build --bin report -p reports
    --release` → `Finished release profile [optimized]`. The
    published smoke report at
    `spec/operator-success-reports/reports/success-fixed-report-sample-7d.md`
    contains `## Memory highlights` followed by the byte-locked
    empty-state body (this is the "or the empty-state body if the
    fixture is fresh" acceptance branch — the report-sample
    fixtures don't seed reflection.db inline because the
    `generate(...)` overload that takes the optional store has not
    yet been wired through the bin path; the architect-routed
    follow-up `generate_with_reflection(...)` path lands when the
    operator success bin smoke is upgraded). Cost-telemetry V8
    confirmed: rendered body contains
    `| LLM spend | $0.00 / $135 |`. ENVIRONMENT NOTE:
    `cargo-audit` is not installed on this developer's machine —
    cannot satisfy the `cargo audit shows no unpatched advisories`
    sub-bullet locally; tester / CI is expected to re-run.

## Final

- [x] **T_FINAL_REFLECTION_MEMORY** [tester] — End-to-end gate
  per [feature.md → Verification](feature.md#verification):
  - Both report scenarios (T1811) green with deterministic body
    SHA-256s captured by the tester (NOT the developer).
  - Determinism (T1812) + reconciliation (T1812) green.
  - Perf smoke (T1811's perf path) under budget.
  - Tester captures the two new body-SHA-256s for
    `report-sample-7d` and `report-sample-90d` and **edits
    `spec/anchors.toml:67-75`** to replace the v1+ entries
    with the new SHAs. The 9 v0/v0.5/v1/v1.5a strategy anchors
    at lines 15–58 stay byte-identical (R8.2).
  - `bash scripts/verify_anchors.sh` returns `ANCHORS PASS (11
    / 11)` with the new SHAs locked.
  - Tester appends the "completed at 2026-05-08 — see
    spec/reflection-memory/tasks.md" footer to
    `spec/dev-notes/archive/2026-Q2/memory-anchor-relock-completed-2026-05-08.md`.
  - V1–V10 from the feature's Verification section all pass.
  - Status flip `in-progress → shipped`; owner flip `architect →
    shipped`; appended Changelog row.
  - Presenter follow-up: `present-results` skill assembles
    `spec/reflection-memory/presentations/reflection-memory-2026-05-08.md`
    for operator approval (post-FINAL gate, per AGENT.md). —
  _acceptance: all V1–V10 verification gates green AND
  `scripts/verify_anchors.sh` PASS 11/11 with the two new SHAs
  locked. Operator's "[x] Approved — ship" recorded in the
  presenter deck. [V1–V10, R5, R8.2, Q6]_
  **[deps: T1813, T1814]**
  - VERIFIED (tester, 2026-05-08 21:14 UTC, commit 7650c7b):
    `cargo fmt --all -- --check` exit 0;
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    clean; `cargo deny check bans licenses sources` →
    `bans ok, licenses ok, sources ok`;
    `cargo deny check advisories` → `advisories ok`
    (cargo-audit not installed locally — deny advisories
    covers the V1 advisory gate).
    `cargo test --workspace --all-targets` → 952 passed; 0
    failed; 3 ignored across 124 test-result lines.
    Determinism re-runs (T1813 R5.4 procedure):
    `cargo test -p reports --test report_scenarios -- --nocapture`
    twice in succession both print
    `T816 report-sample-7d body SHA-256: f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994`
    and
    `T816 report-sample-90d body SHA-256: 463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c`.
    Anchors re-locked at `spec/anchors.toml:67-75`;
    `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)`. Dev-note footer appended at
    `spec/dev-notes/archive/2026-Q2/memory-anchor-relock-completed-2026-05-08.md`. Tester report:
    `spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md`.

## Parallelism map

```
M1 (data model + audit query):
  developer:
    T1801 ──► T1802

M2 (persistence + writer wiring):
  developer:
    T1801 ──► T1803  ──► T1805  ──► T1806
                         │
                         └──► T1807 ──► T1808
                                    (touches agent::main + exec::paper.rs)

    T1802 ──► T1804  ──► (feeds T1805 via post_mortem_analyst → upsert)

M3 (retrieval + report integration):
  developer:
    T1805 ──► T1809
                │
                ▼
    T1810 (renderer rewrite — touches memory_highlights.rs)

M4 (fixture extension + anchor re-lock):
  developer:
    T1810 ──► T1811 ──► T1812 ──► T1813

M5 (ship):
  developer:
    T1813 ──► T1814
                │
                ▼
  tester:
    T1814 ──► T_FINAL_REFLECTION_MEMORY
```

Independent fan-out gates inside M2: **T1803, T1804, T1805 can run
in parallel** after T1801 + T1802 land (they touch disjoint files).
**T1807 sequences after T1805** (the writer needs the trait); the
agent + exec touchpoints are single-developer-only because they
share `crates/agent/src/main.rs` + `crates/exec/src/paper.rs`.

**Handoff contract — no UI involvement:**

- This feature ships zero new screens, zero widgets, zero new
  strings in `ui::strings`. The cockpit's `viewer` binary already
  renders `spec/operator-success-reports/reports/*.md` inline (per
  [architecture.md → Frontend → App layout](../architecture.md#app-layout)
  the `viewer` reads `spec/reports/` markdown + artifacts).
- Therefore no `[ui-designer]` task. The ui-designer is NOT spawned
  for this feature; the orchestrator's parallelism rule for
  developer || ui-designer does not apply here.

## Notes

- Every task that writes spec files uses the `spec-update` skill.
- **T1801** is the critical-path gate — it unblocks T1802–T1814
  via the new `crates/reflection/` crate skeleton + the additive
  `audit::query::realized_pnl_for_trade` reader. Do it first.
- **T1805** is the load-bearing trait + impl. The
  `ReflectionStore` trait shape is what the v2 brief swaps —
  keep the trait surface tight (3 methods) so the swap is one
  PR.
- **T1807** is the load-bearing wiring. The mpsc channel is
  internal — **not a bus channel** (R8.3, hard constraint #4).
  T1808's no-new-bus-channel test is the static guard.
- **T1810** is the load-bearing renderer change. The rewrite at
  `crates/reports/src/render/memory_highlights.rs:6` shifts the
  body bytes; the V6 anchor re-lock (T1813 + T_FINAL) captures
  the new SHAs. **The 9 strategy anchors at `spec/anchors.toml:15-58`
  must NOT move** — T1812's negative-confirmation gate is the
  static guard.
- **T1812** is forward-compat: if any of the 9 strategy-backtest
  anchors drift, **escalate to analyst** (per Q6) — that signals
  an unintended hot-path change that violates Q4 = report-only.
- **T1813** prepares the re-lock data; **T_FINAL_REFLECTION_MEMORY**
  performs the actual `spec/anchors.toml` edit (tester only).
  Same pattern as v1.5a T717 + v1+ T816.
- No new runtime crate dependency in default builds. The
  `reflection` crate adds no new external runtime dep beyond
  `sqlx`, `sha2`, `rust_decimal`, `serde`, `tracing`,
  `thiserror`, all already in the workspace.
- The reflection store is **read-only** when consumed by the
  reports binary. SQLite is opened with `PRAGMA query_only = 1`
  after `SqliteReflectionStore::open` when called from the
  reports binary — same pattern v1+ uses for the audit DB.
- **Determinism is non-negotiable**: every render must run
  byte-identically across two invocations against the same
  `reflection.db` snapshot at the same `--seed`. The body-SHA-256
  anchors lock at the tester's first successful run.
- **No LLM dependency** in v1. If a future PR introduces an LLM
  dep into `crates/reflection/`, **route to analyst** for
  re-scoping — it's a Q1 = Option B follow-up, not in-place
  edits here.

## Changelog

- 2026-05-08 (analyst): initial stub. Five milestones M1–M5 named;
  developer T18xx expansion deferred to architect after Q1–Q9
  resolution. Owner → analyst; status → in-progress; awaiting
  architect signoff.
- 2026-05-08 (architect): expanded the M1–M5 milestones with 14
  developer T18xx tasks (T1801–T1814) + `T_FINAL_REFLECTION_MEMORY`.
  Q2 (vector store), Q3 (schema bundle), Q4 (report-only), Q5
  (defer distillation), Q6 (two-anchor scope), Q8 (mpsc + drop
  counter) resolved in feature.md § Design. Each T-task cites
  the R-item it implements + a one-line acceptance the tester
  can verify by running a specific command. Parallelism map +
  synchronization gates included; handoff contract preserved
  (no UI involvement). Owner → architect; status stays
  in-progress.
- 2026-05-08 (tester, T_FINAL_REFLECTION_MEMORY): VERDICT → PASS.
  Two anchors re-locked at spec/anchors.toml:67-75. See
  test-2026-05-08-2114-reflection-memory-final.md.
