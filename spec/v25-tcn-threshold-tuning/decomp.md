---
slug: v25-tcn-threshold-tuning
phase: M-T1
owner: architect
date: 2026-05-21
status: locked
---

# M-T1 — Architect decomposition (v25-tcn-threshold-tuning v0.1.0)

> Architect lock for the cheap τ × ε sweep on top of the recalibrated
> v2.5 TCN checkpoints. Operator approved Q1-Q6 = analyst defaults on
> 2026-05-21 via standing "Autoapprove all". This decomposition is
> **anchor-additive**; the 26 predecessor anchors stay byte-identical
> body-SHAs (note pre-existing `verify_anchors.sh` glob-resolver
> collision documented in § 6, NOT introduced by this feature).

## 1. Architect-decide resolutions

### T-AR-1 — Design lock

The decisions D-AR-1.a … D-AR-1.j below land verbatim as the
`## Design` section of `spec/v25-tcn-threshold-tuning/feature.md`
(architect appends at M-T1). This decomp.md is the canonical
architecture reference; feature.md § Design is a cross-pointer.

#### D-AR-1.a — Bin name + location

The sweep tool ships at
[`crates/forecast/src/bin/threshold_sweep.rs`](../../crates/forecast/src/bin/threshold_sweep.rs)
(new file). Analyst's suggested name confirmed because:

- Co-locates with the existing investigation-bin family:
  [`forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs),
  [`sharpe_comparison.rs`](../../crates/forecast/src/bin/sharpe_comparison.rs),
  [`recalibrate_sigma_train.rs`](../../crates/forecast/src/bin/recalibrate_sigma_train.rs).
  Cross-grep `crates/forecast/src/bin/` enumerates every read-only
  model-introspection bin in one shot.
- Mirrors the verb-noun shape (`forecast_distribution`,
  `sharpe_comparison`, `recalibrate_sigma_train`, `threshold_sweep`).
- Keeps `forecast_distribution.rs` + `sharpe_comparison.rs` BODIES
  byte-identical (no extension), so the predecessor's 4 anchored
  bodies (`forecast-distribution-bs{1,2}-realdata-recalibrated`,
  `recalibrate-sigma-train-bs{1,2}`) stay byte-identical by
  construction.

Rejected alternatives:

- **Extend `forecast_distribution.rs` with `--sweep` mode** — rejected.
  That file's body owns the F-verdict report shape per ADR-0033 § D2.a;
  adding sweep behavior to it would diverge the 2-run determinism gate
  from the predecessor's anchored shape. Anchor-blast radius too large.
- **Extend `sharpe_comparison.rs`** — rejected. Same anchor-blast
  concern. Plus `sharpe_comparison.rs` is the v1-vs-TCN comparison
  shape; a 9 × 5 sweep is a different report grammar.
- **`crates/backtest/src/bin/threshold_sweep.rs`** — rejected. The
  sweep is fundamentally a forecast-distribution / overlay-introspection
  task; it lives next to the other investigation bins, NOT under the
  matching-engine binary. Keeps `crates/backtest/` clean.

#### D-AR-1.b — Bin shape — orchestrator-not-spawner

The sweep bin does NOT shell out to `backtest` 90 times. It:

1. Loads the recalibrated checkpoint in-process via
   [`TcnForecaster::load_from_paths(safetensors, metadata_recalibrated_path)`](../../crates/forecast/src/tcn.rs)
   — same shipped API the `forecast_distribution.rs` `--metadata-path`
   branch uses (ADR-0035 D3 contract).
2. Loads real-Binance bars once via
   [`backtest::RealDataBarSource`](../../crates/backtest/src/main.rs#L712-L721)
   (read across the v25-2023-FY-bs1 span and the v25-2024-FY-bs2 span;
   re-uses the same data-revision-SHA pin from
   `data/binance/REVISION.toml` = `3a8b96c4…`).
3. For each `(τ, ε)` cell, constructs an in-process
   `TcnOverlayMomentumStrategy` via the new
   `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` builder (D-AR-1.f) — NOT via
   the existing `with_tcn_bs{1,2}` builder which is anchor-load-bearing.
4. Runs the backtest in-process by calling
   [`backtest::scenarios::tcn_overlay_weights::run`](../../crates/backtest/src/scenarios/tcn_overlay_weights.rs)
   with the cell's strategy substituted (or by calling a thin sweep-cell
   helper in `crates/backtest/src/scenarios/threshold_sweep.rs` — see
   D-AR-1.c). Re-uses the existing scenario contract; deterministic on
   the same `seed = 0xC0FFEE`.
5. Parses the resulting metrics (Sharpe ann, Sortino ann, Calmar, max
   drawdown, total return, trades, dampen rate) directly from the
   in-memory `TcnOverlayRunResult` struct returned by `run()`. NO
   markdown re-parse needed (in-memory is faster + deterministic).
6. Aggregates 45 cells per checkpoint into the heatmap report
   (D-AR-1.h).

This shape is faster (loads the model once, loads bars once) AND avoids
the predecessor's CLI-shell-out overhead AND keeps the
`backtest --scenario top10-{2023,2024}-fy-tcn-overlay-realdata` default
invocation byte-identical (which Q5=(c) requires).

Rejected alternatives:

- **(a)** Shell out to `cargo run -p backtest …` 90 times — rejected.
  ~30s × 90 = 45min wall-clock + per-shell-out cost ~3-5s × 90 ≈
  ~5-7min overhead. Total ~50-55min. The in-process path saves load-
  model-and-bars cost (load once = ~30s; in-process per-cell ~20-25s
  → total ~30-40min).
- **(b)** Wire `--tcn-tau` + `--tcn-epsilon` flags onto the
  `backtest` CLI (per K6 / analyst's Q1 alt option) — rejected. Adds
  surface to a binary the spec-auditor considers anchor-critical (any
  new CLI surface on `backtest` requires anchor-neutrality proof per
  ADR-0032). The in-process sweep keeps the `backtest` CLI byte-identical;
  R8 invariant easier to verify.

#### D-AR-1.c — Optional thin sweep-cell helper

If the developer finds that exposing
`backtest::scenarios::tcn_overlay_weights::run` with a custom
pre-constructed strategy is awkward (the existing `run()` accepts a
`TcnScenarioInput` and builds its own strategy internally; see
[`tcn_overlay_weights.rs:79-85`](../../crates/backtest/src/scenarios/tcn_overlay_weights.rs)),
Wave A T-D-N2 adds a thin helper to
`crates/backtest/src/scenarios/`:

```rust
// crates/backtest/src/scenarios/threshold_sweep.rs (NEW, ~80 lines)
pub async fn run_cell(
    input: TcnScenarioInput,
    seed: u64,
    overlay_strategy: strategy::TcnOverlayMomentumStrategy,
) -> Result<TcnOverlayRunResult>;
```

This is mechanically a copy-paste of `tcn_overlay_weights::run` with
the strategy-construction block (lines 74-85) replaced by an "use
caller-supplied strategy" assignment. The existing run() stays
byte-identical (zero behavioral delta for the
`top10-{2023,2024}-fy-tcn-overlay-{,weights-}realdata` anchor bodies).

**Default choice**: developer at T-D-N2 may EITHER extract the helper
OR refactor `tcn_overlay_weights::run` to accept an optional
pre-constructed strategy (additive arg with default = build via existing
path). Both shapes preserve the 26 predecessor anchors. Architect
recommends the helper (cleaner separation; smaller diff to anchored
file).

#### D-AR-1.d — CLI surface (5 args, mirrors `recalibrate_sigma_train`)

```rust
// crates/forecast/src/bin/threshold_sweep.rs
#[derive(Parser, Debug)]
#[command(
    name = "threshold_sweep",
    about = "Sweep τ × ε grid (9 × 5) on top of recalibrated TCN checkpoints; emit Sharpe-delta heatmap report",
    long_about = "Loads the anchored TCN checkpoint by --scenario, applies the \
                  recalibrated σ_train overlay from --metadata-path (per ADR-0035 \
                  D3), loads real-Binance bars once, then runs the realdata \
                  backtest in-process at each (τ, ε) cell (9 × 5 = 45 cells). \
                  Emits a 5-heatmap markdown report under --out-dir. Read-only \
                  against safetensors + metadata; no retraining; no σ_train change. \
                  Original .metadata.json + .safetensors + .metadata.recalibrated.json \
                  files stay byte-identical."
)]
struct Args {
    /// Which anchored checkpoint to sweep.
    #[arg(long, value_enum)]
    scenario: ScenarioArg,                        // Bs1 | Bs2

