---
slug: binance-corpus-expansion
status: proposed
owner: architect
updated: 2026-06-15
version: 0.1.0
trace: REQ-BINANCE-CORPUS-EXPANSION-001
---

# Expand the Binance backtest corpus — wider down-market coverage for strategy-checking

## Changelog

- 2026-06-15 (analyst): v0.1.0 draft. Scopes the highest-value addition to the
  pinned Binance OHLCV corpus that firms up the
  [trend-following-down-market-hedge finding](../dev-notes/realdata-simple-strategy-survey-2026-06-13.md)
  (currently rests on only **2 down-market data points**, AVAX/DOT 2024).
  Recommends a **bounded v0.1**: add the **2021–2022 hourly history** for the
  existing 10-symbol universe as a **new sibling corpus root**
  (`data/binance-2122/`) with its own `REVISION.toml` pin. Surfaces the single
  riskiest architect question (the `ReplayFeed` path layout carries **no
  timeframe segment** — daily bars cannot share the 1h root). Handoff →
  architect.

## Why (motivation)

The 2026-06-13 real-data survey
([`spec/dev-notes/realdata-simple-strategy-survey-2026-06-13.md`](../dev-notes/realdata-simple-strategy-survey-2026-06-13.md))
ran the four shipped simple strategies (sma / macd / rsi / bbands) on the real
Binance **hourly** corpus — all 10 symbols, 2023 + 2024, pinned `3a8b96c4` — net
of 4 bps taker cost, vs buy-and-hold. The headline finding:

> Passive dominates in UP markets; **trend-following (SMA, MACD) protects capital
> in DOWN markets** — flat-to-positive while buy-and-hold bled.

That down-market protection property is the genuinely interesting, ship-relevant
result. But the survey's own § Caveats is honest that it rests on a **2-point
down-market sample** (AVAX 2024 B&H −8.2%; DOT 2024 B&H −19.6%) — "suggestive,
not statistically conclusive." Every other (symbol·year) cell in 2023–24 was a
crypto bull run, because **2023 and 2024 were both broadly up years**. The corpus
simply does not contain a real bear market.

**2022 was the deep crypto bear**: BTC ≈ −64% on the year (≈$47k → ≈$16.5k),
ETH ≈ −67%, with the LUNA/3AC (May–Jun 2022) and FTX (Nov 2022) capitulations
inside it — a market-wide, multi-month drawdown across the whole universe, not
two idiosyncratic alt-coin dips. 2021 was a two-peak bull (Apr + Nov) with a
−50% mid-year drawdown. Adding **2021–2022 hourly** for the existing 10 symbols
turns a 2-point down sample into a ~10–12-symbol-year down/sideways sample (the
entire universe in 2022 + the H1-2021→H2-2021 drawdown), which is what the
survey says is needed to firm up the hedge finding. It also broadens the base
for any future strategy-checking re-run (the same `realdata_simple_strategy_survey`
harness and any successor robustness work).

This is a **data-corpus feature**, not a strategy feature: it adds re-fetchable,
pinned historical bars. It ships **no alpha verdict** — the survey re-run that
consumes the wider corpus is downstream work, out of scope here (see § Non-goals).

## Current state (map)

| Concern | Where | Note |
|---|---|---|
| Corpus on disk | `data/binance/<SYM>/<YEAR>/<MM>.parquet` | 10 symbols × 2023–24, **hourly**, 240 files, ~5.5 MB, gitignored |
| Revision pin | `data/binance/REVISION.toml` (`3a8b96c4…`) | per-file SHA-256 + aggregate; only file tracked in git for this root |
| Fetcher | [`crates/data/src/bin/fetch_binance_klines.rs`](../../crates/data/src/bin/fetch_binance_klines.rs) | `--symbols --start --end --interval --out --emit-revision-manifest`; already supports `1d` and `2021-01-01` |
| Aggregate-SHA algo | [`crates/data/src/revision.rs`](../../crates/data/src/revision.rs) | single source of truth, writer + verifier identical (ADR-0032 § 2) |
| Reader | [`crates/data/src/replay_feed.rs:101`](../../crates/data/src/replay_feed.rs) `parquet_files()` | joins `<root>/<symbol>/<year>/…` — **NO timeframe path segment** |
| Backtest data-path | [`crates/backtest/src/realdata.rs`](../../crates/backtest/src/realdata.rs) (feature `realdata`) | verifies REVISION + loads via `ReplayFeed::merge_symbols` |
| Sibling-root precedent | `data/binance-broaduni/` | 35 mid-cap symbols, 2023–24 hourly, 744 files, ~16 MB, own `REVISION.toml`, own `.gitignore` stanza (lines 19–24) |
| ADRs | [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md) (corpus pin), [ADR-0040](../architecture/adr/0040-yahoo-realdata-path.md) (Yahoo data domain) | corpus-pin + data-domain contracts |

