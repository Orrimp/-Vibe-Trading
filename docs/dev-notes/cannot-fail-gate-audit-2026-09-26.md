# Cannot-fail gate audit — 2026-09-26

> **A gate that cannot fail is indistinguishable from a gate that passes.**

Follow-on to bug-log `#102`, `#104`, `#112`, `#113`, `#114`. Scope: `crates/*/tests/`,
`crates/*/src/` test modules, `scripts/`, `.github/workflows/`.

**Provenance.** Produced by a read-only audit agent, then spot-verified by the orchestrator before
being written. **Independently re-verified here:** finding 2 (`check_determinism_anchors.py` reports
`OK — 14 literal(s) match … 0 skipped` while `determinism.rs` has **25** `const ANCHOR` sites — both
numbers re-derived); finding 1's load-bearing precondition (`evidence/backtest-real-binance-data/`
does **not** exist, while `evidence/v1/backtest-real-binance-data` does); finding 10
(`verify_anchors.sh:243` prints `($total / $total)` and the script contains **zero** occurrences of
`passed`). **Not independently re-verified:** the remaining eleven CONFIRMED findings and all three
SUSPECTED ones. Their `file:line` citations are the agent's reading. Each is actionable but should be
re-checked at the point of fixing, not taken from this note.

Ranked by consequence: anchors and the FROZEN robustness gate first, then money-adjacent and secrets,
then instrumentation, then cosmetic.

## Counts

**CONFIRMED 14 · SUSPECTED 3 · counter-examples worth copying 10.**

## The three highest-consequence findings

### 1. Two neutrality gates hash a committed file and compare it to a constant copied from that file

`crates/forecast/tests/patchtst_overlay_neutrality.rs:34,150,212` and
`crates/trader/tests/llm_forecaster_neutrality.rs:36,141,224`.

The run writes to `evidence/backtest-real-binance-data/reports/`
(`crates/backtest/src/main.rs:2569-2573` + `:2549-2552`) — **a directory that does not exist in this
tree** (verified). Both tests search only `evidence/v1/…`, so `find_latest_report` resolves the newest
*committed* body, whose body-SHA is bit-identical to `EXPECTED_SHA`. **The run's own output is never
examined.** Bug-log `#112` measured this scenario producing `b6d88fa6…`; the drift these gates exist
to catch is live and both are green.

Compounding: `EXPECTED_SHA` is the `v2.6.0-realdata + noop-baseline` row, three generations behind
the live `v5-sqrt-impact-2026-05` row; and with `.current_dir(&ws_root)` and no `--reports-dir`, a run
plants a body under `evidence/` — `#113`'s landmine in two more places.

**Fix:** delete both bespoke harnesses and add the two scenarios to
`determinism.rs::assert_reproduces_canonical_anchor`, per `#112` requirement (0) — extend the
apparatus that is already right.

### 2. The primary anchor-drift linter is blind to 11 of 25 sites and reports "0 skipped"

`scripts/check_determinism_anchors.py` — ADR-0045 § D7.1's gate. Re-derived: `determinism.rs` has
**25** `const ANCHOR` sites; the script prints `OK — 14 literal(s) match (8 canonical, 6 synthetic;
0 skipped: cfg-gated)`.

A site only enters `sites` if `scenario_body_hex("…")` appears within 7 lines (`:150`). Sites whose
runner is `scenario_body_hex_candle(` (the m3 pair) or `assert_reproduces_canonical_anchor(` /
`assert_reproduces_or_report_unmeasured(` (**the nine R-REPRO gates written today**) are dropped at
`:196-204` — not counted, not warned, not reported as skipped. 25 − 11 = 14, matching the output
exactly.

The dropped sites are precisely the ones `#112`/`#111` proved stale. **The meta-gate cannot see the
gates that matter, and its "0 skipped" says the opposite.** Secondary: total regex failure is a WARN
and `exit 0` (`:519-521`); and unlike its five sibling gates it has no `--self-test`.

**Fix:** an unresolvable `const ANCHOR` site must `return 1`; print `n_sites_found` against a declared
floor; add `--self-test` covering all three runner spellings.

