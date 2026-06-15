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

- [x] **developer/operator** — T1: run the fetch (read-only historical HTTP).
      **Exact command** (copy-paste verbatim):
      ```bash
      cargo run -p data --bin fetch_binance_klines -- \
        --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT \
        --start 2021-01-01 --end 2022-12-31 --interval 1h \
        --out data/binance-2122 --emit-revision-manifest
      ```
      _file: data/binance-2122/ (240 parquets) + data/binance-2122/REVISION.toml_
      _test: `find data/binance-2122 -name "*.parquet" | wc -l` → 240_
      _output: `[REVISION] data/binance-2122/REVISION.toml written — aggregate SHA: 4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62`_

- [x] **developer/operator** — T2: aggregate SHA + per-symbol bar totals captured.
      No ragged coverage: all 10 symbols have 744 bars for 2021-01.
      Aggregate SHA: `4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62`.
      _file: spec/binance-corpus-expansion/reports/fetch-2026-06-15-binance-2122.md_
      _test: grep `\[REVISION\]` /tmp/binance-2122-fetch.log_
      _output: `[REVISION] data/binance-2122/REVISION.toml written — aggregate SHA: 4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62`_

- [x] **developer** — T3: manifest internal consistency verified by the always-on
      `manifest_internal_consistency` test (T6 below covers this exactly —
      `compute_aggregate_sha` from `[files]` map == claimed `[revision].sha256`).
      _file: crates/data/tests/binance_2122_revision_consistency.rs_
      _test: `cargo test -p data --test binance_2122_revision_consistency manifest_internal_consistency`_
      _output: `test manifest_internal_consistency ... ok`_

## M2 — Repo plumbing (developer)

- [x] **developer** — T4: add the `.gitignore` stanza, placed **adjacent to the
      existing `binance-*` stanzas** (after the `binance-basis` block).
      _file: .gitignore (after binance-basis block)_
      _test: `git ls-files --others --exclude-standard data/binance-2122/`_
      _output: `data/binance-2122/REVISION.toml` (only — no parquets listed)_

- [x] **developer** — T5: `git ls-files --others --exclude-standard data/binance-2122/`
      shows ONLY `data/binance-2122/REVISION.toml`; no `*.parquet` present.
      _file: .gitignore_
      _test: `git ls-files --others --exclude-standard data/binance-2122/`_
      _output: `data/binance-2122/REVISION.toml`_

## M3 — Manifest-consistency test + smoke consumer (developer)

- [x] **developer** — T6: always-on manifest-internal-consistency test added.
      Parses `data/binance-2122/REVISION.toml`, recomputes aggregate via
      `data::revision::compute_aggregate_sha`, asserts == `[revision].sha256`.
      Also asserts `files_map.len() == 240`. CI-safe (no parquet needed).
      _file: crates/data/tests/binance_2122_revision_consistency.rs (manifest_internal_consistency)_
      _test: `cargo test -p data --test binance_2122_revision_consistency manifest_internal_consistency`_
      _output: `test manifest_internal_consistency ... ok`_

- [x] **developer** — T7: `#[ignore]` smoke consumer added. SKIP-guards on
      `data/binance-2122/BTCUSDT/2022/01.parquet` absence (AC5). When present:
      loads all BTCUSDT bars via `ReplayFeed::new(root, true)`, asserts ≥ 100
      bars, asserts `close.get()` != zero (AC6 — Decimal not f64). Read 17 507
      bars on first run.
      _file: crates/data/tests/binance_2122_revision_consistency.rs (smoke_consumer_btcusdt_2022)_
      _test: `cargo test -p data --test binance_2122_revision_consistency smoke_consumer_btcusdt_2022 -- --ignored --nocapture`_
      _output: `OK binance-2122 smoke: BTCUSDT read 17507 bars from ".../data/binance-2122"\ntest smoke_consumer_btcusdt_2022 ... ok`_

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

- [x] **developer** — T8: reproduce pointer added to `spec/backlog.md` Recent
      (2026-06-15 cohort) with corpus root, fetch command, and pinned aggregate
      SHA `4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62`.
      _file: spec/backlog.md (### 2026-06-15 cohort section, before 2026-06-12 cohort)_
      _test: `grep -A5 "binance-2122" spec/backlog.md | head -10`_
      _output: shows canonical fetch command + aggregate SHA_
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
