---
slug: v25-tcn-horizon-bump-or-retire
status: shipped
owner: operator
updated: 2026-05-21
---

# Tasks — v2.5 TCN horizon-bump or retire

> Analyst-decomposed T-A rows landed 2026-05-21. Architect / developer
> / tester rows are scope-dependent placeholders until **Q1 (primary
> scope)** is operator-resolved at M-OD. **Q1 has NO safe analyst
> default — no work past M-OD until operator answers.**

## Analyst rows (T-A)

- [x] **T-A1** (2026-05-21) — Read predecessor materials.
  Confirmed full v2.5 TCN journey state:
  - `v25-tcn-overlay v2.5.0` (parent, in-progress) — original ship;
    BS-1 + BS-2 anchored checkpoints; 22 anchors on disk.
  - `v25-tcn-alpha-investigation v0.1.0` — F4 verdict; 4-bucket
    failure-mode taxonomy (R4); σ_train calibration anomaly surfaced.
  - `v25-tcn-recalibrate v0.1.0` — σ_train 608× / 580× inflation
    eliminated; gate-survival jumped 0% → 40-89%; F-verdict stays F4
    under immutable ADR-0033 § D3.
  - `v25-tcn-threshold-tuning v0.1.0` — joint T-MARGINAL + T-MARGINAL
    verdict (BS-1 +0.018 / BS-2 +0.045 at τ=0.1, ε=0.001); H1
    falsified.
  Operator's routing (c) from the recalibrate presenter deck (cheap
  τ-sweep first, multi-week retrain queued as fallback) is now
  exercised; this feature is that fallback.
  Cited: `spec/v25-tcn-threshold-tuning/feature.md § Verification`,
  `spec/v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md`,
  `spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md § D3`
  (immutable F-verdict), `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`
  (post-training σ_train recalibration; horizon-agnostic).

- [x] **T-A2** (2026-05-21) — Locate the canonical horizon-target
  derivation site.
  - `crates/forecast/src/features.rs:627-628` — `target_logret =
    (close_t1 / close_t).ln() as f32` (1h target).
  - `crates/forecast/src/tcn.rs:99` — `CONTEXT_LEN=256`.
  - `crates/forecast/src/tcn.rs:1090-1099` — RF=1021 bars (~42 days
    at hourly).
  - `crates/forecast/src/tcn.rs:302-311` — 1×1 conv head topology
    (K5 invariant).
  - `crates/forecast/src/bin/train_tcn.rs:733-741` — σ_train bug
    site (stays as-is; ADR-0035 contract).
  - `crates/forecast/src/bin/train_tcn.rs:87` — existing `epochs`
    CLI flag (`--target-horizon-bars` lands adjacent in scope a / c).
  - `crates/strategy/src/tcn_overlay_momentum.rs:391-404` — overlay
    direction extraction from `r_hat` + ε deadband (horizon-agnostic
    by construction).
  Confirmed: the existing scaffold is horizon-extensible with minimal
  changes (only the target derivation flips; topology + loss +
  optimiser + overlay composition all carry forward unchanged).

- [x] **T-A3** (2026-05-21) — Author `feature.md` brief.
  Frontmatter (`status: draft`, `owner: analyst`, `version: 0.1.0`,
  predecessor: `v25-tcn-threshold-tuning v0.1.0`, parent:
  `v25-tcn-overlay v2.5.0 (in-progress)`). R1-R8 requirements
  (scope-dependent annotation). Hypothesis register H1-H3. Risk
  register K1-K7. Open questions Q1-Q7 — **Q1 explicitly flagged
  as HARD BLOCKER, no safe analyst default**; Q2-Q7 carry analyst-
  recommended defaults. Non-regression contract (10 invariants).
  Acceptance per milestone (M-OD / M-T1 / M-RETRAIN / M-FORECAST-DIST
  / M-SHARPE / M-FINAL / M-PRESENTER). Cost estimate (per scope:
  ~7-10 days / ~4-6 weeks / ~6-9 weeks / ~30-90 days). Out-of-scope
  guardrails. Sources cited.

- [x] **T-A4** (2026-05-21) — Open `[[req]]` row in `spec/trace.toml`.
  `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001` in `draft` state. `arch`,
  `crates`, `tests`, `anchors` columns empty (architect / developer /
  tester fill at M-T1 / M-D / M-FINAL respectively). Predecessor
  chain `REQ-V25-TCN-THRESHOLD-TUNING-001 → REQ-V25-TCN-RECALIBRATE-001
  → REQ-V25-TCN-ALPHA-001 → REQ-V25-TCN-OVERLAY-001` documented in
  the title field.

