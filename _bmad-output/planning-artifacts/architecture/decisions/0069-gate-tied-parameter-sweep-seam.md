---
adr: 0069
title: Gate-tied hyperparameter sweep seam — a parameter-grid sibling of the bake-off, scored through the frozen robustness gate
status: accepted
date: 2026-06-24
supersedes: none
superseded-by: none
---

# ADR-0069: Gate-tied hyperparameter sweep seam (paper/sim)

## Context

Leaderboard epic item #3 (`REQ-ADVISOR-PARAM-TUNING-001`): the operator wants to
hand-tune strategy hyperparameters. The operator explicitly chose the **gate-tied
sweep** option — pick a family + parameter ranges, sweep them, and score EACH config
through the SAME frozen robustness gate so overfit configs are visibly **FRAGILE** —
NOT a naive single-config editor. The product thesis ("no active strategy robustly
beats just holding") makes an untied tuning UI an overfitting footgun; tying it to
the gate (`classify_verdict`, the 5-signal weakest-link composite, byte-frozen since
the 2026-05-30 pre-registration) makes it honest by construction.

Four facts shape this decision (all verified in code, 2026-06-24):

1. **`crates/backtest/src/bin/param_robustness_sweep.rs` is bin-only AND in the
   wrong universe.** Its `main()` (`:3397`) builds a **10-symbol cross-sectional
   momentum universe** (`top10_symbols_with_prices()`, `:3435`) + a
   `BlockBootstrapPathGen` and sweeps a `const ThetaCell` grid (`:176`, axes
   `lookback_minutes/k_long/drift_threshold`) — the momentum/MR/carry/TS/basis
   families, NOT the single-coin SMA/MACD/RSI/Bollinger rule engines the advisor
   ranks. It is **not** the seam to call. What it DOES share is already extracted to
   the library (fact 2).

2. **The verdict classifier and the single-equity bootstrap are ALREADY library
   functions** (ADR-0059 § D4 relocation):
   - `backtest::bakeoff::robustness::classify_verdict(&DistributionSummary) ->
     ParamRobustnessVerdict` (`robustness.rs:120`) + FROZEN `verdict_bands`
     (`robustness.rs:85`).
   - `backtest::bakeoff::bootstrap::compute_robustness_flag(&[Decimal], paths, seed)
     -> RobustnessFlag` (`bootstrap.rs:111`) — the EXACT per-candidate gate
     `run_bakeoff` calls (`bakeoff/mod.rs:684`): Politis–White block length +
     ChaCha20 sub-seeds + `classify_verdict`.

3. **`run_bakeoff` (`bakeoff/mod.rs:592`) is the orchestration template** — preload
   bars once (apples-to-apples), loop the field, `run_scenario` per arm,
   `compute_robustness_flag`, cancellation + `BakeoffProgress` between arms,
   mirror via `from_report` at the single `ui` boundary
   (`leaderboard/state.rs:223`). A sweep is the same shape with *parameterised
   configs of one family* substituted for *strategy ids*.

4. **Only SMA has a runtime param-override seam.** `ScenarioConfig.sma_fast_len/
   sma_slow_len` (`cli_types.rs:683-695`) → `SmaCrossover::new(fast, slow)`
   (`runtime.rs:347`). MACD/RSI/Bollinger params are **literals inside the `signal`
   DSL string** of `config/strategies/<id>.toml` (`macd_hist(12,26,9)`, `rsi(14)`,
   `bollinger_lower(20,2)`) — loaded only via `from_file`, **no override path**. BUT
   `ComposedStrategyConfig::from_str(toml_str, stem)` (`composed/config.rs:96`) parses
   a TOML **string**, so the params CAN be injected by generating the `signal` string
   in memory.

## Decision

**D1 — A new library entry point `backtest::bakeoff::sweep::run_param_sweep`, a
parameter-grid sibling of `run_bakeoff`. The `param_robustness_sweep` bin is NOT
reused.** Homed in `crates/backtest` (ADR-0059 § D1 precedent: next to
`run_scenario` + `stats` + the bootstrap; `ui` already imports `backtest` → the
result is consumable through the identical sanctioned seam with **zero new `ui` dep
edge**; the `ui→strategy` non-edge is preserved by construction). New module
`crates/backtest/src/bakeoff/sweep.rs` exposing `SweepFamily` (closed enum: Sma /
Macd / Rsi / Bollinger), `SweepGrid` (family-specific {min,max,step} axes),
`SweepConfig`, `SweepCellResult`, `SweepReport`, and
`async fn run_param_sweep(cfg, cancel_rx, progress_tx, sweep_progress_tx) ->
Result<SweepReport, RunError>`. The sweep loops the (capped) grid, runs each cell via
the EXISTING `run_scenario`, scores it via the EXISTING bootstrap, and collects per
cell `{params, in-sample KPIs, verdict, bootstrap distribution}`.

**D2 — The bootstrap distribution is surfaced, not just the flag, via an additive
behaviour-preserving sibling.** The sweep must show the operator the bootstrap
**distribution** (p5/p50/p95 + the 5 gate signals) so a gaudy in-sample config with a
negative p5 Sharpe reads as FRAGILE — that distribution IS the anti-overfitting
affordance. `compute_robustness_flag` internally builds a `DistributionSummary` then
discards it; so `bootstrap.rs` gets ONE additive sibling
`compute_robustness_distribution(&[Decimal], paths, seed) ->
Option<(DistributionSummary, ParamRobustnessVerdict)>`, and `compute_robustness_flag`
is refactored to delegate to it and drop the summary. The refactor is
**behaviour-preserving** (identical seed stream, identical block-length policy,
identical `classify_verdict` call) and is proven bit-identical by a FAIL-before test.
**No `verdict_bands` edit, no seed-rule edit — the gate stays byte-frozen** (reaffirms
ADR-0059 § D4 / ADR-0063 § D4 / ADR-0066 D3).

**D3 — Per-family param injection: SMA via the existing typed override; MACD/RSI/
Bollinger via a generated-`signal`-DSL TOML string built in memory.** A new pure
`sweep::build_swept_strategy(family, params)` returns the parameterised strategy:
for SMA it sets `sma_fast_len/sma_slow_len`; for the composed families it substitutes
the swept numbers into a per-family `signal` template, builds the TOML string, and
parses it through `ComposedStrategyConfig::from_str` (no DSL change, no new
indicator). This is the engine gap the bin never needed: **MACD/RSI/Bollinger have NO
existing runtime parameterisation** (`build_registry_for`, `runtime.rs:369-413`,
hardcodes the filename per id). Sequencing: ship **SMA-only** first (the family with
the proven seam), THEN MACD/RSI/Bollinger once `build_swept_strategy`'s string
generation is proven by a round-trip-through-`from_str` unit test.

**D4 — The grid is CAPPED at `MAX_SWEEP_CONFIGS = 24` with HONEST truncation.** Each
cell = 1 `run_scenario` + 1000 bootstrap paths (~2× the 13-arm bake-off, the ceiling
of "still an interactive click"). The cap is a `const` in `backtest::bakeoff::sweep`
(single source of truth; the UI reads it). If the operator's cartesian product
exceeds 24, the builder takes the first 24 in a deterministic axis-major order and
flags `SweepReport.truncated = true` + `requested_count`; the UI shows a "Showing 24
of {N}…" banner. **No silent drop.** Invalid cells (e.g. SMA `fast ≥ slow`) are
dropped pre-run and reported. Exhaustive sweeps are a headless-bin concern, out of
scope for the cockpit editor.

**D5 — FRAGILE is the prominent, promotion-blocking state; the result surface shows
the distribution.** The result is a grid (one row per cell) reusing the leaderboard's
Fragile-badge treatment verbatim (the `DOWN_50`/`DOWN_500` pill the
`fragile_badge_clay` guard checks). The Sharpe column shows p5/p50/p95; the four other
gate signals get columns; the in-sample point estimate is shown but de-emphasised
("the distribution is what the gate judges"). A "Use this config →" promotion
affordance is **disabled + greyed on FRAGILE rows** (mirrors the leaderboard "Fragile
cannot be crowned" lock, `rank.rs` eligibility). The shipped config is always present
as a labelled baseline row; buy-and-hold KPIs are always in the header strip.
**Promotion wiring (carrying a tuned config into F4/F5) is OUT OF SCOPE for v0.1** —
the editor *shows* the verdict; the disabled-on-fragile affordance ships now so the
honesty is visible from day 1.

**D6 — The editor is a Lab sub-view reached by drill-down, NOT a new top-level nav
screen.** A new `Screen::Tune` variant exists for routing but is **navigable, not
sidebar-default-routed** — entered via `Message::OpenTuneEditor{family,coin,lookback}`
from (a) a per-row "Tune…" button on the Leaderboard (mirroring the existing
`InspectStrategyFromLeaderboard` drill-down, `state.rs:1841`) and (b) a Lab entry
point. Adding a 13th sidebar screen for a power-user tool is rejected (proliferation).
A modal overlay (à la `leaderboard_inspect_overlay`) is an acceptable ui-designer
substitute — the state/runner/mirror design is identical either way. Result crosses
the seam as `backtest::SweepReport` mirrored into a pure-`ui` `SweepReportMirror` at a
single `from_report` boundary (the `BakeoffReportMirror` precedent).

**D7 — Async mirrors the bake-off arms 1:1.** Reuse `BakeoffProgress`'s
`{done,total,current_id}` determinate shape (as `SweepProgress`); message lifecycle
`SweepRunRequested / SweepProgress / SweepRunCompleted` + `SweepCancelRequested`,
paralleling the bake-off; `RunCancelReceiver` checked before each cell; fixtures /
no-`live` build resolves immediately with a friendly `Err` (the `spawn_bakeoff`
pattern, `runner.rs:180`). No iced thread blocking.

**D8 — Day-1 divergence gate (CLAUDE.md non-negotiable, the v3-vol-overlay-noop
precedent).** The sweep changes a decision variable (params → signals → trades →
equity), so a math + report assertion is insufficient. The required gate is
`crates/backtest/tests/param_sweep_divergence_end_to_end.rs`: over a ≥2-cell grid per
family on a ≥1-fill fixture, assert (a) ≥1 swept cell's realized equity diverges from
`report.baseline` (the shipped config) by ≥1 bp — FAIL-before = `build_swept_strategy`
silently returning the shipped config; (b) cells are not all identical to each other;
(c) a concrete SMA pin `(10,20)` ≠ `(20,50)` baseline. Modelled on
`vol_targeting_overlay_end_to_end.rs` + `combination_slate_divergence_end_to_end.rs`.

**D9 — Anchor-safe by construction (119/119) + render-pixel verification.** Every
cell runs `write_report = false` (the advisor-bake-off path, ADR-0059 § D3) → no
anchored report body written, none of the 9 `spec/anchors.toml` SHAs touched, the
frozen bands untouched → `verify_anchors.sh` stays **119/119**, no anchor-mutation ADR
required. UI verification is at the rendered-PIXEL layer per CLAUDE.md
(`param_sweep_render.rs`, `#![cfg(target_os="macos")]` per ADR-0057 § D2): a populated
grid with a FRAGILE badge + the distribution columns + a NEGATIVE control (Empty
paints no grid) + the promotion-disabled-on-fragile discriminator + the determinate
progress bar. The implementer MUST plan around the macOS CoreText/cosmic-text
font-mutex deadlock (`docs/dev-notes/iced-ui-render-verification.md`) by serialising
the screenshot harnesses, as the leaderboard suite does.

**D10 — No new dependency.** Everything reuses workspace crates; `cargo tree -p ui`
unchanged.

## Alternatives considered

- **Call/extend `param_robustness_sweep.rs` directly** — REJECTED. It is bin-only and
  built for a 10-symbol momentum universe with a `const ThetaCell` grid; its swept
  axes (`lookback_minutes/k_long/drift`) have nothing to do with the single-coin
  SMA/MACD/RSI/Bollinger params. Calling it would mean either a massive rewrite or a
  fake mapping. The clean reuse is the *library* primitives it already shares
  (`classify_verdict`, `compute_robustness_flag`), via a fresh `run_bakeoff`-shaped
  orchestrator.
- **A naive single-config editor (no gate)** — REJECTED by the operator (the whole
  point of item #3 is gate-tied). Also betrays the product thesis.
- **Show only the verdict flag per cell, not the distribution** — REJECTED. The flag
  alone hides *why* a config is fragile; the p5/p50/p95 spread is the affordance that
  teaches the operator to distrust an in-sample-gaudy/tail-negative config. Hence D2.
- **Sweep MACD/RSI/Bollinger by editing the TOML files on disk** — REJECTED. Mutating
  `config/strategies/*.toml` per cell is racy, non-deterministic, and pollutes the
  committed configs. The in-memory `from_str` path (D3) is pure and leaves disk
  untouched.
- **A 13th top-level nav screen** — REJECTED (D6): sidebar proliferation for a
  power-user tool; the drill-down + Lab-sub-view model already exists
  (`InspectStrategyFromLeaderboard`) and is the right precedent.
- **Float `k`/threshold steps with a `step` field** — REJECTED for Bollinger `k`:
  modelled as a `{1.5,2.0,2.5,3.0}` preset list to stay `Decimal`-exact (ADR-0003)
  and avoid float-step accumulation drift. Integer axes keep their {min,max,step}.
- **Open the secondary signal clauses (trend filter / support window / volume
  confirm) to the sweep in v0.1** — REJECTED. They are structural to each strategy's
  thesis, not its sensitivity knobs; sweeping them multiplies the grid and dilutes
  the honest signal. Deferred to a v0.2 "advanced" toggle as a deliberate scope cut.

## Consequences

- A second on-demand long operation joins the bake-off; both share the determinate-
  progress + cancellation discipline, so the cockpit's "interactive heavy op" budget
  is now ~2 such operations. The 24-cap keeps each within the click-not-batch ceiling.
- `bootstrap.rs` grows one additive sibling; `compute_robustness_flag`'s output is
  unchanged (proven). The frozen gate is reaffirmed, not amended.
- A real engine gap is closed: MACD/RSI/Bollinger gain in-memory parameterisation
  (`build_swept_strategy`). This is reusable beyond the sweep (e.g. a future Lab A/B
  on composed families) but is introduced minimally, SMA-first, behind a round-trip
  test.
- `ui` gains a `Screen::Tune` + a `SweepReportMirror`; NO new crate edge.
- Anchors, money-type, RNG, and the frozen bands/benchmark are all untouched →
  119/119 by construction.

## Changelog
- 2026-06-24 (architect): accepted. Initial record for feature
  `advisor-param-tuning`.
