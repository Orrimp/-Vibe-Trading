---
slug: 118-anchor-archaeology-2026-09-27
date: 2026-09-27
authors: spec-auditor
status: evidence-for-operator-decision
related:
  - docs/dev-notes/bug-log.md#118
  - docs/dev-notes/anchor-gate-coverage-audit-2026-09-26.md
  - docs/dev-notes/1-27-d6b-re-emission-2026-09-26.md
  - _bmad-output/planning-artifacts/architecture/decisions/0033-tcn-alpha-investigation-report-shape.md
  - _bmad-output/planning-artifacts/architecture/decisions/0038-vol-forecast-verdict-shape.md
  - _bmad-output/planning-artifacts/architecture/decisions/0040-yahoo-realdata-path.md
---

# `#118` anchor archaeology — the two structurally unreproducible rows

Read-only history reconstruction for the two `evidence/anchors.toml` rows that bug-log
`#118` flags as needing an operator **disposition decision** rather than a fix:
`sharpe-comparison-realdata` (2 rows) and `eth-yahoo-2024-1d-sma-cross` (1 row).

Nothing under `evidence/`, `crates/`, `scripts/` or `_bmad-output/` was modified to
produce this note.

## Provenance — what is verified, what is not

**VERIFIED by git history or by running a read-only gate** (every claim below is
reproducible with the commands in § 6):

- Both anchor rows' introducing commits (SHA, date, subject) and the file path they
  were introduced at (`spec/anchors.toml`, pre-2026-07-25).
- The producing code as it existed at each introducing commit, by `git show <sha>:<path>`.
- The exact commit, timestamp and diff that ended each producer's ability to emit the
  anchored body — including the full rename diff for case 1 and the emit-shape diff for
  case 2.
- The anchored report bodies on disk, their body-SHAs recomputed with
  `scripts/hash_report.py`, and a line-by-line comparison against today's emitted shape.
- The current gate state: `bash scripts/verify_anchors.sh --explain` →
  `ANCHORS PASS (119 / 119) [declared 119 · floor 119]`, with both scenarios PASS and the
  resolved file named.
- What `evidence/anchors.toml`'s schema, `scripts/verify_anchors.sh` and
  `scripts/spec_lint.py` actually do and do not support, read from source at HEAD.
- Every live and frozen citation of both scenario names, both body-SHAs, and the
  anchored reports' distinctive numbers.

**HYPOTHESIS / UNVERIFIED — labelled inline as such:**

- Whether the constant `ETH_ANCHOR_SHA = c854ff2b…` in
  `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs:156` still matches what the code
  produces today. Recorded 2026-05-28; **not re-measured here** (needs a
  `--features yahoo` build). § 2.7 gives the reasoning both ways.
- The exact wall-clock cost of any disposition option. Not measured.

**NOT ATTEMPTED (deliberately):**

- No re-run of either producer, no `--write`, no edit to `evidence/`.
- No `git checkout` — the tree stayed on HEAD throughout.

---

## 1. `sharpe-comparison-realdata` — a rename that also changed the body

### 1.1 One body, pinned twice

| # | line | namespace | sha256 |
|---|---|---|---|
| 1 | `evidence/anchors.toml:220` | `v2.6.0-alpha-investigation + noop-baseline` | `17d2e96c…` |
| 2 | `evidence/anchors.toml:570` | `v2.6.0-alpha-investigation + v5-realdata-medium-2026-05` | `17d2e96c…` |

**The two SHAs are identical.** Both rows resolve to the same single file, verified:

```
PASS  sharpe-comparison-realdata            17d2e96c1bb79c0d…
      resolved evidence/v1/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md
```

(printed twice by `verify_anchors.sh --explain`). So this is **one body pinned twice**,
not two independent claims. Row 2 was created by the v5 namespace duplication at
`c223d110` (2026-05-27, `feat(v5-latency-slippage-sim-v0.2.0-anchor-migration)`), which
copied the whole table into the canonical namespace **carrying the same SHA forward** —
i.e. a declaration that this scenario is friction-invariant, not a re-measurement.

### 1.2 When the row was added — and what produced it

| what | commit | timestamp | subject |
|---|---|---|---|
| producer + report landed | `b8a29a8a183eeb9c51c2f79abaea99d5a855ab58` | 2026-05-19 08:04:03 +0200 | `feat(v25-tcn-alpha-investigation): waves 1-4 complete (T-D-1..T-D-10) + cockpit-training-control Qs resolved` |
| anchor row 1 locked | `50567390b327cd2c24fe1c28c53d6d5c3feb0bc1` | 2026-05-19 09:30:53 +0200 | `feat: v25-tcn-alpha-investigation tester re-gate PASS (22/22) + cockpit-training-control architect lands` |
| anchor row 2 (namespace copy) | `c223d110441d8062c7e51e19450434ee0f16b229` | 2026-05-27 | `feat(v5-latency-slippage-sim-v0.2.0-anchor-migration): Wave A-D M-DEV — 68/68 anchors` |