- [x] **T-A5** (2026-05-21) — Promote Queue → Active in
  `spec/backlog.md`. Entry moved from `Queue § Strategy` (ACTIVATION
  TRIGGERED marker) to top of `## Active` block. Activation source:
  threshold-tuning ship's joint T-MARGINAL verdict + operator's
  routing (c) directive at the recalibrate presenter deck.

- [x] **T-A6** (2026-05-21) — Emit analyst handoff envelope.
  TOML envelope from=`analyst`, to=`operator`, verdict=`READY-FOR-
  OPERATOR-DECIDE`, with Q1-Q7 surfaced and **explicit flag** that
  Q1 (primary scope: a / b / c / d) is the load-bearing decision
  with NO safe analyst default. Predecessor signals
  (T-MARGINAL +0.018 / +0.045, F4-after-σ_train-fix, gate-survival
  0% → 40-89%) cited as motivating evidence.

## M-OD — Operator-decide (Q1-Q7) — resolved 2026-05-21

> **Q1 = (b)** locked by operator at 2026-05-21 via the orchestrator's
> hard-blocker prompt (no autoapprove default). The v2.5 TCN journey
> stack across 3 ships (alpha-investigation → recalibrate →
> threshold-tuning) has established that 1h-horizon TCN cannot extract
> alpha on real Binance OHLCV. Operator decision: **retire v2.5 TCN
> AT 1H HORIZON; pivot the multi-week budget to v2.5a PatchTST**
> (phase 2 of the 4-phase DL roadmap). Q2-Q7 are MOOT under scope (b)
> — no retrain happens, no new checkpoint, no new training anchor.

- [x] **T-OD1** — **Q1 = (b)** — Retire v2.5 TCN; promote `v25a-patchtst-overlay`
  Queue → Active. Operator rationale: 3 v2.5 TCN ships (alpha-investigation
  F4 + recalibrate σ_train-fixed-but-F4 + threshold-tuning T-MARGINAL)
  exhaust the 1h-horizon hypothesis; marginal +0.018 / +0.045 Sharpe-delta
  is below noise floor; better to invest the ~4-6 weeks in PatchTST than
  ~2-3 more weeks chasing another TCN horizon.
- [x] **T-OD2** — Q2 MOOT under Q1=(b). No retrain.
- [x] **T-OD3** — Q3 MOOT under Q1=(b). No new checkpoint.
- [x] **T-OD4** — Q4 MOOT under Q1=(b). No topology change.
- [x] **T-OD5** — Q5 MOOT under Q1=(b). No data span selection.
- [x] **T-OD6** — Q6 = (a) **RETIRE IMMEDIATELY**. The horizon-bump
  decision-threshold is preemptively triggered by the operator's
  Q1=(b) choice (skipping the horizon-bump entirely).
- [x] **T-OD7** — Q7 = no new anchors under this feature. 28 existing
  anchors stay byte-identical. v25a-patchtst-overlay will lock its
  own anchors under its own version string at its own ship cycle.

> **Once Q1 resolves**, frontmatter flips `status: draft → proposed`,
> `owner: analyst → architect`. The architect spawn proceeds from
> M-T1 with the scope-dependent T-AR rows below.

## Architect rows (T-AR) — scope-dependent, locked at M-T1

### Scope (a) / (c) — horizon-bump retrain

- [ ] **T-AR-1 (scope a/c)** — Lock § Design block in `feature.md` +
  `decomp.md`. Wave A: training scaffold extension (`--target-horizon-bars`,
  target-derivation extension, bounds check). Wave B: training run
  itself (long-running; orchestrator-monitored). Wave C: σ_train
  recalibration of new checkpoint. Wave D: forecast_distribution +
  backtest of new checkpoint. Wave E: tester gate.
- [ ] **T-AR-2 (scope a/c)** — Author ADR-0037
  `0037-tcn-horizon-bump-training-contract.md`. Codifies (D1)
  `--target-horizon-bars` CLI flag default (1) + valid range
  ({1, 24, 48, 168}); (D2) overlapping-vs-non-overlapping target
  policy (overlapping default per Q2a); (D3) the
  canonical-architecture-descriptor delta that lets `model_revision`
  SHA disambiguate horizon-bumped from 1h checkpoints; (D4)
  R3 cost tripwire (epoch wall-clock > 3× rolling median → escalate).
  Cross-references ADR-0029 (canonicaliser) + ADR-0035 (σ_train).
