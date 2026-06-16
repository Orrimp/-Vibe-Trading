---
slug: lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit
version: 0.1.0
status: retired
owner: developer
updated: 2026-06-16
predecessor: lab-yahoo-realdata-v0.1.3 v0.1.0
priority: P2
---

# Lab Yahoo realdata v0.1.4 — bulk-ticker re-emit (9 new + ETH-daily redo)

> **🚫 RETIRED 2026-06-16 (operator decision).** Non-load-bearing completeness item
> (full 10-ticker Yahoo coverage + a "2 → 10 tickers" badge). The active-vs-passive
> research program concluded 2026-06-08 (ship passive), so 9 more Yahoo SMA backtests
> change no conclusion and unlock nothing; it was also blocked on a manual operator-side
> Yahoo fetch (Yahoo blocks automated requests). Owned-debt from the v0.1.3 deck, honestly
> retired in wind-down rather than forced. The brief below is retained as archaeology.

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
  [`../lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/presentations/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1-2026-05-28.md`](../archive/presentations-2026-Q2.tar.gz)
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

## Design

> Architect M-T1 ratification 2026-05-29. M-OD resolved both LOAD-BEARING
> rows at the DURABLE picks: **Q1=(a)** 9 Binance H1 scenarios (zero K1
> fallbacks), **Q2=(a)** single namespace `lab-yahoo-realdata-v0.1.4`.
> Analyst's M-T1 fast-skip framing CONFIRMED — ADR-0040 § Changelog
> amendment only, no new ADR (per D-V0.1.3-7 + D-V0.1.2-6 precedent).
> Zero new architectural primitives; pure operational scaling of the v0.1.3
> helper (`crates/backtest/src/report/yahoo.rs`) + v0.1.3 D-V0.1.3-5
> H1-registration template across 9 ticker rows.

### D-V0.1.4-1 — Q1 ratification: 9 Binance H1 scenarios registered, zero K1 fallbacks

**Decision.** Architect RATIFIES operator Q1=(a) DURABLE. Each of the 9
new tickers (BNB, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK, MATIC) registers
a `{ticker-lc}-2024-h1-sma-cross` Binance hourly scenario in
`crates/backtest/src/main.rs` at the **three** match-arm sites identified
by D-V0.1.3-5 ETH-H1 template:

| Site | Anchor in main.rs (HEAD) | Mirror shape |
|---|---|---|
| Scenario config dispatch | L252-269 `btc-2024-h1-sma-cross`, L271-288 `eth-2024-h1-sma-cross` | New arm per ticker; `bar_count: 262_800` (see D-V0.1.4-2); SmaCrossover{20,50}; capital $100k; slip 2bps; taker 4bps |
| Synthetic fallback start price | L1118 `btc → dec!(42_000)`, L1119 `eth → dec!(2_400)` | New arm per ticker with 2024-Q1-open approximation (developer T-D7 picks the constant; non-load-bearing — real-parquet path always wins) |
| `scenario_to_feature` SMA group | L1939-1940 `"btc-2024-h1-sma-cross" \| "eth-2024-h1-sma-cross" => "v0-paper-sma"` | Extend the alternation with all 9 new arms; report dir stays `spec/v0-paper-sma/reports/` |

**Rationale.** Verbatim mirror of v0.1.3 D-V0.1.3-5 shipped 2026-05-29
for `eth-2024-h1-sma-cross` (anchor row 71 `bd4001e4…` byte-identical
over 2 independent runs). Each registration is ~6-8 mechanical LoC across
3 sites. Zero new architectural decisions — operationalises the v0.1.3
pattern across 9 ticker rows.

**Anchor neutrality.** New scenarios are append-only on `spec/anchors.toml`
under namespace `lab-yahoo-realdata-v0.1.4` (D-V0.1.4-4). No existing
match-arm body mutates; rows 69 (BTC), 71 (ETH-H1) byte-identical.

### D-V0.1.4-2 — K1 pre-flight: `bar_count: 262_800` mirrors v0.1.3 pattern verbatim

