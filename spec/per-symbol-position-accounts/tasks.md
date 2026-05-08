---
slug: per-symbol-position-accounts
status: shipped
owner: developer
updated: 2026-05-03
---

# Tasks — Per-symbol position accounts

Ordered, testable task list derived from
[spec/per-symbol-position-accounts/feature.md → Design](../features/per-symbol-position-accounts.md#design)
and the eight architect resolutions (Q1–Q8) recorded in that
Design section. Cross-references to the analyst's R/V items use the
format `Rn` / `Vn`; cross-references to the architect's resolutions
use `Qn`.

T8xx is taken by [operator-success-reports](operator-success-reports.md);
T9xx is taken by [live-cockpit-unified](live-cockpit-unified.md);
T10xx is taken by [real-mtm-unrealized-pnl](real-mtm-unrealized-pnl.md);
this feature uses **T1101–T1107** + `T_FINAL_PER_SYMBOL`.

Owner tags:
- `[developer]` — backend Rust + SQL work in `crates/audit/`,
  `crates/reports/tests/fixtures/`. Wave-2 tasks fan out across
  three independent files.
- `[no UI work]` — this is a chart-of-accounts plumbing feature.
  No screens, no widgets, no copy. ui-designer not involved.
- `[tester]` — sole owner of `T_FINAL_PER_SYMBOL`.

**Parallelism gates** (shared files — only one task at a time
touches each):

- `crates/audit/migrations/006_per_symbol_position_accounts.sql`
  (NEW) — T1101 is the sole creator.
- `crates/audit/src/journal.rs:82,135` (existing `post_fill`
  function body) — T1102 is the sole writer.
- `crates/audit/src/bootstrap.rs:65 seed_universe_accounts`
  (existing) — T1103 is the sole writer (adds `#[deprecated]` attribute).
- `crates/audit/src/query.rs::open_positions_at`,
  `pnl_by_symbol`, `recent_fills` (existing) — T1102 is the sole
  writer of the defensive Q4 cross-check (added in same wave as
  the `journal.rs` writer flip to keep the cross-check anchored
  to the writer change).
- `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`
  (existing T1004 fixture; non-anchored) — T1104 is the sole
  writer of the mixed-row extension.
- `crates/audit/tests/per_symbol_post_fill.rs` (NEW) — T1105 is
  the sole creator. V1 + V2 + V5 + V8.
- `crates/reports/tests/open_positions_mixed_ledger.rs` (NEW) —
  T1106 is the sole creator. V3 + V7.

**Synchronization points** (block downstream tasks):

- **T1101** — migration `006` lands. Blocks T1102 (post_fill
  cannot write to a per-pair account that doesn't exist in
  `accounts`), T1104 (fixture extension's `post_fill` calls would
  also FK-fail), T1105 (test setup runs migrations), T1106 (same).
- **T1102** — `post_fill` writer flip + reader cross-check. Blocks
  T1105 (V1 asserts the new writer's output) and T1106 (V3 reads
  via the updated readers).
- **T1104** — fixture extension. Blocks T1106 (V3 + V7 read it).
- **T1107** — anchor regression sweep. Depends on T1102 + T1104
  shipping. Independent of T1105 / T1106 from a code-touch
  perspective; sequenced after T1102 because the writer flip is
  what could regress anchors (Q7 says it won't — V4 verifies).

**Granularity:** ½ day per task except T_FINAL_PER_SYMBOL (tester
gate). Smaller than real-mtm-unrealized-pnl because this is
plumbing-only on top of an already-implemented chart-of-accounts
table — no new types, no new public API, no new render path.

## Wave 1 — migration

- [x] **T1101** [developer] — Migration
  `006_per_symbol_position_accounts.sql` per
  [Design → Migration shape — exact SQL](../features/per-symbol-position-accounts.md#migration-shape--exact-sql):
  - New file `crates/audit/migrations/006_per_symbol_position_accounts.sql`.
    Pure SQL — 10 `INSERT OR IGNORE INTO accounts (id, kind, currency)
    VALUES (?, ?, ?)` lines, one per pair-symbol in
    `config/agent.toml:62-65 [funding].universe`:
    `BTCUSDT, ETHUSDT, BNBUSDT, SOLUSDT, XRPUSDT, ADAUSDT, DOGEUSDT,
    AVAXUSDT, DOTUSDT, LINKUSDT`.
  - `kind = 'asset'`, `currency = 'USDT'` (matches the row shape of
    the legacy `assets:position:BTC` row at
    `crates/audit/src/bootstrap.rs:19`).
  - File header comment matches the cadence of `004_*.sql` and
    `005_*.sql` (analyst-brief context line, R-item references,
    anchor-impact note).
  - **No schema change** (Q1 option a). No DDL. No CREATE TABLE.
    No ALTER TABLE.
  - **Idempotent.** Re-running migration `006` against an audit DB
    that already has the rows is a no-op (the `OR IGNORE` clause).
  - **Library checklist:** N/A (no new dep; pure SQL file picked up
    by the existing `sqlx::migrate!("./migrations")` macro at
    `crates/audit/src/ledger.rs:32`).
  _acceptance: `cargo build -p audit` clean (sqlx finds and embeds
  the new file via the proc-macro at compile time);
  `cargo test -p audit --tests` passes — every existing test that
  bootstraps a ledger via `Ledger::open(...)` now applies migration
  006 transparently with no test-side change required;
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (no
  test rendering touched yet)._
  **[gate for T1102, T1104, T1105, T1106]**

  **Citation (developer, 2026-05-01):**
  - **migration file:** `crates/audit/migrations/006_per_symbol_position_accounts.sql:1-25`
    (10 `INSERT OR IGNORE` lines for the universe pair-symbols, exact SQL
    from Design § Migration shape).
  - **smoke test file:** `crates/audit/tests/migration_006_smoke.rs:1-89`
    (`t1101_migration_006_seeds_per_symbol_accounts` asserts all 10
    `assets:position:<SYMBOL>` rows present after fresh `Ledger::in_memory()`;
    `t1101_migration_006_is_idempotent` asserts re-applying the INSERTs
    keeps the row count at 10).
  - **test cmd:** `cargo test -p audit --test migration_006_smoke`
  - **output line:** `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
  - **collateral:** `crates/audit/tests/ledger_integration.rs:44-83` updated
    — the `t05_account_list_returns_all_v0_accounts` and
    `t05_bootstrap_is_idempotent` tests' hardcoded "13 accounts" expectation
    is bumped to 23 and the expected-list extended with the 10 new
    per-symbol position accounts. Test-side update only — no `src/` change.
  - **gates:** `cargo build -p audit` clean; `cargo test -p audit` →
    all suites green (8/8 ledger_integration; 2/2 migration_006_smoke;
    plus all other audit tests); `cargo clippy --workspace --all-targets
    --all-features -- -D warnings` clean; `cargo fmt --all -- --check`
    clean; `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)`.

## Wave 2 — writer flip + cross-check + deprecation + fixture (parallel)

- [x] **T1102** [developer] — `post_fill` writes per-pair account
  + readers gain Q4 defensive cross-check per
  [Design → Crate map delta](../features/per-symbol-position-accounts.md#crate-map-delta)
  and [Q4](../features/per-symbol-position-accounts.md#q4--reader-keep-description-parse-as-primary-account-id-as-defensive-cross-check):
  - **Writer side** — at `crates/audit/src/journal.rs:82` (Buy
    debit) and line 135 (Sell credit), replace the literal
    `"assets:position:BTC"` with a hoisted local:
    ```rust
    let position_account_id = format!("assets:position:{}", fill.symbol);
    ```
    declared once at the top of `post_fill`'s function body
    (after the existing `description = format!(...)` line at
    `journal.rs:48-51`, where `fill.symbol` is already used) and
    bound to both the Buy debit `insert_entry` call and the Sell
    credit `insert_entry` call. **No signature change**: T802's
    `pub async fn post_fill(ledger, fill, strategy_id) -> Result<...>`
    stays byte-identical.
  - **Reader side (Q4 cross-check)** — at
    `crates/audit/src/query.rs::open_positions_at` and at
    `pnl_by_symbol` and `recent_fills` (the three readers that
    consume `extract_symbol_from_description`), AFTER the existing
    description-parse, add a single defensive check:
    ```rust
    // Q4 cross-check: account_id should be either the legacy BTC
    // bucket or the per-pair form for the parsed symbol.
    if account_id.starts_with("assets:position:") {
        let expected_per_pair = format!("assets:position:{}", parsed_symbol);
        if account_id != "assets:position:BTC" && account_id != expected_per_pair {
            tracing::warn!(
                target: "audit::query",
                %account_id,
                parsed_symbol = %parsed_symbol,
                "account_id / description-symbol mismatch; falling back to description"
            );
        }
    }
    ```
    The reader continues to use `parsed_symbol` regardless. The
    cross-check is observation-only (Q4 rationale).
  - **Doc-comment update** on `extract_symbol_from_description` at
    `query.rs:512` per Q5: add a paragraph "Primary symbol source
    for both pre- and post-migration rows; the account-id-suffix
    path (Q4 cross-check) is a defensive observation only. New code
    that needs structural symbol attribution SHOULD call
    `open_positions_at` or `pnl_by_symbol` rather than parsing
    description directly." Do NOT add `#[deprecated]` (Q5).
  - **Determinism:** the writer change is byte-identical for
    descriptions (`format!("{} {} {} @ {}", ...)` unchanged); only
    the account-id literal moves from compile-time constant to
    runtime `format!(...)`. The reader cross-check uses
    `tracing::warn!` (no return-value branch), so two reads of the
    same ledger remain byte-identical.
  - **No new dep**, no `Cargo.toml` change, no `unsafe`.
  _acceptance: `cargo build -p audit` clean; `cargo clippy -p audit
  --all-targets --all-features -- -D warnings` clean; `cargo fmt -p
  audit -- --check` clean; existing test
  `crates/audit/tests/ledger_integration.rs` (T802 suite) still
  passes verbatim (writer signature unchanged); existing test
  `crates/audit/tests/open_positions_at.rs` (T1005 V1) still passes
  (reader public surface unchanged); `bash scripts/verify_anchors.sh`
  → `ANCHORS PASS  (11 / 11)` (Q7 / V4)._
  **[gate for T1105, T1106, T1107 — deps: T1101]**

  **Citation (developer, 2026-05-01):**
  - **writer file:** `crates/audit/src/journal.rs:63` hoists
    `let position_account_id = format!("assets:position:{}", fill.symbol);`
    after the existing `description = format!(...)` block; line 94 (Buy
    debit, was BTC hardcode at original line 82) and line 147 (Sell credit,
    was BTC hardcode at original line 135) now bind `&position_account_id`
    instead of the literal `"assets:position:BTC"`. `post_fill` signature
    is byte-identical (T802 / R8 invariant preserved).
  - **reader file:** `crates/audit/src/query.rs:1043` extends the
    `open_positions_at` SELECT to LEFT JOIN `journal_entries` on
    `account_id LIKE 'assets:position:%'` (returning the position-side
    account-id alongside the existing description fields); lines 1084-1104
    add the Q4 defensive cross-check — when `account_id` neither matches
    the legacy `"assets:position:BTC"` whitelist nor
    `format!("assets:position:{}", parsed_symbol)`, emit
    `tracing::warn!(target: "audit::query", ...)` and continue with the
    description-parsed symbol (Q4 description-parse stays primary). Never
    raises; never branches the return value. Doc-comment updated at
    `query.rs:973-981` to document the cross-check.
  - **test file:** `crates/audit/tests/t1102_per_symbol_post_fill.rs:1-237`
    (`t1102_post_fill_writes_per_symbol_account` — posts ETHUSDT/BTCUSDT/
    SOLUSDT fills and asserts each lands on its per-pair account-id with
    zero rows on the legacy `assets:position:BTC` bucket;
    `t1102_open_positions_at_handles_legacy_rows` — mixed-shape ledger
    with one post-T1102 ETHUSDT row + one hand-crafted legacy
    `assets:position:BTC` row whose description carries `BTCUSDT`; the
    reader returns 2 correct `OpenPosition` rows sorted alphabetically,
    proving Q4 description-parse handles both shapes).
  - **test cmd:** `cargo test -p audit --test t1102_per_symbol_post_fill`
  - **output line:** `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
  - **gates:** `cargo build -p audit` clean; `cargo test -p audit` →
    all suites green (2/2 t1102_per_symbol_post_fill plus all other
    audit tests, including the existing T1005 V1 open_positions_at and
    the T802 ledger_integration suites — reader / writer signatures
    unchanged); `cargo clippy --workspace --all-targets --all-features
    -- -D warnings` clean; `cargo fmt --all -- --check` clean; `bash
    scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (Q7 / R5 / V4
    confirmed byte-identical).

- [x] **T1103** [developer] — `seed_universe_accounts` deprecation
  per [Q8](../features/per-symbol-position-accounts.md#q8--seed_universe_accounts-deprecate-do-not-delete-migration-is-the-source-of-truth):
  - Done 2026-05-01 (developer). `#[deprecated(since = "1.6.0",
    note = "...")]` attribute landed at
    `crates/audit/src/bootstrap.rs:64-71`, immediately above the
    `#[instrument]` attribute and the `pub async fn
    seed_universe_accounts` signature. Body unchanged. Function name
    unchanged. Public-surface change is the attribute only.
  - Verified zero callers via `grep -rn "seed_universe_accounts"
    --include='*.rs' .` → only line returned is the function
    definition itself at `crates/audit/src/bootstrap.rs:73:pub async
    fn seed_universe_accounts(`. No HANDOFF → architect needed.
  - `cargo build -p audit` → `Finished `dev` profile [unoptimized
    + debuginfo] target(s) in 1.67s`. Clean, no errors, no
    deprecation warning emitted (silent because zero callers — Q8
    prediction holds).
  - `cargo test -p audit` → all suites green. Sample lines:
    `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured` for
    `v15a_journal_test`; `test result: ok. 6 passed; 0 failed`
    for `uptime_intervals_test`; `test result: ok. 5 passed; 0
    failed` for `chart_of_accounts` cross-suite. No deprecation
    warnings emitted by any test (zero callers).
  - `cargo clippy -p audit --all-targets --all-features -- -D
    warnings` → reports two errors, both in
    `crates/audit/src/query.rs:976,978` (`clippy::doc_markdown` on
    `account_id` doc-comment) — these are owned by T1102 (running
    in parallel; the dev there is editing `query.rs`). Zero clippy
    hits on `bootstrap.rs` or `seed_universe_accounts` (verified by
    `grep -E "(bootstrap|seed_universe)"` over the clippy output —
    empty). My T1103 surface is clippy-clean.
  - `cargo fmt -p audit -- --check` → clean (empty output).
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.
    No anchor drift.
  - At `crates/audit/src/bootstrap.rs:65`, add the attribute:
    ```rust
    #[deprecated(
        since = "1.6.0",
        note = "shape mismatch — takes base assets (e.g. \"BTC\") but \
                migration 006_per_symbol_position_accounts.sql seeds \
                pair symbols (e.g. \"BTCUSDT\"). The migration is the \
                canonical seed; this function has zero callers and \
                will be removed in a follow-up wave."
    )]
    ```
    Body unchanged. Function name unchanged. Public-surface change is
    the attribute only.
  - Verify via `grep -rn "seed_universe_accounts" crates/` that there
    are STILL zero callers across the workspace (the `#[deprecated]`
    warning is therefore silent in normal builds; if the grep
    surfaces anything, route HANDOFF → architect, do not tick).
  - **No removal** in this task — Q8 explicitly keeps the function
    in tree until a separate clean-up wave.
  - **Library checklist:** N/A (no dep change).
  _acceptance: `cargo build -p audit` clean; `cargo clippy -p audit
  --all-targets --all-features -- -D warnings` clean (deprecation
  warning is silent because no caller); `cargo test -p audit
  --tests` passes (existing tests of `seed_universe_accounts`, if
  any, would emit a deprecation warning — that is not a clippy
  hard failure with the workspace's `-D warnings` because
  `#[deprecated]` is a `warn` lint not an `error` lint at clippy's
  default config; if a test does emit the warning, the test file
  may need a `#[allow(deprecated)]` attribute on the test
  function — note this in the honest-tick deviation block);
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`._
  **[parallel-safe with T1102, T1104; deps: T1101]**

- [x] **T1104** [developer] — Extend
  `build_ledger_with_open_positions_7d` fixture with mixed
  legacy/new rows per
  [Q6](../features/per-symbol-position-accounts.md#q6--fixture-extend-build_ledger_with_open_positions_7d):

  **Citation (developer, 2026-05-01):**
  - **fixture extension file:**
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs:384-445`
    — new public async fn `build_ledger_mixed_legacy_and_per_symbol_7d(
    ledger: &Ledger) -> Result<Vec<JournalEntryView>, LedgerError>`
    pre-pends two legacy `assets:position:BTC` rows via raw
    `sqlx::query` (BTCUSDT Buy 1.0@60_000 and ETHUSDT Buy 5.0@2_500,
    descriptions `"buy 1.0 BTCUSDT @ 60000"` and
    `"buy 5.0 ETHUSDT @ 2500"`) and appends one post-006 SOLUSDT Buy
    `qty=10.0@price=100, strategy_id=Some("test_strategy")` via
    `audit::journal::post_fill`.
  - **legacy-row helper:**
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs:447-503`
    — `insert_legacy_buy(...)` writes one balanced
    `(Dr assets:position:BTC, Cr assets:cash:USDT)` double-entry
    transaction per legacy row, deliberately bypassing the post-T1102
    `post_fill` writer to faithfully reproduce the pre-006 ledger
    shape (Q6 design: "deliberate raw-SQL bypass of `post_fill`").
  - **deviation note:** the spec literally writes
    `"Buy 1.0 BTCUSDT @ 60000"` (capital "Buy"), but the actual
    `Side::Display` impl at `crates/core/src/symbol.rs:69` writes
    lowercase `"buy"`. Used lowercase in descriptions to match
    `post_fill`'s actual format and `open_positions_at`'s
    `description LIKE 'buy %' OR description LIKE 'sell %'` filter
    (`crates/audit/src/query.rs:1042`). Without lowercase the legacy
    rows would be invisible to the reader — the whole point of V3 is
    to exercise reader correctness across the migration boundary.
  - **deterministic UUIDs:** seeds `MIXED_BTC_TXN_SEED` =
    `0xC114_0101` etc. (constants at lines 372-379 of the same file),
    distinct from the existing `0xF333_xxxx` family used by the
    closed/dangling plan so the two builders never collide on a
    primary key.
  - **fixed timestamps:** `LEGACY_BTC_FILL_RFC3339 =
    "2026-04-27T19:00:00Z"`, `LEGACY_ETH_FILL_RFC3339 =
    "2026-04-27T19:00:01Z"`, `POST_SOL_FILL_RFC3339 =
    "2026-04-27T19:00:02Z"` — chronological + before
    `PERIOD_END_RFC3339`. RFC-3339 second-precision (matches
    `post_fill`'s timestamp format at
    `crates/audit/src/journal.rs:42-45`).
  - **test cmd:** `cargo test -p reports --test
    fixture_with_open_positions_smoke`
  - **output line:** `test result: ok. 3 passed; 0 failed; 0 ignored;
    0 measured; 0 filtered out; finished in 0.05s` — the existing
    T1004 smoke tests
    (`t1004_fixture_emits_two_open_positions`,
    `t1004_fixture_two_builds_byte_identical_fills`,
    `t1004_fixture_has_expected_open_positions_at_period_end`) still
    pass against the unchanged
    `build_ledger_with_open_positions_7d`. The new
    `build_ledger_mixed_legacy_and_per_symbol_7d` is dead-code
    (gated by `#[allow(dead_code)]`) until T1106 adds its
    consumer test in `crates/reports/tests/open_positions_mixed_ledger.rs`.
  - **gates:** `cargo build -p reports --tests` clean; `cargo clippy
    --workspace --all-targets --all-features -- -D warnings` clean;
    `cargo fmt --all -- --check` clean; `cargo test -p reports` →
    all suites green; `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)`.
  - **scope discipline:** sole edit was to
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`.
    No edits to `crates/audit/src/journal.rs`, `query.rs`,
    `bootstrap.rs`, or any other source file — those are owned by
    T1102/T1103 (running in parallel).

  - Edit
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`
    in place. The existing fixture is non-anchored (T1004 created
    it specifically so anchor-locked fixtures stay stable) — Q6
    overrides analyst's b accordingly.
  - Add a new public function (alongside the existing one):
    ```rust
    pub async fn build_ledger_mixed_legacy_and_per_symbol_7d(
        ledger: &Ledger,
    ) -> Vec<JournalEntryView>;
    ```
    Body:
    1. Pre-pend two legacy rows via direct SQL `INSERT INTO
       journal_transactions (...) VALUES (...)` + `INSERT INTO
       journal_entries (account_id='assets:position:BTC', ...)`.
       These mimic pre-006 fills: one BTCUSDT Buy and one ETHUSDT
       Buy, both writing to the literal `assets:position:BTC`
       account, descriptions `"Buy 1.0 BTCUSDT @ 60000"` and
       `"Buy 5.0 ETHUSDT @ 2500"` respectively. Use canonical
       UUIDs from the existing fixture's seed (`FIXTURE_SEED =
       0xC0FFEE` precedent at `build_ledger_7d.rs`) so output is
       deterministic.
    2. Append one post-006 fill via the updated
       `audit::journal::post_fill`: a SOLUSDT Buy of `qty = 10.0
       @ price = 100`, strategy_id `Some("test_strategy")`. This
       writes to `assets:position:SOLUSDT`.
    3. Total 3 open positions in the resulting ledger.
  - Add a doc-comment block on the new function explaining the
    fixture's purpose: "Exercises V3 + V7 across the migration
    boundary. Pre-006 rows write the literal
    `assets:position:BTC` account regardless of underlying symbol
    (deliberate raw-SQL bypass of `post_fill`); post-006 rows
    write the per-pair account-id."
  - **Determinism:** `BTreeMap`-ordered emit; no `HashMap`; no
    `time::Instant`; raw-SQL `INSERT` uses the same `Rfc3339`
    microsecond format as `post_fill`. Two fixture builds against
    the same temp DB return byte-identical `Vec<JournalEntryView>`.
  - **No new dep**, no `Cargo.toml` change.
  _acceptance: `cargo build -p reports --tests` clean; `cargo
  clippy -p reports --tests --all-features -- -D warnings` clean;
  the existing `build_ledger_with_open_positions_7d` function
  still compiles unchanged; the new
  `build_ledger_mixed_legacy_and_per_symbol_7d` function is
  exercised by T1106 V3 in Wave 3._
  **[parallel-safe with T1102, T1103; deps: T1101]**

## Wave 3 — verification tests (parallel)

- [x] **T1105** [developer] — V1 + V2 + V5 + V8 tests in
  `crates/audit/tests/per_symbol_post_fill.rs` per
  [Design → Test strategy](../features/per-symbol-position-accounts.md#test-strategy-per-v-item):

  **Citation (developer, 2026-05-01):**
  - **test file:**
    `crates/audit/tests/per_symbol_post_fill.rs:1-352` — new
    `#[tokio::test]` integration test target with four functions:
    - `t1105_v1_post_fill_writes_per_symbol_account` (line 144) —
      posts ETHUSDT/BTCUSDT/SOLUSDT Buys via
      `audit::journal::post_fill`; asserts the position-side
      `journal_entries` rows group by exactly the three per-pair
      account-ids (BTCUSDT/ETHUSDT/SOLUSDT, 1 row each, sorted) and
      zero rows reference the legacy `assets:position:BTC` bucket
      (V1).
    - `t1105_v2_legacy_row_readable_after_migration` (line 199) —
      hand-crafts a legacy ETHUSDT Sell via raw SQL targeting the
      pre-T1102 hardcode `assets:position:BTC` (description
      `"sell 0.5 ETHUSDT @ 2200"`, balanced
      `Dr cash 1100 / Cr position:BTC 1000 / Cr realized_pnl 100`).
      Asserts (a) `account_id` preserved verbatim post-006 (R3),
      (b) `journal::verify_balance(...)` returns `Ok(())` (R6), (c)
      `query::pnl_by_symbol(...)` returns
      `[(Symbol::new("ETHUSDT"), Money::from_decimal(dec!(100)))]`
      via the description-parse fallback (R7).
    - `t1105_v5_balance_invariant_pre_and_post_migration` (line
      287) — re-uses T1104's
      `build_ledger_mixed_legacy_and_per_symbol_7d` fixture (mixed
      legacy `assets:position:BTC` rows + post-006
      `assets:position:SOLUSDT` row); iterates every
      `journal_transactions.id` and asserts
      `journal::verify_balance(...) == Ok(())`; then asserts
      `Σ debit_amount == Σ credit_amount` globally on
      `journal_entries` (R6).
    - `t1105_v8_universe_coverage` (line 332) — parses
      `config/agent.toml`'s `[funding].universe` directly via
      `toml::from_str` into a typed `AgentTomlSlice`/`FundingTomlSlice`
      (cannot import `agent::Config` — `agent` depends on `audit`,
      cycle); for every symbol asserts the post-006 `accounts` table
      contains a row with id `assets:position:<SYMBOL>` (R11).
  - **fixture mount:**
    `crates/audit/tests/per_symbol_post_fill.rs:64-66` — re-mounts
    `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`
    via `#[path]` + `#[allow(dead_code)]` (the file exports both the
    closed/dangling 7d plan and the mixed builder; this test only
    consumes the latter — same pattern as T1005's
    `crates/audit/tests/open_positions_at.rs:54`).
  - **dev-dep added:** `crates/audit/Cargo.toml:33` — added
    `toml.workspace = true` to `[dev-dependencies]` so the V8 test
    can deserialize the universe slice directly. Workspace `toml =
    "0.8"` is already pinned at `Cargo.toml:55`; no version drift.
  - **deviation note:** the spec acceptance says "load
    `config/agent.toml` via the existing config reader" referring
    to `agent::config::Config::load`, but `agent` already depends on
    `audit` (cycle). I parse the same TOML key path
    (`[funding].universe`) directly via `toml::from_str` into a
    typed slice that mirrors `agent::config::FundingConfig`'s shape;
    the V8 acceptance assertion (R11 — every universe symbol has an
    `assets:position:<SYMBOL>` chart-of-accounts row) is unchanged.
  - **test cmd:** `cargo test -p audit --test per_symbol_post_fill`
  - **output line:**
    `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
  - **gates:**
    - `cargo test -p audit` → all 14 audit suites green; zero
      failures across the workspace's audit-side test surface
      (sample tail lines: `test result: ok. 9 passed` for
      `v15a_journal_test`; `test result: ok. 6 passed` for
      `uptime_intervals_test`; `test result: ok. 2 passed` for
      `t1102_per_symbol_post_fill`).
    - `cargo clippy --workspace --all-targets --all-features -- -D
      warnings` → `Finished `dev` profile [unoptimized + debuginfo]
      target(s) in 1.47s` (no warnings on
      `crates/audit/tests/per_symbol_post_fill.rs`; the doc list
      formatting was reshaped to free-form paragraphs to satisfy
      `clippy::doc_lazy_continuation` /
      `clippy::doc_overindented_list_items`).
    - `cargo fmt --all -- --check` → clean.
    - `bash scripts/verify_anchors.sh` →
      `ANCHORS PASS  (11 / 11)`. Zero anchor drift (the new test
      file is read-only over the audit DB; no rendering path; no
      report body cells touched — Q7 holds).
  - **scope discipline:** sole edits were
    `crates/audit/tests/per_symbol_post_fill.rs` (new) and
    `crates/audit/Cargo.toml` (`toml` dev-dep added with explanatory
    comment at line 33–39). No edits to `crates/audit/src/journal.rs`,
    `query.rs`, `bootstrap.rs` (Wave 2 territory) or anything in
    `crates/reports/` (T1106 dev's parallel territory).

  - New file `crates/audit/tests/per_symbol_post_fill.rs`. Four
    `#[tokio::test]` functions:
    - `t1105_v1_post_fill_writes_per_symbol_account` — boot empty
      audit DB via `Ledger::open(":memory:")`; chart_of_accounts +
      migrations 002–006 auto-applied. Post one ETHUSDT Buy + one
      BTCUSDT Buy + one SOLUSDT Buy via `post_fill`. Assert via
      `SELECT account_id, COUNT(*) FROM journal_entries WHERE
      account_id LIKE 'assets:position:%' GROUP BY account_id
      ORDER BY account_id` returns exactly 3 distinct account-ids:
      `assets:position:BTCUSDT`, `assets:position:ETHUSDT`,
      `assets:position:SOLUSDT`. Assert zero rows reference
      `assets:position:BTC` (the legacy account is now empty for
      post-T1102 fills).
    - `t1105_v2_legacy_row_readable_after_migration` — hand-craft
      a pre-006 audit DB: open ledger, run migrations 001–005 only
      (NB: this requires a test-only helper since `Ledger::open`
      runs ALL migrations; alternatively, run all migrations then
      directly `INSERT` a legacy-shape row that writes ETHUSDT to
      `assets:position:BTC`). Migration 006 is then applied
      (either a no-op if all migrations were already run, or
      explicitly via a re-`open`). Assert: (1) `SELECT account_id
      FROM journal_entries WHERE id = ?` still returns
      `assets:position:BTC` (legacy row unchanged, R3); (2)
      `audit::verify_balance(transaction_id) == Ok(())`; (3)
      `audit::query::pnl_by_symbol(...)` correctly buckets the row
      under `Symbol::new("ETHUSDT")` (description-parse
      fallback, R7).
    - `t1105_v5_balance_invariant_pre_and_post_migration` — same
      mixed fixture as V3 (T1106 fixture), but iterates all
      `journal_transactions.id` values and asserts
      `audit::verify_balance(txn_id) == Ok(())` for each. Then
      asserts `Σ debit_amount == Σ credit_amount` globally (one
      `SELECT SUM(...)` query per side). R6.
    - `t1105_v8_universe_coverage` — load `config/agent.toml` via
      the existing config reader; iterate
      `cfg.funding.universe` (10 symbols); for each, assert
      `SELECT 1 FROM accounts WHERE id = 'assets:position:<SYM>'`
      returns one row. R11.
  - **Determinism:** `BTreeMap`-ordered iteration over the
    universe; `assert_eq!` on `Vec<String>` of account-ids sorted
    lexicographically.
  - **Library checklist:** N/A (uses existing test deps —
    `tokio`, `audit`, `trading_core`, `agent::config` for the
    universe loader).
  _acceptance: `cargo test -p audit --test per_symbol_post_fill`
  → 4 / 4 PASS; `cargo clippy -p audit --tests --all-features
  -- -D warnings` clean; `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)`._
  **[parallel-safe with T1106, T1107 — deps: T1101, T1102]**

- [x] **T1106** [developer] — V3 + V7 tests in
  `crates/reports/tests/open_positions_mixed_ledger.rs` per
  [Design → Test strategy](../features/per-symbol-position-accounts.md#test-strategy-per-v-item):

  **Citation (developer, 2026-05-01):**
  - **test file:**
    `crates/reports/tests/open_positions_mixed_ledger.rs:1-167` —
    new integration test target with two `#[tokio::test]` functions
    (`t1106_v3_mixed_ledger_correct_open_positions` and
    `t1106_v7_two_reads_byte_identical`) that consume T1104's
    `build_ledger_mixed_legacy_and_per_symbol_7d` via the standard
    `#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]`
    pattern that T1004/T1005 established.
  - **V3 assertion shape:** `crates/reports/tests/open_positions_mixed_ledger.rs:65-128`
    — `assert_eq!(positions, expected)` where `expected` is the
    hand-computed 3-row vec sorted alphabetically by symbol
    (BTCUSDT row 0, ETHUSDT row 1, SOLUSDT row 2). Each row carries
    the full `OpenPosition` tuple (`symbol`, `qty`,
    `avg_cost_basis`, `opened_at`, `strategy_id`). Legacy rows
    (BTCUSDT, ETHUSDT) carry `strategy_id: None` because the raw-SQL
    insert in `insert_legacy_buy` writes
    `journal_transactions.strategy_id = NULL`; the post-006 SOLUSDT
    row carries `Some(StrategyId::new("test_strategy"))` per the
    `post_fill` call in the fixture.
  - **V7 assertion shape:** `crates/reports/tests/open_positions_mixed_ledger.rs:138-167`
    — two consecutive `query::open_positions_at(&ledger, period_end)`
    calls on the same opened ledger, then `assert_eq!(first, second)`.
    Belt-and-braces `assert_eq!(first.len(), 3)` defends against a
    future regression that returned an empty Vec from both reads
    (which would otherwise satisfy byte-equality vacuously).
  - **scope discipline:** sole new file is
    `crates/reports/tests/open_positions_mixed_ledger.rs`. NO edits
    to `crates/audit/` (T1105's parallel territory) or
    `crates/reports/src/`. The test mounts the existing T1104
    fixture via `#[path = "fixtures/..."]` — no fixture
    modification needed.
  - **dead_code suppression:** `#![allow(dead_code)]` at file top
    (line 1) — the T1004 fixture exposes 8+ public helpers consumed
    by sibling tests (T1004 smoke, T1005 V1/V4/V7) but only the
    mixed builder + `parse_rfc3339` are consumed here. Mirrors how
    T1005 handles the same `#[path]`-mounted module (also in audit
    side via `#[path = "../../reports/tests/fixtures/..."]`).
  - **test cmd:** `cargo test -p reports --test open_positions_mixed_ledger`
  - **output line:** `test result: ok. 2 passed; 0 failed; 0 ignored;
    0 measured; 0 filtered out; finished in 0.03s` — both
    `t1106_v3_mixed_ledger_correct_open_positions` and
    `t1106_v7_two_reads_byte_identical` PASS on the mixed-ledger
    fixture with the expected 3-row `Vec<OpenPosition>`.
  - **gates:** `cargo test -p reports` → 98 + 2 + … all green
    (zero failures, zero filtered); `cargo clippy -p reports
    --all-targets --all-features -- -D warnings` → clean (only
    `dead_code` warnings from the path-mounted fixture, suppressed
    via the file-top `#![allow]`); `cargo fmt -p reports --check`
    → clean (no diffs); `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)`.
  - **note on workspace clippy:** workspace-wide `cargo clippy
    --workspace --all-targets --all-features -- -D warnings`
    surfaces 11 errors, ALL in `crates/audit/tests/per_symbol_post_fill.rs`
    (T1105's parallel-territory file — 7 dead_code on the
    `#[path]`-mounted fixture + 4 doc_lazy_continuation). T1106's
    file is clean per the per-crate clippy run; T1105's developer
    owns those 11 errors.

  - New file
    `crates/reports/tests/open_positions_mixed_ledger.rs`. Two
    `#[tokio::test]` functions:
    - `t1106_v3_mixed_ledger_correct_open_positions` — call
      `build_ledger_mixed_legacy_and_per_symbol_7d(&ledger)` from
      T1104. Then call
      `audit::query::open_positions_at(&ledger, period_end)`.
      Assert returned `Vec<OpenPosition>` has length 3, sorted
      `(BTCUSDT, ETHUSDT, SOLUSDT)` per the existing T1002 sort
      key `(symbol ASC, strategy_id ASC, None last)`. Assert each
      OpenPosition has the expected `(qty, avg_cost_basis,
      strategy_id)` tuple from the fixture.
    - `t1106_v7_two_reads_byte_identical` — build the same fixture
      once; call `open_positions_at(&ledger, period_end)` twice
      back-to-back; `assert_eq!(first, second);` using
      `OpenPosition`'s derived `PartialEq`. R10.
  - The Q4 cross-check warn-emit is NOT explicitly asserted here
    — it's an observation-only path. A follow-up test can capture
    `tracing::warn!` if regression risk surfaces, but for V3 + V7
    the assertions on `Vec<OpenPosition>` shape are sufficient.
  - **Determinism:** uses T1104's deterministic fixture; reader
    is deterministic per T1002.
  - **Library checklist:** N/A (uses existing test deps).
  _acceptance: `cargo test -p reports --test open_positions_mixed_ledger`
  → 2 / 2 PASS; `cargo clippy -p reports --tests --all-features
  -- -D warnings` clean; `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)`._
  **[parallel-safe with T1105, T1107 — deps: T1101, T1102, T1104]**

## Wave 4 — anchor regression sweep

- [x] **T1107** [developer] — Anchor regression sweep + V4
  verification per
  [Design → Risks & mitigations § 5](../features/per-symbol-position-accounts.md#risks--mitigations):

  **Citation (developer, 2026-05-01):**
  - **freshly-rendered v1+ reports (T816 fixture re-run produced
    these; mtime 09:54 today):**
    - `spec/operator-success-reports/reports/success-fixed-report-sample-7d.md` —
      body SHA-256 matches anchor
      `ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c`.
    - `spec/operator-success-reports/reports/success-fixed-report-sample-90d.md` —
      body SHA-256 matches anchor
      `2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f`.
  - **test cmd (re-renders the 2 v1+ reports):**
    `cargo test -p reports --test report_scenarios --release`
  - **test output line:**
    `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.53s`
    — covers `t816_report_sample_7d_determinism_and_anchor_lock`,
    `t816_report_sample_90d_determinism_and_anchor_lock`, plus the
    two `t816_v10_cron_friendly_3x_parallel_*` determinism tests.
  - **anchor gate cmd:** `bash scripts/verify_anchors.sh`
  - **verbatim stdout (11 / 11 PASS — Q7 zero-anchor-risk
    confirmed across all 9 backtest scenarios + the 2 v1+
    operator-success-report scenarios):**

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
  - **scope discipline:** verification-only. No Rust source edits;
    no `spec/anchors.toml` edits; no edits to any report file
    (those are append-only). Sole spec edit was this honest-tick
    citation block on T1107 in
    `spec/per-symbol-position-accounts/tasks.md`.
  - **deviation note from acceptance:** the spec acceptance also
    lists `cargo test --workspace --all-targets`, `cargo clippy
    --workspace --all-targets --all-features -- -D warnings`, and
    `cargo fmt --all -- --check`. The orchestrator's T1107 brief
    scopes this task down to "Re-run the 2 v1+ report scenarios
    + verify all 11 anchors" (procedure steps 1 + 2; step 3
    marked optional). The workspace-wide clippy/fmt/test sweep
    is owned by the tester gate (T_FINAL_PER_SYMBOL → V6
    `cargo test --workspace --all-targets`) and is left for that
    agent to execute and merge into its report. Per AGENT.md
    process discipline, T_FINAL_PER_SYMBOL is tester-only;
    developer must not tick it.

  - Run `bash scripts/verify_anchors.sh`. Expect output line
    `ANCHORS PASS  (11 / 11)`. Capture the exact stdout into the
    honest-tick block.
  - If a single anchor FAILs, do NOT tick T1107. Route
    `HANDOFF → architect` with the diff (run
    `python3 scripts/hash_report.py spec/reports/<report>.md`
    against the failing anchor's source report and surface the
    body byte-diff). Q7 says zero anchors should drift; a drift
    means either a renderer regression (route to architect) or a
    grep miss (route to architect for re-investigation).
  - Run `cargo test --workspace --all-targets` and confirm zero
    failures. The 5 operator-success-reports invariants (T802,
    T805, T806, T809, T810) and the 11 live-cockpit-unified
    invariants (T901, T903a-d, T905, T906–T908, T910, T911,
    T912) all remain green via their existing test files
    (verified by `cargo test --workspace --all-targets` PASS) —
    the per-symbol writer flip is line-edit-only inside
    `post_fill`'s body and changes no public surface.
  - Run `cargo clippy --workspace --all-targets --all-features
    -- -D warnings` clean.
  - Run `cargo fmt --all -- --check` clean.
  - **No new test code** — this task is a meta-gate that runs
    existing tests + the anchor verifier.
  _acceptance: all four commands pass with zero failures;
  honest-tick block captures the verbatim stdout of
  `verify_anchors.sh` and a "0 failures" summary from
  `cargo test --workspace --all-targets`._
  **[deps: T1102, T1104, T1105, T1106]**

## Tester-final gate

- [x] **T_FINAL_PER_SYMBOL** [tester] — End-to-end gate.
  Tester-only. Per AGENT.md process discipline: developer NEVER
  ticks `T_FINAL_*`.

  **Citation (tester, 2026-05-03):**
  - **test report:** `spec/archive/test-2026-05-03-0803-per-symbol-position-accounts-final.md (archived; see spec/archive/README.md)`
  - **gate cmds (all PASS):**
    - `cargo fmt --all -- --check` → clean (zero diff)
    - `cargo build --workspace --all-targets` → `Finished dev profile [unoptimized + debuginfo] target(s) in 23.92s` (zero warnings)
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings` → `Finished dev profile [unoptimized + debuginfo] target(s) in 1.01s` (clean)
    - `cargo test --workspace --all-targets` → ~641 passed across 85 binaries; 0 failed
    - `cargo test --workspace --doc` → all doc-test runners report `0 passed; 0 failed` (no doc-tests in this workspace)
    - `cargo build -p agent --features in_process_cron` → `Finished dev profile [unoptimized + debuginfo] target(s) in 11.51s` (Inv-T810 clean)
    - `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (re-verified pre- and post-test sweep; identical output both times)
  - **V1–V8 verification matrix:** all 8 VERIFIED (see report § 8).
    - V1 — `t1105_v1_post_fill_writes_per_symbol_account` PASS
    - V2 — `t1105_v2_legacy_row_readable_after_migration` PASS
    - V3 — `t1106_v3_mixed_ledger_correct_open_positions` PASS
    - V4 — `verify_anchors.sh` 11/11 PASS
    - V5 — `t1105_v5_balance_invariant_pre_and_post_migration` PASS
    - V6 — `cargo test --workspace --all-targets` 0 failures (5 operator-success-reports invariants + 11 live-cockpit-unified invariants)
    - V7 — `t1106_v7_two_reads_byte_identical` PASS
    - V8 — `t1105_v8_universe_coverage` PASS
  - **operator-success-reports + live-cockpit-unified invariants:** all 16 VERIFIED (see report § 9). Inv-T802 / T805 / T806 / T809 / T810 / T901 / T902 / T903a-d / T905 / T906–T908 / T910 / T911 / T912 — every one of them tied to a green test in the workspace sweep.
  - **anchor stability:** `spec/anchors.toml` unchanged at 11 entries. Two consecutive `verify_anchors.sh` runs (pre-test sweep + post-test sweep) returned `ANCHORS PASS (11 / 11)` with identical SHAs.
  - **scope discipline:** verification-only. No production code touched. Sole spec edits: this T_FINAL_PER_SYMBOL tick + citation; status frontmatter bump in this file and `spec/per-symbol-position-accounts/feature.md`; new test report under `spec/<slug>/reports/`.

  Fans out into the standard `rust-validate` + `rust-test` +
  `verify-anchors` parallel skill calls and merges into one
  report at
  `spec/reports/test-<timestamp>-per-symbol-position-accounts.md`.
  The report's verification matrix MUST cover all 8 V-items + the
  11/11 anchor gate + the 5 operator-success-reports invariants
  + the 11 live-cockpit-unified invariants.

  | Gate | Test |
  |------|------|
  | V1 writer correctness | `cargo test -p audit --test per_symbol_post_fill -- t1105_v1_post_fill_writes_per_symbol_account` |
  | V2 legacy row readable | `cargo test -p audit --test per_symbol_post_fill -- t1105_v2_legacy_row_readable_after_migration` |
  | V3 mixed-ledger reader | `cargo test -p reports --test open_positions_mixed_ledger -- t1106_v3_mixed_ledger_correct_open_positions` |
  | V4 anchor regression | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` |
  | V5 reconciliation invariant | `cargo test -p audit --test per_symbol_post_fill -- t1105_v5_balance_invariant_pre_and_post_migration` |
  | V6 operator-success-reports + live-cockpit invariants | `cargo test --workspace --all-targets` (zero failures) |
  | V7 determinism | `cargo test -p reports --test open_positions_mixed_ledger -- t1106_v7_two_reads_byte_identical` |
  | V8 universe coverage | `cargo test -p audit --test per_symbol_post_fill -- t1105_v8_universe_coverage` |
  | Inv-T802 | `post_fill` signature scan — `grep "pub async fn post_fill" crates/audit/src/journal.rs` (single match, unchanged) |
  | Inv-T805 | existing `feed_reconnect_test` green |
  | Inv-T806 | existing `uptime_intervals_test` green |
  | Inv-T809 | existing `kill_switch_dual_write_test` green |
  | Inv-T810 | `cargo build -p agent --features in_process_cron` clean |
  | Inv-anchors | gate (V4 above) |

  - On any FAIL, route `HANDOFF → developer` (or `→ architect` if
    a 9 v0/v0.5/v1/v1.5a anchor drifts — that points at a
    rendering side-effect the architect must reconcile, since
    `crates/backtest/` does not call `post_fill` per Q7's
    re-verification).
  - On full PASS, bump the feature file's `status:` from
    `in-progress` to `shipped` and tick this row.
  _acceptance: tester's report template populated with all 8
  V-items + 11 / 11 anchor gate + 5 operator-success-reports +
  11 live-cockpit-unified invariants; status flips
  in-progress → shipped._
  **[deps: T1101, T1102, T1103, T1104, T1105, T1106, T1107]**

## Parallelism map

```
                ┌──────┐
                │T1101 │  migration 006 (CRITICAL PATH GATE)
                │ SQL  │
                └───┬──┘
                    │
        ┌───────────┼─────────────┐
        ▼           ▼             ▼
    ┌──────┐    ┌──────┐      ┌──────┐
    │T1102 │    │T1103 │      │T1104 │  fixture extension
    │writer│    │ dep  │      │      │
    │ + Q4 │    │ recat│      └──┬───┘
    └───┬──┘    └──┬───┘         │
        │          │             │
        │          │ (T1103      │
        │          │  unblocked  │
        │          │  by T1101;  │
        │          │  no down-   │
        │          │  stream     │
        │          │  deps)      │
        │          │             │
        ├──────────┴─────────────┤
        ▼                        ▼
    ┌──────┐                 ┌──────┐
    │T1105 │                 │T1106 │  V3 + V7
    │V1+V2 │                 │      │
    │+V5+V8│                 └──┬───┘
    └───┬──┘                    │
        │                       │
        └────────────┬──────────┘
                     │
                  ┌──▼───┐
                  │T1107 │  anchor sweep
                  │  V4  │
                  └───┬──┘
                      │
                ┌─────▼────────────┐
                │T_FINAL_PER_SYMBOL│  [tester]
                │ V1–V8 + anchors  │
                │ + 16 invariants  │
                └──────────────────┘
```

**Sync points** (tasks below the line block on tasks above):

1. **T1101** is the critical-path gate. The migration MUST land
   first because:
   - T1102's writer flip would FK-fail without the per-pair
     account rows.
   - T1104's fixture extension calls `post_fill` for the SOLUSDT
     row (would also FK-fail).
   - T1105 + T1106's tests bootstrap a ledger via `Ledger::open`,
     which auto-applies all migrations; without 006, V1 / V8
     would not see the per-pair rows.

2. **After T1101**: T1102, T1103, T1104 fan out **in parallel**.
   - T1102 touches `journal.rs` + `query.rs` (3 reader call
     sites, all in one file).
   - T1103 touches `bootstrap.rs:65` (one attribute add).
   - T1104 touches
     `build_ledger_with_open_positions_7d.rs` (one new
     function appended; existing function unchanged).
   No file overlap; no conflict.

3. **After T1102 + T1104**: T1105 + T1106 fan out **in
   parallel**. T1105 needs T1102 (for V1 to assert the new
   writer's output) but NOT T1104 (V1 / V2 / V5 / V8 don't read
   the fixture). T1106 needs both T1102 (for the reader Q4
   cross-check) and T1104 (for the mixed fixture).

4. **After T1102 + T1104 + T1105 + T1106**: T1107 (anchor sweep).
   Independent of T1105 / T1106 from a code-touch perspective
   (T1107 runs no test code of its own); sequenced after the
   writer flip because that is the only Wave-2 task that COULD
   regress anchors (Q7 says it won't — V4 verifies).

5. **T_FINAL_PER_SYMBOL** is sequential — single tester agent
   merges the verification matrix.

**Parallel-safe boundary check:**

| Pair | Files touched (left) | Files touched (right) | Conflict? |
|------|----------------------|------------------------|-----------|
| T1102 ‖ T1103 | `crates/audit/src/journal.rs:82,135` + `crates/audit/src/query.rs::{open_positions_at, pnl_by_symbol, recent_fills}` | `crates/audit/src/bootstrap.rs:65` | NO |
| T1102 ‖ T1104 | `crates/audit/src/{journal.rs, query.rs}` | `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs` | NO |
| T1103 ‖ T1104 | `crates/audit/src/bootstrap.rs:65` | `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs` | NO |
| T1105 ‖ T1106 | `crates/audit/tests/per_symbol_post_fill.rs` (NEW) | `crates/reports/tests/open_positions_mixed_ledger.rs` (NEW) | NO |
| T1107 vs others | None (read-only meta-gate) | All | NO |

Three Wave-2 tasks fan out cleanly: 3-way developer parallelism
on Wave 2 (T1102 ‖ T1103 ‖ T1104), then 2-way on Wave 3
(T1105 ‖ T1106), single agent for Wave 4 (T1107) and the tester
gate.

**Wave summary:**

- Wave 1: T1101 — single developer, ½ day. Migration `006`.
- Wave 2: T1102 ‖ T1103 ‖ T1104 — three developers in
  parallel, ½ day each. Writer flip + Q4 cross-check + reader
  doc-comment, deprecation attribute, fixture extension.
- Wave 3: T1105 ‖ T1106 — two developers in parallel, ½ day
  each. V1+V2+V5+V8 tests, V3+V7 tests.
- Wave 4: T1107 — single developer, ¼ day. Anchor sweep
  meta-gate + workspace test sweep.
- Tester: T_FINAL_PER_SYMBOL — single tester agent, fans out
  into rust-validate + rust-test + verify-anchors parallel
  skill calls.

**Total duration estimate:** ~2 days wall-clock if Wave 2 fans
out fully (3 developers in parallel); ~3 days sequential. The
real-mtm-unrealized-pnl precedent suggests the tester gate adds
~½ day for the report write-up.

## Notes

- **Migration cadence.** The migration list at
  [spec/architecture.md → Audit migration list — current](../architecture.md#audit-migration-list--current)
  reserves `006` for the **conditional** real-mtm follow-up
  `006_open_positions_index.sql` ("lands ONLY if V8 perf gate
  fails"). Per the real-mtm tester report (PASS at 0.287ms vs
  100ms budget), that conditional migration was **not** locked —
  the slot is free. This feature claims `006` as
  `006_per_symbol_position_accounts.sql`. The architecture-doc
  update under deliverable 3 reflects this.
- **Universe drift.** R11 + V8 are the operational guard.
  Adding a symbol to a strategy config without seeding
  `assets:position:<SYMBOL>` would FK-fail at first fill in
  paper / live. V8 catches this at PR time rather than at
  production-issue time. A future feature may mechanize the
  universe → migration generation (e.g. a `cargo xtask
  generate-universe-migration`) — out of scope here.
- **Backfill is irreversible.** Q3's "purely additive" is the
  correct posture forever for an append-only audit DB. Even if a
  future operator cares about the structural attribution of
  legacy rows, the right move is a derived view (e.g. a
  materialized `journal_entries_normalized` table built via a
  read-side projection), not a destructive rewrite.
- **`#[deprecated]` clippy interaction.** T1103's deprecation
  warning is silent because the function has zero callers. If
  the grep surfaces a hidden caller, T1103 routes back to
  architect (the deprecation message says "zero callers, will
  be removed" — a hidden caller invalidates that claim and
  reopens Q8).

## Changelog

- 2026-05-03 (tester): T_FINAL_PER_SYMBOL ticked. All gates green —
  fmt clean, build clean (23.92s, zero warnings), clippy clean,
  `cargo test --workspace --all-targets` ~641 passed / 0 failed
  across 85 binaries, doc tests clean, `cargo build -p agent --features
  in_process_cron` clean, anchors PASS 11/11 (re-verified pre- and
  post-tests). V1–V8 all VERIFIED; 16 operator-success-reports +
  live-cockpit-unified invariants all VERIFIED. T1102 reader
  cross-check confirmed warn-only with description-parse primary;
  T1103 deprecation confirmed silent (zero callers across workspace);
  T1104 lowercase Buy/buy capitalization fix confirmed; T1107 anchor
  stability re-verified twice. Test report at
  `spec/archive/test-2026-05-03-0803-per-symbol-position-accounts-final.md (archived; see spec/archive/README.md)`.
  Status bumped in-progress → shipped on this task file and on the
  feature file. `VERDICT → PASS`.
- 2026-05-01 (developer, T1107 close-out): anchor regression sweep
  green (11 / 11 PASS). Ticked T1101–T1107 with citation blocks.
  HANDOFF → tester for T_FINAL_PER_SYMBOL.
- 2026-05-03 (architect): tasks file landed. T1101–T1107 +
  T_FINAL_PER_SYMBOL filed against the analyst's R/V items + the
  architect's Q1–Q8 design resolutions. Parallelism map shows
  three Wave-2 tasks fanning out cleanly (T1102 ‖ T1103 ‖ T1104),
  two Wave-3 tasks (T1105 ‖ T1106), single Wave-4 (T1107) and a
  tester gate to finish.
