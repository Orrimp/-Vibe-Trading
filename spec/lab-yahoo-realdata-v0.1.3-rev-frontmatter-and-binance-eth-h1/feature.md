---
slug: lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1
version: 0.1.0
status: in-progress
owner: developer
updated: 2026-05-28
predecessor: lab-yahoo-realdata-v0.1.2 v0.1.0
priority: P2
---

# Lab Yahoo realdata v0.1.3 — REVISION rev= body→front-matter + Binance ETH H1 scenario

> Closes the 2 architect-flagged design notes from v0.1.2 M-FINAL: (1) the
> `rev=<sha>` substring in the Yahoo report body couples body-SHA stability
> to the `REVISION.toml` aggregate, causing spurious BTC SHA drift any
> time the operator fetches a new ticker; (2) H1 was discharged via a
> Yahoo-to-Yahoo K1 fallback because no `eth-2024-h1-sma-cross` Binance
> scenario was registered.

## Why

v0.1.2 SOFT-PASS surfaced two distinct durability defects:

**Defect 1 — body SHA pollution.** `run_yahoo_sma.rs:259` writes
`Data source: yahoo-cache:{ticker}/1d/2024 rev={revision_sha:.12}` into
the report body. The `rev=` suffix mutates whenever ANY ticker in
`data/yahoo/` is fetched (changes the aggregate SHA). The pattern is
guaranteed to recur for BNB/SOL/XRP/… fetches at v0.1.4+.

**Defect 2 — missing Binance ETH H1 scenario.** v0.1.2 H1 was discharged
via K1 fallback (Yahoo ETH daily +0.35% vs Yahoo BTC daily +1.20% = 0.84%
delta). Binance ETHUSDT 2024 parquets exist on disk (12 files confirmed
by v0.1.2 tester); the missing piece was a registered scenario in
`crates/backtest/src/main.rs`. v0.1.3 registers it so the direct
Yahoo-daily-vs-Binance-hourly comparison becomes basis-of-record.

Both are backend hygiene; both pay back across every future Yahoo ticker
fetch and every future per-ticker Binance H1 hypothesis.

## Scope (v0.1.0)

- Move `rev=<sha>` from report body → YAML front-matter `revision_sha:`.
- Extract a canonical Yahoo report-emit helper (Q1=(a)) so future
  `run_yahoo_macd` / `_rsi` / `_bbands` binaries inherit byte-identically.
- Re-emit BTC anchor row 69 under the new emit shape; update SHA in-place
  under existing namespace `lab-yahoo-realdata-v0.1.1` (Q2=(a)).
- Register `eth-2024-h1-sma-cross` Binance hourly scenario; lock as new
  anchor row 71.
- Re-discharge H1 for ETH directly (Yahoo daily vs Binance hourly).
- Anchor count delta: 70 → 71 (1 in-place BTC update + 1 new ETH H1 row).

## Out of scope

- 8 remaining unanchored crypto-mirror tickers (BNB → LINK) — v0.1.4+.
- Multi-strategy on Yahoo (MACD / RSI / BBands) — v0.2.0+.
- Re-emit of v0.1.2 ETH daily row 70 (stays byte-identical; bulk ticker
  re-emit deferred to v0.1.4 BNB ship).
- Any UI surface change for `revision_sha:` frontmatter (consumers are
  `verify_anchors.sh` + future emitters, not the cockpit).
- New design tokens / new `strings.rs` entries.

## Requirements

### R1 — REVISION.toml `rev=` body→front-matter migration

- **R1.1** Yahoo report body MUST NOT contain any `rev=<sha>` substring.
  `Data source: yahoo-cache:{ticker}/1d/2024 rev={sha}` becomes
  `Data source: yahoo-cache:{ticker}/1d/2024`.
- **R1.2** Yahoo report front-matter gains `revision_sha: <full 64-char hex>`.
- **R1.3** Canonical Yahoo report-emit helper extracted (developer-chosen
  location — recommended `crates/backtest/src/report/yahoo.rs`) so the
  body-vs-frontmatter split lives in exactly one place. Helper shape
  MUST permit a 1-LoC call-site for any future `run_yahoo_*` binary.
- **R1.4** Post-condition grep: `grep -RIn "rev=" spec/lab-yahoo-realdata-v0.1.3-*/reports/`
  returns zero matches for the v0.1.3 newly-emitted reports.
- **Acceptance**: re-emitted BTC body contains no `rev=`; front-matter
  `revision_sha:` matches `data/yahoo/REVISION.toml` aggregate at emission.

### R2 — Binance ETH H1 scenario registration

