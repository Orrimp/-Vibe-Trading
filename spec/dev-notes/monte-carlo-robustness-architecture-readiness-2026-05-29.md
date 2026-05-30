---
slug: monte-carlo-robustness-architecture-readiness
status: draft
owner: architect
updated: 2026-05-30
kind: dev-note (read-only architecture-readiness audit)
companion: spec/dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md (analyst — WHAT/WHY; not yet on disk at audit time)
---

# Monte-Carlo Robustness — Architecture-Readiness Audit

**Question asked.** Can the *current* engine support (a) Monte-Carlo synthetic-path
strategy evaluation and (b) a robustness/sensitivity harness, and how does
stochastic multi-path evaluation coexist with the project's byte-identical anchor
system?

**One-line verdict.** Yes, with **bounded, mostly-additive** work. The hardest
part is **not** the engine (a near-complete prototype of the exact pattern already
exists — `crates/backtest/src/bin/threshold_sweep.rs`); it is the **anchor-unit
decision** (§3) and the **cross-platform f64-determinism boundary** (§3) that the
distribution-summary report inherits. Seeded determinism *does* save the anchor
system, but only under the same "Apple-Silicon-canonical-box" constraint the
project has already accepted for the sqrt-impact path (ADR-0043).

This audit is READ-ONLY. It proposes no feature folder, ADR, trace row, or
backlog edit. Sizing in §2/§6 is intended to dovetail with the analyst's §6
candidate list.

> **Retired-line guard.** GARCH and Markov/regime-switching were retired as
> *alpha sources* (`spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md`).
> This note treats them only as **synthetic-data / path generators** — a distinct
> use that does not relitigate the alpha retirement. The distinction matters:
> a generator's job is to produce plausible *price paths to stress a strategy on*,
> not to *predict returns*. §2 flags where this distinction must be stated in any
> follow-up ADR so a reviewer does not reject "GARCH" on sight.

---

## §1. Current synthetic-data + backtest path (map it)

### 1.1 How synthetic data is produced today — a single deterministic path

There is **one** synthetic generator pattern, duplicated in two places, and it is
a **single deterministic GBM path per `(symbol, seed)`**:

| Producer | File | Shape |
|---|---|---|
| `synthetic_bars_hourly` | `crates/backtest/src/scenarios/momentum.rs:98` | GBM via Box-Muller; `ChaCha20Rng::seed_from_u64(seed)`; one path. |
| `synthetic_bars_det` | `crates/backtest/tests/determinism.rs:37` | Same algorithm (test-local copy), minute bars. |

Mechanics (momentum.rs:125-173):
- `z = sqrt(-2·ln(u1))·cos(2π·u2)` (Box-Muller standard normal),
- `ret = per_hour_drift + per_hour_vol·z` with **hardcoded** `per_hour_vol = 0.012`,
  `per_hour_drift = 0.000_03`,
- `next = close·(1+ret)` clamped, plus intrabar high/low noise and a random volume.
- **All randomness flows from one `ChaCha20Rng` seeded by a single `u64`.**

Multi-symbol runs derive a per-symbol seed deterministically:
`sym_seed = seed.wrapping_add(idx · 0x9E37_79B9)` (momentum.rs:245, mirrored in
threshold_sweep.rs:120). Symbol streams are time-merged by
`data::ReplayFeed::merge_synthetic` (`crates/data/src/replay_feed.rs:273`) — a pure
sort by `(open_ts, symbol)`. No randomness in the merge.

**Crucial gap for Monte-Carlo.** There is **no path-ensemble primitive** and **no
block-bootstrap / resampler** anywhere in `crates/`
(grep for `simulate_path|sample_path|bootstrap` returns only Baum-Welch comments and
UI test helpers). `GarchModel::forecast_step` (`crates/forecast/src/garch.rs:104`)
is a **single-step vol recurrence**, not a path sampler. A MC generator would be net
new code (but small — see §2).

`FakeFeed` (`crates/data/src/fake_feed.rs`) is an in-memory replay of a *fixed*
`Vec<Bar>` — it is a transport, not a generator. Not relevant to path synthesis.

### 1.2 The scenario system — one fixed path per scenario

`crates/backtest/src/scenarios/` (dispatched by `engine::run_scenario`, registered
in `scenarios/mod.rs`) contains one `run()` per strategy family
(momentum / pairs / tcn_overlay / sma_composed / regime_dispatcher / …). Each `run`:

1. Builds the universe + start prices (`top10_symbols_with_prices()` etc.),
2. Calls `synthetic_bars_hourly(sym, count, sym_seed, …)` **once per symbol** (or
   loads real Binance/Yahoo parquet bars when `--features realdata` and
   `bars_override` is set),
3. Steps a `PaperEngine` (`crates/backtest/src/paper.rs`) bar-by-bar,
4. Emits **one** `RunReport` → **one** markdown report → **one** body-SHA anchor.

A scenario's data is **one fixed series**. The seed is the global CLI `--seed`
(default `0xC0FFEE`); the matching engine is also seeded with the same value
(`PaperEngine::new(config, seed)`).

### 1.3 The Strategy trait + parameterization seam

`strategy::Strategy` (`crates/strategy/src/traits.rs`) is intentionally narrow:
```
fn id(&self) -> StrategyId;
fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;
fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
fn config_schema() -> serde_json::Value;       // Self: Sized
fn quantity_scale(&self, _symbol: &Symbol) -> f64 { 1.0 }
```
**Params live in typed configs loaded from TOML**, not in the trait. Example —
`CrossSectionalMomentumConfig` (`crates/strategy/src/cross_sectional/config.rs:63`)
exposes `universe`, `lookback_minutes: u32`, `rebalance_minutes: u32`,
`exposure_cap: Decimal`, `drift_rebalance_threshold: Decimal`, `vol_floor: Decimal`.
These are read from `config/strategies/*.toml` per scenario and are **hardcoded per
scenario config** — there is no sweep loop in the production scenario path.

**BUT** a parameter-sweep seam already exists, see §1.4.

### 1.4 The hidden asset — `threshold_sweep` is a working sweep+aggregate prototype

`crates/backtest/src/bin/threshold_sweep.rs` already implements **exactly the
robustness-harness pattern**, for the τ×ε TCN tuning feature:

- Enumerates a **9×5 = 45-cell** parameter grid (`TAU_GRID`, `EPSILON_GRID`),
- Runs each cell **in parallel** via a dedicated `rayon` thread pool
  (`sweep_pool.install(|| cell_indices.into_par_iter().map(...))`),
- Each cell calls `scenarios::threshold_sweep::run_cell(input, SEED, strategy)`
  (`crates/backtest/src/scenarios/threshold_sweep.rs`) — a thin wrapper that runs
  one backtest with a caller-supplied, pre-parameterized strategy,
- **Sorts cells lexicographically by `(τ, ε)` BEFORE rendering**
  (threshold_sweep.rs:1065) → order-invariant body → byte-identical across runs,
- Splits run-varying fields into **YAML front-matter** (`generated`, `wall_clock_s`,
  `host`, `git_commit`, data-revision-SHA) and the **deterministic body** (the
  heatmaps) — the exact front-matter-vs-body discipline the anchor gate requires,
- Emits **one summary report** (4 heatmaps + headline cell) — i.e. **N runs → 1
  report → would be 1 anchor**.

This is a parameter-sweep over a *deterministic* path. Monte-Carlo is the dual:
**N paths over a fixed parameter set**. The harness shape is identical; only the
inner loop's varying axis changes (path-seed instead of (τ,ε)). **This bin is the
single most important finding in this audit** — the seam is ~80% built.

### 1.5 Report rendering & the anchor unit

`backtest::report_body_hash` / `extract_report_body` (`crates/backtest/src/lib.rs:64`)
define the contract: the body is everything **after** the second `---` line; the
front-matter (first→second `---`) is stripped before hashing. `spec/anchors.toml`
currently holds **84** locked body-SHAs across namespaces
(`noop-baseline`, `v5-realdata-medium-2026-05`, `v5-sqrt-impact-2026-05`,
`lab-yahoo-realdata-*`, …). `scripts/verify_anchors.sh` resolves each anchor to the
newest matching report (namespace-aware) and recomputes the body-SHA. **One report
= one path = one body-SHA today.**

### 1.6 Data-flow map (today)

