---
slug: v3-volatility-forecaster
status: proposed
owner: architect
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

## Architect rows (T-AR) — PLACEHOLDER (resolved at M-T1)

- [ ] **T-AR-1** — Topology + Wave A-F lock. GARCH(1,1)
  hyperparameter ranges (ω/α/β initial + convergence tol). Under Q2
  ≠ (a): DL architecture + parameter count. Wave A: vol.rs +
  garch.rs + features.rs extension + 4-5 unit tests. Wave B:
  per-symbol GARCH fit. Wave C: (Q2 ≠ (a) only) DL training. Wave D:
  vol_verdict + sharpe_comparison ext + vol_targeting_overlay +
  backtest scenario. Wave E: tester. Wave F: presenter.
- [ ] **T-AR-2** — ADR-0038 V-verdict shape (NEW). D1 V1-V5 + V_ALPHA
  priority tree; D2 report shape (per-symbol QLIKE table +
  calibration scatter + verdict section + follow-on routing); D3
  mutual-exclusivity test contract; D4 GARCH(1,1) baseline contract
  (per-symbol fit, parameter ranges, convergence tolerance, JSON
  checkpoint shape); D5 strategy-side vs risk-engine composition
  decision.
- [ ] **T-AR-3** — Replay-cache namespace extension (`"vol_forecast"`
  additive; existing `"forecast"` byte-identical).
- [ ] **T-AR-4** — K-vol-3 byte-identity unit tests (tcn.rs +
  patchtst.rs sibling files stay empty git-diff after the vol ship).
- [ ] **T-AR-5** — ADR-0029 canonical-arch-descriptor additive
  extension (GARCH params + conditional DL params; v2.5 model_revision
  SHAs unchanged).
- [ ] **T-AR-6** — `crates/risk/` (or `crates/cost/src/budget.rs`)
  reference audit — the brief's reference to `risk_state.rs` is
  stale; architect verifies actual paths and confirms strategy-side
  composition is the v0.1.0 default. Risk-engine integration
  (Q3=(c)) deferred to v0.1.1 per analyst-default.

## Developer rows (T-D) — PLACEHOLDER (resolved at M-D)

- [ ] **T-D-N1..N6** — Wave A: `crates/forecast/src/vol.rs` (trait
  + types) + `crates/forecast/src/garch.rs` (GARCH fitter) +
  `crates/forecast/src/features.rs` extension (Parkinson target) +
  4-5 unit tests (garch_fit_determinism, vol_target_derivation,
  vol_verdict_mutual_exclusivity, tcn_byte_identity_after_vol_ship,
  patchtst_byte_identity_after_vol_ship).
- [ ] **T-D-N7** — Wave B: per-symbol GARCH(1,1) fit on BS-1 span
  (10 symbols × seconds; 30 min total wall-clock).
- [ ] **T-D-N8** — Wave C (Q2 ≠ (a) only): DL training run; watch
  recipe per R9.
- [ ] **T-D-N9..N12** — Wave D: vol_verdict bin + sharpe_comparison
  additive dispatch + vol_targeting_overlay strategy + backtest
  scenario integration.

## Tester rows (T-T) — PLACEHOLDER (resolved at M-FINAL)

- [ ] **T-T1** — R11 verification gates (12 gates per feature.md §
  R11).
- [ ] **T-T2** — Anchor lock (3 or 4 new anchors per Q5 sub-question;
  30 originals byte-identical).
- [ ] **T-T3** — V-verdict + T-classifier joint advisory verdict
  recorded in feature.md § Verification.

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
