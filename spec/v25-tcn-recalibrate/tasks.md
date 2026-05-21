---
slug: v25-tcn-recalibrate
status: proposed
owner: architect
updated: 2026-05-21
---

# Tasks — v2.5 TCN σ_train recalibration

> Analyst-decomposed T-A rows landed 2026-05-21. Architect / developer
> / tester rows are placeholders for the next phase. M-R2 (recalibration
> bin) and M-R4 (re-run + verdict) are independent after M-R3 (architect
> lock) and can run in parallel.

## Analyst rows (T-A)

- [x] **T-A1** (2026-05-21) — Diagnose σ_train computation bug.
  Located the training-time accumulation site at
  `crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741` (per-batch
  predicted r_hat appended without inter-epoch reset → std dominated
  by pre-convergence trajectory variance). Located the inference-time
  read site at `crates/forecast/src/tcn.rs:534,937` (gate computation
  divides `r_hat.abs()` by the inflated scalar). Confirmed both sites
  operate on raw log-returns — the mismatch is **accumulation-window**,
  not **unit conversion**. Confirmed safetensors load
  (`crates/forecast/src/tcn.rs:541-548`) does not consume σ_train —
  metadata-only fix is feasible. Cited in `feature.md § Why`.

- [x] **T-A2** (2026-05-21) — Author `feature.md` brief.
  Frontmatter (status `draft`, owner `analyst`, version `0.1.0`,
  predecessor `v25-tcn-alpha-investigation v0.1.0`). R1-R8 requirements.
  Hypothesis register H1-H3. Risk register K1-K5. Open questions Q1-Q5
  with analyst-recommended defaults. Cost estimate (~4-5 hours wall-clock).
  Out-of-scope guardrails. Sources cited.

- [x] **T-A3** (2026-05-21) — Open `[[req]]` row in `spec/trace.toml`.
  `REQ-V25-TCN-RECALIBRATE-001` in `draft` state. `arch`, `crates`,
  `tests`, `anchors` columns empty (architect / developer / tester fill).

- [x] **T-A4** (2026-05-21) — Promote to `## Active` in `spec/backlog.md`.
  Entry placed directly above the recently-shipped
  `v25-tcn-alpha-investigation` row, citing the predecessor's
  presenter deck recommendation as the promotion source.

- [x] **T-A5** (2026-05-21) — Emit analyst handoff envelope.
  TOML envelope from=`analyst`, to=`operator`, verdict=`READY-FOR-OPERATOR-DECIDE`,
  with Q1-Q5 surfaced and the σ_train unit-mismatch diagnostic finding
  (file:line citation) attached.

- [ ] **T-A6** — Write the diagnosis dev-note.
  Path: `spec/dev-notes/analysis-2026-05-21-tcn-sigma-train-bug.md`.
  Content per R1: training-time site, inference-time site, unit
  consistency check, recommended fix shape. Deferred until operator
  approves the Q1-Q5 defaults so the dev-note matches the locked
  decisions. _Owner: analyst (this same author, in a follow-up tick)._

## M-OD — Operator-decide (Q1-Q5) — resolved 2026-05-21

> All 5 analyst-recommended defaults accepted in one tick via the
> operator's standing "Autoapprove all" directive (confirmed
> 2026-05-21 against the analyst hand-off envelope).

- [x] T-OD1 — Q1 = (a) re-derive σ_train once cleanly from the
  converged-model forward pass over the training-data span declared
  in `metadata.data_span`.
- [x] T-OD2 — Q2 = (a) metadata-only fix feasible (no `.safetensors`
  touch); architect formalises as unit test at T-AR-2.
- [x] T-OD3 — Q3 = (a) new anchor names `forecast-distribution-bs{1,2}-realdata-recalibrated`
  under new version `v2.6.1-alpha-investigation-recalibrated`;
  predecessor 22 anchors stay byte-identical (anchor-additive only).
- [x] T-OD4 — Q4 = (a) + (c) — keep ADR-0033 § D3 F-verdict algorithm
  immutable AND surface gate-survival delta (pre vs post recalibration)
  as a standalone `## Recalibration delta` body section regardless of
  F-label; operator routes on combined signal (honest reading per
  H2-refined: F-verdict may stay F4 since `frac_inside_epsilon` 0.031/
  0.057 < 0.5 threshold).
