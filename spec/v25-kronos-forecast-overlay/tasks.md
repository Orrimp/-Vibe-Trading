---
slug: v25-kronos-forecast-overlay
status: in-progress
owner: architect
updated: 2026-05-16
---

# Tasks — v2.5 Kronos forecast overlay

> **Skeleton authored by analyst 2026-05-16.** Architect refines task
> shapes and counts after answering open questions Q2 / Q3 / Q6 / Q8 /
> Q9 / Q10 / Q12 / Q13. Numbering follows the v2-llm-strategy
> convention (M0 / M1 / M2 milestones with T-prefix tasks).

## M0 — Architect setup (spec-only, no code)

- [ ] T-M0-1 — Resolve Q9 (crate split): confirm `crates/forecast/`
  vs absorb. — _acceptance: feature.md ## Design names the chosen
  layout._
- [ ] T-M0-2 — Resolve Q10 (shared `ReplayCache<K, V>` extraction):
  decide and write a 2-day budget if extract; ship-separate
  otherwise. — _acceptance: ADR or feature.md decision row._
- [ ] T-M0-3 — Resolve Q12 (ONNX vendored vs build-script): decide.
  — _acceptance: feature.md ## Design records the choice with
  git-LFS sizing._
- [ ] T-M0-4 — Resolve Q13 (signal-level vs risk-level overlay
  composition): decide. — _acceptance: ## Design ADR row._
- [ ] T-M0-5 — Confirm or override the analyst's Q2 default (`base`
  102.3M params, 512-ctx). — _acceptance: ## Design row + memory /
  latency budget._
- [ ] T-M0-6 — ADR-NNNN — v2.5 Kronos overlay design synthesis,
  cross-linked from `spec/architecture/02-strategy-registry.md`
  and `spec/architecture/05-llm-and-reflection.md`. —
  _acceptance: ADR landed in `spec/architecture/adr/`._

## M1 — ONNX export + tract loader (developer)

- [ ] T-M1-1 — One-off script: convert Kronos `base` checkpoint to
  ONNX via `torch.onnx.export`. — _acceptance: `.onnx` artifact at
  `crates/forecast/assets/kronos-base.onnx` with SHA recorded in
  `Cargo.toml` build metadata._
- [ ] T-M1-2 — `crates/forecast/`: new crate scaffold + tract
  loader. — _acceptance: `cargo build -p forecast` clean._
- [ ] T-M1-3 — Tokenizer parity test: round-trip an OHLCV bar
  through the Rust tokenizer vs Python reference; assert
  byte-identical tokens. — _acceptance: `cargo test -p forecast
  tokenizer_parity` green._
- [ ] T-M1-4 — Inference smoke test: load `.onnx` + tract, run
  forecast on a canned OHLCV window, assert non-NaN forecast.
  — _acceptance: `cargo test -p forecast inference_smoke` green._

## M2 — Replay cache (developer; depends on M0 T-M0-2 outcome)

- [ ] T-M2-1 — `crates/forecast/src/replay.rs` (or shared
  `crates/replay-cache/`) — schema mirrors v2 LLM
  ([feature.md Q8](../v2-llm-strategy/feature.md)). —
  _acceptance: SQLite schema + `schema_version = 1` migration +
  WAL mode._
- [ ] T-M2-2 — Cache-key canonical JSON for
  `(model_revision, ohlcv_window, temperature, top_p, top_k,
  max_tokens, sampling_seed)`. — _acceptance: SHA-256 over
  `serde-canonical-json` matches an architect-pinned reference._
- [ ] T-M2-3 — Fixture cache committed at
  `crates/forecast/tests/fixtures/kronos-replay.db`. —
  _acceptance: 2 canned forecasts per BS-1 + BS-2 sampling-seed
  combo._

## M3 — Strategy impl + overlay composition (developer)

- [ ] T-M3-1 — `ForecastOverlay` type in `crates/core/` (architect
  confirms exact shape per R2.3). — _acceptance: type compiles +
  serde round-trip test._
- [ ] T-M3-2 — `crates/strategy/src/kronos_forecast.rs`:
  `Strategy` impl that pulls forecasts and emits `Signal` per the
  overlay design. — _acceptance: `cargo test -p strategy
  kronos_forecast` green._
- [ ] T-M3-3 — Compose with v1 momentum (R2.1). —
  _acceptance: integration test exercising the composed
  pipeline._
- [ ] T-M3-4 — Audit-row emission on forecast: `correlation_id`,
  forecasted OHLCV, sampling seed, cache-hit-or-miss. —
  _acceptance: journal entry shape locked + tested._

## M4 — Cost telemetry (developer; small)

- [ ] T-M4-1 — Emit `CostEvent::Infra { line: "kronos_inference",
  ... }` on every forecast (default `usd = 0`). —
  _acceptance: ledger entry visible under
  `expense:infra:kronos_inference` when operator opts in to
  non-zero `KronosConfig.energy_cost_per_kwh`._

## M5 — Backtest scenarios + anchor lock (tester)

- [ ] T-M5-1 — Run BS-1 (`top10-2024-h1-kronos-momentum`) via
  `backtest` skill. — _acceptance: backtest report committed
  + body-SHA captured._
- [ ] T-M5-2 — Run BS-2 (`top10-2024-h2-kronos-momentum`) via
  `backtest` skill. — _acceptance: same as T-M5-1._
- [ ] T-M5-3 — `scripts/pre_stage_anchors.sh` stages the 2 new
  SHAs. — _acceptance: PR-ready diff for `spec/anchors.toml`._
- [ ] T-M5-4 — `scripts/verify_anchors.sh` →
  `ANCHORS PASS (13/13)` including the 9 strategy + 2
  report-sample anchors unchanged + 2 new Kronos anchors. —
  _acceptance: stdout line + exit 0._

## M6 — Non-regression sweep (tester)

- [ ] T-M6-1 — BS-3: re-run all 9 strategy + 2 report-sample
  anchors against the v2.5 build. — _acceptance: 11/11 byte-
  identical with the existing SHAs at
  [`spec/anchors.toml`](../anchors.toml) lines 15–83._

## M7 — Final gate (tester)

- [ ] T_FINAL_V25_KRONOS — Spec-lint + verify-anchors PASS;
  evaluator VERDICT → PASS; tester locks the 2 new anchors at
  `spec/anchors.toml` and ticks `state = "shipped"` in
  `spec/trace.toml` for `REQ-V25-KRONOS-001`. — _acceptance:
  evaluator report `spec/v25-kronos-forecast-overlay/reports/
  evaluation-<ts>.md` with `VERDICT → PASS` + presenter spawn._

## Notes

- Per AGENT.md, the **developer never ticks `T_FINAL_*`** — only
  the tester does, and only after `verify-anchors` PASS.
- All write paths follow the
  [spec-update skill](../../.claude/skills/spec-update/SKILL.md);
  no raw Write/Edit to `spec/` files.
- Open questions Q1 (pre-trained vs fine-tuned) and Q11 (fine-tuning
  in v2.5 vs v2.5.x) route to the **operator** before architect can
  start; the rest are architect-decide.
- If Q3 fallback fires (ONNX conversion blocked), T-M1-1 splits into
  a fallback subprocess track per R4.3; architect reshapes M1
  accordingly.
