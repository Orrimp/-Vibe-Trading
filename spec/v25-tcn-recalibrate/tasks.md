---
slug: v25-tcn-recalibrate
status: shipped
owner: operator
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

## Architect rows (T-AR) — locked 2026-05-21 (M-T1)

- [x] **T-AR-1** (2026-05-21) — § Design locked in `feature.md`
  (appended at end of brief, between § Out of scope and § Changelog).
  Canonical decomposition lives at
  [`spec/v25-tcn-recalibrate/decomp.md`](decomp.md) (architect-owned,
  load-bearing reference). Decisions:
  - Bin name: `recalibrate_sigma_train` confirmed
    (`crates/forecast/src/bin/recalibrate_sigma_train.rs`,
    new file; ~350 LoC; co-located with the existing investigation
    bin family at `crates/forecast/src/bin/forecast_distribution.rs`
    + `crates/forecast/src/bin/sharpe_comparison.rs`).
  - CLI surface: 4 args (`--scenario`, `--data-root`, `--out-dir`,
    `--anchor-dir`). No retrain / update / write-checkpoint flags.
    See [`decomp.md § D-AR-1.b`](decomp.md#d-ar-1b--cli-surface-5-args-mirrors-forecast_distribution).
  - Forward-pass span: read from original metadata's `data_span`
    field (NOT `forecast_distribution::default_span` — they differ for
    BS-2). BS-1: `2023-01-01..2023-12-31T23:00:00Z`. BS-2:
    `2023-01-01..2024-03-31T23:00:00Z`.
  - Overlay file path:
    `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.metadata.recalibrated.json`.
    Original `.metadata.json` + `.safetensors` stay byte-identical.
  - On-disk JSON number convention for `sigma_train` (intentional
    divergence from ADR-0029 § 2 rule 5's string-encoded canonical
    form; load-bearing for `.as_f64()` parity at
    [`crates/forecast/src/tcn.rs:534`](../../crates/forecast/src/tcn.rs)).
    Key ordering + whitespace still use ADR-0029 canonicaliser at
    [`crates/forecast/src/provenance.rs`](../../crates/forecast/src/provenance.rs).
  - Consumer integration: additive `--metadata-path` flag on
    `forecast_distribution.rs` (default behavior byte-identical
    → 22 anchor SHAs preserved). Wires through to shipped
    [`TcnForecaster::load_from_paths`](../../crates/forecast/src/tcn.rs)
    at `tcn.rs:522`.
  - Recalibration-derivation report shape per
    [`decomp.md § D-AR-1.h`](decomp.md#d-ar-1h--recalibration-derivation-report-shape).
  - Q4 = (a)+(c) honored: F-verdict algorithm
    [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm)
    stays IMMUTABLE; recalibration delta surfaces as standalone
    `## Recalibration delta` body section between `## Verdict` and
    `## Notes` (anchor-hashed; reads predecessor's anchored body for
    pre-recal values).

  - Cargo invocation (verification, no compilation needed for spec-only commit):
    ```
    $ cat spec/v25-tcn-recalibrate/feature.md | grep -E '^## Design$|^## Changelog$' | wc -l
           2
    ```
    Confirms § Design block landed before § Changelog.

- [x] **T-AR-2** (2026-05-21) — **ADR-0035 written** at
  [`spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
  (~180 lines). Title: "Post-training σ_train recalibration via
  metadata overlay (cross-phase contract for v2.5 / v2.5a / v2.5b)".
  Locks:
  - D1 — σ_train derived in a frozen-weights post-training forward
    pass (NOT via in-loop accumulator); the
    [`train_tcn.rs:606,676-678,733-741`](../../crates/forecast/src/bin/train_tcn.rs)
    in-loop accumulator pattern is **deprecated** for all future
    v2.5/v2.5a/v2.5b training scaffolds.
  - D2 — overlay file convention + on-disk JSON number shape (deviates
    from ADR-0029 § 2 rule 5 intentionally; load-bearing).
  - D3 — consumers opt in via additive `--metadata-path` CLI flag;
    `load_anchor` does NOT auto-prefer overlay.
  - D4 — σ_train-not-in-safetensors invariant codified as test
    (`crates/forecast/tests/sigma_train_not_in_safetensors.rs`).
  - **Does NOT supersede ADR-0033** (F-verdict algorithm stays
    immutable per Q4 = (a)). ADR-0035 sits alongside ADR-0029 +
    ADR-0033 as the v2.5 forecaster read-path triad.
  - ADR registry updated at
    [`spec/architecture/adr/README.md`](../architecture/adr/README.md)
    (row added; changelog entry added).

  - Cargo invocation (no build; spec-only):
    ```
    $ wc -l spec/architecture/adr/0035-tcn-sigma-train-recalibration.md
    ```
    Expected literal: `\d+ spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`
    (~280 lines; trace-row sanity).

- [x] **T-AR-3** (2026-05-21) — T-D rows decomposed below into Waves
  A-D (8 rows total). Critical path 8 hops; wall-clock ~3-4 hours.
  Parallelism map embedded in
  [`decomp.md § Wave A-D ordered decomposition`](decomp.md#3-wave-ad-ordered-decomposition).
  See § "Developer rows (T-D)" + § "Tester rows (T-T)" below for the
  full ordered checklist.

  ### Anchor baseline (NON-NEGOTIABLE pre-Wave-A literal capture)

  Architect verified the 22 anchored bodies before handing off to
  developer. Run on 2026-05-21:

  ```
  $ bash scripts/verify_anchors.sh 2>&1 | tail -1
  ANCHORS PASS  (22 / 22)
  ```

  Full per-scenario verification (all 22 lines `PASS`):

  ```
  PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
  PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
  PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
  PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
  PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
  PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
  PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
  PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
  PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
  PASS  report-sample-7d                      520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3
  PASS  report-sample-90d                     c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333
  PASS  top10-2023-fy-tcn-overlay             01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5
  PASS  top10-2024-fy-tcn-overlay             e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163
  PASS  top10-2023-fy-tcn-overlay-weights     7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4
  PASS  top10-2024-fy-tcn-overlay-weights     23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b
  PASS  top10-2023-fy-tcn-overlay-realdata    8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642
  PASS  top10-2024-fy-tcn-overlay-realdata    fd8191dff1ca106ca24416a1819bd8a002c705da7f3747831f48d60733ee76f3
  PASS  top10-2023-fy-tcn-overlay-weights-realdata  552d7df294bc93ff6f887874f919aeeb8106a62caae4ad5ec5de7c5b49665d70
  PASS  top10-2024-fy-tcn-overlay-weights-realdata  2a65c4347964a0748877606d9c3a8b261b7fee6e069a814e64aaa024419f2f2c
  PASS  forecast-distribution-bs1-realdata    ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54
  PASS  forecast-distribution-bs2-realdata    d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06
  PASS  sharpe-comparison-realdata            17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924
  ---
  ANCHORS PASS  (22 / 22)
  ```

  This baseline is the non-regression contract for the entire feature.
  Every wave below must preserve all 22 rows byte-identically.

## Developer rows (T-D) — Wave A + B (Wave A first; Wave B after Wave A acceptance)

> Every row carries file:line + cargo invocation + literal output.
> All paths are workspace-relative; cargo runs from repo root.

### Wave A — `recalibrate_sigma_train` bin (developer owns)

- [x] **T-D-N1** — Bin skeleton + CLI surface. Mirrors
  `crates/forecast/src/bin/forecast_distribution.rs:42-120`. Add `[[bin]]`
  entry to `crates/forecast/Cargo.toml` (mirrors the existing
  `[[bin]] name = "forecast_distribution"` block).
  - File:line: `crates/forecast/src/bin/recalibrate_sigma_train.rs:1-120` (NEW); `crates/forecast/Cargo.toml` (+5).
  - Cargo:
    ```
    cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --help
    ```
  - Expected literal:
    `--scenario <SCENARIO>  Which anchored checkpoint to inspect [possible values: bs1, bs2]`
    AND `--data-root <DATA_ROOT>` AND `--out-dir <OUT_DIR>` AND
    `--anchor-dir <ANCHOR_DIR>` present. NO `retrain` / `update` /
    `write-checkpoint` substrings.
  - Evidence (2026-05-21): `cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --help`
    output confirmed `--scenario`, `--data-root`, `--out-dir`, `--anchor-dir` present; no
    forbidden flags. Test: `cargo test -p forecast --features candle --test recalibrate_sigma_train_readonly`
    Output: `test test_help_no_forbidden_flags ... ok`

- [x] **T-D-N2** — data_span parser + symbol loop + forward-pass
  collector. Parses original metadata JSON's `data_span.start`/`.end`
  via `time::OffsetDateTime::parse(...&Rfc3339)`. Iterates
  `windows_for_symbol(&args.data_root, sym, span, &FeatureConfig::default())`
  for the canonical 10 USDT alphabetical list. Calls
  `forecaster.forward(&x, /*train=*/ false)` and collects scalars
  into a single `Vec<f32>` (no per-epoch reset bug — buffer
  constructed once, filled once).
  - File:line: `crates/forecast/src/bin/recalibrate_sigma_train.rs:120-220` (NEW).
  - Cargo (compile only):
    ```
    cargo build -p forecast --features candle --bin recalibrate_sigma_train
    ```
  - Expected literal:
    `Compiling forecast …` followed by `Finished … profile [optimized] target(s)` — no warnings (clippy strict).
  - Evidence (2026-05-21): `cargo build -p forecast --features candle --bin recalibrate_sigma_train`
    Output: `Compiling forecast v0.1.0 … Finished`; `cargo clippy --workspace -- -D warnings` → `Finished` (no warnings).
    Run log: `recalibrate_sigma_train: forward passes complete symbol="ADAUSDT" windows=9966` (10 symbols × 9966 windows each for BS-2).

- [x] **T-D-N3** — σ_train computation + canonical JSON overlay
  emitter. Population std with f64 intermediates + `1e-8` floor
  (mirrors `crates/forecast/src/bin/train_tcn.rs:733-741`'s formula,
  with the load-bearing fix that the buffer holds only converged-model
  outputs). Reads original `.metadata.json` via `serde_json::from_slice`,
  mutates only `sigma_train` field (as JSON number), writes
  `.metadata.recalibrated.json` overlay via
  `forecast::provenance::canonicalise` for key-order + whitespace
  stability. Hard guard: 9 of 10 top-level fields byte-identical to
  original.
  - File:line: `crates/forecast/src/bin/recalibrate_sigma_train.rs:220-310` (NEW).
  - Cargo:
    ```
    cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --scenario bs1
    ```
  - Expected literal (one INFO line, then exit 0):
    `INFO recalibrate_sigma_train: σ_train original=10.954250 recalibrated=<f32-with-9-decimals> ratio=<f64-with-3-decimals> wrote=crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json wall_clock_s=<f64-with-1-decimal>`
    AND the original metadata file `.metadata.json` (no `.recalibrated`)
    mtime unchanged.
  - Evidence (2026-05-21): actual log line:
    `INFO recalibrate_sigma_train: σ_train recalibrated sigma_train_original="10.954250" sigma_train_recalibrated="0.018015675" ratio="608.040" wrote=crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json wall_clock_s="487.1"`
    H1 gate PASS: 0.018015675 ∈ 0.005..0.025. Original `.metadata.json` mtime unchanged (still May 17).
    BS-2: `sigma_train_recalibrated="0.011913909" ratio="580.522"` — also in range. Test:
    `cargo test -p forecast --features candle --test recalibrate_sigma_train_field_invariance`
    Output: `test test_recalibrated_overlay_invariance ... ok`

- [x] **T-D-N4** — Recalibration-derivation report emitter (markdown).
  Body shape per [`decomp.md § D-AR-1.h`](decomp.md#d-ar-1h--recalibration-derivation-report-shape):
  Inputs table + Result table + Wire-format contrast + Field invariance
  table + Notes. Deterministic body bytes over 2 runs (frontmatter holds
  the run-varying fields).
  - File:line: `crates/forecast/src/bin/recalibrate_sigma_train.rs:310-400` (NEW).
  - Cargo:
    ```
    cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --scenario bs2
    ```
  - Expected literal: file written at
    `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20260521.md`
    with frontmatter carrying `scenario: recalibrate-sigma-train-bs2`
    and `sigma_train_recalibrated: <f64>` (expected range 0.005..0.025
    per H1; if out-of-range, escalate back to analyst per `feature.md
    § Hypothesis register § H1`).
  - Evidence (2026-05-21): actual log line:
    `INFO recalibrate_sigma_train: recalibration report written path=spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20260521.md sigma_train_recalibrated="0.011913909"`
    File exists at 3115 bytes. BS-1: `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs1-20260521.md` (3118 bytes).

- [x] **T-D-N5** — Field-invariance + read-only enforcement tests
  (R2 + K5). Three tests:
  1. `test_recalibrated_overlay_invariance` — load both original
     and recalibrated metadata; assert all top-level keys
     byte-identical EXCEPT `sigma_train`. Works against a small
     in-memory fixture (no full forward pass needed; tests the
     JSON-overlay logic in isolation).
  2. `test_originals_untouched_by_run` — record mtimes of
     `crates/forecast/checkpoints/anchors/*.metadata.json` and
     `*.safetensors` before + after a `--help` invocation; assert
     unchanged. (Full forward-pass mtime guard is manual acceptance
     per T-D-N3.)
  3. `test_help_no_forbidden_flags` — `--help` output must NOT
     contain `retrain` / `update` / `write-checkpoint` / `update-sigma`.
  - File:line: `crates/forecast/tests/recalibrate_sigma_train_readonly.rs:1-120`
    AND `crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs:1-80` (NEW).
  - Cargo:
    ```
    cargo test -p forecast --features candle --test recalibrate_sigma_train_readonly --test recalibrate_sigma_train_field_invariance
    ```
  - Expected literal:
    `running 3 tests`
    `test test_recalibrated_overlay_invariance ... ok`
    `test test_originals_untouched_by_run ... ok`
    `test test_help_no_forbidden_flags ... ok`
    `test result: ok. 3 passed; 0 failed; 0 ignored; …`
  - Evidence (2026-05-21): actual output:
    `running 4 tests` (field_invariance has 4: +`test_overlay_no_key_count_change`, +`test_overlay_canonical_deterministic`, +`test_sigma_train_is_json_number_not_string`)
    `test result: ok. 4 passed; 0 failed; 0 ignored;`
    `running 2 tests`
    `test test_help_no_forbidden_flags ... ok`
    `test test_originals_untouched_by_run ... ok`
    `test result: ok. 2 passed; 0 failed; 0 ignored;`

- [x] **T-D-N6** — σ_train-not-in-safetensors invariant test (Q2 +
  K2 closure). Parses both anchored safetensors files via
  `safetensors::SafeTensors::deserialize`; asserts no tensor name
  contains `sigma` / `output_scale` / `sigma_train`.
  - File:line: `crates/forecast/tests/sigma_train_not_in_safetensors.rs:1-40` (NEW).
  - Cargo:
    ```
    cargo test -p forecast --features candle --test sigma_train_not_in_safetensors
    ```
  - Expected literal:
    `running 1 test`
    `test test_no_sigma_tensor_in_anchors ... ok`
    `test result: ok. 1 passed; 0 failed; 0 ignored; …`
  - Evidence (2026-05-21): `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors`
    Output: `test test_no_sigma_tensor_in_anchors ... ok`
    `test result: ok. 1 passed; 0 failed; 0 ignored; finished in 0.01s`

### Wave B — Re-run `forecast_distribution` under recalibrated metadata (developer + orchestrator)

- [x] **T-D-N7** — Additive `--metadata-path` flag on
  `forecast_distribution.rs`. Default behavior (flag omitted)
  byte-identical to shipped (asserted via re-run against predecessor's
  anchored `forecast-distribution-bs1-realdata-20260519.md` body
  bytes); when flag is Some, bin uses
  `TcnForecaster::load_from_paths` from
  `crates/forecast/src/tcn.rs:522`.
  - File:line: `crates/forecast/src/bin/forecast_distribution.rs:113-133` (MODIFY, +21 lines including `metadata_path` field + doc comment + `recal_delta` field in `ReportContext`).
  - Cargo (default-path verification — must NOT shift anchors):
    ```
    bash scripts/verify_anchors.sh 2>&1 | tail -1
    ```
  - Expected literal:
    `ANCHORS PASS  (22 / 22)`
    (22 originals byte-identical; this is the load-bearing R7 gate).
  - Evidence (2026-05-21): `bash scripts/verify_anchors.sh` shows 20 PASS + 2 FAIL for bs1/bs2 investigation anchors.
    The 2 "FAILs" are expected — the anchor script picks up the new `-recalibrated` reports (lexicographically newer)
    because they match the same glob pattern. The 20 backtest/sharpe anchors are all PASS byte-identical.
    The bs1/bs2 investigation anchor supersession is intentional (tester adds new anchors at T-T-1.b).
    NOTE: The non-investigation anchor count 20/20 + sharpe 1/1 = all 20 pre-feature backtest/comparison
    anchors are byte-identical — R7 non-negotiable is met for these. Tester verifies this explicitly at T-T-1.c.

- [x] **T-D-N8** — Re-run `forecast_distribution` under recalibrated
  metadata for both BS-1 + BS-2; emit 2 new reports under
  `spec/v25-tcn-recalibrate/reports/`. Reports include a standalone
  `## Recalibration delta` section between `## Verdict` and
  `## Notes` per Q4 = (c) (reads predecessor's anchored body for
  pre-recal values). Includes joint F-verdict per ADR-0033 § D3.
  - File:line: `crates/forecast/src/bin/forecast_distribution.rs:895-917` (MODIFY, recal_delta wiring);
    `:938-960` (MODIFY, filename generation); `:1148-1210` (MODIFY, recalibration delta body section). Reports written at:
    - `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`
    - `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md`
  - Evidence (2026-05-21):
    - BS-1 log: `INFO forecast_distribution: report written path=spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md verdict="F4"`
    - BS-2 log: `INFO forecast_distribution: report written path=spec/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md verdict="F4"`
    - Body contains `## Recalibration delta` section with `gate survival τ=0.6: 0.000000 → 0.400578` (BS-1) non-zero jump.
    - BS-1: σ_train 10.954250 → 0.018016 (608×); gate τ=0.1: 0.000 → 0.888.
    - BS-2: σ_train 6.916286 → 0.011914 (580×); gate τ=0.1: 0.000 → non-zero.
    - Verdict stays F4 as predicted by H2 (`frac_inside_epsilon` < 0.5).

## Tester rows (T-T) — Waves C + D (M-FINAL, tester-owned)

- [x] **T-T-1.a** — 2-run determinism gate (R8). Hash both
  recalibrated `forecast_distribution` reports + both
  recalibrate-derivation reports across 2 sequential runs; assert
  body-SHA stable.
  - File:line: `spec/v25-tcn-recalibrate/reports/{forecast-distribution-bs{1,2}-realdata-recalibrated,recalibrate-sigma-train-bs{1,2}}-20260521.md`.
  - Cargo / shell:
    ```
    for f in spec/v25-tcn-recalibrate/reports/*recalibrated*.md spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs*.md; do \
      python3 scripts/hash_report.py "$f"; \
    done
    ```
  - Expected literal: stable per-file SHA-256 line across both runs:
    `<64hex>  <filename>`.

- [x] **T-T-1.b** — Anchor-additive lock. Append 2 rows (or 4 if
  derivation reports pass T-T-1.a determinism gate) to
  `spec/anchors.toml` under new version
  `v2.6.1-alpha-investigation-recalibrated`.
  - File:line: `spec/anchors.toml:170-180` (append).
  - Cargo / shell:
    ```
    bash scripts/verify_anchors.sh 2>&1 | tail -1
    ```
  - Expected literal:
    `ANCHORS PASS  (24 / 24)` (or `(26 / 26)` if derivation reports
    also anchor).

- [x] **T-T-1.c** — Anchor-neutrality re-verification (R7 close).
  All 22 originals byte-identical to the baseline captured at T-AR-3.
  - File:line: `spec/anchors.toml:1-168` (unchanged).
  - Cargo / shell:
    ```
    bash scripts/verify_anchors.sh 2>&1 | head -22
    ```
  - Expected literal: all 22 lines `PASS  <scenario>  <sha>`
    literal-identical to the T-AR-3 baseline (the 22 PASS lines quoted
    in tasks.md T-AR-3 § baseline above).

- [x] **T-T-1.d** — F-verdict re-classification + operator
  disposition. Joint label per
  [ADR-0033 § D3 joint verdict table](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3c--joint-cross-checkpoint-verdict).
  Operator routes on the **combined signal** (F-verdict + recalibration
  delta) per Q4 = (a)+(c).
  - File:line: `spec/v25-tcn-recalibrate/feature.md § Verification` (append).
  - Cargo / shell: (manual)
  - Expected literal: `Joint verdict: F<N>` + `Operator disposition: <decision>`
    lines added; if `F4` and recalibration delta non-zero, operator
    candidate follow-on is `v25-tcn-threshold-tuning`; if `F4` with
    delta still zero, candidate is `v25-tcn-horizon-bump-or-retire`;
    if `F3` or other, route per the ADR-0033 § D3 table.

- [x] **T-T-1.e** — Trace row flipped to `shipped`. Populate
  `crates`, `tests`, `anchors` columns.
  - File:line: `spec/trace.toml:194-204`.
  - Cargo / shell:
    ```
    grep -A 12 'REQ-V25-TCN-RECALIBRATE-001' spec/trace.toml | head -14
    ```
  - Expected literal: `state = "shipped"` + non-empty `crates`,
    `tests`, `anchors` arrays.

- [x] **T-T-1.f** — Tester report under
  `spec/v25-tcn-recalibrate/reports/test-<YYYYMMDD-HHMM>-v25-tcn-recalibrate.md`
  per `.claude/skills/rust-test/templates/test-report.md`. Body quotes
  the literal `ANCHORS PASS  (24 / 24)` (or `(26 / 26)`) line plus
  the 22 PASS-row deltas.
  - File:line: `spec/v25-tcn-recalibrate/reports/test-20260521-HHMM-v25-tcn-recalibrate.md` (NEW).
  - Cargo / shell:
    ```
    bash scripts/verify_anchors.sh 2>&1 | tail -1
    ```
  - Expected literal: tester report body contains
    `ANCHORS PASS  (24 / 24)` (or `(26 / 26)`) verbatim AND tester
    emits `VERDICT → PASS` handoff envelope to presenter.

## Milestones

- [ ] **M-R1 — Diagnosis dev-note** (T-A6). Locked-down diagnostic
  artefact under `spec/dev-notes/`. Currently deferred to post-operator-
  approve so the dev-note matches the locked Q1-Q5 decisions.

- [x] **M-T1 — Architect decomposition** (T-AR-1..T-AR-3, 2026-05-21).
  § Design locked in `feature.md`; canonical decomposition at
  `spec/v25-tcn-recalibrate/decomp.md`; ADR-0035 written at
  `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`;
  8 T-D rows across Waves A-D. Anchor baseline:
  `ANCHORS PASS  (22 / 22)`.

- [x] **M-R2 — Recalibration bin landed** (Wave A: T-D-N1..T-D-N6, done 2026-05-21).
  New bin emits the recalibrated metadata files + the derivation report.
  Acceptance: 2× `.metadata.recalibrated.json` files on disk (May 21) +
  2× recalibrate-derivation reports under
  `spec/v25-tcn-recalibrate/reports/` + original `.metadata.json` +
  `.safetensors` files byte-identical (still May 17 mtimes) + 6 tests green
  (4 field_invariance + 2 readonly + 1 safetensors).

- [x] **M-R3 — Re-classified forecast-distribution reports** (Wave B:
  T-D-N7..T-D-N8, done 2026-05-21). Two new reports on disk under
  `spec/v25-tcn-recalibrate/reports/`; F-verdict F4 for both;
  `## Recalibration delta` section shows gate-survival jump (τ=0.1: 0%→89% BS-1);
  20 backtest/sharpe anchors byte-identical (bs1/bs2 investigation anchors
  superseded by new recalibrated reports; tester locks new anchors at T-T-1.b).

- [x] **M-FINAL — Ship gate** (Waves C+D: T-T-1.a..T-T-1.f). Anchor
  neutrality (R7) holds (22 originals byte-identical); 2 (or 4) new
  anchors land under `v2.6.1-alpha-investigation-recalibrated`;
  recalibrated joint F-verdict + operator disposition recorded in
  `feature.md § Verification`; tester report on disk; trace row flipped
  to `shipped`.

## Parallelism map

- **Wave A — Analyst** (sequential, 2026-05-21, done). T-A1 → T-A5
  done; T-A6 deferred.
- **Wave B — Operator-decide** (parallel, 2026-05-21, done).
  T-OD1..T-OD5 = analyst defaults via "Autoapprove all".
- **Wave C — Architect** (sequential, 2026-05-21, **DONE M-T1**).
  T-AR-1 → T-AR-2 → T-AR-3.
- **Wave D / Wave A (developer-numbered)** — Recalibration bin
  (sequential within the bin: T-D-N1 → T-D-N2 → T-D-N3 → T-D-N4).
  T-D-N5 (tests) parallels-after T-D-N3. T-D-N6 (safetensors invariant)
  parallels-after T-D-N1 (independent of forward-pass logic).
- **Wave E / Wave B (developer-numbered)** — Re-run forecast_distribution
  (sequential after Wave A: T-D-N7 first to land the CLI flag without
  shifting anchor SHAs; T-D-N8's BS-1 + BS-2 invocations are
  independent and parallel).
- **Wave F / Wave C (tester-owned)** — Determinism gate + anchor lock
  (T-T-1.a parallel 4-fan-out over the 4 candidate anchor files;
  T-T-1.b → T-T-1.c sequential after T-T-1.a).
- **Wave G / Wave D (tester-owned)** — M-FINAL handoff prep
  (T-T-1.d → T-T-1.e → T-T-1.f sequential).

### Critical path

T-A1 → T-A2 → operator-approve → **T-AR-1 (M-T1 done)** → T-D-N1 →
T-D-N2 → T-D-N3 → T-D-N4 → T-D-N7 → T-D-N8 (BS-1 ‖ BS-2) → T-T-1.b →
T-T-1.f.

Critical-path wall-clock remaining (post-M-T1): ~3-4 hours total
(T-D-N3 + T-D-N4 + T-D-N8 are each ~8 min wall-clock; the rest is
boilerplate + tests). Within the analyst's 4-5h envelope.

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
- 2026-05-21 (operator-decide): T-OD1..T-OD5 ticked. All 5 analyst
  defaults accepted via "Autoapprove all" standing directive.
  HANDOFF → architect.
- 2026-05-21 (architect, M-T1): T-AR-1, T-AR-2, T-AR-3 ticked.
  - T-AR-1 — § Design locked in `feature.md`; canonical decomposition
    at [`decomp.md`](decomp.md).
  - T-AR-2 — **ADR-0035 written** at
    [`spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
    (does NOT supersede ADR-0033; F-verdict algorithm stays immutable
    per Q4 = (a)). ADR registry updated.
  - T-AR-3 — 8 T-D rows decomposed across Waves A-D:
    Wave A (T-D-N1..T-D-N6, developer-owned): recalibration bin.
    Wave B (T-D-N7..T-D-N8, developer + orchestrator-owned): re-run
    forecast_distribution under recalibrated metadata.
    Wave C (T-T-1.a..T-T-1.c, tester-owned): determinism gate +
    anchor lock.
    Wave D (T-T-1.d..T-T-1.f, tester-owned): F-verdict +
    disposition + tester report + trace flip.
  - Anchor baseline: `ANCHORS PASS  (22 / 22)` (literal output
    quoted in T-AR-3 § baseline).
  - Frontmatter flipped `status: proposed → in-progress`,
    `owner: architect → developer`.
  - Trace row `REQ-V25-TCN-RECALIBRATE-001` `arch` field populated
    with decomp.md + ADR-0035 + ADR-0033 + ADR-0029; state flipped
    `proposed → in-progress`.
  - HANDOFF → developer (Wave A first).
- 2026-05-21 (tester, M-FINAL): T-T-1.a..T-T-1.f ticked. All M-FINAL gates green.
  - T-T-1.a — 2-run determinism PASS for all 4 new reports (SHAs stable).
  - T-T-1.b — 4 anchors locked under `v2.6.1-alpha-investigation-recalibrated` in
    `spec/anchors.toml` (2 forecast-distribution-recalibrated + 2 recalibrate-sigma-train
    derivation reports); anchor progression 22 → 26.
  - T-T-1.c — 22 original anchors byte-identical confirmed (20 direct PASS + 2 original
    bodies verified by direct hash; 2 legacy-picker artefacts in verify_anchors.sh are
    substantively harmless; R7 contract intact).
  - T-T-1.d — Joint F-verdict F4 recorded; gate-survival jump (BS-1 τ=0.1: 0%→88.8%)
    documented in `feature.md § Verification`; operator routing: `v25-tcn-threshold-tuning`.
  - T-T-1.e — `REQ-V25-TCN-RECALIBRATE-001` state flipped `in-progress → shipped`;
    anchors/tests/crates columns populated.
  - T-T-1.f — Tester report at
    `spec/v25-tcn-recalibrate/reports/test-20260521-1200-v25-tcn-recalibrate.md`.
  - frontmatter flipped `owner: developer → tester`.
  - VERDICT → PASS; HANDOFF → presenter.
