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
