---
slug: v3-volatility-forecaster
status: in-progress
owner: developer
updated: 2026-05-22
---

# Tasks — v3 volatility forecaster (predict σ, not μ)

> **Analyst-decomposed T-A rows landed 2026-05-22** as the first of
> three parallel analyst passes triggered by the operator's 2026-05-22
> hybrid-sequence decision (Q-PICK = C1+C2+C5; Q-BUDGET ~6-8 weeks;
> Q-SEQ HYBRID — build C1 first; C2/C5 analyst-only spec until C1
> verdict). Architect / developer / tester / presenter rows are
> placeholders until **M-OD (Q1-Q6) resolves**.

## Analyst rows (T-A)

- [x] **T-A1** (2026-05-22) — Read predecessor materials.
  Confirmed the v2.5 DL forecast overlay umbrella retirement state:
  - **Strategy reformulation survey
    [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`](../dev-notes/strategy-reformulation-survey-2026-05-22.md)
    § Candidate 1 (volatility forecasting)** — survey-time cost /
    EV / reuse scoping. Survey rated C1 as **highest EV per
    wall-clock week** with MEDIUM-HIGH prior of clearing +0.10
    Sharpe-delta on the v1 cross-sectional momentum baseline.
  - **v2.5 DL journey retrospective
    [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
    § Lessons learned + § What the next research direction COULD
    usefully chase** — established (a) cheap-first investigation
    order lesson #1, (b) F-verdict immutability lesson #2 (ADR-0033
    stays immutable for return-target forecasters), (c) σ_train
    derivation load-bearing for confidence gating lesson #3, (d)
    architecture-paradigm tests beat hyperparameter sweeps lesson #4.
  - **Joint F4-F4-F4 evidence** across TCN BS-1/BS-2 @ 1h
    (+0.018/+0.045 Sharpe-delta T-MARGINAL) + PatchTST BS-1 @ 24h
    (+0.006 T-MARGINAL-lower) — predicting μ over the 5-feature
    OHLCV window does not extract +0.10 alpha on the v1 baseline at
    hourly cadence. Vol-target (σ-prediction) is orthogonal to this
    evidence chain.
  - **Operator routing 2026-05-22:** Q-PICK = C1+C2+C5 (3 picks);
    Q-BUDGET ~6-8 weeks total cap; Q-SEQ HYBRID (build C1 first);
    Q-PROCESS 3 analyst passes in parallel (this is C1).
  Cited: `spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`,
  `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`,
  `spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md
  § D3`, `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md
  § D1`, `spec/v25a-patchtst-overlay/feature.md § Why + § Outcome`.

- [x] **T-A2** (2026-05-22) — Survey vol-forecast literature + Rust
  ecosystem.
  - **GARCH(1,1) literature:** Bollerslev 1986 *Generalized
    Autoregressive Conditional Heteroskedasticity* — foundational;
    closed-form likelihood + quasi-Newton convergence. Crypto
    benchmarks: Catania-Grassi 2017 *Forecasting cryptocurrency
    volatility* — GARCH(1,1) β ≈ 0.85 empirical fit on hourly
    BTC/ETH; half-life ~24-72 hours.
  - **Range-based estimators:** Parkinson 1980 *The extreme value
    method for estimating the variance of the rate of return* —
    `σ̂_P = sqrt((1/(4·ln 2)) · (ln(high/low))²)`; 5-7× more
    sample-efficient than realized-vol-from-close per Parkinson's
    Brownian-motion analysis. Garman-Klass 1980 — extension with
    open/close adds another ~2× efficiency but assumes zero overnight
    gap (less-applicable to crypto 24/7 markets; Parkinson is the
    cleaner default).
  - **Vol-forecast loss:** Patton 2011 *Volatility forecast
    comparison using imperfect volatility proxies* — QLIKE is the
    robust loss when the realized-vol proxy is noisy (e.g.
    Parkinson-derived target). QLIKE preferred over MSE for vol
    forecasts because it is **invariant to the proxy's noise scale**.
  - **Vol-targeting precedent:** Moreira & Muir 2017
    *Volatility-Managed Portfolios* — Journal of Finance 72(4) —
    vol-targeting on momentum delivers 0.15-0.40 Sharpe lift on
    equity factor portfolios. Crypto-hourly empirical evidence is
    thinner — this feature is the load-bearing empirical answer.
  - **Pure-Rust GARCH:**
    - `rust-quant` v0.0.10 — GARCH-family fitters per the
      [README](https://github.com/avhz/RustQuant). License: Apache-2.0.
      Architect-decide at M-T1 whether the API surface fits or
      hand-rolled is cleaner.
    - Hand-rolled MLE in ~80-120 LoC of pure Rust — closed-form
      gradient over (ω, α, β); quasi-Newton via `argmin` or
      hand-written. Zero new dependency. Analyst-default if
      `rust-quant` adds incidental scope.
  - **Candle for DL refinement (Q2 ≠ (a)):** existing ADR-0028
    candle ML framework covers vol forecasters explicitly. Small
    TCN-shape (~100k params) reuses
    `crates/forecast/src/tcn.rs` patterns at halved depth/channels;
    LSTM (~50k params) needs a candle-LSTM primitive verified at
    M-T1; PatchTST-shape (~150k params) reuses
    `crates/forecast/src/patchtst.rs` at smaller config.
  - **TCN/PatchTST scaffold reuse for vol target:**
    `crates/forecast/src/features.rs:489,627-628` —
    `windows_for_symbol` + target derivation; horizon-configurable
    extension lands in Wave A (developer) to emit Parkinson σ
    alongside the existing log-return target.
    `crates/forecast/src/overlay.rs::combine()` — **NOT reused**
    for the strategy-side vol-targeting overlay (consumer shape is
    risk-level multiplier, not signal-level direction modulation).

- [x] **T-A3** (2026-05-22) — Locate the canonical extension sites.
  - **`crates/forecast/src/lib.rs:44`** — existing
    `pub trait ForecastProvider` (direction-target). Vol forecaster
    adds a **sibling trait** `VolForecastProvider` (Q4=(a)
    analyst-default) per § R4 — keeps the existing trait byte-
    identical; cleanest separation for the risk-level consumer
    shape.
  - **`crates/forecast/src/features.rs:489,627-628`** — `WindowIterator`
    + target derivation; Wave A extends with `vol_target_kind: VolTargetKind`
    enum (`Parkinson` / `RealizedVol`) and emits a parallel target
    scalar alongside the existing `target_logret`.
  - **`crates/forecast/src/bin/forecast_distribution.rs`** — existing
    F-verdict bin (read-only). Vol forecaster gets a **sibling bin**
    `vol_verdict.rs` per § R5 — not an extension of `forecast_distribution.rs`
    (V-verdict has different inputs: per-symbol QLIKE table, not
    `frac_inside_epsilon` / gate-survival). ADR-0033 stays IMMUTABLE
    per Q4=(b) default + retrospective lesson #2.
  - **`crates/forecast/src/bin/sharpe_comparison.rs`** — existing
    Sharpe-comparison bin (Wave D T-D-N26 unit-tested per
    `spec/v25a-patchtst-overlay/feature.md § crates` enum). Vol
    forecaster extends additively with `--scenario
    vol-target-bs1` dispatch.
  - **`crates/forecast/checkpoints/anchors/`** — existing TCN (BS-1
    + BS-2) + PatchTST (BS-1) safetensors + metadata. Vol forecaster
    adds `garch-bs1-<sha>.json` per-symbol params; under Q2 ≠ (a)
    DL refinement adds `vol-{tcn,lstm,patchtst}-bs1-<sha>.safetensors`
    + `.metadata.json`.
  - **`crates/strategy/`** — existing momentum + composed strategies
    + mean-reversion pairs + TCN/PatchTST overlay momentum. Vol
    forecaster adds **3 new files** per § R6:
    `vol_targeting_overlay.rs` (R6.a; primary deliverable),
    `vol_killswitch_overlay.rs` (R6.b; secondary), and
    `vol_meanreversion.rs` (R6.c; tertiary). All siblings; zero
    refactor of existing strategy files.
  - **`crates/cost/src/`** — the brief's reference to
    `risk_state.rs` is **STALE** (closest is `crates/cost/src/budget.rs`).
    Risk-engine integration (Q3=(c)) is **deferred to v0.1.1**;
    v0.1.0 ships strategy-side composition only per Q3=(b/d)
    analyst-default.
  - **`crates/replay-cache/`** — existing strict-replay cache.
    Vol forecast determinism extends additively with namespace
    `"vol_forecast"`; existing `"forecast"` namespace stays
    byte-identical.
  - **`crates/audit/`** — existing audit ledger. Vol forecaster
    emits `JournalEntry { kind: "vol_forecast_emitted", … }` rows;
    additive only.
  Confirmed: existing scaffold is **vol-extensible by sibling
  addition**; Wave A introduces only NEW files (`vol.rs`, `garch.rs`,
  `vol_verdict.rs` bin, `vol_targeting_overlay.rs`, sibling backtest
  scenario, 4-5 new unit tests); existing TCN + PatchTST files stay
  byte-identical modulo additive enum variants in
  `sharpe_comparison.rs` dispatch (architect designs at M-T1 to
  preserve existing byte-output).

- [x] **T-A4** (2026-05-22) — Author `feature.md` brief.
  Frontmatter (`status: draft`, `owner: analyst`, `version: 0.1.0`,
  predecessor: `v25a-patchtst-overlay v0.1.0 (RETIRED-evidence-source)`,
  parent: `(none — new strategy lane; first ship in post-v2.5
  reformulation)`). **R1-R12 requirements** (vol target derivation +
  GARCH(1,1) baseline + conditional DL refinement + vol forecaster
  trait/impl + V-verdict algorithm + ADR-0038 + vol-targeting overlay
  strategy + backtest scenario + Sharpe-comparison + watch recipe +
  non-regression contract + verification gates + risk-engine
  integration deferral). **Hypothesis register H1-H4** (DL beats
  GARCH ≥5% QLIKE; vol-targeting Sharpe-delta ≥ +0.10 vs un-targeted;
  3-4 week cheap-first ship feasibility; hourly crypto vol IS
  predictable). **Risk register K-vol-1..6** (turnover eats lift;
  strategy-side vs risk-engine ADR amendment; scope creep into v2.5
  forecast crate; H4 falsification surface; V-verdict shape
  disagreement; Q3=(d) under-delivers). **Open questions Q1-Q6** —
  all with analyst-recommended defaults; "autoapprove" activates the
  bundle (Q1=(b) Parkinson + Q2=(a) GARCH-only-MVP + Q3=(d)
  all-3-builders + Q4=(b) ADR-0038 NEW + Q5=(a) v3.0.0-volatility +
  Q6=(a) BS-1 train + BS-2 val). **Non-regression contract** (13
  invariants; 30 anchors stay byte-identical). **Acceptance per
  milestone** (M-OD / M-T1 / M-D / M-V-VERDICT / M-SHARPE / M-FINAL
  / M-PRESENTER). **Cost estimate** under Q2=(a): ~2-3 weeks best
  case, ~3-4 weeks with one retry. Under Q2 ≠ (a): ~4-6 weeks.
  Out-of-scope guardrails. Sources cited.

- [x] **T-A5** (2026-05-22) — Open `[[req]]` row in `spec/trace.toml`.
  `REQ-V3-VOL-FORECASTER-001` opened in `draft` state. `arch` field
  pre-populated with ADR cross-refs (ADR-0028 candle for DL refinement;
  ADR-0029 checkpoint provenance; ADR-0032 backtest realdata path;
  ADR-0033 F-verdict IMMUTABLE — vol uses NEW ADR-0038; ADR-0035
  σ_train pattern applies to DL refinement only). `crates` field
  pre-populated with intended scope (`crates/forecast`,
  `crates/strategy`, `crates/backtest`, `crates/replay-cache`,
  `crates/audit`, `crates/core`). `tests` + `anchors` columns stay
  empty (architect / developer / tester fill at M-T1 / M-D / M-FINAL
  respectively).

- [x] **T-A6** (2026-05-22) — Promote to Active in `spec/backlog.md`.
  Entry added at top of `## Active` block. Activation source:
  operator's 2026-05-22 hybrid-sequence decision (Q-SEQ HYBRID —
  build C1 first). C2 + C5 are spec-only analyst-pass-in-parallel
  this same day; they stay in Queue § Strategy until C1's verdict
  routes whether to promote them.

