---
slug: advisor-reflection-decision-loop
status: proposed
owner: analyst
updated: 2026-06-26
version: 0.1.0
directive: "C4 — deterministic learning loop (product.md core pillar 3; backlog.md § Future fresh program); reflection-feedback decision seam"
---

# C4 — Reflection decision-support memory surface for the advisor (the honest C4)

## Why (the directive)

`spec/product.md` names a **deterministic learning loop** as a CORE pillar
(pillar 3, "future deterministic learning loop" in § Differentiator / the
§ Pillar stack ratified 2026-05-30), and `spec/backlog.md § Future fresh
program` carries **C4 — deterministic learning loop** as *never built*: it
"would adapt param/route selection from the reflection store through the
sanctioned ADR-0041 layering seam." This feature scopes C4 **honestly** — and
the honest answer is NOT an autonomous param-mutating loop.

## The honest tension (load-bearing — read this before designing)

C4's original framing — *adapt param/route selection from the reflection
store* — is **moot as an alpha loop on this product, and a footgun if built
literally.** Three facts make this unavoidable, and the design must own them
inline rather than route around them:

1. **The active-edge hunt concluded "ship passive" (2026-06-08).** Across the
   three reachable channels no active strategy beat passive buy-and-hold net of
   cost under the frozen block-bootstrap Monte-Carlo rule
   (`product.md § Why this is honest`; `runbooks/passive-baseline.md`). The
   product's honest recommendation is **usually "just hold"**
   (`Recommendation::BenchmarkWins`, the **modal** real-crypto outcome per
   ADR-0066 — every active arm is FRAGILE).
2. **There is no live strategy whose params/routes need adapting.** `backlog.md`
   states C4 plainly: **"Moot while passive is the shipped strategy."** A loop
   that mutates SMA windows or re-orders which family to try first per regime is
   adapting a knob on an arm that the gate keeps rejecting anyway. It would
   manufacture motion, not edge — and it is **exactly the overfit footgun** the
   whole codebase's "measured robustness, not asserted alpha" core exists to
   prevent (the pre-registration discipline in `advisor-combination-search`,
   `advisor-signal-library-expansion`, `advisor-short-selling` is the same
   defense).
3. **The frozen robustness gate is the credibility layer and stays frozen.** A
   closed-loop param suggester that fed "robust-tuned" configs back as future
   bake-off defaults presumes robust configs *exist* and *recur* — they are rare
   and, where they appear, suspect of path-luck. Promoting them as defaults is a
   silent gate-bypass.

**Verdict: C4-as-autonomous-alpha-loop is moot. The genuinely-useful,
highest-integrity shippable slice is a deterministic DECISION-SUPPORT MEMORY
SURFACE** — read the existing reflection store at the advisor's decision points
(the bake-off Leaderboard, the Tune editor, the forward Plan) and tell the
operator, deterministically, **"you've tested this coin/strategy before; here's
what the gate said last time."** It *informs* a human decision; it does not
*take* one. It is still a "deterministic learning loop" in the honest sense —
outcomes recorded as lessons feed back to improve the *next decision* — but the
adapter is the **operator's judgement**, not an autonomous mutator. This
reframing mirrors the LLM treatment the operator already ratified across the
advisor (F9 narration: the LLM *renders* the structured result, never *enters*
the ranking) — memory **surfaces** prior evidence, never *re-ranks* the
bake-off.

If the operator wants the literal C4 (autonomous param/route adaptation), this
brief argues against it on integrity grounds and records it as an explicitly
**out-of-scope, gated** follow-on (see § Out of v1) — not as v1.

## What `crates/reflection` provides today (grounded — the loop is ~90% built)

The surprising, load-bearing finding from the code audit: **the reflection loop
is NOT just "store + retrieval." Both the WRITE side and the READ side are
wired end-to-end and shipped.** C4's net-new is small.

### The data — `LessonCard` (`crates/reflection/src/types.rs:79`)

A `LessonCard` is a **deterministic post-mortem of one closed trade**, derived
from the audit ledger (not a forecast). Fields (`types.rs:79-103`): `card_id`
(sha256 content-hash, idempotent — `types.rs:133`), `closed_at`,
`symbol_or_pair`, `strategy_id`, `signed_pnl`, `opening_capital`,
`holding_period_bars`, `entry_regime`, `exit_regime`, `outcome_class`
(Win/Loss/Scratch), `note` (reserved for v2 LLM enrichment, always `None` in
v1). Every field is deterministic over the closed-trade input.

### The embedding — 32-dim, deterministic (`crates/reflection/src/embedding.rs:81`)