- **R2.1** New scenario `eth-2024-h1-sma-cross` in `crates/backtest/src/main.rs`
  mirroring the existing `btc-2024-h1-sma-cross` arm (symbol `ETHUSDT`,
  2024 H1 window, `bar_count: 262_800`, SmaCrossover{20,50}).
- **R2.2** Data path `data/binance/ETHUSDT/2024/` (12 parquets confirmed);
  developer pre-flight schema parity check at T-D1.
- **R2.3** Anchor row 71 appended to `spec/anchors.toml` under namespace
  `lab-yahoo-realdata-v0.1.3`.
- **R2.4** Determinism verified ≥ 2 independent re-runs.
- **Acceptance**: `verify_anchors.sh` 71/71 PASS.

### R3 — Anchor count delta + non-regression

- **R3.1** Anchor count 70 → 71. Row 69 BTC SHA updates in-place (Q2=(a)
  in-place under existing namespace `lab-yahoo-realdata-v0.1.1`); row 70
  ETH daily byte-identical; row 71 = new `eth-2024-h1-sma-cross`.
- **R3.2** 68 non-Yahoo anchors (rows 1-68) byte-identical.
- **R3.3** Row 70 (ETH daily v0.1.2 `e59a5f87…`) byte-identical — not
  re-emitted at v0.1.3. Bulk Yahoo-ticker re-emit deferred to v0.1.4 BNB.
- **R3.4** Workspace lib test count maintained.
- **Acceptance**: `verify_anchors.sh` 71/71 PASS; tester recomputes BTC +
  ETH H1 SHAs on M-FINAL.

### R4 — H1 ETH direct re-discharge

- **R4.1** With R2 shipped: Yahoo ETH-USD daily 2024 SMA(20,50) seed
  0xC0FFEE vs Binance ETHUSDT 2024 hourly SMA(20,50) seed 0xC0FFEE; same
  threshold < 30%.
- **R4.2** Expected delta 5-15% (BTC was 9.03%; ETH 2024 bull-run shape
  comparable). Falsifier: ≥ 30% → K4.
- **R4.3** Findings recorded in `dev-notes/yahoo-vs-binance-eth-h1-2026-05-XX.md`
  mirroring v0.1.1 BTC dev-note shape.
- **Acceptance**: H1 PASS direct; v0.1.2 K1 fallback retired.

### R5 — Non-regression contract

- **R5.1** 68 non-Yahoo anchors byte-identical.
- **R5.2** ETH daily row 70 (`e59a5f87…`) byte-identical.
- **R5.3** `cache_state_badge` + `cache_state_summary_badge` UI widgets
  UNCHANGED (zero UI files touched).
- **R5.4** Workspace lib tests (411 ui + non-ui) stay green.

### R-NR — Cross-cutting

- **R-NR.1** Zero new design tokens.
- **R-NR.2** Zero new `strings.rs` entries.
- **R-NR.3** `cargo fmt --check` + `clippy -D warnings` clean on touched
  paths (pre-existing 9 ui clippy errors carried over per v0.1.2 budget).
- **R-NR.4** spec-lint baseline-stable (current 78/5 confirmed M0 — no
  NEW categories from v0.1.3).

## Operator-decide Q-rows

- **Q1 — Body-migration scope (LOAD-BEARING; durable per AGENT.md 2026-05-28).**
  (a) **[Recommended — DURABLE]** extract canonical Yahoo report-emit helper;
  migrate `run_yahoo_sma.rs` to use it. Future Yahoo emitters (MACD/RSI/BBands
  at v0.2.0+) inherit byte-identically with zero per-binary re-implementation.
  ~1.5 days dev. (b) **[cheap fallback]** fix only `run_yahoo_sma.rs` inline,
  no helper. ~0.5 days now but +0.5-1 day v0.2.0 cleanup PER new Yahoo binary
  (3 planned = +1.5-3 days follow-on, 3 drift-risk events). Net wall-clock ≥
  (a). *Analyst recommends (a)* — helper extraction is correct regardless of
  emitter count, but MUST land before the second emitter exists or retrofit
  costs more than doing it now.

- **Q2 — Anchor namespace for re-emitted BTC row 69.**
  (a) **[Recommended — DURABLE]** update SHA in-place under existing namespace
  `lab-yahoo-realdata-v0.1.1` (matches v5 v0.3.0+v0.4.0 in-place precedent;
  single Yahoo namespace per BTC row keeps future v0.1.4+ tracking one origin).
  (b) **[cheap fallback]** introduce new namespace `lab-yahoo-realdata-v0.1.3`
  for re-emitted BTC; leave row 69 as old-shape pin. Semantically clean
  per-ship but creates a 2nd Yahoo namespace v0.1.4 / v0.2.0 must reconcile.
  *Analyst recommends (a)* — precedent established, lower bookkeeping.

## Risks / falsifiers (K-rows)

