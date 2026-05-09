---
slug: reflection-memory
status: in-progress
owner: architect
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

[RESOLVED 2026-05-08 — see ## Design § reflection-memory Q2]

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

[RESOLVED 2026-05-08 — see ## Design § reflection-memory Q3]

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

[RESOLVED 2026-05-08 — see ## Design § reflection-memory Q4]

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

[RESOLVED 2026-05-08 — see ## Design § reflection-memory Q5]

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

[RESOLVED 2026-05-08 — see ## Design § reflection-memory Q6]

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

[RESOLVED 2026-05-08 — see ## Design § reflection-memory Q8]

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

## Design

Translates R1–R8 into crate / module additions, Rust types, the
`ReflectionStore` trait, the SQLite schema, the embedding feature
spec, the report-time retrieval-scoping rule, the empty-state body
constant, the card-write channel + back-pressure mechanics, and the
two-anchor re-lock procedure. All decisions anchor to the analyst's
nine open questions (Q1–Q9) — six of which are resolved here, two
of which were operator-resolved (Q1 = Option A, Q7 = analyst
strawman), and one of which is N/A under Option A (Q9).

This feature is **non-strategy and non-LLM** (Q1 = Option A) and
**report-only** (Q4 = report-only). It introduces:

- **One new leaf crate** `crates/reflection/` (lib only, no bin) —
  data model + `ReflectionStore` trait + v1 SQLite store impl +
  retrieval API + deterministic embedding + regime / outcome
  classifiers + `post_mortem_analyst::generate_card`.
- **One additive `audit::query` reader** —
  `realized_pnl_for_trade(ledger, trade_id)` (R2.2) — sibling of
  `realized_pnl_since` / `pnl_by_strategy`, same Decimal-in /
  Decimal-out / no-`sqlx`-types contract.
- **One new sibling SQLite file** `reflection.db` — held outside
  the audit DB so the chart of accounts (R6.2) and the audit
  schema migrations stay untouched. Q3a resolution.
- **One internal tokio mpsc channel** between the executor's
  fill-handler hook and the reflection writer task in `agent::main`
  — bounded, `try_send`, dropped-card metric. Q8 resolution. **Not
  a bus channel** (R8.3, hard constraint #4).
- **Two render-side additions** in `crates/reports/` — a
  `render_with_lessons(decayed, lessons)` entry-point on
  `crates/reports/src/render/memory_highlights.rs:6`, and a
  retrieval-query-construction helper that picks the
  largest-abs-PnL strategy / symbol per Q3f. Composed with the
  existing `render_with_decay` so the strategy-decay heuristic
  from operator-success-reports T811 still runs.
- **Two re-locked anchors** (`report-sample-7d`,
  `report-sample-90d`) at `spec/anchors.toml` lines 67–75. The 9
  v0/v0.5/v1/v1.5a strategy backtest anchors at lines 15–58 are
  **byte-identical** post-feature (R8.2 / V6) — verified
  negatively in `T1812` below. Q6 resolution.

**Hot-path invariants preserved:**

- The `Strategy` trait shape is unchanged (R8.1, hard constraint
  #3). The trader does not consume retrieval — Q4 = report-only.
  The 9 strategy backtest anchors are isolated from this feature.
- The audit chart of accounts has no new account (R6.1, hard
  constraint #6). Lesson cards are derived artefacts, not ledger
  events. The reconciliation invariant from
  operator-success-reports R11 holds byte-for-byte.
- The body-vs-front-matter discipline from
  operator-success-reports R10 (hard constraint #7) is enforced
  on every card line: `closed_at` is the only timestamp that
  appears in the body, and it sources from the ledger (RFC3339,
  microsecond precision), not from wall-clock at render time.
- The atomic-write contract from operator-success-reports R12 is
  reused via `crates/reports/src/atomic_write.rs:38` for every
  card row that survives the writer task's batch flush (hard
  constraint #6 — no card lands half-written). The reflection
  store's SQLite path uses the same tempfile + rename pattern at
  database-close time so a crash mid-flush never leaves a
  partial WAL frame committed against an inconsistent header.
- No LLM provider crate, no `tokens` field, no
  `expense:llm:*` ledger row impact (R8.4, hard constraint #5).
  The cost-telemetry V8 stays at `$0.00 / $135`.

### Q-resolution summary

| Q  | Topic                                       | Resolution                                                                                                                                                                         |
|----|---------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Q1 | Deterministic v1 vs LLM-enabled v1          | **Option A — deterministic v1** (operator-resolved 2026-05-08). No LLM provider crate; no `expense:llm:*` row impact; LLM enrichment lands as a follow-up brief after v2 LLM ships. |
| Q2 | Vector store choice                         | **New SQLite table inside a new sibling `reflection.db`, linear-scan top-K** behind a `ReflectionStore` trait. v2 swap to qdrant or sqlite-vss is a single-trait-impl change.       |
| Q3 | Schema details (bundle: storage / regime / thresholds / embedding / K / scoping / fixture content) | See **Q3a–Q3g** sub-decisions below. Storage = sibling `reflection.db`; regime = BTC 7d return ±2% (analyst strawman pinned); outcome thresholds = ±0.5% of opening capital (analyst strawman pinned); embedding = deterministic 32-dim hand-crafted vector (schema below); K = 5; scoping = largest-abs-PnL strategy + symbol; fixture content = ≥3/≥10. |
| Q4 | Retrieval at decision time                  | **Report-only this round.** Trader-side wiring is a follow-up brief named `reflection-memory-trader-wiring`. The `Strategy` trait is unchanged.                                     |
| Q5 | Periodic distillation (product.md layer 4)  | **Defer to a follow-up brief** named `reflection-memory-distillation`. Bundling hurts both deliverables; distillation needs cards on disk to cluster.                              |
| Q6 | Anchor re-lock cadence                      | **Confirmed scope is the two `report-sample-*` v1+ anchors only.** The 9 strategy-backtest anchors at `spec/anchors.toml:15-58` are byte-identical post-feature (R8.2 / V6).        |
| Q7 | Empty-state body wording                    | **Operator-resolved 2026-05-08** — `_no closed trades yet — lesson cards will appear after the first closed trade._`. Locked as `pub const REFLECTION_MEMORY_EMPTY_STATE: &str`.   |
| Q8 | Card-write channel + back-pressure          | **Bounded tokio mpsc (capacity 1024), `try_send` on the producer side, Prometheus counter `reflection_card_dropped_total{reason="back_pressure"}`** on drop. Internal — not a bus channel. |
| Q9 | Cost-telemetry under Option B               | **N/A** under Option A. Carried as a note for the LLM-enrichment follow-up brief.                                                                                                  |

#### reflection-memory Q2 — Vector store choice: **new SQLite table in a sibling `reflection.db`, linear-scan top-K, behind a `ReflectionStore` trait**

**Decision:** v1 ships **a single SQLite table `lesson_cards` inside
a new sibling SQLite file `reflection.db`**, kept outside the audit
DB. Top-K retrieval is a **deterministic linear scan** over all rows
(scores computed in-process; no `sqlite-vss` extension required).
The retrieval API is exposed behind a `ReflectionStore` trait so v2
can swap in qdrant or sqlite-vss without rewriting any consumer.

**Rationale:**

- **Zero new infra-spend dependency** — single-binary friendly per
  the architect compatibility checklist (`.claude/agents/architect.md`
  lines 47–72). qdrant means a second daemon to supervise; we ship
  one Rust binary today (R7 of operator-success-reports preserved).
  `sqlite-vss` requires a SQLite extension load (system C dep
  without a `bundled` feature on most platforms — a checklist
  reject).
- **Right-sized for v1 scale.** The 1-year fixture produces
  100–500 closed trades (R7.2) → ≤500 lesson cards × 32-dim
  Decimal-as-i64-quantised vector (R2.5 / Q3d below) ≈ 64 KiB hot
  set, sub-millisecond per-row cosine score, well inside R7.2's
  100ms top-K budget. v2 with LLM embeddings (768-dim, 100k+
  cards) will need ANN — that's a follow-up under the
  `ReflectionStore` trait.
- **Append-only contract is byte-stable.** The store's idempotency
  key is `card_id` (the content-hash from R1.1); the linear scan
  returns rows in deterministic `(score DESC, closed_at ASC)`
  order so retrieval is byte-stable across runs (R3.2 — load-bearing
  for the body-SHA-256 lock).
- **Trait abstraction is the v2 escape hatch.** The trait surface
  matches the swap candidates: `upsert(card)`, `top_k(query, k)`,
  `count()`, `len_at(closed_at)`. qdrant and sqlite-vss both speak
  the same shape; a v2 brief can swap the impl in a single PR.

**Alternatives considered:**

- **qdrant** — second-process supervision, qdrant-client crate as
  a hard dep, ANN we don't need at 500-card scale. Reject for
  v1; reconsider at v2 LLM scale.
- **`sqlite-vss`** — in-process but the extension is loaded via
  `PRAGMA load_extension` and the `sqlite-vss` Rust binding tracks
  a system C dep without a `bundled` feature. Reject on the
  single-binary discipline.
- **Store cards as a new table inside the audit DB** — would
  conflate the chart-of-accounts boundary (R6.2) and force every
  audit migration to consider a non-ledger table. Reject;
  `reflection.db` is a sibling, not a co-tenant.

**How it shows up in code:**

- New crate `crates/reflection/` (lib).
- Trait `reflection::store::ReflectionStore` (Send + Sync) in
  `crates/reflection/src/store/mod.rs` — async, every method
  returns `Result<_, ReflectionStoreError>`. Methods:
  `upsert(card: &LessonCard) -> Result<bool, …>` (returns `true`
  when a new row was inserted, `false` when the `card_id` already
  existed — R2.4 idempotency); `top_k(query: &RetrievalQuery, k:
  usize) -> Result<Vec<LessonCard>, …>`; `count() -> Result<u64, …>`.
- Default impl `reflection::store::sqlite::SqliteReflectionStore`
  in `crates/reflection/src/store/sqlite.rs` — owns a `sqlx::SqlitePool`
  pointed at `<ledger_root>/reflection.db`. Migration
  `crates/reflection/migrations/001_lesson_cards.sql` creates the
  `lesson_cards` table (schema below at Q3d).
- The `ReportRunCfg` shape (forwarded to `reports::generate`) gains
  a single optional field `reflection_store: Option<Arc<dyn
  ReflectionStore>>`. When `None`, the renderer emits the
  empty-state body (R4.4) — keeps the reports binary runnable
  against pre-reflection ledgers.
- Tests grep surface: `crates/reflection/src/store/`,
  `crates/reflection/tests/store_idempotency.rs`,
  `crates/reflection/tests/store_top_k_determinism.rs`.

#### reflection-memory Q3 — Lesson-card schema details

The Q3 bundle resolves seven sub-decisions. Each takes the
analyst's strawman from R1.3 / R1.4 / R2.5 / R3.1 / R4.3 unless an
explicit reason emerges to override.

##### Q3a — Storage location: **sibling `reflection.db` SQLite file**

**Decision:** Lesson cards live in a single new SQLite file
`reflection.db`, **separate from the audit DB**, written under the
configured ledger root (production: `<config.audit.path>/../reflection.db`;
tests / fixture: `target/test-ledgers/reflection-<scenario>.db`).

**Rationale:**

- **Audit DB schema untouched** (R6.2). The chart of accounts and
  the eight audit migrations (`001_chart_of_accounts.sql` through
  `008_journal_transactions_venue.sql`) ship unchanged. v1+'s R11
  reconciliation invariant `cash + Σ positions = equity` cannot be
  affected by anything that lives in `reflection.db` because the
  reconciliation engine reads only from the audit DB
  (`crates/reports/src/reconcile.rs`).
- **Crash-safety boundary.** A crash mid-card-write corrupts at
  worst the WAL of `reflection.db`; the audit DB's WAL is
  unaffected. Lesson cards are derived artefacts (R6.1) — losing
  the most-recent-second of cards is recoverable by re-deriving
  from the ledger; losing audit rows is not.
- **Deterministic test-isolation.** Test fixtures wire
  `target/test-ledgers/reflection-<test-name>.db` so two parallel
  `cargo test` invocations don't race on the same file. The
  fixture builders at
  `crates/reports/tests/fixtures/build_ledger_{7d,90d}.rs` gain a
  sibling `build_reflection_store_{7d,90d}.rs` builder per Q3g.

**How it shows up in code:**

- `reflection::store::sqlite::SqliteReflectionStore::open(path: &Path)`
  is the single entry-point; production callers pass
  `cfg.reflection.path` (a new TOML key — additive, defaults to
  `<cfg.audit.path>/../reflection.db`).
- Config plumbing: `agent::config::ReflectionCfg { path: PathBuf,
  channel_capacity: usize (default 1024), enable_writer: bool
  (default true) }` lands in `crates/agent/src/config.rs`. v1 default
  is `enable_writer = true`; operators can disable for a no-op
  (e.g. read-only research mode) via `[reflection] enable_writer =
  false`.
- The 1-year perf-smoke fixture at
  `crates/reports/tests/fixtures/build_ledger_1y.rs` extends to
  also seed `reflection-1y.db` via the new
  `build_reflection_store_1y.rs` companion (Q3g).

##### Q3b — Regime classifier: **BTC 7-day return ±2% (analyst strawman pinned, deterministic, pure-function)**

**Decision:** ship the analyst's strawman from R1.3 verbatim. The
classifier is a pure function `classify_regime(btc_close_series:
&[Decimal], at: Timestamp) -> RegimeTag` over BTC 1m closes the
agent already reads (`data/binance/BTCUSDT/...`):

- `Bull` if `(close[at] - close[at - 7d]) / close[at - 7d] > +0.02`,
- `Bear` if same ratio `< -0.02`,
- `Chop` otherwise.

**Rationale:** v1 prioritises shipping over richness. The
classifier is the **single regime feature in the embedding** (Q3d
below) — making it richer in v1 (e.g. realised volatility regime,
BTC-vs-ETH cross-correlation) means more parameters to pin into
the body-SHA-256, and more code to retire when v2's LLM regime
classifier ships. The BTC-anchored choice is admittedly
BTC-centric for an altcoin pair like `BNBUSDT-BTCUSDT`, but the
embedding's `strategy_one_hot` + `pair_hash` slots dominate
ranking for pair-MR cards anyway (the regime slot is one of three
`f32`s out of 32; see Q3d).

**Alternatives considered:**

- Realised-volatility regime (`σ(BTC, 7d)` bucketed into
  `LowVol|MidVol|HighVol`) — adds a `decimal_std`-over-7d-of-1m
  bars compute (~10080 cells) per card-write; not free, and
  requires another constant in the body. Reject for v1.
- Per-strategy regime (e.g. mean-reversion strategies use spread-
  z-score regime) — couples the classifier to strategy code,
  defeats the deterministic-pure-function-over-prices simplicity.
  Reject; the v2 LLM classifier can do this richer mapping.

**How it shows up in code:**

- `reflection::regime::classify_regime(btc_closes: &[(Timestamp,
  Decimal)], at: Timestamp) -> Result<RegimeTag, RegimeError>` —
  pure, no I/O, no clock. `RegimeTag` is a `#[derive(Serialize,
  Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]` enum
  `{ Bull, Bear, Chop }`. Display impl emits exactly
  `bull|bear|chop` (lowercase, no quotes) for body bytes.
- Determinism property: same `btc_closes` slice + same `at` →
  byte-identical `RegimeTag`. Asserted by
  `crates/reflection/tests/regime_classifier.rs::t1801_classify_regime_byte_stable`.
- The classifier consumes BTC closes via the **`MarkSource` trait
  from `crates/reports/src/marks.rs:732`** (re-used; no new crate
  edge). v1 of the classifier accepts a `&[(Timestamp, Decimal)]`
  slice so unit tests inject a hand-rolled vector; the production
  `post_mortem_analyst::generate_card` calls
  `MarkSource::close_series(BTCUSDT, at - 7d, at, cadence_minutes
  = 1440)` to fetch the 7 daily samples and passes them in.

##### Q3c — Outcome thresholds: **±0.5% of opening capital (analyst strawman pinned, fee-aware)**

**Decision:** ship R1.4's `±0.5%` Win/Loss threshold verbatim. The
threshold is computed against the **opening capital at trade open**
(the cash balance at the timestamp of the buy-side
`journal_transactions` row that opened the position), so a 1bp net
profit after fees does not get classified as a Win.

- `Win` iff `signed_pnl_usdt / opening_capital_usdt > +0.005`,
- `Loss` iff same ratio `< -0.005`,
- `Scratch` otherwise.

**Rationale:**

- **Fee-aware by construction.** `signed_pnl_usdt` already
  subtracts `expense:fees:taker` per the journal double-entry
  shape at `crates/audit/src/journal.rs:160-194`, so the
  threshold sees net P&L. The `0.5%` line catches exactly the
  "noise vs signal" boundary the v1.5a backtests revealed
  (median per-trade P&L magnitude ≈ 0.3%–0.7%; 0.5% straddles
  it).
- **Strategy-agnostic.** Architect could pin per-strategy
  thresholds (mean-reversion has tighter per-trade variance than
  momentum), but that adds a new TOML knob, more strategy-coupled
  code in `reflection`, and shifts the body when a strategy is
  added. v1 keeps it strategy-agnostic; v2 LLM enrichment can
  reclassify under richer rules without changing the deterministic
  v1 outcome class.

**How it shows up in code:**

- `reflection::outcome::classify_outcome(signed_pnl: Money<Usdt>,
  opening_capital: Money<Usdt>) -> OutcomeClass` — pure, three-arm
  branch. `OutcomeClass` is `#[derive(Serialize, Deserialize, Copy,
  Clone, Debug, PartialEq, Eq, Hash)]` enum
  `{ Win, Loss, Scratch }`; Display emits `Win|Loss|Scratch`
  (PascalCase, matches the body-line shape in R4.2).
- One-line threshold constant `pub const OUTCOME_THRESHOLD_PCT:
  Decimal = dec!(0.005)` at `crates/reflection/src/outcome.rs:5`
  so a future architect can grep + change in one place.
- Tests: `crates/reflection/tests/outcome_classifier.rs`
  exercises (a) +0.6% → Win, (b) -0.6% → Loss, (c) 0.4% → Scratch,
  (d) -0.4% → Scratch, (e) `opening_capital == 0` → returns
  `OutcomeClass::Scratch` (defensive — denominator-zero is treated
  as a no-signal case, not an error, since the audit ledger never
  produces `opening_capital == 0` for a real fill).

##### Q3d — Embedding dimensions and feature schema: **deterministic 32-dim vector, fixed packed layout**

**Decision:** the embedding is a **deterministic 32-dim vector of
Decimals** (no `f64` in the embedding compute, no LLM, no learned
features). Layout pinned below.

| Slot index | Slot name              | Width (cells) | Encoding                                                                                                              |
|-----------:|------------------------|--------------:|----------------------------------------------------------------------------------------------------------------------|
| 0..6       | `strategy_one_hot`     | 7             | `1.0` in the slot indexed by `STRATEGY_INDEX[strategy_id]`; `0.0` elsewhere. The 7 strategy slots are pinned at compile time (`sma_crossover`, `macd_trend`, `rsi_reversion`, `bbands_mean_revert`, `top10_momentum_h1`, `pairs_mr_h1`, `(unattributed)`). New strategies extend the slot map at the end (one per future feature); the embedding dim grows by one each time, and the SHA changes — call this out in any future feature brief that adds a strategy. |
| 7..9       | `regime_one_hot`       | 3             | `1.0` in `Bull`/`Bear`/`Chop` slot; `0.0` elsewhere.                                                                  |
| 10..12     | `outcome_one_hot`      | 3             | `1.0` in `Win`/`Loss`/`Scratch` slot; `0.0` elsewhere.                                                                |
| 13         | `signed_pnl_sign`      | 1             | `+1.0` if `signed_pnl > 0`, `-1.0` if `< 0`, `0.0` if `== 0`.                                                          |
| 14         | `log_pnl_magnitude`    | 1             | `log10(\|signed_pnl_usdt\| + 1)` truncated to 4 decimal places via `Decimal`-only arithmetic (`features::math::decimal_log10` reuse, same precision as v1.5a `decimal_ln`). |
| 15         | `log_holding_period`   | 1             | `log10(holding_period_bars + 1)` same shape.                                                                          |
| 16         | `pair_hash_norm`       | 1             | For pair trades: a stable `[0.0, 1.0]` projection of the `PairKey` content hash (`sha256(pair_key.to_string()) → first 8 bytes → u64 → / u64::MAX → Decimal`). For single-leg trades: `0.0`. Sufficient to distinguish across the 3-pair v1.5a universe; not a true semantic embedding (that's v2). |
| 17         | `single_symbol_hash_norm` | 1          | Mirror of slot 16 but for `Symbol` content hash; `0.0` for pair trades.                                               |
| 18..31     | `reserved`             | 14            | All `0.0` in v1. Reserved for v2 LLM-augmented features (e.g. text-embedding 384-dim → PCA-down to 14 dims).            |

The vector is stored in `lesson_cards.embedding_blob` as a packed
TEXT column — 32 comma-separated `Decimal::to_string()` values. Why
TEXT not BLOB: keeps the 1-year fixture's `reflection.db`
hex-comparable across runs without a binary-diff dance, and matches
the audit DB's TEXT-amount convention.

**Similarity metric:** **cosine similarity computed in `Decimal`**
(no `f64`). The denominator is `(\|q\| · \|c\|).max(dec!(1e-12))` to
avoid divide-by-zero for the embedding-zero case (a zero embedding
arises only on a pathological cardless retrieval — the empty-store
path returns `Ok(vec![])` short-circuit earlier).

**Rationale:**

- **Deterministic by construction.** No floats, no learned weights,
  no unseeded `rand`. Property test in
  `crates/reflection/tests/embedding_determinism.rs` proves
  byte-identity over 1000 random card inputs.
- **Right-sized for the 100ms / 500-card budget** (R7.2). Per-row
  cosine over 32 Decimal slots ≈ 5µs on x86_64; 500 rows ≈ 2.5ms,
  40× under budget.
- **Reserved slots for v2.** The 14 reserved slots mean v2's LLM
  embedding can land without re-quantising every card already on
  disk — v2 architect adds a sibling
  `lesson_cards_v2_embeddings(card_id, embedding_blob)` table
  (cards stay where they are; the vector grows alongside under a
  schema migration).

**How it shows up in code:**

- Module `reflection::embedding` with `pub fn embed(card:
  &LessonCard) -> [Decimal; 32]` — pure, total over its inputs.
- The `STRATEGY_INDEX` map lives in
  `crates/reflection/src/embedding.rs` as a `pub const
  STRATEGY_SLOTS: &[&str; 7]` array — grep target for any future
  feature that adds a strategy. The sibling const
  `pub const EMBEDDING_DIM: usize = 32` is the only place the
  width is named.
- Cosine helper in `reflection::embedding::cosine(a: &[Decimal;
  32], b: &[Decimal; 32]) -> Decimal`.

##### Q3e — Default K for top-K retrieval at report time: **K = 5 (analyst strawman pinned)**

**Decision:** `K = 5` at report time. Pinned as
`pub const REPORT_TIME_TOP_K: usize = 5` in
`crates/reflection/src/lib.rs` so a future architect changes it in
one place (and locks new SHAs accordingly).

**Rationale:** five is the operator's eyeball ceiling — three is
sparse, ten is too many to scan in a paragraph. K affects body byte
length linearly (one card per line ~80 bytes) but not byte
determinism. K = 5 gives a 5×80 ≈ 400-byte memory-highlights
section, comparable in scale to operator-success-reports' R4 risk
metrics block.

**How it shows up in code:** the top-K constant is a `pub const`,
and the renderer at
`crates/reports/src/render/memory_highlights.rs:6` calls
`reflection::retrieve_top_k(store, &query, REPORT_TIME_TOP_K)`.

##### Q3f — Retrieval-query scoping rule at report time: **largest-abs-PnL strategy + symbol, ledger-wide history**

**Decision:** at report time, the retrieval query is built per the
analyst's strawman in R4.3:

1. `strategy_id` = the strategy with the **largest absolute P&L
   this period** (over the report's `[period_start, period_end]`
   window). Tie-break on lex-sorted `strategy_id` ASC for
   determinism. The `(unattributed)` synthetic bucket is excluded
   from this selection — if it would otherwise win, fall through to
   the next non-unattributed strategy. (If none, the empty-state
   body fires per R4.4.)
2. `symbol_or_pair` = the **symbol with the largest absolute P&L
   under that strategy this period**. Tie-break: lex-sort ASC. For
   pair strategies (`pairs_mr_h1`), the resolved symbol is the
   `a` leg of the pair (the traded long-only leg per v1.5a Q3 / R5).
3. `current_regime_tag` = `classify_regime(btc_closes, period_end)`
   per Q3b.
4. `time-window` = **unbounded** (retrieve over the entire history
   of `reflection.db`, not just this period — older lessons
   matter more for the operator's institutional-memory framing).

**Rationale:**

- **Single, deterministic, ledger-grounded selection.** The
  largest-abs-PnL strategy is the one the operator is most likely
  to ask "what did we learn about it?" about this period. The
  largest-abs-PnL symbol under that strategy is the same logic at
  one finer granularity.
- **Unbounded time-window** because the moat-bet framing is
  "what did the agent learn from the **last 7 days** of trading?"
  — the **report period** is the trader's recent activity, but
  the **lessons** the operator wants to see are the most-relevant
  cards over all time, not just this period. (If only this period
  is desired, the operator picks a longer `--period`.)

**Alternatives considered:**

- **All strategies, retrieve K each, then merge** — produces a
  noisier highlights block when one strategy dominates the
  period; the operator gets one obvious card per strategy
  regardless of relevance. Reject; the largest-abs-PnL focus is
  the operator's mental model.
- **Most-recently-active strategy** — too noisy (the last 5
  minutes of trading shouldn't shape the highlights). Reject.

**How it shows up in code:**

- New helper
  `crates/reports/src/render/memory_highlights.rs::build_retrieval_query(pnls:
  &[StrategyPnl], current_regime: RegimeTag) -> Option<RetrievalQuery>`.
  Returns `None` iff the active set has zero non-unattributed
  strategies (the empty-state path).
- Determinism gates: tie-break order asserted by
  `crates/reports/tests/memory_highlights.rs::t1814_largest_abs_pnl_tie_break_lex_ascending`.

##### Q3g — Fixture content extension: **7d ≥3 cards, 90d ≥10 cards (analyst strawman pinned), pinned to a 6×9 coverage matrix on 90d**

**Decision:** the fixture extension follows the analyst's prior
verbatim:

- `report-sample-7d` fixture: extend
  `crates/reports/tests/fixtures/build_ledger_7d.rs:1` to produce
  exactly **3 closed trades across 2 strategies** (1 Win + 1 Loss
  + 1 Scratch; 1 trade in Bull regime + 1 in Chop regime + 1 in
  Bear regime — yes, all three regimes inside a 7d window via
  selectively-chosen synthetic BTC closes). `reflection.db` is
  built by the new sibling
  `crates/reports/tests/fixtures/build_reflection_store_7d.rs`
  helper that consumes the same closed-trade list and writes 3
  cards.
- `report-sample-90d` fixture: extend
  `crates/reports/tests/fixtures/build_ledger_90d.rs:1` to produce
  **10 closed trades across 3 strategies** (`sma_crossover`,
  `pairs_mr_h1`, `top10_momentum_h1`) with the **6×9 outcome ×
  regime coverage matrix** below — all 9 (Win|Loss|Scratch) ×
  (Bull|Bear|Chop) cells exercised at least once, plus one
  pair-MR pair-leg trade.

**Rationale:**

- **Edge-case coverage at 90d** — the larger fixture exercises
  every (outcome, regime) cell so the body bytes encode every
  rendered code-path at least once. The 7d fixture exercises a
  small subset for speed.
- **Determinism gate.** Both fixtures are seeded by `FIXTURE_SEED
  = 0xC0FFEE` (already pinned in
  `crates/reports/tests/fixtures/build_ledger_7d.rs:52`) and
  produce byte-identical bodies across two re-runs. The two new
  body-SHA-256s are captured by the tester at the V6 gate.

**How it shows up in code:**

- New file `crates/reports/tests/fixtures/build_reflection_store_7d.rs`
  with `pub fn build_reflection_store_7d(audit_path: &Path,
  reflection_path: &Path, marks: &dyn MarkSource) -> Vec<LessonCard>`.
  Reads closed trades from the audit fixture, calls
  `post_mortem_analyst::generate_card` for each, writes to the
  store, returns the card list for assertion.
- Sibling `build_reflection_store_90d.rs` for the 90d shape.
- Same shape applies to `build_reflection_store_1y.rs` for the
  perf-smoke fixture (used by the R7.2 < 100ms top-K assertion).

#### reflection-memory Q4 — Retrieval at decision time: **report-only this round**

**Decision:** retrieval is **report-only in this brief**. The
trader's `Strategy` trait shape is unchanged. The trader does not
consume retrieval; the only consumer of `reflection::retrieve_top_k`
is the operator success report's R6 memory-highlights renderer.
Trader-side wiring is a follow-up brief named
`reflection-memory-trader-wiring` and is not in scope here.

**Rationale:**

- **9 strategy backtest anchors stay byte-identical** (R8.2, hard
  constraint #2). Wiring retrieval into `Strategy::on_bar` would
  add a per-bar lookup; the lookup result feeds into signal
  generation; the signal series shifts; fills shift; journal rows
  shift; backtest-report bytes shift; the 9 anchors at
  `spec/anchors.toml:15-58` re-anchor. That re-lock is a major
  scope expansion that this feature is explicitly avoiding (the
  brief's `## Why` calls it out at line 79).
- **`Strategy` trait is a v0 invariant** (R8.1, hard constraint
  #3). The trait has been stable since v0; preserving it is a
  cross-feature contract. Adding a retrieval seam is
  trait-additive and the architect's call belongs in a separate
  brief, not folded into a presentation-layer feature.
- **Product fidelity is the follow-up brief's job.** The
  operator's "what did the agent learn?" question has two answers
  — "here's what the report can show you" (this brief) and
  "here's what the agent actually used at decision time" (the
  trader-wiring follow-up). Decoupling the two ships the
  operator-visible value first.

**How it shows up in code:**

- `reflection::retrieve_top_k(...)` is callable from
  `crates/reports/src/render/memory_highlights.rs` only. No
  caller in `crates/strategy/`, `crates/agent/`, `crates/exec/`.
- Negative-confirmation test: `crates/reflection/tests/no_strategy_caller.rs`
  asserts that no symbol from `crates/strategy/` resolves
  `reflection::retrieve_top_k` via `cargo metadata` + a
  `rg --type rust 'reflection::retrieve_top_k'` filter restricted
  to `crates/strategy/`. Test fails if a future PR wires the
  trader without a follow-up brief — same defensive-static-grep
  pattern as the body-no-volatile-metadata test in
  `crates/reports/tests/body_no_volatile_metadata.rs`.

#### reflection-memory Q5 — Periodic distillation: **defer to follow-up brief `reflection-memory-distillation`**

**Decision:** distillation (product.md layer 4 — "weekly job
clusters lesson cards into rules the user can review and promote
into the prompt library") is **deferred to a follow-up brief**.
Not in scope here.

**Rationale:**

- **Distillation depends on cards on disk.** Bundling means the
  distillation tests must seed-and-cluster a synthetic 50-card
  store, while card-write tests must work bottom-up from closed
  trades — two different test surfaces in one feature.
- **The "promote into prompt library" step is meaningless without
  an LLM consumer.** The prompt library is a v2 LLM concept; v1
  has no consumer for the distilled rules. Building distillation
  before the consumer is buildware-without-a-customer.
- **Independent failure modes.** A clustering bug should not
  block a card-write fix; deferring isolates the failure surface.

**How it shows up in code:**

- **Nothing in this feature** — by design. The follow-up brief
  `reflection-memory-distillation` opens after this feature ships
  and after v2 LLM ships. Backlog entry to be added by the
  analyst at feature-ship time.
- Forward-compat scaffolding: a one-paragraph rustdoc note at
  `crates/reflection/src/lib.rs:1` documents the layer-4 deferral
  so the future architect can grep for it (mirror of the v1+
  T811 / Q9 placeholder note pattern at
  `crates/reports/src/render/memory_highlights.rs:6`).

#### reflection-memory Q6 — Anchor re-lock cadence: **confirmed scope is the two `report-sample-*` v1+ anchors only**

**Decision:** the V6 re-lock scope is **exactly two anchors** —
`report-sample-7d` and `report-sample-90d` at `spec/anchors.toml`
lines 67–75. The 9 strategy backtest anchors at lines 15–58 are
**byte-identical** post-feature.

**Rationale:**

- **Memory highlights is a report-only section** — only the
  operator success report renders `## Memory highlights`. The
  strategy backtest binary at `crates/backtest/src/main.rs::write_report`
  emits a different report shape and does not touch
  `crates/reports/src/render/memory_highlights.rs`. Confirmed by
  static read of the two render paths.
- **Q4 = report-only** means no trader-side change → no fills /
  journal-row shift → no backtest-body shift → 9 anchors
  unchanged.
- **Defensive negative test** lives at `T1812` below: tester runs
  `bash scripts/verify_anchors.sh`, expects all 9 strategy
  anchors at lines 15–58 unchanged, and expects only the two
  v1+ anchors at lines 67–75 to be replaced. If any of the 9
  drift, **escalate to analyst** — the cause is an unintended
  hot-path change that violates Q4 = report-only.

**How it shows up in code:**

- The re-lock procedure (R5.4) is enumerated step-by-step in
  task `T1813` below.
- `spec/anchors.toml` is **not** edited by the architect or
  developer. The tester captures the new SHAs in `T_FINAL_REFLECTION_MEMORY`
  and edits lines 67–75 only — same pattern as v1+ T816 and
  v1.5a T717.
- The dev-note at
  `spec/dev-notes/memory-anchor-relock-TBD.md:1` gains a
  "completed at <date>" footer pointing to this feature's
  `tasks.md` once the tester closes V6.

#### reflection-memory Q8 — Card-write channel + back-pressure: **bounded tokio mpsc, `try_send`, Prometheus drop counter**

**Decision:** the card-write channel is a **bounded tokio mpsc**
with capacity `1024`, fed by the executor's fill-handler hook (a
new tap point — see "How it shows up in code" below) and consumed
by a single writer task in `agent::main`. The producer side uses
`try_send`; on `TrySendError::Full` (back-pressure under burst),
the producer **drops the message** and increments the Prometheus
counter `reflection_card_dropped_total{reason="back_pressure"}`.

**Rationale:**

- **R7.1 hot-path invariant.** Card writes MUST NOT add measurable
  latency to the executor's submit-fill path. `try_send` is
  zero-await; a full-channel `try_send` returns immediately with
  `TrySendError::Full`. Worst-case under burst: the executor's
  fill-handler observes a 0-allocation fast-fail; the writer task
  catches up at its own pace.
- **Drop-not-block under back-pressure.** A bounded channel with
  block-on-full would convert a producer burst into latency on
  the executor's hot path — exactly what R7.1 forbids. Dropping
  is safe because (a) lesson cards are **derived artefacts**
  (R6.1) — losing one is recoverable by re-deriving from the
  ledger if the operator wants to backfill, and (b) the Prometheus
  counter makes the drop **observable**, so an alert on
  `reflection_card_dropped_total > 0 over 1h` surfaces the
  back-pressure event.
- **Capacity 1024.** At v1 scale (≤500 cards across a 1-year
  fixture; production rate ≤ a few cards per minute), 1024 is
  ~17 hours of queue at the production fill rate. The writer
  task is `tokio::select!` over the receiver; it consumes at
  ~1ms per card (SQLite WAL write to a small DB), so the
  steady-state queue depth is `O(1)`. Capacity 1024 is the
  catastrophic-backpressure safety net, not the steady-state
  capacity.
- **Internal — not a bus channel** (R8.3, hard constraint #4).
  The `agent::bus::Bus` v0 channel set at
  `crates/agent/src/bus.rs` is unchanged; the mpsc is a private
  field on the new `ReflectionWriter` type.

**Alternatives considered:**

- **Direct write inside a non-blocking thread pool** — `tokio::spawn`
  per fill — produces unbounded task spawn under burst, no
  observability into drop, GC pressure on tokio. Reject.
- **Periodic flush from an in-memory queue** — adds latency
  variance to the report's "card visible after close" semantics;
  on graceful shutdown, the unflushed queue is lost (worse than a
  bounded-mpsc drop because there's no counter). Reject.
- **Unbounded mpsc** — converts a producer burst into unbounded
  memory growth on the writer task. Reject (R7.4 RSS budget).

**How it shows up in code:**

- New module `reflection::writer` with:
  - `pub struct ReflectionWriter { tx: mpsc::Sender<LessonCardWriteRequest>, dropped: Arc<AtomicU64> }`,
  - `pub fn new(store: Arc<dyn ReflectionStore>, capacity: usize) -> (Self, ReflectionWriterTask)`,
  - `pub fn try_enqueue(&self, req: LessonCardWriteRequest) -> Result<(), TryEnqueueError>` — calls `tx.try_send(req)`; on `TrySendError::Full` → `dropped.fetch_add(1, Ordering::Relaxed)` + `Err(TryEnqueueError::BackPressure)`.
- New module `reflection::writer::task::ReflectionWriterTask` with:
  - `pub async fn run(self) -> Result<(), ReflectionStoreError>` — drains the receiver, calls `post_mortem_analyst::generate_card` per request, calls `store.upsert(card)`, logs idempotent skips at `tracing::debug` level.
- The fill-handler tap is a one-line addition at
  `crates/exec/src/paper.rs::PaperEngine::on_signal` (the trade-close
  detection logic — when a sell-side fill brings the per-symbol
  position to zero) and at any other engine that closes positions.
  The handler calls
  `reflection_writer.try_enqueue(LessonCardWriteRequest { closed_trade, opening_capital, .. })`.
- Prometheus metric:
  `reflection_card_dropped_total{reason="back_pressure"}` is
  registered via `metrics::register_counter!`; consumed by the
  cockpit's existing Prometheus scrape (no new endpoint).
- Bus invariant test: `crates/agent/tests/no_new_bus_channel.rs`
  asserts the `agent::bus::Bus` struct's set of public fields is
  unchanged from the v1+ snapshot. Static-grep style, mirrors the
  body-no-volatile-metadata test pattern.

### Crate / module surface

This subsection lists every file the developer creates and every
existing file they modify, with line-number citations where I can
pin them. The table is the dev's grep-target — they can walk it
top-to-bottom from M1 to M5.

**New files (created by the developer in T1801–T1814):**

| Path                                                                                    | Created in task | Purpose                                                                            |
|-----------------------------------------------------------------------------------------|-----------------|-----------------------------------------------------------------------------------|
| `crates/reflection/Cargo.toml`                                                          | T1801           | New leaf crate manifest (lib only).                                                |
| `crates/reflection/src/lib.rs`                                                          | T1801           | Crate root; re-exports `LessonCard`, `RegimeTag`, `OutcomeClass`, `RetrievalQuery`, `ReflectionStore`, `retrieve_top_k`, `REPORT_TIME_TOP_K`. |
| `crates/reflection/src/types.rs`                                                        | T1801           | `LessonCard`, `RetrievalQuery`, `LessonCardWriteRequest`, `Card`-id content hash function. |
| `crates/reflection/src/regime.rs`                                                       | T1802           | `classify_regime` + `RegimeTag` enum.                                              |
| `crates/reflection/src/outcome.rs`                                                      | T1802           | `classify_outcome` + `OutcomeClass` enum + `OUTCOME_THRESHOLD_PCT` constant.       |
| `crates/reflection/src/embedding.rs`                                                    | T1803           | `embed`, `cosine`, `STRATEGY_SLOTS`, `EMBEDDING_DIM`.                              |
| `crates/reflection/src/post_mortem_analyst.rs`                                          | T1804           | `generate_card(closed_trade, opening_capital, btc_closes) -> LessonCard`.          |
| `crates/reflection/src/store/mod.rs`                                                    | T1805           | `ReflectionStore` trait + `ReflectionStoreError`.                                  |
| `crates/reflection/src/store/sqlite.rs`                                                 | T1805           | `SqliteReflectionStore` impl (linear-scan top-K).                                  |
| `crates/reflection/migrations/001_lesson_cards.sql`                                     | T1805           | The one schema migration that creates `lesson_cards` table.                        |
| `crates/reflection/src/writer/mod.rs`                                                   | T1807           | `ReflectionWriter` (producer side) + `LessonCardWriteRequest` + back-pressure metric. |
| `crates/reflection/src/writer/task.rs`                                                  | T1807           | `ReflectionWriterTask::run` (consumer side).                                       |
| `crates/reflection/src/retrieval.rs`                                                    | T1809           | `retrieve_top_k(store, query, k)` — public entry-point.                            |
| `crates/reflection/tests/store_idempotency.rs`                                          | T1806           | R2.4 idempotency.                                                                  |
| `crates/reflection/tests/store_top_k_determinism.rs`                                    | T1810           | R3.2 deterministic top-K.                                                          |
| `crates/reflection/tests/regime_classifier.rs`                                          | T1802           | R1.3 acceptance.                                                                   |
| `crates/reflection/tests/outcome_classifier.rs`                                         | T1802           | R1.4 acceptance.                                                                   |
| `crates/reflection/tests/embedding_determinism.rs`                                      | T1803           | R2.5 acceptance.                                                                   |
| `crates/reflection/tests/post_mortem_generate_card.rs`                                  | T1804           | R1 + R2.3 acceptance.                                                              |
| `crates/reflection/tests/writer_back_pressure.rs`                                       | T1808           | Q8 / R7.1 acceptance.                                                              |
| `crates/reflection/tests/no_strategy_caller.rs`                                         | T1809           | Q4 / R8.1 negative-confirmation test.                                              |
| `crates/reports/tests/fixtures/build_reflection_store_7d.rs`                            | T1811           | Q3g 7-day fixture extension.                                                        |
| `crates/reports/tests/fixtures/build_reflection_store_90d.rs`                           | T1811           | Q3g 90-day fixture extension.                                                       |
| `crates/reports/tests/fixtures/build_reflection_store_1y.rs`                            | T1811           | Perf-smoke fixture (R7.2).                                                          |
| `crates/reports/tests/memory_highlights_with_lessons.rs`                                | T1810           | R4.2 + R4.4 byte-stable rendered body.                                              |
| `crates/agent/tests/no_new_bus_channel.rs`                                              | T1808           | R8.3 invariant — `agent::bus::Bus` shape unchanged.                                 |

**Existing files modified (by the developer):**

| Path                                                                  | Task    | Change                                                                                                                            |
|-----------------------------------------------------------------------|---------|-----------------------------------------------------------------------------------------------------------------------------------|
| `Cargo.toml` (workspace root)                                         | T1801   | Add `crates/reflection` to `members`.                                                                                              |
| `crates/audit/src/query.rs:36`                                        | T1801   | Add `pub async fn realized_pnl_for_trade(ledger, trade_id) -> Result<Money<Usdt>, LedgerError>` (sibling of `realized_pnl_since`). |
| `crates/agent/src/config.rs`                                          | T1807   | Add `ReflectionCfg { path, channel_capacity, enable_writer }`.                                                                    |
| `crates/agent/src/main.rs`                                            | T1807   | Wire `ReflectionWriter::new` + spawn `ReflectionWriterTask::run`; gate behind `cfg.reflection.enable_writer`.                       |
| `crates/exec/src/paper.rs`                                            | T1807   | Tap point: on a sell-side fill that brings position to zero, call `reflection_writer.try_enqueue(LessonCardWriteRequest { … })`.   |
| `crates/reports/src/lib.rs`                                           | T1810   | Forward an optional `reflection_store: Option<Arc<dyn ReflectionStore>>` field on the `generate(...)` arg-struct.                  |
| `crates/reports/src/render/memory_highlights.rs:6`                    | T1810   | Add `pub const REFLECTION_MEMORY_EMPTY_STATE: &str = "_no closed trades yet — lesson cards will appear after the first closed trade._\n"` (Q7-locked); add `pub fn render_with_lessons(decayed, lessons) -> String`; add `pub fn build_retrieval_query(pnls, current_regime) -> Option<RetrievalQuery>`. The PLACEHOLDER constant for the v1+ scope is removed; replaced by the empty-state constant + the with-lessons render path. |
| `crates/reports/Cargo.toml`                                           | T1810   | Add `reflection = { path = "../reflection" }` dev-dep + runtime-dep (the renderer needs the trait + types).                         |
| `crates/reports/tests/fixtures/build_ledger_7d.rs:1`                  | T1811   | Extend to produce 3 closed trades across 2 strategies (Q3g 7d shape).                                                              |
| `crates/reports/tests/fixtures/build_ledger_90d.rs:1`                 | T1811   | Extend to produce 10 closed trades across 3 strategies × 6×9 outcome × regime coverage.                                            |
| `crates/reports/tests/fixtures/build_ledger_1y.rs:1`                  | T1811   | Extend to seed `reflection-1y.db` for the perf-smoke fixture.                                                                      |
| `spec/dev-notes/memory-anchor-relock-TBD.md`                          | T_FINAL | Append "completed at 2026-05-08 — see spec/reflection-memory/tasks.md" footer.                                                     |
| `spec/anchors.toml:67-75`                                             | T_FINAL | **Tester only** — replace the two v1+ entries with the new captured SHA-256s. The 9 strategy anchors at lines 15–58 are byte-identical (negative invariant). |

**Existing files explicitly NOT modified (negative invariants):**

- `crates/core/src/lib.rs` — no `Strategy` trait change (R8.1).
- `crates/strategy/src/**` — no strategy-side change (R8.1, Q4).
- `crates/audit/migrations/**` — no new audit migration; `reflection.db` is a sibling file with its own migration set under `crates/reflection/migrations/` (R6.2, Q3a).
- `crates/audit/src/journal.rs` — no new account; no new writer; the reflection.db is read/written via a sibling pool (R6.1).
- `crates/agent/src/bus.rs` — no new bus channel (R8.3, hard constraint #4); the writer mpsc is a private field of `ReflectionWriter`.
- `crates/ui/src/**` — no UI surface (V10).
- `spec/anchors.toml:15-58` — the 9 strategy-backtest anchors stay byte-identical (R8.2 / V6 / hard constraint #2).

### Determinism plan (R5.1, R5.3)

Every byte that lands in the body is sourced from a deterministic path. The five known non-determinism sources and their mitigations:

| Source of non-determinism (potential)                                                       | Mitigation                                                                                                                                                                                                            |
|---------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `HashMap` iteration over cards or strategies                                                | All card-keyed structures are `BTreeMap<CardId, _>` and all strategy-keyed structures are `BTreeMap<StrategyId, _>`. Same v1+ pattern at `crates/reports/src/render/strategy_attribution.rs`.                          |
| Wall-clock leakage into card lines                                                          | `closed_at` is the only timestamp in the rendered card line and it sources from the audit ledger's `journal_transactions.ts` (RFC3339, 6-digit fractional seconds) — never from `OffsetDateTime::now_utc()`.            |
| Embedding `f64` round-off                                                                   | The embedding is `[Decimal; 32]`, no `f64`. Cosine score is `Decimal`. The `to_u32` clamp pattern from sparkline encoding (`crates/reports/src/sparkline.rs:38`) is reused for any rank-derived integer.                |
| Tie-break in retrieval ordering                                                             | Score-tie tie-break on `closed_at ASC` (older cards first) per R3.1. Asserted by `crates/reflection/tests/store_top_k_determinism.rs::t1810_score_tie_breaks_on_closed_at_ascending`.                                  |
| `chrono` / `time` timezone drift                                                            | Same v1+ rule: every timestamp is RFC3339 UTC + microsecond precision via the journal-writer format string at `crates/audit/src/journal.rs:51-56`. Cards write via the same format helper.                              |

The R5.3 negative-invariant test extends operator-success-reports'
`crates/reports/tests/body_no_volatile_metadata.rs` with one new
assertion: the rendered body containing the new `## Memory
highlights` content also contains none of the eight forbidden
substrings (`generated:`, `run_id:`, `wall_clock_s:`,
`ledger_snapshot_sha:`, `data_source:`, `agent_pid:`, `host:`,
`git_commit:`).

### Test strategy

| Layer                                            | Tests                                                                                                                                                                       | Crate(s)             | Tool         |
|--------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------|--------------|
| **Unit — regime classifier**                     | BTC closes with +3% / -3% / +1% 7d returns map to `Bull` / `Bear` / `Chop`. Boundary at exactly ±2% maps to `Chop` (strict inequality). R1.3.                              | `reflection`         | `cargo test` |
| **Unit — outcome classifier**                    | ±0.6% Win/Loss; ±0.4% Scratch; `opening_capital == 0` → Scratch. R1.4.                                                                                                       | `reflection`         | `cargo test` |
| **Unit — `card_id` content hash**                | Same fixture closed-trade in twice → same `card_id`. Different `signed_pnl` → different `card_id`. R1.1.                                                                  | `reflection`         | `cargo test` |
| **Unit — embedding determinism**                 | 1000 random `LessonCard`s embed-twice → byte-identical `[Decimal; 32]`. R2.5.                                                                                              | `reflection`         | `proptest`   |
| **Unit — `post_mortem_analyst::generate_card`**  | Fixture closed trade with known fee + qty + price → expected `LessonCard` with all R1.1 fields populated. R2.3.                                                            | `reflection`         | `cargo test` |
| **Integration — store idempotency**              | 10 deliberate closed trades → 10 cards on first run, 0 inserts on second run. R2.4.                                                                                         | `reflection`         | `cargo test` |
| **Integration — top-K determinism**              | Seed 100 cards, run `retrieve_top_k(query, 5)` twice, assert byte-identical card order. Score tie → `closed_at ASC` tie-break. R3.1, R3.2.                                  | `reflection`         | `cargo test` |
| **Integration — empty-store path**               | `retrieve_top_k` against a 0-card store returns `Ok(vec![])`. R3.4.                                                                                                          | `reflection`         | `cargo test` |
| **Unit — back-pressure**                         | Fill the 1024-capacity mpsc; assert `try_enqueue` returns `Err(TryEnqueueError::BackPressure)` and `reflection_card_dropped_total` increments. R7.1, Q8.                    | `reflection`         | `cargo test` |
| **Integration — `realized_pnl_for_trade`**       | Audit fixture with 3 closed trades; assert query returns the right `Money<Usdt>` per `trade_id`; sums equal `realized_pnl_since(period_start)`. R2.2.                       | `audit`              | `cargo test` |
| **Integration — `render_with_lessons` byte-stable** | Fixture store seeded with K=5 cards → rendered body matches a hand-computed expected string byte-for-byte. R4.2.                                                            | `reports`            | `cargo test` |
| **Integration — empty-state body byte-stable**   | Fresh ledger / 0-card store → body equals `REFLECTION_MEMORY_EMPTY_STATE`. R4.4 / Q7.                                                                                       | `reports`            | `cargo test` |
| **Integration — decay co-render**                | `render_with_lessons` composed with `render_with_decay` produces the union body when both fire. R4.1.                                                                       | `reports`            | `cargo test` |
| **Integration — body-no-volatile-metadata extended** | The new body section contains none of the 8 forbidden substrings. R5.3.                                                                                                     | `reports`            | `cargo test` |
| **Integration — V6 anchor regression**           | After Q3g fixture extension + render integration: `report-sample-7d` + `report-sample-90d` both produce byte-stable bodies across 2 runs. The 9 strategy anchors at lines 15–58 of `spec/anchors.toml` stay byte-identical (negative). R5, R8.2. | `reports`, `tester`  | `bash scripts/verify_anchors.sh` |
| **Performance smoke**                            | Top-K at K=5 against a 500-card store completes < 100ms. Report wall-clock against the 1-year fixture stays < 10s. R7.2 / R7.3.                                              | `reflection`, `reports` | `cargo test` |
| **Static — no `Strategy::*` consumer of retrieval** | grep-style negative test: no symbol from `crates/strategy/` resolves `reflection::retrieve_top_k`. R8.1, Q4.                                                                | `reflection`         | `cargo test` |
| **Static — no new bus channel**                  | `crates/agent/tests/no_new_bus_channel.rs` asserts `agent::bus::Bus` public field set unchanged from v1+ snapshot. R8.3, Q8.                                                | `agent`              | `cargo test` |

### Risk register & mitigations

| Risk                                                                                                       | Severity | Mitigation                                                                                                                                                                                                                                                                                  |
|------------------------------------------------------------------------------------------------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **R-1** Determinism leak via wall-clock in card body (`closed_at` accidentally set to render-time)         | high     | `LessonCard.closed_at` sources only from `journal_transactions.ts` (the audit ledger). Architect-level invariant tested by `crates/reports/tests/body_no_volatile_metadata.rs` — same R10.4 pattern as v1+. Same lesson as v0.5 HF-1 / v1.5a HF-1 — codified.                                |
| **R-2** Embedding non-determinism via floating-point arithmetic                                            | high     | `[Decimal; 32]` end-to-end; `f64` is forbidden in the `reflection` crate (deny-lint enforced via `#![deny(clippy::float_arithmetic)]` in `crates/reflection/src/lib.rs`).                                                                                                                    |
| **R-3** SQLite reader contention between agent's writer task and the reports binary's reader              | medium   | Reflection's writer task uses WAL mode (`PRAGMA journal_mode = WAL`); the reports binary opens with `PRAGMA query_only = 1` after open (same pattern v1+ uses for the audit DB). Concurrent reads while the writer holds the WAL write lock are safe.                                       |
| **R-4** Card-write back-pressure under burst causes silent data loss                                       | medium   | Drop is **observable** via `reflection_card_dropped_total{reason="back_pressure"}`. Operators alert on `> 0 over 1h`. Capacity 1024 is 17 hours of steady-state queue at production rate — only catastrophic burst hits the drop path.                                                       |
| **R-5** A new strategy added in a future feature changes `STRATEGY_SLOTS` order → embedding shifts → SHAs drift | medium   | `STRATEGY_SLOTS` is **append-only**. Any future feature that adds a strategy: (a) appends to the end of `STRATEGY_SLOTS`, (b) re-runs the V6 fixtures, (c) captures + re-locks the two v1+ anchors. Documented at the rustdoc note on `STRATEGY_SLOTS` in `crates/reflection/src/embedding.rs`. |
| **R-6** Reflection.db file location drift between dev / production / fixtures                              | low      | Path is computed as `cfg.reflection.path` — defaults to `<cfg.audit.path>/../reflection.db`. Tests pass an absolute path under `target/test-ledgers/`. No discoverable ambiguity at runtime.                                                                                                |
| **R-7** Q4 = report-only contract violated by a future PR (someone wires retrieval into the trader)        | low      | `crates/reflection/tests/no_strategy_caller.rs` is the static-grep negative test. Fails CI if a future PR wires the trader without a follow-up brief.                                                                                                                                       |
| **R-8** Reflection.db corruption (e.g. mid-write power loss)                                               | low      | SQLite WAL + `PRAGMA synchronous = NORMAL` per audit-DB precedent; cards are derived (re-derivable from the audit ledger), so a wipe-and-rebuild script for `reflection.db` is a follow-up runbook entry — not a v1 deliverable.                                                              |
| **R-9** Q3g fixture extension produces non-deterministic body bytes (e.g. via `HashMap` iteration in the fixture builder) | low | Fixture builders use `BTreeMap` for card-write loops. Determinism check at the V4 gate.                                                                                                                                                                                                       |

### Performance plan (R7)

| Path                                                                                | Budget    | v1 expectation                                                                                          |
|-------------------------------------------------------------------------------------|-----------|---------------------------------------------------------------------------------------------------------|
| `try_enqueue` on the executor's hot path                                            | < 10µs    | ~500ns (no allocation; `mpsc::try_send` is atomic CAS).                                                  |
| Writer task per-card SQLite write                                                   | < 5ms     | ~1ms (WAL append; transaction per card; idempotency check is a single `SELECT 1 FROM lesson_cards WHERE card_id = ?` over the unique index). |
| `retrieve_top_k(K=5)` over 500-card store                                           | < 100ms   | ~3ms (linear scan, 32-dim cosine, `BinaryHeap`-of-K).                                                    |
| `retrieve_top_k(K=5)` over 5000-card store (forward-compat envelope)                | < 1s      | ~30ms.                                                                                                   |
| `report-sample-90d` total wall-clock (R7.3 R13 from operator-success-reports)       | < 10s     | ~3s (v1+ baseline ≈ 2–3s + ≤ 100ms top-K + ≤ 50ms render).                                               |
| RSS for the 1-year fixture                                                          | < 256 MiB | ~55 MiB (v1+ baseline ≈ 50 MiB + ~5 MiB for `reflection.db`'s 500 cards × ~10KB row).                    |

### Mapping R/V → tasks

| R-item / V-item                                              | Tasks                                                                                  |
|--------------------------------------------------------------|----------------------------------------------------------------------------------------|
| R1.1 lesson-card data model                                  | T1801, T1802, T1803, T1804                                                             |
| R1.2 immutable cards                                         | T1805, T1806                                                                           |
| R1.3 regime tag                                              | T1802                                                                                  |
| R1.4 outcome class                                           | T1802                                                                                  |
| R2.1 persistent store                                        | T1805                                                                                  |
| R2.2 `realized_pnl_for_trade` audit query                    | T1801                                                                                  |
| R2.3 `post_mortem_analyst::generate_card`                    | T1804                                                                                  |
| R2.4 store idempotency                                       | T1806                                                                                  |
| R2.5 deterministic embedding                                 | T1803                                                                                  |
| R3 retrieval API                                             | T1809, T1810                                                                           |
| R4 report integration                                        | T1810                                                                                  |
| R5 determinism + anchor re-lock                              | T1812, T1813, T_FINAL_REFLECTION_MEMORY                                                |
| R6 reconciliation invariant preserved                        | T1812 (negative confirmation)                                                          |
| R7.1 hot-path off-budget                                     | T1807, T1808                                                                           |
| R7.2 retrieval perf                                          | T1810, T1811                                                                           |
| R7.3 / R7.4 report wall-clock + RSS                          | T1811, T1812                                                                           |
| R8.1 no `Strategy` trait change                              | T1809 (negative-confirmation test)                                                     |
| R8.2 9-anchor byte-identical                                 | T1812, T_FINAL_REFLECTION_MEMORY                                                       |
| R8.3 no new bus channel                                      | T1808 (negative-confirmation test)                                                     |
| R8.4 zero LLM tokens                                         | T_FINAL_REFLECTION_MEMORY (V8 gate)                                                    |
| V1 static checks                                             | T_FINAL_REFLECTION_MEMORY                                                              |
| V2 cargo test                                                | T1801–T1814, T_FINAL_REFLECTION_MEMORY                                                 |
| V3 both scenarios run                                        | T1812, T1813, T_FINAL_REFLECTION_MEMORY                                                |
| V4 body determinism                                          | T1810, T1812                                                                           |
| V5 reconciliation invariant                                  | T1812                                                                                  |
| V6 11 / 11 anchor PASS                                       | T1813, T_FINAL_REFLECTION_MEMORY                                                       |
| V7 audit-query API surface preserved                         | T1801                                                                                  |
| V8 cost telemetry zero                                       | T_FINAL_REFLECTION_MEMORY                                                              |
| V9 perf                                                      | T1811, T1812                                                                           |
| V10 no-UI invariant                                          | T_FINAL_REFLECTION_MEMORY                                                              |

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
- 2026-05-08 (architect): appended `## Design` section resolving
  the six [ARCHITECT-DECIDE] questions —
  - **Q2 → new SQLite table inside a sibling `reflection.db`,
    linear-scan top-K, behind a `ReflectionStore` trait** (analyst
    strawman pinned). v2 swap to qdrant or sqlite-vss is a
    single-trait-impl change.
  - **Q3 → analyst strawmans pinned across the bundle** —
    sibling `reflection.db` (Q3a), BTC 7d return ±2% regime
    classifier (Q3b), ±0.5% Win/Loss outcome thresholds (Q3c),
    deterministic 32-dim hand-crafted embedding with pinned slot
    layout (Q3d), K = 5 (Q3e), largest-abs-PnL strategy + symbol
    retrieval scoping (Q3f), 7d ≥3 / 90d ≥10 fixture content with
    a 6×9 outcome × regime coverage matrix on 90d (Q3g).
  - **Q4 → report-only this round** (analyst strawman pinned).
    Trader-side wiring is a follow-up brief
    `reflection-memory-trader-wiring`. Strategy trait shape and
    the 9 strategy backtest anchors stay byte-identical.
  - **Q5 → defer to follow-up brief** `reflection-memory-distillation`
    (analyst strawman pinned). Distillation needs cards on disk
    to cluster + an LLM consumer of the prompt library; both are
    follow-ups.
  - **Q6 → confirmed scope = two `report-sample-*` v1+ anchors only**.
    The 9 strategy backtest anchors at `spec/anchors.toml:15-58`
    are byte-identical post-feature; T1812 is the negative-
    confirmation gate.
  - **Q8 → bounded tokio mpsc (capacity 1024), `try_send`,
    Prometheus counter `reflection_card_dropped_total`** on drop
    (analyst strawman pinned + capacity sized).
  Expanded `tasks.md` with 14 developer T18xx tasks (T1801–T1814)
  + `T_FINAL_REFLECTION_MEMORY`. Crate / module surface lists 23
  new files and 11 modified existing files. Owner → architect;
  status stays `in-progress`.

HANDOFF → architect
Input files: spec/reflection-memory/feature.md, spec/reflection-memory/tasks.md
Open questions: Q1–Q9 (see "## Notes / Open questions").
