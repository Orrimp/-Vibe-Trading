---
slug: lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge
version: 0.1.0
status: in-progress
owner: architect
updated: 2026-05-27
predecessor: lab-yahoo-realdata v0.1.1
priority: P2
---

# Lab Yahoo realdata v0.1.2 — ETH-USD anchor + cache-state summary badge

> **Operator decision 2026-05-27 (multi-select option C)**: close
> Q1 + Q3 of the v0.1.1 presenter deck's open list — (1) lock
> ETH-USD as anchor row 70 (namespace `lab-yahoo-realdata-v0.1.2`);
> (2) ship the deferred T-D2 **cache-state summary badge** — an
> aggregate multi-ticker indicator that COMPLEMENTS (does NOT
> replace) the per-pair Fresh/Stale/Empty pill shipped at v0.1.0
> Wave D-followup.

## Why

The v0.1.1 sprint review (2026-05-27, anchor 69 locked, H1 PASS at
9.03%) surfaced three deferred items: ETH-USD as the next anchor,
multi-strategy on Yahoo, and the T-D2 cache-state badge. Operator
picked option C — items 1 + 3 — because both are cheap, both close
v0.1.1's open list, and neither requires architectural change.

**ETH-USD specifically**: highest-liquidity altcoin; quasi-
independent of BTC (cleaner A/B); cache already populated at
`data/yahoo/ETH-USD/1d/2024/` (12 parquets, REVISION.toml
aggregate SHA `e018f876…`); Binance reference exists at
`data/binance/ETHUSDT/2024/`.

**Aggregate cache-state badge** (Q3 in v0.1.1 deck): the v0.1.0
Wave D-followup already shipped a per-pair pill; what's missing is
the multi-ticker summary — "how many tickers are populated overall,
when was the last fetch" — a UX prerequisite for v0.1.3 multi-
strategy expansion.

## Scope (v0.1.0)

- Lock `eth-yahoo-2024-1d-sma-cross` as anchor row 70.
- Discharge H1 for ETH (Yahoo daily vs Binance hourly < 30% divergence).
- New widget `cache_state_summary_badge` next to the existing pill.
- Extend `run_yahoo_sma.rs` with `--ticker` flag (Q1 recommend extend).
- Preserve anchors 69 → 70 additive; zero existing rows touched.

## Out of scope

- 8 remaining unanchored crypto-mirror tickers (BNB → LINK).
- Multi-strategy on Yahoo (MACD / RSI / BBands) — v0.1.3.
- Click-to-drill from summary badge — display-only at v0.1.0.
- Per-ticker last-fetch timestamps in the summary — single newest
  mtime only; per-ticker fan-out is v0.2.0.
- Auto-refresh of cache (Q8=(b) stance from v0.1.0 unchanged).

## Architecture findings

- **F1 — Per-pair `cache_state_badge` already exists; v0.1.2 ships
  a SIBLING.** Current HEAD has the per-pair pill widget +
  `cache_state::probe` + `binance_to_yahoo_ticker_lookup` shipped at
  v0.1.0 Wave D-followup, wired at `screens/lab.rs:226` in the
  source-toggle row when `data_source = YahooCache`. v0.1.2 adds an
  additional chip in the SAME row (per-pair pill LEFT, summary RIGHT,
  separated by `space::S`); the new widget reuses
  `cache_state::probe` looped per ticker (max 10).
- **F2 — Q1 ticker handling: analyst recommends extend.** Two
  options for ETH support in `run_yahoo_sma.rs`: (a) extend with
  `--ticker` flag, default `BTC-USD` — LOC delta ~15, default
  invocation byte-identical to v0.1.1, scales DRY to 8 future
  tickers; (b) parallel `run_yahoo_sma_eth.rs` binary — +250 LOC,
  +2000 LOC of near-duplicates over the next 8 anchors, rejected by
  YAGNI/DRY. Anchor preservation (H3) verified via integration test
  asserting default-invocation BTC SHA `8045623b…`.
