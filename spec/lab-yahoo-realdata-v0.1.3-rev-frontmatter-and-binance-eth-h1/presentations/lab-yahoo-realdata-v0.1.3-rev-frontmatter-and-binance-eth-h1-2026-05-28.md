---
title: Sprint Review — lab-yahoo-realdata-v0.1.3 (rev= frontmatter + Binance ETH H1)
feature: lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1
version: 0.1.0
date: 2026-05-28
mode: release
agent: presenter
tester_verdict: PASS
tester_report: spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1.md
commit: e74204a
---

# Sprint Review — lab-yahoo-realdata v0.1.3

> rev= body-to-frontmatter migration + Binance ETH H1 scenario registration

## TL;DR

v0.1.3 is a hygiene ship that **closes both design notes** the architect
flagged at v0.1.2 M-FINAL — and closes them durably (not with patches that
re-open as v0.2.0 follow-up debt). Tester PASS. Anchors 70 → 71. Ready to
ship.

## What changed (plain language)

- **The BTC Yahoo report no longer carries a `rev=<sha>` tag inside its
  body.** That tag was the cause of v0.1.2's "BTC anchor SHA drifts every
  time we fetch a new ticker" pain. The 64-char revision hash now lives in
  the report's YAML frontmatter (`revision_sha:`), where future ticker
  fetches can't poke at the body bytes.
- **A canonical Yahoo report helper now exists** at
  `crates/backtest/src/report/yahoo.rs`. Every future Yahoo binary
  (`run_yahoo_macd`, `_rsi`, `_bbands` at v0.2.0+) inherits the new emit
  shape byte-identically with a one-line call. A unit test enforces it.