- [x] **T-A7** (2026-05-22) — Emit analyst handoff envelope.
  TOML envelope `from = "analyst"`, `to = "operator"`,
  `verdict = "READY-FOR-OPERATOR-DECIDE"`, with Q1-Q6 surfaced and
  analyst-recommended defaults flagged. Predecessor evidence (joint
  F4-F4-F4 across v2.5 DL chain; survey C1 = highest EV per
  wall-clock week; retrospective's "could usefully chase"
  vol-forecaster head item) cited as motivating context.

## M-OD — Operator-decide (Q1-Q6) — resolved 2026-05-22

> All 8 analyst-recommended defaults accepted in one tick via the
> operator's standing "Autoapprove all" directive (confirmed
> 2026-05-22 against the analyst hand-off envelope). Internally
> consistent bundle: Q1=(b) Parkinson + Q2=(a) GARCH-only-MVP +
> Q3=(d) all-3-builders + Q4=(b) ADR-0038 NEW + Q5=(a) v3.0.0-
> volatility + Q6=(a) BS-1 train + BS-2 val + Q-anchors-sub=3 new
> + Q3-sub analyst defaults.

- [x] **T-OD1** — Q1 = (b) Parkinson estimator (high/low-based
  realized vol; OHLCV high+low columns already present).