`b8a29a8a` is an ancestor of `50567390` (checked with `git merge-base --is-ancestor`), so
the producer existed 86 minutes before the row was locked. `50567390`'s own commit
message enumerates the lock:

```
- 3 new anchors locked under v2.6.0-alpha-investigation:
    forecast-distribution-bs1-realdata    ef73cb8d…
    forecast-distribution-bs2-realdata    d7cd08e6…
    sharpe-comparison-realdata            17d2e96c…
```

**Producer at that revision** — `crates/forecast/src/bin/sharpe_comparison.rs`, verified
with `git grep -n 'sharpe-comparison' 50567390 -- crates/`:

- `:694` — front-matter literal `scenario: sharpe-comparison-realdata`
- `:955` — `let filename = format!("sharpe-comparison-realdata-{today}.md");`
- `SCENARIOS: [&str; 4]` — the four TCN `-realdata` scenarios
- `render_report(results: &[RerunResult; 4], …)`
- `out_dir` default `spec/v25-tcn-alpha-investigation/reports/`

Only one report file with that name ever existed:
`…/sharpe-comparison-realdata-20260519.md` (checked with
`git log --all --diff-filter=A -- '*sharpe-comparison-realdata-*.md'`). ADR-0033:749
names a `sharpe-comparison-realdata-20260518.md` — that is a **planning-time filename
that never materialised**, not a lost file.

### 1.3 What happened to it — renamed in place, one commit, 23 hours before it was documented as live

`git log -S 'sharpe-comparison-realdata' -- crates/forecast/src/bin/sharpe_comparison.rs`
returns **exactly two commits**: `b8a29a8a` (added the literal) and:

> `bd7313386a2794c3cd59dc18f885eeb3e97136dd` — 2026-05-22 00:24:57 +0200 —
> `test(v25a-patchtst-overlay): M-FINAL tester sweep — HANDOFF developer (K4 fix)`

That commit **overwrote the producer in place**. The renaming half of the diff:

```diff
         format!(
             "---\n\
-             slug: v25-tcn-alpha-investigation\n\
-             scenario: sharpe-comparison-realdata\n\
+             slug: v25a-patchtst-overlay\n\
+             scenario: sharpe-comparison-patchtst-bs1-realdata\n\
```

```diff
-    let filename = format!("sharpe-comparison-realdata-{today}.md");
+    let filename = format!("sharpe-comparison-patchtst-bs1-realdata-{today}.md");
```

```diff
-    #[arg(long, default_value = "spec/v25-tcn-alpha-investigation/reports/")]
+    #[arg(long, default_value = "spec/v25a-patchtst-overlay/reports/")]
```

**So the rename hypothesis in `#118` is CONFIRMED, and the survivor is
`sharpe-comparison-patchtst-bs1-realdata`.** At HEAD that path is the **default**
scenario family — `ScenarioFamily::Tcn` at `crates/forecast/src/bin/sharpe_comparison.rs:52-67`,
whose doc comment reads *"`Tcn` (default) → existing 5-scenario TCN + PatchTST run"*;
emit sites `:753` (front-matter) and `:2370` (filename).

### 1.4 …but the rename was NOT body-preserving — and that is the forcing fact

The **same** commit `bd731338` also changed what the report contains:

| change | before | after |
|---|---|---|
| scenario set | `pub const SCENARIOS: [&str; 4]` | `[&str; 5]` (+ `top10-2023-fy-patchtst-overlay-realdata`) |
| render signature | `render_report(&[RerunResult; 4], …)` | `&[RerunResult; 5]` |
| H1 title | `# Sharpe / drawdown comparison — v2.6.0-realdata scenarios` | `… — v2.6.0-realdata + v2.5a-patchtst-overlay scenarios` |
| verdict row | `\| Sharpe delta \|` | `\| Sharpe delta (TCN) \|` **+ a new** `\| Sharpe delta (PatchTST) \|` **row** |
| Honest reading | "across all four scenarios — TCN overlay is a no-op" | "across all five scenarios — overlay models are no-ops" |
| Conclusion | "TCN at v2.5 / v2.6.0-realdata … Verdict gated by M-R-HAT's F-verdict" | "TCN and PatchTST at v2.5a … PatchTST F-verdict: F4" |
| Notes ×2 | "the four -realdata reports" / "the four backtest scenarios" | "five" / "five" |

