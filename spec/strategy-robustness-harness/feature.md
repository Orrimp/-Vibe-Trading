---
slug: strategy-robustness-harness
version: 0.1.1
status: dev-done
owner: tester
priority: P2
updated: 2026-05-30
---

# Strategy robustness harness — distribution-summary backtest mode — v0.1.1

> **Monte-Carlo robustness lane — C2 (first slice, wave 2 of 2).** Per the
> operator's 4 locked strategic decisions (2026-05-30). Depends on **C1**
> ([`monte-carlo-bootstrap-path-generator`](../monte-carlo-bootstrap-path-generator/feature.md)).
> This is the **consumer + verdict surface**: run a strategy over the N
> bootstrap paths C1 produces, reduce the N per-path outcomes into a
> **distribution summary** (Sharpe p5/p50/p95, max-drawdown tail, prob-of-loss),
> and emit **ONE anchored summary report** per the operator's Q2 (seed the
> ensemble → anchor the distribution summary, NOT N per-path anchors).
> Grounded in the direction note
> [`strategy-robustness-monte-carlo-direction-2026-05-29.md`](../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md)
> § 3 / § 6 (C2) + the architect readiness audit
> [`monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
> § 2(c) / § 3 / § 6.2 Phase MC-1.

## Why

### From point-estimate to distribution (the direction note § 3)

Today a backtest produces `backtest(strategy, θ*, path) → {sharpe, sortino,
max_dd, equity}` — scalars on **one** path. The load-bearing example: v1
momentum has **73% max-drawdown on 2023-FY real Binance data**
([`strategic-reset-2026-05-23.md`](../dev-notes/strategic-reset-2026-05-23.md)
§ 4.2). That is one number on one path. We do not know its p5/p50/p95 across
plausible alternative 2023s.

This harness produces `robustness(strategy, θ*, {P_1..P_N}) → distribution`:

| Output | Point-estimate today | Distribution under N paths |
|---|---|---|
| Sharpe | `~0.00` (harness real-path: 0.003; the fabricated 1.40 from product.md:78 is retracted per adversarial review 2026-05-30) | p5 / p25 / **p50** / p75 / p95 |
| Max drawdown | `73%` (one path) | drawdown **tail**: p50 MaxDD + p95 MaxDD (the number that should gate `paper→live`) |
| Probability of loss | undefined | `P(final_equity < initial)` across the ensemble |
| P(Sharpe > 0) / P(Sharpe > 1) | undefined | fraction of paths clearing each bar |

Two things land that the project has never produced: a **drawdown tail** (the
number the `paper→live` gate should actually use) and the foundation for a
**plateau-vs-peak** parameter verdict (the param-sweep axis is C3, a Queue
follow-on; v0.1.0 is single-`θ*` over N paths).

### The harness shape already exists (architect's single most important finding)

`crates/backtest/src/bin/threshold_sweep.rs` is a **working sweep+aggregate
prototype**: it enumerates a 45-cell parameter grid, runs each cell in parallel
via a rayon pool, **sorts cells lexicographically before rendering** (→
order-invariant body → byte-identical across runs), splits run-varying fields
into YAML front-matter and the deterministic body into the hashed region, and
emits **one summary report = one anchor**. Monte-Carlo is the **dual**: N paths
over a fixed parameter set instead of N parameter cells over a fixed path. The
harness shape is identical; only the inner loop's varying axis changes
(path-seed instead of (τ,ε)). **C2 reuses this seam** (the architect's
`threshold_sweep::run_cell` "load-once-use-N-times" pattern), it does not
reinvent it.

The per-metric scalar calculators C2 needs **already exist** as free functions
in `threshold_sweep.rs` (`compute_sharpe_hourly:234`, `compute_sortino_hourly:260`,
`compute_calmar:286`, `compute_max_drawdown_f64:311`, `compute_total_return:334`).
The architect's § 2(c) recommends lifting them to a shared module. **The new
code is the percentile/moment reducer over N samples** (the rest is wiring).

### Operator Q2 (locked): seed the ensemble → anchor the distribution summary

The operator locked: **ChaCha20 from one master seed → byte-identical N-path SET
→ the distribution summary is itself deterministic → anchor ONE summary report**
(not N per-path anchors). This resolves the hard stochastic-vs-anchor tension the
direction note § 8 flagged: same master seed → same N sub-seeds → same N paths →
same N per-path metrics → same percentiles → same body-SHA. The architect's
§ 3.3 readiness audit independently recommends the same anchor unit (Option A:
one summary report = one anchor; reject N per-path anchors).

## Dependency on C1

C2 **cannot run without C1**. C1 produces the `Vec<Vec<Bar>>` ensemble; C2
consumes it. C1 has standalone reuse value (C3/C5 also consume it), which is why
they are two features (see C1 § Two-feature decomposition). Sequencing: C1 lands
first (anchor-free, pure-function determinism), then C2 layers the harness +
the one anchored report on top. **C2 carries the ADR-0051 obligation** (§ ADR
flag below); C1 does not.

## Requirements

### R1 — Robustness backtest mode (the N-path runner)

- **R1.1** A new backtest entry point — a `--robustness --paths N` CLI flag (or
  a dedicated `bin/monte_carlo.rs` driver; architect M-T1 picks, mirroring the
  `threshold_sweep` bin precedent) — runs a strategy over the N bootstrap paths
  produced by C1's `BlockBootstrapPathGen` (the headline generator; `GbmPathGen`
  is the smoke-test fallback).
- **R1.2** Each of the N paths runs **exactly today's deterministic single-path
  backtest** with `path_seed_j` substituted for the global seed — no change to
  `PaperEngine`, `MatchingEngine`, or any scenario `run()`. The cell wrapper
  mirrors `scenarios::threshold_sweep::run_cell` (a thin
  `scenarios::montecarlo::run_path(input, path_seed_j, strategy)` per architect
  § 2(c)).
- **R1.3** The N cells run **in parallel** via a rayon pool (reuse the
  `threshold_sweep` pool pattern), but `path_seed_j` is bound to the **index
  `j`**, never to completion order (the determinism caveat — ADR-0051 D1). The
  master seed is `0xC0FFEE` canonical (or an explicit `--ensemble-seed`; architect
  M-T1 decides whether the ensemble seed is orthogonal to the existing
  fill-tie-break `--seed` — the direction note § 8 open question).
- **R1.4** v0.1.0 runs **one strategy family** (cross-sectional momentum, the
  load-bearing v1 baseline) at a **single fixed `θ*`** over N paths. Parameter
  sweep (the plateau-vs-peak heatmap) is **C3, a Queue follow-on** — NOT in this
  brief. The report's parameter-stability section is a **stub/N-A at v0.1.0**
  and lights up when C3 lands.

### R2 — Distribution-summary reducer + report (the verdict surface)

- **R2.1** The aggregator consumes the N `RunReport`s (or N equity curves) and
  computes, per metric (Sharpe / Sortino / Calmar / max-drawdown / total-return),
  the moment+percentile summary `{mean, std, p5, p25, p50, p75, p95, min, max}`.
- **R2.2** Plus the ensemble-level robustness numbers the direction note § 3.1
  names: `P(final_equity < initial)` (prob-of-loss), `P(Sharpe > 0)`, and
  `P(Sharpe > 1.0)` (the `paper→live` gate fraction). The **max-drawdown tail**
  (p50 MaxDD + p95 MaxDD) is surfaced as the headline gate number.
- **R2.3** The reducer uses a **fixed reduction order** (collect N per-path
  metrics into a `Vec` indexed by `j`, reduce sequentially in index order) and a
  **frozen percentile-selection rule** (sort with a total order, NaN asserted
  absent, index by the nearest-rank-or-linear-interpolation rule frozen in
  ADR-0051 D2). f64 mean/std are not associative; an unordered parallel fold is
  forbidden in the reducer. This is the f64-determinism boundary the architect's
  § 3.4 flags — see § ADR flag.
- **R2.4** A new `robustness-*.md` report type (operator Q-MC-2 durable path from
  the direction note § 7: a distribution report is structurally different from a
  single-path report and forcing it into the single-path template spawns a
  refactor). Front-matter / body split per the architect § 5 mandate:
  - **Front-matter (run-varying, NOT hashed):** `generated`, `wall_clock_s`,
    `host`, `pid`, `git_commit`, `data_revision_sha`.
  - **Body (deterministic, hashed by the anchor) — every distribution input
    printed so changing any of them changes the SHA:** `master_seed`, `n_paths`,
    `sub_seed_rule` (frozen string), `generator` (`block-bootstrap-real` |
    `gbm-smoke`), `generator_params` (block-length policy + the **selected
    auto-`L`** + source `revision_sha`), `bootstrap_mode` (per C1 K3:
    `per-symbol-independent` | `shared-index`), `param_set` (the fixed `θ*`),
    then the per-metric distribution table + the prob-of-loss / PPSR / DD-tail
    block, all at **fixed decimal precision** (`{:.6}` etc. per
    `threshold_sweep`).
- **R2.5** Rows/sections sorted deterministically before render (metric order
  fixed) so the body is byte-stable. **One report → one anchor** under a new
  namespace (e.g. `mc-robustness-2026-06`).

### R3 — Anchor + determinism contract (the ADR-0051 obligation)

- **R3.1** Two-run byte-identity: running the whole ensemble twice at the same
  master seed yields an **identical summary body-SHA** (extends the
  `crates/backtest/tests/determinism.rs` pattern to the ensemble — run twice,
  assert identical body hash).
- **R3.2** The anchor is the **body-SHA of the single distribution-summary
  report** (architect § 3.3 Option A). v0.1.0 adds exactly **+1 anchor** under a
  new namespace. No per-path anchors (Q2 locked; architect § 3.3 rejects Option
  B).
- **R3.3** **Determinism scope is declared explicitly**: byte-identical on the
  **Apple-Silicon canonical box**; cross-platform parity is **NOT contracted** —
  inheriting the ADR-0043 §"f64 conversion boundary" precedent verbatim
  (architect § 3.4 mandate #1). `verify_anchors.sh` runs on the canonical box.
- **R3.4** Fixed-precision formatting of every hashed float (architect § 3.4
  mitigation #2 — the single most effective lever: a 6-dp formatted Sharpe is
  identical even if the underlying f64 differs in bit 52). Architect M-T1 decides
  whether the *aggregation layer* additionally uses `Decimal`-quantized
  percentile selection (architect § 3.4 mitigation #3) — a durable upgrade that
  makes selection platform-independent, gated on whether cross-platform parity is
  ever wanted (it is not today).

### R-NR — Non-regression contract

- **R-NR.1** `bash scripts/verify_anchors.sh` → all-PASS byte-identical for the
  existing 84 anchors pre/post (the MC code touches none of their code paths,
  given C1's R4 behaviour-preserving GBM lift). C2 adds exactly **+1** new anchor
  (the distribution-summary report) under a new namespace — the routine additive
  extension, but it **requires ADR-0051** to lock the determinism contract (NOT
  for the additive row itself, but for the sub-seed rule + reduction order +
  report shape + scope + anchor unit; architect § 3.5).
- **R-NR.2** Zero behaviour change to `PaperEngine`, `MatchingEngine`, the
  `Strategy` trait, or any existing scenario `run()`. C2 is additive: a new
  `scenarios::montecarlo` cell wrapper + a new driver + the lifted shared metric
  calculators (the lift itself is behaviour-preserving — same arithmetic, just
  relocated from the bin to a shared module; assert via R-NR.5).
- **R-NR.3** Money math stays `Decimal`; only the statistical metric layer
  (Sharpe/Sortino/Calmar) uses f64 — unchanged from today's single-path backtest.
  The reducer's f64 use is bounded and order-fixed (R2.3).
- **R-NR.4** `cargo clippy -- -D warnings` + `cargo fmt` clean; no `.unwrap()` in
  library code; the path-generator I/O is behind C1's trait so the harness is
  testable with a fake generator.
- **R-NR.5** Lifting the metric calculators (`compute_sharpe_hourly` et al.) from
  the `threshold_sweep` bin to a shared module MUST be behaviour-preserving —
  the existing `threshold_sweep` report's anchor (if any) stays byte-identical.
  Verbatim relocation, no arithmetic change.
- **R-NR.6 — CLAUDE.md overlay-e2e-style gate (the C2 half, adapted, MANDATORY
  from day 1).** Per the CLAUDE.md non-negotiable (every strategy
  overlay/sizing-modifier ships with a baseline-equity-divergence e2e test from
  day 1; the v3-vol-overlay noop precedent). Adapted to a distribution harness,
  the **two-part gate** is:
  - **(a) Divergence**: the distribution summary **diverges from a single-path
    baseline** by a testable epsilon — i.e. the p50 (or any percentile) of the
    ensemble's equity/Sharpe is NOT byte-equal to the single deterministic
    baseline-path backtest when the N paths are non-degenerate. This catches the
    failure mode where the harness secretly collapses to running one path N times
    (the no-op signature in harness clothing — the direction note § 8 "subtle
    risk"). Assert `|p50_ensemble_metric − single_baseline_metric| ≥ epsilon` OR
    `spread(p95 − p5) ≥ epsilon`.
  - **(b) Determinism**: the summary is **byte-identical across two seeded runs**
    (R3.1). A harness that diverges from baseline but is non-reproducible is
    equally unfit.
  Together (a)+(b) are the day-1 gate: the distribution is *real* (diverges from
  the degenerate single-path) AND *reproducible* (anchorable). This is the
  literal adaptation the operator asked for. Pattern reference (single-path
  analogue): `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.

## ADR flag — ADR-0051 (architect deliverable at M-T1; do NOT write here)

> **This brief FLAGS ADR-0051; it does not author it.** Per the operator's Q2 and
> the architect's § 5, a new ADR is **required** at C2's M-T1 to lock the
> Monte-Carlo determinism + distribution-report anchoring contract. The next free
> number is **0051** (confirmed: `spec/architecture/adr/` max is `0050`).
>
> **Proposed title:** `ADR-0051 — Monte-Carlo robustness: synthetic-path
> ensembles, distribution-report shape, and anchor determinism`.
>
> **The ADR MUST lock (the architect's § 5 D1–D5, restated as the contract this
> brief depends on):**
>
> - **D1 — master-seed → sub-seed derivation rule.** The frozen rule mapping one
>   master seed to the N per-path `path_seed_j` (e.g. `master.wrapping_add(j *
>   0x9E37_79B9)`, the project's existing idiom at `momentum.rs:245` /
>   `threshold_sweep.rs:120`, OR the j-th draw of a `ChaCha20Rng` seeded by the
>   master). Bound to index `j`, never completion order. **C1's R2 determinism
>   contract must be consistent with whatever D1 picks** — C1 accepts a single
>   `path_seed: u64`; D1 owns how the harness derives it.
> - **D2 — aggregation reduction order + percentile-selection rule.** The fixed
>   sequential reduction order for mean/std (f64 is non-associative) and the
>   frozen percentile rule (sort + total order + nearest-rank-or-interpolation).
>   This is the f64 cross-platform-determinism boundary.
> - **D3 — distribution-report front-matter-vs-body split + fixed precision.**
>   Which fields are run-varying (front-matter, not hashed) vs distribution-input
>   (body, hashed), and the fixed decimal precision of every hashed float
>   (R2.4 / R3.4).
> - **D4 — anchor unit = ONE summary report** under a new namespace (architect
>   § 3.3 Option A; Q2 locked). Reject per-path anchors.
> - **D5 — determinism scope = Apple-Silicon canonical box** (inherit the
>   ADR-0043 §"f64 conversion boundary" precedent **verbatim**; cross-platform
>   byte-identity explicitly NOT contracted; that would be a separate larger ADR,
>   out of scope).
>
> The ADR SHOULD also state the **GARCH/regime-as-generator-not-alpha**
> distinction explicitly (architect § 2 retired-line guard) so a future reviewer
> does not reject the lane on sight — though v0.1.0 ships **only** the block
> bootstrap, not GARCH (that is C3+/Queue).

## Falsifiers (K)

- **K1 — The harness silently collapses to one path run N times (the no-op
  signature).** The direction note § 8 "subtle risk": byte-identity across
  structurally-different runs is the noop signature. If the per-path seed wiring
  is broken so all N paths are identical, the "distribution" is a spike and the
  anchor is invariant to the strategy — looks reproducible, IS a no-op.
  **Mitigation**: R-NR.6(a) divergence gate — the ensemble spread must exceed an
  epsilon AND the p50 must differ from the single-path baseline. This is the
  mandatory day-1 e2e per CLAUDE.md.
- **K2 — f64 non-determinism breaks the anchor on the canonical box.** Even
  same-machine, an unordered parallel reduction or unformatted float could make
  two runs differ in the last bit → anchor flaps. **Mitigation**: R2.3 fixed
  reduction order + R3.4 fixed-precision formatted body (the architect's § 3.4
  mitigation #2, "the single most effective lever"). The two-run byte-identity
  test (R3.1) is the gate.
- **K3 — The anchor is insensitive to the strategy/params** (changing `θ*` or the
  strategy does NOT change the summary SHA). The inverse of K1: an anchor that
  doesn't move when inputs move is not gating anything. **Mitigation**: every
  distribution input (`param_set`, `generator`, `master_seed`, `n_paths`,
  `bootstrap_mode`, selected `L`, source `revision_sha`) is printed in the hashed
  body (R2.4); a test asserts two different `θ*` produce two different body-SHAs.
- **K4 — The drawdown-tail number is GBM-optimistic because the run used the
  smoke-test generator by mistake.** If the headline robustness report is
  accidentally generated with `GbmPathGen` (no fat tails) instead of
  `BlockBootstrapPathGen`, the p95 MaxDD understates real risk and the
  `paper→live` gate trusts a wrong number. **Mitigation**: the `generator` field
  is in the hashed body (R2.4) and the report header states it prominently; a
  smoke-test run is visibly labelled `generator: gbm-smoke` and the operator
  success-report gate documentation (future) must require `block-bootstrap-real`
  before trusting the DD tail.

## Hypotheses (H)

- **H1 — The percentile/moment reducer is ~SMALL** (architect § 2(c)): the math
  is a sort + index + sequential mean/std. The MEDIUM risk is *determinism care*,
  not volume. Falsifier: > ~150 LoC for the reducer → over-built.
- **H2 — Two-run byte-identity holds on the canonical box** with fixed reduction
  order + fixed-precision formatting (architect § 3.1/§ 3.4). Falsifier: two runs
  differ → a reduction is unordered or a float is unformatted; fix before
  anchoring.
- **H3 — The harness reuses ≥ 80% of the `threshold_sweep` seam** (parallel
  cells, sort-before-render, FM/body split, lifted calculators). Falsifier: if
  the harness needs substantial new infra beyond the reducer + the path-injection
  edge, the architect's "seam is ~80% built" finding was wrong — re-scope.

## Operator decisions

> Per AGENT.md 2026-05-28 durable-over-quick: `(Recommended)` on the most durable
> option. Q-MC-2 (new report type) and Q-MC-3 (sequencing) from the direction
> note § 7 are **already locked by the operator** (Q2 = new report type / anchor
> the summary; Q3 = harness first, learning loop last) and are NOT re-asked here.
> The open C2-local decisions:

### Q-RH-1 — Ensemble size N at v0.1.0

**Q.** What is the default N (path count) for the v0.1.0 anchored robustness
report?

**(Recommended — DURABLE) Option A — N = 500.** The direction note § 3.3 sketch
and the standard robustness-testing literature use a few hundred to ~1000 paths
for stable tail percentiles (p95 MaxDD needs enough mass in the tail to be
meaningful — at N=100 the p95 is the 5th-worst path, noisy). 500 is a durable
default: stable p5/p95, runs in parallel (rayon) in a tractable wall-clock for a
single momentum scenario, and the anchored report's percentiles are
seed-deterministic regardless of N. Durable because re-anchoring at a different N
later changes the body-SHA (N is a hashed input) — picking a defensible N now
avoids a v0.2.0 re-anchor "just to get more paths."

**Cost.** Wall-clock ~N × single-path-backtest / rayon-parallelism. For a single
momentum scenario this is minutes, not hours. The architect M-T1 sizes the
wall-clock budget and confirms N=500 is tractable; if not, N=200 is the fallback
(still tail-meaningful, faster). **Emit a `watch -n` probe block** per the
long-running-task contract since an N-path run is > 2 min.

**Option B (cheap fallback).** N = 100. Faster first look but the p95 tail is the
5th-worst path — noisy and not yet decision-grade for the `paper→live` gate.
Fallback only if the architect finds N=500 wall-clock intractable on the canonical
box. Spawns a v0.2.0 re-anchor at higher N once the gate trusts the tail.

**Default**: A (Recommended DURABLE), N=200 as the if-budget-tightens fallback.

### Q-RH-2 — Driver shape: CLI flag on the existing backtest bin, or a dedicated `bin/monte_carlo.rs`?

**Q.** Does the robustness mode live as `--robustness --paths N` on the existing
backtest binary, or as a dedicated `bin/monte_carlo.rs` (mirroring
`bin/threshold_sweep.rs`)?

**(Recommended — DURABLE) Option A — dedicated `bin/monte_carlo.rs`.** Mirrors
the established `bin/threshold_sweep.rs` precedent exactly (the seam C2 reuses).
Keeps the single-path backtest binary's CLI surface uncluttered and its anchor
contract (84 single-path anchors) cleanly separated from the new
distribution-report anchor. Durable: the MC driver evolves independently (C3
param-sweep, C5 CPCV layer onto the MC bin family, not the single-path bin).

**Cost.** ~0 extra (a new bin is a Cargo target; the cell wrapper + reducer are
shared lib code either way). Same as `threshold_sweep`.

**Option B (cheap fallback).** Add `--robustness --paths N` to the existing
backtest binary. **Rejected at analyst level** — it entangles the
distribution-report path with the single-path backtest CLI and risks an operator
accidentally running the single-path anchor flow with `--robustness` set, or vice
versa. Fallback only if the architect prefers one binary for operational
simplicity.

**Default**: A (Recommended DURABLE; matches the `threshold_sweep` precedent).

## Verdict tree (pre-drawn)

Load-bearing axes: Q-RH-1 (N) and Q-RH-2 (driver shape). The operator's already-
locked Q2 (anchor the summary) and Q3 (harness first) are the frame, not axes.

| Q-RH-1 \ Q-RH-2 | Q-RH-2=(a) dedicated bin | Q-RH-2=(b) CLI flag on backtest bin |
|---|---|---|
| **Q-RH-1=(a) N=500** | **DURABLE — Recommended.** Decision-grade tail + clean bin separation; matches `threshold_sweep`. Ships if wall-clock tractable. | MIXED — decision-grade tail but entangled CLI / anchor surfaces. Operator-override only. |
| **Q-RH-1=(b) N=100/200** | DURABLE-CORE fallback — clean bin, faster, noisier tail; v0.2.0 re-anchor at higher N. The budget-tightened ship. | MINIMUM-VIABLE — both cheap paths; fastest first look, noisiest tail + entangled CLI; carries two v0.2.0 carve-outs. Hard-budget fallback only. |

## Design

> **Architect M-T1 (2026-05-30).** Trace `arch`: **ADR-0051** (authored this pass
> — D1 sub-seed rule, D2 reduction order, D3 report shape, D4 anchor unit, D5
> scope), ADR-0043 (f64-conversion-boundary scope inherited by D5), ADR-0032
> (realdata path + revision pin reused for the source series). Depends on **C1**
> ([`monte-carlo-bootstrap-path-generator`](../monte-carlo-bootstrap-path-generator/feature.md)) —
> build C1 first. Q-RH-1 and Q-RH-2 resolved below.

### D-C2.1 — ADR-0051 authored (the determinism + anchoring contract)

The brief's § ADR flag is discharged:
[`ADR-0051 — Monte-Carlo robustness: sub-seed derivation, distribution-report
shape, and anchor determinism`](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md),
status `accepted`, registered atomically in the ADR README. D1-D5 are exactly the
contract this brief depends on; the design below cites them. The reducer + report
implement D2/D3 verbatim; the seed wiring implements D1; the anchor unit is D4;
the scope declaration is D5.

### D-C2.2 — Module layout + Q-RH-2 = dedicated `bin/monte_carlo.rs` (RATIFIED)

**RATIFICATION: dedicated `crates/backtest/src/bin/monte_carlo.rs` (Q-RH-2 =
Option A, durable).** Mirrors the `bin/threshold_sweep.rs` precedent exactly (the
seam C2 reuses). Keeps the single-path `backtest` bin's CLI + its 84 single-path
anchors cleanly separated from the new distribution-report anchor; the MC bin
family evolves independently (C3 param-sweep, C5 CPCV layer onto it, not onto the
single-path bin). Cost ~0 (a new Cargo target; the cell wrapper + reducer are
shared lib code). Option B (`--robustness` flag on the `backtest` bin) is rejected
at analyst+architect level — it entangles the distribution-report path with the
single-path CLI and risks an operator running the single-path anchor flow with
`--robustness` set.

```text
crates/backtest/src/
├── bin/monte_carlo.rs       # NEW driver — fan out over 0..N (rayon), reduce, render
├── scenarios/montecarlo.rs  # NEW — run_path(input, path_seed_j, strategy) cell wrapper
└── stats/                   # NEW shared module — lifted compute_* + the reducer
    └── mod.rs               # compute_sharpe_hourly/sortino/calmar/max_dd/total_return
                             #   (lifted from bin/threshold_sweep.rs, behaviour-preserving)
                             #   + DistributionSummary reducer (the only genuinely new math)
```

The metric calculators (`compute_sharpe_hourly:234`, `compute_sortino_hourly:260`,
`compute_calmar:286`, `compute_max_drawdown_f64:311`, `compute_total_return:334`)
are currently free functions **inside the `threshold_sweep` bin**. Lift them
**verbatim** (R-NR.5 — same arithmetic, just relocated) into `backtest::stats` and
have `bin/threshold_sweep.rs` import them from there. This is behaviour-preserving:
the `threshold_sweep` report bytes are unchanged (the functions are identical, only
their path changes). The genuinely new code is `stats::DistributionSummary` — the
percentile/moment reducer (audit § 2(c): "the new code is the reducer; the rest is
wiring").

### D-C2.3 — `run_cell` generalizes to the path ensemble (CONFIRMED at code level)

The audit § 1.4 claim is confirmed by reading
`crates/backtest/src/scenarios/threshold_sweep.rs::run_cell`: it takes
`(TcnScenarioInput, seed: u64, overlay_strategy)`, accepts `input.bars_override:
Option<Vec<Bar>>` (pre-loaded bars), runs the standard bar-loop on a fresh
`PaperEngine::new(match_config, seed)`, and returns a `TcnOverlayRunResult` whose
`equity_curve: Vec<Decimal>` is exactly what the `compute_*` calculators consume.
**Monte-Carlo is the dual**: instead of N parameter cells over a fixed
`bars_override`, C2 runs N path-ensembles (each path's `bars_by_symbol` merged via
`data::ReplayFeed::merge_synthetic`) over a **fixed** strategy + θ*.

The new `scenarios::montecarlo::run_path` cell wrapper is a thin sibling of
`run_cell`:

```rust
// crates/backtest/src/scenarios/montecarlo.rs
pub async fn run_path(
    input: TcnScenarioInput,        // bars_override = this path's merged Vec<Bar>
    fill_seed: u64,                 // the FIXED fill-tie-break seed (ADR-0051 D1)
    strategy: <the fixed θ* momentum strategy>,
) -> Result<PathRunResult>          // carries equity_curve: Vec<Decimal>
```

`run_path` MUST be a behaviour-preserving sibling of `run_cell` (same bar-loop,
same `PaperEngine`, same risk limits) — it does NOT change `PaperEngine`,
`MatchingEngine`, or any scenario `run()` (R1.2 / R-NR.2). The cleanest
implementation: generalize the **existing `run_cell` body** into a
strategy-and-bars-parameterized helper and have BOTH `run_cell` and `run_path`
call it; OR copy the `run_cell` body into `run_path` (verbatim, then swap the
strategy type). Developer picks; both keep the anchored `run_cell` path intact.

> **Strategy type note.** v0.1.0 runs the v1 cross-sectional momentum baseline at
> a fixed θ* (R1.4). `run_cell` is currently typed to
> `TcnOverlayMomentumStrategy`; `run_path` should be typed to (or generic over) the
> momentum `Strategy` so the harness runs plain momentum, not the TCN overlay. If a
> shared generic helper is used, parameterize it `<S: Strategy>`; the momentum
> strategy is constructed once from `config/strategies/top10_momentum_h1.toml` and
> **cloned per path** (the strategy is re-instantiated per path so each path's run
> is independent — mirror how `threshold_sweep` builds a fresh strategy per cell).

### D-C2.4 — Seed wiring (ADR-0051 D1) + ensemble/fill-seed orthogonality (resolves direction § 8 open-Q2)

- **Master ensemble seed** `--ensemble-seed` (default `0xC0FFEE`). Per path index
  `j ∈ 0..N`: `path_seed_j = ensemble_seed.wrapping_add((j as u64).
  wrapping_mul(0x9E37_79B9))` (ADR-0051 D1; the project's existing idiom on the
  path axis). Bound to `j`, **never** to rayon completion order.
- **Fill-tie-break seed** is HELD CONSTANT across all paths at `0xC0FFEE`
  (ADR-0051 D1). The path is supplied by C1's ensemble, so the engine seed no
  longer generates the path; holding the fill seed constant ensures the only
  varying input across paths is **the path itself**, not the fill tie-break
  (which would be a confounding second noise source). **This is the resolution to
  direction-note § 8 open-Q2** (does the ensemble seed need to be orthogonal to
  the fill-tie-break seed): yes — they are separate knobs; the ensemble seed
  varies the paths, the fill seed is pinned. Both are printed in the hashed body
  (D3) so the anchor is sensitive to either.
- C2 calls `C1::BlockBootstrapPathGen::generate(universe, n_bars, path_seed_j)` per
  path; the returned `bars_by_symbol` is merged via
  `data::ReplayFeed::merge_synthetic` into the `Vec<Bar>` passed as
  `input.bars_override` to `run_path`.

### D-C2.5 — Q-RH-1 = N=500 (RATIFIED) + wall-clock budget + the `watch` probe

**RATIFICATION: N = 500 (Q-RH-1 = Option A, durable); N = 200 the
if-budget-tightens fallback.** 500 gives stable p5/p95 tail percentiles (at N=100
the p95 is the 5th-worst path — too noisy for the `paper→live` gate). N is a
hashed body field (D3), so re-anchoring at a different N later changes the SHA —
picking a defensible N now avoids a v0.2.0 re-anchor.

**Wall-clock budget (architect sizing).** One single-path momentum backtest over a
full year is 8760 hourly bars × 10 symbols ≈ the `top10-*-momentum` scenario cost
(seconds on the canonical box). N=500 paths over a rayon pool (the
`threshold_sweep` pattern ran 45 cells comfortably) is **minutes, not hours** —
each path is independent and embarrassingly parallel. Tractable on the canonical
box. If the dry-run measures > ~10 min wall-clock, fall back to N=200 (still
tail-meaningful). **N=500 ratified pending the M-DEV dry-run confirming
wall-clock < ~10 min**; the developer reports the measured wall-clock in
`§ Implementation` and the operator confirms N=500 vs the N=200 fallback before
the anchor is locked.

> **`watch` probe (long-running-task contract — MANDATORY since an N=500 run is
> > 2 min).** While the MC run executes, monitor with:
> ```bash
> watch -n 10 'ls -t spec/strategy-robustness-harness/reports/robustness-*.md 2>/dev/null | head -1 | xargs -I{} sh -c "echo {}; tail -20 {}"'
> ```
> (Shows the newest robustness report as it lands. Before the report exists the
> command prints nothing — that is expected during the fan-out.) Expected result:
> one `robustness-*.md` appears after the fan-out + reduce completes. Failure
> diagnosis: if no file after ~15 min at N=500, the fan-out stalled — check the
> driver's rayon pool (mirror `threshold_sweep`'s dedicated `sweep_pool` to avoid
> executor-context issues) or fall back to N=200.

### D-C2.6 — `DistributionSummary` reducer (ADR-0051 D2 — the f64 boundary)

The only genuinely new math. `backtest::stats::DistributionSummary`:

```rust
pub struct MetricDistribution {     // per metric
    pub mean: f64, pub std: f64,
    pub p5: f64, pub p25: f64, pub p50: f64, pub p75: f64, pub p95: f64,
    pub min: f64, pub max: f64,
}
pub struct DistributionSummary {
    pub sharpe: MetricDistribution,
    pub sortino: MetricDistribution,
    pub calmar: MetricDistribution,
    pub max_drawdown: MetricDistribution,
    pub total_return: MetricDistribution,
    pub prob_loss: f64,             // P(final_equity < initial)
    pub prob_sharpe_gt_0: f64,
    pub prob_sharpe_gt_1: f64,
    pub max_dd_tail_p50: f64,       // headline paper→live gate number
    pub max_dd_tail_p95: f64,
}
```

Reduction is **frozen per ADR-0051 D2** (carry the load-bearing comment in code):

- Collect the N per-path metrics into a `Vec<f64>` **indexed by path index `j`**
  (the parallel map returns `(j, PathRunResult)` or writes into a pre-sized vec at
  index `j`); reduce **sequentially in ascending-`j` order**. An unordered
  parallel fold is FORBIDDEN (`// ADR-0051 D2: index-order reduction is
  load-bearing — do NOT parallelize`).
- `mean` = sequential left-fold sum / N. `std` = **two-pass** population std
  (`var = Σ(x_j − mean)²/N`, `std = var.sqrt()`).
- Percentiles = **sort with `f64::total_cmp`** (NEVER `partial_cmp().unwrap()`),
  **NaN asserted absent first**, then **type-7 linear** interpolation:
  `h = (N−1)·p/100`, value `= sorted[⌊h⌋] + (h−⌊h⌋)·(sorted[⌈h⌉] − sorted[⌊h⌋])`.
- Probabilities = integer count / N (platform-independent count; only the final
  division is f64).

### D-C2.7 — `robustness-*.md` report (ADR-0051 D3) + anchor (D4)

New report type per ADR-0051 D3 (front-matter/body split + fixed-precision
formatting + fixed metric order). The body MUST print every distribution input so
the anchor is sensitive to it (K3): `master_seed`, `fill_seed`, `n_paths`,
`sub_seed_rule` (frozen string `"master + j*0x9E3779B9"`), `reduction_rule` (frozen
string), `generator` (`block-bootstrap-real` | `gbm-smoke`), `bootstrap_mode`
(`shared-index` from C1), `block_length_policy`, `selected_block_length_L` (from
C1's `GeneratedPath`), `source_revision_sha` (the resampled real-data revision),
`param_set` (the fixed θ*), then the per-metric distribution table (metrics in the
fixed order `sharpe, sortino, calmar, max_drawdown, total_return`) + the
prob-of-loss / PPSR / DD-tail block. All hashed floats at `{:.6}` (ratios/probs) /
`{:.2}%` (drawdowns) — verbatim `threshold_sweep` conventions.

- **Anchor unit = ONE summary report** (ADR-0051 D4) under the **new namespace
  `mc-robustness-2026-06`**. v0.1.0 adds exactly **+1** anchor. Render the report
  via the existing `backtest::report_body_hash` contract (body = everything after
  the second `---`). `scripts/verify_anchors.sh` is namespace-aware; add the new
  namespace + report-dir to its resolver (the routine additive extension —
  precedent: every prior namespace add; e.g. ADR-0047 D5 / ADR-0045 D2). The
  tester locks the body-SHA after two byte-identical runs.
- **GBM-smoke variant is NON-anchored** and visibly labelled `generator:
  gbm-smoke` (K4) — it proves the N-path harness runs but is never the verdict
  source.

### D-C2.8 — The MANDATORY day-1 gate (R-NR.6, CLAUDE.md non-negotiable)

Two e2e tests, both required from day 1 (the v3-vol-overlay-noop precedent
adapted to a distribution harness):

- **(a) Divergence-from-single-path-baseline (R-NR.6a — catches the harness
  collapsing to one path run N times, the noop-in-harness-clothing signature).**
  Run the single deterministic baseline-path backtest (today's momentum scenario
  on one path) to get `single_baseline_sharpe` (and equity). Run the N-path
  ensemble. Assert the ensemble is NOT a degenerate spike:
  `|p50_ensemble_sharpe − single_baseline_sharpe| ≥ epsilon` **OR**
  `spread = (p95_sharpe − p5_sharpe) ≥ epsilon`. If the per-path seed wiring (D1)
  is broken so all N paths are identical, the spread collapses to 0 and the p50
  equals the single baseline → this test FAILS. **This is the falsification probe
  FP-C2.1.**
- **(b) Two-run byte-identity of the summary body-SHA (R-NR.6b / R3.1).** Run the
  whole ensemble twice at the same `--ensemble-seed`; assert identical
  `report_body_hash`. Extends `tests/determinism.rs` to the ensemble. If a
  reduction is unordered (D2 violated) or a float is unformatted (D3 violated),
  the two runs differ → FAILS.

Together (a)+(b): the distribution is **real** (diverges from the degenerate
single-path) AND **reproducible** (anchorable). This is the literal CLAUDE.md
adaptation the operator asked for. Pattern reference (single-path analogue):
`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.

### D-C2.9 — Non-regression conformance

- `verify_anchors.sh` → 84 existing anchors byte-identical pre/post (R-NR.1; the
  MC code touches none of their code paths — C1's Q-MCB-3 thin-wrap leaves the
  anchored GBM untouched, and the `compute_*` lift is verbatim per R-NR.5).
- Zero behaviour change to `PaperEngine`/`MatchingEngine`/`Strategy`/scenario
  `run()` (R-NR.2). The `compute_*` lift is verbatim relocation (R-NR.5 — assert
  the `threshold_sweep` report, if anchored, stays byte-identical).
- Money math `Decimal`; only the statistical metric layer + the reducer use f64,
  order-fixed per D2 (R-NR.3). `cargo clippy -- -D warnings` + `cargo fmt` clean;
  no `.unwrap()` in library code; the path generator is behind C1's trait so the
  harness is testable with a fake `MonteCarloPathGen` (R-NR.4).

### Falsification probes (C2 — for the developer M-DEV dry-run)

- **FP-C2.1 — force the ensemble to ONE path → the divergence gate MUST FAIL.**
  Temporarily wire all N `path_seed_j` to the same constant (or N=1 replicated to
  N). The R-NR.6a divergence test MUST go red (spread → 0, p50 == single
  baseline). This proves the divergence gate actually detects the noop-collapse —
  if it stays green with one path, the gate is itself a no-op. Revert after.
- **FP-C2.2 — anchor sensitivity to θ* (K3).** Run two ensembles with different
  momentum θ* (e.g. `lookback_minutes` 60 vs 120) at the same `--ensemble-seed`;
  assert the two summary body-SHAs DIFFER. Proves the anchor moves when inputs
  move (the inverse of the noop signature). A SHA that does not move when θ*
  changes means a distribution input is missing from the hashed body (D3 bug).
- **FP-C2.3 — two-run determinism (R3.1).** Same `--ensemble-seed` twice ⇒
  identical body-SHA. (The R-NR.6b gate; run as an adversarial check that no
  reduction snuck in an unordered fold.)
- **FP-C2.4 — generator-label honesty (K4).** A `gbm-smoke` run emits
  `generator: gbm-smoke` in the body and is NOT under the `mc-robustness-2026-06`
  anchor namespace; a `block-bootstrap-real` run emits `generator:
  block-bootstrap-real`. Assert the label matches the generator used (catches an
  accidental GBM-optimistic DD tail).

## Backtest Scenarios

_The v0.1.0 anchored scenario: **cross-sectional momentum (v1 baseline) at its
shipped `θ*` over N block-bootstrap paths of 2023-FY real Binance returns** →
one distribution-summary report under namespace `mc-robustness-2026-06`.
architect + analyst finalize the exact scenario name + source-revision pin at
M-T1. The GBM-smoke variant is a separate, visibly-labelled
(`generator: gbm-smoke`) NON-anchored smoke-test that proves the N-path harness
runs._

## Implementation

**As-built by developer (2026-05-30).**

### New modules

| File | Role |
|------|------|
| `crates/backtest/src/stats/mod.rs` | M-DEV-1: verbatim calculator lift + M-DEV-2: `DistributionSummary` reducer |
| `crates/backtest/src/scenarios/montecarlo.rs` | M-DEV-3: `run_path` cell wrapper (MomentumStrategy, not TCN overlay) |
| `crates/backtest/src/bin/monte_carlo.rs` | M-DEV-4+5: driver + report renderer (ADR-0051 D1/D3/D4) |
| `crates/backtest/tests/montecarlo_e2e.rs` | M-DEV-6: mandatory day-1 e2e gate (R-NR.6 adapted) |

### Chosen N + measured wall-clock

- **N = 500** (Q-RH-1 Option A, RATIFIED — durable; stable p5/p95 tail).
- **Wall-clock: 183.5 seconds** on Apple-Silicon M-series (~11.5 rayon cores).
- N=500 is well under the 10-min budget; N=200 fallback NOT needed.

### First distribution-summary numbers (headline result)

Scenario: v1 cross-sectional momentum at shipped θ* over N=500 block-bootstrap paths of 2023-FY real Binance returns (10 USDT pairs, hourly, 8760 bars). Block length Auto-selected L=204.

| Metric | p5 | p50 | p95 |
|--------|------|------|------|
| Sharpe | −0.068 | **−0.022** | 0.003 |
| Sortino | −0.095 | −0.031 | 0.004 |
| Calmar | −0.311 | −0.109 | 0.017 |
| Max drawdown | 61.3% | **85.3%** | 100.0% |
| Total return | −97.6% | −60.5% | +11.5% |

Ensemble robustness:
- P(final_equity < initial): **86.8%**
- P(Sharpe > 0): 13.2%
- P(Sharpe > 1.0): 0.0%
- MaxDD tail p50 / p95: **85.3%** / **100%**

**Verdict: WEAK (v0.1.0; FRAGILE per decision rule — see v0.1.1 re-run below).** p50 Sharpe ≤ 0; the v1 momentum strategy is NOT robust on 2023-FY block-bootstrap paths of real Binance data. The paper→live gate is BLOCKED by this distribution. This is the honest answer to "is v1 momentum robust or one lucky path?": the real 2023 path had Sharpe **0.003** (NOT 1.40 — the "1.40" was a fabricated number from product.md:78, retracted per adversarial review 2026-05-30; see Correction A below); the ensemble confirms the real path is already near the p95 of the ensemble — plain v1 momentum was already weak on real 2023 and the distribution confirms it is also fragile under resampling.

### Anchor (v0.1.0 — superseded by v0.1.1 re-emission)

- Body SHA (v0.1.0, BUG): `72fc7089c5f04885e8a2169d91c242a50e47b7820eea38b446a4dfaa2c1938c4`
- **RETIRED** per ADR-0038 §D6.b wiring-bug-fix re-emission (Bug B — cash solvency).
- See v0.1.1 § Implementation below for the corrected anchor.

### Design notes / deviations from spec

1. **Equity clamping for NaN prevention**: When equity goes negative on a ruin path (GBM smoke test with high volatility), `compute_sharpe_hourly` would produce NaN via `ln(curr/prev)` with `curr < 0`. The driver (`run_one_path`) clamps equity to `1e-6` Decimal before calling the calculators. This is NOT a change to the verbatim-lifted calculator functions (R-NR.5 preserved); it's a driver-level guard. The ADR-0051 D2 NaN-absent assertion is enforced by construction (the clamp prevents NaN, `debug_assert!` catches any structural failure). This clamping makes ruin paths produce a large-negative finite Sharpe rather than NaN — correct behavior for a ruin path.

2. **`run_path` loads config for universe but uses caller-supplied strategy**: To keep the bar-loop simple and consistent with `run_cell`, `run_path` loads the momentum config (universe list) but uses the caller-supplied `MomentumStrategy` instance. The config load is for universe metadata only; the actual strategy is constructed once per path in the driver and passed in.

3. **Generator-label honesty**: The `generator: gbm-smoke` label in the body is produced by the `GeneratorKind::label()` method, not a hardcoded string. The `mc-robustness-2026-06` anchor check in `verify_anchors.sh` only looks in `spec/strategy-robustness-harness/reports/` — a GBM smoke run written to `/tmp/` cannot accidentally satisfy it (K4 mitigated by directory scoping + label check).

4. **`sim.rs` pre-existing clippy errors**: `cargo clippy -p backtest --tests -- -D warnings` shows pre-existing errors in `engine.rs`, `paths.rs`, `progress.rs`, `sma_composed_run.rs`, `cli_types.rs`, and `sim.rs`. These are baseline errors that existed before this feature and are NOT introduced by C2 code. The new lib code (`stats/mod.rs`, `scenarios/montecarlo.rs`) has zero new errors.

## Implementation v0.1.1 (developer 2026-05-30 — Bug B fix + Correction A + re-anchor)

### Bug B fix — long-only solvency guard (`montecarlo.rs`)

**Root cause**: `crates/backtest/src/scenarios/montecarlo.rs` Buy branch computed
`notional = equity * 0.10` and then `cash -= notional_fill + fee` with NO check
that cash was sufficient. On fee-churn paths (5,343 trades/year on resampled
momentum data), `cash` went negative → `equity` went negative → driver clamped
to `1e-6` → false 100% MaxDD / −100% total_return (impossible for a long-only
book where no coin fell > 52%).

**Fix** (v0.1.1, `crates/backtest/src/scenarios/montecarlo.rs`):

1. Cap notional against available cash: `notional = min(equity * 0.10, cash)`.
2. Pre-flight solvency check: `if cash < notional + fee_estimate { continue; }`.
3. Defensive guard inside fill loop: `if total_cost > cash { warn and skip fill; }`.

The strategy's 10%-of-equity intent is preserved when cash is sufficient. Cash and
equity are guaranteed >= 0 at all steps on all paths.

### Correction A — fabricated Sharpe 1.40 retracted (`feature.md:41`)

The motivating table's "Point-estimate today: Sharpe 1.40" was FABRICATED (traced
to `product.md:78` LLM-narration example). The real harness Sharpe on the
chronological 2023 path is **0.003** (total return +13.48%, maxDD 73.73%).
The table entry is corrected to `~0.00`. The implementation note in § Implementation
v0.1.0 referencing "Sharpe ~1.40" is retracted and corrected.

### Solvency invariant tests (day-1 gate — CLAUDE.md non-negotiable)

Two new tests in `crates/backtest/tests/montecarlo_e2e.rs` (v0.1.1):

- `solvency_invariant_equity_curve_never_negative_across_paths`: asserts
  `max_dd_tail_p95 <= 1.0` AND `total_return.min > -1.0` across N=20 synthetic paths.
  A MaxDD > 1.0 (100%) is only possible if equity went negative — the pre-v0.1.1 signature.
- `solvency_guard_arithmetic_unit_test`: directly tests the Bug B solvency guard arithmetic
  with a concrete cash=$50 / equity=$10,050 / target_notional=$1,005 scenario, proving:
  (a) old code (no cap, no check) drives cash to -$959 (impossible); (b) the cap reduces to
  $50 but fee still pushes negative → the pre-flight check fires correctly; (c) with large
  cash ($100k) the buy DOES go through (guard is not over-conservative).

**FP-C2.1 red-on-bug proof** (per the honest-tick rule): both tests would fail if
the solvency guard were removed (reverting the `min(target, cash)` cap). The
`solvency_guard_arithmetic_unit_test` explicitly computes `cash_after_old_bug < 0`
and asserts it, proving the old code would produce impossible negative cash.

### Re-anchor (ADR-0038 §D6.b wiring-bug-fix re-emission)

- **Old SHA (v0.1.0, Bug B)**: `72fc7089c5f04885e8a2169d91c242a50e47b7820eea38b446a4dfaa2c1938c4`
- **New SHA (v0.1.1, fixed)**: `7dbf562887cbf6790f6a85b5276392388f429d098a955a139d81eedc7fd0ef20`
- Report: `robustness-20260530-130137-v1-momentum-2023-block-bootstrap-real-fy-mc.md`
- Wall-clock: **179.4s** on Apple-Silicon M-series (N=500, rayon ~11.5 cores).
- Determinism: byte-identical body SHA across 2 independent N=10 runs (FP-C2.3 PASS for N=10;
  tester to verify full N=500 second run).
- `scripts/verify_anchors.sh` → **85/85 PASS** with new SHA.

### New distribution numbers (v0.1.1 — realistic tail, same FRAGILE verdict)

| Metric | p5 | p50 | p95 |
|--------|-----|------|------|
| Sharpe | -0.050 | **-0.010** | 0.009 |
| Sortino | -0.071 | -0.015 | 0.013 |
| Calmar | -0.187 | -0.049 | 0.044 |
| Max drawdown | 61.3% | **81.4%** | **91.5%** |
| Total return | -84.2% | -31.5% | +39.3% |

Ensemble robustness:
- P(final_equity < initial): **75.2%** (was 86.8% with false ruin paths)
- P(Sharpe > 0): 24.8% (was 13.2%)
- P(Sharpe > 1.0): 0.0% (unchanged)
- MaxDD tail p50 / p95: **81.4%** / **91.5%** (was 85.3% / **100%** — the artifact)

**Verdict: FRAGILE** (per decision rule, superseding the v0.1.0 "WEAK" label). All 5
primary signals are FRAGILE: p5 Sharpe = −0.050 < 0; p50 Sharpe = −0.010 < 0.5;
P(loss) = 75.2% >> 35%; P(Sharpe>1) = 0%; p95 MaxDD = 91.5% >> 70%.
The p95 MaxDD drop from 100% to 91.5% removes the "impossible full wipeout" artifact
but does NOT rescue the verdict — 91.5% is still firmly in the FRAGILE band (> 70%).
The decision-rule panel remains FRAGILE; the paper→live gate is BLOCKED.

## Verification

_tester links to reports here. The day-1 gates: R-NR.6(a) divergence-from-
single-path-baseline + R-NR.6(b) two-run byte-identity + R3.2 single new anchor +
K3 anchor-sensitivity-to-`θ*`._

## Changelog

- 2026-05-30 (analyst): Feature brief authored as **C2** of the Monte-Carlo
  robustness lane (M0 pass), under the operator's 4 locked strategic decisions
  (2026-05-30): Q1 = stationary block bootstrap (consumed via C1) / GBM demoted;
  Q2 = seed the ensemble → anchor the distribution summary (ONE anchor, not N);
  Q3 = robustness harness first / learning loop (C4) last. Grounded in
  [`strategy-robustness-monte-carlo-direction-2026-05-29.md`](../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md)
  § 3 / § 6 (C2) + the architect readiness audit
  [`monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
  § 2(c) / § 3 / § 6.2 Phase MC-1 (reuses the `threshold_sweep` sweep+aggregate
  seam). R1-R3 + R-NR (6 clauses, incl. the MANDATORY adapted CLAUDE.md
  distribution-harness gate R-NR.6 = divergence-from-single-path-baseline +
  two-run byte-identity) + K1-K4 + H1-H3 + Q-RH-1/2 + pre-drawn 4-cell verdict
  tree. **ADR-0051 FLAGGED for the architect** (D1–D5 contract restated; next
  free number confirmed 0051; this brief does NOT author it). Depends on C1
  ([`monte-carlo-bootstrap-path-generator`](../monte-carlo-bootstrap-path-generator/feature.md)).
  Trace row `REQ-STRATEGY-ROBUSTNESS-HARNESS-001` opened `proposed`. HANDOFF →
  architect (M-T1 + ADR-0051; C1+C2 bundle).
- 2026-05-30 (architect, M-T1): `## Design` authored (D-C2.1..D-C2.9) + 4
  falsification probes (FP-C2.1..FP-C2.4). **ADR-0051 AUTHORED** (D1-D5,
  status accepted, atomically registered in the ADR README) — discharges the
  § ADR flag. **Q-RH-1 = N=500 RATIFIED** (stable p5/p95 tail; N is a hashed
  body field so picking a defensible N now avoids a v0.2.0 re-anchor; wall-clock
  sized as minutes-not-hours on the canonical box; N=200 the if-budget-tightens
  fallback pending the M-DEV dry-run confirming < ~10 min; mandatory `watch -n 10`
  probe spec'd). **Q-RH-2 = dedicated `bin/monte_carlo.rs` RATIFIED** (mirrors the
  `bin/threshold_sweep.rs` precedent; keeps the single-path 84-anchor CLI cleanly
  separated; the MC bin family carries C3/C5). Confirmed at code level that
  `threshold_sweep::run_cell` generalizes (it takes `bars_override` + a
  caller-supplied strategy + a fixed seed and returns `equity_curve: Vec<Decimal>`)
  → `scenarios::montecarlo::run_path` is a behaviour-preserving sibling (resolves
  direction § 8 open-Q3). Ensemble-seed-vs-fill-tie-break orthogonality resolved
  (direction § 8 open-Q2): `--ensemble-seed` varies the paths via D1; the
  fill-tie-break seed is HELD CONSTANT at `0xC0FFEE` across all paths so the only
  varying input is the path itself (no confounding tie-break noise). `compute_*`
  calculators lift verbatim from the `threshold_sweep` bin into a new
  `backtest::stats` shared module (R-NR.5 behaviour-preserving); the only new math
  is `stats::DistributionSummary` (index-order reduction + `total_cmp` sort +
  type-7 linear percentile per ADR-0051 D2). Day-1 gate (R-NR.6) spec'd as two
  e2e tests: (a) divergence-from-single-path-baseline + (b) two-run byte-identity.
  +1 anchor under new namespace `mc-robustness-2026-06` (D4). Trace `arch` filled
  (+ADR-0051 added); status proposed → arch-done; owner → developer. `tasks.md`
  created with M-DEV rows. HANDOFF → developer (build C1 first, then C2).