- **F3 — H1 ETH discharge: pre-flight passes.** Yahoo ETH-USD cache
  populated (12 parquets, REVISION.toml entries 31-42); Binance
  ETHUSDT 2024 reference exists. Expected divergence 5-20% (BTC was
  9.03%; ETH 2024 H1 had a similar bull-run shape $2.3k → $3.4k).
  K1 falsifier: if Binance ref missing/stale at M-DEV, route back
  with synthetic-comparison fallback (Yahoo ETH vs Yahoo BTC
  same-window).
- **F4 — Q2 placement: analyst recommends source-toggle row.**
  (a) co-located with the per-pair pill — cheap `Row::push`, same
  Lumen tokens, +140 px fits in 1280 px cockpit slack; (b) Lab tab
  toolbar — pulls eye away from source-toggle context; (c) bottom
  status bar — fights the activity-tape dynamism contract.
- **F5 — Q3 content: analyst recommends middle-ground.** (a)
  minimal `"Cache: 2 tickers"` — no freshness signal; (b) verbose
  `"BTC-USD 366b · ETH-USD 366b · last 2026-05-27 21:10 UTC"` —
  doesn't scale past 3; (c) middle-ground
  `"Cache: 2 tickers · last 2026-05-27"` — scales to 10, single
  timestamp captures freshness, matches v0.1.1 deck exemplar.

## Requirements

### R1 — Lock ETH-USD as anchor row 70

- **R1.1** New scenario `eth-yahoo-2024-1d-sma-cross` (2024 full year, daily, fast=20 / slow=50, seed 0xC0FFEE, 2 bps slip / 4 bps taker — mirrors BTC v0.1.1).
- **R1.2** Anchor row appended to `spec/anchors.toml` under namespace `lab-yahoo-realdata-v0.1.2`.
- **R1.3** Body-SHA determinism verified ≥ 2 independent re-runs (developer M-DEV + tester M-FINAL).
- **R1.4** `bash scripts/verify_anchors.sh` → `ANCHORS PASS (70 / 70)`.
- **Acceptance**: anchored report under `reports/`; SHA matches `scripts/hash_report.py`.

### R2 — H1 hypothesis discharge for ETH-USD

- **R2.1** Reproduce v0.1.1 H1 procedure on ETH: Yahoo daily H1 2024 vs Binance hourly H1 2024, same strategy/seed/fees.
- **R2.2** Threshold < 30%; expected 5-20%.
- **R2.3** Findings recorded in `dev-notes/yahoo-vs-binance-divergence-eth-2026-05-XX.md` mirroring v0.1.1 BTC dev-note shape.
- **Acceptance**: H1 PASS OR K1 fallback route.

### R3 — Cache-state summary badge widget (ui-designer scope)

- **R3.1** New widget `crates/ui/src/widgets/cache_state_summary_badge.rs` parallel to existing `cache_state_badge.rs`.
- **R3.2** Renders `"Cache: {N} tickers · last {YYYY-MM-DD}"` when N ≥ 1; renders existing `LAB_CACHE_STATE_EMPTY` when N = 0.
- **R3.3** Inputs: `count: usize`, `last_fetch: Option<SystemTime>`, `mode: ThemeMode`. Outputs: `Element<'static>`.
- **R3.4** Lumen tokens reused byte-identical to per-pair pill: `text::MICRO`, `radius::R3`, `space::XXS`/`S`, `PANEL_RAISED`, `BORDER_1`.
- **R3.5** Probe extension in `crates/ui/src/lab/cache_state.rs`: `probe_summary(cache_root, tickers, now) -> CacheSummary { populated_count, newest_mtime }`.
- **R3.6** Wired into `screens/lab.rs` source-toggle row AFTER the per-pair pill, gated on `data_source = YahooCache`.
- **Acceptance**: 4 gallery cells (`__empty`, `__one_ticker`, `__two_tickers`, `__ten_tickers`); UI unit tests for label, count, ISO date formats.