    /// Parquet root for real OHLCV bars.
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Path to the recalibrated metadata overlay
    /// (`tcn-bs{1,2}-<sha>.metadata.recalibrated.json`).
    /// Required — the sweep is meaningless against the original
    /// inflated σ_train.
    #[arg(long)]
    metadata_path: PathBuf,

    /// Output directory for the heatmap report.
    #[arg(long, default_value = "spec/v25-tcn-threshold-tuning/reports/")]
    out_dir: PathBuf,

    /// Pinned data revision SHA (defaults to v2.6.0-realdata pin).
    /// Override only when re-fetching upstream parquets.
    #[arg(
        long,
        default_value = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7"
    )]
    expected_revision_sha: String,
}
```

Read-only contract (mirrors ADR-0033 § D1.c + ADR-0035 D3):

- No `--retrain`, `--update-original`, `--write-safetensors`,
  `--write-metadata` flags. Clap denies unknown flags by default.
- Bin's only writes: 1 markdown report under `--out-dir`.
- `--metadata-path` is REQUIRED (no default). Forces the operator to
  point at the recalibrated overlay file (which is the load-bearing
  precondition for a meaningful sweep — the original σ_train is
  608× / 580× inflated).

#### D-AR-1.e — Grid + cell enumeration

```rust
const TAU_GRID: [f32; 9] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
const EPSILON_GRID: [f32; 5] = [0.0001, 0.0005, 0.001, 0.005, 0.01];

// Cell iteration order: τ outer, ε inner (matches heatmap row=τ, col=ε
// shape per R2 / D-AR-1.h). Order-invariant assembly per R9 / K3.
let mut cells: Vec<(f32, f32, TcnOverlayRunResult)> = Vec::with_capacity(45);
for &tau in &TAU_GRID {
    for &eps in &EPSILON_GRID {
        let strategy = TcnOverlayMomentumStrategy::with_tcn_bs1_ledger_tuned(
            base.clone(), Decimal::try_from(tau as f64)?, Decimal::try_from(eps as f64)?,
        )?;
        let result = backtest::scenarios::threshold_sweep::run_cell(
            input.clone(), 0xC0FFEE, strategy,
        ).await?;
        cells.push((tau, eps, result));
    }
}
// Sort by (τ, ε) lexicographic key BEFORE rendering — guarantees
// order-invariant assembly even if parallel execution lands rows out
// of order (D-AR-1.j parallelism contract).
cells.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
```

f32 → Decimal conversion uses `Decimal::try_from(f64)` (not `dec!()`
since these are runtime values); the canonical Decimal precision is
preserved through to the strategy `confidence_threshold` field.

#### D-AR-1.f — `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` builder

Per Q5=(c), the additive builders sit alongside the existing
`with_tcn_bs{1,2}_ledger` (and underneath `with_tcn_bs{1,2}`) — those
stay byte-identical (`dec!(0.6)` literal). New shape:

```rust
// crates/strategy/src/tcn_overlay_momentum.rs (additive only)

#[cfg(feature = "forecast-audit-tick")]
pub fn with_tcn_bs1_ledger_tuned(
    base: MomentumStrategy,
    ledger: audit::Ledger,
    confidence_threshold: Decimal,
    direction_epsilon: Decimal,
) -> Result<Self, forecast::tcn::TcnForecasterError> {
    let forecaster = TcnSyncForecaster::load_bs1()?
        .with_ledger(ledger)
        .with_forecast_context("tcn_overlay_momentum_bs1".to_string(), "MULTI".to_string())
        .with_direction_epsilon(direction_epsilon);                       // NEW (D-AR-1.g)
    Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
}

#[cfg(feature = "forecast-audit-tick")]
pub fn with_tcn_bs2_ledger_tuned(
    base: MomentumStrategy,
    ledger: audit::Ledger,
    confidence_threshold: Decimal,
    direction_epsilon: Decimal,
) -> Result<Self, forecast::tcn::TcnForecasterError> {
    let forecaster = TcnSyncForecaster::load_bs2()?
        .with_ledger(ledger)
        .with_forecast_context("tcn_overlay_momentum_bs2".to_string(), "MULTI".to_string())
        .with_direction_epsilon(direction_epsilon);                       // NEW (D-AR-1.g)
    Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
}
```

NO default-arg overloading. Every caller of `_tuned` MUST pass explicit
`(τ, ε)`. Rejected alternative — `Option<Decimal>` args defaulting to
`(0.6, 0.0005)` — silently re-implements the existing builder behavior
and invites cargo-cult use; explicit args are the cheaper-to-reason-about
contract.

**Sweep bin invocation path** — the sweep uses a NON-audit variant
because the sweep bin is read-only and the 90 backtests do NOT emit
`forecast_events` SQL rows (avoids polluting the audit DB with 90 ×
~87,500 = ~7.9M synthetic events). So the sweep needs ALSO a
non-ledger `_tuned` builder:

```rust
// Sweep-path variant — no ledger, no forecast_context, opt-in (τ, ε).
#[cfg(feature = "forecast")]
pub fn with_tcn_bs1_tuned(
    base: MomentumStrategy,
    confidence_threshold: Decimal,
    direction_epsilon: Decimal,
) -> Result<Self, forecast::tcn::TcnForecasterError> {
    let forecaster = TcnSyncForecaster::load_bs1()?.with_direction_epsilon(direction_epsilon);
    Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
}

