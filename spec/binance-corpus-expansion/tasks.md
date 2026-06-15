---
slug: binance-corpus-expansion
status: in-progress
owner: developer
updated: 2026-06-15
---

# Tasks — binance-corpus-expansion (v0.1.0)

Add **2021–2022 hourly** Binance OHLCV for the existing 10-symbol universe as a
new sibling corpus root `data/binance-2122/`, pinned via its own `REVISION.toml`.
**No fetcher code change** (confirmed against the CLI surface — see
[feature.md § Design](feature.md)). Architect M-T1 lock is **closed**; the four
open questions are resolved in [feature.md § Design](feature.md) and the forward
timeframe-layout convention is locked in
[ADR-0056](../architecture/adr/0056-binance-corpus-timeframe-layout-convention.md).

Legend: `[ ]` open · `[~]` in progress · `[x]` done. Owner in **bold**.

## M0 — Architect lock (CLOSED)

- [x] **architect** — M-T1: resolved Q1 (own-root-per-timeframe — **ADR-0056**),
      Q2 (one root-level manifest — yes), Q3 (tolerate + document ragged
      early-2021 — yes), Q4 (ship the `#[ignore]` smoke consumer — yes).
      Confirmed sibling-root option (a); `data/binance` stays byte-identical.
- [x] **architect** — ADR decision: v0.1 reuses the ADR-0032 pin contract
      **verbatim** on a new root (no amendment to 0032). The Q1 forward
      convention is a **new** ADR-0056, registered atomically. No new dependency
      → CLAUDE.md library-compat checklist N/A.

## M1 — Fetch + pin the 2021–22 hourly corpus (developer / operator)

- [ ] **developer/operator** — T1: run the fetch (read-only historical HTTP).
      **Exact command** (copy-paste verbatim):
      ```bash
      cargo run -p data --bin fetch_binance_klines -- \
        --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT \
        --start 2021-01-01 --end 2022-12-31 --interval 1h \
        --out data/binance-2122 --emit-revision-manifest
      ```
      _acceptance: exits 0; `data/binance-2122/` holds 240 parquet files (10
      symbols × 24 months) + a `REVISION.toml`._

      Long-running (10 × 24 months, 200 ms inter-request sleep ≈ a few min). This
      is > 2 min, so run it in the background and watch progress with this block:
      ```bash
      watch -n 10 'find data/binance-2122 -name "*.parquet" | wc -l; \
        echo "target=240"; ls data/binance-2122/REVISION.toml 2>/dev/null && echo "MANIFEST WRITTEN"'
      ```
- [ ] **developer/operator** — T2: capture the printed
      `[REVISION] … aggregate SHA: <hex>` line and per-symbol bar totals (grep
      the `[OK] …` lines or the `paginated klines` tracing). Note **any**
      symbol-month the fetcher logged as short (ragged early-2021 coverage, Q3)
      for the report. _acceptance: aggregate SHA + per-symbol first-available
      month recorded for the M5 report._
- [ ] **developer** — T3: verify the on-disk manifest is internally consistent
      via a one-off `data::revision::read_and_verify_revision_manifest(
      Path::new("data/binance-2122"))` (or the existing
      `crates/backtest/tests/realdata_revision_verify.rs` pattern pointed at the
      new root). _acceptance: recomputed aggregate == claimed `[revision].sha256`._

## M2 — Repo plumbing (developer)

- [ ] **developer** — T4: add the `.gitignore` stanza, placed **adjacent to the
      existing `binance-*` stanzas** (after the `binance-basis` block, ~line 31)
      with a one-line comment pointing at this feature:
      ```gitignore
      # 2021–22 hourly down-market corpus (feature binance-corpus-expansion,
      # 2026-06-15): same 10 symbols as data/binance, 2021–2022. Bulk parquets
      # gitignored (mirrors data/binance); only the REVISION.toml pin is tracked.
      !/data/binance-2122/
      /data/binance-2122/*
      !/data/binance-2122/REVISION.toml
      ```
      _acceptance: stanza mirrors the `binance-broaduni` shape (`.gitignore`
      lines 22–24)._
- [ ] **developer** — T5: confirm `git status --porcelain data/binance-2122/`
      shows **only** `data/binance-2122/REVISION.toml`; **no** `*.parquet`.
      _acceptance: AC3 — zero parquet staged/untracked-and-addable._

## M3 — Manifest-consistency test + smoke consumer (developer)