**Decision.** All 9 new Binance H1 scenarios use `bar_count: 262_800` —
**identical** to `btc-2024-h1-sma-cross` (L258) and
`eth-2024-h1-sma-cross` (L277). This is the **1-minute equivalent**
count (~182.5 days × 1440 bars/day), NOT the true H1 count.

**Why this is correct (v0.1.3 T-D4 resolution, ratified architect-side):**
The real-parquet auto-detect path in `crates/backtest/src/main.rs` overrides
`scenario.bar_count` with `bars.len()` from
`data/binance/{SYM}USDT/2024/*.parquet` (12 monthly hourly parquets,
~17,543 bars) at L1149-1150 (`let bar_count = bars.len();`). The
`scenario.bar_count` field becomes the **synthetic-fallback warm-up size**
only — the realdata path NEVER consumes it for bar-count enforcement.
v0.1.3 ETH-H1 anchor row 71 (`bd4001e4…`) is the existence proof: 2 runs
byte-identical with `bar_count: 262_800` declared and 17,543 actual.

**Developer pre-flight contract (T-D7).** Developer MUST copy the constant
verbatim from the BTC/ETH-H1 arms; do NOT recompute to 8_760 (true H1
count). Recomputation would diverge from v0.1.3 D-V0.1.3-5 pattern with
zero functional benefit and one regression risk (synthetic-fallback test
paths reading the value would differ).

**Falsifier.** If `scripts/verify_anchors.sh` flips row 71 ETH-H1 SHA
during developer Wave B work, that proves the `bar_count` is in fact
load-bearing on the realdata path → route back analyst. (Architect bets
this does NOT happen — v0.1.3 already proved the pattern.)

### D-V0.1.4-3 — K3 partial-year-listing edge for AVAX/MATIC: 95% threshold + per-ticker route-back

**Decision.** Architect ratifies the analyst's K3 falsifier shape with
two clarifications:

1. **Threshold (ADR-0040 § R3): 95% present (≥ 348 daily bars of 366
   possible)** — applies UNIFORMLY across all 9 new tickers. No per-ticker
   threshold widening at v0.1.4 architect-side.

2. **Operator-side fan-out surfaces K1 BEFORE M-DEV consumes it.** R1
   fetch command (`fetch_yahoo_klines --tickers BNB-USD,…,MATIC-USD …`)
   is the trip-wire: if AVAX or MATIC return < 95% (348 bars), the
   `YahooError::MissingData` fires at operator-side cache populate,
   BEFORE the developer attempts re-emit. Operator routes back analyst
   with the per-ticker drop-or-widen decision.

**Handling matrix (operator-decide on K1 fire):**

| Option | Action | Anchor cascade | Deck impact |
|---|---|---|---|
| (i) Drop ticker | Ship 8 + ETH-daily (cascade 71 → 79 not 80); namespace `lab-yahoo-realdata-v0.1.4` shrinks by 1 row | Net 71 → 79 | Deck § Carve-outs: "{ticker} deferred to v0.1.5 partial-year-listing-edge brief" |
| (ii) Widen for that ticker | Per-ticker threshold override (e.g. 80% for AVAX); requires ADR-0040 § R3 amendment | Net 71 → 80 (ticker ships at lower coverage) | Deck § Quality carve-out: "{ticker} shipped at X% coverage per K1 widening decision" |

**Architect default if K1 fires.** Prefer (i) **drop** — shipping 8 +
ETH-daily preserves the durable contract (no per-ticker thresholds
fragment ADR-0040 § R3) and the dropped ticker gets its own v0.1.5
brief with a focused partial-year-listing-edge ADR amendment. Option
(ii) requires architect re-engagement to amend ADR-0040 § R3 — NOT
fast-skip.

**Expected outcome.** AVAX-USD 2024 1d has full 366 bars in Yahoo (AVAX
launched Sept 2020 with continuous trading on AVAX-USD pair). MATIC-USD
2024 1d is the higher risk (Polygon rebrand Sept 2024 → POL; Yahoo
may have continued mirroring MATIC-USD or switched cadence). K1 is a
**legitimate** falsifier candidate for MATIC specifically. R1 operator
fetch settles it.

