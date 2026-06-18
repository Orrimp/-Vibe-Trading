---
slug: point-in-time-data-discipline
status: proposed
owner: analyst
updated: 2026-06-18
version: 0.1.0
---

# Point-in-time / as-of data discipline

A **structural** guarantee that signal/feature computation cannot read future
(look-ahead) data. Today the project enforces point-in-time (PIT) cleanliness
*by hand*, feature by feature — a binary-search as-of join copy-pasted three
times, plus per-spike `--leak-check` falsifiers. This feature consolidates that
informal seam into **one reusable, guarded as-of-join API** and a **self-proving
look-ahead falsifier**, so "join future data onto a bar" becomes hard or
impossible by construction rather than a discipline each future signal must
re-earn.

This is a **data-discipline hardening of the EXISTING pipeline**. It is NOT a new
database, NOT a PIT-data vendor integration, and NOT a re-opening of the
concluded active-edge search. See [§ Out of scope](#out-of-scope).

---

## Why

### The qlib gap this closes

The [qlib feature-gap analysis](../dev-notes/qlib-feature-gap-2026-06-17.md)
(2026-06-17) compared Microsoft qlib against this project and found that almost
all of qlib's surface area is *alpha-prediction* machinery this project tested
and retired. The honest residue was three or four genuinely-scope-fitting gaps,
and **gap #1 — a first-class point-in-time / as-of data discipline — is named as
"the one structurally-worthwhile gap"** (qlib note § Genuinely-relevant gaps,
ranked, #1). The note's verdict, verbatim:

> Today PIT-cleanliness is re-proven per feature by hand (leak-checks, day-1
> falsifiers). qlib bakes it into the data layer so look-ahead is impossible by
> construction. For a project whose entire credibility rests on an *honest
> negative result*, a structural "you cannot join future data" guarantee hardens
> the most important claim we make. **Likely a focused as-of-join helper + a
> lint, not a new database.** Scope-fitting; would strengthen the moat.

This feature is scoped to exactly that sentence: a focused as-of-join helper + a
falsifier (and an optional lint), **not** a new database.

### The moat it protects — Differentiator (5)

The product's ratified epistemic core is
[Differentiator (5) "measured robustness, not asserted alpha"](../product.md)
(product.md § Differentiator, § Pillar stack core pillar 2). The program's
shippable deliverable is **the robustness machine + an auditable negative result
across three orthogonal channels** (price/OHLCV, derivatives-positioning,
on-chain), under a frozen block-bootstrap decision rule, terminal-ratified
2026-06-08.

The credibility of that negative result rests on one load-bearing assumption:
**no reported result is contaminated by look-ahead.** A single silent future-data
leak in a sidecar join would not just void one number — it would put an asterisk
on the whole "active ≤ passive, honestly measured" claim, which is the entire
product. Today that assumption is defended *per feature, by hand*. The on-chain
spike's PIT leak-check is the canonical example: it is what distinguished a
causal join from a look-ahead one and let the program treat the stablecoin
FRAGILE verdict as a real signal result rather than a contaminated one
([onchain-netflow-spike § 1.3](../dev-notes/onchain-netflow-spike-2026-06-08.md)).
Making that discipline **structural** converts a per-feature manual proof into a
property the pipeline carries by construction — directly hardening the moat.

### What is manual-PIT today (the seam to formalize)

The as-of-join logic already exists, **hand-rolled and duplicated three times**,
each with its own copy of the binary-search algorithm and its own no-look-ahead
test:

| Copy | File:fn | Type | Falsifier |
|---|---|---|---|
| 1 | [`funding_as_of`](../../crates/backtest/src/funding_data.rs) (`crates/backtest/src/funding_data.rs:378`) | `&[(i64, Decimal)]` → `Vec<Option<Decimal>>` | `no_look_ahead_falsifier` (line 519) |
| 2 | [`basis_as_of`](../../crates/backtest/src/basis_data.rs) (`crates/backtest/src/basis_data.rs:397`) | `&[(i64, Decimal)]` → `Vec<Option<Decimal>>` | `no_look_ahead_falsifier` (line 554) |
| 3 | `funding_as_of` in [`basis_diag.rs`](../../crates/data/examples/basis_diag.rs) (`crates/data/examples/basis_diag.rs:219`) | `&[(i64, f64)]` → `f64` | inline `--leak-check` (line 382) |

All three implement the **same** algorithm: `partition_point(|&(t, _)| t <= bar_ts)`
to find the rightmost record at-or-before the query timestamp, returning `None`
(warm-up) when no record precedes the bar. The architect's own
`OQ-CARRY-SEM` note in `spec/trace.toml` names this seam precisely — "**existing
per-bar funding-ring + as-of join**" (REQ-HORIZON-RETEST row). This is the seam
this feature formalizes into a single guarded API, retiring the duplication.

The duplication is the risk: a **fourth** consumer (a future on-chain or
cross-asset signal — exactly the kind the qlib note #2/#3 contemplate if the
operator ever opens a fresh channel) would copy the pattern a fourth time, and
*nothing structural* forces that copy to be causal. The only guard today is that
a developer remembers to write another `no_look_ahead_falsifier`. That is the
manual discipline this feature replaces.

### What is already structurally clean (honest scoping — the gap is smaller than the qlib note implied)

A frank survey of the code shows the **price/OHLCV channel is already PIT-clean
by construction**, and the brief must not invent scope by pretending otherwise:

- The `Strategy` trait is **streaming**: `fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>`
  ([`crates/strategy/src/traits.rs:8`](../../crates/strategy/src/traits.rs)).
  A strategy is fed bars one at a time and emits signals from state it has
  accumulated; it **physically cannot read a bar it has not yet been handed.**
  Look-ahead on the price tape is impossible at this layer — there is no future
  bar in scope.
- Per-symbol rolling state is held in a [`RingBuffer`](../../crates/features/src/ring_buffer.rs)
  that exposes only `last()` / `get_back(n)` (past-only); the streaming overlays
  (`patchtst_overlay_momentum.rs`, `tcn_overlay_momentum.rs`) keep
  `windows: BTreeMap<Symbol, Vec<Bar>>` that, by construction, contains only
  bars already streamed.

So the genuine look-ahead surface is **narrower than "the data layer"** — it is
exactly two seams:

1. **Sidecar as-of joins** (funding / basis / on-chain onto bars) — the
   hand-rolled `*_as_of` family above. This is where a wrong join key or a
   forward-shifted series leaks future data. **This is the seam the typed API
   guards.**
2. **Forward-return construction** in research probes and the bootstrap
   (`r[k] = ln(close[k+1]/close[k])` at
   [`crates/data/src/synth/bootstrap.rs:217`](../../crates/data/src/synth/bootstrap.rs);
   forward windows `[t, t+L)` in `basis_diag.rs:362/434`). Forward returns are
   *intentionally* future-looking (they are the label/target, not a feature) —
   the discipline there is that they must never be fed back as a **signal**. The
   falsifier convention (a leaked-contemporaneous IC that MUST differ from the
   causal IC) is what proves the signal side stayed past-only.

The honest framing for the architect: this feature is **modest, not
foundational** — it hardens the one seam that is genuinely manual (the sidecar
as-of join) and codifies the falsifier that already exists informally. It is
worth doing because that seam is the exact place the moat-protecting claim could
silently break, and because the duplication invites a fourth uncaught copy — not
because the pipeline is currently leaking (no known leak exists; the three
hand-checks pass).

---

## Requirements

- **R1 — A single guarded as-of-join API.** Provide one reusable, typed as-of
  helper that, given a query timestamp `query_ts` and a sorted timestamped
  series, returns only the record(s) with `ts ≤ query_ts` (the most-recent
  at-or-before, `None` on warm-up). It MUST make "join a record with
  `ts > query_ts`" either impossible-by-construction (type-level) or
  caught-at-runtime (a guarded constructor / debug assertion), reusing the
  existing `partition_point` seam — NOT a parallel implementation. The three
  existing copies (`funding_as_of`, `basis_as_of`, the `basis_diag.rs` clone)
  are migrated to it (the `f64` diag copy may keep a thin local adapter; see
  Open decisions).

- **R2 — A self-proving look-ahead falsifier (the verification floor).** Ship a
  test that **deliberately feeds future data** through the API (a forward-shifted
  series, or a query against a record that post-dates `query_ts`) and asserts the
  guard **rejects or excludes** it — i.e. the causal result differs from the
  leaked result, mirroring the existing `no_look_ahead_falsifier` and the
  on-chain `--leak-check`. This falsifier is the contract: if it ever passes when
  it should fail, the guarantee is broken. It is the "day-1 falsifier" for this
  feature.

- **R3 — Migration preserves behaviour byte-for-byte where anchored.** The
  funding-carry and basis arms feed anchored backtest surfaces. The migration of
  `funding_as_of` / `basis_as_of` to the shared API MUST produce **identical**
  as-of values (same `partition_point` `≤` convention, same `Option` warm-up
  semantics, same `Decimal` precision — no f64 round-trip), so all existing
  regression anchors stay byte-identical (no anchor delta). This is a refactor
  behind a guarantee, not a numerics change.

- **R4 — (Optional, architect-decided) A look-ahead lint.** Evaluate whether a
  `scripts/`-level lint can cheaply catch obvious look-ahead patterns in feature
  code (e.g. a sidecar join that does NOT route through the guarded API, or an
  indexing pattern that reads `series[t + k]` as a signal input). If the typed
  API + falsifier already make the seam safe-by-construction, the lint may be
  judged redundant and dropped — state which, with reasoning. (See Open
  decisions: lint vs typed-API-only.)

---

## Scope

- ONE guarded as-of-join API (a small typed helper / newtype / view), living in
  a single home crate (Open decisions: core vs data vs backtest).
- Migration of the three existing hand-rolled as-of copies onto it, behaviour-
  preserving for anchored arms (R3).
- The self-proving look-ahead falsifier test (R2) as the verification floor.
- Optionally, a `scripts/`-level look-ahead lint (R4) IF the architect judges it
  adds coverage the typed API does not.
- Documentation: a one-paragraph "PIT discipline" note wherever the API lives, so
  a future signal author reaches for it by default.

## Out of scope

- **No new database / no PIT store.** The qlib note explicitly rules this out
  ("Likely a focused as-of-join helper + a lint, **not** a new database"). The
  existing pinned-parquet corpus + REVISION.toml SHA-locking is the data
  substrate; this feature does not touch it.
- **No PIT-data *vendor* integration.** No CryptoQuant / Glassnode / new-feed
  work. (The on-chain spike already established the reachable free PIT-clean
  series; that search is concluded.)
- **No re-opening the active-edge search.** This is a *discipline* feature. It
  builds no new signal, no new strategy, no new ScoreSource, and produces no new
  backtest verdict. The terminal "ship passive" conclusion (product.md, 2026-06-08)
  is untouched.
- **No change to the streaming price path.** `on_bar` is already PIT-clean by
  construction (above); this feature does not re-plumb it.
- **No change to forward-return / label construction.** Forward returns are
  intentionally future-looking targets; the falsifier convention already governs
  that they never re-enter as a signal. The API guards the *feature/sidecar* side,
  not the label side.
- **No anchored-report edits.** Per CLAUDE.md, `spec/*/reports/*.md` are
  byte-immutable; this feature touches none.

---

## Design direction

> The architect owns the final design (M-T1 lock). This section grounds it in the
> real seam and types so the design starts from the code, not a blank page.

**The seam to reuse.** Both `funding_as_of`
(`crates/backtest/src/funding_data.rs:378`) and `basis_as_of`
(`crates/backtest/src/basis_data.rs:397`) are the *same* function modulo the
doc-comment: a binary `partition_point(|&(t, _)| t <= bar_ts)` over a sorted
`&[(i64, Decimal)]`, returning `Some(series[idx-1].1)` or `None` for warm-up.
The shared API is the generalization of exactly this — same algorithm, same
`≤` causal convention, same `Option` warm-up semantics — so the migration is
mechanical and R3-preserving by construction.

**Two candidate shapes** (the type-level-vs-runtime-guard tension — architect to
resolve, tradeoff stated here):

- **(a) Type-level — a `PitSeries<T>` newtype + an `AsOf<T>` view.** Construct a
  `PitSeries` from a sorted timestamped series (constructor enforces sortedness);
  its only query method is `as_of(query_ts) -> Option<AsOf<T>>` where `AsOf<T>`
  carries the value plus the proof-timestamp `ts ≤ query_ts`. There is **no API
  surface** that returns a record at `ts > query_ts`, so look-ahead is
  unrepresentable. *Pro:* the guarantee is compile-time; a fourth consumer
  literally cannot leak. *Con:* more types to thread; the bootstrap's
  `Vec<Vec<Option<Decimal>>>` materialization (`build_funding_at_return`) and the
  `f64` diag path need adapters; slightly more typing now.
- **(b) Runtime-guard — keep the free function, add a guarded constructor +
  debug-assert.** One `as_of_join(query_ts, sorted_series) -> Option<&T>` that
  debug-asserts the series is sorted and that the returned record's `ts ≤
  query_ts`. *Pro:* minimal churn, drop-in for all three call sites, smallest
  diff. *Con:* the guarantee is a runtime/debug check, not a compile-time
  property; a future author could still call `partition_point` by hand and
  bypass it (the lint R4 becomes the backstop that closes that hole).

**Analyst lean: (a) type-level `PitSeries`/`AsOf` is the durable choice
(Recommended)** — per the durable-over-quick rule, the Recommended tag goes on
the option whose M-T1 lock carries forward across future signal versions without
amendment. (a) makes look-ahead *unrepresentable*, so the moat-protecting
guarantee holds for the fourth, fifth, Nth consumer by construction — no future
"remember to add a falsifier" discipline, no lint needed as a backstop for the
core join. It costs slightly more typing now (the `AsOf<T>` threading + two
adapters) but spawns **zero v0.2.0 cleanup brief**. **If-budget-tightens
fallback: (b) runtime-guard + the R4 lint** — ~1 day less work, drop-in
migration, but the guarantee is debug-time-only and it commits the project to
maintaining the lint as the bypass backstop (a small ongoing carry-cost). (b) is
a fully defensible cheaper path; it is the fallback label, not the Recommended
one, because a hand-rolled `partition_point` can still sidestep it.

**Where it lives (architect-decided, see Open decisions).** The seam is consumed
in `crates/backtest` (the `*_data.rs` loaders) and `crates/data` (the diag
probe). The types it joins (`Timestamp`, `Symbol`, `Decimal`) live in
`crates/core`. A `crates/core` home makes the API available to every crate
without a new dependency edge; a `crates/data` home keeps it next to the loaders
that use it most. Lean: `crates/core` (it is a domain primitive, like `Bar` /
`Timestamp`), but the architect should weigh the dependency graph.

**The falsifier (R2)** is the existing `no_look_ahead_falsifier` shape, lifted to
the shared API: build a series, query causally, then forward-shift the series and
query again, assert the two results differ. It becomes the single canonical
falsifier the migrated call sites all point at (the per-loader copies can be
retired or kept as thin regression guards — architect's call).

---

## Acceptance criteria

- **AC1 — One guarded as-of API exists and is the single join path.** The shared
  helper/newtype is implemented in its home crate; `funding_as_of` and
  `basis_as_of` are migrated to it (or are thin wrappers over it). No backtest
  consumer reaches future data: every sidecar join routes through the guarded
  API. (R1)

- **AC2 — The self-proving look-ahead falsifier passes as a falsifier.** A test
  feeds deliberately-future data through the API and asserts the guard
  rejects/excludes it — the causal result MUST differ from the leaked result. The
  test is wired so that *removing the guard makes it fail* (it proves the
  guarantee, not just exercises the happy path). (R2)

- **AC3 — Zero anchor delta.** `scripts/verify_anchors.sh` reports the same
  119/119 (or current count) byte-identical anchors before and after the
  migration: the funding-carry and basis arms produce identical as-of values, so
  every anchored surface is byte-unchanged. (R3)

- **AC4 — Gates green.** `python3 scripts/spec_lint.py spec/point-in-time-data-discipline`
  passes (valid frontmatter, no dead links); `cargo clippy -- -D warnings` and
  the migrated crates' tests pass.

- **AC5 — (If R4 lint is built) the lint catches a planted look-ahead.** A
  deliberately-introduced bypass (a hand-rolled future-reading join in a test
  fixture) is flagged by the lint; a clean tree passes. If the architect drops
  the lint (typed-API-sufficient), AC5 is recorded N/A with the
  type-level-sufficiency reasoning. (R4)

---

## Verification floor — and why the day-1 e2e divergence gate does NOT apply here

CLAUDE.md mandates that **"every strategy overlay or sizing-modifier ships with a
baseline-equity-divergence end-to-end test from day 1"** (the
`v3-volatility-forecaster-noop-fix` precedent: a no-op overlay where `scale` was
computed but never applied). That gate is **scoped to overlays / sizing
modifiers** — code that changes the strategy decision variable and must be proven
to actually move equity.

**This feature is a data-discipline hardening, NOT a strategy overlay or sizing
modifier.** It introduces no new decision variable, no scaling, no signal — it
refactors an existing as-of join behind a guarantee. There is no "did the overlay
actually apply?" question, because there is no overlay. Applying the
equity-divergence gate here would be category error: the *correct* outcome is
that equity does **not** change (AC3 — zero anchor delta is the explicit success
condition).

The real verification floor for this feature is therefore:

1. **The self-proving look-ahead falsifier (AC2)** — the analogue of the
   day-1-falsifier convention and the on-chain `--leak-check`. A deliberate
   future-data feed is provably rejected. *This* is the day-1 gate for a
   discipline feature.
2. **Zero anchor delta (AC3)** — the behaviour-preservation proof that the
   refactor changed no reported number.

Together these are stricter, for this feature's risk profile, than an
equity-divergence test would be: AC2 proves the guard *works*, AC3 proves the
migration *changed nothing*. (If the architect's chosen design somehow does
touch a decision path — it should not — the CLAUDE.md gate re-applies and this
section must be revisited.)

---

## Open decisions (for the architect)

1. **Type-level vs runtime guard (R1).** `PitSeries<T>`/`AsOf<T>`
   (look-ahead unrepresentable, compile-time) vs a guarded free function +
   debug-assert (drop-in, runtime/debug-time). Analyst lean: type-level
   (durable, Recommended); runtime-guard is the if-budget-tightens fallback.
   Resolve and lock in M-T1.

2. **Lint vs typed-API-only (R4 / AC5).** Is a `scripts/`-level look-ahead lint
   worth building, or does the typed API make it redundant? If type-level (a) is
   chosen, the lint is likely unnecessary for the core join (but may still catch
   *new hand-rolled* joins that bypass the API). If runtime-guard (b) is chosen,
   the lint becomes the backstop that closes the bypass hole. Decide and record.

3. **Where the seam lives — core vs data vs backtest.** Analyst lean:
   `crates/core` (it is a domain primitive alongside `Bar` / `Timestamp`), so
   every crate can reach it without a new dependency edge. Architect to weigh the
   workspace dependency graph (does `core` want this, or does it belong next to
   the loaders in `data`?).

4. **Migrate the `f64` `basis_diag.rs` copy, or leave it as a research adapter?**
   The diag probe uses `f64` (research-grade, not the Decimal production path).
   Folding it into the shared (Decimal) API may be more friction than value for a
   read-only example. Option: keep a thin `f64` local adapter in the probe that
   documents it mirrors the shared API, rather than forcing the probe onto the
   production type. Architect's call.

5. **Falsifier home + retirement of the per-loader copies.** Does the shared
   falsifier (AC2) replace the two existing `no_look_ahead_falsifier` tests, or
   do those stay as thin per-loader regression guards? (Retiring them reduces
   duplication; keeping them adds belt-and-suspenders.)

---

## Changelog

- 2026-06-18 (analyst): authored the brief. Scoped the one structurally-worthwhile
  qlib gap (PIT / as-of discipline, qlib-note #1) as a **focused as-of-join helper
  + falsifier (+ optional lint), NOT a new database**. Grounded it in the real
  seam: `funding_as_of` (`crates/backtest/src/funding_data.rs:378`) and
  `basis_as_of` (`crates/backtest/src/basis_data.rs:397`) are the SAME hand-rolled
  `partition_point` as-of join, copied a third time (`f64`) in
  `crates/data/examples/basis_diag.rs:219`, each with its own
  `no_look_ahead_falsifier` — the seam the architect's own `OQ-CARRY-SEM` trace
  note calls "per-bar funding-ring + as-of join". Honest scoping: the price path
  is ALREADY PIT-clean by construction (`Strategy::on_bar` streaming +
  past-only `RingBuffer`), so the genuine look-ahead surface is just the sidecar
  as-of join (guarded by this API) + forward-return labels (out of scope —
  intentionally future). Analyst lean: type-level `PitSeries`/`AsOf` (durable,
  Recommended) over a runtime-guard fallback. Verification floor = the
  self-proving look-ahead falsifier (AC2) + zero anchor delta (AC3); the CLAUDE.md
  day-1 equity-divergence gate does NOT apply (not an overlay/sizing modifier —
  the correct outcome is equity UNCHANGED). NO code; NO new database; NO vendor
  feed; NO re-opening the concluded active-edge search; NO anchored-report edits.
  Created REQ-POINT-IN-TIME-DATA-001 (proposed). HANDOFF → architect.