`embed(card) -> [Decimal; 32]` (`EMBEDDING_DIM = 32`, `embedding.rs:51`), pure /
byte-identical across runs. Slots (`embedding.rs:57-129`): 0-6 strategy one-hot
(`STRATEGY_SLOTS` = the **exact 6 base strategies** `sma_crossover` / `macd_trend`
/ `rsi_reversion` / `bbands_mean_revert` / `top10_momentum_h1` / `pairs_mr_h1`
+ `(unattributed)`, append-only), 7-9 + 18-19 entry-regime one-hot (Bull/Bear/
Chop/Volatile/Calm, ADR-0049 byte-identity contract), 10-12 outcome one-hot,
13 pnl-sign, 14 log|pnl|, 15 log holding-period, 16/17 pair / single-symbol
hash-norm, 20-31 reserved-zero. **Key consequence for C4: similarity retrieval
by (symbol + regime + strategy + outcome) already works** — the embedding
encodes exactly the keys a "have I tested this coin/strategy/regime before?"
surface needs.

### Retrieval — `retrieve_top_k` (`crates/reflection/src/retrieval.rs:22`)

Cosine-ranked top-K over the embedding, filtered by a `RetrievalQuery`
(`types.rs:108`: `strategy_id` + `symbol_or_pair` + `current_regime`), with a
deterministic tie-break on `closed_at` ascending (test
`store_top_k_determinism.rs`). Plus two recency reads in
`crates/reflection/src/query.rs`: `open_and_list_recent(db_path, limit)`
(`query.rs:41`, opens the sqlite pool inside the crate boundary, fail-soft to
`Ok(vec![])` on a missing DB) and `list_recent_lesson_cards(pool, limit)`
(`query.rs:88`, `ORDER BY closed_at DESC`).

### The store — sqlite (`crates/reflection/src/store/sqlite.rs`)

`SqliteReflectionStore` implements the 3-method `ReflectionStore` trait
(`store.rs`); schema at `crates/reflection/migrations/001_lesson_cards.sql`;
`NullReflectionStore` is the no-op for tests.

### The regime tagger (`crates/reflection/src/regime.rs:109`)

`classify_regime(...) -> RegimeTag` — deterministic, 5-state (Bull/Bear/Chop/
Volatile/Calm), used by `generate_card` to stamp entry/exit regime.

### The WRITE side IS WIRED (this is the key audit finding)

The loop's write side — recording decisions+outcomes as lessons — is **already
plumbed and shipped**, NOT a stub:

- **Producer:** `crates/exec/src/paper.rs::on_trade_close` (`paper.rs:123`) taps
  every sell-side trade-close fill in the **paper engine** and calls
  `writer.try_enqueue(request)` (`paper.rs:127`) — back-pressure-safe
  (`mpsc::try_send`, test `writer_back_pressure.rs`). 3 enqueue sites.
- **Consumer:** `ReflectionWriterTask::run` (`crates/reflection/src/writer/task.rs:38`)
  drains the bounded mpsc, calls `post_mortem_analyst::generate_card`
  (`crates/reflection/src/post_mortem_analyst.rs:35` → `classify_regime` +
  `classify_outcome`) → `store.upsert` (idempotent on `card_id`).
- **Wiring:** the writer task is spawned in `crates/agent/src/main.rs` (regression
  test `crates/agent/tests/reflection_wiring_regression.rs`); the regime daily-
  closes feed is loaded in `crates/agent/src/runtime.rs:1598` (warn-only on
  miss).

**The honest nuance C4 must respect:** lessons are written from **forward
paper-trade closes** (journey step 5), NOT from bake-off backtests — the
bake-off path (`crates/backtest/src/bakeoff`, `crates/agent/src/plan.rs`)
**writes no lessons** (verified: zero reflection touch). So today's store
records *"how the crowned strategy actually did when you paper-traded it
forward,"* not *"every backtest you ever ran."* This shapes what the surface can
honestly claim (see § What the surface says — and what it must NOT).

### The READ side IS WIRED too — the Memory screen (`crates/ui/src/screens/memory.rs`)

The cockpit **Memory** screen shows lessons from the **real** store: boot-time
hydration at `crates/ui/src/bin/cockpit_live.rs:872-921` calls
`reflection::query::open_and_list_recent(&db_path, 50)` → maps to the UI's own
`LessonCardCard` (`crates/ui/src/memory/state.rs:44` — deliberately **distinct
from** `reflection::LessonCard` to avoid leaking the reflection dep into `ui`,
`state.rs:39`) → `Message::MemoryHydrate(Vec<LessonCardCard>)`
(`crates/ui/src/state.rs:2123`). It renders each lesson as a card (symbol,
strategy, signed P&L, outcome, closed-at) with filter/view modes
(`MemoryViewMode`, `MemoryFilter`).