- [ ] **developer** — T6: add an **always-on** manifest-internal-consistency
      test that runs on the **committed `REVISION.toml` alone** (no parquet on
      disk), modeled on `crates/data/tests/yahoo_revision_verify.rs`. Suggested
      home: `crates/data/tests/binance_2122_revision_consistency.rs`. It parses
      `data/binance-2122/REVISION.toml`, recomputes the aggregate from the
      `[files]` map via `data::revision::compute_aggregate_sha` (the single
      source of truth, `revision.rs:105`), and asserts `== [revision].sha256`.
      _acceptance: passes in CI with no parquet present; AC1-partial, AC7-friendly._
- [ ] **developer** — T7: add an `#[ignore]` smoke consumer (Q4 = yes). Suggested
      home: a `#[test] #[ignore]` in the same test file or
      `crates/backtest/tests/`. It must:
      1. **SKIP-guard** when the corpus is absent — mirror
         `realdata_simple_strategy_survey.rs:112`: if
         `data/binance-2122/BTCUSDT/2022/01.parquet` is not a file, print
         `eprintln!("SKIP binance-2122 smoke: corpus absent")` and return.
      2. `ReplayFeed::new("data/binance-2122", true)` (or `new_with_pace`), load
         one 2022 month for one symbol via the standard read path, assert ≥ 1 bar.
      3. Assert `open/high/low/close` parse to `rust_decimal::Decimal` via the
         existing price-parse path (AC6 — no f64 introduced).
      _acceptance: passes with `--ignored` when the corpus is present; the
      always-on suite SKIPs cleanly when it is absent (AC5)._

## M4 — Tester gate

- [ ] **tester** — AC1: 240 parquet present; recomputed aggregate == claimed;
      record the captured aggregate SHA in `reports/`.
- [ ] **tester** — AC2: `data/binance/REVISION.toml` still
      `sha256 = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7"`
      byte-for-byte (diff the first non-comment line before/after);
      `scripts/verify_anchors.sh` → **119/119** (four `-realdata` anchors green).
- [ ] **tester** — AC4: re-fetch determinism — fetch twice into two temp roots
      OR re-run with idempotent skip; diff `[files]` + `[revision].sha256`;
      assert identical.
- [ ] **tester** — AC5: a build/test run on a path **without**
      `data/binance-2122/` passes (the smoke consumer SKIPs; the
      manifest-consistency test still runs on the committed manifest).
- [ ] **tester** — AC6: no f64 in the read path (prices `Utf8` → `Decimal`);
      the T7 smoke assertion is the evidence.
- [ ] **tester** — AC7: `python3 scripts/spec_lint.py` — **70** findings, **zero
      new** vs baseline (all feature.md / tasks.md / ADR-0056 links resolve).
- [ ] **tester** — write `reports/test-2026-06-15-binance-corpus-expansion.md`,
      including the canonical fetch command + captured aggregate SHA so the pin
      is reproducible.

## M5 — Docs / pin record

- [ ] **developer** — T8: one-line note in `spec/backlog.md` Recent (via
      `spec-update`) recording the new corpus root `data/binance-2122/`, its
      canonical fetch command, and its pinned aggregate SHA, so a future operator
      can re-materialize it. _acceptance: backlog Recent has the reproduce
      pointer._
- [ ] **analyst** *(follow-on, NOT v0.1)* — once the corpus lands, queue the
      survey re-run over 2021–22 (the downstream feature that produces the
      firmed-up down-market finding). Backlog idea, not a v0.1 task.

## Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Re-emitting `data/binance/REVISION.toml` by accident → anchor break | low | sibling root never touches `data/binance`; AC2 asserts `3a8b96c4` byte-identical + `verify_anchors.sh` 119/119 |
| Daily-timeframe layout baked wrong before the convention is locked | resolved | ADR-0056 locks own-root-per-timeframe; v0.1 is hourly only; daily v0.2 = a new `-1d` suffixed sibling root, no reader change |
| Ragged early-2021 coverage for newer listings | low | tolerate + document per-symbol first-available-month (Q3); fetcher already logs short months and writes a short parquet |
| A committed parquet (gitignore miss) bloats the repo | low | AC3 `git status` gate; stanza mirrors the proven `binance-broaduni` pattern |
| Binance API rate-limit / transient 4xx mid-fetch | low | the fetcher's 200 ms inter-request sleep + `should_skip` idempotency make re-runs safe/resumable |

## Notes

- **No new dependency** → CLAUDE.md library-compat checklist N/A.
- **Baseline-equity-divergence e2e gate N/A** — data-fetch+layout feature, no
  equity produced (see [feature.md § Design](feature.md)). The
  recomputed-aggregate-SHA match (AC1) + re-fetch determinism (AC4) are the
  evidence-analogues.
- The fetch (T1) is an **operator/developer** runtime action — the architect
  specified the command but does **not** run it.
