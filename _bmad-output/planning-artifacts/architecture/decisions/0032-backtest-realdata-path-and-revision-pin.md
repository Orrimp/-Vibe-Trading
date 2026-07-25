---
adr: 0032
title: Backtest real-Binance-data path + REVISION.toml data-revision pin
status: accepted
date: 2026-05-18
supersedes: none
superseded-by: none
---

# ADR-0032: Backtest real-Binance-data path + REVISION.toml data-revision pin

## Context

The backtest harness in `crates/backtest/src/main.rs` has always
generated synthetic GBM bars via seeded `ChaCha20Rng` for every
multi-symbol scenario. M3 of `v25-tcn-overlay` (2026-05-18) shipped two
TCN checkpoints trained on **real** Binance hourly OHLCV from
`data/binance/` (val Huber loss ~1.5e-5). Both checkpoints produced
`dampened = 0` on synthetic backtest bars because the synthetic GBM's
i.i.d. Gaussian log-returns sit out-of-distribution relative to real
crypto returns (volatility clustering, fat tails, overnight gaps) —
the model's `r_hat` falls inside the ε = 0.0005 deadband on every
bar, so `Direction::Flat` on every signal. See
[`evidence/v1/v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md`](../../../../evidence/v1/v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md)
§ Finding.

The fix is to wire the backtest harness to read real Binance hourly
OHLCV from `data/binance/` for a new `-realdata` scenario family.
The `crates/forecast/` crate already does this for training
(`crates/forecast/src/features.rs:489 windows_for_symbol()` on
`load_bars()` at line 253), and the `crates/data/` crate has a
production-tested parquet reader
(`crates/data/src/replay_feed.rs:227 ReplayFeed::merge_symbols()`)
that returns `trading_core::Bar` values with the same k-way merge
key `(open_ts ASC, symbol ASC)` the synthetic path uses.

Operator-locked constraints (`spec/backtest-real-binance-data/feature.md`
§ Operator-decide questions, all resolved 2026-05-18): parallel
`-realdata` scenario family (NOT in-place replacement), universe
pinned to 10 USDT pairs on disk, wire-only scope (no alpha verdict).
The 15 existing anchors stay byte-identical.

This ADR locks the four cross-cutting decisions that the per-feature
brief defers to architect: (1) where the parquet-read path lives,
(2) the `REVISION.toml` data-revision-pin contract, (3) the
ScenarioStrategy dispatch shape, (4) the determinism contract surface
(which lines of the report body carry `data_revision_sha`).

## Decision

### 1. Parquet-read path lives in `crates/backtest/src/realdata.rs`

A new private module `crates/backtest/src/realdata.rs`, gated behind
the new cargo feature `realdata` on the `backtest` crate, owns the
real-data bar source. The module **reuses the existing
`data::ReplayFeed::merge_symbols()`** parquet reader rather than
duplicating polars code or pulling forecast's private `load_bars()`
into a cross-crate dependency.

Module surface (private, no leakage outside `crates/backtest`):

```rust
// crates/backtest/src/realdata.rs
//
// Cargo feature: realdata (off by default).
// All public items in this module are private to the crate.

pub(crate) struct RealDataBarSource {
    parquet_root: PathBuf,
    universe: Vec<Symbol>,
}

pub(crate) struct LoadedBars {
    pub bars: Vec<trading_core::Bar>,   // k-way merged, (open_ts, sym) ASC
    pub revision_sha: String,           // hex(sha256), 64 chars
}

impl RealDataBarSource {
    pub fn new(parquet_root: PathBuf, universe: Vec<Symbol>) -> Self;

    /// Load + verify + merge in one call.
    /// 1. Verify data/binance/REVISION.toml exists and is internally
    ///    consistent (the per-file SHA-256 map matches actual on-disk
    ///    SHAs for every file the scenario will read).
    /// 2. Read parquet for `universe` over `span` via
    ///    `data::ReplayFeed::merge_symbols`.
    /// 3. Enforce R3 missing-bar tolerance (>= 99.5%).
    /// 4. Force-set `local_recv_ts = close_ts` on every Bar so the two
    ///    backtest code paths (synthetic + realdata) produce
    ///    `local_recv_ts` deterministically (vs `Timestamp::now()`
    ///    inside `ReplayFeed::read_parquet_bars`). See
    ///    `crates/data/src/replay_feed.rs:196`.
    pub fn load(&self, span: TimeSpan) -> Result<LoadedBars, RealDataError>;
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum RealDataError {
    #[error("data/binance/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },
    #[error(
        "data revision mismatch for {file}: \
         manifest={manifest_sha}, on-disk={actual_sha}"
    )]
    RevisionMismatch { file: String, manifest_sha: String, actual_sha: String },
    #[error(
        "scenario {scenario} expected {expected} bars across {symbols} symbols \
         in [{span_start}..{span_end}); got {actual} ({pct:.2}% present), \
         below tolerance 99.50%"
    )]
    MissingData {
        scenario: String,
        expected: usize,
        actual: usize,
        symbols: usize,
        pct: f64,
        span_start: String,
        span_end: String,
    },
    #[error("parquet read error: {0}")]
    Feed(#[from] trading_core::FeedError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
}
```

