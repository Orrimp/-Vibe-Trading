# Anchor-gate coverage audit — every anchored scenario, 2026-09-26

**Provenance.** Produced by a read-only audit agent, then spot-verified by the orchestrator before
being written. **Independently re-verified here:** the three off-anchor in-test pins (`4d8192af`,
`4a744788`, `5037accb` — each **0 occurrences** in `anchors.toml`); `ETH_ANCHOR_SHA = c854ff2b…`
(**0 occurrences**, while `anchors.toml` pins `e59a5f87…`); `sharpe-comparison-realdata` has **no
producer** (`grep -rl` over `crates/` returns nothing); `strategy_anchors_unchanged.rs` reads from
disk (`std::fs::read_to_string` at `:403`, no `Command::new`). **Not independently re-verified:** the
per-scenario producer line numbers, the wall-clock table in § 5.7, and the resolver-directory census
in § 5.3. Those are the agent's reading; treat them as a map, not as measurement.

**Scope.** `evidence/anchors.toml` carries **119 rows over 77 distinct scenario names**. For each of
the 77: who emits it, whether any test re-runs it and compares against the anchor, what condition
such a re-run needs, and whether that is reachable.

Builds on bug-log `#93` (the corpus gate cannot see drift), `#111` (the `#67` blast radius was scoped
by lane), `#112` (condition = CWD × feature set), `#113` (a `_determinism` test that compares a run
against itself), `#114` (a precondition that could never succeed).

## 1. Terms, used strictly

- **anchor-comparing gate** — RE-RUNS the producer and compares the body-SHA against a value that is
  *also a row in `anchors.toml`*. Only this can see code-vs-evidence drift.
- **self-comparing test** — runs the producer twice, compares run 1 against run 2. Proves
  determinism. **Not coverage** (`#113`).
- **off-anchor pin** — re-runs, but compares against a literal that is *not* any `anchors.toml` row.
  Reads as coverage of the anchor; is not.
- **disk-hash gate** — hashes a committed body. Proves storage integrity. **Not coverage** (`#93`).

## 2. The counts

| class | count |
|---|---|
| distinct anchored scenarios | **77** |
| rows in `anchors.toml` | **119** |
| **real anchor-comparing coverage** | **15** — 5 GREEN, 10 RED |
| **off-anchor pin** | **4** |
| **self-comparing only** (the `#113` trap) | **2** |
| **no re-run gate of any kind** | **56** |
| | 15 + 4 + 2 + 56 = **77** |

By reachability: **CI-reachable 13**, local-only 62, **structurally unreachable 2**.

The 5 GREEN: `report-sample-{7d,90d}`, `btc-yahoo-2024-1d-sma-cross`,
`btc-2023-1m-sma-{cross,baseline-refresh}`.

The 10 RED: 4 `top10-*` in-test (`#[ignore]`, cause bisected to `#67`), 2 `m3_*` weights (**not**
`#[ignore]`d), 4 R-REPRO `-realdata`.

The 56 with nothing: 34 θ-surfaces + 1 MC + 15 forecast/report-binary scenarios +
`eth-2024-h1-sma-cross` + 5 backtest `-realdata`.

### The correction this audit forces on the previous tally

Last night's note said "5 gate-confirmed to reproduce". **Only 2 of those reproduce an
`anchors.toml` row.** The `btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}` gates pin
`4d8192af` / `4a744788` / `5037accb`, and **none of those three strings appears in `anchors.toml`** —
verified. They pin the *synthetic* body while the canonical row is the *real-data* body (17 544 bars
vs 525 601). That is deliberate and documented (`determinism.rs:538-541`, ADR-0045 § D6.3, and the
values are mirrored in `check_determinism_anchors.py:53-57`) — but the narrow consequence stands:
**those three anchors.toml rows have no re-run coverage.**

The count of genuinely-green anchor coverage is still 5, because the audit found three real gates
nobody had counted: `report-sample-{7d,90d}` and `btc-yahoo-2024-1d-sma-cross`.

## 3. Structurally unreachable (2)

1. **`sharpe-comparison-realdata`** — 2 rows, and **no code path at HEAD emits the name**.
   `sharpe_comparison.rs` emits four names, none of them the bare one. The resolved report dates to
   2026-05-19; `docs/dev-notes/retired-surface-inventory-2026-05-22.md:157` lists the surface as
   retired. **The anchor outlived its producer**, and `verify_anchors.sh` has reported PASS on both
   rows every day since, because it hashes a file. Disposition is a decision, not a measurement:
   relabel the rows as historical with no reproduction claim, or trace the rename and re-key them.
   Re-emitting is not available.
2. **`eth-yahoo-2024-1d-sma-cross`** — the anchored body's Data-source row reads
   `yahoo-cache:ETH-USD/1d/2024 rev=e018f876c36a`; `D-V0.1.3-1` moved `rev=` out of the body into
   front-matter, so no invocation at HEAD can produce that body. Its "second-witness" test
   (`run_yahoo_sma_ticker_flag.rs:228`, named `eth_ticker_sha_matches_anchor_70`) asserts against
   `c854ff2b…`, which **is not the anchor and appears nowhere else in the repo** — verified. `#93`
   with a written confession inside the test that looks like its coverage.

## 4. Why 62 are CI-unreachable — one shared reason