```
                          --seed 0xC0FFEE (single u64)
                                   │
        ┌──────────────────────────┴───────────────────────────┐
        ▼                                                        ▼
 scenario config                                        PaperEngine::new(cfg, seed)
 (universe, TOML params)                                        │
        │                                                        │
        ▼   sym_seed = seed + idx·0x9E3779B9                     │
 synthetic_bars_hourly(sym, n, sym_seed)  ◄── ChaCha20Rng       │
        │   (ONE GBM path per symbol)                            │
        ▼                                                        │
 ReplayFeed::merge_synthetic  ──►  Vec<Bar> (one fixed series)   │
        │                                                        │
        └──────────────► for bar in bars: strategy.on_bar ──────►│  step(bar, orders)
                                   │                              │
                                   ▼                              ▼
                          RunReport (equity curve, KPIs)  ◄── fills
                                   │
                                   ▼
                  render report  →  ── front-matter (generated/host/…) ──
                                     ── body (KPIs, deterministic) ──────► report_body_hash
                                   │                                          │
                                   ▼                                          ▼
                          one .md report                         spec/anchors.toml (1 SHA)
```

The dotted-future overlay (§2) inserts a **path index `j ∈ 0..N`** between `--seed`
and `synthetic_bars_hourly`, and an **aggregator** between `RunReport` and `render`.

---

## §2. The Monte-Carlo seam — what would it take?

For each capability: where the seam is, what changes, and a size. Sizes assume the
`threshold_sweep` pattern is reused (parallel cells, sort-before-render, FM/body
split) rather than reinvented.

### (a) Inject N synthetic paths per scenario — **SMALL→MEDIUM**

The seam is the seed→generator edge. `synthetic_bars_hourly` already takes a `seed`
and is pure. A path ensemble is:
```
for j in 0..N:  path_seed_j = master_seed ⊕ mix(j)   // e.g. ChaCha-derived sub-seed
                bars_j = generate(universe, n, path_seed_j)
                report_j = run_backtest(bars_j, fixed_params)
```
- **SMALL** if the only generator is the existing GBM: add a `path_seed` derivation
  helper + an N-loop. No engine change; `run_cell`-style wrapper already accepts a
  caller-supplied strategy and returns a result struct with the equity curve.
- **MEDIUM** if richer generators are wanted (GARCH-driven vol path; block-bootstrap
  of real returns — the more *defensible* MC for "robustness"). These are net-new
  code: a `MonteCarloPathGen` trait + 2-3 impls (GBM / GARCH-vol / stationary-block-
  bootstrap-of-real-bars). GARCH recurrence primitive (`forecast_step`) exists and is
  reusable; bootstrap is ~50 LOC.

**Files that change:** new module (see "where it lives" below); **no change** to
`Strategy`, `MatchingEngine`, `PaperEngine`, or any existing scenario `run()`.
Anchor-additive by construction.

### (b) Parameter sweep across a strategy's param space — **SMALL (already exists)**

`threshold_sweep` *is* a parameter sweep. Generalizing it from "(τ,ε) on a TCN
overlay" to "arbitrary param grid on any `Strategy`" is a refactor, not new
architecture:
- A `ParamGrid` description (Cartesian product of named axes → concrete config),
- A `build_strategy(config) -> Box<dyn Strategy>` closure per family (the registry
  in `crates/strategy/src/registry.rs` already maps TOML → strategy).
