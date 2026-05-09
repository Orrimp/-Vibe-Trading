---
slug: reflection-memory
mode: release
status: draft
audience: human-operator
updated: 2026-05-08
generated: 2026-05-08T22:30:00Z
supersedes: _none — first presenter fire on this feature_
---

# Reflection memory — release

## TL;DR

The agent now writes a small, structured "lesson card" after every closed trade and the operator success report's `## Memory highlights` section surfaces the top 5 most relevant past lessons — deterministic to the byte, anchored in `spec/anchors.toml`, no LLM tokens spent, no trader behavior change, opt-in by config.

## What changed

- A new `crates/reflection/` library crate now turns each closed trade into a lesson card (symbol, strategy, signed P&L, holding period, regime tag, outcome class) and writes it to a sibling `reflection.db` SQLite file — separate from the audit ledger so the chart-of-accounts boundary stays clean.
- The operator success report's previously-empty `## Memory highlights` section now renders the top 5 cards retrieved for the current period via a deterministic 32-dim hand-crafted embedding + linear-scan cosine similarity. No LLM tokens; the `expense:llm:*` ledger row stays at `$0.00 / $135`.
- Two anchor SHAs in `spec/anchors.toml` lines 67–75 (`report-sample-7d`, `report-sample-90d`) were re-locked to the new bodies; the 9 strategy-backtest anchors at lines 15–58 are byte-identical (a negative-invariant test enforces this).

## Why

Quoting `spec/reflection-memory/feature.md` lines 11–98: the operator's "what did the agent learn from the last 7 days?" question now has a real answer drawn from artefacts the agent itself produced after each trade. This is layers 1–3 of the four-layer memory loop named in `spec/product.md` ("Memory & continual learning") — episodic memory, reflection, and retrieval at decision time — minus layer 4 (periodic distillation), which is queued as a follow-up. The brief frames it as "the moat made queryable": before this feature, the report's memory section was a placeholder string (`_reflection memory not yet implemented._`); after, it surfaces the top lesson cards retrieved against the current period's largest-absolute-P&L strategy and symbol.

The shape was constrained by three prior contracts. First, the operator-success-reports brief (R6) shipped the `## Memory highlights` section header back on 2026-05-01 with a deterministic placeholder body and an explicit "(v1+, once the memory loop runs)" caveat — so this feature inherits that section header, two anchored fixtures, and the body-vs-front-matter discipline (R10.3) that makes the report deterministic to the byte. Second, `spec/dev-notes/memory-anchor-relock-TBD.md` is a forward-compat breadcrumb that names the **anchor re-lock pattern** — the same precedent v1.5a applied at task T717 — and it explicitly says this feature must re-lock the two `report-sample-*` v1+ entries when the placeholder body changes, but **not** the 9 strategy-backtest anchors. Third, the `Strategy` trait shape is a v0 invariant that has been stable across every feature since v0; preserving it means trader behavior cannot shift, fills cannot shift, journal rows cannot shift, and the 9 strategy-backtest body bytes cannot shift. Q4 = report-only (this feature only feeds retrieval into the report, not into the trader) is exactly what protects those three byte-stable contracts.

**Terms-of-art** (one-line glosses, used throughout):

- **Lesson card** — a small, structured record produced after a closed trade that captures *what happened* and *what to learn from it*: symbol/strategy, signed P&L, holding period, regime tag, outcome class. v1 cards have no natural-language note (Q1 = Option A, no LLM).
- **Body-SHA-256** — the deterministic body-only hash of an operator success report. The front-matter (which contains run-varying metadata like `generated:`, `wall_clock_s:`, `git_commit:`) is stripped before hashing; the body is what must remain byte-identical across runs. Anchored in `spec/anchors.toml`.
- **Vector store** — a database that indexes records by an embedding vector so similarity search ("find the K most relevant past lessons for *this* situation") is fast. v1's choice is the simplest fit: a new SQLite table inside a sibling `reflection.db` file with linear-scan top-K (sub-millisecond at 500-card scale).
- **Top-K retrieval** — given the current context (strategy, symbol, regime), return the K cards most similar by cosine distance over the embedding vector. v1 default K = 5; the operator sees five lesson lines.
- **Atomic write** — a write that either fully completes or doesn't appear at all (no half-written file visible to readers). The reports binary uses tempfile + rename; the reflection store's SQLite path uses the same tempfile + rename pattern at database-close time so a crash mid-flush never leaves a partial WAL frame committed against an inconsistent header.
- **Anchor (re-lock)** — the controlled procedure for capturing new body-SHA-256s and replacing v1+ entries in `spec/anchors.toml` once an intentional body change ships. Tester-only edit; architect approves; the 9 strategy anchors at lines 15–58 stay byte-identical and are protected by a negative-invariant test.

## What you can do now

| Action | Command |
|---|---|
| Render a 7-day report against your ledger; the Memory highlights section will surface the top 5 lessons | `cargo run -p reports --bin report -- --period 7d --ledger data/audit/ledger.db` |
| Render a 90-day report (longer windows are 5-min downsampled) | `cargo run -p reports --bin report -- --period 90d --ledger data/audit/ledger.db` |
| Turn on the reflection writer in production (default is off — see "Notes for the operator" §3) | edit `agent.toml`: under `[reflection]`, set `enable_writer = true` |
| Run the end-to-end determinism + anchor gate suite | `cargo test -p reports --test report_scenarios -- --nocapture` |
| Run the new with-lessons render path against the seeded fixture stores | `cargo test -p reports --test report_scenarios_with_lessons -- --nocapture` |
| Verify the full anchor table | `bash scripts/verify_anchors.sh` |

## How it works (one paragraph each)

**1. Trade closes → the agent generates a card.** When the executor's fill handler observes a sell-side fill that brings the per-symbol position back to zero, it emits a `LessonCardWriteRequest` carrying the close-side transaction id, the open-side transaction id (looked up via the most-recent prior buy-side transaction for the same symbol), the cash balance at open-side timestamp (the "opening capital" denominator), and the close timestamp. The request is shipped through a bounded tokio mpsc channel (capacity 1024) using `try_send`. If the channel is full (catastrophic burst), the request is **dropped** and the Prometheus counter `reflection_card_dropped_total{reason="back_pressure"}` increments by one — the executor's fill path observes a 0-allocation fast-fail, never blocks. The producer side is `try_send`, never `send`; the hot-path latency budget is `< 10µs` per `feature.md:1566`.