Comparing the two bodies on disk confirms the consequence. The four TCN rows in
`evidence/v1/v25a-patchtst-overlay/reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md`
are **numerically identical** to the anchored body's four rows (`13.48% / 73.73% / 6203 /
0.003098 …`), and the successor adds a fifth row plus the restructured verdict.

> **Therefore: the anchored 4-scenario body cannot be re-keyed to the surviving name.**
> `17d2e96c…` is the hash of a 4-row table with a one-line Sharpe-delta verdict. The
> surviving name's body is a 5-row table with a two-line verdict. Re-keying would pin a
> SHA that the re-keyed scenario has never produced and cannot produce — it would convert
> a *stale* claim into a *false* one.

Two further facts against re-keying:

1. **The survivor is deliberately un-anchored.** `evidence/anchors.toml:280-281`:
   *"NOTE: sharpe-comparison-patchtst-bs1-realdata is NOT anchored at v0.1.0 per decomp.md
   § Anchor gate (defer to v2.6 bake-off…)"*. Re-keying would silently reverse a recorded
   decision not to pin that name.
2. **Nothing at HEAD builds the bare name dynamically.** All four `scenario:` literals
   (`:753`, `:1130`, `:1474`, `:1858`) and all four `filename` format strings (`:2144`,
   `:2219`, `:2289`, `:2370`) are hard-coded. Verified — `#118`'s "not even via a
   `format!`" holds.

### 1.5 Is the body still cited? — the decision-relevant sweep

**LIVE artifacts (editable, still load-bearing):**

| where | what it says |
|---|---|
| `_bmad-output/planning-artifacts/trace.toml:316` | `"sharpe-comparison-realdata",  # locked by tester T-T-1 on 2026-05-19 (2-run byte-identical)` — inside `REQ-V25-TCN-ALPHA-001`.`anchors`, row `state = "shipped"` |
| ADR-0033 (`status: accepted`) `:336`, `:343` | § D2.b **defines the report shape** and names the file `sharpe-comparison-realdata-YYYYMMDD.md` |
| ADR-0033 `:672` | § D4 lists it as a new anchor **with a fallback clause** (see § 3.5) |
| ADR-0033 `:749` | names `…-20260518.md` — a file that never existed |
| ADR-0038 `:425` | § D2.b calls the vol-target report the *"Sibling of the v25-tcn-alpha-investigation `sharpe-comparison-realdata` report (ADR-0033 § D2.b)"* |
| `docs/dev-notes/retired-surface-inventory-2026-05-22.md:157` | asserts `sharpe_comparison.rs` emits `sharpe-comparison-realdata` — **already false when written**, see § 5 |

**FROZEN evidence (byte-immutable; cannot be edited without tripping the corpus rule):**

- **9 `test-final-*.md` reports** quote the gate line `PASS sharpe-comparison-realdata`:
  `evidence/v1/{cockpit-training-control, audit-tick-consumer-envelope,
  ui-rethink-phase-b-lab-run, ui-rethink-phase-c-sidebar-ia, ui-rethink-phase-d-trail,
  ui-rethink-phase-d-trail-followup, ui-rethink-phase-e-compare,
  reflection-memory-trader-wiring, v3-volatility-forecaster-noop-fix}/reports/…`.
  Four of them also quote the SHA `17d2e96c…`.
- **3 v5 `sharpe-delta-table-*.md`** reports carry the row
  `| sharpe-comparison-realdata | analysis | — | =noop |`
  (v0.2.0, v0.3.0 and v0.4.0 migration dirs).

**Are its NUMBERS uniquely quoted anywhere live? No.** The report's distinctive figures
(`0.003098`, `0.017263`) do appear in `docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md:122-124`
and one archived dev-note — **but those same values are also the passthrough-2023 row of
the successor report**, so the attribution is not exclusive. `0.001389` and `0.006447`
appear in no live document at all.

**Net:** the *name* is cited in a live ledger (trace.toml) and in two accepted ADRs, and
the *gate's PASS line* is quoted in 9 frozen reports. The *numbers* are not uniquely
sourced to this body anywhere. Every frozen citation is a citation of the gate's output,
not of the finding.

---

## 2. `eth-yahoo-2024-1d-sma-cross` — a deliberate deferral whose target was retired

### 2.1 The row

`evidence/anchors.toml:655-657`, version `lab-yahoo-realdata-v0.1.2`,
sha `e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a`.
Introduced by:

> `bd7e04b605723576634ed52387fa57dfaacecf9a` — 2026-05-28 —
> `feat(lab-yahoo-realdata-v0.1.2): M-DEV + M-DEV-UI parallel lanes complete (69 → 70 anchors)`

Three report files exist and **all three hash to the anchor** (recomputed with
`scripts/hash_report.py`) — the 3-run determinism witness from the original ship. The
resolver takes `sort | tail -1`, i.e. `…-215652-…`.