### D-V0.1.4-4 — Q2 ratification: anchor cascade 71 → 80 (1 in-place + 9 appends)

**Decision.** Architect RATIFIES operator Q2=(a) DURABLE single
namespace. Cascade matches feature.md § R3 verbatim:

| Row | Scenario | v0.1.3 state | v0.1.4 outcome | Namespace |
|---:|---|---|---|---|
| 1–68 | (non-Yahoo) | byte-identical | byte-identical | preserved |
| 69 | `btc-yahoo-2024-1d-sma-cross` | `076929bb…` | **byte-identical** (determinism witness) | `lab-yahoo-realdata-v0.1.1` (preserved per Q2=(a)) |
| 70 | `eth-yahoo-2024-1d-sma-cross` | `e59a5f87…` (OLD emit shape) | **SHA in-place UPDATE** to v0.1.3 helper shape | `lab-yahoo-realdata-v0.1.2` (preserved per v0.1.3 D-V0.1.3-4 in-place precedent) |
| 71 | `eth-2024-h1-sma-cross` | `bd4001e4…` | byte-identical | `lab-yahoo-realdata-v0.1.3` (preserved) |
| 72 | `bnb-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 73 | `sol-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 74 | `xrp-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 75 | `ada-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 76 | `doge-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 77 | `avax-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 78 | `dot-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 79 | `link-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |
| 80 | `matic-yahoo-2024-1d-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.4` |

**Net: 71 → 80** (1 in-place row 70 update + 9 appends rows 72-80).
**Architect-explicit: 9 NEW Binance H1 anchors (`{ticker}-2024-h1-sma-cross`)
emitted by Wave B do NOT add anchor rows at v0.1.4** — they are
H1-discharge artifacts that prove H1 hypothesis < 30% per ticker. If
operator wants those H1 rows anchored at v0.1.5+, that ships as a
follow-on brief (single-namespace `lab-yahoo-realdata-v0.1.5` would
append rows 81-89). Architect rationale: the H1-discharge happens
*directly* per ticker; anchoring it locks the regression. Trading
v0.1.4 surface cost (deck length, namespace churn) for v0.1.5 cleanup
debt is the cheaper path — analyst can re-litigate at v0.1.5 framing.

**In-place precedent invocation.** Row 70 SHA flips from `e59a5f87…` to
NEW under namespace `lab-yahoo-realdata-v0.1.2` per ADR-0038 § D6.b
wiring-bug-fix re-emission protocol (OR its in-place variant per
D-V0.1.3-4 + v5 v0.3.0+v0.4.0 precedent). The body bytes change because
the v0.1.3 helper-extracted emit shape replaces `rev=<sha>` body
substring with `revision_sha:` frontmatter line — same data, different
placement. Determinism witness: BTC row 69 SHA STAYS at `076929bb…`
(unchanged at v0.1.3 because v0.1.3 already re-emitted BTC under helper
shape). Therefore the regression guard for the v0.1.4 in-place mutation
is BTC row 69's continued byte-identity (`076929bb…`).

### D-V0.1.4-5 — ADR-0040 § Changelog amendment (NOT new ADR)

**Decision.** Architect ratifies analyst's M-T1 fast-skip framing.
ADR-0040 receives a § Changelog amendment dated **2026-05-29 (architect,
M-T1 lab-yahoo-realdata-v0.1.4)**. NO new ADR.

**Rationale (mirrors D-V0.1.3-7 verbatim, scoped to v0.1.4):**

- The 9 new Binance H1 scenario registrations are **mechanical scaling**
  of the v0.1.3 D-V0.1.3-5 ETH-H1 template (1 → 10 H1 scenarios). No
  new API surface, no new dispatch arm, no new strategy type.
- The 9 new Yahoo-daily anchor rows are **mechanical scaling** of the
  v0.1.2 per-ticker pattern via the v0.1.3 helper (`run_yahoo_sma --ticker
  {T}`). The helper at `crates/backtest/src/report/yahoo.rs` is the
  FROZEN durable boundary per R5.7 — zero diff allowed.