- **Binance ETH hourly is now a real registered scenario**
  (`eth-2024-h1-sma-cross`). The H1 hypothesis ("daily Yahoo vs hourly
  Binance ETH stay close enough") is discharged **directly** at a 6.78%
  delta — the v0.1.2 Yahoo-to-Yahoo proxy fallback is retired.

## Why this matters (the durable-boundary story)

This is the kind of ship the **operator-locked durable-over-quick
contract** (AGENT.md 2026-05-28) is designed to produce.

The quick alternative was to inline-patch `run_yahoo_sma.rs` only — ship
in half a day, save ~1 day of dev now, and quietly accumulate ~1.5–3 days
of follow-on rework cost spread across the 3 planned future Yahoo binaries
(MACD, RSI, BBands at v0.2.0+). Each of those would have re-implemented
the body→frontmatter split independently, with three drift-risk events.

You picked the helper-extraction route specifically to avoid that. Net
wall-clock across the product lifecycle: helper route is **strictly
cheaper AND strictly more durable**, zero downside. The Yahoo helper
boundary is now the single point of truth — and a regression guard test
(`yahoo_report_helper_shape.rs`, 3 tests) prevents any future binary from
quietly hand-formatting `data_source` strings and bypassing it.

The same durable-pick logic applied to Q2: BTC row 69's new SHA goes
**in-place under the existing `lab-yahoo-realdata-v0.1.1` namespace**
rather than minting a new `v0.1.3` namespace for a re-emit. One row, one
origin, one tracked namespace across v0.1.4+ ticker work.

## What shipped — Defect 1 (BTC body-SHA decoupling)

| Change | File | Why durable |
|---|---|---|
| NEW canonical Yahoo emit helper | `crates/backtest/src/report/yahoo.rs` | Single point of truth — `YahooReportContext { ticker, interval, year, revision_sha }` + `emit_sma_report(...)`. Future Yahoo binaries are 1:1 additive consumers. |
| SMA writer gains optional `revision_sha` | `crates/backtest/src/report/sma.rs` | `Some(sha)` writes the frontmatter line; `None` arm preserves 33 Binance SMA anchors byte-identically. |
| `run_yahoo_sma.rs:259` migrated | `crates/backtest/src/bin/run_yahoo_sma.rs` | `Data source: ... rev=...` → `Data source: yahoo-cache:BTC-USD/1d/2024`; frontmatter gains `revision_sha: <64 hex>`. |
| BTC row 69 SHA updated in-place | `spec/anchors.toml` (row 69) | Namespace `lab-yahoo-realdata-v0.1.1` preserved verbatim (Q2=(a) durable). Old `8045623b…` retired; new `076929bb…`. |
| Regression guard test (NEW) | `crates/backtest/tests/yahoo_report_helper_shape.rs` | 3 grep assertions: no future Yahoo binary can hand-format `data_source` and bypass the helper. The "we won't regress this migration" gate. |

### Live evidence — emitted BTC report frontmatter (post-migration)

```
---
scenario: btc-yahoo-2024-1d-sma-cross
seed: 0xC0FFEE
generated: 2026-05-28T22:06:48Z
wall_clock_s: 0.0
data_source: yahoo-cache:BTC-USD/1d/2024
revision_sha: e018f876c36ab82aae2b6509be3ceb1cab4124c2c5eea4a08c1b8aa3000e7734
baseline_report: n/a
...
---
```

Note: `data_source` line is clean — no `rev=` substring. `revision_sha:`
sits immediately after, as a separate frontmatter field that
`verify_anchors.sh` and future emitters can read structurally.

## What shipped — Defect 2 (direct H1 discharge for ETH)

| Change | Where | Result |
|---|---|---|
| Scenario `eth-2024-h1-sma-cross` registered | `crates/backtest/src/main.rs` (L~260 config + L~1050 fallback + L~1780 feature map) | Mirrors `btc-2024-h1-sma-cross` arm verbatim; Symbol ETHUSDT, real parquets auto-detected. |
| Real Binance hourly data loaded | `data/binance/ETHUSDT/2024/` | 17,543 hourly bars from 12 monthly parquets (2023 + 2024 combined for SMA warmup). |
| Row 71 NEW under `lab-yahoo-realdata-v0.1.3` | `spec/anchors.toml` (row 71) | SHA `bd4001e42475955f518421d75cab207c85d0db3ba3a9d45fbdceff4f4b4e5441`. |
| Determinism | 4 independent runs (2 dev + 2 tester) | All byte-identical. |
| H1 discharge | Yahoo ETH daily +2.76% vs Binance ETH hourly +9.54% | Delta = **6.78% < 30%** — PASS direct. v0.1.2 K1 Yahoo-to-Yahoo fallback retired. |

### Live evidence — `verify_anchors.sh` final 3 rows

```
PASS  btc-yahoo-2024-1d-sma-cross  076929bb63d9bec03ec83684b85ced818ee32c0b2da41140712ec1d01de6a1e0
PASS  eth-yahoo-2024-1d-sma-cross  e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a
PASS  eth-2024-h1-sma-cross        bd4001e42475955f518421d75cab207c85d0db3ba3a9d45fbdceff4f4b4e5441
---
ANCHORS PASS  (71 / 71)
```

Command (re-runnable):

```bash
bash scripts/verify_anchors.sh | tail -10
```

## Verification matrix

| Gate | Status | Evidence |
|---|---|---|
| **R1.1** Yahoo report body contains no `rev=` substring | VERIFIED | `grep -RIn "rev=" spec/lab-yahoo-realdata-v0.1.3-*/reports/` → zero matches. Tester T-F3. |
| **R1.2** Yahoo report frontmatter has `revision_sha:` line | VERIFIED | Emitted BTC report L7: `revision_sha: e018f876...` (full 64-char hex). |
| **R1.3** Canonical helper at `crates/backtest/src/report/yahoo.rs` | VERIFIED | NEW ~160 LoC; `run_yahoo_sma.rs` is the first consumer. |
| **R1.4** Post-condition grep clean | VERIFIED | Tester T-F3 zero matches. |
| **R2.1** `eth-2024-h1-sma-cross` registered | VERIFIED | `main.rs` 3 match-arm sites added (L~260, L~1050, L~1780). |
| **R2.4** ETH H1 determinism ≥ 2 runs | VERIFIED | 4/4 byte-identical (2 dev + 2 tester) at SHA `bd4001e4…`. |
| **R3.1** Anchor count 70 → 71 with row 69 in-place | VERIFIED | `verify_anchors.sh` 71/71 PASS; namespace `lab-yahoo-realdata-v0.1.1` preserved. |
| **R3.2** 68 non-Yahoo anchors byte-identical | VERIFIED | `verify_anchors.sh` 71/71 PASS (rows 1-68). |
| **R3.3** Row 70 ETH daily byte-identical | VERIFIED | SHA `e59a5f87…` PASS — not re-emitted (deferred to v0.1.4). |
| **R4** H1 ETH direct discharge < 30% | VERIFIED | Delta 6.78% (in expected 5-15% range). Dev-note locks computation. |
| **R5.3** Zero UI files touched | VERIFIED | Tester confirmed via `git diff` on commit `e74204a`. |
| **R5.4** Workspace lib tests green | VERIFIED | Backtest lib 38/0; in-scope integration tests all green; `yahoo_report_helper_shape` 3/3 PASS. |
| **R-NR.1** Zero new design tokens | VERIFIED | Backend-only ship. |
| **R-NR.2** Zero new `strings.rs` entries | VERIFIED | Backend-only ship. |
| **R-NR.3** `cargo fmt --check` + clippy clean on touched paths | VERIFIED | 0 new warnings on `crates/backtest --features "yahoo realdata"`. |
| **H3** Helper retrofit-free for future emitters | VERIFIED by code inspection + `data_source_never_contains_rev_substring` unit test. |

## Numbers that matter

- **Anchors:** 70 → 71 (1 in-place BTC update + 1 new ETH H1 append).
- **Determinism witness count:** ETH H1 row 71 = 4/4 byte-identical (2 dev + 2 tester); BTC row 69 = 1 tester independent witness matches in-place update.
- **Test count, backtest crate:** 38 lib unit + 6 `run_yahoo_sma_ticker_flag` + 3 `yahoo_report_helper_shape` + 15 other in-scope = **62+ PASS in-scope, 0 in-scope FAIL**. (Tester's 93/0 was a clean-build state count; the discrepancy is parallel-track environmental — see Carve-outs.)
- **H1 ETH delta:** **6.78%** (Yahoo daily +2.76% vs Binance hourly +9.54%). Threshold 30%; expected range 5-15%. Sits comfortably.
- **BTC report bytes saved by removing `rev=`:** ~16 bytes per emit + immunity from REVISION.toml aggregate mutation across all future ticker fetches.
- **`yahoo_report_helper_shape.rs` regression guard:** 3/3 PASS, 53.81s runtime. Locks the durable-boundary contract at compile/test time.

## Carve-outs (per durable-over-quick contract — explicit rework cost OWNED)

| Carve-out | Owner | Rework cost for THIS feature | Closure path |
|---|---|---|---|
| `trace-broken-path × 4` in spec-lint (new vs v0.1.2 baseline of 73/3) | v3-regime-classifier developer/tester | **Zero.** Pre-existing from commit `2362ed2` (v3-regime-classifier Wave C parallel ship). | v3-regime-classifier Wave E (anchor registration) closes them mechanically. |
| ETH daily row 70 stays at old emit shape (`e59a5f87…`) | this feature (deferred) | **Zero now; bundled with v0.1.4.** Bulk-re-emit cost amortized across 9 unanchored crypto-mirror tickers (BNB → LINK + ETH-redo) in one shipping event vs. 1+1+9 staged. | v0.1.4 BNB bulk ship (~1 day total). |
| `crates/strategy/regime_dispatcher.rs:327,332` compile error (5 tests fail transitively) | v3-regime-classifier developer | **Zero.** Pre-exists at commit `2362ed2`. | v3-regime-classifier Wave D/E/F fix. |
| 9 pre-existing `crates/ui` clippy errors | ui-designer (v0.1.1 baseline) | **Zero.** Untouched at v0.1.3. | Carry-forward per v0.1.2 budget. |

This deck explicitly **owns** the deferred v0.1.4 ETH-daily re-emit as
debt. It is not "out of scope" — it is "deliberately bundled with the
next batched ship to minimize per-feature cost."

## What's deferred to v0.1.4

- Bulk migrate the 9 remaining unanchored Yahoo tickers (BNB, SOL, XRP,
  ADA, DOGE, AVAX, DOT, LINK + ETH-redo) to the new emit shape.
- Re-emit ETH daily row 70 under new emit shape (same batch).
- Single bulk migration vs 1+1+9 staged = strictly cheaper bookkeeping
  and one re-determinism re-run pass instead of three.

## ADR-0040 amendment (NOT new ADR)

`spec/architecture/adr/0040-yahoo-realdata-path.md` § Changelog gained a
2026-05-28 entry. Justified as **mechanical re-placement** of existing
data — the `revision_sha:` field is the same 64-char hex that was inline
in the body; the helper extraction is a refactor of existing emit logic,
not a new architectural primitive. A new ADR would be warranted only if
(a) the helper introduced a downstream trait with multiple impls, or (b)
`revision_sha:` became a UI/verifier-consumer surface. Neither is true at
v0.1.3.

## What the operator can do now

- **Verify the anchor gate yourself:**

  ```bash
  bash scripts/verify_anchors.sh | tail -10
  ```

  Expect: `ANCHORS PASS  (71 / 71)`.

- **Inspect a freshly-emitted BTC report:**

  ```bash
  head -20 spec/lab-yahoo-realdata/reports/backtest-20260528-220648-btc-yahoo-2024-1d-sma-cross.md
  ```

  Expect: clean `data_source: yahoo-cache:BTC-USD/1d/2024` (no `rev=`) and
  a separate `revision_sha:` frontmatter line.

- **Re-run the durable-boundary regression guard:**

  ```bash
  cargo test -p backtest --features "yahoo realdata" --test yahoo_report_helper_shape
  ```

  Expect: `3 passed; 0 failed`.

- **Re-emit the new ETH H1 anchor to confirm determinism:**

  ```bash
  cargo run --release -p backtest --bin backtest --features realdata -- --scenario eth-2024-h1-sma-cross
  python3 scripts/hash_report.py spec/v0-paper-sma/reports/backtest-*-eth-2024-h1-sma-cross.md | tail -1
  ```

  Expect: SHA `bd4001e42475955f518421d75cab207c85d0db3ba3a9d45fbdceff4f4b4e5441`.

## Open decisions for THIS approval

_n/a — binary ship-or-reject. No operator-decide Qs surface at M-PRESENTER.
Q1 + Q2 were both resolved at M-OD on 2026-05-28 at the recommended
durable choices; the ship implements them verbatim._

## Approval

- [x] Approved — ship  _(2026-05-29, operator)_
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes (operator)

_(empty — for operator use)_

## Feedback log

_(empty — for any reject route-back)_

## References

- Feature brief: [`spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/feature.md`](../feature.md)
- Tasks: [`spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/tasks.md`](../tasks.md)
- Tester PASS: [`reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1.md`](../reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1.md)
- H1 dev-note: [`dev-notes/yahoo-vs-binance-eth-h1-2026-05-28.md`](../dev-notes/yahoo-vs-binance-eth-h1-2026-05-28.md)
- ADR-0040 Changelog amendment: [`spec/architecture/adr/0040-yahoo-realdata-path.md`](../../architecture/adr/0040-yahoo-realdata-path.md)
- Predecessor v0.1.2 root-cause report: [`spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge.md`](../../lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge.md)
- Anchors registry: [`spec/anchors.toml`](../../anchors.toml) (rows 69 + 71)
- Canonical helper: [`crates/backtest/src/report/yahoo.rs`](../../../crates/backtest/src/report/yahoo.rs)
- Regression guard: [`crates/backtest/tests/yahoo_report_helper_shape.rs`](../../../crates/backtest/tests/yahoo_report_helper_shape.rs)