### 2.2 The `rev=` is in the hashed body, not the front-matter

```
line  6 (front-matter, stripped):  data_source: yahoo-cache:ETH-USD/1d/2024 rev=e018f876c36a
line 41 (BODY, hashed):            | Data source          | yahoo-cache:ETH-USD/1d/2024 rev=e018f876c36a              |
```

### 2.3 What emitted it

`crates/backtest/src/bin/run_yahoo_sma.rs`, at the line the migration commit's own
message names as `run_yahoo_sma.rs:259`:

```rust
let data_source = format!("yahoo-cache:{ticker}/1d/2024 rev={revision_sha:.12}");
…
backtest::report::sma::write(&sma_input, &result.state, INITIAL_CAPITAL, final_equity,
                             SEED, &data_source, elapsed_secs, &report_path, &strategy_meta);
```

### 2.4 The commit that made the body unreachable

> `e74204a949d132977c339847f6359c63d925479f` — 2026-05-28 23:47:07 +0200 —
> `feat(lab-yahoo-realdata-v0.1.3): M-DEV — durable helper + rev=migration + ETH H1 anchor`

This is `D-V0.1.3-1`. The diff replaces the hand-formatted string with a helper call:

```diff
-    let data_source = format!("yahoo-cache:{ticker}/1d/2024 rev={revision_sha:.12}");
     let strategy_meta: StrategyMeta = result.strategy_meta.clone();
 
-    backtest::report::sma::write(
+    let yahoo_ctx = backtest::report::yahoo::YahooReportContext {
+        ticker,
+        interval: "1d",
+        year: 2024,
+        revision_sha: &revision_sha,
+    };
+    backtest::report::yahoo::emit_sma_report(
+        &yahoo_ctx,
         &sma_input,
         …
-        &data_source,
```

At HEAD the replacement is the **single constructor** of Yahoo data-source strings,
`crates/backtest/src/report/yahoo.rs:60-66`:

```rust
pub fn data_source(&self) -> String {
    format!("yahoo-cache:{}/{}/{}", self.ticker, self.interval, self.year)
}
```

and the full 64-char SHA goes to front-matter instead, via `Some(ctx.revision_sha)` at
`yahoo.rs:106`.

### 2.5 The no-`rev=` contract is double-guarded — the old body is structurally, not
### accidentally, unreachable

- `crates/backtest/src/report/yahoo.rs:127-128` — unit test asserts
  `!ds.contains("rev=")`.
- `crates/backtest/tests/yahoo_report_helper_shape.rs:181-183` — e2e grep over a freshly
  emitted body: *"Report body MUST NOT contain 'rev=' (D-V0.1.3-1 body-shape contract)"*.
- `yahoo.rs:1-27` module doc: *"single point of truth … `run_yahoo_*` binaries MUST NOT
  hand-format this string"*.

So reproducing `e59a5f87…` would require **deleting a shipped contract and two tests**,
not just re-running something.

### 2.6 The row was knowingly left behind — and this is recorded, not inferred

The same commit `e74204a9` updated the BTC sibling in place and left ETH alone. Its
anchors diff:

```diff
 scenario = "btc-yahoo-2024-1d-sma-cross"
-sha256   = "8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867"
+sha256   = "076929bb63d9bec03ec83684b85ced818ee32c0b2da41140712ec1d01de6a1e0"
```

— and the ETH row untouched. Its commit message states the intent verbatim:

```
Anchor cascade locked exactly per architect:
  Row 69 btc-yahoo-2024-1d-sma-cross — in-place SHA update …
  Row 70 eth-yahoo-2024-1d-sma-cross — UNCHANGED (`e59a5f87...`)
  Row 71 NEW eth-2024-h1-sma-cross `bd4001e4...`
…
  - ETH daily row 70 byte-identical (`e59a5f87...`; not re-emitted at
    v0.1.3 per analyst defer)
```

### 2.7 The successor SHA already exists in the repo

`crates/backtest/tests/run_yahoo_sma_ticker_flag.rs:149-156`:

```rust
/// ETH body SHA under the v0.1.3 emit shape (no `rev=` in body; `revision_sha:` in frontmatter).
///
/// The v0.1.3 body→frontmatter migration changed this SHA from the v0.1.2
/// anchor (`e59a5f87...`).  Row 70 in evidence/anchors.toml retains the OLD SHA
/// (`e59a5f87...`) because the on-disk archived file is byte-immutable
/// (ADR-0038 § D6.b); bulk Yahoo ticker re-emit deferred to v0.1.4 BNB ship
/// (D-V0.1.3-6).  This constant tracks the CURRENT live-emission shape…
const ETH_ANCHOR_SHA: &str = "c854ff2b2a97a876deb978a9db1cd0bf132de2ce5649f16a06d8dfa6cb475da2";
```