- [ ] **T-AR-3 (scope a/c)** — Decompose into T-D rows for Wave A
  (developer-callable). Estimated 4-8 T-D rows: (1) CLI flag,
  (2) target derivation extension, (3) bounds-check unit test,
  (4) default-invariance unit test (Q1=1 produces existing
  checkpoint SHA), (5) `train_events` row emission for cockpit
  visibility, (6) cost-tripwire assert.
- [ ] **T-AR-4 (scope a/c)** — K5 topology-immutability unit test
  designed. Asserts `CONTEXT_LEN == 256`, `CHANNELS == 96`,
  `N_BLOCKS == 8`, `DILATIONS == [1,2,4,8,16,32,64,128]`, `KERNEL_SIZE == 3`.
- [ ] **T-AR-5 (scope a/c)** — K6 anchor-neutrality test designed.
  CI gate runs `verify_anchors.sh` PRE and POST and asserts 28
  originals byte-identical.

### Scope (b) — retire-promote-PatchTST

- [ ] **T-AR-1 (scope b)** — Draft ADR-0036
  `0036-v25-tcn-retire-decision.md`. Codifies retire scope
  (research-mode), reversibility (status flip, not code deletion),
  evidence chain (F4 + σ_train fix + T-MARGINAL).
- [ ] **T-AR-2 (scope b)** — Prepare v25-tcn-overlay frontmatter for
  `status: in-progress → retired-research-mode-only` flip.
- [ ] **T-AR-3 (scope b)** — Queue PatchTST analyst-pass: add
  v25a-patchtst-overlay activation block to backlog § Active.
- [ ] **T-AR-4 (scope b)** — No code change. Tester gate is just
  anchor-neutrality + lint baseline.

### Scope (d) — defer-on-live

- [ ] **T-AR-1 (scope d)** — Draft ADR-0036-alt
  `0036-v25-tcn-defer-on-live.md`. Codifies defer-duration,
  decision-criteria, reversibility.
- [ ] **T-AR-2 (scope d)** — Queue `v25-tcn-live-trade-eval`
  follow-on Queue entry with defer-duration + decision criteria.
- [ ] **T-AR-3 (scope d)** — No code change. Tester gate is just
  anchor-neutrality + lint baseline.

## Developer rows (T-D) — scope-dependent

### Scope (a) / (c) — landed at M-T1; here as placeholders

- [ ] **T-D-N1 (scope a/c)** — Implement `--target-horizon-bars` CLI
  flag on `train_tcn.rs`. Default 1.
- [ ] **T-D-N2 (scope a/c)** — Extend target-derivation in
  `features.rs:627-628` to support `(close_{t+N} / close_t).ln()`
  for configured N. Bounds check on iterator `max_cursor`.
- [ ] **T-D-N3 (scope a/c)** — Unit test: default invocation
  (no `--target-horizon-bars`) produces `target_horizon_bars=1`
  metadata field + byte-identical existing checkpoint behaviour.
- [ ] **T-D-N4 (scope a/c)** — Unit test: K5 topology constants
  unchanged.
- [ ] **T-D-N5 (scope a/c)** — Unit test: K6 anchor-neutrality
  (28 originals byte-identical after merge).
- [ ] **T-D-N6 (scope a/c)** — Emit `train_events` rows per epoch
  per ADR-0034 so cockpit training-control surfaces progress.
- [ ] **T-D-N7 (scope a/c)** — Run training (LONG-RUNNING, ~5-7 days
  wall-clock). MUST emit watch recipe per MEMORY.md:
  ```bash
  watch -n 60 'tail -20 /tmp/train_tcn-h24-bs1.log && \
               echo "---" && \
               ps -p <PID> -o pcpu,pmem,etime,command | tail -2'
  ```
- [ ] **T-D-N8 (scope a/c)** — Run `recalibrate_sigma_train` against
  new checkpoint; emit `.metadata.recalibrated.json` overlay +
  derivation report.
- [ ] **T-D-N9 (scope a/c)** — Run `forecast_distribution
  --metadata-path <overlay>` against new checkpoint; emit
  `forecast-distribution-bs1-h<Q2>-realdata-recalibrated-<date>.md`
  report.
- [ ] **T-D-N10 (scope a/c)** — Run real-Binance backtest on new
  checkpoint via `backtest` scenario contract; emit
  `top10-2023-fy-tcn-overlay-h<Q2>-realdata-<date>.md` report.

### Scope (b) / (d) — no developer work in THIS feature

## Tester rows (T-T) — scope-dependent

### Scope (a) / (c)

- [ ] **T-T-1.a (scope a/c)** — Run `verify_anchors.sh` PRE + POST;
  assert 28 originals byte-identical + new anchors land under
  `v2.7.0-horizon-bump`.
