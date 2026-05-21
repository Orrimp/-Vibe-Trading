---
slug: v25-tcn-threshold-tuning
status: proposed
owner: architect
updated: 2026-05-21
---

# Tasks — v2.5 TCN threshold tuning

> Analyst-decomposed T-A rows landed 2026-05-21. Architect / developer
> / tester rows are placeholders for the next phase. The 90-backtest
> sweep can run in parallel after the developer wave lands the bin.

## Analyst rows (T-A)

- [x] **T-A1** (2026-05-21) — Read predecessor materials.
  Confirmed: recalibrate ship shipped 2026-05-21 with joint F-verdict
  F4 + gate-survival jump (BS-1 τ=0.6: 0% → 40.1%; τ=0.1: 0% → 88.8%;
  BS-2 τ=0.6: 0% → 34.5%; τ=0.1: 0% → 86.4%). Operator routing chose
  option (c) — threshold-tuning first, horizon-bump as fallback.
  Cited: `spec/v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md`,
  `spec/v25-tcn-recalibrate/feature.md § Verification`,
  `spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md § D3`
  (immutable F-verdict).

- [x] **T-A2** (2026-05-21) — Locate the canonical τ + ε knob sites.
  `confidence_threshold` ships as `dec!(0.6)` literal in
  `crates/strategy/src/tcn_overlay_momentum.rs:417-420`
  (`with_tcn_bs1_ledger`) and `:434-437` (`with_tcn_bs2_ledger`),
  both passing through to `Self::new(base, Box::new(forecaster),
  dec!(0.6))`. The deadband ε defaults to `0.0005` per
  `spec/v25-tcn-overlay/feature.md § R6`; the `combine_with_direction`
  gate body lives near `crates/strategy/src/tcn_overlay_momentum.rs:~145-170`
  (architect confirms exact range at M-T1). `confidence_gate_survival`
  array at `crates/forecast/src/bin/forecast_distribution.rs:325`
  already sweeps τ ∈ {0.1..0.9} integer-tenths — Q1 = (a) reuses this
  grid for free. Cited in `feature.md § Why` and § R1.

- [x] **T-A3** (2026-05-21) — Author `feature.md` brief.
  Frontmatter (`status: draft`, `owner: analyst`, `version: 0.1.0`,
  predecessor: `v25-tcn-recalibrate v0.1.0`, parent: `v25-tcn-overlay
  v2.5.0 (in-progress)`). R1-R9 requirements. Hypothesis register
  H1-H3. Risk register K1-K6. Open questions Q1-Q6 with analyst-
  recommended defaults. Cost estimate (~6-10 hours wall-clock).
  Out-of-scope guardrails. Sources cited.

- [x] **T-A4** (2026-05-21) — Open `[[req]]` row in `spec/trace.toml`.
  `REQ-V25-TCN-THRESHOLD-TUNING-001` in `draft` state. `arch`,
  `crates`, `tests`, `anchors` columns empty (architect / developer /
  tester fill). Predecessor `REQ-V25-TCN-RECALIBRATE-001` referenced
  in title.

- [x] **T-A5** (2026-05-21) — Promote to `## Active` in
  `spec/backlog.md`. Entry placed at the top of the Active block,
  directly above the recently-shipped `v25-tcn-recalibrate` row /
  the live `v25-tcn-alpha-investigation` placeholder. Cites the
  predecessor's presenter deck routing-(c) choice as the promotion
  source.

- [x] **T-A6** (2026-05-21) — Add `v25-tcn-horizon-bump-or-retire`
  stub to `spec/backlog.md § Queue § Strategy`. Stub-only (no feature
  folder yet); pointer back to this brief's verdict-driven activation.
  Trigger condition: joint T-verdict on this feature returns
  `T-NO-ALPHA` (or `T-MARGINAL` with operator concurrence) at
  M-FINAL.

- [x] **T-A7** (2026-05-21) — Emit analyst handoff envelope.
  TOML envelope from=`analyst`, to=`operator`, verdict=`READY-FOR-
  OPERATOR-DECIDE`, with Q1-Q6 surfaced and the gate-survival jump
  finding (BS-1 0% → 40.1% at τ=0.6) cited as the predecessor signal
  that motivates this feature.

## M-OD — Operator-decide (Q1-Q6) — resolved 2026-05-21

> All 6 analyst-recommended defaults accepted in one tick via the
> operator's standing "Autoapprove all" directive (confirmed
> 2026-05-21 against the analyst hand-off envelope).

- [x] T-OD1 — Q1 = (a) 9 integer-tenths `{0.1, 0.2, …, 0.9}` τ grid;
  reuses existing `confidence_gate_survival` array at
  `crates/forecast/src/bin/forecast_distribution.rs:325`.
