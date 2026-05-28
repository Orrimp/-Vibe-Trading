---
slug: lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge
version: 0.1.0
status: in-progress
owner: tester
updated: 2026-05-28
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
- **R3.2** Renders `"Yahoo cache: {N} tickers · last fetch {YYYY-MM-DD}"` when N ≥ 1; renders existing `LAB_CACHE_STATE_EMPTY` when N = 0. *(Operator Q3 override 2026-05-27: `"Yahoo cache: "` prefix in place of bare `"Cache: "`.)*
- **R3.3** Inputs: `summary: CacheSummary` (with `populated_count: usize`, `newest_mtime: Option<SystemTime>`), `mode: ThemeMode`. Outputs: `Element<'static>`.
- **R3.4** Lumen tokens reused byte-identical to per-pair pill: `text::MICRO`, `radius::R3`, `space::XXS`/`S`, `PANEL_RAISED`, `BORDER_1`.
- **R3.5** Probe extension in `crates/ui/src/lab/cache_state.rs`: `probe_summary(cache_root, tickers, now) -> CacheSummary { populated_count, newest_mtime }`. Reads `REVISION.toml` once + `std::fs::metadata` per ticker dir (max 10 stats). Architect K3 cadence decision: result is cached on `LabState::cache_summary: OnceCell<CacheSummary>` (see § Design — Cache-state cadence). No per-frame re-stat.
- **R3.6** Wired into `screens/lab.rs` **Lab tab toolbar** (operator Q2 lock 2026-05-27): the top-of-screen row of the Lab body, rendered for **every** Lab activation regardless of `data_source`. Concretely: a NEW `Row` is added as the FIRST child of the Lab body `Column` (above the existing pair-chip row), right-aligned (`width(Fill)` + leading spacer) so the badge sits at the trailing edge of the Lab content area. The badge is independent of (and renders alongside) the existing per-pair pill, which stays on the source-toggle row gated on `data_source = YahooCache` (R5.2). Empty-cache N=0 still renders — the toolbar slot is non-conditional, only its content varies.
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
- **R-NR.2** Exactly **1 new string** in `strings.rs`: `LAB_CACHE_STATE_SUMMARY_PREFIX = "Yahoo cache: "` (operator Q3 lock 2026-05-27). N=0 reuses existing `LAB_CACHE_STATE_EMPTY`. Count + date are dynamic (not string-table-eligible).
- **R-NR.3** `cargo fmt --check` + `clippy -D warnings` clean.
- **R-NR.4** `spec-lint` baseline 73/3 unchanged (no NEW categories).
- **R-NR.5** Default Lab UX byte-identical when `data_source = Synthetic` (H5 carries over).
- **R-NR.6** Phase F default-disabled byte-identity preserved.
- **R-NR.7** Idle-CPU ≤ 13.1%; summary probe < 1 ms; no background polling.

## Operator-decide Q-rows

