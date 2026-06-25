---
slug: advisor-param-tuning
status: in-progress
owner: developer
updated: 2026-06-25
version: 0.1.0
phase-1-shipped: 2026-06-25   # T1–T5 (engine + mirror); T6–T11 remain
---

# Gate-tied hyperparameter sweep editor

## Why
_analyst owns the full framing; the architect summarises the operator directive here._

Leaderboard epic item #3: the operator wants to **tune strategy hyperparameters
by hand** — pick a strategy family (SMA / MACD / RSI / Bollinger), pick parameter
ranges, sweep them, and see how each config scores. The operator explicitly chose
the **gate-tied sweep** option over a naive single-config editor: every swept config
is scored through the SAME frozen robustness gate (`classify_verdict`, the 5-signal
weakest-link composite) so an overfit config that looks great in-sample but falls
apart under resampling is rendered **FRAGILE** and made visibly ineligible.

This is the honest framing the product demands. The product thesis — repeatedly
validated — is "no active strategy robustly beats just holding." A tuning UI is an
overfitting footgun *unless* the same out-of-sample-credibility ruler the bake-off
uses is applied to every config. Tying the editor to the gate makes it honest by
construction: the operator can hunt for a better config, but the gate tells the
truth about whether the config survives resampling.

## Requirements
_analyst owns; restated from the directive for the design's sake._

- R1 — The operator picks a strategy family and a parameter **range/grid** (not a
  single config). The system sweeps the grid.
