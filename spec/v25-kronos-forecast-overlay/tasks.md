---
slug: v25-kronos-forecast-overlay
status: in-progress
owner: architect
updated: 2026-05-16
---

# Tasks — v2.5 Kronos forecast overlay

> **Architect refinement landed 2026-05-16.** Skeleton authored by
> analyst, refined by architect after operator-decided Q1 / Q3 / Q9 /
> Q10-budget / Q11 / Q12 / Q13, architect-decided Q2 / Q4 / Q5 / Q6 /
> Q7 / Q8. Cross-references:
> [feature.md § Design](feature.md#design),
> [ADR-0027](../architecture/adr/0027-kronos-onnx-tract-integration.md),
> [architecture/12-forecast-overlay.md](../architecture/12-forecast-overlay.md).
> Numbering follows the v2-llm-strategy M0–M7 convention with
> T-prefix tasks. Developer ticks M1–M6; tester ticks M7
> (`T_FINAL_*`).

## M0 — Architect setup (DONE — spec-only, no code)

All M0 items resolved in the architect's 2026-05-16 pass. Retained
here as historical breadcrumb for the developer.

- [x] T-M0-1 — Resolve Q9 (crate split): new `crates/forecast/`. —
  _resolved: [feature.md § Crate layout](feature.md#crate-layout)._
- [x] T-M0-2 — Resolve Q10 (shared `ReplayCache<K, V>` extraction):
  attempt extraction with **2-dev-day budget** (operator-locked);
  ship duplicate caches on budget exit. — _resolved: [ADR-0027 § Q6](../architecture/adr/0027-kronos-onnx-tract-integration.md#q6--determinism-contract-inherit-v2-llm-recordreplay-q8-pattern)._
- [x] T-M0-3 — Resolve Q12 (ONNX vendored vs build-script): vendored
  `.onnx` at `crates/forecast/assets/kronos-base.onnx` via git LFS. —
  _resolved: operator-locked._
- [x] T-M0-4 — Resolve Q13 (signal-level vs risk-level overlay):
  **signal-level inside `Strategy::tick()`**. — _resolved:
  [architecture/12-forecast-overlay.md § Overlay composition pattern](../architecture/12-forecast-overlay.md#overlay-composition-pattern-signal-level)._
- [x] T-M0-5 — Confirm Q2 (model size): `base` 102.3M / 512-ctx
  accepted. — _resolved: [ADR-0027 § Q2](../architecture/adr/0027-kronos-onnx-tract-integration.md#q2--model-size-base-1023m-params-512-ctx)._
- [x] T-M0-6 — ADR landed: [ADR-0027](../architecture/adr/0027-kronos-onnx-tract-integration.md).
  — _resolved._
- [x] T-M0-7 — Resolve Q4 (forecast horizon): single-bar (next-bar)
  only at v2.5. — _resolved: [ADR-0027 § Q4](../architecture/adr/0027-kronos-onnx-tract-integration.md#q4--forecast-horizon-single-bar-next-bar-only)._
- [x] T-M0-8 — Resolve Q6 (determinism): inherit v2 LLM record/replay
  pattern wholesale. — _resolved: [ADR-0027 § Q6](../architecture/adr/0027-kronos-onnx-tract-integration.md#q6--determinism-contract-inherit-v2-llm-recordreplay-q8-pattern)._
- [x] T-M0-9 — Resolve Q8 (cost telemetry shape):
  `CostEvent::Infra { line: "kronos_inference", … }`, default-zero
  usd. — _resolved: [ADR-0027 § Q8](../architecture/adr/0027-kronos-onnx-tract-integration.md#q8--cost-telemetry-costeventinfra-with-default-zero-usd)._
- [x] T-M0-10 — Cross-cutting architecture section authored at
  [`spec/architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md).
  — _resolved._

## M1 — Crate scaffold + replay-cache extraction spike (developer)

Goal: stand up `crates/forecast/` and decide the replay-cache shape
within the 2-dev-day operator-locked budget.

- [ ] T-M1-1 — Create `crates/forecast/` virtual-workspace member +
  empty `lib.rs` + `Cargo.toml` with edition = 2024 + `package.name
  = "forecast"` (NOT a stdlib-collision name per ADR-0001). —
  _acceptance: `cargo build -p forecast` clean._
- [ ] T-M1-2 — Add `ForecastOverlay` + `Direction` + `ForecastRequest`
  + `ForecastResponse` + `ForecastError` to
  `crates/core/src/forecast.rs`. — _acceptance: serde round-trip
  property test green; `confidence` is `rust_decimal::Decimal`._
- [ ] T-M1-3 — Define `ForecastProvider` async-trait in
  `crates/forecast/src/lib.rs`. Mirror shape per [architecture/12
  § `ForecastProvider` trait](../architecture/12-forecast-overlay.md#forecastprovider-trait).
  — _acceptance: trait compiles + mockall mock for tests._
- [ ] T-M1-4 — **2-day-budget spike**: extract
  `crates/replay-cache/` generic `ReplayCache<K, V>` and migrate
  `crates/llm/src/replay.rs` to use it. **Start a 2-day timer at
  T-M1-4 first commit. Exit at T-M1-4-EXIT below.** —
  _acceptance (success): both `crates/llm` and `crates/forecast`
  consume `crates/replay-cache` with identical `schema_version = 1`
  rows; `cargo test --workspace` green._
- [ ] T-M1-4-EXIT — **Budget exit marker.** If T-M1-4 not green
  after 2 dev-days, developer aborts the extraction, copies the v2
  LLM cache into `crates/forecast/src/replay.rs` as a sibling, and
  flags the abort in the v2.5 dev-notes for the architect to open a
  `replay-cache-extraction` v2.5.x brief. — _acceptance: either
  T-M1-4 green OR T-M1-4-EXIT recorded with a note in
  `spec/dev-notes/kronos-replay-cache-budget-2026-MM-DD.md`._

## M2 — ONNX vendoring + checksum gate (developer)

Goal: get the Kronos `base` checkpoint into the repo in a form the
build can verify.

- [ ] T-M2-1 — One-off conversion script (Python, not committed to
  runtime path): `scripts/dev/kronos_torch_to_onnx.py` converts the
  HF `NeoQuasar/Kronos-base` weights to ONNX via
  `torch.onnx.export`. Pinned HF revision SHA in the script header.
  — _acceptance: produces `kronos-base.onnx` + SHA-256 digest +
  license tag asserted MIT._
- [ ] T-M2-2 — Git LFS bootstrap for the repo if not already done:
  `.gitattributes` declares `crates/forecast/assets/*.onnx` as LFS.
  `git lfs install` documented in
  `spec/runbooks/dev-onboarding.md` (or create that file if missing,
  via spec-update skill). — _acceptance: `git lfs ls-files` lists
  the checkpoint after commit._
- [ ] T-M2-3 — Vendor `kronos-base.onnx` at
  `crates/forecast/assets/kronos-base.onnx` via LFS. —
  _acceptance: file present + SHA-256 recorded in
  `crates/forecast/build.rs` const + License-MIT marker file
  committed alongside._
- [ ] T-M2-4 — Build-script checksum gate: `crates/forecast/build.rs`
  asserts the on-disk SHA-256 matches the const. — _acceptance:
  `cargo build -p forecast` fails if the checkpoint mutates._

## M3 — `tract` integration: load + forward + tokenisation (developer)

Goal: forward-pass an OHLCV window through `tract` and get bytes-out
that match the Python reference.

- [ ] T-M3-1 — `crates/forecast/src/tract_loader.rs`: load
  `kronos-base.onnx` via `tract` at provider construction time;
  cache the prepared model in `Arc<TractModel>`. — _acceptance:
  `cargo test -p forecast load_smoke` green._
- [ ] T-M3-2 — `crates/forecast/src/tokenizer.rs`: port the Kronos
  hierarchical OHLCV tokenizer from Python to Rust. — _acceptance:
  byte-identical tokens against a Python reference fixture for 100
  canned OHLCV bars._
- [ ] T-M3-3 — Forward-pass: `KronosForecaster::forecast()`
  tokenises → `tract.run()` → detokenises → emits
  `ForecastResponse`. — _acceptance:
  `cargo test -p forecast forward_smoke` green; output is non-NaN
  and within expected OHLCV ranges._
- [ ] T-M3-4 — **Q3-fallback gate**: if T-M3-1 or T-M3-3 fails on
  unsupported `tract` ops, raise to the architect for the 1-day
  spike per [ADR-0027 § Q3](../architecture/adr/0027-kronos-onnx-tract-integration.md#q3--integration-path-onnx-export--tract-in-process).
  Do NOT silently switch to subprocess. — _acceptance: either
  T-M3-1 + T-M3-3 green OR architect-approved fallback path
  recorded in a v2.5 dev-note._

## M4 — Replay cache wiring + sampling-seed determinism (developer)

Goal: every `ForecastProvider::forecast()` call hits the cache in
research mode (strict-replay-only).

- [ ] T-M4-1 — Cache-key canonical JSON over `(model_revision,
  ohlcv_window, temperature, top_p, top_k, max_tokens,
  sampling_seed)`. Use the v2 LLM `serde-canonical-json` helper. —
  _acceptance: SHA-256 over the canonical form matches an
  architect-pinned reference for a fixed input._
- [ ] T-M4-2 — Wire `ChaCha20Rng::from_seed(sampling_seed.into())`
  into the tract sampler per [ADR-0002](../architecture/adr/0002-rng-chacha20.md).
  — _acceptance: two runs at the same seed produce byte-identical
  sampled tokens._
- [ ] T-M4-3 — Strict-replay mode: in research-mode, a cache miss
  returns `ForecastError::ReplayMiss`. — _acceptance: unit test
  with empty fixture cache asserts the error._
- [ ] T-M4-4 — Fixture cache committed at
  `crates/forecast/tests/fixtures/kronos-replay.db` with canned
  forecasts for BS-1 + BS-2 sampling seeds. — _acceptance: 2+
  rows present; SHA-256 of the DB file recorded for reproducibility._

## M5 — Strategy impl + overlay composition (developer)

Goal: `kronos_momentum` strategy emits modulated `Signal` per Q13.

- [ ] T-M5-1 — `crates/strategy/src/kronos_momentum.rs`: `Strategy`
  impl that composes v1 momentum's signal with `ForecastOverlay`
  per [feature.md § `tick()` flow](feature.md#tick-flow--overlay-composition).
  — _acceptance: `cargo test -p strategy kronos_momentum_combine`
  green with the 3 cases (agree+confident, disagree+confident,
  flat-or-low-confidence)._
- [ ] T-M5-2 — `KronosConfig` (model revision SHA pin, sampling
  params, `overlay_confidence_threshold = 0.6` default,
  `energy_cost_per_kwh = 0` default). — _acceptance: serde
  round-trip + a `from_env`-style loader test._
- [ ] T-M5-3 — Integration test exercising the composed pipeline
  end-to-end against a fixture replay cache. — _acceptance:
  `cargo test -p strategy kronos_momentum_pipeline` green with no
  network access._
- [ ] T-M5-4 — Audit-row emission: one
  `JournalEntry { kind: "forecast_emitted", correlation_id, … }`
  per forecast call per [architecture/12 § Audit-row shape](../architecture/12-forecast-overlay.md#audit-row-shape--forecast-emission).
  — _acceptance: ledger query test asserts the row shape +
  6-digit fractional-second `posted_at` per [ADR-0004](../architecture/adr/0004-fractional-second-timestamps.md)._

## M6 — Cost telemetry (developer; small)

Goal: every forecast call posts a `CostEvent::Infra` row with the
correct line label and default-zero usd.

- [ ] T-M6-1 — Emit `CostEvent::Infra { line: "kronos_inference",
  usd, period: CostPeriod::PerCall }` on every forecast. With
  default `energy_cost_per_kwh = 0` the `usd` is zero. —
  _acceptance: cost-sink fixture asserts one row per forecast call;
  zero dollars at default config._
- [ ] T-M6-2 — Opt-in test: with
  `energy_cost_per_kwh = Decimal::from_str("0.15")`, the
  `expense:infra:kronos_inference` ledger account receives a
  non-zero posting per forecast. — _acceptance: ledger query test
  shows the posting + a comment in the test noting "operator
  opt-in only; fixture reports stay byte-identical at default"._

## M7 — Backtest scenarios + anchor lock (tester)

Goal: BS-1 + BS-2 anchored; BS-3 non-regression sweep green; T_FINAL
locks the new anchors.

- [ ] T-M7-1 — Run **BS-1** (`top10-2023-fy-kronos-momentum`,
  2023-01-01 → 2023-12-31, top-10 USDT, 1h bars,
  seed `0xC0FFEE`) via `backtest` skill. — _acceptance: report
  committed at `spec/v25-kronos-forecast-overlay/reports/backtest-bs1-<ts>.md`;
  body-SHA-256 captured via `scripts/hash_report.py`._
- [ ] T-M7-2 — Run **BS-2** (`top10-2024-fy-kronos-momentum`,
  2024-01-01 → 2024-12-31, same config). — _acceptance: same as
  T-M7-1 with BS-2 path._
- [ ] T-M7-3 — If architect-preferred path: run the v1-baseline
  for BS-2 (`top10-2024-fy-momentum`) and lock its anchor in the
  same tester pass. Tester may skip and compute-live each
  verify-anchors run if BS-2 baseline runtime is acceptable. —
  _acceptance: either anchor locked OR a tester-note explaining
  the compute-live choice._
- [ ] T-M7-4 — Pass criterion: Sharpe(Kronos overlay) ≥ Sharpe(v1
  baseline) × 1.05 on **BS-1**. — _acceptance: backtest report
  table shows the side-by-side; tester verdict ROUTES BACK to
  analyst (auto-open v2.5.x fine-tuning brief) if criterion fails._
- [ ] T-M7-5 — Pass criterion repeat on **BS-2**. — _acceptance:
  same as T-M7-4 with BS-2 numbers._
- [ ] T-M7-6 — `scripts/pre_stage_anchors.sh` stages the 2 new
  (or 3 new conditional) SHAs into `spec/anchors.toml`. —
  _acceptance: PR-ready diff; new rows have `version = "v2.5"`._
- [ ] T-M7-7 — **BS-3 non-regression sweep**:
  `scripts/verify_anchors.sh` against the v2.5 build asserts the 9
  existing strategy anchors + 2 existing `report-sample-*` anchors
  stay byte-identical. — _acceptance: stdout reports `11/11
  byte-identical` for existing rows + the new rows pass._
- [ ] T-M7-8 — Final `scripts/verify_anchors.sh` →
  `ANCHORS PASS (13/13)` or `ANCHORS PASS (14/14)`. —
  _acceptance: stdout line + exit 0._
- [ ] T_FINAL_V25_KRONOS — Spec-lint + verify-anchors PASS;
  evaluator VERDICT → PASS; tester locks the new anchors at
  `spec/anchors.toml` and updates `spec/trace.toml` row
  `REQ-V25-KRONOS-001` to `state = "shipped"` plus appends the new
  scenario names to its `anchors = […]` array. — _acceptance:
  evaluator report at
  `spec/v25-kronos-forecast-overlay/reports/evaluation-<ts>.md`
  with `VERDICT → PASS` and presenter spawn._

## Notes

- Per AGENT.md, the **developer never ticks `T_FINAL_*`** — only the
  tester does, and only after `verify-anchors` PASS.
- All write paths follow the
  [spec-update skill](../../.claude/skills/spec-update/SKILL.md); no
  raw Write/Edit to `spec/` files.
- M1 includes the **operator-locked 2-day budget for replay-cache
  extraction** (T-M1-4 + T-M1-4-EXIT). Developer commits the budget
  start time and respects the exit.
- M3 includes the **Q3-fallback gate** (T-M3-4): if ONNX conversion
  blocks on `tract` op-set compat, route back through the architect.
  Do NOT silently switch to subprocess + IPC.
- The `kronos_momentum` strategy is the v2.5 ship gate consumer.
  Future consumers (additional base strategies; pure-Kronos at v2.6)
  reuse `crates/forecast/` unchanged.
- All `CostEvent::Infra` postings default to `usd = 0`; the fixture
  `report-sample-*` anchors stay byte-identical at default config
  per [ADR-0027 § Q7](../architecture/adr/0027-kronos-onnx-tract-integration.md#q7--anchor-impact-11-existing-stay-2-new-at-ship).
- Backtest baselines: BS-1 compares to the **existing** v1 anchor
  `top10-2023-1h-momentum` (lines 41–43 of `anchors.toml`); BS-2
  needs a **new** `top10-2024-fy-momentum` v1 baseline anchor
  unless tester chooses compute-live.