- [x] **T-OD2** — Q2 = (a) **GARCH(1,1)-only-MVP** per cheap-first
  lesson from v25-tcn-journey retrospective. Defer DL refinement
  to v0.1.1 if H1 GARCH baseline passes but vol-targeting H2
  marginally misses +0.10.
- [x] **T-OD3** — Q3 = (d) all 3 builders shipped as opt-in
  (`with_garch_vol_strategy`, `with_garch_vol_overlay_momentum`,
  `with_garch_vol_kill_switch`); primary anchor target =
  vol-targeting overlay on v1 momentum.
- [x] **T-OD4** — Q4 = (b) **NEW ADR-0038 V-verdict** (V1-V5 +
  V_ALPHA parallel-tree shape mirrors ADR-0033 § D3 structure).
  ADR-0033 § D3 stays IMMUTABLE per retrospective lesson #2.
- [x] **T-OD5** — Q5 = (a) version `v3.0.0-volatility`; N_new = 3
  anchors at ship (vol-forecast-distribution-bs1 + vol-overlay-
  momentum-bs1-realdata + sharpe-comparison-volatility).
- [x] **T-OD6** — Q6 = (a) BS-1 (2023) train + BS-2 (2024) val
  span; mirrors v2.5 convention for apples-to-apples comparison
  vs retired v2.5 baseline.
