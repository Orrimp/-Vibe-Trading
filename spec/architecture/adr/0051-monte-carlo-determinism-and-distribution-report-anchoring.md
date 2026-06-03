---
adr: 0051
title: Monte-Carlo robustness — sub-seed derivation, distribution-report shape, and anchor determinism
status: accepted
date: 2026-05-30
deciders: analyst (M0, 2026-05-30) → architect (M-T1, 2026-05-30)
supersedes: []
superseded_by: []
related:
  - "ADR-0002 rng-chacha20"
  - "ADR-0003 decimal-money-math"
  - "ADR-0032 backtest-realdata-path-and-revision-pin"
  - "ADR-0038 vol-forecast-verdict-shape (anchor-bounded-set / byte-immutable reports)"
  - "ADR-0043 simulated-latency-and-slippage (f64 conversion-boundary determinism scope)"
---

# ADR-0051 — Monte-Carlo robustness: sub-seed derivation, distribution-report shape, and anchor determinism

> Locks the determinism + anchoring contract for the Monte-Carlo robustness
> lane (C1 path generator + C2 robustness harness). C1
> ([`monte-carlo-bootstrap-path-generator`](../../monte-carlo-bootstrap-path-generator/feature.md))
> produces a deterministic `Vec<Vec<Bar>>` ensemble from a single `path_seed: u64`;
> C2 ([`strategy-robustness-harness`](../../strategy-robustness-harness/feature.md))
> derives the N per-path seeds, runs the existing single-path backtest per path,
> reduces the N outcomes to a distribution summary, and emits **ONE** anchored
> report. This ADR is the contract that makes a *stochastic* method coexist with
> the project's byte-identical anchor gate (`scripts/verify_anchors.sh`, 84 locked
> body-SHAs, ADR-0038 § D6 byte-immutable reports).

## Context

The whole quality gate assumes **one deterministic output per scenario**. Monte
Carlo is, naively, stochastic: a robustness run produces a *distribution* of
outcomes over an ensemble of synthetic paths. The architect readiness audit
([`monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../../dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
§ 3) established that seeded determinism **does** recover anchorability, but only
under five sub-decisions that must be frozen so the summary body-SHA never drifts:

1. how one master seed maps to the N per-path seeds (and that the mapping is
   bound to the path *index*, never to parallel-completion order);
2. the f64 reduction order + percentile-selection rule (f64 mean/std are
   non-associative; an unordered parallel fold flaps the last bit);
3. which report fields are run-varying (front-matter, not hashed) vs
   distribution-input (body, hashed), and the fixed decimal precision of every
   hashed float;
4. the anchor *unit* — one summary report, not N per-path reports;
5. the determinism *scope* — Apple-Silicon canonical box, inheriting ADR-0043's
   f64-conversion-boundary precedent verbatim; cross-platform byte-identity is
   explicitly NOT contracted.

The operator locked four strategic decisions (2026-05-30): Q1 stationary block
bootstrap first (GBM demoted to smoke-test); **Q2 seed the ensemble → anchor ONE
distribution-summary report**; Q3 harness-first / learning-loop-last; Q4
LLM-as-support. Q2 is the load-bearing input to this ADR.

> **Retired-line guard (standing).** GARCH and Markov/regime-switching were
> retired as *alpha sources*
> ([`v3-vol-retirement-and-c5-promotion-2026-05-22.md`](../../dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md)).
> The Monte-Carlo lane treats such models only as **synthetic-path / data
> generators** — a categorically distinct role (it produces price paths to
> stress an *already-shipped* strategy on; it predicts nothing and claims no
> alpha). v0.1.0 ships **only** the stationary block bootstrap of real returns
> (no GARCH, no regime gen). This distinction is stated here so a future reviewer
> does not reject the lane on sight; it is NOT relitigated.

## Decision

### D1 — Master-seed → per-path sub-seed derivation rule (frozen)

The harness (C2) owns the master seed and derives one `path_seed_j` per path
index `j ∈ 0..N`. The frozen rule is the project's existing splitmix-style
additive idiom, extended to the path axis:

```text
path_seed_j = master_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))
```

- `0x9E37_79B9` is the 32-bit golden-ratio increment already used verbatim for
  per-symbol seed derivation at `momentum.rs:245` and `threshold_sweep.rs:120`
  (`sym_seed = seed.wrapping_add(idx * 0x9E37_79B9)`). D1 reuses the **same
  constant and the same `wrapping_add(idx * k)` shape** on a different axis (path
  index instead of symbol index), so the codebase has exactly one seed-mixing
  idiom across both axes.
- **Bound to the index `j`, never to completion order.** The N paths run in
  parallel (rayon, D-RH below); the seed a path receives is a pure function of
  its index, so parallel scheduling cannot change which seed any path gets. This
  is the same invariant `threshold_sweep` enforces by sorting cells before
  render (R9/K3 there); here it is enforced one step earlier, at seed assignment.
- **Composition with the per-symbol axis (C1 internal).** Within a single path,
  C1's shared-index block bootstrap (Q-MCB-2 = shared-index, ratified in C1's
  `## Design`) draws ONE resampling-index sequence per path from a single
  `ChaCha20Rng::seed_from_u64(path_seed_j)` and applies it to all symbols. The
  per-symbol price reconstruction consumes the SAME shared index sequence — there
  is **no second per-symbol seed mix inside a path** (that would decorrelate the
  symbols, defeating Q-MCB-2). So the two-level seed hierarchy is:
  `master_seed → path_seed_j (D1) → one ChaCha20 stream per path → one shared
  block-index sequence applied across all symbols`.
- **Orthogonality to the fill-tie-break seed (resolves direction-note § 8
  open-Q2).** Today `--seed` (default `0xC0FFEE`) drives BOTH the synthetic
  fixture AND `PaperEngine`/`MatchingEngine` fill tie-breaking. In a robustness
  run the *path* is supplied by C1's ensemble, so the path no longer comes from
  the engine seed. D1 makes the **ensemble master seed orthogonal to the
  fill-tie-break seed**: C2 exposes `--ensemble-seed` (default `0xC0FFEE`) for
  the path stream, and the per-path backtest is run with a **fixed** fill-tie-break
  seed `0xC0FFEE` (the canonical value) for ALL paths. Rationale: the robustness
  question is "how does the outcome vary as the *path* varies", so the fill
  tie-break must be held constant across paths or it injects a second,
  confounding noise source. Both seeds are printed in the hashed body (D3) so the
  anchor is sensitive to either changing. At v0.1.0 they take the same canonical
  literal (`0xC0FFEE`); the flag exists so a future run can vary the ensemble
  seed without touching the fill-tie-break seed.

### D2 — Aggregation reduction order + percentile-selection rule (the f64 boundary)

The reducer consumes N per-path scalar metrics (Sharpe, Sortino, Calmar,
max-drawdown, total-return — each computed by the existing `compute_*` free
functions lifted to a shared module per C2 R-NR.5) and emits, per metric,
`{mean, std, p5, p25, p50, p75, p95, min, max}` plus the ensemble-level
`P(final_equity < initial)`, `P(Sharpe > 0)`, `P(Sharpe > 1.0)`. The reduction is
frozen as follows:

1. **Collect, then index-order.** Collect the N per-path metric values into a
   `Vec<f64>` **indexed by path index `j`** (NOT by completion order). The
   parallel map returns `(j, metric)` pairs (or writes into a pre-sized `Vec` at
   index `j`); the collected vector is in ascending-`j` order before any
   reduction. An unordered parallel fold over the metrics is **forbidden**.
2. **Mean** = sequential left-fold sum in ascending-`j` order, divided by `N`:
   `mean = (Σ_{j=0}^{N-1} x_j) / N`, summed in index order with a single `f64`
   accumulator. (f64 addition is non-associative; the order is fixed so the sum
   is byte-reproducible.)