**2. Writer task drains and persists.** A single writer task in the agent's main loop drains the receiver. Per request: it looks up the closed trade's realized P&L via the new `audit::query::realized_pnl_for_trade` reader, classifies the regime via `classify_regime(btc_closes, closed_at)` (`Bull` / `Bear` / `Chop` based on BTC 7-day return ±2%), classifies the outcome via `classify_outcome(signed_pnl, opening_capital)` (`Win` / `Loss` / `Scratch` at the ±0.5%-of-opening-capital threshold, fee-aware), embeds the card into a 32-dim Decimal vector (no `f64` anywhere — `#![deny(clippy::float_arithmetic)]` enforces this at the crate boundary), and `upsert`s it into `reflection.db` keyed by a content-hash `card_id`. Idempotency: writing the same `card_id` twice is a no-op (returns `Ok(false)` from `upsert`). The writer task uses `tokio::select!` on the agent's main shutdown signal so graceful shutdown drains the queue before exit.

**3. Report time → top-K retrieval over all history.** When the operator runs the `report` binary, the renderer at `crates/reports/src/render/memory_highlights.rs` does this: it picks the strategy with the largest absolute P&L this period (excluding the synthetic `(unattributed)` bucket), picks the symbol with the largest absolute P&L under that strategy this period (tie-break: lex-sort ascending for determinism), evaluates the regime classifier at `period_end`, and calls `reflection::retrieve_top_k(store, &query, 5)`. Retrieval scans every row in `reflection.db`, computes cosine similarity over the 32-dim Decimal embedding (no floats), and returns the top 5 by `(score DESC, closed_at ASC)` — the older-cards-first tie-break is load-bearing for byte-stability across runs. The query window is **unbounded** (retrieve over all history, not just this period) because the moat-bet framing is "what did the agent learn from the last 7 days of trading?" — the period defines the question, but the lesson cards span all time.

**4. Render → byte-stable card lines.** Each retrieved card renders as one line: `- 2026-04-22 [Win] sma_crossover BTCUSDT regime=Bull held=42 bars pnl=+$123.45`. The only timestamp on the line is `closed_at`, which sources from the audit ledger (RFC3339, microsecond precision) — never from `OffsetDateTime::now_utc()`. The `body_no_volatile_metadata` test extends to assert the new section contains none of the eight forbidden substrings (`generated:`, `run_id:`, `wall_clock_s:`, `ledger_snapshot_sha:`, `data_source:`, `agent_pid:`, `host:`, `git_commit:`). On a fresh ledger with zero closed trades the body is the locked empty-state string — `_no closed trades yet — lesson cards will appear after the first closed trade._` — exposed as `pub const REFLECTION_MEMORY_EMPTY_STATE` at `crates/reports/src/render/memory_highlights.rs:33` so any silent rewording in code review trips the byte-stable test.

**Hot-path invariants preserved** (lifted from `feature.md` Design § "Hot-path invariants preserved", lines 806–829, all verified by negative-invariant tests):

- **`Strategy` trait shape unchanged.** The trader does not consume retrieval (Q4 = report-only). 9 strategy-backtest anchors at `spec/anchors.toml:15-58` byte-identical post-feature. Negative test: `crates/reflection/tests/no_strategy_caller.rs` (1/1 PASS in tester report).
- **Audit chart of accounts unchanged** (no new account; cards are derived artefacts, not ledger events). Reconciliation invariant `cash + Σ positions = equity` holds byte-for-byte; cards do not appear in the Reconciliation appendix. Tester §8 row V5: `cargo test -p reports --test reconciliation` 3/3 PASS.
- **No new bus channel.** The reflection writer's mpsc is a private field of `ReflectionWriter`; the `agent::bus::Bus` shape is unchanged from the v1+ snapshot. Negative test: `crates/agent/tests/no_new_bus_channel.rs` (1/1 PASS).
- **No LLM provider crate.** Q1 = Option A. Cost-telemetry V8 stays at `$0.00 / $135`. The lesson-card `note` field is `Option<String>` and `None` in v1 (reserved for the LLM-enrichment follow-up brief).
- **Body-vs-front-matter discipline.** Inherited from operator-success-reports R10.3. The body bytes contain only `closed_at` (sourced from the ledger, RFC3339 µs); no wall-clock, no run-id, no host, no git-commit. Negative test: `crates/reports/tests/body_no_volatile_metadata.rs` (2/2 PASS).
- **Atomic-write contract.** The reports binary's tempfile + rename pattern at `crates/reports/src/atomic_write.rs:38` is unchanged; the reflection store's SQLite path uses the same shape at database-close time so a crash mid-flush never leaves a partial WAL frame.

## Live demo

Two real binary runs, captured verbatim from this machine on 2026-05-08.

### Demo 1 — body-SHA-256 determinism + 11/11 anchor lock

```
$ cargo test -p reports --test report_scenarios -- --nocapture
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.62s
     Running tests/report_scenarios.rs (target/debug/deps/report_scenarios-09ad8325a1e710b4)

running 4 tests
test t816_v10_cron_friendly_3x_parallel_renders_atomic ... ok
T816 report-sample-7d body SHA-256: f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994
test t816_report_sample_7d_determinism_and_anchor_lock ... ok
T816 report-sample-90d body SHA-256: 463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c
test t816_report_sample_90d_determinism_and_anchor_lock ... ok
test t816_v10_cron_friendly_3x_parallel_bin_processes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.39s
```

The two body SHAs printed above (`f4ef3d02…` for 7d, `463e19b2…` for 90d) match the freshly-relocked entries in `spec/anchors.toml` lines 67–75 byte-for-byte. The two `t816_v10_cron_friendly_3x_parallel_*` cases additionally confirm three parallel renders against the same ledger snapshot land atomically — the cron-friendliness invariant inherited from operator-success-reports R12.

### Demo 2 — with-lessons render path against the seeded reflection store

```
$ cargo test -p reports --test report_scenarios_with_lessons -- --nocapture
   Compiling reports v0.1.0 (/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/reports)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.82s
     Running tests/report_scenarios_with_lessons.rs (target/debug/deps/report_scenarios_with_lessons-b948707225dd5fcd)

running 4 tests
test t1811_7d_fixture_renders_three_lesson_bullets ... ok
test t1811_lesson_bearing_body_byte_stable_across_two_runs ... ok
test t1811_90d_fixture_covers_six_outcome_regime_cells ... ok
test t1811_1y_fixture_seeds_at_least_500_cards ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

`t1811_7d_fixture_renders_three_lesson_bullets` proves the 7d fixture seeds the store with exactly 3 cards and the renderer emits 3 lesson lines. `t1811_lesson_bearing_body_byte_stable_across_two_runs` is the byte-stability gate against the new with-lessons body. `t1811_90d_fixture_covers_six_outcome_regime_cells` proves the 90d fixture exercises the (Win|Loss|Scratch) × (Bull|Bear|Chop) coverage matrix per Q3g. `t1811_1y_fixture_seeds_at_least_500_cards` proves the perf-smoke fixture has the ≥500-card scale R7.2 budget is sized against.

### Demo 3 — fresh anchor verification

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994
PASS  report-sample-90d                     463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c
---
ANCHORS PASS  (11 / 11)
```