The module **does not** depend on `crates/forecast/` — that crate is a
modeling concern, not a backtest data-path concern. The fact that
forecast and backtest both happen to read the same parquet directory
is incidental; coupling them would invert the dependency arrow
(`backtest → forecast`) and bake the inverse into the regression
gate.

### 2. `REVISION.toml` data-revision pin

**Location.** `data/binance/REVISION.toml`. One file per data root.
The file is **not** in git (`data/binance/` is already
`.gitignore`-listed) — it is regenerated locally by the operator
each time they refresh data. The `data_revision_sha` it produces is
what binds an anchor to a data revision; the manifest file itself is
local evidence.

**Schema (canonical TOML, sorted keys for determinism):**

```toml
# data/binance/REVISION.toml
#
# Generated by `cargo run -p data --bin fetch_binance_klines --
#   --emit-revision-manifest` after every fetch. Hand-editing this
# file invalidates the anchor lock — re-fetch instead.

[revision]
# Aggregate SHA over the (filename → file-sha256) sorted map below.
# Computed as sha256("{relpath}\t{sha256}\n" for each (relpath, sha256)
# in sorted order, joined; trailing newline; UTF-8). This is the
# `data_revision_sha` that lands in the report body.
sha256 = "<64 hex chars>"

# Fetch-time metadata, advisory only — NOT part of the aggregate SHA.
# Operator forensics: which fetch produced this revision?
[revision.metadata]
generated_at  = "2026-05-18T12:00:00Z"   # advisory only, RFC3339
binance_base  = "https://api.binance.com"
fetch_tool    = "fetch_binance_klines"
fetch_version = "0.1.0"                  # crate version of fetch tool
interval      = "1h"

# Sorted (lexicographic) map of relative-path → SHA-256.
# Relative to the parent of this manifest file.
# One entry per parquet file under data/binance/.
[files]
"ADAUSDT/2023/01.parquet"   = "<64 hex chars>"
"ADAUSDT/2023/02.parquet"   = "<64 hex chars>"
# ... 238 more entries ...
"XRPUSDT/2024/12.parquet"   = "<64 hex chars>"
```

**Aggregate SHA algorithm (pseudocode, identical in writer and verifier):**

```text
entries = sorted_lexicographically(files.entries())   # by relative path
buf     = b""
for (relpath, sha256) in entries:
    buf += relpath.as_bytes() + b"\t" + sha256.as_bytes() + b"\n"