3. **Std** = population std with the same fixed order:
   `var = (Σ_{j=0}^{N-1} (x_j − mean)^2) / N`, `std = var.sqrt()`. (`f64::sqrt`
   is IEEE-754 correctly-rounded → reproducible on a fixed arch.) Single-pass or
   two-pass is an implementation choice **as long as it is the same every run**;
   the design specifies **two-pass** (compute `mean` first, then the
   centered-square sum) to avoid catastrophic-cancellation variance between a
   one-pass and two-pass formula — pin two-pass in the code so it never silently
   switches.
4. **Percentile selection** = **sort-then-index, nearest-rank with linear
   interpolation (type-7 / "linear" quantile), NaN asserted absent.** Steps:
   - Assert no `NaN` in the N samples (a `NaN` Sharpe is a strategy/data bug, not
     a tail; fail loudly rather than silently sorting it to an end).
   - Sort the N samples ascending with `f64::total_cmp` (a *total* order over
     f64, so ties and signed-zero are deterministic — never `partial_cmp` +
     `unwrap`, which is undefined on `NaN` and was the latent hazard the audit
     flagged).
   - For percentile `p ∈ {5,25,50,75,95}`, the rank is `h = (N − 1) · p/100`
     (zero-based, type-7); the value is
     `sorted[floor(h)] + (h − floor(h)) · (sorted[ceil(h)] − sorted[floor(h)])`.
     This is the R-default / NumPy-default `"linear"` method; freezing the method
     name in this ADR is what makes p50 byte-stable run-to-run.
   - `min = sorted[0]`, `max = sorted[N-1]`.
5. **Probabilities** are exact integer counts over the index-ordered vector
   divided by `N`: e.g. `P(Sharpe > 1.0) = (#{j : sharpe_j > 1.0}) / N`. The
   comparison threshold (`> 0`, `> 1.0`) is a fixed f64 literal; the count is an
   integer so it is platform-independent; only the final division is f64 and is
   formatted at fixed precision (D3).

The reduction order + the `total_cmp` sort + the frozen `"linear"` percentile
method are jointly the **f64 cross-platform-determinism boundary**: they
guarantee byte-stability **on the canonical box** (D5), and they make the
*selection* (which sample lands at p95) deterministic everywhere even when the
last-bit f64 *value* might differ off-box.

### D3 — Distribution-report front-matter / body split + fixed-precision formatting

A new `robustness-*.md` report type (C2 R2.4; operator Q-MC-2 durable path). The
split mirrors `threshold_sweep`'s proven layout and the standing
report-format guardrail (architect.md § Determinism & report-format guardrails).

**Front-matter (run-varying; NOT hashed — stripped before `report_body_hash`):**

```yaml
---
slug: strategy-robustness-harness
scenario: <scenario-name>
generated: <ISO-8601 wall-clock>
wall_clock_s: <float>
host: <hostname>
pid: <int>
git_commit: <sha>
data_revision_sha: <sha>     # forensic copy; the body carries the load-bearing one
---
```

**Body (deterministic; hashed by the anchor) — every distribution input is
printed so changing any of them changes the SHA:**

```text
master_seed:        0xC0FFEE          # the --ensemble-seed (D1)
fill_seed:          0xC0FFEE          # the per-path fill-tie-break seed (D1)
n_paths:            <N>
sub_seed_rule:      "master + j*0x9E3779B9"   # frozen D1 string
reduction_rule:     "index-order mean/std; total_cmp sort; type-7 linear pct"  # frozen D2 string
generator:          block-bootstrap-real | gbm-smoke
bootstrap_mode:     shared-index | per-symbol-independent   # C1 K3
block_length_policy: auto | fixed(<L>)
selected_block_length_L: <usize>      # the Auto-chosen L (a distribution input — D2 of C1)
source_revision_sha: <sha>            # the resampled real-data revision (C1 R2.3)
param_set:          <the fixed θ* — momentum lookback/rebalance/k_long/exposure_cap/drift/vol_floor>
--- per-metric distribution table (metrics in a FIXED order) ---
metric  | mean | std | p5 | p25 | p50 | p75 | p95 | min | max
sharpe  | ...
sortino | ...
calmar  | ...
max_drawdown | ...
total_return | ...
--- ensemble robustness block ---
P(final_equity < initial): <f>
P(Sharpe > 0):             <f>
P(Sharpe > 1.0):           <f>
max_drawdown_tail: p50=<f> p95=<f>     # the headline paper→live gate number
```