The test `eth_ticker_sha_matches_anchor_70` (`:228`) asserts live emission equals
`c854ff2b…`, and its failure message spells out the split (`:278-281`).

**So the disposition for this row was already decided in 2026-05-28** — re-emit at
v0.1.4 — and the target SHA was measured and committed. The coverage audit's remark that
`c854ff2b…` *"appears nowhere else in the repo"* is factually right but reads as
suspicious; it is simply a forward-looking constant awaiting a ship.

> **HYPOTHESIS, not verified here:** that `c854ff2b…` is still what the code produces.
> Arguing for: `rev=` now lives in front-matter, which `hash_report.py` strips, so
> REVISION.toml churn no longer moves the body. Arguing against: the BTC test's comment
> at `:219-222` still warns *"If REVISION.toml changed (new ticker fetched), re-emit and
> update BTC_ANCHOR_SHA"* — reasoning that predates the migration and may be stale in
> either direction. **Measuring this is one `cargo test -p backtest --features yahoo
> eth_ticker_sha_matches_anchor_70` away** and would settle option (d) below.

### 2.8 The deferral target was retired — the debt silently became permanent

`_bmad-output/planning-artifacts/trace.toml:2270`:

> `state = "retired"  # RETIRED 2026-06-16 (operator decision): non-load-bearing
> completeness item … Research concluded 2026-06-08 (ship passive) → 9 more Yahoo
> backtests change no conclusion; was blocked on a manual operator Yahoo fetch. Owned-debt
> from the v0.1.3 deck, honestly retired in wind-down rather than forced.`

The story is `epics.md:1042-1052` (Story 2.46,
`lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit`), and `trace.toml:2269` still reads
`anchors = []  # tester M-FINAL fills with row 70 (in-place update …)`.

**That is the honest shape of case 2:** the row was parked on a named future ship; the
ship was retired for good reasons that did not mention the parked row; nobody reconciled
the two. The retirement note says "9 more Yahoo backtests change no conclusion" — true of
the 9 *new* tickers, silent about the 1 *in-place* row the same story owned.

### 2.9 Unlike case 1, the producer still runs

`run_yahoo_sma --ticker ETH-USD` works at HEAD (ETH-USD is in `ALLOWED_YAHOO_TICKERS`),
and the local cache `data/yahoo/ETH-USD/1d/2024/` is present. Note the corpus is
**gitignored** — `git ls-files data/yahoo` returns only `REVISION.toml` — so this is
reproducible on the operator's machine, not in CI.

### 2.10 Is the body still cited?

**LIVE:**

| where | what |
|---|---|
| `trace.toml:2025` | `"eth-yahoo-2024-1d-sma-cross"` in `REQ-LAB-YAHOO-REALDATA-V0-1-2-001`.`anchors`, `state = "shipped"` |
| `trace.toml:2150` | same name in `REQ-LAB-YAHOO-REALDATA-V0-1-3-001`.`anchors`, `state = "shipped"` |
| `trace.toml:2002`, `:2020`, `:2146` | comments naming the row and the full SHA |
| ADR-0040 `:508`, `:528`, `:550` | the scenario-naming rule and the "deferred to v0.1.4" note |
| `CHANGELOG.md:198` | feature-level line only; does not name the scenario |
| `crates/backtest/…` | `run_yahoo_sma.rs:32`, `:131`, `:338`; `run_yahoo_sma_ticker_flag.rs:151-152`, `:269`, `:280`, `:343` |

**The numbers ARE quoted, and one of them is load-bearing** —
`evidence/anchors.toml` itself:

```
:650  # Final equity: $102,760.76 (+2.76%).  Determinism: body SHA identical on 3 independent runs.
:664  # H1 PASS direct: Yahoo ETH daily (+2.76%) vs Binance ETH hourly (+9.54%) delta = 6.78% < 30%.
```

Line 664 is a **hypothesis-discharge claim** (`H1`) resting on this body's `+2.76%`.
Outside `anchors.toml` those figures appear only in `docs/archive/` (two pre-BMAD
dev-notes and `backlog-recent-2026-05.md:50`, `:105-106`).

**Net, and this is the sharpest contrast with case 1:** case 2's body supplies a number
that a still-standing verification claim depends on, and the successor number is already
known. Case 1's body supplies no number anyone still cites.

---

## 3. What the mechanisms actually allow — checked at HEAD, not assumed

### 3.1 `anchors.toml` schema: no "historical" field exists