**So what is genuinely missing?** The Memory screen is a **standalone** browser,
**disconnected from the decision moment.** Nothing queries reflection at the
advisor's actual decision points — verified: `leaderboard.rs`, `tune.rs`,
`forward_plan.rs`, and `crates/backtest/src/bakeoff` carry **zero** reflection
reads. That disconnect — *not* a missing store and *not* a missing write side —
**is the C4 gap.**

## The proposed honest scope (v1): a deterministic memory surface at the decision points

C4 v1 = **read the existing reflection store at the three advisor decision
points and surface the relevant prior lessons in-context, deterministically.**
No autonomous mutation, no re-ranking, no gate bypass.

### S1 — Leaderboard memory chip (the primary surface)

When the Leaderboard renders a crowned/ranked pick for `(symbol, regime)`, query
the store (recency + `retrieve_top_k` by symbol+regime+strategy) and show a
**deterministic, factual** chip per surfaced strategy: *"You've paper-traded
`macd_trend` on XRPUSDT before — last forward run closed LOSS (−$X) in a Bear
regime."* If the store has no matching lessons (the dominant fresh-workstation
path — `open_and_list_recent` returns `Ok(vec![])`), the chip simply does not
render (no empty-state noise on the Leaderboard). The chip **annotates**; it
**never** reorders the ranking or overrides the gate.

### S2 — Tune editor memory note

When the operator opens the Tune editor for a `(coin, strategy-family)`, surface
prior lessons for that family on that coin: *"Last time you forward-ran a tuned
`macd_trend` on XRPUSDT it closed FRAGILE-arm LOSS."* This is the operator's
ask in the directive — a memory-surfacing aid on the Tune screen, not an
auto-tuner.

### S3 — Forward-plan memory context (optional within v1)

On the forward-Plan surface (F6), show "the last time this exact stance ran
forward, here's what happened." Lowest priority of the three; the architect may
defer S3 to a v0.2 of this feature if budget tightens.

### What the surface says — and what it must NOT (the honesty contract)

- **It says** (factual, deterministic, sourced from real recorded outcomes):
  "tested before / not tested before," the prior **outcome class** (Win / Loss /
  Scratch), the prior **signed P&L**, the prior **regime**, the prior
  **strategy**. All already on the `LessonCard`.
- **It must NOT** say or imply: "this will work again" / "expected return" /
  "win probability" / any forward prediction. Past lessons are
  past-performance, full stop. The same not-advice +
  past-performance-not-indicative disclaimer the rest of the advisor carries
  (`product.md § What this product IS NOT`) applies, plus an explicit
  "informational memory, not a recommendation" label on the chip.
- **It must NOT** alter the ranking, the crown, the fragility gate, or the
  benchmark. `BenchmarkWins` / `AllFragile` reachability is **UNCHANGED**. The
  surface is strictly downstream of `rank_candidates`, exactly like F9
  narration.
- **Honest scoping of the claim:** because lessons come from *forward
  paper-runs* not *bake-off backtests*, the surface honestly says "you've
  **paper-traded** this before," NOT "you've **backtested** this before." A
  fresh operator who has only ever run bake-offs (never started a forward run)
  will see **no** memory chips — and that is correct, not a bug. (Whether to
  *also* record bake-off results as lessons is an open decision, Q3 — it
  changes the corpus the surface draws on.)

## Reuse vs new

**Reuse (≈ everything heavy):** the entire `crates/reflection` crate
(`LessonCard`, `embed`, `retrieve_top_k`, `open_and_list_recent`, the sqlite
store, `classify_regime`); the **write side end-to-end** (paper-engine tap →
writer task → store — untouched); the Memory screen's `LessonCardCard` mapping
(`crates/ui/src/bin/cockpit_live.rs:872`) as the proven `reflection → ui` read
pattern to copy.

**Net-new (small, additive):**
1. A **trader-layer read helper** that, given `(symbol, regime, strategy)`,
   returns the relevant prior lessons for a decision point (thin wrapper over
   `retrieve_top_k` + `open_and_list_recent`) — see § ADR-0041 seam for WHERE it
   lives.
2. The **three decision-point surfaces** (S1 Leaderboard chip, S2 Tune note,
   S3 forward-plan context) — UI render + a `Message::…Hydrate` mirror per
   screen, copying the Memory-screen boot-hydrate pattern.
3. **(Conditional, Q3)** if the operator wants bake-off results recorded as
   lessons too: a write-side tap on the bake-off path — but this is an
   open decision, defaulting OFF for v1.