- Row 70 ETH-daily in-place SHA update follows the **v0.1.3 D-V0.1.3-4
  in-place precedent** (BTC row 69 namespace `lab-yahoo-realdata-v0.1.1`
  preserved; SHA mutated). ADR-0040 already documents this pattern.
- No new external crate dep (CLAUDE.md non-negotiable not triggered).
- No new architectural primitive (helper trait, dispatch enum, module).

A new ADR-0051 would be warranted IF (a) the 9 H1 registrations
required a multi-ticker dispatch refactor (e.g., `MultiH1Scenario`
trait), or (b) the K1 partial-year-listing edge forced ADR-0040 § R3
per-ticker threshold amendments. Neither is true at M-T1 architect-side
analysis — both are mechanical or operator-decide-at-fire.

**ADR-0040 § Changelog amendment shape** (≤55 lines, locked at T-T1.5;
emitted as part of this M-T1 close):

> 2026-05-29 (architect, M-T1 lab-yahoo-realdata-v0.1.4): bulk-ticker
> re-emit of 9 remaining Yahoo crypto-mirror tickers + ETH-daily row 70
> migration to v0.1.3 helper shape + 9 Binance H1 scenario registrations
> mirroring v0.1.3 D-V0.1.3-5 ETH template. **No new architectural
> decisions** — pure operational scaling of v0.1.2 D-V0.1.2-6 per-ticker
> pattern + v0.1.3 D-V0.1.3-1 helper boundary + v0.1.3 D-V0.1.3-5 H1
> registration template across 9 ticker rows.

### D-V0.1.4-6 — Determinism + non-regression contract (R5.7 FROZEN boundary)

**Decision.** R5.7 **FROZEN durable boundary** ratified verbatim from
analyst brief. Zero diff allowed in:

- `crates/backtest/src/report/yahoo.rs` (v0.1.3 helper — D-V0.1.3-1)
- `crates/backtest/src/report/sma.rs` (v0.1.3 helper consumer)
- `crates/backtest/src/report/mod.rs` (re-export surface)
- `crates/backtest/src/bin/run_yahoo_sma.rs` (CLI shape — D-V0.1.2-6)

Tester confirms at M-FINAL T-F4 via `git diff HEAD~ HEAD -- <4 files>`
returning empty.

**Allowed diff at v0.1.4** (developer Wave B):

- `crates/backtest/src/main.rs` — 9 new scenario arms at 3 sites each
  (D-V0.1.4-1)
- `spec/anchors.toml` — row 70 SHA in-place update + 9 row appends
  (D-V0.1.4-4)
- `spec/<slug>/reports/` per-ticker dev-notes (Wave C)
- `data/yahoo/REVISION.toml` — operator-side R1 populate (BLOCKER)
- Developer T-D1 evidence paste-in to trace row

**Determinism gates (≥ 2 independent re-runs before anchor insertion):**

- T-D2 BTC row 69: SHA must match `076929bb…` on ≥ 2 runs (determinism
  witness — proves helper boundary unchanged + Yahoo cache stable).
- T-D3 ETH-daily row 70: SHA must match across ≥ 2 runs before in-place
  update (determinism witness for the new SHA).
- T-D4 each of 9 new tickers: SHA must match across ≥ 2 runs before
  anchor append (per-ticker determinism witness).

### D-V0.1.4-7 — Wave decomposition (M-DEV: Wave A ‖ Wave B → Wave C → Wave D gates)

**Decision.** Architect decomposes developer M-DEV into 4 waves per
orchestrator directive. Wave A and Wave B run in **parallel** — neither
blocks the other on file scope (Wave A touches `spec/anchors.toml` rows
70 + 72-80; Wave B touches `crates/backtest/src/main.rs` only).

**Wave A — Bulk re-emit (R2 + R3, ~1.5 days)**

Helper-instantiation only; zero new code. Operator-side R1 fetch is the
gate.