- [x] **T-OD-Q3-sub** — Q3-sub analyst defaults accepted:
  vol-targeting clamp `[0.5×, 2×]`; target_vol = 0.02
  daily-equivalent; kill-switch threshold = 3× per-symbol median σ̂.
- [x] **T-OD-Q-anchors-sub** — N_new = 3 anchors at v0.1.0; the
  4th-anchor option (kill-switch backtest) deferred to v0.1.1 if
  Q3=(d) kill-switch builder lands but its anchor body isn't
  byte-deterministic at v0.1.0 ship time.

> **Once Q1-Q6 resolve**, frontmatter flips `status: draft →
> proposed`, `owner: analyst → architect`. The architect spawn
> proceeds from M-T1 with the T-AR rows below.

## Architect rows (T-AR) — resolved 2026-05-22 at M-T1

> **All 10 T-AR rows ticked 2026-05-22.** Architect lock landed via
> `spec/v3-volatility-forecaster/decomp.md` +
> `spec/architecture/adr/0038-vol-forecast-verdict-shape.md`. Baseline
> anchor gate confirmed PASS before lock:
> `bash scripts/verify_anchors.sh` →
> `ANCHORS PASS  (30 / 30)`.

- [x] **T-AR-1** (2026-05-22) — Topology + GARCH MLE implementation
  choice locked: **hand-rolled MLE in `crates/forecast/src/garch.rs`**
  (~120 LoC, zero new dependency). `rust-quant` v0.0.10 rejected per
  4 reasons (CLAUDE.md § Library compatibility checklist + API fit +
  maintained status + determinism contract); see decomp.md § T-AR-1
  + ADR-0038 § D3. Hyperparameters locked (ω init=1e-6; α init=0.10;
  β init=0.85; convergence tol=1e-8; max_iters=500; L-BFGS optimiser;
  stationarity constraint α+β<1). Wave A-E ordered with parallelism
  (Wave A ∥ Wave B; C depends A+B; D depends C; E depends D); Wave F
  (presenter) collapses into Wave E here (the deck is part of M-FINAL
  handoff). Q2=(a) skips DL training entirely — no Wave C
  (former DL-training slot is now the V-verdict-bin slot).
- [x] **T-AR-2** (2026-05-22) — ADR-0038 V-verdict shape (NEW) authored at
  `spec/architecture/adr/0038-vol-forecast-verdict-shape.md` (status:
  accepted). D1 V1→V2→V3→V4→V5 priority tree + V_ALPHA strategy-side
  gate sibling; D2 report body shape (frontmatter advisory; body
  hashed; per-symbol QLIKE table; floating-point canonicalisation
  `%.6f`; symbol-row order alphabetical USDT-quote); D3 GARCH(1,1)
  baseline contract (hand-rolled MLE; JSON checkpoint schema;
  aggregate SHA derivation); D4 replay-cache namespace additive
  (`CacheNamespace::VolForecast`); D5 strategy-side composition
  v0.1.0; D6 anchor + version naming `v3.0.0-volatility` (N_new=3).
  V-verdict thresholds locked: V1 CoV(σ̂)<1e-3; V2 qlike_dispersion>3.0;
  V3 mean_calibration_ratio outside [0.7,1.4]; V4 n_improving<7/10;
  V5 fallback. Parallel to ADR-0033 § D3 (Q4=(b) operator default);
  ADR-0033 stays IMMUTABLE per retrospective lesson #2.
- [x] **T-AR-3** (2026-05-22) — Parkinson target derivation site locked:
  `crates/forecast/src/features.rs:642-656` extension (additive
  `VolTargetKind` enum + `vol_target_kind: Option<VolTargetKind>` in
  `FeatureConfig` + `target_parkinson_vol: Option<f32>` in
  `FeatureWindow` + derivation block per ADR-0038 § D3). Single-horizon
  per-window scalar (NOT rolling-window; NOT both); horizon defaults
  to `target_horizon_bars = 24` per Q1+Q6. Existing TCN/PatchTST
  callers pass `vol_target_kind: None` — iteration order + window
  contents + `target_logret` byte-identical (R11.7 + R11.8 guard).
