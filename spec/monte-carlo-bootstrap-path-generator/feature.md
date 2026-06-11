---
slug: monte-carlo-bootstrap-path-generator
version: 0.1.0
status: tester-done
owner: tester
priority: P2
updated: 2026-05-30
---

# Monte-Carlo stationary-block-bootstrap path generator — v0.1.0

> **Monte-Carlo robustness lane — C1 (first slice, wave 1 of 2).** Per the
> operator's 4 locked strategic decisions (2026-05-30) and the analyst
> direction note
> [`strategy-robustness-monte-carlo-direction-2026-05-29.md`](../dev-notes/archive/2026-Q2/strategy-robustness-monte-carlo-direction-2026-05-29.md)
> § 6 (C1) + the architect readiness audit
> [`monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../dev-notes/archive/2026-Q2/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
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
  [`v3-vol-retirement-and-c5-promotion-2026-05-22.md`](../dev-notes/archive/2026-Q2/v3-vol-retirement-and-c5-promotion-2026-05-22.md),
  [`v25-dl-journey-retrospective-2026-05-22.md`](../dev-notes/archive/2026-Q2/v25-dl-journey-retrospective-2026-05-22.md)).
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
  bootstrap draws **ONE shared index sequence and applies it to all symbols**
  (the shared-index design — **Q-MCB-2 = Option A, RATIFIED**; see the Decision
  Record and FP-C1.5), so cross-sectional co-movement **is preserved in v0.1.0**.
  *(Supersedes the pre-ratification draft that deferred co-movement to a v0.2.0
  carve-out; the tester confirmed FP-C1.5 is a genuine guard — per-symbol-
  independent resampling collapses cross-symbol correlation to −0.079.)*
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

> **Architect M-T1 (2026-05-30).** Trace `arch`: ADR-0051 (D1 sub-seed
> consistency, D2/D3 anchor determinism owned by C2), ADR-0002 (ChaCha20 RNG),
> ADR-0003 (Decimal money math). All three Qs resolved below; the load-bearing
> calls are **Q-MCB-2 = shared-index (RATIFIED)** and **Q-MCB-3 = thin-wrap +
> defer (the documented exception — full dedup is NOT byte-safe; evidence below)**.

### D-C1.1 — Module layout: `crates/data/src/synth/`

New module tree under `crates/data` (sited alongside `fake_feed` / `mock_feed` /
`replay_feed` per the readiness audit § 2 "where the MC path generator lives"):

```text
crates/data/src/synth/
├── mod.rs          # MonteCarloPathGen trait + BlockLengthPolicy enum + re-exports
├── bootstrap.rs    # BlockBootstrapPathGen (the headline generator; R1.2, R1.3)
├── block_length.rs # Politis–White / PPW-2009 auto-L selection (R3; Q-MCB-1 hand-roll)
└── gbm.rs          # GbmPathGen (the demoted smoke-test; R1.4, Q-MCB-3 thin-wrap)
```

`crates/data/src/lib.rs` gains `pub mod synth;` and re-exports
`pub use synth::{MonteCarloPathGen, BlockBootstrapPathGen, GbmPathGen,
BlockLengthPolicy};`. No new crate (audit § 2: a new crate is overkill for ~3
generators and adds a workspace edge for no isolation benefit). **No new
dependency** — block bootstrap + auto-`L` are hand-rolled (Q-MCB-1 = A); the only
crates touched are already-present `rand` / `rand_chacha` / `rust_decimal` /
`trading_core` (for `Bar`/`Symbol`/`Price`).

### D-C1.2 — `MonteCarloPathGen` trait (ratified signature)

R1.1's sketch is ratified with two refinements: (a) the return type carries the
selected block length so C2 can print it in the anchored body (R3.2), and (b) the
universe element is `(Symbol, Decimal)` — `(symbol, start_price)` — matching
`top10_symbols_with_prices()` (`momentum.rs:63`) so the harness threads the exact
universe shape the scenarios already use.

```rust
// crates/data/src/synth/mod.rs
use rust_decimal::Decimal;
use trading_core::{Bar, Symbol};

/// One synthetic ensemble member: the per-symbol bar series for a single path.
pub struct GeneratedPath {
    /// Outer Vec is per-symbol (universe order preserved); inner is the bar series.
    pub bars_by_symbol: Vec<Vec<Bar>>,
    /// The block length actually used (Auto-selected or Fixed). A distribution
    /// input — C2 prints it in the hashed report body (ADR-0051 D3). `None` for
    /// generators that have no block-length concept (GbmPathGen).
    pub selected_block_length: Option<usize>,
}

pub trait MonteCarloPathGen {
    /// Pure: identical (universe, n_bars, path_seed) ⇒ identical `GeneratedPath`.
    /// MUST NOT read wall-clock, env, thread_rng, or any global mutable state.
    /// All randomness flows from `ChaCha20Rng::seed_from_u64(path_seed)`.
    fn generate(&self, universe: &[(Symbol, Decimal)], n_bars: usize, path_seed: u64)
        -> GeneratedPath;
}
```

Rationale for `GeneratedPath` over the bare `Vec<Vec<Bar>>` in R1.1: the
auto-selected `L` (R3.2) must surface to the caller and it would be awkward to
bolt a getter onto a trait that returns a plain `Vec`. A struct return keeps the
generator pure (the `L` is a function of the source series, so it is part of the
deterministic output) and gives C2 exactly the field it needs for the body.
**This is the minimal upgrade to R1.1; the developer MAY keep the bare-Vec
signature and add a separate `selected_block_length()` getter if they prefer —
either satisfies R3.2 as long as the chosen `L` reaches C2's body.**

### D-C1.3 — `BlockBootstrapPathGen` + Q-MCB-2 = SHARED-INDEX (RATIFIED, methodologically load-bearing)

**RATIFICATION: shared-index block bootstrap (Q-MCB-2 = Option A). This upgrades
R1.3's literal per-symbol-independent default.** The single most important
methodological call in the slice.

The construction (one path):

1. Build the source **return series per symbol** from the real bar series:
   `r_sym[t] = (close[t] / close[t-1]).ln()` (log returns; `T-1` returns for `T`
   bars). All symbols share the **same length `T`** (the scenarios load a fixed
   `bar_count` per symbol — 8760/8784 — so the real ensemble is rectangular;
   assert equal lengths and `bail`/`Err` on ragged input).
2. Seed **one** `ChaCha20Rng::seed_from_u64(path_seed)` for the whole path (D1).
3. Draw the **stationary-bootstrap index sequence ONCE** (Politis–Romano): emit
   indices `i_0, i_1, …, i_{n_bars-2}` (`n_bars-1` return indices for `n_bars`
   output bars) by: pick a uniform start `i_0 ∈ [0, T-1)`; with probability
   `p = 1/L` start a **new** block (fresh uniform start index), else continue the
   current block by advancing `i_{k} = (i_{k-1} + 1) mod (T-1)` (circular wrap).
   `L` is the expected block length (geometric block lengths with mean `L` —
   R3.4; the `p = 1/L` Bernoulli-restart is the stationary-bootstrap definition,
   NOT fixed-`L` moving blocks).
4. **Apply the SAME index sequence to ALL symbols** (the shared-index step):
   `r'_sym[k] = r_sym[i_k]` for every symbol. Because index `i_k` selects the
   **same real timestamp** across all symbols, the contemporaneous cross-symbol
   co-movement on that timestamp is preserved in the resample.
5. Reconstruct each symbol's price path from its real start price (the universe
   `Decimal`), compounding the resampled returns: `p_sym[0] = start_price`,
   `p_sym[k+1] = p_sym[k] * exp(r'_sym[k])`, rounded to `Decimal` at the `Bar`
   boundary (R-NR.3 — f64 return-space arithmetic, deterministic round to
   `Decimal` for `Bar.close`). OHLC/volume/timestamps follow the existing
   `synthetic_bars_hourly` Bar-construction conventions (epoch-based `open_ts`/
   `close_ts` from a fixed start year; high/low bracket the open/close; volume a
   resampled real value or a fixed proxy — developer matches `Bar` field
   semantics; only `close` is load-bearing for the strategy's returns).

**Why shared-index is the right null (the analyst's flag, ratified).** For a
**cross-sectional** momentum strategy the edge is *relative* ranking across
symbols at each timestamp. A per-symbol-independent bootstrap (R1.3 literal)
draws a different index sequence per symbol, which **destroys the contemporaneous
correlation** — it manufactures diversification the real market does not offer in
a crash, so the robustness distribution would make the strategy look **more
robust than it is** (understated tail co-movement, optimistic p95 MaxDD). That is
precisely the wrong-null failure mode. Shared-index resamples *time blocks*
jointly across the universe, preserving the cross-sectional structure the
strategy actually trades on → the distribution C2 produces is **decision-grade
for the real strategy**. Cost over per-symbol is marginal (~0.5 day per the
analyst): the index draw is shared; only the per-symbol price reconstruction
loops. C2's body prints `bootstrap_mode: shared-index` (ADR-0051 D3 / C1 K3) so
the anchor reflects the ratified mode and would move if a future v0.2.0 ever
changed it.

> **Determinism note (ADR-0051 D1 composition).** There is **exactly one**
> `ChaCha20Rng` per path, seeded by `path_seed_j`. The shared index sequence is
> the only RNG-consuming step; price reconstruction is deterministic arithmetic.
> So "same `path_seed` ⇒ same index sequence ⇒ same `Vec<Vec<Bar>>`" (R2.1/R2.4)
> holds by construction, and it composes cleanly with C2's D1 sub-seed rule (C2
> derives `path_seed_j`; C1 is agnostic to how).

### D-C1.4 — Auto block length: Q-MCB-1 = HAND-ROLL (RATIFIED), Politis–White / PPW-2009

**RATIFICATION: hand-roll in `synth::block_length` (Q-MCB-1 = Option A, durable).**
No crate. The library-compat checklist was run against the candidate Rust crates:
none is a clear win and all forfeit determinism control —

- `blocklength` (R-port idea) / `arch` (Python) / `np::b.star` (R) are the
  *reference* impls but are **not Rust crates we can depend on**; there is no
  maintained, single-binary-friendly Rust crate implementing PWSD auto-`L` that
  passed the checklist (maintained ≤ 18 mo + no system C deps + edition-2024
  clean + determinism-controllable). A crate's internal f64 reductions would sit
  **outside our ADR-0051 D2 determinism boundary** and could silently break the
  anchor on a crate bump (the chosen `L` flows into C2's hashed body — D3). The
  "cheap" path is not cheaper once the determinism boundary is accounted for.

The hand-rolled algorithm (Politis–White 2004 PWSD, with the Patton–Politis–White
2009 / Nordman 2008 correction; confirmed against the `np::b.star` /
`arch.bootstrap.optimal_block_length` / `blocklength::pwsd` reference docs):

1. Compute the sample autocorrelations `ρ̂(k)` of the (single representative)
   return series for `k = 1 … M`.
2. **`m̂` selection**: `m̂` is the smallest lag such that `K_N` *consecutive*
   autocorrelations `ρ̂(m̂), …, ρ̂(m̂+K_N-1)` all fall inside the band
   `±2·sqrt(log10(N)/N)`, where `K_N = max(5, ⌈log10(N)⌉)` and `N = T-1` (the
   return count). Cap the search at `M = ⌈sqrt(N)⌉ + K_N`.
3. **Flat-top lag window** (Politis–Romano 1995): `λ(s) = 1` for `|s| ≤ 1/2`,
   `λ(s) = 2(1−|s|)` for `1/2 < |s| ≤ 1`, `0` otherwise. Estimate
   `ĝ = Σ_{k=-2m̂}^{2m̂} λ(k/m̂)·|k|·γ̂(k)` and
   `Ĝ_hat = Σ_{k=-2m̂}^{2m̂} λ(k/m̂)·γ̂(k)` from the autocovariances `γ̂(k)`.
4. **`b̂` (stationary bootstrap, PPW-2009 corrected constant)**:
   `b̂ = ( 2·Ĝ_hat² / D_SB )^{1/3} · N^{1/3}`, where for the **stationary**
   bootstrap `D_SB = 2·Ĝ_hat²` per the PPW-2009 correction (the 2004 paper's
   constant was corrected following Nordman 2008). Clamp
   `b̂ ∈ [1, ⌈min(3·sqrt(N), N/3)⌉]` (the reference-impl upper guard) and round to
   the nearest integer ≥ 1. `L := b̂`.

**Determinism**: every step is a pure function of the source series (no RNG); the
chosen `L` is therefore part of C1's deterministic output (surfaced via
`GeneratedPath.selected_block_length` — D-C1.2). The f64 reductions here are
C1-internal and do **not** enter a hashed report directly — only the resulting
**integer `L`** does (C2 prints `selected_block_length_L: <usize>`), so the
auto-`L` f64 math is one integer-quantization away from the anchor and is robust
to last-bit noise by construction (the integer rounding absorbs it).

**`BlockLengthPolicy`** (R3.1): `enum BlockLengthPolicy { Fixed(usize), Auto }`.
`Auto` runs the above; `Fixed(L)` is the test/smoke escape hatch (R3.3) — a small
fixed `L` makes the resampling trivially checkable and `L=1` degenerates to iid
resampling (falsifier probe FP-C1.3).

> **Which series feeds auto-`L` under shared-index?** Auto-`L` needs ONE `L` for
> the shared index sequence (the whole universe resamples on one `L`). Ratified
> rule: compute `ρ̂(k)` on the **universe-average absolute log-return series**
> `r̄[t] = mean_sym |r_sym[t]|` (a single representative series capturing the
> common volatility-clustering timescale), then run PWSD on `r̄`. This is
> deterministic, gives one integer `L`, and ties the block length to the
> *common* serial-dependence structure the shared-index bootstrap preserves.
> (Alternative considered: per-symbol `L` then take the median — rejected, it
> reintroduces a per-symbol step the shared-index design eliminated and the
> median of integers is a weaker estimator. The universe-average-|return| series
> is the cleaner single input.)

### D-C1.5 — Q-MCB-3 = THIN-WRAP + DEFER (the documented Recommended=durable EXCEPTION)

**RATIFICATION: Option B (thin-wrap + defer the 3-copy dedup to v0.2.0). Full
dedup is NOT behaviour-preserving and would re-lock anchors — evidence below.
This is the deliberate exception per the analyst brief: the cheap option is the
honest, anchor-safe ship per CLAUDE.md byte-immutability.**

**Lift-safety evidence (the decisive finding — read of the three call sites).**
The brief's premise that "THREE GBM copies exist" is true at the *grep* level but
**they are NOT three copies of one function** — they are three *distinct*
generators with different parameters, draw structure, and output shape:

| Site | File:line | Bar TF | per-Δ vol / drift | intrabar scale | volume draw | trade_count draw | clamp | Anchor-load-bearing? |
|---|---|---|---|---|---|---|---|---|
| `synthetic_bars_hourly` | `scenarios/momentum.rs:98` | **hour** | `0.012` / `0.000_03` | `close*0.002` | `rng*500+10` | `random_range(100..5000)` | `[0.01, 10_000_000]` | **YES — the 84 anchors** |
| `synthetic_bars` | `main.rs:951` | **minute** | `0.001_10` / `0.000_001_9` | `close*0.000_5` | `rng*50+1` | (separate path) | `[1_000, 500_000]` | indirectly (main CLI) |
| `synthetic_bars_det` | `tests/determinism.rs:37` | **minute** | `0.001_10` / `0.000_001_9` (inline) | none | `rng*50+1` | `random_range(10..500)` | `[1_000, 500_000]` | test-local only |

A "single source-of-truth GBM" would have to be a **parameterized** function
taking `(timeframe, vol, drift, intrabar_scale, volume_lo, volume_hi,
trade_count_range, clamp_lo, clamp_hi)` and then prove that **each of the three
call sites reproduces its exact current `ChaCha20Rng` draw sequence and arithmetic
byte-for-byte**. That is exactly the ADR-0035 Phase-B verbatim-extraction risk,
amplified ×3 across sites with *different parameter values* and even **different
RNG draw counts per bar** (the hourly site draws `trade_count` via
`random_range`; the minute sites differ). The probability of an off-by-one draw
or a clamp-order difference re-emitting one of the 84 synthetic anchors (K4) is
**high**, and per CLAUDE.md anchored reports are byte-immutable. **A behaviour-
preserving full dedup cannot be proven byte-safe here**, so per the analyst-brief
exception rule (Recommended=durable yields to anchor-safety) the honest ship is:

- **`GbmPathGen` is a NEW, independent impl in `synth::gbm`** that produces a GBM
  ensemble for the smoke-test (R1.4). It is parameterized for the harness's needs
  (universe + n_bars + path_seed via the trait) and is **anchor-free** — it does
  NOT feed any of the 84 anchors and is never the robustness verdict source
  (headline = `BlockBootstrapPathGen`).
- **The three existing GBM functions are NOT touched, NOT moved, NOT re-routed**
  at v0.1.0. `synthetic_bars_hourly` stays byte-identical in `momentum.rs` (the
  84 anchors are untouched by construction — R-NR.1/R-NR.2), `synthetic_bars`
  stays in `main.rs`, `synthetic_bars_det` stays in the test. Zero behaviour
  change → `verify_anchors.sh` is byte-identical PASS pre/post **trivially**
  (R4.3 is satisfied because no anchored code path is edited at all).
- **GbmPathGen's draw order** SHOULD mirror the hourly site's Box-Muller +
  intrabar + volume + trade_count sequence (so it is a faithful GBM smoke-test),
  but it does NOT need byte-parity with any anchor (it produces its own,
  un-anchored paths). The developer copies the *shape* of `synthetic_bars_hourly`
  into `synth::gbm` as a starting point and adapts it to the trait — a fresh impl
  informed by the existing one, not a behaviour-preserving lift.

**The v0.2.0 carve-out (named, not built).** Dedup the three GBM functions into
one canonical `synth::gbm` source-of-truth IS still the durable end-state, but it
requires the full ADR-0035 § Phase-B verbatim-extraction protocol (or a
deliberate, operator-approved anchor re-emission). Tracked as a v0.2.0 follow-on;
NOT in C1's blast radius. **This keeps C1 strictly additive and anchor-safe** —
the brief's R4.1 "behaviour-preserving lift" is satisfied vacuously because C1
introduces an *independent* generator rather than lifting the anchored one.

> **Net blast-radius for the developer**: C1 adds a new `synth/` module and
> touches NO anchored Rust. `verify_anchors.sh` is byte-identical PASS because no
> anchored code is edited. The Q-MCB-3 decision **shrinks** the dev's risk to the
> new module only.

### D-C1.6 — Determinism + money-math conformance (ADR-0002 / ADR-0003)

- **RNG**: `ChaCha20Rng::seed_from_u64(path_seed)` only (R2.2; ADR-0002). No
  `thread_rng`, no `SmallRng`, no wall-clock, no env. One RNG per path.
- **Money math**: `Bar` prices are `Decimal` (R-NR.3; ADR-0003). The bootstrap's
  log-return / compounding arithmetic is f64 (returns are dimensionless), rounded
  to `Decimal` at the `Bar` boundary deterministically (`Decimal::try_from(f64)`
  with the existing `synthetic_bars_hourly` `to_dec` clamp pattern). The engine's
  `#![deny(clippy::float_arithmetic)]` posture is unaffected — `synth/` is in
  `crates/data`, and the f64 use is annotated `#[allow(clippy::float_arithmetic)]`
  exactly as `synthetic_bars_hourly` already is (`momentum.rs:94`).
- **No anchor**: C1 adds none (R-NR.1). The anchor unit is C2's summary report
  (ADR-0051 D4).

### Falsification probes (C1 — for the developer M-DEV dry-run)

- **FP-C1.1 — same-seed determinism (R2.4).** `generate(univ, n, S)` twice ⇒
  element-wise-equal `Vec<Vec<Bar>>`. Mirrors `tests/determinism.rs` pattern.
- **FP-C1.2 — different-seed divergence (K1, the noop-in-generator-clothing
  guard).** `generate(univ, n, S1)` vs `generate(univ, n, S2)` for `S1 ≠ S2` ⇒
  the two ensembles differ (assert the BTC close-series are not element-wise
  equal). Catches a resampler that ignores the seed.
- **FP-C1.3 — `L=1` degenerates to iid (K2).** With `BlockLengthPolicy::Fixed(1)`
  the index sequence is a fresh uniform draw every step (every step restarts a
  block, `p=1/1=1`) ⇒ the resample is iid bootstrap. Assert the empirical lag-1
  autocorrelation of the resampled returns ≈ 0 (within tolerance), confirming
  block structure collapses at `L=1`.
- **FP-C1.4 — moment preservation (R-NR.6a).** The resampled return series' mean
  and variance ≈ the source series' mean and variance (within tolerance, large
  `n_bars`). Asserts the resampler is not silently degenerate (not emitting a
  constant or zero-variance series — the K1 collapse signature).