#[cfg(feature = "forecast")]
pub fn with_tcn_bs2_tuned(
    base: MomentumStrategy,
    confidence_threshold: Decimal,
    direction_epsilon: Decimal,
) -> Result<Self, forecast::tcn::TcnForecasterError> {
    let forecaster = TcnSyncForecaster::load_bs2()?.with_direction_epsilon(direction_epsilon);
    Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
}
```

Both pairs (audit + non-audit) are additive and gated on existing
features (`forecast` / `forecast-audit-tick`). The 4 existing
`with_tcn_bs{1,2}{,_ledger}` builders stay byte-identical (`dec!(0.6)`
literal pass-through).

Anchor-byte-safety: a unit test at T-D-N6 invokes
`with_tcn_bs1` and `with_tcn_bs2` (NOT `_tuned`) → asserts the
constructed strategy's `confidence_threshold` field equals
`dec!(0.6)`. Tester at M-FINAL runs the predecessor anchor
backtest under default invocation; body-SHA stable.

#### D-AR-1.g — `TcnSyncForecaster::with_direction_epsilon` (NEW, additive)

The deadband ε currently ships as the CONST
[`pub const DIRECTION_EPSILON: f32 = 0.000_5_f32`](../../crates/forecast/src/tcn.rs#L653)
at `crates/forecast/src/tcn.rs:653`, consumed at line 938 by
`r_hat_to_direction(r_hat, DIRECTION_EPSILON)` AND at line 305-307 of
`crates/strategy/src/tcn_overlay_momentum.rs::TcnSyncForecaster::infer`.
The const stays byte-identical (load-bearing for default callers); a
NEW `direction_epsilon: Option<f32>` field gets added to
`TcnSyncForecaster`:

```rust
// crates/strategy/src/tcn_overlay_momentum.rs (additive)
#[cfg(feature = "forecast")]
pub struct TcnSyncForecaster {
    forecaster: forecast::tcn::TcnForecaster,
    direction_epsilon: Option<f32>,  // NEW; None ⇒ use forecast::tcn::DIRECTION_EPSILON
}

#[cfg(feature = "forecast")]
impl TcnSyncForecaster {
    pub fn with_direction_epsilon(mut self, eps: Decimal) -> Self {
        use rust_decimal::prelude::ToPrimitive;
        self.direction_epsilon = Some(eps.to_f32().unwrap_or(forecast::tcn::DIRECTION_EPSILON));
        self
    }
}
```

The `load_bs1` / `load_bs2` constructors initialise `direction_epsilon:
None`. The `infer()` body at lines 305-307 reads:

```rust
let eps = self.direction_epsilon.unwrap_or(forecast::tcn::DIRECTION_EPSILON);
let direction = if r_hat > eps {
    ForecastDirection::Up
} else if r_hat < -eps {
    ForecastDirection::Down
} else {
    ForecastDirection::Flat
};
```

`None` branch is the existing const-load path → existing default
callers see ZERO behavioral change (compiler can const-fold the
`unwrap_or` at the call site). The 26 predecessor anchors stay
byte-identical by construction (R8 / K4).

Rejected alternatives:

- **Replace `DIRECTION_EPSILON` const with a runtime config** —
  rejected. The const is read at `tcn.rs:938` in `TcnForecaster::infer`
  too; replacing it would force a strategy → forecaster wiring change
  that flips a load-bearing tile of the inference path. Additive
  override is cheaper.
- **Thread ε through `combine_with_direction()` instead of the
  forecaster** — rejected. The ε at `combine_with_direction` would
  filter the COMBINED direction (after agree/disagree resolution);
  but per the feature.md § Why discussion, ε is sized to the raw
  `r_hat` magnitude (gate denominator → flat direction). Placement at
  the forecaster's `r_hat → direction` decision is semantically
  correct AND matches the existing const-site at tcn.rs:938.

#### D-AR-1.h — Heatmap report shape

Path:
```
spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md
```

Frontmatter (advisory; NOT hashed) — mirrors ADR-0033 § D2.a
discipline:

```yaml
---
slug: v25-tcn-threshold-tuning
scenario: threshold-sweep-bs1-realdata-recalibrated   # or bs2
generated: 2026-05-21T12:34:56Z          # advisory
wall_clock_s: 1842.5                     # advisory (45 cells × ~25-40s each)
host: <hostname>                         # advisory
git_commit: <40 hex>                     # advisory
model_revision: d1c3696d…                # 64 hex (unchanged from recalibrate ship)
sigma_train_recalibrated: 0.018015573    # %.9f
data_revision_sha: 3a8b96c4…             # v2.6.0-realdata pin
baseline_anchor_bs1: top10-2023-fy-tcn-overlay-realdata   # the v1-momentum-default-cell reference body
baseline_sha_bs1: 8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642
default_cell_anchor: top10-2023-fy-tcn-overlay-weights-realdata  # the τ=0.6+ε=0.0005 reference body
default_cell_sha: 552d7df294bc93ff6f887874f919aeeb8106a62caae4ad5ec5de7c5b49665d70
verdict: T-NO-ALPHA                       # advisory mirror of body verdict (filled at render time)
---
```

Body (deterministic; HASHED by the anchor):

```markdown
# Threshold sweep — BS-1 (realdata, recalibrated σ_train)

## Inputs

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Anchor scenario   | bs1                                            |
| model_revision    | d1c3696d…  (UNCHANGED — weights byte-identical) |
| weights_sha256    | 4ed9064a…  (UNCHANGED)                          |
| σ_train (recal)   | 0.018015573                                    |
| Training span     | 2023-01-01T00:00:00Z .. 2023-12-31T23:00:00Z   |
| Eval span         | 2023-01-01T00:00:00Z .. 2023-12-31T23:00:00Z   |
| Data revision SHA | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7 |
| Cells             | 45 (9 τ × 5 ε)                                 |
| Bar count / cell  | 87600                                          |

## Baseline references

| Field                     | Value           |
|---------------------------|-----------------|
| v1 momentum baseline      | `top10-2023-1h-momentum` @ SHA `3b60ef07…` |
| v1 Sharpe (ann.)          | <%.6f read from baseline body> |
| v1 Sortino (ann.)         | <%.6f> |
| v1 Calmar                 | <%.6f> |
| v1 max drawdown           | <%.2f%%> |
| v1 total return           | <%.2f%%> |
| TCN-overlay default cell  | `top10-2023-fy-tcn-overlay-weights-realdata` @ SHA `552d7df2…` |
| default-cell Sharpe       | <%.6f> |
| default-cell total return | <%.2f%%> |

Pre-feature defaults: τ=0.600000, ε=0.000500. Read from anchored
body, NOT re-computed. Per-cell deltas signed against the v1 momentum
Sharpe.

## Heatmap A — Sharpe (ann.) delta vs v1 momentum

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|----------|
| 0.100000    | <%+.6f>  | <%+.6f>  | <%+.6f>  | <%+.6f>  | <%+.6f>  |
| 0.200000    | …        | …        | …        | …        | …        |
| …           | …        | …        | …        | …        | …        |
| 0.900000    | …        | …        | …        | …        | …        |

## Heatmap B — Total return delta vs v1 momentum (percentage points)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|----------|
| 0.100000    | <%+.2f%%>| …        | …        | …        | …        |
| …           | …        | …        | …        | …        | …        |

## Heatmap C — Max drawdown (absolute value per cell)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|----------|
| 0.100000    | <%.2f%%> | …        | …        | …        | …        |
| …           | …        | …        | …        | …        | …        |

## Heatmap D — Gate-survivor count (collapsed to 1-D row over τ; ε-invariant)

| τ           | Gate survivors (of 87600) |
|-------------|---------------------------|
| 0.100000    | <%d>                      |
| 0.200000    | <%d>                      |
| …           | …                         |
| 0.900000    | <%d>                      |