**Load-bearing structural fact** (drives the riskiest question below):
`ReplayFeed::parquet_files()` resolves `parquet_root.join(symbol).join(year)` and
recursively globs `*.parquet`. There is **no `/1h/` or `/1d/` directory level**
in the Binance layout — the interval is recorded only in `REVISION.toml`
metadata (advisory, not hashed). The **Yahoo** corpus *does* embed the interval
(`<SYM>/1d/<year>/<MM>.parquet`); the Binance corpus does not. Therefore a corpus
root is implicitly **single-timeframe**: `data/binance` *is* the 1h corpus. You
cannot drop daily bars into `data/binance/BTCUSDT/2024/` without colliding with
the existing hourly `01.parquet…12.parquet`.

## Scope — the v0.1 addition

**Recommended v0.1 (highest value, lowest risk): add 2021–2022 HOURLY for the
existing 10-symbol universe as a new sibling root `data/binance-2122/`.**

- **Symbols (10, unchanged):** BTC, ETH, BNB, SOL, XRP, ADA, DOGE, AVAX, DOT,
  LINK USDT pairs — exact same universe as `data/binance`, so the survey harness
  and any cross-corpus comparison line up symbol-for-symbol.
- **Range:** 2021-01-01 .. 2022-12-31 inclusive, **hourly** (`--interval 1h`).
- **Layout:** `data/binance-2122/<SYM>/<YEAR>/<MM>.parquet` — identical shape to
  the existing root, just a different root directory. 10 × 24 months = **240
  parquet files**, ≈ **5.5 MB on disk** (matches the existing root's footprint;
  hourly month parquet ≈ 25 KB).
- **Pin:** one new `data/binance-2122/REVISION.toml`, written by
  `fetch_binance_klines --emit-revision-manifest`, **tracked in git**; the bulk
  parquets stay gitignored via a new `.gitignore` stanza mirroring
  `binance-broaduni` (lines 19–24).

**Why a sibling root, not extending `data/binance`:** the existing `3a8b96c4`
pin and its aggregate SHA cover exactly the 2023–24 files. Adding 2021–22
*into* `data/binance` would force a **re-emit of `REVISION.toml`** → a new
aggregate SHA → and would silently change the file-set that any
`data/binance`-rooted reader globs. A sibling root keeps `3a8b96c4`
byte-identical (zero regression risk to the shipped survey and the four
`-realdata` anchors), follows the established `binance-broaduni` precedent
exactly, and makes "the 2021–22 corpus" a first-class, independently-pinnable,
independently-deletable artifact. This is the **durable** choice (see § Decision
framing).

### What is deliberately NOT in v0.1 (and why)

- **Daily timeframe.** High research value (the survey § Caveats explicitly wants
  a daily study; SMA 20/50 on daily = 20d/50d is a different, classic regime),
  but it is **blocked on the riskiest architect question** (no timeframe path
  segment). Sequencing daily *after* the architect rules on the layout avoids
  baking a wrong directory convention into a pinned, gitignored corpus that is
  painful to reshape later. Daily is the natural **v0.2** once the layout is
  locked. (If the architect picks layout option (b) below, daily becomes a
  trivial follow-on; if (a), it needs a one-line `ReplayFeed` change first.)