- R2 — EACH swept config is scored through the frozen robustness gate. FRAGILE
  configs are prominently flagged and **cannot be promoted** (mirrors the
  leaderboard's "Fragile cannot be crowned" credibility lock).
- R3 — The result surface shows, per config: the robustness verdict + headline KPIs
  + the bootstrap **distribution** (p5/p50/p95), so the operator sees uncertainty,
  not just a single in-sample point estimate.
- R4 — Sweeps are expensive (N configs × 1000 bootstrap paths). The grid size is
  **capped**; truncation is communicated honestly.
- R5 — Async with a **determinate progress bar** + **cancellation** (mirrors the
  bake-off's `BakeoffProgress`).
- R6 — Persistent honesty copy: not advice; a FRAGILE config is overfit; the bake-off
  already searches sensible defaults; this is paper/sim.
- R7 — This is a **sibling of the leaderboard bake-off**, reusing its runner /
  progress / mirror patterns. NOT a new subsystem.

## Design
_architect (this section)._

### 0. The honest summary

This feature is the **gate-tied sweep**: a parameter-grid sibling of the bake-off.
Where `run_bakeoff` loops over N *strategy ids* and scores each through the
bootstrap gate, the sweep loops over N *parameterised configs of ONE family* and
scores each through the **identical** gate (`compute_robustness_flag` →
`classify_verdict`). The result surface is a grid of `{config → verdict + KPIs +
bootstrap distribution}` where FRAGILE is the prominent, promotion-blocking state.

It does **NOT** reuse `crates/backtest/src/bin/param_robustness_sweep.rs`. That bin
is bin-only and lives in the wrong universe (see § 2). The reused machinery is the
already-extracted **library** pieces: the verdict classifier and the single-equity
moving-block bootstrap, plus the bake-off's orchestration / mirror / progress
patterns.

### 1. Where the editor lives (design question #1)

**Decision: a Lab sub-view ("Tune"), reached from a "Tune…" affordance on a
Leaderboard row — NOT a new top-level nav screen.**

Mirror the EXISTING precedent: the leaderboard already has a row-level drill-down,
`Message::InspectStrategyFrom­Leaderboard` (`crates/ui/src/state.rs:1841`,
`crates/ui/tests/inspect_strategy_from_leaderboard.rs`), which navigates to a
strategy-detail overlay/sub-view off a leaderboard row click. The tuning editor is
the same shape: the operator ranks the field, sees (say) MACD ranked, and clicks
"Tune…" to sweep MACD's params and see how the gate judges neighbours of the
shipped config.

The nav set today (`Screen` enum, `state.rs:114`) is Lab / Live / Compare / Baseline
/ Leaderboard / ForwardPlan / Strategies / Memory / Models / Reports / Trail /
Settings. Adding a 13th top-level screen for a power-user tuning tool would
proliferate the sidebar for a feature most journeys never touch. Instead:

- The screen body lives in a new `crates/ui/src/screens/tune.rs` rendered as a
  **Lab sub-view** (the Lab is the "chart-centric workshop" — the natural home for
  A/B parameter experimentation; the Lab already does SMA fast/slow A/B via
  `sma_fast_len/slow_len`, see `cli_types.rs:683-695`).
- It is entered via `Message::OpenTuneEditor { family, coin, lookback }` dispatched
  from (a) a per-row "Tune…" button on the Leaderboard and (b) a Lab entry point.
- A new `Screen::Tune` variant exists for routing, but it is **navigable, not
  sidebar-default-routed** — reached by drill-down, exactly like the inspect overlay
  and `ForwardPlan`. (If the ui-designer prefers a modal overlay over a routed
  screen — as `leaderboard_inspect_overlay_render.rs` does — that is acceptable; the
  state/runner/mirror design below is identical either way.)

**Alternatives considered:**
- *New top-level "Tune" screen* — rejected: sidebar proliferation for a power-user
  tool; violates R7 ("reuse, don't proliferate").
- *Inline expansion inside the Leaderboard row* — rejected: a sweep grid (N rows ×
  distribution columns) is too large to nest inside one leaderboard row without
  wrecking the table layout; it needs its own canvas.
- *A Strategies-screen sub-tab* — rejected: Strategies is the registry/config
  inspector for *deployed* strategies; the sweep is an exploratory research tool,
  which is the Lab's job.

### 2. The engine seam (design question #2)

**Finding (verified in code): there is NO reusable library entry point for a
single-coin multi-family parameter sweep. One MUST be extracted. This is the first
developer task.**

What exists:
- `crates/backtest/src/bin/param_robustness_sweep.rs` is **bin-only** and operates on
  a completely different world: its `main()` (`:3397`) builds a **10-symbol
  cross-sectional momentum universe** (`top10_symbols_with_prices()`, `:3435`), a
  `BlockBootstrapPathGen`, and sweeps a **`const ThetaCell` grid** (`:176`) whose
  axes are `lookback_minutes / k_long / drift_threshold` — the momentum/MR/carry/TS/
  basis families, NOT the single-coin SMA/MACD/RSI/Bollinger rule engines. It shares
  with us ONLY the verdict classifier.
- The verdict classifier IS already a library function:
  `backtest::bakeoff::robustness::classify_verdict(&DistributionSummary) ->
  ParamRobustnessVerdict` (`robustness.rs:120`) with the FROZEN `verdict_bands`
  (`robustness.rs:85-109`). The sweep bin re-imports it (`param_robustness_sweep.rs:87`).
- The single-equity moving-block bootstrap IS already a library function:
  `backtest::bakeoff::bootstrap::compute_robustness_flag(&[Decimal], paths, seed) ->
  RobustnessFlag` (`bootstrap.rs:111`) — the EXACT function `run_bakeoff` calls per
  candidate (`bakeoff/mod.rs:684`). It returns a `RobustnessFlag` (Robust/Marginal/
  Fragile/Skipped) computed via Politis–White block length + ChaCha20 sub-seeds.
- `run_bakeoff` (`bakeoff/mod.rs:592`) is the orchestration template: preload bars
  ONCE (apples-to-apples), loop the field, `run_scenario` per arm, `compute_robustness_flag`,
  collect `CandidateResult`, cancellation + `BakeoffProgress` between arms.

**The new library entry point** — homed in `crates/backtest` (ADR-0059 § D1
precedent: the orchestrator lives next to `run_scenario` + `stats` + the bootstrap;
`ui` already imports `backtest`, so the result is consumable with **zero new `ui`
dep edge**). New module `crates/backtest/src/bakeoff/sweep.rs`:

```rust
/// One strategy family that can be swept. Closed enum (no string parsing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepFamily { Sma, Macd, Rsi, Bollinger }

/// The operator's chosen parameter grid for a family. Each axis is a closed
/// inclusive {min, max, step} range; the builder enumerates the cartesian
/// product and TRUNCATES at MAX_SWEEP_CONFIGS (see § 4 cap).
#[derive(Debug, Clone)]
pub struct SweepGrid { /* family-specific axes; see § 3 */ }

/// Config for one sweep run — mirrors BakeoffConfig.
#[derive(Debug, Clone)]
pub struct SweepConfig {
    pub family: SweepFamily,
    pub grid: SweepGrid,
    pub symbol: trading_core::Symbol,
    pub range: backtest::engine::DateRange,
    pub seed: [u8; 32],
    pub data_source: ScenarioDataSource,
    /// Bootstrap path count (default 1000 — same as the bake-off gate).
    pub paths: usize,
}

/// One swept config's outcome — the sweep analogue of CandidateResult, but
/// carrying the FULL bootstrap distribution (R3), not just the flag.
#[derive(Debug, Clone)]
pub struct SweepCellResult {
    /// The concrete params for this cell (display + identity).
    pub params: SweptParams,
    /// In-sample KPIs from the single realized run (the point estimate).
    pub kpis: CandidateKpis,
    /// The bootstrap verdict (Robust/Marginal/Fragile) — the SAME gate.
    pub verdict: ParamRobustnessVerdict,
    /// The bootstrap distribution summary (p5/p50/p95 Sharpe, prob_loss,
    /// P(Sharpe>1), p95 MaxDD) — the five gate signals, surfaced (R3).
    pub distribution: backtest::stats::DistributionSummary,
}

#[derive(Debug, Clone)]
pub struct SweepReport {
    pub config_echo: SweepRequestEcho, // family + coin + range_label + grid size + truncated?
    pub cells: Vec<SweepCellResult>,   // insertion order = grid order
    pub baseline: SweepCellResult,     // the SHIPPED config (the divergence anchor, § 8)
    pub benchmark: CandidateKpis,      // buy-and-hold KPIs (always shown — "vs just holding")
}

pub async fn run_param_sweep(
    cfg: SweepConfig,
    cancel_rx: RunCancelReceiver,
    progress_tx: ProgressSender,
    sweep_progress_tx: SweepProgressSender, // reuse BakeoffProgress shape; see § 5
) -> Result<SweepReport, crate::engine::RunError>;
```

**Why `compute_robustness_flag` is not enough on its own (R3):** the bake-off only
needs the *flag*. The sweep needs the *distribution* (p5/p50/p95) to show the
operator uncertainty. `compute_robustness_flag` internally builds the
`DistributionSummary` and then calls `classify_verdict` on it, but discards the
summary. So `bootstrap.rs` gets ONE additive sibling that returns both:

```rust
/// Like `compute_robustness_flag` but returns the full distribution summary
/// alongside the verdict (the sweep surfaces the distribution; the bake-off
/// only needs the flag). `compute_robustness_flag` is refactored to delegate
/// to this and discard the summary — BEHAVIOUR-PRESERVING (identical seed
/// stream, identical block-length policy, identical classify call).
pub fn compute_robustness_distribution(
    equity_decimals: &[Decimal],
    paths: usize,
    master_seed: u64,
) -> Option<(DistributionSummary, ParamRobustnessVerdict)>;  // None iff curve too short
```

This keeps the gate **byte-frozen** (no band touched, no seed rule touched) and
`compute_robustness_flag`'s output **unchanged** — it just stops throwing the
summary away. T1's test asserts `compute_robustness_flag` is bit-identical
before/after the refactor.

**The per-config parameterisation seam (THE crux — verified in code):**

| Family | Param injection seam | Status |
|--------|----------------------|--------|
| SMA | typed `SmaCrossover::new(fast, slow)` via the existing `ScenarioConfig.sma_fast_len/sma_slow_len` override (`cli_types.rs:683-695`, `runtime.rs:347-350`) | EXISTS |
| MACD / RSI / Bollinger | params are **literals inside the `signal` DSL string** of `config/strategies/<id>.toml` (`macd_hist(12,26,9)`, `rsi(14)`, `bollinger_lower(20,2)`); loaded via `ComposedStrategyConfig::from_file` only — **no runtime override path** | MUST BUILD |

The decisive enabler: `ComposedStrategyConfig::from_str(toml_str: &str, stem: &str)`
(`composed/config.rs:96`) parses a TOML **string**, not just a file. So the sweep
parameterises MACD/RSI/Bollinger by **generating the `signal` DSL string in memory**
(substituting the swept numbers into a per-family template), building a
`ComposedStrategy` from the generated TOML string, and running it. This is a small,
well-bounded new function (`sweep::build_swept_strategy(family, params) ->
ComposedStrategy | SmaCrossover`) and is the second crux task (T2). It reuses the
`signal` grammar VERBATIM — no DSL change, no new indicator.

> ⚠️ FLAG FOR DEVELOPER (engine gap): MACD/RSI/Bollinger have NO existing runtime
> param override — `build_registry_for` (`runtime.rs:369-413`) hardcodes the TOML
> filename per id. The sweep needs `build_swept_strategy` (T2). Until T2 lands, ONLY
> SMA is sweepable. Sequencing: ship SMA-only first (T1-T6, the full vertical slice
> on the family that already has the seam), THEN add MACD/RSI/Bollinger (T7) once
> the string-generation builder is proven by a unit test that round-trips a generated
> TOML through `from_str` and asserts the parsed AST matches a hand-written fixture.

**The ONE-boundary mirror discipline (`from_report` precedent):** `ui` consumes
`backtest::SweepReport` through the existing `backtest` dep and mirrors it into a
pure-`ui` `SweepReportMirror` at a single `from_report` seam — exactly as
`BakeoffReportMirror::from_report` (`leaderboard/state.rs:223`) is the ONLY place a
`BakeoffReport` is read. `ui` gains NO new crate edge.

### 3. What's editable per family + how a range is specified (design question #3)

Each family exposes its real indicator params as **inclusive {min, max, step}
integer ranges** (Bollinger's `k` is the one fractional axis — modelled as a small
preset list, not a float step, to stay `Decimal`-exact and avoid float-step drift).
The operator edits a min/max/step per axis; the builder enumerates the cartesian
product. To keep the UI honest and the grid bounded, each axis ALSO ships **2–3
presets** (a "narrow / shipped / wide" chip set) so the common case is one click and
the cap is rarely hit.

| Family | Axes (with the SHIPPED config as the centre) | Sweepable range guard |
|--------|----------------------------------------------|------------------------|
| **SMA** | `fast_len` (shipped 20), `slow_len` (shipped 50) | `1 ≤ fast < slow ≤ 400`; invalid `fast ≥ slow` cells are dropped (not run) and reported as skipped |
| **MACD** | `fast` (12), `slow` (26), `signal` (9) | `1 ≤ fast < slow ≤ 200`, `1 ≤ signal ≤ 50`; EMA(200) trend filter kept fixed (not swept in v0.1) |
| **RSI** | `period` (14), `oversold` threshold (30) | `2 ≤ period ≤ 100`, `5 ≤ oversold ≤ 50`; the `min(low,20)` support window kept fixed in v0.1 |
| **Bollinger** | `period` (20), `k` (2.0, preset {1.5, 2.0, 2.5, 3.0}) | `2 ≤ period ≤ 100`, `k ∈ {1.5,2.0,2.5,3.0}`; the `1.5×avg(volume,20)` confirm kept fixed in v0.1 |

Rationale for fixing the secondary clauses (trend filter / support window / volume
confirm) in v0.1: they are structural to each strategy's *thesis*, not its
sensitivity knobs; sweeping them multiplies the grid and dilutes the honest signal.
A v0.2 can open them behind an "advanced" toggle. This is recorded as a deliberate
scope cut, not an oversight.

**Grid specification UX:** per axis — a labelled min field, max field, step field
(typed `Message::SweepAxisEdit { axis, field, value }`, never a stringly-typed
blob), plus the narrow/shipped/wide preset chips. A live "**N configs → ~M
bootstrap runs (~T)**" readout updates as the operator edits, so the cost is visible
*before* pressing Run.

### 4. The grid-size cap + honest truncation (design question #3 cont.)

**Decision: `MAX_SWEEP_CONFIGS = 24` per sweep.** Cost model: each config = 1
realized `run_scenario` + 1000 bootstrap paths. The bake-off runs 13 arms ×
(1 + 1000) at the `advisor_robustness()` 1000-path setting and is already the
heaviest interactive operation; 24 configs is ~2× that — the ceiling of "still an
interactive click, not a batch job" on the determinate-progress on-demand path. The
cap is a `const` in `backtest::bakeoff::sweep` (single source of truth; the UI reads
it for the readout).

**Honest truncation:** if the operator's {min,max,step} cartesian product exceeds 24
cells, the builder takes the **first 24 in a deterministic enumeration order**
(axis-major, ascending) and sets `SweepRequestEcho.truncated = true` +
`requested_count`. The UI renders a prominent banner: *"Showing 24 of {requested}
configs — narrow your ranges or increase the step to see the rest. (Sweeps are
capped to keep each run interactive.)"* No silent drop. The truncation is part of the
report so it is testable.

(If a future operator needs an exhaustive sweep, that is a headless bin job with its
own anchored report — explicitly out of scope for the interactive cockpit editor.)

### 5. Async + progress lifecycle (design question #5)

Mirror the bake-off arms 1:1. Reuse `backtest::progress::BakeoffProgress`
(`progress.rs:41`) shape — a `{done, total, current_id}` determinate tick — via a
`SweepProgress` alias (or the same type; the developer picks). The message lifecycle
parallels `BakeoffRunRequested / BakeoffProgress / BakeoffRunCompleted`:

```text
Message::SweepRunRequested            // operator pressed "Run sweep"
  └─ runner::spawn_sweep(rt, cfg, cancel, progress_tx, sweep_progress_tx)
       └─ rt.spawn(backtest::run_param_sweep(cfg, …))  // side-thread tokio
            └─ per cell: sweep_progress_tx.try_send({done, total, current_id: params_label})
            └─ oneshot → iced::Task::perform
                 └─ Message::SweepRunCompleted(Result<SweepReportMirror, SmolStr>)
Message::SweepCancelRequested         // operator pressed "Cancel" → cancel_rx trips
```

- **Cancellation** — `RunCancelReceiver` checked before each cell (exactly as
  `run_bakeoff` checks before each arm, `bakeoff/mod.rs:633`). A cancelled sweep
  returns `RunError::Cancelled` → friendly UI state.
- **Determinate bar** — `total = grid cell count` (post-truncation), `done`
  increments per completed cell, `current_id` = the human params label (e.g.
  "fast=15, slow=40"). The leaderboard's `BakeoffProgressRecipe`
  (`live.rs:1052`) is the pattern to copy for the iced subscription.
- **Fixtures / no-`live` build** — `spawn_sweep` resolves immediately with a friendly
  `Err` ("run needs the live build"), exactly as `spawn_bakeoff` does
  (`runner.rs:180-186`), so the render harness + fixtures cockpit never hang.

### 6. The result surface + how FRAGILE is surfaced (design question #4)

A **grid/table**, one row per swept config, columns:

| Config (params) | Verdict | Return (in-sample) | Sharpe p5 / p50 / p95 | P(loss) | P(Sharpe>1) | Max-DD p95 |
|-----------------|---------|--------------------|------------------------|---------|-------------|------------|

- **FRAGILE is the prominent, promotion-blocking state.** Reuse the leaderboard's
  Fragile badge treatment verbatim: a `DOWN_50`-tinted pill with a `DOWN_500`
  "fragile" label in the params column (the same pixels the leaderboard guard checks,
  `leaderboard_populated_render.rs:415` `fragile_badge_clay`). Robust = an `UP`-tinted
  pill; Marginal = a neutral pill.
- **The bootstrap distribution is shown, not just the point estimate (R3).** The
  Sharpe column shows **p5 / p50 / p95** (a mini three-number spread), and the four
  other gate signals (P(loss), P(Sharpe>1), Max-DD p95) get their own columns. This is
  the anti-overfitting affordance made literal: the operator sees that a config with a
  gaudy in-sample return has a *negative p5 Sharpe* → the tail loses money → FRAGILE.
  The in-sample Return/Sharpe point estimate is shown but visually de-emphasised
  relative to the distribution, with a one-line caption "in-sample point estimate —
  the distribution is what the gate judges."
- **FRAGILE configs are made visibly ineligible to promote.** A "Use this config →"
  action (which would carry the params forward to a Lab A/B or a forward paper run)
  is **disabled + greyed** on FRAGILE rows, with hover/inline copy "this config is
  fragile under resampling — promoting it would be overfitting." This mirrors the
  leaderboard's "Fragile cannot be crowned" lock (`rank.rs` eligibility partition).
  Promotion wiring itself is OUT OF SCOPE for v0.1 (the editor *shows* the gate
  verdict; carrying a tuned config into F4/F5 is a v0.2 follow-on) — but the
  disabled-on-fragile affordance ships now so the honesty is visible from day 1.
- **The shipped config is always present as a labelled baseline row** ("shipped"
  tag), so the operator can see whether any swept neighbour actually beats the
  default the bake-off already uses — and the divergence gate (§ 8) keys off it.
- **Buy-and-hold KPIs** are shown in a header strip ("vs just holding {coin}: …"),
  the same "the benchmark is always in view" discipline as the leaderboard.

**Sort:** default by Sharpe p50 descending among non-fragile, fragile sink to the
bottom (mirrors `rank_candidates`' eligibility-first ordering). Pure, deterministic.

### 7. Honesty framing / copy (design question #6)

Persistent, non-dismissible footer (UI-owned strings, like the leaderboard's
not-advice disclaimer):

> *Tuning is paper/sim research, not advice. A config that looks great in-sample but
> is flagged FRAGILE is overfit — it won fit to noise that resampling dissolves. The
> bake-off already searches sensible defaults; a tuned config is only worth carrying
> forward if it is Robust AND beats just holding {coin}.*

Plus inline copy: the distribution caption (§ 6), the FRAGILE-row promotion-disabled
hover, and the truncation banner (§ 4). All strings live in `crates/ui/src/strings.rs`
(the established home), never inline in `view`.

### 8. Day-1 divergence gate (design question #8 / CLAUDE.md non-negotiable)

The sweep changes a **decision variable** (the strategy's params drive its signals,
hence its trades, hence its equity). Per the `v3-volatility-forecaster-noop-fix`
2026-05-22 precedent, a math-layer + report assertion is NOT sufficient — there MUST
be an **end-to-end test that the swept output actually DIVERGES from the default
config**, so the editor cannot silently be a no-op (e.g. if `build_swept_strategy`
forgot to apply the params and every cell secretly ran the shipped TOML).

**The gate (T-DIV, ships with T2, FAIL-before/PASS-after):**
`crates/backtest/tests/param_sweep_divergence_end_to_end.rs` — run `run_param_sweep`
over a ≥2-cell grid for EACH family on a fixture with ≥1 fill, and assert:

1. At least one swept cell's **realized equity curve** diverges from the
   `report.baseline` (shipped-config) curve by ≥ 1 bp at some bar (the params are
   actually applied — the FAIL-before is `build_swept_strategy` returning the shipped
   config for every cell).
2. The cells are **not all identical** to each other (the grid axis genuinely varies
   the strategy — distinct params produce distinct equity).
3. For SMA specifically (the family with a known-good seam), a hand-picked
   `(fast=10, slow=20)` cell differs from `(fast=20, slow=50)` baseline — a concrete
   pin, not just "something diverged."

This is modelled on `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` and
`combination_slate_divergence_end_to_end.rs` (the ADR-0067 precedent).

### 9. Testability + render verification plan (design question #7)

Per CLAUDE.md the cockpit UI is verified at the **rendered-PIXEL layer**, not
unit/text-snapshot/no-panic-boot. Plan, modelled on `leaderboard_populated_render.rs`:

**Render-layer guards** (new `crates/ui/tests/param_sweep_render.rs`, gated
`#![cfg(target_os = "macos")]` per ADR-0057 — see the font-mutex hazard below):
- `sweep_populated_paints_grid_and_fragile_badge` — a populated `SweepReportMirror`
  fixture (a mix of Robust + Marginal + ≥1 FRAGILE cell) paints: the grid rows
  (foreground floor), the **FRAGILE badge clay** in the params column (the
  load-bearing honesty pixel — reuse `fragile_badge_clay`), the distribution columns
  (p5/p50/p95 numbers drew), and the shipped-baseline row's tag.
- `sweep_empty_paints_no_grid` — **negative control**: `PanelState::Empty` ("set
  ranges and press Run sweep") paints NO grid (≈0 fragile clay, far less foreground).
  Proves the populated guard is not a tautology.
- `sweep_fragile_promote_disabled_paints` — a FRAGILE row paints its
  promotion-disabled (greyed) "Use this config" affordance, distinct from a Robust
  row's enabled accent affordance (a strictly-more-accent discriminator between a
  Robust and a Fragile row's action cell).
- `sweep_progress_determinate_paints` — mid-sweep `SweepProgress {done:3, total:12}`
  paints a partially-filled determinate bar (model the existing
  `bakeoff_progress_render.rs`).

**FAIL-before logic tests** (pure, fast, in `backtest`):
- `sweep_grid_truncates_at_cap` — a grid request of >24 cells yields exactly 24 cells
  + `truncated == true` + the right `requested_count`.
- `sweep_drops_invalid_sma_cells` — a grid where some cells have `fast ≥ slow` runs
  only the valid cells.
- `compute_robustness_distribution_matches_flag` — the new distribution fn's verdict
  is bit-identical to `compute_robustness_flag`'s on the same input (proves the
  refactor is behaviour-preserving; the gate stays frozen).
- `build_swept_strategy_macd_roundtrips` — a generated MACD TOML string parses
  through `ComposedStrategyConfig::from_str` and its AST matches a hand-written
  fixture for the same params (proves the string generation is correct, T7's gate).
- The T-DIV divergence e2e (§ 8).

**Known render hazard to plan around (flag for implementer):** the macOS
CoreText/cosmic-text **font-mutex deadlock** documented in
`spec/dev-notes/iced-ui-render-verification.md`. Render tests that spin up multiple
`iced_test::Emulator::screenshot` harnesses in one process can deadlock on the
shared font DB. The leaderboard render suite handles this; the new suite MUST follow
the same single-threaded / serialised-harness pattern (and the
`#![cfg(target_os = "macos")]` file-level gate per ADR-0057 § D2 keeps the pixel
assertions on the canonical box).

### 10. Determinism, money, anchors

- **Seed** — reuse `crate::lab::defaults::LAB_DEFAULT_SEED` (same as the bake-off);
  per-cell bootstrap master seeds via the existing `derive_master_seed(seed_u64,
  cell_index)` (`bootstrap.rs:90`) so each cell draws a distinct, reproducible
  resample stream. RNG is `ChaCha20Rng` throughout (inherited from
  `compute_robustness_flag`). No new RNG, no new seed rule.
- **Money** — all KPIs are `rust_decimal::Decimal` / `Money<Usdt>` (inherited from
  `CandidateKpis` / the equity series). No `f64` money. The only `f64` is the
  statistical Sharpe/Sortino/distribution layer, which is exactly where the bake-off
  already uses `f64` (and is `#![allow(clippy::float_arithmetic)]`-scoped).
- **Anchors** — anchor-safe BY CONSTRUCTION: `run_param_sweep` runs every cell with
  `write_report = false` (like the advisor bake-off path, ADR-0059 § D3 / runner.rs:74
  comment), so **no anchored report body is written** and none of the 9
  `spec/anchors.toml` SHAs is touched. The verdict bands + the bootstrap seed rule are
  **byte-frozen** (no `verdict_bands` edit, the `compute_robustness_flag` refactor is
  proven bit-identical by T1's test). `verify_anchors.sh` stays **119/119**. No
  anchor mutation → no anchor-mutation ADR required.

### 11. Crate / dependency check

No new dependency. Everything reuses crates already in the workspace
(`backtest`, `strategy` via `backtest`, `rust_decimal`, `smol_str`, `iced` in `ui`).
The compatibility checklist (single-binary / no system-C / edition-2024 / no
stdlib-name-shadow / maintained / license) is **N/A — zero new crates**. `cargo tree
-p ui` is unchanged (the `ui→strategy` non-edge is preserved by the
`SweepReportMirror::from_report` boundary).

## Backtest Scenarios
_analyst + architect — N/A as a new anchored scenario._ The sweep produces NO
anchored report (`write_report = false`); its correctness floor is the divergence
e2e (§ 8) + the render guards (§ 9) + the frozen-gate behaviour-preserving test (T1).
It runs against the existing pinned Binance corpus + the ADR-0061 dynamic-fetch path
the bake-off already uses (`resolve_bakeoff_bars`), so coin/lookback coverage is
identical to the leaderboard.

## Implementation

Phase 1 (T1–T5, engine foundation) shipped 2026-06-25.

**T1 — `compute_robustness_distribution` + delegation refactor**
- Added `compute_robustness_distribution` at `crates/backtest/src/bakeoff/bootstrap.rs:119`.
- Refactored `compute_robustness_flag` to delegate to it at `bootstrap.rs:177`.
- 8 bit-identity tests in `crates/backtest/tests/compute_robustness_distribution_matches_flag.rs` all pass.
- Gate bands and seed rule untouched (GOLDEN_GAMMA preserved, ChaCha20 seed derivation unchanged).

**T2 — `SweepFamily` / `SweptParams` / `SweepGrid` / `SmaGrid` / `build_swept_config` (SMA arm)**
- New module `crates/backtest/src/bakeoff/sweep.rs`.
- `MAX_SWEEP_CONFIGS = 24` at line 60 (single source of truth).
- `build_swept_config` at line 355 (SMA arm, threads `sma_fast_len/sma_slow_len` into `ScenarioConfig` with `write_report = false`).
- MACD/RSI/Bollinger are T7 stubs (SweepGrid variants present, `enumerate_and_validate` returns empty for them).
- 12 unit tests in `sweep.rs::tests` all pass.

**T3 — `run_param_sweep` orchestrator**
- At `crates/backtest/src/bakeoff/sweep.rs:481`.
- Mirrors `run_bakeoff` shape: preload bars once, run baseline first, loop grid, run buy-and-hold benchmark.
- Anchor-safe: all cells use `write_report = false` (ADR-0069 D9).
- Cancellation via `RunCancelReceiver::sibling()` checked before each cell.
- Verified via 9 integration tests (see T4).

**T4 — Day-1 divergence end-to-end gate**
- `crates/backtest/tests/param_sweep_divergence_end_to_end.rs` (9 tests).
- Primary gate `t4_swept_cells_diverge_from_baseline` at line 228.
- Concrete pin `t4_concrete_pin_fast10_slow20_differs_from_baseline` at line 354.
- FAIL-before / PASS-after control `t4_identical_params_produce_identical_equity_the_positive_control` at line 395.
- All 9 pass. Anchors: 119/119 (verified).

**T5 — `SweepReportMirror::from_report` (the ONE boundary)**
- New `crates/ui/src/tune/` module with `state.rs`.
- `SweepVerdictLabel = RobustnessLabel` type alias (reuses existing UI enum).
- `SweepDistributionMirror`, `SweepCellRow`, `SweepBenchmarkKpis`, `SweepReportMirror` at `state.rs:54-159`.
- `from_report` at `state.rs:168` (ONLY place a `backtest::SweepReport` is read).
- `promotable = !matches!(verdict, RobustnessLabel::Fragile)` at `state.rs:193`.
- `cargo tree -p ui` unchanged (no new crate edge — `backtest` was already a dep).
- 8 unit tests in `tune::state::tests` all pass.

**T7 — `build_swept_config` for MACD / RSI / Bollinger (the string-generation gap)**
- `MacdGrid`, `RsiGrid`, `BollingerGrid` structs with `Default` and `enumerate_valid()` at `sweep.rs:204-343`.
- `SweepGrid` enum upgraded from unit stubs to data-carrying variants at `sweep.rs:348-370`.
- `SweptParams` enum extended with `Macd`, `Rsi`, `Bollinger` variants at `sweep.rs:378-441`.
- TOML generation functions `macd_toml`, `rsi_toml`, `bbands_toml` at `sweep.rs:549-618`.
- `build_swept_config` extended with Macd/Rsi/Bollinger arms at `sweep.rs:620-783` (validates params, generates TOML, round-trip validates via `from_str`, sets `composed_toml_override: Some(toml_str)` in `ScenarioConfig`).
- `sma_composed_run::run` extended with `else if let Some(toml_str) = &input.composed_toml_override` branch at `sma_composed_run.rs:~124` (in-memory TOML load, bypasses disk).
- Identity guard tests (`macd/rsi/bbands_toml_shipped_params_round_trip`) at `sweep.rs:1429-1514`, `#[ignore]` (require CWD=workspace root + committed TOML).
- T7 divergence e2e extended: `t7_macd_sweep_cells_diverge_from_baseline`, `t7_rsi_sweep_cells_diverge_from_baseline`, `t7_bbands_sweep_cells_diverge_from_baseline` all pass (12/12 total in `param_sweep_divergence_end_to_end.rs`).
- 26 unit tests pass (3 ignored identity guards) in `bakeoff::sweep::tests`.

**T6 — Tune screen + guided range form + `Screen::Tune` + `OpenTuneEditor`**
- New routed `Screen::Tune` (navigable, NOT sidebar-default-routed — added to neither
  `SIDEBAR_ENTRIES_PHASE_A` nor `SIDEBAR_GROUPS_PHASE_C`; the flatten-invariant test is
  untouched). Reached via a "Tune…" drill-down off the Lab run-row (`Message::OpenTuneEditor
  {family,coin,lookback}`, mirroring `InspectStrategyFromLeaderboard`).
- `crates/ui/src/screens/tune.rs` — header + Run, range form (family chips, SMA fast/slow
  `{min,max,step}` inputs + narrow/shipped/wide presets, live cap-aware grid readout reading
  `MAX_SWEEP_CONFIGS`), result grid (params·verdict·return·Sharpe p5/p50/p95·P(loss)·
  P(Sharpe>1)·Max-DD p95·Promote), FRAGILE pill + `DOWN_500` row-wash + LOCKED promote
  affordance ("Fragile cannot be crowned"), shipped-baseline row, buy-and-hold strip,
  truncation banner, persistent honesty footer; `PanelState` loading/empty/error.
- `crates/ui/src/tune/screen_state.rs` — `TuneScreenState` (pure form + run lifecycle,
  mirroring `LeaderboardScreenState`). 7 new `Message` variants (`OpenTuneEditor`,
  `SweepSelectFamily`, `SweepAxisEdit`, `SweepAxisPreset`, `SweepRunRequested`,
  `SweepRunCompleted`, `SweepProgress`) + pure `update` arms in `state.rs`.
- ~50 `TUNE_*` strings (honesty copy per ADR-0069 §7). Zero new theme tokens. Dark + Light
  view-construction verified.
- Composed families (MACD/RSI/Bollinger) appear in the picker but `is_runnable()==false`
  (honest `TUNE_FAMILY_PENDING_NOTE`); flipping them ON is T7b (the engine already supports them).

**T8 — fixtures** — `fixtures.rs::fake_sweep_report_mirror` (Robust/Marginal/**Fragile** mix
+ baseline + buy-and-hold), `_truncated`, `fake_cockpit_tune`, `fake_cockpit_tune_running_progress`;
`test_support.rs::tune_screen_program` bare-body harness. Deterministic + engine-free.

**T9 — render-pixel guards** — `crates/ui/tests/param_sweep_render.rs` (5 guards,
`#![cfg(target_os="macos")]`, serialized). `sweep_populated_paints_grid_and_fragile_badge`
(the grid + clay FRAGILE badge + distribution columns), `sweep_empty_paints_no_grid`
(negative control — FAIL-before proven by stubbing the grid → got 0 clay px),
`sweep_fragile_promote_disabled_paints`, `sweep_allfragile_*`, `sweep_progress_determinate_paints`.
5/5 pass; PNGs written to /tmp + read (the populated grid, the locked FRAGILE row).

**T10 — runner glue** — `crates/ui/src/tune/runner.rs::spawn_sweep` (mirrors `spawn_bakeoff`:
no-`live` → immediate `Err(TUNE_RUN_NEEDS_LIVE)`; `live` → `run_param_sweep` on the side-thread
runtime → `SweepReportMirror::from_report` INSIDE the task → `Message::SweepRunCompleted`).
`sweep_config_from_state` (form → `SweepConfig`, `BinanceCache`/`LAB_DEFAULT_SEED`/`paths=1000`).
`live.rs::SweepProgressRecipe` drains the `BakeoffProgress` wire type → determinate bar;
cancellation handle held on `self.sweep_cancel` (the F4 lifetime fix), cleared on completion.
The engine `SweepReport` never crosses into iced state (the ONE mirror boundary).

**Clippy / fmt:** `cargo clippy --workspace --all-targets -- -D warnings` passes cleanly (exit 0).
**Format:** `cargo fmt -p backtest --check` passes (exit 0). Pre-existing diffs in `crates/audit/tests/` and `crates/core/` are not T7-owned.
**Anchors:** 119/119 (scripts/verify_anchors.sh verified after T7).

## Verification
_tester links to reports here._

## Changelog
- 2026-06-24 (architect): initial design — gate-tied hyperparameter sweep editor as a
  Lab sub-view sibling of the bake-off; new `backtest::bakeoff::sweep::run_param_sweep`
  library seam (the `param_robustness_sweep` bin is bin-only + wrong-universe, NOT
  reused); `compute_robustness_distribution` additive sibling surfaces the bootstrap
  distribution while keeping the gate byte-frozen; per-family param injection via the
  existing SMA override + generated-`signal`-DSL TOML strings for MACD/RSI/Bollinger;
  grid cap 24 + honest truncation; FRAGILE prominent + promotion-disabled; day-1
  divergence e2e + render-pixel guards. See ADR-0069.