- [x] **T-AR-4** (2026-05-22) — Consumer shape: all 3 builders ship as
  opt-in in v0.1.0 per Q3=(d) (`with_garch_vol_strategy`,
  `with_garch_vol_overlay_momentum`, `with_garch_vol_kill_switch`);
  primary anchor target = vol-targeting overlay on v1 momentum (R6.a).
  Kill-switch backtest scenario deferred to v0.1.1
  (Q-anchors-sub=3); standalone strategy unit-tested only in v0.1.0.
  Strategy-side composition only (no risk-engine in v0.1.0); see
  ADR-0038 § D5.
- [x] **T-AR-5** (2026-05-22) — V-verdict bin locked at
  `crates/forecast/src/bin/vol_verdict.rs` (sibling of
  `forecast_distribution.rs`, ~280 LoC). CLI surface = 5 args
  (--scenario, --data-root, --out-dir, --span-start, --span-end);
  default scenario `bs1`; default out-dir
  `spec/v3-volatility-forecaster/reports/`. Read-only contract guards
  lifted from ADR-0033 § D1.c verbatim. Mutual-exclusivity test:
  `crates/forecast/tests/vol_verdict_mutual_exclusivity.rs` per
  ADR-0038 § D1.b + R11.5.
- [x] **T-AR-6** (2026-05-22) — Backtest scenario integration locked:
  new file `crates/backtest/src/scenarios/garch_vol_target_overlay.rs`
  (sibling of `tcn_overlay_weights.rs`); register
  `pub mod garch_vol_target_overlay;` in
  `crates/backtest/src/scenarios/mod.rs:20`; new variant
  `ScenarioStrategy::GarchVolTargetOverlayMomentum { config_id,
  forecaster_id }` in `crates/backtest/src/main.rs:104-136`; match
  arm in `Scenario::from_name` placed after the existing
  `top10-2023-fy-patchtst-overlay-realdata` arm at
  `crates/backtest/src/main.rs:536-558` (alphabetical). Strategy
  config `crates/strategy/config/vol_target_overlay_momentum.toml`
  pinning target_vol=0.02 + scale_clamp=[0.5,2.0] +
  momentum_config_id="top10_momentum".
- [x] **T-AR-7** (2026-05-22) — Sharpe-comparison extension locked:
  additive `ScenarioFamily` enum (Tcn/Patchtst/VolTarget) +
  `--scenario vol-target-bs1` dispatch arm in
  `crates/forecast/src/bin/sharpe_comparison.rs`. Existing
  `Tcn`/`Patchtst` dispatch byte-identical (anchored
  `sharpe-comparison-realdata` SHA 17d2e96c… untouched). VolTarget
  sources = un-targeted v1 momentum baseline + vol-target overlay
  scenario. T-classifier verdict (T-VOL-ALPHA-UNLOCKED /
  T-VOL-MARGINAL / T-VOL-NO-ALPHA) embedded in report body per
  ADR-0038 § D1.c.
- [x] **T-AR-8** (2026-05-22) — Wave shape locked: 5 waves (A-E); Wave C
  is V-verdict bin (DL training collapsed away under Q2=(a)). Wave
  A ∥ Wave B (parallel-eligible: A touches `forecast::{vol,garch}`;
  B touches `features.rs`); C depends on A+B; D depends on C; E
  depends on D. Wave row breakdown in
  `spec/v3-volatility-forecaster/decomp.md` § 3 with file:line
  targets + cargo invocations + expected literal outputs (honest-tick
  rule). T-D-N1..T-D-N28 rows appended below.
- [x] **T-AR-9** (2026-05-22) — Training cost negligible. ~5-10 seconds
  total wall-clock for 10 per-symbol GARCH MLE fits on ~8760 hourly
  bars/symbol. No watch recipe needed (R9 only fires under Q2 ≠ (a));
  single longest-running step in v0.1.0 is the backtest scenario itself
  (~40s for `top10-2023-fy-vol-target-overlay-realdata`).
- [x] **T-AR-10** (2026-05-22) — Wave map / parallelism finalised:
  developer can interleave A+B on a single thread (recommended) or
  spawn 2 developers truly parallel; A's output (`GarchVolForecaster`)
  + B's output (`target_parkinson_vol`) both need to land before C
  starts; D depends on C's V-verdict report shape for backtest
  deterministic expectations; E ticks tester gate (R11) + presenter
  handoff. Rollback shape per wave documented in decomp.md § 5
  (`git revert <wave-commit>` works at every boundary because every
  wave's diff is additive against the previous wave's `main`).