## The ADR-0041 layering seam (the sanctioned reflection→decision seam — non-negotiable)

ADR-0041 (`spec/architecture/adr/0041-trader-crate-split.md`, accepted
2026-05-26) **forbids the analyst-layer `crates/strategy` from consuming
reflection-retrieval** — "memory-aware decision synthesis is a trader-layer
concern." It created `crates/trader/` as the legitimate consumer and **dropped
the `reflection` path-dep from `crates/strategy/Cargo.toml`** so the rule is
*structurally* enforced (strategy literally cannot link reflection). A defensive
gate, `crates/reflection/tests/no_strategy_caller.rs::t1809…`, fails CI if any
`crates/strategy/src/**` file references `reflection::retrieve_top_k` /
`reflection::store::` / `reflection::ReflectionStore`; its sibling T1810 asserts
`crates/trader/src/**` keeps at least one `retrieve_top_k` consumer.

**What this binds for C4:**
- The new read helper (net-new #1) **MUST live in `crates/trader`** (or a
  similarly-sanctioned non-analyst layer) — **NOT** in `crates/strategy`, and
  **NOT** in `crates/ui` (which has no sqlx/reflection dep and must keep its own
  `LessonCardCard` boundary, `crates/ui/src/memory/state.rs:39`). The `ui`
  surfaces receive already-mapped UI structs via a `Message`, exactly as the
  Memory screen does today.
- The seam is **read-only decision support**: reflection → (trader read helper)
  → message → ui. It does **not** feed back into `crates/strategy` signal
  generation or the bake-off ranking — which is precisely why it does not
  re-open the C4-as-alpha-loop footgun, and why it keeps the frozen gate frozen.
- **Architect M-T1 lock to confirm:** whether the new helper extends
  `crates/trader` (the ADR-0041-sanctioned home) directly, or whether the
  bake-off-result write tap (Q3, if approved) needs its own ADR amendment for a
  *write* from the bake-off path. The *read* surfaces need no new ADR — they sit
  squarely inside the ADR-0041 trader-layer seam.

## Anchor safety

**119/119 anchored backtest body-SHAs stay byte-identical.** C4 v1 touches **no
`write_report` path** — it only **reads** the reflection store and renders UI.
The bake-off / backtest scenario emitters that produce the anchored reports are
not modified (the read helper sits downstream of them; the surfaces are pure UI
adds). Verified baseline: `bash scripts/verify_anchors.sh` → **ANCHORS PASS
(119/119)** at scoping time. The conditional Q3 bake-off write-tap, **if**
approved, must be scoped to write only to the *reflection* sqlite store (never
to a `spec/*/reports/` anchored file), preserving the invariant — but Q3
defaults OFF for v1 precisely to keep the anchor surface trivially clean.

## The honest framing (carry this inline in every downstream artifact)

C4-as-autonomous-alpha-loop is **moot on a ship-passive product** and a
**pre-registration footgun**. The valuable, shippable, high-integrity slice is a
**deterministic decision-support memory surface** that reuses the already-built
reflection loop (write side + store + retrieval + the Memory screen's read
pattern) to inform the operator at the bake-off / Tune / forward-plan decision
points with **factual, sourced, past-only** lessons — never a prediction, never
a re-ranking, never a gate bypass. It is a "learning loop" in the honest sense
(recorded outcomes improve the *next human decision*), with the **operator** as
the adapter. The deliverable is **honest memory in-context**, NOT alpha. A store
that is often empty for a given coin (no prior forward run) is the **expected,
correct** state — and the surface degrades silently to "nothing to show," which
is itself an honest answer.

## Open decisions (for architect / operator)

- [ ] **Q1 — Scope shape (the central fork).** **(a, Recommended)** ship the
  **decision-support memory surface** (S1-S3) — the honest, durable slice that
  reuses the built loop and never re-ranks; **(b)** ship a **regime-routing**
  adapter (which family to try first per regime from past lessons) — *rejected
  for v1*: moot while the live field is all-Fragile, and it edges toward
  changing the bake-off (gate-bypass risk); **(c)** ship a **closed-loop param
  suggester** (robust-tuned configs → future bake-off defaults) — *rejected for
  v1*: presumes robust configs recur (they are rare + path-luck-suspect), a
  silent gate bypass. Recommended **(a)** on durability + integrity grounds; it
  carries no v0.X follow-on cleanup commitment and never touches the frozen gate.
  *If-budget-tightens:* ship **S1 only** (the Leaderboard chip — the single
  highest-value surface), defer S2/S3 to a v0.2 of this feature.