- **K1 — Body migration breaks anchor body-SHAs for OTHER Yahoo
  reports.** *Mitigation*: pre-ship grep for ALL `rev=` occurrences in
  any v0.1.2+ Yahoo report body; developer confirms only
  `run_yahoo_sma.rs` emits it. If a forgotten emitter exists, route back
  to architect.
- **K2 — `revision_sha:` frontmatter key collides with existing
  frontmatter key.** *Mitigation*: M-T1 architect grep against existing
  Yahoo report frontmatter + a `spec_lint` check; analyst pre-confirms
  v0.1.1 + v0.1.2 BTC/ETH reports have no `revision_sha:` key today.
- **K3 — Binance ETHUSDT 2024 parquet schema differs from BTCUSDT.**
  *Mitigation*: developer pre-flight at T-D1 (open one parquet, confirm
  schema parity). 12 parquets present per v0.1.2 tester — schema risk
  low.
- **K4 — H1 ETH Yahoo-daily vs Binance-hourly divergence > 30%.**
  *Mitigation*: very low prior probability. If it fires, route back to
  analyst with operator-decide on (i) widening threshold (data-driven
  justification) or (ii) accepting v0.1.2 K1 fallback as carry-forward.

## Hypotheses (H-rows)

- **H1** — Yahoo ETH-USD daily vs Binance ETHUSDT hourly H1 2024
  divergence < 30% direct (replaces v0.1.2 K1 fallback). Falsifier: ≥ 30% → K4.
- **H2** — body→frontmatter migration preserves cache populate + Lab
  dispatch behavior byte-identically (emit shape changes; consumer
  surface does not). Falsifier: any non-emit-shape diff in loaded bars
  or dispatch output → revert R1.
- **H3** — Future Yahoo report emitters pick up the new contract via
  the shared helper with zero per-bin re-implementation. Falsifier:
  helper shape forces per-bin override at first new emitter — refactor
  scope grew.

## Cost framing (durable-vs-quick per AGENT.md 2026-05-28)

| Phase | Owner | Estimate |
|---|---|---:|
| M0 / M-OD / M-T1 (fast-skip likely) | analyst+operator+architect | ~1h combined |
| M-DEV Q1=(a) [Recommended] | developer | ~1.5 days |
| M-DEV Q1=(b) [cheap fallback] | developer | ~0.5 days + 1.5-3 days v0.2.0+ |
| M-FINAL + M-PRESENTER | tester+presenter | ~1-1.5h combined |
| **Total wall-clock — Q1=(a)** | — | **~1.5-2 days** |

Q1=(a) is +1 day at v0.1.3 vs Q1=(b) but -1.5 to -3 days follow-on
across v0.2.0+ emitters. Net wall-clock strictly better; durability
strictly better; zero downside. Q1=(b) IS cheaper now, only by deferring
cost into 3 future shipping events with drift-risk amplification.

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

ALL GREEN gates: `verify_anchors.sh` 71/71 + `cargo fmt --check` +
`cargo clippy -D warnings` (touched paths) + workspace lib tests green +
spec-lint no NEW categories vs 78/5 baseline + H1 direct PASS + H2
emit-shape no-functional-regression PASS + H3 helper retrofit-free
verified by code inspection.

## References

- v0.1.2 predecessor + tester report (root-cause of both defects):
  [`spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/feature.md`](../lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/feature.md),
  [`reports/test-final-2026-05-28-...md`](../lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge.md).
- Binary to migrate: [`crates/backtest/src/bin/run_yahoo_sma.rs`](../../crates/backtest/src/bin/run_yahoo_sma.rs)
  (`Data source: ... rev=...` line at L259).
- Scenario-registry to extend: [`crates/backtest/src/main.rs`](../../crates/backtest/src/main.rs)
  (existing `btc-2024-h1-sma-cross` arm at L242 — template).
- Anchors registry: [`spec/anchors.toml`](../anchors.toml) (row 69 BTC
  in-place; row 71 new ETH H1 append).
- ADR-0040 § Changelog — architect amends per D-V0.1.2-6 ADR-extend-not-new
  precedent: [`spec/architecture/adr/0040-yahoo-realdata-path.md`](../architecture/adr/0040-yahoo-realdata-path.md).

## Design

> **M-T1 ratification 2026-05-28 (architect).** Operator picked Q1=(a)
> helper-extraction and Q2=(a) in-place SHA under existing namespace
> `lab-yahoo-realdata-v0.1.1`. Both picks ratified verbatim under the
> AGENT.md 2026-05-28 durable-over-quick contract. M-T1 is **NOT a
> fast-skip** — Q1=(a) introduces a real new module boundary
> (`crates/backtest/src/report/yahoo.rs`) and an operator-facing
> frontmatter contract (`revision_sha:`). ADR-0040 § Changelog is
> amended (not superseded — no new ADR), but the Design § below locks
> the helper shape such that future Yahoo emitters (MACD/RSI/BBands
> at v0.2.0+) CANNOT regress the body→frontmatter migration.

