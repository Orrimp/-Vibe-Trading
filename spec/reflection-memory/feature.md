---
slug: reflection-memory
status: in-progress
owner: analyst
updated: 2026-05-08
version: 1.8.0
---

# Reflection memory

## Why

This brief promotes the **Reflection memory** queue item from
[backlog.md → Active](../backlog.md#active) into a real feature. It
replaces the fixed placeholder body that
[`crates/reports/src/render/memory_highlights.rs`](../../crates/reports/src/render/memory_highlights.rs)
emits today (`_reflection memory not yet implemented._`) with **real
lesson-card output** drawn from a per-trade reflection memory and a
top-K retrieval over it.

The contract this feature inherits is set by three documents:

1. [`spec/operator-success-reports/feature.md` → R6 — Memory highlights
   — placeholder](../operator-success-reports/feature.md#r6--memory-highlights--placeholder)
   ships the `## Memory highlights` section header in every operator
   success report with a deterministic placeholder body, with the
   explicit "(v1+, once the memory loop runs)" caveat. Two body-only
   SHA-256 anchors (`report-sample-7d`, `report-sample-90d`) in
   [`spec/anchors.toml`](../anchors.toml) lines 67–75 are locked over
   that placeholder.
2. [`spec/dev-notes/memory-anchor-relock-TBD.md`](../dev-notes/memory-anchor-relock-TBD.md)
   is a forward-compat breadcrumb that names the **anchor re-lock
   pattern** (the same precedent v1.5a applied to the top10-momentum
   anchors at task **T717** of
   [`spec/v15a-mean-reversion-pairs/tasks.md`](../v15a-mean-reversion-pairs/tasks.md)).
   That pattern is part of this feature's deliverable; without re-lock,
   the determinism gate FAILs on the first run after the placeholder
   body changes.
3. [`spec/product.md` → Memory & continual learning](../product.md#memory--continual-learning)
   (lines 262–273) names the four-layer shape of the moat bet:
   episodic memory, reflection loop, retrieval at decision time,
   periodic distillation. It also says — at line 539 — the operator
   report should carry **"Memory highlights — top lesson cards the
   trader retrieved this period"**.

**Terms-of-art (one-line glosses, used throughout):**

- **Lesson card** — a small, structured record produced after a closed
  trade that captures *what happened* and *what to learn from it*
  (symbol/strategy, signed P&L, holding period, regime tag, optional
  natural-language note).
- **Vector store** — a database that indexes records by an embedding
  vector so similarity search ("find the K most relevant past lessons
  for *this* situation") is fast. Candidates: `qdrant` (separate
  process, mature, more deps) vs `SQLite + sqlite-vss` (in-process,
  fewer deps, smaller community).
- **Body-SHA-256** — the deterministic body-only hash of an operator
  success report, locked into the regression gate per
  [operator-success-reports R10.3](../operator-success-reports/feature.md#r10--determinism-body-vs-front-matter-discipline).
- **Anchor re-lock** — the controlled procedure to capture the new
  body-SHA-256s and replace the v1+ entries in
  [`spec/anchors.toml`](../anchors.toml) once the placeholder body
  changes. Precedent: v1.5a T717.

**Scope decision: deterministic v1, LLM enrichment deferred.** The
agent has shipped four strategies entirely without LLM tokens; the
cost ledger's `expense:llm:*` accounts sit at `$0.00` against a
`$135/mo` cap per
[architecture.md → Cost telemetry](../architecture.md#cost-telemetry--dedicated-cost-crate--confirmed-2026-04-17),
and **v2 LLM strategy is queued but NOT yet shipped** (see
[backlog.md → Queue / Strategy](../backlog.md#queue)). This brief
picks the **deterministic per-trade lesson-card generator** as the
v1 of reflection memory; an **LLM-enriched `post_mortem_analyst`
follow-up** is surfaced as Q1 for the operator. Justification:
independence from v2 LLM ship; deterministic body-SHA-256
compatibility (seeded LLM sampling is a larger design question);
composability (the LLM v2 wraps `enrich(card) -> String` around the
v1 card without unbuilding); retrieval-at-decision-time stays out
of the trader (Q4) so the 9 strategy-backtest anchors don't move;
distillation deferred (Q5) since it needs cards on disk to cluster.

This feature is **non-strategy and (in v1) non-LLM**. No edge claim,
no trading-logic change, no impact on the 9 locked strategy-backtest
anchor SHA-256s. The **only** existing anchor SHAs that change are
the two `report-sample-*` v1+ operator-success-report anchors,
which are re-locked as the last step before VERDICT → PASS. Success
is "the report's `## Memory highlights` section now shows real
lesson cards retrieved from real closed trades, the reconciliation
invariant from operator-success-reports R11 still holds, and the
determinism gate from R10.3 holds against the **new** body hash."

This feature is **the moat made queryable** — the first iteration
of the persistent reflection memory the
[product.md → Differentiator](../product.md#differentiator) calls
out. After this feature ships, the operator can ask "what did the
agent learn from the last 7 days of trading?" and the report
answers from artefacts the agent itself produced after each trade.

## Requirements

Numbered, testable, derived from
[product.md → Memory & continual learning](../product.md#memory--continual-learning),
[product.md → What every report contains](../product.md#what-every-report-contains)
(line 539, "Memory highlights — top lesson cards the trader retrieved
this period"),
[operator-success-reports → R6](../operator-success-reports/feature.md#r6--memory-highlights--placeholder),
and the existing read-only audit query surface in
[`crates/audit/src/query.rs`](../../crates/audit/src/query.rs). Each
ends with a one-line **acceptance** the tester can verify. All
requirements preserve the `Strategy` trait shape (no trait changes),
the audit chart of accounts (no new accounts), the `strategy_events`
schema (additive only — see Q3), the public `audit::query::*` surface
(additive only), and the 9 locked strategy-backtest anchor SHA-256s
(no impact). This feature **adds no new bus channels** and consumes
**no LLM tokens** in v1 (Q1).

### R1 — Lesson-card data model

- **R1.1** A **lesson card** is a typed record produced when a trade
  *closes* (position size returns to zero on a previously open
  symbol/pair). v1 lesson-card fields are deterministic-only:
  - `card_id: [u8; 32]` — content-hash of the card's deterministic
    fields (so the same closed trade in the same fixture produces the
    same `card_id` byte-for-byte).
  - `closed_at: Timestamp` — RFC3339 UTC, the trade's `closed_ts` from
    the audit ledger.
  - `symbol_or_pair: Symbol | PairKey` — single-leg trades carry
    `Symbol`; v1.5a pair-switch trades carry `PairKey` (the
    [`PairKey`](../../crates/core/src/pair.rs) v1.5a type).
  - `strategy_id: StrategyId` — the v1+ `journal_transactions.strategy_id`
    column (added at v1+ T802 — see
    [operator-success-reports Q2](../operator-success-reports/feature.md#q2--pnl_by_strategy-query-design--schema-migration)).
    NULL maps to `(unattributed)` per the v1+ convention.
  - `signed_pnl_usdt: Money<Usdt>` — realized P&L for the closed
    trade. Source: `audit::query::realized_pnl_for_trade(trade_id)`
    (additive query — see R2.2).
  - `holding_period_bars: u32` — bar count between open and close at
    1m cadence; for v1.5a pair-switches, "open" is the leg-rotation
    boundary.
  - `entry_regime: RegimeTag` — see R1.3.
  - `exit_regime: RegimeTag` — see R1.3.
  - `outcome_class: OutcomeClass` — `Win | Loss | Scratch` per
    R1.4.
  - `note: Option<String>` — left **`None` in v1**; reserved for the
    LLM v2 (Q1).
- **R1.2** Cards are **immutable once written**. No update path; a
  reflection-memory bug surfaces as a *new* card with a different
  `card_id`, never as an in-place edit. Same append-only discipline
  as the audit ledger
  ([architecture.md → Audit & ledger](../architecture.md#audit--ledger)).
- **R1.3** **Regime tag** is a deterministic classification of market
  state at a moment in time. v1's classifier is a **simple rule** —
  the architect's call (Q3) but the analyst's strawman is:
  - `Bull` — 7d BTC return > +2%.
  - `Bear` — 7d BTC return < −2%.
  - `Chop` — otherwise.
  Pure-function over BTC close prices the agent already reads
  (`data/binance/BTCUSDT/...`). No LLM, no ML. v2 with LLM may
  promote this to a richer regime classifier (Q1).
- **R1.4** **Outcome class** is a deterministic three-way bucketing of
  `signed_pnl_usdt`:
  - `Win` if `signed_pnl_usdt > +0.5%` of opening capital at trade
    open (taker-fee-aware threshold so a 1bp win after fees is not
    counted as a Win).
  - `Loss` if `signed_pnl_usdt < −0.5%`.
  - `Scratch` otherwise.
  The `0.5%` threshold is analyst's pick; architect may override
  (Q3).
- **Acceptance:** a unit test against a fixture of closed trades
  asserts each emitted card has all R1.1 fields populated, that
  `card_id` is byte-stable across two runs over the same fixture,
  and that `Outcome::Scratch` cards do not appear with
  `|signed_pnl_usdt|` greater than the threshold.

### R2 — Card persistence layer

- **R2.1** Lesson cards are written to a **persistent store** keyed
  by `card_id`, queryable by `(strategy_id, symbol_or_pair,
  closed_at_range, outcome_class, regime_tag)`. The store choice
  (qdrant vs sqlite-vss vs new SQLite table inside the audit DB) is
  **architect's call** (Q2 + Q3).
- **R2.2** A new **read-only audit query** surfaces realized P&L for
  a single closed trade so R1.1's `signed_pnl_usdt` field can be
  computed deterministically:
  ```
  audit::query::realized_pnl_for_trade(
      ledger: &Ledger,
      trade_id: TradeId,
  ) -> Result<Money<Usdt>, LedgerError>
  ```
  Additive sibling of `realized_pnl_since` and `pnl_by_strategy`
  (lines 37 + the v1+ T803 addition in
  [`crates/audit/src/query.rs`](../../crates/audit/src/query.rs)).
  Same Decimal-in / Decimal-out / no-`sqlx`-types contract.
- **R2.3** The card **writer** is the `post_mortem_analyst` module
  (the product-side name for what v1 actually ships as a
  deterministic generator; the name is preserved so the LLM v2 can
  swap the implementation without renaming the consumer). It runs
  **on each trade-close event** off the agent's main loop. v1's
  generator is **pure over its inputs** — same closed-trade fixture
  in, same card out.
- **R2.4** The store carries an **idempotency contract**: writing the
  same `card_id` twice is a no-op. Same append-only discipline as
  the journal ledger.
- **R2.5** Cards are **embedded** at write time so retrieval is
  similarity-based (R3.1). v1's embedding is a **deterministic,
  non-LLM** vector — the analyst's strawman is a 32-dim hand-crafted
  feature vector encoding `(strategy_one_hot, regime_one_hot,
  outcome_one_hot, log_holding_period, log_pnl_magnitude,
  signed_pnl_sign, ... )`. **Architect's call** (Q3) on the exact
  feature schema; the analyst's prior is "no LLM embeddings in v1".
- **Acceptance:** an integration test against a fixture audit ledger
  with N=10 deliberate closed trades asserts (a) exactly N cards
  land in the store, (b) `card_id`s are unique, (c) writing the same
  N cards a second time produces 0 inserts (R2.4 idempotency),
  (d) the new audit query in R2.2 returns the correct
  `signed_pnl_usdt` for each `trade_id`.

### R3 — Retrieval API

- **R3.1** A new top-K retrieval API:
  ```
  reflection::retrieve_top_k(
      store:  &dyn ReflectionStore,
      query:  &RetrievalQuery,
      k:      usize,
  ) -> Result<Vec<LessonCard>, RetrievalError>
  ```
  where `RetrievalQuery` carries the *current context* the trader (or
  v1 the report) has on hand: `(strategy_id, symbol_or_pair,
  current_regime_tag, optional time-window)`. Returns the K cards
  most relevant to that context, ordered by similarity score
  descending. Tie-break on `closed_at` ascending (older cards first
  — they have proven longer-run signal) for deterministic ordering.
- **R3.2** **Retrieval is deterministic** under the v1 deterministic
  embedding — same store + same query + same K → same K cards in the
  same order, byte-stable across runs. This is load-bearing for the
  body-SHA-256 lock (R5).
- **R3.3** Retrieval is **read-only** over the store; it never
  mutates card content. (No "card promotion" semantics in v1; that
  belongs to the deferred distillation layer — Q5.)
- **R3.4** **Empty-store / fresh-ledger graceful path:** if the
  store has zero cards (a fresh ledger with zero closed trades),
  `retrieve_top_k` returns `Ok(vec![])`. The renderer (R4) then
  emits the empty-state body string (Q7).
- **Acceptance:** a unit test seeds the store with 100 deterministic
  cards, runs `retrieve_top_k` against a fixed query at K=5 twice,
  asserts the same five cards in the same order both times; a second
  test asserts retrieval against an empty store returns `Ok(vec![])`.

### R4 — Report integration (R6 of operator-success-reports)

- **R4.1** The R6 placeholder body in
  [`crates/reports/src/render/memory_highlights.rs`](../../crates/reports/src/render/memory_highlights.rs)
  is **replaced** with a `render_with_lessons(lessons: &[LessonCard])`
  entry point. The previous `render()` and `render_with_decay()`
  functions remain (the strategy-decay heuristic from
  operator-success-reports T811 is **not** re-scoped here — see Q6);
  `render_with_lessons` composes with `render_with_decay` so a
  decayed strategy is still flagged in the same section.
- **R4.2** The body shape rendered by `render_with_lessons` is:
  ```
  ## Memory highlights

  Top N lesson cards retrieved this period:

  - 2026-04-22 [Win] sma_crossover BTCUSDT regime=Bull held=42 bars pnl=+$123.45
  - 2026-04-19 [Loss] pairs_mr_h1 BTCUSDT-ETHUSDT regime=Chop held=8 bars pnl=-$67.89
  - …

  decay candidates: <strategy_id_csv>   # only if decay_fired
  ```
  - `N` = `K` from R3.1, default `5` (architect may pin a different
    K — Q3).
  - Card lines are **deterministic** byte-for-byte over the same
    retrieval result — no timestamps from wall-clock, no run-id
    leakage. Same body-vs-front-matter discipline as
    [operator-success-reports R10.2](../operator-success-reports/feature.md#r10--determinism-body-vs-front-matter-discipline).
- **R4.3** **Retrieval query construction at report time**:
  - `strategy_id` = the strategy with the largest absolute P&L this
    period (analyst's strawman; architect may pick a different scoping
    rule — Q3). Tie-break on lex-sorted strategy_id ascending for
    determinism.
  - `symbol_or_pair` = the symbol with the largest absolute P&L
    under that strategy this period.
  - `current_regime_tag` = R1.3 classifier evaluated at
    `period_end`.
  - `time-window` = unbounded (retrieve over all history).
- **R4.4** **Empty-state graceful path** (R3.4): on a fresh ledger
  with zero closed trades, the body renders the literal string
  ```
  ## Memory highlights

  _no closed trades yet — lesson cards will appear after the first
  closed trade._
  ```
  — **byte-stable** across runs (Q7 finalises wording).
- **R4.5** **No new front-matter fields** are added in v1. The
  current 12-field front-matter set established by
  [operator-success-reports Q7](../operator-success-reports/feature.md#q-resolution-summary)
  stays as-is. (LLM v2 will add `llm_model`, `llm_tokens_consumed`,
  etc. — Q1.)
- **Acceptance:** a unit test against a fixture store seeded with
  K=5 deterministic cards asserts the rendered body matches a
  hand-computed expected string byte-for-byte; a second test against
  an empty store asserts the R4.4 empty-state string is rendered
  byte-for-byte.

### R5 — Determinism + body-SHA-256 anchor re-lock

- **R5.1** **Body-only determinism preserved.** The two
  `report-sample-7d` and `report-sample-90d` fixture scenarios from
  [operator-success-reports → Backtest Scenarios](../operator-success-reports/feature.md#backtest-scenarios)
  re-run against the new fixtures (which now include closed trades
  so cards are produced) and produce **byte-identical bodies across
  two runs** at seed `0xC0FFEE`. Same R10.3 contract from
  operator-success-reports.
- **R5.2** **Anchor re-lock as the last gate before VERDICT → PASS.**
  Per the breadcrumb at
  [`spec/dev-notes/memory-anchor-relock-TBD.md`](../dev-notes/memory-anchor-relock-TBD.md),
  the v1+ entries at
  [`spec/anchors.toml`](../anchors.toml) lines 67–75 are **replaced**
  with the new body-SHA-256s captured by the tester at the first
  successful run after the placeholder body changes. The 9
  v0/v0.5/v1/v1.5a strategy-backtest anchors are **NOT touched** —
  Memory highlights is a report-only section, and the strategy
  backtest reports do not render it (Q6).
- **R5.3** **Determinism negative invariant continues to hold.**
  The R10.4 substring-absence test from operator-success-reports
  passes on the new body — no new body bytes contain `generated:`,
  `run_id:`, `wall_clock_s:`, `ledger_snapshot_sha:`, or
  `data_source:`.
- **R5.4** **Re-lock procedure** — the architect's design step
  enumerates exactly the sequence in
  [`spec/dev-notes/memory-anchor-relock-TBD.md` → "What the eventual
  architect must do"](../dev-notes/memory-anchor-relock-TBD.md#what-the-eventual-architect-must-do).
  Tester executes that sequence as the final task of the feature.
- **Acceptance:** the two scenarios re-run, the tester captures the
  new body-SHA-256s, the values are written into
  [`spec/anchors.toml`](../anchors.toml) replacing the v1+ entries,
  `scripts/verify_anchors.sh` returns `ANCHORS PASS (11 / 11)`, and
  the determinism + reconciliation gates remain green.

### R6 — Reconciliation invariant preserved

- **R6.1** The reconciliation invariant from
  [operator-success-reports → R11](../operator-success-reports/feature.md#r11--reconciliation-invariant)
  continues to hold. Lesson cards are **derived artefacts**, not
  ledger events; they sit *outside* the chart-of-accounts identity
  `cash + Σ positions = equity`. No card P&L surfaces in the
  Reconciliation appendix; cards are presentation-layer over the
  ledger, not the ledger itself.
- **R6.2** The card-write path **does not write to
  `journal_transactions`** — the audit ledger remains the canonical
  P&L source of truth, and reflection memory is a read-only consumer
  of it. (Architect may revisit this if Q3 lands "store cards in a
  new audit DB table" — but even then the cards table is *additive*
  and outside the chart of accounts.)
- **Acceptance:** the existing R11 reconciliation tests in the
  reports crate continue to pass byte-for-byte; no new chart-of-
  accounts entry appears.

### R7 — Performance + memory budget

- **R7.1** Card writes happen **off the trade-decision hot path** —
  asynchronously, after the trade-close event. Adding a card MUST
  NOT add measurable latency to the executor's submit-fill path.
  Architect's call on the exact channel (a tokio mpsc to a writer
  task is the analyst's strawman — Q8).
- **R7.2** Top-K retrieval at report time **completes in < 100ms**
  on the 1-year fixture ledger (which after this feature has on the
  order of 100–500 closed trades, hence ≤500 cards). v1 uses the
  small-N retrieval path; v2 with real LLM embeddings may need a
  proper ANN index — that's a follow-up.
- **R7.3** **Total report wall-clock budget unchanged** — the report
  binary still completes `< 10s` on the 1-year fixture per
  [operator-success-reports R13](../operator-success-reports/feature.md#r13--performance).
- **R7.4** **Memory ceiling unchanged** — RSS stays `< 256 MiB` on
  the 1-year fixture per the same R13.3.
- **Acceptance:** a perf-smoke test asserts the report wall-clock
  budget still passes after this feature ships, and a separate
  micro-benchmark asserts retrieval at K=5 against a 500-card store
  completes in `< 100ms`.

### R8 — No regression in non-report code paths

- **R8.1** **No `Strategy` trait change.** The trader does not
  consume retrieval in v1 (Q4). Wiring retrieval into the trader is
  a follow-up brief.
- **R8.2** **No change to the 9 locked strategy-backtest anchor
  SHA-256s.** The strategy backtest reports do not render the
  Memory highlights section, so their bodies are unaffected (Q6).
- **R8.3** **No new bus channels** beyond the internal mpsc the
  card-writer task may use (Q8); no UI strings (the cockpit's
  `viewer` already renders the report markdown inline — same
  contract as
  [operator-success-reports → Handoff contract — no UI involvement](../operator-success-reports/tasks.md)).
- **R8.4** **No LLM token consumption in v1.** The cost ledger's
  `expense:llm:*` accounts remain at `$0.00` against the `$135/mo`
  cap. The report's System Health line still reads
  `LLM spend: $0.00 / $135` (Q1 deferral; v2 LLM `post_mortem_analyst`
  changes this).
- **Acceptance:** the existing 9 strategy-backtest anchor SHAs from
  [`spec/anchors.toml`](../anchors.toml) lines 15–58 stay
  byte-identical; `cargo test --workspace --all-targets` is green;
  no new dependency on an LLM provider crate is introduced.

## Backtest Scenarios

This feature is **non-strategy** — it does not validate edge. The
scenarios below are **report scenarios**, the same two from
[operator-success-reports → Backtest Scenarios](../operator-success-reports/feature.md#backtest-scenarios)
re-run after the R6 body changes, with **new fixture content** that
includes closed trades so lesson cards are produced.

### Scenario: `report-sample-7d` (re-locked)

- **Fixture ledger:** the existing
  `crates/reports/tests/fixtures/build_ledger_7d.rs` builder is
  **extended** so the 7-day window contains ≥3 closed trades across
  ≥2 strategies, producing ≥3 lesson cards. Architect to decide
  exact mix (Q3).
- **Period:** `--period 7d`.
- **Seed:** `0xC0FFEE`.
- **Expected output:** body-SHA256 captured by tester at first
  successful run; **replaces** the `ab06dbcb…` v1+ entry in
  [`spec/anchors.toml`](../anchors.toml).

### Scenario: `report-sample-90d` (re-locked)

- **Fixture ledger:** `build_ledger_90d.rs` extended so the 90-day
  window contains ≥10 closed trades across ≥3 strategies, exercising
  Win + Loss + Scratch outcomes and Bull + Bear + Chop regimes
  across the cards.
- **Period:** `--period 90d`.
- **Seed:** `0xC0FFEE`.
- **Expected output:** body-SHA256 captured by tester at first
  successful run; **replaces** the `2ef403f1…` v1+ entry in
  [`spec/anchors.toml`](../anchors.toml).

**Both scenarios re-lock under the same R10.3 determinism gate that
operator-success-reports established.** The 9 strategy-backtest
anchors are **not** in scope (V6).

## Verification

The tester's contract for declaring this feature done. All items
must be green before VERDICT → PASS. Mapping to R-numbered
requirements is explicit.

- **V1 Static checks pass.** `cargo fmt --check` clean,
  `cargo clippy --workspace --all-targets -- -D warnings` clean,
  `cargo audit` shows no unpatched advisories,
  `cargo deny check` passes. Same gate as v0–v1+ V1.
- **V2 `cargo test --workspace` green.** Zero failures, zero
  unexplained `#[ignore]`. Includes the new test surfaces:
  - **R1** lesson-card data-model tests (deterministic `card_id`,
    field population, outcome thresholding).
  - **R2** card-persistence tests (write idempotency, the new
    `realized_pnl_for_trade` query, embedding determinism).
  - **R3** retrieval tests (top-K determinism + tie-break,
    empty-store path).
  - **R4** report-integration tests (rendered body byte-stability,
    empty-state path, decay-co-render).
  - **R5** body-SHA-256 determinism across two runs against the
    new fixtures.
  - **R6** reconciliation invariant from operator-success-reports
    R11 re-runs green.
  - **R7** retrieval-perf smoke + report wall-clock smoke.
  - **R8** the 9 strategy-backtest anchor SHAs remain
    byte-identical.
- **V3 Both report scenarios run end-to-end.**
  - `report-sample-7d` produces a body containing the new
    `## Memory highlights` content per R4.2 (or the empty-state
    string per R4.4 if the fixture deliberately ships zero closed
    trades — fixture decision belongs to architect, Q3).
  - `report-sample-90d` likewise.
  - Both produce byte-stable bodies across two runs at seed
    `0xC0FFEE` ten seconds apart.
- **V4 Body-only determinism (R5).** Same R10.3 contract from
  operator-success-reports — bodies byte-identical, front-matter
  differs on `generated:`. R5.3 negative-invariant test passes.
- **V5 Reconciliation invariant (R6).** `Δ = $0.00` on every row
  of the Reconciliation appendix in both scenarios. Cards are
  presentation-layer; they do not appear in the appendix.
- **V6 Anchor re-lock**: tester captures new body-SHA-256s for
  the two `report-sample-*` scenarios and **replaces** the v1+
  entries in [`spec/anchors.toml`](../anchors.toml) lines 67–75.
  `scripts/verify_anchors.sh` returns `ANCHORS PASS (11 / 11)`
  with the new SHAs. The 9 strategy-backtest anchor SHAs are
  byte-identical (R8.2).
- **V7 Audit-query API surface preserved.** The existing read-only
  API in [`crates/audit/src/query.rs`](../../crates/audit/src/query.rs)
  is **extended additively only** — the new
  `realized_pnl_for_trade` query (R2.2) is the only addition.
  All v0/v0.5/v1/v1.5a/v1+ queries retain their current shape.
- **V8 Cost telemetry (R8.4).** The reports binary still uses zero
  LLM tokens; the cost ledger's `expense:llm:*` accounts remain
  zero. The rendered report shows `LLM spend: $0.00 / $135` in
  System Health.
- **V9 Performance.** Report wall-clock for `report-sample-90d`
  on the 1-year fixture is `< 10s`. Top-K retrieval at K=5 against
  a 500-card store is `< 100ms`. RSS `< 256 MiB`.
- **V10 No-UI invariant.** Zero new `ui::strings` entries, zero new
  widgets. The cockpit's `viewer` binary renders the new Memory
  highlights body inline without changes.

Failure routing:

- Static / test failure → `developer`.
- Determinism / re-lock failure → `developer` (or `architect` if
  the failure mode reveals a card-shape policy gap).
- Audit-query surface change required (e.g.
  `realized_pnl_for_trade` shape) → `architect`.
- Reflection memory model breaks (e.g. v2 LLM enrichment lands and
  alters card schema) → `analyst` (re-scope this brief; that
  belongs to a follow-up brief, not in-place edits here).

## Notes / Open questions

The analyst defers these decisions to the architect (or to the
operator where flagged). The brief is written so each question can
be answered without reshaping the requirements above.

### Q1 — LLM-driven `post_mortem_analyst` vs deterministic v1

[RESOLVED 2026-05-08 — operator picked **Option A** (deterministic v1) via orchestrator chat. R-items above already assume Option A; no R-item revisions needed. LLM enrichment becomes a follow-up brief after v2 LLM ships.]

[product.md → Memory & continual learning](../product.md#memory--continual-learning)
names the reflection layer as a `post_mortem_analyst` that "writes a
lesson card into a vector store" — implying an LLM. But the agent
ships entirely on `$0.00 expense:llm:*` against a `$135/mo` cap, and
v2 LLM strategy is **queued but not shipped** (see
[backlog.md → Queue / Strategy](../backlog.md#queue)).

**Tradeoff:**
- **Option A (deterministic v1):** ships now, no v2 LLM dependency,
  byte-stable body, anchor re-lock is straightforward. Lesson cards
  carry no natural-language `note` field; retrieval scoring is over
  hand-crafted features. Operator gets institutional memory in the
  report immediately. Follow-up brief later wraps the LLM
  enrichment.
- **Option B (LLM-enabled v1):** richer cards (LLM-written `note`
  summarising "what to learn from this trade"); blocks on v2 LLM
  strategy ship; introduces non-determinism in body bytes
  (architect must solve seeded-sampling or carve LLM prose into
  front-matter); first non-zero `expense:llm:*` ledger row in the
  project's history; cost-telemetry budget gating becomes
  load-bearing for this feature, not just for v2.

[ANALYST-RECOMMENDATION]: **Option A**. Reasons in `## Why` —
independence from v2 LLM ship, deterministic anchor compatibility,
composability (v2 wraps `post_mortem_analyst::enrich(card)` around
the v1 card without unbuilding anything). All R-items above assume
Option A; if the operator picks Option B, R1.1 gains an
`note: String` field, R2.5's embedding becomes an LLM embedding,
and R5.1's body determinism becomes a seeded-sampling contract for
the architect.

### Q2 — Vector store choice: qdrant vs SQLite + sqlite-vss vs new SQLite table

[ARCHITECT-DECIDE]

[product.md line 269](../product.md#memory--continual-learning)
offers "qdrant or SQLite + sqlite-vss". A third option exists:
add a new SQLite table inside the existing audit DB and ship a
naive linear-scan top-K (works fine at v1 scale of ≤500 cards;
ships zero new dependencies).

**Tradeoff:**
- **qdrant** — separate process, mature ANN index, scales to
  millions of vectors. Adds a new daemon to the ops surface (the
  agent is currently a single-process Rust binary; introducing
  qdrant means a second process to supervise, monitor, and
  reconcile shutdown order with). Adds a `qdrant-client` Rust
  dependency.
- **SQLite + sqlite-vss** — in-process via SQLite extension; no
  new daemon; smaller community than qdrant; the audit DB is
  already SQLite, so it's a natural fit.
- **New SQLite table + linear scan** — zero new deps, zero new
  process, smallest surface. v1 has ≤500 cards on the 1-year
  fixture so linear scan over 32-dim vectors is sub-millisecond.
  The "vector store" name in product.md is aspirational; v1 may
  not actually need ANN. v2 with LLM embeddings (768-dim+) and
  100k+ cards might.

[ANALYST-RECOMMENDATION]: **start with the new-table + linear-scan
option** for v1, with a `ReflectionStore` trait so swapping to
qdrant or sqlite-vss in v2 is a single-trait-impl change. Justify
with the v1 scale (≤500 cards × 32 dims is a 16KB hot loop, well
inside L1 cache). Architect may push back if they want to ship
qdrant or sqlite-vss now to avoid a v2 swap.

### Q3 — Lesson-card schema details + storage location + regime classifier + outcome thresholds + retrieval scoping rule

[ARCHITECT-DECIDE]

R1–R4 above leave several detail-level shape questions unresolved.
Bundled here so the architect resolves them as one design slice:

- **Q3a** Storage location: new audit-DB table, new sibling SQLite
  file, or vector-store-only artefact. Analyst's prior: new sibling
  SQLite file (`reflection.db`) under `target/` for tests and the
  configured ledger root in production. Keeps the audit DB schema
  untouched; reflection is a sibling, not a co-tenant. Architect's
  call.
- **Q3b** Regime classifier exact form: the analyst's strawman in
  R1.3 is BTC 7-day return ±2%. Architect may pick a richer
  classifier (volatility regime, BTC-vs-ETH cross-correlation, etc.)
  or push back on BTC-anchored regime as too BTC-centric for an
  altcoin pair like `BNBUSDT-BTCUSDT`.
- **Q3c** Outcome thresholds: R1.4's ±0.5% Win/Loss threshold.
  Architect may pin a different value or make it strategy-specific
  (e.g. mean-reversion strategies have lower per-trade P&L
  variance).
- **Q3d** Embedding dimensions and exact features: R2.5's 32-dim
  hand-crafted vector. Architect names the exact feature set.
- **Q3e** Default K for top-K retrieval at report time: R3.1
  defaults to K=5; architect may pick differently. Note: K affects
  the body byte length, not byte determinism.
- **Q3f** Retrieval-query scoping rule at report time: R4.3 picks
  the strategy-with-largest-abs-PnL as the query strategy. Architect
  may pick "all strategies, retrieve K each, then merge" or "just
  the most-recently-active strategy".
- **Q3g** Fixture content extension: how many cards to seed in the
  re-locked `report-sample-7d` and `report-sample-90d` fixtures.
  Analyst's prior in Backtest Scenarios: ≥3 cards in 7d, ≥10 cards
  in 90d. Architect may pick differently for better Win/Loss/Scratch
  + Bull/Bear/Chop coverage.

[ANALYST-RECOMMENDATION]: ship the analyst's straw choices unless
specific reasons emerge to override; document the choices in the
architect's Design section.

### Q4 — Retrieval at decision time: trader vs report-only

[ARCHITECT-DECIDE]

product.md layer 3: "Retrieval at decision time — trader retrieves
top-K relevant past lessons before composing the order." That's the
canonical reflection-memory loop.

**Tradeoff:**
- **Wire retrieval into the trader/Strategy trait now.** Highest
  product fidelity — the agent really does *use* its memory at
  decision time. But: changes hot-path behavior; could shift
  body-SHA-256s of all 9 strategy backtest anchors (any strategy
  whose decision now consults retrieved cards has a different
  signal series → different fills → different journal rows →
  different body). Re-anchoring 9 backtests is a major scope
  expansion for this feature. Also requires a `Strategy` trait
  change (the trait currently has no retrieval seam); v0 invariant
  preservation matters across the project.
- **Report-only this round.** Smaller scope; only the two `report-
  sample-*` anchors re-lock. Memory highlights surfaces "which past
  lessons would have informed this period's trading" — informative
  for the operator, not yet consumed by the trader. Trader-side
  wiring becomes a follow-up brief.

[ANALYST-RECOMMENDATION]: **report-only in this brief**; the
trader-side wiring is a follow-up brief named e.g.
`reflection-memory-trader-wiring`. Reasons enumerated in `## Why`
(non-strategy invariant; 9-anchor protection). Architect may push
back if there's a strong product-fidelity reason to do trader-side
in this feature.

### Q5 — Periodic distillation (product.md layer 4)

[ARCHITECT-DECIDE]

product.md layer 4: "Periodic distillation — weekly job clusters
lesson cards into rules the user can review and promote into the
prompt library."

**Tradeoff:**
- **Ship distillation in this feature.** Product completeness;
  operator gets all four layers in one feature.
- **Defer distillation to a follow-up.** Distillation is an
  independent weekly cron job; it depends on **having ≥some lesson
  cards on disk to cluster**, which won't exist until this feature
  ships. Bundling means the distillation tests must seed-and-cluster
  a synthetic 50-card store, while card-write tests must work
  bottom-up from closed trades — two different test surfaces in one
  feature. Distillation also implies a "promote rule into prompt
  library" UI surface, and the prompt library itself doesn't exist
  in v1 (it's an LLM v2 concept).

[ANALYST-RECOMMENDATION]: **defer to a follow-up brief** named e.g.
`reflection-memory-distillation`. The follow-up runs after this
feature ships and after v2 LLM ships (since "promote into prompt
library" is meaningless without an LLM consumer of the prompt
library). Architect may push back; analyst's prior is the bundling
hurts both deliverables.

### Q6 — Anchor re-lock cadence: scope is the two `report-sample-*` only

[ARCHITECT-DECIDE — confirm only]

The `report-sample-7d` and `report-sample-90d` v1+ anchors at
[`spec/anchors.toml`](../anchors.toml) lines 67–75 **must** re-lock
because the R6 body changes. The 9 v0/v0.5/v1/v1.5a strategy
backtest anchors at lines 15–58 **must NOT** re-lock — Memory
highlights is a report-only section, and the strategy backtest
reports do not render it.

[ANALYST-RECOMMENDATION]: confirm scope is two anchors only;
architect's Design section names this explicitly so the tester
gate fails closed if a strategy-backtest anchor moves. (If a
strategy-backtest anchor moves, that signals an unintended hot-path
change — the architect's Q4 resolution either leaked, or some
ambient change broke a strategy. Either way, **escalate to
analyst**, do not re-lock those 9.)

### Q7 — Empty-state body wording on a fresh ledger with zero closed trades

[RESOLVED 2026-05-08 — operator accepted analyst strawman via orchestrator chat. The body byte locked at re-lock time is exactly: `_no closed trades yet — lesson cards will appear after the first closed trade._`. Architect's Design section names this string as a `pub const REFLECTION_MEMORY_EMPTY_STATE: &str` in `crates/reports/src/render/memory_highlights.rs` so the deny-drift discipline (R10.4 from operator-success-reports) catches any silent rewording in code review.]

R3.4 + R4.4 specify a graceful empty-state path. Analyst's strawman
wording:

```
_no closed trades yet — lesson cards will appear after the first
closed trade._
```

[ANALYST-RECOMMENDATION]: ship the strawman; operator may prefer a
different tone (e.g. "Reflection memory is empty — trades will fill
this section as they close.") The exact wording locks into the body
byte and therefore into the body-SHA-256, so the operator's choice
is the byte that gets re-locked. Once chosen, it cannot drift across
runs without breaking the determinism gate.

### Q8 — Card-write channel + back-pressure

[ARCHITECT-DECIDE]

R7.1 demands card writes happen off the trade-decision hot path.
Analyst's strawman: a tokio mpsc from the executor's fill handler
to a writer task in the agent's main loop, bounded so back-pressure
is observable. Architect may pick a different mechanism (e.g.
direct write inside a non-blocking thread pool, or batching with a
periodic flush).

[ANALYST-RECOMMENDATION]: tokio mpsc with `try_send` on the
producer side and a Prometheus counter for `reflection_card_dropped_total`
so a dropped card under back-pressure is observable. Architect to
finalise.

### Q9 — Cost-telemetry implication if Q1 picks Option B

[N/A 2026-05-08 — Q1 resolved to Option A; this question is moot for v1. Carry as a note for the LLM-enrichment follow-up brief that will ship after v2 LLM lands.]

If Q1 lands Option B (LLM-enabled v1), the report's
`LLM spend: $0.00 / $135` line begins to show non-zero spend, and
the architect must bake in token-budget gating: per-card token cap,
per-day token cap, daily-budget kill (skip card enrichment when
the budget is reached for the day). The card emission itself must
NOT block on LLM availability — a card always lands; the `note`
field is `None` if the LLM call failed or was budget-capped.

[ANALYST-RECOMMENDATION]: not relevant under Option A. Carry as a
note for the LLM-enrichment follow-up brief if Q1 lands Option A.

## Changelog

- 2026-05-08 (analyst): initial brief. Promoted from
  [backlog.md → Active](../backlog.md#active). Scope picks
  deterministic v1 (Option A from Q1), report-only retrieval (Q4
  defer), distillation deferred (Q5), two-anchor re-lock scope (Q6
  confirm). Nine open questions (Q1–Q9) for architect / operator
  resolution. Cross-references the v1+ R6 placeholder lifecycle
  contract and the `memory-anchor-relock-TBD.md` breadcrumb. Owner
  → analyst; status → in-progress; awaiting architect signoff on
  Q-resolutions before task expansion.
- 2026-05-08 (orchestrator, operator-relayed via chat): operator
  resolved the [OPERATOR-DECIDE] questions —
  - **Q1 → Option A** (deterministic v1, no LLM dependency).
  - **Q7 → analyst strawman accepted** (body string: `_no closed trades yet — lesson cards will appear after the first closed trade._`).
  - **Q9 → N/A** (only relevant under Option B).
  Six [ARCHITECT-DECIDE] questions remain (Q2 vector store, Q3
  schema/storage/regime/thresholds/embedding/K/scoping/fixture-content,
  Q4 retrieval-into-trader vs report-only, Q5 distillation defer
  confirm, Q6 anchor-scope confirm, Q8 card-write channel +
  back-pressure). Routing → architect.

HANDOFF → architect
Input files: spec/reflection-memory/feature.md, spec/reflection-memory/tasks.md
Open questions: Q1–Q9 (see "## Notes / Open questions").
