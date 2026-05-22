---
adr: 0033
title: v2.5 TCN alpha-investigation — forecast-distribution + Sharpe-comparison report shape and F-verdict algorithm
status: accepted
date: 2026-05-18
supersedes: none
superseded-by: none
---

# ADR-0033: v2.5 TCN alpha-investigation report shape & F-verdict algorithm

## Context

[ADR-0028](0028-v25-dl-forecast-overlay-candle.md) commits the v2.5 forecast
slot to `candle`-trained TCN/PatchTST/Transformer. [ADR-0029](0029-tcn-checkpoint-provenance.md)
locks checkpoint provenance + LFS-anchor strategy. [ADR-0032](0032-backtest-realdata-path-and-revision-pin.md)
landed the real-Binance backtest path. All four `-realdata` scenarios shipped
on 2026-05-18 with `dampened=0`, the same finding M3 reported on synthetic
GBM, falsifying the M3 "out-of-distribution silence" hypothesis since the
training distribution **is** the real Binance hourly OHLCV.

[`spec/v25-tcn-alpha-investigation/feature.md`](../../v25-tcn-alpha-investigation/feature.md)
(analyst-locked 2026-05-18, operator picked MINIMAL scope) asks for a
read-only investigation across two milestones:

- **M-R-HAT** — emit a forecast-distribution report per checkpoint
  (BS-1, BS-2) showing the raw `r_hat` distribution against
  `sigma_train`, `ε = 0.0005`, and `confidence_threshold = 0.6`, and
  classify into one of four named failure modes (F1/F2/F3/F4).
- **M-SHARPE** — emit a single Sharpe / Sortino / Calmar / drawdown
  comparison report across the four `-realdata` scenarios; honest
  reporting contract holds.

The reports must be byte-deterministic so they can be locked as
anchors under a new `v2.6.0-alpha-investigation` version. The F-verdict
in M-R-HAT routes the operator's next funding decision; it cannot be
authored by hand-eyeballing a histogram — the algorithm has to be
code-checkable and reproducible.

Three orthogonal decisions to lock here, cited from `feature.md`:

1. **Read-path placement** — new bin under `crates/forecast/src/bin/`
   vs. extending the existing backtest TCN dispatch with a
   `--emit-r-hat-histogram` side-effect.
2. **Report-body shape** — what's body (hashed) vs. frontmatter
   (advisory), histogram representation, F-verdict location, Sharpe
   table column ordering. Both report families must follow the
   ADR-0032 § D4 precedent (run-varying fields in frontmatter only).
3. **F-verdict decision algorithm** — turning R4's verbal table into a
   deterministic, code-checkable function with explicit operator
   evidence (cited percentiles & gate-survival fractions).

## Decision

### D1. Read-path: new bin `crates/forecast/src/bin/forecast_distribution.rs`

A new read-only binary lives at
`crates/forecast/src/bin/forecast_distribution.rs`, mirroring the
`train_tcn.rs` shape. The bin owns its own `clap::Parser`, loads the
checkpoint via the shipped `TcnForecaster::load_anchor(AnchorScenario)`
API (`crates/forecast/src/tcn.rs:472`), iterates
`windows_for_symbol()` (`crates/forecast/src/features.rs:489`) for
each of the 10 USDT symbols over the requested span, runs
`TcnModel::forward()` (NOT `TcnForecaster::forecast()` — see D1.b
below), and emits a markdown report into
`spec/v25-tcn-alpha-investigation/reports/`.

**(A) bin** chosen over **(B) backtest dispatch extension** for these
reasons:

- **Separation of concerns.** The backtest harness exists to evaluate
  *strategies* against *bar streams*; this investigation evaluates a
  *model* against its *training distribution*. Bolting a model-
  introspection flag onto `run_tcn_overlay_weights_backtest()` would
  couple the strategy-eval pipeline to a model-eval concern that only
  this feature uses. Future v2.5a/v2.5b alpha-investigations want the
  same shape; an investigation bin generalises across forecaster
  families. The strategy dispatch does not.
- **Anchor neutrality (R6).** Touching `crates/backtest/src/main.rs`
  risks moving the 4 byte-locked `-realdata` anchors. A new bin in a
  different crate touches zero of the anchored paths. The R6
  non-regression contract is the load-bearing invariant of this
  investigation; the cheapest way to honour it is to write only
  new files.
- **Wall-clock budget alignment.** The backtest dispatch builds the
  full equity engine (order routing, ledger, k-way merge) — none of
  which the histogram pass needs. Reading 87,500 windows + forward
  pass is the same wall-clock budget (~40s/scenario per the
  backtest-real-binance-data test report § 4), but the bin runs
  zero equity-engine code so a future regression in equity logic
  cannot flip the histogram anchor.