revision_sha = hex(sha256(buf))
```

This algorithm is **identical** in `crates/data/src/bin/fetch_binance_klines.rs`
(writer at fetch time) and `crates/backtest/src/realdata.rs::verify_revision()`
(verifier at scenario load time). The two implementations agree byte-for-byte
or the anchor lock is meaningless.

**Generator.** `cargo run -p data --bin fetch_binance_klines --
--emit-revision-manifest` writes / updates `data/binance/REVISION.toml`.
On `--force` re-fetch the manifest is regenerated from scratch; on
incremental fetch only the changed files' SHA-256 entries are
recomputed and the aggregate `[revision].sha256` is recomputed.
Operator runs the tool; CI never writes the manifest — CI on a
machine without `data/binance/` runs default-features build (no
`realdata`) and skips the four new scenarios.

**Verifier.** `RealDataBarSource::load()` verifies in this order:

1. `data/binance/REVISION.toml` exists → otherwise `RevisionMissing`.
2. For every file the scenario will read (universe × year months),
   recompute the on-disk SHA-256 and compare to the manifest's
   `[files]` entry → otherwise `RevisionMismatch { file, ... }`.
3. Recompute the aggregate `[revision].sha256` from the manifest's
   own `[files]` entries (defense against hand-edits where someone
   updated `[files]` but forgot to recompute the aggregate) → must
   equal `[revision].sha256` in the file. Otherwise `RevisionMismatch`.

The `data_revision_sha` returned by `LoadedBars` is the **aggregate**
SHA from step 3 — never the manifest's claimed value, always the
recomputed one. This way a hand-edit cannot fool the anchor lock
even if both `[files]` and `[revision].sha256` are forged
consistently with each other (the on-disk SHAs in step 2 still
have to match the manifest, so the only way to forge is to also
modify every parquet file — at which point the operator has
explicitly chosen a new data revision and the anchor mismatch is
the loud signal we want).

### 3. ScenarioStrategy dispatch — orthogonal data-source axis

The existing `ScenarioStrategy` enum encodes the **strategy** (Sma,
Composed, Momentum, MeanReversionPairs, TcnOverlayMomentum,
TcnOverlayMomentumWeights). Adding `TcnOverlayMomentumRealData` /
`TcnOverlayMomentumWeightsRealData` variants doubles the variant count
and bakes the data axis into the strategy axis — wrong shape.

**Decision: orthogonal `ScenarioDataSource` axis on `Scenario` (not
on `ScenarioStrategy`).**

```rust
// crates/backtest/src/main.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScenarioDataSource {
    /// Seeded ChaCha20Rng GBM. Default for all v0/v0.5/v1/v1.5a/v2.5
    /// scenarios. No data-on-disk requirement.
    Synthetic,
    /// data/binance/<SYM>/<YEAR>/<MM>.parquet via RealDataBarSource.
    /// Requires --features realdata; refuses to run without
    /// data/binance/REVISION.toml. Available only on the four new
    /// `-realdata` scenarios.
    RealData,
}

struct Scenario {
    // ... existing fields ...
    data_source: ScenarioDataSource,    // NEW
    /// Pinned by R7 at lock time. None for Synthetic; Some(64-hex)
    /// for RealData and asserted on every run.
    expected_revision_sha: Option<String>,  // NEW; tester sets at M5
}
```

`ScenarioStrategy::TcnOverlayMomentum { config_id, forecaster_id }`
stays. The four new `-realdata` scenarios use the existing
`TcnOverlayMomentum` / `TcnOverlayMomentumWeights` variants with
`data_source = RealData` and an `expected_revision_sha`. The
dispatch in `main.rs` runs the same `run_tcn_overlay_backtest` /
`run_tcn_overlay_weights_backtest` function, but the bar-generation
prelude branches on `scenario.data_source`:

```rust
let bars: Vec<Bar> = match scenario.data_source {
    ScenarioDataSource::Synthetic => {
        // existing synthetic_bars_hourly path — UNCHANGED.
        synthetic_top10_bars(scenario, seed)
    }
    ScenarioDataSource::RealData => {
        #[cfg(not(feature = "realdata"))]
        anyhow::bail!(
            "scenario '{}' requires --features realdata", scenario.name
        );
        #[cfg(feature = "realdata")]
        {
            let src = realdata::RealDataBarSource::new(
                scenario.data_root.clone(),
                top10_symbols_with_prices()
                    .into_iter().map(|(s, _)| s).collect(),
            );
            let loaded = src.load(scenario.span())?;
            // Assert pinned revision matches what we just verified.
            if let Some(pinned) = &scenario.expected_revision_sha {
                if pinned != &loaded.revision_sha {
                    anyhow::bail!(
                        "data revision mismatch: scenario pinned {} \
                         but on-disk computed {}",
                        pinned, loaded.revision_sha
                    );
                }
            }
            loaded.bars
        }
    }
};
```

This shape (a) keeps the synthetic path byte-identical (no new branch
runs through it), (b) makes the new code path additive and
mockable (the `RealDataBarSource` trait surface stays
`pub(crate)`, so tests can construct one with a `tempdir` fixture),
(c) keeps the strategy logic unchanged, and (d) gives the dispatch
a single read site for `expected_revision_sha` (the assertion
above) and a single read site for the bar loader (the `match` above).

### 4. Determinism contract surface — `data_revision_sha` in two places

The report frontmatter and report body BOTH carry the
`data_revision_sha`, but only the body version is covered by the
anchor's body-SHA-256 hash. The reason is forensics vs integrity:

**Frontmatter (forensics — excluded from body hash):**

```yaml
---
scenario: top10-2023-fy-tcn-overlay-realdata
seed: 0xC0FFEE
generated: 2026-05-18T12:34:56Z
wall_clock_s: 47.3
data_source: real (Binance Vision via data/binance/, v2.6.0-realdata)
data_revision_sha: 8a7c3f1d…   # NEW — frontmatter only, advisory
baseline_report: n/a
...
---
```

**Body (anchor integrity — covered by body hash):**

The body's existing "Notes" section in `write_tcn_overlay_report`
(see `crates/backtest/src/main.rs:2059-2066`) currently ends with:

```text
- Data: synthetic hourly bars, 10 independent ChaCha20Rng streams
```

For `-realdata` scenarios the renderer emits a NEW dedicated section
between "Universe" and "Notes" (so the existing "Notes" lines stay
byte-identical for the four synthetic anchors):

```markdown
## Data source