- The momentum config's `lookback_minutes / rebalance_minutes / exposure_cap /
  drift_rebalance_threshold / vol_floor` are the natural first sweep axes.

**Files that change:** new generic sweep driver (lift `threshold_sweep`'s body into a
reusable `scenarios::sweep` module); strategy builders per family. **MEDIUM** only if
we insist on a *generic* (reflection-free) param-injection API across all 7 strategy
families at once; **SMALL** if we sweep one family (momentum) first.

### (c) Aggregate N runs into an outcome distribution — **SMALL→MEDIUM**

The aggregator consumes `Vec<RunReport>` (or `Vec<equity_curve>`) and emits a
distribution summary: per-metric `{mean, std, p5, p25, p50, p75, p95, min, max}` for
Sharpe / Sortino / Calmar / max-drawdown / total-return. The per-metric scalar
calculators **already exist** as free functions in `threshold_sweep.rs`
(`compute_sharpe_hourly`, `compute_sortino_hourly`, `compute_calmar`,
`compute_max_drawdown_f64`, `compute_total_return`) — lift them to a shared module
(they currently live in the bin; promote to `backtest` lib or a small `stats`
module). The new code is the **percentile / moment reducer** over N samples.

- **SMALL** for the reducer math.
- **MEDIUM** because **this is where the f64-determinism risk concentrates** (§3) —
  percentile selection and mean/std over N f64 samples must use a *fixed reduction
  order* and a *deterministic selection rule* (sort-then-index, ties broken
  explicitly). This is design care, not volume.

### Where the MC path generator lives — **extend `crates/data`, do NOT make a new crate**

Recommendation: a new module **`crates/data/src/synth/`** (e.g. `synth/gbm.rs`,
`synth/garch_path.rs`, `synth/bootstrap.rs`) behind a `MonteCarloPathGen` trait.
Rationale:
- Synthetic-bar generation is already conceptually a `crates/data` concern (it sits
  next to `fake_feed`, `mock_feed`, `replay_feed`).
- It keeps the generator testable/mockable independent of the engine (`Strategy`-
  first testability mandate).
- A new crate is overkill for ~3 generators and would add a workspace edge for no
  isolation benefit.
- **Lift** the duplicated GBM out of `momentum.rs`/`determinism.rs` into
  `data::synth::gbm` as the single source of truth — but **carefully**: the existing
  `synthetic_bars_hourly` body is anchor-load-bearing (its byte output feeds 84
  anchors). Any lift must be a **behaviour-preserving extraction** (same RNG draw
  order, same arithmetic) or it re-emits every synthetic-path anchor. Treat exactly
  like the ADR-0035 Phase-B scenario extraction (verbatim-copy discipline).

### Where the N-run aggregator lives — **`crates/backtest` (new `scenarios::montecarlo` + a `mc_runner` bin)**

The aggregator needs `RunReport` / equity curves and the per-metric calculators, all
of which are `crates/backtest`-local. Model it on `threshold_sweep`: a
`scenarios::montecarlo::run_path(input, path_seed, strategy)` cell wrapper + a
`bin/monte_carlo.rs` driver that fans out over `0..N` with rayon, sorts by path index
before render, and emits one distribution report.

### Trait sketch (illustrative; for the analyst's §6 sizing, not a commitment)

```
// crates/data/src/synth/mod.rs
pub trait MonteCarloPathGen {
    /// Pure: identical (universe, n, path_seed) ⇒ identical Vec<Bar>.
    fn generate(&self, universe: &[(Symbol, Decimal)], n: usize, path_seed: u64) -> Vec<Vec<Bar>>;
}
// impls: GbmPathGen (lift existing), GarchVolPathGen (reuse forecast_step),
//         BlockBootstrapPathGen (resample real parquet returns).
```

---

## §3. THE HARD TENSION — Monte-Carlo vs the anchor system (load-bearing)

The whole quality gate (`scripts/verify_anchors.sh`, 84 body-SHAs, ADR-0038 §D6
byte-immutable reports) assumes **one deterministic output per scenario**. MC is
stochastic. Reconciliation below.

### 3.1 Can seeded determinism save it? — **Yes, in principle, with a caveat.**

A *fixed* master seed → a *deterministic set* of N sub-seeds → N deterministic GBM
paths → N deterministic per-path metrics → a deterministic distribution summary.
Re-run with the same master seed ⇒ byte-identical summary. **This is structurally
sound** and is the same trick `threshold_sweep` uses (fixed `SEED = 0xC0FFEE`, sorted
cells, fixed-precision float formatting → 2-run byte-identity).

**Caveat 1 — sub-seed derivation must itself be deterministic & order-free.** Derive
path seeds by a fixed rule, e.g. `path_seed_j = ChaCha20Rng::seed_from_u64(master) →
j-th draw`, or `master.wrapping_add(j · 0x9E3779B9)` (the project's existing idiom,
momentum.rs:245). Either is fine; **lock it in the feature file + an ADR** so it never
drifts. Parallel execution (rayon) must NOT influence which seed a path gets — bind
`path_seed_j` to the *index* `j`, never to completion order.

**Caveat 2 — the aggregation reduction order must be fixed.** `mean = Σx/N` and
`std` over f64 are **not associative**; a parallel/unordered fold can produce
different last-bit results. Mandate: **collect all N per-path metrics into a
`Vec` indexed by `j`, sort or keep index order, then reduce sequentially in a single
fixed order.** Percentiles: sort the N samples with a total order (handle NaN
explicitly → there should be none, but assert), then index by the
nearest-rank/linear-interpolation rule chosen *and frozen* in the ADR. This mirrors
threshold_sweep's "sort cells before render" rule but applied to the reducer.

### 3.2 Does the existing seed discipline extend to a path ensemble? — **Yes.**

The scenario path already threads a single `--seed` into both the generator and
`PaperEngine`. Extending to an ensemble is additive: the master seed seeds a
sub-seed stream; each path's backtest is exactly today's deterministic single-path
backtest with `path_seed_j` substituted for the global seed. `0xC0FFEE` stays the
canonical master. **No change to the determinism *model* — only one more
deterministic index layered on top.** The 2-run byte-identity test pattern
(`tests/determinism.rs`) extends directly: run the whole ensemble twice, assert
identical summary body-SHA.

### 3.3 What is the anchor UNIT? — **Recommend: one distribution-summary report = ONE anchor. Do NOT anchor per-path.**

| Option | Anchors added | Pros | Cons | Verdict |
|---|---|---|---|---|
| **A. One summary report → 1 anchor** | +1 per MC scenario | Matches `threshold_sweep` precedent exactly; cheap to verify; the distribution IS the deliverable; survives N changes gracefully | Per-path detail not individually gated (but is reproducible from the seed) | **RECOMMENDED** |
| B. N per-path reports → N anchors | +N per scenario | Each path independently gated | N×84-style explosion; verify_anchors cost ×N; brittle (any path-count change re-locks all); defeats the point | Reject |
| C. Hybrid: summary anchored + a sampled-path digest in the body | +1 | Summary gated AND a few representative path hashes embedded in the (hashed) body for extra tamper-evidence | Slightly more body content to keep deterministic | Acceptable upgrade of A if extra assurance wanted |

**Recommendation: A (optionally C).** The anchor unit is the **body-SHA of the single
distribution-summary report**, with N, the master seed, the sub-seed rule, and the
generator id all printed **in the hashed body** (so changing any of them changes the
anchor — which is correct, they are inputs to the distribution). Run-varying fields
(`generated`, `host`, `wall_clock_s`, `git_commit`) stay in front-matter, exactly as
`threshold_sweep` does.

### 3.4 Float determinism — **the real risk, and it is cross-platform, not same-platform.**

The project has **already conceded** that f64 determinism is **same-machine only**.
ADR-0043 §"f64 conversion boundary" (lines 301-310) locks the sqrt-impact path's
determinism contract as *"bit-stable across Apple-Silicon canonical-box runs"* —
explicitly **not** cross-platform, and explicitly distinguished from the v2.5 TCN
Metal-vs-CPU divergence precedent. MC aggregation **multiplies** f64 operations:
per-path Sharpe/Sortino/Calmar (each a mean/std/sqrt over the equity curve) ×N, then
a percentile/moment reduction over N. Each is `f64::sqrt`, `ln`, `powi`, summation.

Risk assessment:
- **Same-machine (Apple Silicon canonical box): LOW.** `f64::sqrt`/`ln` are
  IEEE-754 correctly-rounded; a fixed reduction order makes the whole pipeline
  reproducible. This is the regime the anchor gate already runs in (CI + dev on the
  same arch). The existing 84 anchors live under this same constraint.
- **Cross-platform (x86 CI, different libm, FMA contraction): MEDIUM→HIGH.** `ln`
  is **not** required to be correctly-rounded by IEEE-754; libm implementations
  differ in the last bits. FMA-contraction of `a*b+c` (e.g. in `mean/std`) differs by
  target/codegen flags. Summation over many terms amplifies last-bit divergence,
  and percentile *selection* can flip an index if two samples are last-bit-close.
  **A cross-platform byte-identical distribution summary is NOT guaranteed.**

**Mandate for the follow-up feature (flag, do not solve here):**
1. **Declare the determinism scope explicitly** in the feature file + ADR:
   "byte-identical on the Apple-Silicon canonical box; cross-platform parity is
   NOT contracted" — inheriting ADR-0043's precedent verbatim. The anchor is locked
   on the canonical box; `verify_anchors.sh` runs there.
2. **Quantize the hashed summary** to a fixed decimal precision in the body
   (threshold_sweep already prints Sharpe at `{:.6}` etc.). Hashing the *formatted*
   string at fixed precision **absorbs sub-precision last-bit noise** and is the
   cheapest robustness lever — a 6-dp formatted Sharpe is identical even if the
   underlying f64 differs in bit 52. This is the single most effective mitigation
   and is already the de-facto pattern.
3. **Consider Decimal for the reducer** where feasible: percentile *selection* can be
   done on Decimal-quantized metrics (round each per-path metric to fixed dp as
   Decimal, sort, index) to make selection itself platform-independent. The
   underlying per-path metric compute stays f64 (Sharpe needs `ln`/`sqrt`), but the
   *aggregation* layer can be Decimal. **This is a design option for §6, gated by
   whether cross-platform parity is ever required.**
4. **If cross-platform byte-identity is ever required** (it is not today), that is a
   separate, larger effort (pinned softfloat libm, `-C target-feature` lockdown,
   no-FMA) and would need its own ADR. **Out of scope; name it, don't build it.**

**Net:** seeded determinism + fixed reduction order + fixed-precision formatted body
makes the distribution summary anchorable **on the canonical box** with the same
confidence as today's 84 anchors. Cross-platform is a known, accepted limitation, not
a new one.

### 3.5 Anchor-SHA / anchors.toml implications

A new MC scenario adds **one new anchor** under a **new namespace** (e.g.
`mc-robustness-2026-06`). This is **anchor-additive** — the 84 existing anchors stay
byte-identical (the MC code touches none of their code paths, provided the GBM lift
in §2 is behaviour-preserving). Per CLAUDE.md, adding/altering the 9 `spec/anchors.toml`
"anchor SHAs" guard requires an ADR; a *new additive* anchor row under a new
namespace is the routine `verify_anchors.sh` extension (precedent: every prior
namespace add). **An ADR IS needed** — not for the additive row, but to **lock the
MC determinism contract** (sub-seed rule, reduction order, precision, scope, anchor
unit). Name: see §5.

---

## §4. The learning-loop seam

**Finding: the loop is structurally CLOSED for the LLM-forecaster path and OPEN
(write-only) for parametric strategy/param selection.**

### 4.1 Reflection is NOT write-only — it already has a live read consumer

`reflection::retrieve_top_k` (`crates/reflection/src/retrieval.rs:22`) is wired into
the trader crate. The exact decision seam:

- **`ForecastContext::from_runtime`** (`crates/trader/src/llm_forecaster/types.rs:496`)
  builds a `RetrievalQuery{strategy_id, symbol_or_pair, current_regime}`, calls
  `retrieve_top_k(store, &query, REPORT_TIME_TOP_K=5)` (line 516), and stores the
  result in `ForecastContext.top_k_lessons` (field at types.rs:447).
- Those lessons are fed into the LLM forecaster **prompt** (the trader's
  `llm_forecaster/prompt.rs` consumes the context). So retrieved LessonCards
  **already influence a decision** — but only the *LLM forecaster's* decision, via
  natural-language prompt context.

So the architectural plumbing (store → retrieval → decision context) **exists and is
exercised**. The `no_strategy_caller.rs` grep gate (per ADR-0041) deliberately keeps
the *strategy* crate consumer-free; the *trader* crate is the legitimate consumer.

### 4.2 Where a "lesson-informed *parametric/MC* decision" would hook in — the gap

For the robustness/MC direction, the desired loop is: *"a robustness run produces a
distribution → distill a LessonCard → next strategy/param selection reads it."* The
**read seam for parametric selection does not exist**:

- The MC/sweep harness (§2) selects params from a **static `ParamGrid`** — there is
  no call to `retrieve_top_k` in the sweep/scenario path, and `Strategy` has no
  lesson-aware constructor.
- The natural hook is **at param-set selection time** in the new sweep/MC driver:
  before building the grid (or before picking the "headline" cell to promote), call
  `retrieve_top_k(store, query, k)` and let prior lessons **prune or weight** the grid
  (e.g. "in Volatile regime, lookback < 30 underperformed → skip those cells"). This
  is a NEW call site in the new `bin/monte_carlo.rs` / `scenarios::sweep` module,
  mirroring `ForecastContext::from_runtime` but feeding a *grid filter* instead of a
  *prompt*.
- The **write side** already exists end-to-end: `reflection::writer`,
  `post_mortem_analyst::generate_card` (deterministic card generation), `ClosedTrade`
  → `LessonCard` → `ReflectionStore::upsert`. A robustness run would write a card
  summarizing the distribution (e.g. "param-set X: median Sharpe 0.4, p5 −0.8 →
  fragile").

### 4.3 Effort to close the loop for parametric/MC selection — **MEDIUM**

- Write path: **SMALL** (reuse `reflection::writer` + a new card kind / outcome
  summarizing a distribution). The `LessonCard`/`LessonCardWriteRequest` types exist.
- Read-into-selection path: **MEDIUM** — a new retrieval call site in the harness +
  a deterministic grid-filter/weighting rule. The risk is **determinism**: lesson
  retrieval order must be deterministic (the store's `top_k` ordering must be a total
  order — verify it sorts by a stable key; audit-DB timestamp ties are a known hazard
  per architect.md, use 6-dp fractional seconds) so that a lesson-pruned grid is
  reproducible and thus still anchorable. **If lessons feed an anchored report, the
  retrieval must be deterministic or the report can't be anchored** — this couples §4
  to §3.
- **Recommendation:** ship MC/sweep **without** the learning loop first (anchorable,
  deterministic, no reflection coupling), then close the loop in a **separate phase**
  where the determinism of retrieval-into-selection is designed deliberately. Do not
  entangle the loop with the first anchorable MC deliverable.

---

## §5. Determinism + report-format guardrails (standing mandate)

A new **distribution report** type. Mandated split (per architect.md §Determinism &
report-format guardrails, and matching `threshold_sweep`'s proven layout):

**Front-matter (run-varying; NOT hashed):**
```
generated:        <wall-clock ISO-8601>
wall_clock_s:     <float>
host:             <hostname>
pid:              <int>            # if emitted
git_commit:       <sha>
data_revision_sha:<sha or n/a>     # if real-data bootstrap source
```

**Body (deterministic; hashed by the anchor) — MUST include every distribution
input so changing it changes the SHA:**
```
master_seed:      0xC0FFEE
n_paths:          <N>
sub_seed_rule:    <frozen string, e.g. "master + j*0x9E3779B9">
generator:        <gbm | garch-vol | block-bootstrap-real>
generator_params: <vol/drift or GARCH ω/α/β or block-length+source-revision>
param_set:        <the fixed strategy params under test>
--- per-metric distribution table ---
metric | mean | std | p5 | p25 | p50 | p75 | p95 | min | max   (all at fixed dp)
```

Guardrail specifics:
- **Money math stays `Decimal`** (`rust_decimal`). Equity curves, fills, fees are
  already Decimal end-to-end (`#![deny(clippy::float_arithmetic)]` in the backtest
  lib enforces this in the engine). Only the *statistical metric layer* (Sharpe etc.)
  uses f64 — unchanged from today.