## Developer rows (T-D) — appended at M-T1 close 2026-05-22

> **Honest-tick rule:** every row carries file:line target + cargo
> invocation + expected literal output line. Developer ticks the row
> only after running the invocation and quoting the literal output
> back into this file.

### Wave A — GARCH(1,1) fitter + vol forecaster trait (Days 1-3; parallel ∥ Wave B)

- [ ] **T-D-N1** — `crates/forecast/src/garch.rs` (new) — `GarchModel
  { omega, alpha, beta, unconditional_var }` struct + hand-rolled
  L-BFGS MLE per ADR-0038 § D3 (~120 LoC).
  cargo: `cargo build -p forecast --features candle` →
  `Finished ... in ...`.
- [ ] **T-D-N2** — `crates/forecast/src/vol.rs` (new) —
  `VolForecastProvider` async trait + `VolRequest` / `VolResponse`
  types per ADR-0038 § D1.a (~80 LoC).
  cargo: `cargo build -p forecast --features candle` →
  `Finished ... in ...`.
- [ ] **T-D-N3** — `crates/forecast/src/lib.rs` additive
  `pub mod garch; pub mod vol;` lines.
  cargo: `cargo check -p forecast` → `Finished ... in ...`.
- [ ] **T-D-N4** — `crates/forecast/src/bin/train_garch.rs` (new) —
  per-symbol MLE driver; emits
  `crates/forecast/checkpoints/anchors/garch-bs1-<sha>.json` per
  ADR-0038 § D3 JSON schema (~100 LoC).
  cargo: `cargo run -p forecast --bin train_garch --features candle --release -- --scenario bs1` →
  `garch-bs1 fitted 10 symbols in N.N s; checkpoint_revision = <64-hex>`.
- [ ] **T-D-N5** — `crates/forecast/tests/garch_fit_determinism.rs`
  (new) — 2-run byte-identity of per-symbol JSON outputs (R11.4).
  cargo: `cargo test -p forecast --test garch_fit_determinism --features candle` →
  `test result: ok. 1 passed; 0 failed`.
- [ ] **T-D-N6** — `crates/forecast/tests/tcn_byte_identity.rs` (new) —
  R11.7 K-vol-3 guard (`git diff HEAD -- crates/forecast/src/tcn.rs`
  empty modulo comment-only).
  cargo: `cargo test -p forecast --test tcn_byte_identity --features candle` →
  `test result: ok. 1 passed; 0 failed`.
- [ ] **T-D-N7** — `crates/forecast/tests/patchtst_byte_identity.rs`
  (new) — R11.8 K-vol-3 guard.
  cargo: `cargo test -p forecast --test patchtst_byte_identity --features candle` →
  `test result: ok. 1 passed; 0 failed`.
- [ ] **T-D-N8** — `crates/replay-cache/src/lib.rs` additive
  `CacheNamespace::VolForecast` variant per ADR-0038 § D4.
  cargo: `cargo build -p replay-cache` → `Finished ... in ...`.

### Wave B — Parkinson target derivation (Days 1-2; parallel ∥ Wave A)

- [ ] **T-D-N9** — `crates/forecast/src/features.rs:499-687` additive
  `VolTargetKind` enum + `vol_target_kind: Option<VolTargetKind>` in
  `FeatureConfig` + `target_parkinson_vol: Option<f32>` in
  `FeatureWindow` + Parkinson derivation block at line 642-656 per
  T-AR-3 / ADR-0038 § D3.
  cargo: `cargo build -p forecast` → `Finished ... in ...`.
- [ ] **T-D-N10** —
  `crates/forecast/tests/parkinson_target_derivation.rs` (new) —
  25-bar hand-built fixture; Parkinson σ matches closed-form to 6
  decimals.
  cargo: `cargo test -p forecast --test parkinson_target_derivation` →
  `test result: ok. 1 passed; 0 failed`.
- [ ] **T-D-N11** — Confirm existing TCN/PatchTST callers green
  (additive-field invariant).
  cargo: `cargo test -p forecast --features candle --lib` →
  `test result: ok. N passed; 0 failed`.

### Wave C — V-verdict bin + report (Days 3-4; depends A+B)

- [ ] **T-D-N12** — `crates/forecast/src/bin/vol_verdict.rs` (new) —
  sibling of `forecast_distribution.rs` per ADR-0038 § D2.a;
  ~280 LoC.
  cargo: `cargo build -p forecast --bin vol_verdict --features candle` →
  `Finished ... in ...`.