(Read from the predecessor's anchored
`forecast-distribution-bs1-realdata-recalibrated-20260521.md` body —
`confidence_gate_survival` row — NOT re-computed.)

## Headline cell

| Field              | Value                |
|--------------------|----------------------|
| arg-max(τ, ε)      | (<%.6f>, <%.6f>)     |
| Sharpe delta       | <%+.6f>              |
| Total return delta | <%+.2f%%>            |
| Max drawdown       | <%.2f%%>             |
| Sharpe (cell)      | <%.6f>               |
| Sortino (cell)     | <%.6f>               |
| Calmar (cell)      | <%.6f>               |
| Total return (cell)| <%.2f%%>             |
| Trades (cell)      | <%d>                 |
| Dampen rate (cell) | <%.2f%%>             |

## Smoothness statistic

| Field                        | Value       |
|------------------------------|-------------|
| Sharpe-delta range           | <%.6f>      |
| max(|cell − 8-neighbour|)    | <%.6f>      |
| Smoothness ratio             | <%.6f>      |
| H2 verdict                   | confirmed | falsified |

Per feature.md § H2 — smoothness ratio ≤ 0.25 ⇒ H2 confirmed; > 0.25
⇒ H2 falsified (operator routes to analyst triage regardless of
T-verdict).

## Verdict

T-classifier per feature.md § R3:

- `T-ALPHA-UNLOCKED` ⇔ max-cell Sharpe delta ≥ +0.10
- `T-MARGINAL`       ⇔ max-cell Sharpe delta ∈ [0.0, +0.10)
- `T-NO-ALPHA`       ⇔ max-cell Sharpe delta < 0

This cell: **<T-NO-ALPHA | T-MARGINAL | T-ALPHA-UNLOCKED>**.

(Advisory verdict — does NOT amend ADR-0033 § D3 F-verdict algorithm
per Q4=(c). The F-verdict for this checkpoint remains F4 per the
predecessor's anchored
`forecast-distribution-bs1-realdata-recalibrated` body.)

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…safetensors`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.json`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.recalibrated.json`.
- σ_train value sourced from `--metadata-path` overlay (ADR-0035 D3).
- Backtest seed fixed at `0xC0FFEE` per ADR-0032 § D4.
- Cell ordering: lexicographic by (τ, ε) — NOT completion order
  (R9 / K3 invariant; D-AR-1.j parallelism contract).
- Sharpe / Sortino / Calmar / drawdown formulas inherit from
  `crates/forecast/src/bin/sharpe_comparison.rs::metrics`.
```

Floating-point format rules (locked here to forestall K3 drift):

| Field family                    | Format                         |
|---------------------------------|--------------------------------|
| τ, ε, σ_train                   | `format!("{:.6}", x)` (6 decimals) |
| Sharpe / Sortino / Calmar       | `format!("{:.6}", x)` (6 decimals) |
| Sharpe delta                    | `format!("{:+.6}", x)` (signed, 6 decimals) |
| Return / drawdown / dampen rate | `format!("{:.2}%", x*100.0)` (2 decimals, %) |
| Return / drawdown delta         | `format!("{:+.2}%", x*100.0)` (signed, 2 decimals, %) |
| Trade counts                    | `format!("{}", x)` (integer)   |
| Bar counts                      | `format!("{}", x)` (integer)   |
| Gate-survivor counts            | `format!("{}", x)` (integer)   |
| Smoothness ratio                | `format!("{:.6}", x)` (6 decimals) |

ASCII-only, LF-only line endings, fixed-precision floats — inherit
ADR-0033 § D2.a canonicalisation contract.

#### D-AR-1.i — T-classifier thresholds (Q4=(c))

The 3-label classifier embeds in the body's `## Verdict` section
(NOT as a new ADR per Q4=(c)). Thresholds (confirmed at M-T1 from
analyst-recommended defaults in feature.md § R3):

```rust
const T_ALPHA_UNLOCKED_FLOOR: f64 = 0.10;  // Sharpe-delta units (annualised)
const T_NO_ALPHA_CEILING: f64 = 0.0;       // Sharpe-delta units (signed)

fn t_classifier(max_sharpe_delta: f64) -> &'static str {
    if max_sharpe_delta >= T_ALPHA_UNLOCKED_FLOOR {
        "T-ALPHA-UNLOCKED"
    } else if max_sharpe_delta >= T_NO_ALPHA_CEILING {
        "T-MARGINAL"
    } else {
        "T-NO-ALPHA"
    }
}
```

Joint verdict (computed externally — at presenter / tester time, NOT
in this bin):

| BS-1 verdict       | BS-2 verdict       | Joint               |
|--------------------|--------------------|---------------------|
| T-ALPHA-UNLOCKED   | T-ALPHA-UNLOCKED   | T-ALPHA-UNLOCKED    |
| T-ALPHA-UNLOCKED   | T-MARGINAL / T-NO-ALPHA | T-ALPHA-MIXED  |
| T-MARGINAL / T-NO-ALPHA | T-ALPHA-UNLOCKED | T-ALPHA-MIXED     |
| T-MARGINAL         | T-MARGINAL         | T-MARGINAL          |
| T-NO-ALPHA         | T-NO-ALPHA         | T-NO-ALPHA          |
| any other mismatch | —                  | T-MIXED             |

The bin emits the per-checkpoint label only; the joint label appears
in the M-FINAL tester report and the M-PRESENTER deck (NOT in the
heatmap body, which is per-checkpoint).

#### D-AR-1.j — Parallelism + determinism contract (R9 / K3)

**Per-cell parallelism: PERMITTED, BUT order-invariant assembly.**

```rust
// Pseudocode for the cell-execution loop (sweep bin main()).
// SAFE: rayon::par_iter over the 45 cells.
let cells: Vec<(f32, f32, TcnOverlayRunResult)> = (0..45)
    .into_par_iter()
    .map(|idx| {
        let tau = TAU_GRID[idx / 5];
        let eps = EPSILON_GRID[idx % 5];
        // Each cell independently:
        //   1. construct strategy via with_tcn_bs{1,2}_tuned(τ, ε)
        //   2. clone the input bars (Vec<Bar> is Clone; cheap-ish)
        //   3. call backtest::scenarios::threshold_sweep::run_cell(...)
        //   4. return (τ, ε, result)
        (tau, eps, run_cell_blocking(tau, eps, &shared_bars, &shared_input))
    })
    .collect();
// Sort BEFORE rendering — guarantees identical body across runs.
let mut cells_sorted = cells;
cells_sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
```

Determinism invariants the parallelism MUST preserve:

1. **Per-cell seed is identical** — `seed = 0xC0FFEE` (ADR-0032 § D4).
   Per-cell RNG paths are within `backtest::scenarios::threshold_sweep::run_cell`;
   each cell is a fresh strategy instance + fresh engine state → no
   cross-cell state leakage.
2. **Bars are shared read-only** — `Arc<Vec<Bar>>` (or clone if cheaper).
   Each cell sees the byte-identical bar slice; no mutation.
