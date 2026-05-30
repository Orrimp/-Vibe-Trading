---
slug: monte-carlo-bootstrap-path-generator
version: 0.1.0
status: proposed
owner: analyst
priority: P2
updated: 2026-05-30
---

# Monte-Carlo stationary-block-bootstrap path generator — v0.1.0

> **Monte-Carlo robustness lane — C1 (first slice, wave 1 of 2).** Per the
> operator's 4 locked strategic decisions (2026-05-30) and the analyst
> direction note
> [`strategy-robustness-monte-carlo-direction-2026-05-29.md`](../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md)
> § 6 (C1) + the architect readiness audit
> [`monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
> § 2(a) / § 6.2 Phase MC-1. This is the **path-generator primitive**: given
> a real crypto return series + a master seed + N, produce N synthetic price
> paths via **stationary block bootstrap** (Politis–Romano 1994), deterministic
> via ChaCha20. It is a reusable `crates/data` building block. The consumer —
> the strategy robustness harness — is the sibling brief
> [`strategy-robustness-harness`](../strategy-robustness-harness/feature.md)
> (C2), which depends on this.

## Why

### The named limit (from the direction note § 1)

Every shipped strategy is judged by **one number per scenario computed on one
deterministic historical price path**. A strategy that scores Sharpe 1.4 on the
single 2023-FY path and a strategy that scores 1.4-only-on-that-exact-path are
indistinguishable today. Robustness needs the distribution of
`f(strategy, params, P_i)` over an ensemble `{P_i}` of plausible paths — and
today `N = 1`.

This brief builds the **`{P_i}` generator**. It does not run any strategy and
emits no report — that is C2's job. C1 alone produces paths nothing consumes;
that is acceptable because C1 is the minimum reusable primitive and C1+C2 ship
as the coherent first slice (see § Two-feature decomposition below).

### Why stationary block bootstrap (operator Q1, locked)

The operator locked **stationary block bootstrap first** (durable / Recommended
path). The method resamples blocks of consecutive **real** crypto returns with
random (geometric-distributed) block lengths, producing a new stationary series
that — because it splices real return blocks — inherits the real fat tails and
within-block volatility clustering **without any distributional assumption**
([Politis–Romano 1994](https://www.tandfonline.com/doi/abs/10.1080/01621459.1994.10476870)).

Why this is the durable first bet, not the cheap one:

- **Lowest calibration risk.** One tunable parameter (expected block length),
  and it is **auto-selectable** via the spectral-density / flat-top-lag-window
  method of [Politis–White (2004)](https://public.econ.duke.edu/~ap172/Politis_White_2004.pdf),
  with the [Patton–Politis–White (2009)](https://public.econ.duke.edu/~ap172/Patton_Politis_White_2009.pdf)
  correction (following Nordman 2008's correction of Lahiri's theoretical
  result). No generative model to mis-specify.
- **Categorically NOT the retired bet.** GARCH and regime-switching were retired
  as **alpha sources** (forecasting cousins;
  [`v3-vol-retirement-and-c5-promotion-2026-05-22.md`](../dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md),
  [`v25-dl-journey-retrospective-2026-05-22.md`](../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)).
  Bootstrap-of-real-returns **predicts nothing** — it resamples an existing
  series to measure the variance of an already-shipped strategy's outcome under
  input perturbation. One hunts signal; this quantifies uncertainty. **This
  distinction is not relitigated here** (operator constraint); it is stated so a
  reviewer does not reject "bootstrap robustness" on sight.

### GBM demoted to smoke-test (operator Q1, locked)

The existing GBM generator (`crates/backtest/src/main.rs` `synthetic_bars`, and
the duplicate `synthetic_bars_hourly` at `crates/backtest/src/scenarios/momentum.rs:98`)
is **demoted to a smoke-test baseline**. Gaussian returns produce no fat tails
and no volatility clustering, so GBM-derived drawdown/VaR systematically
**understate** crypto tail risk
([ScienceDirect overview](https://www.sciencedirect.com/topics/engineering/geometric-brownian-motion)).
GBM stays only to answer "does the N-path harness run at all?" — never as the
robustness verdict source. C1 wires GBM as a `MonteCarloPathGen` impl so the
harness can smoke-test, but the **headline generator is the block bootstrap**.

## Two-feature decomposition (analyst decision — justified)

The first slice is split into **two features**, not one multi-wave feature:

- **C1 = this brief** (`monte-carlo-bootstrap-path-generator`, `crates/data`):
  the pure path-generator primitive.
- **C2 = sibling brief**
  ([`strategy-robustness-harness`](../strategy-robustness-harness/feature.md),
  `crates/backtest`): the consumer that runs a strategy over the N paths and
  emits the anchored distribution-summary report.

**Why two, not one (durable choice):**

1. **C1 has standalone reuse value.** A seeded stationary-block-bootstrap path
   generator in `crates/data::synth` is consumed not only by C2 but by the Queue
   follow-ons: C3 (param-sweep over bootstrap paths), C5 (CPCV/Deflated-Sharpe
   over resampled real data), and any future generative work. Bundling it inside
   the harness feature would bury a cross-cutting primitive inside one consumer.
2. **Different crates, different test surfaces.** C1's contract is a *pure
   function* determinism + statistical-property test (`crates/data`); C2's
   contract is a *distribution-summary anchor* + baseline-divergence e2e
   (`crates/backtest`). Splitting keeps each test isolated.
3. **Different determinism risk profiles.** C1's determinism is the simple
   "same seed → same `Vec<Bar>`" (no f64 reduction). The hard f64-reduction-order
   / ADR-0051 anchor-determinism risk lives **entirely in C2**. The split keeps
   C1 shippable and anchor-free while C2 carries the ADR.
4. **Architect audit already scoped them apart.** § 2(a) (path injection,
   `crates/data`) vs § 2(c) (aggregator, `crates/backtest`) are separately sized;
   the GBM behaviour-preserving-lift risk is C1-local.

The cheap alternative (one feature, two waves) is the **fallback** if the
operator wants a single trace row — but it forfeits C1's clean reuse boundary and
spawns a v0.2.0 "extract the generator" refactor when C3/C5 need it. Rejected at
analyst level; named here for completeness.

## Requirements

### R1 — `MonteCarloPathGen` trait + stationary-block-bootstrap impl

- **R1.1** A new module `crates/data/src/synth/` exists, exposing a
  `MonteCarloPathGen` trait. Sketch (architect M-T1 ratifies the exact
  signature):
  ```rust
  pub trait MonteCarloPathGen {
      /// Pure: identical (universe, n_bars, path_seed) ⇒ identical Vec<Vec<Bar>>.
      /// Outer Vec is per-symbol; inner Vec is the bar series for that symbol.
      fn generate(&self, universe: &[(Symbol, Decimal)], n_bars: usize, path_seed: u64)
          -> Vec<Vec<Bar>>;
  }
  ```
- **R1.2** A `BlockBootstrapPathGen` impl produces a synthetic return series by
  the **stationary bootstrap** of Politis–Romano: starting indices drawn
  uniformly from the real series, block lengths drawn from a geometric
  distribution with mean `L`, wrapping at the series end (circular), concatenated
  until `n_bars` returns are produced. Returns are converted back to a price path
  from the real series' start price (or a caller-supplied start price).
- **R1.3** The bootstrap resamples a **real** crypto return series loaded from a
  parquet revision (Binance OHLCV), NOT a synthetic series. The generator takes
  the source series + its **revision SHA** as inputs (see R2.3). The block
  bootstrap is applied **per symbol independently at v0.1.0** (cross-sectional
  co-movement preservation is a v0.2.0 carve-out — see K3 / Q-MCB-2).
- **R1.4** A `GbmPathGen` impl (the demoted smoke-test) is the
  behaviour-preserving lift of the existing GBM generator into
  `data::synth::gbm` (see R4 for the lift discipline). It exists so C2 can
  smoke-test the N-path harness; it is **not** the robustness generator.

### R2 — Determinism contract (pure function of stated inputs)

- **R2.1** `generate` is a **pure function** of `(source series, revision SHA,
  path_seed, n_bars, block-length policy)`. Identical inputs ⇒ byte-identical
  `Vec<Vec<Bar>>`. No `thread_rng`, no `SmallRng`, no wall-clock, no env reads.
- **R2.2** All randomness flows from `ChaCha20Rng` (`seed_from_u64` /
  `from_seed`) — the only RNG the generators may use, matching the existing
  `synthetic_bars` idiom and the architect's § 5 mandate. The per-path seed
  derivation (`master_seed → path_seed_j`) is **owned by C2's master-seed →
  sub-seed rule** (ADR-0051 D1); C1 accepts a single `path_seed: u64` and is
  agnostic to how the caller derived it. C1 MUST be deterministic given that
  one `u64`.
- **R2.3** The source-series **revision SHA** is an explicit input the caller
  threads in (from `data::revision` / `REVISION.toml`). C1 does not read the SHA
  from disk itself; it accepts it so C2 can print it in the anchored report body
  (a different resampled-source ⇒ different distribution ⇒ different anchor SHA,
  which is correct).
- **R2.4** A two-run byte-identity test asserts: `generate(...)` called twice
  with the same args yields `Vec<Vec<Bar>>` that are element-wise equal
  (mirrors the existing `crates/backtest/tests/determinism.rs` pattern).

### R3 — Auto-tunable block length (Politis–White / Patton–Politis–White)

- **R3.1** The block-length policy is a typed enum:
  `BlockLengthPolicy::Fixed(usize)` | `BlockLengthPolicy::Auto`.
- **R3.2** `Auto` implements the [Politis–White (2004)](https://public.econ.duke.edu/~ap172/Politis_White_2004.pdf)
  spectral-density / flat-top-lag-window automatic block-length selection, with
  the [Patton–Politis–White (2009)](https://public.econ.duke.edu/~ap172/Patton_Politis_White_2009.pdf)
  correction. The chosen `L` is **computed from the source series** and is itself
  deterministic (a pure function of the series). The selected `L` MUST be
  exposed on the generator (a getter or a returned struct field) so C2 can print
  it in the anchored report body — the chosen block length is a distribution
  input and must change the anchor SHA if it changes.
- **R3.3** `Fixed(L)` is the escape hatch for tests and for the smoke-test (a
  fixed small `L` makes the math trivially checkable). The headline robustness
  run uses `Auto`.
- **R3.4** Block lengths within a single bootstrap draw are
  **geometric-distributed with mean `L`** (the stationary-bootstrap definition),
  not fixed-`L` blocks (that would be the moving-block bootstrap, which the
  direction note § 2.2 explicitly distinguishes — stationary is "less sensitive
  to block-size misspecification").

### R4 — Behaviour-preserving GBM lift (the anchor-safety clause)

- **R4.1** The existing GBM generator(s) feed the project's anchored synthetic
  backtests (the architect counts 84 anchors; some are synthetic-path-derived).
  Lifting GBM into `data::synth::gbm` MUST be a **behaviour-preserving
  extraction**: identical RNG draw order, identical arithmetic, identical
  clamping — so the byte output is unchanged and **no synthetic-path anchor
  re-emits**. Treat exactly like the ADR-0035 Phase-B scenario extraction
  (verbatim-copy discipline) per the architect's § 2 "where the MC path generator
  lives" note.
- **R4.2** There are currently **two** GBM copies (`main.rs` `synthetic_bars`
  + `scenarios/momentum.rs:98` `synthetic_bars_hourly` + a third test-local copy
  in `tests/determinism.rs:37`). The architect M-T1 decides whether v0.1.0 lifts
  ALL copies to the single `data::synth::gbm` source-of-truth (durable) or lifts
  only what C1 needs and defers the dedup (fallback). **Analyst recommendation:
  full dedup is durable** but it widens the anchor-re-emit blast radius — flagged
  as Q-MCB-3 for the operator/architect because it touches anchored code paths.
- **R4.3** A regression guard: after the lift,
  `bash scripts/verify_anchors.sh` is byte-identical PASS pre/post (any synthetic
  anchor that moves means the lift was NOT behaviour-preserving — that is a
  blocker, not an acceptable diff).

### R-NR — Non-regression contract

- **R-NR.1** `bash scripts/verify_anchors.sh` → all-PASS **byte-identical**
  pre/post. C1 adds NO new anchor (the anchor unit is C2's distribution-summary
  report). Any movement in an existing synthetic-path anchor is a R4.1 violation.
- **R-NR.2** Zero behaviour change to any existing scenario `run()`, to
  `PaperEngine`, to `MatchingEngine`, or to the `Strategy` trait. C1 is purely
  additive (new `synth/` module + a behaviour-preserving GBM lift).
- **R-NR.3** Money math stays `Decimal` (`rust_decimal`); the
  `#![deny(clippy::float_arithmetic)]` posture in the engine is unaffected. Bar
  prices in the generated paths are `Decimal` (matching `Bar`). The bootstrap's
  return-space arithmetic may use f64 internally but MUST round to `Decimal` at
  the `Bar` boundary deterministically.
- **R-NR.4** `cargo clippy -- -D warnings` clean; `cargo fmt` clean. No
  `.unwrap()` in library code; all external I/O (parquet load) behind the
  existing `crates/data` trait so tests can fake it.
- **R-NR.5** No new heavyweight deps without architect signoff. Block-bootstrap
  is ~50 LoC of resampling per the architect (§ 2(a)); the Politis–White
  auto-`L` is a spectral-density computation — architect M-T1 decides whether to
  hand-roll it (durable, no dep, ~80 LoC) or pull a crate (faster but adds a
  workspace edge). **Analyst lean: hand-roll** to keep the dep-free posture and
  full determinism control (Q-MCB-1).
- **R-NR.6** **Adapted CLAUDE.md overlay-e2e-style gate (C1 half).** C1 cannot
  diverge equity (it runs no strategy), so the day-1 gate adapted to a generator
  is a **statistical-property + determinism** gate: (a) the bootstrap output
  series' empirical distribution preserves the source series' first two moments
  within a tolerance (mean / variance of resampled returns ≈ source returns,
  asserting the resampler is not silently degenerate — e.g. not emitting a
  constant series), AND (b) two seeded runs are byte-identical (R2.4). The
  "diverges from a single-path baseline" half of the CLAUDE.md gate is C2's
  (it owns the strategy run); C1 owns the "is the generator actually producing a
  non-degenerate, deterministic ensemble" half. Together C1+C2 satisfy the full
  non-negotiable.

## Falsifiers (K)

- **K1 — The bootstrap silently collapses to a near-constant or
  near-deterministic series.** A resampler bug (e.g. block length effectively 1
  everywhere, or always drawing the same start index) would produce paths with
  no real variance — the no-op signature in generator clothing. **Mitigation**:
  R-NR.6(a) moment-preservation assertion + a test asserting the N paths are
  **not** mutually identical (pairwise divergence ≥ epsilon for distinct
  `path_seed`s). This is the C1 analogue of the v3-vol-overlay noop discovery.
- **K2 — Auto block-length selection is mis-implemented and silently returns a
  pathological `L`** (e.g. `L=1` → i.i.d. bootstrap losing all clustering, or
  `L=n` → one block = the original series, losing all resampling). **Mitigation**:
  a unit test on `Auto` against a series with known autocorrelation structure,
  asserting `1 < L < n` and that `L` grows with injected serial dependence. Cite
  the Patton–Politis–White corrected algorithm as the reference; pin a
  small-fixture expected `L` once computed.
- **K3 — Per-symbol independent bootstrap destroys cross-sectional co-movement,
  making the ensemble unrealistic for a cross-sectional momentum strategy.**
  v0.1.0 bootstraps each symbol independently (R1.3), which breaks the
  contemporaneous correlation structure that cross-sectional momentum trades on.
  This is a **known v0.1.0 limitation**, not a bug. **Mitigation**: documented in
  Q-MCB-2 with the durable path (block-bootstrap a **common index sequence**
  shared across symbols, preserving co-movement) named as the v0.2.0 carve-out.
  C2's report MUST print `bootstrap_mode: per-symbol-independent` in the anchored
  body so the limitation is legible and the anchor changes when v0.2.0 fixes it.
- **K4 — The GBM lift accidentally changes byte output and re-emits 84 anchors.**
  The single highest-blast-radius risk. **Mitigation**: R4.1 verbatim-copy
  discipline + R4.3 byte-identical anchor gate as a hard blocker. If the architect
  cannot guarantee a behaviour-preserving lift, the fallback is to NOT lift GBM in
  v0.1.0 (leave the duplicate, add a thin `GbmPathGen` wrapper that *calls* the
  existing function) — Q-MCB-3.

## Hypotheses (H)

- **H1 — Block-bootstrap resampler is ~50 LoC.** Per architect § 2(a). The
  geometric-block stationary bootstrap is a tight loop. Falsifier: > 150 LoC means
  the design is over-built; revisit.
- **H2 — Auto-`L` (Politis–White / PPW-2009) is ~80–120 LoC hand-rolled.** A
  spectral-density estimate via a flat-top lag window + the corrected block-size
  formula. Falsifier: > 200 LoC or a needed external crate → route to Q-MCB-1
  (pull a vetted crate instead).
- **H3 — The GBM lift is byte-preserving on the first attempt** if done as a
  pure cut-paste (no "while I'm here" cleanups). Falsifier: any synthetic anchor
  moves → the lift touched arithmetic/draw-order; revert and re-do verbatim.

## Operator decisions

> Per AGENT.md 2026-05-28 durable-over-quick: the `(Recommended)` tag is on the
> **most durable** option, with an explicit if-budget-tightens fallback named.

### Q-MCB-1 — Auto block-length: hand-roll or pull a crate?

**Q.** Does v0.1.0 hand-roll the Politis–White / PPW-2009 auto-`L` selection
(dep-free, full determinism control) or pull an existing crate?

**(Recommended — DURABLE) Option A — hand-roll in `data::synth`.** ~80–120 LoC
(H2). Keeps the established dep-free posture, gives full control over the f64
reduction order (which matters because the chosen `L` flows into C2's anchored
report — see ADR-0051), and avoids a workspace edge for a single function.
Durable because a third-party crate's internal float reductions are outside our
determinism contract and could silently break the anchor on a crate bump.

**Cost.** ~1 extra dev-day vs pulling a crate; zero follow-on (no dep to audit
on upgrade).

**Option B (cheap fallback).** Pull a vetted block-length crate (if a maintained
Rust one exists; the reference impls are R `blocklength::pwsd` and MATLAB).
~0.5 day. **Fallback only if** hand-rolling exceeds H2's ~200 LoC ceiling. Adds a
dep-audit + a determinism-scope question (does the crate use `f64` reductions we
don't control?) — that question would have to be answered before the `L` can feed
an anchored report, so the "cheap" path may not actually be cheaper once the
ADR-0051 determinism boundary is accounted for.

**Default**: A (Recommended DURABLE).

### Q-MCB-2 — Cross-sectional co-movement at v0.1.0?

**Q.** Does the v0.1.0 bootstrap preserve contemporaneous cross-symbol
co-movement (shared resampling index across the universe), or bootstrap each
symbol independently?

**(Recommended — DURABLE) Option A — shared-index block bootstrap.** Draw ONE
sequence of (start-index, block-length) blocks and apply it to **all symbols
simultaneously**, so symbol returns that co-moved on a given real timestamp stay
together in the resample. This preserves the cross-sectional correlation that
cross-sectional momentum (the load-bearing v1 strategy) actually trades on, so
the robustness distribution C2 produces is **decision-grade for the real
strategy** rather than for an unrealistic decorrelated market.

**Cost.** Marginally more than per-symbol (~0.5 day): the resampling index is
shared, the per-symbol price reconstruction is unchanged. Durable because
per-symbol-independent (Option B) understates tail co-movement and would force a
v0.2.0 "fix the correlation structure" re-emit of every MC anchor.

**Option B (cheap fallback — REJECTED at analyst level).** Bootstrap each symbol
independently (R1.3 as drafted). **Rejected** because for a *cross-sectional*
strategy a decorrelated ensemble is the wrong null — it would make the strategy
look more robust than it is (independent symbols → diversification that the real
market does not offer in a crash). If the operator wants the absolute-minimum
first slice, B ships with `bootstrap_mode: per-symbol-independent` printed in
C2's anchored body and a documented v0.2.0 carve-out (K3).

**Default**: A (Recommended DURABLE). *Note: R1.3 currently drafts the per-symbol
fallback as the literal default; if the operator picks A (recommended), R1.3
upgrades to shared-index. The architect M-T1 ratifies which lands in v0.1.0.*

### Q-MCB-3 — GBM dedup blast radius

**Q.** Does v0.1.0 lift ALL three GBM copies (`main.rs`, `momentum.rs`,
`determinism.rs`) to the single `data::synth::gbm` source-of-truth, or only wrap
what C1 needs and defer the dedup?

**(Recommended — DURABLE) Option A — full dedup to one source-of-truth.**
Eliminates the duplicate-divergence risk (three copies drifting) and gives
`data::synth::gbm` a single canonical GBM. Durable: no "which copy is real?"
ambiguity for future MC work.

**Cost.** Wider anchor-re-emit blast radius — touches the anchored synthetic
backtest code paths, so R4.1 verbatim-copy discipline + R4.3 byte-identical gate
must hold across ALL lifted call sites. ~1 extra dev-day of careful extraction.

**Option B (cheap fallback).** Add `GbmPathGen` as a thin wrapper that *calls*
the existing `synthetic_bars` in place; defer dedup to v0.2.0. **Fallback if**
the architect judges the all-copies lift too risky for the synthetic anchors.
Adds a v0.2.0 "dedup the GBM copies" cleanup commitment and leaves the
three-copy drift risk open. Per CLAUDE.md the anchored reports are byte-immutable,
so the *cheap* path is actually the *safer* path here — this is the one Q where
the fallback may be the right call. **Architect M-T1 decides on the lift-safety
evidence.**

**Default**: A (Recommended DURABLE) **IF** the architect can prove the lift is
byte-preserving; else B is the honest fallback. This is the deliberate exception
to "Recommended = durable" per the analyst-brief exception rule — flagged for the
architect to resolve with lift-safety evidence at M-T1.

## Verdict tree (pre-drawn)

The load-bearing axes are Q-MCB-2 (co-movement) and Q-MCB-3 (GBM dedup).
Q-MCB-1 (hand-roll vs crate) is orthogonal and folds in.

| Q-MCB-2 \ Q-MCB-3 | Q-MCB-3=(a) full dedup | Q-MCB-3=(b) wrap + defer |
|---|---|---|
| **Q-MCB-2=(a) shared-index** | **DURABLE — Recommended.** Decision-grade ensemble for the real cross-sectional strategy + one canonical GBM. Ships only if R4 lift proven byte-safe. | **DURABLE-CORE, anchor-safe fallback.** Best ensemble + lowest anchor blast radius. The pragmatic ship if lift-safety is uncertain. |
| **Q-MCB-2=(b) per-symbol** | MIXED — clean GBM but unrealistic decorrelated ensemble; spawns v0.2.0 co-movement re-emit. Operator-override only. | MINIMUM-VIABLE — both cheap paths; ships fastest but carries TWO v0.2.0 carve-outs (co-movement + GBM dedup). Fallback only under hard budget pressure. |

## Design

_architect fills this at M-T1. Inputs flagged: ADR-0051 (owned by C2; C1's
per-path determinism contract R2 must be consistent with ADR-0051 D1's
master-seed → sub-seed rule), the R4 GBM-lift safety evidence (resolves Q-MCB-3),
and the Q-MCB-1/Q-MCB-2 ratifications._

## Backtest Scenarios

_C1 emits no backtest report (it runs no strategy). The scenarios live in the
sibling C2 brief. C1's verification is unit/property tests on the generator
(R2.4 determinism, R-NR.6 moment-preservation, K1 non-collapse, K2 auto-`L`)._

## Implementation

_developer fills this._

## Verification

_tester links to reports here._

## Changelog

- 2026-05-30 (analyst): Feature brief authored as **C1** of the Monte-Carlo
  robustness lane (M0 pass), under the operator's 4 locked strategic decisions
  (2026-05-30): Q1 = stationary block bootstrap first / GBM demoted to
  smoke-test; Q3 = robustness harness first / learning loop last. Grounded in
  [`strategy-robustness-monte-carlo-direction-2026-05-29.md`](../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md)
  § 6 (C1) + the architect readiness audit
  [`monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
  § 2(a) / § 6.2 Phase MC-1. R1-R4 + R-NR (6 clauses, incl. the adapted
  CLAUDE.md generator-half gate R-NR.6) + K1-K4 + H1-H3 + Q-MCB-1/2/3 +
  pre-drawn 4-cell verdict tree. All Qs bias DURABLE per AGENT.md 2026-05-28;
  Q-MCB-3 carries the explicit Recommended=durable EXCEPTION (anchor-safety may
  make the cheap wrap the honest ship — architect resolves at M-T1).
  Decomposition decision recorded: TWO features (C1 reusable `crates/data`
  primitive + C2 `crates/backtest` consumer) over one multi-wave feature, with
  justification. Politis–Romano (1994) + Politis–White (2004) + PPW (2009)
  auto-block-length citation chain pinned. Trace row
  `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001` opened `proposed`. HANDOFF → architect
  (M-T1; C1+C2 bundle; ADR-0051 owned by C2).
