---
slug: v3-volatility-forecaster-rebaseline
status: in-progress
owner: architect
updated: 2026-05-22
parent: v3-volatility-forecaster
parent_version: 0.1.0
---

# decomp.md — v3 volatility forecaster RE-BASELINE pass (M-T1 architect decomposition)

> **Authored 2026-05-22 by the architect** after operator-decide T-OD1..T-OD3
> resolved under standing Autoapprove (Q1=(b) introduce `top10-2023-fy-momentum-realdata`;
> Q2=(a) anchor `sharpe-comparison-vol-target-bs1-realbaseline` under NEW
> `[v3.0.0-volatility-rebaseline]` namespace; Q3=(a) deliverable under
> `spec/v3-volatility-forecaster-rebaseline/reports/`). The architect resolves
> T-AR-1..T-AR-4 below; the developer takes Waves A + B in parallel followed
> by Wave C end-to-end.
>
> **Baseline anchor gate (pre-feature):** `bash scripts/verify_anchors.sh`
> reports `ANCHORS PASS  (33 / 33)` on 2026-05-22 (quoted literal line from
> the architect's run). All 33 SHAs stay byte-identical through this ship;
> N_new = +1 added at M-FINAL.

## Table of contents

1. [T-AR-1..T-AR-4 resolutions with file:line citations](#section-1)
2. [Module / file change-map](#section-2)
3. [Wave A / B / C ordered breakdown with file:line targets + cargo invocations](#section-3)
4. [Spike requirement assessment](#section-4)
5. [Rollback shape per wave](#section-5)
6. [Anchor namespace block verbatim shape](#section-6)

<a id="section-1"></a>

## 1. T-AR-1..T-AR-4 resolutions

### T-AR-1 — Scenario shape lock in `Scenario::from_name`

**Resolved → new `#[cfg(feature = "realdata")]`-gated arm
`top10-2023-fy-momentum-realdata` in
`crates/backtest/src/main.rs::Scenario::from_name`.** Mirrors the existing
realdata overlay scenarios shape (lines 449-592). ~25 LoC additive; no
refactor.

**Placement decision:** alphabetical among the `#[cfg(feature = "realdata")]`
arms. The existing realdata cluster is:

```
top10-2023-fy-tcn-overlay-realdata           (line 450)
top10-2024-fy-tcn-overlay-realdata           (line 477)
top10-2023-fy-tcn-overlay-weights-realdata   (line 500)
top10-2024-fy-tcn-overlay-weights-realdata   (line 522)
top10-2023-fy-patchtst-overlay-realdata      (line 546)
top10-2023-fy-vol-target-overlay-realdata    (line 571)
```

The new arm goes **immediately before `top10-2023-fy-patchtst-overlay-realdata`
at line 546** (alphabetical: `…-momentum-realdata` < `…-patchtst-overlay-realdata`).
The match-block ordering inside `from_name` is the file's load-bearing
documentation contract; preserving alphabetical order forestalls
arm-reordering drift between the parent v0.1.0 and this re-baseline pass.

**Locked scenario shape (exactly mirrors the parent vol-target-realdata
arm at lines 571-592 for apples-to-apples comparison):**

```rust
// v3.0.0-volatility-rebaseline: un-targeted v1 cross-sectional momentum on
// real Binance hourly data. Sibling baseline for the
// sharpe-comparison-vol-target-bs1-realbaseline report (Q1=(b) default).
// Same dataset SHA + initial_capital + slippage + fees as the parent
// vol-target-overlay-realdata scenario for apples-to-apples Sharpe delta.
#[cfg(feature = "realdata")]
"top10-2023-fy-momentum-realdata" => Ok(Self {
    name: name.to_string(),
    body_name: name.to_string(),
    body_elapsed_override: None,
    symbol: Symbol::new("multi"),
    start_year: 2023,
    // Full 2023: 365 days × 24 h = 8760 hourly bars per symbol × 10 symbols.
    bar_count: 8760,
    strategy: ScenarioStrategy::Momentum {
        config_id: "top10_momentum_h1".to_string(),
    },
    initial_capital: dec!(100_000),
    slippage_bps: 2,
    taker_fee_bps: 4,
    baseline_report: None,
    data_root,
    data_source: ScenarioDataSource::RealData,
    // Same dataset SHA as the parent vol-target-realdata scenario — pinned
    // 2026-05-18 per ADR-0032.
    expected_revision_sha: Some(
        "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
    ),
}),
```

**Apples-to-apples invariants** (verified against the parent vol-target arm
at lines 571-592 of `crates/backtest/src/main.rs`):

| Field | Value | Identical to parent vol-target arm? |
|-------|-------|--------------------------------------|
| `start_year` | 2023 | YES (line 576) |
| `bar_count` | 8760 | YES (line 577) |
| `initial_capital` | `dec!(100_000)` | YES (line 582) |
| `slippage_bps` | 2 | YES (line 583) |
| `taker_fee_bps` | 4 | YES (line 584) |
| `data_source` | `ScenarioDataSource::RealData` | YES (line 587) |
| `expected_revision_sha` | `Some("3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into())` | YES (line 589-591) |
| `strategy` | `ScenarioStrategy::Momentum { config_id: "top10_momentum_h1" }` | NO — un-targeted baseline (matches the SYNTHETIC `top10-2023-1h-momentum` arm strategy at line 292-294 verbatim) |

The `strategy` field is the **only** intentional divergence — by design
(this is the un-targeted baseline that the parent vol-target overlay is
compared against). Every other field matches by-value so the sharpe-delta
isolates the strategy variable.

**No new `ScenarioStrategy` enum variant.** The existing
`ScenarioStrategy::Momentum { config_id }` variant is reused; the new arm
plugs into the un-targeted v1 momentum dispatch path that the synthetic
`top10-2023-1h-momentum` already exercises. This keeps the diff minimal
and re-uses the existing un-targeted momentum strategy implementation
verbatim.

**Acceptance:** Wave A patch applies cleanly to a v0.1.0-anchor snapshot;
`cargo run -p backtest --release --features candle,realdata --bin
backtest -- --scenario top10-2023-fy-momentum-realdata --seed 0xC0FFEE`
emits a backtest report and asserts `data_revision_sha =
3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` in the
report frontmatter.

**Citations:**

- `crates/backtest/src/main.rs:283-321` — synthetic `top10-2023-1h-momentum`
  + `top10-2024-h1-momentum` reference shape (strategy = `Momentum { config_id
  = "top10_momentum_h1" }`).
- `crates/backtest/src/main.rs:571-592` — parent
  `top10-2023-fy-vol-target-overlay-realdata` scenario (anchor source of
  every field except `strategy`).
- `crates/backtest/src/main.rs:449-475` — existing `-realdata` arm pattern
  (cfg-gate + revision-sha pin precedent).
- ADR-0032 — realdata path + revision-pin contract.
- feature.md § R2 — operator-default invariants (architect-confirmed above).

### T-AR-2 — `sharpe_comparison.rs` patch shape lock

**Resolved → hard-coded scenario swap + NEW `ScenarioFamily::VolTargetRebaseline`
enum variant (NOT a `--baseline-scenario` CLI flag).**

**Rationale (cost/benefit):**

| Option | Cost | Benefit | Decision |
|--------|------|---------|----------|
| **(a) Hard-coded swap inside existing `VolTarget` arm.** Change `"top10-2023-1h-momentum"` → `"top10-2023-fy-momentum-realdata"` at line 1293; update 3 advisory string literals at 975/1049/1082; update output dir at 1243; update filename at 1327. | ~10 LoC; mutates the body bytes of `sharpe-comparison-vol-target-bs1-realdata` report. | **MUTATES PARENT ANCHOR `ef048366...`** — violates anchor-additive contract per ADR-0038 § D6. REJECTED. |
| **(b) NEW `ScenarioFamily::VolTargetRebaseline` variant** alongside existing `VolTarget`. New `--scenario vol-target-bs1-rebaseline` dispatch arm copies the existing `VolTarget` arm body with the 5 surface swaps. Existing `VolTarget` arm stays byte-identical. | ~30 LoC additive; parent's `--scenario vol-target-bs1` dispatch keeps emitting the byte-identical report (parent anchor `ef048366...` re-verifies on every CI run). | **DEFAULT** — preserves parent anchor; cleanly separates the rebaseline cohort; future re-baseline passes (if any) get an obvious extension point. |
| **(c) `--baseline-scenario <name>` CLI flag.** | Parameterizes the dispatch; ~50 LoC; widens the public CLI surface; tester acceptance gains a new flag-combinatorics surface. | Flexibility for future re-baseline passes (no concrete sibling demand surfaced in the 1-day scope). | **REJECTED per T-AR-3 below** — scope creep against the 1-day budget. |

**Architect chooses (b).** This is a critical correction to the M-T1 brief's
default (which proposed (a) "single hard-coded swap"): a hard-coded swap
would mutate the parent anchor `ef048366...` body and trigger an
`ANCHORS REGRESSION` at the next CI run. ADR-0038 § D6 anchor-additive
contract is non-negotiable. The NEW-variant approach adds the same total
LoC (~30 instead of ~10) but maintains byte-identity of the parent's three
`[v3.0.0-volatility]` anchors. The 20-LoC delta is the price of correctness.

**Locked patch shape:**

```rust
// crates/forecast/src/bin/sharpe_comparison.rs:43-56 (additive variant)
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ScenarioFamily {
    /// TCN + PatchTST BS-1 (default; 5 scenarios).
    Tcn,
    /// GARCH vol-targeting overlay vs SYNTHETIC v1 momentum baseline (parent;
    /// v3.0.0-volatility anchor `ef048366...`; byte-immutable).
    #[value(name = "vol-target-bs1")]
    VolTarget,
    /// GARCH vol-targeting overlay vs REAL-data v1 momentum baseline
    /// (v3.0.0-volatility-rebaseline; 2026-05-22+ T-AR-2 lock).
    #[value(name = "vol-target-bs1-rebaseline")]
    VolTargetRebaseline,
}
```

```rust
// crates/forecast/src/bin/sharpe_comparison.rs:1242-1245 (additive arm)
let out_dir: PathBuf = args.out_dir.clone().unwrap_or_else(|| match args.scenario {
    ScenarioFamily::VolTarget => PathBuf::from("spec/v3-volatility-forecaster/reports/"),
    ScenarioFamily::VolTargetRebaseline => {
        PathBuf::from("spec/v3-volatility-forecaster-rebaseline/reports/")
    }
    ScenarioFamily::Tcn => PathBuf::from("spec/v25a-patchtst-overlay/reports/"),
});
```

```rust
// crates/forecast/src/bin/sharpe_comparison.rs:1284 (new dispatch arm; ADDITIVE
// — clones the parent VolTarget arm body with the 5 surface swaps; parent arm
// at lines 1285-1352 stays byte-identical).
if args.scenario == ScenarioFamily::VolTargetRebaseline {
    if args.skip_rerun {
        anyhow::bail!("--skip-rerun is not implemented for vol-target-bs1-rebaseline.");
    }
    let tmpdir = tempfile::TempDir::new().context("creating tempdir")?;

    // Re-run REAL-DATA v1 momentum baseline + vol-target overlay (realdata).
    let vol_target_scenarios = [
        "top10-2023-fy-momentum-realdata",                  // Swap #1: T-AR-1 new scenario
        "top10-2023-fy-vol-target-overlay-realdata",        // Unchanged: byte-identical to parent
    ];

    // ... rest of arm body mirrors VolTarget arm verbatim except for the
    // calls into render_vol_target_rebaseline (see below) ...

    let filename = format!("sharpe-comparison-vol-target-bs1-realbaseline-{today}.md");
    // ^ Swap #2: filename template — "realdata" → "realbaseline" (Q2=(a) default).

    // ...
    return Ok(());
}
```

**Render module split** — to avoid mutating `render_vol_target::render_report`
(which builds the parent anchor `ef048366...` body), the new arm calls a
sibling `render_vol_target_rebaseline::render_report` that is a near-copy
of the existing render but with these advisory-string deltas:

| Line in parent module | Original literal | New literal in `render_vol_target_rebaseline` |
|----------------------|------------------|------------------------------------------------|
| 975 | `"\| Baseline scenario \| top10-2023-1h-momentum (v1 cross-sectional momentum, synthetic) \|"` | `"\| Baseline scenario \| top10-2023-fy-momentum-realdata (v1 cross-sectional momentum, real Binance data) \|"` |
| 1049 | `"\| Sharpe baseline     \| {:.6} (top10-2023-1h-momentum) \|"` | `"\| Sharpe baseline     \| {:.6} (top10-2023-fy-momentum-realdata) \|"` |
| 1082 (Notes section) | `"Baseline (top10-2023-1h-momentum) uses synthetic GBM bars; overlay uses real Binance 2023 data"` | `"Baseline (top10-2023-fy-momentum-realdata) and overlay (top10-2023-fy-vol-target-overlay-realdata) both use real Binance 2023 hourly data — apples-to-apples comparison per v0.1.0-rebaseline disambiguation."` |

The render module split keeps the parent `render_vol_target` byte-identical
(critical for the parent anchor SHA). The developer factors out shared
helpers into a `render_vol_target_common` sub-module if the duplication
exceeds 60% — architect leaves the refactor threshold to the developer's
judgement at Wave B execution time.

**File:line patch surfaces (Wave B summary):**

| # | File | Line(s) | Change | LoC |
|---|------|---------|--------|-----|
| 1 | `crates/forecast/src/bin/sharpe_comparison.rs` | 50-56 | Add `VolTargetRebaseline` variant to `ScenarioFamily` enum | +5 |
| 2 | `crates/forecast/src/bin/sharpe_comparison.rs` | 1242-1245 | Add `VolTargetRebaseline` arm to out-dir match | +3 |
| 3 | `crates/forecast/src/bin/sharpe_comparison.rs` | 1284 (insert before) | New `if args.scenario == ScenarioFamily::VolTargetRebaseline { ... }` dispatch arm (~50 LoC; clones the existing VolTarget arm body 1285-1352 with the 2 swaps locked above) | +50 |
| 4 | `crates/forecast/src/bin/sharpe_comparison.rs` | new sibling module `render_vol_target_rebaseline` (near-copy of `render_vol_target`; ~250 LoC OR shared `render_vol_target_common` factor-out if duplication >60%) | n/a | +50 to +250 |
| **TOTAL** | — | — | — | **~110-300 LoC** depending on duplication-vs-extract |

**Cap:** if `render_vol_target_rebaseline` LoC exceeds 250, the developer
MUST factor out a shared `render_vol_target_common` module. Architect
accepts either path; the test surface (R5 2-run byte-identity) decides
correctness independently of LoC.

**Acceptance:** Wave B patches apply cleanly; `cargo build -p forecast
--features candle,realdata --bin sharpe_comparison` PASS; existing
`--scenario vol-target-bs1` (parent arm) emits a byte-identical report
(anchor `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1`
re-verifies); new `--scenario vol-target-bs1-rebaseline` emits a fresh
report at the locked path with the locked filename template.

**Citations:**

- `crates/forecast/src/bin/sharpe_comparison.rs:43-56` — existing
  `ScenarioFamily` enum (extension site).
- `crates/forecast/src/bin/sharpe_comparison.rs:1242-1245` — existing
  out-dir match (extension site).
- `crates/forecast/src/bin/sharpe_comparison.rs:1284-1352` — existing
  `VolTarget` arm (the new arm clones this body byte-identical except for
  the locked swaps).
- `crates/forecast/src/bin/sharpe_comparison.rs:975`, `1049`, `1082` —
  advisory string-literal sites (preserved in parent module;
  re-implemented in the new sibling module).
- ADR-0038 § D6 — anchor-additive contract (the load-bearing reason for
  the NEW-variant approach over the hard-coded swap).
- feature.md § Q2=(a) default — `[v3.0.0-volatility-rebaseline]` namespace
  + `…-realbaseline-<YYYYMMDD>.md` filename template.

### T-AR-3 — CLI flag scope-creep rejection

**Resolved → REJECT `--baseline-scenario <name>` CLI flag for this pass.**

The analyst recommendation was hard-coded swap (rejected by T-AR-2 above for
anchor-mutation reasons). The architect's correction (NEW variant approach)
is still anchor-safe AND scope-tight: the new `--scenario
vol-target-bs1-rebaseline` keyword is a discrete extension to the existing
`ScenarioFamily` enum, not a parameterized flag. Future re-baseline passes
(if any) get a new enum variant; no public CLI surface area drift.

**Rejection rationale:**

1. **No concrete sibling demand.** Only one re-baseline pass is queued
   (this one). Adding `--baseline-scenario <name>` parameterizes for an
   unobserved use case.
2. **1-day budget.** A new flag adds (a) Clap argument validation surface,
   (b) test-matrix expansion (re-test both `vol-target-bs1` AND
   `vol-target-bs1-rebaseline` with every possible flag value), (c)
   documentation in the bin's `--help` output, (d) handoff cost between
   architect / developer / tester. Each adds wall-clock against the budget.
3. **Enum variants are the load-bearing extension shape.** The existing
   `ScenarioFamily::{Tcn, VolTarget}` pattern is the precedent the
   architect mirrors. Adding `VolTargetRebaseline` is the canonical
   extension point.

**Acceptance:** decomp.md § Scope explicitly rejects the CLI flag (this
section). Wave B introduces a new enum variant only, not a new flag.

### T-AR-4 — Anchor namespace block shape lock

**Resolved → NEW `[v3.0.0-volatility-rebaseline]` namespace block in
`spec/anchors.toml`** following the parent `[v3.0.0-volatility]` block at
lines 239-263 verbatim (commentary preamble + 1 anchor row). Net anchor
count: 33 → 34 PASS at M-FINAL.

**Locked block shape** (verbatim — to be inserted after the existing
parent `[v3.0.0-volatility]` block ends at line 263; tester writes this at
T-T-2 with the computed body-SHA-256):

```toml
# v3.0.0-volatility-rebaseline: 1-day re-baseline pass swapping the
# synthetic v1 momentum baseline for a real-data un-targeted v1 momentum
# baseline.  Spawned 2026-05-22 by operator routing pick (b) RE-BASELINE
# FIRST on parent v3-volatility-forecaster v0.1.0 (joint advisory verdict
# V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA shipped with data
# caveat).  Locked by tester on 2026-05-23+ against data/binance/
# REVISION.toml manifest SHA
# 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7.
# 2-run byte-identical (tester-verified via hash_report.py).
# V-verdict carries forward verbatim from parent: V3
# (mean_calibration_ratio = 2.952191 outside [0.7, 1.4]) — GARCH-only
# diagnostic, baseline-independent per H-rebase-2.
# T-classifier re-evaluated against new net_delta = sharpe_overlay -
# sharpe_real_baseline; cell recorded in feature.md § Verification per
# the 4-row R-O1..R-O4 routing table.
# The 33 pre-rebaseline anchors above stay byte-immutable
# (anchor-additive only per ADR-0038 § D6 + parent Q5=(a) +
# rebaseline Q2=(a)).

[[anchors]]
scenario = "sharpe-comparison-vol-target-bs1-realbaseline"
version  = "v3.0.0-volatility-rebaseline"
sha256   = "<64-hex — tester writes at T-T-2>"
```

**Anchor-additive contract** (per ADR-0038 § D6):

| Anchor | Block | SHA at this pass close | Must stay immutable? |
|--------|-------|------------------------|----------------------|
| `vol-verdict-bs1-realdata` | `[v3.0.0-volatility]` | `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21` | YES |
| `top10-2023-fy-vol-target-overlay-realdata` | `[v3.0.0-volatility]` | `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65` | YES |
| `sharpe-comparison-vol-target-bs1-realdata` | `[v3.0.0-volatility]` | `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1` | YES — **load-bearing for T-AR-2 decision** |
| `sharpe-comparison-vol-target-bs1-realbaseline` | `[v3.0.0-volatility-rebaseline]` (NEW) | `<computed at T-T-2>` | n/a — added at this pass close |

**Operator opt-in: NO baseline-backtest anchor** (Q2=(a) default, not
Q2=(b)). The new `top10-2023-fy-momentum-realdata` backtest body is
emitted at Wave A but NOT anchored. Future re-baseline passes (if
spawned under R-O2/R-O3 routing) can opt in to anchor it under the same
namespace; this pass leaves the +1 anchor delta clean.

**Acceptance:** at M-FINAL, `bash scripts/verify_anchors.sh` reports
`ANCHORS PASS  (34 / 34)`. Body-SHA-256 of the new
`sharpe-comparison-vol-target-bs1-realbaseline-<date>.md` report is
written to the `sha256 = ...` field above by the tester at T-T-2 per the
existing parent T-T2 pattern.

**Citations:**

- `spec/anchors.toml:239-263` — parent `[v3.0.0-volatility]` block
  (reference shape).
- ADR-0038 § D6 — anchor-additive contract.
- feature.md § R4 + Q2=(a) — namespace + naming lock.

<a id="section-2"></a>

## 2. Module / file change-map

### NEW files (created by developer at Waves A-B)

| File | LoC | Wave | Purpose |
|------|-----|------|---------|
| `spec/v3-volatility-forecaster-rebaseline/reports/backtest-<YYYYMMDD>-<HHMMSS>-top10-2023-fy-momentum-realdata.md` | (generated) | A | Backtest report for the new un-targeted realdata baseline; un-anchored in v0.1.0 per Q2=(a). |
| `spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-<YYYYMMDD>.md` | (generated) | B/C | Re-baselined Sharpe-delta verdict; body-SHA-256 anchored at M-FINAL under `[v3.0.0-volatility-rebaseline]`. |

### MODIFIED files (touched additively)

| File | Change | Wave | Anchor-neutrality contract |
|------|--------|------|---------------------------|
| `crates/backtest/src/main.rs` | Additive `#[cfg(feature = "realdata")]` match arm `top10-2023-fy-momentum-realdata` inserted before line 546 (alphabetical) per T-AR-1; ~25 LoC. | A | Existing match arms byte-identical; no existing scenario name re-routes. Anchors 30/30 pre-existing + 3/3 parent v3.0.0-volatility stay byte-identical. |
| `crates/forecast/src/bin/sharpe_comparison.rs` | Additive `ScenarioFamily::VolTargetRebaseline` enum variant (line 50-56); additive out-dir match arm (line 1242-1245); additive `VolTargetRebaseline` dispatch arm before line 1284 (~50 LoC); additive `render_vol_target_rebaseline` sibling module (~50-250 LoC). | B | Existing `Tcn` + `VolTarget` dispatch byte-identical; parent anchor `ef048366...` re-verifies on every CI run. |
| `spec/anchors.toml` | NEW `[v3.0.0-volatility-rebaseline]` namespace block + 1 anchor row appended after line 263 per T-AR-4. | C (M-FINAL — tester T-T-2) | Existing 33 anchor rows byte-identical. |
| `spec/trace.toml` | `REQ-V3-VOL-FORECASTER-REBASELINE-001` state `proposed → in-progress`; `arch` already populated by analyst pass — no additional rows needed; `crates` already populated by analyst pass. | (this M-T1 close) | Existing trace row content extended additively. |
| `spec/v3-volatility-forecaster-rebaseline/tasks.md` | T-AR-1..T-AR-4 ticked with literal output / decision rationale per row. Frontmatter `owner: analyst → architect`. | (this M-T1 close) | n/a — tasks.md is mutable per spec-update skill. |

### UNTOUCHED files (R10 invariants — explicit)

- `crates/forecast/src/garch.rs` — byte-identical (no model retrain per Out-of-scope).
- `crates/forecast/checkpoints/anchors/garch-bs1-<sha>.json` — byte-identical (no GARCH refit).
- `crates/strategy/src/vol_targeting_overlay.rs` — byte-identical (overlay strategy unchanged).
- `crates/strategy/config/vol_target_overlay_momentum.toml` — byte-identical (no hyperparameter tweak).
- `crates/forecast/src/bin/vol_verdict.rs` — byte-identical (V-verdict carry-forward per H-rebase-2).
- `spec/v3-volatility-forecaster/reports/*` — byte-identical (parent reports under `[v3.0.0-volatility]`).
- `spec/architecture/adr/0038-vol-forecast-verdict-shape.md` — byte-identical (no D1.c grid change; no D6 contract amendment).
- `crates/backtest/src/scenarios/garch_vol_target_overlay.rs` — byte-identical (overlay scenario unchanged).
- All other realdata scenarios in `Scenario::from_name` — byte-identical (additive arm only).
- `vendor/iced_tiny_skia/` — untouched (CLAUDE.md operator lock).
- `crates/forecast/Cargo.toml` + workspace `Cargo.toml` — byte-identical (zero new deps).

<a id="section-3"></a>

## 3. Wave A / B / C ordered breakdown

> **Honest-tick rule:** each T-D-N* / T-T* row carries the file:line target
> + the cargo invocation + the expected literal output line. The developer
> ticks the row only after running the invocation and quoting the literal
> output back into tasks.md.

### Wave A — Add realdata baseline scenario (Day 1, parallel-eligible with Wave B)

Surface: `crates/backtest/src/main.rs`. No dependency on Wave B.

| Row | Surface | cargo invocation | Expected literal |
|-----|---------|------------------|------------------|
| **T-D-N1** | `crates/backtest/src/main.rs` — insert `#[cfg(feature = "realdata")] "top10-2023-fy-momentum-realdata" => Ok(Self { ... })` arm immediately before line 546 per T-AR-1; ~25 LoC additive. | `cargo build -p backtest --features realdata,candle` | `Finished ... in ...` |
| **T-D-N2** | Run new backtest end-to-end; emit `backtest-<YYYYMMDD>-<HHMMSS>-top10-2023-fy-momentum-realdata.md` under `spec/v3-volatility-forecaster-rebaseline/reports/`. | `cargo run -p backtest --release --features candle,realdata --bin backtest -- --scenario top10-2023-fy-momentum-realdata --seed 0xC0FFEE` | `BACKTEST PASS  top10-2023-fy-momentum-realdata  body-SHA256 = <64-hex>` |
| **T-D-N3** | Confirm `data_revision_sha = 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` appears in the report frontmatter (R2 acceptance). | `grep '^data_revision_sha:' spec/v3-volatility-forecaster-rebaseline/reports/backtest-*-top10-2023-fy-momentum-realdata.md` | `data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` |
| **T-D-N4** | Confirm existing 33 anchors stay byte-identical (anchor-additive guard). | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (33 / 33)` |

### Wave B — Extend `sharpe_comparison.rs` (Day 1, parallel-eligible with Wave A)

Surface: `crates/forecast/src/bin/sharpe_comparison.rs`. No dependency on
Wave A at compile time (Wave A's scenario name is a string literal in
Wave B's new dispatch arm; the string compiles independent of whether the
scenario is currently registered). Wave A is required at **runtime** —
Wave C (end-to-end) cannot complete until Wave A's match arm exists.

| Row | Surface | cargo invocation | Expected literal |
|-----|---------|------------------|------------------|
| **T-D-N5** | `crates/forecast/src/bin/sharpe_comparison.rs:50-56` — additive `ScenarioFamily::VolTargetRebaseline` variant per T-AR-2. | `cargo build -p forecast --bin sharpe_comparison --features candle,realdata` | `Finished ... in ...` |
| **T-D-N6** | `crates/forecast/src/bin/sharpe_comparison.rs:1242-1245` — additive out-dir match arm `VolTargetRebaseline => spec/v3-volatility-forecaster-rebaseline/reports/`. | (rolled into N5 build) | (same) |
| **T-D-N7** | `crates/forecast/src/bin/sharpe_comparison.rs:1284` — additive `if args.scenario == ScenarioFamily::VolTargetRebaseline { ... }` dispatch arm; clones the existing `VolTarget` arm body verbatim except (a) `vol_target_scenarios[0] = "top10-2023-fy-momentum-realdata"`, (b) `filename = format!("sharpe-comparison-vol-target-bs1-realbaseline-{today}.md")`, (c) calls `render_vol_target_rebaseline::render_report`. | (rolled into N5 build) | (same) |
| **T-D-N8** | New `render_vol_target_rebaseline` sibling module (~50-250 LoC depending on duplication-vs-extract decision) — advisory string literals at the 3 sites swapped per T-AR-2 lock. | (rolled into N5 build) | (same) |
| **T-D-N9** | Anchor-neutrality guard: parent `vol-target-bs1` arm still emits byte-identical report → re-runs sharpe-comparison on parent arm to verify (un-anchored sanity check; the M-FINAL `verify_anchors.sh` is the load-bearing gate). | `cargo run -p forecast --release --features candle,realdata --bin sharpe_comparison -- --scenario vol-target-bs1 && bash scripts/verify_anchors.sh` | `ANCHORS PASS  (33 / 33)` (parent anchor `ef048366...` still verifies) |

### Wave C — End-to-end run + tester M-FINAL (Day 1, depends on Wave A + B)

| Row | Surface | cargo invocation | Expected literal |
|-----|---------|------------------|------------------|
| **T-D-N10** | Run new sharpe-comparison end-to-end; emit `sharpe-comparison-vol-target-bs1-realbaseline-<YYYYMMDD>.md` at the locked path. | `cargo run -p forecast --release --features candle,realdata --bin sharpe_comparison -- --scenario vol-target-bs1-rebaseline` | `wrote spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-<YYYYMMDD>.md; T-classifier = T-VOL-{ALPHA-UNLOCKED\|MARGINAL\|NO-ALPHA}` |
| **T-D-N11** | 2-run byte-identity (R5). Re-run the sharpe-comparison bin from a clean tempdir; compare body-SHA-256 via `scripts/hash_report.py` against the first-run body. | `cargo run -p forecast --release --features candle,realdata --bin sharpe_comparison -- --scenario vol-target-bs1-rebaseline && python3 scripts/hash_report.py spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-*.md` | matching body-SHA-256 across both runs |
| **T-D-N12** | Spec hygiene gates: `cargo fmt --check`, `cargo clippy --workspace --features candle,realdata -- -D warnings`, `cargo test --workspace --lib --features candle,realdata`. | (concatenated) | each: `(no output)` for fmt; `Finished` for clippy/test PASS |
| **T-T-1** | Tester M-FINAL — verify all four cargo gates per T-D-N12; quote literal outputs. | (same as N12) | (same) |
| **T-T-2** | Tester writes the new `[v3.0.0-volatility-rebaseline]` namespace block per T-AR-4 with computed body-SHA-256; re-run `verify_anchors.sh`. | `python3 scripts/hash_report.py spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-*.md && bash scripts/verify_anchors.sh` | `ANCHORS PASS  (34 / 34)` |
| **T-T-3** | Tester records the T-classifier verdict cell (R-O1 / R-O2 / R-O3 / R-O4) in `spec/v3-volatility-forecaster-rebaseline/reports/test-final-<YYYY-MM-DD>.md` per feature.md § Routes. | (no cargo invocation — spec edit only) | (test-final report written; verdict cell named) |
| **T-T-4** | HANDOFF → presenter. | (no cargo) | n/a |

### Watch recipe for the longest-running step

Backtest re-run at T-D-N2 is the longest single step (~40s per the parent
`top10-2023-fy-vol-target-overlay-realdata` precedent). Under the 2-minute
threshold of the watch-recipe rule; no `watch` block strictly required.
If the run unexpectedly exceeds 2 minutes (e.g. on a cold-cache Binance
parquet load), use:

```sh
watch -n 5 'ls -lah spec/v3-volatility-forecaster-rebaseline/reports/ 2>/dev/null | tail -20; echo "---"; pgrep -f "backtest|sharpe_comparison" | xargs -I {} ps -p {} -o pid,pcpu,etime,comm 2>/dev/null'
```

<a id="section-4"></a>

## 4. Spike requirement assessment

**Decision: NO spike required.**

- **New realdata scenario** — additive enum-arm + match-arm mirroring 6
  existing `-realdata` scenarios verbatim. Risk = MINIMAL.
- **`ScenarioFamily::VolTargetRebaseline` variant** — additive enum
  variant + dispatch arm mirroring the existing `VolTarget` arm verbatim.
  Risk = MINIMAL.
- **Sibling render module** — near-copy of `render_vol_target` with 3
  advisory-string deltas. The duplication-vs-extract refactor is a Wave B
  developer decision, not an architecture-level risk. Risk = LOW.
- **Anchor namespace addition** — additive TOML block mirroring 5+ prior
  namespace blocks verbatim. Risk = MINIMAL.

The only **architecture-level** decision under uncertainty was T-AR-2
(hard-coded-swap vs NEW-variant). The architect resolves it definitively
above by appealing to ADR-0038 § D6 anchor-additive contract — the
hard-coded swap path mutates a load-bearing parent anchor and is
structurally rejected. No spike clarifies that decision further.

**If a spike WERE required**, it would cover: confirming that the new
`top10-2023-fy-momentum-realdata` backtest emits a byte-identical body
across 2 consecutive runs from a clean tempdir. This is rolled into Wave
A T-D-N2 + Wave C T-D-N11 (the 2-run R5 determinism check). The R5 check
serves as the de-facto spike.

<a id="section-5"></a>

## 5. Rollback shape per wave

> Every wave has a clean rollback that leaves `main` in a green state.
> Rollback = `git revert <wave-commit>` works at every boundary because
> every wave's diff is additive against the previous wave's `main`.

### Wave A rollback

`git revert <Wave-A-merge-commit>` removes:

- The `top10-2023-fy-momentum-realdata` match arm in
  `crates/backtest/src/main.rs` (additive arm; revert removes it; existing
  arms untouched).
- The generated `backtest-<…>-top10-2023-fy-momentum-realdata.md` report
  under `spec/v3-volatility-forecaster-rebaseline/reports/` (safe to
  delete — not anchored in v0.1.0 per Q2=(a)).

Leaves: 33 anchored body-SHAs byte-identical (none touched); existing
backtest scenarios green; Wave B independently revertible.

### Wave B rollback

`git revert <Wave-B-merge-commit>` removes:

- `ScenarioFamily::VolTargetRebaseline` enum variant in
  `crates/forecast/src/bin/sharpe_comparison.rs:50-56`.
- Out-dir match arm at lines 1242-1245.
- The `VolTargetRebaseline` dispatch arm inserted before line 1284.
- The `render_vol_target_rebaseline` sibling module.

Leaves: parent `VolTarget` arm byte-identical (parent anchor `ef048366...`
still verifies); Wave A code intact but un-exercised (no caller invokes
`top10-2023-fy-momentum-realdata` after Wave B revert).

### Wave C rollback

`git revert <Wave-C-merge-commit>` removes the 1 anchor row from
`spec/anchors.toml` + un-flips the trace.toml state + removes the joint
advisory verdict from `feature.md § Verification`. The presenter deck (if
already written) is append-only history; rollback marks it `superseded`.

Leaves: 33-anchor baseline restored; Waves A-B code intact but un-anchored
— operator can re-trigger M-FINAL after the rollback root-cause is fixed.

<a id="section-6"></a>

## 6. Anchor namespace block verbatim shape

Locked block (tester writes at T-T-2 after computing the body-SHA-256
via `scripts/hash_report.py`):

```toml
# v3.0.0-volatility-rebaseline: 1-day re-baseline pass swapping the
# synthetic v1 momentum baseline for a real-data un-targeted v1 momentum
# baseline.  Spawned 2026-05-22 by operator routing pick (b) RE-BASELINE
# FIRST on parent v3-volatility-forecaster v0.1.0 (joint advisory verdict
# V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA shipped with data
# caveat).  Locked by tester on 2026-05-23+ against data/binance/
# REVISION.toml manifest SHA
# 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7.
# 2-run byte-identical (tester-verified via hash_report.py).
# V-verdict carries forward verbatim from parent: V3
# (mean_calibration_ratio = 2.952191 outside [0.7, 1.4]) — GARCH-only
# diagnostic, baseline-independent per H-rebase-2.
# T-classifier re-evaluated against new net_delta = sharpe_overlay -
# sharpe_real_baseline; cell recorded in feature.md § Verification per
# the 4-row R-O1..R-O4 routing table.
# The 33 pre-rebaseline anchors above stay byte-immutable
# (anchor-additive only per ADR-0038 § D6 + parent Q5=(a) +
# rebaseline Q2=(a)).

[[anchors]]
scenario = "sharpe-comparison-vol-target-bs1-realbaseline"
version  = "v3.0.0-volatility-rebaseline"
sha256   = "<64-hex — tester writes at T-T-2>"
```

**Insertion site:** append after `spec/anchors.toml` line 263 (end of the
parent `[v3.0.0-volatility]` block, which terminates after the
`sharpe-comparison-vol-target-bs1-realdata` anchor row).

## References

- [feature.md](feature.md) — R1-R5, K-rebase-1..4, H-rebase-1..2, Q1-Q3 +
  Routes table.
- [tasks.md](tasks.md) — T-A1..T-A3 (analyst, ticked); T-OD1..T-OD3
  (resolved 2026-05-22); T-AR-1..T-AR-4 (this pass); T-D-N1..T-D-N12
  (developer); T-T-1..T-T-4 (tester); T-P-1 (presenter).
- [Parent decomp.md](../v3-volatility-forecaster/decomp.md) — the
  decomposition shape this mirrors.
- [Parent feature.md](../v3-volatility-forecaster/feature.md) — parent
  brief; § Verification carries the contaminated-baseline caveat this
  pass resolves.
- [Parent presenter deck](../v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md)
  — routing pick (b) RE-BASELINE FIRST ratified.
- [ADR-0038](../architecture/adr/0038-vol-forecast-verdict-shape.md) §
  D1.c — T-classifier threshold grid (unchanged); § D6 — anchor-additive
  contract (load-bearing for T-AR-2).
- [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  — realdata path + revision-pin (the new scenario inherits revision
  `3a8b96c43f...`).
- [spec/anchors.toml:239-263](../anchors.toml) — parent
  `[v3.0.0-volatility]` block (reference shape for T-AR-4 lock).
- [spec/trace.toml:427-448](../trace.toml) —
  `REQ-V3-VOL-FORECASTER-REBASELINE-001` row (flipped to `in-progress`
  at this pass close).

## Changelog

- 2026-05-22 (architect): authored v0.1.0 decomposition. T-AR-1..T-AR-4
  resolved with file:line citations + cargo invocations + expected
  literal outputs. Critical correction to M-T1 brief's T-AR-2 default:
  hard-coded swap REJECTED (would mutate parent anchor `ef048366...`);
  NEW `ScenarioFamily::VolTargetRebaseline` variant chosen instead
  (preserves anchor-additive contract per ADR-0038 § D6). Wave A ∥ Wave
  B parallel-eligible; Wave C M-FINAL depends on both. NO spike
  required (LOW risk across all surfaces). Anchor delta: +1 at M-FINAL
  (33 → 34 PASS). Baseline anchor gate quoted literal: `ANCHORS PASS
  (33 / 33)`. HANDOFF → developer for Wave A + Wave B parallel start.