- **FP-C1.5 — shared-index co-movement (Q-MCB-2 ratification guard).** Generate a
  2-symbol ensemble where the two real series are positively correlated; assert
  the resampled series retain a positive contemporaneous correlation ≥ a fraction
  of the source correlation (proves the shared index actually co-moves the
  symbols — a per-symbol-independent bug would drive it toward 0). **This is the
  test that proves the methodologically-load-bearing decision is wired.**
- **FP-C1.6 — auto-`L` sanity (K2).** On a series with injected serial dependence
  (e.g. an AR(1) with `φ=0.6`), assert `1 < L < n` AND `L` grows vs an iid series
  of the same length. Pin the small-fixture expected `L` once computed.

## Backtest Scenarios

_C1 emits no backtest report (it runs no strategy). The scenarios live in the
sibling C2 brief. C1's verification is unit/property tests on the generator
(R2.4 determinism, R-NR.6 moment-preservation, K1 non-collapse, K2 auto-`L`)._

## Implementation

Developer: completed 2026-05-30.

### As-built notes

**Module layout** (D-C1.1): `crates/data/src/synth/` with 4 files:
- `mod.rs` — `MonteCarloPathGen` trait + `GeneratedPath` struct + `BlockLengthPolicy` enum + `SynthError` (thiserror).
- `block_length.rs` — `politis_white_block_length(returns: &[f64]) -> usize` (pure, no RNG, hand-rolled PWSD + PPW-2009 corrected SB constant).
- `bootstrap.rs` — `BlockBootstrapPathGen` (the headline stationary-block-bootstrap generator; shared-index Q-MCB-2).
- `gbm.rs` — `GbmPathGen` (new independent GBM smoke-test; NOT a lift of the 3 existing generators — Q-MCB-3 thin-wrap).

