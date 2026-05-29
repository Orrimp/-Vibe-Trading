---
slug: lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit
version: 0.1.0
status: draft
owner: analyst
updated: 2026-05-29
predecessor: lab-yahoo-realdata-v0.1.3 v0.1.0
priority: P2
---

# Lab Yahoo realdata v0.1.4 — bulk-ticker re-emit (9 new + ETH-daily redo)

> Closes the v0.1.3 deck's explicit **owned-debt** commitment: re-emit the 9
> remaining Yahoo crypto-mirror tickers (BNB, SOL, XRP, ADA, DOGE, AVAX, DOT,
> LINK, MATIC) AND re-emit ETH-daily row 70 under the v0.1.3 helper-extracted
> emit shape (`revision_sha:` in front-matter, no `rev=` in body). After
> ship, all 10 crypto-mirror tickers carry anchored 2024 SMA(20,50) backtests;
> `cache_state_summary_badge` flips "2 tickers" → "10 tickers" — meaningful
> at full universe coverage.

## Why

v0.1.3 shipped 2026-05-29; ETH-daily row 70 stayed at OLD emit shape per
explicit deferral. v0.1.3 deck § Carve-outs: "deliberately bundled with the
next batched ship". v0.1.4 IS that ship. Bundling 9 + 1 wins: single
determinism re-run pass, single operator-fetch step, one namespace line,
one badge-flip event.

## Scope

- R1 operator-side fetch (BLOCKER) → 9 tickers × 2024 1d populated.
- R2 bulk re-emit of 10 tickers + ETH-daily via v0.1.3 helper.
- R3 anchor cascade 71 → 80 (1 in-place row 70 update + 9 appends).
- R4 badge meaningfulness (2 → 10; zero UI code change).
- R-Q1 Recommended: 9 new Binance hourly scenarios for direct per-ticker H1.

## Out of scope

- Any change to `crates/backtest/src/report/yahoo.rs`, `report/sma.rs`,
  `report/mod.rs`, `run_yahoo_sma.rs` — FROZEN per v0.1.3 D-V0.1.3-1 (R5.7).
- Multi-strategy on Yahoo (MACD/RSI/BBands) — v0.2.0+.
- New design tokens / `strings.rs` — backend + scenario-reg only.
- Years beyond 2024.
- UI code for badge — automatic on `REVISION.toml` populated-count.

## Requirements

### R1 — Operator-side cache populate (BLOCKER for M-DEV)

- **R1.1** Operator MUST run exactly:
  ```bash
  cargo run --release -p data --features yahoo-online --bin fetch_yahoo_klines -- \
    --tickers BNB-USD,SOL-USD,XRP-USD,ADA-USD,DOGE-USD,AVAX-USD,DOT-USD,LINK-USD,MATIC-USD \
    --interval 1d \
    --start 2024-01-01 \
    --end 2024-12-31
  ```
- **R1.2** Post: `REVISION.toml` `[files]` +108 monthly rows; `[revision].sha256`
  recomputed; `[revision.yahoo_response]` +9 keys.
- **R1.3** Dev cannot proceed without R1 evidence. Trace blocks until
  operator pastes the post-fetch aggregate SHA into M-DEV T-D1.
- **Acceptance**: aggregate SHA recomputed; +108 file rows + 9 yahoo_response.

### R2 — Bulk re-emit contract

- **R2.1** For each of {BTC, ETH, BNB, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK,
  MATIC}: `cargo run --release -p backtest --features yahoo --bin run_yahoo_sma -- --ticker {TICKER}`.
- **R2.2** BTC is determinism witness (must match `076929bb…`); ETH-daily
  mutates row 70 SHA from `e59a5f87…` → NEW under v0.1.3 helper shape.
- **R2.3** Byte-identity guaranteed by v0.1.3 regression guard
  (`tests/yahoo_report_helper_shape.rs`, 3/3 PASS preserved).