### R4 — `run_yahoo_sma.rs` extended with `--ticker` flag

- **R4.1** Add `--ticker <TICKER>` Clap arg, default `BTC-USD`.
- **R4.2** Scenario name derived from ticker (BTC-USD → `btc-yahoo-2024-1d-sma-cross`; ETH-USD → `eth-yahoo-2024-1d-sma-cross`).
- **R4.3** Validation against the 10-row crypto-mirror table; unknown ticker → exit 2 with actionable error.
- **R4.4** Default invocation (no `--ticker`) emits BTC body SHA `8045623b…` byte-identical to v0.1.1.
- **Acceptance**: integration test asserts both BTC (anchor 69) and ETH (anchor 70) SHAs.

### R5 — Non-regression contract

- **R5.1** Anchors 69 → 70 append-only; existing 69 rows byte-identical.
- **R5.2** Existing per-pair `cache_state_badge` unchanged.
- **R5.3** 1187+ workspace lib tests stay green.
- **R5.4** `data/yahoo/REVISION.toml` read-only at v0.1.2.

### R-NR — Cross-cutting

- **R-NR.1** Zero new design tokens.
- **R-NR.2** Exactly **1 new string** in `strings.rs`: `LAB_CACHE_STATE_SUMMARY_PREFIX = "Cache: "`. N=0 reuses existing `LAB_CACHE_STATE_EMPTY`. Count + date are dynamic (not string-table-eligible).
- **R-NR.3** `cargo fmt --check` + `clippy -D warnings` clean.
- **R-NR.4** `spec-lint` baseline 73/3 unchanged (no NEW categories).
- **R-NR.5** Default Lab UX byte-identical when `data_source = Synthetic` (H5 carries over).
- **R-NR.6** Phase F default-disabled byte-identity preserved.
- **R-NR.7** Idle-CPU ≤ 13.1%; summary probe < 1 ms; no background polling.

## Operator-decide Q-rows

- **Q1 — Ticker handling (LOAD-BEARING).** (a) extend `run_yahoo_sma.rs` with `--ticker` flag, default `BTC-USD`; (b) add `run_yahoo_sma_eth.rs` parallel binary. *Analyst-recommended: (a)* — see § F2.
- **Q2 — Summary badge placement.** (a) source-toggle row; (b) Lab toolbar; (c) bottom status bar. *Analyst-recommended: (a)* — see § F4.
- **Q3 — Summary badge content.** (a) minimal; (b) verbose per-ticker; (c) middle-ground `"Cache: 2 tickers · last 2026-05-27"`. *Analyst-recommended: (c)* — see § F5.

## Risks (K-rows / falsifiers)

- **K1 — Binance ETH-USDT reference data missing/stale.** H1
  cannot be discharged mechanically. *Mitigation*: developer pre-
  flight at M-DEV; if missing, route back to analyst with
  operator-decide on synthetic-comparison fallback.
- **K2 — Body-SHA non-determinism on ticker swap.** *Mitigation*:
  developer re-runs ×3 at M-DEV, tester re-runs ×2 at M-FINAL.
- **K3 — Aggregate probe N+1 filesystem stat budget.** 10-ticker
  probe = up to 30 directory stats per render. *Mitigation*:
  cache `(count, newest_mtime)` summary in `LabState`; refresh on
  coarse cadence (Lab-Run-complete event or `data_source` toggle);
  no per-frame recompute. Architect-decide at M-T1.
- **K4 — Source-toggle row horizontal overflow.** Row gains a third
  chip (~140 px wider). *Mitigation*: ui-designer verifies layout
  at 1280 / 1024 / 960 px breakpoints at M-DEV-UI.

## Hypotheses (H-rows)

- **H1** — ETH Yahoo daily vs Binance hourly H1 2024 equity
  divergence < 30%. Falsifier: ≥ 30% → K1.
- **H2** — Body-SHA stable across ≥ 2 independent re-runs of
  `--ticker ETH-USD`. Falsifier: drift → K2.