`grep -n 'historical\|reproducible\|no-reproduction\|reproduction_claim'` over
`evidence/anchors.toml` and `scripts/*.{sh,py}` finds **nothing** relevant (the one
`historical` hit is the prose word in the `noop-baseline` header comment). There is no
`reproducible = false`, no skip list, no `historical = true`.

### 3.2 `verify_anchors.sh` ignores unknown keys — but requires every block to carry a SHA

The script was hardened on 2026-09-27 (bug-log `#127`). Relevant behaviour at HEAD:

- It parses **only** three regexes: `scenario`, `version`, `sha256`. Any extra key is
  invisible to it.
- `declared="$(grep -c '^\[\[anchors\]\]' "$anchors")"` and then:
  `if [[ "$total" -ne "$declared" ]]` → **FAIL** with
  *"N row(s) were never compared"*.
  **Consequence: you cannot mark a row historical by removing its `sha256` line.** That
  route is closed by design.
- `floor="${MIN_EXPECTED_ANCHORS:-119}"`, and `total < floor` → **FAIL** with
  *"the corpus shrank … export MIN_EXPECTED_ANCHORS to ratify a deliberate change"*.
  **This is the one existing affordance for a deliberate row removal** — an explicit,
  documented ratification hook, not a silent one.
- It hashes the file on disk. For an unreproducible scenario that is still a real check —
  it is a **byte-immutability** check on frozen evidence. It is not worthless; it is
  mislabelled. The gate's claim ("this scenario reproduces") is what is false, not its
  measurement.

### 3.3 `spec_lint.py` tolerates extra keys, and enforces trace ↔ anchors both ways

- `check_anchors` (`:532-550`, the test at `:541`) computes
  `sorted({"scenario","version","sha256"} - a.keys())`
  — it checks only for **missing** required keys. **An added key would not break it.**
- `check_trace` (`:601-606`) — every anchor cited by a trace row must exist in
  `anchors.toml`, else `trace-broken-path`.
- `check_trace` (`:608-613`) — every scenario in `anchors.toml` must be cited by some
  trace row, else `unreferenced-anchor`.

**Consequence: any re-key or removal must edit `trace.toml` in the same pass**, or the
lint fires. For case 1 that is `trace.toml:316`; for case 2 it is `:2025` **and** `:2150`.

**Consequence for a relabel:** a new key such as `reproduction = "none"` can be added
today without breaking either reader — but **neither reader would act on it**. It would
be documentation sitting in a machine file, and the gate would keep printing `PASS` on a
reproduction claim it is not making. Making the label mean something requires a
`verify_anchors.sh` change (a distinct PASS token, e.g. `FROZEN`, counted separately).

### 3.4 ADR-0038 § D6.b forbids row deletion, and names where an amendment goes

D6.b's **"Not in scope of this protocol"** paragraph:

> silent mutations (forbidden by D6 spirit), namespace bifurcation …, **row deletion
> (forbidden — historical evidence stays linked even after re-emission via the dev-note +
> feature.md cross-references)**

and its closing line:

> If the protocol itself needs revision …, the revision lands as **D6.c** (additive
> amendment subsection, not in-place mutation of D6.b).

**So (i) deleting a row is currently forbidden by the ADR corpus, and (ii) `D6.c` is the
already-named slot for whatever new protocol the operator rules for.** No `D6.c` exists
yet — verified, `grep -n 'D6\.c'` finds only that forward reference.

And D6.b cannot be invoked as-is: step 2 requires *"cite the bug site with `file:line`"*
and step 3 requires a *would-have-caught test run RED against pre-fix code*. Neither case
has a bug. Case 1 has a rename; case 2 has an intended contract change plus an
un-discharged defer.

### 3.5 There is one ADR-blessed precedent for an un-anchored report — and it is about
### this very scenario

ADR-0033 § D4 (`:670-674`):

> - `sharpe-comparison-realdata` (subject to determinism check; **falls through to "ship
>   un-anchored with `## Not anchorable` body section"** if the body fails the two-run
>   byte-identity gate)

The corpus therefore already contemplates *"this report ships without an anchor"* for this
exact scenario. Two caveats: the trigger written there is byte-identity failure, not
producer loss; and literally adopting it now would mean **editing a byte-immutable report
body** to add a `## Not anchorable` section — which the anchor rule blocks. It is a
precedent for the *shape* of the answer, not a ready mechanism.

---

## 4. Disposition options — neutral, with cost and honesty consequence

No option is free; the honesty column is the one that matters, since this decides what the
project claims is reproducible.

### (a) Relabel the rows as historical, carrying no reproduction claim

