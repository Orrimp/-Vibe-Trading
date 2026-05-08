---
slug: reflection-memory
status: in-progress
owner: analyst
updated: 2026-05-08
---

# Tasks — Reflection memory

High-level milestones derived from
[spec/reflection-memory/feature.md](feature.md). **Numbered T18xx
developer tasks are NOT enumerated here yet** — that's the architect's
job. The architect expands these milestones into ordered T18xx tasks
(with dependencies, parallelism gates, and synchronization points)
**after the operator and architect resolve the nine open questions
Q1–Q9** in
[feature.md → Notes / Open questions](feature.md#notes--open-questions).

Task numbering convention: **T18xx** so the v0 T0xx, v0.5 T5xx, v1
T6xx, v1.5a T7xx, and v1+ T8xx namespaces stay intact (operator
success reports occupies T8xx). The reflection-memory expansion is
expected to be **comparable to v1+'s T801–T817 in scope** — roughly
12–18 developer tasks across data model, persistence, retrieval,
report integration, fixture extension, and anchor re-lock.

## Milestones

### M1 — Lesson-card data model + audit query addition

Covers feature.md **R1** (data model) + **R2.2** (the new
`audit::query::realized_pnl_for_trade` additive query). Architect's
Q3a/Q3b/Q3c resolutions land here (storage location, regime
classifier shape, outcome thresholds).

**Likely T18xx tasks (architect to expand):**
- `LessonCard`, `RegimeTag`, `OutcomeClass`, `RetrievalQuery`
  Rust types in a new module (architect picks crate — likely a new
  `crates/reflection/` leaf crate or an additive module under
  `crates/reports/`).
- Regime classifier pure function over BTC close prices.
- Outcome classifier pure function over `signed_pnl_usdt` +
  opening capital.
- `audit::query::realized_pnl_for_trade(ledger, trade_id)` additive
  query (mirror of `realized_pnl_since`).
- Card-id content hash (deterministic byte-stable).

**Acceptance for M1:** types compile; `cargo test -p <crate>` clean
on the data-model + classifier surfaces; the new audit query is
unit-tested against a fixture ledger; existing audit-query callers
unchanged.

### M2 — Persistence layer + writer wiring

Covers feature.md **R2** (card persistence) + **R7.1** (off-hot-path
write channel). Architect's Q2 (vector store choice) and Q8
(card-write channel + back-pressure) resolutions land here.

**Likely T18xx tasks:**
- `ReflectionStore` trait (read-only retrieval API + idempotent
  write API).
- v1 `ReflectionStore` implementation per Q2 resolution
  (analyst's prior: new SQLite table + linear-scan top-K).
- Embedding function (Q3d) — deterministic 32-dim hand-crafted
  vector or whatever the architect's Q3d resolution names.
- `post_mortem_analyst::generate_card(closed_trade, ledger) ->
  LessonCard` deterministic pure function.
- Writer task in the agent's main loop, mpsc-fed from the
  executor fill-handler (Q8 resolution).
- Idempotency + dedup test surface.

**Acceptance for M2:** integration test seeds N=10 closed trades,
asserts N cards land + idempotent re-write produces 0 inserts;
back-pressure smoke (writer backlog under synthetic burst).

### M3 — Retrieval API + report integration

Covers feature.md **R3** (top-K retrieval) + **R4** (report
integration). Architect's Q3e (default K) and Q3f (retrieval-query
scoping rule at report time) and Q7 (empty-state wording)
resolutions land here.

**Likely T18xx tasks:**
- `reflection::retrieve_top_k(store, query, k)` deterministic
  top-K with tie-break on `closed_at` ASC.
- Empty-store graceful path (`Ok(vec![])`).
- `crates/reports/src/render/memory_highlights.rs::render_with_lessons`
  replaces `render()` for the live path; keeps `render_with_decay`
  composed.
- Body shape per R4.2 — byte-stable card lines, decay co-render.
- Empty-state body string (Q7 wording).
- Report-time retrieval-query construction per R4.3 (largest-abs-PnL
  strategy + symbol + `period_end` regime; tie-break ASC).

**Acceptance for M3:** unit test against 100-card fixture asserts
top-K determinism + tie-break order; rendered body byte-stable
across two runs; empty-store body byte-stable.

### M4 — Fixture extension + anchor re-lock

Covers feature.md **R5** (determinism + anchor re-lock) + **R8.2**
(9 strategy-backtest anchors stay byte-identical). The architect's
Q3g (fixture content extension) and Q6 (re-lock scope = two anchors
only) resolutions land here. **This milestone is explicitly the
v1.5a T717 precedent** — same anchor-re-lock pattern, scoped to the
two `report-sample-*` v1+ anchors only.

**Likely T18xx tasks:**
- Extend `crates/reports/tests/fixtures/build_ledger_7d.rs` so
  the 7-day window contains ≥3 closed trades across ≥2 strategies
  (Q3g resolution).
- Extend `crates/reports/tests/fixtures/build_ledger_90d.rs` so
  the 90-day window contains ≥10 closed trades exercising
  Win/Loss/Scratch + Bull/Bear/Chop coverage.
- Re-run both scenarios; tester captures new body-SHA-256s.
- **Replace** the `report-sample-7d` and `report-sample-90d`
  entries at [`spec/anchors.toml`](../anchors.toml) lines 67–75
  with the new SHAs. Same single-edit pattern as v1.5a T717.
- Verify the 9 strategy-backtest anchors at lines 15–58 stay
  byte-identical (negative confirmation).
- Update [`spec/dev-notes/memory-anchor-relock-TBD.md`](../dev-notes/memory-anchor-relock-TBD.md)
  with a "completed at <date>" footer note pointing to this
  feature's tasks.md.

**Acceptance for M4:** `scripts/verify_anchors.sh` returns
`ANCHORS PASS (11 / 11)` with the new SHAs; the 9 strategy
anchors are byte-identical; determinism gate (R5.1) and
reconciliation gate (R6) both green on the re-locked bodies.

### M5 — Ship: VERDICT → PASS

Covers feature.md **V1–V10** (Verification matrix). Tester closes
the loop.

**Likely T18xx tasks:**
- `T_FINAL_REFLECTION` — tester end-to-end gate. Mirror the
  v1+ `T_FINAL_REPORTS` shape from
  [`spec/operator-success-reports/tasks.md`](../operator-success-reports/tasks.md):
  static-checks + workspace tests + both report scenarios + V4
  determinism + V5 reconciliation + V6 11/11 anchor PASS + V7
  audit-query surface preserved + V8 cost-telemetry zero + V9
  perf budget + V10 no-UI invariant.
- Status flip `in-progress → shipped`; owner flip `tester →
  shipped`; appended Changelog row.
- Presenter follow-up: `present-results` skill assembles
  `spec/reflection-memory/presentations/reflection-memory-<date>.md`
  for operator approval (post-FINAL gate, per AGENT.md).

**Acceptance for M5:** all V1–V10 VERIFIED in the tester report;
operator's "[x] Approved — ship" recorded in the presenter deck.

## Tasks T18xx will be expanded by the architect after Q-resolution

The architect's Design section in
[feature.md](feature.md) is expected to:

1. Land Q1–Q9 resolutions (with operator input on Q1, Q7, and Q9 as
   relevant — see the [OPERATOR-DECIDE] tags on those questions).
2. Expand each milestone above into ordered T18xx tasks with
   per-task acceptance criteria and dependency arrows.
3. Add a Parallelism map (mirroring
   [`spec/operator-success-reports/tasks.md` → Parallelism map](../operator-success-reports/tasks.md)).
4. Specify the synchronization gates (which task blocks which).

Until then, this file is a **stub** documenting the milestone
shape only.

## Notes

- **No `[ui-designer]` task is expected.** Per
  [feature.md → R8.3](feature.md#r8--no-regression-in-non-report-code-paths)
  and the precedent at
  [`spec/operator-success-reports/tasks.md` → "Handoff contract — no
  UI involvement"](../operator-success-reports/tasks.md), the cockpit's
  `viewer` binary already renders report markdown inline; no widget
  or fixture change is required.
- **No backtest-binary change.** The two re-locked anchors live
  under `spec/operator-success-reports/reports/`, not under any
  strategy-backtest path. The 9 v0/v0.5/v1/v1.5a anchors are
  untouched per R8.2 / V6.
- **No new bus channel** beyond the internal mpsc the card-writer
  task may use (Q8). No new audit ledger account; no new chart of
  accounts entry.
- **No LLM dependency in v1** per Q1's analyst recommendation
  (Option A). If the operator picks Q1 = Option B (LLM-enabled),
  the architect inserts an additional milestone before M2 covering
  LLM provider trait + prompt + cost-budget gating; the analyst
  brief would then need re-scoping.

## Changelog

- 2026-05-08 (analyst): initial stub. Five milestones M1–M5 named;
  developer T18xx expansion deferred to architect after Q1–Q9
  resolution. Owner → analyst; status → in-progress; awaiting
  architect signoff.