- **More symbols.** `binance-broaduni` already provides 35 mid-caps for
  *breadth* (2023–24). The survey's gap is **down-market depth over time**, not
  symbol count — adding more 2023–24 symbols adds more bull-market cells, which
  is exactly the regime the survey already has 18 of. Low marginal value for
  the stated motivation.
- **Sub-hourly (1m/5m).** Out of scope — different research question (microstructure
  / latency), large disk footprint (1m ≈ 60× hourly ≈ 330 MB for the same span),
  no demand from the survey finding.

## Pin / reproducibility approach (the riskiest non-structural concern)

The corpus is **never committed** (ADR-0032 § 2 / ADR-0040 data-domain). Only
the manifest is tracked. The reproducibility story for the new range reuses the
existing machinery verbatim:

1. **Fetch** (operator, one command, read-only historical HTTP):
   ```
   cargo run -p data --bin fetch_binance_klines -- \
     --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT \
     --start 2021-01-01 --end 2022-12-31 --interval 1h \
     --out data/binance-2122 --emit-revision-manifest
   ```
   The fetcher's `should_skip` idempotency check (row-count == expected bars/month
   for 1h) makes re-runs safe and resumable. No code change to the fetcher is
   required — it already accepts these flags.
2. **Pin.** `--emit-revision-manifest` writes `data/binance-2122/REVISION.toml`
   with a per-file SHA-256 map + the deterministic aggregate SHA
   (`compute_aggregate_sha`, [`revision.rs:105`](../../crates/data/src/revision.rs)).
   The aggregate is content-only (metadata excluded), so **the same fetch twice
   yields the same pin** — the determinism property ADR-0032 § 2 depends on.
3. **Track the manifest, ignore the bulk.** New `.gitignore` stanza:
   ```
   !/data/binance-2122/
   /data/binance-2122/*
   !/data/binance-2122/REVISION.toml
   ```
4. **Verify on read.** Any consumer routed at `data/binance-2122` verifies via
   `read_and_verify_revision_manifest` (re-hashes every file, recomputes the
   aggregate, returns the *recomputed* value — a hand-edit cannot fool it).
5. **Document the new pin** in the feature's report + a one-line backlog/runbook
   note so a future operator knows the canonical fetch command and the expected
   aggregate SHA (captured at fetch time by the tester).

**Disk / CI implications.** +5.5 MB local (gitignored — **zero** repo-size
impact; only the ~30 KB `REVISION.toml` is committed). CI builds on machines
**without** `data/binance-2122` continue to pass: any consumer must SKIP cleanly
when the corpus is absent (the `realdata_simple_strategy_survey` harness already
does this — it checks `…/2023/01.parquet` exists and prints `SKIP` otherwise; a
2021–22 consumer follows the same guard). **No new always-on CI cost.** The one
new always-runnable check is a cheap manifest-internal-consistency test
(re-hash the aggregate from the manifest's own `[files]` map — runs on the
committed `REVISION.toml` alone, no parquet needed), mirroring the existing
`yahoo_revision_verify` pattern.

## Acceptance criteria (v0.1)

- **AC1 (data present, pinned):** `data/binance-2122/` contains 240 hourly
  parquet files (10 symbols × 2021–2022) and a `REVISION.toml` whose recomputed
  aggregate SHA equals its claimed `[revision].sha256`. Captured aggregate SHA
  recorded in the feature report.
- **AC2 (`3a8b96c4` untouched):** `data/binance/REVISION.toml` aggregate SHA is
  **byte-identical** to before this feature (`3a8b96c4…`). The existing four
  `-realdata` anchors and `scripts/verify_anchors.sh` stay green.
- **AC3 (gitignore correct):** the bulk `data/binance-2122/*.parquet` are
  gitignored; **only** `data/binance-2122/REVISION.toml` is tracked. `git status`
  shows no parquet.