**Needs:** a `verify_anchors.sh` change (new key parsed; a distinct output token so a
frozen row does not print `PASS` next to a reproducing one; a separate count in the
summary so `119 / 119` does not silently mean "117 reproduce + 2 are frozen"). Plus an
ADR amendment — `D6.c` is the named slot. `anchors.toml` and `spec_lint.py` need no schema
work (§ 3.3).

**Cost:** smallest code change of the three that actually alter the gate; one ADR + its
Registry row (atomic per AD-18); the summary-line change touches a number quoted in many
frozen reports as `(N / N)`, so the new shape needs a deliberate format decision.

**Honesty consequence:** best available. The rows keep pointing at real evidence, the
frozen citation chain in 9 test reports stays intact, and the gate stops asserting
something it cannot test. It converts a false claim into a true, narrower one
(byte-immutability). It does *not* restore reproduction — the label is the admission that
reproduction is gone.

**Note:** this is the option the two 2026-09-26 audits and bug-log `#118` already lean
toward, and `docs/dev-notes/bug-log.md:2749` shows it was offered as choice **(c)** for a
different row and **not** chosen there — the operator ruled **(a) re-run with declared
flags** in that case. That precedent does not transfer: that row *had* a constructible
invocation.

### (b) Trace the rename and re-key the row to the surviving scenario name

**The evidence refutes this for case 1.** § 1.4: the surviving name's body has five table
rows and a two-line verdict; `17d2e96c…` is a four-row, one-line body. Re-keying makes
the gate assert a SHA the target name has never produced — it would go red immediately if
anyone ever ran it, or stay green only because the resolver keeps finding the *old* file
under the *new* name's pin. It also silently reverses the recorded decision at
`anchors.toml:280-281` not to anchor `sharpe-comparison-patchtst-bs1-realdata`.

**For case 2 there is no rename at all** — the scenario name never changed; only the body
shape did. So (b) is not applicable to case 2 in any form.

**Cost if pursued anyway:** low mechanically (2 lines in `anchors.toml` + `trace.toml:316`).
**Honesty consequence:** worst of the options. It replaces a stale claim with a false one
and destroys the ability to detect the substitution later.

### (c) Leave as-is and document the gap

**Needs:** nothing mechanical. The two audits and `#118` already exist; this note is the
archaeology.

**Cost:** zero today. Recurring: every future reader of `ANCHORS PASS (119 / 119)` is told
119 scenarios reproduce, and two do not — so the count keeps overstating coverage, and the
next audit re-derives this note.

**Honesty consequence:** the gap is documented in `docs/`, but the **machine artifact
keeps asserting the false thing**. This is precisely the shape the repo's own house rule
names — *"a gate that cannot fail is indistinguishable from a gate that passes"* — with
the documentation living somewhere the gate's reader never goes. It is defensible as a
holding position; it is not defensible as an answer.

### (d) For case 2 only — discharge the 2026-05-28 defer: re-emit and update the row

**Not in the original three options, but the evidence surfaces it as available.** Unlike
case 1, case 2's producer runs at HEAD (§ 2.9), the intended new SHA was already measured
and committed (`c854ff2b…`, § 2.7), the in-place-update precedent exists in the same
commit (BTC row 69, § 2.6), and the plan was ratified at the time — it only lost its
vehicle when v0.1.4 was retired (§ 2.8).

**Needs:** confirm `c854ff2b…` still reproduces (one `cargo test -p backtest --features
yahoo` run — § 2.7 flags this as the open measurement); land a fresh report under
`evidence/…/reports/`; update `anchors.toml:657` in place under the preserved namespace;
and decide whether `anchors.toml:650/:664` — the `+2.76%` figures and the `H1` discharge
claim — still hold under the new body. The numbers themselves should not move (the change
is a body *string*, not the backtest), but that must be measured, not assumed.

**Cost:** one run + one anchor edit + a `trace.toml`/CHANGELOG reconciliation for the
retired-story remnant at `trace.toml:2269`. Needs a ruling on which protocol authorises
the in-place edit, since D6.b's preconditions are not met (§ 3.4) — most likely the same
`D6.c` amendment option (a) needs.

**Honesty consequence:** the only option that ends with the row making a **true
reproduction claim**. It also resolves the `#118`-adjacent finding that
`run_yahoo_sma_ticker_flag.rs` currently ships a test whose own doc comment confesses the
anchor is stale.

### Does the evidence force a disposition?

**Partly — and differently for the two cases.** Stated plainly:

- **Case 1 (`sharpe-comparison-realdata`): option (b) is FORCED OUT.** The forcing fact is
  § 1.4 — the rename commit `bd731338` changed `SCENARIOS: [&str; 4]` to `[&str; 5]` and
  restructured the verdict table in the same diff, and the two bodies on disk confirm the
  surviving name emits five rows where the anchored body has four. Re-keying is therefore
  not a bookkeeping move; it is asserting a false SHA. Among the remainder the evidence
  does **not** force (a) over (c) — that is a judgement about whether a machine gate may
  keep a claim its documentation retracts.
