---
slug: real-mtm-unrealized-pnl
status: shipped
owner: tester
updated: 2026-05-02
---

# Tasks — Real mark-to-market unrealized P&L

Ordered, testable task list derived from
[spec/real-mtm-unrealized-pnl/feature.md → Design](../features/real-mtm-unrealized-pnl.md#design)
and the eight architect resolutions (Q1–Q8) + R10 deferral recorded
in the same Design section. Cross-references to the analyst's R/V
items use the format `Rn` / `Vn`; cross-references to the
architect's open questions use `Qn`.

T8xx is taken by [operator-success-reports](operator-success-reports.md);
T9xx is taken by [live-cockpit-unified](live-cockpit-unified.md);
this feature uses **T1001–T1008**.

Owner tags:
- `[developer]` — backend Rust work across `trading_core`, `audit`,
  `reports`.
- `[no UI work — reads existing bus PNL channel]` — the cockpit's
  PNL panel reads the bus's `pnl` channel (T903c reconciler);
  once `generate(...)` computes real unrealized, the cockpit picks
  it up automatically. **No `[ui-designer]` tasks.**
- `[tester]` — sole owner of `T_FINAL_REAL_MTM`.

**Parallelism gates** (shared files — only one task at a time
touches each):

- `crates/core/src/lib.rs` + `crates/core/src/position.rs` (NEW
  module) — T1001 is the sole writer; everything downstream reads
  the post-T1001 shape.
- `crates/audit/src/query.rs` — T1002 is the sole writer; appends
  the new reader at the bottom of the file.
- `crates/reports/src/lib.rs::generate(...)` — T1003 is the sole
  writer of the orchestrator diff (lines 135–150).
- `crates/reports/src/render/reconciliation.rs` (or equivalent) —
  T1003 also writes the `mark_unavailable: bool` field plumbing.
- `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`
  (NEW file) — T1004 is the sole creator.
- `crates/audit/tests/open_positions_at.rs` (NEW file) — T1005 is
  the sole writer.
- `crates/reports/tests/unrealized_orchestrator.rs` (NEW file) —
  T1006 is the sole writer.
- `crates/reports/tests/mark_unavailable_warns.rs` (NEW file) —
  T1006 also covers (V6 negative branch).
- `crates/reports/tests/perf_smoke_open_positions.rs` (NEW file)
  — T1007 is the sole writer.

**Synchronization points** (block downstream tasks):

- **T1001** — `OpenPosition` struct in `trading_core`. Critical-path
  gate; T1002 (audit reader) and T1003 (orchestrator) both import
  the type.
- **T1002** — `audit::query::open_positions_at` reader. Blocks T1003
  (orchestrator calls the reader), T1005 (V1 + V4 + V7 tests
  exercise it), T1007 (V8 perf test).
- **T1004** — new fixture `build_ledger_with_open_positions_7d.rs`.
  Blocks T1005 (V1/V4/V7 reads it), T1006 (V2 reads it), T1007
  (V8 reads a perf-extended variant of it).
- **T1003** — orchestrator diff. Blocks T1006 (V2 + V6 verify body
  bytes via `generate(...)`).
- **T1008** — anchor regression sweep (V3 / V5). Depends on T1003
  shipping. Independent of T1005/T1006/T1007 from a code-touch
  perspective; sequenced after T1003 only.

**Granularity:** ~½ day per task except T1006 (two integration
tests in one wave) and T_FINAL_REAL_MTM (tester gate). Tasks
numbered T10xx so v0 T0xx, v0.5 T5xx, v1 T6xx, v1.5a T7xx, v1+
T8xx, live-cockpit-unified T9xx namespaces stay intact.

## Week 1 — types, reader, orchestrator, fixture

- [x] **T1001** [developer] — `trading_core::OpenPosition` struct
  per [Design → Q2](../features/real-mtm-unrealized-pnl.md#q2--openposition-struct-fields-and-location):
  - New module `crates/core/src/position.rs` containing
    `pub struct OpenPosition { symbol: Symbol, qty: Decimal,
    avg_cost_basis: Money<Usdt>, opened_at: Timestamp,
    strategy_id: Option<StrategyId> }`.
  - `crates/core/src/lib.rs` adds `pub mod position;` +
    `pub use position::OpenPosition;`.
  - Derives: `Debug, Clone, PartialEq, Eq` (no `serde::Serialize` —
    not on the wire / not in front-matter; if a future cockpit-bus
    consumer needs `Serialize`, add additively).
  - Doc-comment on `avg_cost_basis` field calls out **per-unit**
    (USDT per unit of `symbol`), not notional, to prevent
    consumer-side confusion.
  - **Library checklist:** no new dep; `Symbol`, `Money`, `Usdt`,
    `StrategyId`, `Timestamp` already in scope.
  _acceptance: `cargo build -p trading_core` clean; new doctest
  `crates/core/src/position.rs::tests::t1001_open_position_partialeq_round_trip`
  passes; `cargo clippy --workspace -- -D warnings` clean; the
  `cargo test --workspace --doc` gate (the one that blew up on
  the `core` package-name shadow back in Week 1) stays green._
  **[gate for T1002, T1003]**

  **Honest-tick citations (2026-05-02, developer):**
  - file:line — `crates/core/src/position.rs:88` (`pub struct OpenPosition`);
    `crates/core/src/lib.rs:39` (re-export `pub use position::{OpenPosition, Position};`).
  - test cmd — `cargo test -p trading_core --lib position::tests`.
  - test output —
    ```
    test position::tests::t1001_open_position_partialeq_round_trip ... ok
    test position::tests::t1001_open_position_partialeq_distinguishes_strategy_id ... ok
    test position::tests::t1001_open_position_partialeq_distinguishes_qty_and_cost_basis ... ok

    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 42 filtered out; finished in 0.00s
    ```
  - validation gates — `cargo build -p trading_core` clean; `cargo clippy
    --workspace --all-targets --all-features -- -D warnings` clean;
    `cargo fmt --all -- --check` clean; `bash scripts/verify_anchors.sh`
    → `ANCHORS PASS  (11 / 11)`.
  - **deviation note:** the architect's Design § Q2 specifies derives
    `Debug, Clone, PartialEq, Eq` (no `serde::Serialize` — task body explicitly
    says "no `serde::Serialize` — not on the wire / not in front-matter; if a
    future cockpit-bus consumer needs `Serialize`, add additively"). Followed
    spec; round-trip test uses `Clone + PartialEq` (the acceptance-named
    `t1001_open_position_partialeq_round_trip` test) rather than serde.

- [x] **T1002** [developer] — `audit::query::open_positions_at`
  reader per
  [Design → Q1, Q3, Q7, Q8](../features/real-mtm-unrealized-pnl.md#q1--reader-signature-shape):
  - New `pub async fn open_positions_at(ledger: &Ledger, ts:
    Timestamp) -> Result<Vec<OpenPosition>, LedgerError>` appended
    to `crates/audit/src/query.rs`.
  - Reader query: `SELECT id, ts, description, strategy_id FROM
    journal_transactions WHERE (description LIKE 'buy %' OR
    description LIKE 'sell %') AND ts <= ? ORDER BY ts ASC`.
  - Symbol parse via the existing private
    `extract_symbol_from_description(&description)` helper at
    `query.rs:512` — same parser `pnl_by_symbol` and
    `recent_fills` use. Side parse + qty parse + price parse
    follow the existing `parse_fill_view_from_description`
    pattern at `query.rs:162-200`.
  - Per-`(symbol, strategy_id)` `BTreeMap<(Symbol,
    Option<StrategyId>), (running_qty, running_notional,
    first_buy_ts)>` accumulator. Per-fill update per
    [Design → Q7](../features/real-mtm-unrealized-pnl.md#q7--cost-basis-weighted-average-with-proportional-release).
  - End-of-scan: emit `OpenPosition` for each group with
    `running_qty > 0`. Net-zero groups skipped. `running_qty < 0`
    raises `LedgerError::Database("open_positions_at:
    net-negative qty for group …")` per
    [Q8](../features/real-mtm-unrealized-pnl.md#q8--long-only-at-v1-short--malformed-ledger-error).
  - Sort the emitted Vec by `(symbol ASC, strategy_id ASC, None
    last)` for determinism (R6).
  - **No new SQL index** (Q3). If V8 fails, follow-up
    `006_open_positions_index.sql` lands as a separate task.
  - **Determinism:** `BTreeMap` only (no `HashMap` per
    `query.rs::pnl_by_symbol` precedent line 480); `Decimal` only;
    `extract_symbol_from_description` is deterministic.
  _acceptance: `cargo build -p audit` clean; `cargo clippy -p audit
  --tests -- -D warnings` clean; the function appears in
  `audit::query::*` re-exports (`crates/audit/src/lib.rs`); a new
  unit test `crates/audit/src/query.rs::tests::t1002_open_positions_at_skips_zero_net_groups`
  exercises a 2-Buy + 2-Sell fixture and asserts an empty Vec._
  **[gate for T1003, T1005, T1007 — deps: T1001]**

  **Honest-tick citations (2026-05-01, developer):**
  - file:line — `crates/audit/src/query.rs:1008` (`pub async fn open_positions_at`);
    test file `crates/audit/tests/open_positions.rs:1` (8 tests covering
    empty / single / closed / weighted-avg / partial-close / multi-symbol-sort /
    strategy_id / net-negative branches).
  - test cmd — `cargo test -p audit --test open_positions`.
  - test output —
    ```
    running 8 tests
    test t1002_weighted_avg_cost_basis ... ok
    test t1002_empty_ledger_returns_empty_vec ... ok
    test t1002_net_negative_returns_err ... ok
    test t1002_closed_position_excluded ... ok
    test t1002_single_open_position ... ok
    test t1002_strategy_id_preserved ... ok
    test t1002_partial_close ... ok
    test t1002_multi_symbol_sorted ... ok

    test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
    ```
  - validation gates — `cargo build -p audit` clean; `cargo clippy -p audit
    -p trading_core --all-targets --all-features -- -D warnings` clean;
    `cargo fmt -p audit -p trading_core -- --check` clean; `bash
    scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.
  - **deviation note:** the architect's task body suggests an in-module
    unit test `query.rs::tests::t1002_open_positions_at_skips_zero_net_groups`,
    but the existing `query.rs` has no `#[cfg(test)] mod tests` block (the
    module-level `#![deny(clippy::expect_used, clippy::unwrap_used)]` in
    `audit/src/lib.rs` makes test ergonomics tricky for in-source tests
    relative to in-tree integration tests). Substituted an integration-test
    file `crates/audit/tests/open_positions.rs` with 8 tests including the
    closed-position case (`t1002_closed_position_excluded`) which fulfils
    the architect's intent. T1005 will further extend in-tree tests with
    V1 / V4 / V7 + the net-negative branch using the new fixture (already
    smoke-covered here via `t1002_net_negative_returns_err`).

- [x] **T1003** [developer] — Orchestrator integration in
  `crates/reports/src/lib.rs::generate(...)` per
  [Design → Orchestrator integration](../features/real-mtm-unrealized-pnl.md#orchestrator-integration--the-exact-diff):
  - Replace the hardcoded `let unrealized: Decimal = Decimal::ZERO;`
    block (lib.rs:135–150) with the open-positions loop documented
    in Design § Orchestrator integration.
  - Match arms: `Ok(mark)` → `unrealized += pos.qty * (mark -
    pos.avg_cost_basis.amount());`. `Err(MarkError::OutOfRange{..})`
    → `tracing::warn!` + `mark_misses += 1` + skip (Q6). Other
    `Err(e)` → `return Err(ReportError::Marks(e));`.
  - Track `let mark_unavailable_footnote = mark_misses > 0;` and
    thread the bool into the R11 reconciliation renderer's input
    struct (additive field `mark_unavailable: bool`). Default for
    the field is `false`; existing renderer call sites (no open
    positions) pass `false` and emit byte-identical bodies.
  - When `mark_unavailable_footnote == true`, the renderer
    appends a deterministic Markdown footnote string to the
    `unrealized` cell in the R11.1 reconciliation table:
    `*one or more open-position marks were unavailable at
    period_end; see logs*`. The string is constant; no
    interpolation of run-varying data.
  - The BTC baseline lookup (`marks.close_at(&btc_symbol,
    period_start)` etc.) keeps its existing `.ok()` pattern —
    untouched.
  - `recon_inputs.unrealized` and
    `recon_inputs.equity_check_sum` are already wired; the only
    semantic change is `unrealized` is no longer always zero.
  - `equity-<window>.csv`'s `unrealized_pnl_usdt` column now
    reflects the real value at the final row.
  - **Out of scope:** R3 equity-curve sampler stays
    cash-balance-only. Per-bar MTM curve walk is a v2+ wave.
  _acceptance: `cargo build -p reports` clean; `cargo clippy -p
  reports --tests --all-features -- -D warnings` clean; `cargo
  test -p reports --lib` (which exercises the existing
  no-open-positions code path against in-tree ledgers) stays
  green._
  **[gate for T1006, T1008 — deps: T1001, T1002]**

  **Honest-tick citations (2026-05-01, developer):**
  - file:line — `crates/reports/src/lib.rs:148` (open-positions
    loop replaces the `Decimal::ZERO` placeholder; reads via
    `audit::query::open_positions_at(&ledger, period_end)`,
    folds `qty * (mark - avg_cost_basis)` per resolved mark, and
    routes `MarkError::OutOfRange` through `tracing::warn!` +
    `mark_misses += 1` per Q6); `crates/reports/src/lib.rs:343`
    (renderer call extended to
    `render::reconciliation::render(&recon, mark_unavailable_footnote)`);
    `crates/reports/src/render/reconciliation.rs:21`
    (`pub const MARK_UNAVAILABLE_FOOTNOTE` — the deterministic
    Q6 body footnote); `crates/reports/src/render/reconciliation.rs:33`
    (`pub fn render(report, mark_unavailable: bool)` — the
    additive boolean field threading the footnote on/off);
    `crates/reports/tests/t1003_orchestrator_smoke.rs:120`
    (`t1003_orchestrator_with_zero_open_positions_keeps_anchor_byte_identical`),
    `crates/reports/tests/t1003_orchestrator_smoke.rs:165`
    (`t1003_orchestrator_with_open_positions_computes_unrealized`),
    `crates/reports/tests/t1003_orchestrator_smoke.rs:236`
    (`t1003_orchestrator_handles_mark_miss`).
  - test cmd — `cargo test -p reports --test t1003_orchestrator_smoke`.
  - test output —
    ```
    running 3 tests
    test t1003_orchestrator_with_zero_open_positions_keeps_anchor_byte_identical ... ok
    test t1003_orchestrator_with_open_positions_computes_unrealized ... ok
    test t1003_orchestrator_handles_mark_miss ... ok

    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
    ```
  - validation gates — `cargo build -p reports` clean; `cargo test
    -p reports` (full crate, 21 integration test binaries + 98 unit
    tests) clean; `cargo clippy --workspace --all-targets
    --all-features -- -D warnings` clean; `cargo fmt --all -- --check`
    clean; `bash scripts/verify_anchors.sh` →
    ```
    PASS  report-sample-7d                      ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c
    PASS  report-sample-90d                     2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f
    ---
    ANCHORS PASS  (11 / 11)
    ```
    confirming Q4's load-bearing claim — both v1+ anchor bodies
    are byte-identical to the pre-T1003 path (`build_ledger_7d` /
    `build_ledger_90d` have zero open positions at `period_end`,
    so the new loop is a no-op, `unrealized` stays `Decimal::ZERO`,
    `mark_misses` stays `0`, the footnote does not render, and the
    body bytes match the locked SHA-256s exactly).
  - **deviation note (collateral fix):** the
    `crates/reports/tests/perf_smoke.rs::t815_perf_smoke_90d_under_10s_and_under_256mib`
    test broke after T1003 because `build_ledger_1y.rs`'s side
    selection (`fills_written.is_multiple_of(2)`) interacted with
    the 4-cycle `(strategy, symbol)` group index to produce two
    Sell-only groups out of four — a malformed long-only ledger
    that `audit::query::open_positions_at` (T1002) raises on per
    Q8. Patched the fixture in place (`build_ledger_1y.rs:96-132`)
    to alternate side **within** each `(strategy, symbol)` group
    via a per-group lot-index counter, and to mirror each Sell's
    qty against the immediately-preceding Buy's qty so every group
    walks `0 → +qty → 0` (long-only at all times). The fixture is
    used **only** by the perf smoke (no anchor implications,
    verified by grep across `crates/` and `spec/`); the perf budget
    holds (`T815 wall-clock: 4.39s (budget < 10s) — PASS`). This
    is operationally a "long-only invariant" defect in the perf
    fixture that was never exercised before the T1003 reader
    landed; not a regression of T1003's behavior. Architect Q8
    explicitly mandates the loud error on net-negative qty, so
    relaxing `open_positions_at` is not an option.
  - **placement note:** the architect's Design § Q6 says the
    footnote is appended to the R11.1 reconciliation row's
    `unrealized` cell. Implemented as a separate footnote line
    appended **after** the appendix table (still within the
    `## Reconciliation` section — adjacent to the unrealized
    cell), gated by a new boolean parameter on
    `render::reconciliation::render`. Inlining the footnote text
    into a Decimal cell would have required reformatting the
    cell's value (currently rendered via `to_appendix_table()` as
    `Decimal::Display`). The literal string matches the
    architect's exact wording (`*one or more open-position marks
    were unavailable at period_end; see logs*`) so T1006 V6's
    forward-compat assertion will resolve unchanged. The flag is
    `false` on every empty-positions / fully-resolved code path,
    so existing fixtures emit byte-identical bodies (verified by
    smoke #1 + the 11/11 anchor sweep).
  - tester-owned: `T_FINAL_REAL_MTM` (per AGENT.md process
    discipline: developer never ticks `T_FINAL_*`).

- [x] **T1004** [developer] — Fixture
  `build_ledger_with_open_positions_7d.rs` per
  [Design → Q5](../features/real-mtm-unrealized-pnl.md#q5--fixture-choice-add-a-third-test-only-non-anchored):
  - New file
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`.
  - Constants mirror `build_ledger_7d.rs`: `FIXTURE_SEED =
    0x00C0_FFEE`, `PERIOD_START_RFC3339`, `PERIOD_END_RFC3339`,
    `FAR_FUTURE_RFC3339`. Same RFC-3339 + 6-digit-micros helpers.
  - Fill plan: copy the existing 7d 12-fill plan (6 (Buy, Sell)
    pairs across `strat_alpha`/BTCUSDT + `strat_beta`/ETHUSDT)
    AND add 2 dangling Buy fills at `(day=6, hour=20)`:
    - `(strat_alpha, BTCUSDT, Side::Buy, qty=0.01, price=60_000)`
    - `(strat_beta, ETHUSDT, Side::Buy, qty=0.20, price=3_000)`
  - Sibling helper `frozen_marks_csv() -> &'static str` returns a
    CSV body covering BTCUSDT @ 70_000 + ETHUSDT @ 3_500 at
    `period_end` (and at `period_start` so V2's sparkline /
    BTC-baseline reads also resolve).
  - Doc-comment: this fixture is **test-only**, **NOT** anchored.
    A `#[allow(dead_code)]` on every public helper because the
    fixture is loaded via `#[path = "..."]` from multiple test
    files (matching the existing pattern in
    `build_ledger_7d.rs:303`).
  - **Determinism:** all timestamps fixed RFC-3339; no
    `Uuid::new_v4()` (deterministic UUID derivation per the
    existing `deterministic_uuid` helper); RNG only for fee jitter
    seeded from `FIXTURE_SEED`.
  _acceptance: `cargo build -p reports --tests` clean; the
  fixture compiles standalone (no consumer yet); fixture-builder
  unit test `t1004_fixture_emits_two_open_positions` (a thin
  smoke that opens the ledger, calls `audit::query::recent_fills`,
  asserts 14 rows = 12 closed + 2 open) — kept lightweight; the
  full V1 assertions land in T1005._
  **[gate for T1005, T1006, T1007]**

  **Honest-tick citations (2026-05-01, developer):**
  - file:line —
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs:88`
    (`pub async fn build_ledger_with_open_positions_7d`);
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs:239`
    (`pub fn frozen_marks_csv`);
    `crates/reports/tests/fixture_with_open_positions_smoke.rs:25`
    (smoke `t1004_fixture_emits_two_open_positions`);
    `crates/reports/tests/fixture_with_open_positions_smoke.rs:115`
    (smoke `t1004_fixture_has_expected_open_positions_at_period_end`,
    exercising T1002's `audit::query::open_positions_at`).
  - test cmd — `cargo test -p reports --test fixture_with_open_positions_smoke`.
  - test output —
    ```
    running 3 tests
    test t1004_fixture_has_expected_open_positions_at_period_end ... ok
    test t1004_fixture_emits_two_open_positions ... ok
    test t1004_fixture_two_builds_byte_identical_fills ... ok

    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
    ```
  - validation gates — `cargo build -p reports --tests` clean;
    `cargo test -p reports` (full crate) clean; `cargo clippy
    --workspace --all-targets --all-features -- -D warnings` clean;
    `cargo fmt --all -- --check` clean; `bash scripts/verify_anchors.sh`
    → `ANCHORS PASS  (11 / 11)` (T1004 only adds a NEW test-only
    non-anchored fixture file + a NEW integration test target; existing
    fixtures `build_ledger_7d.rs` and `build_ledger_90d.rs` are
    untouched, so anchors stay byte-identical by construction).
  - **deviation note:** the smoke tests live in a sibling integration
    target `crates/reports/tests/fixture_with_open_positions_smoke.rs`
    rather than inline `#[cfg(test)] mod tests` because cargo only
    auto-generates test binaries from top-level `tests/*.rs` files —
    not from files re-mounted via `#[path]`. T1002's
    `audit::query::open_positions_at` had already landed on the trunk
    by the time T1004 ran, so the open-positions probe is wired up
    end-to-end (no `#[ignore]` needed). The spec's named acceptance
    test `t1004_fixture_emits_two_open_positions` is in place exactly
    as written, plus two parallel-safe siblings:
    `t1004_fixture_two_builds_byte_identical_fills` (determinism
    smoke; same seed → same `recent_fills` projection) and
    `t1004_fixture_has_expected_open_positions_at_period_end`
    (calls T1002's reader and asserts 2 open positions in
    alphabetical sort: BTCUSDT before ETHUSDT — full V1 byte-equality
    assertions still owned by T1005).

- [x] **T1005** [developer] — V1 + V4 + V7 tests in
  `crates/audit/tests/open_positions_at.rs`:
  - **V1 reader correctness:** open the
    `build_ledger_with_open_positions_7d` fixture; call
    `audit::query::open_positions_at(&ledger, period_end).await?`;
    assert the returned `Vec<OpenPosition>` is byte-identical (via
    `assert_eq!`) to a hand-computed expected vec of 2
    `OpenPosition` rows:
    - `{ symbol: BTCUSDT, qty: 0.01, avg_cost_basis: Money(60_000),
       opened_at: <fixture day-6 hour-20 ts>, strategy_id:
       Some("strat_alpha") }`
    - `{ symbol: ETHUSDT, qty: 0.20, avg_cost_basis: Money(3_000),
       opened_at: <fixture day-6 hour-20 ts>, strategy_id:
       Some("strat_beta") }`
    Order: BTCUSDT before ETHUSDT (alphabetical, R6).
  - **V4 reconciliation invariant:** for every `transaction_id` in
    the fixture's `journal_transactions`, call
    `audit::verify_balance(&ledger, txn_id).await?` and assert
    Ok. (The fixture writes only standard `post_fill` transactions
    + memo-row inception, so all double-entry sums are zero.)
  - **V7 determinism:** call `open_positions_at(&ledger,
    period_end)` twice in succession on the same opened ledger;
    `assert_eq!` the two `Vec<OpenPosition>` results.
  - Net-negative branch sub-test: build a tiny in-tempfile fixture
    with one Sell of `qty=1` against zero Buys; assert the
    returned error matches the
    `LedgerError::Database("open_positions_at: net-negative
    qty…")` pattern.
  _acceptance: `cargo test -p audit --test open_positions_at` →
  4/4 pass (`t1005_v1_reader_emits_two_open_positions`,
  `t1005_v4_balance_invariant_per_txn`,
  `t1005_v7_two_reads_byte_identical`,
  `t1005_q8_short_position_raises`)._
  **[deps: T1002, T1004]**

  **Honest-tick citations (2026-05-01, developer):**
  - file:line —
    `crates/audit/tests/open_positions_at.rs:83`
    (`t1005_v1_reader_emits_two_open_positions` — opens the T1004
    fixture via `#[path = "../../reports/tests/fixtures/build_ledger_with_open_positions_7d.rs"]`,
    calls `audit::query::open_positions_at(&ledger, period_end)`,
    asserts the returned `Vec<OpenPosition>` matches the architect's
    hand-computed expected vec byte-for-byte: BTCUSDT @ qty=0.01 /
    cost=60_000 / opened_at=2026-04-27T20:00:00Z / strat_alpha,
    then ETHUSDT @ qty=0.20 / cost=3_000 / opened_at=2026-04-27T20:00:00Z
    / strat_beta — alphabetical R6 sort);
    `crates/audit/tests/open_positions_at.rs:175`
    (`t1005_v4_balance_invariant_per_txn` — pulls every `id` from
    `journal_transactions` via `sqlx::query_as` and asserts
    `audit::journal::verify_balance(&ledger, txn_id).await == Ok(())`
    on each, covering all 14 `post_fill` rows + the bootstrap memo);
    `crates/audit/tests/open_positions_at.rs:212`
    (`t1005_v7_two_reads_byte_identical` — two consecutive
    `open_positions_at(&ledger, period_end)` calls on the same opened
    ledger return `assert_eq!`-equal `Vec<OpenPosition>` slices,
    plus a belt-and-braces check that `fixture_period_end()` matches
    the builder's returned `period_end`);
    `crates/audit/tests/open_positions_at.rs:241`
    (`t1005_q8_short_position_raises` — tiny in-tempfile fixture with
    one `Sell` of `qty=1` against zero Buys; asserts the returned
    error is `LedgerError::Database` and the message contains both
    `"net-negative qty"` and `"open_positions_at"` per architect Q8).
  - test cmd — `cargo test -p audit --test open_positions_at`.
  - test output —
    ```
    running 4 tests
    test t1005_q8_short_position_raises ... ok
    test t1005_v4_balance_invariant_per_txn ... ok
    test t1005_v1_reader_emits_two_open_positions ... ok
    test t1005_v7_two_reads_byte_identical ... ok

    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
    ```
  - validation gates — `cargo test -p audit` (full suite, all targets
    green); `cargo clippy --workspace --all-targets --all-features --
    -D warnings` clean; `cargo fmt --check` clean; `bash
    scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (T1005
    only adds a NEW test target + 2 dev-deps to `crates/audit/Cargo.toml`;
    no source/render path touched, anchors stay byte-identical by
    construction).
  - **deviation note (additive):** the T1004 fixture is mounted via
    `#[path = "../../reports/tests/fixtures/build_ledger_with_open_positions_7d.rs"]`
    so the V1 / V4 / V7 tests exercise the same deterministic ledger
    that T1003 (orchestrator) and T1006 (V2 / V6 in
    `crates/reports/tests/`) consume — single source of truth for the
    14-fill activity plan. The fixture's fee-jitter RNG is seeded by
    `ChaCha20Rng::seed_from_u64(FIXTURE_SEED)`; two dev-deps were
    added to `crates/audit/Cargo.toml` (`rand.workspace = true` /
    `rand_chacha.workspace = true`) solely so the fixture compiles
    when re-mounted from the audit tests directory. No `crates/audit/src/`
    change. No `crates/reports/` change. T1006/T1007 owners are
    untouched.
  - tester-owned: `T_FINAL_REAL_MTM` (per AGENT.md process discipline:
    developer never ticks `T_FINAL_*`).

- [x] **T1006** [developer] — V2 + V6 tests in
  `crates/reports/tests/unrealized_orchestrator.rs` +
  `crates/reports/tests/mark_unavailable_warns.rs`:
  - **V2 (`unrealized_orchestrator.rs`):** open the
    `build_ledger_with_open_positions_7d` fixture; load a
    `FrozenMarkSource::from_csv_str(frozen_marks_csv())`; call
    `reports::generate(...)`; parse the rendered markdown body
    and assert:
    - The R11.1 reconciliation row reports `unrealized = +200.00
      USDT` (BTC contribution +100 + ETH contribution +100).
    - `equity-<window>.csv`'s `unrealized_pnl_usdt` column at the
      last row equals `200.00`.
    - The `mark_unavailable` footnote is NOT present.
    - The body parses cleanly, no reconciliation FAIL exit.
  - **V6 negative (`mark_unavailable_warns.rs`):** same fixture
    but a `FrozenMarkSource` covering BTCUSDT only (ETHUSDT
    omitted). Call `generate(...)` and assert:
    - `unrealized = +100.00 USDT` (BTC contribution; ETH
      contribution = 0 because of the miss).
    - The body's R11.1 row contains the literal footnote string
      `*one or more open-position marks were unavailable at
      period_end; see logs*`.
    - The test captures `tracing` output via
      `tracing_subscriber::fmt::TestWriter` (or the existing
      pattern under `crates/reports/tests/`) and asserts the
      `"mark unavailable for open position"` log line fires
      exactly once with `symbol=ETHUSDT`.
    - The run does NOT return `Err(ReportError::Marks(...))`.
  - Both tests use the shared fixture through the `#[path =
    "fixtures/build_ledger_with_open_positions_7d.rs"]` pattern.
  _acceptance: `cargo test -p reports --test
  unrealized_orchestrator` → `t1006_v2_unrealized_equals_200_usdt`
  pass; `cargo test -p reports --test mark_unavailable_warns` →
  `t1006_v6_mark_miss_warns_and_zeroes`,
  `t1006_v6_footnote_present_when_miss` pass._
  **[deps: T1003, T1004]**

  **Honest-tick citations (2026-05-01, developer):**
  - file:line —
    `crates/reports/tests/unrealized_orchestrator.rs:92`
    (`t1006_v2_unrealized_equals_200_usdt` — V2 positive path: drives
    `reports::generate(...)` against the T1004 fixture under a
    `FrozenMarkSource` carrying BTCUSDT @ 70_000 + ETHUSDT @ 3_500 at
    `period_end`, parses the R11 reconciliation appendix's headline-row
    Ledger-side cell, and asserts `+200` USDT — BTC `0.01 × (70_000 −
    60_000) = +100` plus ETH `0.20 × (3_500 − 3_000) = +100` — plus
    `MARK_UNAVAILABLE_FOOTNOTE` absent + front-matter `reconciliation:
    PASS`);
    `crates/reports/tests/mark_unavailable_warns.rs:154`
    (`t1006_v6_mark_miss_warns_and_zeroes` — V6 negative path: drives
    `generate(...)` against the T1004 fixture under a `FrozenMarkSource`
    that omits ETHUSDT, captures every WARN-level event matching
    `"mark unavailable for open position"` via a custom
    `tracing_subscriber::Layer` + `WarnVisitor`, asserts exactly ONE
    captured event carrying `symbol = ETHUSDT`, asserts
    `generate(...)` returns `Ok(_)` per Q6, and asserts
    `MARK_UNAVAILABLE_FOOTNOTE` is present in the body);
    `crates/reports/tests/mark_unavailable_warns.rs:271`
    (`t1006_v6_footnote_present_when_miss` — body footnote literal
    asserted verbatim against the architect-locked Q6 string
    `*one or more open-position marks were unavailable at period_end;
    see logs*`, plus a forward-compat guard `assert_eq!` on
    `MARK_UNAVAILABLE_FOOTNOTE` itself so accidental edits to the
    constant in `crates/reports/src/render/reconciliation.rs:21` are
    caught here on the test side).
  - test cmd —
    `cargo test -p reports --test unrealized_orchestrator --test mark_unavailable_warns`.
  - test output —
    ```
    running 2 tests
    test t1006_v6_mark_miss_warns_and_zeroes ... ok
    test t1006_v6_footnote_present_when_miss ... ok

    test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

    running 1 test
    test t1006_v2_unrealized_equals_200_usdt ... ok

    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
    ```
  - validation gates — `cargo test -p reports` (full crate, all
    integration test binaries green including the new T1006 targets);
    `cargo clippy --workspace --all-targets --all-features --
    -D warnings` clean; `cargo fmt --all -- --check` clean; `bash
    scripts/verify_anchors.sh` →
    ```
    PASS  report-sample-7d                      ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c
    PASS  report-sample-90d                     2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f
    ---
    ANCHORS PASS  (11 / 11)
    ```
    confirming no anchor drift from the new test infrastructure (T1006
    only adds two new integration test targets + a `tracing-subscriber`
    dev-dep in `crates/reports/Cargo.toml`; no `crates/reports/src/`
    change).
  - **deviation note (V2 acceptance line 506-507):** the T1006 acceptance
    block lists "`equity-<window>.csv`'s `unrealized_pnl_usdt` column
    at the last row equals `200.00`" as a V2 sub-assertion. This is
    stale relative to the architect-approved scope-out documented in
    Design § R3 (lines 658-661) and reaffirmed by T1003's task body
    ("**Out of scope:** R3 equity-curve sampler stays cash-balance-
    only"). The orchestrator at `crates/reports/src/lib.rs:519-525`
    intentionally hardcodes `unrealized_pnl: Decimal::ZERO` per
    sample — per-bar MTM is the v2+ wave. Asserting `200.00` in the
    CSV would falsify a load-bearing scope decision. The V2 test
    therefore asserts the architect-approved load-bearing surface
    (R11.1 reconciliation row Ledger-side cell == +200 USDT), which
    is what `equity_check_sum = realized_period.amount() + unrealized`
    derives directly from the new `unrealized` arithmetic. The CSV
    column-cell assertion is documented here as superseded; if the
    architect wants V2 to assert the per-sample CSV instead, that is
    a Design § R3 scope reopening and routes back through architect.
  - **deviation note (V6 tracing capture API):** the V6 acceptance
    block at line 519 suggests `tracing_subscriber::fmt::TestWriter`.
    Used a custom `tracing_subscriber::Layer` + `WarnVisitor`
    instead because `TestWriter` captures the *formatted text* of
    fmt events (a string match against `format!`-output, brittle
    against any rendering tweak), whereas a `Layer` captures
    *structured fields* directly — letting V6 assert
    `symbol = ETHUSDT` against the actual recorded field, not against
    a fmt rendering. Both APIs ship in the same `tracing-subscriber`
    crate (already a workspace dep used by `agent`); the dev-dep was
    added to `crates/reports/Cargo.toml` (only Cargo.toml change in
    this task — no `crates/reports/src/` mutation). The architect-
    locked filter literal `"mark unavailable for open position"` is
    matched as a substring of the orchestrator's actual emit
    `"mark unavailable for open position; treating unrealized as
    zero"` (lib.rs:160-164), so the test is robust to suffix edits
    on the message but anchored on the architect's spec literal.

  **Stabilization (orchestrator-spawned, 2026-05-02):** the tester's
  re-run of T1006 surfaced a `tracing::Dispatch` thread-local cache
  race in the original
  `crates/reports/tests/mark_unavailable_warns.rs` binary (2 PASS / 2
  FAIL across 4 runs — see
  `spec/archive/test-2026-05-02-2113-real-mtm-unrealized-pnl-final.md (archived; see spec/archive/README.md)`
  § 3 for the root-cause writeup). Production code is correct (the
  `tracing::warn!` site at `crates/reports/src/lib.rs:160-164` fires,
  `mark_misses` increments, footnote renders) — only the test's
  capture infrastructure was unreliable when the no-subscriber
  `t1006_v6_footnote_present_when_miss` ran in parallel with the
  `with_default(...)`-scoped `t1006_v6_mark_miss_warns_and_zeroes`.

  **Option chosen — option 4 (separate test binaries).** Cargo runs
  each `tests/*.rs` integration-test binary in its own process; tests
  *within* a binary parallelise. Splitting the original file into
  two single-test binaries puts the warn-capture test in its own
  process with no parallel sibling, guaranteeing a clean
  `tracing::Dispatch` cache on the test thread. Zero new
  dev-deps; preserves test granularity (V6 warn-capture and V6
  body-footnote remain independently named/citable); deterministic
  by construction. Rejected option 1 (`serial_test`) and option 3
  (`dispatcher::set_default`) because they add either a new
  dev-dep or a global-subscriber side-effect; rejected option 2
  (combine into one test) because it loses granularity.

  - new file:line —
    `crates/reports/tests/mark_unavailable_warns_capture.rs:146`
    (`#[test] fn t1006_v6_mark_miss_warns_and_zeroes` — verbatim
    body from the original file at the same logical line; the
    `with_default(subscriber, || rt.block_on(...))` capture-scope
    body is unchanged).
    `crates/reports/tests/mark_unavailable_warns_footnote.rs:43`
    (`#[tokio::test] async fn t1006_v6_footnote_present_when_miss`
    — verbatim body from the original file).
    Original file `crates/reports/tests/mark_unavailable_warns.rs`
    deleted (no longer referenced; its content is now split across
    the two new files above with no semantic change).
  - test cmd —
    `cargo test -p reports --test mark_unavailable_warns_capture --test mark_unavailable_warns_footnote`.
  - **Stress-test (5 consecutive runs, 2026-05-02, dev box):** all
    5 runs PASS — the `tracing::Dispatch` cache race is gone under
    cargo's per-binary process isolation.
    ```
    Run 1: test t1006_v6_mark_miss_warns_and_zeroes ... ok
           test t1006_v6_footnote_present_when_miss ... ok
    Run 2: test t1006_v6_mark_miss_warns_and_zeroes ... ok
           test t1006_v6_footnote_present_when_miss ... ok
    Run 3: test t1006_v6_mark_miss_warns_and_zeroes ... ok
           test t1006_v6_footnote_present_when_miss ... ok
    Run 4: test t1006_v6_mark_miss_warns_and_zeroes ... ok
           test t1006_v6_footnote_present_when_miss ... ok
    Run 5: test t1006_v6_mark_miss_warns_and_zeroes ... ok
           test t1006_v6_footnote_present_when_miss ... ok
    ```
    Each run reported `test result: ok. 1 passed; 0 failed; 0 ignored`
    in both binaries. **5/5 PASS consecutive — stability gate met.**
  - validation gates — `cargo build` clean (7.40s);
    `cargo test --workspace --all-targets` PASS (no failures across
    all crates including the two new T1006 binaries);
    `cargo clippy --workspace --all-targets --all-features --
    -D warnings` clean; `cargo fmt --all -- --check` clean;
    `bash scripts/verify_anchors.sh` →
    ```
    PASS  report-sample-7d                      ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c
    PASS  report-sample-90d                     2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f
    ---
    ANCHORS PASS  (11 / 11)
    ```
    No production code was touched (no `crates/reports/src/`
    mutation, no `Cargo.toml` mutation, no `spec/anchors.toml`
    mutation) — this is a test-binary split only. T1006 remains
    `[x]`-ticked on its original implementation merit; this
    stabilization sub-block adds the missing reproducibility on
    the V6 warn-capture half so the tester can re-run the
    `T_FINAL_REAL_MTM` gate without the flake.

  - tester-owned: `T_FINAL_REAL_MTM` (per AGENT.md process
    discipline: developer never ticks `T_FINAL_*`).

- [x] **T1007** [developer] — V8 perf smoke in
  `crates/reports/tests/perf_smoke_open_positions.rs`:
  - New fixture variant (or in-test ledger builder) with **100
    fills**: 50 (Buy, Sell) pairs (closed) + 5 unmatched Buys
    across 5 distinct symbols/strategies = 5 expected
    `OpenPosition` rows.
  - `let t = std::time::Instant::now();
    audit::query::open_positions_at(&ledger,
    period_end).await?;` then
    `assert!(t.elapsed() < std::time::Duration::from_millis(100),
    "open_positions_at exceeded 100ms perf budget: {:?}",
    t.elapsed());`
  - Run 3× warmup iterations + 1 measured iteration to amortize
    SQLite page-cache cold-start.
  - **If V8 fails on the developer's box**, route `HANDOFF →
    architect` to escalate the conditional follow-up migration
    `006_open_positions_index.sql` (Q3 escape hatch).
  _acceptance: `cargo test -p reports --test
  perf_smoke_open_positions --release` → pass under 100ms;
  follows the existing `crates/reports/tests/perf_smoke.rs`
  precedent (T815)._
  **[deps: T1002, T1004]**

  **Honest-tick citations (2026-05-01, developer):**
  - file:line —
    `crates/reports/tests/perf_smoke_open_positions.rs:189`
    (`#[tokio::test(flavor = "multi_thread")] async fn
    t1007_perf_smoke_open_positions_under_100ms`); fixture builder
    inline at `crates/reports/tests/perf_smoke_open_positions.rs:128`
    (`async fn build_perf_fixture`) — emits 50 fully-closed (Buy,
    Sell) pairs (each pair gets its own `pair_strat_<i>`
    strategy_id so the (symbol, strategy_id) group nets to zero
    by construction; long-only invariant preserved per Q8) plus
    5 dangling Buys across 5 distinct symbols (BTCUSDT, ETHUSDT,
    SOLUSDT, BNBUSDT, XRPUSDT) pinned per-symbol to 5 distinct
    strategy_ids — 105 total fills, 5 expected `OpenPosition`
    rows. Sanity-probes the `OpenPosition` count before timing
    so a fixture-shape regression surfaces as a clean assertion
    rather than a silently-wrong perf number; runs 3 warmup
    iterations (the sanity probe + 2 explicit) before the
    measured iteration to amortize the SQLite page-cache
    cold-start; uses `Ledger::in_memory()` so the budget reflects
    audit-query CPU/parser work, not disk latency.
  - test cmd —
    `cargo test -p reports --test perf_smoke_open_positions
    --release -- --nocapture`.
  - test output —
    ```
    running 1 test
    T1007 V8 wall-clock: 0.287ms (budget < 100ms) — PASS
    test t1007_perf_smoke_open_positions_under_100ms ... ok

    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
    ```
    Measured wall-clock **0.287ms** vs the 100ms architect-mandated
    V8 budget — three orders of magnitude under the gate (the
    full-table description-prefix scan + Rust-side weighted-avg
    fold finishes well within budget at v1+ scale, so Q3's
    conditional follow-up migration `006_open_positions_index.sql`
    does NOT need to land). An earlier dev-build run logged
    `0.218ms` on the same machine; both runs comfortably clear
    the budget.
  - validation gates — `cargo clippy --workspace --all-targets
    --all-features -- -D warnings` clean; `cargo fmt --all --
    --check` clean (FMT_OK); `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)` (T1007 only adds a NEW test target
    + ledger-builder fixture inline; no source mutation under
    `crates/reports/src/` or `crates/audit/src/`, so anchors
    stay byte-identical by construction).
  - **deviation note:** the architect's task body suggests "a
    new fixture variant (or in-test ledger builder)". I chose
    the in-test ledger builder (`build_perf_fixture` inline in
    `perf_smoke_open_positions.rs:128`) over a new
    `crates/reports/tests/fixtures/build_ledger_perf.rs` file
    because (a) the perf fixture is consumed by exactly one
    test target — sharing it via `#[path]` would create
    ceremony with no readers — and (b) it side-steps the
    `build_ledger_with_open_positions_7d.rs` 14-fill plan,
    which doesn't scale to 100+ fills without a re-design.
    Functionally identical: the builder calls
    `journal::post_fill` end-to-end so the
    description-parser path inside `open_positions_at` is
    exercised for real (matching how the 8 unit tests at
    `crates/audit/tests/open_positions.rs` are structured).
    `tracing-subscriber` and other dev-deps in
    `crates/reports/Cargo.toml` were untouched (the new test
    target uses only existing `[dev-dependencies]`:
    `audit`, `trading_core`, `tokio`, `rust_decimal`,
    `rust_decimal_macros`, `time`).

- [x] **T1008** [developer] — Anchor regression sweep (V3 + V5):
  - Run `cargo test -p reports --test report_scenarios --release`
    (the existing T816 anchor harness) and assert
    `report-sample-7d` + `report-sample-90d` body bytes are
    byte-identical to the locked SHAs `ab06dbcb…` /
    `2ef403f1…`.
  - Run `bash scripts/verify_anchors.sh` and assert
    `ANCHORS PASS  (11 / 11)` — all 9 v0/v0.5/v1/v1.5a +
    2 v1+ anchors green.
  - If any anchor drifts, route `HANDOFF → architect` with a
    body diff. Architect's first-line hypothesis: a render-module
    serialization slip on `+0.00` vs `0.00` (Q4 falsification
    note).
  - **No source touched** under T1008 — this is a verification
    sweep on the post-T1003 code. The developer ticks T1008 only
    after running both gates locally and seeing 11/11 PASS.
  _acceptance: `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)`; `cargo test -p reports --test
  report_scenarios --release` → all anchored scenarios pass;
  honest-tick on the developer side links to the green output._
  **[deps: T1003]**

  **Honest-tick citations (2026-05-01, developer):**
  - file:line — verification-only task; no source modified. Gates
    run against post-T1003 trunk: `crates/reports/src/lib.rs:148`
    (open-positions loop), `crates/reports/src/render/reconciliation.rs:21`
    (`MARK_UNAVAILABLE_FOOTNOTE` const), and
    `crates/reports/src/render/reconciliation.rs:33`
    (`render(report, mark_unavailable: bool)`) — all unchanged
    by T1008.
  - test cmd #1 — `cargo test -p reports --test report_scenarios --release`.
  - test output #1 —
    ```
    running 4 tests
    test t816_v10_cron_friendly_3x_parallel_renders_atomic ... ok
    test t816_report_sample_7d_determinism_and_anchor_lock ... ok
    test t816_report_sample_90d_determinism_and_anchor_lock ... ok
    test t816_v10_cron_friendly_3x_parallel_bin_processes ... ok

    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
    ```
    Confirms both v1+ anchored scenarios (`report-sample-7d`,
    `report-sample-90d`) re-rendered to byte-identical bodies
    matching the locked SHAs `ab06dbcb…` / `2ef403f1…`. The
    in-test `t816_*_determinism_and_anchor_lock` assertions
    compare the rendered SHA to the locked anchor SHA inline
    (any drift = test failure), so a green run IS the byte-
    identical proof. Fresh report files written at
    `spec/operator-success-reports/reports/success-fixed-report-sample-7d.md`
    and `success-fixed-report-sample-90d.md` (timestamped
    2026-05-02 22:49 — current run).
  - test cmd #2 — `bash scripts/verify_anchors.sh`.
  - test output #2 —
    ```
    PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
    PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
    PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
    PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
    PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
    PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
    PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
    PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
    PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
    PASS  report-sample-7d                      ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c
    PASS  report-sample-90d                     2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f
    ---
    ANCHORS PASS  (11 / 11)
    ```
    All 11 anchors green: 9 v0/v0.5/v1/v1.5a backtest anchors
    (verified against the locked backtest-*-<scenario>.md reports
    under `spec/<slug>/reports/`) PLUS 2 v1+ operator-success-report
    anchors (verified against the freshly re-rendered
    `success-fixed-report-sample-{7d,90d}.md`). Q4's load-bearing
    claim — "build_ledger_7d / build_ledger_90d have zero open
    positions at period_end → unrealized = 0 → bodies stay
    byte-identical" — confirmed empirically. The 9 backtest
    anchors are unaffected by this feature (touches no
    strategy/exec/backtest code path) and stay locked at their
    historical SHAs.
  - **deviation note:** none. T1008 is verification-only and ran
    exactly as specified — no source edits, no spec/anchors.toml
    edits, no report edits. The 9 backtest anchors verify against
    their existing locked reports under `spec/<feature>/reports/backtest-*`
    (the `verify_anchors.sh` script's first-glob branch resolves
    them automatically); no need to re-run those scenarios since
    the script's per-anchor SHA comparison IS the regression check.
  - tester-owned: `T_FINAL_REAL_MTM` (per AGENT.md process
    discipline: developer never ticks `T_FINAL_*`).

- [x] **T_FINAL_REAL_MTM** [tester] — End-to-end gate. Tester-only.

  **Tester verdict (orchestrator-finalized, 2026-05-02 23:18):** PASS.
  - First tester run (`spec/archive/test-2026-05-02-2113-real-mtm-unrealized-pnl-final.md (archived; see spec/archive/README.md)`): VERDICT FAIL on V6 warn-capture flake; V1/V2/V3/V4/V5/V7/V8 VERIFIED.
  - Stabilization dev (orchestrator-spawned): split `mark_unavailable_warns.rs` into 2 separate test binaries (`mark_unavailable_warns_capture.rs` + `mark_unavailable_warns_footnote.rs`); 5/5 consecutive PASS.
  - Re-run tester: confirmed flake fixed (5 stress invocations PASS), `cargo test --workspace --all-targets` 0 failures, `bash scripts/verify_anchors.sh` `ANCHORS PASS (11 / 11)`.
  - Tester re-run was sandbox-blocked on `cargo fmt --check` and `cargo clippy --all-features`; orchestrator ran both from project root: BOTH CLEAN. The verification matrix is satisfied across all 8 V-items + 11/11 anchors + 5 operator-success-reports invariants.
  - V1: `t1005_v1_reader_emits_two_open_positions` PASS.
  - V2: `t1006_v2_unrealized_equals_200_usdt` PASS (R11.1 Ledger-side cell = +200 USDT).
  - V3: `report_scenarios` test PASS — both v1+ anchors byte-identical.
  - V4: `t1005_v4_balance_invariant_per_txn` PASS.
  - V5: 11/11 anchor SHAs match `spec/anchors.toml`.
  - V6: `t1006_v6_mark_miss_warns_and_zeroes` (in `mark_unavailable_warns_capture.rs:146`) + `t1006_v6_footnote_present_when_miss` (in `mark_unavailable_warns_footnote.rs:43`) BOTH PASS, 5/5 consecutive (post-stabilization).
  - V7: `t1005_v7_two_reads_byte_identical` PASS.
  - V8: `t1007_v8_perf_smoke` PASS at 0.287ms vs 100ms budget.
  - All 5 invariants (T802, T805, T809, T810, anchors) VERIFIED.
  Fans out into the standard `rust-validate` + `rust-test` +
  `verify-anchors` parallel skill calls and merges into one
  report at `spec/reports/test-<timestamp>-real-mtm-final.md`.
  The report's verification matrix MUST cover all 8 V-items
  + 11/11 anchor gate + the 5 operator-success-reports
  invariants from
  [Design → Operator-success-reports invariants](../features/real-mtm-unrealized-pnl.md#operator-success-reports-invariants-that-must-hold):

  | Gate | Test |
  |------|------|
  | V1 reader correctness | `t1005_v1_reader_emits_two_open_positions` |
  | V2 orchestrator unrealized | `t1006_v2_unrealized_equals_200_usdt` |
  | V3 empty-positions backwards compat | `cargo test -p reports --test report_scenarios --release` (T1008) |
  | V4 reconciliation invariant | `t1005_v4_balance_invariant_per_txn` |
  | V5 anchor regression | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` |
  | V6 existing event invariants + mark-miss | `cargo test -p audit --tests` + `t1006_v6_mark_miss_warns_and_zeroes` |
  | V7 determinism | `t1005_v7_two_reads_byte_identical` |
  | V8 perf | `cargo test -p reports --test perf_smoke_open_positions --release` |
  | Inv-T802 | `post_fill` signature scan via `grep "pub async fn post_fill"` (single match, unchanged) |
  | Inv-T805 | existing `feed_reconnect_smoke` test green |
  | Inv-T809 | existing `kill_switch_dual_write` test green |
  | Inv-T810 | `cargo build -p agent --features in_process_cron` clean |
  | Inv-anchors | gate (V5 above) |

  - On any FAIL, route `HANDOFF → developer` (or `→ architect` if
    a 9 v0/v0.5/v1/v1.5a anchor drifts — that points at a
    rendering side-effect the architect must reconcile).
  - On full PASS, bump the feature file's `status:` to `shipped`
    and tick this row.
  _acceptance: tester's report template populated with all 8
  V-items + 11/11 anchor gate + 5 operator-success-reports
  invariants; status flips `in-progress → shipped`._
  **[deps: T1005, T1006, T1007, T1008]**

## Parallelism map

```
                    ┌──────┐
                    │T1001 │  trading_core::OpenPosition
                    │ struct│  (CRITICAL PATH GATE)
                    └───┬──┘
                        │
            ┌───────────┴────────────┐
            ▼                        ▼
        ┌──────┐                 ┌──────┐
        │T1002 │                 │T1004 │  fixture w/ open positions
        │audit │                 │      │  (PARALLEL-SAFE w/ T1002)
        │reader│                 └──┬───┘
        └───┬──┘                    │
            │                       │
            ▼                       │
        ┌──────┐                    │
        │T1003 │                    │
        │orch  │                    │
        │diff  │                    │
        └───┬──┘                    │
            │                       │
            ├──────────────┬────────┤
            ▼              ▼        ▼
        ┌──────┐       ┌──────┐ ┌──────┐
        │T1005 │       │T1006 │ │T1007 │  perf smoke
        │V1+V4 │       │V2+V6 │ │ V8   │  (PARALLEL-SAFE)
        │+V7   │       └───┬──┘ └───┬──┘
        └───┬──┘           │        │
            │              │        │
            └──────┬───────┴────────┘
                   │
                   │   (T1008 separate; runs after T1003,
                   │   independent of T1005/6/7 from
                   │   a code-touch standpoint)
                   │
                ┌──▼───┐
                │T1008 │  anchor regression sweep
                │V3+V5 │
                └───┬──┘
                    │
              ┌─────▼──────────┐
              │ T_FINAL_REAL_MTM │  [tester]
              │ V1–V8 + anchors │
              │ + 5 invariants  │
              └────────────────┘
```

**Sync points** (tasks below the line block on tasks above):
1. **After T1001** (line 1): T1002 + T1004 fan out **in parallel**.
   Different crates (`audit` vs `reports/tests/fixtures`); no
   shared file.
2. **After T1002 + T1004** (line 2): T1003 lands (uses both the
   reader and the fixture indirectly through render tests in
   T1006). T1005 can also start in parallel with T1003 because
   T1005 only consumes T1002 + T1004, not T1003.
3. **After T1003 + T1004** (line 3): T1006 + T1007 fan out **in
   parallel**. Different test files; T1006 reads the orchestrator
   path (T1003), T1007 reads the audit reader (T1002).
4. **After T1003**: T1008 (the anchor sweep) lands. Independent
   of T1005/T1006/T1007 from a code-touch perspective; sequenced
   after T1003 because the orchestrator change is what could
   break anchors. Can run in parallel with T1005/T1006/T1007.
5. **T_FINAL_REAL_MTM** is sequential — single tester agent
   merges the verification matrix.

**Parallel-safe boundary check:**

| Pair | Files touched (left) | Files touched (right) | Conflict? |
|------|----------------------|------------------------|-----------|
| T1002 ‖ T1004 | `crates/audit/src/query.rs` | `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs` (NEW) | NO |
| T1005 ‖ T1006 | `crates/audit/tests/open_positions_at.rs` (NEW) | `crates/reports/tests/unrealized_orchestrator.rs` + `crates/reports/tests/mark_unavailable_warns.rs` (NEW) | NO |
| T1005 ‖ T1007 | `crates/audit/tests/open_positions_at.rs` | `crates/reports/tests/perf_smoke_open_positions.rs` (NEW) | NO |
| T1006 ‖ T1007 | both new test files in `crates/reports/tests/` | distinct files | NO |
| T1006 ‖ T1008 | `crates/reports/tests/unrealized_orchestrator.rs` | runs `bash scripts/verify_anchors.sh` + `cargo test -p reports --test report_scenarios` (existing files; NO source mutation) | NO |

Wave 1 (after T1001): T1002 + T1004 — **2-way fan-out**.
Wave 2 (after T1003): T1005 + T1006 + T1007 + T1008 —
**4-way fan-out**.
Wave 3: T_FINAL_REAL_MTM — sequential tester.

## Notes

- Every task that writes spec files uses the `spec-update` skill.
- **T1001** is the critical-path gate — adding `OpenPosition` to
  `trading_core` triggers a workspace rebuild on every dep crate.
  Do it first; everything downstream waits.
- **T1002** is the load-bearing audit-side change. Description-
  parser semantics (Q3 / R10 deferral) match `pnl_by_symbol`
  exactly; reuse `extract_symbol_from_description` rather than
  duplicate the parse.
- **T1003** is the only orchestrator change. Keep the diff
  surgical to the lib.rs:135–150 block + the
  `render::reconciliation::render(...)` `mark_unavailable` field
  thread-through. Do NOT modify `sample_equity_curve` or any R3
  / R4 rendering — those stay cash-balance-only at v1+ scope.
- **T1004** is the test-only fixture. NEVER modify
  `build_ledger_7d.rs` or `build_ledger_90d.rs` (Q4: anchors
  stay byte-identical). PR reviewer rejects any commit touching
  those files in this feature.
- **T1008** is the anchor regression sweep. If it fails on the
  9 v0/v0.5/v1/v1.5a anchors, the orchestrator's diff has
  serialization side-effects beyond what Q4 anticipated; route
  `HANDOFF → architect`. If it fails on the 2 v1+ anchors only,
  re-read `build_ledger_7d.rs` + `build_ledger_90d.rs` for any
  drift; then route `HANDOFF → architect` for re-lock approval.
- The 9 v0/v0.5/v1/v1.5a anchor hashes are **non-negotiable**
  (this feature touches no strategy/exec/backtest code path).
- The 2 v1+ anchor hashes are **byte-identical** by Q4 resolution.
- No new runtime crate dependency. Workspace edition 2021
  unchanged.
- **R10 deferral** (`post_fill` BTC hardcode at
  `crates/audit/src/journal.rs:82,135`) is filed as a follow-up
  brief `spec/per-symbol-position-accounts/feature.md`
  (analyst-owned; not authored under this task list).
- The `reports` orchestrator stays read-only over the audit DB.
  No new writes; the new reader only reads `journal_transactions`.

## Changelog

- 2026-05-02 (architect): initial task breakdown — 8 tasks
  (T1001–T1008) + `T_FINAL_REAL_MTM`. Covers `trading_core::OpenPosition`
  type addition, `audit::query::open_positions_at` reader,
  `reports::generate(...)` orchestrator diff (lines 135–150),
  test-only fixture `build_ledger_with_open_positions_7d.rs`,
  V1–V8 verification across `crates/audit/tests/` +
  `crates/reports/tests/`, anchor regression sweep (11/11
  byte-identical per Q4), tester end-to-end gate. **No
  `[ui-designer]` tasks** — feature reads existing bus PNL
  channel; no UI surface changes. Parallelism: 2-way fan-out
  Wave 1 (T1002 ‖ T1004); 4-way fan-out Wave 2 (T1005 ‖ T1006
  ‖ T1007 ‖ T1008). HANDOFF → developer (Wave 1: T1001
  sequential, then T1002 ‖ T1004).
- 2026-05-02 (developer): T1001 ticked. Added `OpenPosition` struct at
  `crates/core/src/position.rs:88` (alongside the existing `Position`)
  with the architect-mandated fields `{symbol, qty, avg_cost_basis:
  Money<Usdt> per-unit, opened_at, strategy_id: Option<StrategyId>}`
  and derives `Debug, Clone, PartialEq, Eq`. Re-exported via
  `crates/core/src/lib.rs:39` (`pub use position::{OpenPosition,
  Position};`). Three round-trip / distinguishability tests added
  (`t1001_open_position_partialeq_round_trip` +
  `_distinguishes_strategy_id` + `_distinguishes_qty_and_cost_basis`)
  — 3/3 PASS. Validation: `cargo build -p trading_core` clean,
  `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` clean, `cargo fmt --all -- --check` clean, `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` (T1001
  doesn't touch any backtest/render path; verified anyway per
  process discipline). Wave 1 fan-out (T1002 ‖ T1004) unblocked.
  HANDOFF → orchestrator.
- 2026-05-01 (developer, parallel-Wave-1): T1002 ticked. Added
  `pub async fn open_positions_at(ledger: &Ledger, ts: Timestamp) ->
  Result<Vec<OpenPosition>, LedgerError>` at
  `crates/audit/src/query.rs:1008`, parsing the symbol from the
  transaction description via the existing
  `extract_symbol_from_description` helper (R10 deferral honoured —
  `journal::post_fill` still hardcodes `assets:position:BTC` for every
  symbol per `journal.rs:82,135`, but the description IS symbol-faithful).
  Algorithm follows Design § Q7 (weighted-average cost basis with
  proportional release on Sells; `BTreeMap<(Symbol, Option<String>),
  Acc>` accumulator for R6 determinism), Q8 (long-only — net-negative
  qty raises `LedgerError::Database`), and Q3 (no new SQL index — the
  full-table description-prefix scan matches the `recent_fills`
  pattern). On a re-opening lot (`running_qty == 0` → next Buy),
  `opened_at` and `strategy_id` refresh to that Buy's row. Output
  sorted `(symbol ASC, strategy_id ASC, None last)`. Eight integration
  tests added at `crates/audit/tests/open_positions.rs`
  (`t1002_empty_ledger_returns_empty_vec`,
  `t1002_single_open_position`, `t1002_closed_position_excluded`,
  `t1002_weighted_avg_cost_basis`, `t1002_partial_close`,
  `t1002_multi_symbol_sorted`, `t1002_strategy_id_preserved`,
  `t1002_net_negative_returns_err`) — 8/8 PASS. Validation: `cargo
  test -p audit` (full suite) clean; `cargo clippy -p audit
  -p trading_core --all-targets --all-features -- -D warnings`
  clean; `cargo fmt -p audit -p trading_core -- --check` clean;
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`.
  Workspace-wide `cargo clippy` blocked by parallel T1004's WIP
  fixture-smoke test (`crates/reports/tests/fixture_with_open_positions_smoke.rs`),
  outside this developer's crate boundary. T1003 unblocked.
  HANDOFF → orchestrator.
- 2026-05-01 (developer, parallel-Wave-1): T1004 ticked. Added
  test-only non-anchored fixture
  `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs:88`
  (`pub async fn build_ledger_with_open_positions_7d`) mirroring the
  `build_ledger_7d` shape: same `FIXTURE_SEED = 0x00C0_FFEE`,
  `PERIOD_START_RFC3339 = "2026-04-21T00:00:00Z"`,
  `PERIOD_END_RFC3339 = "2026-04-28T00:00:00Z"`, deterministic UUIDs
  via `deterministic_uuid(u64)`, RFC-3339 timestamps + 6-digit-micros
  helpers for strategy events. Activity plan: 12 perfectly-symmetric
  (Buy, Sell) pairs (architect-mandated; identical to `build_ledger_7d`)
  + 2 dangling Buys at `(day=6, hour=20)` per Design § Q5 —
  `(strat_alpha, BTCUSDT, qty=0.01, price=60_000)` and
  `(strat_beta, ETHUSDT, qty=0.20, price=3_000)`. The first dangling
  Buy lands in the same `(strat_alpha, BTCUSDT)` group that already
  saw 3 closed pairs, exercising the Q7 weighted-average / proportional-
  release path (`running_qty` walks `0 → +qty → 0 → +qty → 0 → +qty
  → 0 → +0.01`; final `avg_cost_basis = 60_000`). Sibling helper
  `frozen_marks_csv()` at line 239 returns the V2 CSV body
  (BTCUSDT @ 60_000/70_000 + ETHUSDT @ 3_000/3_500 at
  period_start/period_end) for downstream T1006 use; constants
  `BTC_MARK_AT_PERIOD_END = 70_000` and `ETH_MARK_AT_PERIOD_END = 3_500`
  pin the V2 hand-computed expected `Σ unrealized = +200.00 USDT`.
  Smoke target `crates/reports/tests/fixture_with_open_positions_smoke.rs`
  (3 tests, 3/3 PASS): `t1004_fixture_emits_two_open_positions`
  (the spec-named acceptance — 14 fills = 12 closed + 2 open;
  per-symbol counts BTCUSDT=7, ETHUSDT=7),
  `t1004_fixture_two_builds_byte_identical_fills` (determinism —
  same seed → byte-identical `recent_fills` projection at every
  index), and `t1004_fixture_has_expected_open_positions_at_period_end`
  (calls T1002's `audit::query::open_positions_at(&ledger, period_end)`
  and asserts 2 open positions in alphabetical sort: BTCUSDT before
  ETHUSDT — full V1 byte-equality assertions still owned by T1005).
  The spec mentions inline `#[cfg(test)] mod tests` in the fixture
  file; I moved the smoke tests to the sibling integration target
  because cargo only auto-generates test binaries from top-level
  `tests/*.rs` files, not from files re-mounted via `#[path]`. The
  fixture itself is `pub`-API-only and consumes cleanly via
  `#[path = "fixtures/build_ledger_with_open_positions_7d.rs"] mod ...`
  from any future T1005 / T1006 / T1007 test target.
  Existing fixtures `build_ledger_7d.rs` and `build_ledger_90d.rs`
  are NOT modified (Q4: anchors stay byte-identical; reviewer guard).
  Validation: `cargo build -p reports --tests` clean; `cargo test -p
  reports` (full suite) clean; `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` clean (unblocks T1002 dev's deferred
  workspace-wide gate); `cargo fmt --all -- --check` clean; `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`. T1005, T1006,
  T1007 unblocked. HANDOFF → orchestrator.
- 2026-05-01 (developer, Wave-2 entry): T1003 ticked. Replaced the
  `let unrealized: Decimal = Decimal::ZERO;` placeholder in
  `crates/reports/src/lib.rs:148` (the orchestrator's R11.1 unrealized
  feed) with the architect-spec'd open-positions loop: read the typed
  slice via `audit::query::open_positions_at(&ledger, period_end)` (T1002),
  fold `unrealized += pos.qty * (mark - pos.avg_cost_basis.amount())` per
  resolved mark, and route `MarkError::OutOfRange` through
  `tracing::warn!` + `mark_misses += 1` per Q6 (warn + zero + body
  footnote). The `mark_unavailable_footnote` boolean is threaded into
  `render::reconciliation::render(&recon, mark_unavailable_footnote)`
  via an additive parameter (`crates/reports/src/render/reconciliation.rs:33`);
  on `false` (every empty-positions / fully-resolved code path) the
  body emits ZERO new bytes vs the pre-T1003 path, satisfying Q4's
  "anchors stay byte-identical" load-bearing claim. The deterministic
  footnote string is `pub const MARK_UNAVAILABLE_FOOTNOTE` at
  `crates/reports/src/render/reconciliation.rs:21` (matches the
  architect's exact wording so T1006 V6 will resolve unchanged).
  Three inline smoke tests at `crates/reports/tests/t1003_orchestrator_smoke.rs`
  cover the three branches (zero open positions → SHA matches the
  locked `report-sample-7d` anchor `ab06dbcb…`; open positions +
  resolving marks → `unrealized = +200 USDT` end-to-end at the R11.1
  cell; mark miss → unrealized = 0 + footnote present in body) —
  3/3 PASS. Collateral fix: the perf-only `build_ledger_1y.rs`
  fixture's side-selection scheme produced two Sell-only
  `(strategy, symbol)` groups out of four — a malformed long-only
  ledger that `open_positions_at` (T1002) raises on per Q8.
  Patched in place at `build_ledger_1y.rs:96-132` to alternate
  side **within** each `(symbol, strategy)` group via a per-group
  lot-index counter, mirroring each Sell's qty against the
  preceding Buy's qty so every group walks `0 → +qty → 0` (long-only
  at all times). Fixture is used **only** by `t815_perf_smoke` (no
  anchor implications, verified by grep across `crates/` + `spec/`);
  perf budget holds (`T815 wall-clock: 4.39s, budget < 10s — PASS`).
  Validation: `cargo build -p reports` clean; `cargo test -p reports`
  (full crate, 21 integration test binaries + 98 unit tests, all
  green); `cargo test --workspace` (zero failures); `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` clean;
  `cargo fmt --all -- --check` clean; `bash scripts/verify_anchors.sh`
  → `ANCHORS PASS (11 / 11)` confirming both `report-sample-7d` and
  `report-sample-90d` v1+ bodies stay byte-identical to the locked
  SHAs (the load-bearing Q4 invariant of this feature). Wave 2
  fan-out (T1005 ‖ T1006 ‖ T1007 ‖ T1008) unblocked. HANDOFF →
  orchestrator (T1003 done; Wave 2 fan-out unblocked).
- 2026-05-01 (developer, Wave-2): T1008 ticked. Anchor regression
  sweep (V3 + V5) — verification-only, no source touched.
  Re-ran the existing T816 anchor harness `cargo test -p reports
  --test report_scenarios --release` — 4/4 PASS including
  `t816_report_sample_7d_determinism_and_anchor_lock` and
  `t816_report_sample_90d_determinism_and_anchor_lock` (the
  in-test SHA comparisons against the locked anchors are the
  byte-identical proof; fresh reports re-rendered to
  `spec/operator-success-reports/reports/success-fixed-report-sample-{7d,90d}.md`).
  Then ran `bash scripts/verify_anchors.sh` → `ANCHORS PASS
  (11 / 11)` — all 9 v0/v0.5/v1/v1.5a backtest anchors verified
  against their locked `spec/<feature>/reports/backtest-*-<scenario>.md`
  reports (untouched by this feature; this feature touches no
  strategy/exec/backtest code path) PLUS 2 v1+ anchors verified
  against the freshly re-rendered success reports. Q4's
  load-bearing claim — `build_ledger_7d` and `build_ledger_90d`
  have zero open positions at `period_end` → `unrealized = 0` →
  bodies stay byte-identical to the pre-T1003 path — confirmed
  empirically. No spec/anchors.toml edits, no report edits, no
  Rust source edits. T_FINAL_REAL_MTM (tester-only) unblocked.
  HANDOFF → orchestrator (T1008 done; Wave 2 complete; tester
  gate next).
- 2026-05-01 (developer, Wave-2): T1007 ticked. V8 perf smoke
  added at `crates/reports/tests/perf_smoke_open_positions.rs:189`
  (`#[tokio::test(flavor = "multi_thread")] async fn
  t1007_perf_smoke_open_positions_under_100ms`). Inline ledger
  builder at `perf_smoke_open_positions.rs:128`
  (`async fn build_perf_fixture`) emits 50 fully-closed (Buy,
  Sell) pairs (each pair gets its own `pair_strat_<i>`
  strategy_id so the (symbol, strategy_id) group nets to zero
  by construction; long-only invariant preserved per Q8) plus
  5 dangling Buys across 5 distinct symbols (BTCUSDT, ETHUSDT,
  SOLUSDT, BNBUSDT, XRPUSDT) pinned per-symbol to 5 distinct
  strategy_ids — 105 total fills, 5 expected `OpenPosition`
  rows. Sanity-probes the open-position count before timing,
  runs 3 warmup iterations (sanity probe + 2 explicit) before
  the measured iteration to amortize the SQLite page-cache
  cold-start. Uses `Ledger::in_memory()` so the budget reflects
  audit-query CPU/parser work rather than disk latency. Test
  output (release mode):
  `T1007 V8 wall-clock: 0.287ms (budget < 100ms) — PASS` —
  three orders of magnitude under the architect-mandated V8
  budget; Q3's conditional follow-up migration
  `006_open_positions_index.sql` does NOT need to land. An
  earlier dev-build run logged `0.218ms` on the same machine;
  both runs comfortably clear the budget. Validation: `cargo
  test -p reports --test perf_smoke_open_positions --release
  -- --nocapture` → 1/1 PASS; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` clean (one
  fix-up: `clippy::doc_lazy_continuation` on the
  `build_perf_fixture` doc-comment); `cargo fmt --all --
  --check` clean; `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)` (T1007 only adds a NEW test target
  + inline ledger-builder; no source mutation under
  `crates/reports/src/` or `crates/audit/src/`, so anchors
  stay byte-identical by construction). Deviation note:
  chose the in-test ledger builder over a new
  `crates/reports/tests/fixtures/build_ledger_perf.rs` file
  because the perf fixture is consumed by exactly one test
  target (sharing it via `#[path]` would create ceremony with
  no readers) and side-steps the `build_ledger_with_open_positions_7d.rs`
  14-fill plan, which doesn't scale to 100+ fills without a
  re-design. Functionally identical: the builder calls
  `journal::post_fill` end-to-end so the description-parser
  path inside `open_positions_at` is exercised for real. No
  `Cargo.toml` edits (uses only existing dev-deps: `audit`,
  `trading_core`, `tokio`, `rust_decimal`, `rust_decimal_macros`,
  `time`). T_FINAL_REAL_MTM (tester-only) is the remaining
  gate. HANDOFF → orchestrator (T1007 done; only
  T_FINAL_REAL_MTM remains, owned by tester per AGENT.md
  process discipline).
- 2026-05-01 (developer, Wave-2): T1005 ticked. Added new
  integration test target `crates/audit/tests/open_positions_at.rs`
  with 4 tests covering V1 / V4 / V7 + the Q8 net-negative branch.
  V1 (`t1005_v1_reader_emits_two_open_positions`, line 83) opens
  the T1004 fixture via `#[path = "../../reports/tests/fixtures/build_ledger_with_open_positions_7d.rs"]`,
  calls `audit::query::open_positions_at(&ledger, period_end)`,
  and asserts the returned `Vec<OpenPosition>` matches the
  architect's hand-computed expected vec byte-for-byte (BTCUSDT
  qty=0.01 / cost=60_000 / opened_at=2026-04-27T20:00:00Z /
  strat_alpha, then ETHUSDT qty=0.20 / cost=3_000 /
  opened_at=2026-04-27T20:00:00Z / strat_beta — alphabetical R6
  sort). V4 (`t1005_v4_balance_invariant_per_txn`, line 175)
  enumerates every `id` from `journal_transactions` and asserts
  `audit::journal::verify_balance(...)` returns `Ok` on each (14
  post_fill rows + bootstrap memo). V7 (`t1005_v7_two_reads_byte_identical`,
  line 212) calls `open_positions_at(&ledger, period_end)` twice
  and `assert_eq!`s the two `Vec<OpenPosition>` slices, plus a
  belt-and-braces check that `fixture_period_end()` matches the
  builder's returned `period_end`. Q8 branch
  (`t1005_q8_short_position_raises`, line 241) builds a tiny
  in-tempfile fixture with one Sell of qty=1 against zero Buys
  and asserts the returned `LedgerError::Database` message
  contains both `"net-negative qty"` and `"open_positions_at"`.
  Two dev-deps added to `crates/audit/Cargo.toml`
  (`rand.workspace = true` + `rand_chacha.workspace = true`)
  solely so the T1004 fixture compiles when re-mounted from the
  audit tests directory; no `crates/audit/src/` or `crates/reports/`
  change (parallel devs T1006/T1007/T1008 untouched).
  Validation: `cargo test -p audit --test open_positions_at` →
  4/4 PASS; `cargo test -p audit` (full suite) clean; `cargo
  clippy --workspace --all-targets --all-features -- -D warnings`
  clean; `cargo fmt --check` clean; `bash scripts/verify_anchors.sh`
  → `ANCHORS PASS (11 / 11)` (T1005 only adds a NEW test target
  + 2 dev-deps; no source/render path touched, both v1+ anchor
  bodies stay byte-identical to the locked SHAs by construction).
  HANDOFF → orchestrator (T1005 done; T_FINAL_REAL_MTM
  remains, owned by tester per AGENT.md process discipline).
- 2026-05-01 (developer, Wave-2): T1006 ticked. Added two new
  integration test targets covering V2 (positive — open positions
  + resolving marks → unrealized = +200 USDT) and V6 (negative —
  mark miss → warn + footnote + Ok return).
  `crates/reports/tests/unrealized_orchestrator.rs:92`
  (`t1006_v2_unrealized_equals_200_usdt`) drives `reports::generate(...)`
  against the T1004 fixture under a `FrozenMarkSource` covering
  BTCUSDT @ 60_000/70_000 + ETHUSDT @ 3_000/3_500 (period_start /
  period_end), parses the R11 reconciliation appendix's headline-row
  Ledger-side cell (= `realized + unrealized`, 0 + 200 = 200), and
  asserts `+200` USDT — BTC `0.01 × (70_000 − 60_000) = +100` plus
  ETH `0.20 × (3_500 − 3_000) = +100` — plus
  `MARK_UNAVAILABLE_FOOTNOTE` absent and front-matter
  `reconciliation: PASS`.
  `crates/reports/tests/mark_unavailable_warns.rs:154`
  (`t1006_v6_mark_miss_warns_and_zeroes`) installs a custom
  `tracing_subscriber::Layer` (`CaptureLayer` + `WarnVisitor`) via
  `tracing::subscriber::with_default(...)`, builds a
  `tokio::runtime::current_thread` runtime INSIDE the dispatch
  scope, drives `generate(...)` against the T1004 fixture under a
  BTC-only `FrozenMarkSource`, and asserts: (a) `generate(...)`
  returned `Ok(_)` per Q6 (no `Err(ReportError::Marks)`
  propagation), (b) exactly ONE WARN-level event matching
  `"mark unavailable for open position"` was captured, (c) the
  captured event carries `symbol = ETHUSDT` (BTCUSDT resolves
  cleanly so it does not warn), (d) the body carries
  `MARK_UNAVAILABLE_FOOTNOTE`.
  `crates/reports/tests/mark_unavailable_warns.rs:271`
  (`t1006_v6_footnote_present_when_miss`) is the body-literal
  guard: drives `generate(...)` under standard tokio test runtime
  (no tracing capture) and asserts the rendered body contains
  `MARK_UNAVAILABLE_FOOTNOTE` verbatim, plus a forward-compat
  `assert_eq!` against the architect-locked Q6 string `*one or
  more open-position marks were unavailable at period_end; see
  logs*` so accidental edits to the constant in
  `crates/reports/src/render/reconciliation.rs:21` are caught
  test-side. Both targets share the T1004 fixture via
  `#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]`.
  One dev-dep added to `crates/reports/Cargo.toml`
  (`tracing-subscriber.workspace = true`) for the `Layer` API
  used by V6's capture wiring; already a workspace dep used by
  the agent crate so no new transitive deps. **No
  `crates/reports/src/` mutation** — orchestrator path
  (T1003-owned) and renderer (`render::reconciliation` —
  T1003-owned) untouched. Two deviation notes recorded inline
  on the task body: (1) V2 acceptance line 506-507 mentions
  `equity-<window>.csv`'s `unrealized_pnl_usdt` column; this is
  stale relative to architect Design § R3 scope-out (lines
  658-661) and T1003's "**Out of scope:** R3 equity-curve
  sampler stays cash-balance-only" — the orchestrator at
  `crates/reports/src/lib.rs:519-525` hardcodes
  `unrealized_pnl: Decimal::ZERO` per sample as architect-
  intended; V2 instead asserts the load-bearing R11.1 cell
  which derives directly from the new `unrealized` arithmetic.
  (2) V6 acceptance line 519 suggests
  `tracing_subscriber::fmt::TestWriter`; used `Layer` +
  `WarnVisitor` instead so the `symbol = ETHUSDT` assertion runs
  against structured fields rather than fmt-rendered text.
  Validation: `cargo test -p reports --test
  unrealized_orchestrator --test mark_unavailable_warns` →
  3/3 PASS; `cargo test -p reports` (full crate, all
  integration test binaries green) clean; `cargo clippy
  --workspace --all-targets --all-features -- -D warnings`
  clean; `cargo fmt --all -- --check` clean; `bash
  scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` (T1006
  only adds two NEW test targets + one Cargo.toml dev-dep; no
  source/render path touched, both v1+ anchor bodies stay
  byte-identical by construction). HANDOFF → orchestrator
  (T1006 done; T_FINAL_REAL_MTM remains, owned by tester per
  AGENT.md process discipline).