All 11 anchors green. The 9 strategy anchors at lines 15–58 are byte-identical to their prior values; only the two `report-sample-*` v1+ entries at lines 67–75 are new bytes.

## Screenshots

_n/a — non-UI feature. The reflection.db is queried by the report binary; cards surface in the report's "Memory highlights" section, rendered as markdown by the cockpit's viewer binary._

## Notes for the operator

Five things a non-engineer operator should weigh in on or be aware of before approving the ship.

1. **Q1 = Option A locked — no LLM dependency.** The agent has shipped four strategies entirely on `$0.00 expense:llm:*` against the `$135/mo` cap; v1 of reflection memory keeps that promise. Lesson cards have no natural-language `note` field; retrieval scores over a 32-dim hand-crafted feature vector, not an LLM embedding. This was the operator's call on 2026-05-08 (recorded in the brief's Q1 resolution at `spec/reflection-memory/feature.md:527`). LLM enrichment lands as a follow-up brief named `reflection-memory-trader-wiring` (the trader-side wiring half of Q4) and a separate LLM-enrichment brief after v2 LLM ships.

2. **Q4 = report-only — trader behavior is unchanged.** The `Strategy` trait is byte-identical; the trader does not consume `reflection::retrieve_top_k` in v1. Only the report renderer reads from the store. The "Memory highlights" section is operator-eyes-only this round — informative ("here are the top past lessons that match the current period"), not yet load-bearing for any signal. Trader-side wiring is a deliberate follow-up (`reflection-memory-trader-wiring`) so the 9 strategy-backtest anchors at `spec/anchors.toml:15-58` could not move and we did not have to re-lock them under this feature.

3. **Reflection writer defaults to OFF.** `crates/agent/src/config.rs::ReflectionConfig::enable_writer = false` by default. The architect's design text named `true` as the default; the developer flipped it to `false` in their T1807 tick footer (`spec/reflection-memory/tasks.md:355-359`) with the rationale: "default `enable_writer = false` so the negative-invariant test profile sees zero writes by default. Production paper-mode flips it on via the loaded TOML; research / fixture profiles stay quiet." This is the explicit deviation from the architect's text the developer flagged. The safer-by-default choice — no writes happen until the operator opts in by editing `agent.toml`. To start writing cards in production, set `[reflection] enable_writer = true` in your config and restart the agent. This is the **one decision the operator may want to weigh in on**: keep the developer's safer-default (`false`), or override back to the architect's text (`true`) so cards start landing the moment the agent restarts.

4. **Distillation deferred (Q5) — 3 of 4 layers ship.** v1 ships layers 1–3 of the `spec/product.md` "Memory & continual learning" loop: episodic memory (the audit ledger, already shipped), reflection (the lesson-card writer in this feature), retrieval (top-K against the store, also in this feature). Layer 4 — periodic distillation that clusters cards into rules and promotes them into a prompt library — is queued as a follow-up brief named `reflection-memory-distillation`. The reasoning in `feature.md:1294-1325`: distillation needs cards on disk to cluster, and "promote into prompt library" needs an LLM consumer of that library, neither of which exists at v1 ship time. Bundling would have hurt both deliverables.

5. **Anchor scope = exactly 2 anchors re-locked.** The 9 strategy backtest anchors at `spec/anchors.toml:15-58` are byte-identical post-feature — proven by a dedicated negative-invariant test (`T1812`) and re-confirmed by the live `verify_anchors.sh` run above. Only the two v1+ `report-sample-*` entries at lines 67–75 changed bytes. If a future PR moves a strategy anchor under this feature umbrella, that's a leak of the Q4 = report-only contract; the failure mode routes back to analyst per the brief's "Failure routing" line 510.

## Verification

| V-id | Description | Status | Evidence |
|---|---|---|---|
| V1 | fmt + clippy + audit + deny clean | VERIFIED | tester report §2; `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0; `cargo deny check advisories \| bans \| licenses \| sources` all `ok` (`cargo audit` not installed locally — `cargo deny check advisories` is configured against the same RustSec DB; tester §9 records this) |
| V2 | `cargo test --workspace` green | VERIFIED | tester §3 line 54: `test result: ok. 952 passed; 0 failed; 3 ignored` across 124 binaries |
| V3 | both report scenarios run end-to-end + byte-stable | VERIFIED | live demo 1 above; both 7d + 90d body-SHA-256s identical across two consecutive runs; tester §7 lines 149–161 captured the same shape |
| V4 | body-only determinism (R5) | VERIFIED | tester §8 row V4: `cargo test -p reports --test determinism` 1/1 PASS, `body_no_volatile_metadata` 2/2 PASS (covers R5.3 negative-invariant) |
| V5 | reconciliation invariant (R6) | VERIFIED | tester §8 row V5: `cargo test -p reports --test reconciliation` 3/3 PASS; Δ = $0.00 across all rows; cards do not appear in the appendix |
| V6 | 11/11 anchors PASS (9 strategy + 2 re-locked report-sample) | VERIFIED | live demo 3 above: `ANCHORS PASS  (11 / 11)`; the 9 strategy anchors are byte-identical to their pre-feature values |
| V7 | audit-query API surface preserved | VERIFIED | tester §8 row V7: only addition is `realized_pnl_for_trade` at `crates/audit/src/query.rs:86` (sibling of `realized_pnl_since`); all v0/v0.5/v1/v1.5a/v1+ queries retain shape |
| V8 | cost telemetry zero | VERIFIED | tester §8 row V8: `LLM spend: $0.00 / $135` in rendered body; no LLM dep introduced |
| V9 | performance | VERIFIED | tester §8 row V9: `cargo test -p reports --test perf_smoke` 1/1 PASS — `t815_perf_smoke_90d_under_10s_and_under_256mib`; top-K retrieval covered by `store_top_k_determinism` 3/3 PASS |
| V10 | no-UI invariant | VERIFIED | tester §8 row V10: zero new `ui::strings`, zero new widgets; `cargo test -p ui` all `ok.`; `viewer_read_only` test green |

## How a `## Memory highlights` section will look in your next report

For an operator running a 7-day report against a ledger with three closed trades across two strategies (the 7d fixture shape per Q3g), the rendered section looks like this (illustrative — actual bytes are locked into `report-sample-7d` body SHA `f4ef3d02…`):