- **H3 (anchor-preserving)** — Default invocation (no `--ticker`)
  emits BTC body SHA `8045623b…` byte-identical. Falsifier: drift
  → revert Q1=(a), reconsider Q1=(b).
- **H4** — Summary-badge probe latency < 5 ms for 10-ticker cache.
  Falsifier: ≥ 5 ms → switch to cached-summary state field (K3).

## Cost framing

| Phase | Owner | Estimate |
|---|---|---:|
| M0 — analyst (this brief) | analyst | 20-30 min |
| M-OD — operator decide on Q1/Q2/Q3 | operator | < 5 min |
| M-T1 — architect ratifies + decomposes | architect | 30-45 min |
| M-DEV — backend (`--ticker` extension + ETH anchor + H1) | developer | 60-90 min |
| M-DEV-UI — summary badge widget + wiring + gallery | ui-designer | 60-90 min |
| M-FINAL — tester gates | tester | 30-45 min |
| M-PRESENTER — sprint-review deck | presenter | 30-45 min |
| **Total wall-clock** | — | **~4-6 hours** |

Dev + ui-designer run in parallel per AGENT.md § Parallelism —
backend touches `crates/backtest/`, `spec/anchors.toml`, dev-note
dir; UI touches `crates/ui/src/widgets/`, `crates/ui/src/strings.rs`,
`crates/ui/src/screens/lab.rs`, `crates/ui/src/lab/cache_state.rs`,
gallery routes. Zero file overlap.

## Verdict tree (pre-drawn 2-cell)

```
       M-FINAL tester gates
              │
      ┌───────┴───────┐
   ALL GREEN       ANY RED
      │              │
   PASS         REGRESSION
      │              │
   presenter →   route back to
   operator →    analyst with
   ship          K1/K2/K3/K4
```

ALL GREEN gates: anchors 70/70 + `cargo fmt --check` + `cargo clippy
-D warnings` + workspace lib tests green + UI gallery snapshots green
+ spec-lint no NEW categories + H1/H2/H3/H4 PASS.

## References

- v0.1.1 brief, deck, BTC H1 dev-note (pattern to replicate for ETH):
  [`spec/lab-yahoo-realdata/feature.md`](../lab-yahoo-realdata/feature.md),
  [`presentations/lab-yahoo-realdata-v0.1.1-2026-05-27.md`](../lab-yahoo-realdata/presentations/lab-yahoo-realdata-v0.1.1-2026-05-27.md),
  [`dev-notes/yahoo-vs-binance-divergence-2026-05-27.md`](../lab-yahoo-realdata/dev-notes/yahoo-vs-binance-divergence-2026-05-27.md).
- Binary to extend: [`crates/backtest/src/bin/run_yahoo_sma.rs`](../../crates/backtest/src/bin/run_yahoo_sma.rs).
- Existing per-pair pill + probe + wiring:
  [`crates/ui/src/widgets/cache_state_badge.rs`](../../crates/ui/src/widgets/cache_state_badge.rs),
  [`crates/ui/src/lab/cache_state.rs`](../../crates/ui/src/lab/cache_state.rs),
  [`crates/ui/src/screens/lab.rs`](../../crates/ui/src/screens/lab.rs).
- Cache state + anchors registry + ADR-0040:
  [`data/yahoo/REVISION.toml`](../../data/yahoo/REVISION.toml),
  [`spec/anchors.toml`](../anchors.toml),
  [`spec/architecture/adr/0040-yahoo-realdata-path.md`](../architecture/adr/0040-yahoo-realdata-path.md).

## Changelog

- 2026-05-27 (analyst): M0 brief — operator multi-select option C
  (ETH-USD anchor + cache-state summary badge); 5 R / 4 K / 4 H /
  3 Q; analyst-recommended defaults logged; cost ~4-6 hours; trace
  row REQ-LAB-YAHOO-REALDATA-V0-1-2-001 opened `proposed`; backlog
  Active row appended.