- **Mockability.** A bin's input surface is `(scenario, data_root,
  out_dir, span)`. The four arguments are trivially fixturable in a
  test; the equivalent for the backtest dispatch requires
  constructing a `Scenario` struct + RNG seed + ledger init + …
  which is real plumbing to repeat per test.

The one cost of (A) over (B) — duplicating the parquet-load loop
that the backtest harness already has — is mitigated by reusing
`crates/forecast/src/features.rs::windows_for_symbol()` (the
training-time iterator that also lives behind a parquet directory
contract). Both call sites converge on the same iterator surface.

#### D1.a — Bin CLI surface

```rust
// crates/forecast/src/bin/forecast_distribution.rs
#[derive(clap::Parser)]
struct Args {
    /// Which anchored checkpoint to inspect.
    #[arg(long, value_enum)]
    scenario: ScenarioArg,         // Bs1 | Bs2

    /// Parquet root for real OHLCV bars (default: data/binance/).
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Output directory for the report (default: spec/v25-tcn-alpha-investigation/reports/).
    #[arg(long, default_value = "spec/v25-tcn-alpha-investigation/reports/")]
    out_dir: PathBuf,

    /// Evaluation span lower bound (UTC inclusive). Default matches the
    /// `-realdata` scenarios' span for the chosen checkpoint:
    ///   BS-1: 2023-01-01T00:00:00Z .. 2024-01-01T00:00:00Z
    ///   BS-2: 2024-01-01T00:00:00Z .. 2025-01-01T00:00:00Z
    #[arg(long)]
    span_start: Option<String>,

    /// Evaluation span upper bound (UTC exclusive). See `--span-start`.
    #[arg(long)]
    span_end: Option<String>,
}
```

When `--span-start` / `--span-end` are omitted (the default invocation
the developer wires into `Cargo.toml` example invocations), the span
is auto-derived from `--scenario` so the report's "evaluation span"
matches exactly the `-realdata` scenarios' span. This keeps the
report's body bytes deterministic across operator invocations.

#### D1.b — Forward-pass call site

The bin **calls `TcnModel::forward()` directly** with a manually-built
input tensor, not `TcnForecaster::forecast()`. Reasons:

- `forecast()` runs the full direction-quantisation + sigma-train-
  confidence-clamp + (optional) cache lookup path. We want the raw
  scalar `r_hat` BEFORE quantisation; quantising loses the
  information we're investigating.
- `forecast()` writes a cache row by default. The investigation is
  read-only against checkpoints and the cache — letting it write
  anywhere is scope creep.
- `forward()` is the shipped public method on `TcnModel`
  (`crates/forecast/src/tcn.rs:322`) and on `TcnForecaster`
  (`crates/forecast/src/tcn.rs:572`); calling it directly is a
  documented public-API consumption, not a private reach.

The tensor input is built from `FeatureWindow.features` (which is a
`candle_core::Tensor` shaped `[context_bars, 5]` under
`feature = "candle"`). The bin reshapes to `[1, 5, context_bars]`
matching the model's expected `(batch, channels, time)` convention
(see `tcn.rs:843`), runs `forward(&x, false)` (train=false), then
extracts the scalar via `flatten_all().to_vec1::<f32>()` exactly as
`forecast()` does at lines 850-854. No code drift.

#### D1.c — Failure-mode guards (K5 enforcement)

The bin enforces the read-only contract at the type level:

- **No writes to `crates/forecast/checkpoints/`.** The bin's only
  filesystem-write call is `std::fs::write(out_path, body)` where
  `out_path` is under `--out-dir`. Any other write site is a code
  review reject.
- **No invocation of `TcnForecaster::with_cache()` or
  `with_strict_replay()`.** The bin constructs `TcnForecaster` via
  `load_anchor()` only and uses it solely as a holder for `model`,
  `device`, `sigma_train`, `model_revision`.
- **No `--retrain`, `--update-sigma`, `--write-checkpoint` flags.** The
  CLI surface (D1.a) has exactly four args: `--scenario`,
  `--data-root`, `--out-dir`, `--span-{start,end}`.

These guards land as inline `#[deny(...)]` lints + a dedicated test
(`tests/forecast_distribution_bin_readonly.rs` — see tasks.md T-D-5).

### D2. Report shape — frontmatter vs. body discipline

Both report families follow the ADR-0032 § D4 precedent exactly:
run-varying fields go in YAML frontmatter (excluded from the body
hash via `scripts/hash_report.py`); deterministic content goes in
the body (hashed by the anchor).

#### D2.a — `forecast-distribution-bs{1,2}-realdata-YYYYMMDD.md`

**Frontmatter (advisory, NOT hashed):**

```yaml
---
slug: v25-tcn-alpha-investigation
scenario: forecast-distribution-bs1-realdata     # or bs2
generated: 2026-05-18T12:34:56Z                  # ISO-8601, second precision
wall_clock_s: 47.3                               # f64, one decimal
host: <hostname>                                 # advisory only
git_commit: <40 hex>                             # advisory only
model_revision: d1c3696d…                        # 64 hex (BS-1) / 3fabcabe… (BS-2)
sigma_train: 10.954250                           # f32, %.6f
data_revision_sha: 3a8b96c4…                     # 64 hex from data/binance/REVISION.toml
verdict: F1                                      # OR F2 / F3 / F4 — mirror of body
---
```

**Body (deterministic, hashed by anchor):**

