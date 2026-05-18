---
slug: backtest-real-binance-data
status: in-progress
owner: architect
updated: 2026-05-18
---

# Tasks — backtest-real-binance-data

> **Analyst-authored milestone skeleton.** Task IDs (`T-AR-N`,
> `T-D-N`, `T-T-N`) are NOT yet decomposed beyond the milestone
> level — that is the architect's purview after the three
> operator-decide questions in
> [`feature.md`](feature.md#operator-decide-questions) land.
>
> The wave order below is the recommended sequence; the architect
> is free to reorder within the M0 → M-FINAL frame as long as
> every milestone's acceptance criterion is met before the next
> begins.

## Milestone skeleton

### M0 — Spike + architect lock

- [ ] **T-AR-1** — architect lock on R9 (crate placement / module
  boundaries) + R10 (cargo-feature vs CLI flag gating). Output:
  a Design section in [`feature.md`](feature.md#design) replacing
  the "_architect fills this_" stub, plus an updated trace row
  `REQ-BACKTEST-REALDATA-001` carrying the `arch` paths.
  _acceptance_: Design section landed with named files / module
  boundaries and a chosen feature-gate strategy; trace row passes
  `spec-lint` clean.

- [ ] **T-AR-2** — architect decomposes M1-M-FINAL into ordered
  `T-D-N` developer tasks (each with one-line acceptance). The
  developer should be able to start without further architect
  input.
  _acceptance_: tasks.md M1-M-FINAL sections carry concrete
  T-D-N rows.

- [ ] **T-AR-3** — architect opens (or principled-defers) an ADR
  for the data-revision pinning rule (R7) — likely ADR-0032 or
  next-available number. The ADR captures the canonical-JSON
  shape of `REVISION.toml` + the verify-vs-recompute discipline.
  _acceptance_: ADR row in `spec/architecture/adr/README.md` and
  the section file `spec/architecture/01-data-flow.md` updated to
  reference it, OR a recorded principled deferral in the
  architect's HANDOFF envelope.

- [x] **T-OP-1** (2026-05-18) — Q1 **RESOLVED: parallel `-realdata` family**
  (analyst default accepted). Existing 15 anchors stay byte-identical; new
  scenarios lock under version `v2.6.0-realdata`. Audit trail: orchestrator
  prompt confirmed all three operator-decides at analyst recommendation.

- [x] **T-OP-2** (2026-05-18) — Q4 **RESOLVED: pin to 10 USDT pairs on disk**
  (analyst default accepted). Universe = {ADA, AVAX, BNB, BTC, DOGE, DOT,
  ETH, LINK, SOL, XRP}USDT — matches the v25-tcn training universe; no
  separate snapshot-date logic.

- [x] **T-OP-3** (2026-05-18) — Q8 **RESOLVED: wire-only scope** (analyst
  default accepted). This feature ships the parquet-read path + 4 new
  `-realdata` anchors + non-regression on the 15 originals. The v2.5 alpha
  verdict (Sharpe / drawdown / trade-count comparison) is a follow-on
  v25-tcn tester re-spawn against the new anchors, tracked as a separate
  backlog item.

### M1 — Parquet read path

Wire the parquet-read code path into the backtest harness without
touching any existing synthetic scenario. End-to-end test:
`cargo run -p backtest --release --features realdata -- --scenario
top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE` runs to
completion and produces a report.

_acceptance_: a developer can read 87 600 bars (10 symbols × 8 760 h)
from `data/binance/` into the existing `MomentumStrategy` /
`TcnOverlayMomentumStrategy` flow and write a report file. K-way
merge alignment (by `open_time`) holds. R3 hard-fail on > 0.5%
missing bars fires correctly under a synthetic data-gap injection
test.

### M2 — Data revision manifest

Add `data/binance/REVISION.toml` generation (one-time at fetch
time) and verify-on-read. The four new scenarios refuse to run if
the manifest is missing or mismatched.

_acceptance_: `scripts/fetch_binance_klines` (or its successor)
emits / updates `REVISION.toml` with a per-file SHA-256 map; the
backtest harness validates against it before reading any bar; a
deliberate tamper of one byte in one parquet file produces a
`DataRevisionMismatch` error and exits non-zero.

### M3 — Wire the four new scenarios

Add the four `-realdata` scenario rows to `Scenario::from_name`,
gated behind the chosen feature flag (per T-AR-1). The two
`-weights-realdata` scenarios additionally require `--features
candle`.

_acceptance_:

- `backtest --features realdata -- --scenario
  top10-2023-fy-tcn-overlay-realdata` runs to completion.
- `backtest --features realdata -- --scenario
  top10-2024-fy-tcn-overlay-realdata` runs to completion.
- `backtest --features "realdata candle" -- --scenario
  top10-2023-fy-tcn-overlay-weights-realdata` runs to completion.
- `backtest --features "realdata candle" -- --scenario
  top10-2024-fy-tcn-overlay-weights-realdata` runs to completion.
- Without `--features realdata`, the four scenarios produce a
  clean error rather than a panic or silent fallback.

### M4 — Determinism gate

Two sequential `--release` runs of each new scenario at seed
`0xC0FFEE` produce byte-identical body-SHA-256. Existing
determinism test infrastructure in
[`crates/backtest/tests/determinism.rs`](../../crates/backtest/tests/determinism.rs)
extends.

_acceptance_: 4 new `#[cfg(feature = "realdata")] #[test]`
determinism tests pass; if `--features candle` is also enabled,
the 2 `-weights-realdata` tests also pass.

### M5 — Anchor lock

Tester runs each new scenario twice, confirms determinism, locks
the body SHA into `spec/anchors.toml` under version
`v2.6.0-realdata`. The trace row's `anchors` field is updated to
include the four new anchor names.

_acceptance_:

- 4 new `[[anchors]]` rows in `spec/anchors.toml`.
- `bash scripts/verify_anchors.sh → ANCHORS PASS (19 / 19)`.
- The trace row `REQ-BACKTEST-REALDATA-001` has all four anchor
  names listed in its `anchors` field.

### M-FINAL — Ship gate

- [ ] **T-T-1** — anchor-neutrality gate. All 15 pre-existing
  anchors (9 strategy synthetic + 2 v2.5 TCN passthrough + 2 v2.5
  TCN real-weights + 2 operator-success) stay byte-identical.
  Verified by `verify_anchors.sh` (which is mandatory for any PR
  that touches `crates/backtest/` per
  [architecture/11-regression-gate.md](../architecture/11-regression-gate.md)).
  _acceptance_: 15/15 stay PASS in addition to the 4/4 new
  anchors.

- [ ] **T-T-2** — rust-validate clean (fmt + clippy `-D warnings`
  + cargo-deny + docs).
  _acceptance_: per the `rust-validate` skill output.

- [ ] **T-T-3** — CI-portable gate. Default-features (no
  `realdata`) build passes all 15 existing anchors; presence of
  `data/binance/` is NOT required for the default build.
  _acceptance_: a build with the `data/binance/` directory
  temporarily moved (or missing on a clean CI runner) passes all
  default tests and skips the 4 new ones with a clean error.

- [ ] **T-T-4** — feature.md status flip `draft → in-progress
  → shipped` at architect / tester / operator gates as
  per [AGENT.md](../../AGENT.md).
  _acceptance_: each transition has a changelog row.

- [ ] **T_FINAL** — presenter pass + operator approval.
  _acceptance_: deck at
  `spec/backtest-real-binance-data/presentations/backtest-real-binance-data-<date>.md`
  carries `[x] Approved — ship` and the backlog entry moves
  Active → Recent.

## Notes

- **Out of scope.** Sharpe / drawdown / trade-count verdict on
  the four new `-realdata` scenarios. That is the
  `v25-tcn-overlay` alpha-gate re-spawn (see R8 + feature.md
  § Implementation).
- **Out of scope.** Sub-hourly bars, additional pairs beyond the
  current 10 USDT pairs on disk, alternate venues (Coinbase /
  Kraken parquet) — each is a follow-on feature if requested.
- **Out of scope.** The optional `T-D-7` Metal-vs-CPU exit gate
  from v25-tcn-overlay is unaffected — separate, low-priority
  follow-on, no dependency on this feature.

## Changelog

- 2026-05-18 (analyst): milestone skeleton authored. M0 → M-FINAL
  carrying the architect's lock surface (T-AR-1..3), the three
  operator-decide questions (T-OP-1..3), the four implementation
  milestones (M1-M4), the anchor lock (M5), and the ship gate
  (T-T-1..4, T_FINAL). Per-task `T-D-N` decomposition deferred
  to architect's T-AR-2.