- **Acceptance**: 11 reports emitted; BTC matches; 10 SHAs recorded.

### R3 — Anchor cascade (71 → 80)

| Row | Scenario | v0.1.3 state | v0.1.4 outcome | Namespace |
|---:|---|---|---|---|
| 1–68 | (non-Yahoo) | byte-identical | byte-identical | preserved |
| 69 | `btc-yahoo-2024-1d-sma-cross` | `076929bb…` | byte-identical (det. witness) | `lab-yahoo-realdata-v0.1.1` |
| 70 | `eth-yahoo-2024-1d-sma-cross` | `e59a5f87…` (old emit) | **SHA in-place UPDATE** to v0.1.3 helper shape | `lab-yahoo-realdata-v0.1.2` (preserved per v0.1.3 D-V0.1.3-4 precedent) |
| 71 | `eth-2024-h1-sma-cross` | `bd4001e4…` | byte-identical | `lab-yahoo-realdata-v0.1.3` |
| 72 | `bnb-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 73 | `sol-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 74 | `xrp-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 75 | `ada-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 76 | `doge-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 77 | `avax-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 78 | `dot-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 79 | `link-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 80 | `matic-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |

- **R3.1** Net 71 → 80 (1 in-place + 9 appends).
- **R3.2** Row 70 namespace preserved per v0.1.3 in-place precedent; SHA mutates.
- **R3.3** 9 new rows under single namespace `lab-yahoo-realdata-v0.1.4` (Q2=(a)).
- **R3.4** Determinism: each new SHA ≥ 2 independent re-runs before insertion.
- **Acceptance**: `verify_anchors.sh` ANCHORS PASS (80 / 80).

### R4 — Aggregate cache-state badge meaningfulness

- **R4.1** Before: badge "Yahoo cache: 2 tickers · last YYYY-MM-DD".
- **R4.2** After R1: badge "Yahoo cache: 10 tickers · last YYYY-MM-DD" —
  aligned with ADR-0040 D7 universe.
- **R4.3** Zero UI code change — `cache_state::probe_summary` picks up new
  parquets automatically.
- **Acceptance**: operator Lab tab shows "10 tickers" post-R1.

### R5 — Non-regression contract

- **R5.1** Rows 1-68 byte-identical.
- **R5.2** Row 69 BTC byte-identical at `076929bb…` (det. witness).
- **R5.3** Row 71 ETH-H1 byte-identical at `bd4001e4…`.
- **R5.4** Helper-bypass regression guard 3/3 PASS preserved.
- **R5.5** H2 fetch success ≥ 95% across 9 tickers.
- **R5.6** Workspace lib tests green (411 ui + non-ui baseline).
- **R5.7** **Durable boundary FROZEN** — zero diff in
  `crates/backtest/src/report/yahoo.rs`, `report/sma.rs`, `report/mod.rs`,
  `run_yahoo_sma.rs`. Tester confirms via `git diff` at M-FINAL.

### R-NR — Cross-cutting

- **R-NR.1** Zero new design tokens.
- **R-NR.2** Zero new `strings.rs` entries.
- **R-NR.3** `cargo fmt --check` + `clippy -D warnings` clean on touched
  paths (only `crates/backtest/src/main.rs` for Q1=(a) arms; pre-existing
  9 ui clippy carried over).
- **R-NR.4** spec-lint baseline-stable vs 77/4 v0.1.3 baseline.

## Operator-decide Q-rows

- **Q1 — Per-ticker H1 verification scope (LOAD-BEARING; durable per AGENT.md 2026-05-29).**
  **(a) [Recommended — DURABLE]** Each of 9 new tickers registers a Binance
  hourly scenario (`{ticker-lc}-2024-h1-sma-cross`) in
  `crates/backtest/src/main.rs` (3 match-arm sites per v0.1.3 D-V0.1.3-5).
  H1 discharges DIRECTLY per ticker. ~+0.5 day/ticker (~+5 days total).
  Zero K1 Yahoo-to-Yahoo fallbacks ship; uniform H1 contract across 10.
  **(b) [cheap fallback]** Register Binance H1 for BNB only; 8 others
  discharge H1 via K1 Yahoo-to-Yahoo fallback. 8 K1 fallbacks ship + 8
  v0.1.5+ cleanup briefs accumulate (~+5 days deferred PER ticker + 8 H1
  carve-outs in v0.1.4 deck — silent-deferral pattern operator dislikes).
  *Analyst recommends (a)* — v0.1.3 helper-extraction precedent: durable
  picks strictly better net wall-clock + zero downside on additive work.

- **Q2 — Anchor namespace for 9 new + re-emitted ETH-daily.**
  **(a) [Recommended — DURABLE]** Single namespace
  `lab-yahoo-realdata-v0.1.4` for 9 new rows; ETH-daily row 70 stays
  `lab-yahoo-realdata-v0.1.2` with in-place SHA update (v0.1.3 in-place
  precedent for BTC row 69). One namespace per Yahoo origin event.
  **(b) [cheap fallback]** Per-ticker namespaces; fragments tracking; 9
  namespace lines; future v0.2.0 rollouts recur consolidation cost.
  *Analyst recommends (a)* — v0.1.2 + v0.1.3 precedent.

## Cost framing (durable-vs-quick)

| Phase | Q1=(a)+Q2=(a) Recommended | Q1=(b)+Q2=(b) cheap |
|---|---:|---:|
| M0 / M-OD / M-T1 fast-skip | ~1h | ~1h |
| R1 operator fetch | ~10-30 min | same |
| M-DEV Wave A bulk-emit (10 + anchors) | ~1 day | ~1 day |
| M-DEV Wave B Binance H1 reg | ~3-4 days | ~0.5 day (BNB only) |
| M-FINAL + M-PRESENTER | ~1.5 day | ~1.5 day (+ 8 K1 carve-outs documented) |
| **Total wall-clock** | **~5-7 days (≈ 1 week)** | **~1.5-2.5 days** |
| **Deferred (v0.1.5+)** | **0 carve-outs** | **+5-7 days × 8 future briefs + 8 H1 carve-outs in v0.1.4 deck** |

Q1=(a) is +2-4 days at v0.1.4 vs Q1=(b) but -5-7 days across v0.1.5+ with
0 deferred carve-outs. Net wall-clock strictly better; durability strictly
better; zero downside. Q1=(b) IS cheaper now, only by accumulating 8
future ships with H1-carve-out drift — the debt v0.1.3 CLOSED for BTC + ETH.

## Risks / falsifiers (K-rows)

- **K1 — Yahoo ticker returns < 366 bars for 2024 (partial-year listing).**
  Likely for AVAX (mid-2020 listing edge) or MATIC (rebrand cadence). 95%
  threshold (ADR-0040 § R3) catches it. *Mitigation*: any fetch surfacing
  `YahooError::MissingData` routes back analyst with operator-decide on
  (i) drop ticker (ship 8 + ETH-daily), (ii) widen threshold for that
  ticker only. Operator-side fan-out surfaces K1 BEFORE M-DEV consumes it.
- **K2 — `REVISION.toml` grows unbounded.** v0.1.3: ~60 file rows. v0.1.4
  adds 108 monthly + 9 yahoo_response (~177 total). Aggregate SHA
  recomputed. *Mitigation*: file stays human-readable; ADR-0040 § D3
  schema unchanged.
- **K3 — Yahoo throttling at 9 successive fetches.** Free-tier soft-throttles
  bursts. *Mitigation*: H2 sets ≥ 95% success; ADR-0040 § D5
  `YahooError::RateLimited` + backoff. If H2 fires, operator chooses
  (i) re-run after 1h, (ii) bisect, (iii) accept partial.
- **K4 — Re-emitted ticker scenario fails to converge / errors.** Possible
  if a ticker's 2024 vol regime trips SMA(20,50) edge case or history <
  50-bar warm-up. *Mitigation*: identical engine path as BTC + ETH; same
  seed; same SmaCrossover{20,50}. If one fails, route per-ticker-skip.

## Hypotheses (H-rows)

- **H1** Yahoo daily vs Binance hourly equity divergence < 30% for all 9
  new tickers (mirrors BTC 9.03% / ETH 6.78%). *Falsifier*: any ≥ 30% → K4.
- **H2** Fetch success ≥ 95% across 9 tickers in one batch.
  *Falsifier*: < 95% → K3.
- **H3** `cache_state_summary_badge` shows "10 tickers" after R1 — no code
  change required. *Falsifier*: badge ≠ 10 → probe bug; route ui-designer
  + architect.

## Verdict tree (pre-drawn 2-cell)

```
       M-FINAL tester gates
              │
      ┌───────┴───────┐
   ALL GREEN       ANY RED
      │              │
   PASS         REGRESSION
      │              │
   presenter →   route back analyst
   operator →    with K1/K2/K3/K4
   ship
