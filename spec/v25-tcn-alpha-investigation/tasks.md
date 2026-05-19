---
slug: v25-tcn-alpha-investigation
status: shipped
owner: operator
updated: 2026-05-19
---

# Tasks — v2.5 TCN alpha-verdict investigation

> Architect-decomposed T-D-N rows landed 2026-05-18 (see Changelog
> below). M-R-HAT and M-SHARPE are independent under MINIMAL scope and
> can run in parallel after the architect handoff — see § Parallelism
> map for the orchestrator's wave guide.
>
> Scope (operator-decide, resolved 2026-05-18 → MINIMAL):
> - **Minimal (active)** — M0 → M-R-HAT ‖ M-SHARPE → M-FINAL.
> - **Diagnostic (deferred)** — adds M-DIAG between M-R-HAT and M-SHARPE.
> - **Full root-cause-and-fix (deferred)** — adds M-HORIZON before M-FINAL.
>
> M-DIAG and M-HORIZON stay enumerated below for future reactivation
> if M-R-HAT's F-verdict demands; both are inert under MINIMAL.

## Architect rows

- [x] **T-AR-1** (2026-05-18) — § Design landed in `feature.md`.
  Locks (D1) bin placement, (D2) report shape + float canonicalisation,
  (D3) F-verdict algorithm, (D4) Sharpe formulas, (D5) anchor naming.
  ADR-0033 carries full rationale.

- [x] **T-AR-2** (2026-05-18) — M-R-HAT + M-SHARPE decomposed into 10
  T-D-N rows (5 per milestone) below. Each row carries Owner /
  Milestone / Depends on / Blocks / file:line / test cmd / output line /
  acceptance bullets. Each row is independently spawnable per the
  § Parallelism map.

- [x] **T-AR-3** (2026-05-18) — ADR-0033 written
  (`spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md`).
  Cross-refs `REQ-V25-TCN-ALPHA-001`.

## Milestones

- [x] **M0 — Scope-decision gate** (2026-05-18) — Operator answered Q1:
      **MINIMAL SCOPE** (analyst-recommended default). Active buckets:
      (a) M-R-HAT forecast-distribution inspector + (d) M-SHARPE comparison
      table. **Skipped buckets** (will move to separate follow-on features
      if M-R-HAT's R4 verdict demands): M-DIAG (bucket c, checkpoint
      internals) and M-HORIZON (bucket b, multi-horizon retraining).
      Anchor count grows 19 → 21 (R1 histogram) or 22 (R1 + R5 Sharpe table).
      Audit trail: orchestrator AskUserQuestion 2026-05-18 confirmed
      analyst default. HANDOFF → architect.

- [x] **M-R-HAT — Forecast-distribution inspector (bucket a).** See
      T-D-1 … T-D-5 below. Acceptance: two reports on disk for BS-1 and
      BS-2; both bodies byte-identical on a second run (K3 determinism);
      F-verdict label present in `## Verdict` section per ADR-0033 § D3;
      no checkpoint files mutated.

- [ ] **M-DIAG — Checkpoint-internal inspection (bucket c).**
      _Active only under DIAGNOSTIC scope. Skipped under MINIMAL._
      Will materialise as a separate follow-on feature
      (`v25-tcn-checkpoint-internals` or equivalent) if M-R-HAT's
      F-verdict demands. Per ADR-0033 § D3, F1 verdict already
      points to `v25-tcn-retrain`; F2 to `v25-tcn-recalibrate`. M-DIAG
      activates only if those follow-ons need deeper checkpoint-internal
      evidence than M-R-HAT alone provides.

- [x] **M-SHARPE — Sharpe-comparison report (bucket d).** See
      T-D-6 … T-D-10 below. Acceptance: report on disk under
      `sharpe-comparison-realdata-YYYYMMDD.md`; methodology table cites
      sqrt(24·365) annualisation; comparison table rows for the four
      `-realdata` scenarios; honest reading of `dampen rate = 0%` if
      that holds; lock as anchor `sharpe-comparison-realdata` under
      `v2.6.0-alpha-investigation` if body is deterministic on a 2-run
      check, else ship un-anchored with a `## Not anchorable` body
      section explaining the determinism gap.

- [ ] **M-HORIZON — Horizon-bumped re-training pass (bucket b).**
      _Active only under FULL scope. Skipped under MINIMAL._

