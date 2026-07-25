---
slug: advisor-data-quality-surface
status: shipped
owner: operator
version: 2.0.0
updated: 2026-07-06
---

# P1-7 DATA-Stage Trust/Quality Surface

A plain, `ui`-owned display DTO — `DataQualityView` — giving the operator a
"how much can I trust this data" readout on the Leaderboard screen: venue +
provenance, a venue-trust classification, an always-present survival-bias
caveat, and zero or more plain-language data-quality warnings (thin
liquidity / wash-trading suspicion / pump-and-dump). **Display-only** — it
never feeds any gate, rank, or verdict.

**Design reference:** [`v2-architecture.md`](../v2-architecture.md) §1 P1-7
(`[A]` additive feature). Research:
[`research/crypto-market-structure/application-data-integrity.md`](../../../research/crypto-market-structure/application-data-integrity.md)
§6 C (universe screen / data-quality context). Reuses the venue-trust
classifications shipped with P1-6:
[`docs/dev-notes/venue-trust-map-2026-07-01.md`](../../../docs/dev-notes/venue-trust-map-2026-07-01.md).

## History note (REDO)

A prior attempt at this feature died to an Anthropic API overload mid-session
with roughly 80% of the work on disk; the orchestrator reverted it to a clean
slate because the spec scaffold (this file + `tasks.md` + the trace row) was
never created — the missing piece that made the partial code un-auditable.
This increment starts from that clean slate and closes the gap.

## What was built

### 1. `DataQualityView` DTO (`crates/ui/src/leaderboard/state.rs`)

A plain `ui`-owned struct — no `strategy`/`exec`/`llm`/`models`/`data` dep
edge. Fields:

| Field | Type | Purpose |
|---|---|---|
| `venue` | `String` | Which exchange the price series is sourced from (e.g. `"Binance"`). |
| `provenance` | `String` | One-line sourcing mechanics (e.g. "Hourly close from Binance klines, cached in the pinned backtest corpus."). |
| `venue_trust` | `VenueTrust` | Closed enum — `HighReconcilable` / `ConditionalWatch` / `LowFabricatedRisk`, mapped from the venue-trust-map dev-note's three-tier scheme. |
| `survival_note` | `String` | ALWAYS present — the survivorship-bias caveat ("coins that failed to reach today are absent…"). |
| `warnings` | `Vec<DataQualityWarning>` | Zero or more of `ThinLiquidity` / `WashTradingSuspicion` / `PumpAndDump`, each with a `.copy()` plain-language description. Empty for the default deep-liquidity Binance-corpus universe — the honest "nothing to flag" case, not a placeholder. |

`DataQualityView::for_symbol(symbol: &str) -> Self` is the MVP constructor:
every symbol in `BAKEOFF_COIN_UNIVERSE` shares the SAME pinned Binance-klines
corpus, so the lookup resolves every default symbol (including `BTCUSDT`) to
the same tuple: Binance / the standard provenance line / `HighReconcilable`
/ the standard survival note / no warnings. `symbol` is accepted (shaped for
a future per-symbol classification) but not currently branched on — a later
increment can widen the match without a call-site change.

### 2. Mirror wiring (`BakeoffReportMirror`)

`BakeoffReportMirror.data_quality: DataQualityView` — a NEW, always-`Some`-
shaped field (not `Option`, unlike `scorecard`/`tail`: there is no
"degenerate" DATA-quality state to suppress — every bake-off runs on a known
symbol). Populated in `BakeoffReportMirror::from_report` from
`report.request.symbol.0.as_str()` — the SAME symbol the `coin` field
echoes, so the DATA panel always describes the symbol actually ranked.

### 3. Render panel (`crates/ui/src/screens/leaderboard.rs`)

`data_quality_block()` — a `frame::panel`-titled "Data quality" block,
rendered ABOVE the recommendation + ranked table in `ready_pane` (the
DATA → ANALYSIS → SUGGEST workflow spine: this is the first honesty layer,
describing the INPUT before the crown/scorecard/risk-story describe the
OUTPUT). Reuses the existing `scorecard_fact` label/value/hint composition —
zero new theme tokens, zero new widgets. Rows: Venue, Provenance, Trust
level, Survival bias (always), Warnings (only when non-empty), and the
load-bearing "informational, not a gate" note at the bottom (matching the
scorecard's and Risk story's framing).

### 4. Strings

18 new `LEADERBOARD_DATA_QUALITY_*` constants in `crate::strings`,
registered in `strings::all()` (the completeness self-test).

## Anchor safety

Display-only DTO on the advisor bake-off path (`write_report=false` in the
UI's bake-off dispatch); no anchored CLI path reads `BakeoffReportMirror` at
all. Verified BEFORE and AFTER: `bash scripts/verify_anchors.sh` → 119/119.

## Not in this increment

- No per-symbol trust/warning differentiation beyond the MVP hardcode — every
  `BAKEOFF_COIN_UNIVERSE` symbol resolves identically today (all sourced from
  the same pinned Binance corpus). A future increment can widen
  `for_symbol`'s match once a non-Binance-sourced or thin-liquidity symbol
  enters the universe.
- No live venue-metadata service — the lookup is a hardcoded MVP mapping, not
  a queryable trust API.
- No overlay end-to-end divergence test — CLAUDE.md's day-1-e2e rule applies
  to strategy overlays/sizing-modifiers that change a decision variable; this
  is a display-only DTO with no decision-variable output, so the rule does not
  apply (the architect names this constraint explicitly in §1 P1-7: "No
  behavior, no overlay e2e").