The bulk corpora are gitignored. `data/binance`, `data/binance-funding`, `data/binance-basis`,
`data/binance-broaduni`, `data/binance-2122`, `data/yahoo` each have **exactly one tracked file**
(`REVISION.toml`). Only `data/yahoo-sample/` is committed whole — 13 files — and it is the only
corpus with a green CI reproduction gate. That is not a coincidence; it is the design of story 6-12,
and it generalises: **a scenario whose corpus is committed needs no skip path at all.** Every
`unmeasured` in this audit traces to a corpus that is not committed.

## 5. Tests that look like coverage and are not — the full list

1. **`crates/reports/tests/strategy_anchors_unchanged.rs`** — the big one, and its own bug-log entry.
   Three tests (`:436`, `:489`, `:542`) check 25 anchor rows across three namespace tables against
   **report files read off disk** (`:403`); the module doc at `:29-31` says it *"mirrors
   `scripts/verify_anchors.sh:63-110`"*. It is the corpus gate re-implemented in Rust, **inside
   `cargo test --workspace`** — so a developer running the suite sees three green tests named
   `*_strategy_anchors_unchanged` covering exactly the scenarios that do not reproduce. Worse than
   `verify_anchors.sh`, because a *test* reads as a reproduction check.
2. **`crates/backtest/tests/multi_pair_determinism.rs:73,93`** — self-comparing; the file has no
   `ANCHOR` const. The two `pairs-*` scenarios are the **cheapest coverage gap in the corpus**:
   CI-runnable, no corpus needed, runner already present, needs one `assert_eq!`.
3. **`crates/backtest/tests/determinism.rs` T-D-13/14/15** — `realdata_*_determinism`,
   self-comparing. Now correctly sitting *next to* the R-REPRO gates, which is the right shape — but
   the names still read as coverage.
4. **`run_yahoo_sma_ticker_flag.rs:228`** — see § 3.2.
5. **`crates/ui/tests/lab_yahoo_anchor.rs`** — doc at `:48` claims the body-SHA is anchored as
   `btc-yahoo-2024-1d-sma-cross` and cites `8045623b…` at `:52`; that row pins `076929bb…`. The test
   body is a scaffold — `expected_final_equity`, `FINAL_EQUITY_TOLERANCE` and `EXPECTED_TRADE_COUNT`
   are all discarded with `let _ = …` — and it is both `#[ignore]`d and feature-gated.
6. **`param_sweep_e2e.rs:870`, `montecarlo_e2e.rs:475`** — these pin the scenario **NAME**. Genuinely
   valuable (a name drift orphans an anchor silently) and honest about it. Just not body coverage,
   and the 35 scenarios behind them have none.
7. **`crates/forecast/tests/sharpe_comparison_determinism.rs:317`** — renders a fixture twice.

## 6. A guard still checking a proxy for its condition

`determinism.rs::real_binance_data_available()` tests `data/binance/REVISION.toml`.`exists()` — the
**one tracked path** in that directory, standing in for 240 gitignored parquets. This is bug-log
`#66`'s exact shape, already corrected once in
`forecast::features::tests::windows_determinism_on_real_data`. Harmless in CI today only because
`--features realdata` is never enabled there, so these tests are compiled out rather than skipped.
Add `realdata` to a CI leg and the guard returns true while the bytes are absent. Cheap fix, same as
`#66`: guard on a parquet the run actually reads.

## 7. The θ-surfaces are cheap, and the blocker is the corpus, not compute

**32 of the 34 re-emit in 9–43 s**; only `v1-momentum-theta-surface-2023` (1460.8 s) and
`v1-mr-theta-surface-2023` (2257.6 s) are expensive — which matches the orchestrator's own measured
run of 2026-09-25 (74.4 min for all 34, two surfaces being 83 % of it). "Needs a 28-minute sweep" is
true of exactly two.

All 34 were re-emitted on 2026-09-25 and their anchor rows now match the new bodies, so a θ-surface
gate built today would very likely be **green** — the cheapest way to convert `#93`'s blind spot into
observed coverage for 34 of the 77. The blocker for the other 32 is the gitignored corpus.

One exception worth flagging: **`v1-momentum-2023-block-bootstrap-real-fy-mc` was NOT re-emitted**
on 2026-09-25 — it has exactly one generation, from 2026-05-30. It is therefore *not* part of the
1-26 re-lock and its reproduction state is genuinely unknown.

## 8. What the reproduction gate should be extended from

`#112` concluded the gate is an extension of `determinism.rs`, not a new sweep script. Worth adding:
a second, independent gate in this repo already implements every requirement `#112` and `#113`
derived, in about 60 lines — **`crates/backtest/tests/reproducibility_sample_figure.rs`**. It re-runs
the binary the runbook names, declares its corpus explicitly, asserts a **human-legible condition
witness before the SHA**, writes to `--reports-dir` tempdir, hashes with the repo's own
`scripts/hash_report.py` rather than a re-implementation, compares against the **anchor**, forbids
re-pinning by name (`#77`), and runs in CI on a fresh clone.

It leaves requirement (3) — a missing corpus would fail rather than report `unmeasured`. Which is
correct *there*, because its corpus is committed and can never legitimately be absent. That is the
whole trick, and § 4 is the general form of it.
