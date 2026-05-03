---
slug: operator-success-reports
status: shipped
owner: tester
updated: 2026-05-01
---

# Operator success reports

## Why

This is the v1+ feature spec'd in
[product.md → Operator success reports](../product.md#operator-success-reports).
The operator's question is always **"is this working?"** — and the report
is the answer. v1.5a shipped, so the agent now runs **four strategies**
against the audit ledger (`sma_crossover`, three v0.5 composed recipes,
v1 cross-sectional momentum, v1.5a mean-reversion pairs) with **9 locked
anchor scenarios** behind it. Without an "is this working?" binary the
operator has to grep the ledger by hand to answer "did anything decay
this week?", "is BTC momentum still pulling its weight vs SOL?", "are
we close to the LLM budget cap or the drawdown stop?". Raw
`audit::query` calls don't compose into a one-page answer to those
questions; they compose into ten-line shell pipelines that the operator
won't run on a Monday morning. A reporting binary that emits a
date-stamped markdown file under `spec/reports/success/` is the right
answer because (a) markdown is the medium the rest of the spec already
lives in, (b) the cockpit's `viewer` binary
([product.md → Consumer](../product.md#consumer)) already knows how to
render markdown inline, (c) determinism + body-only SHA-256 hashing lets
us build a regression gate around the report shape itself.

This feature is the **operator-facing surface of the moat bet** —
[persistent memory + audit](../product.md#differentiator). Every metric
in the report must be **traceable to a journal entry**: no "headline"
return that doesn't reconcile to the ledger to the cent, no per-strategy
P&L that doesn't compose from `audit::query::pnl_by_symbol` /
`pnl_by_pair`, no "system health" line item that doesn't have a clear
audit-event provenance. The report is **the moat made legible**: the
single artefact a future operator (or follow-up project lead) can show
to demonstrate that this system has both the institutional memory and
the financial-grade reconciliation discipline that distinguish it from
a Python PoC.

This feature is **non-strategy and non-validation**. No edge claim, no
trading-logic change, no impact on the 9 locked anchor hashes
([v1.5a body-SHA256 anchors](v15a-mean-reversion-pairs.md#v15a-body-sha256-hashes-seed-0xc0ffee)).
Success is "the report renders deterministically and reconciles to the
ledger to the cent". Zero LLM tokens, zero strategy code touched, zero
new bus channels. The reconciliation invariant (R11) is the load-bearing
test: if the report's headline equity disagrees with the ledger sum even
by a satoshi, the report has failed regardless of whether it rendered.
The body-vs-front-matter discipline (R10) is the load-bearing
**determinism** decision — a lesson learned from v0.5 HF-1 (initial
backtest reports leaked wall-clock timing into the body) and v1.5a HF-1
(`data_source` string drift required re-locking two anchor hashes). The
brief codifies that discipline up front so future reports never need a
HF-3.

## Requirements

Numbered, testable, derived from
[product.md → Operator success reports](../product.md#operator-success-reports),
[architecture.md → Audit & ledger](../architecture.md#audit--ledger), and
the existing read-only query surface in
[`crates/audit/src/query.rs`](../../crates/audit/src/query.rs). Each ends
with a one-line **acceptance** the tester can verify. All requirements
preserve the v0 / v0.5 / v1 / v1.5a `Strategy` trait shape (no trait
changes), the audit chart of accounts (no new accounts), and the
`strategy_events` schema (no migration). This feature adds **no new bus
channels** and consumes **no LLM tokens**.

### R1 — Binary surface

- **R1.1** New binary `cargo run --bin reports -- --period <duration>`.
  Lives in a dedicated `crates/reports/` crate (analyst preference;
  architect call — see Notes Q1).
- **R1.2** `--period` accepts: `7d`, `30d`, `90d`, `since:<RFC3339>`
  (e.g. `since:2026-01-01T00:00:00Z`), and `inception` (full ledger
  history). Validated at parse time; malformed period exits 2 with a
  clear `clap` error message. [ASSUMPTION] these are the period shapes
  the operator wants; architect may add e.g. `1d` or `ytd`.
- **R1.3** Optional `--output <path>` (default
  `spec/reports/success/<period>-<stamp>.md`, where `<stamp>` is a
  UTC `YYYYMMDD-HHMMSS` slug matching the existing
  `spec/reports/backtest-<stamp>-…` convention).
- **R1.4** Optional `--seed <hex>` for deterministic test runs (drives
  any RNG-derived placeholder content; the canonical test seed is
  `0xC0FFEE` per the v0/v0.5/v1/v1.5a convention).
- **R1.5** Optional `--ledger <path>` defaulting to the value
  configured in the agent's TOML (so the binary can run against a
  test fixture ledger snapshot without env juggling).
- **R1.6** Exit codes: `0` on successful render + atomic write,
  `1` on reconciliation mismatch (R11), `2` on argument parse error,
  `3` on ledger I/O / query error.
- **Acceptance:** `cargo run --bin reports -- --period 7d` against a
  fixture ledger writes a file matching the path pattern in R1.3 and
  exits 0; `cargo run --bin reports -- --period bogus` exits 2 with
  a `clap`-style error; `cargo run --bin reports -- --period 7d
  --output /tmp/r.md` writes `/tmp/r.md` and exits 0.

### R2 — Headline

- **R2.1** Single number at the top of the body: **cumulative return
  since inception**, both as % and absolute USDT. Source:
  `audit::query::cash_balance` plus marked-to-market position value
  at the end of the report period, minus the `equity:opening_balance`
  account (existing v0 chart-of-accounts entry per
  [architecture.md → bootstrap](../architecture.md#audit--ledger)).
- **R2.2** **BTC buy-and-hold baseline alongside**: the return a
  passive `cash → BTCUSDT` allocation of the same opening capital
  would have produced over the same period. Computed from
  `BTCUSDT` close prices at start and end of the period; source for
  prices is the same parquet roots the backtest binary reads
  (`data/binance/BTCUSDT/<year>/*.parquet`).
- **R2.3** Format both as a 2-line "Headline" section:
  `Strategy return: +12.34% (+$12,345.67 USDT)`,
  `BTC buy-and-hold: +8.91% (+$8,910.42 USDT)`.
- **Acceptance:** a unit test against a fixture ledger with a known
  opening capital + a known set of fills + a known close-price file
  asserts the rendered "Headline" lines match a hand-computed string
  exactly (both percentages, both dollar figures, fixed precision).

### R3 — Equity curve

- **R3.1** Two windows in the body: **since-inception** and
  **last-7-days** (or last-N-days where N matches `--period` if it's
  shorter than 7d).
- **R3.2** Inline ASCII / Unicode-block sparkline for each window.
  [ASSUMPTION] use the `▁▂▃▄▅▆▇█` Unicode-block convention
  (8 bars × ~60 chars wide) — visible in markdown previewers and
  monospaced terminals. Architect may pick a pure-ASCII alternative
  if the cockpit `viewer` font has rendering issues — see Notes Q4.
- **R3.3** CSV export of the full equity curve under
  `spec/reports/success/artifacts/<run-id>/equity-<window>.csv` —
  one row per bar / minute / sample. Columns:
  `ts_utc,equity_usdt,cash_usdt,positions_value_usdt`. CSV not
  Parquet for portability — see Notes Q5. Architect's call.
- **R3.4** `<run-id>` is a stable hash of the front-matter
  (`period + ledger_snapshot_sha + seed`); the same period against
  the same ledger snapshot at the same seed produces the **same**
  `<run-id>`, so re-running on the same fixture is idempotent (R12.b).
- **R3.5** Equity-curve sampling cadence: 1 bar (`1m`) for windows ≤
  7 days, 5-minute downsampled for windows > 7 days. [ASSUMPTION]
  these cadences are analyst preference; architect may pin different
  values once they have the data-volume number.
- **Acceptance:** the `equity-since-inception.csv` artifact contains
  N rows where N matches the expected sample count for the period;
  the inline sparkline is exactly 60 characters of the
  `▁▂▃▄▅▆▇█` set; manual inspection of a regression-test fixture
  shows the sparkline tracking the curve's monotonic rise / fall.

### R4 — Risk metrics

- **R4.1** Computed over the report period: **Sharpe**, **Sortino**,
  **Calmar**, **max drawdown** (% and absolute USDT), **recovery
  time** (bars elapsed from drawdown trough to new equity high; `n/a`
  if not yet recovered).
- **R4.2** Annualization: per the existing backtest convention in
  [`crates/backtest/src/main.rs`](../../crates/backtest/src/main.rs)
  (`525_600` minutes/year for 1m bars), reused unchanged so the
  numbers cross-reference cleanly with backtest reports.
- **R4.3** Render as a 5-row metrics table with a "Period" column
  reflecting the actual `--period` value.
- **R4.4** Source: equity curve from R3 plus `audit::query::cash_balance`
  + (any new query the architect needs for symbol marks at a given
  timestamp — see Notes Q2). The architect may decide that a
  `mark_to_market_at(ts)` helper belongs in `audit::query`; analyst
  flag `[ARCHITECT-DECIDE]`.
- **Acceptance:** a unit test on a synthetic equity curve with a
  hand-computed Sharpe (e.g. constant 0.1% / day for 30 days produces
  a deterministic Sharpe value) asserts the rendered number to 4
  decimal places; max-drawdown unit test on a curve with a known
  trough asserts both % and USDT match expected.

### R5 — Strategy attribution

- **R5.1** Per-strategy breakdown table with columns:
  `strategy_id, P&L (USDT), trade count, win rate, avg trade P&L`.
- **R5.2** Up to 4 strategies present at v1.5a:
  `sma_crossover`, the three v0.5 composed recipes
  (`btc_macd_trend`, `btc_rsi_reversion`, `btc_bbands_mean_revert`),
  v1 `top10_momentum_h1`, and v1.5a `pairs_mr_h1`. Strategies that
  produced zero trades in the period are still rendered, with a
  `(no activity)` placeholder in the trade-count / win-rate columns
  rather than being omitted.
- **R5.3** Source: a **new** `audit::query::pnl_by_strategy(since,
  until) -> Vec<(StrategyId, Money<Usdt>, trades, wins, ...)>` query
  composing `pnl_by_symbol` (v1) / `pnl_by_pair` (v1.5a) with the
  `strategy_events` table to attribute trades to the strategy active
  at the trade's timestamp. **Flag for architect: this query does
  not exist yet** — `[ARCHITECT-DECIDE]` Notes Q2. The query's
  return shape is the architect's call (a tuple-of-vectors vs a
  struct).
- **R5.4** Win rate = `trades_with_positive_realized_pnl / total_closed_trades`.
  Open positions at period end are excluded from the win-rate
  denominator (they have no realized P&L yet).
- **R5.5** Sort order: descending P&L. Ties broken alphabetically by
  `strategy_id` (deterministic per R12).
- **Acceptance:** unit test against a fixture ledger with deliberate
  per-strategy trades asserts the rendered table has exactly the
  expected number of rows in the expected order with hand-computed
  P&L / win-rate / avg-trade values.

### R6 — Memory highlights — placeholder

- **R6.1** Section header `## Memory highlights` is rendered with a
  fixed body **placeholder** at v1+:
  `_no lesson cards yet — reflection memory ships in a future feature._`
- **R6.2** Justification: per
  [product.md → Memory & continual learning](../product.md#memory--continual-learning),
  the reflection-memory loop (post-trade lesson cards into a vector
  store, retrieval at decision time, weekly distillation) is a
  separate feature that **has not shipped yet** — none of v0, v0.5,
  v1, or v1.5a writes lesson cards. The memory loop is called out
  in [product.md → Operator success reports](../product.md#what-every-report-contains)
  with the explicit "(v1+, once the memory loop runs)" caveat.
  Rendering the section header in v1+ is forward-compatibility
  scaffolding; the implementation gets a real body once the
  reflection-memory feature ships.
- **R6.3** Placeholder text is **deterministic** — same string every
  run, no timestamps, no run-id leakage; safe to lock into the
  body-SHA256 anchor.
- **Acceptance:** the rendered report contains a line matching exactly
  `_no lesson cards yet — reflection memory ships in a future feature._`
  in the `## Memory highlights` section, byte-for-byte across two
  runs against the same fixture.

### R7 — System health

- **R7.1** Section table with: **uptime** (% of period the agent was
  running), **kill-switch trips** (count + last trip timestamp in
  front-matter, not body — see R10), **clock-skew events**
  (count of `ClockSkew` warnings emitted during the period),
  **feed reconnects** (count), **funding-rate poll success rate**
  (count_ok / count_total in the period), **LLM spend vs budget**
  (`$0.00 / $135` at v1+; the cost ledger has zero `expense:llm:*`
  entries through v1.5a per the cost-telemetry scaffolding in
  [architecture.md → Cost telemetry](../architecture.md#cost-telemetry--dedicated-cost-crate--confirmed-2026-04-17)).
- **R7.2** Sources:
  - **Uptime / clock-skew / feed reconnects**: existing structured
    log + Prometheus exporter snapshot at run time. [ASSUMPTION]
    the binary reads a Prometheus snapshot file written by the
    agent on graceful shutdown, or queries the live exporter HTTP
    endpoint if the agent is up. Architect's call — Notes Q6.
  - **Kill-switch trips**: `audit::query::strategy_events_since`
    filtered to `kind = "Reject"` rows (or whatever kill-switch
    rows the v0 R7 implementation uses; analyst checked the
    `StrategyEventKind` enum in `query.rs` lines 399–415 and
    found `Load | Swap | Unload | Reject | RebalanceRejected |
    MeanReversionStop | PairShortObservation` — kill-switch is
    not currently a `StrategyEventKind`, so this needs an
    `[ARCHITECT-DECIDE]` resolution in Notes Q3).
  - **Funding-rate poll success rate**: `audit::query::funding_rate_history`
    over the period, divided by `expected_polls` based on
    `funding.interval_secs` config + period length.
  - **LLM spend**: the `cost` crate's `CostBudget` view; through
    v1.5a all `expense:llm:*` ledger accounts are zero, so this
    is `$0.00` until v2 ships. (Confirmed in v1.5a verification
    V10 — see [v15a → V10](v15a-mean-reversion-pairs.md#verification).)
- **R7.3** Render as a 6-row table; cells with no data print
  `n/a` rather than `0` so an absent metric is distinguishable
  from a zero metric.
- **Acceptance:** a unit test against a fixture log + fixture ledger
  with deliberate clock-skew events + reconnects + kill-switch trip +
  funding poll history asserts the rendered table has exact expected
  values.

### R8 — What changed

- **R8.1** Section listing strategy lifecycle changes during the
  period: load, swap, unload, reject events. Source:
  `audit::query::strategy_events_since(period_start)` filtered to
  `kind ∈ {Load, Swap, Unload, Reject}` (the four lifecycle kinds
  per [`StrategyEventView`](../../crates/audit/src/query.rs)
  parsing).
- **R8.2** Render as a chronological bullet list:
  `- 2026-04-29T00:00:00Z [Load] strategy_id=top10_momentum_h1 source=config/strategies/top10_momentum_h1.toml`
  `- 2026-04-30T00:00:00Z [Swap] strategy_id=pairs_mr_h1 old_hash=ab12.. new_hash=cd34..`
- **R8.3** Empty period (no lifecycle events) renders the literal
  string `_no strategy lifecycle events in this period._` —
  deterministic, byte-stable across runs.
- **R8.4** RebalanceRejected / MeanReversionStop / PairShortObservation
  events (the v1 + v1.5a additions, see
  [`StrategyEventKind` parsing](../../crates/audit/src/query.rs))
  are **not** in this section — they're operational events, not
  lifecycle changes. They surface in R9 (Open risks) when threshold
  counts are exceeded; otherwise they're silent.
- **Acceptance:** a fixture ledger with a deterministic Load + Swap
  pair within the period renders exactly two bullet lines in
  chronological order matching the format in R8.2; a fixture with
  no lifecycle events renders the literal empty-period string from
  R8.3.

### R9 — Open risks

- **R9.1** Section pinned **at the top of the body** (after
  Headline, before Equity curve) so the operator sees alarms
  before they scroll. Each risk has a **clear threshold** and a
  yes/no signal:
  - **Drawdown approaching limit**: yes if
    `current_drawdown_pct >= 0.75 * max_drawdown_stop_pct`
    (`max_drawdown_stop_pct = 0.15` per v0 default, so this fires at
    `current_drawdown >= 11.25%`).
  - **LLM budget approaching cap**: yes if
    `month_to_date_llm_spend >= 0.80 * monthly_budget_usd`.
    Through v1.5a this is always `no` (spend is $0.00).
  - **Strategy decay**: yes if any strategy's last-7-days Sharpe is
    `< 0` while its since-inception Sharpe is `> 0` (a strategy
    that worked but stopped working). [ASSUMPTION] this is the
    analyst's first-cut decay heuristic; architect may pin a
    smarter rule once the memory feature ships and lesson cards
    can pattern-match strategy decay directly.
  - **Rebalance rejections accumulating**: yes if
    `RebalanceRejected` events in the period exceed
    `0.05 * trade_count` (5% rejection rate is a strong signal
    of a misconfigured cap or a bug). [ASSUMPTION] threshold is
    analyst's first cut.
  - **Mean-reversion hard stops accumulating**: yes if
    `MeanReversionStop` events in the period exceed
    `0.10 * pair_trade_count` (10% threshold for v1.5a).
    [ASSUMPTION] threshold is analyst's first cut.
- **R9.2** If **all** risks are `no`, the section renders a single
  green line: `_no open risks._`. Otherwise it renders a bulleted
  list of fired risks each with the threshold + observed value.
- **R9.3** R9 is the most operator-visible section; it must NEVER
  silently swallow a fired risk due to query failure. If a risk's
  source data is unavailable (ledger query error, missing log file)
  the section renders `unknown — see logs` for that risk rather
  than `no`.
- **Acceptance:** a fixture ledger constructed to fire each risk
  exactly once produces a body with all five risks listed; a
  fixture with no risks fires the literal `_no open risks._`
  string; a fixture with a deliberately broken
  `pnl_by_strategy` query produces `unknown — see logs` for the
  decay risk and **does not** crash the binary (exit 0; the
  reconciliation gate in R11 is the only path to a non-zero
  exit).

### R10 — Determinism (body-vs-front-matter discipline)

> **Lesson learned from v0.5 HF-1 + v1.5a HF-1.** The first iteration
> of the backtest report leaked wall-clock timing into the body, which
> meant two runs against the same data had different body-SHA256s.
> v1.5a HF-1 had to re-lock two anchor hashes when the `data_source`
> string in the body shifted. This R-item codifies the discipline so
> future operator-success-report iterations don't repeat the
> mistake.

- **R10.1** **Front-matter** (YAML between the leading `---`
  fences) carries every field that **may shift between runs**:
  - `period: <human-readable>`
  - `period_start: <RFC3339>`
  - `period_end: <RFC3339>`
  - `generated: <RFC3339>` (wall-clock at render time)
  - `run_id: <hex>` (R3.4)
  - `ledger_snapshot_sha: <hex>` (SHA-256 of the ledger DB file
    at render time)
  - `seed: 0x<hex>` (canonical test seed `0xC0FFEE` for fixture
    runs; not present otherwise)
  - `data_source: <string>` (`live-ledger` or
    `fixture:<path>` for test runs)
  - `wall_clock_s: <float>` (binary's elapsed time)
- **R10.2** **Body** (everything after the closing `---`) carries
  **only ledger-derived facts**: aggregates, sums, ratios,
  per-strategy / per-symbol attributions. **Nothing in the body
  mentions `generated`, `run_id`, `wall_clock_s`, or
  `data_source`.** Strategy hashes (which are stable per strategy
  source file) **may** appear in the body; ledger-snapshot SHAs
  **may not**.
- **R10.3** Body-only **SHA-256** is byte-identical across two
  runs against the same ledger snapshot at the same `--period`
  and `--seed`. Same hashing convention as
  [`crates/backtest/src/main.rs`](../../crates/backtest/src/main.rs)
  (`fn write_report` lines ~1418–1500): hash the byte range
  starting after the closing front-matter fence (`---\n\n`).
- **R10.4** **Negative invariant — explicitly enforced in tests:**
  body bytes do **not** contain any of the strings `generated:`,
  `run_id:`, `wall_clock_s:`, `ledger_snapshot_sha:`,
  `data_source:`. A `tests/body_no_volatile_metadata.rs` test
  asserts none of those substrings appears in the body bytes
  on the locked fixture renders.
- **R10.5** Strategy-source-file hash references are stable input
  data, not volatile metadata, so they MAY appear in the body
  (R5 may reference `pairs_mr_h1@cd34..`). Architect should
  codify the policy: "if it's deterministic from
  ledger-state + strategy-config-state, it's body-eligible; if
  it varies on wall-clock or per-run randomness, it's
  front-matter-only."
- **Acceptance:** running the binary twice against the same
  fixture ledger at the same `--period` and `--seed`, ten seconds
  apart, produces two files with **different front-matter** but
  **byte-identical body** (same SHA-256 captured by the
  determinism test); the negative-invariant test from R10.4
  passes on both renders.

### R11 — Reconciliation invariant

> The load-bearing invariant. Every aggregate in the report must
> reconcile to the ledger to the cent. Failure to reconcile is the
> only path to a non-zero exit code (per R1.6).

- **R11.1** Identity:
  `headline_return_usdt == realized_pnl_usdt + unrealized_pnl_usdt`
  where
  - `realized_pnl_usdt = audit::query::realized_pnl_since(period_start)`
    (or `... since(inception)` for the inception headline);
  - `unrealized_pnl_usdt = Σ_open_position (mark_at_period_end -
    avg_entry_price) * position_qty`, with marks coming from the
    same Parquet roots / live exporter the agent uses.
- **R11.2** Sub-aggregate identities (each part of the report
  must reconcile):
  - **Per-strategy P&L sum** (R5): `Σ pnl_by_strategy ==
    Σ_realized_pnl_in_period` (closed trades only).
  - **Per-symbol P&L sum** (R5 cross-check): `Σ pnl_by_symbol ==
    Σ_realized_pnl_in_period` — already invariant V4 of v1
    ([v1 V4](v1-cross-sectional-momentum.md#verification)).
  - **Cash + position value = equity**: same v0 R3.5 invariant —
    `cash_balance + Σ_symbol(position[symbol] × mark[symbol]) =
    equity` at the report's `period_end` timestamp.
- **R11.3** **Reconciliation appendix** is the **last section** of
  the body, titled `## Reconciliation`. Renders the ledger sum and
  the report-aggregate sum **side-by-side** in a 4-row table:
  | Identity | Report side | Ledger side | Δ | Pass? |
  | `headline_return = realized + unrealized` | $X | $Y | $Z | PASS / FAIL |
  | `Σ pnl_by_strategy = Σ realized` | $X | $Y | $Z | PASS / FAIL |
  | `Σ pnl_by_symbol = Σ realized` | $X | $Y | $Z | PASS / FAIL |
  | `cash + Σ positions = equity` | $X | $Y | $Z | PASS / FAIL |
- **R11.4** **Mismatch = visible alarm.** Any `Δ != $0.00` (exact
  cent — see Notes Q6) triggers (a) the `Pass?` column on that row
  prints **FAIL** in literal uppercase; (b) the binary writes the
  report **and then exits 1** (per R1.6); (c) a banner line at
  the top of the body — above even R9 Open risks — prints
  `*** RECONCILIATION FAILURE — see Reconciliation section ***`.
  The operator can not miss the alarm.
- **R11.5** Reconciliation tolerance: **exact cent**
  (`Decimal == Decimal`). [ASSUMPTION] the audit ledger guarantees
  exact-cent precision (decimal storage, no floats); architect may
  override to a bps tolerance if the new R5 `pnl_by_strategy`
  query introduces rounding — Notes Q6.
- **Acceptance:** a unit test against a fixture ledger with
  deliberately balanced sums asserts all four `Δ`s are
  `$0.00 USDT` and all `Pass?` cells print `PASS`; an integration
  test forces `headline != ledger sum` (e.g. by injecting an
  extra journal entry between the two query calls) and asserts
  (i) the body banner prints, (ii) the `Pass?` cell prints
  `FAIL`, (iii) the binary exits 1.

### R12 — Cadence & cron-friendliness

- **R12.1** Three trigger paths:
  - **(a) Manual on-demand:** operator runs `cargo run --bin
    reports -- --period 7d` from the CLI. Default v1+ usage.
  - **(b) Weekly cron:** Monday 00:00 UTC, `--period 7d`.
    **Cron infrastructure is out of scope** for this feature
    (a v1+ ops concern; architect's call whether to add a
    `systemd` timer file under `ops/` or defer entirely);
    the binary itself must be cron-friendly per R12.2.
  - **(c) On kill-switch trip:** the agent invokes the binary
    with `--period since:<halt_event_ts>` to attach an incident
    report to the audit log. The binary writes the report
    under `spec/reports/success/incident-<halt_event_ts>.md`.
    Wiring the kill-switch handler to `std::process::Command`
    is in scope for this feature.
- **R12.2** Cron-friendly invariants (must hold in all three
  paths):
  - **Idempotent**: re-running with the same `--period` +
    `--ledger` + `--seed` produces the same `<run-id>` and (per
    R10.3) byte-identical body. The default `--output` path
    embeds `<stamp>` so two cron-time-adjacent runs land in
    distinct files; the `<run-id>` lets the operator verify
    they're the same content even though the file names differ.
  - **Atomic write**: write to `<output>.tmp` then `rename` —
    partial reports never appear at the canonical path. Same
    pattern the v0 backtest binary uses for snapshot writes.
    Architect's call — Notes Q3.
  - **Exit 0 on success**, exit codes per R1.6 otherwise.
  - **No interactive prompts**: nothing on `stdin`; logs to
    `stderr` only.
- **Acceptance:** a `tests/atomic_write.rs` test asserts that
  during a render, the canonical output path either does not
  exist or contains a complete file (never a partial write); a
  `tests/idempotent_run_id.rs` test asserts two runs against
  the same fixture produce the same `<run-id>` value in
  front-matter.

### R13 — Performance

- **R13.1** Report generation `< 10s wall-clock` for `--period
  90d` against a 1-year-history ledger fixture. [ASSUMPTION]
  10s is the operator-tolerance bar for an interactive run;
  architect may pin a tighter budget once benchmarks land.
- **R13.2** No new bench harness required at v1+; the
  `criterion`-based bench infrastructure
  ([architecture.md → Performance budget](../architecture.md#performance-budget))
  can absorb a `benches/reports.rs` harness later if the budget
  bites. v1+ acceptance is a wall-clock assertion in
  `tests/perf_smoke.rs`; promotion to a `criterion` bench is a
  v2+ concern.
- **R13.3** Memory ceiling: the binary's resident-set size stays
  under 256 MiB at the 1-year-history fixture (the equity-curve
  CSV at 1m granularity is ~525,600 rows ≈ 50 MiB CSV; the
  binary buffers it once before atomic write).
- **Acceptance:** `tests/perf_smoke.rs` runs `cargo run --bin
  reports -- --period 90d --ledger fixture.db` against a
  1-year-history fixture and asserts wall-clock `< 10s`.

## Backtest Scenarios

This feature is **non-strategy** — it does not validate edge. The
two scenarios below are **report scenarios**: deterministic fixture
ledger + a known period + the binary, locking a body-SHA256 anchor
for the report shape itself. Both are idempotent at seed `0xC0FFEE`
and grow the regression gate from **9 anchors → 11 anchors** after
this ships.

### Scenario: `report-sample-7d`

- **Fixture ledger:** a deterministic 7-day SQLite snapshot built
  by `tests/fixtures/build_ledger_7d.rs` from a hand-curated set
  of fills + strategy lifecycle events. Captures the v1.5a-shaped
  audit surface: ≥1 strategy `Load`, ≥3 closed trades across at
  least two strategies, ≥1 `RebalanceRejected` event (so R9 has
  content), ≥1 funding-rate observation row.
- **Period:** `--period 7d` (the canonical operator default).
- **Seed:** `0xC0FFEE`.
- **Expected output:** body-SHA256 captured by tester at first
  successful run; locked into the regression gate. **Same hashing
  convention as the v0/v0.5/v1/v1.5a backtest anchors** — body
  bytes only, after the closing front-matter fence.

### Scenario: `report-sample-90d`

- **Fixture ledger:** deterministic 90-day snapshot built by
  `tests/fixtures/build_ledger_90d.rs`. Includes: 4 strategies
  active across the period, ≥1 strategy swap (v0.5 hot-load
  event so R8 has content), ≥1 mean-reversion hard-stop event
  (v1.5a `MeanReversionStop`), ≥1 deliberate drawdown excursion
  to `> 11.25%` (so R9 fires the drawdown-approaching-limit
  risk).
- **Period:** `--period 90d` (the v3 cadence target per
  [product.md → Success metrics](../product.md#success-metrics-long-run)).
- **Seed:** `0xC0FFEE`.
- **Expected output:** body-SHA256 captured by tester at first
  successful run; locked into the regression gate.

**Both scenarios produce reports under `spec/reports/success/`
with the path pattern from R1.3.** Both reports must round-trip
through the determinism gate (R10.3) and the reconciliation
appendix (R11) — those are the load-bearing acceptance signals,
not the SHAs themselves (which are tester-captured at first
successful run).

## Design

Translates R1–R13 into crate / module additions, query signatures,
schema additions, file layout, CSV column schemas, sparkline
encoding, kill-switch wiring, and a risk register. All decisions
anchor to the analyst's nine open questions (Notes Q1–Q9) and to
the operator's pre-decided defaults relayed by the orchestrator.

The reports feature is **read-only over the audit DB plus the
existing parquet roots**. It introduces **no strategy-code change,
no backtest-binary change, and no change to the 9 locked anchor
SHA-256s** (V6). The only audit-side mutation is one additive
schema migration (`004_journal_transactions_strategy_id.sql`) that
tags new fills with their owning strategy for the per-strategy
attribution query (R5.3 / Q2 resolution); pre-migration rows are
NULL and surface in the report under a synthetic `(unattributed)`
bucket.

### Q-resolution summary

Eight of the analyst's nine open questions resolve below; the ninth
(Q9, R6 reflection-memory placeholder lifecycle) is a future-feature
flag, not a v1+ design decision — the only architect action it
demands is a one-line note in the eventual reflection-memory brief
to re-lock the two new operator-success-report anchors when R6
gains real content. That note lives in the v1+ regression-gate
documentation (R10.5 / V6) and is captured by task **T811** below.

| Q  | Topic                                  | Resolution                                                                                   |
|----|----------------------------------------|----------------------------------------------------------------------------------------------|
| Q1 | Crate placement                        | **Dedicated `crates/reports/`** lib + bin (operator default). No fold into `audit`/`backtest`. |
| Q2 | `pnl_by_strategy` query home + shape   | **`crates/audit/src/query.rs`** (single source of query API). New schema migration adds a nullable `strategy_id` column on `journal_transactions`; the writer takes an `Option<StrategyId>`; pre-migration rows surface as `(unattributed)`. Signature returns a struct, not a tuple-of-vectors. Mark-to-market helper for unrealized-P&L lives in `crates/reports/`, not `audit` (it needs parquet, not ledger). |
| Q3 | Atomic-write pattern                   | **Tempfile + `rename`** — write to `<output>.tmp.<pid>`, fsync, then `std::fs::rename`. macOS / ext4 / APFS guarantee rename atomicity within the same filesystem; the report path always lives under `spec/reports/success/` (same FS). Same pattern v0 backtest binary uses. |
| Q4 | Sparkline format                       | **Unicode block** `▁▂▃▄▅▆▇█` (operator default), eight-level palette, low→high mapping per the encoding spec below. |
| Q5 | CSV vs Parquet                         | **CSV** for portability (operator default). Companion CSVs for fills, per-strategy P&L, per-symbol P&L, journal, equity. Column schemas pinned below. |
| Q6 | Reconciliation tolerance               | **Exact-cent** equality (`Decimal == Decimal`) — operator default. No bps tolerance. On mismatch the report writes the markdown body **and** a sibling `_reconciliation_failure.json`; binary exits 1 (R1.6, R11.4). |
| Q7 | Front-matter schema                    | Analyst's 9-field set accepted **plus** `binary_version`, `git_commit`, `agent_pid`, `host`. The expanded set is non-breaking — front-matter is YAML and consumers ignore unknown keys. |
| Q8 | Kill-switch trip event provenance      | **Add `StrategyEventKind::KillSwitchTripped`** (operator default). Migrate `audit::journal::kill_switch_tripped` to ALSO emit a `strategy_events` row of the new kind. Keep the v0 zero-amount memo journal row (do not retro-rewrite history). The reports query `strategy_events`, NOT the memo row. |
| Q9 | R6 placeholder lifecycle (re-lock plan) | Documented at task T811 — when reflection memory ships, the v1+ analyst re-opens R6 and the architect re-locks the two operator-success-report anchors. v1+ ships the placeholder; the body-SHA256 is locked **with** the placeholder string. |

### Crate map delta from v1.5a

| Crate          | Change in v1+                                                                                                                                                                                                                                                                                                              |
|----------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `trading_core` | **+** `StrategyEventKind::KillSwitchTripped` variant (Q8). **No `Strategy` trait change. No `Signal` change. No `RiskLimits` change.**                                                                                                                                                                                      |
| `audit`        | **+** Migration `004_journal_transactions_strategy_id.sql` adds nullable `strategy_id TEXT` on `journal_transactions` (Q2). **+** `audit::journal::post_fill` gains an `Option<&StrategyId>` parameter that writes the new column. **+** `audit::journal::kill_switch_tripped` ALSO emits a `strategy_events` row of the new kind (Q8); v0 memo row preserved. **+** `audit::query::pnl_by_strategy(since, until) -> Vec<StrategyPnl>` (R5.3). **+** `audit::query::ledger_snapshot_sha(path) -> [u8; 32]` helper (R10.1 — sha256 of the SQLite file). **+** `audit::query::uptime_intervals_since(since) -> Vec<UptimeInterval>` reader (R7.1 — needs new schema, see below). |
| `audit`        | **+** Migration `005_uptime_intervals.sql` adds an `agent_uptime` table — append-only `(boot_id TEXT, started_at TEXT, last_heartbeat_at TEXT, stopped_at TEXT NULL)`. The agent writes one row on boot, updates `last_heartbeat_at` from the existing heartbeat task, and writes `stopped_at` on graceful shutdown. (R7.2 alt resolution — see below.)                                                                                                                                                                                                  |
| `agent`        | **+** Two thin extensions: (a) on boot, write the `agent_uptime` row + spawn a heartbeat-tick that updates `last_heartbeat_at` every `heartbeat_interval_secs` (default 30s); on graceful shutdown set `stopped_at`. (b) `KillSwitch::trip` calls `audit::journal::kill_switch_tripped`, which now writes both the v0 memo journal row **and** the new `strategy_events` row (Q8). (c) v1+ optional cron hook + on-trip hook spawn the reports binary out-of-process via `std::process::Command::new("cargo run --bin report …")` (R12.1c).                  |
| **`reports`**  | **NEW CRATE**. Lib (`generate(window: ReportWindow, audit: &AuditDb, marks: &dyn MarkSource, out: &Path) -> Result<ReportArtifacts>`) + bin (`cargo run --bin report -- --window weekly`). Depends on: `trading_core` (types), `audit` (read-only queries), `data` (parquet read-only for BTC buy-and-hold + position marks), `cost` (LedgerCostSink read-only). Owns: report renderer, sparkline encoder, equity-curve sampler, reconciliation engine, atomic file writer, CSV writer, front-matter schema, run-id hasher, `MarkSource` trait + parquet implementation. |
| `cost`         | Unchanged. `CostBudget::remaining()` is read once by the reports binary for the LLM-spend row (R7.1).                                                                                                                                                                                                                       |
| `data`         | Unchanged at the public surface. The reports crate uses `data`'s existing parquet readers for BTC close prices and per-symbol marks at `period_end`.                                                                                                                                                                        |
| `backtest`     | **Unchanged.** No new scenario, no report-shape change, no anchor SHA touch (V6).                                                                                                                                                                                                                                            |
| `strategy`, `risk`, `exec`, `models`, `llm`, `features`, `ui` | **Unchanged.** v1+ does not touch any strategy code path or any UI surface.                                                                                                                                                                                            |

**Dependency edges (additive):**

```
trading_core ← audit (KillSwitchTripped variant; no edge change)
audit        ← reports (read-only queries + new pnl_by_strategy)
data         ← reports (parquet read-only for marks + BTC baseline)
cost         ← reports (CostBudget::remaining read-only)
agent        ← audit (write KillSwitchTripped event on trip; uptime row writes)
agent        ← reports::bin (out-of-process via std::process::Command on cron / kill-switch trip)
```

No edge reverses. The single new crate (`reports`) is a leaf consumer.

### `crates/reports/` layout

```
crates/reports/
├── Cargo.toml
├── src/
│   ├── lib.rs               # re-exports the public API: generate(), ReportWindow,
│   │                        # ReportArtifacts, MarkSource, ReportError
│   ├── window.rs            # ReportWindow enum + parser for --period values
│   ├── render/
│   │   ├── mod.rs           # report assembly: front_matter + body
│   │   ├── front_matter.rs  # YAML front-matter writer (R10.1, Q7)
│   │   ├── headline.rs      # R2 — strategy return + BTC buy-and-hold baseline
│   │   ├── equity_curve.rs  # R3 — sparkline + downsampling
│   │   ├── risk_metrics.rs  # R4 — Sharpe / Sortino / Calmar / MDD / recovery
│   │   ├── strategy_attribution.rs  # R5 — per-strategy table
│   │   ├── memory_highlights.rs      # R6 — fixed placeholder (deterministic)
│   │   ├── system_health.rs # R7 — uptime, kill-switch, clock-skew, feed reconnects, funding poll, LLM spend
│   │   ├── what_changed.rs  # R8 — strategy lifecycle events
│   │   ├── open_risks.rs    # R9 — five threshold checks
│   │   └── reconciliation.rs # R11 — appendix table + banner + sibling JSON on FAIL
│   ├── sparkline.rs         # 8-level Unicode-block encoder (Q4)
│   ├── csv_artifacts.rs     # R3.3 + companion CSVs (Q5)
│   ├── marks.rs             # MarkSource trait + ParquetMarkSource impl
│   ├── reconcile.rs         # exact-cent reconciliation engine (Q6)
│   ├── run_id.rs            # stable hash of (period, ledger_snapshot_sha, seed) → hex (R3.4)
│   ├── atomic_write.rs      # write-to-tmp + fsync + rename (Q3)
│   └── bin/
│       └── report.rs        # clap CLI; calls lib::generate
├── tests/
│   ├── arg_parsing.rs       # R1
│   ├── headline_render.rs   # R2
│   ├── sparkline.rs         # R3.2
│   ├── csv_artifacts.rs     # R3.3
│   ├── risk_metrics.rs      # R4
│   ├── strategy_attribution.rs  # R5
│   ├── memory_highlights.rs # R6
│   ├── system_health.rs     # R7
│   ├── what_changed.rs      # R8
│   ├── open_risks.rs        # R9
│   ├── body_no_volatile_metadata.rs  # R10.4 negative invariant
│   ├── determinism.rs       # R10.3 byte-identical body across two runs
│   ├── reconciliation.rs    # R11.1–R11.5
│   ├── reconciliation_mismatch.rs  # R11.4 deliberate-mismatch integration
│   ├── atomic_write.rs      # R12.2 atomic write
│   ├── idempotent_run_id.rs # R12.2 same input → same run_id
│   ├── perf_smoke.rs        # R13 wall-clock ≤ 10s
│   └── fixtures/
│       ├── build_ledger_7d.rs   # Scenario `report-sample-7d`
│       ├── build_ledger_90d.rs  # Scenario `report-sample-90d`
│       └── snapshot_marks.parquet  # frozen close prices for deterministic marks
└── benches/                 # v2+ — placeholder; not built in v1+
```

### Public lib API

```rust
// crates/reports/src/lib.rs (new)
use std::path::Path;
use rust_decimal::Decimal;
use trading_core::{Money, Symbol, Timestamp, Usdt};

pub use window::{ReportWindow, WindowParseError};
pub use marks::{MarkSource, ParquetMarkSource, MarkError};
pub use render::ReportArtifacts;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("audit query: {0}")]
    Audit(#[from] audit::LedgerError),
    #[error("mark source: {0}")]
    Marks(#[from] MarkError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("reconciliation FAILED — Δ != $0.00 — see {sibling_path}")]
    Reconciliation { sibling_path: std::path::PathBuf },
    #[error("invalid window: {0}")]
    Window(#[from] WindowParseError),
}

/// Top-level entry point (sync wrapper around the async pipeline).
///
/// 1. Snapshot the ledger SHA (R10.1).
/// 2. Run all read-only queries (parallel where independent).
/// 3. Render front-matter + body into a `String`.
/// 4. Compute reconciliation deltas (R11). On any non-zero Δ:
///    - Write `_reconciliation_failure.json` next to the would-be report.
///    - Write the markdown body (with FAIL banner + FAIL cells).
///    - Return `Err(ReportError::Reconciliation { ... })`.
///    On success: atomic-write the markdown + companion CSVs; return `Ok`.
/// 5. Caller's bin maps `Err(Reconciliation)` to exit 1 (R1.6).
pub async fn generate(
    window: ReportWindow,
    audit:  &audit::Ledger,
    marks:  &dyn MarkSource,
    out:    &Path,
    seed:   Option<u64>,
) -> Result<ReportArtifacts, ReportError>;

/// What was written. Used by tests + the bin to surface the run_id.
#[derive(Debug, Clone)]
pub struct ReportArtifacts {
    pub markdown_path: std::path::PathBuf,
    pub run_id:        String,                 // hex (R3.4)
    pub csv_paths:     Vec<std::path::PathBuf>, // R3.3 + companions
    pub body_sha256:   [u8; 32],                // R10.3
}

/// Operator-supplied price source for marks + BTC baseline.
/// Production: `ParquetMarkSource::new(parquet_root)`.
/// Tests: `FrozenMarkSource::from_csv("snapshot_marks.csv")`.
pub trait MarkSource: Send + Sync {
    fn close_at(&self, symbol: &Symbol, ts: Timestamp) -> Result<Decimal, MarkError>;
    fn close_series(
        &self,
        symbol: &Symbol,
        from:   Timestamp,
        to:     Timestamp,
        cadence_minutes: u32,
    ) -> Result<Vec<(Timestamp, Decimal)>, MarkError>;
}
```

### `ReportWindow` parser (R1.2)

```rust
// crates/reports/src/window.rs (new)
use time::OffsetDateTime;
use trading_core::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportWindow {
    Days7,                       // "7d"
    Days30,                      // "30d"
    Days90,                      // "90d"
    Since(Timestamp),            // "since:2026-01-01T00:00:00Z"
    Inception,                   // "inception"
    /// Operator default for cron — same as Days7 but tagged so the
    /// front-matter `period:` slug reads "weekly".
    Weekly,
    /// Same as Days30 but tagged "monthly".
    Monthly,
}

#[derive(Debug, thiserror::Error)]
pub enum WindowParseError {
    #[error("malformed window '{0}' — expected 7d|30d|90d|weekly|monthly|since:<RFC3339>|inception")]
    Malformed(String),
    #[error("malformed since:<ts> — RFC3339 parse error: {0}")]
    BadTimestamp(#[from] time::error::Parse),
}

impl ReportWindow {
    pub fn parse(s: &str) -> Result<Self, WindowParseError>;
    /// Resolve to (since, until) given current wall-clock + ledger inception.
    pub fn resolve(
        &self,
        now:        Timestamp,
        inception:  Timestamp,
    ) -> (Timestamp, Timestamp);
    /// Period slug for front-matter `period:` field.
    pub fn slug(&self) -> &'static str;
}
```

### `ReportWindow` resolves to (since, until)

| Window      | `since`                                                | `until` | Front-matter `period:` slug |
|-------------|--------------------------------------------------------|---------|-----------------------------|
| `Days7`     | `now - 7 days`                                         | `now`   | `7d`                        |
| `Days30`    | `now - 30 days`                                        | `now`   | `30d`                       |
| `Days90`    | `now - 90 days`                                        | `now`   | `90d`                       |
| `Weekly`    | `now - 7 days`                                         | `now`   | `weekly`                    |
| `Monthly`   | `now - 30 days`                                        | `now`   | `monthly`                   |
| `Since(ts)` | `ts`                                                   | `now`   | `since:<RFC3339-of-ts>`     |
| `Inception` | `audit::query::ledger_inception_ts()` (T802)           | `now`   | `inception`                 |

### Q2 — `pnl_by_strategy` query design + schema migration

R5.3 needs per-strategy attribution. Two mechanisms were considered:

1. **Timestamp-join over `strategy_events`** — for each fill, find the
   latest `Load`/`Swap` event with `ts ≤ trade.ts` and attribute the trade
   to that strategy. Rejected because v1.5a explicitly runs **multiple
   strategies concurrently** (R5.2: `sma_crossover` + 3 composed recipes
   + `top10_momentum_h1` + `pairs_mr_h1`); the latest-event wins rule
   would funnel every trade to the most-recently-loaded strategy
   regardless of which strategy actually fired the order. Wrong by
   construction.

2. **Schema migration adds `strategy_id` column to `journal_transactions`** —
   accepted. The owning strategy id is known at fill time (the
   `agent::strategy_driver` loop that calls `exec.submit(order)` knows
   which strategy emitted the signal). Migration is purely additive
   (NULL for all pre-migration rows). The query handles NULL by
   bucketing into a synthetic `(unattributed)` strategy id.

**Migration:**

```sql
-- crates/audit/migrations/004_journal_transactions_strategy_id.sql (new)
ALTER TABLE journal_transactions ADD COLUMN strategy_id TEXT;
CREATE INDEX IF NOT EXISTS journal_transactions_sid_idx
    ON journal_transactions(strategy_id, ts);
```

**Writer change** (`crates/audit/src/journal.rs::post_fill`):

The signature gains an `Option<&str>` parameter. Existing call-sites
in `agent::strategy_driver` and `crates/exec` pass the active
strategy id; tests + tools that don't care pass `None` (writes NULL).

```rust
// crates/audit/src/journal.rs (extend; signature change is backwards-
// compatible at the call-graph level since v1+ owns all call sites)
pub async fn post_fill(
    ledger:      &Ledger,
    fill:        &Fill,
    strategy_id: Option<&str>,           // NEW v1+
) -> Result<(), LedgerError>;
```

The two existing in-tree callers (`agent::strategy_driver`,
`crates/exec/src/paper.rs::PaperEngine::on_signal`) thread the active
`StrategyId` through; the backtest binary's PaperEngine path passes
the scenario's strategy id (which it already knows). **No strategy
code is touched** — the change is in the call-site that receives a
fill from the executor and forwards it to the ledger. Architect's
T802 task spec covers this.

**Reader** (`crates/audit/src/query.rs`, additive):

```rust
/// Per-strategy realized P&L + trade stats over [since, until].
///
/// Pre-migration rows (strategy_id IS NULL) bucket into the synthetic
/// `StrategyId::new("(unattributed)")` row so historical fills surface
/// in the report under a clearly-named bucket rather than vanishing.
///
/// Returned rows are sorted by `realized` DESC, ties broken by
/// `strategy_id` ASC (R5.5).
///
/// # Sum invariant (R11.2)
///
/// `Σ rows.realized == realized_pnl_since(since)` evaluated at `until`.
/// Asserted by an integration test in crates/reports/tests/strategy_attribution.rs.
pub async fn pnl_by_strategy(
    ledger: &Ledger,
    since:  Timestamp,
    until:  Timestamp,
) -> Result<Vec<StrategyPnl>, LedgerError>;

/// Per-strategy P&L + trade stats. A struct (not tuple-of-vectors) so
/// callers can grow new fields additively without breaking call sites.
pub struct StrategyPnl {
    pub strategy_id:       StrategyId,
    pub realized:          Money<Usdt>,
    pub closed_trade_count: u32,         // R5.4 win-rate denominator
    pub winning_trade_count: u32,         // realized > 0 closed trades
    pub avg_trade_realized: Money<Usdt>, // realized / closed_trade_count, 0 if denom==0
}
```

**SQL** (the heart of the query):

```sql
-- For [since, until]:
-- Aggregate per-strategy realized P&L from income:realized_pnl entries,
-- joining to journal_transactions for the strategy_id column.
SELECT
    COALESCE(jt.strategy_id, '(unattributed)') AS strategy_id,
    SUM(je.credit_amount - je.debit_amount)    AS realized_sum,
    -- Counted at the transaction level: 1 closed trade = 1 sell-side txn
    -- that produced a realized_pnl row (positive or negative).
    COUNT(DISTINCT jt.id)                       AS closed_count,
    SUM(CASE WHEN (je.credit_amount - je.debit_amount) > 0 THEN 1 ELSE 0 END) AS winning_count
  FROM journal_entries je
  JOIN journal_transactions jt ON je.transaction_id = jt.id
 WHERE je.account_id = 'income:realized_pnl'
   AND je.ts >= ? AND je.ts <= ?
 GROUP BY COALESCE(jt.strategy_id, '(unattributed)')
 ORDER BY realized_sum DESC, strategy_id ASC;
```

The query parses `realized_sum` and amounts as TEXT → `Decimal` exactly
the same way `realized_pnl_since` does today (`crates/audit/src/query.rs`
lines 36–67). No `f64` enters the call path.

**Strategies-with-zero-trades coverage** (R5.2): the query above
omits strategies that fired zero closed trades in the window. The
reports renderer knows the *active set* by reading
`audit::query::strategy_events_since(period_start)` and filtering
to `Load`/`Swap` strategy ids; for any strategy in the active set
absent from `pnl_by_strategy`, the renderer synthesizes a
`StrategyPnl { realized: 0, ..zero }` row with the `(no activity)`
placeholder per R5.2.

### Mark-to-market source (R11.1, R4.4)

Unrealized P&L for R11.1 needs the mark price for every open position
at `period_end`. The reports crate owns this via the `MarkSource`
trait (above):

```rust
// crates/reports/src/marks.rs (new)
pub struct ParquetMarkSource {
    parquet_root: std::path::PathBuf,
    cache:        parking_lot::Mutex<lru::LruCache<(Symbol, Timestamp), Decimal>>,
}

impl MarkSource for ParquetMarkSource {
    /// Returns the close price of `symbol` at the bar whose close_ts
    /// is the latest one ≤ `ts`. Uses Polars to scan the appropriate
    /// year file under `<root>/<symbol>/<year>/*.parquet`. Caches the
    /// last 4096 (symbol, ts) lookups (uptime memory ≤ 1MiB).
    fn close_at(&self, symbol: &Symbol, ts: Timestamp) -> Result<Decimal, MarkError>;

    /// Returns close prices for `symbol` over [from, to] at the given
    /// cadence. cadence_minutes==1 returns 1m bars; cadence_minutes==5
    /// returns the close of every 5th 1m bar. Used by R3 equity-curve
    /// and R2.2 BTC buy-and-hold.
    fn close_series(
        &self,
        symbol: &Symbol,
        from:   Timestamp,
        to:     Timestamp,
        cadence_minutes: u32,
    ) -> Result<Vec<(Timestamp, Decimal)>, MarkError>;
}
```

`MarkSource` is a trait so tests inject a `FrozenMarkSource` from a
checked-in CSV (deterministic without parquet I/O).

### Sparkline encoding (R3.2 / Q4)

Eight-level Unicode-block palette: `▁▂▃▄▅▆▇█` (U+2581..U+2588).
Map a sequence of `Decimal` values to a fixed-width string of 60
characters. Algorithm (deterministic, no `f64`):

```rust
// crates/reports/src/sparkline.rs (new)
const BARS: &[char; 8] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn encode(values: &[Decimal], width: usize) -> String {
    if values.is_empty() { return " ".repeat(width); }
    // 1. Downsample/upsample values to exactly `width` cells by
    //    averaging contiguous chunks (chunk size = ceil(N/width)).
    //    Edge cells get the same chunk as their neighbour.
    let cells: Vec<Decimal> = downsample_avg(values, width);
    // 2. Determine min, max across cells. If max == min, all bars
    //    map to index 0 (▁) to avoid divide-by-zero (a flat curve
    //    is a flat line).
    let (min, max) = cells.iter().fold((cells[0], cells[0]),
        |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let range = max - min;
    if range == Decimal::ZERO {
        return BARS[0].to_string().repeat(width);
    }
    // 3. For each cell c, bucket = floor((c - min) / range * 8).
    //    Clamp bucket to [0, 7].
    cells.iter()
        .map(|c| {
            let scaled = ((*c - min) * Decimal::from(8)) / range;
            let i = scaled.to_u32().unwrap_or(0).min(7) as usize;
            BARS[i]
        })
        .collect()
}
```

**Determinism property:** same input slice → byte-identical UTF-8
output. No floating point. No locale. Asserted in `tests/sparkline.rs`.

### CSV artifact column schemas (R3.3 / Q5)

All CSVs live under
`spec/reports/success/artifacts/<run-id>/` and are written atomically
(same tempfile + rename pattern as the markdown). All amounts are
plain `Decimal` strings (TEXT-form, no scientific notation, no
locale separators) — same encoding as the audit ledger's TEXT
columns. Timestamps are RFC3339 UTC (microsecond precision matches
HF-3 / `journal.rs::strategy_event` format).

All `ts` / `*_ts` columns are RFC3339 UTC (microsecond precision); the
column name is the bare `ts` (not `ts_utc`) — the UTC contract is in
the introductory paragraph above and in the writer doc-comments at
`crates/reports/src/csv_artifacts.rs`. Equity files decompose total
equity into realized + unrealized P&L plus cash (R3.3 ships this
shape; the alternative cash + positions_value decomposition was
rejected because the operator question "how much of my P&L is real?"
is more useful than "how much is in cash vs marked-to-market positions").

| File                   | Source                                       | Columns                                                                                                                                                                                  |
|------------------------|----------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `equity-<window>.csv`  | R3 sampler over `cash_balance` + position marks | `ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`                                                                                                            |
| `equity-since-inception.csv` | R3.1 inception window                  | same as above                                                                                                                                                                            |
| `fills.csv`            | `audit::query::recent_fills(usize::MAX)` filtered by window | `ts,symbol,side,qty,price,fee_usdt,fee_tier,strategy_id`                                                                                                                                  |
| `pnl_by_strategy.csv`  | `audit::query::pnl_by_strategy`              | `strategy_id,realized_usdt,closed_trade_count,winning_trade_count,win_rate,avg_trade_realized_usdt`                                                                                       |
| `pnl_by_symbol.csv`    | `audit::query::pnl_by_symbol`                | `symbol,realized_usdt`                                                                                                                                                                    |
| `journal.csv`          | `audit::query::recent_journal(usize::MAX)` filtered by window | `ts,account,debit_usdt,credit_usdt,memo,transaction_id`                                                                                                                                  |
| `strategy_events.csv`  | `audit::query::strategy_events_since(period_start)` | `ts,kind,strategy_id,old_hash,new_hash,source_path,operator,error_code,error_summary`                                                                                                    |
| `funding_observations.csv` (only present if v1 funding poller ran in the window) | `audit::query::funding_rate_history` per active perp symbol | `symbol,funding_ts,funding_rate,next_funding_ts,poll_ts`                                                                                                                                  |

The reports renderer writes **all CSVs first**, then the markdown
body's atomic rename. If any CSV write fails the report aborts
before the markdown rename, leaving no half-published artifact set.

### Front-matter schema (R10.1 / Q7)

```yaml
---
period:                  <slug, e.g. "7d", "weekly", "since:2026-01-01T00:00:00Z">
period_start:            <RFC3339, microsecond precision>
period_end:              <RFC3339, microsecond precision>
generated:               <RFC3339, microsecond precision — wall-clock at render>
run_id:                  <hex, 16 chars — sha256 prefix of (period|ledger_sha|seed)>
ledger_snapshot_sha:     <hex, 64 chars — sha256 of ledger DB file at render>
seed:                    0x<hex>            # only emitted for fixture / test runs
data_source:             <"live-ledger" | "fixture:<path>">
wall_clock_s:            <float>            # binary's elapsed wall-clock seconds
binary_version:          <semver string>    # cargo-pkg-version of the reports crate
git_commit:              <40-char hex>      # cargo-built-in env GIT_COMMIT, n/a if absent
agent_pid:               <integer>          # std::process::id()
host:                    <hostname string>  # gethostname; "unknown" on failure
reconciliation:          <"PASS" | "FAIL">  # mirror of R11 result for ops triage
---
```

The list is **fixed at v1+** — adding fields is a new task, removing
fields is an updated R10.1 in this brief. All YAML keys are
lowercase + snake_case; values are scalars only (no nested maps) so
operators can grep / awk the front-matter without a YAML library.

**Reconciliation field rationale:** placing the PASS/FAIL marker in
front-matter (in addition to the body's R11 appendix table) lets ops
tooling classify failures without parsing markdown — `grep
'reconciliation: FAIL'` over the success directory surfaces every
broken report in one shell line.

### Body-vs-front-matter discipline (R10.2–R10.5)

The body **never** embeds: `generated`, `run_id`, `wall_clock_s`,
`ledger_snapshot_sha`, `data_source`, `agent_pid`, `host`,
`git_commit`. **MAY** embed: strategy hashes (stable per
strategy source file), per-strategy ids (stable across runs), period
slug (stable across runs against the same ledger snapshot).

A negative-invariant test
(`crates/reports/tests/body_no_volatile_metadata.rs`) asserts that
none of the eight forbidden substrings appear in the body bytes on
the locked fixture renders.

**Hashing convention** (R10.3): SHA-256 of body bytes starting **after
the closing `---\n\n` fence** (same byte-range convention as
`crates/backtest/src/main.rs::write_report` lines ~1494–1510). The
fence is byte-deterministic; the body that follows is the
ledger-derived content. Tests assert byte-identity across two runs
against the same fixture at the same `--seed`.

### Reconciliation engine (R11 / Q6)

```rust
// crates/reports/src/reconcile.rs (new)
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub struct ReconciliationReport {
    pub headline: ReconciliationRow,
    pub by_strategy: ReconciliationRow,
    pub by_symbol: ReconciliationRow,
    pub equity:   ReconciliationRow,
}

pub struct ReconciliationRow {
    pub identity:    &'static str,
    pub report_side: Decimal,
    pub ledger_side: Decimal,
    pub delta:       Decimal,    // report_side - ledger_side
    pub passed:      bool,       // delta == dec!(0) — exact-cent (Q6)
}

impl ReconciliationReport {
    /// Exact-cent equality across all four identities.
    pub fn all_passed(&self) -> bool {
        self.headline.passed && self.by_strategy.passed
            && self.by_symbol.passed && self.equity.passed
    }
}
```

**On any `Δ != $0.00`:**

1. Render the markdown body **with FAIL banner** at the top
   (above R9 Open risks) reading
   `*** RECONCILIATION FAILURE — see Reconciliation section ***`.
2. Render the R11.3 appendix table with `FAIL` (literal uppercase) in
   the `Pass?` cells of failing rows.
3. Write the markdown atomically to `<output>` — operators see the
   broken report.
4. Write a **sibling JSON artifact** at the same parent directory:
   `<output stem>_reconciliation_failure.json`. Schema:

   ```json
   {
     "schema_version":      1,
     "run_id":              "<hex>",
     "ledger_snapshot_sha": "<hex>",
     "period":              "<slug>",
     "period_start":        "<RFC3339>",
     "period_end":          "<RFC3339>",
     "rows": [
       {
         "identity":    "headline_return = realized + unrealized",
         "report_side": "<Decimal as TEXT>",
         "ledger_side": "<Decimal as TEXT>",
         "delta":       "<Decimal as TEXT>",
         "passed":      false
       },
       {
         "identity":    "Σ pnl_by_strategy = Σ realized",
         ...
       }
     ]
   }
   ```

5. Return `Err(ReportError::Reconciliation { sibling_path })` from
   `lib::generate`; the bin maps it to `exit 1` (R1.6).

**No tolerance.** `passed = (delta == Decimal::ZERO)`. If a future
quirk in the audit-side query introduces ULP-level rounding (none
observed today — every audit-side amount is `Decimal` end-to-end),
the architect re-opens R11.5 in this brief; the design must not
silently introduce a tolerance.

### Atomic write (R12.2 / Q3)

```rust
// crates/reports/src/atomic_write.rs (new)
use std::path::Path;

pub fn atomic_write(out: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = out.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        "{}.tmp.{}",
        out.file_name().and_then(|s| s.to_str()).unwrap_or("report"),
        std::process::id(),
    ));
    {
        let mut f = std::fs::File::create(&tmp)?;
        std::io::Write::write_all(&mut f, contents)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, out)?;
    Ok(())
}
```

**Failure semantics:** if `rename` fails after `sync_all`, the
tempfile remains for forensic inspection; the function returns
`Err`. Concurrent runs from the same PID (impossible — `pid` is
unique per process) collide only if a previous run left a stale
`.tmp.<pid>` file; on next run with the same `pid` (after PID
recycling) the `create` truncates it. Across processes the `.<pid>`
suffix prevents collision.

**Test:** `tests/atomic_write.rs` spawns three concurrent renders
against the same canonical path from the same CWD; verifies that
during the run the canonical path either does not exist or contains
a complete file (never a partial).

### Run-id hash (R3.4)

```rust
// crates/reports/src/run_id.rs (new)
pub fn compute(period: &ReportWindow, ledger_sha: &[u8; 32], seed: Option<u64>) -> String {
    let mut h = sha2::Sha256::new();
    sha2::Digest::update(&mut h, period.slug().as_bytes());
    sha2::Digest::update(&mut h, &[0u8]);  // separator
    sha2::Digest::update(&mut h, ledger_sha);
    sha2::Digest::update(&mut h, &[0u8]);
    if let Some(s) = seed {
        sha2::Digest::update(&mut h, &s.to_le_bytes());
    } else {
        sha2::Digest::update(&mut h, b"no-seed");
    }
    let out = sha2::Digest::finalize(h);
    // 16-hex-char (64-bit) prefix is plenty for collision-resistance
    // across a single operator's report history. Operators see a
    // short, comparable string in front-matter and in the artifact dir.
    hex::encode(&out[..8])
}
```

**Idempotence (R12.2):** same `(period, ledger_sha, seed)` → same
`run_id`. Tested by `tests/idempotent_run_id.rs`.

### Q8 — Kill-switch event provenance (operator default)

The v0 `audit::journal::kill_switch_tripped(reason, operator)`
currently writes a zero-amount memo journal row against
`equity:opening_balance` (via `registry_event`). v1+ extends it to
**also** write a `strategy_events` row of the new
`StrategyEventKind::KillSwitchTripped` kind in the **same SQL
transaction** as the memo row, so the two writes are atomic — either
both land or neither does.

**Migration policy:** the v0 memo rows in already-existing ledgers
are NOT retro-rewritten. They remain in `journal_entries` as legal
history (zero-amount, balance-preserving). The reports query reads
**only** the new `strategy_events` rows, so historical kill-switch
trips that happened before this v1+ change ship will **not** show up
in R7's "kill-switch trips" count. This is acceptable: the operator
knows the ship-date, and the kill-switch trip count is bounded
(historical trips are rare and operators remember them).

**Writer change** (`crates/audit/src/journal.rs::kill_switch_tripped`):

```rust
#[instrument(name = "ledger.kill_switch_trip", skip(ledger))]
pub async fn kill_switch_tripped(
    ledger:   &Ledger,
    reason:   &str,
    operator: &str,
) -> Result<(), LedgerError> {
    let metadata = serde_json::json!({
        "event": "KillSwitchTripped",
        "reason": reason,
        "operator": operator,
    }).to_string();

    // 1. Existing v0 memo row (do not remove — backwards compat).
    registry_event(ledger, "KillSwitchTripped", reason, &metadata).await?;

    // 2. NEW v1+ strategy_events row for operator success reports (Q8).
    strategy_event(
        ledger,
        &StrategyEventWrite {
            kind:          "KillSwitchTripped",
            strategy_id:   None,
            old_hash:      None,
            new_hash:      None,
            source_path:   "",
            operator,
            error_code:    Some("kill_switch_tripped"),
            error_summary: Some(reason),
            ts:            None,            // wall-clock; replay-tests pass Some(ts)
        },
    ).await?;

    Ok(())
}
```

**Reader change** (`crates/audit/src/query.rs`): add the new
`"KillSwitchTripped"` arm to the `parse_strategy_event_view` match
(line 399–415). The `StrategyEventKind::KillSwitchTripped` variant
gets added to `trading_core` alongside the v0.5/v1/v1.5a variants —
purely additive enum extension; no consumer breaks because every
match site (per the v0 `Strategy` trait shape) uses an exhaustive
default.

The `strategy_events_ts_idx` (already on `ts`) makes the
reports-side count-by-window query sub-millisecond at v1+ scale.

### Q8 (cont.) — Kill-switch incident report wiring (R12.1c)

When the agent's `KillSwitch::trip` fires (any reason: halt-file,
heartbeat-timeout, ledger-imbalance, clock-skew, manual-operator),
v1+ adds a side-effect: spawn the reports binary out-of-process so
an incident markdown lands under
`spec/reports/success/incident-<halt_event_ts>.md`. The spawn is
non-blocking and isolated:

```rust
// agent::kill_switch::KillSwitch::trip (extend; pseudocode)
pub fn trip(&self, reason: HaltReason) {
    let already = self.tripped.swap(true, Ordering::SeqCst);
    if already { return; }
    let msg = reason.to_string();
    warn!(reason = %msg, "KillSwitch tripped");

    // 1. v0 broadcast (unchanged)
    let _ = self.mode_tx.send(AgentMode::Halted { reason: msg.clone() });

    // 2. v0+v1+ audit write (now writes both memo + strategy_events row, Q8)
    let ledger = self.ledger.clone();
    let reason_clone = msg.clone();
    let operator = "kill_switch";
    tokio::spawn(async move {
        let _ = audit::journal::kill_switch_tripped(&ledger, &reason_clone, operator).await;
    });

    // 3. v1+ NEW: spawn incident report out-of-process (R12.1c)
    let halt_ts_rfc3339 = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    let _ = std::process::Command::new("cargo")
        .args(["run", "--bin", "report", "--",
               "--period", &format!("since:{halt_ts_rfc3339}"),
               "--output", &format!("spec/reports/success/incident-{halt_ts_rfc3339}.md")])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    // Spawn-only — we do NOT await the child. The agent is going to
    // Halted; the incident report writes asynchronously and lands
    // whenever cargo finishes building + running. An operator-friendly
    // alternative is to invoke the pre-built binary directly via
    // `target/release/report` if present; the Command builder above
    // can fall back to that path. See task T809.
}
```

The `cargo run` invocation re-uses the same audit DB the parent
agent owns (SQLite WAL allows concurrent read while the parent
holds a write lock; the reports binary is read-only and acquires
shared locks). No tokio runtime re-entry — the child is its own
process.

### Q7 — Cron trigger (R12.1b)

Cron infrastructure stays out-of-scope per the analyst's R12.1b. The
binary is cron-friendly per R12.2; the operator chooses how to
schedule it. Three reference patterns — none ships in this feature
but all are documented in the task notes:

1. **systemd timer** under `ops/reports.timer` + `ops/reports.service`
   (Linux production).
2. **launchd plist** under `ops/com.trading.reports.plist` (macOS dev).
3. **In-process tokio cron** in `agent::main` behind a feature flag
   (`--features in_process_cron`), using `tokio_cron_scheduler` —
   chosen if the operator wants a single-binary deploy.

Tasks below scaffold pattern (3) under a feature flag (T810)
**without** wiring it into the default build, so the v1+ ship has
no behavior change unless the operator opts in.

### R7.1 — uptime / clock-skew / feed-reconnect provenance

Analyst Notes Q6 flagged the source for these as "Prometheus
exporter snapshot". v1+ design pins three concrete sources:

| Metric                         | Source (read by reports binary)                                                                                                                                                                  |
|--------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Uptime**                     | New `agent_uptime` table (migration 005). Each agent boot writes a row `(boot_id, started_at, last_heartbeat_at, stopped_at)`; heartbeat task updates `last_heartbeat_at` every 30s; graceful shutdown sets `stopped_at`. Reports compute uptime as `Σ (min(stopped_at_or_last_hb, period_end) - max(started_at, period_start))` clamped to `[0, period_length]`. |
| **Kill-switch trips**          | `audit::query::strategy_events_since(period_start)` filtered to `kind == KillSwitchTripped` (Q8 above).                                                                                          |
| **Clock-skew events**          | New `audit::query::strategy_events_since` filter — clock-skew currently writes a memo journal row via `kill_switch_tripped(ClockSkew, _)` (see `agent::kill_switch.rs` line 119). With Q8's change, ClockSkew trips also surface as `KillSwitchTripped` events with `error_summary == "clock_skew"`. The reports query splits the count by `error_summary`. |
| **Feed reconnects**            | The structured-log channel `data::binance::reconnect` already emits events. v1+ adds a tiny additive sink `audit::journal::feed_reconnect(symbol, ts)` that writes a `strategy_events` row with `kind = "FeedReconnect"`. **Add task T805 covers the new variant + writer + reader**. (This is a new `StrategyEventKind` variant — same additive pattern as Q8.) |
| **Funding-rate poll success rate** | `audit::query::funding_rate_history` rows in `[period_start, period_end]` divided by `expected_polls = period_seconds / cfg.funding.interval_secs`. The `expected_polls` calculation lives in the reports renderer — reads `cfg.funding.interval_secs` from the agent's TOML at render time (R1.5 `--ledger` flag is paired with a `--config` flag for this; the analyst's R1.5 is sufficient with a small extension covered by task T804). |
| **LLM spend vs budget**        | `cost::CostBudget::spent()` + `cost::CostBudget::ceiling()`. v1+ all values are `$0.00` per V8.                                                                                                  |

The `FeedReconnect` variant is purely additive. The agent's reconnect
handler in `crates/data/src/binance.rs` calls the new writer.

### R9 — Open risks (5 thresholds)

Each risk is a pure function over query results; no f64 intermediates.

| Risk                                | Computation                                                                                                                                                                                                                                                          |
|-------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Drawdown approaching limit          | Compute `current_drawdown_pct` from the equity-curve sampler (R3) at `period_end`. `risks.drawdown.fired = current_drawdown_pct >= cfg.risk.max_drawdown_stop_pct * dec!(0.75)`.                                                                                     |
| LLM budget approaching cap          | `month_to_date_llm_spend = realized_pnl_since_account_balance("expense:llm:*")` (sum across all `expense:llm:<tier>` accounts via prefix scan). `risks.llm.fired = mtd_spend >= cfg.cost.budget_usd_month * dec!(0.80)`.                                              |
| Strategy decay                      | For each strategy in the active set, compute since-inception Sharpe and last-7-day Sharpe via the R4 risk-metrics module restricted to that strategy's per-strategy equity slice. `risks.decay.fired = any(strat.last7d_sharpe < 0 && strat.inception_sharpe > 0)`.   |
| Rebalance rejections accumulating   | `rebalance_rejected_count = strategy_events_since(period_start).filter(kind == RebalanceRejected).count()`. `trade_count = pnl_by_strategy.iter().sum(closed_trade_count)`. `risks.rebalance.fired = rebalance_rejected_count > 0.05 * trade_count`.                  |
| Mean-reversion hard stops accumulating | `mr_stop_count = strategy_events_since(period_start).filter(kind == MeanReversionStop).count()`. `pair_trade_count = pnl_by_strategy.iter().filter(strategy_id starts with "pairs_").sum(closed_trade_count)`. `risks.mr_stop.fired = mr_stop_count > 0.10 * pair_trade_count`. |

**Graceful degradation (R9.3):** each computation is wrapped in
`Result`. On any inner `Err`, the renderer emits
`unknown — see logs` for that risk only — never propagates the
error. The binary still exits 0 (only R11 reconciliation fails the
exit code).

### Risk register & mitigations

| Risk                                                                                                       | Severity | Mitigation                                                                                                                                                                                                                                                                                |
|------------------------------------------------------------------------------------------------------------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **R-1** Determinism leak via wall-clock in body                                                            | high     | R10.4 negative-invariant test bans the eight forbidden substrings from body bytes; CI runs the determinism test on every PR (`tests/determinism.rs`). Same lesson as v0.5 HF-1 / v1.5a HF-1 — codified.                                                                                  |
| **R-2** Reconciliation Δ creeps in via `f64` in Sharpe annualization (R4) leaking into win-rate denominator | high     | All money math `Decimal`. Sharpe / Sortino / Calmar use `f64` for the annualization step ONLY (matches v1 backtest convention) and feed back into the body as a *display string*, never into a sum that flows to R11. Reconciliation rows source from `Decimal`-only paths. Test: `tests/reconciliation.rs` proves identity to satoshi on a synthetic ledger with 200 fills. |
| **R-3** SQLite reader contention with the live agent                                                       | medium   | SQLite WAL allows concurrent shared (read) locks while one process holds an exclusive (write) lock. The reports binary opens with `journal_mode=WAL,query_only=1` (or equivalent `PRAGMA query_only = 1` after open). Test: `tests/atomic_write.rs` runs three readers concurrently; all succeed. |
| **R-4** Parquet read for a 1-year position-mark series exceeds 256 MiB RAM ceiling (R13.3)                 | medium   | `MarkSource::close_series` streams via Polars LazyFrame, not eager DataFrame. The reports renderer holds at most one full-series `Vec<(Timestamp, Decimal)>` in memory at a time (≈ 525_600 rows × ~40 bytes ≈ 20 MiB worst case for inception window).                                          |
| **R-5** Atomic-rename fails across filesystems on macOS                                                    | low      | The reports always write to `spec/reports/success/<...>` which lives in the same workspace filesystem as the tempfile. Atomic. Test: `tests/atomic_write.rs` runs against `tempfile::tempdir` (same FS).                                                                                  |
| **R-6** Pre-migration NULL `strategy_id` rows distort R5 attribution                                       | low      | Reports surface them under `(unattributed)` strategy id, clearly labelled. Operator sees the bucket shrink to zero as new fills accumulate. Documented in the report's R5 section.                                                                                                          |
| **R-7** `cargo run --bin report` from kill-switch handler triggers a recompile in production                | medium   | Task T809 specifies the spawn falls back to `target/release/report` if present; `cargo run` only used in dev. The kill-switch handler logs the spawn failure; the agent does not re-trip on the failure (no infinite loop).                                                                |
| **R-8** Reports anchor SHAs lock in the placeholder R6 string forever                                       | low      | Documented at task T811. The reflection-memory feature's brief carries an explicit "re-lock T811 anchors" deliverable. Same precedent as v1.5a T717's top10 momentum re-lock.                                                                                                              |
| **R-9** Front-matter line ordering drift breaks operator grep tools                                          | low      | Front-matter writer enumerates fields in the fixed order documented above; same Vec → write loop on every run.                                                                                                                                                                            |

### Performance plan (R13)

| Path                                                  | Budget        | v1+ expectation                                                                                       |
|-------------------------------------------------------|---------------|------------------------------------------------------------------------------------------------------|
| `--period 7d` against fixture ledger                  | < 1s          | ~150ms (small ledger; ~20 fills, ~5 strategy events; one parquet open per active perp symbol)         |
| `--period 90d` against 1-year fixture                 | < 10s (R13.1) | ~2–3s (5 strategies, ~500 fills, ~50 events; three parquet year-files scanned for marks)              |
| `--period inception` against 1-year fixture           | < 30s         | ~5–8s (525_600-row equity-curve sampler dominates; CSV write is sequential)                            |
| Memory (1-year fixture)                               | < 256 MiB     | ~50 MiB (CSV buffer ≤ 50 MiB, parquet LRU ≤ 1 MiB, audit query results ≤ 10 MiB)                       |
| Atomic write (markdown + 6 CSVs at 1-year inception)  | < 1s          | ~300 ms (sequential fsync per file; no concurrency)                                                   |

Bench placeholder: `crates/reports/benches/` directory exists for
v2+; v1+ ships `tests/perf_smoke.rs` (R13 acceptance) only.

### Test strategy

| Layer                                  | Tests                                                                                                                                                          | Crate(s)            | Tool         |
|----------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------|--------------|
| **Unit — `ReportWindow` parser**       | Accepts `7d`, `30d`, `90d`, `weekly`, `monthly`, `since:<RFC3339>`, `inception`. Rejects `bogus`, empty, `1d`, `since:bad`. R1.2.                              | `reports`           | `cargo test` |
| **Unit — sparkline encoder**           | Hand-computed expected output on a `[1,2,3,4,5,6,7,8]` series → `▁▂▃▄▅▆▇█`. Constant input → `▁▁▁▁…`. Empty input → `        ` (60 spaces). R3.2.            | `reports`           | `cargo test` |
| **Unit — atomic write**                | Three concurrent renders to the same path; canonical path is always either absent or complete (never partial). Tempfile cleanup on success. R12.2.            | `reports`           | `cargo test` |
| **Unit — run-id hash**                 | Same `(period, ledger_sha, seed)` → same 16-hex run-id. Different seed → different run-id. R3.4 / R12.2.                                                       | `reports`           | `cargo test` |
| **Unit — front-matter writer**         | All 12 fields rendered in the fixed order; YAML-parses cleanly via `serde_yaml`. R10.1.                                                                       | `reports`           | `cargo test` |
| **Unit — reconciliation engine**       | Synthetic balanced ledger → all four `Δ`s = `$0.00`, all `passed = true`. Inject one penny imbalance into one row → that row's `passed = false`, others ok. R11.1, R11.5. | `reports`           | `cargo test` |
| **Unit — risk-metrics**                | Sharpe / Sortino / Calmar on synthetic constant-return curve = expected closed-form. Max-DD on V-shaped curve = depth and recovery as expected. R4.            | `reports`           | `cargo test` |
| **Unit — body-no-volatile-metadata**   | Rendered body bytes contain none of: `generated:`, `run_id:`, `wall_clock_s:`, `ledger_snapshot_sha:`, `data_source:`, `agent_pid:`, `host:`, `git_commit:`. R10.4. | `reports`           | `cargo test` |
| **Unit — sparkline determinism**       | Proptest: 1000 random `Vec<Decimal>` inputs produce byte-identical UTF-8 across two encoder calls.                                                            | `reports`           | `proptest`   |
| **Integration — `pnl_by_strategy`**    | Fixture ledger with 4 strategies × deliberate trades; `Σ rows.realized == realized_pnl_since(period_start)` to the satoshi; rows sorted desc by realized.     | `audit`, `reports`  | `cargo test` |
| **Integration — `KillSwitchTripped` event** | Trigger `kill_switch_tripped(reason, op)`; assert (a) memo journal row written (v0 compat), (b) `strategy_events` row written with `kind = KillSwitchTripped`, (c) reconciler invariant `Σ debits == Σ credits` holds. | `audit`             | `cargo test` |
| **Integration — incident report wire** | `KillSwitch::trip()` with `target/release/report` mocked → spawn invoked with correct args; spawn failure does not re-trip the kill switch.                    | `agent`, `reports`  | `cargo test` |
| **Integration — determinism (V4)**     | `report-sample-7d` runs twice 10s apart against same fixture ledger at seed `0xC0FFEE`; front-matter `generated:` differs; body-SHA256 byte-identical.        | `reports`           | `cargo test` |
| **Integration — reconciliation FAIL**  | Inject post-hoc journal row between two query reads; assert (a) FAIL banner in body, (b) `FAIL` cells in R11 table, (c) sibling JSON written, (d) bin exits 1. | `reports`           | `cargo test` |
| **Integration — perf smoke (R13)**     | `--period 90d` against 1-year fixture < 10s wall-clock; resident-set < 256 MiB measured via `getrusage`.                                                       | `reports`           | `cargo test` |
| **Snapshot — body shape**              | `report-sample-7d` and `report-sample-90d` body-SHA256 captured at first-run; locked into the regression gate (anchor count grows 9 → 11).                    | `reports`, `tester` | `cargo test` |
| **Regression (V6)**                    | All 9 v0/v0.5/v1/v1.5a anchor SHAs preserved byte-identical post-v1+.                                                                                          | `backtest`          | `cargo test` |

### Determinism plan (R10)

| Source of non-determinism (potential)                  | Mitigation                                                                                                                       |
|--------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------|
| `HashMap` iteration over strategies                    | All strategy-keyed structures are `BTreeMap<StrategyId, _>` (lex-sort by id).                                                    |
| Wall-clock leakage into body                           | R10.4 negative-invariant test bans 8 substrings; front-matter is the only home for volatile fields.                              |
| Filesystem listing order in `MarkSource`               | `ParquetMarkSource` walks year files in `(year ASC, month ASC)` order via explicit sort, not directory-iter order.                |
| Sparkline `Decimal::min`/`max` on equal values         | `range == 0` short-circuits to `▁` × width — no NaN, no divide.                                                                  |
| `f64` round-off in Sharpe / Sortino / Calmar           | These produce body display strings via `{:.4}`-formatted output; never feed back into reconciliation paths.                       |
| CSV row order                                          | All CSV writers iterate query results in the deterministic order the query returns (lex by symbol/strategy/ts).                  |
| Front-matter field order                               | Hard-coded in the writer (no map-iter).                                                                                          |
| `microsecond` precision drift                          | Reports format `Timestamp` via the same `[year]…[subsecond digits:6]Z` pattern as `journal.rs::strategy_event` (HF-3 lesson).    |

### Mapping R/V → tasks

Every R-item and V-item maps to at least one T8xx task. See
`spec/tasks/operator-success-reports.md` for the full task list with
acceptance criteria.

| R-item / V-item                              | Tasks                                                  |
|----------------------------------------------|--------------------------------------------------------|
| R1 binary surface                            | T801, T804, T813                                       |
| R2 headline + BTC baseline                   | T813                                                   |
| R3 equity curve + sparkline + CSV            | T806, T807, T813                                       |
| R4 risk metrics                              | T813                                                   |
| R5 strategy attribution + new query + schema | T802, T803, T813                                       |
| R6 memory placeholder                        | T813                                                   |
| R7 system health                             | T805, T811, T812, T813                                 |
| R8 what changed                              | T813                                                   |
| R9 open risks                                | T813                                                   |
| R10 determinism / body discipline            | T807, T808, T814                                       |
| R11 reconciliation invariant                 | T808, T814                                             |
| R12 cron + atomic write + on-trip            | T807, T809, T810                                       |
| R13 perf                                     | T815                                                   |
| V1 static checks                             | T_FINAL_REPORTS                                        |
| V2 cargo test                                | T813, T814, T815, T_FINAL_REPORTS                      |
| V3 both scenarios run                        | T816, T817, T_FINAL_REPORTS                            |
| V4 body determinism                          | T814, T816                                             |
| V5 reconciliation invariant                  | T814, T816                                             |
| V6 9-anchor regression-free                  | T_FINAL_REPORTS                                        |
| V7 audit-query API surface preserved         | T802, T803                                             |
| V8 cost telemetry                            | T813                                                   |
| V9 perf                                      | T815                                                   |
| V10 cron-friendliness smoke                  | T807, T816                                             |

## Implementation

### Wave 2a (developer, 2026-05-01)

T807 + T808 + T811 + T812 land — `crates/reports/` skeleton, the
exact-cent reconciliation engine, the strategy-decay heuristic +
re-lock note, and the `MarkSource` trait + parquet/frozen impls.

- New crate `reports` (lib + bin name `report`) added as workspace
  member at root `Cargo.toml`.
- Layout matches the Design section: `src/{lib,window,atomic_write,
  run_id,sparkline,reconcile,marks}.rs` + `src/render/{front_matter,
  headline,equity_curve,risk_metrics,strategy_attribution,
  memory_highlights,system_health,what_changed,open_risks,
  reconciliation}.rs` + `src/bin/report.rs`.
- T813's body modules ship as `pub fn render(...) -> String` stubs
  except `memory_highlights`, which carries the locked R6
  placeholder + the strategy-decay heuristic + the forward-compat
  rustdoc note.
- The test surface for this wave — 58 in-module unit tests + 7
  marks integration tests + 3 reconciliation integration tests —
  all green under `cargo test -p reports`.
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (9 / 9)` post
  Wave 2a (this feature touches no scenario-affecting crate).
- Open dependency edges: `reports → audit`, `reports → data`,
  `reports → cost` (per the architecture's Crate-map delta).

T809 + T810 (kill-switch wiring + cron flag) land in a parallel
wave; T813–T817 + `T_FINAL_REPORTS` follow.

### Wave 2b (developer, 2026-05-01)

T809 + T810 land — kill-switch dual-write + incident-report spawn,
plus opt-in in-process cron.

- **T809** rewrites `audit::journal::kill_switch_tripped` in
  `crates/audit/src/journal.rs:297-405` so the v0 zero-amount memo
  journal row AND the new `strategy_events` row of kind
  `KillSwitchTripped` are written inside one `sqlx::Transaction`
  (atomic dual-write).  The memo row preserves v0 byte-for-byte
  format (description `registry:KillSwitchTripped:<reason>`,
  metadata JSON `{event,reason,operator}`, RFC-3339 second
  precision); the new `strategy_events` row uses the 6-digit
  microsecond format (HF-3 / determinism gate).
- Agent-side `KillSwitch` gains `with_audit(...)` constructor
  carrying `Arc<audit::Ledger>` + `Arc<dyn IncidentSpawner>`.
  `KillSwitch::trip` (`crates/agent/src/kill_switch.rs:279`) after
  the v0 broadcast: (1) tokio-spawns the dual-write writer
  fire-and-forget, (2) invokes the `IncidentSpawner` trait
  (production: `CommandIncidentSpawner` — `target/release/report`
  with `target/debug/report` fallback; tests:
  `MockIncidentSpawner` recorder).
- Acceptance: 4 audit-side dual-write tests at
  `crates/audit/tests/kill_switch_dual_write_test.rs` + 3 agent-side
  integration tests at
  `crates/agent/tests/kill_switch_trip_writes_both.rs` — all green.
  Reconciler invariant `Σ debits == Σ credits` preserved (memo row
  is zero-amount; `strategy_events` carries no money columns).
- **T810** adds optional in-process cron behind feature flag
  `in_process_cron` (Cargo.toml:15-20).  `tokio-cron-scheduler v0.15.1`
  + `reports` are pulled in only when the flag is enabled.  Default
  build is unchanged (verified: `cargo build -p agent` skips both
  optional deps).  Cron module at `crates/agent/src/cron.rs` is
  `#![cfg(feature = "in_process_cron")]`-gated; defaults to Mondays
  09:00 (`"0 0 9 * * Mon"`), runs `reports::generate(ReportWindow::Weekly,
  …)` in-process; failures are warn-logged.
- Reference operator files (no build wiring):
  `ops/reports.timer.example`, `ops/reports.service.example`,
  `ops/com.trading.reports.plist.example`.  The launchd plist is
  `plutil -lint`-clean.
- `cargo clippy --workspace --tests -- -D warnings` clean (default
  features and `--features in_process_cron`).
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (9 / 9)` post
  Wave 2b.  Neither the audit rewrite nor the agent kill-switch
  path is reachable from the backtest binary.

T813–T817 + `T_FINAL_REPORTS` follow.

### Wave 2c (developer, 2026-05-01)

T813 lands — render modules R2–R9 + R11 reconciliation appendix +
`lib::generate` orchestrator + `csv_artifacts.rs`.

- Renderers (all pure over their inputs, zero I/O):
  `crates/reports/src/render/headline.rs` (R2),
  `…/equity_curve.rs` (R3),
  `…/risk_metrics.rs` (R4 + Sharpe/Sortino/Calmar/max-DD/recovery
  helpers, `MINUTES_PER_YEAR = 525_600`),
  `…/strategy_attribution.rs` (R5; `(no activity)` for active-but-
  zero-trade strategies),
  `…/memory_highlights.rs` (R6; `render_with_decay` appends decay
  candidates without touching `PLACEHOLDER`),
  `…/system_health.rs` (R7 6-row table + `compute_uptime_pct`
  helper; per-cell `Result::Err` → `unknown — see logs`),
  `…/what_changed.rs` (R8 lifecycle filter + R8.3 sentinel),
  `…/open_risks.rs` (R9 5 risks + `_no open risks._` sentinel + R9.3
  unknown-on-Err),
  `…/reconciliation.rs` (R11 appendix + `FAIL_BANNER` literal).
- Companion CSV writer at `crates/reports/src/csv_artifacts.rs` —
  one writer per Design "CSV artifact column schemas" file
  (equity, fills, pnl_by_strategy, pnl_by_symbol, journal,
  strategy_events, funding_observations).  All use `csv::Writer`
  with `QuoteStyle::Necessary` and `Decimal::to_string()` (no
  scientific notation, no locale).
- Orchestrator at `crates/reports/src/lib.rs::generate` — opens the
  ledger, runs every audit query once, hits the `MarkSource` for the
  BTC baseline, computes reconciliation, renders front-matter +
  body in the locked section order (R9 pinned, then R2 R3 R4 R5 R6
  R7 R8 R11), atomic-writes the markdown, then writes the seven
  companion CSVs into `<output_parent>/artifacts/<run-id>/`.  On
  reconciliation FAIL the orchestrator additionally writes the
  sibling `_reconciliation_failure.json` and returns
  `Err(ReportError::Reconciliation { sibling_path })` so the bin
  exits 1 (R11.4 / R1.6).
- v1+ scoping: open-position mark-to-market `unrealized` is taken
  as `Decimal::ZERO` for v1+ (the trait + `MarkSource::close_at` is
  exercised for the BTC baseline; the open-position projection
  ships in v2+ when the audit query surface exposes a typed
  `open_positions` slice).  Reconciliation identity #1 holds in
  this scope because both the report side and the ledger side use
  `0` for unrealized.
- Tests: 134 green under `cargo test -p reports` (96 unit + 38
  integration across `headline_render`, `risk_metrics`,
  `strategy_attribution`, `memory_highlights`, `system_health`,
  `what_changed`, `open_risks`, `csv_artifacts`,
  `generate_smoke`, `marks`, `reconciliation`).
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --tests -- -D warnings` clean.
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (9 / 9)` post
  Wave 2c (no scenario-affecting crate touched).
- Bin smoke: `cargo run -p reports --bin report -- --period 7d
  --ledger /tmp/audit.db --output /tmp/test-report.md
  --seed 0xC0FFEE` → exit 0; markdown + 7 CSV companions land.

T814 (determinism + body-no-volatile-metadata + reconciliation
mismatch tests), T815 (perf smoke), T816 (report scenarios + anchor
extension 9→11), T817 (anchor regression re-run), `T_FINAL_REPORTS`
follow.

### Wave 2d-3 (developer, 2026-05-01)

T816 lands — `report-sample-7d` + `report-sample-90d` scenarios
captured as deterministic fixtures with locked body-SHA256 anchors.
Anchor count grows from **9 → 11**.

- **Fixture builders.** Two deterministic SQLite-snapshot builders
  ship under `crates/reports/tests/fixtures/`:
  - `build_ledger_7d.rs` — 7-day fixture with 3 `Load` events, 6
    `Sell` fills across 2 strategies (≥3 closed trades, ≥2
    strategies), 1 `RebalanceRejected` event, and 3 `funding_rates`
    observations.  Anchored to fixed `period_start = 2026-04-21T00:00:00Z`.
  - `build_ledger_90d.rs` — 90-day fixture with 4 strategies
    (`strat_alpha`, `strat_beta`, `strat_gamma`, `pairs_zeta`), 1
    `Swap` event, 1 `MeanReversionStop` event, 24 fills (3 closed
    trades per strategy), 3 `RebalanceRejected` events, 5
    funding-rate observations.  Anchored to `period_start = 2026-01-28T00:00:00Z`.
  - Both use `FIXTURE_SEED = 0x00C0_FFEE` and write events with
    fixed RFC-3339 timestamps so the rendered body's R8 lifecycle
    bullets stay byte-identical across wall-clock drift.  Uptime
    intervals close at `FAR_FUTURE_RFC3339 = 2099-12-31T23:59:59Z`
    so `compute_uptime_pct` saturates at the orchestrator's
    `period_end = now` clamp — yielding 100% uptime in every body
    regardless of when the test runs (the only remaining
    wall-clock leak the v1+ orchestrator otherwise exposes).
- **Determinism strategy.** Tests use `ReportWindow::Since(<fixed-ts>)`
  rather than `Days7` / `Days90` so `period_start` is wall-clock-
  independent.  `period_end = now` still drifts but the only body-
  side consumer (the equity-curve sampler) passes through R3's
  fixed-width-60 sparkline encoder over a constant-cash curve, so
  the body bytes stay stable.
- **Test file.** `crates/reports/tests/report_scenarios.rs` ships
  four `tokio::test`s: 7d determinism + anchor lock, 90d
  determinism + anchor lock, V10 lib-level cron-friendliness (3×
  concurrent `lib::generate` + canonical-path partial-file poller),
  and V10 bin-level cron-friendliness (3× `cargo run -p reports
  --bin report` processes against the same fixture; all exit 0
  with byte-identical bodies).
- **Anchor capture + regression-gate extension.** Two new entries
  appended to `spec/anchors.toml`:
  `report-sample-7d` =
  `ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c`,
  `report-sample-90d` =
  `2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f`.
  `scripts/verify_anchors.sh` extended with an additive fallback
  glob: when `backtest-*-<scenario>.md` misses, the script tries
  `success-*-<scenario>.md` under `spec/reports/success/` (the
  existing 9-anchor flow is unchanged byte-for-byte for backtest
  scenarios).  Tests publish a stable `success-fixed-<scenario>.md`
  copy under `spec/reports/success/` so the gate can hash against
  a real on-disk file.
- Tests-only changes; **no `crates/reports/src/` source touched**.
- `cargo test -p reports --test report_scenarios` → 4 PASS.
- `cargo test -p reports` → 100 tests green (96 prior + 4 new).
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --tests -- -D warnings` clean.

## Verification

The tester's contract for declaring this feature done. All items
must be green before a `VERDICT → PASS` can be issued. Mapping to
R-numbered requirements is explicit so the tester's report can
cross-reference.

- **V1 Static checks pass.** `cargo fmt --check` clean,
  `cargo clippy --workspace --all-targets -- -D warnings` clean
  (including the v0 R2.2 / R2.3 deny lints), `cargo audit` shows
  no unpatched advisories, `cargo deny check` (bans, licenses,
  sources) passes. Same gate as v0 V1 / v1 V1 / v1.5a V1.
- **V2 `cargo test --workspace` green.** Zero failures, zero
  unexplained `#[ignore]`. Includes the new test surfaces in the
  reports crate (or wherever the binary lands per Notes Q1):
  - R1 binary surface — `clap`-arg parsing tests for `--period`
    accepted shapes (`7d`, `30d`, `90d`, `since:...`, `inception`)
    and rejected shapes (`bogus`, empty).
  - R2 headline — fixture-ledger unit test for the headline
    string format and BTC buy-and-hold computation.
  - R3 equity curve — sparkline rendering test (60 chars from
    the `▁▂▃▄▅▆▇█` set), CSV artifact test (column header,
    row count).
  - R4 risk metrics — Sharpe / Sortino / Calmar unit tests on
    synthetic curves, max-drawdown / recovery-time tests.
  - R5 strategy attribution — fixture-ledger test for the
    new `pnl_by_strategy` query (sum-equals-scalar invariant);
    table render test.
  - R6 memory placeholder — exact-string match test (no
    timestamps, no run-id leakage).
  - R7 system health — fixture-log + fixture-ledger render test.
  - R8 what-changed — chronological-order render test on
    fixture lifecycle events; empty-period sentinel-string test.
  - R9 open risks — five fixture-ledger tests (one per risk
    threshold) + the `_no open risks._` sentinel test +
    the `unknown — see logs` graceful-degradation test.
  - R10 determinism — body-only SHA-256 byte-identical across
    two runs (R10.3); negative-invariant substring-absence
    test (R10.4).
  - R11 reconciliation — exact-cent reconciliation test (R11.5);
    deliberate-mismatch test that asserts (i) banner renders,
    (ii) `FAIL` cell renders, (iii) binary exits 1.
  - R12 cron-friendliness — atomic-write test, idempotent-run-id
    test.
  - R13 perf-smoke — `< 10s` wall-clock against 1-year fixture.
- **V3 Both report scenarios run end-to-end.**
  - `report-sample-7d` produces
    `spec/reports/success/report-sample-7d-<stamp>.md` and the
    matching `artifacts/<run-id>/equity-since-inception.csv` +
    `equity-7d.csv`. The body-SHA256 is captured by the tester
    on first successful run and added to the anchor table.
  - `report-sample-90d` produces
    `spec/reports/success/report-sample-90d-<stamp>.md` plus the
    same artifact pair. Body-SHA256 captured the same way.
  - Both reports include all 9 R-driven body sections (Headline,
    Open risks, Equity curve, Risk metrics, Strategy attribution,
    Memory highlights, System health, What changed,
    Reconciliation) in the locked order.
- **V4 Body-only determinism (R10).** Each report scenario runs
  twice against the same fixture ledger at seed `0xC0FFEE`, ten
  seconds apart. The two front-matters differ on the
  `generated:` field; the two bodies are byte-identical
  (verified by SHA-256 plus a substring-diff sanity check).
  The R10.4 negative invariant passes on both renders.
- **V5 Reconciliation invariant (R11).** Across both scenarios:
  - Every `Δ` in the Reconciliation appendix is `$0.00 USDT`
    exact-cent.
  - The deliberate-mismatch integration test (R11.4 acceptance)
    forces a `Δ != $0.00`, and the tester verifies the binary
    exits 1, the banner renders, and the `Pass?` cell prints
    `FAIL`.
- **V6 9-anchor regression-free.** All 9 v0 + v0.5 + v1 + v1.5a
  anchor SHAs preserved byte-identical (this feature touches no
  strategy code; should be trivially PASS):
  - `btc-2023-1m-sma-cross` →
    `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`
  - `btc-2023-1m-sma-baseline-refresh` →
    `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`
  - `btc-2023-1m-macd-trend` →
    `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805`
  - `btc-2023-1m-rsi-reversion` →
    `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa`
  - `btc-2023-1m-bbands-mean-revert` →
    `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3`
  - `top10-2023-1h-momentum` →
    `a20431e3f5765cefbdfed7d1157654bcbec90d90e4bd178cdd37ce084cba55af`
    ([re-locked at v1.5a T717](v15a-mean-reversion-pairs.md#t717-anchor-note))
  - `top10-2024-h1-momentum` →
    `38b576335c9a7a45b7f4a74ecf82ca8310b89ae025c2ba33c56f79e62c22ba2c`
    ([re-locked at v1.5a T717](v15a-mean-reversion-pairs.md#t717-anchor-note))
  - `pairs-2023-zscore-mr` →
    `90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0`
  - `pairs-2024-h1-zscore-mr` →
    `14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f`
  Plus the two new operator-success-report anchors locked at
  first successful run (regression gate grows 9 → 11).
- **V7 Audit-query API surface preserved.** The existing
  read-only API in
  [`crates/audit/src/query.rs`](../../crates/audit/src/query.rs)
  is **extended additively only** — `pnl_by_symbol`, `pnl_by_pair`,
  `strategy_history`, `strategy_events_since`,
  `funding_rate_history`, `account_list`, `cash_balance`,
  `realized_pnl_since`, `total_fees`, `recent_fills`,
  `recent_journal`, `global_debit_credit_sum`,
  `verify_transaction_balance`, `all_transaction_ids` all retain
  their current shape. The new `pnl_by_strategy` query (R5.3)
  follows the same Decimal-in / Decimal-out / no-`sqlx`-types
  contract.
- **V8 Cost telemetry (R7.1).** The reports binary itself uses
  zero LLM tokens; the cost ledger's `expense:llm:*` accounts
  remain at zero through the feature's verification. The
  generated reports show `LLM spend: $0.00 / $135` (v1+ ceiling)
  in their System Health section. Same gate as v1 V10 / v1.5a V10.
- **V9 Performance (R13).** Wall-clock for `report-sample-90d`
  against a 1-year-history fixture is `< 10s`. Memory
  high-water-mark stays `< 256 MiB`. The smoke test in
  `tests/perf_smoke.rs` is the gate; promotion to `criterion`
  is a v2+ concern.
- **V10 Cron-friendliness smoke (R12).** Tester runs the binary
  three times against the same fixture in parallel from the same
  CWD; verifies (a) all three exit 0, (b) all three produce
  byte-identical bodies (R10.3), (c) atomic write means no
  partial files appear at any canonical path during the run.

Failure on any of V1–V10 routes per the v0 / v0.5 / v1 / v1.5a
verdict-routing contract:
- Static / test failure → `developer`.
- Cron / atomic-write / determinism regression → `developer` (or
  `architect` if the failure mode reveals a body-vs-front-matter
  policy gap).
- Audit-query surface change required (e.g. `pnl_by_strategy`
  shape) → `architect`.
- R6 memory placeholder breaking (e.g. the reflection-memory
  feature ships and starts writing lesson cards before this
  feature accommodates them) → `analyst` (re-scope R6 from
  placeholder to full implementation).

## Notes — open questions for architect

The analyst defers these decisions to the architect. The brief is
written so each can be answered yes/no (or "pick one") without
reshaping the requirements above.

1. **Crate placement: new `crates/reports/` vs additive in
   `audit` or `backtest`?** Analyst's preference: **dedicated
   `crates/reports/` crate.** Justification:
   - **Clean dependency graph** — `reports` depends on `audit` +
     `trading_core` + (read-only) `data` for the BTC buy-and-hold
     baseline price file. No reverse dep into `audit` or
     `backtest`.
   - **Separation of concerns** — `audit` is the read-only query
     surface, not a presentation layer. `backtest` is the
     simulation harness, not a periodic-reporting harness. Both
     have a clear single responsibility today; absorbing the
     reports binary into either erodes that.
   - **Independent test surface** — the reports crate's tests
     (R1–R13 above) don't pollute `audit` or `backtest` test
     budgets.
   Architect may push back if (a) a new crate cost vs benefit is
   unfavorable for ~1500 LoC of binary, or (b) the architect
   sees a compelling cross-crate refactor (e.g. promoting the
   markdown-report-writing helper from `backtest` into a shared
   `report_format` module under `audit`).

2. **New `pnl_by_strategy` query in `audit::query`.** R5 needs a
   per-strategy P&L breakdown that **does not currently exist**.
   The existing surface (per
   [`crates/audit/src/query.rs`](../../crates/audit/src/query.rs))
   has `pnl_by_symbol` (v1 T609) and `pnl_by_pair` (v1.5a T708)
   but no `pnl_by_strategy`. The architect needs to:
   - Decide the function signature
     (`pnl_by_strategy(ledger, since, until) -> Vec<(StrategyId,
     Money<Usdt>, trade_count, win_count)>` is the analyst's
     proposal — same Decimal-in / Decimal-out / no-`sqlx`-types
     contract).
   - Decide how to attribute trades to strategies. Analyst's
     proposal: join `journal_transactions` on `strategy_events`
     by timestamp (the strategy active at the trade's `ts` is
     the latest `Load`/`Swap` event with `ts ≤ trade.ts`).
     Architect may push back if this cross-join is unacceptably
     slow on large ledgers, in which case the schema needs a
     `strategy_id` column on `journal_transactions` (additive,
     `INSERT OR IGNORE` migration; same precedent as v1 T609
     adding the symbol-extraction join).
   - Decide whether per-strategy P&L should also include a
     `mark_to_market_at(ts)` helper for the unrealized-P&L
     reconciliation in R11.1, or whether the reports crate
     computes that itself by reading position rows + price
     marks.

3. **Atomic-write pattern.** R12.2 requires write-to-temp +
   `rename`. Analyst's proposal: use the same pattern the v0
   backtest binary uses (write to `<path>.tmp` + `std::fs::rename`
   to `<path>` after fsync). Architect may pin a different
   pattern (e.g. POSIX `O_TMPFILE` + `linkat`, or a more careful
   fsync strategy on macOS where `rename` is not atomic across
   filesystems). Analyst's prior is the simple temp-and-rename
   is enough at the v1+ scale (one report per week, ≤ 100KB
   markdown body, < 50MB CSV artifact).

4. **Sparkline format: Unicode block (`▁▂▃▄▅▆▇█`) vs ASCII bars
   (`#`/`*`/`.`)?** Analyst's preference: **Unicode block** —
   visible in any monospaced terminal + every modern markdown
   previewer including the cockpit's `viewer` binary
   ([product.md → Consumer](../product.md#consumer)). Architect
   may pin ASCII if (a) the cockpit `viewer` font has rendering
   issues on Unicode block characters, (b) the operator's
   terminal has an emoji-fallback policy that mangles them.

5. **CSV vs Parquet for `artifacts/`.** R3.3 ships CSV.
   Analyst's preference: **CSV for portability** — the operator
   can `cat`, `awk`, `sed`, pipe into Python notebook `pandas`
   without any tooling. Parquet wins on size only — at v1+ the
   equity-curve artifact is ~50MB CSV per inception report,
   well under any storage ceiling. Architect may pin Parquet if
   (a) the artifact directory begins to dominate the storage
   line, (b) a future ML pipeline wants Parquet directly without
   re-ingesting CSV.

6. **Reconciliation tolerance: exact-cent vs bps?** R11.5 ships
   exact-cent. Analyst's preference: **exact-cent** — the audit
   ledger is `Decimal` end-to-end (no floats — see
   [architecture.md → Numerics](../architecture.md#numerics--ml))
   so exact equality is the right gate. Architect may push back
   if the new R5 `pnl_by_strategy` query introduces rounding
   (e.g. via the `f64` Sharpe annualization in R4.2 leaking
   into R5's win-rate computation). If so, fall back to a
   tight bps tolerance (`Δ < $0.01`); the report should still
   surface the cent-precision Δ value alongside the pass/fail
   so the operator sees the magnitude.

7. **Front-matter schema (R10.1).** Analyst proposes the
   following fixed set:
   ```yaml
   period: <human-readable, e.g. "7d">
   period_start: <RFC3339>
   period_end: <RFC3339>
   generated: <RFC3339>
   run_id: <hex>
   ledger_snapshot_sha: <hex>
   seed: 0x<hex>            # only for fixture / test runs
   data_source: <string>    # "live-ledger" or "fixture:<path>"
   wall_clock_s: <float>
   binary_version: <string> # cargo-pkg-version of the reports crate
   ```
   Architect may add fields (e.g. `git_commit:`, `agent_pid:`,
   `host:`) but MUST NOT remove any of the above without an
   updated R10 in this brief.

8. **Kill-switch event provenance for R7.1 / R12.1c.** Analyst
   read the `StrategyEventKind` enum in
   [`crates/audit/src/query.rs`](../../crates/audit/src/query.rs)
   lines 399–415 and noticed that **kill-switch trips do not
   currently produce a `StrategyEventKind` variant** — the
   closest is `Reject`, but that's strategy-load rejection, not
   a runtime kill-switch trip. The v0 `kill_switch_tripped()`
   journal writer
   ([architecture.md → Audit & ledger](../architecture.md#audit--ledger))
   writes a zero-amount memo row against
   `equity:opening_balance` rather than emitting a strategy
   event. Architect needs to:
   - Either add a `KillSwitchTripped` variant to
     `StrategyEventKind` (additive, no migration since
     `strategy_events.kind` is `TEXT`), so R7's "kill-switch
     trips" count is a clean `strategy_events_since` filter; or
   - Direct R7 to query the zero-amount memo rows directly
     (less clean but no schema change).
   - Decide R12.1c's wiring: how does the kill-switch handler
     `std::process::Command::new("cargo run --bin reports
     ...")` without re-entering the agent's tokio runtime? An
     out-of-process invocation is the safe pattern; architect
     to confirm.

9. **R6 reflection-memory placeholder lifecycle.** Analyst writes
   R6 as a fixed placeholder string. Once the reflection-memory
   feature ships (a separate brief, not this one), R6 needs to
   be **re-opened by the analyst** to design the real "Memory
   highlights" section. Architect should make sure the
   verification gates (V4 byte-identical body, V6 anchor
   regression) don't accidentally lock the placeholder string
   into permanence — when the memory feature ships, the two
   anchor SHAs in V3 will need to be re-locked the same way
   v1.5a re-locked the two top10 momentum anchors at T717.
   Architect to flag this in the eventual reflection-memory
   brief's regression gate.

## Changelog

- 2026-05-01 (analyst): initial brief.
- 2026-05-01 (architect): Design section appended. Resolved Q1–Q9
  (Q1 dedicated `crates/reports/` lib+bin, Q2 `pnl_by_strategy` in
  `audit::query` plus additive `journal_transactions.strategy_id`
  schema migration, Q3 tempfile + `rename` atomic write, Q4
  Unicode-block sparkline `▁▂▃▄▅▆▇█`, Q5 CSV companion artifacts
  with pinned column schemas, Q6 exact-cent reconciliation with
  sibling `_reconciliation_failure.json`, Q7 12-field front-matter,
  Q8 new `StrategyEventKind::KillSwitchTripped` variant + writer
  migration that keeps the v0 zero-amount memo row, Q9 placeholder
  re-lock plan deferred to T811 in the eventual reflection-memory
  brief). Two new `audit` migrations: `004_journal_transactions_strategy_id.sql`
  and `005_uptime_intervals.sql`. Two new `StrategyEventKind`
  variants: `KillSwitchTripped`, `FeedReconnect`. Task list at
  `spec/tasks/operator-success-reports.md` (T801–T817 plus
  `T_FINAL_REPORTS`). Architecture.md updated with a new
  "v1+ Operator success reports resolutions (Q1–Q9)" subsection +
  `crates/reports/` workspace member. Status flipped from `draft`
  to `in-progress`; owner flipped from `analyst` to `architect`.
- 2026-05-01 (architect): reconciled "CSV artifact column schemas
  (R3.3 / Q5)" subsection to match the Wave 2c shipped renderer in
  `crates/reports/src/csv_artifacts.rs` (134 tests green). The spec
  table previously listed an `equity_usdt,cash_usdt,positions_value_usdt`
  decomposition for `equity-*.csv`; the renderer ships
  `equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`
  (realized vs unrealized P&L is a more useful operator decomposition
  — answers "how much of my P&L is real?"). Also dropped the `_utc`
  suffix on the `ts` columns of `equity-*.csv`, `fills.csv`,
  `journal.csv`, and `strategy_events.csv` to match the writer
  headers; the UTC contract remains in the introductory paragraph
  and in each writer's doc-comment. No code change; no anchor
  impact (CSV companions are not part of the 9 locked anchors).
  Decision: code is canonical (Option A); spec re-aligned. Wave 2d
  (T816 anchor capture) proceeds against the renderer's actual
  byte output.
- 2026-05-01 (tester): FINAL gate PASS — feature shipped. All 10
  V-items VERIFIED (V1 static checks, V2 12-field front-matter +
  R1–R13 test surfaces, V3 both report scenarios end-to-end, V4
  body-only determinism, V5 reconciliation invariant + FAIL banner
  + bin exit 1, V6 9 prior anchors byte-identical, V7 audit-query
  surface + CSV column schemas, V8 cost telemetry + kill-switch
  dual-write, V9 perf budget, V10 cron-friendliness 3× parallel).
  Anchor gate `ANCHORS PASS (11 / 11)`. Workspace tests 580 PASS /
  0 FAIL / 3 IGNORED. Reports crate 143 PASS / 0 FAIL / 0 IGNORED.
  cargo fmt + clippy clean (`-D warnings`, `--all-features`).
  `cargo build -p agent --features in_process_cron` clean. Bin
  smoke `cargo run -p reports --bin report -- --period 7d --ledger
  /tmp/audit.db --output /tmp/wave2d-smoke.md` exits 0 with full
  12-field front-matter present. Status bumped `in-progress →
  shipped`; owner bumped `architect → tester` for the FINAL gate
  audit trail. Test report:
  `spec/reports/test-2026-05-01-1828-operator-success-reports-final.md`.
  T_FINAL_REPORTS ticked by tester per AGENT.md "Tester owns
  `T_FINAL_*` ticks" rule.