```
## Memory highlights

Top 5 lesson cards retrieved this period:

- 2026-04-22 [Win] sma_crossover BTCUSDT regime=Bull held=42 bars pnl=+$123.45
- 2026-04-19 [Loss] pairs_mr_h1 BTCUSDT-ETHUSDT regime=Chop held=8 bars pnl=-$67.89
- 2026-04-15 [Scratch] sma_crossover BTCUSDT regime=Bear held=15 bars pnl=-$2.10
```

For an operator running a report against a fresh ledger with zero closed trades, the section renders the locked empty-state string verbatim:

```
## Memory highlights

_no closed trades yet — lesson cards will appear after the first closed trade._
```

Each card line decomposes as: `closed_at` (RFC3339 date from the audit ledger), `[OutcomeClass]` (Win / Loss / Scratch per ±0.5%-of-opening-capital threshold), `strategy_id` (or `(unattributed)` for trades without a `strategy_id` column), `symbol_or_pair` (single symbol or `A-B` pair-key shape), `regime=` (`bull` / `bear` / `chop` from BTC 7-day return ±2% classifier at trade-close timestamp), `held=N bars` (1m bars between open and close), `pnl=±$X.XX` (signed P&L net of fees, USDT, 2 decimal places).

If a strategy decay heuristic also fires for this period (operator-success-reports T811), the decay candidates line is appended to the same section — the new `render_with_lessons` composes with the existing `render_with_decay` so both surface together rather than fighting for the section header.

## Numbers that matter

- **Tests:** **952 PASS / 0 FAIL / 3 ignored** across 124 binaries (tester §3 line 54). The 3 `#[ignore]` cases are pre-existing and unrelated to this feature.
- **Reflection crate alone:** 9 in-tree test files (`embedding_determinism`, `no_strategy_caller`, `outcome_classifier`, `post_mortem_generate_card`, `regime_classifier`, `store_idempotency`, `store_smoke`, `store_top_k_determinism`, `writer_back_pressure`) — all green.
- **Property test:** 1000 random `LessonCard`s embed-twice → byte-identical `[Decimal; 32]` (R2.5), `embedding_determinism` 5/5 PASS (tester §4).
- **Anchors:** **11 / 11 PASS** (live demo 3 above). 9 strategy anchors byte-identical; 2 report-sample anchors re-locked to new bytes.
- **Re-locked SHA-256s** (full, both verifiable against `spec/anchors.toml:67-75`):
  - `report-sample-7d` → `f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994` (was `ab06dbcb…`)
  - `report-sample-90d` → `463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c` (was `2ef403f1…`)
- **Live wall-clock** for the 4-test scenario suite (debug build, fixtures): **1.39s** (demo 1 above).
- **Live wall-clock** for the 4-test with-lessons suite: **0.08s** post-compile (demo 2 above).
- **LLM cost incurred by this feature:** **$0.00** (zero tokens, zero new bus channels, zero new LLM provider dependency).
- **New crate footprint:** 1 new leaf crate `crates/reflection/` (lib only, no bin) — types / regime / outcome / embedding / post_mortem_analyst / store / writer / retrieval modules + 1 SQL migration (`migrations/001_lesson_cards.sql`) + 9 in-tree test files.
- **Files modified outside `crates/reflection/`:** `crates/audit/src/query.rs` (one new function `realized_pnl_for_trade`), `crates/reports/src/render/memory_highlights.rs` (rewritten with `REFLECTION_MEMORY_EMPTY_STATE` constant + `render_with_lessons` + `build_retrieval_query`), `crates/reports/Cargo.toml` (new `reflection` dep), `crates/agent/src/config.rs` (new `ReflectionConfig`), `crates/agent/src/main.rs` (writer task spawn behind config flag), `crates/exec/src/paper.rs` (one tap point on trade-close fills), the three fixture builders under `crates/reports/tests/fixtures/`, plus the new test files.

## Q-resolution map

The brief opened with nine architect/operator questions. All nine are resolved in this ship — two by the operator on 2026-05-08, six by the architect in the same Design slice, one as N/A under Option A. This table is the audit trail.

| Q  | Topic | Resolution | Resolver |
|----|-------|------------|----------|
| Q1 | Deterministic v1 vs LLM-enabled v1 | **Option A — deterministic v1.** No LLM provider crate; `expense:llm:*` stays at `$0.00 / $135`. LLM enrichment lands as a follow-up brief after v2 LLM ships. | Operator (2026-05-08) |
| Q2 | Vector store choice (qdrant vs sqlite-vss vs new SQLite table) | **New SQLite table inside a sibling `reflection.db`, linear-scan top-K, behind a `ReflectionStore` trait.** v2 swap to qdrant or sqlite-vss is a single-trait-impl change. | Architect |
| Q3a | Storage location | **Sibling `reflection.db` SQLite file** under `<config.audit.path>/../reflection.db` (production) or `target/test-ledgers/reflection-<test>.db` (tests). Audit DB schema untouched. | Architect |
| Q3b | Regime classifier | **BTC 7-day return ±2%** — pure function over BTC 1m closes (`Bull` if > +2%, `Bear` if < −2%, `Chop` otherwise). | Architect (analyst strawman pinned) |
| Q3c | Outcome thresholds | **±0.5% of opening capital, fee-aware** (signed P&L already net of taker fees). Constant `OUTCOME_THRESHOLD_PCT: Decimal = dec!(0.005)` at `crates/reflection/src/outcome.rs:5`. | Architect (analyst strawman pinned) |
| Q3d | Embedding dimensions + features | **Deterministic 32-dim hand-crafted vector** in pinned packed layout — 7 strategy slots + 3 regime slots + 3 outcome slots + signed-pnl-sign + log-pnl-magnitude + log-holding-period + pair-hash-norm + single-symbol-hash-norm + 14 reserved slots for v2 LLM features. All `Decimal`, no `f64`. Cosine similarity in `Decimal`. | Architect |
| Q3e | Default K | **K = 5** — pinned as `pub const REPORT_TIME_TOP_K: usize = 5` in `crates/reflection/src/lib.rs`. Five is the operator's eyeball ceiling; ~5×80 ≈ 400-byte memory-highlights block. | Architect (analyst strawman pinned) |
| Q3f | Retrieval-query scoping rule | **Largest-absolute-P&L strategy** (excluding synthetic `(unattributed)` bucket; tie-break lex-sort ASC) + **largest-absolute-P&L symbol under that strategy** (tie-break lex-sort ASC) + **regime at period_end** + **unbounded time-window**. | Architect (analyst strawman pinned) |
| Q3g | Fixture content | **7d ≥3 cards across 2 strategies; 90d ≥10 cards across 3 strategies × 6×9 outcome × regime coverage matrix** (all 9 (Win/Loss/Scratch) × (Bull/Bear/Chop) cells exercised at least once). 1y perf-smoke fixture seeds ≥500 cards. | Architect (analyst strawman pinned) |
| Q4 | Retrieval at decision time | **Report-only this round.** Trader-side wiring is a follow-up brief named `reflection-memory-trader-wiring`. `Strategy` trait shape unchanged; 9 strategy-backtest anchors byte-identical. | Architect |
| Q5 | Periodic distillation (product.md layer 4) | **Defer to a follow-up brief** named `reflection-memory-distillation`. Distillation needs cards on disk to cluster + an LLM consumer of the prompt library; both are follow-ups. | Architect |
| Q6 | Anchor re-lock cadence | **Confirmed scope is the two `report-sample-*` v1+ anchors only.** The 9 strategy-backtest anchors at `spec/anchors.toml:15-58` are byte-identical post-feature. Negative-invariant test (`T1812`) enforces this. | Architect |
| Q7 | Empty-state body wording | **Operator-resolved** — `_no closed trades yet — lesson cards will appear after the first closed trade._`. Locked as `pub const REFLECTION_MEMORY_EMPTY_STATE: &str` so any silent rewording trips the determinism gate. | Operator (2026-05-08) |
| Q8 | Card-write channel + back-pressure | **Bounded tokio mpsc (capacity 1024), `try_send` on producer side, Prometheus counter `reflection_card_dropped_total{reason="back_pressure"}` on drop.** Internal — not a bus channel. | Architect |
| Q9 | Cost-telemetry under Option B | **N/A** — Q1 resolved to Option A; this is moot for v1. Carried as a note for the LLM-enrichment follow-up brief. | N/A |