### 3. `crates/forecast/src/tcn.rs:496-497` — a `#114` clone, with the same wrong diagnostic

`PathBuf::from("crates/forecast/checkpoints/anchors")` under the comment *"In tests and binaries, the
CWD is the workspace root."* Cargo runs integration tests with CWD = the **package** root — the repo
documents this in four places including `bug-log.md:192`. So under `cargo test -p forecast` the path
resolves to `crates/forecast/crates/forecast/checkpoints/anchors`, `load_anchor` returns
`CheckpointNotFound`, and `crates/forecast/tests/anchors_load.rs` skips on every machine printing
*"run `git lfs pull`"* — while the 1.67 MB checkpoint sits resolved on disk.

Identical to `#114` including the misattributed blame. **Fix:** resolve from
`env!("CARGO_MANIFEST_DIR")` in both TCN and PatchTST, and print the resolved path on success.

## The remaining CONFIRMED findings

| # | file:line | why it cannot fail | fix |
|---|---|---|---|
| 4 | `compute_robustness_distribution_matches_flag.rs:59-66,76-176` | Asserts a function against its own delegate: `compute_robustness_flag` **is** `compute_robustness_distribution` + `From`, and the test hand-copies that `From`. Every assertion is `g(h(x)) == g′(h(x))` with `g ≡ g′`. The pre-refactor implementation it claims bit-identity against appears nowhere in the file. Companion loose thresholds at `:218-238` (`p5 ≤ p50 ≤ p95`, probabilities in `[0,1]`, a drawdown `≥ 0`) are all guaranteed by construction | Pin three golden `(equity, paths, seed) → (verdict, p5/p50/p95, prob_loss)` tuples. Only a pinned value can witness a change to a byte-frozen gate |
| 5 | `fdr_annex_identity.rs:62` | Runs with `RobustnessMode::Skip`, and `Skip → None` (`bakeoff/mod.rs:1368`, `:1455`). The frozen gate whose invariance is being proved **never executes**. The file's own RED-on-revert clause names seeding an RNG from the row count — the bootstrap RNG is exactly what `Skip` prevents | Run one matrix arm under `Bootstrap { paths: 150, seed }` and assert the per-candidate flag vector, not only crown + ranking |
| 6 | `scripts/orch_determinism_check.sh:64,77,84-86` | `shas_1="$(shasum -a 256 $glob 2>&1 \| sort)"` — an empty glob makes `shasum` error, `2>&1` folds the message into the value, and both runs capture identical text. Prints `DETERMINISM PASS (byte-identical across two runs)` having hashed **zero files** | Drop `2>&1`; expand into an array and `exit 2` when empty; print the file count in the PASS line |
| 7 | `scripts/check_no_clocks_in_ui_tests.sh:36,58-61,92` | One watchlist entry (`crates/ui/src/screens/charts.rs`) was deleted in `f3cb7f92`; a missing entry `continue`s with no counter, and the PASS line prints the **array length** ("8 files") having scanned 7 | Count files actually scanned and print that; a missing watchlist entry is a FAIL — renamed means re-point, not drop |
| 8 | `sigma_train_not_in_safetensors.rs:68,117,120-125` (+ patchtst sibling) | Counts what it verified and never asserts on the count: `if tested > 0 { println!(…) }`. Both checkpoint filenames are hardcoded full SHAs; a re-train renames them, the loop `continue`s twice, `tested == 0`, and the test passes printing **nothing at all** | `assert!(tested > 0)` and `assert_eq!(tested, checkpoints.len())` |
| 9 | `.github/workflows/ci.yml:236-241` | The skip-visibility instrument cannot distinguish "no skips" from "did not build": `cargo test … \|\| true` then `grep -c "[skip]" \|\| true`; a compile failure yields `N=0` and the echo then asserts *"0 + green means the guards ran for real"* — `#114`'s signature. Also greps two binaries while ~64 soft-skip sites exist across 26 files | Assert the cargo invocation succeeded before interpreting `N`; make `[skip]` repo-wide and grep the whole log |
| 10 | `scripts/verify_anchors.sh:243` | `echo "ANCHORS PASS ($total / $total)"` — same variable twice, zero occurrences of `passed` in the script (**verified**). The gate *does* fail on a mismatch, so it is not unfailable — but **the number quoted repo-wide as coverage evidence carries no coverage information**. Related: `total` only increments on a 64-lowercase-hex SHA (`:88-90`), so a differently-spelled row is silently neither counted nor checked | Keep a separate `passed` and print `passed / total`; `exit 2` if `total` is 0 |
| 11 | `scripts/check_no_secrets_in_llm_artifacts.sh` | `#104` residue: `strings … 2>/dev/null` still swallows a missing `strings`, every `find` is `2>/dev/null`, scan functions `return 0` on an absent file, and there is **no file counter** — so `V9 PASS` prints identically for zero files. Ranked low only because `#104` moved the load-bearing gate into `crates/llm/tests/no_secrets_in_artifacts_test.rs`, which does assert `scanned > 0` | Add `files_scanned`; `exit 2` at zero; `command -v strings \|\| exit 2` |
| 12 | `scripts/precheck.sh:21` | `grep -RhE … 2>/dev/null \|\| true` — an unreadable tree or missing `grep` reads as "no clash", and a spelling drift in the regex silently disables it. No `--self-test`, unlike five siblings. Low consequence: the guarded failure surfaces as a compile error anyway | Count `^name = ` matches, fail at zero, add `--self-test` |
| 13 | `dvol_bakeoff_path_gate.rs:697`, `p2_verdict_rerun.rs:147-174`, `dynamic_cache_anchor_safety.rs:38-49` | Proxy-not-condition guards. `p2_verdict_rerun` maps `Ok(empty)` and `Err(_)` to the same `None` — the any-`Err`→skip shape that `lab_binance_divergence.rs:145-150` itself calls out as "vacuous on every machine". `snapshot_dir` returns an empty map for a missing dir, so an anchor-safety diff between two empty snapshots is trivially equal | Split probe-absent (skip) from loader-error (panic); assert the snapshot is non-empty before diffing |
| 14 | `smoke_train.rs:296-318` | Two stacked soft skips fold three outcomes into one green — "no corpus", "zero training windows", "trained fine" — because `run_one_epoch_smoke` returns `NAN` on absent data. And `assert!(val_loss.is_finite() \|\| val_loss.is_nan())` excludes only ±infinity: a threshold that cannot trip | Make "0 windows loaded with the corpus present" a FAIL; drop the `\|\| is_nan()` escape |