3. **Forecaster is cloned per cell** — `TcnForecaster` is NOT Clone (it
   owns `candle_core::Tensor` handles). The strategy constructs a
   FRESH forecaster per cell via `load_bs{1,2}` → load-from-disk cost
   ~150-300ms × 45 cells = ~7-14s extra wall-clock. Acceptable for the
   determinism guarantee. (Alternative: refactor `TcnForecaster` to
   support `Arc<...>` shared handles — out of scope for this feature.)
4. **Cell assembly order** — sort by `(τ, ε)` lexicographic BEFORE
   render. Render is a pure function of the sorted Vec → byte-identical
   body across runs regardless of execution order.

Tester at M-FINAL runs the 2-run byte-identity gate per R9:

```bash
cargo run -p forecast --features candle --bin threshold_sweep -- \
    --scenario bs1 \
    --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json
# Hash the body
python3 scripts/hash_report.py spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md
# Re-run; hash again; SHAs must match.
```

**Wall-clock estimate** (M3 Mac Studio, 4-way parallel):

- BS-1: 45 cells, ~25-40s each → ~5-8min total at 4-way parallel.
- BS-2: 45 cells, ~25-40s each → ~5-8min.
- Total: ~10-16min for both checkpoints. (vs. analyst's 12-min 4-way
  estimate — matches.)

### T-AR-2 — Report shape lock

Locked in D-AR-1.h above. Two reports:

- `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md`
- `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md`

Joint-verdict + headline-cell table goes in the tester report at
M-FINAL (NOT in the per-checkpoint heatmap report bodies — keeps the
checkpoint reports independently anchorable).

### T-AR-3 — Parallelism map

Locked in D-AR-1.j. 4-way `rayon::par_iter` over 45 cells; deterministic
via `(τ, ε)`-sorted assembly. The architect rejects shell-out-parallel
(spawn 4 `backtest` processes) because in-process avoids per-shell-out
load-model + load-bars overhead.

### T-AR-4 — Tuned builders — `_tuned` always-explicit args

Locked in D-AR-1.f. The new builders require explicit `(τ, ε)` —
**no default-arg overloading**, no `Option<Decimal>` cascading defaults.
Anchor-byte-safety guaranteed by the existing
`with_tcn_bs{1,2}_ledger` keeping their `dec!(0.6)` literal pass-through
unchanged.

### T-AR-5 — Anchor strategy (Q6=(a))

Locked. Two new heatmap anchors land at M-FINAL:

| Anchor name | Version | Path |
|-------------|---------|------|
| `threshold-sweep-bs1-realdata-recalibrated` | `v2.6.2-threshold-tuning` | `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md` |
| `threshold-sweep-bs2-realdata-recalibrated` | `v2.6.2-threshold-tuning` | `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md` |

**No per-cell tuned-winner anchors at M-FINAL.** Per feature.md § R4
+ § R7, if `T-ALPHA-UNLOCKED` fires, the tuned-cell backtest report
is **the operator's promotion artefact for a follow-on v2.5.1** —
not a M-FINAL deliverable of this feature. The reason: the H1 outcome
is empirical; the analyst-recommended +0.10 Sharpe floor may not bind
on the first sweep. Adding the tuned-winner anchor only if `T-ALPHA-UNLOCKED`
keeps the M-FINAL gate uniform across all 3 outcomes (T-NO-ALPHA,
T-MARGINAL, T-ALPHA-UNLOCKED).

If `T-ALPHA-UNLOCKED` fires, the follow-on v2.5.1 feature
(`v25-tcn-tuned-promotion`, not yet authored) locks the tuned-winner
anchors (`top10-2023-fy-tcn-overlay-realdata-tuned-bs1-{τ*,ε*}` etc.)
under `v2.6.2-threshold-tuning` or `v2.6.3-tuned-promotion`. Architect
authors that feature only after this feature's M-FINAL outcome.

Anchor count progression:

- Pre-feature: 26 (recalibrate ship's lock).
- Post-feature (all outcomes): 28 (just the 2 sweep heatmaps).

### T-AR-6 — Spike requirement: NONE

Architecture is straightforward — extend the existing
`forecast_distribution.rs` τ-sweep precedent + the existing
backtest scenario contract. No new external crate, no experimental
API. The `rayon` dep is already in the workspace (used by
`crates/forecast` for some training paths).

If the developer hits an unexpected at T-D-N3 (e.g. the per-cell
backtest determinism breaks under `par_iter` despite the sort-by-
key safeguard), **escalate back to architect, do NOT band-aid in
Wave A**. The fall-back is sequential execution (45 cells × ~30s
≈ 22min single-threaded per checkpoint).

## 2. Module/file change-map

| Path | Action | Lines (est) | Notes |
|------|--------|-------------|-------|
| `crates/forecast/src/bin/threshold_sweep.rs` | **NEW** | ~450 | New bin — D-AR-1.a..D-AR-1.j. |
| `crates/forecast/Cargo.toml` | **MODIFY** | +5 | `[[bin]]` entry mirroring existing `recalibrate_sigma_train` block. NO new external deps. |
| `crates/forecast/src/tcn.rs` | **NO CHANGE** | 0 | `DIRECTION_EPSILON` const stays byte-identical (R7 / K4 / K5). |
| `crates/strategy/src/tcn_overlay_momentum.rs` | **MODIFY** | +90 | Add 4 additive builders: `with_tcn_bs{1,2}_tuned(τ, ε)` (feature `forecast`) + `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` (feature `forecast-audit-tick`) per D-AR-1.f. Add `with_direction_epsilon` builder + `direction_epsilon: Option<f32>` field on `TcnSyncForecaster` per D-AR-1.g. Existing 4 builders byte-identical. |
| `crates/strategy/src/tcn_overlay_momentum.rs::TcnSyncForecaster::infer` | **MODIFY** | +2 | Change `if r_hat > forecast::tcn::DIRECTION_EPSILON` → read `let eps = self.direction_epsilon.unwrap_or(forecast::tcn::DIRECTION_EPSILON);` then `if r_hat > eps`. Lines 305-307. Default path (`None`) is const-fold-identical to existing. |
| `crates/backtest/src/scenarios/threshold_sweep.rs` | **NEW** | ~110 | Thin `run_cell` helper per D-AR-1.c. Behavior-preserving extraction of `tcn_overlay_weights::run` with custom-strategy hook. |
| `crates/backtest/src/scenarios/mod.rs` | **MODIFY** | +2 | `pub mod threshold_sweep;` |
| `crates/strategy/tests/tcn_overlay_tuned_builder.rs` | **NEW** | ~120 | Unit tests for D-AR-1.f + D-AR-1.g — assertion that `with_tcn_bs1` constructs `confidence_threshold = dec!(0.6)`; `with_tcn_bs1_tuned(τ, ε)` constructs the supplied (τ, ε); the `_tuned` builders set `direction_epsilon = Some(ε)`; the existing `with_tcn_bs1` builder sets `direction_epsilon = None`. |
| `crates/forecast/tests/threshold_sweep_readonly.rs` | **NEW** | ~120 | Mirror of `recalibrate_sigma_train_readonly.rs`: help-surface assertions (no `--retrain`/`--write-*` substrings) + checkpoint mtime guard + report-output-path assertion. |
| `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md` | **NEW** (developer-emitted, Wave B) | ~250 | The BS-1 heatmap report. |
| `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md` | **NEW** (developer-emitted, Wave B) | ~250 | The BS-2 heatmap report. |
| `spec/v25-tcn-threshold-tuning/feature.md` | **MODIFY** | +120 | Append `## Design` section that cross-points back to this decomp.md. Flip frontmatter `status: proposed → in-progress`, `owner: architect → developer`. |
| `spec/v25-tcn-threshold-tuning/tasks.md` | **MODIFY** | +180 | Tick T-AR-1..T-AR-6 with file:line + cargo invocation + literal output. Append T-D-N1..T-D-N9 + T-T-1.a..T-T-1.f rows. Flip frontmatter `owner: developer`, `status: in-progress`. |
| `spec/trace.toml` | **MODIFY** | +6 | Flip `REQ-V25-TCN-THRESHOLD-TUNING-001` state `proposed → in-progress`. Populate `arch = [...]`. |
| `spec/anchors.toml` | **NO CHANGE in M-T1** | — | Tester adds 2 rows at M-FINAL T-T-1.b under `v2.6.2-threshold-tuning`. 26 originals byte-identical. |
| ADR-0036 | **NOT WRITTEN** (Q4=(c) closure) | 0 | Embedded in report body only. Rationale: 1-line note in feature.md § Design. |

**Anchor neutrality** (R7 / R8): every NEW file path is non-anchored.
Every MODIFY file is either spec-only (`feature.md`, `tasks.md`,
`trace.toml`) or non-anchored code with additive-only semantics
(`crates/strategy/src/tcn_overlay_momentum.rs` adds builders +
`Option<f32>` field; `infer()` change is `unwrap_or(CONST)` = const-fold
identical for default callers). The unit test at T-D-N6 explicitly
asserts the default-builder byte-identity at the type level.

## 3. Wave A–C ordered decomposition

```
                Wave A  ──────────────────►   Wave B  ──────────────►   Wave C
       (builder + bin + helper + tests)     (run 90 backtests)        (M-FINAL gate)
                  developer                     orchestrator              tester
```

### Wave A — `_tuned` builders + `threshold_sweep` bin (developer)

| Row | Description | File:line | Cargo invocation | Expected literal output |
|-----|-------------|-----------|------------------|--------------------------|
| **T-D-N1** | `with_direction_epsilon` builder + `direction_epsilon: Option<f32>` field on `TcnSyncForecaster`. `infer()` body change at lines 305-307 → `let eps = self.direction_epsilon.unwrap_or(forecast::tcn::DIRECTION_EPSILON);`. Lines 158-214 (struct + impl block) get +1 field + ~10 lines. | `crates/strategy/src/tcn_overlay_momentum.rs:158-214,305-307` (modify) | `cargo build -p strategy --features forecast` | `Compiling strategy …` followed by `Finished … profile [optimized] target(s) in <Ns>` — no warnings. |
| **T-D-N2** | 4 `_tuned` builders (D-AR-1.f). Additive after `with_tcn_bs{1,2}_ledger` at line 441. | `crates/strategy/src/tcn_overlay_momentum.rs:441+` (append; new lines 441-530) | `cargo build -p strategy --features forecast,forecast-audit-tick` | `Compiling strategy …` `Finished … in <Ns>`. |
| **T-D-N3** | Unit tests for builder default-invariance + tuned-value-passthrough. 5 tests: (1) `with_tcn_bs1.confidence_threshold == dec!(0.6)`; (2) `with_tcn_bs1.direction_epsilon == None`; (3) `with_tcn_bs1_tuned(τ, ε).confidence_threshold == τ`; (4) `with_tcn_bs1_tuned(τ, ε).direction_epsilon == Some(ε.to_f32())`; (5) ditto for BS-2. | `crates/strategy/tests/tcn_overlay_tuned_builder.rs:1-120` (new) | `cargo test -p strategy --features forecast --test tcn_overlay_tuned_builder` | `running 5 tests … test result: ok. 5 passed; 0 failed` |
| **T-D-N4** | Bin skeleton + CLI surface (D-AR-1.d). Mirrors `recalibrate_sigma_train.rs:1-120`. Add `[[bin]]` to `crates/forecast/Cargo.toml`. | `crates/forecast/src/bin/threshold_sweep.rs:1-130` (new) | `cargo run -p forecast --features candle --bin threshold_sweep -- --help` | Help text containing `--scenario`, `--data-root`, `--metadata-path`, `--out-dir`, `--expected-revision-sha`; NO `retrain`/`update`/`write-checkpoint`/`write-metadata` substrings. |
| **T-D-N5** | Thin `run_cell` helper in `crates/backtest/src/scenarios/threshold_sweep.rs` (D-AR-1.c). Behavior-preserving copy of `tcn_overlay_weights::run` with caller-supplied strategy. Re-uses `momentum::top10_symbols_with_prices` + the realdata bar loader. | `crates/backtest/src/scenarios/threshold_sweep.rs:1-110` (new) + `crates/backtest/src/scenarios/mod.rs:+1` | `cargo build -p backtest --features candle,realdata` | `Compiling backtest …` `Finished … in <Ns>`. |
| **T-D-N6** | Bin body — grid enumeration + parallel cell execution (D-AR-1.e + D-AR-1.j). Uses `rayon::par_iter` over 45 cells; cells loaded fresh per iteration; sort-by-(τ,ε) before render. | `crates/forecast/src/bin/threshold_sweep.rs:130-300` (new) | `cargo build -p forecast --features candle --bin threshold_sweep` | `Compiling forecast …` `Finished … in <Ns>` — no warnings. |
| **T-D-N7** | Heatmap renderer (D-AR-1.h). Reads pre-recal v1-momentum baseline metrics from anchored body `spec/v1-cross-sectional-momentum/reports/backtest-*-top10-2023-1h-momentum.md` (or wherever the v1 reports live; resolves via the anchor name). Reads gate-survivor counts from anchored predecessor body `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`. Emits the markdown report. | `crates/forecast/src/bin/threshold_sweep.rs:300-450` (new) | `cargo run -p forecast --features candle,realdata --bin threshold_sweep -- --scenario bs1 --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json` (note: ~8-12min wall-clock at 4-way; watch recipe required) | `INFO threshold_sweep: <scenario=bs1, cells=45, headline=(τ*,ε*), sharpe_delta=<f64>, verdict=T-<LABEL>, wall_clock_s=<f64>>` AND markdown file at `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md`. |
| **T-D-N8** | Read-only enforcement test. 1× test: help surface has no forbidden flags. 1× test: `tcn-bs{1,2}-<sha>.{safetensors,metadata.json,metadata.recalibrated.json}` mtimes unchanged after a sweep run (mirror `recalibrate_sigma_train_readonly.rs`). | `crates/forecast/tests/threshold_sweep_readonly.rs:1-120` (new) | `cargo test -p forecast --features candle,realdata --test threshold_sweep_readonly` | `running 2 tests … test result: ok. 2 passed; 0 failed` |
| **T-D-N9** | `--features candle,realdata,forecast,forecast-audit-tick` workspace clippy + fmt gate. | (workspace-wide) | `cargo fmt --check` + `cargo clippy --workspace --features candle,realdata,forecast,forecast-audit-tick -- -D warnings` | Both commands exit 0; clippy output ends `Checking … Finished` with no warnings. |

**Wave A acceptance** = T-D-N1..T-D-N9 ticked + both
`threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md` files
on disk + the 26 predecessor anchor bodies byte-identical (run
`bash scripts/verify_anchors.sh` mid-Wave-A and post-Wave-A;
26 entries PASS in the same shape as the M-T1 baseline § 6).

### Wave B — Orchestrator: anchor verification + tester handoff prep

| Row | Description | Path | Cargo / shell invocation | Expected literal output |
|-----|-------------|------|--------------------------|--------------------------|
| **T-D-N10** | Orchestrator confirms BS-1 + BS-2 sweep reports exist + body-SHAs are stable across 2 runs (determinism gate prep). | `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md` | `python3 scripts/hash_report.py spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md` (twice; SHAs must match) | `<64hex>  …threshold-sweep-bs1-realdata-recalibrated-20260521.md` literal-stable across 2 runs. |
| **T-D-N11** | Orchestrator runs `bash scripts/verify_anchors.sh` POST-Wave-A. Captures literal output. | (script) | `bash scripts/verify_anchors.sh` | The 24 PASS rows for the 26-anchor baseline stay byte-identical (modulo the 2 pre-existing FAILs from the `forecast-distribution-bs{1,2}-realdata` glob collision documented at § 6 — those are not introduced by this feature; orchestrator surfaces them to spec-auditor punch-list separately). |

### Wave C — M-FINAL tester gate

| Row | Description | Path | Cargo / shell invocation | Expected literal output |
|-----|-------------|------|--------------------------|--------------------------|
| **T-T-1.a** | 2-run byte-identity determinism on both heatmap reports + on the 4 predecessor recalibrate-ship anchored bodies (regression-safety). | (heatmap files) | `python3 scripts/hash_report.py spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md` (each twice; SHAs match) | `<64hex>  threshold-sweep-bs1-realdata-recalibrated-20260521.md` literal-stable. |
| **T-T-1.b** | Anchor-additive lock: append 2 rows to `spec/anchors.toml` under version `v2.6.2-threshold-tuning`. | `spec/anchors.toml:199+` (append) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (28 / 28)` (assumes pre-existing glob-collision FAILs at § 6 are first triaged + fixed; otherwise `ANCHORS FAIL` with 2 spurious FAILs + 26 PASS rows). |
| **T-T-1.c** | Anchor-neutrality check: all 26 originals body-SHA byte-identical to baseline. | `spec/anchors.toml` rows for 26 pre-feature anchors | `bash scripts/verify_anchors.sh | grep -E "^(PASS|FAIL)"` | All 26 lines match the M-T1 baseline captured in tasks.md T-AR-4 § baseline (modulo the § 6 pre-existing FAILs). |
| **T-T-1.d** | Joint T-verdict recorded in `feature.md § Verification`. Routing decision per § R3 table. | `spec/v25-tcn-threshold-tuning/feature.md § Verification` (append) | (manual) | `Joint verdict: T-<LABEL>` + `Operator routing: <decision>` lines added to feature.md. |
| **T-T-1.e** | Trace row flipped to `shipped`. `crates`, `tests`, `anchors` columns populated. | `spec/trace.toml:222-231` | `python3 scripts/spec_brief.py v25-tcn-threshold-tuning --check-trace` (or manual `grep`) | `state = "shipped"` + non-empty arrays. |
| **T-T-1.f** | Tester report under `spec/v25-tcn-threshold-tuning/reports/test-<YYYYMMDD-HHMM>-v25-tcn-threshold-tuning.md` per `.claude/skills/rust-test/templates/test-report.md`. Carries the 26/26 (PRE-feature) → 28/28 (POST-feature) anchor-progression line literal. | `spec/v25-tcn-threshold-tuning/reports/test-<YYYYMMDD-HHMM>-v25-tcn-threshold-tuning.md` (new) | `bash scripts/verify_anchors.sh` (quote literal in report body) | Tester report cites `ANCHORS PASS  (28 / 28)` as the post-lock literal. |

**Wave C acceptance** = T-T-1.a..T-T-1.f ticked + tester emits
`VERDICT → PASS` envelope to presenter (M-FINAL close).

### Parallelism map

```
Wave A:
  T-D-N1 → T-D-N2 → T-D-N3                          (sequential — builders first, then tests)
                                ↘
                                  T-D-N4 (parallel-after-T-D-N1; bin CLI surface; no builder dep)
                                  T-D-N5 (parallel-after-T-D-N1; backtest helper; no builder dep)
  T-D-N6 (after T-D-N2 + T-D-N4 + T-D-N5)           (bin body wires it all)
  T-D-N7 (after T-D-N6)                             (run the actual 90 backtests; long-running)
  T-D-N8 (parallel-after-T-D-N4)                    (read-only test on the bin CLI; no run needed)
  T-D-N9 (after T-D-N7 + T-D-N8)                    (workspace-wide clippy/fmt gate)

Wave B:
  T-D-N10 (sequential after T-D-N7)
  T-D-N11 (parallel-after-T-D-N7)

Wave C:
  T-T-1.a (BS-1 + BS-2 + 4 predecessor parallel; 6-fan-out)
  T-T-1.b → T-T-1.c (sequential after T-T-1.a)
  T-T-1.d → T-T-1.e → T-T-1.f (sequential)
```

Critical path: **T-D-N1 → T-D-N2 → T-D-N6 → T-D-N7 → T-T-1.b → T-T-1.f**.
Wall-clock estimate ~3-4 hours (analyst's § Cost estimate budget of
~6-10h is conservative — most of the time is the 2 × ~8-12min sweep
runs at T-D-N7).

### Watch recipe for the long-running rows

T-D-N7 invocations are each ~8-12min at 4-way parallel (45 cells × ~25-40s
each, 4 in flight). The orchestrator MUST kick each off in background
and probe via:

```bash
# Background the BS-1 sweep:
cargo run -p forecast --features candle,realdata --release --bin threshold_sweep -- \
    --scenario bs1 \
    --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json \
    > /tmp/threshold-sweep-bs1.log 2>&1 &

# Probe progress every 30s:
watch -n 30 'tail -n 60 /tmp/threshold-sweep-bs1.log; \
             echo "---"; \
             ls -la spec/v25-tcn-threshold-tuning/reports/ 2>/dev/null'
```

Then do the same with `--scenario bs2` after BS-1 finishes (or in
parallel if memory allows — each cell loads ~5MB of safetensors +
~30MB of bar data; 4 + 4 = 8-way parallel needs ~280MB which is fine
on the M3 Mac Studio).

## 4. Spike requirement

**NONE.** The analyst pinned every API the bin needs to exact file:line
in feature.md § Sources cited. The new
[`TcnSyncForecaster::with_direction_epsilon`](../../crates/strategy/src/tcn_overlay_momentum.rs)
builder is a 5-line additive pattern. The 4
`with_tcn_bs{1,2}{,_ledger}_tuned` builders are copy-paste-adapt of
the 4 existing builders at lines 379-440.

The `rayon::par_iter` precedent ships in `crates/forecast/src/bin/train_tcn.rs`
(check that — at least the candle-side already pulls `rayon` indirectly
via `candle-core`). If `rayon` is not a direct dep of `forecast`, add it
under `[dependencies]` (already in the workspace transitively → zero
new external crate per CLAUDE.md "no new external crate deps").

## 5. Rollback shape per wave

Every wave is independently revertable. The additive-only contract
on the strategy builders is the load-bearing reason this is true:

| Wave | Rollback action | Cost |
|------|-----------------|------|
| **A** | Delete the 4 new `_tuned` builders + the `direction_epsilon` field/builder + the new bin + the new tests + the `crates/backtest/src/scenarios/threshold_sweep.rs` helper. Revert `infer()` line 305-307 to use the const directly. Existing 4 builders + 26 anchors stay byte-identical (`infer()` change is const-fold-identical for default callers). | ~5 minutes (`git revert <sha>`). |
| **B** | Delete the 2 heatmap reports under `spec/v25-tcn-threshold-tuning/reports/`. Existing 26 anchors stay byte-identical (those reports were only ever new files). | ~1 minute (`rm` + `git revert <sha>`). |
| **C** | Revert the 2 new rows in `spec/anchors.toml`. The 26 originals were byte-identical the whole time → no migration. Revert the `trace.toml` state-flip + the `feature.md § Verification` block. | ~3 minutes (`git revert <sha>`). |

**Full-feature rollback** = `git revert` the wave commits + `rm` the
2 heatmap reports. Original 26 anchors stay byte-identical throughout
(R7 / R8 hard invariant).

## 6. Anchor neutrality baseline

The M-T1 baseline captured at architect-spawn time (2026-05-21):

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -3
PASS  recalibrate-sigma-train-bs1           baa658fb7ad96796f643d8fecab9156362b17faad97afc37be77867850336ad9
PASS  recalibrate-sigma-train-bs2           bfa8104ace81dd6a98f42a65cd0a5bd584089fa93fbafa4aa6f11d02954b47e0
---
ANCHORS FAIL  (mismatches detected; route HANDOFF -> developer with body diff)
```

**Honest disclosure**: `verify_anchors.sh` currently reports
`ANCHORS FAIL` due to **2 pre-existing FAIL lines NOT introduced by
this feature**:

```
FAIL  forecast-distribution-bs1-realdata
      expected ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54
      actual   8a548042f552899cbccfa4d9b8d6eca6306f7de5c1a1bd7ed18201b08a06f80f
      file     /…/spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md
FAIL  forecast-distribution-bs2-realdata
      expected d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06
      actual   d6c1e17ca162469e94b8dacd7c4485ec4d8cd77b6768f9e7ebe2f7deaf4b4151
      file     /…/spec/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md
```

**Root cause** (pre-existing recalibrate-ship glitch, not a feature
issue): the `scripts/verify_anchors.sh` resolver at line 45 globs
`"*/reports/$scenario-*.md"` which for `scenario =
forecast-distribution-bs1-realdata` greedy-matches BOTH
`…realdata-20260519.md` (the predecessor anchor body) AND
`…realdata-recalibrated-20260521.md` (the recalibrate-ship anchor
body) and the newer date wins the `sort | tail -1`. The actual
predecessor file body-SHA is still `ef73cb8d…` (verified directly:
`python3 scripts/hash_report.py spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md`
→ `ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54`).

**Decision**: this is a **spec-auditor punch-list item for the
recalibrate ship's tester** (the glob-resolver collision is a `verify_anchors.sh`
bug; the candidate fix is to add `-realdata.md` end-anchor or
prefer-shortest-suffix). M-T1 architect surfaces this finding to the
orchestrator as an out-of-feature concern. This feature's anchor
contract is **anchor-additive** and **all 26 individual file bodies
stay byte-identical** — `python3 scripts/hash_report.py …` direct
hashes confirm byte-identity for every anchored file.

