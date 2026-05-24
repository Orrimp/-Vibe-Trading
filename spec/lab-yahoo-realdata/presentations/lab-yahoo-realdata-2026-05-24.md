---
slug: lab-yahoo-realdata
status: in-progress
owner: presenter
updated: 2026-05-24
mode: release
feature_version: 0.1.0
commits: [7ab924e, 04e059f, a87bbc4]
test_report: ../reports/test-final-2026-05-24.md
adr: ../../architecture/adr/0040-yahoo-realdata-path.md
---

# Lab Yahoo realdata — v0.1.0 sprint review (2026-05-24)

## TL;DR

The Lab now reads real Yahoo historical bars instead of synthetic GBM
when you flip the new **Source = YahooCache** toggle. Pair picker
re-populates with 10 Binance-style crypto tickers (`BTCUSDT` …
`LINKUSDT`); cadence auto-adapts to range (1m ≤7d, 1h 7-60d, 1d
>60d); the equity curve + buy/sell markers render against real
Yahoo bars via the same `engine::run_scenario` arms used today. All
34 anchored Binance reports stay byte-identical (zero new anchors at
v0.1.0). Two hypotheses (H1 divergence ≤30% vs Binance, H2 fetch
success ≥95%) are deferred to v0.1.1 until a live cache exists.

## Before / after — Lab UX

**Before (synthetic only):**
- Lab top-bar has no source toggle. Strategy + pair + range run
  against `synthetic_bars_minute(symbol, count, seed, …)` —
  `ChaCha20Rng` per-pair seed; same shape for every run.
- Cadence is implicit (1m bars, count derived from range).
- Pair list = `XRP_FIRST_UNIVERSE` (10 crypto USDT pairs, Binance).

**After (Wave C-3, commit `a87bbc4`):**
- Lab top-bar shows two chips: **Synthetic** (default, active = `ACCENT`
  token) and **YahooCache** (inactive = `SURFACE_2`). One click swaps
  the source — no rebuild (`yahoo` feature is a runtime state field,
  not a compile-time gate at the UI layer).
- A **cadence badge** chip (`1m` / `1h` / `1d`) appears next to the
  source toggle, derived from the selected range via
  `CadenceLabel::derive_from_range`. Boundary table: 1m ≤7d, 1h
  7-60d, 1d >60d.
- When `YahooCache` is active, the pair chip row switches to
  `YAHOO_CRYPTO_UNIVERSE` (10 entries, XRP-first, same Binance-style
  spellings — `BTCUSDT`, `ETHUSDT`, `BNBUSDT`, `XRPUSDT`, `ADAUSDT`,
  `SOLUSDT`, `DOGEUSDT`, `DOTUSDT`, `MATICUSDT`, `LINKUSDT`).
- The strategy chip row filters to `SINGLE_SYMBOL_STRATEGIES`
  (`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`); cross-sectional
  strategies are hidden under YahooCache to prevent
  `RunError::UnsupportedDataSource`.
- Clicking **Run** dispatches through `lab_config_to_scenario`:
  `BTCUSDT` is converted to Yahoo's `BTC-USD` at the boundary,
  cached parquet is read from
  `data/yahoo/<TICKER>/<INTERVAL>/<YEAR>/<MONTH>.parquet`, bars
  thread through `bars_override` into the existing engine arms.
  Equity curve + buy/sell markers render against real bars.

> _Caveat — no live render captured._ This deck describes the visual
> in prose; the headless tester run produced no screenshot. To
> confirm visually:
> `cargo run -p ui --bin cockpit_live --features yahoo` then open
> Lab and click `YahooCache`.

## What shipped (functional surface)

| Surface | Where | Commit |
|---|---|---|
| `YahooBarSource` + parquet cache reader | `crates/data/src/yahoo.rs` | `04e059f` |
| `fetch_yahoo_klines` CLI | `crates/data/src/bin/fetch_yahoo_klines.rs` | `04e059f` |
| `Venue::Yahoo` variant + match-arm cascade | `crates/core/src/venue.rs` | `04e059f` |
| `LabDataSource` enum + `LabRunConfig.data_source` field | `crates/ui/src/lab/{state,runner}.rs` | `a87bbc4` |
| Source toggle widget | `crates/ui/src/widgets/source_toggle.rs` | `a87bbc4` |
| Cadence badge widget | `crates/ui/src/widgets/cadence_badge.rs` | `a87bbc4` |
| `YAHOO_CRYPTO_UNIVERSE` (10 tickers, XRP-first) | `crates/ui/src/lab/universe.rs:56` | `a87bbc4` |
| `binance_to_yahoo_ticker` boundary converter | `crates/data/src/yahoo.rs` | `04e059f` |
| ADR-0040 — Yahoo realdata path + revision pin | `spec/architecture/adr/0040-yahoo-realdata-path.md` | `7ab924e` |
| `yahoo_finance_api = "=4.1.0"` workspace dep (default-off) | `Cargo.toml:129` | `04e059f` |

**Operator command (smoke):**
```
cargo run -p data --features yahoo,yahoo-online --bin fetch_yahoo_klines -- \
    --ticker BTC-USD --interval 1d --start 2024-01-01 --end 2024-01-31
cargo run -p ui --bin cockpit_live --features yahoo
```