- **AC4 (re-fetch determinism):** re-running the fetch command against an existing
  `data/binance-2122/` produces a `REVISION.toml` with an **identical** aggregate
  SHA (no spurious churn). (Tester: fetch twice into two temp roots OR re-run with
  idempotency skip and diff the manifest's `[files]` + `[revision].sha256`.)
- **AC5 (consumer SKIP-safe):** a build / test run on a machine **without**
  `data/binance-2122/` does not fail — any new consumer guard prints SKIP, mirroring
  the existing survey harness.
- **AC6 (Decimal, never f64):** prices remain string-typed in parquet and parse
  via `parse_price_str` → `rust_decimal::Decimal` on read (unchanged path; assert
  no f64 introduced).
- **AC7 (spec-lint zero-new):** `python3 scripts/spec_lint.py` adds **zero** new
  violations vs the 70-violation baseline (all new spec links resolve).

## Non-goals

- **No alpha verdict / survey re-run.** Producing the firmed-up down-market
  finding by re-running `realdata_simple_strategy_survey` over 2021–22 is the
  *downstream* feature this corpus unblocks — not part of v0.1.
- **No daily / sub-hourly / extra symbols** in v0.1 (see § What is deliberately
  NOT in v0.1).
- **No live trading.** Fetch is read-only historical HTTP. (Per
  [`spec/dev-notes/live-trading-removed-2026-06-12.md`](../dev-notes/live-trading-removed-2026-06-12.md)
  — out of scope indefinitely.)
- **No new strategy code, no engine changes** beyond (possibly) a corpus-root
  selector if a consumer is wired in a later feature.

## Decision framing — durable over quick (operator pref 2026-05-28)

Two shapes for "where do the new bars live":

- **(a) Sibling root `data/binance-2122/` (Recommended) — durable.** Keeps the
  `3a8b96c4` pin byte-identical (zero regression to the shipped survey + four
  anchors), follows the `binance-broaduni` precedent exactly, makes the 2021–22
  corpus independently pinnable / deletable / re-fetchable. Effort: ~½ day
  (fetch + 1 gitignore stanza + 1 manifest-consistency test + report). **No
  v0.2 cleanup commitment.** This is the right base whether or not daily ever
  lands.
- **(b) Extend `data/binance/` in place — cheap-looking, NOT recommended.**
  Re-emits `data/binance/REVISION.toml` → new aggregate SHA → forces re-pinning
  the four `-realdata` anchors and re-validating the shipped survey against a
  changed file-set. Smaller directory count, but spawns anchor-rebaseline rework
  and couples the new range's fate to the existing one. **If-budget-tightens:**
  even then (a) is cheaper *over the sum* — (b)'s anchor re-pin is the expensive
  tail. (a) stays Recommended unconditionally here.

## Architecture findings (for the architect's M-T1 lock)

The cheap path (a) is **proven** not to spawn rework: a sibling root touches
neither `data/binance/REVISION.toml` nor any `realdata` anchor nor the survey
harness's hard-coded `data/binance` path — there is no carve-out, no MIGRATION
annotation, no `-realdata` re-emit. So here the durable choice and the
cheap-to-build choice **coincide** (sibling root is both). The expensive,
rework-spawning option is (b), which is why (b) is the fallback label.

## Open questions for the architect

1. **[RISKIEST] Timeframe path layout — does the corpus root encode the
   interval?** `ReplayFeed::parquet_files()` has **no `/1h/` segment**, so each
   Binance root is implicitly single-timeframe. v0.1 (hourly) is unaffected — a
   2021–22 *hourly* sibling root works today with zero reader changes. But the
   architect must decide the **forward** convention for the **daily v0.2**:
   - **(a)** keep the flat layout and make daily its **own root**
     (`data/binance-2122-1d/` or `data/binance-1d/`) — zero `ReplayFeed` change,
     more roots; OR
   - **(b)** add a `/1h/` ∣ `/1d/` segment to the Binance layout (aligning it
     with the Yahoo convention `<SYM>/<TF>/<YEAR>/…`) — one `ReplayFeed`
     path-join change + a one-time reshuffle of the *existing* `data/binance`
     1h files into `data/binance/.../1h/...` (which **would** re-emit `3a8b96c4`
     — anchor cost) OR a new root that adopts the segmented layout while the old
     root stays flat (layout skew).
     Recommended for the **architect's durable lock: (b) segmented layout on a
     go-forward basis** (new roots are segmented; the legacy flat `data/binance`
     stays as-is behind a compat shim) — it makes Binance and Yahoo converge and
     lets one root hold multiple timeframes. But this is an M-T1 architect call
     with an anchor-safety dimension; **v0.1 does not depend on it** (hourly
     sibling root is correct under either answer). Flagged now so v0.2 daily
     doesn't bake a convention by accident.
2. **Pin scope — one manifest for the whole 2021–22 root, or one per year?**
   Recommend **one root-level manifest** (matches ADR-0032 § "Per-scenario
   manifest — rejected"; finer-grained per-file tamper detection at no cost).
3. **Symbol-set drift over time.** All 10 USDT pairs traded on Binance for all of
   2021–22 *except* possibly a few early-2021 months for the newer listings
   (e.g. DOT listed Aug-2020, AVAX Sep-2020, LINK 2019 — all fine; verify SOL
   2021-01 and DOGE early-2021 have full hourly coverage). If any symbol-month is
   thin, the fetcher writes a short parquet and logs it; the architect should
   confirm whether v0.1 **tolerates** ragged early-2021 coverage (recommend: yes,
   document per-symbol first-available-month in the report — do **not** drop the
   symbol) or requires a uniform start. Recommend tolerate-and-document.
4. **Consumer wiring.** v0.1 ships data + pin only. Does the architect want a
   minimal `#[ignore]` smoke consumer (load one 2022 month, assert bars parse
   `Decimal`) inside v0.1 for AC1/AC5/AC6 evidence, or is the manifest-consistency
   test plus a manual recipe sufficient? Recommend the **one small `#[ignore]`
   smoke test** — it's the cheapest credible AC5/AC6 evidence and mirrors the
   survey harness's SKIP guard.

## References

- Survey finding (the why): [`spec/dev-notes/realdata-simple-strategy-survey-2026-06-13.md`](../dev-notes/realdata-simple-strategy-survey-2026-06-13.md) § Finding 1 + § Caveats.
- Corpus-pin contract: [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md) §§ 2.
- Data-domain contract: [ADR-0040](../architecture/adr/0040-yahoo-realdata-path.md).
- Sibling-root precedent: `data/binance-broaduni/REVISION.toml` + `.gitignore` lines 19–24.
- Aggregate-SHA algorithm: [`crates/data/src/revision.rs`](../../crates/data/src/revision.rs).
- Fetcher: [`crates/data/src/bin/fetch_binance_klines.rs`](../../crates/data/src/bin/fetch_binance_klines.rs).
- Reader (no-timeframe-segment fact): [`crates/data/src/replay_feed.rs`](../../crates/data/src/replay_feed.rs) `parquet_files()`.

## Design

_Architect, 2026-06-15 (M-T1 lock). Reviewed the fetcher CLI surface, the
`ReplayFeed` glob, the four sibling `.gitignore` stanzas, the Yahoo on-disk
layout, and ADR-0032/0040. The analyst's recommended defaults are adopted; the
one place I depart from the analyst's **lean** is Q1 (see below)._

### Open-question resolutions

| Q | Decision | One-line rationale |
|---|---|---|
| **Q1 [RISKIEST]** — forward timeframe-layout convention | **Own-root-per-timeframe (flat layout retained).** Locked in **[ADR-0056](../architecture/adr/0056-binance-corpus-timeframe-layout-convention.md)**. | A `/<TF>/` segment á la Yahoo would re-emit `data/binance`'s `3a8b96c4` pin (REGRESSION the four `-realdata` anchors) **or** skew `ReplayFeed`'s glob to two shapes; `ReplayFeed` takes no interval arg, so a Binance root is correctly a single-timeframe corpus selected by **root path**. I depart from the analyst's (b)-lean precisely because (b)'s convergence-with-Yahoo upside is outweighed by the anchor-re-emit / glob-skew cost — and Binance↔Yahoo *should* diverge (different access patterns). |
| **Q2** — pin scope | **One root-level `REVISION.toml`** for the whole 2021–22 root. | Matches ADR-0032 § "per-scenario manifest — rejected"; finer per-file tamper detection at no cost; exactly what `--emit-revision-manifest` already writes. |
| **Q3** — ragged early-2021 coverage | **Tolerate + document**, do not drop the symbol or require a uniform start. | The fetcher already writes a short parquet and logs the month; all 10 USDT pairs listed ≥ 2020 (SOL/AVAX/DOT 2020, DOGE/LINK earlier) so coverage is expected full, but a thin month must not fail the fetch. Developer records per-symbol first-available-month + bar totals in the report. |
| **Q4** — consumer wiring | **Ship one small `#[ignore]` smoke consumer** in v0.1 + the always-on manifest-consistency test. | Cheapest credible AC5 (SKIP-safe) + AC6 (`Decimal`, never f64) evidence; mirrors the survey harness's `…/2023/01.parquet` SKIP guard. |

### Sibling-root decision — CONFIRMED (option (a))

`data/binance-2122/` as a **new sibling root** is confirmed. The decisive,
verified facts:

- **`data/binance` stays byte-identical.** A sibling root touches neither
  `data/binance/REVISION.toml` (pin `3a8b96c4`, read at line 1 of that file) nor
  any `-realdata` anchor nor the survey harness's hard-coded
  `data/binance/BTCUSDT/2023/01.parquet` path
  ([`realdata_simple_strategy_survey.rs:112`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs)).
  AC2 asserts this. **Do NOT extend the existing root** — that is the rejected
  option (b) and is REGRESSION-class.
- **Precedent is exact.** `data/binance-broaduni/`, `data/binance-funding/`,
  `data/binance-basis/`, `data/defillama-stablecoins/` already follow this
  pattern — own root, own `REVISION.toml`, own three-line `.gitignore` stanza
  (`.gitignore` lines 16–39). `data/binance-2122/` slots in identically.

### Zero-fetcher-code-change — CONFIRMED (read the CLI surface)

`crates/data/src/bin/fetch_binance_klines.rs` already exposes every flag the
fetch needs — verified against the source, not the brief:

- `--symbols` (`-s`, comma-delimited) · `--start` · `--end` · `--interval`
  (default `1h`) · `--out` (default `data/binance`) · `--force` ·
  `--emit-revision-manifest` (lines 47–77).
- Output path is built at lines 487–491 as
  `cli.out.join(SYMBOL).join(year).join("{MM}.parquet")` — the **flat**
  `<out>/<SYMBOL>/<YEAR>/<MM>.parquet` shape ADR-0056 D1 retains; passing
  `--out data/binance-2122` is the only knob.
- `should_skip` (lines 383–424) makes re-runs **idempotent + resumable**: for
  `1h` it reads the existing parquet's row count and skips when it equals
  `expected_bars_per_month` (744 for a 31-day month). This is what gives AC4
  (re-fetch determinism) for free.
- `--emit-revision-manifest` calls `data::revision::write_revision_manifest`
  (lines 548–556) and prints `[REVISION] … aggregate SHA: <hex>` — the value the
  developer/tester captures for AC1.

**Conclusion: no `.rs` edit, no `Cargo.toml` edit.** The CLAUDE.md library-compat
checklist is **N/A** — this feature adds **no new dependency**; it reuses the
shipped `data` binary and the shipped `revision` module verbatim.

### Anchor-safety & determinism guardrails

- **Anchors:** the four `-realdata` anchors + `scripts/verify_anchors.sh` stay
  **119/119 green by construction** — no `data/binance` file, no anchored report,
  and no `anchors.toml` row is in scope. This satisfies the "anchors do not
  mutate silently" rule (no ADR anchor change requested).
- **Determinism / report format:** the new `REVISION.toml` is the only emitted
  artifact. Its aggregate SHA is **content-only** (the `[revision.metadata]`
  wall-clock `generated_at` is carved out of the hash, per ADR-0032 § D2), so the
  same fetch twice yields the same aggregate — that is the AC4 property. No
  byte-comparable report body is produced by this feature, so there is no
  front-matter/body split to enforce here.
- **Decimal, never f64:** unchanged read path. Prices stay `Utf8` in parquet
  (fetcher schema, lines 350–354 / the `open/high/low/close` columns) and parse
  to `rust_decimal::Decimal` via the existing `ReplayFeed` path on read — the
  smoke consumer (Q4) asserts this for AC6.

### Why the day-1 baseline-equity-divergence e2e gate is N/A (explicit)

The CLAUDE.md non-negotiable "every strategy overlay or sizing-modifier ships
with a baseline-equity-divergence e2e test from day 1" **does not apply** here:
this feature adds **no strategy code, no overlay, no sizing modifier, and
produces no equity curve**. It is a data-acquisition + on-disk-layout feature
(re-fetchable pinned bars + one `.gitignore` stanza + a manifest pin). There is
no "un-targeted baseline equity" to diverge from because nothing computes equity.
The evidence-analogue that plays the gate's role here is **AC1** (recomputed
aggregate SHA == claimed — the data is what the pin says) plus **AC4** (re-fetch
determinism — the pin is reproducible). The downstream survey re-run that *does*
produce equity is out of scope (§ Non-goals) and will carry its own gates.

### Anchor-neutrality note (verification handle for the tester)

The single load-bearing safety property — `data/binance/REVISION.toml` ==
`3a8b96c4…` byte-for-byte after this feature — is verifiable with a one-liner
the tester runs before and after: the first non-comment line of
`data/binance/REVISION.toml` is `sha256 = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7"`.
Coupled with `scripts/verify_anchors.sh → 119/119`, this closes AC2.

## Implementation

Developer, 2026-06-15.

- **M1 (fetch):** `cargo run -p data --bin fetch_binance_klines -- --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT --start 2021-01-01 --end 2022-12-31 --interval 1h --out data/binance-2122 --emit-revision-manifest` completed exit 0. 240 files written. Aggregate SHA: `4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62`. No ragged early-2021 coverage: all 10 symbols had 744 bars in 2021-01.
- **M2 (gitignore):** stanza added to `.gitignore` after `binance-basis` block. `git ls-files --others --exclude-standard data/binance-2122/` → `data/binance-2122/REVISION.toml` only. No parquet untracked.
- **M3 (tests):** `crates/data/tests/binance_2122_revision_consistency.rs` added. Two tests: `manifest_internal_consistency` (always-on, T6) and `smoke_consumer_btcusdt_2022` (`#[ignore]`, T7). T6 passes with no parquet. T7 reads 17 507 BTCUSDT bars and asserts `Decimal` close prices.
- **M5 (docs):** report at `spec/binance-corpus-expansion/reports/fetch-2026-06-15-binance-2122.md`. Backlog Recent note added.
- **Anchor safety:** `data/binance/REVISION.toml` SHA == `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` — unchanged. `verify_anchors.sh` → 119/119.
- **Clippy:** `cargo clippy --tests -p data -- -D warnings` → 0 warnings.
- **Baseline-equity-divergence gate:** N/A (no strategy code, no overlay, no equity produced). AC1 + AC4 are the evidence-analogues.

## Changelog

- 2026-06-15 (architect): M-T1 lock. Resolved Q1–Q4; wrote **ADR-0056**
  (own-root-per-timeframe forward convention, registered atomically in the ADR
  README). Confirmed sibling-root `data/binance-2122/` (option (a)) and the
  zero-fetcher-code-change claim by reading the `fetch_binance_klines` CLI
  surface (flat `<out>/<SYM>/<YEAR>/<MM>.parquet`, idempotent `should_skip`,
  content-only aggregate SHA). Stated the baseline-equity-divergence e2e gate
  N/A with justification (no equity produced). Library-compat checklist N/A (no
  new dep). Ownership → architect; tasks.md updated for the developer. HANDOFF →
  developer.