**Deviations from spec (none material)**:
- `SynthError` added as a proper `thiserror`-derived enum (the spec implied `bail`/`Err`; same result, cleaner boundary).
- `BlockBootstrapPathGen::new()` validates inputs at construction time in addition to `generate()` to fail early.
- GBM per-symbol seeds derived by `path_seed.wrapping_add((i as u64).wrapping_mul(0x9E37_79B9))` — same idiom as momentum.rs:245, matching ADR-0051 D1 spirit even though the spec left GBM's sub-seeding unspecified.

**Pinned FP-C1.6 auto-L fixture** (canonical Apple-Silicon box, 2026-05-30):
- AR(1) φ=0.6, N=500 samples: auto-L = **7**
- iid uniform N=500 samples: auto-L = **1**
- Confirms: serial dependence increases block length; white noise gets L=1 (i.i.d. floor).

**Test coverage (23 tests, all PASS)**:
- `synth::block_length::tests` — 4 tests: degenerate/short series, zero-variance, valid-range, FP-C1.6.
- `synth::bootstrap::tests` — 13 tests: FP-C1.1..5 + error cases + output shape + selected-block-length.
- `synth::gbm::tests` — 6 tests: same-seed determinism, different-seed divergence, block-length None, bar count, error cases.

**Gates verified** (all green, 2026-05-30):
- `cargo test -p data --lib -- synth` → 23/23 PASS
- `cargo build -p data` → clean
- `cargo fmt -p data --check` → zero diff
- `cargo clippy -p data --tests -- -D warnings` → zero errors
- `bash scripts/verify_anchors.sh` → 84/84 PASS (C1 touches no anchored Rust — trivially satisfied)

