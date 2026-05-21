---
slug: v25-tcn-threshold-tuning
status: in-progress
owner: developer
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

## Architect rows (T-AR) — DONE (architect at M-T1, 2026-05-21)

- [x] **T-AR-1** (2026-05-21) — § Design block locked in `decomp.md`
  §§ 1 (D-AR-1.a..D-AR-1.j) + § 2 (file change-map). Bin name +
  location: `crates/forecast/src/bin/threshold_sweep.rs` (new). CLI
  surface: 5 args (`--scenario`, `--data-root`, `--metadata-path`,
  `--out-dir`, `--expected-revision-sha`). Per-cell parallelism
  contract: 4-way `rayon::par_iter`, deterministic via `(τ, ε)`-sorted
  assembly before render (R9 / K3). 45 cells per checkpoint × 2 =
  90 backtests total. `feature.md § Design` cross-pointer to be
  appended at the same M-T1 commit. File:line target:
  `spec/v25-tcn-threshold-tuning/decomp.md:1-650`.

- [x] **T-AR-2** (2026-05-21) — Report shape locked in `decomp.md
  § D-AR-1.h`. Two heatmaps under
  `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md`.
  Body shape: 4 heatmaps (A=Sharpe-delta, B=return-delta, C=max-DD,
  D=gate-survivor 1D) + headline cell + smoothness statistic (H2
  guard) + T-classifier verdict. Floating-point format rules locked
  (6-dp Sharpe, 2-dp return-pct, signed prefix for deltas, integer
  counts). T-ALPHA-UNLOCKED ≥ +0.10 / T-MARGINAL ∈ [0, +0.10) /
  T-NO-ALPHA < 0 (analyst defaults retained). File:line target:
  `spec/v25-tcn-threshold-tuning/decomp.md` § D-AR-1.h, § D-AR-1.i.

- [x] **T-AR-3** (2026-05-21) — Parallelism map locked in `decomp.md
  § D-AR-1.j`. 4-way `rayon::par_iter` over 45 cells per checkpoint;
  shared `Vec<Bar>` read-only; fresh `TcnForecaster` per cell (load
  ~150-300ms × 45 ≈ ~7-14s overhead — accepted for determinism
  guarantee). Sort by `(τ, ε)` lexicographic key BEFORE render
  (order-invariant body assembly). 2-run byte-identity gate at
  T-T-1.a. Wall-clock estimate 8-12min per checkpoint at 4-way; 10-16min
  for both. File:line target: `spec/v25-tcn-threshold-tuning/decomp.md`
  § D-AR-1.j.

- [x] **T-AR-4** (2026-05-21) — Tuned-builder API locked in `decomp.md
  § D-AR-1.f` + § D-AR-1.g. 4 new additive builders:
  `with_tcn_bs{1,2}_tuned(τ, ε)` under `feature = "forecast"`;
  `with_tcn_bs{1,2}_ledger_tuned(ledger, τ, ε)` under `feature =
  "forecast-audit-tick"`. **Explicit args required** (no
  `Option<Decimal>` cascading defaults). New
  `TcnSyncForecaster::with_direction_epsilon(eps)` builder + new
  `direction_epsilon: Option<f32>` field; `infer()` body lines 305-307
  read `self.direction_epsilon.unwrap_or(forecast::tcn::DIRECTION_EPSILON)`.
  Default path (`None`) const-fold-identical for existing 4 builders.
  26 predecessor anchors stay byte-identical (K4 / R8). File:line
  target: `crates/strategy/src/tcn_overlay_momentum.rs:158-214,
  305-307, 441-530` (new lines 441-530 are additive; existing 413-440
  unchanged).