### D-V0.1.3-1 — Q1 ratification: canonical Yahoo report-emit helper

**Decision.** Extract `crates/backtest/src/report/yahoo.rs` as the
**single point of truth** for all Yahoo-cache-sourced backtest report
emission. The module wraps existing strategy-specific writers
(`report::sma::write` today; `report::macd::write` / `_rsi` / `_bbands`
at v0.2.0+) and owns the two Yahoo-specific concerns:

1. **Data-source string formation.** The body line becomes
   `Data source: yahoo-cache:{ticker}/1d/2024` with **no `rev=`
   substring**. The helper is the only constructor of this string;
   `run_yahoo_*` binaries no longer hand-format it.
2. **`revision_sha:` frontmatter injection.** The full 64-char hex
   aggregate SHA from `data/yahoo/REVISION.toml` is written as a NEW
   top-level frontmatter line `revision_sha: <64 hex>`, ordered
   immediately AFTER `data_source:` (consistent placement across
   strategies).

**Public API contract (architect-locked; developer chooses naming detail):**

```text
crates/backtest/src/report/yahoo.rs

pub struct YahooReportContext<'a> {
    pub ticker: &'a str,              // e.g. "BTC-USD"
    pub interval: &'a str,            // e.g. "1d" (mirrors fetch cadence)
    pub year: u16,                    // e.g. 2024
    pub revision_sha: &'a str,        // full 64-char hex from REVISION.toml
}

impl<'a> YahooReportContext<'a> {
    /// Returns the body Data-source line WITHOUT `rev=`.
    /// Single point of truth — no other caller may format this.
    pub fn data_source(&self) -> String;
    // → "yahoo-cache:{ticker}/{interval}/{year}"
}

/// SMA emission (today's only consumer).
pub fn emit_sma_report(
    ctx: &YahooReportContext<'_>,
    sma_input: &SmaScenarioInput,
    state: &BacktestState,
    initial_capital: Decimal,
    final_equity: Decimal,
    seed: u64,
    elapsed_secs: f64,
    report_path: &Path,
    strategy_meta: &StrategyMeta,
) -> Result<()>;

// v0.2.0+ additive: pub fn emit_macd_report(ctx, ...);
//                   pub fn emit_rsi_report(ctx, ...);
//                   pub fn emit_bbands_report(ctx, ...);
```

**Underlying mechanism (developer-tactical).** `report::sma::write`
gains a NEW optional parameter `revision_sha: Option<&str>` (or a
`YahooFrontmatterExt` struct passed via the existing `SmaScenarioInput`).
When `Some(sha)`, the writer emits `revision_sha: {sha}` immediately
after `data_source:`. When `None`, behavior is byte-identical to today
— THIS PRESERVES THE 33 EXISTING NON-YAHOO SMA ANCHORS BYTE-IDENTICALLY
(Binance SMA path passes `None`). The 1 affected Yahoo SMA anchor
(row 69 BTC) gets a new SHA in-place per Q2=(a).

**Helper landing contract.** `run_yahoo_sma.rs` becomes the FIRST
consumer of `report::yahoo::emit_sma_report`. The migration deletes
the inline `format!("yahoo-cache:{ticker}/1d/2024 rev={revision_sha:.12}")`
at L259 and replaces the direct `report::sma::write` call with a single
`report::yahoo::emit_sma_report(&ctx, &sma_input, ...)` invocation.
Any future `run_yahoo_macd`, `run_yahoo_rsi`, `run_yahoo_bbands`
binary MUST route through `report::yahoo::emit_*` — direct
`report::{macd,rsi,bbands}::write` calls from Yahoo binaries are
prohibited by convention (enforced by code review + the K1 grep
post-condition: `grep -RIn "rev=" crates/backtest/src/bin/run_yahoo_*.rs`
returns zero).

**Why this shape is durable.** The body-vs-frontmatter split lives in
`report/yahoo.rs` — exactly one place. Adding a future Yahoo emitter
costs:

- 1 new `pub fn emit_{strategy}_report(ctx, input, ...)` thin wrapper
  in `report/yahoo.rs` (~10 LoC),
- 1 new optional `revision_sha: Option<&str>` arg threaded into the
  underlying `report::{strategy}::write` (mechanical, Binance path
  passes `None`).

A future Yahoo emitter that hand-formats the data_source string OR
that bypasses `emit_*_report` and calls `report::{strategy}::write`
directly with `Some(sha)` would surface as a missing-helper-call code
review smell. The convention is enforceable.