- **RNG stays `ChaCha20Rng::from_seed(...)` / `seed_from_u64`** — the only RNG the
  generators may use. No `thread_rng`, no `SmallRng`. Seed the path stream from the
  master; print the rule in the body.
- **Fixed-precision formatting** of every hashed float (threshold_sweep uses `{:.6}`
  / `{:+.6}` / `{:.2}%`) — absorbs last-bit f64 noise (§3.4 mitigation #2).
- **Fixed reduction order** for mean/std/percentile (§3.1 caveat 2).
- **Audit-DB timestamps** (if any MC artifact lands in the audit DB) use **6-digit
  fractional seconds**, never `Rfc3339` second precision (ORDER BY tie hazard).

**New anchor-SHA implications → ADR required (name it, don't write it).** A new ADR
should lock: (D1) MC sub-seed derivation rule, (D2) aggregation reduction order +
percentile selection rule, (D3) distribution-report front-matter/body split + fixed
precision, (D4) anchor unit = 1 summary report under a new namespace, (D5)
determinism scope = Apple-Silicon-canonical-box (inherit ADR-0043 precedent verbatim;
cross-platform NOT contracted). **Proposed:** `ADR-0051 — Monte-Carlo robustness:
synthetic-path ensembles, distribution-report shape, and anchor determinism`
(next free number; 0050 is the current max). This audit does **not** author it.

---

## §6. Readiness verdict + rough roadmap

### 6.1 Capability readiness matrix

| Capability | Seam exists today? | Effort | Blocking dependency | ADR needed? |
|---|---|---|---|---|
| **Inject N synthetic paths** | Partial — generator is pure & seeded; **no ensemble loop / no path-gen trait** | **SMALL** (GBM-only) → **MEDIUM** (+GARCH-vol / block-bootstrap) | Behaviour-preserving lift of GBM out of `momentum.rs` (else re-locks synthetic anchors) | Yes — covered by ADR-0051 D1 |
| **Parameter sweep over strategy params** | **Yes** — `threshold_sweep` is a working 45-cell sweep; `registry` maps TOML→strategy | **SMALL** (momentum first) → **MEDIUM** (generic across 7 families) | Lift `threshold_sweep` body into reusable `scenarios::sweep` | Light — folds into ADR-0051 (param-grid determinism) |
| **Aggregate N runs → distribution** | Partial — per-metric calculators exist in the bin; **no percentile/moment reducer**; **no distribution report** | **SMALL** (math) → **MEDIUM** (determinism-careful reducer + report) | Fixed reduction order; fixed-precision body | Yes — ADR-0051 D2/D3/D4 |
| **Close the learning loop (lesson→param selection)** | Write path **exists**; LLM read path **exists**; **parametric read-into-selection seam absent** | **MEDIUM** | Deterministic retrieval ordering (store `top_k` total order; audit ts 6-dp); decouple from first anchorable MC ship | Possibly — only if lessons feed an *anchored* report (determinism of retrieval) |
| **Cross-platform byte-identity** (NOT required today) | No — accepted same-machine-only per ADR-0043 | **LARGE** (softfloat libm, target-feature lockdown) | — | Yes, separate ADR — **out of scope** |

### 6.2 Rough phase ordering (dovetails with analyst §6 candidates)

1. **Phase MC-1 — "GBM path ensemble + distribution report" (SMALL/MEDIUM).**
   Lift GBM into `data::synth::gbm` (behaviour-preserving), add the N-path loop and
   the percentile/moment reducer, emit one anchored distribution report for **one**
   strategy family (momentum) on **synthetic** data. Author ADR-0051 (D1-D5). This is
   the smallest end-to-end slice that proves the anchor-coexistence story. Ships the
   baseline-divergence e2e discipline from day 1 (CLAUDE.md non-negotiable): assert
   the distribution's p50 equity diverges from the single-path baseline by a testable
   epsilon when paths are non-degenerate.
2. **Phase MC-2 — "Generic param-sweep harness" (SMALL/MEDIUM).**
   Generalize `threshold_sweep` into `scenarios::sweep` + per-family strategy
   builders; emit a sensitivity report (the heatmap shape already exists). Reuses
   MC-1's determinism rules.
3. **Phase MC-3 — "Defensible generators" (MEDIUM).**
   Add GARCH-vol-driven and **block-bootstrap-of-real-bars** path generators (the
   latter is the most credible robustness MC — it stresses the strategy on
   resampled *real* return blocks, not a toy GBM). This is also where the
   "GARCH-as-generator-not-alpha" distinction must be stated explicitly in the ADR to
   pre-empt the retired-alpha objection.
4. **Phase MC-4 — "Close the learning loop" (MEDIUM).**
   Add the retrieval-into-selection seam + distribution-summarizing LessonCard
   writer. Design retrieval determinism deliberately. Keep separable from MC-1..3 so
   the anchorable deliverables are not blocked on loop determinism.

### 6.3 Candidate-feature sizing handed to the analyst's §6

- **MC-1** is the **minimum shippable robustness feature** and the natural first
  candidate. ~MEDIUM, mostly additive, one new anchor, one ADR.
- **MC-2** is a fast follow (the prototype already exists).
- **MC-3** is where the *product value* (robustness vs a toy path) really lands —
  recommend block-bootstrap of real Binance parquet returns as the headline generator.
- **MC-4** (learning loop) is genuinely valuable but should **not** gate MC-1..3 and
  carries the subtlest determinism design. Sequence it last.

---

## Cross-references

- Companion (WHAT/WHY): `spec/dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md` (analyst).
- Existing sweep prototype: `crates/backtest/src/bin/threshold_sweep.rs`,
  `crates/backtest/src/scenarios/threshold_sweep.rs`.
- Synthetic generator (single source after lift): `crates/backtest/src/scenarios/momentum.rs:98` (`synthetic_bars_hourly`).
- Anchor/body-hash contract: `crates/backtest/src/lib.rs:64` (`report_body_hash`), `spec/anchors.toml` (84 anchors), `scripts/verify_anchors.sh`.
- f64-determinism precedent (Apple-Silicon-canonical-box scope): `spec/architecture/adr/0043-simulated-latency-and-slippage.md` §"f64 conversion boundary" (lines 301-310); `crates/cost/src/slippage.rs:180-209`.
- Learning-loop read seam (LLM, exists): `crates/trader/src/llm_forecaster/types.rs:496` (`ForecastContext::from_runtime`), `crates/reflection/src/retrieval.rs:22`.
- GARCH recurrence (reusable generator primitive): `crates/forecast/src/garch.rs:104` (`forecast_step`).
- Retired-alpha guard: `spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md`.
- Next free ADR number: **0051** (0050 is current max).

## Changelog

- 2026-05-30 (architect): initial read-only architecture-readiness audit for the
  Monte-Carlo robustness / sensitivity-harness direction. No feature folder, ADR,
  trace row, or backlog edit created — audit only.