## SUSPECTED — needs a run to settle

| # | file:line | why it might not fail | the run that settles it |
|---|---|---|---|
| S1 | `run_yahoo_sma_ticker_flag.rs:70-85`, `yahoo_report_helper_shape.rs:56-69` | `binary_path()` reads `std::env::var("CARGO_BIN_EXE_run_yahoo_sma")` at **runtime**; Cargo sets that variable for **compilation**, which is why `reproducibility_sample_figure.rs:57` uses `env!(…)`. If the runtime lookup always misses, both files fall back to an absent `target/debug/run_yahoo_sma` and both anchor comparisons skip silently | `cargo test -p backtest --features yahoo --test run_yahoo_sma_ticker_flag -- --nocapture`, look for `SKIP: run_yahoo_sma binary not found`. Fix either way: switch to `env!(…)` |
| S2 | `reports_populated_curve_render.rs` (8 sites) | Eight soft-skip `return`s in AD-10 reports-render gates keyed on a hardcoded stem and on `discover_reports()` yielding entries; `discover_reports()` returns `Vec::new()` on an unreadable `evidence/`, after which all eight pass silently, and none emits the `[skip]` token the CI counter looks for | `cargo test -p ui --test reports_populated_curve_render -- --nocapture`. The absent skip accounting is confirmed regardless |
| S3 | `embedded_font_contract.rs:446-459,490,520` | Three glyph-coverage scans assert only `offenders.is_empty()` with no assertion that the walk produced any file — striking, because the same test contains the repo's best non-vacuity assertion six lines earlier | Print `rust_sources(&src).count()` and assert a floor |

## Counter-examples — the pattern to copy