- [x] T-OD5 — Q5 = (a) inherit predecessor's
  `forecast_distribution_bin_readonly` 2-run byte-identity gate; no
  code change to the bin, only the metadata file changes — determinism
  preserved by construction.

## Architect rows (T-AR) — placeholders, architect-spawn after operator-approve

- [ ] **T-AR-1** — Lock § Design in `feature.md`. Cite ADR-0033 § D3
  for the immutable F-verdict algorithm; decide whether to ship a
  superseding ADR-0035 adding F3' (per Q4) or surface the gate-survival
  jump as a standalone `## Recalibration delta` section (analyst
  default). Lock the `recalibrate_sigma_train` bin CLI surface (R2),
  the `.metadata.recalibrated.json` overlay path convention (R3), and
  the read-only enforcement contract (K3).

- [ ] **T-AR-2** — If Q4 → (b) is chosen, write ADR-0035 superseding
  ADR-0033 § D3. Otherwise skip and cite ADR-0033 verbatim.

- [ ] **T-AR-3** — Decompose T-D rows under § "T-D rows" below.
  Architect should aim for 5-7 rows, each independently spawnable per
  the parallelism map.

## Developer rows (T-D) — placeholders, developer-spawn after T-AR-3

- [ ] **T-D-1** — Implement `recalibrate_sigma_train` bin skeleton +
  CLI surface (mirror `forecast_distribution.rs` shape per ADR-0033 § D1.a).
  No forward-pass logic yet.

- [ ] **T-D-2** — Forward-pass collection loop (load anchored
  checkpoint, iterate `windows_for_symbol()` over the training span,
  call `TcnModel::forward()`, accumulate r_hat values).

- [ ] **T-D-3** — Std computation + new metadata JSON emitter. Reuse
  the canonical-JSON canonicaliser from `crates/forecast/src/provenance.rs`
  per ADR-0029. Hard guard: only `sigma_train` field changes; all
  other fields (architecture, data_span, tokenisation, training,
  weights_sha256, model_revision, final_train_loss, final_val_loss,
  epochs_trained) copied verbatim from the original metadata.

- [ ] **T-D-4** — Recalibration-derivation report emitter (markdown
  under `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs{1,2}-<YYYYMMDD>.md`).
  Body shape: original σ_train, recalibrated σ_train, r_hat count, span,
  data revision SHA, model_revision, wire-format contrast.

- [ ] **T-D-5** — Read-only enforcement test (mirror
  `forecast_distribution_bin_readonly`). Asserts no `std::fs::write` /
  `OpenOptions::write(true)` lands outside `--out-dir` or the
  recalibrated-metadata target path. Asserts the original
  `.metadata.json` byte-content survives.

- [ ] **T-D-6** — Optional: extend `TcnForecaster::load_anchor()` to
  prefer `.metadata.recalibrated.json` over `.metadata.json` when both
  exist. Architect decides at T-AR-1 whether this is necessary or
  whether the recalibrate bin writes both files. Default: opt-in
  toggle so the F4-evidence reproducibility (running forecast_distribution
  with the original metadata) stays trivial — just delete the
  `.metadata.recalibrated.json`.

- [ ] **T-D-7** — Re-run `forecast_distribution` bin on both BS-1 +
  BS-2 under the new metadata. Two new reports under
  `spec/v25-tcn-recalibrate/reports/`. Verify the existing 22 anchors
  stay byte-identical via `bash scripts/verify_anchors.sh`.

## Tester rows (T-T) — placeholder, tester-spawn after T-D-7

- [ ] **T-T-1** — Anchor lock + non-regression verification + final report.
  - Anchor neutrality (R7): `bash scripts/verify_anchors.sh` → 22/22
    pre-lock; 24/24 (or 25/25) post-lock; 22 originals byte-identical.
  - Determinism (R8): 2-run byte-identity check on the recalibrate
    bin AND the re-run forecast_distribution bin.
  - Joint F-verdict published per ADR-0033 § D3.
  - Recalibration delta surfaced (gate-survival pre vs post) per Q4
    analyst default.
  - Test report path:
    `spec/v25-tcn-recalibrate/reports/test-<YYYYMMDD-HHMM>-v25-tcn-recalibrate.md`
    per the tester template.
  - Trace row `REQ-V25-TCN-RECALIBRATE-001` flips
    `draft` → `proposed` → `in-progress` → `shipped`; `crates`,
    `tests`, `anchors` columns filled by tester.