For Wave C T-T-1.b literal output, the **proper expected literal**
depends on whether the orchestrator triages the glob collision first.
Two acceptable outcomes:

- **If glob fixed first** (recommended): `ANCHORS PASS  (28 / 28)`.
- **If glob NOT fixed** (current state): `ANCHORS FAIL` with same 2
  spurious FAILs + 26 PASS rows (the 26 individual file bodies are
  still byte-identical; tester at M-FINAL can defend the lock with
  direct `hash_report.py` invocations).

Architect flags the glob fix as a parallel concern (spec-auditor
item), NOT a blocker for this feature's Wave A / Wave B / Wave C
sequencing.

## 7. Cross-references

- Analyst feature brief:
  [`spec/v25-tcn-threshold-tuning/feature.md`](feature.md)
- Predecessor recalibrate decomp:
  [`spec/v25-tcn-recalibrate/decomp.md`](../v25-tcn-recalibrate/decomp.md)
- Predecessor recalibrated reports:
  [`spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`](../v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md),
  [`spec/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md`](../v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md)
- F-verdict algorithm (IMMUTABLE per Q4=(c)):
  [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm)
- σ_train recalibration overlay convention:
  [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
- Realdata backtest path + revision pin:
  [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
- Existing τ-sweep precedent (gate-survival):
  [`crates/forecast/src/bin/forecast_distribution.rs:325`](../../crates/forecast/src/bin/forecast_distribution.rs#L325)
- Existing Sharpe / Sortino / Calmar / drawdown formulas:
  [`crates/forecast/src/bin/sharpe_comparison.rs::metrics`](../../crates/forecast/src/bin/sharpe_comparison.rs)
- Existing TCN-overlay default builders (UNCHANGED; literal `dec!(0.6)` pass-through):
  [`crates/strategy/src/tcn_overlay_momentum.rs:413-421,431-440`](../../crates/strategy/src/tcn_overlay_momentum.rs#L413-L440)
- `combine_with_direction` gate body:
  [`crates/strategy/src/tcn_overlay_momentum.rs:552-582`](../../crates/strategy/src/tcn_overlay_momentum.rs#L552-L582)
- ε source-of-truth (CONST UNCHANGED; additive override per D-AR-1.g):
  [`crates/forecast/src/tcn.rs:653`](../../crates/forecast/src/tcn.rs#L653)
- Realdata backtest scenario (template for the new `threshold_sweep::run_cell` helper):
  [`crates/backtest/src/scenarios/tcn_overlay_weights.rs`](../../crates/backtest/src/scenarios/tcn_overlay_weights.rs)
- Trace row: `REQ-V25-TCN-THRESHOLD-TUNING-001` in
  [`spec/trace.toml:222-231`](../trace.toml).

## Changelog

- 2026-05-21 (architect, M-T1): full lock. T-AR-1 (Design),
  T-AR-2 (report shape), T-AR-3 (4-way `rayon` parallelism;
  order-invariant cell assembly), T-AR-4 (explicit-args `_tuned`
  builders; 4 additive builders + `direction_epsilon` field), T-AR-5
  (2 sweep heatmap anchors at M-FINAL; tuned-winner anchors deferred
  to follow-on v2.5.1 if `T-ALPHA-UNLOCKED` fires), T-AR-6 (NO
  spike; NO ADR-0036; embed T-classifier in body per Q4=(c)).
  Anchor-additive contract confirmed (26 originals byte-identical
  at body level; pre-existing `verify_anchors.sh` glob-collision
  flagged as spec-auditor item, NOT introduced by this feature).
  HANDOFF → developer (Wave A first; Waves B-C tester at M-FINAL).