**Fixed-precision formatting (the single most effective determinism lever —
audit § 3.4 mitigation #2).** Every hashed float is formatted at a frozen
precision before it enters the body string: ratios (`sharpe`, `sortino`,
`calmar`, `total_return`, probabilities) at `{:.6}`; percentage-style drawdowns
at `{:.2}%` (matching `threshold_sweep`'s `{:.6}` / `{:+.6}` / `{:.2}%`
conventions verbatim). Hashing the *formatted decimal string* at fixed precision
absorbs sub-precision last-bit noise: a 6-dp Sharpe is byte-identical even if the
underlying f64 differs in bit 52. **Metric rows are emitted in a fixed
lexical/declared order** (the `sharpe, sortino, calmar, max_drawdown,
total_return` order above), never collection/iteration order, so the body is
byte-stable (mirrors `threshold_sweep`'s sort-before-render).

> **Optional Decimal-quantized selection (deferred, gated on cross-platform
> need).** Audit § 3.4 mitigation #3: percentile *selection* could be done on
> `Decimal`-quantized metrics (round each per-path metric to fixed dp as Decimal,
> sort, index) to make selection itself platform-independent. This is a durable
> upgrade but is **NOT adopted at v0.1.0** because cross-platform parity is out of
> scope (D5). The fixed-precision *formatting* of D3 already makes the hashed
> body byte-stable on the canonical box. If cross-platform parity is ever wanted,
> adopt Decimal-quantized selection then (a body-SHA-neutral internal change as
> long as the formatted output is identical on the canonical box).

### D4 — Anchor unit = ONE summary report per scenario (reject per-path anchors)

The anchor is the **body-SHA-256 of the single distribution-summary report**
(audit § 3.3 Option A; operator Q2 locked). v0.1.0 adds exactly **+1** anchor per
MC scenario under a **new namespace** (`mc-robustness-2026-06`).

Rejected alternative — **N per-path anchors** (audit § 3.3 Option B): one anchor
per synthetic path. Rejected because it is an N×-explosion of the anchor set
(`verify_anchors.sh` cost ×N), brittle (any change to `N` re-locks every per-path
anchor), and defeats the point — the *distribution* is the deliverable, not any
single path, and every path is reproducible from the master seed + D1 anyway.

Rationale for one-report:
- Matches the `threshold_sweep` precedent exactly (N cells → 1 summary → would be
  1 anchor); the harness shape is the dual (N paths → 1 summary → 1 anchor).
- Survives an `N` change gracefully: bumping `N` changes the one body-SHA (because
  `n_paths` is a hashed body field per D3), which is *correct* — a different `N`
  is a different distribution. No fan-out of re-locks.
- The audit § 3.3 Option C (summary anchored + a sampled-path digest embedded in
  the hashed body for extra tamper-evidence) is an **acceptable future upgrade**
  of D4, not adopted at v0.1.0 (keeps the body minimal).

### D5 — Determinism scope = Apple-Silicon canonical box (inherit ADR-0043 verbatim)

Byte-identity of the distribution summary is contracted **only on the
Apple-Silicon canonical box**. Cross-platform byte-identity is **explicitly NOT
contracted**, inheriting the ADR-0043 § "f64 conversion boundary" precedent
(lines 301-310) verbatim:

- **Same-machine (canonical box): LOW risk.** `f64::sqrt`/`ln` are IEEE-754
  correctly-rounded; D2's fixed reduction order + D3's fixed-precision formatting
  make the whole pipeline reproducible. This is the regime the existing 84
  anchors already live in (`verify_anchors.sh` runs on the canonical box; CI
  carries no real-data parquets and skips the realdata anchors).
- **Cross-platform (x86 CI, different libm, FMA contraction): MEDIUM→HIGH and
  out of scope.** `ln` is not required to be correctly-rounded by IEEE-754; libm
  differs in the last bits; FMA-contraction of `a*b+c` differs by target. A
  cross-platform byte-identical summary would require pinned softfloat libm +
  `-C target-feature` lockdown + no-FMA — a separate, larger effort with its own
  ADR. **Named, not built.**

Net: seeded determinism (D1) + fixed reduction order (D2) + fixed-precision
formatted body (D3) make the distribution summary anchorable **on the canonical
box** with the same confidence as today's 84 anchors. Cross-platform is a known,
accepted limitation, not a new one.

### D6 — θ-sweep extension: two-axis sub-seed composition + θ-surface anchor unit (C3 amendment, 2026-05-30)

> **Amendment, not a new ADR.** C3
> ([`momentum-parameter-robustness-sweep`](../../momentum-parameter-robustness-sweep/feature.md))
> wraps the C2 inner harness in an outer θ-loop: for each θ-cell `g` in a
> bounded grid it runs the N-path harness (D1-D4 unchanged) and collects one
> `DistributionSummary`, then emits ONE θ-surface report. C3 adds NO new
> determinism mechanism — it composes D1 on a second axis and reuses D2/D3/D4/D5
> verbatim. Per the architect's `analyst-defaults` cheap-and-correct exception
> (C3 § ADR flag), this is an ADR-0051 amendment because the seed idiom, the
> FM/body split, the fixed-precision formatting, and the one-report anchor unit
> are all reused without change; only the axis count and the report shape change.

**D6.1 — θ-axis seed rule = SAME path-set across all cells (OQ-1 ratified).**
Every θ-cell is judged on the **identical** N resampled histories: the only input
that varies between θ-surface rows is θ itself. Concretely the per-path seed at
cell `g`, path `j` is

```text
cell_seed_g     = ensemble_seed                                   # θ-axis collapsed: SAME for every g
path_seed_{g,j} = cell_seed_g.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))   # = the D1 rule, verbatim
```

i.e. `path_seed_{g,j} = ensemble_seed.wrapping_add(j * 0x9E37_79B9)` for **all**
`g` — **byte-identical to the C2 single-axis D1 seed**. The θ-axis is varied at
the **config level** (a per-cell `CrossSectionalMomentumConfig`), NOT at the seed
level. Consequences:

1. **C1/C2 determinism is unchanged by construction.** Because the θ-axis does not
   touch the seed stream, the C2 path generator, the C2 per-path seeds, and the C2
   reducer are bit-for-bit what they were. The 85 existing anchors (C2 ship) stay
   byte-identical — there is no seed-arithmetic change to audit. This is the
   strongest possible form of the "extends D1 without changing C1/C2
   determinism" requirement: the extension is the *empty change* on the seed axis.
   (Verified arithmetically: with `cell_seed_g := ensemble_seed`, `path_seed_{g,j}`
   equals the C2 `derive_path_seed(ensemble_seed, j)` for every `j` — see § Changelog
   2026-05-30 amendment for the check.)
2. **The θ-surface is a controlled experiment.** Cell `g_a` and cell `g_b` consume
   the identical family of adverse paths; the only causal difference is θ, so any
   difference in their distributions is attributable to θ alone (the cleanest
   anti-confounder — it removes path-sampling variance as an alternative
   explanation for inter-cell differences). This is the same reasoning C2 used to
   hold the fill-tie-break seed constant (D1), now applied to the path stream
   across the θ-axis.
3. **The fill-tie-break seed stays constant (D1 inherited).** `fill_seed = 0xC0FFEE`
   for every (cell, path), exactly as in C2.
4. **L is θ-independent and printed once.** The auto-selected block length is
   computed by Politis–White on the source series' universe-average |log-return|,
   which does not depend on θ; so L is constant across the entire θ-grid and is a
   single shared-input line in the surface header (NOT a per-row field). This is a
   property of D6.1 (SAME source paths ⇒ SAME L) and resolves OQ-3.

**D6.2 — REJECTED: naive additive two-axis composition (the collision hazard).**
The C3 brief's D-C3.4 *fallback* proposed an independent-paths-per-cell rule
`cell_seed_g = ensemble_seed + g*0x9E37_79B9` then
`path_seed_{g,j} = cell_seed_g + j*0x9E37_79B9`. This is **rejected** for v0.1.0 on
two grounds. (a) **It is a seed-collision bug:** the composition collapses to
`ensemble_seed + (g+j)*0x9E37_79B9`, so `(g, j)` and `(g−1, j+1)` receive the
*same* path seed — distinct (cell, path) pairs share a resampled path, which both
corrupts the per-cell distribution and is not the "independent paths" the rule
intends. (Verified arithmetically: a 4×3 (g, j) grid yields 6 colliding pairs.) A
correct independent-paths rule would need a non-commuting mix (e.g. a
`splitmix64`/`hash(g) ⊕ j` two-stage derivation), which is *new* determinism
machinery with its own audit burden — exactly what an amendment must not smuggle
in. (b) **It is not the question C3 asks.** Independent paths per cell would
reintroduce path-sampling variance as a confound between rows; SAME-paths is the
methodologically superior choice regardless of the collision. If a future C3.x
ever wants per-cell path independence (e.g. to estimate inter-cell sampling
variance), it requires its **own** ADR specifying a collision-free two-axis mix —
it does NOT inherit D6.1.

**D6.3 — Anchor unit = ONE θ-surface report (D4 extended to the grid axis).**
C3 adds exactly **+1** anchor under the existing `mc-robustness-2026-06` namespace
(D4): the body-SHA-256 of the single θ-surface report. The *surface* is the
deliverable, not any single cell; every cell is reproducible from
`(ensemble_seed, the frozen cell list, N)`. This is the dual of D4's
one-report-per-N rule: C3 is one-report-per-grid. An N-per-θ anchor set is
rejected for the same reason D4 rejected per-path anchors (a G×-explosion that
re-locks on any grid change). The **θ-grid definition (the explicit cell list),
the per-cell N, and the buy-and-hold control flag are hashed body fields** — a
different grid or a different N is a different surface and MUST move the SHA (K3).
A Tier-2 refine over a different grid is therefore a **separate** run = a
**separate** anchor, never a mutation of the Tier-1 surface.

**D6.4 — Reduction, FM/body split, precision, scope all inherited verbatim.**
Inside each cell the reduction is the unchanged D2 (index-order mean/two-pass std +
`total_cmp` sort + type-7 linear percentile + NaN-absent). The θ-surface report
obeys the D3 FM/body split (run-varying fields in front-matter, every distribution
input in the hashed body) and D3 fixed-precision formatting (`{:.6}` ratios /
`{:.2}%` drawdowns), with **rows sorted by θ-cell index before render** so the body
is order-invariant (the `threshold_sweep` sort-before-render discipline, also used
by the C2 driver's `indexed.sort_by_key(|r| r.j)`). Determinism scope is D5
(Apple-Silicon canonical box; cross-platform NOT contracted). The per-cell verdict
classifier and the family-summary line are deterministic pure functions of the
hashed distribution numbers and the frozen decision-rule bands, so they add no new
determinism surface.

**D6.5 — Strategy-family axis (Momentum vs Reversion) inherits D6.1 (cross-sectional MR amendment, 2026-05-31).**

> **Cross-reference amendment, not a new ADR.** The first robustness pivot
> ([`cross-sectional-mean-reversion-strategy`](../../cross-sectional-mean-reversion-strategy/feature.md),
> § D-MR.6) sweeps a **second strategy family** (cross-sectional mean-reversion =
> the v1 score negated) through the **same C3 θ-surface machinery**. Like the
> θ-axis (D6.1), the **family axis is varied at the strategy/config level — a
> `Direction { Momentum, Reversion }` field on `CrossSectionalMomentumConfig` —
> NOT at the seed level.** Therefore it is the *second instance* of the
> "vary at config level, seed stream untouched ⇒ determinism unchanged by
> construction" pattern, and it inherits D6.1/D6.3/D6.4 with **no new
> determinism mechanism**. Per the architect's `analyst-defaults` cheap-and-
> correct exception (identical to the D6 amendment rationale), this is an
> ADR-0051 cross-reference amendment because the seed idiom, the FM/body split,
> the fixed-precision formatting, and the one-report anchor unit are all reused
> verbatim; only the family-selection logic inside the strategy changes.

1. **SAME-paths (D6.1) holds verbatim.** `path_seed_{g,j} =
   ensemble_seed.wrapping_add(j * 0x9E37_79B9)` for all MR cells — byte-identical
   to the C2/C3 D1 seed. The MR family is the *empty change* on the seed axis (the
   same strongest-form argument D6.1 makes for the θ-axis), so the **86 existing
   anchors hold by construction** — there is no seed arithmetic to audit. The MR
   sweep at year=2023 consumes the byte-identical resampled paths the C3 momentum
   sweep does, so **L is family-independent** and printed once (D6.1.4 inherited).
2. **The signal inversion is anchor-neutral by location.** MR negates the
   `Decimal` output of the anchored `score_vol_adjusted_return` at one cache-write
   line in `MomentumStrategy::on_bar`; the anchored `top_k_long` selector (sorts
   descending, takes top-K) is reused verbatim and naturally selects the bottom-K
   on negated scores. No feature-crate edit, no selector edit, no `run_path` edit
   (`run_path` keeps its concrete `MomentumStrategy` signature — a separate MR
   struct was REJECTED precisely because it would force `run_path` generic/`dyn`
   and risk the anchors; see § D-MR.0). The strategy *config* hash gains a
   `;direction={…}` field (K3 — Momentum vs Reversion at the same θ is a distinct
   strategy), but that is the in-memory config hash, NOT any report body-SHA.
3. **Anchor unit = ONE MR θ-surface report under the EXISTING namespace
   (D6.3 extended).** +1 anchor under `mc-robustness-2026-06` (86 → 87): the
   body-SHA-256 of the single MR θ-surface report, scenario
   `v1-mr-theta-surface-2023-block-bootstrap-real-fy`. Same report *shape* and
   same lane as the momentum θ-surface, so a new namespace would fragment the lane
   for no determinism gain. The MR grid definition (the explicit 6-cell list,
   § D-MR.2-LOCKED), the per-cell N, the `direction` field, and the buy-and-hold
   control flag are hashed body fields (K3 — a different grid/N/direction is a
   different surface and MUST move the SHA). `scripts/verify_anchors.sh`'s
   `mc-robustness-2026-06` handler is extended to also search the MR feature's
   `reports/` dir (the same additive change C3 made for its reports dir).
4. **D2/D3/D5 inherited verbatim** (per-cell index-order reduction; FM/body split;
   `{:.6}`/`{:.2}%`; rows sorted by g before render; Apple-Silicon canonical-box
   scope). The MR θ-surface renderer adds one **additive** column (per-cell trade
   count, for turnover legibility — the MR family's whole design thesis); at
   v0.1.0 this column is gated to MR reports so the momentum #86 body-SHA stays
   byte-identical (no re-lock).

This amendment confirms the C3 machinery generalizes to a **family axis** exactly
as it generalized to a **parameter axis** (D6) — both are config-level variations
over a fixed seed stream. A third axis (e.g. a *third* family, or a venue/universe
axis) would inherit D6.1 the same way provided it too is varied at the config
level and not the seed level; any axis that needs to vary the seed stream requires
its own ADR with a collision-free mix (the D6.2 standing warning).

### D6.6 — A SECOND co-resampled series under the shared index: funding (cross-sectional carry amendment, 2026-05-31)

> **Amendment, and — unlike D6.5 — a REAL-MECHANISM amendment, not a pure
> cross-reference.** The second robustness pivot
> ([`carry-strategy`](../../carry-strategy/feature.md), § D-CARRY.7) is the
> pre-registered rotation after BOTH price families (momentum #86, MR #87) came
> back FAMILY-UNIFORM-FRAGILE on the turnover-killer. Carry differs from MR in a
> way that matters to this ADR: MR consumed the **same price input** as momentum
> (a 1-line score negation — pure config-level variation, D6.5). Carry consumes a
> **different input entirely — the funding rate** — which the bootstrap does not
> currently carry. So carry introduces, for the first time, a **SECOND time series
> that must be co-resampled through the block bootstrap under the SAME shared
> index** as the returns. That is a genuinely new (if small) mechanism, hence a
> substantive § D6.6, not a one-line "carry inherits D6.1." The seed/FM/body/anchor
> contract is still reused verbatim; only the resampling now governs two series
> instead of one.

**D6.6.1 — The shared index governs a SECOND series (the new mechanism).** The C1
shared-index bootstrap (D-C1.3) draws ONE resampling-index sequence `idx_seq` per
path and applies it to all symbols' returns (preserving cross-symbol co-movement,
FP-C1.5). For carry, the **funding series is resampled by the SAME `idx_seq`**: a
per-symbol per-return-step funding series `funding_at_return[s][k]` (the as-of
forward-fill of the real 8h funding onto the real return grid, length `T−1`) is
gathered at `funding_at_return[s][idx_seq[k]]` for output bar `k` — the same index
that selected the return `r_sym[idx_seq[k]]`. Consequences:

1. **Funding↔price co-movement is preserved under resampling — the whole point.**
   Because the funding for output bar `k` is the funding that was *contemporaneous
   with* the return that built bar `k`'s price, the resampled (price, funding) pair
   moves exactly as the real contemporaneous pair did. This is FP-C1.5 (cross-
   *symbol* co-movement) extended to a cross-*series* (price↔funding) co-movement
   under the identical index. A naive timestamp forward-fill onto the **synthetic**
   bar timestamps (`epoch_2023()+k·h`) would be WRONG: those timestamps are NOT
   real calendar time (the bars carry resampled returns), so it would attach
   funding from an unrelated real time and silently decouple price from funding.
   The shared-index gather is the correct and only co-movement-preserving design.
2. **ZERO new randomness → SAME-paths determinism (D1/D6.1) holds TRIVIALLY.** The
   funding gather consumes NO `ChaCha20Rng` calls — `idx_seq` is already fully
   materialized as a `Vec<usize>` before the price-reconstruction loop. The RNG
   stream is byte-identical whether funding is gathered or not. `path_seed_{g,j} =
   ensemble_seed.wrapping_add(j*0x9E37_79B9)` is unchanged; the funding axis is the
   *empty change* on the seed axis, the same strongest-form argument D6.1/D6.5 make.
   This is the de-risk finding: the new mechanism adds **no new determinism surface
   to audit**.

**D6.6.2 — Additive / defaults-absent → the 87 anchors hold by construction.**
The funding path is gated on an **optional** source being present:

- `GeneratedPath` gains a NEW `funding_by_symbol: Option<Vec<Vec<Decimal>>>` field
  (defaults `None`). When the generator has no funding source, it emits `None` and
  takes the existing reconstruction path verbatim — the **bars are computed
  byte-identically**.
- `TcnScenarioInput` gains a NEW `funding_override: Option<…>` field mirroring
  `bars_override` (the proven injection seam). Absent for every momentum / MR /
  buy-and-hold run.
- `run_path` (`montecarlo.rs:76`) stays **CONCRETE** (`MomentumStrategy`). The
  funding-cashflow accrual (at the existing `montecarlo.rs:281` equity push) and
  the carry score path are gated on `funding_override` being `Some` → the accrual
  block is never entered and the score path is the existing
  `score_vol_adjusted_return` for non-carry runs.
- The carry signal is a NEW `ScoreSource { VolAdjustedReturn (default),
  FundingCarry }` enum on `CrossSectionalMomentumConfig` (serde-default sibling to
  `Direction`) → every existing TOML / struct literal deserializes to the existing
  behavior. The strategy *config hash* gains a `;score_source={…}` field (K3 — a
  distinct strategy), but that is the in-memory hash, NOT any report body-SHA.

Therefore the momentum #86 (`0dd989d9…`) and MR #87 (`a708112e…`) θ-surface
body-SHAs are **byte-unchanged with no re-lock** — the same additive discipline
D6.5 used for `direction`.

**D6.6.3 — REJECTED seams that would touch the seed/anchor surface.** Two
alternatives that the brief floated are rejected for anchor-risk:

- **Extend `Bar` with `funding_rate: Option<Decimal>` (brief option (i)).**
  REJECTED. `Bar` is `Serialize`/`Deserialize` and is constructed in the bootstrap
  output path (`bootstrap.rs:247,281`) + every loader + every test; a new field
  changes the bootstrap output *shape* and risks the byte-identity of the anchors'
  upstream bar construction, for a field most of the engine ignores. The parallel
  `funding_by_symbol` (Option, alongside `bars_by_symbol`) achieves the same with
  zero `Bar` change.
- **A separate `CarryStrategy` struct implementing `Strategy` (brief option
  (iii)).** REJECTED — the **exact D6.5.2 trap**. `run_path` is typed to concrete
  `MomentumStrategy` (`montecarlo.rs:79`); a sibling struct forces `run_path`
  generic/`dyn`, re-touching the two `run_path` call-sites
  (`param_robustness_sweep.rs:1294`, `monte_carlo.rs:876`) and risking all 87
  anchors. Carry is a `ScoreSource`-on-config variation of `MomentumStrategy`, the
  direct analogue of MR's `Direction`-on-config.

**D6.6.4 — Anchor unit = ONE carry θ-surface report under the EXISTING namespace
(D6.3 extended).** +1 anchor under `mc-robustness-2026-06` (87 → 88): the
body-SHA-256 of the single carry θ-surface report, scenario
`v1-carry-theta-surface-2023-block-bootstrap-real-fy`. Same report *shape*-family
and same lane as the momentum/MR θ-surfaces (it adds ONE additive column — the
per-cell realized-funding-harvested total, gated to carry reports so the #86/#87
body-SHAs stay byte-identical, exactly as MR added its trade-count column at
D6.5.4), so a new namespace would fragment the lane for no determinism gain. The
carry grid definition (the LOCKED 6-cell list, § D-CARRY.2-LOCKED), the per-cell N,
the `score_source` field, and the funding-revision SHA (`bf1ede44…`) are **hashed
body fields** (K3 — a different grid / N / score-source / funding revision is a
different surface and MUST move the SHA). `scripts/verify_anchors.sh`'s
`mc-robustness-2026-06` handler is extended to also search the carry feature's
`reports/` dir (the same additive change C3 and MR both made). **Per the
frame-diagnostic E1 multi-regime finding, a carry-C3 surface is also produced on
2024-FY** as a day-1 gating read; whether that 2024 surface is locked as a
separate anchor (#89) or kept gating-but-non-anchored is the tester's call at lock
time (locking it is the durable choice). The funding-data revision SHA is a SECOND
revision pin in the report body (alongside the OHLCV `3a8b96c4…`), verified at load
time by the carry funding loader exactly as the OHLCV loader verifies its manifest.

**D6.6.5 — D2/D3/D5 inherited verbatim.** Per-cell index-order reduction; FM/body
split (run-varying fields in front-matter, every distribution input + both
revision SHAs in the hashed body); `{:.6}`/`{:.2}%` fixed precision; rows sorted by
θ-cell index before render; Apple-Silicon canonical-box scope. The funding-cashflow
accrual is `Decimal` throughout (ADR-0003 — no `f64` in money math); only the
stats layer crosses the f64 boundary (D2, unchanged). The realized-funding column
is a `Decimal` sum rendered at fixed precision.

This amendment confirms the shared-index bootstrap generalizes from "one resampled
series (returns)" to "**N co-resampled series under one index**" (here: returns +
funding). Any future second observable that must move with price under the
bootstrap (e.g. a basis series, an open-interest series) co-resamples the same way
— gather by `idx_seq`, gate behind an `Option`, zero new RNG, anchors hold by
construction. An observable that needed its OWN index draw would be a different
mechanism requiring its own ADR (the D6.2 standing warning, generalized).

### D6.7 — A SECOND SELECTOR varied at the config level: time-series long/flat (time-series-momentum amendment, 2026-06-02)

The `time-series-momentum-robustness` feature adds the FIRST **non-cross-sectional**
strategy — per-asset absolute momentum, long/flat on each asset's OWN trailing-return
sign (NO cross-sectional ranking). It is the 3rd instance of the "vary the strategy at
the **config** level, leave the seed untouched ⇒ determinism unchanged by construction"
pattern (MR = § D6.5 `Direction`; carry = § D6.6 `ScoreSource`), and is **strictly
simpler than carry's § D6.6** — it adds NO new data source, NO co-resampled series, and
NO new RNG draw.

**D6.7.1 — the new axis is SELECTION, not score and not seed.** The three
cross-sectional families share the rank→top-K SELECTION shape (`top_k_long`);
TS-momentum deliberately has NO ranking. So the new mechanism is a **2nd selector**,
`select_above_threshold` (long every warmed asset whose OWN score exceeds the entry
threshold, flat the rest — variable cardinality 0..N), gated by a new **`SelectionMode {
CrossSectionalTopK (default), TimeSeriesLongFlat }`** serde-default enum on
`CrossSectionalMomentumConfig`. `build_rebalance_signals` forks on the mode:
`CrossSectionalTopK` → `top_k_long` VERBATIM; `TimeSeriesLongFlat` →
`select_above_threshold`. The score under `TimeSeriesLongFlat` is a **raw cumulative
log-return over L** (`score_trailing_log_return`, NO vol normalization — the band is a
no-trade threshold on the trend itself), computed in the same `on_bar` score region as
the existing branches, which are byte-untouched. A new `#[serde(default)]
entry_threshold: Decimal` (default `ZERO`) is the swept no-trade band.

**D6.7.2 — the variable-cardinality long/flat is PURE on_bar signal emission; the
ENGINE is byte-untouched.** `run_path` (`montecarlo.rs:163-292`) already processes 0..N
Buy/Sell signals and sizes each Buy at a fixed fraction under the exposure cap, going to
cash when 0 Buys are emitted. So the variable cardinality is an emergent property of how
many Buy signals `on_bar` emits — NO `run_path` / `PaperEngine` change. The sizing is
LOCKED to `run_path`'s EXISTING fixed-fraction-per-name (NOT a 1/N rescale — that would
be an engine edit → anchor risk AND would break apples-to-apples with the 3 families
whose verdicts are banked); the `select_above_threshold` weight is a membership sentinel.

**D6.7.3 — additive/defaults-off ⇒ the 89 anchors byte-identical by construction.**
`SelectionMode` defaults `CrossSectionalTopK`; `entry_threshold` defaults `ZERO` and is
read ONLY under `TimeSeriesLongFlat`; the score fork only ADDS a branch;
`select_above_threshold` is a NEW function adding zero bytes to the `top_k_long` path;
`run_path` / `PaperEngine` / `BlockBootstrapPathGen` are UNCHANGED. → momentum #86
(`0dd989d9…`), MR #87 (`a708112e…`), carry #88 (`f03cd714…`), carry #89 (`fd96d5a8…`),
and all pre-existing anchors hold byte-identical, no re-lock. A separate
`TimeSeriesMomentumStrategy` struct is REJECTED (the exact D6.5.2 trap — forces
`run_path` generic/`dyn`).

**D6.7.4 — +1/+2 TS θ-surface anchors under the existing `mc-robustness-2026-06`.**
+1 (2023 → 90, scenario `v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy`) or
+2 (both regimes → 91, the durable choice). The LOCKED 6-cell grid (lookback {24, 168,
720 bars} × entry_threshold {0.00, 0.02}, § D-TSM.3-LOCKED), per-cell N, `selection_mode`,
and `entry_threshold` are hashed body fields (K3). One additive time-in-market /
fraction-flat column is gated to TS reports (momentum/MR/carry body-SHAs byte-identical,
as carry's funding column at D6.6.4 and MR's trade-count column at D6.5.4).
`verify_anchors.sh`'s `mc-robustness-2026-06` handler also searches the TS feature's
`reports/` dir (the same additive change C3, MR, and carry each made).

**D6.7.5 — D1/D2/D3/D5/D6.1 inherited verbatim.** SAME path-set across cells (the method
varies at config level, seed untouched); no new RNG (the de-risk — TS reads only the
closes already in the bootstrapped bars); `select_above_threshold` is a deterministic
pure function over the `BTreeMap` score map (alphabetical iteration, no unordered fold)
so two-run byte-identity holds by construction; FM/body split + fixed precision + Decimal
money unchanged. Amendment, NOT a new ADR — the seed idiom, FM/body split, and
one-report anchor unit are all reused; only a 2nd selector is added at the config level.

## Consequences

### Positive

1. **A stochastic method coexists with the byte-identical anchor gate.** Same
   master seed → same N sub-seeds (D1) → same N paths (C1) → same N per-path
   metrics → same reduction (D2) → same formatted body (D3) → same body-SHA. The
   gate is unchanged; one additive anchor under a new namespace (D4).
2. **The anchor is sensitive to inputs, invariant to re-runs (the noop-fix
   meta-lesson).** Every distribution input — `param_set`, `generator`,
   `master_seed`, `fill_seed`, `n_paths`, `bootstrap_mode`, selected `L`,
   `source_revision_sha` — is in the hashed body (D3), so a different strategy /
   θ* / generator / N moves the SHA (K3), while a re-run at the same seeds does
   not (R3.1). This is the direction-note § 8 "subtle risk" closed by
   construction.
3. **One seed idiom across both axes.** D1 reuses `0x9E37_79B9` verbatim, so
   per-symbol and per-path seed mixing share one constant and one shape — no new
   idiom to audit.
4. **The fill-tie-break confound is removed.** D1 holds the fill seed constant
   across paths so the only varying input is the path itself — the robustness
   distribution measures path-variance, not path-variance entangled with
   tie-break-variance.

### Negative

1. **f64 reduction order is now load-bearing and must be policed.** A future
   refactor that swaps the sequential sum for `par_iter().sum()` (tempting for
   speed) would silently break the anchor on the canonical box. Mitigation: the
   D2 string `reduction_rule` is printed in the body, and the two-run
   byte-identity e2e (R3.1) catches any drift; the reducer carries a
   `// ADR-0051 D2: index-order reduction is load-bearing — do NOT parallelize`
   comment.
2. **The smoke-test generator can produce an optimistic DD tail if used by
   mistake.** A run accidentally using `GbmPathGen` (no fat tails) understates
   p95 MaxDD (C2 K4). Mitigation: `generator` is in the hashed body and the
   report header labels it; the headline robustness run MUST use
   `block-bootstrap-real` before the `paper→live` gate trusts the tail.
3. **Cross-platform parity is forfeited (accepted).** A robustness summary
   generated on x86 CI may differ in the last formatted bit from the canonical
   box. Accepted per D5 (inherits ADR-0043); `verify_anchors.sh` runs on the
   canonical box only.

## Alternatives rejected

- **Per-path anchors (audit § 3.3 Option B).** Rejected — see D4.
- **Unordered parallel reduction for the mean/std/percentiles.** Rejected — f64
  non-associativity flaps the last bit and breaks the anchor even same-machine
  (audit § 3.1 caveat 2). The fixed index-order reduction (D2) is non-negotiable.
- **`partial_cmp().unwrap()` for the percentile sort.** Rejected — undefined on
  `NaN` (panics or mis-sorts). D2 mandates `total_cmp` + an explicit NaN-absent
  assertion.
- **Putting run-varying fields (generated, host, wall_clock) in the hashed
  body.** Rejected — they would flap the SHA every run. D3 puts them in
  front-matter, stripped before hashing (the `report_body_hash` contract,
  `crates/backtest/src/lib.rs:64`).
- **A new `ensemble_seed` that also re-seeds the fill tie-break per path.**
  Rejected — it injects a second varying noise source that confounds the
  path-variance the robustness run is trying to measure. D1 holds the fill seed
  constant across paths.
- **Anchoring N per-path reports AND the summary (hybrid).** Deferred, not
  rejected — audit § 3.3 Option C (embed a sampled-path digest in the hashed
  body) is an acceptable future upgrade of D4 if extra tamper-evidence is wanted;
  not adopted at v0.1.0.
- **(D6 amendment) Independent N paths per θ-cell via naive additive two-axis
  seed.** Rejected — see D6.2. The `ensemble_seed + g*0x9E37_79B9 + j*0x9E37_79B9`
  composition collapses to `+ (g+j)*0x9E37_79B9` and collides distinct (cell, path)
  pairs; and independent paths reintroduce path-sampling variance as an inter-row
  confound. SAME-paths-across-cells (D6.1) is chosen.
- **(D6 amendment) N-per-θ anchor set for the sweep.** Rejected — see D6.3 (a
  G×-explosion that re-locks on any grid change; the surface is the deliverable).
  ONE θ-surface report = +1 anchor.
- **(D6.5 amendment) A separate `CrossSectionalMeanReversionStrategy` struct for
  the MR family.** Rejected — `montecarlo::run_path` takes a **concrete**
  `strategy::MomentumStrategy` (line 79), so a separate struct would force
  `run_path` to become generic (`run_path<S: Strategy>`) or take
  `Box<dyn Strategy>`, **touching the C2-anchored `run_path` and risking all 86
  anchors** for zero functional gain. MR as a direction-flipped `MomentumStrategy`
  (a `Direction` field on the shared config) keeps `run_path` byte-identical so
  the anchors hold by construction, AND keeps the two families on one tested
  ranking path so they cannot silently diverge in plumbing (the failure the
  R-MR.1 divergence falsifier guards against). See § D-MR.0.
- **(D6.5 amendment) A new anchor namespace for the MR θ-surface.** Rejected —
  the MR θ-surface is the same report shape and the same lane as the momentum
  θ-surface; a new namespace fragments the lane for no determinism gain. +1 anchor
  under the existing `mc-robustness-2026-06` (D6.5.3).

## Cross-references

- Feature briefs — [`spec/monte-carlo-bootstrap-path-generator/feature.md`](../../monte-carlo-bootstrap-path-generator/feature.md)
  (C1), [`spec/strategy-robustness-harness/feature.md`](../../strategy-robustness-harness/feature.md) (C2).
- Architecture-readiness audit —
  [`spec/dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md`](../../dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md)
  § 3 (the hard tension), § 5 (the ADR mandate this ADR fulfils).
- Analyst direction — [`spec/dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md`](../../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md)
  § 8 (the stochastic-vs-anchor collision + the three open questions D1 resolves).
- Determinism scope precedent — ADR-0043 § "f64 conversion boundary" (lines
  301-310): Apple-Silicon canonical-box bit-stability, cross-platform NOT
  contracted. Inherited verbatim by D5.
- Seed idiom source — `crates/backtest/src/scenarios/momentum.rs:245`
  (`sym_seed = seed.wrapping_add(idx * 0x9E37_79B9)`), mirrored
  `threshold_sweep.rs:120`. Reused on the path axis by D1.
- Report body-hash contract — `crates/backtest/src/lib.rs:64`
  (`report_body_hash` / `extract_report_body` — body = everything after the
  second `---`).
- Seam reused by C2 — `crates/backtest/src/bin/threshold_sweep.rs` +
  `crates/backtest/src/scenarios/threshold_sweep.rs` (`run_cell`,
  sort-before-render, FM/body split, fixed-precision floats, `compute_*`
  calculators).
- CLAUDE.md non-negotiable — every overlay/sizing-modifier (and, adapted, every
  distribution harness) ships a baseline-divergence e2e from day 1; pattern
  reference `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`. The
  C2 R-NR.6 two-part gate (divergence-from-single-path + two-run determinism) is
  the adaptation.

## Changelog

- 2026-05-30 (architect, M-T1): ADR-0051 authored from C2's § ADR flag (D1-D5
  restated as the binding contract) + the architect readiness audit § 3/§ 5.
  Lands at the next free number (0050 was max). D1 sub-seed rule reuses the
  `0x9E37_79B9` idiom on the path axis + makes the ensemble seed orthogonal to
  the fill-tie-break seed (holds fill seed constant across paths — resolves
  direction-note § 8 open-Q2). D2 freezes index-order mean/std + `total_cmp`
  sort + type-7 linear percentile (the f64 boundary). D3 freezes the
  front-matter/body split + fixed-precision formatting + fixed metric order. D4
  anchor unit = one summary report under `mc-robustness-2026-06` (rejects
  N per-path). D5 determinism scope = Apple-Silicon canonical box (inherits
  ADR-0043 verbatim; cross-platform NOT contracted). Retired-line guard restated
  (GARCH/regime as generators-not-alpha; v0.1.0 ships only block bootstrap).
  Status `accepted`. Registered atomically in `README.md` (architect.md § ADR
  registry contract).
- 2026-05-30 (architect, C3 M-T1): **D6 amendment added** — θ-sweep extension for
  C3 ([`momentum-parameter-robustness-sweep`](../../momentum-parameter-robustness-sweep/feature.md)).
  Resolves the C3 § ADR flag (amendment, NOT a new ADR-0052) and OQ-1/OQ-3/OQ-4.
  **D6.1** ratifies SAME path-set across all θ-cells: `cell_seed_g := ensemble_seed`
  for all g, so `path_seed_{g,j} = ensemble_seed.wrapping_add(j*0x9E37_79B9)` is
  **byte-identical to the C2 D1 seed** — the θ-axis is varied at the config level,
  not the seed level, so C1/C2 determinism (85 anchors) is unchanged *by
  construction* (the extension is the empty change on the seed axis). Verified
  arithmetically: with cell_seed = ensemble_seed, the C3 per-path seed equals
  `derive_path_seed(ensemble_seed, j)` for every j; the g=0 row reduces exactly to
  C2. **D6.2** REJECTS the brief's D-C3.4 fallback (naive additive two-axis
  `+(g+j)*0x9E37_79B9`) as a seed-collision bug (6 colliding pairs in a 4×3 grid)
  AND as a methodological confound; a future per-cell-independence variant needs its
  OWN ADR with a collision-free non-commuting mix. **D6.3** extends D4: ONE
  θ-surface report = +1 anchor under `mc-robustness-2026-06` (85→86); the explicit
  cell list + per-cell N + buy-and-hold flag are hashed body fields (K3). **D6.4**
  inherits D2/D3/D5 verbatim (per-cell reduction unchanged; FM/body split;
  `{:.6}`/`{:.2}%`; rows sorted by θ-cell index before render; canonical-box scope).
  D6.1.4 confirms L is θ-independent (OQ-3 — printed once in the surface header).
  Registry README row + frontmatter `updated:` amended atomically (architect.md
  § ADR registry contract).
- 2026-05-31 (architect, cross-sectional-mean-reversion M-T1): **D6.5
  cross-reference amendment added** — the first robustness pivot
  ([`cross-sectional-mean-reversion-strategy`](../../cross-sectional-mean-reversion-strategy/feature.md))
  sweeps a **second strategy family** (cross-sectional MR = the v1 vol-adjusted
  score negated) through the **same C3 θ-surface machinery**. The family axis,
  like the θ-axis (D6.1), is varied at the **strategy/config level** (a
  `Direction { Momentum, Reversion }` field on `CrossSectionalMomentumConfig`),
  NOT the seed level — so it is the second instance of "vary at config level,
  seed untouched ⇒ determinism unchanged by construction" and inherits
  D6.1/D6.3/D6.4 with no new seed mechanism (the 86 anchors hold by
  construction). The inversion is ONE line negating the `Decimal` output of the
  anchored `score_vol_adjusted_return`; the anchored `top_k_long` is reused
  verbatim (descending top-K → bottom-K on negated scores). **REJECTED a separate
  MR struct** (would force `run_path` generic/`dyn` → anchor risk; `run_path`
  takes a concrete `MomentumStrategy`) and **a new anchor namespace** (same report
  shape/lane). +1 MR θ-surface anchor under the existing `mc-robustness-2026-06`
  (86→87, scenario `v1-mr-theta-surface-2023-block-bootstrap-real-fy`); MR grid +
  N + `direction` + buy-hold flag are hashed body fields (K3). Amendment, NOT a
  new ADR (analyst-defaults cheap-and-correct exception, identical to the D6
  rationale). Registry README row summary + frontmatter `updated:` amended
  atomically (architect.md § ADR registry contract).
- 2026-05-31 (architect, carry-strategy M-T1): **D6.6 amendment added — a
  REAL-MECHANISM amendment (not a pure cross-ref like D6.5).** The second
  robustness pivot ([`carry-strategy`](../../carry-strategy/feature.md), § D-CARRY.7)
  sweeps cross-sectional funding carry. Unlike MR (same price input, 1-line
  negation), carry consumes a **different input (the funding rate)** the bootstrap
  does not carry, so it introduces a **SECOND time series co-resampled under the
  SAME shared `idx_seq`** as the returns — a genuinely new (small) mechanism.
  **D6.6.1** the shared index governs a 2nd series: `funding_at_return[s][idx_seq[k]]`
  is gathered at the SAME index that picks the return, preserving funding↔price
  co-movement (FP-C1.5 extended cross-series); the gather consumes ZERO new
  `ChaCha20Rng` draws (`idx_seq` is already materialized) → SAME-paths determinism
  (D1/D6.1) holds TRIVIALLY, no new determinism surface to audit (the de-risk
  finding). A naive timestamp forward-fill onto the synthetic bar timestamps would
  WRONGLY decouple price/funding (synthetic ts ≠ real calendar time). **D6.6.2**
  additive/defaults-absent: NEW `GeneratedPath.funding_by_symbol: Option<Vec<Vec<
  Decimal>>>` + `TcnScenarioInput.funding_override: Option<…>` (mirrors
  `bars_override`) + a `ScoreSource { VolAdjustedReturn (default), FundingCarry }`
  serde-default enum on the config → 87 anchors (incl. momentum #86 `0dd989d9…`, MR
  #87 `a708112e…`) byte-identical by construction; `run_path` stays CONCRETE.
  **D6.6.3** REJECTED extend-`Bar` (option (i), changes bootstrap output shape →
  anchor risk) + REJECTED a separate `CarryStrategy` struct (option (iii), the
  exact D6.5.2 trap — forces `run_path` generic/`dyn`). **D6.6.4** +1 carry
  θ-surface anchor under the existing `mc-robustness-2026-06` (87→88, scenario
  `v1-carry-theta-surface-2023-block-bootstrap-real-fy`); grid + N + `score_source`
  + funding-revision SHA `bf1ede44…` are hashed body fields (K3); one additive
  realized-funding column gated to carry reports (#86/#87 byte-identical, as MR's
  trade-count column at D6.5.4); per frame-diagnostic E1 a 2024-FY carry-C3 surface
  is a day-1 gating read (anchor #89 optional, tester's call). **D6.6.5** D2/D3/D5
  inherited verbatim; funding cashflow is `Decimal` (ADR-0003). Generalizes the
  shared-index bootstrap to N co-resampled series under one index. Amendment, NOT a
  new ADR-0052 (the seed idiom / FM-body split / one-report anchor unit all reused;
  only the resampling governs a 2nd series). Registry README row summary +
  frontmatter `updated:` amended atomically (architect.md § ADR registry contract).
- 2026-06-02 (architect, time-series-momentum-robustness M-T1): **D6.7 amendment
  added — a SECOND SELECTOR varied at the config level (time-series long/flat), the
  FIRST non-cross-sectional family.** The 3rd "vary-at-config-not-seed ⇒ determinism
  unchanged by construction" instance (MR=D6.5 `Direction`; carry=D6.6 `ScoreSource`)
  and STRICTLY simpler than carry: NO new data source, NO co-resampled series, NO new
  RNG draw. **D6.7.1** the new axis is SELECTION (not score, not seed): the 3 families
  share rank→top-K (`top_k_long`); TS-momentum has NO ranking, so it adds a 2nd
  selector `select_above_threshold` (long every warmed asset whose OWN raw-trend score
  > entry_threshold, flat the rest; variable cardinality 0..N) gated by a new
  `SelectionMode { CrossSectionalTopK (default), TimeSeriesLongFlat }` serde-default
  enum + a `#[serde(default)] entry_threshold: Decimal` (default ZERO); the
  `TimeSeriesLongFlat` score is a RAW cumulative log-return over L
  (`score_trailing_log_return`, NO vol-norm); existing score branches byte-untouched.
  **D6.7.2** the variable-cardinality long/flat is PURE `on_bar` signal emission —
  `run_path` already sizes 0..N Buys at a fixed fraction under the exposure cap and
  goes to cash on 0 Buys, so the ENGINE is byte-untouched; sizing LOCKED to the
  existing fixed-fraction-per-name (NOT a 1/N rescale — engine edit + breaks
  apples-to-apples). **D6.7.3** additive/defaults-off → the 89 anchors (momentum #86
  `0dd989d9…`, MR #87 `a708112e…`, carry #88 `f03cd714…`, carry #89 `fd96d5a8…`, all
  pre-existing) byte-identical by construction; a separate `TimeSeriesMomentumStrategy`
  struct REJECTED (the D6.5.2 trap). **D6.7.4** +1/+2 TS θ-surface anchors under the
  existing `mc-robustness-2026-06` (90 for 2023 `v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy`,
  91 for 2024 the durable choice); LOCKED 6-cell grid (lookback {24,168,720 bars} ×
  entry_threshold {0.00,0.02}, § D-TSM.3-LOCKED) + N + `selection_mode` +
  `entry_threshold` are hashed body fields (K3); one additive time-in-market column
  gated to TS reports (momentum/MR/carry body-SHAs byte-identical). **D6.7.5**
  D1/D2/D3/D5/D6.1 inherited verbatim; no new RNG (the de-risk); `select_above_threshold`
  is a deterministic `BTreeMap`-ordered pure fn → two-run byte-identity by construction.
  Amendment, NOT a new ADR. Registry README row summary + frontmatter `updated:` amended
  atomically (architect.md § ADR registry contract).
- 2026-06-03 (architect, horizon-retest-robustness M-T1): **D6.8 amendment added — a
  HORIZON/DATA-PATH change varied at the LOAD + the CALCULATOR level (4h + daily coarser
  decision cadence on the SAME 10-symbol coins), NOT a new strategy family.** The 4th
  anchor-additive instance (MR=D6.5, carry=D6.6, TS=D6.7); the new wrinkle vs those is
  the variation touches the DATA PATH (an in-memory OHLCV resample) + the CALCULATOR (a
  new annualization fn) rather than the config — but the discipline is IDENTICAL: the
  pre-existing path is byte-verbatim, the new path is additive, the seed is untouched.
  **D6.8.1 — THE LOAD-BEARING decision (R-HR.LOAD), the sharpest anchor-risk in the
  program:** `compute_sharpe_hourly`/`compute_sortino_hourly`/`compute_calmar`
  (`stats/mod.rs:40/70/100`) hardcode 1h-baked constants (Sharpe/Sortino `SQRT_HPY =
  √8575`; Calmar `years=(n−1)/8760`); run through them, a coarse bar inflates the
  annualized Sharpe ≈2.0× (4h) / ≈4.9× (daily) → a silent false-ROBUST. The fix is
  ADDITIVE: the three 1h fns stay BYTE-VERBATIM (re-imported by `bin/threshold_sweep.rs`
  per R-NR.5; feed all 91 anchors) and three NEW siblings `compute_sharpe_periodic(equity,
  periods_per_year: f64)` / `_sortino_periodic` / `_calmar_periodic` annualize by
  `√periods_per_year`. REJECTED parameterizing the existing fns (ULP-change risk → 91-anchor
  REGRESSION) + REJECTED a `Timeframe`-arg signature (hides the leap-year year-awareness).
  The 1h path's two INCONSISTENT constants (√8575 Sharpe, /8760 Calmar) stay verbatim; the
  periodic fns are MATHEMATICALLY CORRECT (no √8575 quirk propagated — 4h/daily are NEW, no
  anchor binds them). `periods_per_year(horizon, year)` is YEAR-aware (4h 2190/2196; daily
  365/366; F-HR.2 asserts √2190=46.797…, √365=19.105…). The sweep picks the path by `if
  horizon==1h { *_hourly /* VERBATIM */ } else { *_periodic(periods_per_year(horizon,year)) }`
  at BOTH metric call-sites (per-cell 1966 + BH 2383). **D6.8.2 — the resampler:**
  `resample_ohlcv` = a pure ORDERED Decimal fold (open=first/high=max/low=min/close=last/
  vol=Σ, UTC integer-division bucketing 14_400_000/86_400_000, single pass over `open_ts`-
  sorted input, NO `HashMap`) wired as a POST-`merge_symbols` fold in `load_real_bars`
  (`param_robustness_sweep.rs:1683`); `horizon==1h` → IDENTITY pass-through → the 1h load
  path byte-untouched; the coverage check stays on the 1h count. The coarse `bar_count`
  (`:2149`) = `1h_count / {6,24}` (exact integers) threads into the UNCHANGED bootstrap +
  BH. **D6.8.3 — OQ-CARRY-SEM resolves for FREE:** the funding as-of join
  (`funding_data.rs:378/421`) is timestamp-driven; the resample-first ordering means
  `build_funding_at_return` keys off the COARSE bar `open_ts` = "last settlement at the
  coarse bar's open" (apples-to-apples with carry #88/#89, NO `funding_data.rs` edit); L is
  re-picked as a coarse-bar ring count; the funding co-resamples under the SAME shared
  `idx_seq` (D6.6) at length `coarse_bar_count−1`. **D6.8.4 — OQ-BOOTSTRAP-TF:** the
  bootstrap timestamp ladder stays cosmetically 1h (`bootstrap.rs:414`, correctness-safe —
  the strategy keys off `close`+ordering, the Sharpe is per-return not per-second); the
  RENDERER prints the REAL horizon (a hashed body field). CONSEQUENCE (the carry subtlety):
  `is_rebalance_bar` measures wall-clock minutes on the 1h ladder, so "rebalance every coarse
  bar" = `rebalance_minutes ≤ 60` and "every 2nd" = 120 — the native coarse cadence is
  grid-encoded per cell. **D6.8.5** additive/defaults-off (`--horizon` defaults `1h`) → the
  91 anchors (momentum #86 `0dd989d9…`, MR #87 `a708112e…`, carry #88 `f03cd714…`, carry #89
  `fd96d5a8…`, TS #90, TS #91, all pre-existing) byte-identical by construction;
  `run_path`/`PaperEngine`/`BlockBootstrapPathGen` byte-UNTOUCHED (the coarse `bar_count` is
  a parameter, not a code change). **D6.8.6 — a NEW namespace `horizon-retest-robustness`**
  (NOT `mc-robustness-2026-06` — the horizon is a distinct experiment axis); reports under
  `spec/horizon-retest-robustness/reports/`; the tester adds the dir to `verify_anchors.sh`
  + registers the namespace at lock time; + up to 8 anchors (TS + carry × 4h + daily ×
  2023/2024); the LOCKED grids (TS-4h {42,180,540}×{0.00,0.02}; TS-daily {5,20,60}×{0.00,0.02};
  CARRY-4h L∈{2,6,12}; CARRY-daily L∈{1,3,7}, § D-HR.4-LOCKED) + N (200 @ 4h, 1000 @ daily)
  + horizon + funding-revision SHA are hashed body fields (K3). **D6.8.7** D1/D2/D3/D5/D6.1
  inherited verbatim; no new RNG (the de-risk); the resampler is a deterministic ordered
  fold → two-run byte-identity by construction (F-HR.5). Amendment, NOT a new ADR. Registry
  README row summary + frontmatter `updated:` amended atomically (architect.md § ADR
  registry contract).