- [ ] **Q2 — Which screens, and in what order.** Recommend S1 (Leaderboard)
  first as the primary surface, then S2 (Tune — the operator's explicit ask),
  then S3 (forward-plan, optional/deferrable). Operator to confirm the screen
  set + priority.
- [ ] **Q3 — Record bake-off results as lessons too?** Today lessons come ONLY
  from forward paper-trade closes, so the surface says "you've **paper-traded**
  this before," never "you've **backtested** this before." **(a, default OFF for
  v1)** keep write side as-is — the surface honestly reflects forward-run
  history only, anchor surface stays trivially clean; **(b)** add a bake-off
  write-tap so every backtest also lands a lesson — richer corpus + the surface
  fires for operators who've only run bake-offs, BUT needs an ADR-0041 *write*
  seam decision, a new `OutcomeClass` semantics call for backtest-vs-forward
  lessons, and careful anchor scoping (write to reflection sqlite only). Recommend
  **(a)** for v1; **(b)** as a clean v0.2 once the read surfaces prove valuable.
- [ ] **Q4 — Recency vs similarity at each surface.** `retrieve_top_k`
  (cosine by symbol+regime+strategy) vs `open_and_list_recent` (most-recent N) —
  or both. Recommend **`retrieve_top_k` filtered to the surfaced strategy +
  current regime** for S1/S2 (most relevant), with a recency fallback when the
  embedding match is thin. Architect to lock per surface.
- [ ] **Q5 — Empty-store behaviour.** Confirm the surface **renders nothing**
  (no chip, no note) when there are no matching lessons — NOT an empty-state
  placeholder at the decision point (that would add noise to the Leaderboard).
  Recommended: silent absence. Operator to confirm.

## Changelog

- 2026-06-26 (analyst, C4 scoping): scoped **C4 — the deterministic learning
  loop** (`product.md` core pillar 3; `backlog.md § Future fresh program`,
  flagged *"moot while passive is the shipped strategy"*) **honestly**. Verdict:
  C4-as-autonomous-alpha-loop is **moot on a ship-passive product** and a
  pre-registration footgun (no live strategy whose params/routes need adapting;
  mutating a knob on an all-Fragile field manufactures motion, not edge); the
  genuinely-useful, highest-integrity shippable slice is a **deterministic
  decision-support memory surface** that reads the **already-built, already-wired
  reflection loop** at the advisor's decision points (Leaderboard / Tune /
  forward-plan) and tells the operator, factually and past-only, *"you've
  paper-traded this coin/strategy before; here's what the gate said"* — never a
  prediction, never a re-ranking, never a gate bypass (the operator is the
  adapter, mirroring the F9 narration-only treatment). **Load-bearing code audit
  findings:** (1) the reflection **WRITE side is wired end-to-end** — paper-engine
  trade-close tap (`crates/exec/src/paper.rs::on_trade_close:123` →
  `try_enqueue`) → `ReflectionWriterTask::run`
  (`crates/reflection/src/writer/task.rs:38`, spawned in
  `crates/agent/src/main.rs`) → `post_mortem_analyst::generate_card` →
  `store.upsert`; (2) the **READ side is wired** — the cockpit Memory screen
  hydrates from the **real** store via
  `reflection::query::open_and_list_recent` (`crates/ui/src/bin/cockpit_live.rs:872`);
  (3) the **actual gap** is that the decision-point screens (`leaderboard.rs`,
  `tune.rs`, `forward_plan.rs`) + the bake-off path carry **zero** reflection
  reads — the Memory screen is a standalone browser, disconnected from the
  decision moment; (4) lessons today come **only from forward paper-runs**, not
  bake-off backtests, which honestly bounds what the surface can claim (Q3). The
  **ADR-0041 seam** binds the net-new read helper to **`crates/trader`** (NOT
  `crates/strategy` — gate `no_strategy_caller.rs::t1809`; NOT `ui`), read-only,
  not feeding back into signal generation or the ranking. **Anchor-safe by
  construction** (`write_report` paths untouched, v1 reads only → 119/119 held,
  verified at scoping). The **frozen robustness gate stays frozen**;
  `BenchmarkWins`/`AllFragile` reachability **UNCHANGED**. Does NOT change
  § What this product IS / IS NOT, the journey, D1-D5, or the 2026-06-08
  ship-passive verdict — it is the honest realization of the long-deferred
  pillar-3 learning loop as decision support, not autonomous alpha. No engine
  code; no anchored content touched; `spec/trace.toml` + `spec/product.md`
  intentionally **not** touched (orchestrator reconciles; sibling analyst
  scoping in parallel).