### D-V0.1.3-2 — K1 grep falsifier result (architect-verified 2026-05-28)

```bash
# Production-binary scope: only `rev=` in any production emit body.
$ grep -rn "rev=" crates/backtest/src/
crates/backtest/src/bin/run_yahoo_sma.rs:259:
  let data_source = format!("yahoo-cache:{ticker}/1d/2024 rev={revision_sha:.12}");
```

**Verdict: PASS.** `rev=` substring appears in exactly ONE production
binary (`run_yahoo_sma.rs:259`). The companion test
`crates/backtest/tests/run_yahoo_sma_ticker_flag.rs:165` mentions it
in a comment only (not an emit). Zero leak into
`run_binance_momentum.rs`, `run_pairs.rs`, `run_tcn_overlay.rs`, or
any future Yahoo binary (none exist yet). Q1=(a) extraction scope is
correctly bounded to the one binary; no analyst loop-back required.

**Wider crate-graph sweep** (`grep -rn "rev=" crates/`) surfaces 4
additional hits — all are non-emit math comments (`r_prev=0`,
`sigma_prev=0` in `forecast/garch.rs:482` and
`strategy/vol_targeting_overlay.rs:479`; `prev=`/`cur=` debug
in `forecast/features.rs:497`). **None mutate report bodies; all
out-of-scope.**

### D-V0.1.3-3 — K2 grep falsifier result (architect-verified 2026-05-28)

```bash
$ grep -RIn "revision_sha:" spec/lab-yahoo-realdata*/reports/
# (no matches)

# Existing frontmatter shape sample (v0.1.2 ETH row 70):
---
scenario: eth-yahoo-2024-1d-sma-cross
seed: 0xC0FFEE
generated: 2026-05-27T21:56:52Z
wall_clock_s: 0.0
data_source: yahoo-cache:ETH-USD/1d/2024 rev=e018f876c36a
baseline_report: n/a
...
---
```

**Verdict: PASS.** No existing Yahoo report frontmatter contains a
`revision_sha:` key. Insertion is collision-free. Post-migration shape
(BTC row 69 re-emit):

```yaml
---
scenario: btc-yahoo-2024-1d-sma-cross
seed: 0xC0FFEE
generated: 2026-05-XXT...
wall_clock_s: 0.0
data_source: yahoo-cache:BTC-USD/1d/2024
revision_sha: <64 hex>
baseline_report: n/a
...
---
```

Ordering rationale: `revision_sha:` immediately AFTER `data_source:`
keeps related fields adjacent and matches the visual locality of the
old inline `rev=` suffix.

### D-V0.1.3-4 — Q2 ratification: in-place SHA under existing namespace

**Decision.** Anchor row 69 `btc-yahoo-2024-1d-sma-cross` SHA updates
in-place under existing namespace `lab-yahoo-realdata-v0.1.1`. Same
row index, same namespace label, new SHA value (because removing
`rev=` from the body bytes mutates the body-SHA).

**Precedent.** v5 v0.3.0+v0.4.0 in-place re-emit pattern (3 occurrences
in `spec/anchors.toml` under `v3.0.0-llm-forecaster` and
`v2.6.0-realdata` where wiring-bug fixes shipped SHA-mutating
re-emissions under the original namespace per ADR-0038 § D6.b).
Single Yahoo namespace per BTC row keeps v0.1.4+ ticker-fetch tracking
to one origin — critical for the v0.1.4 BNB → LINK bulk re-emit
where 9 Yahoo tickers will need consistent per-row namespace
attribution.

**ADR-0038 § D6 wiring-bug-fix re-emission protocol applies.** This is
NOT a documentation-link sweep (§ D6.c hypothetical); it IS a body-shape
fix that mutates the body-SHA by design. The protocol requires (per the
M-FINAL gate): tester recomputes the BTC SHA via independent re-run
≥ 2 times → byte-identical → updated in-place in `spec/anchors.toml`
under `lab-yahoo-realdata-v0.1.1` namespace label preserved. Anchor
count stays 70 for this row (in-place, not append).

### D-V0.1.3-5 — Binance ETH H1 scenario registration shape

**Decision.** New scenario `eth-2024-h1-sma-cross` registered in
`crates/backtest/src/main.rs` at the **three** existing match-arm
sites that the `btc-2024-h1-sma-cross` predecessor touches:

| Site | Line (today) | Pattern |
|---|---:|---|
| Scenario config dispatch | L242 | `"btc-2024-h1-sma-cross" => Ok(Self { symbol: Symbol::new("BTCUSDT"), start_year: 2024, bar_count: 262_800, strategy: ScenarioStrategy::SmaCrossover{20,50}, ... })` |
| Synthetic fallback start price | L1029 | `"btc-2024-h1-sma-cross" => dec!(42_000)` |
| `scenario_to_feature` mapping | L1762 | grouped under `"v0-paper-sma"` |