- T-D1 pre-flight: confirm R1 evidence in `data/yahoo/REVISION.toml`
  (aggregate SHA recomputed; +108 file rows + 9 yahoo_response keys);
  helper-bypass regression guard 3/3 PASS at HEAD.
- T-D2 re-emit BTC `--ticker BTC-USD` × 2 runs → SHA matches row 69
  `076929bb…` (determinism witness; gate before T-D3).
- T-D3 re-emit ETH-daily `--ticker ETH-USD` × 2 runs → record NEW SHA;
  row 70 in-place under `lab-yahoo-realdata-v0.1.2`.
- T-D4 re-emit 9 new tickers (BNB/SOL/XRP/ADA/DOGE/AVAX/DOT/LINK/MATIC)
  × 2 runs each → record 9 SHAs.
- T-D5 `spec/anchors.toml` mutation: row 70 in-place + rows 72-80
  append under `lab-yahoo-realdata-v0.1.4`.
- T-D6 `scripts/verify_anchors.sh` → ANCHORS PASS (80 / 80).

**Wave B — 9 Binance H1 scenario registrations (R-Q1=(a), ~3-4 days)**

Mechanical mirror of v0.1.3 D-V0.1.3-5 × 9. Independent of Wave A on
file scope.

- T-D7 register `{ticker-lc}-2024-h1-sma-cross` × 9 in
  `crates/backtest/src/main.rs` at 3 sites each (D-V0.1.4-1). Copy
  `bar_count: 262_800` verbatim (D-V0.1.4-2).
- T-D8 H1 discharge × 9: `cargo run --features realdata --bin backtest
  -- {ticker-lc}-2024-h1-sma-cross` × 2 runs per ticker → record
  Binance hourly equity per ticker; compute delta vs Yahoo daily per
  ticker; H1 hypothesis threshold 30%.

**Wave C — Per-ticker H1 dev-notes (durable contract, ~0.5 day)**

Per orchestrator directive: durable contract requires per-ticker honest
reporting. Each ticker gets ONE dev-note entry, not a single bulk note.

- T-D9 emit `spec/<slug>/reports/h1-discharge-{ticker-lc}-2026-05-XX.md`
  × 9 (or one consolidated `yahoo-vs-binance-bulk-h1-2026-05-XX.md` with
  9 per-ticker sections — developer chooses). Each section records:
  Yahoo-daily equity, Binance-hourly equity, delta %, pass/fail vs 30%
  threshold, K4 falsifier-fire status.

**Wave D — Gates**

- T-D10 `cargo fmt --check` + `clippy -D warnings` on touched paths
  (pre-existing 9 ui clippy carried over per R-NR.3).
- T-D11 R5.7 frozen-boundary `git diff` empty for the 4 files.
- T-D12 workspace lib tests green (411 ui + non-ui baseline preserved);
  owner flip → tester.

**Wave A ‖ Wave B parallelism rationale.** Wave A consumes
`data/yahoo/*.parquet` + writes `spec/anchors.toml`. Wave B writes
`crates/backtest/src/main.rs` + consumes `data/binance/*.parquet`. Zero
file-scope overlap; zero data-flow dependency. Developer should
sub-agent the two waves where load permits, or sequence A → B if
single-stream is easier (both produce identical M-FINAL inputs).

### D-V0.1.4-8 — File-scope contract for M-DEV

**Allowed file mutations at v0.1.4:**

- `crates/backtest/src/main.rs` — Wave B 9 scenario arms (D-V0.1.4-1)
- `spec/anchors.toml` — Wave A row 70 in-place + 9 row appends
  (D-V0.1.4-4)
- `spec/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/reports/` —
  Wave C per-ticker dev-notes
- `data/yahoo/REVISION.toml` — operator-side R1 populate (BLOCKER for
  M-DEV start)
- `data/yahoo/{BNB,SOL,XRP,ADA,DOGE,AVAX,DOT,LINK,MATIC}-USD/1d/2024/*.parquet`
  — operator-side R1 populate output (`.gitignore`d per ADR-0040 §
  Q10=(b); only REVISION.toml ships to git)
- Frontmatter `owner` flip (developer → tester at M-DEV close)