```

ALL GREEN: `verify_anchors.sh` 80/80 + `cargo fmt --check` + `clippy -D
warnings` (touched paths) + workspace lib tests green + spec-lint
baseline-stable + H1 per-ticker direct PASS (9) + H2 ≥ 95% + H3 badge
shows 10 + R5.7 frozen-boundary `git diff` empty for
`report/yahoo.rs`/`sma.rs`/`mod.rs` and `run_yahoo_sma.rs`.

REGRESSION: any SHA non-deterministic, any H1 ≥ 30%, H2 < 95%, or frozen-
boundary diff non-empty → route back analyst.

## References

- v0.1.3 predecessor + helper precedent:
  [`../lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/feature.md`](../lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/feature.md)
- v0.1.3 deck § What's deferred:
  [`../lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/presentations/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1-2026-05-28.md`](../lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/presentations/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1-2026-05-28.md)
- ADR-0040 § Changelog 2026-05-28:
  [`../architecture/adr/0040-yahoo-realdata-path.md`](../architecture/adr/0040-yahoo-realdata-path.md)
- Durable boundary (DO NOT MODIFY):
  [`../../crates/backtest/src/report/yahoo.rs`](../../crates/backtest/src/report/yahoo.rs)
- Binary re-invoked per ticker:
  [`../../crates/backtest/src/bin/run_yahoo_sma.rs`](../../crates/backtest/src/bin/run_yahoo_sma.rs)
- Ticker conversion table (ADR-0040 D7):
  [`../../crates/data/src/yahoo.rs`](../../crates/data/src/yahoo.rs)
- Cache state on disk:
  [`../../data/yahoo/REVISION.toml`](../../data/yahoo/REVISION.toml)
- v0.1.3 regression guard (must stay 3/3):
  [`../../crates/backtest/tests/yahoo_report_helper_shape.rs`](../../crates/backtest/tests/yahoo_report_helper_shape.rs)

## Changelog

- 2026-05-29 (analyst, M0): brief authored — closes v0.1.3 deck's owned-debt
  commitment (9 remaining crypto-mirror tickers + ETH-daily re-emit under
  v0.1.3 helper shape). 5 R + R-NR + 4 K + 3 H + 2 Q + non-regression +
  pre-drawn 2-cell verdict tree + cost framing (~1 week Q1=(a)+Q2=(a)
  Recommended). Q1=(a) per-ticker Binance H1 = DURABLE; Q1=(b) K1 fallback
  = cheap with 8 deferred briefs explicit. Q2=(a) single
  `lab-yahoo-realdata-v0.1.4` namespace = DURABLE. Anchor cascade 71 → 80
  (1 in-place row 70 + 9 appends rows 72-80). R1 operator cache populate
  recipe locked as M-DEV blocker. Trace row
  `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` opened `proposed`. HANDOFF → architect.