- **Case 2 (`eth-yahoo-2024-1d-sma-cross`): nothing is forced, but the option set is
  larger than `#118` states.** `#118` says *"re-emitting is not available"*; the evidence
  says re-emitting is exactly what was planned, measured and scheduled, and the only thing
  that went missing was the ship that would have carried it. Treating case 2 as the same
  kind of problem as case 1 is the one reading the history does not support.

---

## 5. Two corrections to `#118`'s own text

Both are small, and both matter because `#118` is the input to this decision.

1. **`#118` cites `docs/dev-notes/retired-surface-inventory-2026-05-22.md:157` as
   evidence that "the surface is retired".** That line does not say retired — it is an
   inventory row asserting that `sharpe_comparison.rs` *emits*
   `sharpe-comparison-realdata` (alongside the two vol-target names). That assertion was
   **already false when the doc landed**: the rename `bd731338` is timestamped
   2026-05-22 00:24:57 and the inventory landed in `8dcd72c3` at 2026-05-22 23:41:47 —
   **23 hours later**. The doc's own header also states *"Not a deletion pass — the
   retirement contract is explicit: code stays, anchors locked, no deletion."* So the
   cited line is a stale inventory row, not a retirement record.

2. **`#118` groups `eth-yahoo-2024-1d-sma-cross` with case 1 as *"Related, same shape and
   also unreachable"*.** The anchored *body* is unreachable in both cases, but the shapes
   differ in the way that decides the disposition: case 1's producer cannot emit the name
   at all, while case 2's producer runs today, the replacement SHA is committed in the
   repo, and the deferral was explicit. "Same shape" is true only at the level of "the
   gate passes on a body nothing can regenerate."

Neither correction changes `#118`'s core finding, which holds: two rows report `PASS` for
reproduction claims that cannot be tested.

---

## 6. Reproducing this archaeology

Every command below is read-only.

```bash
# Case 1 — anchor row provenance (the file was spec/anchors.toml before 2026-07-25)
git log --follow --diff-filter=A --format='%H %ad %s' --date=short -- evidence/anchors.toml
git log --follow -S 'sharpe-comparison-realdata' --format='%H %ad %s' --date=short -- evidence/anchors.toml
git show 50567390 -- spec/anchors.toml | grep -B8 -A4 sharpe-comparison

# Case 1 — the producer then, and the rename
git grep -n 'sharpe-comparison' 50567390 -- crates/
git log -S 'sharpe-comparison-realdata' --format='%H %ad %s' --date=iso \
    -- crates/forecast/src/bin/sharpe_comparison.rs
git show bd731338 -- crates/forecast/src/bin/sharpe_comparison.rs

# Case 1 — body comparison (4 rows vs 5)
diff <(sed -n '15,50p' evidence/v1/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md) \
     <(sed -n '16,52p' evidence/v1/v25a-patchtst-overlay/reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md)

# Case 2 — the row, the rev= migration, and the deliberate defer
git log --follow -S 'eth-yahoo-2024-1d-sma-cross' --format='%H %ad %s' --date=short -- evidence/anchors.toml
git show e74204a9 -- crates/backtest/src/bin/run_yahoo_sma.rs
git show e74204a9 -- spec/anchors.toml
git log -1 --format=%B e74204a9 | grep -A4 'Anchor cascade'

# Both — the anchored bodies and their hashes
python3 scripts/hash_report.py evidence/v1/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md
for f in evidence/v1/lab-yahoo-realdata-v0.1.2-*/reports/*eth-yahoo*; do
    python3 scripts/hash_report.py "$f"; done

# Both — current gate state and which file satisfies each row
bash scripts/verify_anchors.sh --explain | grep -A1 -e sharpe-comparison-realdata -e eth-yahoo-2024

# Mechanism checks
sed -n '265,295p' scripts/verify_anchors.sh          # declared / floor non-vacuity (#127)
sed -n '529,615p' scripts/spec_lint.py               # check_anchors + trace <-> anchors both ways
sed -n '608,626p' _bmad-output/planning-artifacts/architecture/decisions/0038-vol-forecast-verdict-shape.md
sed -n '665,676p' _bmad-output/planning-artifacts/architecture/decisions/0033-tcn-alpha-investigation-report-shape.md

# The one open measurement (case 2, needs a yahoo-feature build — NOT run for this note)
cargo test -p backtest --features yahoo eth_ticker_sha_matches_anchor_70 -- --nocapture
```

The `verify_anchors.sh --explain` sweep took under a minute on this machine; nothing here
needs a long-running-job watch block.