- [x] T-OD2 — Q2 = (a) 5-cell ε grid
  `{0.0001, 0.0005 baseline, 0.001, 0.005, 0.01}`; covers 2 orders
  of magnitude of r_hat std.
- [x] T-OD3 — Q3 = (a) realdata only (`v2.6.0-realdata` baseline);
  the predecessor F-verdict was on realdata so threshold-tuning needs
  apples-to-apples Sharpe comparison.
- [x] T-OD4 — Q4 = (c) embed T-classifier in report body; defer
  ADR-0036 until empirical alpha-unlock evidence justifies
  codification. ADR-0033 § D3 F-verdict algorithm stays IMMUTABLE.
- [x] T-OD5 — Q5 = (c) additive
  `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` builder; existing
  `with_tcn_bs{1,2}_ledger` builders stay literal `dec!(0.6)` (no
  behavioral change for existing callers; 26 predecessor anchors
  byte-identical).
- [x] T-OD6 — Q6 = (a) anchor heatmaps eagerly under version
  `v2.6.2-threshold-tuning`; new anchors
  `threshold-sweep-bs{1,2}-realdata-recalibrated` (and potentially
  per-cell tuned-winner anchors if H1 unlocks).

## Architect rows (T-AR) — PENDING (architect at M-T1)

- [ ] **T-AR-1** — § Design block in `feature.md`. Locks all Q-defaults
  consumed; canonical decomposition lives at
  `spec/v25-tcn-threshold-tuning/decomp.md`. Lock per-cell
  parallelisation contract (order-invariant cell collection per
  R9 + K3).
- [ ] **T-AR-2** — Decide whether to author ADR-0036 (Q4 outcome). If
  (a) or (c), one-line rationale lives in feature.md § Design.
- [ ] **T-AR-3** — Decide bin location: new
  `crates/forecast/src/bin/threshold_sweep.rs` (analyst-recommended) vs.
  extension of existing `forecast_distribution.rs` /
  `sharpe_comparison.rs`. Either path keeps the predecessor's 4 anchored
  bodies byte-identical.
- [ ] **T-AR-4** — Decide backtest wiring: (a) new `--tcn-tau` +
  `--tcn-epsilon` flags on backtest CLI vs. (b) in-process via
  `_tuned(τ, ε)` builder. Either path keeps the predecessor's 22
  `top10-{2023,2024}-fy-tcn-overlay-realdata` bodies byte-identical
  under default invocation.
- [ ] **T-AR-5** — Decompose developer T-D rows into Wave A (sweep bin +
  heatmap renderer) + Wave B (`_tuned` builder + tuned-backtest re-run
  if R4 fires). Add tester T-T rows for the M-FINAL gates.
- [ ] **T-AR-6** — Frontmatter flips `status: draft → in-progress`,
  `owner: analyst → architect`.

## Developer rows (T-D) — PENDING (developer at M-D)

- [ ] **T-D-N1..T-D-Nk** — Wave A (sweep bin) — architect-decomposed.
- [ ] **T-D-Nk+1..T-D-Nm** — Wave B (`_tuned` builder + tuned-backtest
  re-run if R4 fires) — architect-decomposed.

## Tester rows (T-T) — PENDING (tester at M-FINAL)

- [ ] **T-T-1..T-T-N** — anchor lock, non-regression check (26 → 28 or
  30), determinism gate, joint T-verdict record — architect-decomposed.

## Presenter rows (T-P) — PENDING (presenter at M-PRESENTER)

- [ ] **T-P-1** — presenter deck under
  `spec/v25-tcn-threshold-tuning/presentations/v25-tcn-threshold-tuning-
  <YYYY-MM-DD>.md`.
- [ ] **T-P-2** — operator approval; frontmatter flips `in-progress →
  shipped`; trace row flips `draft → shipped`.

## Notes

- The 9 × 5 grid = 45 backtest cells per checkpoint × 2 checkpoints =
  90 backtest runs. At ~30s per realdata run, ~45 min single-threaded
  or ~12 min 4-way local.
- The recalibrated metadata overlay files
  (`tcn-bs{1,2}-<sha>.metadata.recalibrated.json`) are read-only inputs;
  R5 prohibits any mutation.
- The 26 predecessor anchors are the load-bearing invariant (R8).
  `bash scripts/verify_anchors.sh` must report `26/26` PRE-lock at
  architect-spawn time AND at developer-handoff time. POST-lock target
  is `28/28` (T-NO-ALPHA / T-MARGINAL) or `30/30` (T-ALPHA-UNLOCKED).