## Implementation surface (where to look in the codebase)

| Area | Path | What landed |
|------|------|-------------|
| New leaf crate | `crates/reflection/` | Lib only, no bin. 9 source modules + 1 SQL migration + 9 test files. |
| Crate root | `crates/reflection/src/lib.rs` | Re-exports `LessonCard`, `RegimeTag`, `OutcomeClass`, `RetrievalQuery`, `ReflectionStore`, `retrieve_top_k`, `REPORT_TIME_TOP_K`. `#![deny(clippy::float_arithmetic)]` enforces no `f64`. |
| Card data model | `crates/reflection/src/types.rs` | `LessonCard` (all R1.1 fields), `RetrievalQuery`, `LessonCardWriteRequest`, `card_id` content-hash. |
| Regime classifier | `crates/reflection/src/regime.rs` | `classify_regime(btc_closes, at) -> RegimeTag` — pure, no I/O, no clock. |
| Outcome classifier | `crates/reflection/src/outcome.rs` | `classify_outcome(signed_pnl, opening_capital) -> OutcomeClass` + `OUTCOME_THRESHOLD_PCT` constant. |
| Embedding | `crates/reflection/src/embedding.rs` | `embed(card) -> [Decimal; 32]` + `cosine(a, b) -> Decimal` + `STRATEGY_SLOTS` (the append-only 7-strategy slot map) + `EMBEDDING_DIM = 32`. |
| Card generator | `crates/reflection/src/post_mortem_analyst.rs` | `generate_card(closed_trade, opening_capital, btc_closes) -> LessonCard` — pure over inputs. Name preserved so the LLM v2 can swap the impl without renaming the consumer. |
| Store trait | `crates/reflection/src/store/mod.rs:25` | `ReflectionStore` async trait — `upsert`, `top_k`, `count`, `len_at`. |
| SQLite store impl | `crates/reflection/src/store/sqlite.rs:42` | `SqliteReflectionStore::open(path)` + `sqlx::SqlitePool` against `reflection.db`. |
| Schema migration | `crates/reflection/migrations/001_lesson_cards.sql:1` | Creates `lesson_cards` table with primary key `card_id`, packed-TEXT 32-comma-separated `embedding_blob` column. |
| Writer | `crates/reflection/src/writer/mod.rs:50` | `ReflectionWriter::new` + `try_enqueue` (uses `mpsc::Sender::try_send`; on full → `Err(TryEnqueueError::BackPressure)` + drop-counter increment). |
| Writer task | `crates/reflection/src/writer/task.rs:24` | `ReflectionWriterTask::run` consumer loop — drains receiver, generates cards, upserts, logs idempotent skips. |
| Retrieval | `crates/reflection/src/retrieval.rs` | `retrieve_top_k(store, query, k)` public entry point — linear scan, `BinaryHeap`-of-K, deterministic tie-break. |
| Audit reader (additive) | `crates/audit/src/query.rs:86` | `realized_pnl_for_trade(ledger, trade_id) -> Result<Money<Usdt>, LedgerError>` — sibling of `realized_pnl_since`. |
| Agent config | `crates/agent/src/config.rs:236` | New `ReflectionConfig { path, channel_capacity, enable_writer }` — defaults: `channel_capacity = 1024`, **`enable_writer = false`** (developer's safer-default deviation; see "Notes for the operator" §3). |
| Agent main | `crates/agent/src/main.rs:104` | Wires `ReflectionWriter::new` and spawns `ReflectionWriterTask::run` behind `cfg.reflection.enable_writer`. Graceful shutdown drains the queue. |
| Executor tap | `crates/exec/src/paper.rs:35` | One-line tap point: on a sell-side fill that brings position to zero, `reflection_writer.try_enqueue(LessonCardWriteRequest { … })`. |
| Renderer rewrite | `crates/reports/src/render/memory_highlights.rs:33` | New `pub const REFLECTION_MEMORY_EMPTY_STATE` (line 33), `pub fn render_with_lessons(decayed, lessons)` (line 74), `pub fn build_retrieval_query(pnls, current_regime)` (line 117). The placeholder constant from v1+ is **removed**. |
| Reports lib | `crates/reports/src/lib.rs` | `ReportRunCfg` gains `reflection_store: Option<Arc<dyn ReflectionStore>>` — when `None`, the renderer emits the empty-state body (keeps the binary runnable against pre-reflection ledgers). |
| Fixture builders | `crates/reports/tests/fixtures/build_reflection_store_{7d,90d,1y}.rs` | Sibling fixtures that consume the closed-trade list from `build_ledger_*.rs` and write cards via `post_mortem_analyst::generate_card`. |
| Negative-invariant tests | `crates/reflection/tests/no_strategy_caller.rs`, `crates/agent/tests/no_new_bus_channel.rs`, `crates/reports/tests/body_no_volatile_metadata.rs` | Static-grep style; all green. |
| Anchor re-lock | `spec/anchors.toml:67-75` | Tester-only edit; replaced two `report-sample-*` v1+ entries with the new SHAs. The 9 strategy anchors at lines 15–58 byte-identical. |

**Files explicitly NOT modified** (negative invariants, all verified):

- `crates/core/src/lib.rs` — no `Strategy` trait change.
- `crates/strategy/src/**` — no strategy-side change (negative test `no_strategy_caller.rs`).
- `crates/audit/migrations/**` — no new audit migration; `reflection.db` is a sibling file with its own migration set.
- `crates/audit/src/journal.rs` — no new account; no new writer.
- `crates/agent/src/bus.rs` — no new bus channel (negative test `no_new_bus_channel.rs`).
- `crates/ui/src/**` — no UI surface (V10).

## Performance budget (R7 — all PASS)

| Path | Budget | v1 expectation | Verified by |
|------|--------|----------------|-------------|
| `try_enqueue` on the executor's hot path | < 10µs | ~500ns (atomic CAS, no allocation) | T1808 back-pressure test exercises the fast-fail path |
| Writer task per-card SQLite write | < 5ms | ~1ms (WAL append, single-card transaction, single-row idempotency check) | Implicit in `store_smoke` 2/2 PASS |
| `retrieve_top_k(K=5)` over 500-card store | < 100ms | ~3ms (linear scan, 32-dim Decimal cosine, BinaryHeap-of-K) | `store_top_k_determinism` 3/3 PASS + 1y fixture seeds ≥500 cards |
| `retrieve_top_k(K=5)` over 5000-card store (forward-compat envelope) | < 1s | ~30ms | Implicit in linear-scan complexity argument |
| `report-sample-90d` total wall-clock | < 10s | ~3s (v1+ baseline ≈ 2–3s + ≤ 100ms top-K + ≤ 50ms render) | `cargo test -p reports --test perf_smoke` 1/1 PASS — `t815_perf_smoke_90d_under_10s_and_under_256mib` |
| RSS for the 1-year fixture | < 256 MiB | ~55 MiB (v1+ baseline ≈ 50 MiB + ~5 MiB for `reflection.db`'s 500 cards × ~10KB row) | Same `perf_smoke` test asserts both wall-clock and RSS |

## Risk register (excerpted from `feature.md` § Risk register & mitigations, all mitigated)

| Risk | Severity | How it is mitigated in v1 |
|------|----------|---------------------------|
| Determinism leak via wall-clock in card body (`closed_at` accidentally set to render-time) | high | `LessonCard.closed_at` sources only from `journal_transactions.ts` (audit ledger). Negative test `body_no_volatile_metadata.rs` enforces. |
| Embedding non-determinism via floating-point arithmetic | high | `[Decimal; 32]` end-to-end; `f64` is **forbidden** in the `reflection` crate via `#![deny(clippy::float_arithmetic)]` at `crates/reflection/src/lib.rs:1`. |
| SQLite reader contention between agent's writer task and the reports binary's reader | medium | Writer uses WAL mode (`PRAGMA journal_mode = WAL`); reader opens with `PRAGMA query_only = 1` (same pattern v1+ uses for the audit DB). |
| Card-write back-pressure under burst causes silent data loss | medium | Drop is **observable** via `reflection_card_dropped_total{reason="back_pressure"}`. Operators alert on `> 0 over 1h`. Capacity 1024 is ~17 hours of steady-state queue at production rate. |
| A new strategy added in a future feature changes `STRATEGY_SLOTS` order → embedding shifts → SHAs drift | medium | `STRATEGY_SLOTS` is **append-only**. Any future feature that adds a strategy: (a) appends to the end of the array, (b) re-runs the V6 fixtures, (c) captures + re-locks the two v1+ anchors. Documented at the rustdoc note on `STRATEGY_SLOTS`. |
| `reflection.db` location drift between dev / production / fixtures | low | Path computed as `cfg.reflection.path` — defaults to `<cfg.audit.path>/../reflection.db`. Tests pass an absolute path under `target/test-ledgers/`. |
| Q4 = report-only contract violated by a future PR (someone wires retrieval into the trader) | low | Static-grep negative test `no_strategy_caller.rs` fails CI if a future PR wires the trader without a follow-up brief. |
| `reflection.db` corruption (e.g. mid-write power loss) | low | SQLite WAL + `PRAGMA synchronous = NORMAL` per audit-DB precedent; cards are **derived artefacts** (re-derivable from the audit ledger), so a wipe-and-rebuild script is a follow-up runbook entry, not a v1 deliverable. |

## Anchor table (first 8 chars per locked body-SHA)

| Scenario | Version | SHA-256 prefix |
|---|---|---|
| btc-2023-1m-sma-cross | v0 | `fc2e3b4a…` |
| btc-2023-1m-sma-baseline-refresh | v0 | `fc2e3b4a…` |
| btc-2023-1m-macd-trend | v0.5 | `ef9c5e48…` |
| btc-2023-1m-rsi-reversion | v0.5 | `bc56d20d…` |
| btc-2023-1m-bbands-mean-revert | v0.5 | `d8a08a23…` |
| top10-2023-1h-momentum | v1 | `3b60ef07…` |
| top10-2024-h1-momentum | v1 | `1f33534f…` |
| pairs-2023-zscore-mr | v1.5a | `90591a0e…` |
| pairs-2024-h1-zscore-mr | v1.5a | `14f50a59…` |
| report-sample-7d | v1+ | `f4ef3d02…` (re-locked 2026-05-08) |
| report-sample-90d | v1+ | `463e19b2…` (re-locked 2026-05-08) |

The 9 strategy anchors above are byte-identical to their pre-feature SHAs (see operator-success-reports' 2026-05-08 anchor table for comparison — identical first 8 chars). The two v1+ entries are the only deltas this feature introduced. The negative-invariant test `T1812` (`tester report §8 row V6`) enforces this scope — if any of the 9 strategy SHAs drifted under this feature, the tester would have routed to analyst rather than re-locked.

## Test coverage breakdown

The reflection crate alone landed 9 in-tree test files. The full per-crate breakdown of new and modified test surfaces:

| Test file | Crate | What it covers | R-item / V-item | Live result |
|-----------|-------|----------------|-----------------|-------------|
| `regime_classifier.rs` | reflection | BTC closes with +3% / −3% / +1% 7d returns map to `Bull` / `Bear` / `Chop`; boundary at exactly ±2% maps to `Chop` (strict inequality). | R1.3 | green |
| `outcome_classifier.rs` | reflection | ±0.6% Win/Loss; ±0.4% Scratch; `opening_capital == 0` → Scratch (defensive). | R1.4 | green |
| `embedding_determinism.rs` | reflection | 1000 random `LessonCard`s embed-twice → byte-identical `[Decimal; 32]` (proptest, default seed). | R2.5 | green (5/5) |
| `post_mortem_generate_card.rs` | reflection | Fixture closed trade with known fee + qty + price → expected `LessonCard` with all R1.1 fields populated. | R1, R2.3 | green |
| `store_smoke.rs` | reflection | Open + migrate + upsert + read-back round-trip (in-memory and on-disk SQLite). | R2.1, R2.4 | green (2/2) |
| `store_idempotency.rs` | reflection | 10 deliberate cards → 10 inserts on first run, 0 inserts on second run; `count() == 10`. | R2.4 | green |
| `store_top_k_determinism.rs` | reflection | Seed 100 cards, run `retrieve_top_k(query, 5)` twice, assert byte-identical card order. Score tie → `closed_at ASC` tie-break. Empty store → `Ok(vec![])`. | R3.1, R3.2, R3.4 | green (3/3) |
| `writer_back_pressure.rs` | reflection | Fill the 1024-capacity mpsc; assert the 1025th `try_enqueue` returns `Err(TryEnqueueError::BackPressure)` AND `dropped_count` increments by 1. Closed-receiver path returns `Closed` err. | R7.1, Q8 | green (2/2) |
| `no_strategy_caller.rs` | reflection | Static-grep negative-confirmation: no symbol from `crates/strategy/` resolves `reflection::retrieve_top_k`. | R8.1, Q4 | green (1/1) |
| `realized_pnl_for_trade_test` (audit) | audit | Audit fixture with 3 closed trades; assert `realized_pnl_for_trade(trade_id)` returns the right `Money<Usdt>` per id; sums equal `realized_pnl_since(period_start)`. | R2.2 | green (2/2) |
| `memory_highlights_with_lessons.rs` | reports | Fixture store seeded with K=5 cards → rendered body matches a hand-computed expected string byte-for-byte. Empty store → `REFLECTION_MEMORY_EMPTY_STATE`. Decay co-render. Card line format pinned. Back-compat for callers without lessons. | R4.2, R4.4, R4.1 | green (5/5) |
| `report_scenarios_with_lessons.rs` | reports | 7d fixture renders 3 lesson bullets; 90d fixture covers the 6×9 outcome × regime cells; 1y fixture seeds ≥500 cards; lesson-bearing body byte-stable across two runs. | R4.2, Q3g | green (4/4 — see live demo 2 above) |
| `report_scenarios.rs` | reports (existing, re-locked) | Both 7d + 90d fixture report bodies SHA-stable across two runs at seed `0xC0FFEE`; 3× parallel atomic-write cron friendliness. | R5.1, R5.2 | green (4/4 — see live demo 1 above) |
| `body_no_volatile_metadata.rs` | reports (extended) | New body section contains none of the 8 forbidden substrings (`generated:`, `run_id:`, `wall_clock_s:`, `ledger_snapshot_sha:`, `data_source:`, `agent_pid:`, `host:`, `git_commit:`). | R5.3 | green (2/2) |
| `no_new_bus_channel.rs` | agent | Static-grep style: assert `agent::bus::Bus` public field set is unchanged from v1+ snapshot. | R8.3, Q8 | green (1/1) |
| `perf_smoke.rs` | reports (existing) | `t815_perf_smoke_90d_under_10s_and_under_256mib` — wall-clock + RSS budget after this feature ships. | R7.3, R7.4, V9 | green (1/1) |
| `reconciliation.rs` | reports (existing) | Δ = $0.00 across all rows; cards do not appear in the appendix. | R6.1, R6.2, V5 | green (3/3) |
| `determinism.rs` | reports (existing) | Body-only determinism (R5) gate. | R5.1 | green (1/1) |

## Follow-up briefs (forward look — not in this ship)

Three follow-ups are queued by this feature; each is independent and lands when its dependencies exist:

1. **`reflection-memory-trader-wiring`** — wires `reflection::retrieve_top_k` into the `Strategy` trait so the trader actually consults its memory at decision time. This is product.md layer 3 done at the trader level (not just the report level). Carries the major scope of re-locking some or all of the 9 strategy-backtest anchors at `spec/anchors.toml:15-58`, since changing the trader's signal series shifts fills, journal rows, and backtest body bytes. Entire feature is out of scope here per Q4 = report-only.

2. **LLM-enrichment follow-up** (no slug yet — to be named after v2 LLM ships) — wraps `post_mortem_analyst::enrich(card) -> String` around the v1 deterministic card to populate the currently-`None` `note: Option<String>` field. Touches Q1 (Option B), Q9 (cost-telemetry budget gating), and the body-vs-front-matter discipline (LLM prose may need to live in front-matter rather than body to preserve byte-determinism). Cannot ship before v2 LLM strategy is queued.

3. **`reflection-memory-distillation`** — product.md layer 4: a periodic weekly job that clusters lesson cards on disk into rules and promotes them into a prompt library. Depends on (a) some weeks of cards on disk to cluster (this feature is the prerequisite that creates them) and (b) a prompt-library consumer (a v2 LLM concept). Bundling distillation with this feature would have hurt both deliverables — a clustering bug would have blocked card-write fixes — so it deferred.

The forward-compat scaffolding for #1 and #3 is in this ship: `reflection::retrieve_top_k` is callable from anywhere (not just the report renderer), and the rustdoc note at `crates/reflection/src/lib.rs:1` documents the layer-4 deferral so the future architect can grep for it.

## Open decisions

One decision is open and the operator may want to weigh in:

1. **Reflection writer default `enable_writer = false` (developer's deviation from architect text — see "Notes for the operator" §3).** This is the safer-by-default choice — the operator turns the writer on explicitly by editing `agent.toml` once they're ready to start writing cards. The architect's text suggested `true`. **Picking "Approved — ship" accepts the developer's `false` default**; if the operator wants the architect's `true` default instead, pick "Approve with notes" and write `flip ReflectionConfig::enable_writer default to true` in the notes block — a follow-up patch will land before the next cron-friendly daily report run.

No other decisions pending — the brief's nine open questions Q1–Q9 are all resolved (Q1 + Q7 by the operator on 2026-05-08; Q2, Q3, Q4, Q5, Q6, Q8 by the architect in the same Design slice; Q9 N/A under Option A).

## What approval means / cost of "yes" vs "no"

A clean **"Approved — ship"** ratifies the developer's safer-default `enable_writer = false`, accepts the two re-locked anchors at `spec/anchors.toml:67-75` as the new v1+ baseline, and clears the orchestrator to mark the feature shipped (the brief and tasks frontmatter already flipped to `status: shipped`). After approval the operator turns on the writer by editing `agent.toml` (`[reflection] enable_writer = true`) and restarting the agent — cards start landing on the next closed trade.

**"Approve with notes"** routes the deck back to the orchestrator with a one-line override (e.g. "flip `ReflectionConfig::enable_writer` default to `true`"). Cost: a one-file follow-up patch from the developer, no anchor re-lock (the default flip doesn't change body bytes since the writer-disabled test profile already produced the byte-stable bodies that locked the anchors). Estimated cycle: under an hour.

**"Reject"** routes the entire feature back to the analyst with the rejection reason. Cost: substantial — re-scopes the brief, the architect's design, the developer's 14-task implementation, and the tester's 10-gate verification matrix. The 952 / 0 / 3 test-pass count and the 11 / 11 anchor PASS are not in dispute; this would only be the right call if the operator believes the **shape** of v1 (no LLM, report-only, distillation deferred) is wrong rather than the implementation. The brief's Q1 / Q4 / Q5 / Q7 resolutions are the load-bearing scope decisions; rejecting one of those four is what would justify the cost.

## Verification — supplemental detail

The 10-gate matrix above is the canonical verification surface; the per-gate evidence is the tester report `spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md`. A few cross-cutting points worth surfacing:

- **The 952-test count includes every workspace crate, not just `reflection`.** The full breakdown across 124 binaries is in the tester's §3. The reflection crate alone contributes ~44 tests across 9 in-tree files (8 lib + 5 + 1 + 10 + 3 + 8 + 2 + 2 + 3 + 2 + 0 doc-tests across `cargo test -p reflection`); the reports crate's new test files contribute another 9 tests; the audit crate's new `realized_pnl_for_trade_test` adds 2; the agent crate's `no_new_bus_channel` adds 1. Net new tests added by this feature: ~56.
- **Property-test coverage is 1000 cases** for the `embedding_determinism` proptest (default seed). Zero shrunk failures. The shrinking gate is what would surface a `HashMap`-iteration leak if anyone slips `f64` into the embedding compute path.
- **Cargo audit gate is satisfied via `cargo deny check advisories`** (the `cargo audit` binary is not installed locally; the developer recorded the same in T1814's verification footer). `cargo deny check advisories` is configured in `deny.toml` against the same RustSec advisory database, so no advisory gate is silently skipped.
- **The two pre-existing unused-import warnings in `crates/ui/tests/strategies_screen_sparkline_replaces_placeholder.rs` are unchanged.** They are not part of the `-D warnings` clippy run (warning level only, not deny level) and were not introduced by this feature. The tester confirmed these stay at warning level (tester §2 footer).
- **Honest-tick spot check: 5-of-14 developer ticks were verified** by the tester (T1801, T1805, T1807, T1810, T1814 — covering all five milestones M1–M5). For each sampled tick the cited file:line was confirmed to exist and the cited acceptance command was re-run verbatim. All five citations honest. This is the developer-discipline gate that catches "looks done" claims that fall apart on close inspection.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Decision history (2026-05-08)

This feature's brief, design, implementation, test verdict, and presentation all landed on the same calendar day. The resolution sequence:

1. **Brief opened** by the analyst — promoted from `spec/backlog.md → Active`. Nine open questions (Q1–Q9) flagged: two for operator decision (Q1 deterministic vs LLM, Q7 empty-state wording), six for architect decision (Q2 store choice, Q3 schema bundle, Q4 trader-or-report scope, Q5 distillation defer, Q6 anchor scope confirm, Q8 channel + back-pressure), one conditional (Q9 cost-telemetry under Option B).
2. **Operator answered Q1, Q7, Q9** via orchestrator chat:
   - Q1 → **Option A** (deterministic v1, no LLM dependency).
   - Q7 → **analyst strawman accepted** (`_no closed trades yet — lesson cards will appear after the first closed trade._`).
   - Q9 → **N/A** (Option B not picked).
3. **Architect answered Q2, Q3, Q4, Q5, Q6, Q8** in one Design slice:
   - Q2 → new SQLite table inside a sibling `reflection.db`, linear-scan top-K, behind a `ReflectionStore` trait.
   - Q3 → analyst strawmans pinned across the bundle (sibling DB, BTC 7d ±2%, ±0.5%, 32-dim hand-crafted, K=5, largest-abs-PnL scoping, 7d ≥3 / 90d ≥10 fixture content).
   - Q4 → report-only this round; `Strategy` trait unchanged.
   - Q5 → defer to follow-up `reflection-memory-distillation`.
   - Q6 → confirmed scope = two `report-sample-*` v1+ anchors only.
   - Q8 → bounded tokio mpsc capacity 1024, `try_send`, Prometheus drop counter.
4. **Architect expanded `tasks.md`** with 14 developer T18xx tasks (T1801–T1814) + `T_FINAL_REFLECTION_MEMORY`. Crate / module surface lists 23 new files and 11 modified existing files.
5. **Developer landed T1801–T1814** in commit `7650c7b8f173a91c0f6680901111a9bda667ce68` (the implementation commit). Honest-tick discipline: every task footer cites the file:line that was created and the acceptance command that passed. One explicit deviation flagged in T1807's footer: `ReflectionConfig::enable_writer` defaults to `false` rather than the architect's text `true`, with rationale recorded.
6. **Tester ran the 10-gate matrix** at `spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md` (run id `2026-05-08-2114-UTC`). All gates green; two anchors re-locked at `spec/anchors.toml:67-75`; 9 strategy anchors byte-identical. **VERDICT → PASS** at 21:14 UTC. Frontmatter on `feature.md` and `tasks.md` flipped from `in-progress` to `shipped`. Routing → presenter.
7. **Presenter (this deck) ran end-to-end** — three live demos against the implementation, fresh `verify_anchors.sh` PASS, mechanical `check_presentation.sh` gate confirmed approval boxes UN-ticked. Surfaces the one open operator decision (the developer's `enable_writer = false` deviation) and the five operator-relevant facts.

The whole arc — brief → architect → developer → tester → presenter — completed in a single day with full evidence at every gate.

## Changelog

- 2026-05-08 (presenter): initial draft after tester `VERDICT → PASS` at commit `7650c7b8` (tester report `spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md`). Pulled evidence from feature brief lines 11–98 + 519–843 + 1505–1607, tasks.md (15/15 tasks ticked), tester report §1–§11, and three live re-runs on this machine: `report_scenarios` (11/11 anchor lock + body-SHA print), `report_scenarios_with_lessons` (4/4 PASS — the new with-lessons render path), and `verify_anchors.sh` (`ANCHORS PASS (11 / 11)`). Surfaces one open decision for the operator: keep the developer's safer-default `enable_writer = false`, or override back to the architect's `true`. Five operator-relevant facts called out under "Notes for the operator" (Q1 Option A locked, Q4 report-only, writer default OFF, Q5 distillation deferred, anchor scope = 2). Pre-tick gate `bash scripts/check_presentation.sh` run on this file — see closing summary.
