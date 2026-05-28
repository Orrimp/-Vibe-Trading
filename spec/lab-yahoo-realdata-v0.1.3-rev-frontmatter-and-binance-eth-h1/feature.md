---
slug: lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1
version: 0.1.0
status: draft
owner: analyst
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

_Architect M-T1 fills this._

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