## Milestones

- [ ] **M-R1 — Diagnosis dev-note** (T-A6). Locked-down diagnostic
  artefact under `spec/dev-notes/`. Currently deferred to post-operator-
  approve so the dev-note matches the locked Q1-Q5 decisions.

- [ ] **M-R2 — Recalibration bin landed** (T-D-1..T-D-5). New bin
  emits the recalibrated metadata files + the derivation report.
  Acceptance: original `.metadata.json` files byte-identical; new
  files exist with corrected σ_train.

- [ ] **M-R3 — Re-classified forecast-distribution reports** (T-D-7).
  Two new reports on disk under `spec/v25-tcn-recalibrate/reports/`;
  both bodies byte-identical on a second run (R8 determinism); F-verdict
  label present per ADR-0033 § D3.

- [ ] **M-FINAL — Ship gate** (T-T-1). Anchor neutrality (R7) holds;
  recalibrated joint F-verdict recorded; operator disposition
  documented in `feature.md § Verification`.

## Parallelism map (architect refines at T-AR-3)

- **Wave A — Analyst** (sequential; this commit). T-A1 → T-A5 done;
  T-A6 deferred to post-operator-approve.
- **Wave B — Architect** (sequential after operator-approve). T-AR-1
  → T-AR-2 (conditional on Q4) → T-AR-3.
- **Wave C — Developer** (parallel after T-AR-3). T-D-1, T-D-2,
  T-D-3, T-D-4, T-D-5 form a tight dependency chain (T-D-2 needs T-D-1;
  T-D-3 needs T-D-2; T-D-4 needs T-D-3; T-D-5 is the guard test that
  runs against T-D-4's output). T-D-6 (optional) parallels T-D-3.
- **Wave D — Orchestrator** (sequential after Wave C). T-D-7 re-runs
  the existing forecast_distribution bin twice (BS-1 + BS-2); these
  two invocations are independent and can run in parallel.
- **Wave E — Tester** (sequential after Wave D). T-T-1 closes the
  gate.

### Critical path

T-A1 → T-A2 → operator-approve → T-AR-1 → T-D-1 → T-D-2 → T-D-3 →
T-D-7 (BS-1 + BS-2 parallel) → T-T-1.

Critical-path wall-clock estimate (per `feature.md § Cost estimate`):
~4-5 hours total.

## Out of scope for tasks.md

- Re-training. No T-D row touches the training loop. Forward-pass-
  against-frozen-weights only.
- ε / τ tuning. That's `v25-tcn-threshold-tuning` (separate spec) if
  the recalibration delta justifies it.
- Horizon bump. That's `v25-tcn-horizon-bump-or-retire` (separate
  spec) if the recalibrated F-verdict stays F4.
- Editing the existing `.metadata.json` files or `.safetensors` files
  in `crates/forecast/checkpoints/anchors/`. R7 hard guard.

## Notes

- The diagnostic finding cited in T-A1 is load-bearing for the
  feature's whole rationale; if it's falsified at T-D-3 (recalibrated
  σ_train doesn't land in the 0.005-0.025 range), the feature
  escalates back to analyst for a re-spawn.
- The F-verdict re-classification at T-D-7 may stay F4 (per H2's
  refined analysis — `frac_inside_epsilon` is well below 0.5 so F3
  won't fire under ADR-0033 § D3 priority tree as written). The
  honest reading per Q4 analyst default: surface the gate-survival
  jump as a standalone delta, keep the F-verdict algorithm immutable,
  let the operator route on the combined signal.

## Changelog

- 2026-05-21 (analyst): T-A1..T-A5 done at brief-write time.
  Diagnostic finding committed at `crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`.
  T-A6 (diagnosis dev-note) deferred to post-operator-approve so the
  note matches locked Q1-Q5 decisions. Architect / developer / tester
  rows are placeholders for the next phase. HANDOFF → operator-decide
  (Q1-Q5) → architect.