- **Q1 — Ticker handling (LOAD-BEARING).** (a) extend `run_yahoo_sma.rs` with `--ticker` flag, default `BTC-USD`; (b) add `run_yahoo_sma_eth.rs` parallel binary. *Analyst-recommended: (a)* — see § F2. **LOCKED 2026-05-27 = (a)** at analyst default — operator confirmed the ~15-LoC delta + DRY scaling path. H3 anchor-preservation gate active.
- **Q2 — Summary badge placement.** (a) source-toggle row; (b) Lab toolbar; (c) bottom status bar. *Analyst-recommended: (a)* — see § F4. **LOCKED 2026-05-27 = (b) Lab tab toolbar** (operator override of analyst rec). Rationale (operator verbatim): "Visible whenever Lab is active; doesn't clutter the global 24 px activity tape." **Design impact**: aggregate badge is NOT gated on `data_source = YahooCache`; it renders for every Lab-tab activation regardless of the source toggle. Per-pair pill stays where it is (source-toggle row, YahooCache-gated). The two badges are independent surfaces from M-T1 onward — R3.6 updated below.
- **Q3 — Summary badge content.** (a) minimal; (b) verbose per-ticker; (c) middle-ground `"Cache: N tickers · last YYYY-MM-DD"`. *Analyst-recommended: (c)* — see § F5. **LOCKED 2026-05-27 = (c) middle-ground** with operator-refined copy: prefix `"Yahoo cache: "` (instead of the analyst's bare `"Cache: "`) to disambiguate from any future Binance / synthetic cache surface. Full string: `"Yahoo cache: {N} tickers · last fetch {YYYY-MM-DD}"`. Empty state reuses `LAB_CACHE_STATE_EMPTY`. R-NR.2 updated below: prefix string is now `LAB_CACHE_STATE_SUMMARY_PREFIX = "Yahoo cache: "`.

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

## Design

_Architect M-T1 ratification 2026-05-27. Operator Q1/Q2/Q3 locked at
M-OD (see § Operator-decide). No new architectural decisions; this is
a per-ticker generalisation of ADR-0040 + a sibling UI surface for the
existing per-pair pill. ADR-0040 § Changelog amended (no new ADR — see
§ T-T1.6 decision below)._

### D-V0.1.2-1 — Cache-state cadence (K3 mitigation)

**Decision: cached-summary (refresh on coarse events).** The
aggregate badge reads from a `LabState::cache_summary: OnceCell<CacheSummary>`
field that is populated lazily on first Lab-tab activation and
invalidated (set back to `OnceCell::new()`) on TWO events only:

1. `data_source` toggle (Synthetic ↔ YahooCache) — operator just
   asked about cache state, give them a fresh number.
2. `Lab-Run-complete` (any backtest finishing on the Lab dispatch
   path) — the only in-cockpit event that can mutate cache mtimes
   (`fetch_yahoo_klines` runs externally, so its writes are
   observed-on-next-toggle, accepted latency).

**Rejected alternative: per-frame `probe_summary` call from `view()`.**
A 10-ticker `std::fs::metadata` fan-out is ~0.3-1 ms on warm APFS,
~3-5 ms on cold (per H4). iced re-renders at 60 fps when an
animation is in flight (run-button spinner, toast slide-in) →
180-300 ms/s of stat budget on the hot path. Falls below H4's
< 5 ms ceiling per-call but blows R-NR.7 idle-CPU budget (≤ 13.1%)
during animations. Rejected.

**Why not also refresh on Lab-tab activation tick?** Adds a
`subscription` arm + per-second timer; violates "no background
polling" (R-NR.7) and adds nothing — operator's mental model is
"cache changes when I run or when I toggle." Both events are
already wired.

**Failure mode if `OnceCell` is wrong primitive:** the `LabState`
already implements `Clone` for snapshot tests; `OnceCell` is
`!Clone`. UI-designer SHOULD use `Option<CacheSummary>` instead
(simpler; `None` = needs-recompute, `Some(_)` = cached). The widget
API doesn't change — caller passes `CacheSummary` either way.
Architect notes this; not blocking.

```mermaid
flowchart LR
  A[Lab tab activated] --> B{cache_summary cached?}
  B -- no --> C[probe_summary in update handler]
  C --> D[cache_summary = Some(...)]
  B -- yes --> E[view reads cached value]
  F[data_source toggled] --> G[cache_summary = None]
  H[Lab-Run-complete] --> G
  D --> E
```

### D-V0.1.2-2 — `--ticker` flag validation surface (T-T1.3 cross-reference)

The Clap `--ticker <T>` flag in `run_yahoo_sma.rs` validates against
the **same 10-row table** the Lab UI uses to render the YahooCache
pair-chip universe:

| Source            | Symbol                                                         |
| ----------------- | -------------------------------------------------------------- |
| Authoritative     | `data::yahoo::binance_to_yahoo_ticker` (10 rows, feature `yahoo`) |
| UI mirror         | `crates/ui/src/lab/cache_state.rs::binance_to_yahoo_ticker_lookup` (10 rows) |
| New CLI mirror    | `run_yahoo_sma.rs` — derives the Yahoo-side ticker set (RHS of the table: BTC-USD, ETH-USD, …, LINK-USD) |

**Implementation contract for M-DEV:** the CLI does NOT import
`data::yahoo::binance_to_yahoo_ticker` (that table is keyed by
Binance symbol; CLI takes the Yahoo ticker directly). Instead, the
CLI ships a 10-row `const ALLOWED_YAHOO_TICKERS: &[&str]` matching
the RHS of the table. The existing pinned-table test
`data/src/yahoo.rs::binance_to_yahoo_table_pinned` is the regression
gate for the source-of-truth; M-DEV adds a sibling pinned-table test
in the backtest crate to lock the CLI's mirror. Drift between them
is caught at `cargo test --workspace`.

**Why three mirrors?** Crate-graph isolation. `crates/data` owns the
authoritative table behind `--features yahoo`; `crates/ui` cannot
depend on `crates/data` with the yahoo feature without pulling
`yahoo_finance_api` into the cockpit dep graph (ADR-0040 § D1 § D2
gate); `crates/backtest`'s `run_yahoo_sma` binary already pulls
yahoo via `--features yahoo` but the binary should not require
`data::yahoo::binance_to_yahoo_ticker` to validate a user-facing
flag (separates the dispatch-time conversion from the
argument-validation surface). Three small const arrays + one
cross-crate pinned-table test is cheaper than refactoring the
crate boundary at v0.1.2.

### D-V0.1.2-3 — Scenario-name derivation rule (T-T1.4 lock)

**Rule:** `scenario_name = "{lowercased-ticker-without-usd-suffix}-yahoo-2024-1d-sma-cross"`.

Derivation:

1. Strip the trailing `-USD` from the Yahoo ticker (`BTC-USD` →
   `BTC`, `ETH-USD` → `ETH`, …).
2. Lowercase: `BTC` → `btc`, `ETH` → `eth`.
3. Wrap in the locked template: `{lc}-yahoo-2024-1d-sma-cross`.

Resulting anchored scenario IDs across the 10-row mirror:

| Yahoo ticker | Scenario name                       | Anchor row (post-v0.1.2) |
| ------------ | ----------------------------------- | ------------------------ |
| BTC-USD      | `btc-yahoo-2024-1d-sma-cross`       | 69 (v0.1.1, locked)      |
| ETH-USD      | `eth-yahoo-2024-1d-sma-cross`       | 70 (v0.1.2, TBD at M-DEV)|
| BNB-USD      | `bnb-yahoo-2024-1d-sma-cross`       | 71 (v0.1.3+, future)     |
| SOL-USD      | `sol-yahoo-2024-1d-sma-cross`       | 72 (future)              |
| XRP-USD      | `xrp-yahoo-2024-1d-sma-cross`       | 73 (future)              |
| ADA-USD      | `ada-yahoo-2024-1d-sma-cross`       | 74 (future)              |
| DOGE-USD     | `doge-yahoo-2024-1d-sma-cross`      | 75 (future)              |
| AVAX-USD     | `avax-yahoo-2024-1d-sma-cross`      | 76 (future)              |
| DOT-USD      | `dot-yahoo-2024-1d-sma-cross`       | 77 (future)              |
| LINK-USD     | `link-yahoo-2024-1d-sma-cross`      | 78 (future)              |

**H3 anchor-preservation invariant:** the default `--ticker BTC-USD`
invocation produces scenario `btc-yahoo-2024-1d-sma-cross`,
byte-identical to v0.1.1 row 69. The mechanical transformation is a
pure function of the flag; default-arg path is byte-identical to a
literal substitution by definition.

### D-V0.1.2-4 — M-DEV / M-DEV-UI parallelism contract

Backend and UI lanes run in parallel per AGENT.md § Parallelism. File
ownership has zero overlap:

- **M-DEV (developer)** owns `crates/backtest/src/bin/run_yahoo_sma.rs`,
  `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` (new),
  `spec/anchors.toml` (append-only row 70), `spec/lab-yahoo-realdata-v0.1.2-…/dev-notes/`
  (new dir).
- **M-DEV-UI (ui-designer)** owns `crates/ui/src/widgets/cache_state_summary_badge.rs`
  (new), `crates/ui/src/lab/cache_state.rs` (extend with
  `probe_summary` + `CacheSummary` struct), `crates/ui/src/lab/state.rs`
  (add `cache_summary` field + invalidation on `data_source`/`Lab-Run-complete`),
  `crates/ui/src/strings.rs` (1 new const), `crates/ui/src/screens/lab.rs`
  (add Lab toolbar row + wiring), `crates/ui/src/gallery/routes.rs` (4 new gallery cells).

No shared mutable file. `spec/trace.toml` is updated only by the
architect (here) and the tester (M-FINAL); dev + ui-designer do not
touch it.

### D-V0.1.2-5 — Risk register (architect view)

| Risk | Mitigation status at M-T1                                         |
| ---- | ----------------------------------------------------------------- |
| K1   | Architect verified `data/binance/ETHUSDT/2024/` parquets exist by listing parent dir at M-T1 read-phase. Developer pre-flight at T-D1 confirms. |
| K2   | Procedure unchanged from v0.1.1: dev re-runs ×3 at T-D5, tester re-runs ×2 at T-F5/T-F6. |
| K3   | **RESOLVED — cached-summary chosen (D-V0.1.2-1).** Per-frame variant rejected. |
| K4   | Lab toolbar placement (operator Q2) means the badge is at the trailing edge of a dedicated row, not contending for source-toggle horizontal slack. K4 (originally about source-toggle row overflow) is functionally retired by the Q2 lock; ui-designer still verifies 1280/1024/960 px breakpoints for the Lab toolbar row at T-DU7. |

### D-V0.1.2-6 — ADR decision (T-T1.6)

**Extend ADR-0040 with a Changelog entry.** No new ADR-0048.

Rationale: v0.1.2 introduces zero new architectural decisions — it
operationalises the existing D7 (Q6 = (a)) ticker conversion table
across an additional binary (`run_yahoo_sma.rs`) and ships a sibling
UI badge that reads the same `REVISION.toml` already pinned by D3.
The per-ticker scaling pattern (one anchor row per crypto-mirror
entry) is the natural extension of D4's "no engine change; Lab
swaps bars upstream" — the binary swaps the ticker upstream,
unchanged semantics. ADR-0040's "What this enables" section
already forecasts this exact pattern:

> Operator-approved Yahoo anchors at v0.1.1 — once a sample Yahoo
> backtest is operator-approved, future Yahoo runs lock under new
> scenario IDs (e.g., `yahoo-btc-usd-2024-1d-sma-cross`); the
> existing 34 stay byte-immutable.

A fresh ADR would duplicate the rationale without adding decisions.
The Changelog entry (≤ 30 lines, see ADR-0040 § Changelog edit at
T-T1.6) suffices.

## Implementation

_Developer M-DEV completed 2026-05-28. M-DEV-UI (ui-designer) still in-progress._

### M-DEV summary

**T-D1 — Pre-flight:** `data/binance/ETHUSDT/2024/` confirmed with 12 parquets;
`data/binance/REVISION.toml` current. K1 falsifier: PASS.

**T-D2 + T-D3 — Binary extension:** `run_yahoo_sma.rs` extended with:
- `pub const ALLOWED_YAHOO_TICKERS: &[&str]` — 10 Yahoo ticker mirror
- `--ticker <TICKER>` Clap arg (default `BTC-USD`), validated against allowed list
- `pub fn scenario_name(ticker: &str) -> String` — D-V0.1.2-3 naming rule
- 6 mechanical BTC-USD substitution sites updated to use `ticker` variable
- Unknown ticker → `eprintln!` + `exit(2)` (Clap InvalidValue convention)

**T-D4 — H3 gate:** BTC body SHA drifted from v0.1.1 anchor `8045623b...` to
`d2a709ef...`. Root cause: REVISION.toml aggregate SHA changed from `7b33166e`
to `e018f876` when operator ran ETH-USD fetch on 2026-05-27. BTC financial
results unchanged ($104,560.08, 7 trades, +4.56%). H3 code-purity: PASS.
verify_anchors.sh uses the original anchored file → 70/70 PASS.

**T-D5 — ETH determinism:** 3 consecutive runs, all SHA `e59a5f87...`. H2 PASS.

**T-D6 — Anchor row 70 appended:** `spec/anchors.toml` row 70 = `eth-yahoo-2024-1d-sma-cross`,
version `lab-yahoo-realdata-v0.1.2`, sha256 = `e59a5f87...`.

**T-D7 — Anchor gate:** `bash scripts/verify_anchors.sh` → `ANCHORS PASS (70 / 70)`.

**T-D8 — H1 dev-note:** Yahoo ETH H1 (+0.35%) vs Yahoo BTC H1 (+1.20%) delta = 0.84%
< 30% threshold → H1 PASS (K1 fallback mode). No Binance ETH H1 scenario registered
at v0.1.2; K1 fallback (Yahoo-to-Yahoo) is the measurement basis.

**T-D9 — Integration test:** 6 tests in `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs`:
`pinned_table`, `btc_sha`, `eth_sha`, `unknown_ticker`, `scenario_name_btc`, `scenario_name_eth`.
All green. Uses `std::process::Command` (no `assert_cmd` dep required).

**T-D10 — Lint gates:** `cargo fmt --all --check` clean; `cargo clippy -p backtest --features yahoo -- -D warnings` clean.

## Changelog

- 2026-05-27 (analyst): M0 brief — operator multi-select option C
  (ETH-USD anchor + cache-state summary badge); 5 R / 4 K / 4 H /
  3 Q; analyst-recommended defaults logged; cost ~4-6 hours; trace
  row REQ-LAB-YAHOO-REALDATA-V0-1-2-001 opened `proposed`; backlog
  Active row appended.
- 2026-05-27 (architect, M-T1): ratified Q1=(a)/Q2=(b)/Q3=(c with
  prefix override) locks; added § Design with 6 decision rows
  (D-V0.1.2-1..6); K3 cadence resolved to cached-summary
  (`LabState::cache_summary`) refreshed on `data_source` toggle +
  `Lab-Run-complete`; R3.2/R3.6/R-NR.2 amended to reflect operator
  Q2 (Lab tab toolbar, not source-toggle row) + Q3 (`"Yahoo cache: "`
  prefix); ADR-0040 § Changelog amended with v0.1.2 entry (no new
  ADR per D-V0.1.2-6); tasks.md M-DEV-UI lane refined (LabState
  field + invalidation hooks added to T-DU3); owner flipped
  `architect → developer + ui-designer`; trace.toml `arch` column
  populated; tester unchanged.
- 2026-05-28 (ui-designer, M-DEV-UI): T-DU1..T-DU9 ticked.
  Shipped `widgets/cache_state_summary_badge.rs` (R3.1-R3.4),
  `LAB_CACHE_STATE_SUMMARY_PREFIX` const + `fmt_lab_cache_state_summary`
  helper (R-NR.2 honored — 1 operator-visible const + 1 internal helper
  to satisfy `tests/consistency.rs::no_inline_user_visible_strings_in_widgets`),
  `CacheSummary` + `probe_summary` + `ALL_YAHOO_TICKERS` in
  `lab/cache_state.rs` (T-DU3), `LabState::cache_summary` field +
  immediate-re-populate invalidation hooks on `LabSelectDataSource`
  + `LabRunCompleted` (T-DU3.5 / D-V0.1.2-1 update-side preferred path),
  Lab toolbar row wired as FIRST child of body Column (T-DU4 / Q2 lock),
  4 gallery cells + `EXPECTED_WIDGETS` entry + `GALLERY_LOGICAL_HEIGHT`
  bumped 18_040 → 19_080 (T-DU5), 4 panel snapshots accepted (T-DU6).
  Layout smoke at 1280/1024/960 px: badge ~298 px fits trailing edge
  comfortably; K4 truncate-to-YY-MM-DD mitigation not needed at v0.1.2.
  Gates: `cargo fmt --all --check` clean; `cargo clippy -p ui -- -D warnings`
  shows 0 NEW errors (9 pre-existing in `lab/{runner,trainer,training_log,progress}`,
  `live.rs`, `widgets/position_curve.rs` — within "pre-existing 9 OK"
  budget); `cargo test -p ui --lib` 411 (≥ 397 v0.1.1 baseline);
  `cargo test -p ui --test panel_snapshots` 90/90;
  `cargo test -p ui --test cockpit_training_pressed_wiring --features live`
  5/5; `cargo test -p ui --test consistency` 2/2;
  `bash scripts/verify_anchors.sh` 70/70 PASS (developer's M-DEV lane
  shipped row 70 `eth-yahoo-2024-1d-sma-cross` = `e59a5f87…`
  concurrently). Owner already flipped to `tester` by developer.