| Field                | Value                                |
|----------------------|--------------------------------------|
| Source               | Binance Vision via data/binance/     |
| Revision SHA         | 8a7c3f1d4b2e9c0…6f8a (64 hex)        |
| Universe size        | 10 symbols                           |
| Bar interval         | 1h                                   |
| Span (UTC, half-open)| 2023-01-01T00:00:00Z .. 2024-01-01T00:00:00Z |
| Expected bars        | 87600                                |
| Loaded bars          | 87600 (100.00% present)              |
```

Six rows, fixed column widths (`%-20s` / `%-37s`), fixed ordering.
The developer renders this section ONLY when `scenario.data_source ==
RealData` — for the four existing synthetic TCN scenarios the
section is absent, preserving byte-identity with the existing
`v2.5.0` / `v2.5.0-tcn-weights` body-SHAs. Equivalently, the
`Data source` row in the existing Summary table changes from
`synthetic (seeded RNG, v2.5 tcn-overlay)` to `real (Binance Vision
via data/binance/, v2.6.0-realdata)` for `-realdata` scenarios; the
synthetic anchors continue to emit their existing string verbatim.

The `Revision SHA` row in the body is what binds the body-SHA-256
to the data revision: re-fetching Binance such that any single
parquet byte changes flips the aggregate `revision_sha`, flips the
"Revision SHA" body line, flips the body-SHA-256, fails the anchor
loudly. This is the regression-gate behaviour we want.

### Summary of new files / changes

- **NEW** `crates/backtest/src/realdata.rs` (`pub(crate)` module,
  feature-gated `realdata`).
- **NEW** `crates/backtest/src/lib.rs` re-export wrapper (just makes
  `realdata` visible to `main.rs` and integration tests under the
  feature flag — `pub(crate) use realdata::{RealDataBarSource,
  RealDataError};`).
- **NEW** cargo feature `realdata` on `crates/backtest/Cargo.toml`
  (default off; adds optional deps on `polars`, `toml`, `sha2`,
  `time` features as needed).
- **NEW** `crates/backtest/tests/realdata_revision_verify.rs`
  fixture-based test of `RealDataBarSource::load()` happy path,
  tamper-detection, revision-mismatch path.
- **MODIFIED** `crates/backtest/src/main.rs`: add
  `ScenarioDataSource` enum, four new `-realdata` scenario rows in
  `from_name`, branching prelude in TCN overlay dispatch.
- **MODIFIED** `crates/data/src/bin/fetch_binance_klines.rs`: add
  `--emit-revision-manifest` flag that writes `REVISION.toml`.
- **NEW** `data/binance/REVISION.toml` (not in git, regenerated on
  fetch).
- **NEW** 4 anchors in `spec/anchors.toml` under
  `v2.6.0-realdata`.

## Alternatives considered

- **Pull a generic `RealDataBarSource` trait into `crates/data/`** —
  rejected because it would force `crates/data/` to depend on `toml`
  + the revision-pin discipline, which is a backtest-specific
  concern (live data feeds don't need or want a manifest pin).
  Pushing the trait up the dependency graph also adds a
  cross-crate API that no other crate currently consumes,
  inflating the surface area for no immediate benefit. Revisit if
  v3 continuous-paper wants the same pin discipline.

- **Reuse `crates/forecast/src/features.rs::load_bars()` directly
  via a `pub` re-export** — rejected because it inverts the
  dependency arrow (`backtest → forecast`), and `RawBar` is a
  forecast-private type (`pub(crate) struct RawBar`). Making it
  `pub` and adding a `to_bar()` converter leaks forecast internals.
  Forecast and backtest happen to read the same parquet directory
  but for different purposes (training-window iterator vs
  backtest-bar stream); converging them is a refactor for a future
  feature, not this one.

- **In-place replacement of synthetic in the existing TCN scenarios
  (Q1 = in-place)** — rejected by operator at T-OP-1 (2026-05-18).
  Six v1/v2.5 anchors would re-anchor and the CI floor (anchors
  passing on a machine without `data/binance/`) would be lost.

- **Add `TcnOverlayMomentumRealData` / `TcnOverlayMomentumWeightsRealData`
  variants to `ScenarioStrategy`** — rejected because it doubles the
  variant count for an axis (data source) that is orthogonal to the
  axis the enum encodes (strategy logic). Adding two more forecast
  families in v2.5a / v2.5b would compound the explosion (8 variants
  for 2 axes × 4 forecasters).

- **CLI flag `--data-source realdata` instead of cargo feature** —
  rejected (R10). A cargo feature composes with the existing
  `candle` feature gating real TCN weights, keeps the binary's
  default build CI-portable (no `data/binance/` required), and the
  four new scenarios become unreachable from `Scenario::from_name`
  without the feature so the dispatch error message is compile-time
  consistent. CLI flag is strictly weaker — would require runtime
  dispatch checks in every scenario row.

- **Forward-fill missing bars** — rejected (R3). The model never
  saw forward-filled bars at training time; inventing them defeats
  the alpha evaluation this feature exists to unblock.

- **Skip-and-realign on missing bars** — rejected (R3). The k-way
  merge in `run_momentum_backtest` assumes all symbols share the
  same wall-clock bar timestamp; skipping breaks that invariant
  silently.

- **`generated_at` inside the aggregate SHA** — rejected. Fetch
  metadata would make the same data fetched twice produce different
  revision SHAs; the manifest must be deterministic over data
  content alone.

- **Per-scenario manifest** (one `REVISION.toml` per scenario) —
  rejected. The four `-realdata` scenarios overlap heavily (same
  10 symbols, 2 years between them); a single root-level manifest
  with a per-file map gives finer-grained tamper detection at no
  cost.

## Consequences

**Enforced by:**

- `bash scripts/verify_anchors.sh` — 19/19 PASS at ship. Existing 15
  anchors stay byte-identical (K6 + K10 mitigation). Run after every
  developer commit (T-D-X gate, per K10).
- `cargo test -p backtest --features realdata
  --test realdata_revision_verify` — tamper test, mismatch test,
  missing-data test, span-tolerance test.
- `cargo test -p backtest --features realdata --test determinism
  realdata_*` — two sequential runs produce byte-identical body
  SHAs for each of the four new scenarios (M4 acceptance).
- `cargo build -p backtest` (default features, no `realdata`) on a
  machine without `data/binance/` — must succeed. The four new
  scenarios are unreachable from `Scenario::from_name` without
  `realdata` so this gate is checked at build time, not run time.

**What breaks if this is violated:**

- A future developer adds a `mut` cross-cutting state (e.g. a
  shared `OnceCell<ReplayFeed>`) that the synthetic path
  accidentally observes → K6 fires: a v1 or v2.5 anchor SHA flips.
  Caught by `verify_anchors.sh`.
- Someone adds a non-deterministic field to the body (e.g.
  `Manifest generated at: <wall clock>`) → the new `-realdata`
  anchors flip between two runs. Caught by the determinism gate
  (M4 acceptance).
- The aggregate-SHA algorithm in `fetch_binance_klines` drifts from
  the one in `realdata.rs::verify_revision` (e.g. someone changes
  the separator from `\t` to space) → every `-realdata` run fails
  with `RevisionMismatch` on an unchanged disk. Caught by the
  determinism gate in CI.

**What this enables:**

- The v25-tcn-overlay tester respawn (R8 follow-on) produces a
  Sharpe-table verdict against real distributional data — the
  reason this feature exists.
- v2.5a PatchTST and v2.5b Transformer phases of the v25-dl umbrella
  inherit the same `-realdata` family at no incremental cost.
- The v2.6 forecast bake-off (REQ-V26-BAKEOFF-001) becomes runnable
  on real data, which is a hard prerequisite for picking a
  production overlay.

## Changelog
- 2026-05-18 (architect): initial accept. Locks four orthogonal
  decisions (module placement, REVISION.toml schema +
  aggregate-SHA algorithm, orthogonal `ScenarioDataSource` axis,
  `data_revision_sha` in frontmatter + body). Covers T-AR-1, T-AR-2
  decomposition surface, T-AR-3 ADR-file deliverable. Cross-refs
  REQ-BACKTEST-REALDATA-001 (`spec/trace.toml`).