- [ ] **T-T-1.b (scope a/c)** — 2-run byte-identity determinism gate
  on new forecast_distribution + backtest reports.
- [ ] **T-T-1.c (scope a/c)** — Workspace cargo fmt + clippy + test
  green.
- [ ] **T-T-1.d (scope a/c)** — Spec-lint baseline check.
- [ ] **T-T-1.e (scope a/c)** — H1 / H2 verdict recorded in
  `feature.md § Verification`.

### Scope (b) / (d)

- [ ] **T-T-1.a (scope b/d)** — Run `verify_anchors.sh` PRE + POST;
  assert 28 byte-identical (no new anchors).
- [ ] **T-T-1.b (scope b/d)** — Workspace cargo fmt + clippy + test
  green (sanity check; no code change).
- [ ] **T-T-1.c (scope b/d)** — Spec-lint baseline check.
- [ ] **T-T-1.d (scope b/d)** — ADR-0036 / ADR-0036-alt body
  cross-reference + retire / defer decision-record audit.

## Presenter rows (T-P) — all scopes

- [ ] **T-P-1** — Author presenter deck at
  `spec/v25-tcn-horizon-bump-or-retire/presentations/v25-tcn-horizon-bump-or-retire-<YYYY-MM-DD>.md`.
  Deck content per scope:
  - Scope (a) / (c): retrain verdict (F1 / F3 / F4) + Sharpe-delta
    + T-classifier (if backtest ran) + recommended next routing.
  - Scope (b): retire decision-record summary + PatchTST follow-on
    queued.
  - Scope (d): defer decision-record summary + live-trade window
    + decision criteria + follow-on `v25-tcn-live-trade-eval` queued.
- [ ] **T-P-2** — Operator approval; frontmatter flips `status:
  in-progress → shipped`; trace row + backlog flip Active → Recent.

## Parallelism map for the orchestrator

> Scope (a) and scope (b) are independent if scope (c) is chosen.

```
                                ┌─ scope (a) waves A→B→C→D→E (5-21 days)
M-OD (Q1) ── [operator] ────────┤
                                ├─ scope (b) waves A→B (PatchTST follow-on; ~4-6 weeks)
                                ├─ scope (c) = (a) || (b) parallel
                                └─ scope (d) waves A only (~30-90 days passive)
```

The orchestrator decides parallelism only AFTER Q1 resolves. Scope
(a) and (b) under (c) can run truly parallel — (a) is on compute
hardware; (b) is on analyst-write + architect-design human bandwidth.

## Watch recipe for long-running tasks (per MEMORY.md)

Sub-agents kicking off training runs MUST emit a copy-pasteable
`watch -n 60 '<probe>'` block. Template for the scope-(a) training run:

```bash
# Tail training log + show parent process resource usage.
# Replace <PID> with the actual cargo process PID (use `pgrep -f train_tcn`).
watch -n 60 'tail -30 /tmp/train_tcn-h24-bs1.log && \
             echo "---" && \
             ps -p <PID> -o pcpu,pmem,etime,command | tail -2 && \
             echo "---" && \
             ls -lh crates/forecast/checkpoints/anchors/tcn-bs1-h24-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
```

A separate `watch -n 300` (5-min cadence) probe for the cockpit
training-control panel is recommended to confirm `train_events`
rows are flowing.

## Anchor gate baseline (captured at analyst-spawn time)

```
$ bash scripts/verify_anchors.sh 2>&1 | grep -c '^PASS'
26
$ bash scripts/verify_anchors.sh 2>&1 | tail -1
ANCHORS FAIL  (mismatches detected; route HANDOFF -> developer with body diff)
```

> Recorded 2026-05-21 (analyst, this file). 26/28 anchors PASS;
> 2 FAILs (`forecast-distribution-bs1-realdata`,
> `forecast-distribution-bs2-realdata`) are **pre-existing glob-
> collision artefacts** from the v25-tcn-recalibrate ship — the
> recalibrated reports `forecast-distribution-bs{1,2}-realdata-
> recalibrated` glob-match the original anchor names. This was
> documented in the v25-tcn-threshold-tuning ship's anchor gate
> baseline (`spec/v25-tcn-threshold-tuning/feature.md § Anchor
> progression` — "2 FAILs are pre-existing glob-collision from
> v25-tcn-recalibrate — NOT introduced by this feature"). Same
> defence applies here: this analyst pass introduces no anchor
> changes. Architect re-captures at M-T1 spawn; tester re-captures
> at M-FINAL PRE-lock. The 26 PASS + 2 known-FAIL originals stay
> byte-identical throughout this feature.