- [x] **T-AR-5** (2026-05-21) — Anchor strategy locked. Q6=(a)
  confirmed at M-T1: 2 new anchors at M-FINAL under version
  `v2.6.2-threshold-tuning`. Tuned-winner per-cell anchors **deferred
  to follow-on v2.5.1** if `T-ALPHA-UNLOCKED` fires (architect
  authors the follow-on after this feature's M-FINAL outcome).
  Anchor count progression: 26 (pre) → 28 (post) regardless of
  T-verdict. File:line target: `spec/v25-tcn-threshold-tuning/decomp.md`
  § T-AR-5.

- [x] **T-AR-6** (2026-05-21) — Spike requirement: **NONE**. ADR-0036:
  **NOT WRITTEN** per Q4=(c) closure (T-classifier embeds in report
  body only). 1-line rationale in `feature.md § Design`. File:line
  target: `spec/v25-tcn-threshold-tuning/decomp.md § 4` (spike) +
  `feature.md § Design § ADR decision` (M-T1 commit).

### Anchor gate baseline (T-AR-3 § baseline)

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -3
PASS  recalibrate-sigma-train-bs1           baa658fb7ad96796f643d8fecab9156362b17faad97afc37be77867850336ad9
PASS  recalibrate-sigma-train-bs2           bfa8104ace81dd6a98f42a65cd0a5bd584089fa93fbafa4aa6f11d02954b47e0
---
ANCHORS FAIL  (mismatches detected; route HANDOFF -> developer with body diff)
```

**Honest disclosure**: the literal output reports `ANCHORS FAIL`. The
2 FAIL lines (`forecast-distribution-bs1-realdata` /
`forecast-distribution-bs2-realdata`) are a pre-existing
`scripts/verify_anchors.sh` glob-resolver collision from the
recalibrate ship — NOT introduced by this feature. The 26 individual
file bodies are byte-identical to their locked SHAs (verified
directly via `python3 scripts/hash_report.py …`):

```
$ python3 scripts/hash_report.py \
    spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md \
    spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260519.md
ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54  …forecast-distribution-bs1-realdata-20260519.md
d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06  …forecast-distribution-bs2-realdata-20260519.md
```

Both match the locked SHAs at `spec/anchors.toml:158, 163`. The
glob-resolver bug at `scripts/verify_anchors.sh:45` (`"*/reports/$scenario-*.md"`
greedy-matches the newer `…recalibrated-…md` file) is flagged to the
orchestrator as a **spec-auditor punch-list item** (recalibrate-ship
tester to address). M-T1 architect proceeds — this feature is
anchor-additive at the body level; the script collision is parallel.
See `decomp.md § 6` for the full root-cause analysis.

## Developer rows (T-D) — Wave A (developer at M-D)

- [ ] **T-D-N1** — `with_direction_epsilon` builder + `direction_epsilon:
  Option<f32>` field on `TcnSyncForecaster`. `infer()` lines 305-307:
  `let eps = self.direction_epsilon.unwrap_or(forecast::tcn::DIRECTION_EPSILON);
  if r_hat > eps { … }`. File:line:
  `crates/strategy/src/tcn_overlay_momentum.rs:158-214,305-307`
  (modify). Cargo:
  `cargo build -p strategy --features forecast`. Expected:
  `Compiling strategy …` then `Finished … profile [optimized] target(s)
  in <Ns>` — no warnings.

- [ ] **T-D-N2** — 4 `_tuned` builders: `with_tcn_bs{1,2}_tuned(τ, ε)`
  under `feature = "forecast"` + `with_tcn_bs{1,2}_ledger_tuned(ledger,
  τ, ε)` under `feature = "forecast-audit-tick"`. Additive after line
  440. File:line: `crates/strategy/src/tcn_overlay_momentum.rs:441-530`
  (new). Cargo:
  `cargo build -p strategy --features forecast,forecast-audit-tick`.
  Expected: `Compiling strategy …` `Finished … in <Ns>`.

- [ ] **T-D-N3** — Unit tests for builder default-invariance +
  tuned-passthrough. 5 tests: (1) `with_tcn_bs1.confidence_threshold
  == dec!(0.6)`; (2) `with_tcn_bs1.direction_epsilon == None` (probe
  via getter or `Debug`); (3) `with_tcn_bs1_tuned(τ,
  ε).confidence_threshold == τ`; (4) `with_tcn_bs1_tuned(τ,
  ε).direction_epsilon == Some(ε.to_f32())`; (5) ditto for BS-2.
  File:line: `crates/strategy/tests/tcn_overlay_tuned_builder.rs:1-120`
  (new). Cargo:
  `cargo test -p strategy --features forecast --test tcn_overlay_tuned_builder`.
  Expected: `running 5 tests … test result: ok. 5 passed; 0 failed`.

- [ ] **T-D-N4** — Bin skeleton + CLI surface per D-AR-1.d. Add
  `[[bin]]` entry to `crates/forecast/Cargo.toml` mirror of
  `recalibrate_sigma_train`. File:line:
  `crates/forecast/src/bin/threshold_sweep.rs:1-130` (new) +
  `crates/forecast/Cargo.toml:+5`. Cargo:
  `cargo run -p forecast --features candle --bin threshold_sweep -- --help`.
  Expected: help text containing `--scenario`, `--data-root`,
  `--metadata-path`, `--out-dir`, `--expected-revision-sha`; NO
  `retrain`/`update`/`write-checkpoint`/`write-metadata` substrings.

- [ ] **T-D-N5** — Thin `run_cell` helper in `crates/backtest/src/scenarios/threshold_sweep.rs`
  (D-AR-1.c). Behavior-preserving copy of `tcn_overlay_weights::run`
  with caller-supplied strategy. Re-uses
  `momentum::top10_symbols_with_prices` + the realdata bar loader.
  File:line: `crates/backtest/src/scenarios/threshold_sweep.rs:1-110`
  (new) + `crates/backtest/src/scenarios/mod.rs:+1`. Cargo:
  `cargo build -p backtest --features candle,realdata`. Expected:
  `Compiling backtest …` `Finished … in <Ns>`.

- [ ] **T-D-N6** — Bin body — grid enumeration (D-AR-1.e) + parallel
  cell execution (D-AR-1.j). `rayon::par_iter` over 45 cells; fresh
  forecaster per cell; sort by `(τ, ε)` BEFORE render. File:line:
  `crates/forecast/src/bin/threshold_sweep.rs:130-300` (new). Cargo:
  `cargo build -p forecast --features candle --bin threshold_sweep`.
  Expected: `Compiling forecast …` `Finished … in <Ns>` — no warnings.

- [ ] **T-D-N7** — Heatmap renderer (D-AR-1.h). Reads pre-recal v1-momentum
  baseline metrics from anchored body
  `spec/v1-cross-sectional-momentum/reports/backtest-*-top10-2023-1h-momentum.md`
  (resolves via the anchor name; uses
  `crates/reports/src/parse.rs::Metrics` parser). Reads
  `confidence_gate_survival` table from anchored predecessor
  `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`.
  Emits the markdown report. **Long-running (~8-12min at 4-way) —
  background + watch recipe required.** File:line:
  `crates/forecast/src/bin/threshold_sweep.rs:300-450` (new). Cargo:
  ```
  cargo run -p forecast --features candle,realdata --release --bin threshold_sweep -- \
      --scenario bs1 \
      --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json
  ```
  AND same with `--scenario bs2 --metadata-path …tcn-bs2-3fabcabe…metadata.recalibrated.json`.
  Expected: `INFO threshold_sweep: scenario=bs1 cells=45 headline_tau=<%.6f>
  headline_eps=<%.6f> sharpe_delta=<%+.6f> verdict=T-<LABEL>
  wall_clock_s=<%.1f>` AND markdown files at
  `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md`.

  **Watch recipe (REQUIRED for >2min cargo runs per MEMORY.md):**
  ```bash
  watch -n 30 'tail -n 60 /tmp/threshold-sweep-bs1.log; \
               echo "---"; \
               ls -la spec/v25-tcn-threshold-tuning/reports/ 2>/dev/null'
  ```

- [ ] **T-D-N8** — Read-only enforcement tests for the new bin.
  Mirror of `recalibrate_sigma_train_readonly.rs`: (1) help surface
  has no `retrain|write|update` substrings; (2) `tcn-bs{1,2}-<sha>.{safetensors,metadata.json,metadata.recalibrated.json}`
  mtimes unchanged after a sweep run (use a small fixture run or a
  CLI dry-mode if added). File:line:
  `crates/forecast/tests/threshold_sweep_readonly.rs:1-120` (new).
  Cargo:
  `cargo test -p forecast --features candle,realdata --test threshold_sweep_readonly`.
  Expected: `running 2 tests … test result: ok. 2 passed; 0 failed`.

- [ ] **T-D-N9** — Workspace clippy + fmt gate.
  File:line: workspace-wide. Cargo:
  `cargo fmt --check` then
  `cargo clippy --workspace --features candle,realdata,forecast,forecast-audit-tick -- -D warnings`.
  Expected: both commands exit 0; clippy ends with
  `Checking … Finished `dev` profile [unoptimized + debuginfo]
  target(s) in <Ns>` and zero warnings.

## Developer / orchestrator rows — Wave B (orchestrator at M-D)

- [ ] **T-D-N10** — 2-run determinism prep (orchestrator). Re-run
  T-D-N7 against both BS-1 + BS-2; compare body-SHAs against the first
  run. File:line: `spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md`.
  Cargo: `python3 scripts/hash_report.py spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md`
  (twice; SHAs match). Expected: `<64hex>  threshold-sweep-bs1-realdata-recalibrated-20260521.md`
  literal-stable.

- [ ] **T-D-N11** — Post-Wave-A anchor verification. Captures literal
  output. File:line: (script). Cargo: `bash scripts/verify_anchors.sh`.
  Expected: 24 PASS rows of the M-T1 baseline + 2 PASS rows for new
  heatmaps (`threshold-sweep-bs{1,2}-realdata-recalibrated`) ⇒ 26 PASS
  + 2 pre-existing spurious FAILs (per § 6 glob collision) until the
  spec-auditor punch-list item is closed. If glob fixed first: 28/28
  PASS clean.

## Tester rows (T-T) — Wave C (tester at M-FINAL)

- [ ] **T-T-1.a** — 2-run byte-identity determinism gate on both
  heatmap reports + on the 4 predecessor recalibrate-ship anchored
  bodies (regression-safety). File:line: heatmap files + predecessor
  files. Cargo:
  `python3 scripts/hash_report.py spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md`
  (each twice; SHAs match). Expected: 6 SHAs stable across 2 runs.

- [ ] **T-T-1.b** — Anchor-additive lock: append 2 rows to
  `spec/anchors.toml` under version `v2.6.2-threshold-tuning`. File:line:
  `spec/anchors.toml:199+` (append). Cargo:
  `bash scripts/verify_anchors.sh`. Expected: `ANCHORS PASS  (28 / 28)`
  IF the pre-existing § 6 glob collision is first triaged + fixed by
  the spec-auditor; otherwise `ANCHORS FAIL` with the same 2 spurious
  FAILs + 26 PASS rows.

- [ ] **T-T-1.c** — Anchor-neutrality check: 26 originals body-SHA
  byte-identical to baseline. File:line: anchored files. Cargo:
  `for f in $(find spec -name 'forecast-distribution-bs*realdata-2026051*.md' -o -name 'sharpe-comparison-realdata-*.md' -o -name '*.md' -path '*/reports/*'); do python3 scripts/hash_report.py "$f"; done`
  (or simpler: `bash scripts/verify_anchors.sh | grep -E "^(PASS|FAIL)"`).
  Expected: all 26 lines match the M-T1 baseline (modulo the 2 § 6
  pre-existing FAILs, which the file-direct hashes show are byte-identical).

- [ ] **T-T-1.d** — Joint T-verdict recorded in `feature.md §
  Verification`. Routing per § R3 table. File:line:
  `spec/v25-tcn-threshold-tuning/feature.md § Verification` (append).
  Expected: `Joint verdict: T-<LABEL>` + `Operator routing: <decision>`
  lines added.

- [ ] **T-T-1.e** — Trace row flipped to `shipped`. `crates`, `tests`,
  `anchors` columns populated. File:line: `spec/trace.toml:222-231`.
  Cargo: `python3 scripts/spec_brief.py v25-tcn-threshold-tuning --check-trace`.
  Expected: `state = "shipped"` + non-empty arrays.

- [ ] **T-T-1.f** — Tester report under
  `spec/v25-tcn-threshold-tuning/reports/test-<YYYYMMDD-HHMM>-v25-tcn-threshold-tuning.md`
  per `.claude/skills/rust-test/templates/test-report.md`. Carries
  the 26 → 28 anchor-progression line literal. File:line:
  `spec/v25-tcn-threshold-tuning/reports/test-<YYYYMMDD-HHMM>-v25-tcn-threshold-tuning.md`
  (new). Cargo: `bash scripts/verify_anchors.sh`. Expected: tester
  report cites `ANCHORS PASS  (28 / 28)` as the post-lock literal
  (or the 26 file-direct hashes if the glob-collision is still open).

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
