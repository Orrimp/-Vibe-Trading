---
slug: lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge
status: draft
audience: human-operator
owner: presenter
updated: 2026-05-28
mode: release
feature_version: 0.1.2
predecessor_deck: ../../lab-yahoo-realdata/presentations/lab-yahoo-realdata-v0.1.1-2026-05-27.md
test_report: ../reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge.md
commit: 9638ff8
adr: ../../architecture/adr/0040-yahoo-realdata-path.md
---

# Lab Yahoo realdata v0.1.2 — sprint review (2026-05-28)

## TL;DR

**The Lab now holds two locked Yahoo anchors (BTC + ETH) and tells you so at a
glance.** Anchor count **69 → 70** with `eth-yahoo-2024-1d-sma-cross` locked at
body-SHA `e59a5f87…`, byte-stable across **5 independent re-runs** (3 developer
+ 2 tester). A new **Lab toolbar badge** reports `Yahoo cache: N tickers · last
fetch YYYY-MM-DD` for every Lab activation. H1 PASS at **0.84%** (via K1
Yahoo-to-Yahoo fallback, < 30% threshold). Verdict from the tester:
**SOFT-PASS** — the qualifier reflects a transitional BTC body-SHA drift caused
by an external `REVISION.toml` aggregate change (operator's ETH fetch), not by
any code change; `verify_anchors.sh` still resolves **70/70** correctly.

## The one-sentence story (v0.1.1 → v0.1.2)

v0.1.1 locked 1 Yahoo ticker (BTC) and gave the operator a per-pair Fresh/Stale
pill. v0.1.2 locks **2** tickers (BTC + ETH) **and** adds the multi-ticker
visibility surface — the operator can now look at the Lab and immediately see
how many tickers are cached + when the cache was last refreshed. The
fetch → see-it-in-the-badge → anchored-backtest loop closes.

## What shipped — Backtest lane (M-DEV)

- **`run_yahoo_sma` extended** with `--ticker <TICKER>` Clap arg (default
  `BTC-USD`, ~15 LoC delta on top of the v0.1.1 binary). Validated against
  `pub const ALLOWED_YAHOO_TICKERS: &[&str]` (10 rows). Unknown ticker → exit 2
  with an actionable error listing the allowed values.
- **`pub fn scenario_name(ticker)` pure helper** implementing the
  `{lc-ticker-no-USD}-yahoo-2024-1d-sma-cross` rule (D-V0.1.2-3) — so the same
  binary will mechanically yield 8 more anchor IDs across BNB → LINK as
  operators fetch them.
- **6 integration tests** in
  [`crates/backtest/tests/run_yahoo_sma_ticker_flag.rs`](../../../crates/backtest/tests/run_yahoo_sma_ticker_flag.rs):
  `pinned_table`, `btc_sha`, `eth_sha`, `unknown_ticker`, `scenario_name_btc`,
  `scenario_name_eth`. All green; uses `std::process::Command` so no new
  workspace dependency.
- **Anchor row 70 appended** to [`spec/anchors.toml`](../../anchors.toml):
  ```toml
  [[anchors]]
  scenario = "eth-yahoo-2024-1d-sma-cross"
  version  = "lab-yahoo-realdata-v0.1.2"
  sha256   = "e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a"
  ```
  All 69 prior rows byte-identical (append-only contract per ADR-0038 § D6).
- **H1 dev-note** at
  [`dev-notes/yahoo-vs-binance-divergence-eth-2026-05-27.md`](../dev-notes/yahoo-vs-binance-divergence-eth-2026-05-27.md)
  records the K1 Yahoo-to-Yahoo fallback discharge (Yahoo ETH +0.35% vs Yahoo
  BTC +1.20% on H1 2024 → Δ = 0.84% < 30%).
- **3 canonical ETH reports** under
  [`spec/lab-yahoo-realdata-v0.1.2-…/reports/`](../reports/) — the first
  (`backtest-20260527-215627-…`) is the anchored file; runs 2 + 3 are the
  on-disk determinism witnesses.

## What shipped — UI lane (M-DEV-UI, parallel; zero file overlap)

- **NEW widget** [`crates/ui/src/widgets/cache_state_summary_badge.rs`](../../../crates/ui/src/widgets/cache_state_summary_badge.rs) — sibling
  to the existing per-pair pill, reusing the same Lumen tokens (`PANEL_RAISED`,
  `BORDER_1`, `R3`, `MICRO` text, `XXS`/`S` spacing). Zero new design tokens.
- **NEW string constant** `LAB_CACHE_STATE_SUMMARY_PREFIX = "Yahoo cache: "`
  (operator Q3 override) + an internal helper `fmt_lab_cache_state_summary` to
  keep `widgets/` literal-free under `tests/consistency.rs`.
- **`CacheSummary` struct + `probe_summary()`** added to
  [`crates/ui/src/lab/cache_state.rs`](../../../crates/ui/src/lab/cache_state.rs)
  plus `pub const ALL_YAHOO_TICKERS: &[&str]` (10-row mirror). Bounded at ~30
  filesystem stats per probe.
- **`LabState::cache_summary: Option<CacheSummary>`** with invalidation +
  immediate re-populate on **two events only** (per architect D-V0.1.2-1):
  `LabSelectDataSource` and `LabRunCompleted`. No per-frame stat; no
  background polling.
- **NEW Lab toolbar Row** wired as the **first child** of the Lab body
  `Column` in [`crates/ui/src/screens/lab.rs`](../../../crates/ui/src/screens/lab.rs)
  (operator Q2 override of analyst's source-toggle-row default). Right-aligned
  via `Space::new().width(Fill)` leading spacer. Renders regardless of
  `data_source` — independent surface from the per-pair pill.
- **4 new gallery cells**: `cache_state_summary_badge__{empty,one_ticker,two_tickers,ten_tickers}`.
  `GALLERY_LOGICAL_HEIGHT` bumped 18_040 → 19_080.
- **4 new panel snapshots** locking the textual summary + Lumen tokens against
  drift (stable 2024-12-31 UTC fixture mtime).
- **14 new lib tests** → ui `--lib` total **397 → 411**.

## What you can do now

| Action | Command |
|---|---|
| Re-verify all 70 anchors locally | `bash scripts/verify_anchors.sh` |
| Run the locked ETH anchor | `cargo run --release -p backtest --features yahoo --bin run_yahoo_sma -- --ticker ETH-USD` |
| Run the locked BTC anchor (default) | `cargo run --release -p backtest --features yahoo --bin run_yahoo_sma --` |
| See the new Lab toolbar badge | `cargo run --release --bin cockpit --features fixtures` → Lab tab |
| Inspect badge gallery cells | `cargo run --release --bin gallery` → scroll to `cache_state_summary_badge__*` |
| Cross-crate pinned-table drift gate | `cargo test -p backtest --features yahoo --test run_yahoo_sma_ticker_flag` |

## Live demo (presenter re-ran the binary)

```
$ cargo run --release -p backtest --features yahoo --bin run_yahoo_sma -- \
      --ticker ETH-USD --reports-dir /tmp/eth-presenter-demo
    Finished `release` profile [optimized] target(s) in 0.75s
     Running `target/release/run_yahoo_sma --ticker ETH-USD --reports-dir /tmp/eth-presenter-demo`
Scenario     : eth-yahoo-2024-1d-sma-cross
Ticker       : ETH-USD
Cache root   : data/yahoo
Period       : 2024-01-01 → 2024-12-31 (1d cadence)
Seed         : 0xC0FFEE
Bars loaded  : 366
Revision SHA : e018f876c36ab82aae2b6509be3ceb1cab4124c2c5eea4a08c1b8aa3000e7734
Bars replayed: 366
Trades       : 7
Final equity : $102760.75 USDT
Elapsed      : 0.0s
Report       : /tmp/eth-presenter-demo/backtest-20260528-061631-eth-yahoo-2024-1d-sma-cross.md

$ python3 scripts/hash_report.py /tmp/eth-presenter-demo/backtest-20260528-061631-eth-yahoo-2024-1d-sma-cross.md
e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a  …
```

This is the **6th independent re-run** of `--ticker ETH-USD` (3 dev + 2 tester
+ 1 presenter); body SHA `e59a5f87…` identical to anchor row 70.

```
$ bash scripts/verify_anchors.sh | tail -3
PASS  btc-yahoo-2024-1d-sma-cross   8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867
PASS  eth-yahoo-2024-1d-sma-cross   e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a
ANCHORS PASS  (70 / 70)
```

## Screenshots

_n/a — gallery cells + panel snapshots serve as the visual contract at v0.1.2
(`crates/ui/tests/panel_snapshots/cache_state_summary_badge__*.snap`, 4 cells,
90/90 green). The operator can launch `cargo run --release --bin gallery` and
scroll to the four new cells, or `cargo run --release --bin cockpit --features
fixtures` and watch the badge in the Lab toolbar row. The cold-start "no cache"
state is acceptable per tester § 9 — first toggle/run populates it._

## Verification

| V-id | Requirement | Status | Evidence |
|---|---|---|---|
| V1 | Anchor row 70 appended; 69 prior rows byte-identical (R1, R5.1) | VERIFIED | `verify_anchors.sh → ANCHORS PASS (70 / 70)`, [test report § 5](../reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge.md#5-backtest-results) |
| V2 | H1 ETH divergence < 30% (R2) | VERIFIED (K1 fallback) | `0.84% < 30%` — Yahoo ETH H1 vs Yahoo BTC H1, [dev-note](../dev-notes/yahoo-vs-binance-divergence-eth-2026-05-27.md) |
| V3 | H2 body-SHA determinism ≥ 2 re-runs (R1.3) | VERIFIED ×5 | All 5 runs SHA `e59a5f87…` — test report § 5 H2 table |
| V4 | `run_yahoo_sma --ticker` flag works; unknown → exit 2 (R4) | VERIFIED | 6/6 integration tests + 2/2 binary unit tests PASS |
| V5 | Summary badge widget shipped (R3.1-R3.4) | VERIFIED | `cargo test -p ui --lib` 411/411; 4 panel snapshots accepted |
| V6 | Badge wired in Lab toolbar Row, first child (R3.6, Q2 override) | VERIFIED | [`screens/lab.rs`](../../../crates/ui/src/screens/lab.rs) + `chart_canvas_height_grows_with_body_height` still passes |
| V7 | Exactly 1 new operator-visible string (R-NR.2) | VERIFIED | `LAB_CACHE_STATE_SUMMARY_PREFIX` + internal helper; `consistency.rs` 2/2 PASS |
| V8 | `cargo fmt --check` clean + 0 new clippy errors (R-NR.3) | VERIFIED | fmt clean; 9 pre-existing ui errors carried, **0 new** |
| V9 | spec-lint no NEW categories vs 73/3 baseline (R-NR.4) | VERIFIED | `spec-lint: FAIL (73 violations in 3 categories)` — baseline match |
| V10 | Cross-feature canary (`cockpit_training_pressed_wiring`) green | VERIFIED | 5/5 PASS — toast-queue v0.2.0 cleanup unaffected |
| V11 | Lab default UX byte-identical when `Synthetic` (R-NR.5) | VERIFIED | gallery + panel snapshots show no regression in pre-existing cells |
| V12 | Idle-CPU / probe budget honored (R-NR.7) | VERIFIED | cached-summary cadence ⇒ probe runs only on `LabSelectDataSource` + `LabRunCompleted`; ≤ 30 stats per fire |

## Numbers that matter

- **Anchors:** 70 / 70 PASS (`verify_anchors.sh`); previous 69 untouched.
- **ETH-USD financial result (anchor row 70):** $102,760.76 final equity,
  +2.76%, 7 trades, 366 bars, REVISION.toml SHA `e018f876…`, body SHA
  `e59a5f87…`.
- **BTC-USD financial result (still anchored at row 69):** $104,560.07–08
  final equity, +4.56%, 7 trades — **identical** to v0.1.1 across all runs;
  only the wrapped body-table `rev=` line differs.
- **H2 determinism:** 5/5 ETH runs SHA-identical (3 dev + 2 tester); 6/6
  including presenter live demo above.
- **H1:** Δ = 0.84% (K1 Yahoo-to-Yahoo fallback), well inside the 30% gate.
  Extrapolated Binance-hourly Δ ≈ 8–15% based on v0.1.1 BTC precedent.
- **Tests:**
  - `backtest` ticker-flag integration: **6 / 6**
  - `backtest` binary unit: **2 / 2** (`scenario_name_btc`/`_eth`)
  - `ui` lib: **411 / 411** (+14 new vs v0.1.1's 397)
  - `ui` panel snapshots: **90 / 90** (+4 new)
  - `ui` consistency: **2 / 2**
  - `ui` cockpit cross-feature canary: **5 / 5**
- **Clippy:** 9 pre-existing ui errors (within declared budget); **0 new**.
- **fmt:** clean.
- **spec-lint:** 73 violations in 3 categories — **matches the pre-existing
  73/3 baseline exactly** (no NEW categories introduced).

## The honest BTC SHA drift footnote

The tester independently re-ran the BTC default invocation
(`cargo run … run_yahoo_sma --`) and got body SHA `d2a709ef…`, **not** the
v0.1.1 anchor `8045623b…`. This was flagged by the developer and confirmed by
the tester. The disposition is **transitional, not a regression**:

1. When the operator ran `fetch_yahoo_klines --tickers ETH-USD` on 2026-05-27,
   `data/yahoo/REVISION.toml` aggregate SHA changed `7b33166e… → e018f876…`
   (ETH-USD entries appended).
2. The `run_yahoo_sma` report includes `Data source: yahoo-cache:BTC-USD/1d/2024
   rev=e018f876c36a` in the body — so the body SHA tracks REVISION.toml.
3. The `--ticker` code change did **not** touch BTC computation. BTC financial
   numbers are byte-identical across old + new: $104,560.07–08, 7 trades,
   +4.56%.
4. `verify_anchors.sh` resolves anchors against the **on-disk anchored
   report** (newest-lex-sort tie-break), which is still the v0.1.1
   `backtest-20260527-143420-…` file with SHA `8045623b…`. No new BTC report
   was committed after the diagnosis, so **70 / 70 PASS** is preserved.

**This pattern will recur every time a new ticker is fetched** (BNB at v0.1.3
will drift BTC + ETH body SHAs again, etc.). The architect should consider
moving the `rev=` line from the report body into front-matter so REVISION.toml
churn stops mutating body SHAs on otherwise-unrelated tickers. Flagged below in
"Deferred to v0.1.3".

## Deferred to v0.1.3 (not in this approval)

1. **Architect-call:** move REVISION.toml `rev=` from report body to
   front-matter (decouple body-SHA stability from REVISION.toml aggregate
   churn). Would convert all future ticker-fetch drifts into front-matter-only
   changes that don't affect anchored body SHAs.
2. **Register `eth-2024-h1-sma-cross` Binance hourly scenario** in
   `crates/backtest/src/main.rs` — discharges H1 directly (no K1 fallback) and
   gives the operator a per-ticker H1 cross-source pair.
3. **8 remaining unanchored crypto-mirror tickers:** BNB-USD, SOL-USD,
   XRP-USD, ADA-USD, DOGE-USD, AVAX-USD, DOT-USD, LINK-USD — all already in
   `ALLOWED_YAHOO_TICKERS`; locking requires only `fetch_yahoo_klines` + a
   `run_yahoo_sma --ticker $T` run per ticker.
4. **Multi-strategy on Yahoo** (MACD / RSI / BBands) — Q2 of v0.1.1's open
   list; still deferred.
5. **Click-to-drill from summary badge** — per-ticker fan-out; display-only
   at v0.1.2.
6. **Per-ticker last-fetch timestamps** in the summary — only a single newest
   mtime today.

## Open decisions

_no decisions pending — ready to ship (or reject). The BTC SHA drift is
transitional and disposed; everything else is anchored, tested, and green._

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_(empty until operator fills)_

## References

- Predecessor deck: [v0.1.1 sprint review (2026-05-27)](../../lab-yahoo-realdata/presentations/lab-yahoo-realdata-v0.1.1-2026-05-27.md)
- Tester report: [test-final-2026-05-28](../reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge.md)
- Feature brief: [feature.md](../feature.md)
- Task list: [tasks.md](../tasks.md)
- H1 dev-note: [yahoo-vs-binance-divergence-eth-2026-05-27.md](../dev-notes/yahoo-vs-binance-divergence-eth-2026-05-27.md)
- ADR-0040 (extended Changelog): [`spec/architecture/adr/0040-yahoo-realdata-path.md`](../../architecture/adr/0040-yahoo-realdata-path.md)

## Changelog

- 2026-05-28 (presenter): initial draft for M-FINAL SOFT-PASS at commit
  `9638ff8` (BTC SHA drift disposed transitional; 70/70 anchors PASS;
  H1 PASS at 0.84% via K1 fallback; H2 ×5 PASS; 411/411 ui lib + 90/90 panel
  snapshots; awaiting operator approval).