`eth-2024-h1-sma-cross` mirrors L242 verbatim with **only** the
following deltas:

- `symbol: Symbol::new("ETHUSDT")`
- `bar_count: 262_800` unchanged (H1 2024 ≈ 8760 bars; the predecessor's
  `262_800` is the 1m equivalent — the developer must verify the
  correct bar_count for H1 cadence at T-D4; recommended `8_760` for
  1h × 365d. **ARCHITECT NOTE: this is a known M-DEV decision point
  — the architect cannot lock without checking the predecessor's actual
  cadence. If `btc-2024-h1-sma-cross` is in fact 1m-as-labeled-H1
  (sloppy naming), then `eth-2024-h1-sma-cross` mirrors verbatim. If
  predecessor is true H1, eth uses 8_760. Developer T-D4 pre-flight
  resolves.**

L1029 synthetic-fallback gets a new arm `"eth-2024-h1-sma-cross" =>
dec!(2_400)` (ETH 2024 opening price ~$2,400). L1762 extends the SMA
group to:

```rust
"btc-2023-1m-sma-cross" | "btc-2023-1m-sma-baseline-refresh"
| "btc-2024-h1-sma-cross" | "eth-2024-h1-sma-cross" => "v0-paper-sma",
```

`scenario_to_feature` points to `"v0-paper-sma"` — NOT to
`lab-yahoo-realdata-v0.1.3` — because the feature label maps to the
crate-report-dir convention (`spec/v0-paper-sma/reports/`), which is
where the runtime emits report files. The anchor entry under
`spec/anchors.toml` IS namespaced `lab-yahoo-realdata-v0.1.3` (anchor
namespace ≠ report-dir feature), matching the v5/v3 pattern.

**Out of D-V0.1.3-5 scope (deferred to developer T-D4 pre-flight):**
correct `bar_count` for H1 cadence (architect calls out the ambiguity,
developer resolves with parquet count check).

### D-V0.1.3-6 — Anchor migration plan (cascade 70 → 71)

| Row | Scenario | v0.1.2 SHA | v0.1.3 outcome | Namespace |
|---:|---|---|---|---|
| 1–68 | (non-Yahoo) | various | **byte-identical** | preserved |
| 69 | `btc-yahoo-2024-1d-sma-cross` | `8045623b…` | **SHA updates in-place** (body bytes change with `rev=` removal) | `lab-yahoo-realdata-v0.1.1` (preserved per Q2=(a)) |
| 70 | `eth-yahoo-2024-1d-sma-cross` | `e59a5f87…` | **byte-identical** (NOT re-emitted at v0.1.3; bulk Yahoo ticker re-emit deferred to v0.1.4 BNB ship) | `lab-yahoo-realdata-v0.1.2` (preserved) |
| 71 | `eth-2024-h1-sma-cross` | — | **NEW append** | `lab-yahoo-realdata-v0.1.3` (new) |

**Net anchor count: 70 → 71** (1 in-place BTC update + 1 new ETH H1
row; row 70 ETH daily NOT re-emitted at v0.1.3 — explicit deferral
to v0.1.4 BNB bulk re-emit to amortize the body→frontmatter migration
cost across all 9 unanchored crypto-mirror tickers in one shipping
event).

**Why row 70 ETH daily is NOT re-emitted at v0.1.3.** Two options
considered:

- (A) Re-emit row 70 at v0.1.3 alongside row 69 — costs 1 extra
  determinism re-run + 1 extra namespace bookkeeping decision (does
  it stay `lab-yahoo-realdata-v0.1.2` or move to `lab-yahoo-realdata-v0.1.3`?).
- (B) **CHOSEN: defer to v0.1.4 BNB.** Row 70 stays byte-identical at
  v0.1.3; v0.1.4 amortizes the body→frontmatter migration across BNB
  + SOL + XRP + ADA + DOGE + AVAX + DOT + LINK + ETH-redo (9 tickers
  in one ship). Single bulk migration is simpler than 1+1+9 staged.

Architect ratifies (B) — analyst's framing was correct.

### D-V0.1.3-7 — ADR-0040 § Changelog amendment (NOT new ADR)

**Decision.** Helper extraction is amendment-worthy, not new-ADR-worthy.
Rationale:

- The frontmatter `revision_sha:` field is **mechanical extraction**
  from the existing inline `rev=` substring — same data, same source
  (`data/yahoo/REVISION.toml` aggregate), same full-64-char-hex
  representation. NOT a new operator-facing data contract; just a
  cleaner placement.
- The `crates/backtest/src/report/yahoo.rs` helper is a **refactor of
  existing emit logic**, not a new architectural primitive. It does
  not introduce a new module type (parallel to `report/sma.rs`,
  `report/momentum.rs`, etc.) — it wraps them.
- D-V0.1.2-6 precedent (2026-05-27) shipped a comparable per-ticker
  scaling pattern + cache-state UI surface as a Changelog amendment;
  this is structurally similar.
- The CLAUDE.md non-negotiable on dep changes is not triggered (no
  new crate; helper uses existing `report::sma` + frontmatter
  string-formatting only).

A new ADR-0050 would be warranted IF (a) the helper introduced a
report-emit trait `pub trait YahooEmitter` with multiple downstream
impls, or (b) the `revision_sha:` field became consumer-facing (UI
widget, verifier script reads it). Neither is true at v0.1.3 — `revision_sha:`
is grep-target metadata for `verify_anchors.sh` and future emitters,
not a user-facing surface.

**ADR-0040 § Changelog amendment shape** (≤50 lines, locked at T-T1.4):

> 2026-05-28 (architect, M-T1 lab-yahoo-realdata-v0.1.3): body→frontmatter
> migration for the `rev=<sha>` substring in Yahoo report emissions, plus
> registration of `eth-2024-h1-sma-cross` Binance H1 scenario.
> **No new architectural decisions** — operationalises existing D3
> (`REVISION.toml` aggregate SHA) + extends the per-ticker scaling
> pattern from D-V0.1.2-6 to the report-emit boundary. Two operational
> extensions: (1) **Canonical Yahoo report-emit helper.**
> `crates/backtest/src/report/yahoo.rs` becomes the single point of
> truth for Yahoo-cache-sourced report emission. The body
> `Data source: yahoo-cache:{ticker}/1d/2024 rev={sha:.12}` shape (D-V0.1.2-6
> default) loses the `rev=<sha>` suffix; the full 64-char hex moves to
> a new top-level frontmatter line `revision_sha:` immediately after
> `data_source:`. Underlying strategy report writers
> (`report::sma::write`, future `_macd`/`_rsi`/`_bbands` at v0.2.0+)
> gain an optional `revision_sha: Option<&str>` parameter — `None`
> preserves byte-identical output for the 33 Binance SMA anchors and
> all non-Yahoo emitters. Anchor row 69 BTC SHA updates in-place under
> namespace `lab-yahoo-realdata-v0.1.1` (Q2=(a) precedent: v5 v0.3.0+v0.4.0
> in-place re-emit; ADR-0038 § D6.b wiring-bug-fix re-emission
> protocol applies). Row 70 ETH daily byte-identical at v0.1.3 — bulk
> Yahoo ticker re-emit deferred to v0.1.4 BNB ship to amortize across
> 9 unanchored tickers. (2) **Binance ETH H1 scenario registration.**
> `eth-2024-h1-sma-cross` arm appended to `crates/backtest/src/main.rs`
> at three match-arm sites (L242 scenario config, L1029 synthetic
> fallback start price `dec!(2_400)`, L1762 `scenario_to_feature →
> "v0-paper-sma"`); retires the v0.1.2 Yahoo-to-Yahoo K1 fallback in
> favor of direct Yahoo-daily-vs-Binance-hourly H1 discharge. New
> anchor row 71 under namespace `lab-yahoo-realdata-v0.1.3`. Net
> anchor count 70 → 71. Closes T-T1.4 of
> `spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/tasks.md`.

### D-V0.1.3-8 — Determinism + non-regression contract

- **Byte-identity gates** (R5.1, R5.2): rows 1-68 (non-Yahoo) +
  row 70 (ETH daily v0.1.2 `e59a5f87…`) byte-identical. Verified by
  `verify_anchors.sh` 71/71 PASS at T-D9 and independently by tester
  T-F1.
- **In-place re-emit determinism** (R3.4): row 69 BTC re-emitted ≥ 2
  independent runs at T-D6; byte-identical. New SHA enters
  `spec/anchors.toml` only after second-run confirmation.
- **New anchor determinism** (R2.4): row 71 ETH H1 re-emitted ≥ 2
  independent runs at T-D7; byte-identical. Append-only insertion.
- **Helper-shape regression test (H3 falsifier).** Recommended
  developer-added test: `crates/backtest/tests/yahoo_report_helper_shape.rs`
  asserting that the emitted BTC report body contains zero `rev=`
  substrings AND that the frontmatter contains exactly one
  `revision_sha:` line matching the full 64-char hex in
  `data/yahoo/REVISION.toml`. Locks the H3 falsifier (future emitter
  retrofit-free) at compile time.

### D-V0.1.3-9 — File-scope contract for M-DEV

Developer touches ONLY:

- `crates/backtest/src/report/yahoo.rs` — NEW (T-D2).
- `crates/backtest/src/report/mod.rs` — add `pub mod yahoo;` (T-D2).
- `crates/backtest/src/report/sma.rs` — add optional `revision_sha:
  Option<&str>` parameter; `None` arm preserves byte-identical
  Binance path (T-D2).
- `crates/backtest/src/bin/run_yahoo_sma.rs` — migrate L259 call site
  to `report::yahoo::emit_sma_report` (T-D3).
- `crates/backtest/src/main.rs` — add three `eth-2024-h1-sma-cross`
  match arms (T-D4, T-D5).
- `spec/anchors.toml` — row 69 SHA in-place; row 71 append (T-D8).
- `spec/dev-notes/yahoo-vs-binance-eth-h1-2026-05-XX.md` — NEW H1
  re-discharge dev-note (T-D10).
- `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` — extend body-shape
  + frontmatter assertions (T-D11 recommended).

Developer does NOT touch:

- Any non-`report/yahoo.rs`, non-`report/sma.rs` file in
  `crates/backtest/src/report/` — Binance momentum/pairs/tcn_overlay
  paths stay byte-identical.
- Any `crates/data/src/` file — `YahooBarSource` API surface (ADR-0040
  § D5) unchanged.
- Any UI file — `cache_state_badge` + `cache_state_summary_badge`
  consume `data_source:` (which keeps the `yahoo-cache:` prefix) and
  do NOT read `revision_sha:`.
- `vendor/iced_tiny_skia/` — operator-locked 2026-05-20.

## Implementation

_Developer M-DEV fills this._

## Changelog

- 2026-05-28 (analyst, M0): brief authored — closes 2 architect-flagged
  design notes from v0.1.2 M-FINAL (rev= body pollution + missing
  Binance ETH H1 scenario). 5 R / 4 K / 3 H / 2 Q + non-regression
  contract + cost framing (~1.5-2 days Q1=(a) Recommended). Q1=(a)
  helper extraction labeled DURABLE; Q1=(b) inline-fix labeled cheap
  fallback per AGENT.md 2026-05-28 contract. Anchor delta 70 → 71
  (in-place BTC re-emit + 1 new ETH H1 Binance row). Trace row
  `REQ-LAB-YAHOO-REALDATA-V0-1-3-001` opened `proposed`. HANDOFF →
  architect.
- 2026-05-28 (operator, M-OD): both Q-rows resolved at recommended
  durable choices per AGENT.md 2026-05-28 durable-over-quick contract.
  Q1 = (a) helper-extraction. Q2 = (a) in-place SHA under existing
  namespace `lab-yahoo-realdata-v0.1.1`. M-T1 architect ratification
  next.
- 2026-05-28 (architect, M-T1): § Design ratifies Q1+Q2 verbatim and
  locks the helper boundary (D-V0.1.3-1 → D-V0.1.3-9). K1 grep
  falsifier PASS (`rev=` substring appears in exactly one production
  binary — `run_yahoo_sma.rs:259`); zero leak to other emitters
  guarantees Q1=(a) scope correctness. K2 grep falsifier PASS
  (`revision_sha:` collision-free against existing Yahoo report
  frontmatter). Helper module locked at
  `crates/backtest/src/report/yahoo.rs` with public API
  `emit_sma_report(&YahooReportContext, ...)`; future MACD/RSI/BBands
  binaries route through `emit_{macd,rsi,bbands}_report` byte-identically.
  Underlying `report::sma::write` gains optional `revision_sha:
  Option<&str>` parameter — `None` arm preserves byte-identity for 33
  Binance SMA anchors. ADR-0040 § Changelog amended (no new ADR);
  helper extraction labeled mechanical refactor + operator-facing
  frontmatter field labeled mechanical re-placement of existing inline
  `rev=` data. Anchor cascade locked: row 69 BTC SHA in-place under
  `lab-yahoo-realdata-v0.1.1` (Q2=(a) + ADR-0038 § D6.b protocol);
  row 70 ETH daily byte-identical (deferred to v0.1.4 BNB bulk
  re-emit); row 71 NEW `eth-2024-h1-sma-cross` under
  `lab-yahoo-realdata-v0.1.3`. Net count 70 → 71. ETH H1 `bar_count`
  cadence ambiguity flagged for developer T-D4 pre-flight (predecessor
  `btc-2024-h1-sma-cross` uses `262_800` which is 1m-equivalent; if
  true H1, use `8_760`; developer resolves with parquet count check).
  Owner flip → developer. Trace `state = arch-done`. HANDOFF →
  developer.
