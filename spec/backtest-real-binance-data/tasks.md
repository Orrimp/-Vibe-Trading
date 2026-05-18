---
slug: backtest-real-binance-data
status: in-progress
owner: developer
updated: 2026-05-18
---

# Tasks — backtest-real-binance-data

> **Architect-decomposed T-D-N rows landed 2026-05-18** (see
> [`feature.md` § Design](feature.md#design) + [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)).
> Owner flipped `architect → developer`. The wave order below is
> the orchestrator's parallelism guide — see § Parallelism map
> at the bottom.

## Milestone skeleton

### M0 — Spike + architect lock

- [x] **T-AR-1** (2026-05-18) — Design section landed in
  [`feature.md` § Design](feature.md#design) (D1-D8 blocks). The
  cargo-feature route (`realdata`, default off) is chosen over CLI
  flag per R10. Trace row `REQ-BACKTEST-REALDATA-001` updated with
  arch paths.

- [x] **T-AR-2** (2026-05-18) — M1-M5 decomposed into 17 T-D-N
  rows + one T-T-1A tester row below, each with file:line
  targets, acceptance commands, and body-line specifications.
  Each row is independently spawnable per the parallelism map.

- [x] **T-AR-3** (2026-05-18) — ADR-0032
  ([`spec/architecture/adr/0032-backtest-realdata-path-and-revision-pin.md`](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md))
  authored. Registry updated. Cross-ref landed in
  [`spec/architecture/01-data-flow.md`](../architecture/01-data-flow.md#backtest-real-binance-data-path-v260-realdata).

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

### M1 — Parquet read path (5 T-D rows)

Wire the parquet read path + REVISION manifest write/verify path.
No new scenario rows yet (those land in M3); end-to-end tested via
a `#[cfg(test)]` integration test against a `tempdir` parquet
fixture.

- [x] **T-D-1** — Cargo feature `realdata` on `crates/backtest`.
  Owner: developer. Milestone: M1. Depends on: none. Blocks: T-D-2…T-D-15.
  _file:line_: `crates/backtest/Cargo.toml:19` (`realdata = ["dep:toml"]`).
  _test_: `cargo build -p backtest && cargo build -p backtest --features realdata`.
  _output_: `Finished \`dev\` profile` (both clean).
  _acceptance_:
  - `crates/backtest/Cargo.toml` carries `[features] realdata = ["dep:toml"]`
    (or equivalent — `sha2` is already a direct dep).
  - `cargo build -p backtest` (default features) succeeds with no `realdata`
    code compiled (`grep -c 'pub(crate) mod realdata' crates/backtest/src/lib.rs`
    returns 1 wrapped in `#[cfg(feature = "realdata")]`).
  - `cargo build -p backtest --features realdata` succeeds.

- [x] **T-D-2** — Aggregate-SHA helper in `crates/data/src/revision.rs`.
  Owner: developer. Milestone: M1. Depends on: none. Blocks: T-D-3, T-D-4.
  _file:line_: `crates/data/src/revision.rs:1` (new file, ~180 LoC).
  _test_: `cargo test -p data --lib revision`.
  _output_: `test result: ok. 5 passed; 0 failed`.
  _acceptance_:
  - New file `crates/data/src/revision.rs` with two public functions:
    `write_revision_manifest(root: &Path) -> Result<String, RevisionError>`
    (returns the aggregate SHA it wrote) and
    `read_and_verify_revision_manifest(root: &Path) -> Result<String, RevisionError>`
    (returns the aggregate SHA recomputed from the manifest).
  - `pub mod revision;` re-exported in `crates/data/src/lib.rs`.
  - Inline `#[cfg(test)]` unit tests for the aggregate-SHA algorithm with
    a 2-file fixture: documented input bytes, hand-computed expected
    SHA. `cargo test -p data revision::tests` PASS.

- [x] **T-D-3** — `--emit-revision-manifest` flag on `fetch_binance_klines`.
  Owner: developer. Milestone: M1. Depends on: T-D-2. Blocks: T-D-15.
  _file:line_: `crates/data/src/bin/fetch_binance_klines.rs:70` (`--emit-revision-manifest` flag).
  _test_: `cargo build -p data --bin fetch_binance_klines`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - `crates/data/src/bin/fetch_binance_klines.rs` adds `--emit-revision-manifest`
    bool flag (default false).
  - When set, after fetch completes, calls `data::revision::write_revision_manifest`
    on `--out`. The manifest body matches ADR-0032 § 2 schema exactly.
  - `cargo run -p data --bin fetch_binance_klines -- --symbols BTCUSDT --start 2023-01-01 --end 2023-01-31 --emit-revision-manifest --out /tmp/rd-t-d-3`
    produces `/tmp/rd-t-d-3/REVISION.toml` with one `[files]` entry and a
    non-zero `[revision].sha256`. (Run under network-allow operator gate.)

- [x] **T-D-4** — `realdata::RealDataBarSource` module.
  Owner: developer. Milestone: M1. Depends on: T-D-1, T-D-2.
  Blocks: T-D-5, T-D-6, T-D-7, T-D-9.
  _file:line_: `crates/backtest/src/realdata.rs:1` (new file, ~260 LoC).
  _test_: `cargo build -p backtest --features realdata`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - New file `crates/backtest/src/realdata.rs` per ADR-0032 § 1 surface.
    All items `pub(crate)`. Compiles only under `#[cfg(feature = "realdata")]`.
  - `crates/backtest/src/lib.rs` adds `#[cfg(feature = "realdata")] pub(crate) mod realdata;`.
  - `RealDataBarSource::load` invokes `data::ReplayFeed::merge_symbols`
    and post-processes every bar to set `local_recv_ts = close_ts`
    (determinism normalization vs `replay_feed.rs:196`).
  - `cargo build -p backtest --features realdata` clean.

- [x] **T-D-5** — Integration test fixture for `RealDataBarSource`.
  Owner: developer. Milestone: M1. Depends on: T-D-2, T-D-4. Blocks: T-D-15.
  _file:line_: `crates/backtest/tests/realdata_revision_verify.rs:1` (new file, 4 tests).
  _test_: `cargo test -p backtest --features realdata --test realdata_revision_verify`.
  _output_: `test result: ok. 4 passed; 0 failed`.
  _acceptance_:
  - New file `crates/backtest/tests/realdata_revision_verify.rs` (gated
    `#[cfg(feature = "realdata")]`).
  - Writes a `tempdir` parquet fixture for 2 symbols × 2 months
    (synthetic OHLCV; reuse `synthetic_bars_hourly` shape if convenient).
  - Asserts 4 paths: (a) happy load returns expected aggregate SHA,
    (b) tampering one byte of one parquet → `RevisionMismatch`,
    (c) deleting the manifest → `RevisionMissing`,
    (d) injecting a 0.6% gap (delete N bars from one symbol) →
    `MissingData` with `pct < 99.5`.
  - `cargo test -p backtest --features realdata --test realdata_revision_verify`
    → 4/4 PASS.

### M2 — Scenario dispatch wiring (4 T-D rows)

Threading the orthogonal `ScenarioDataSource` axis through
`Scenario` + dispatch without breaking the existing 11 multi-symbol
scenarios. The four new scenario rows land here but the scenario
match arm is feature-gated; M3 turns them on.

- [x] **T-D-6** — `ScenarioDataSource` enum + `Scenario` fields.
  Owner: developer. Milestone: M2. Depends on: T-D-4. Blocks: T-D-7, T-D-8.
  _file:line_: `crates/backtest/src/main.rs:72` (enum), `main.rs:133-141` (fields).
  _test_: `cargo build -p backtest && bash scripts/verify_anchors.sh`.
  _output_: `ANCHORS PASS (15 / 15)`.
  _acceptance_:
  - `crates/backtest/src/main.rs:67` adds `enum ScenarioDataSource { Synthetic, RealData }`
    with `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`.
  - `crates/backtest/src/main.rs:91` (`struct Scenario`) gains two
    fields: `data_source: ScenarioDataSource` and
    `expected_revision_sha: Option<String>`.
  - All 11 existing arms in `Scenario::from_name` get
    `data_source: ScenarioDataSource::Synthetic, expected_revision_sha: None,`.
    No other body bytes change.
  - `cargo build -p backtest` (default features) clean.
  - `bash scripts/verify_anchors.sh` → 15/15 PASS (no anchor moves).

- [x] **T-D-7** — RealData branch in TCN dispatch prelude.
  Owner: developer. Milestone: M2. Depends on: T-D-4, T-D-6. Blocks: T-D-9.
  _file:line_: `crates/backtest/src/main.rs:2686-2760` (dispatch match + RealData arm).
  _test_: `cargo build -p backtest --features realdata`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - `crates/backtest/src/main.rs:2421-2466` block modified per
    [`feature.md § Design D3`](feature.md#d3-orthogonal-scenariodatasource-axis-not-new-scenariostrategy-variants):
    branch on `scenario.data_source`. Synthetic arm UNCHANGED. RealData
    arm under `#[cfg(feature = "realdata")]` calls
    `RealDataBarSource::new(...).load(span)?`, asserts pinned revision.
  - With `--features realdata` but no `data/binance/REVISION.toml`,
    a RealData scenario exits with `RevisionMissing` (code 1, no
    panic). Verified by extension to T-D-5 fixture.

- [x] **T-D-8** — `scenario_to_feature` extension for the four new names.
  Owner: developer. Milestone: M2. Depends on: T-D-6. Blocks: T-D-9.
  _file:line_: `crates/backtest/src/main.rs` (`scenario_to_feature` extended with 4 new names).
  _test_: `cargo build -p backtest --features realdata`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - `crates/backtest/src/main.rs:2978-2982` match arm extended:
    `"top10-2023-fy-tcn-overlay-realdata" | "top10-2024-fy-tcn-overlay-realdata" |
    "top10-2023-fy-tcn-overlay-weights-realdata" | "top10-2024-fy-tcn-overlay-weights-realdata"
    => "backtest-real-binance-data"`.
  - `cargo test -p backtest --features realdata -- scenario_to_feature` PASS
    (extend the existing test if there is one, else add a small unit test).

### M3 — Wire the four new scenarios (4 T-D rows)

The four `-realdata` scenario rows materialise here, behind the
`realdata` feature. The two `-weights-realdata` rows additionally
require `--features candle` at run time (dispatch error if
missing).

- [x] **T-D-9** — Scenario rows for `top10-{2023,2024}-fy-tcn-overlay-realdata`.
  Owner: developer. Milestone: M3. Depends on: T-D-6, T-D-7, T-D-8. Blocks: T-D-13.
  _file:line_: `crates/backtest/src/main.rs` (4 new scenario arms under `#[cfg(feature = "realdata")]`).
  _test_: `cargo build -p backtest --features realdata`.
  _output_: `Finished \`dev\` profile` (clean).
  _acceptance_:
  - `crates/backtest/src/main.rs:355` (just before `other =>` bail)
    gets four new arms under `#[cfg(feature = "realdata")]`. Each
    carries: correct `start_year` (2023 / 2024), `bar_count = 8760` (2023)
    or `8784` (2024) — used as the R3 expected base, see
    [`feature.md § Design D5`](feature.md#d5-time-alignment--missing-data--exact-algorithm),
    `data_source: ScenarioDataSource::RealData`, `expected_revision_sha: None`
    (tester fills at M5), strategy `TcnOverlayMomentum` (passthrough)
    or `TcnOverlayMomentumWeights` (real weights) matching siblings.
  - `cargo run -p backtest --release --features realdata -- --scenario top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE`
    runs to completion against a populated `data/binance/` + `REVISION.toml`.
    Stdout shows `Data source  : real (Binance Vision via data/binance/, v2.6.0-realdata)`.
    Report written under `spec/backtest-real-binance-data/reports/backtest-*-top10-2023-fy-tcn-overlay-realdata.md`.

- [x] **T-D-10** — Report-renderer frontmatter line.
  Owner: developer. Milestone: M3. Depends on: T-D-7. Blocks: T-D-13.
  _file:line_: `crates/backtest/src/main.rs` (`write_tcn_overlay_report` frontmatter: `data_revision_sha: {sha}\n`).
  _test_: `bash scripts/verify_anchors.sh`.
  _output_: `ANCHORS PASS (15 / 15)`.
  _acceptance_:
  - `crates/backtest/src/main.rs:2008-2014` (frontmatter format!())
    gets one new line: `data_revision_sha: {sha_or_na}\n` between
    `wall_clock_s:` and `data_source:`.
  - For Synthetic scenarios, the value is the literal `n/a` (so
    existing TCN synthetic anchors stay byte-identical IF this line
    is excluded from the body hash — which it is, frontmatter excluded
    by hash_report.py). Verify: run `cargo run -p backtest -- --scenario top10-2023-fy-tcn-overlay --seed 0xC0FFEE`,
    `bash scripts/verify_anchors.sh` → 15/15 PASS.

- [x] **T-D-11** — Report-renderer body `## Data source` section.
  Owner: developer. Milestone: M3. Depends on: T-D-7. Blocks: T-D-13.
  _file:line_: `crates/backtest/src/main.rs` (`write_tcn_overlay_report`: `data_source_section` table, 7 rows).
  _test_: `bash scripts/verify_anchors.sh`.
  _output_: `ANCHORS PASS (15 / 15)`.
  _acceptance_:
  - `crates/backtest/src/main.rs:write_tcn_overlay_report` body emits
    a new `## Data source` block between `## Universe` and `## Notes`
    ONLY when `scenario.data_source == RealData`. Rows + column widths
    per [`feature.md § Design D4`](feature.md#d4-determinism-contract-surface--data_revision_sha-placement).
  - Summary table `Data source` row text matches the table in D4.
  - Notes-section `Data:` last line varies by data source per D4.
  - `cargo run -p backtest --release -- --scenario top10-2023-fy-tcn-overlay --seed 0xC0FFEE`
    → body SHA matches existing anchor `01d02584…` (no new section
    rendered).
  - `cargo run -p backtest --release --features realdata -- --scenario top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE`
    → body contains `## Data source` section with 7 rows.

- [x] **T-D-12** — Anchor-neutrality gate (K6 + K10).
  Owner: developer. Milestone: M3. Depends on: T-D-6, T-D-7, T-D-8, T-D-9, T-D-10, T-D-11.
  Blocks: T-D-13, T-T-1.
  _file:line_: `scripts/verify_anchors.sh` (existing gate).
  _test_: `bash scripts/verify_anchors.sh`.
  _output_: `ANCHORS PASS (15 / 15)`.
  _acceptance_:
  - After every commit in M1-M3, developer runs `bash scripts/verify_anchors.sh`
    and confirms `ANCHORS PASS (15 / 15)`. (Will become 19 / 19 post-T-D-16.)
  - The developer's M3 handoff includes the verify_anchors output log
    for the final commit of M3 (one verify_anchors run logged in
    `spec/backtest-real-binance-data/reports/m3-anchor-neutrality-<date>.md`
    or equivalent).

### M4 — Determinism gate (4 T-D rows)

Two sequential `--release` runs of each new scenario produce
byte-identical body-SHA-256. Existing test infrastructure in
[`crates/backtest/tests/determinism.rs`](../../crates/backtest/tests/determinism.rs)
extends — one new test per new scenario.

- [x] **T-D-13** — Determinism test for `top10-2023-fy-tcn-overlay-realdata`.
  Owner: developer. Milestone: M4. Depends on: T-D-9, T-D-10, T-D-11, T-D-12.
  Blocks: T-D-16.
  _file:line_: `crates/backtest/tests/determinism.rs` (`realdata_2023_fy_tcn_overlay_determinism`).
  _test_: `cargo test -p backtest --features realdata --test determinism realdata_2023_fy_tcn_overlay_determinism`.
  _output_: `test result: ok. 1 passed; 0 failed`.
  _acceptance_:
  - New `#[cfg(feature = "realdata")] #[tokio::test]` function
    `realdata_2023_fy_tcn_overlay_determinism` in
    `crates/backtest/tests/determinism.rs` that runs the scenario
    twice and asserts `body_sha256(run1) == body_sha256(run2)`.
  - Uses a `tempdir` parquet fixture for the 10-symbol universe
    (≥ 99.5% bar coverage, valid `REVISION.toml`). Reuse the
    fixture-builder helper introduced by T-D-5.
  - `cargo test -p backtest --features realdata --test determinism realdata_2023_fy_tcn_overlay_determinism --release`
    → 1/1 PASS.

- [x] **T-D-14** — Determinism test for `top10-2024-fy-tcn-overlay-realdata`.
  Owner: developer. Milestone: M4. Depends on: T-D-13. Blocks: T-D-16.
  _file:line_: `crates/backtest/tests/determinism.rs` (`realdata_2024_fy_tcn_overlay_determinism`).
  _test_: `cargo test -p backtest --features realdata --test determinism realdata_2024_fy_tcn_overlay_determinism`.
  _output_: `test result: ok. 1 passed; 0 failed`.
  _acceptance_: as T-D-13, scenario `top10-2024-fy-tcn-overlay-realdata`.

- [x] **T-D-15** — Determinism tests for the two `-weights-realdata`
  scenarios (`#[cfg(all(feature = "realdata", feature = "candle"))]`).
  Owner: developer. Milestone: M4. Depends on: T-D-3, T-D-13.
  Blocks: T-D-16.
  _file:line_: `crates/backtest/tests/determinism.rs` (`realdata_2023_fy_tcn_overlay_weights_determinism`, `realdata_2024_fy_tcn_overlay_weights_determinism`).
  _test_: `cargo test -p backtest --features "realdata candle" --test determinism realdata_2023_fy_tcn_overlay_weights_determinism`.
  _output_: `test result: ok. 1 passed; 0 failed`.
  _acceptance_:
  - Two new tests:
    `realdata_2023_fy_tcn_overlay_weights_determinism` and
    `realdata_2024_fy_tcn_overlay_weights_determinism`.
  - `cargo test -p backtest --features "realdata candle" --test determinism realdata_2023_fy_tcn_overlay_weights_determinism --release`
    → 1/1 PASS (and same for 2024).
  - Document the assumption that the two TCN checkpoints
    (`tcn-bs1`, `tcn-bs2`) are LFS-resolved on the developer's
    machine; the test skips with a clear `eprintln!` if the
    checkpoint file is absent (no panic).

### M5 — Anchor lock (3 T-D rows + tester-owned)

Tester runs each new scenario twice, confirms determinism, locks the
body SHA into `spec/anchors.toml` under version `v2.6.0-realdata`.
Developer fills the `expected_revision_sha` field on the four scenario
rows after the tester captures the actual aggregate SHA from a clean
fetch.

- [ ] **T-D-16** — Capture actual aggregate `data_revision_sha`
  from a clean fetch.
  Owner: developer. Milestone: M5. Depends on: T-D-3, T-D-13, T-D-14, T-D-15.
  Blocks: T-D-17, T-T-1.
  _acceptance_:
  - Operator runs the canonical fetch sequence (architect-locked):
    `cargo run -p data --bin fetch_binance_klines -- --symbols ADAUSDT,AVAXUSDT,BNBUSDT,BTCUSDT,DOGEUSDT,DOTUSDT,ETHUSDT,LINKUSDT,SOLUSDT,XRPUSDT --start 2023-01-01 --end 2024-12-31 --interval 1h --out data/binance --emit-revision-manifest --force`.
  - Developer records the resulting `[revision].sha256` value in
    `spec/backtest-real-binance-data/reports/m5-revision-pin-<date>.md`.

- [ ] **T-D-17** — Pin `expected_revision_sha` into the four scenario rows.
  Owner: developer. Milestone: M5. Depends on: T-D-16. Blocks: T-T-1.
  _acceptance_:
  - The four scenario rows added in T-D-9 get
    `expected_revision_sha: Some("<64 hex from T-D-16>".into())`.
  - `cargo run -p backtest --release --features realdata -- --scenario top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE`
    runs clean; tampering the manifest aborts with
    `data revision mismatch: scenario pinned <X> but on-disk computed <Y>`.

- [ ] **T-T-1A** — Tester lock the four new anchor SHAs in
  `spec/anchors.toml` under version `v2.6.0-realdata`. (Owner: tester.)
  Depends on: T-D-13, T-D-14, T-D-15, T-D-17. Blocks: T-T-1.
  _acceptance_:
  - Four `[[anchors]]` rows appended to `spec/anchors.toml` (after
    the existing `top10-2024-fy-tcn-overlay-weights` entry).
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS (19 / 19)`.
  - Trace row `REQ-BACKTEST-REALDATA-001` `anchors` field updated
    with all four new names.

### M-FINAL — Ship gate

- [ ] **T-T-1** — anchor-neutrality gate. All 15 pre-existing
  anchors (9 strategy synthetic + 2 v2.5 TCN passthrough + 2 v2.5
  TCN real-weights + 2 operator-success) stay byte-identical.
  Verified by `verify_anchors.sh` (which is mandatory for any PR
  that touches `crates/backtest/` per
  [architecture/11-regression-gate.md](../architecture/11-regression-gate.md)).
  Depends on: T-D-12 (developer-side mid-feature neutrality gate),
  T-T-1A (the four new anchor rows).
  _acceptance_: `bash scripts/verify_anchors.sh` → `ANCHORS PASS (19 / 19)`
  on a single invocation. 15/15 of the pre-existing rows match their
  pre-feature SHAs exactly.

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

## Parallelism map for the orchestrator

The 17 developer tasks (T-D-1 … T-D-17) plus one tester task
(T-T-1A) plus four ship-gate tasks (T-T-1..4 + T_FINAL) fall into
six waves. Each wave is a parallel-safe spawn batch; sequential
edges between waves are real dependencies.

### Wave 1 — Foundation (no upstream)

Three rows; all independent; orchestrator may spawn all three in
parallel.

- **T-D-1** — `realdata` cargo feature on `crates/backtest`.
  No deps. Five-line edit.
- **T-D-2** — Aggregate-SHA helper in `crates/data/src/revision.rs`.
  No deps. ~80 LoC + unit tests.
- (Reserved for ad-hoc bug fixes from M0 spike, if needed.)

### Wave 2 — Bar source + revision-manifest writer (deps Wave 1)

Both rows depend only on Wave 1; orchestrator may parallel-spawn.

- **T-D-3** — `--emit-revision-manifest` flag on `fetch_binance_klines`.
  Deps: T-D-2.
- **T-D-4** — `realdata::RealDataBarSource` module.
  Deps: T-D-1, T-D-2.

### Wave 3 — Fixture integration test (deps Wave 2)

Single row, but it gates the dispatch wiring in Wave 4 — must
complete before Wave 4 starts.

- **T-D-5** — `realdata_revision_verify.rs` integration test.
  Deps: T-D-2, T-D-4.

### Wave 4 — Scenario dispatch surface (deps Wave 3)

Three rows. T-D-6 is the prerequisite for T-D-7 and T-D-8; the
latter two are independent of each other after T-D-6 lands.
Spawn order: T-D-6 → (T-D-7 ‖ T-D-8).

- **T-D-6** — `ScenarioDataSource` enum + `Scenario` fields.
  Deps: T-D-4. Blocks T-D-7, T-D-8.
- **T-D-7** — TCN dispatch prelude branch. Deps: T-D-4, T-D-6.
- **T-D-8** — `scenario_to_feature` extension. Deps: T-D-6.

### Wave 5 — Scenarios + renderer (deps Wave 4)

Four rows. T-D-9 lands the scenario rows; T-D-10 and T-D-11 modify
the same `write_tcn_overlay_report` function but at different line
ranges (frontmatter vs body section) and can be done in parallel
by a single developer agent or by two if the orchestrator splits
the work; the merge is mechanical. T-D-12 is the anchor-neutrality
gate — runs AFTER T-D-9/10/11 land.

- **T-D-9** — Four new scenario rows in `Scenario::from_name`.
  Deps: T-D-6, T-D-7, T-D-8.
- **T-D-10** — Frontmatter `data_revision_sha:` line. Deps: T-D-7.
- **T-D-11** — Body `## Data source` section. Deps: T-D-7.
- **T-D-12** — Anchor-neutrality gate (mid-feature run).
  Deps: T-D-6, T-D-7, T-D-8, T-D-9, T-D-10, T-D-11.

### Wave 6 — Determinism + lock (deps Wave 5)

Five rows. T-D-13/14/15 are independent of each other (different
scenarios) and may be parallel-spawned after Wave 5. T-D-16 +
T-D-17 are sequential and gated on a clean fetch (operator-run).
T-T-1A is the tester-owned anchor lock; deps T-D-17.

- **T-D-13** — Determinism test, 2023 fy.
  Deps: T-D-9, T-D-10, T-D-11, T-D-12.
- **T-D-14** — Determinism test, 2024 fy. Deps: T-D-13.
  (Strict ordering, not parallel — share the fixture-builder
  helper from T-D-5 to keep test code DRY.)
- **T-D-15** — Determinism tests, 2023 + 2024 weights variants.
  Deps: T-D-3, T-D-13.
- **T-D-16** — Operator clean-fetch + aggregate-SHA capture.
  Deps: T-D-3, T-D-13, T-D-14, T-D-15.
- **T-D-17** — Pin `expected_revision_sha` into scenario rows.
  Deps: T-D-16.
- **T-T-1A** — Tester locks 4 new anchor SHAs in `spec/anchors.toml`.
  Deps: T-D-13, T-D-14, T-D-15, T-D-17.

### Wave 7 — Ship gate (deps Wave 6)

- **T-T-1** — Full 19/19 anchor-neutrality. Deps: T-T-1A, T-D-12.
- **T-T-2** — `rust-validate` clean.
- **T-T-3** — CI-portable gate (default-features build).
- **T-T-4** — feature.md status flips.
- **T_FINAL** — Presenter + operator approval.

### Critical path

T-D-1/T-D-2 → T-D-4 → T-D-5 → T-D-6 → T-D-7 → T-D-9 → T-D-12 →
T-D-13 → T-D-14 → T-D-15 → T-D-16 → T-D-17 → T-T-1A → T-T-1 →
T_FINAL. Wider waves (T-D-10 / T-D-11; T-D-8) collapse into the
critical-path waves at their Wave boundary — they do not extend
critical path length.

## Changelog

- 2026-05-18 (analyst): milestone skeleton authored. M0 → M-FINAL
  carrying the architect's lock surface (T-AR-1..3), the three
  operator-decide questions (T-OP-1..3), the four implementation
  milestones (M1-M4), the anchor lock (M5), and the ship gate
  (T-T-1..4, T_FINAL). Per-task `T-D-N` decomposition deferred
  to architect's T-AR-2.
- 2026-05-18 (architect): T-AR-1/2/3 ticked. T-D-N decomposition
  landed for M1-M5 (17 developer rows + T-T-1A tester row).
  Parallelism map appended (6 waves + ship gate). Owner flipped
  `architect → developer`. ADR-0032 authored
  (`spec/architecture/adr/0032-backtest-realdata-path-and-revision-pin.md`);
  cross-reference added to
  [`01-data-flow.md`](../architecture/01-data-flow.md#backtest-real-binance-data-path-v260-realdata).
  Trace row `REQ-BACKTEST-REALDATA-001` arch column filled with
  three references (ADR-0032, the new 01-data-flow subsection,
  the feature.md § Design section).
- 2026-05-18 (developer): M1-M4 complete (T-D-1..T-D-15 ticked).
  All 15 pre-existing anchors byte-identical (ANCHORS PASS 15/15).
  New files: `crates/data/src/revision.rs`,
  `crates/backtest/src/realdata.rs`,
  `crates/backtest/tests/realdata_revision_verify.rs`.
  Modified: `crates/backtest/src/main.rs` (ScenarioDataSource,
  4 new scenario arms, dispatch, report writer).
  `crates/data/src/bin/fetch_binance_klines.rs` (--emit-revision-manifest).
  Determinism tests T-D-13/14 pass; T-D-15 skips if LFS absent.
  M5 (T-D-16/17) and ship-gate (T-T-1..4, T_FINAL) left for tester.