## Test / quality-gate snapshot

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | PASS (exit 0) |
| `cargo clippy -p ui --lib --bins -- -D warnings` | PASS (0 warnings) |
| `cargo clippy -p backtest --lib -- -D warnings` | PASS (0 warnings) |
| `cargo test --workspace --lib` | PASS — **≥ 878** tests, 0 failed, 5 ignored |
| T-C3.7 `lab_yahoo_dispatch` integration test (`--features yahoo`) | PASS — **7/7** (ticker boundary, ScenarioConfig shape, fixture load, SHA determinism, all 10 mirror pairs, error path) |
| `bash scripts/verify_anchors.sh` | PASS — **34/34 byte-identical** |
| `spec_lint` | BASELINE-STABLE — 60 violations, 1 category (`dead-link`), **0 new from this feature** |
| Hypotheses H3/H4/H5/H6 | PASS (offline path) |
| Hypotheses H1/H2 | Deferred — see § Deferred risks |

Full tester report: [`reports/test-final-2026-05-24.md`](../reports/test-final-2026-05-24.md)
(tester verdict line: `VERDICT → PASS`, commit `a87bbc4`).

## Anchor-additive contract proof

```
ANCHORS PASS  (34 / 34)
```

Mechanism (ADR-0038 § D6.b, ratified in
[`decomp.md § Anchor neutrality proof`](../decomp.md)):
- `ScenarioConfig.data_source` is `#[serde(default, skip_serializing_if = …)]`
  → deserializes to `Synthetic` on every existing scenario JSON.
- `ScenarioConfig.bars_override` is `Option<Vec<Bar>>` defaulting to
  `None` → existing CLI anchor-generating paths never set it.
- Zero new anchors added at v0.1.0 (R6.3 / Q-anchor decision). Yahoo
  anchors lock at v0.1.1 after operator sign-off on a sample run.

## Deferred to v0.1.1 — operator triages these next

| Item | Why deferred | Owner-next | Risk if we skip |
|---|---|---|---|
| **H1** Yahoo daily BTC vs Binance hourly equity divergence < 30% on `v0.sma` | Needs live Yahoo cache populated; offline tester has no fetch | Operator fetches Jan-2024 BTC-USD daily via CLI → developer runs backtest → tester anchors | Yahoo backtest equity could disagree wildly with Binance — undermines the "realdata = realdata" mental model |
| **H2** `yahoo_finance_api` success > 95% over 7-day window | Requires online fetch (rate-limited unofficial API) | Operator runs the fetch CLI for 7 days; developer wires retry/backoff if rate-limited | Yahoo API breakage = silent Lab failure. Mitigated by `K1` (offline cache is read-only at runtime) but fetch-side reliability is unknown |
| **T-T5** cockpit-smoke visual smoke | Needs macOS window + live runtime; tester ran headless | Operator runs `cargo run -p ui --bin cockpit_live --features yahoo` and confirms render | Visual regression in source toggle / cadence badge slips past automated gates |
| **T-T8** idle-CPU regression check | Same — requires runtime profile | Operator clicks `YahooCache` once, leaves idle 60s, checks CPU% | Cache-read path could leak a busy-loop; offline path is unchanged so risk is bounded |
| **Cache-root gap** | `preload_yahoo_bars` uses `data/yahoo` (CWD-relative); tests use `crates/data/tests/fixtures/yahoo/` directly | v0.1.1 — make cache root configurable, or add `data/yahoo/` fixture | Lab runs from a non-repo-root CWD will silently fail to load |
| **`dead_code` warnings under `--features yahoo --no-default-features`** | `range_to_ms_pair` + `preload_yahoo_bars` are gated on `yahoo` but called only under `live`; clippy flags them dead in `yahoo,!live` | Add `#[allow(dead_code)]` doc OR a `yahoo,live` combined test | Cosmetic on `main` (default build is clean); blocks a future `yahoo`-only release build |

Pre-existing (not introduced by this feature, NOT blocking ship):
- `consistency.rs::no_inline_user_visible_strings_in_widgets` — pre-existing
  failure carry-forward from `ui-rethink-phase-d-trail` (`6d7f90d`); Wave
  C-3 touched none of the flagged files.
- 60 `dead-link` spec-lint violations — baseline-stable from prior features.

## Open decisions for the operator

1. **Ship v0.1.0 as wiring-only?** Tester verdict is PASS; anchor-
   additive contract holds; H1/H2 are deferred to v0.1.1 by design
   (R6.3 ratified at M-T1). The operator's "yes" here ships Wave
   C-1 .. C-4 + tester reports + presenter deck to `main` as
   v0.1.0 and parks v0.1.1 (live H1/H2 + Yahoo anchors) as the
   next backlog row.

2. **v0.1.1 trigger.** Approval implicitly commits to: (a) the
   operator running the `fetch_yahoo_klines` CLI at least once
   against BTC-USD Jan-2024 (or similar small window); (b) a
   follow-up developer wave that wires Yahoo anchors into
   `spec/anchors.toml` + tester re-locks. If you'd rather defer
   v0.1.1 indefinitely (e.g. Yahoo fails the H1 divergence sanity-
   check), the wiring stays in tree but the Lab's YahooCache toggle
   is documented as experimental.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / reason
_operator fills in_

## Changelog

- 2026-05-24 (presenter): v0.1.0 sprint-review deck — covers Waves
  C-1..C-4 + tester M-FINAL PASS; commits `7ab924e`, `04e059f`,
  `a87bbc4`.