```markdown
# Forecast-distribution report — BS-1 (real Binance hourly OHLCV)

## Checkpoint

| Field            | Value                                          |
|------------------|------------------------------------------------|
| Anchor scenario  | bs1                                            |
| model_revision   | d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2 |
| sigma_train      | 10.954250                                      |
| ε (deadband)     | 0.000500                                       |
| τ (confidence)   | 0.600000                                       |

## Evaluation span

| Field            | Value                                          |
|------------------|------------------------------------------------|
| Source           | Binance Vision via data/binance/               |
| Revision SHA     | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7 |
| Span (UTC, half-open) | 2023-01-01T00:00:00Z .. 2024-01-01T00:00:00Z |
| Symbols (10)     | ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT |
| Inferences       | 87590                                          |

## Summary statistics — r_hat (raw, pre-direction-quantisation)

| Stat         | Value           |
|--------------|-----------------|
| count        | 87590           |
| mean         | 0.000000123     |
| std          | 0.000001456     |
| min          | -0.000004321    |
| p01          | -0.000003456    |
| p05          | -0.000002123    |
| p10          | -0.000001789    |
| p25          | -0.000000567    |
| p50          | 0.000000089     |
| p75          | 0.000000678     |
| p90          | 0.000001890     |
| p95          | 0.000002234     |
| p99          | 0.000003567     |
| max          | 0.000004890     |
| abs_p50      | 0.000000234     |
| abs_p95      | 0.000002456     |
| abs_p99      | 0.000003789     |

| Gate              | Fraction of bars |
|-------------------|------------------|
| \|r_hat\| ≤ ε     | 0.999987         |
| \|r_hat\|/σ_train ≥ τ | 0.000000     |

## Histogram — r_hat over [-3σ_train, +3σ_train]

100 fixed bins, half-open `[low, high)`. Bin counts as integers.

| bin_idx | bin_low (×10⁻⁶) | bin_high (×10⁻⁶) | count   |
|---------|-----------------|-------------------|---------|
| 000     | -32862750       | -32205095         | 0       |
| 001     | -32205095       | -31547440         | 0       |
| …       | …               | …                 | …       |
| 050     |   -328627       |   +328627         | 87590   |
| …       | …               | …                 | …       |
| 099     | +32205095       | +32862750         | 0       |

## Confidence-gate survival — \|r_hat\|/σ_train per candidate τ

| τ    | bars surviving | fraction       |
|------|----------------|----------------|
| 0.10 | 0              | 0.000000       |
| 0.20 | 0              | 0.000000       |
| 0.30 | 0              | 0.000000       |
| 0.40 | 0              | 0.000000       |
| 0.50 | 0              | 0.000000       |
| 0.60 | 0              | 0.000000       |
| 0.70 | 0              | 0.000000       |
| 0.80 | 0              | 0.000000       |
| 0.90 | 0              | 0.000000       |

## Verdict

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Case              | F1                                             |
| Trigger evidence  | abs_p95 = 0.000002456 < 1e-6 across BS-1 (this report) AND BS-2 (cross-check required) |
| Recommended follow-on | spawn feature `v25-tcn-retrain` (revised loss: MSE on z-scored returns OR quantile head). Operator-decide. |

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…safetensors`.
- ε = 0.0005 per [v25-tcn-overlay/feature.md § R6](../../v25-tcn-overlay/feature.md#r6--output--forecastoverlay-closes-q6).
- τ = 0.6 per [v25-tcn-overlay/feature.md § D5](../../v25-tcn-overlay/feature.md#d5--tcn_overlay_momentum-strategy-thresholds).
- Histogram representation: 100 fixed bins over [-3·σ_train, +3·σ_train],
  ASCII-only, LF-only line endings, integer counts, fixed-precision
  floats (%.6f for stats, %.6f for gate fractions, %d for counts).
- F-verdict algorithm: see [ADR-0033 § D3](#d3-f-verdict-decision-algorithm).
```

**Floating-point canonicalisation** (locked here to forestall K4 drift):

| Field family            | Format                            |
|-------------------------|-----------------------------------|
| `sigma_train`, ε, τ     | `format!("{:.6}", x)` (6 decimals)|
| Histogram bin counts    | `format!("{}", x)` (integer)      |
| Histogram bin edges     | `format!("{}", (x * 1e6) as i64)` (microreturn integers) |
| Percentiles, abs_pNN    | `format!("{:.9}", x)` (9 decimals)|
| Mean, std, min, max     | `format!("{:.9}", x)` (9 decimals)|
| Gate fractions          | `format!("{:.6}", x)` (6 decimals)|

The bin-edge encoding as `(x * 1e6) as i64` (microreturn integers)
sidesteps IEEE-754 round-trip drift for the edges. The model output's
natural scale on hourly log-returns is 10⁻⁵ to 10⁻³; microreturns
(10⁻⁶ resolution) is finer than the meaningful signal. The 100-bin
table is wide enough to read at a glance and tight enough to be
hand-checkable.

**Percentile algorithm**: type-7 quantile (linear interpolation between
the two nearest order-stat values; default in numpy / R). Locked here
because percentile flavour drifts silently across stats libraries.
Implementation: sort the `r_hat` vector ascending in f32, compute
`h = (n - 1) * q`, return `v[floor(h)] + (h - floor(h)) * (v[ceil(h)]
- v[floor(h)])`. Sort uses `f32::total_cmp` (a total order that
treats NaN consistently — though `r_hat` from a healthy checkpoint
should never produce NaN, the contract is defence-in-depth).

#### D2.b — `sharpe-comparison-realdata-YYYYMMDD.md`

**Frontmatter (advisory, NOT hashed):**

```yaml
---
slug: v25-tcn-alpha-investigation
scenario: sharpe-comparison-realdata
generated: 2026-05-18T13:00:00Z
wall_clock_s: 165.2
host: <hostname>
git_commit: <40 hex>
data_revision_sha: 3a8b96c4…
sources:
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2023-fy-tcn-overlay-realdata.md
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2024-fy-tcn-overlay-realdata.md
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2023-fy-tcn-overlay-weights-realdata.md
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2024-fy-tcn-overlay-weights-realdata.md
---
```

**Body (deterministic, hashed by anchor):**

```markdown
# Sharpe / drawdown comparison — v2.6.0-realdata scenarios

## Methodology

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Source equity     | Re-derived from the four `-realdata` scenarios' anchored body bytes (see Sources). |
| Bar interval      | 1h                                             |
| Annualisation     | √(24·365) = 92.601295 (hourly → annual)         |
| Risk-free rate    | 0.000000 (constant)                            |
| Sharpe formula    | (mean_r - r_f) / std_r * √(24·365), arithmetic returns |
| Sortino formula   | (mean_r - r_f) / std_downside_r * √(24·365), downside vs r_f |
| Calmar formula    | (CAGR) / abs(max_drawdown), where CAGR = (final/initial)^(1/years) - 1, years = bars/8760 |
| Max drawdown      | max over t of (peak_equity_t - equity_t) / peak_equity_t, on the realised equity curve |
| Equity series     | Per-bar `equity_curve: Vec<Decimal>` length = bars + 1, starting at $100000.00 |

## Comparison table

| Scenario                                       | Variant      | Bars  | Final equity | Total return | Max drawdown | Trades | Dampen rate | Sharpe (ann) | Sortino (ann) | Calmar |
|------------------------------------------------|--------------|-------|--------------|--------------|--------------|--------|-------------|--------------|---------------|--------|
| top10-2023-fy-tcn-overlay-realdata             | passthrough  | 87590 | $113479.98   | 13.48%       | 73.73%       | 6203   | 0.00%       | 0.0123       | 0.0145        | 0.183  |
| top10-2024-fy-tcn-overlay-realdata             | passthrough  | 87840 | …            | …            | …            | …      | 0.00%       | …            | …             | …      |
| top10-2023-fy-tcn-overlay-weights-realdata     | real-weights | 87590 | $113479.98   | 13.48%       | 73.73%       | 6203   | 0.00%       | 0.0123       | 0.0145        | 0.183  |
| top10-2024-fy-tcn-overlay-weights-realdata     | real-weights | 87840 | …            | …            | …            | …      | 0.00%       | …            | …             | …      |

## Verdict

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Honest reading    | dampen rate = 0.00% across all four scenarios — TCN overlay is a no-op; equity curves are byte-identical between passthrough and real-weights variants per year. |
| Sharpe delta      | 0.0000 (passthrough vs. real-weights, 2023) / 0.0000 (2024) |
| Conclusion        | TCN at v2.5 / v2.6.0-realdata produces no alpha lift over the v1 momentum baseline. Verdict gated by M-R-HAT's F-verdict (this report alone cannot diagnose why). |
| Recommended follow-on | (a) wait for M-R-HAT verdict; (b) if M-R-HAT lands F4, fund `v25-tcn-horizon-bump` OR retire TCN at v2.6 bake-off. |

## Notes

- Read-only against the four `-realdata` reports listed in frontmatter.
- This report does NOT re-run any backtest — it computes Sharpe /
  Sortino / Calmar / drawdown from the anchored body bytes' Summary
  table (final equity, bars, trades) augmented with the per-bar equity
  curve which the report-emitting backtest writes alongside the
  report at `…/backtest-…-equity.csv` (see D2.b.i below).
- ASCII-only, LF-only line endings; floats `%.6f` (Sharpe / Sortino /
  Calmar) or `%.2f%%` (returns / drawdown / dampen rate); integer
  bar/trade counts. See D2.a percentile/format rules.
```

#### D2.b.i — Equity-series side artifact (CSV)

The Sharpe report computes Sharpe / Sortino / Calmar from a per-bar
equity series. The four `-realdata` anchored reports do NOT currently
emit the equity series (they emit only the final equity + max drawdown).
Three options:

- **Option α — Re-run the four scenarios** inside the M-SHARPE bin and
  collect `BacktestState.equity_curve` directly. Wall-clock: ~165s
  (4 × 40s). Pays the cost but is robust: the equity series exists
  in-memory at the moment Sharpe is computed.
- **Option β — Parse the four anchored report bodies** for final
  equity + bars and approximate Sharpe from those scalars. Wall-clock:
  instant, but Sharpe-from-scalars is not Sharpe; it's a return / max-
  drawdown bound. Fails the analyst's "Sharpe annualised" contract.
- **Option γ — Extend the backtest report writer** to also emit an
  `…-equity.csv` sibling per `-realdata` scenario, then the M-SHARPE
  bin reads four CSVs. Wall-clock: instant for M-SHARPE. Cost: a body
  change to `write_tcn_overlay_report()`, which moves the four
  `-realdata` anchor SHAs unless the CSV is written **outside** the
  report body and the report itself is unchanged.

**Decision: Option α — re-run the four scenarios.** Reasons:

- Option β violates the analyst contract (Sharpe annualised is
  load-bearing for the F4 verdict in M-R-HAT).
- Option γ requires touching `write_tcn_overlay_report()` and
  proving that the report-body bytes do not change. The four
  `-realdata` anchors are byte-locked in `spec/anchors.toml` as of
  2026-05-18; any developer error here flips them all. The cost is
  not worth the savings.
- Option α's wall-clock cost (~165s once at M-SHARPE close) is
  paid once. The investigation runs only on operator-trigger, not in
  CI. K2 wall-clock budget (analyst feature.md § Risk register)
  accommodates this.
- Option α uses the existing `cargo run -p backtest --features
  realdata,candle -- --scenario <name>` invocation as a subprocess.
  The M-SHARPE bin can shell out, parse the report it just produced,
  and compute Sharpe — or invoke `crates/backtest`'s public scenario
  API directly (revisit at developer level; the architect prefers
  the subprocess shape for tighter isolation).

The M-SHARPE bin's invocation contract:

```rust
// crates/forecast/src/bin/sharpe_comparison.rs
// (architect places this here too — both investigation bins live in
//  forecast/src/bin/ to keep the bin family co-located.)
#[derive(clap::Parser)]
struct Args {
    /// Output directory (default: spec/v25-tcn-alpha-investigation/reports/).
    #[arg(long, default_value = "spec/v25-tcn-alpha-investigation/reports/")]
    out_dir: PathBuf,

    /// Backtest binary path (default: target/release/backtest).
    #[arg(long, default_value = "target/release/backtest")]
    backtest_bin: PathBuf,

    /// Skip the re-run step; read the latest report under reports/ instead.
    /// Lets the operator re-author the Sharpe table without paying the
    /// wall-clock cost when the equity series is already on-disk via D2.b.i
    /// reconstruction.
    #[arg(long, default_value_t = false)]
    skip_rerun: bool,
}
```

**Reconstruction note**: if Option α is too expensive in some future
operator workflow, the M-SHARPE bin can be re-purposed to read a
per-scenario `equity_curve.bin` (raw `Vec<Decimal>` pickled) that a
future task could emit from `crates/backtest`. That's a follow-on.

### D3. F-verdict decision algorithm

The R4 table is verbal. This ADR turns it into a deterministic, code-
checkable algorithm that runs over the M-R-HAT histogram statistics
of BOTH checkpoints (BS-1 + BS-2) jointly. The algorithm produces
exactly one of `F1` / `F2` / `F3` / `F4` for each checkpoint's report,
plus a joint verdict that the orchestrator can route on.

#### D3.a — Per-checkpoint inputs

For each checkpoint `c ∈ {bs1, bs2}` the M-R-HAT report produces:

```rust
struct CheckpointStats {
    abs_p95: f32,       // 95th percentile of |r_hat|
    abs_p99: f32,       // 99th percentile of |r_hat|
    std: f32,           // sample stdev of r_hat (NOT |r_hat|)
    sigma_train: f32,   // pinned constant from checkpoint metadata
    epsilon: f32,       // = 0.0005, R6 constant
    tau: f32,           // = 0.6, D5 constant
    frac_inside_epsilon: f32,                 // fraction of bars with |r_hat| ≤ ε
    frac_passes_confidence_gate: f32,         // fraction with |r_hat|/σ_train ≥ τ
    confidence_gate_survival: [f32; 9],       // bars surviving τ ∈ {0.1, 0.2, …, 0.9}
}
```

All eight fields are emitted in the report body's tables (D2.a).

#### D3.b — Per-checkpoint verdict function

```rust
fn classify(s: &CheckpointStats) -> Verdict {
    // F1 — Training collapse.
    //
    // The model output is numerically zero everywhere. We define
    // "numerically zero" as |r_hat| p95 < 1e-6 — a very tight bound
    // chosen because (i) p95 means 95% of bars are tighter, (ii) at
    // 1e-6 the model output is below the bid-ask spread of any
    // reasonable venue, (iii) the contract is "the model fails to
    // express ANY signal magnitude," not "the model expresses small
    // signal." Cross-check against BS-2 happens in D3.c.
    if s.abs_p95 < 1e-6 {
        return Verdict::F1 {
            evidence: format!("abs_p95 = {:.9} < 1e-6", s.abs_p95),
            follow_on: "v25-tcn-retrain",
        };
    }

    // F2 — sigma_train mis-calibration.
    //
    // The model output has meaningful spread (std > 0.1 * sigma_train,
    // i.e. at-inference variance is at least 10% of the at-training
    // sigma) BUT no bar passes the confidence gate
    // (frac_passes_confidence_gate < 1e-6, i.e. essentially zero of
    // the ~87,500 bars). This is the signature of sigma_train being
    // pinned too high — division by a stale calibration value
    // squashes confidence to zero even when raw r_hat has spread.
    if s.std > 0.1 * s.sigma_train
        && s.frac_passes_confidence_gate < 1e-6
    {
        return Verdict::F2 {
            evidence: format!(
                "std = {:.9}, sigma_train = {:.6}, std/sigma_train = {:.3}, \
                 frac_passes_confidence_gate = {:.9}",
                s.std, s.sigma_train, s.std / s.sigma_train,
                s.frac_passes_confidence_gate
            ),
            follow_on: "v25-tcn-recalibrate",
        };
    }

    // F3 — Gating too tight.
    //
    // Real but small-magnitude signal exists: the IQR (p25..p75) of
    // |r_hat| straddles ε (so half of bars produce r_hat inside the
    // deadband and half outside), AND the confidence-gate survival at
    // τ = 0.6 is non-trivial (≥ 1e-4 = 0.01% of ~87,500 bars = ~9
    // bars). The model emits real signal but the (ε, τ) pair filters
    // it. Note: the analyst F3 trigger ("p25..p75 straddles ε") is
    // operationalised as "p75(|r_hat|) > ε > p25(|r_hat|)" — which is
    // exactly the IQR straddle condition. We use abs_p95 < ε on top
    // to confirm the signal is small-magnitude (not just isolated
    // outliers); this distinguishes F3 from F4.
    let abs_p25 = s.abs_p99 * 0.0;  // placeholder; bin emits real p25 in body
    let abs_p75 = s.abs_p99 * 0.0;  // placeholder; bin emits real p75 in body
    // IMPORTANT: the architect describes the gates here; the developer
    // wires the actual p25/p75 fields into CheckpointStats at
    // implementation time. Keep the four gates F1-F4 mutually
    // exclusive (asserted by a test).
    if s.confidence_gate_survival[5 /* τ=0.6 index */] >= 1e-4
        && s.frac_inside_epsilon > 0.5
    {
        return Verdict::F3 {
            evidence: format!(
                "frac_inside_epsilon = {:.6}, confidence_gate_survival[τ=0.6] = {:.6}",
                s.frac_inside_epsilon, s.confidence_gate_survival[5]
            ),
            follow_on: "v25-tcn-threshold-tuning",
        };
    }

    // F4 — Model genuinely has no signal at 1h horizon.
    //
    // Fallback case: the |r_hat| distribution is wide enough that
    // F1/F2/F3 are all false. The model emits forecasts that survive
    // both ε and τ but those forecasts are directionally wrong or
    // uncorrelated with realised next-bar returns — which the
    // M-SHARPE report independently confirms (Sharpe ≤ baseline).
    // The two reports' verdicts MUST be consistent: F4 in M-R-HAT
    // requires the M-SHARPE table to show no alpha lift, and the
    // M-SHARPE verdict body cross-references the M-R-HAT verdict.
    Verdict::F4 {
        evidence: format!(
            "abs_p95 = {:.9} >= 1e-6, std/sigma_train = {:.3} <= 0.1 OR \
             frac_passes_confidence_gate = {:.9} >= 1e-6, \
             frac_inside_epsilon = {:.6} <= 0.5",
            s.abs_p95, s.std / s.sigma_train,
            s.frac_passes_confidence_gate, s.frac_inside_epsilon
        ),
        follow_on: "v25-tcn-horizon-bump-or-retire",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    F1 { evidence: String, follow_on: &'static str },
    F2 { evidence: String, follow_on: &'static str },
    F3 { evidence: String, follow_on: &'static str },
    F4 { evidence: String, follow_on: &'static str },
}
```

**Mutual exclusivity** — F1 / F2 / F3 / F4 are evaluated in priority
order (F1 → F2 → F3 → F4 fallthrough). The first triggering case
returns. A unit test in `tests/forecast_distribution_verdict.rs`
asserts mutual exclusivity over a small hand-built fixture grid.

**Numerical-zero threshold** — `1e-6` is chosen because the model's
training huber-delta is `1e-3` and a healthy non-collapsed model is
expected to emit r_hat at least an order of magnitude above the
delta knee on average; `1e-6` is three orders of magnitude tighter
than the delta knee, which is unambiguous "the model output is
saturated at zero." If a future checkpoint legitimately emits r_hat
in the 1e-7 to 1e-5 range as real signal, F1's threshold revisits
in a follow-on ADR.

#### D3.c — Joint (cross-checkpoint) verdict

The M-R-HAT pass produces two reports (BS-1 + BS-2). Each carries its
own per-checkpoint verdict. The orchestrator routes on a **joint**
verdict that combines them:

| BS-1 verdict | BS-2 verdict | Joint verdict       | Follow-on              |
|--------------|--------------|---------------------|------------------------|
| F1           | F1           | F1                  | `v25-tcn-retrain`      |
| F2           | F2           | F2                  | `v25-tcn-recalibrate`  |
| F3           | F3           | F3                  | `v25-tcn-threshold-tuning` |
| F4           | F4           | F4                  | `v25-tcn-horizon-bump-or-retire` |
| F1/F2/F3/F4 mismatch | ditto | F-MIXED             | open analyst-spawn to triage; operator-decide. |

If the two checkpoints disagree (e.g. BS-1 = F1, BS-2 = F2), the
joint verdict is `F-MIXED` and the follow-on is "analyst triage of
the disagreement" — this is a 5th outcome that the analyst's R4
table did not enumerate but is the honest reading of a real
divergence. The investigation closes with `F-MIXED` recorded in
both reports' Verdict sections AND in `feature.md § Verification`.

The joint verdict is recorded in the M-R-HAT BS-1 report body's
Verdict section (per-checkpoint), but NOT in a separate file — the
orchestrator reads BS-1 + BS-2 verdicts together to derive the joint
label. (One report per checkpoint, two report files, joint
disposition surfaced in `feature.md § Verification` at M-FINAL.)

#### D3.d — Algorithm reproducibility contract

The decision tree is implemented in `forecast_distribution.rs` and
covered by a unit test that exercises one fixture per verdict label
(F1 / F2 / F3 / F4) plus a mutual-exclusivity property test (random
fixture → exactly one verdict returned). A future analyst spawning
`v25-tcn-retrain` / `v25-tcn-recalibrate` / etc. cites this ADR for
the canonical algorithm — neither the analyst nor a future architect
re-derives the thresholds.

If a future feature wants to tune the F-thresholds (e.g. F1 at
`1e-7` instead of `1e-6` because BS-1 + BS-2 outputs are non-zero
but tiny), that's a superseding ADR. This ADR does not amend its
own thresholds.

### D4. Anchor + version naming

- **New anchors** (under version `v2.6.0-alpha-investigation`):
  - `forecast-distribution-bs1-realdata`
  - `forecast-distribution-bs2-realdata`
  - `sharpe-comparison-realdata` (subject to determinism check; falls
    through to "ship un-anchored with `## Not anchorable` body section"
    if the body fails the two-run byte-identity gate)
- **Existing 19 anchors stay byte-identical.** The new bin writes
  only to `spec/v25-tcn-alpha-investigation/reports/`. The Sharpe-
  comparison bin **does not** invoke `crates/backtest` in any mode
  that writes to `spec/backtest-real-binance-data/reports/` (re-runs
  go through `--out` redirected to a tempdir; the four `-realdata`
  anchors are read by file-pattern matching the existing tempdir
  output filenames).

Anchor count progression:
- Pre-feature: 19 (R6 contract).
- Post M-R-HAT: 21 (+ 2 forecast-distribution reports).
- Post M-SHARPE: 21 OR 22 (depending on Sharpe-report anchorability).

## Alternatives considered

1. **Extend `crates/backtest` with `--emit-r-hat-histogram`** — the
   `feature.md § R3` alternative. Rejected per D1 above: couples
   strategy-eval to model-eval; risks moving the 4 `-realdata`
   anchors; doesn't generalise to v2.5a/v2.5b alpha-investigations
   (PatchTST has a different forward-pass call shape).

2. **Write the F-verdict by hand in the report Verdict section** —
   skipping the algorithm. Rejected: the F-verdict is load-bearing
   for follow-on routing (K1 mitigation); the operator needs a
   reproducible derivation. A free-text Verdict section invites
   subtle drift across runs and is not anchor-friendly (the operator-
   typed text bytes change).

3. **Bake F-thresholds as TOML config** instead of source constants.
   Rejected: thresholds are load-bearing for the algorithm; a TOML
   config invites operators to tune them silently between runs and
   defeats the anchor contract. Future tuning happens in a
   superseding ADR with an explicit follow-on feature.

4. **Higher-resolution histogram (1000 bins)** — Rejected: 100 bins
   over `[-3σ, +3σ]` is already 7-bin-per-σ resolution, which is
   finer than the visual question being answered ("is the
   distribution all stacked at zero, somewhat-spread, or wide?").
   1000 bins makes the report body 10× larger with no decision
   benefit.

5. **Variable-edge histogram (e.g. log-spaced)** — Rejected:
   variable edges drift more easily across implementations than fixed
   edges (different sort+bisect routines), and the operator's
   question is "what's the magnitude distribution near zero," which
   fixed edges centred on zero answer naturally.

6. **Compute Sharpe inline in `crates/backtest`** (i.e. modify
   `write_tcn_overlay_report` to emit Sharpe rows in the body) —
   Rejected: would move the 4 `-realdata` anchors. The M-SHARPE
   computation lives in a side-binary that reads pinned anchors.

7. **Annualisation by sqrt(525_600)** — the existing
   `crates/backtest::compute_sharpe()` formula. Rejected: that
   annualises minute-resolution returns (correct for the minute-bar
   v0/v1 scenarios it was written for), but is wrong by 24× for
   hourly bars. The M-SHARPE bin uses sqrt(24·365) per the analyst
   contract; the existing `compute_sharpe` is NOT reused. A
   dedicated `compute_sharpe_hourly` / `compute_sortino_hourly`
   helper lives in `crates/forecast/src/bin/sharpe_comparison.rs` or
   a shared module — developer call at T-D-7.

## Consequences

**New files (this ADR scope):**
- This file: `spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md`
- `crates/forecast/src/bin/forecast_distribution.rs` (~350 LoC)
- `crates/forecast/src/bin/sharpe_comparison.rs` (~250 LoC)
- `crates/forecast/tests/forecast_distribution_verdict.rs` (~80 LoC,
  unit test of the F-verdict algorithm)
- `crates/forecast/tests/forecast_distribution_bin_readonly.rs`
  (~40 LoC, asserts no writes outside `--out-dir`)
- `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260518.md`
- `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260518.md`
- `spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260518.md`

**Modified files:**
- `spec/architecture/adr/README.md` — registry row added for ADR-0033.
- `spec/anchors.toml` — 2 (or 3) new anchor rows under
  `v2.6.0-alpha-investigation`.
- `spec/trace.toml` — `REQ-V25-TCN-ALPHA-001` `arch` / `crates` /
  `tests` / `anchors` columns filled.
- `spec/v25-tcn-alpha-investigation/feature.md` — § Design block
  added, changelog entry.
- `spec/v25-tcn-alpha-investigation/tasks.md` — T-D-N decomposition
  added under M-R-HAT + M-SHARPE.

**Cross-phase implications:**
- v2.5a (PatchTST) and v2.5b (Transformer) alpha-investigations, if
  they happen, inherit this report shape verbatim — substitute
  `tcn-bs{1,2}` with `patchtst-bs{1,2}` / `tx-bs{1,2}` in the bin
  paths. The F-verdict algorithm applies as-is to any 1h-horizon
  scalar-output forecaster.

**Enforced by:**
- `cargo test -p forecast --test forecast_distribution_verdict` —
  one fixture per F-label + mutual-exclusivity property test.
- `cargo test -p forecast --test forecast_distribution_bin_readonly`
  — fail-loud on any write outside `--out-dir`.
- `bash scripts/verify_anchors.sh` — must report `21/21` (R1 only)
  or `22/22` (R1 + R5) post-M-FINAL; pre-M-FINAL must report
  `ANCHORS PASS (19/19)` (the R6 contract).
- M-SHARPE bin determinism: two sequential runs produce body-SHA
  byte-identity per the ADR-0032 § D4 precedent (tester gate).

**What breaks if this is violated:**
- An F-verdict authored without the algorithm (e.g. hand-written
  in a report body) → mutual-exclusivity check fails or the
  evidence string doesn't match the values, and the orchestrator
  cannot route. Caught by `forecast_distribution_verdict` test.
- A developer adds a write site outside `--out-dir` in the
  inspector bin → `forecast_distribution_bin_readonly` test fails
  and CI rejects.
- Floating-point format drift (e.g. someone changes `%.6f` to
  `%.5f`) → body SHA flips on a second run, anchor lock fails.
  Caught by the determinism gate at M-FINAL.

**What this enables:**
- The operator gets a code-checkable F-verdict that routes
  follow-on funding decisions without requiring eyeballing
  histograms.
- v2.5a PatchTST + v2.5b Transformer alpha-investigations reuse
  the report shape + F-verdict algorithm verbatim, dropping their
  authoring cost to ~1 day.
- The v2.6 bake-off (REQ-V26-BAKEOFF-001) has a deterministic,
  cross-phase Sharpe-comparison template it can extend.

## References

- [ADR-0028](0028-v25-dl-forecast-overlay-candle.md) — parent
  framework decision.
- [ADR-0029](0029-tcn-checkpoint-provenance.md) — checkpoint
  provenance + LFS-anchor.
- [ADR-0032](0032-backtest-realdata-path-and-revision-pin.md) —
  `-realdata` path + frontmatter-vs-body discipline (the precedent
  this ADR's D2 follows).
- [`spec/v25-tcn-alpha-investigation/feature.md`](../../v25-tcn-alpha-investigation/feature.md)
  — analyst R1-R6 + F1-F4 + operator-locked minimal scope.
- [`spec/v25-tcn-overlay/feature.md`](../../v25-tcn-overlay/feature.md)
  § R6, § D5 — ε / τ constants this ADR pins into the report body.
- `crates/forecast/src/tcn.rs:472` — `TcnForecaster::load_anchor`.
- `crates/forecast/src/tcn.rs:572`, `:322` — `forward()` public API.
- `crates/forecast/src/features.rs:489` — `windows_for_symbol`.
- `crates/backtest/src/main.rs:2428` — existing `compute_sharpe`
  (minute-annualised; NOT reused).

## Changelog

- 2026-05-18 (architect): initial accept. Locks four orthogonal
  decisions (read-path placement, report shape + canonicalisation,
  F-verdict decision algorithm, anchor naming). Covers T-AR-2
  decomposition surface for M-R-HAT + M-SHARPE. Cross-refs
  `REQ-V25-TCN-ALPHA-001` in `spec/trace.toml`.