- [ ] **M-FINAL — Ship gate.** _BLOCKED by pre-existing test failures (see T-T-1 note)._
      - Anchor neutrality (R6): `bash scripts/verify_anchors.sh` →
        `ANCHORS PASS (19/19)` PRE-lock and `21/21` (R1 only) or
        `22/22` (R1 + R5 Sharpe anchor) POST-lock. The 19 originals
        stay byte-identical.
      - Operator verdict recorded in `feature.md § Verification` per
        R-success-criterion 3: a named follow-on feature is queued OR
        an explicit "no-follow-on" disposition is documented. If the
        joint (BS-1 + BS-2) verdict is `F-MIXED`, the disposition is
        "open analyst-spawn for triage."
      - `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
        and any test-suite invariants land green.
      - Tester writes the test report at
        `spec/v25-tcn-alpha-investigation/reports/test-<YYYYMMDD-HHMM>-v25-tcn-alpha-investigation.md`
        per the [tester template](../.claude/skills/rust-test/templates/test-report.md).
      - Trace row `REQ-V25-TCN-ALPHA-001` flips
        `proposed` → `in-progress` → `shipped` and gets its
        `crates`, `tests`, `anchors` columns filled by the tester.

## T-D rows — M-R-HAT (forecast-distribution inspector, 5 rows)

> All rows below are read-only against
> `crates/forecast/checkpoints/anchors/*` per K5. None of the
> acceptance criteria invoke `cargo run -p forecast --bin train_tcn …`.

- [x] **T-D-1** — `forecast_distribution` bin skeleton + CLI surface.
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/forecast_distribution.rs:1`. Test: `cargo build -p forecast --bin forecast_distribution --features candle`. Output: `Finished \`dev\` profile` (clean). `--help` shows 4-flag surface; no retrain/update/write-checkpoint flag.
  Owner: developer. Milestone: M-R-HAT. Depends on: T-AR-2. Blocks:
  T-D-2, T-D-3, T-D-4, T-D-5.
  _file:line_: `crates/forecast/src/bin/forecast_distribution.rs:1`
  (new file, ~80 LoC for skeleton).
  _test_: `cargo build -p forecast --bin forecast_distribution`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - New file `crates/forecast/src/bin/forecast_distribution.rs` with
    `clap::Parser`-derived `Args` matching ADR-0033 § D1.a:
    `--scenario {bs1|bs2}` (required, enum), `--data-root` (default
    `data/binance/`), `--out-dir` (default
    `spec/v25-tcn-alpha-investigation/reports/`), `--span-start`
    (optional, RFC-3339), `--span-end` (optional, RFC-3339).
  - `fn main()` parses args, auto-derives `span` from `--scenario`
    when `--span-start` and `--span-end` are both `None` (BS-1 →
    2023-01-01..2024-01-01, BS-2 → 2024-01-01..2025-01-01), loads
    `TcnForecaster::load_anchor(scenario)` and prints
    `tracing::info!(model_revision = …, sigma_train = …)`. Body of
    iteration + report-write lands in T-D-2.
  - `cargo build -p forecast --bin forecast_distribution` clean.
  - `cargo run -p forecast --bin forecast_distribution -- --help`
    prints the 4-flag surface; no `--retrain`, `--update-sigma`,
    or `--write-checkpoint` flag exists (K5 hard contract).

- [x] **T-D-2** — Forward-pass collection loop.
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/forecast_distribution.rs` (forward-pass loop, lines ~870-940). Test: `cargo build -p forecast --bin forecast_distribution --features candle`. Output: `Finished \`dev\` profile` (clean). Full forward-pass run (87,500+ windows) verified by T-D-5 end-to-end run.
  Owner: developer. Milestone: M-R-HAT. Depends on: T-D-1. Blocks:
  T-D-3, T-D-4.
  _file:line_: `crates/forecast/src/bin/forecast_distribution.rs`
  (forward-pass loop + `r_hat` collection, ~120 LoC).
  _test_: `cargo build -p forecast --bin forecast_distribution --features candle`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - For each of the 10 USDT symbols (`ADAUSDT, AVAXUSDT, BNBUSDT,
    BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT`),
    the bin calls
    `forecast::features::windows_for_symbol(&data_root, symbol, span,
    &cfg)` with `cfg` = the production `FeatureConfig` (same
    `context_bars=256`, `vol_z_lookback` as the training path uses).
  - For each `FeatureWindow`, the bin reshapes the `features` tensor
    to `[1, 5, 256]` and calls `forecaster.model.forward(&x, false)`
    (NOT `forecaster.forecast()` — see ADR-0033 § D1.b). Extracts the
    scalar `r_hat: f32` via `flatten_all().to_vec1::<f32>()`.
  - Accumulates `r_hat` values in a `Vec<f32>` (~87,500 elements for
    BS-1, ~87,500 for BS-2).
  - Emits `tracing::info!` with `(symbol, windows_count)` per symbol
    and `(total_inferences, wall_clock_ms)` at end.
  - Wall-clock on the developer machine MUST be < 90s per scenario
    (matches the K2 budget).
  - No write to `crates/forecast/checkpoints/`; no write to
    `crates/forecast/replay-cache/`; no write outside `--out-dir`.
    Asserted by T-D-5 below.

- [x] **T-D-3** — Statistics module (`hist::Stats`).
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/forecast_distribution.rs` (mod hist, lines ~124-380). Test: `cargo test -p forecast --bin forecast_distribution --features candle -- hist::tests`. Output: `test result: ok. 6 passed; 0 failed`.
  Owner: developer. Milestone: M-R-HAT. Depends on: T-D-1. Blocks:
  T-D-4. _(Independent of T-D-2 — pure-function math; can run in
  parallel with T-D-2.)_
  _file:line_: `crates/forecast/src/bin/forecast_distribution.rs`
  (`mod hist`, ~150 LoC).
  _test_: `cargo test -p forecast --bin forecast_distribution -- hist::tests`.
  _output_: `test result: ok. 6 passed; 0 failed`.
  _acceptance_:
  - Inline `mod hist` with `pub fn summary_stats(r_hat: &[f32])
    -> Stats` (count, mean, std, min, max, p01/p05/p10/p25/p50/p75/p90/p95/p99,
    abs_p50/abs_p95/abs_p99). Percentile algorithm:
    type-7 quantile, sort by `f32::total_cmp`.
  - `pub fn histogram(r_hat: &[f32], sigma_train: f32) -> Histogram`
    builds 100 fixed bins over `[-3·σ, +3·σ]`, half-open `[low, high)`,
    `count: Vec<u64>` length 100. Out-of-range values bucketed into
    the first / last bin (saturating).
  - `pub fn gate_survival(r_hat: &[f32], sigma_train: f32) ->
    [f32; 9]` for τ ∈ {0.1, 0.2, …, 0.9}: fraction of bars with
    `|r_hat|/σ_train ≥ τ`.
  - 6 inline unit tests cover: (a) `summary_stats` on a 9-element
    fixture with hand-computed percentiles, (b) percentile-of-empty
    returns zeros / NaN sentinel, (c) histogram bin-edge
    inclusiveness, (d) histogram clamping for out-of-range r_hat,
    (e) gate survival monotone-decreasing in τ, (f) determinism:
    `summary_stats(v) == summary_stats(v)` byte-identical on a
    re-run (sort + math are deterministic).
  - `cargo test -p forecast --bin forecast_distribution -- hist::tests`
    → 6/6 PASS.

- [x] **T-D-4** — F-verdict classifier + report renderer.
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/forecast_distribution.rs` (mod verdict, lines ~383-500; render_report_full lines ~600-800) + `crates/forecast/tests/forecast_distribution_verdict.rs:1` (new file). Test: `cargo test -p forecast --test forecast_distribution_verdict`. Output: `test result: ok. 5 passed; 0 failed`.
  Owner: developer. Milestone: M-R-HAT. Depends on: T-D-2, T-D-3.
  Blocks: T-D-5.
  _file:line_: `crates/forecast/src/bin/forecast_distribution.rs`
  (`mod verdict` + `fn render_report`, ~200 LoC).
  _test_: `cargo test -p forecast --test forecast_distribution_verdict`.
  _output_: `test result: ok. 5 passed; 0 failed`.
  _acceptance_:
  - `mod verdict` exports `fn classify(stats: &CheckpointStats) ->
    Verdict` implementing ADR-0033 § D3.b's priority-ordered
    F1→F2→F3→F4 algorithm. `enum Verdict { F1 { evidence, follow_on },
    F2 {...}, F3 {...}, F4 {...} }` with `follow_on: &'static str`
    matching the ADR's mapping.
  - `fn render_report(stats: &Stats, hist: &Histogram, gate: &[f32; 9],
    verdict: &Verdict, ctx: &ReportContext) -> String` produces the
    markdown body per ADR-0033 § D2.a. Frontmatter rendered
    separately by the caller from `ReportContext` (carries
    `generated`, `wall_clock_s`, `host`, `git_commit`,
    `model_revision`, `sigma_train`, `data_revision_sha`).
  - Float canonicalisation per ADR-0033 § D2.a:
    `format!("{:.6}", x)` for σ/ε/τ/gates;
    `format!("{:.9}", x)` for percentiles + mean/std/min/max;
    `format!("{}", (x * 1e6) as i64)` for bin edges;
    `format!("{}", x)` for bin counts; ASCII-only, LF-only.
  - New test file `crates/forecast/tests/forecast_distribution_verdict.rs`
    with 5 tests: (1) F1 fixture (`abs_p95 = 1e-9`) → `Verdict::F1`;
    (2) F2 fixture (`std = 5.0, sigma_train = 10.0, frac_gate = 0.0`)
    → `Verdict::F2`; (3) F3 fixture (`abs_p95 = 0.0003,
    confidence_gate_survival[5] = 0.001, frac_inside_epsilon = 0.7`)
    → `Verdict::F3`; (4) F4 fixture (wide spread, no other gate
    triggers) → `Verdict::F4`; (5) mutual-exclusivity property: for
    `N = 100` random `CheckpointStats` (`ChaCha20Rng` seed
    `0xDEADBEEF`), `classify` returns exactly one of F1/F2/F3/F4 every
    time, and the four cases never co-trigger.
  - `cargo test -p forecast --test forecast_distribution_verdict`
    → 5/5 PASS.

- [x] **T-D-5** — End-to-end M-R-HAT bin run + read-only guard test.
  Owner: developer. Milestone: M-R-HAT. Depends on: T-D-2, T-D-3, T-D-4.
  Blocks: M-FINAL.
  _Ticked 2026-05-19 (orchestrator)._ BS-1 + BS-2 reports on disk;
  determinism re-run produced byte-identical body SHA per scenario:
  - BS-1: `ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54` (verdict F4)
  - BS-2: `d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06` (verdict F4)
  Joint verdict: F4 (both agree, no F-MIXED). Follow-on: `v25-tcn-horizon-bump-or-retire`.
  cmd: `cargo run -p forecast --release --features candle --bin forecast_distribution -- --scenario {bs1,bs2}`
  output: `report written verdict="F4"` for both scenarios.
  Read-only guard test: `cargo test -p forecast --test forecast_distribution_bin_readonly` PASS (dev).
  Determinism: 2-run body-SHA byte-identical (orchestrator verified via `python3 scripts/hash_report.py`).
  _file:line_:
  `crates/forecast/tests/forecast_distribution_bin_readonly.rs:1`
  (new file, ~80 LoC) +
  `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260518.md`
  (new) +
  `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260518.md`
  (new).
  _test_:
  `cargo run -p forecast --bin forecast_distribution --features candle -- --scenario bs1 && cargo run -p forecast --bin forecast_distribution --features candle -- --scenario bs2`
  followed by
  `cargo test -p forecast --test forecast_distribution_bin_readonly`.
  _output_: 2 report files on disk (one per scenario);
  `test result: ok. 2 passed; 0 failed` for the read-only guard.
  _acceptance_:
  - Two markdown reports written under
    `spec/v25-tcn-alpha-investigation/reports/`, named
    `forecast-distribution-bs{1,2}-realdata-YYYYMMDD.md`.
  - Each report's body carries the seven sections enumerated in
    ADR-0033 § D2.a in order: Checkpoint / Evaluation span / Summary
    statistics / Histogram / Confidence-gate survival / Verdict /
    Notes. Frontmatter carries the advisory fields per ADR-0033 § D2.a.
  - Two sequential runs of `cargo run -p forecast --bin
    forecast_distribution -- --scenario bs1` produce a report body
    whose SHA256 (computed via `python3 scripts/hash_report.py`) is
    byte-identical across the two runs (K3 determinism).
  - The `## Verdict` table in each report carries one of F1/F2/F3/F4
    + the evidence string + the named follow-on feature.
  - New file `crates/forecast/tests/forecast_distribution_bin_readonly.rs`
    contains 2 tests: (a) fixture run with `--out-dir` redirected to
    a tempdir; assert NO writes occurred outside the tempdir (snapshot
    the `crates/forecast/checkpoints/` mtimes before + after, assert
    equal); assert NO writes occurred under any subdirectory of
    `crates/forecast/replay-cache/` (same mtime gate). (b) `--help`
    output does NOT contain any of `retrain`, `update`, `write-checkpoint`.
  - `cargo test -p forecast --test forecast_distribution_bin_readonly`
    → 2/2 PASS.

## T-D rows — M-SHARPE (Sharpe-comparison report, 5 rows)

- [x] **T-D-6** — `sharpe_comparison` bin skeleton + CLI surface.
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/sharpe_comparison.rs:1` (new file). Test: `cargo build -p forecast --bin sharpe_comparison`. Output: `Finished \`dev\` profile` (clean). `--help` shows 3-flag surface (--out-dir, --backtest-bin, --skip-rerun); no retrain/anchor-mutation flag.
  Owner: developer. Milestone: M-SHARPE. Depends on: T-AR-2. Blocks:
  T-D-7, T-D-8, T-D-9, T-D-10. _(Independent of T-D-1..T-D-5 — runs
  in parallel.)_
  _file:line_: `crates/forecast/src/bin/sharpe_comparison.rs:1` (new
  file, ~60 LoC for skeleton).
  _test_: `cargo build -p forecast --bin sharpe_comparison`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - New file `crates/forecast/src/bin/sharpe_comparison.rs` with
    `clap::Parser`-derived `Args` matching ADR-0033 § D2.b.i:
    `--out-dir` (default
    `spec/v25-tcn-alpha-investigation/reports/`),
    `--backtest-bin` (default `target/release/backtest`),
    `--skip-rerun` (default false).
  - `fn main()` parses args, prints `tracing::info!(scenarios = …)`
    listing the four `-realdata` scenarios it will read. Body of
    re-run + Sharpe-compute + render lands in T-D-7..T-D-10.
  - `cargo build -p forecast --bin sharpe_comparison` clean.
  - `cargo run -p forecast --bin sharpe_comparison -- --help` prints
    the 3-flag surface; no flag implies retraining or anchor mutation
    (K5 hard contract).

- [x] **T-D-7** — Sharpe / Sortino / Calmar / max-DD math module.
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/sharpe_comparison.rs` (mod metrics, lines ~55-220). Test: `cargo test -p forecast --bin sharpe_comparison -- metrics::tests`. Output: `test result: ok. 5 passed; 0 failed`. SQRT_HOURS_PER_YEAR = 92.601295; NOT reusing crates/backtest::compute_sharpe (minute-annualised).
  Owner: developer. Milestone: M-SHARPE. Depends on: T-D-6. Blocks:
  T-D-9.
  _file:line_: `crates/forecast/src/bin/sharpe_comparison.rs`
  (`mod metrics`, ~120 LoC).
  _test_: `cargo test -p forecast --bin sharpe_comparison -- metrics::tests`.
  _output_: `test result: ok. 5 passed; 0 failed`.
  _acceptance_:
  - Inline `mod metrics` with:
    `pub fn compute_sharpe_hourly(equity: &[Decimal]) -> f64`
    (mean_r / std_r * sqrt(24·365)),
    `pub fn compute_sortino_hourly(equity: &[Decimal]) -> f64`
    (mean_r / downside_r * sqrt(24·365) with rf=0; `downside_r` =
    sqrt(mean(min(r,0)^2))),
    `pub fn compute_calmar(equity: &[Decimal]) -> f64`
    (cagr / abs(max_dd); cagr from (final/initial)^(1/years) - 1;
    years = (equity.len() - 1) / 8760.0),
    `pub fn compute_max_drawdown(equity: &[Decimal]) -> f64`
    (max over t of (peak - equity_t) / peak as f64).
  - All four functions take `&[Decimal]` (rust_decimal), convert per
    bar to f64 for the math (acceptable for risk metrics; the
    underlying equity stays Decimal). Annualisation constant
    `SQRT_HOURS_PER_YEAR = (24.0 * 365.0).sqrt()` (≈ 92.601295).
  - 5 inline unit tests cover: (a) Sharpe on a hand-built equity
    curve with known mean / std, asserted to 4 decimals;
    (b) Sortino vs Sharpe on an asymmetric-return curve (downside
    smaller → Sortino > Sharpe); (c) Calmar on a curve with known
    CAGR + DD; (d) max_drawdown on a peak-then-trough curve;
    (e) edge case: 1-element equity curve returns 0.0 for all four
    without panic.
  - `cargo test -p forecast --bin sharpe_comparison -- metrics::tests`
    → 5/5 PASS.
  - **Anti-reuse note**: the existing
    `crates/backtest::compute_sharpe()` (`crates/backtest/src/main.rs:2428`)
    annualises by `sqrt(525_600)` (minute resolution) and IS NOT
    reused. Cite this row's `compute_sharpe_hourly` from the M-SHARPE
    report body's `## Methodology` section.

- [x] **T-D-8** — Re-run orchestration (Option α).
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/sharpe_comparison.rs` (mod rerun, lines ~225-380) + `crates/backtest/src/main.rs` (Args: --emit-equity-bin + --reports-dir; TcnOverlayRunResult.equity_curve; emit logic in TCN dispatch branches). Test: `cargo test -p backtest --test backtest_sharpe_emit_equity_bin`. Output: `test result: ok. 3 passed; 0 failed`. `cargo build -p backtest --features realdata,candle` → Finished clean. Anchor neutrality: `cargo build -p backtest --features realdata,candle` passes; verify_anchors.sh 19/19 confirmed pre-change (tester T-T-1 gate).
  Owner: developer. Milestone: M-SHARPE. Depends on: T-D-6. Blocks:
  T-D-9. _(Independent of T-D-7 — can run in parallel.)_
  _file:line_: `crates/forecast/src/bin/sharpe_comparison.rs`
  (`mod rerun`, ~150 LoC).
  _test_: `cargo build -p forecast --bin sharpe_comparison` +
  manual smoke (T-D-10 covers the integration).
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - Inline `mod rerun` with `pub fn rerun_scenario(name: &str,
    backtest_bin: &Path, tempdir: &Path) -> Result<RerunResult>`
    where `name ∈ { top10-2023-fy-tcn-overlay-realdata,
    top10-2024-fy-tcn-overlay-realdata,
    top10-2023-fy-tcn-overlay-weights-realdata,
    top10-2024-fy-tcn-overlay-weights-realdata }`.
  - The function `std::process::Command`-spawns
    `<backtest_bin> --scenario <name> --reports-dir <tempdir>`
    with `--features realdata,candle` already compiled into the
    binary (T-D-6 ensures the bin is feature-flagged correctly in
    the release build); `tempdir` is a `tempfile::TempDir` so the
    four anchored `-realdata` reports under
    `spec/backtest-real-binance-data/reports/` are NEVER touched
    (R6 contract).
  - Parses the produced report's frontmatter (`scenario`,
    `data_revision_sha`, `wall_clock_s`) AND body Summary table
    (`Final equity`, `Bars (total)`, `Trades`, `Total return`,
    `Max drawdown` — for cross-check vs. the architect's computed
    values) AND, critically, reads a NEW sibling artifact:
    `<tempdir>/<scenario>-equity.bin` containing the raw
    `Vec<Decimal>` equity curve.
  - **Equity-curve emission**: T-D-8 piggybacks an additive flag
    `--emit-equity-bin <path>` on the existing `crates/backtest`
    binary (new flag, `clap::Arg`; defaults to disabled so the
    existing flag surface is byte-additive for the anchored
    `-realdata` runs which DO NOT pass `--emit-equity-bin`). When
    set, after `write_tcn_overlay_report()` completes, the harness
    writes `bincode::serialize(&equity_curve)` to the path. The
    report body is unchanged (the side file is outside `reports/`),
    so the anchor SHA stays byte-identical. _The architect notes
    this is a single-line additive `if let Some(path) =
    args.emit_equity_bin { fs::write(path, bincode::serialize(...)?)?; }`
    block after the existing report write — minimal blast radius._
  - 4 `RerunResult` values (one per scenario) collected by
    `fn main()`, passed to T-D-9's renderer.
  - **Anchor neutrality test** (covers T-D-8's main risk):
    `cargo test -p backtest --test backtest_sharpe_emit_equity_bin`
    new test asserts (a) running `--scenario
    top10-2023-fy-tcn-overlay-realdata` WITHOUT `--emit-equity-bin`
    produces a report body with SHA matching the anchored value in
    `spec/anchors.toml`; (b) running the same scenario WITH
    `--emit-equity-bin /tmp/eq.bin` also produces a report body
    with the same SHA; (c) `/tmp/eq.bin` is non-empty and
    `bincode::deserialize::<Vec<Decimal>>(...)` succeeds.
  - **K3-flavoured risk acknowledgment**: T-D-8 is the only row
    that touches `crates/backtest`. The change is strictly additive
    (one new optional CLI flag + one conditional file write).
    `bash scripts/verify_anchors.sh` MUST report
    `ANCHORS PASS (19/19)` after T-D-8 lands — same as before.
    If it does not, T-D-8 is rejected and the developer escalates
    to architect for redesign.

- [x] **T-D-9** — Comparison-table renderer.
  _Ticked by developer 2026-05-19._ file:line: `crates/forecast/src/bin/sharpe_comparison.rs` (mod render, lines ~385-540). Test: `cargo test -p forecast --bin sharpe_comparison -- render::tests`. Output: `test result: ok. 3 passed; 0 failed`. Sections: Methodology / Comparison table / Verdict / Notes per ADR-0033 § D2.b. Float canonicalisation: %.6f Sharpe/Sortino/Calmar, %.2f%% return/DD/dampen. Honest-reading zero-dampen branch tested.
  Owner: developer. Milestone: M-SHARPE. Depends on: T-D-7, T-D-8.
  Blocks: T-D-10.
  _file:line_: `crates/forecast/src/bin/sharpe_comparison.rs`
  (`mod render`, ~180 LoC).
  _test_: `cargo test -p forecast --bin sharpe_comparison -- render::tests`.
  _output_: `test result: ok. 3 passed; 0 failed`.
  _acceptance_:
  - Inline `mod render` with `pub fn render_report(results: &[RerunResult; 4],
    ctx: &ReportContext) -> String` producing the markdown body per
    ADR-0033 § D2.b in order: Methodology / Comparison table /
    Verdict / Notes.
  - Float canonicalisation per ADR-0033 § D2: `%.6f` for
    Sharpe/Sortino/Calmar; `%.2f%%` for total return / max drawdown /
    dampen rate; integer bars/trades; final equity as
    `format!("${:.2}", x)` (Decimal → f64 → 2 decimals); ASCII-only,
    LF-only.
  - The 4-row table is ordered: passthrough-2023, passthrough-2024,
    real-weights-2023, real-weights-2024 (per ADR-0033 § D2.b).
  - The `## Verdict` table carries: (a) the honest reading line
    (`dampen rate = 0% → equity curves are byte-identical between
    passthrough and real-weights per year`) if `dampen_rate < 1e-6`
    across the 4 scenarios, OR an alpha-bearing reading if dampen
    rate is non-zero; (b) Sharpe delta between passthrough vs
    real-weights per year; (c) recommended follow-on gated by the
    M-R-HAT verdict (cross-reference the BS-1 + BS-2 report names if
    they exist under `--out-dir`; if not, the verdict says "awaiting
    M-R-HAT").
  - 3 inline unit tests cover: (a) renderer output for a hand-built
    4-result fixture matches a golden body byte-for-byte; (b) the
    `## Verdict` table picks the honest-reading branch when
    `dampen_rate = 0`; (c) the renderer is deterministic — two
    invocations with the same `(results, ctx)` produce byte-identical
    output.
  - `cargo test -p forecast --bin sharpe_comparison -- render::tests`
    → 3/3 PASS.

- [x] **T-D-10** — End-to-end M-SHARPE bin run + anchorability check.
  Owner: developer. Milestone: M-SHARPE. Depends on: T-D-7, T-D-8, T-D-9.
  Blocks: M-FINAL.
  _Ticked 2026-05-19 (orchestrator)._ Report on disk:
  `spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md`
  with body SHA `17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924`.
  2-run determinism: byte-identical body SHA across two sequential
  `cargo run -p forecast --release --bin sharpe_comparison` invocations
  (orchestrator verified). Report is anchorable. 4 scenarios re-run into
  tempdir produced 5800 / 5917 / 5800 / 5917 trades; `dampened=0` for
  all four (the alpha-investigation's headline finding).
  _file:line_:
  `spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260518.md`
  (new) + `crates/forecast/tests/sharpe_comparison_determinism.rs:1`
  (new file, ~60 LoC).
  _test_:
  `cargo run -p forecast --bin sharpe_comparison` (twice;
  compare body SHAs) followed by
  `cargo test -p forecast --test sharpe_comparison_determinism`.
  _output_: 1 report on disk; `test result: ok. 1 passed; 0 failed`
  for the determinism gate.
  _acceptance_:
  - One markdown report written under
    `spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-YYYYMMDD.md`.
  - The report's body carries the four sections enumerated in
    ADR-0033 § D2.b in order: Methodology / Comparison table /
    Verdict / Notes. Frontmatter carries the advisory fields per
    ADR-0033 § D2.b.
  - **Determinism gate (K3)**: two sequential runs of `cargo run -p
    forecast --bin sharpe_comparison` produce report bodies whose
    SHA256 (computed via `python3 scripts/hash_report.py`) is
    byte-identical. If yes → ready to anchor as
    `sharpe-comparison-realdata` under `v2.6.0-alpha-investigation`
    in T-D-11 (tester). If no → ship un-anchored with a
    `## Not anchorable` section that names the source of the
    non-determinism; T-D-11 (tester) records the gap and the feature
    ships 21 anchors instead of 22.
  - `crates/forecast/tests/sharpe_comparison_determinism.rs` runs
    the bin twice against a fixture tempdir (or against a mocked
    `RerunResult` fixture if the full re-run is too slow for the
    test budget — developer's call), and asserts body-SHA byte-
    identity. If a fixture-based test is used, it covers the
    `render::render_report` determinism only; the full-pipeline
    determinism is asserted in the M-FINAL test report from the
    tester. PASS expected.
  - **Anchor neutrality** post T-D-10:
    `bash scripts/verify_anchors.sh` → `ANCHORS PASS (19/19)` —
    the new bins write under `spec/v25-tcn-alpha-investigation/reports/`
    only; the 4 `-realdata` anchors stay byte-identical. (The
    `sharpe-comparison-realdata` anchor lands in M-FINAL via tester
    T-T-1, not T-D-10.)

## T-T row — M-FINAL (tester-owned, 1 row)

- [ ] **T-T-1** — Tester anchor lock + non-regression verification +
  ship-gate report. _ANCHOR LOCK DONE (22/22). VERDICT FAIL (re-gate 2026-05-19-1100) —
  Fix 1 (parse.rs) CONFIRMED FIXED. Fix 2 (determinism.rs mutex) PARTIALLY FIXED:
  26/26 PASS under --features realdata,candle, but introduced a NEW compile error under
  default features (cargo test --workspace): `ensure_realdata_binary()` (line 882, ungated)
  references `BACKTEST_BUILD_MU` (line 863, gated #[cfg(feature = "realdata")]) → E0425.
  Developer must add `#[cfg(feature = "realdata")]` to ensure_realdata_binary(),
  BACKTEST_COPY_COUNTER, and copy_to_unique() in determinism.rs._
  Owner: tester (blocked on developer fix). Milestone: M-FINAL. Depends on: T-D-5, T-D-10.
  Blocks: ship.
  _file:line_: `spec/anchors.toml` (3 new rows under
  `v2.6.0-alpha-investigation`) +
  `spec/v25-tcn-alpha-investigation/reports/test-YYYYMMDD-HHMM-v25-tcn-alpha-investigation.md`
  (new test report).
  _test_:
  `bash scripts/verify_anchors.sh` (pre-lock 19/19, post-lock 21/21 or 22/22) +
  `python3 scripts/hash_report.py <reports>` +
  `cargo test -p forecast --tests` +
  `cargo fmt --check` +
  `cargo clippy --workspace -- -D warnings`.
  _output_: `ANCHORS PASS (21/21)` or `ANCHORS PASS (22/22)`
  post-lock; test report on disk.
  _acceptance_:
  - PRE-anchor-lock: `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS (19/19)` (R6 contract; the 19 existing anchors
    stay byte-identical).
  - Add `forecast-distribution-bs1-realdata` and
    `forecast-distribution-bs2-realdata` anchors to `spec/anchors.toml`
    under `version = "v2.6.0-alpha-investigation"` with body-SHA256
    from `python3 scripts/hash_report.py` against each report.
  - If T-D-10's two-run body-SHA matched: add `sharpe-comparison-realdata`
    too (anchor count 22). If not: omit it; the test report records
    the gap.
  - POST-anchor-lock: `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS (21/21)` OR `ANCHORS PASS (22/22)`.
  - Test report at
    `spec/v25-tcn-alpha-investigation/reports/test-YYYYMMDD-HHMM-v25-tcn-alpha-investigation.md`
    per [tester template](../../.claude/skills/rust-test/templates/test-report.md):
    sections (1) summary verdict, (2) anchor delta + diff vs prior,
    (3) determinism evidence (4-run body-SHA comparison for the 3
    new anchors), (4) clippy/fmt summary, (5) joint F-verdict
    (BS-1 + BS-2) recorded with operator-disposition stub.
  - Trace row `REQ-V25-TCN-ALPHA-001` `crates` / `tests` / `anchors`
    columns filled by tester via `spec-update`.

## Parallelism map for the orchestrator

- M0 sequential (operator + architect gate). **Complete.**
- M-R-HAT and M-SHARPE are **independent** under MINIMAL scope and
  CAN run in parallel after T-AR-2: M-R-HAT is the read-only
  forecast-distribution bin; M-SHARPE is the comparison-table bin
  that piggybacks an additive CLI flag on `crates/backtest`. The two
  touch disjoint files except for `crates/forecast/src/bin/` (two
  sibling files) and `crates/forecast/Cargo.toml` (two new
  `[[bin]]` rows — devepoper merges).
- M-DIAG (if active) depends on M-R-HAT. **Inert under MINIMAL.**
- M-HORIZON (if active) is a heavy sequential step before M-FINAL.
  **Inert under MINIMAL.**
- M-FINAL (T-T-1) is sequential (tester gate after both M-R-HAT and
  M-SHARPE close).

### Wave 1 — Skeletons (parallel, no upstream past T-AR-2)

- **T-D-1** (forecast_distribution bin skeleton) — developer A.
- **T-D-6** (sharpe_comparison bin skeleton) — developer B.

### Wave 2 — Math + collection (parallel within each bin family)

Spawnable in parallel after Wave 1 closes:

- **T-D-2** (forward-pass loop, deps T-D-1) — developer A.
- **T-D-3** (hist statistics module, deps T-D-1) — developer A2 / parallel.
- **T-D-7** (metrics module, deps T-D-6) — developer B.
- **T-D-8** (rerun orchestration + `--emit-equity-bin` on backtest,
  deps T-D-6) — developer B2 / parallel.

> Note: T-D-2 and T-D-3 are independent of each other; T-D-7 and
> T-D-8 are independent of each other. A 4-way parallel dev launch
> at Wave 2 is feasible if the orchestrator has the headcount,
> though the historical 2-way launch (one dev per milestone) is the
> safer default.

### Wave 3 — Renderers + integration

- **T-D-4** (F-verdict + report renderer, deps T-D-2 + T-D-3) — developer A.
- **T-D-9** (comparison-table renderer, deps T-D-7 + T-D-8) — developer B.

### Wave 4 — End-to-end runs + determinism

- **T-D-5** (M-R-HAT bin run + read-only guard, deps T-D-4) — developer A.
- **T-D-10** (M-SHARPE bin run + determinism check, deps T-D-9) — developer B.

### Wave 5 — Ship gate (sequential)

- **T-T-1** (tester anchor lock, deps T-D-5 + T-D-10).

### Critical path

T-AR-2 → T-D-1 → T-D-2 → T-D-4 → T-D-5 → T-T-1. Wall-clock estimate
on the developer machine (single-dev sequential): ~6h
(skeleton + math + render + report) + ~3min (full bin runs) +
~30min tester gate ≈ 6.5h. 2-way parallel (dev A + dev B) cuts to
~3.5h. The same wall-clock estimate applies whether the joint
verdict lands F1/F2/F3/F4 — the report shape is identical; only
the body bytes differ.

## Out of scope for tasks.md

- M-DIAG / M-HORIZON T-D decomposition (deferred to follow-on features
  if M-R-HAT's F-verdict demands).
- Tuning F-thresholds in ADR-0033 § D3.b (superseding ADR required;
  this feature does not amend the algorithm).
- Anchor count > 22 (R2 / R6 contract — this feature lands at most
  3 new anchors).

## Notes

- Anchor naming convention (locked):
  `forecast-distribution-bs1-realdata`,
  `forecast-distribution-bs2-realdata`, and (if anchorable)
  `sharpe-comparison-realdata`. Under version
  `v2.6.0-alpha-investigation`.
- All T-D-N rows are read-only against the trained checkpoints
  (LFS-tracked under `crates/forecast/checkpoints/anchors/`). No row
  modifies a checkpoint, including T-D-8 which adds an additive
  `--emit-equity-bin` flag on `crates/backtest`'s existing
  `Scenario` runner — the body of the resulting report is unchanged
  (the equity-curve side file lands outside `reports/`).
- T-D-8 carries the only risk to the 19 byte-locked existing anchors.
  Its acceptance bullets explicitly include the
  `bash scripts/verify_anchors.sh` gate; failure rejects the row
  and escalates to architect.

## Changelog

- 2026-05-19 (tester re-gate at 5056739): T-T-1 re-gated at commit 5056739.
  Fix 1 (parse.rs skip presentations/) CONFIRMED FIXED: parse::tests::all_anchored_reports_parse_ok PASS.
  Fix 2 (determinism.rs mutex + unique copy paths) PARTIALLY FIXED: 26/26 PASS under
  --features realdata,candle, but introduces a new compile error under default features.
  `ensure_realdata_binary()` (line 882, ungated) references `BACKTEST_BUILD_MU` (line 863,
  #[cfg(feature = "realdata")]) → E0425 under `cargo test --workspace`. VERDICT → FAIL.
  HANDOFF → developer: add #[cfg(feature = "realdata")] to ensure_realdata_binary(),
  BACKTEST_COPY_COUNTER, and copy_to_unique() in crates/backtest/tests/determinism.rs.
  3-line fix. Test report: spec/v25-tcn-alpha-investigation/reports/test-20260519-1100-v25-tcn-alpha-investigation.md
- 2026-05-19 (tester): T-T-1 executed at commit b8a29a8. Anchor lock: 19→22 (22/22 PASS).
  All feature-specific tests PASS (fmt, clippy, 4 forecast tests, backtest_sharpe_emit_equity_bin,
  backtest --features realdata --test determinism 22/22). Two pre-existing failures found:
  (1) `parse::tests::all_anchored_reports_parse_ok` — presentation file naming collision from
  commit 664bb59; (2) `realdata_2023/2024_fy_tcn_overlay_determinism` concurrent binary-clobber
  race — exposed by LFS-resolved T-D-15 tests. Joint F4 verdict confirmed. VERDICT → FAIL.
  HANDOFF → developer for pre-existing test infrastructure fixes. Feature investigation
  itself is complete (F4, 3 anchors locked, Sharpe table published).
- 2026-05-18 (architect): T-D decomposition landed. M-R-HAT broken
  into T-D-1..T-D-5 (5 rows: bin skeleton, forward-pass collection,
  hist::Stats math, F-verdict + renderer, end-to-end + read-only
  guard). M-SHARPE broken into T-D-6..T-D-10 (5 rows: bin skeleton,
  metrics math (hourly Sharpe/Sortino/Calmar/DD), rerun orchestration
  (+ additive `--emit-equity-bin` flag on `crates/backtest`),
  comparison-table renderer, end-to-end + determinism check). 1
  tester row T-T-1 closes M-FINAL with anchor lock + non-regression
  gate. § Parallelism map: 4 waves + sequential ship gate; M-R-HAT
  and M-SHARPE waves run in parallel (2-way), Wave 2 further
  parallelisable to 4-way at orchestrator discretion. Critical
  path: T-AR-2 → T-D-1 → T-D-2 → T-D-4 → T-D-5 → T-T-1 (~6.5h
  single-dev sequential, ~3.5h 2-way). T-AR-1 + T-AR-2 + T-AR-3
  ticked. Status: in-progress. Owner: architect → developer at
  handoff.
- 2026-05-18 (analyst): milestone-only skeleton authored. M0 ticked
  under operator MINIMAL scope. M-R-HAT + M-SHARPE active;
  M-DIAG + M-HORIZON enumerated but inert.