- [ ] **T-D-N13** —
  `crates/forecast/tests/vol_verdict_mutual_exclusivity.rs` (new) —
  R11.5 V1-V5 priority tree per ADR-0038 § D1.b.
  cargo: `cargo test -p forecast --test vol_verdict_mutual_exclusivity --features candle` →
  `test result: ok. N passed; 0 failed` (N ≥ 6: 5 per-label
  fixtures + 1 property test).
- [ ] **T-D-N14** — End-to-end run; emit first
  `vol-verdict-bs1-realdata-<date>.md` under
  `spec/v3-volatility-forecaster/reports/`.
  cargo: `cargo run -p forecast --bin vol_verdict --features candle --release -- --scenario bs1` →
  `wrote spec/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-<YYYYMMDD>.md (body-SHA256 = <64-hex>)`.
- [ ] **T-D-N15** — 2-run byte-identity gate on the new report
  (R11.9). Re-run + body-bytes-diff.
  cargo: `cargo run -p forecast --bin vol_verdict --features candle --release -- --scenario bs1`
  (twice) + diff body bytes excluding frontmatter → (empty diff).

### Wave D — 3 consumer builders + backtest scenario + sharpe-comparison ext (Days 5-7; depends C)

- [x] **T-D-N16** — `crates/strategy/src/vol_targeting_overlay.rs`
  (new) — R6.a primary deliverable per ADR-0038 § D5; wraps inner
  v1 momentum + scales order quantities by clamped
  `target_vol / sigma_hat`; ~500 LoC (includes tests + checkpoint_loader).
  file: `crates/strategy/src/vol_targeting_overlay.rs:181`.
  cargo: `cargo build -p strategy` → `Finished dev profile ... in 1.42s`.
- [x] **T-D-N17** — `crates/strategy/src/vol_killswitch_overlay.rs`
  (new) — R6.b secondary; ~330 LoC.
  file: `crates/strategy/src/vol_killswitch_overlay.rs:108`.
  cargo: `cargo build -p strategy` → `Finished dev profile ... in 1.42s`.
- [x] **T-D-N18** — `crates/strategy/src/vol_meanreversion.rs` (new)
  — R6.c tertiary; ~265 LoC.
  file: `crates/strategy/src/vol_meanreversion.rs:95`.
  cargo: `cargo build -p strategy` → `Finished dev profile ... in 1.42s`.
- [x] **T-D-N19** — `crates/strategy/src/lib.rs:53-111` — 3 new builder fns
  (`with_garch_vol_strategy`, `with_garch_vol_overlay_momentum`,
  `with_garch_vol_kill_switch`).
  file: `crates/strategy/src/lib.rs:66`.
  cargo: `cargo build -p strategy` → `Finished dev profile ... in 1.42s`.
- [x] **T-D-N20** —
  `crates/strategy/config/vol_target_overlay_momentum.toml` (new) —
  target_vol=0.02 + scale_clamp=[0.5,2.0] + momentum_config_id="top10_momentum".
  file: `crates/strategy/config/vol_target_overlay_momentum.toml` (written 2026-05-22).
- [x] **T-D-N21** — `crates/strategy/tests/vol_targeting_overlay.rs`
  (new) — R11.6 overlay wrap correctness + scale clamp invariants +
  zero-sigma defensive guard (8 tests).
  file: `crates/strategy/tests/vol_targeting_overlay.rs`.
  cargo: `cargo test -p strategy --test vol_targeting_overlay` →
  `test result: ok. 8 passed; 0 failed`.
- [x] **T-D-N22** —
  `crates/backtest/src/scenarios/garch_vol_target_overlay.rs` (new)
  — mirror of `tcn_overlay_weights.rs` per T-AR-6; ~310 LoC.
  file: `crates/backtest/src/scenarios/garch_vol_target_overlay.rs`.
  cargo: `cargo build -p backtest --features realdata,candle` →
  `Finished dev profile ... in 0.58s`.
- [x] **T-D-N23** — `crates/backtest/src/scenarios/mod.rs:14`
  additive `pub mod garch_vol_target_overlay;` (rolled into N22).
  file: `crates/backtest/src/scenarios/mod.rs:14`.
  cargo: (rolled into N22) → `Finished dev profile ... in 0.58s`.
- [x] **T-D-N24** — `crates/backtest/src/main.rs` —
  `ScenarioStrategy::GarchVolTargetOverlayMomentum { config_id, forecaster_id }`
  variant + `Scenario::from_name` match arm + main() dispatch block.
  file: `crates/backtest/src/main.rs` (new variant + match arm + dispatch).
  cargo: `cargo build -p backtest --features realdata,candle` →
  `Finished dev profile ... in 0.58s`.
- [x] **T-D-N25** — Ran backtest end-to-end; emitted
  `spec/v3-volatility-forecaster/reports/backtest-20260522-082901-top10-2023-fy-vol-target-overlay-realdata.md`.
  body-SHA256 = `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65`.
  cargo: `cargo run -p backtest --release --features realdata,candle --bin backtest -- --scenario top10-2023-fy-vol-target-overlay-realdata --seed 0xC0FFEE` →
  `Final equity : $113479.97 USDT`.
