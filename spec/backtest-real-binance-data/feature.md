---
slug: backtest-real-binance-data
status: in-progress
owner: architect
updated: 2026-05-18
version: 0.1.0
predecessor: v25-tcn-overlay v2.5.0
---

# Real-Binance-data backtest path

> **Wire the backtest harness in
> [`crates/backtest/src/main.rs`](../../crates/backtest/src/main.rs) to
> read real Binance hourly OHLCV from `data/binance/` instead of the
> current `ChaCha20Rng` synthetic GBM.** Unblocks the alpha-evaluation
> gap surfaced by
> [`v25-tcn-overlay` M3](v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md)
> (TCN's `r_hat` falls inside the ε=0.0005 deadband on i.i.d. Gaussian
> returns → `dampened=0` → real-weights output byte-identical to
> passthrough). Mandatory prerequisite for the
> [v2.6 forecast bake-off](../v26-forecast-bakeoff/feature.md) — three
> forecast families (TCN / PatchTST / Transformer) cannot be compared
> head-to-head on synthetic GBM, by construction.

## Why

The backtest harness has always used a seeded `ChaCha20Rng` GBM
generator (`synthetic_bars_hourly` at
[`crates/backtest/src/main.rs:474`](../../crates/backtest/src/main.rs#L474))
for every multi-symbol scenario — the v1 momentum scenarios
(`top10-{2023,2024}-*h-momentum`), the v1.5a pairs scenarios
(`pairs-{2023,2024}-*-zscore-mr`), and the four v2.5 TCN scenarios
(`top10-{2023,2024}-fy-tcn-overlay[-weights]`). The synthetic path
was a deliberate v0/v1 simplification: deterministic by construction,
zero data dependency, fast (< 10s per scenario).

The cost of that simplification was hidden until M3 of `v25-tcn-overlay`
shipped (2026-05-18, commits `e85b25d` → `1e0b73d`). The two trained TCN
checkpoints (`tcn-bs1`, `tcn-bs2`) — trained on **real Binance hourly
OHLCV from `data/binance/`** with final val Huber loss ~1.5e-5 — produce
**zero dampenings** on the synthetic backtest data. From
[`m3-bs1-training-2026-05-18.md § Finding`](v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md):

> The TCN model's output `r_hat` falls within the epsilon=0.0005
> deadband for all synthetic bars, producing `Direction::Flat` for every
> signal […]. Synthetic data from a simple GBM (ChaCha20Rng random walk)
> has different distributional properties [than real crypto returns] —
> specifically, the log-returns are i.i.d. Gaussian, while real crypto
> returns exhibit:
> - Volatility clustering (GARCH-like)
> - Fat tails (kurtosis >> 3)
> - Overnight gaps and news spikes

This defeats v2.5 alpha evaluation entirely (we cannot tell whether the
TCN is adding signal because the input is out-of-distribution by
construction) AND defeats the v2.6 bake-off prerequisite (PatchTST and
Transformer will land with the same problem unless the data path is
fixed first). It is also a credibility issue for the project frame
("real, working, auditable agent architecture") — a backtest harness
that never sees the real distribution it was trained on is not
auditable as a strategy-evaluation tool.

**The forecast crate already reads `data/binance/` correctly.** The
working pattern is at
[`crates/forecast/src/features.rs:489`](../../crates/forecast/src/features.rs#L489)
(`windows_for_symbol()`), built on
[`load_bars()` at line 253](../../crates/forecast/src/features.rs#L253)
which uses `polars::LazyFrame::scan_parquet` against the parquet
directory layout `<root>/<SYMBOL>/<YEAR>/<MM>.parquet`. This feature
wires the same read pattern into the backtest harness — no new
dependencies, no new file format, no new schema.

Data on disk (verified 2026-05-18):

| Pair        | Years on disk | Files            |
|-------------|---------------|------------------|
| ADAUSDT     | 2023, 2024    | 12 monthly each  |
| AVAXUSDT    | 2023, 2024    | 12 monthly each  |
| BNBUSDT     | 2023, 2024    | 12 monthly each  |
| BTCUSDT     | 2023, 2024    | 12 monthly each  |
| DOGEUSDT    | 2023, 2024    | 12 monthly each  |
| DOTUSDT     | 2023, 2024    | 12 monthly each  |
| ETHUSDT     | 2023, 2024    | 12 monthly each  |
| LINKUSDT    | 2023, 2024    | 12 monthly each  |
| SOLUSDT     | 2023, 2024    | 12 monthly each  |
| XRPUSDT     | 2023, 2024    | 12 monthly each  |

Total: 240 parquet files, ~25 KB / file → **≈ 6.0 MB on disk** for 10
symbols × 2 years × 12 months. At 1h cadence, full year = 8 760 bars/sym
(2023) or 8 784 bars/sym (2024, leap year), so a 10-symbol top-10
scenario reads ~87 600 bars (≈3.5 MB resident as `RawBar` structs at
~40 bytes each). **No memory budget concerns at any plausible
multiple.**

## Requirements

### R1 — Parallel `-realdata` scenario family (closes Q1)

**Default: ADD a parallel `-realdata` scenario family. DO NOT replace
synthetic in-place.** New scenarios:

- `top10-2023-fy-tcn-overlay-realdata` (real Binance hourly, full 2023)
- `top10-2024-fy-tcn-overlay-realdata` (real Binance hourly, full 2024)
- `top10-2023-fy-tcn-overlay-weights-realdata` (same, real TCN weights)
- `top10-2024-fy-tcn-overlay-weights-realdata` (same, real TCN weights)

Existing nine synthetic-data anchors stay byte-identical and serve as
the **deterministic CI floor** — they keep the backtest harness's
strategy + risk + audit + report-renderer code honest against any
silent drift introduced by the new data-source plumbing, even if the
operator's local `data/binance/` is empty or corrupted.

**Rationale.** Replacing in-place would re-anchor the four v2.5 TCN
anchors AND the two v1 momentum anchors (`top10-2023-1h-momentum`,
`top10-2024-h1-momentum` — both currently on the synthetic path; see
[main.rs:2427-2438](../../crates/backtest/src/main.rs#L2427)) AND
potentially the two v1.5a pairs anchors. That's six locked v1/v2.5
anchors moved in one feature, which the
[regression-gate doctrine](../architecture/11-regression-gate.md)
treats as architect-approved-only migrations one at a time. The
parallel-family route preserves the audit trail (v1/v2.5 synthetic
anchors lock the *strategy/audit/report-renderer* contract; new
`-realdata` anchors lock the *real-data backtest distribution*
contract). The two contracts are orthogonal and should anchor
orthogonally.

**Cost of the parallel-family route**: scenario catalogue grows from 9
to 13 (4 new). The four new scenarios add ~6 MB of parquet read per
run and probably ~20-40 s of additional wall-clock time per scenario
(architect to confirm in M0 spike). All four are gated behind a new
`--features candle` cargo feature OR a new `--data-source realdata`
CLI flag (architect-decide in T-AR-1) so CI without `data/binance/`
never tries to read parquet.

- **Operator decision needed?** Yes — Q1 in operator-decide block.
  Analyst recommendation: parallel family. Operator may prefer the
  in-place replacement if the project frame ("real, working, auditable")
  argues that the synthetic anchors are a v0/v1 expedient that should
  be retired now that real data is on disk. That argument is honest
  but it is a load-bearing direction-of-travel call.

### R2 — Anchor-version naming (closes Q2)

The four new `-realdata` scenarios lock under version
**`v2.6.0-realdata`** in
[`spec/anchors.toml`](../anchors.toml). Naming convention follows the
existing `v2.5.0-tcn-weights` precedent (version-suffix indicates the
distinguishing axis, not a semver bump).

Existing 15 anchors (9 strategy synthetic + 2 v2.5 passthrough TCN +
2 v2.5 real-weights TCN + 2 operator-success) **stay byte-identical**
under their existing version labels (`v0`, `v0.5`, `v1`, `v1.5a`,
`v2.5.0`, `v2.5.0-tcn-weights`, `v2.0.0`).

This locks 19 anchors total at feature ship (4 new + 15 stable).

- **Operator decision needed?** No — architect-locks. The
  version-suffix pattern already shipped on v2.5.0-tcn-weights;
  reusing it keeps the operator's mental model consistent.
- **Cost if wrong**: anchor-version label is metadata only — never
  appears in the body-SHA-256 hash. Trivial to relabel pre-ship.

### R3 — Time-alignment policy on data gaps (closes Q3)

**Default: hard-fail the scenario if ANY symbol in the universe is
missing more than 0.5% of expected bars in the scenario's date span.**
Forward-fill is rejected because it silently invents data the model
never saw at training time; skip-and-realign is rejected because it
desynchronises the k-way merge (the v1 momentum strategy depends on
all symbols sharing the same wall-clock bar timestamp, see
[`run_momentum_backtest`](../../crates/backtest/src/main.rs#L596)).

**Determinism contract**: for the parquet files currently on disk
(verified 2026-05-18, 12 monthly files × 10 symbols × 2 years), the
expected bar count per scenario is `bars_per_year(year) × 10 symbols`:

- 2023 full year: 8 760 h × 10 = 87 600 expected bars.
- 2024 full year: 8 784 h × 10 = 87 840 expected bars (leap year).

Tolerance: ≥ 99.5% of expected bars must be present, i.e. **at most
438 missing bars across all 10 symbols** for 2023 (≈ 0.5% of 87 600).
The harness computes the actual bar count after k-way merge alignment
and aborts with a `MissingData` error if below tolerance — same
error path as a missing parquet file today.

This pushes the *determinism question* back onto the data-on-disk
contract (R7). If `data/binance/` is byte-stable across operator
machines, the backtest result is deterministic by construction.

- **Operator decision needed?** No — architect-locks. Forward-fill /
  skip-and-realign are options the operator can request later if hard-
  fail is too strict in practice; the analyst recommends starting
  strict.
- **Cost if wrong**: too strict → CI flakiness on minor data gaps;
  too lenient → silent invention of bars the model never saw. The
  strict default fails loudly, which is the correct direction.

### R4 — Universe snapshot policy (closes Q4)

**Default: pin the universe to the 10 USDT pairs currently on disk —
`{ADA, AVAX, BNB, BTC, DOGE, DOT, ETH, LINK, SOL, XRP}USDT`.** The
universe snapshot date is **the date `data/binance/` was last
refreshed** (recorded in a new `data/binance/REVISION.toml`
manifest — see R7).

Rationale: the top-10 USDT universe by market cap has changed over
2023-2026 (e.g. AVAX dropped out at one point, others rotated). The
*existing* `top10_symbols_with_prices()` list at
[main.rs:578](../../crates/backtest/src/main.rs#L578) hard-codes the
2023 snapshot via the start prices; the *real* universe-of-record for
this feature is whichever 10 pairs the operator has on disk. Honoring
disk over imagined-historical-top-10 keeps the universe and the data
in lockstep.

If the operator later wants to add the historical 2023-Q1 top-10
universe (which may have included BCH or LTC instead of, say, AVAX),
that is a `backtest-historical-universe` follow-on feature: it requires
fetching the historical bars first. Not in scope here.

- **Operator decision needed?** Soft yes — Q4 in operator-decide. The
  10 pairs on disk match the v1 hard-coded universe by happy
  coincidence; if the operator wants to fetch more symbols (e.g. add
  USDC pairs per v1.5b) the universe expands. Analyst recommends
  staying at the 10 USDT pairs already on disk for v0.1 ship.
- **Cost if wrong**: universe drift between disk and code surfaces as
  R3's MissingData error, which fails loudly.

### R5 — Date range mapping (closes Q5)

The existing four TCN scenario bar counts in main.rs do NOT match the
real-data bar counts on disk:

| Scenario                                | Current bar_count | Real-data bar_count |
|-----------------------------------------|-------------------|---------------------|
| `top10-2023-fy-tcn-overlay`             | 2 208             | 8 760 (full 2023)   |
| `top10-2024-fy-tcn-overlay`             | 6 600             | 8 784 (full 2024)   |
| `top10-2023-fy-tcn-overlay-weights`     | 2 208             | 8 760               |
| `top10-2024-fy-tcn-overlay-weights`     | 6 600             | 8 784               |

The synthetic scenarios use 2 208 (≈92 days) and 6 600 (≈275 days)
rather than the full year — likely an early prototype dial that was
never reconciled. The new `-realdata` scenarios use the **full year
on disk**:

- `top10-2023-fy-tcn-overlay-realdata`: 8 760 h × 10 sym = 87 600 bars
- `top10-2024-fy-tcn-overlay-realdata`: 8 784 h × 10 sym = 87 840 bars

Memory budget: `RawBar` is ~40 bytes; `Bar` (the trading_core type) is
~120 bytes including `Decimal` price fields. Full 2023+2024 in memory
for one scenario is ≈ 88 K bars × 120 B = **~10.5 MB resident** —
well under any reasonable budget. Polars parquet scan is streaming and
itself well-bounded.

- **Operator decision needed?** No — architect-locks the full-year
  default. The synthetic bar_count values (2 208 / 6 600) are
  preserved on the existing 4 synthetic-data anchors (byte-identical
  per R2) so no anchor moves.
- **Cost if wrong**: longer scenarios run slower. Full-year × 10
  symbols × the backtest loop should still be < 90 s on M-series
  hardware (architect to spike).

### R6 — Bar aggregation (closes Q6)

**Default: read 1h parquet bars directly.** No aggregation needed:
`data/binance/<SYM>/<YEAR>/<MM>.parquet` files already contain 1h
OHLCV (confirmed by inspection of the
[`load_bars` schema parser](../../crates/forecast/src/features.rs#L253)
which reads `open_time`, `high`, `low`, `close`, `volume` directly
without any resampling step).

The existing
[`crates/data/src/bar_aggregator.rs`](../../crates/data/src/bar_aggregator.rs)
is **not invoked** by this feature. If a future feature wants
sub-hourly bars (e.g. 1m for tighter exit timing), that is a separate
brief which would also need to fetch 1m parquet to disk first.

- **Operator decision needed?** No — architect-locks.
- **Cost if wrong**: nothing — we just read what's there.

### R7 — Determinism contract: data revision pinning (closes Q7)

**Default: pin every `-realdata` scenario to a `data_revision` SHA
recorded in a new `data/binance/REVISION.toml` manifest.** Each
parquet file's SHA-256 is computed once at fetch time and recorded;
the aggregate SHA over the (filename → file-SHA) sorted map gives a
`data_revision_sha`. The new scenario carries `data_revision_sha`
verbatim into the report frontmatter (excluded from body-SHA, like
`git_commit` is today) AND into the body in a fixed "Data source"
section (so the body-SHA covers it).

Why two places: frontmatter for operator forensics ("what data did
this run actually read?"); body for anchor integrity ("if you re-fetch
Binance and one bar changed, the anchor MUST fail").

If `data/binance/REVISION.toml` doesn't exist or any file's actual
SHA doesn't match the manifest, the harness aborts with a
`DataRevisionMismatch` error.

`scripts/fetch_binance_klines` is the existing tool for refreshing
the data (see
[`crates/data/src/binance.rs`](../../crates/data/src/binance.rs));
a small extension is needed to emit / update `REVISION.toml` after
each fetch. Architect to lock this as T-AR-2.

- **Operator decision needed?** No — architect-locks. The manifest
  pattern is the analog of `Cargo.lock` for data, and the regression
  gate doctrine already mandates that *all* run-varying inputs either
  stay constant or move into frontmatter.
- **Cost if wrong**: if the operator re-fetches Binance and a single
  bar changed (Binance occasionally republishes corrected klines),
  the anchor breaks loudly and the operator decides whether to
  re-lock — exactly the regression-gate flow we want.

### R8 — Scope of this feature (closes Q8)

**Default: WIRE-ONLY scope.** This feature delivers the data path +
the four new `-realdata` scenarios + the four new locked anchors.
It does NOT deliver an "alpha verdict" (Sharpe / drawdown / trade
count vs baseline) on the new scenarios — that is the
[`v25-tcn-overlay`](../v25-tcn-overlay/feature.md) alpha-gate
evaluation, which now becomes runnable once this feature ships.

**Rationale.** Splitting the wiring from the verdict keeps two
distinct review surfaces: this feature's tester verifies determinism
+ data integrity (anchor lock); v25-tcn-overlay's tester (in the next
brief revision) verifies signal quality (Sharpe table vs v1
baseline). Each PR is independently reviewable. The orchestrator can
route the alpha-verdict re-spawn to analyst/tester immediately after
this feature ships — no separate operator decision needed.

If the operator prefers a combined ship ("don't lock the data anchor
until you also have a Sharpe table"), that's the Q8 operator-decide
flip.

- **Operator decision needed?** Yes — Q8 in operator-decide block.
  Analyst recommendation: WIRE-ONLY. Wider scope risks coupling two
  unrelated review surfaces (plumbing correctness vs strategy
  signal-quality) into one tester pass.

### R9 — Crate placement and module boundaries (architect-fills detail)

Out of analyst scope. The natural shapes (the analyst is NOT
prescribing them — architect overrides freely):

- A new `crates/backtest/src/realdata.rs` (or similar) housing the
  parquet-read path, distinct from `synthetic_bars*` so the two
  data sources never share state.
- A new `data_revision` helper in `crates/data/src/` (or a new
  small `crates/data/src/revision.rs`) for the REVISION.toml read
  + verify step.
- One enum extension to `ScenarioStrategy` (or a new
  `ScenarioDataSource` axis orthogonal to strategy) so the four
  new scenarios are distinguishable from their synthetic siblings.
- The existing `data::ReplayFeed` may or may not be the right
  abstraction — architect-decide. It's currently 1m-bar oriented;
  the forecast crate's `load_bars` is more directly applicable.

### R10 — CI / cargo-features posture

Gate the four new scenarios behind one of:

- A new cargo feature `realdata` on `crates/backtest`, defaulting
  OFF, that enables the parquet-read code path AND the four new
  scenarios in the `Scenario::from_name` match arms; OR
- A CLI flag `--data-source realdata` on the backtest binary that
  forces parquet reads regardless of feature flags.

Architect-decide which one. Recommendation: cargo feature, because
it composes cleanly with the existing `candle` feature gating the
real-weights scenarios (`backtest --features "candle realdata" --
--scenario top10-2023-fy-tcn-overlay-weights-realdata`).

CI on machines without `data/binance/` builds without the `realdata`
feature; CI on machines with the data fetches first, then runs both.
The 15 existing anchors must stay PASS in both configurations (no
data dependency in the synthetic paths). T-D-X verifies this.

### Operator-decide questions

**STATUS: ALL THREE RESOLVED 2026-05-18 — operator confirmed analyst defaults.**
Architect is unblocked. See Changelog entry below for the locked direction.

The analyst surfaces three questions for operator-async lock before
the architect spawns. Each has a recommended default; the operator
may confirm-as-default or override.

1. **Q1 (R1) — Replace synthetic in-place vs add parallel `-realdata`
   family.** Analyst default: ADD parallel family. Operator may
   prefer in-place if the project frame argues synthetic anchors
   should be retired. Operator confirm required because in-place
   re-anchors six v1/v2.5 anchors (lots of churn) AND loses the
   CI-on-empty-disk floor that synthetic anchors provide today.

2. **Q4 (R4) — Universe snapshot strategy.** Analyst default: pin
   to the 10 USDT pairs currently on disk
   (`{ADA, AVAX, BNB, BTC, DOGE, DOT, ETH, LINK, SOL, XRP}USDT`).
   Operator may instead want a calendar-year-aware top-N universe
   that changes across scenarios. Default is the operator-friendly
   path; soft confirm.

3. **Q8 (R8) — Scope: wire-only vs include alpha verdict in the
   same feature.** Analyst default: WIRE-ONLY (this feature locks
   anchors; a follow-on respawn of v25-tcn-overlay's tester
   produces the Sharpe table). Operator may prefer combined ship
   if "don't anchor what you haven't justified" is the read.

Q2, Q3, Q5, Q6, Q7 are architect-lockable without operator input —
all five have a recommended default that the architect ratifies
(or principled-overrides per AGENT.md).

## Design

_architect fills this — locks the parquet read pattern, the
`ScenarioDataSource` axis (or equivalent), the REVISION.toml shape,
the cargo-feature gate, and the M0 → M-FINAL milestone breakdown
under `T-D-N` task IDs._

## Backtest Scenarios

### New `-realdata` scenarios (this feature)

| Scenario                                          | Strategy            | Forecaster      | Bar source        | Anchor SHA |
|---------------------------------------------------|---------------------|-----------------|-------------------|------------|
| `top10-2023-fy-tcn-overlay-realdata`              | TcnOverlayMomentum  | Passthrough     | Binance 1h 2023fy | locked at M-FINAL |
| `top10-2024-fy-tcn-overlay-realdata`              | TcnOverlayMomentum  | Passthrough     | Binance 1h 2024fy | locked at M-FINAL |
| `top10-2023-fy-tcn-overlay-weights-realdata`      | TcnOverlayMomentum  | tcn-bs1 weights | Binance 1h 2023fy | locked at M-FINAL |
| `top10-2024-fy-tcn-overlay-weights-realdata`      | TcnOverlayMomentum  | tcn-bs2 weights | Binance 1h 2024fy | locked at M-FINAL |

Universe: `{ADA, AVAX, BNB, BTC, DOGE, DOT, ETH, LINK, SOL, XRP}USDT`
(R4). Date span: full calendar year (R5). Data revision: pinned to
`data/binance/REVISION.toml` SHA at lock time (R7).

### Success criterion (this feature)

This feature is **wire-only** (R8). Success is:

1. The four new scenarios run to completion on a clean
   `data/binance/` checkout.
2. Two sequential runs of each scenario produce byte-identical body
   SHA-256 (determinism gate).
3. The four new anchors lock in `spec/anchors.toml` under version
   `v2.6.0-realdata`.
4. The 15 existing anchors (9 strategy synthetic + 2 v2.5
   passthrough TCN + 2 v2.5 real-weights TCN + 2 operator-success)
   stay byte-identical (anchor neutrality gate — T-T-1).

Sharpe / drawdown / trade-count verdict is **out of scope** here —
the v25-tcn-overlay tester re-spawn after ship handles that, with
the new `-realdata` reports as input.

### Carry-forward: v25-tcn-overlay synthetic anchors

The four `top10-{2023,2024}-fy-tcn-overlay[-weights]` synthetic
anchors stay byte-identical under their existing `v2.5.0` /
`v2.5.0-tcn-weights` versions. Together with the new four
`-realdata` anchors, this gives the v2.6 bake-off **eight TCN
data-points** (synthetic / real × 2023 / 2024 × passthrough / real
weights) — exactly enough to falsify "the TCN signal only works on
in-distribution data" definitively.

## Non-regression contract

| Anchor                                            | Disposition                       |
|---------------------------------------------------|-----------------------------------|
| `btc-2023-1m-sma-cross` v0                        | byte-identical                    |
| `btc-2023-1m-sma-baseline-refresh` v0             | byte-identical                    |
| `btc-2023-1m-macd-trend` v0.5                     | byte-identical                    |
| `btc-2023-1m-rsi-reversion` v0.5                  | byte-identical                    |
| `btc-2023-1m-bbands-mean-revert` v0.5             | byte-identical                    |
| `top10-2023-1h-momentum` v1                       | byte-identical (stays synthetic)  |
| `top10-2024-h1-momentum` v1                       | byte-identical (stays synthetic)  |
| `pairs-2023-zscore-mr` v1.5a                      | byte-identical                    |
| `pairs-2024-h1-zscore-mr` v1.5a                   | byte-identical                    |
| `report-sample-7d` v2.0.0                         | byte-identical                    |
| `report-sample-90d` v2.0.0                        | byte-identical                    |
| `top10-2023-fy-tcn-overlay` v2.5.0                | byte-identical (synthetic, kept)  |
| `top10-2024-fy-tcn-overlay` v2.5.0                | byte-identical (synthetic, kept)  |
| `top10-2023-fy-tcn-overlay-weights` v2.5.0-tcn-w. | byte-identical (synthetic, kept)  |
| `top10-2024-fy-tcn-overlay-weights` v2.5.0-tcn-w. | byte-identical (synthetic, kept)  |
| `top10-2023-fy-tcn-overlay-realdata` v2.6.0-rd    | **NEW — lock at M-FINAL**         |
| `top10-2024-fy-tcn-overlay-realdata` v2.6.0-rd    | **NEW — lock at M-FINAL**         |
| `top10-2023-fy-tcn-overlay-weights-realdata` v2.6.0-rd | **NEW — lock at M-FINAL**    |
| `top10-2024-fy-tcn-overlay-weights-realdata` v2.6.0-rd | **NEW — lock at M-FINAL**    |

Total at ship: 19 anchors (15 stable + 4 new).

The architecture/11-regression-gate.md table grows by 4 rows; the
new rows credit ADR-0028 (the v25-dl-forecast-overlay umbrella) +
a new ADR-0032 (or whatever number is next) the architect opens for
the data-revision-pinning rule. Architect-decide whether ADR-0032 is
strictly required or whether R7's prose in this feature.md is
sufficient.

## Risk register

| ID  | Risk                                                                                                    | Severity | Mitigation                                                                                                                                                                                                                                  |
|-----|---------------------------------------------------------------------------------------------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| K1  | Parquet schema drift between data fetched today and tomorrow (Binance changes endpoint format).         | medium   | R7 — pin the data revision SHA. Loud failure on mismatch. Schema parser is in `crates/forecast/src/features.rs:load_bars`; mirror exactly to avoid divergence.                                                                              |
| K2  | Missing data days on minor pairs (DOT had a 4 h outage Q4 2023, e.g.).                                  | medium   | R3 — hard-fail if > 0.5% missing. Tolerance band is the explicit knob. If real outages are common, architect may relax to skip-and-realign at cost of determinism guarantees (regression gate would need a per-scenario expected-bars knob). |
| K3  | Memory blow-up on full-year × 10-symbol × 1m bars.                                                      | low      | We're staying at 1h cadence (R6). Memory is ~10.5 MB resident for full year — well under any budget.                                                                                                                                        |
| K4  | Time-alignment ambiguity: one symbol's `close_time` is 23:59:59 of hour T, another's is 00:00:00 of T+1.| medium   | The forecast crate already aligns on `open_time` (load_bars sorts by `open_time` ASC). Reuse the same key. Architect verifies that all 10 symbol parquets share the same `open_time` cadence (T-AR-1).                                      |
| K5  | Determinism breakage if `data/binance/` differs between operator machines.                              | high     | R7 — REVISION.toml manifest pins SHAs. Hard-fail on mismatch. CI on machine without the manifest must NOT lock anchors.                                                                                                                     |
| K6  | The 15 existing synthetic anchors break because the parquet-read code path shares state with synthetic. | high     | Architect MUST keep the two code paths fully separate. The synthetic_bars_hourly path stays untouched. A T-T-1 anchor-neutrality gate runs `verify_anchors.sh` on every developer commit.                                                   |
| K7  | TCN inference on real data is much slower than synthetic (Metal vs CPU; cache misses) → wallclock blows up. | medium   | Reuse the v25-tcn-overlay replay cache — once the first run hits, second runs are O(1) cache reads. Architect to spike wallclock in M0.                                                                                                     |
| K8  | The two scenarios with real TCN weights require `--features candle`, but `data/binance/` may be present without candle. Operator gets a half-broken state. | low      | R10 — the `realdata` cargo feature is independent of `candle`. The four scenarios split: 2 require `realdata`, 2 require `realdata + candle`. The CLI dispatches with a clear error for missing feature combos.                              |
| K9  | The "alpha verdict" follow-on (v25-tcn-overlay re-spawn) finds the TCN has *zero* alpha on real data either. | medium   | That is signal for the v2.6 bake-off retirement decision (per v25-tcn-overlay feature.md § Backtest Scenarios). NOT a failure of this feature; this feature ships the data path, alpha is downstream.                                       |
| K10 | The four new scenarios accidentally re-anchor the v25-tcn-overlay synthetic anchors (e.g. by reading parquet when feature flag is off due to a Default-derive accident). | high     | T-T-1 negative-invariant: anchor-neutrality gate runs unconditionally. The 15 anchors stay PASS or the tester FAILs the ship.                                                                                                              |

## Implementation

_developer fills this — wave plan per architect's M0 → M-FINAL
milestones, citing the T-D-N task IDs from
[`tasks.md`](tasks.md)._

## Verification

_tester fills this — the gate is:_

1. _Two sequential runs of each of the 4 new scenarios produce
   byte-identical body-SHA-256 (R8 success criterion 2)._
2. _Four new anchors locked at `v2.6.0-realdata` (R8 success
   criterion 3)._
3. _15 existing anchors stay byte-identical
   (`bash scripts/verify_anchors.sh → ANCHORS PASS (19 / 19)`)
   (R8 success criterion 4 + K10 mitigation)._
4. _Workspace tests green (`cargo test --workspace --features realdata`
   if cargo-feature route is chosen)._
5. _`rust-validate` clean (fmt + clippy `-D warnings` + cargo-deny)._
6. _The new code paths are CI-portable: a build on a machine without
   `data/binance/` (default-features-only) passes all 15 existing
   anchors and skips the 4 new ones with a clean MISSING DATA error
   (R10)._

## Changelog

- 2026-05-18 (operator): **All three operator-decides confirmed at analyst
  defaults.** Q1 → parallel `-realdata` family (NOT in-place). Q4 → pin
  universe to 10 USDT pairs on disk. Q8 → wire-only scope (alpha verdict
  follow-on via v25-tcn tester re-spawn). T-OP-1, T-OP-2, T-OP-3 ticked.
  Architect unblocked. HANDOFF → architect.
- 2026-05-18 (analyst): full analyst pass. Closed Q1-Q8 with defaults
  (R1-R10). Three operator-decide questions surfaced (Q1 in-place vs
  parallel `-realdata` family — strong recommendation: parallel; Q4
  universe snapshot strategy — recommend disk-pin; Q8 wire-only vs
  combined alpha verdict — strong recommendation: wire-only). No
  sources beyond the working real-parquet read pattern at
  `crates/forecast/src/features.rs:windows_for_symbol()` +
  `load_bars()`, the M3 training reports' "TCN outputs Flat on
  synthetic data" finding, and the regression-gate doctrine. Trace
  row `REQ-BACKTEST-REALDATA-001` created in proposed state. Status:
  draft → draft (awaits 3 operator-decide responses). Owner:
  pending-analyst → analyst. HANDOFF → operator-decide (3 Qs) →
  architect.