Every fix above is already implemented, correctly, somewhere in this repo.

1. **`crates/llm/tests/no_secrets_in_artifacts_test.rs:216,224-231,245-252`** — the gold standard, and
   unsurprisingly the `#104` fix. `scan_tree` **returns** the number of files read, documented as
   *"so callers can prove the walk was not empty"*; asserts `scanned > 0` with *"an empty-set result,
   not a clean one"*; and plants a decoy key in a throwaway tree and asserts the walk flags it, with
   the message *"the gate cannot fail"*. Non-vacuity **and** red-proof in one test.
2. **`crates/ui/tests/embedded_font_contract.rs:440-444`** — asserts the *instrument* read something
   and names both hypotheses so a failure cannot be misattributed: *"the cmap parse is wrong, not the
   UI"*. `#114`'s lesson in one assertion.
3. **`crates/backtest/tests/dvol_bakeoff_path_gate.rs:660-723`** —
   `corpus_gated_tests_are_declared_and_counted`, corpus-*independent* so it always runs. Asserts the
   `#[ignore]` count equals a declared inventory, asserts each declared name exists, prints the
   re-run command, and warns *louder* when the corpus IS present. Its docstring states the principle:
   *"It fails when the skips become invisible."* This is the answer to finding 9.
4. **`crates/ui/tests/lab_binance_divergence.rs:125-176`** — the probe/error split: skip only when the
   probe is genuinely absent; when present and the loader errors, **panic**. Plus
   `pin_cwd_to_workspace_root()`, which pins the `#112` condition instead of hoping.
5. **`crates/forecast/src/features.rs:1263-1278`** — checks the **condition** (two named parquets) not
   a **proxy** (the tracked directory), with the reasoning inline.
6. **`crates/ui/tests/baseline_error_state.rs:132-150`** — refuses a skip on principle: *"a silently-
   skipped gate is indistinguishable from a passing one in every report anybody reads"*.
7. **`crates/backtest/tests/reproducibility_sample_figure.rs`** — the reproduction gate done right,
   and the exact inverse of finding 1. Its `## What this does NOT prove` section is a model of scoped
   honesty.
8. **`scripts/check_no_raw_asof_join.sh`** — `--self-test` writes an OFFENDING fixture and asserts a
   hit, then a CLEAN one and asserts none; the PASS line prints the **real** scanned count. One
   residual gap: `run_scan` never asserts the list is non-empty.
9. **`.github/workflows/ci.yml:69-79`** — *"`shell: bash` is LOAD-BEARING, not style (bug-log #84)"*:
   GitHub's default has no `pipefail`, so `verify_anchors.sh | tail -1` exited with *tail's* status and
   AD-2's only remote enforcement was inert from activation. Carries the demonstration inline.
10. **`.github/workflows/ci.yml:162-179`** — `always() &&` on the UI gates, with the finding recorded
    above them: *"a gate that never executes is indistinguishable from a gate that passed."* The
    sentence this audit is named after was already in the CI file.

## Two notes that are not gate findings

- **`.github/workflows/ci.yml:1-9`** — the banner says the file is *"Parked as `.deferred` so GitHub
  Actions does NOT run it"*. The file on disk is `ci.yml`, with five `fix(ci)` commits against it. A
  reader who trusts the banner concludes every gate in the file is inert. **Delete the banner.**
- **A doc-comment run command that does not run the test.**
  `crates/trader/tests/llm_forecaster_neutrality.rs:18-20` says `cargo test -p strategy …` while the
  target lives in the `trader` package. The documented command exits "no test target"; an operator
  running M-FINAL's T-T3 by copy-paste never executes the gate.

## The shape, restated

Six of the fourteen CONFIRMED findings are one mechanism: **the gate's expectation and the gate's
measurement come from the same place.** Finding 1 hashes a committed file against a constant copied
from it. Finding 4 compares a function to its own delegate through a copied mapping. Findings 6, 7,
10 and 11 each report a count they did not measure.

The distinguishing move is unchanged from `#114`: **make the success path observable.** Print what
satisfied the guard, print how many files the walk read, and assert on that number — not on the
absence of findings in a set nobody proved was non-empty.