- [x] **T-D-N26** — 2-run byte-identity confirmed on
  `backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md`.
  Both SHA-256 = `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65`.
- [x] **T-D-N27** — `crates/forecast/src/bin/sharpe_comparison.rs`
  additive `ScenarioFamily` enum (Tcn/VolTarget) + `--scenario vol-target-bs1`
  dispatch + `render_vol_target` module (T-classifier logic).
  file: `crates/forecast/src/bin/sharpe_comparison.rs`.
  cargo: `cargo build -p forecast --bin sharpe_comparison --features candle` →
  `Finished release profile ... in 3.90s`.
- [x] **T-D-N28** — Ran sharpe-comparison; emitted
  `spec/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md`.
  body-SHA256 = `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1`.
  T-classifier = **T-VOL-NO-ALPHA** (net_delta < +0.05).
  cargo: `cargo run -p forecast --bin sharpe_comparison --features candle --release -- --scenario vol-target-bs1` →
  `wrote spec/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md; T-classifier = T-VOL-NO-ALPHA`.

### Wave E — Tester gate + ADR-0038 finalisation + presenter handoff (Days 7-8; depends D)

(See tester rows T-T1..T-T3 + presenter T-P1 below; Wave E is the
join milestone.)

## Tester rows (T-T) — Wave E (resolved at M-FINAL)

- [ ] **T-T1** — R11 verification gates 1-12 (per feature.md § R11).
  cargo: `cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace --lib && bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (33 / 33)` (3 new + 30 existing byte-identical).
- [ ] **T-T2** — Anchor lock — add 3 new rows to `spec/anchors.toml`
  under `[v3.0.0-volatility]` namespace per ADR-0038 § D6
  (vol-verdict-bs1-realdata + top10-2023-fy-vol-target-overlay-realdata
  + sharpe-comparison-vol-target-bs1-realdata).
  cargo: `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (33 / 33)`.
- [ ] **T-T3** — V-verdict + T-classifier joint advisory verdict
  recorded in `feature.md § Verification` per ADR-0038 § D1.c joint
  table (5 rows: V5×T-VOL-ALPHA-UNLOCKED → ALPHA-UNLOCKED; V5×T-VOL-MARGINAL
  → MARGINAL; V5×T-VOL-NO-ALPHA → NO-ALPHA; V1/V2/V3 → MODEL-BROKEN;
  V4 → DATA-PATHOLOGY).

## Presenter rows (T-P) — PLACEHOLDER (resolved at M-PRESENTER)

- [ ] **T-P1** — Presenter deck at
  `spec/v3-volatility-forecaster/presentations/v3-volatility-forecaster-<YYYY-MM-DD>.md`
  carrying joint advisory verdict + recommended next routing per §
  M-PRESENTER (T-VOL-ALPHA-UNLOCKED → ship + promote C2/C5 to
  active; T-VOL-MARGINAL → spawn `v3-vol-target-tuning`;
  T-VOL-NO-ALPHA → analyst spawn for C1 retirement decision + route
  budget to C2).

## Notes

- **Hybrid sequencing context:** This is C1 (vol forecaster), the
  first-and-only-to-code of the operator's 3-pick hybrid sequence.
  C2 (regime classifier) and C5 (LLM-as-forecaster overlay) get
  analyst-only passes in parallel 2026-05-22; they stay in Queue §
  Strategy with `status: draft` analyst briefs but no architect /
  developer commitment until C1's verdict.
- **Cap discipline:** Operator's Q-BUDGET is ~6-8 weeks total cap
  across C1+C2+C5. Under Q2=(a) GARCH-only-MVP (analyst-default),
  C1 ships in ~3-4 weeks, leaving ~3-4 weeks for whichever of C2/C5
  the operator promotes after C1's verdict. Under Q2 ≠ (a), C1 eats
  4-6 weeks, leaving only ~2-3 weeks — operator should weight the
  Q2 default heavily.
- **F-verdict immutability honored:** ADR-0033 stays unchanged.
  V-verdict is the parallel shape for the new task (Q4=(b) default).
- **Cheap-first lesson applied:** Q2=(a) GARCH-only-MVP defers DL
  refinement to v0.1.1 if v0.1.0 finishes T-VOL-MARGINAL — exactly
  mirroring the retrospective's lesson #1 (cheap-first investigation
  order produces clean evidence; expensive retrains last).
- **Strategy-lane shift signaled by v3 anchor version:** the
  `v3.0.0-volatility` naming (Q5=(a) default) cleanly separates this
  lane from the retired `v2.5*` chain.