**Disallowed file mutations at v0.1.4 (R5.7 FROZEN boundary):**

- `crates/backtest/src/report/yahoo.rs`
- `crates/backtest/src/report/sma.rs`
- `crates/backtest/src/report/mod.rs`
- `crates/backtest/src/bin/run_yahoo_sma.rs`
- `crates/data/src/yahoo.rs` (Yahoo cache loader — ADR-0040 § D5)
- Any anchored report file in `spec/*/reports/` other than row 70
  (CLAUDE.md non-negotiable: anchored reports byte-immutable)

### D-V0.1.4-9 — Operator-side R1 fetch is M-DEV start gate

**Decision.** R1 operator-side cache populate (feature.md § R1.1 verbatim
command) is the **M-DEV start gate**. Developer cannot proceed past T-D1
pre-flight without:

1. Operator paste of post-fetch `data/yahoo/REVISION.toml`
   `[revision].sha256` aggregate into M-DEV T-D1 trace row.
2. Operator confirmation that `[files]` grew by +108 monthly rows
   (9 tickers × 12 months = 108).
3. Operator confirmation that `[revision.yahoo_response]` grew by +9
   keys (1 per ticker).

K1 partial-year-listing falsifier (D-V0.1.4-3) fires HERE at operator
side BEFORE developer consumes data. If R1 reports `YahooError::MissingData`
for AVAX or MATIC, operator routes back analyst with drop-or-widen
decision per D-V0.1.4-3 matrix.

**Architect-side acknowledgement.** Architect M-T1 close does NOT block
on R1 — the brief + tasks + design are operator-fetch-independent. R1
is M-DEV's gate, not M-T1's.

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
- 2026-05-29 (architect, M-T1 fast-skip): § Design appended (D-V0.1.4-1
  through D-V0.1.4-9). M-OD rows ratified at DURABLE picks — Q1=(a) 9
  Binance H1 scenarios, Q2=(a) single namespace `lab-yahoo-realdata-v0.1.4`.
  Analyst's M-T1 fast-skip framing CONFIRMED — ADR-0040 § Changelog
  amendment only, no new ADR (per D-V0.1.3-7 + D-V0.1.2-6 precedent).
  K1 pre-flight CONFIRMED: `bar_count: 262_800` mirrors v0.1.3 BTC+ETH-H1
  pattern verbatim (real-parquet auto-detect overrides per v0.1.3 T-D4
  resolution). K3 partial-year-listing edge for AVAX/MATIC: 95% threshold
  uniform; operator-side R1 fetch surfaces K1 BEFORE M-DEV; default
  drop-on-fire (option (i)) preserves ADR-0040 § R3 single-threshold
  durability. Anchor cascade ratified: row 70 in-place under
  `lab-yahoo-realdata-v0.1.2`, rows 72-80 append under
  `lab-yahoo-realdata-v0.1.4`; 9 NEW Binance H1 anchors emitted by Wave B
  NOT added at v0.1.4 (operator can re-litigate at v0.1.5 follow-on).
  Wave decomposition: Wave A bulk re-emit (R2+R3, ~1.5d) ‖ Wave B 9
  Binance H1 regs (~3-4d) → Wave C per-ticker H1 dev-notes (~0.5d, durable
  honest-reporting contract) → Wave D gates. Operator-side R1 fetch
  command is M-DEV START GATE (T-D1 evidence paste-in required). R5.7
  FROZEN boundary contract: zero diff in 4 files (`report/yahoo.rs`,
  `report/sma.rs`, `report/mod.rs`, `run_yahoo_sma.rs`); tester confirms
  at M-FINAL T-F4. Frontmatter flipped `owner: analyst → developer`,
  `status: draft → arch-done`. Trace row `REQ-LAB-YAHOO-REALDATA-V0-1-4-001`
  `arch` column populated; state `proposed → arch-done`. backlog Active
  annotation appended. ADR-0040 § Changelog amended (D-V0.1.4-5 shape).
  HANDOFF → developer (Wave A ‖ Wave B; R1 operator fetch is start gate).
