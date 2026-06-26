---
slug: advisor-crossasset-macro-regime
status: proposed
owner: analyst
version: 0.1.0
updated: 2026-06-26
---

# Cross-Asset / Macro Regime Probe — fresh untested channel into the bake-off

## Why

The Single-Coin Investment Advisor ranks strategies on a `(coin, window)` under
a FROZEN robustness gate (`classify_verdict` / the 5-signal weakest-link
bootstrap; FRAGILE ⇒ ineligible to crown), with buy-and-hold always the
benchmark. The active-edge research **concluded 2026-06-08 ("ship passive")**
across three reachable channels — price/OHLCV, derivatives-positioning,
on-chain — none beat passive net of cost under the frozen rule.

**Cross-asset / macro is the one named-but-UNTESTED orthogonal channel.**
`spec/backlog.md` § "Future fresh program" (line 140-142) lists it verbatim:

> **Untested orthogonal channels** — options/implied-vol (Deribit DVOL),
> cross-asset/macro (DXY, rates, SPX), social/sentiment. … each would be a
> **fresh** program with its own data adapter and backtest, not a re-open of
> this one.

This feature is the honest **"we also checked the macro channel"** — the
deliverable is **honest coverage of an untested channel, NOT an alpha claim**.

> **Honest framing (load-bearing, not boilerplate).** The prior is that a
> macro-regime arm is **also Fragile / does not beat hold** net of cost under
> the frozen gate. A **null result ("the macro-regime arm is also Fragile, hold
> still stands") is the EXPECTED, valid, shippable outcome.** The probe is
> scored by the IDENTICAL frozen gate + buy-and-hold benchmark as every other
> arm — no special-casing, no band relaxation. We ship the honest coverage
> either way; a positive result would be a surprise requiring out-of-sample
> confirmation before any promotion.

This is operator-directed **fresh-channel probe #4** (cross-asset/macro), a
sibling of the #1–#3 channels already concluded.

---

## Data feasibility — THE crux (assessed first, brutally honest)

**Verdict: the Yahoo *fetch path* trivially supplies the macro tickers, but
three real seams sit between "fetch DXY" and "a macro-regime bake-off arm". The
probe is NOT cheap-and-free; it is a small-but-real feature, ~3 net-new pieces.**

### F-1 — Can the Yahoo adapter fetch DXY / SPX / rates / gold? YES, mechanically free.

The fetch CLI accepts **any** valid Yahoo ticker. There is **no allow-list on
the fetch path**:

- `crates/data/src/bin/fetch_yahoo_klines.rs:48-50` — *"Comma-separated Yahoo
  tickers … v0.1.0 accepts **any valid Yahoo Finance ticker symbol**."*
- The URL builder (`fetch_yahoo_klines.rs:199-205`) interpolates the ticker
  straight into `…/v8/finance/chart/{ticker}?…` — so `^GSPC`, `^TNX`,
  `DX-Y.NYB` (or `^DXY`), `GC=F` are one CLI invocation each, **zero code
  change** to fetch + pin.
- `YahooBarSource::load_cached` (`crates/data/src/yahoo.rs:266`) is
  ticker-agnostic and interval-parameterized (`1d`/`1h`/`1m`); the corpus
  already carries `1d` for 12 crypto pairs (`data/yahoo/<TICKER>/1d/<YEAR>/<MONTH>.parquet`).
- **PIT-honest + immutable**: every parquet is SHA-256-pinned in
  `data/yahoo/REVISION.toml`; `load_cached` re-verifies per-file SHA against the
  manifest on every read (`yahoo.rs:308-324`) and forces `local_recv_ts =
  close_ts` for determinism (`yahoo.rs:354-357`). Daily OHLC, no intraday
  look-ahead within a bar. **This satisfies the PIT/immutability bar.**

So: fetching + pinning the macro series is genuinely free. The cost is
*downstream* of the fetch.

### F-2 — The 95% coverage gate ASSUMES 24/7 data; equities/rates/FX are ~5/7. (BLOCKER if naïve.)

`yahoo.rs:982-984` is explicit: *"Yahoo crypto runs 24/7; **equities are
handled at v0.2.0 with a market-calendar layer.**"* `expected_bars_for_range`
(`yahoo.rs:984-992`) is a pure wall-clock division — for a `1d` window it
computes `expected = calendar_days`. The coverage gate
(`MISSING_DATA_THRESHOLD_PCT = 95`, `yahoo.rs:56`, enforced at
`yahoo.rs:338-352`) then demands ≥ 95% of *calendar* days have a bar.

**SPX / ^TNX / DXY trade ~5 of 7 calendar days (~71%) plus holidays.** A 90-day
`1d` load of `^GSPC` would compute `expected = 90`, find ~62-64 trading-day
bars, and **fail `YahooError::MissingData` (≈70% < 95%)**. This is a hard,
code-confirmed obstacle. → see **Q-MACRO-2**; the most likely answer is a
**market-calendar-aware expected-count** (the deferred "v0.2.0 market-calendar
layer" the code names) OR loading macro at `1d` through a relaxed/by-pass
coverage path dedicated to non-24/7 tickers. This is the single biggest hidden
cost and it sizes the feature.

### F-3 — The macro series (Yahoo) and the coin series (Binance) live in DIFFERENT corpora. (The join is cross-adapter.)

The advisor bake-off does **NOT** read Yahoo for the coin — it reads **Binance**:

- `crates/ui/src/leaderboard/runner.rs:108` & `:144` —
  `data_source: ScenarioDataSource::BinanceCache` for both the default and the
  F3 guided-input config. (Yahoo is *Lab-only at v0.1.0* —
  `crates/backtest/src/engine.rs:166`.)
- The coin's hourly bars are preloaded once from `data/binance` via
  `ReplayFeed::merge_symbols` (`crates/backtest/src/bakeoff/mod.rs:93-102`) and
  passed to every arm as `bars_override` (`bakeoff/mod.rs:718`).

So the join is **coin = Binance hourly ‖ macro = Yahoo daily** — two adapters,
two corpora, two cadences. `merge_symbols` (`replay_feed.rs:281`) reads from a
**single `parquet_root`** and cannot merge a Binance coin with a Yahoo ticker;
the cross-corpus + cross-cadence join is net-new plumbing (see § Seam).

### As-of daily→hourly join (no look-ahead) — the resample seam

The macro signal is **daily**, the coin bake-off is **hourly** (or
H4/D1 via the resample knob, `bakeoff/mod.rs:668-669`). The regime arm must
align the daily macro level to each hourly coin bar **without look-ahead**:

- **As-of / LOCF rule (pre-registered):** for an hourly coin bar opening at time
  `t`, the macro regime uses the **most-recent macro DAILY bar whose `close_ts ≤
  t`** (last-observation-carried-forward). A macro daily bar dated `D` (UTC
  close) becomes visible **only at/after `D`'s close** — never to coin bars
  earlier that day. This is the same determinism discipline as
  `local_recv_ts = close_ts` already enforced on Yahoo bars (`yahoo.rs:354`).
- **Concrete seam:** carry `last_macro_close[ticker]` forward; recompute the
  regime boolean only when a new macro daily close arrives; hold it constant
  across the ~24 intervening hourly coin bars. This is a "last value carried
  forward" gate inside the arm's `on_bar`, identical in spirit to how
  `RegimeDispatcher` carries regime state across bars
  (`crates/strategy/src/regime_dispatcher.rs:33-37`).
- **Weekend/holiday gap:** when no new macro bar arrives (weekend), the last
  Friday close carries through Sat/Sun crypto bars — correct and look-ahead-free
  by construction.

---

## The hypothesis + the pre-registered signal (FIXED — no search, no tuning)

**Hypothesis.** A coin returns conditioned on a cross-asset *risk regime* —
long the coin only in a risk-ON macro regime, else flat/cash — is structurally
**orthogonal** to the coin's own price/volume signals (the existing arms).

**Decorrelation rationale (why this is a genuinely fresh channel).** Every
existing arm derives its signal from the **coin's own OHLCV**: the 4 base rule
engines + the ADR-0071 signal-library arms (`bakeoff/mod.rs:363-377`), the
ensembles (`bakeoff/mod.rs:440-454`), and even `RegimeDispatcher`, whose regime
classifier is **fit on the coin's own log-returns** (`regime_dispatcher.rs:33-37`).
**None** consume an exogenous (non-coin) series. A macro risk-regime is computed
from DXY/SPX/rates — a different information set — so its decision variable is
structurally decorrelated from the existing field. That orthogonality is the
*only* a-priori reason a macro arm could add anything; it is also why a null
result is informative (it closes the channel honestly).

**Pre-registered signal (locked literals — the architect/operator may swap the
exact rule via Q-MACRO-3, but it is FIXED before any results are read — the
overfit-safe discipline mirroring the ADR-0071 / ADR-0067 pre-registered slates).**

> **`v0.macro_riskon`** — long the coin **only** when the macro regime is
> risk-ON, else flat (cash). Risk-ON ≙ ALL of:
> 1. **SPX trend up:** `^GSPC close > SMA(^GSPC close, 50 daily bars)`, AND
> 2. **Dollar not bid:** `DX-Y.NYB close < SMA(DX-Y.NYB close, 50 daily bars)`, AND
> 3. **Rates not spiking:** `^TNX close < SMA(^TNX close, 20 daily bars)`.
>
> When risk-ON → emit the coin's buy/hold-long signal (a passive long while the
> regime holds); when risk-OFF → flat (no position / exit to cash). All
> thresholds are SMA-relative (no magic price levels), all lookbacks are fixed
> literals. **This is one arm.** Whether to also register a small slate (e.g. a
> SPX-only `v0.macro_spx_trend` and a 2-of-3 `v0.macro_riskon_2of3` ablation to
> measure each leg's contribution) is **Q-MACRO-4**.

The regime gate is a **pure long/flat overlay on buy-and-hold** — it never
trades on the coin's own indicators, isolating the macro channel's contribution
cleanly against the buy-and-hold benchmark.

---

## The seam — how it becomes a bake-off arm (engineering size, honest)

**This needs a new "exogenous regime series" seam. It does NOT cleanly reuse the
multi-symbol (pairs/cross_sectional) merged-stream path.** Reasoning, grounded:

1. **The multi-symbol mechanism is "merge symbols into ONE time-ordered stream;
   the strategy demuxes by `bar.symbol`."** `merge_symbols`
   (`replay_feed.rs:281-322`) k-way-merges multiple symbols' bars sorted by
   `(open_ts, symbol)`. The pairs strategy works under this because it
   **explicitly universe-filters** `bar.symbol` and returns `vec![]` for
   out-of-universe bars (`crates/strategy/src/pairs/mean_reversion.rs:9-12`),
   maintaining per-symbol state internally.

2. **A `ComposedStrategy`-style arm has NO such demux.** `ComposedStrategy::on_bar`
   (`crates/strategy/src/composed/node.rs:1395`) processes **every** bar handed
   to it and emits a signal stamped with the **incoming** `bar.symbol`
   (`node.rs:1374-1381`). Feed it a merged coin‖macro stream and it would try to
   **trade the macro ticker as if it were a coin** — wrong. So a composed-style
   macro-regime arm cannot just ride the merged stream; it needs the macro as a
   **side input**, not as tradeable bars.

3. **Cross-corpus + cross-cadence:** even setting (2) aside, `merge_symbols`
   reads a single corpus root; the coin is Binance-hourly and the macro is
   Yahoo-daily (§ F-3), so there is no existing call that merges them.

**Realistic seam (the net-new plumbing the architect must size):**

- **(a) An exogenous-series loader** that, for a `(macro_ticker, window)`,
  loads the Yahoo `1d` bars (reusing `YahooBarSource::load_cached`,
  `yahoo.rs:266` — no change to the read path) **subject to the F-2 coverage
  fix**, and produces a **forward-filled daily regime series** keyed by close
  timestamp.
- **(b) A regime-overlay arm** — either (i) a hand-written `Strategy` impl that
  holds the precomputed as-of regime series + the coin bars and emits long/flat,
  or (ii) a `ComposedStrategy` extension that can read a named **auxiliary
  series** (a new "exogenous input" the signal DSL can reference). **(i) is the
  smaller, lower-blast-radius build; (ii) is the more composable long-term seam
  but is a real DSL/typecheck change.** → **Q-MACRO-1**.
- **(c) Arm registration + the bake-off threading** of the macro series into
  that one arm only. Today `ScenarioConfig.bars_override` is a **single**
  `Option<Vec<Bar>>` (`engine.rs:232`); the macro series needs a **second,
  optional, arm-scoped channel** (e.g. `macro_regime_series: Option<...>` on the
  arm's config, ignored by every existing arm → byte-identical for them). The
  bake-off loop builds the per-arm `ScenarioConfig` at `bakeoff/mod.rs:707-730`;
  this is the insertion point.

**Honest size estimate (for the architect to confirm/refute):** **medium, not
small.** Net-new: the F-2 coverage fix (calendar-aware OR a relaxed non-24/7
path), the cross-corpus exogenous loader + as-of/LOCF join, the regime-overlay
arm, the second optional config channel, the arm registration, and the
day-1 divergence test. Reuse carries the rest (see matrix). This is **not** a
"fetch a ticker and add a line" feature — the 24/7-coverage assumption (F-2) and
the absence of any exogenous-series seam (seam-2) are the two things that make
it real. **If the operator wants the cheapest viable probe**, the
if-budget-tightens fallback is § "Minimum-viable carve-out" below.

---

## Reuse vs net-new (explicit)

| Reused (no change)                                                         | Net-new (this feature)                                                                 |
|---------------------------------------------------------------------------|----------------------------------------------------------------------------------------|
| The bake-off loop + ranking (`run_bakeoff`, `rank_candidates`) — `bakeoff/mod.rs:636`, `bakeoff/rank.rs` | The **macro-series fetch + pin** (CLI invocations for `^GSPC`/`^TNX`/`DX-Y.NYB`; new `data/yahoo/<TICKER>/1d/…`) |
| The FROZEN robustness gate + bands + buy-and-hold benchmark (NOT touched) — `bakeoff/robustness.rs`, `bakeoff/buyhold.rs` | The **F-2 coverage fix** for non-24/7 daily series (market-calendar expected-count OR relaxed path) |
| `YahooBarSource::load_cached` read path + REVISION SHA verify — `yahoo.rs:266`/`:308` | The **cross-corpus exogenous loader** (Yahoo-daily macro ‖ Binance-hourly coin)        |
| The Yahoo fetch CLI (any ticker, no allow-list) — `fetch_yahoo_klines.rs:48` | The **as-of daily→hourly LOCF join** (look-ahead-free regime alignment)                 |
| The per-arm `ScenarioConfig` build seam — `bakeoff/mod.rs:707-730`        | The **`v0.macro_riskon` regime-overlay arm** (long/flat over buy-and-hold)              |
| `write_report = false` anchor-safe arm convention — `bakeoff/mod.rs:712`  | The **second optional arm-scoped config channel** for the macro series (`engine.rs` `ScenarioConfig`) |
| The merged-stream multi-symbol *primitive* `merge_symbols` (as a reference, NOT a direct reuse) — `replay_feed.rs:281` | The **day-1 baseline-equity-divergence e2e test** (CLAUDE.md non-negotiable)            |

---

## Anchor safety + frozen gate (NON-NEGOTIABLE)

- **Anchors: 119/119 before AND after.** Baseline confirmed this scoping pass:
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)`.
- The new arm runs with **`write_report = false`** (the established advisor-arm
  convention — `bakeoff/mod.rs:712`, ADR-0059), so **no report body is written**
  → anchor-additive by construction. Existing arms + their anchored
  `spec/*/reports/` files stay **byte-identical**.
- **The gate, bands, and benchmark stay FROZEN.** This is **NOT** a band
  proposal and **NOT** a `classify_verdict` change. The macro arm is scored by
  the identical 5-signal weakest-link bootstrap as every other arm; FRAGILE ⇒
  ineligible to crown, exactly as today.
- No edits to any anchored report file. Any incidental doc-link touch near
  `spec/*/reports/` must obey the ADR-0038 § D6 immutability contract.

---

## Day-1 divergence gate (CLAUDE.md non-negotiable — REQUIRED)

Per CLAUDE.md ("Every strategy overlay or sizing-modifier ships with a
baseline-equity-divergence end-to-end test from day 1") and the
`v3-volatility-forecaster-noop-fix` precedent: the macro-regime arm **IS an
overlay** (it gates buy-and-hold long/flat on an exogenous signal). A no-op bug
— where the regime boolean is computed but never actually suppresses the
position — is exactly the failure class that unit tests + anchored reports do
**not** catch.

**Required e2e (pattern reference:
`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`):** construct a
window + a synthetic/fixture macro series in which the pre-registered regime
**flips** at least once (a risk-OFF stretch), run `v0.macro_riskon` vs the
plain buy-and-hold baseline on the SAME coin bars, and **assert the macro arm's
output equity diverges from the un-gated buy-and-hold equity by ≥ 1 bp** across
the risk-OFF stretch (the arm goes flat → its equity must depart from
always-long). A negative control (a regime series pinned risk-ON for the whole
window) must produce **≈ buy-and-hold equity** (overlay correctly no-ops when the
regime never gates). Both directions are mandatory — the divergence test AND the
no-op-when-always-on control.

---

## Open decisions (for the architect / operator)

- **Q-MACRO-1 (seam — the load-bearing build decision).** Implement the arm as
  **(a)** a dedicated hand-written `Strategy` impl carrying a precomputed as-of
  regime series _(Recommended — smaller blast radius, no DSL change, ships the
  honest probe fastest while still being a real exogenous-series seam)_ **— but
  with the architect explicitly noting whether a second macro arm is foreseen
  (if yes, prefer (b))**; or **(b)** extend the `ComposedStrategy` DSL with a
  named **auxiliary/exogenous series** the signal grammar can reference (more
  composable for *future* macro/exogenous arms, but a real
  parser/typecheck/`node.rs` change + its own tests). **Durable-vs-quick note:**
  (b) is the more durable seam **iff** the operator expects ≥ 2 future exogenous
  arms (options-IV, more macro legs); if this is a one-shot honest-coverage
  probe (the stated intent), (a) is correct and does **not** spawn a v0.2.0
  cleanup. Architect to confirm which world we are in.
- **Q-MACRO-2 (the F-2 coverage BLOCKER).** How to load non-24/7 daily macro
  series past the 95% calendar-day coverage gate (`yahoo.rs:56`/`:338`):
  **(a)** add the market-calendar-aware `expected_bars_for_range` the code
  already names as the "v0.2.0 market-calendar layer" (`yahoo.rs:982-984`) —
  durable, unblocks all future equities/FX work; **(b)** a dedicated relaxed /
  bypass coverage path for macro tickers only — cheaper, carries a v0.2.0
  cleanup commitment. _(Recommended: (a) — it is the durable unblock the code
  itself flags as the right home, and equities expansion is already on the
  roadmap.)_ This is the single biggest cost driver — architect must size it
  first.
- **Q-MACRO-3 (the exact macro series + the pre-registered rule).** Confirm the
  ticker set (`^GSPC` + `DX-Y.NYB`/`^DXY` + `^TNX`; include `GC=F` gold?) and
  the locked regime literals (SMA(50)/SMA(50)/SMA(20); the 3-condition AND).
  Any change MUST be locked **before** results are read (pre-registration
  discipline). Note `DX-Y.NYB` vs `^DXY` Yahoo-symbol availability needs a quick
  fetch-dry-run check.
- **Q-MACRO-4 (one arm or a small slate?).** Ship only `v0.macro_riskon`, or
  also register a SPX-only `v0.macro_spx_trend` + a 2-of-3 ablation
  `v0.macro_riskon_2of3` to attribute each leg? A 3-arm pre-registered slate
  gives a cleaner "which macro leg (if any) mattered" read at marginal extra
  cost (all `write_report = false`, all gate-scored). Default if undecided:
  **the single `v0.macro_riskon` arm** (minimum honest coverage), expand only on
  operator request.
- **Q-MACRO-5 (window/cadence).** The advisor default is hourly (H4/D1 via the
  resample knob). Confirm the as-of join holds the daily macro level across the
  resampled coarser bars identically (it should — LOCF is cadence-agnostic), and
  that the divergence test exercises the hourly path (the strictest as-of case).

### Minimum-viable carve-out (if-budget-tightens fallback)

If the operator wants the **cheapest** honest probe rather than the durable
build: implement **Q-MACRO-1 (a)** + **Q-MACRO-2 (b)** (hand-written arm +
macro-only relaxed coverage path), ship the single `v0.macro_riskon` arm, and
defer the market-calendar layer (Q-MACRO-2 (a)) + the DSL exogenous-series seam
(Q-MACRO-1 (b)) to a v0.2.0 follow-on. This still delivers the honest
"we checked the macro channel" coverage and the day-1 divergence gate, at the
cost of a documented v0.2.0 cleanup commitment (the relaxed coverage path is a
carve-out, not the durable market-calendar fix). **This is the fallback label,
not the Recommended one** — the Recommended path is the durable
Q-MACRO-1(a)+Q-MACRO-2(a) combination.

---

## Requirements (testable)

- **R1** — Fetch + pin the pre-registered macro tickers (Q-MACRO-3) into
  `data/yahoo/<TICKER>/1d/…` with REVISION SHA entries; PIT-immutable, verified
  by `YahooBarSource::load_cached`'s per-file SHA check.
- **R2** — Resolve the F-2 coverage gate for non-24/7 daily series (Q-MACRO-2)
  so a multi-month `1d` macro load succeeds (does NOT trip `YahooError::MissingData`).
- **R3** — Implement the as-of daily→hourly LOCF join: an hourly coin bar at `t`
  sees only the most-recent macro daily bar with `close_ts ≤ t`; no look-ahead;
  weekend gaps carry the prior close forward.
- **R4** — Register the `v0.macro_riskon` arm (Q-MACRO-1) into the bake-off with
  `write_report = false`; it is scored by the FROZEN gate + buy-and-hold
  benchmark identically to every other arm.
- **R5** — Day-1 baseline-equity-divergence e2e test: regime-flip window ⇒
  macro-arm equity diverges from un-gated buy-and-hold by ≥ 1 bp; always-risk-ON
  control ⇒ ≈ buy-and-hold (no-op verified). Both directions mandatory.
- **R6** — `bash scripts/verify_anchors.sh` → 119/119 before AND after; no
  anchored report file mutated.
- **R7** — Honest-coverage acceptance: the feature ships its result whether the
  macro arm is FRAGILE (expected) or not; the UI/rationale honesty branches
  (`BenchmarkWins` / `AllFragile`) require **no** change — the macro arm flows
  through them as-is.

## Design
_architect fills this — start at Q-MACRO-2 (the coverage blocker) and Q-MACRO-1
(the seam), they dominate the size._

## Backtest Scenarios
_analyst + architect fill this; the day-1 divergence + no-op-control scenarios
(R5) are the gating fixtures — model them on
`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`._

## Implementation
_developer fills this._

## Verification
_tester links to reports here._

## Changelog
- 2026-06-26 (analyst): scoped fresh-channel probe #4 (cross-asset/macro
  regime). Data-feasibility verdict: Yahoo *fetch* path supplies DXY/SPX/rates
  free (any-ticker, PIT-pinned), but (F-2) the 95% coverage gate assumes 24/7
  data and BLOCKS non-24/7 daily macro loads, and (F-3 / seam) macro=Yahoo-daily
  vs coin=Binance-hourly is a cross-corpus + cross-cadence join with NO existing
  exogenous-series seam (ComposedStrategy has no demux; merge_symbols is single-
  corpus) → medium-size feature, not a one-liner. Pre-registered
  `v0.macro_riskon` long/flat overlay. Anchors 119/119 baseline. Open decisions
  Q-MACRO-1..5. feature.md only; trace.toml/product.md left for orchestrator.