**C2 consumption interface**:
```rust
use data::{BlockBootstrapPathGen, BlockLengthPolicy, GbmPathGen, MonteCarloPathGen, GeneratedPath};
// headline:
let bgen = BlockBootstrapPathGen::new(source_bars, BlockLengthPolicy::Auto)?;
let path: GeneratedPath = bgen.generate(&universe, n_bars, path_seed_j)?;
// smoke-test:
let ggen = GbmPathGen::new();
let path: GeneratedPath = ggen.generate(&universe, n_bars, path_seed_j)?;
```
C2 derives `path_seed_j = master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))` (ADR-0051 D1).

## Verification

_tester links to reports here._

## Changelog

- 2026-05-30 (analyst): Feature brief authored as **C1** of the Monte-Carlo
  robustness lane (M0 pass), under the operator's 4 locked strategic decisions
  (2026-05-30): Q1 = stationary block bootstrap first / GBM demoted to
  smoke-test; Q3 = robustness harness first / learning loop last. Grounded in
  [`strategy-robustness-monte-carlo-direction-2026-05-29.md`](../dev-notes/archive/2026-Q2/strategy-robustness-monte-carlo-direction-2026-05-29.md)
  § 6 (C1) + the architect readiness audit
  [`monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../dev-notes/archive/2026-Q2/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
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
- 2026-05-30 (developer): M-DEV-1..7 complete. New `crates/data/src/synth/` module
  (mod.rs + block_length.rs + bootstrap.rs + gbm.rs). 23 unit tests — all PASS.
  FP-C1.1..6 all verified. `verify_anchors.sh` 84/84. Clippy + fmt clean. No
  anchored Rust touched. Status: arch-done → dev-done. HANDOFF → tester.
- 2026-05-30 (architect, M-T1): `## Design` authored (D-C1.1..D-C1.6) + 6
  falsification probes (FP-C1.1..FP-C1.6). All three Qs resolved:
  **Q-MCB-1 = hand-roll** the Politis–White/PPW-2009 auto-`L` (no maintained
  single-binary-friendly Rust crate passed the compat checklist; a crate's f64
  reductions sit outside the ADR-0051 D2 determinism boundary). **Q-MCB-2 =
  shared-index RATIFIED** (the methodologically load-bearing call — upgrades
  R1.3's literal per-symbol default; one shared resampling-index sequence applied
  across all symbols preserves the cross-sectional co-movement the v1
  cross-sectional-momentum strategy trades on; per-symbol-independent is the
  wrong null — it manufactures crash-time diversification and understates p95
  MaxDD). **Q-MCB-3 = thin-wrap + defer (Option B, the documented Recommended=
  durable EXCEPTION)**: code-level read of the three "GBM copies" proves they are
  three DISTINCT generators (hourly vs minute TF; different vol/drift/intrabar/
  volume/trade_count/clamp; different per-bar RNG draw counts), NOT one
  copy-pasted function — a "full dedup to one source-of-truth" cannot be a
  behaviour-preserving verbatim lift and would re-emit synthetic anchors (K4).
  Per CLAUDE.md byte-immutability, `GbmPathGen` lands as a NEW independent
  anchor-free smoke-test impl in `synth::gbm`; the three existing functions are
  NOT touched → `verify_anchors.sh` byte-identical PASS trivially (no anchored
  code edited). 3-copy dedup deferred to a v0.2.0 carve-out (ADR-0035 § Phase-B
  protocol). ADR-0051 authored (owned via C2; C1's R2 per-path determinism
  composes with D1's `master + j*0x9E37_79B9` sub-seed rule — one ChaCha20 per
  path, one shared index sequence). `MonteCarloPathGen` signature ratified with a
  `GeneratedPath` return carrying `selected_block_length` for C2's hashed body
  (R3.2). Trace `arch` filled; status proposed → arch-done; owner → developer.
  `tasks.md` created with M-DEV rows. HANDOFF → developer (build C1 first).
