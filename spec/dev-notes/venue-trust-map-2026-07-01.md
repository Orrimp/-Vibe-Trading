---
title: Crypto Venue Trust Map
date: 2026-07-01
author: developer
status: reference
feature: advisor-cost-model-opt-in (P1-6)
---

# Crypto Venue Trust Map — P1-6 Reference

> **Display-only.** This is an operator reference document; it does NOT
> change any cost model, gate band, or backtest path. Code changes live in
> `crates/cost/src/slippage.rs` (ADR-0081). This document records the
> research-grounded venue/metric trust policy for human review and narration.

---

## Summary table

| Metric | Trust | Trusted venues | Do NOT use |
|--------|-------|----------------|------------|
| **Spot price** | HIGH (with caveats) | Binance (price-discovery leader cross-venue), Coinbase, Kraken | Unregulated DEX-only, cross-country premium feeds (Kimchi = capital controls, not value) |
| **Open interest (OI)** | LOW except Kraken/HTX | Kraken, HTX | **Bybit** (irreconcilable on every sub-period), **OKX** (large misreporting), **Binance inverse perps** (worsened in 2023), all aggregated OI feeds |
| **Volume** | MEDIUM (Binance), LOW (others) | Binance (relatively honest on volume) | Unregulated CEXs (>70% wash-traded), DEXs (>30% wash-traded on IDEX/EtherDelta) |
| **Order-book depth / imbalance** | LOW universally | N/A — all venues spoofable | Any venue; ~31% of large Coinbase BTC/ETH orders could be spoofing |

---

## Source citations

### Open interest fabrication

Research `crypto-market-structure[91]` applies the hard accounting identity
`|ΔOI| ≤ traded_volume` to seven exchanges over 2023. Findings:

- **Bybit:** irreconcilable on *any* sub-period (wrong every day, ~99% of hours,
  >70% of minutes). Implied missing volume: $156–213B, greater than Binance's
  actual volume.
- **OKX:** large systematic misreporting.
- **Binance inverse perps:** OI misreporting appearing and worsening in 2023.
- **HTX (Huobi):** reconcilable on essentially every sub-period.
- **Kraken:** reconcilable; a trustworthy OI source.
- **BitMEX:** reconcilable to a degree.

**Advisory:** for any signal or overlay that uses OI, source only from Kraken/HTX.
Never use Bybit, OKX, or Binance-inverse OI. Never use aggregated OI feeds
(aggregators propagate the fabricated numbers).

### Volume fabrication

Research `crypto-market-structure[19]` shows:
- >70% of reported volume on unregulated CEXs is wash trading (detectable via
  Benford's Law, trade-size roundness, and power-law tail regularities).
- Regulated exchanges match the authentic-trading fingerprints; unregulated ones do not.
- **Binance** passes the regulatory fingerprint test (relatively honest on volume).

**Advisory:** treat volume from unregulated CEXs as unreliable. Do not build
volume-confirmation signals (e.g. OBV) on unregulated-venue data. Binance volume
is the most defensible source for our backtest corpus.

### Price discovery leadership

Research `crypto-market-structure[42][69]`:
- **CME futures lead BTC spot** price discovery.
- **Binance and Huobi lead** cross-venue price formation (CEX).
- CEX leads DEX (one-way, zero reverse causality).
- Cross-venue deviations stay inside fee-defined no-arb bands and mean-revert fast —
  not a retail edge.

**Advisory:** anchor price on a deep major-venue USD/USDT feed (Binance/Coinbase/Kraken).
We trade CEX spot, so we lag CME slightly — this is acceptable and cannot be exploited.

### Order-book depth / imbalance spoofing

Research `crypto-market-structure[90]`:
- ~31% of large Coinbase BTC/ETH orders could be spoofing (posting *distance* matters
  as much as size).
- Spoofing/layering was rife around the LUNA crash.
- Statistical-physics detection outperforms Z-score anomaly detection.

**Advisory:** never assume mid-price fills based on visible depth. Visible order-book
depth overstates true liquidity. This is the empirical justification for the cost-model
widening in `SlippageModel::VolScaledSpread`.

---

## Implications for the cost model (what P1-6 implements)

| Research finding | Implementation consequence |
|------------------|---------------------------|
| Spreads widen 2–3× in high-vol / stress regimes `backtesting[47]` | `VolScaledSpread.vol_multiplier = 2.0` (midpoint of 2–3× range) |
| Visible depth is partly spoofable → mid-price fills too optimistic `crypto-market-structure[90]` | `base_bps = 8` (same as Linear; a higher-than-zero floor is already in place) |
| OI on Bybit/OKX/Binance-inverse is fabricated `crypto-market-structure[91]` | Do not build OI-dependent signals without cross-checking against Kraken/HTX |
| Volume on unregulated CEXs >70% fake `crypto-market-structure[19]` | OBV arm (`v0.obv`) uses Binance volume from the pinned corpus — acceptable; do not extend to unregulated venues |
| Liquidation cascades blow slippage 5–10× `crypto-market-structure[33][37]` | `MAX_SLIPPAGE_BPS = 1_000` (10%) cap covers cascade-regime tail in a backtest context |

---

## Periodicity note

Fabrication is time-varying and deteriorating (OI misreporting *worsened* over 2023,
spread to new venues). This trust map reflects the 2023/2024 research state. Review
annually or when a new venue is added to the universe.

---

## What is NOT in scope (and why)

- **Order-book-imbalance / depth-based overlays:** depth is ~31% spoofable and
  sub-second microstructure edges die on costs at our daily horizon — explicitly
  `spec/research §8 G: DEAD END / DO-NOT-BUILD`.
- **Stablecoin peg stress monitor:** tail event, rarely binds; deferred to P3 as
  an optional exogenous-arm probe (`spec/v2/v2-architecture.md §1 P1-7`).
- **Cross-country premium trades (Kimchi etc.):** measure capital controls, not value.
  Out of scope.
- **Aggregated OI feeds:** all propagate fabricated numbers — do not use.
