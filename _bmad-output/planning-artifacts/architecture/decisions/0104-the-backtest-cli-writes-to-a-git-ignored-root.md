---
adr: 0104
title: The backtest CLI writes to a git-ignored root and reads its baselines from the committed corpus
status: accepted
date: 2026-11-24
supersedes: none
superseded-by: none
related: ["ADR-0037 phase-b-scenario-dispatch-extraction", "ADR-0038 vol-forecast-verdict-shape (§ D6.b re-emission protocol)", "ADR-0055 lab-run-persistence-topology-and-anchor-safety"]
---

# ADR-0104: The backtest CLI writes to a git-ignored root and reads its baselines from the committed corpus

## Context

Bug-log `#210`. The `backtest` bin (`crates/backtest/src/main.rs`) defaulted its report
writes to `evidence/<feature>/reports/` (container depth 0) through
`fn report_dir_for_scenario`, and read the baseline that four scenarios declare from the
same directory through `find_latest_report`. The `spec/v1` reorganisation
(`1395520629dc`, 2026-06-28) moved all 10 mapped features to `evidence/v1/<feature>/`
and left the code alone, so:

- **The baseline lookup found nothing, silently.** `find_latest_report` opened with
  `if !dir.exists() { return None; }` and neither call site had an `else`, so every run of
  `btc-2024-h1-sma-cross`, `btc-2023-1m-macd-trend`, `btc-2023-1m-rsi-reversion` and
  `btc-2023-1m-bbands-mean-revert` wrote `baseline_report: n/a` (the `#66` skip-as-pass
  shape). MEASURED: the committed 2026-11-08 v0.3.0 re-emission reports of macd, rsi and
  bbands say `baseline_report: n/a` in their front matter (line 7), while their
  pre-reorganisation reports name
  `backtest-20260420-151944-btc-2023-1m-sma-baseline-refresh.md`.
- **A default run planted a twin tree inside the corpus.** Nothing ignored
  `evidence/<feature>/reports/`. `scripts/verify_anchors.sh` resolves each anchor with a
  byte sort over full paths (`sort | tail -1`), so for six of the ten features a fresh
  twin sorted AFTER the committed body and replaced it in the comparison; since `#205`
  the Reports picker and the Lab also read container depth 0 under `evidence/`.

The deeper defect predates the reorganisation: the default write location pointed INTO
the anchored corpus, so before it every default run planted a body the gate's
newest-wins sort would compare (the `#113` class). The only contract text was
ADR-0037's "produces the same `spec/<feature>/reports/backtest-<stamp>-<scenario>.md`
file at the same path", written for the container-depth-0 layout.

## Decision

**Scope.** This ADR binds the `backtest` bin (`crates/backtest/src/main.rs`; D1-D6) and,
by the 2026-11-28 amendment below (bug-log `#227`), `run_yahoo_sma`, `monte_carlo` and
`param_robustness_sweep` (D1, D2, D7). `threshold_sweep` and the `forecast` family are
bug-log `#128`; the `passive_baseline_equity` example is not bound.

- **D1 — Default output is git-ignored.** With no `--reports-dir`, the bin writes
  `backtest-runs/<feature>/reports/backtest-<stamp>-<scenario>.md` (and its
  `artifacts/backtest-<stamp>-<scenario>/equity-<stamp>.csv` companion,
  `report::write_equity_companion`), relative to the working directory, with `<feature>`
  from `backtest::cli_paths::scenario_to_feature` (unknown scenarios: `_unknown`).
  `.gitignore` carries the unanchored `backtest-runs/`, following ADR-0055 D1/D2's
  `/lab-runs/` precedent: git-ignored and outside `evidence/` ⇒ outside every
  `evidence/**` glob ⇒ invisible to `verify_anchors.sh`. Because the root is cwd-relative,
  the guarantee holds for any working directory outside `evidence/`; a run started from
  inside `evidence/` lands under the gate's walk, as at HEAD, and `.gitignore` carves out
  `!/evidence/**/backtest-runs/` so that such a run stays visible in `git status`.
- **D2 — Emitting into `evidence/` is explicit.** `--reports-dir <dir>` keeps its
  semantics and is the one deliberate way to write a report into the corpus (the
  ADR-0038 § D6.b re-emission practice, e.g. `scripts/wave_b_emit.sh`). The CLI does not
  refuse it. (`scripts/relock/run_lane_scenarios.sh` uses the same flag the other way: it
  re-runs into a scratch dir and refuses an out-dir inside `evidence/`.)
- **D3 — Baselines come from the committed corpus.** `backtest::cli_paths::resolve_baseline`
  looks up the declared baseline (`BASELINE_DECLARATIONS`) in the baseline scenario's
  feature dir under `evidence/` at container depth ≤ `CORPUS_MAX_CONTAINER_DEPTH` (= 1,
  ADR-0055 D8's bound; a dir named `reports` is never a container). Exactly one home is
  required: none is `NoCorpusHome`, two or more is `TwoCorpusHomes` (a stray twin is
  refused, never silently picked). The newest exact `backtest-*-<baseline>.md` file wins,
  the gate's own match (HEAD used `contains`). The output root is never consulted, and
  neither is `--reports-dir`, because a committed report must not name a git-ignored file.
- **D4 — A missing baseline refuses the run, before any write.** A declared baseline that
  resolves to nothing, or whose home cannot be found or read, is an error naming the
  scenario and its baseline. The bin resolves it directly after `Scenario::from_name`,
  before any data load or `create_dir_all`, so a refusal writes nothing (`#66`, `#187`).
- **D5 — The anchor gate is unchanged.** `verify_anchors.sh`'s subject is the committed
  corpus; fresh-run reproduction belongs to the determinism / R-REPRO apparatus.
- **D6 — The depth bound is mirrored, and pinned.** `backtest` cannot depend on `ui`, so
  `CORPUS_MAX_CONTAINER_DEPTH` re-implements `ui::reports::layout::MAX_CONTAINER_DEPTH`
  narrowly. A `ui` lib unit test (`layout.rs`, LY-210) asserts the two are equal; `ui`
  already depends on `backtest`, and the lib unittests target is not a declared target of
  `scripts/ci_test_target_counts.py`, so it moves no declared count and no pixel. (The
  design's first draft kept a doc-link-only mirror on the claim that a `ui` test would
  change declared target counts; that claim was false, and an unpinned copy is the
  `#178`/`#180` drift shape.)

The resolver lives in the lib (`crates/backtest/src/cli_paths.rs`), not in `main.rs`. A
bin target could carry its own `#[cfg(test)]` unit tests; the lib is chosen because the
integration test `tests/cli_report_paths.rs` must call the same functions the bin calls.

## Alternatives considered

- **`evidence/v1/<feature>/reports` as the default** — rejected: every default run would
  plant a body the gate compares (`#113`).
- **`lab-runs/` as the default** — rejected: the Lab's cold path (`get_or_load_roots`,
  which diverges from ADR-0055 § D5; bug-log `#209`) makes `lab-runs/` win every tuple
  collision, and under D5's own most-recent-wins rule a fresh CLI run would win too, so
  CLI runs would shadow the committed evidence in the Lab.
- **Baseline from the output root first** — rejected: a committed report would name a
  git-ignored file.
- **A `warn!` plus `baseline_report: n/a` on a miss** — rejected: that is the defect.
- **`verify_anchors.sh` takes the CLI root as an input** (bug-log `#210` Fix 1's aside) —
  rejected here: the gate's subject is the committed corpus, and changing the gate needs
  its own self-test, probes and wiring (`#190`).
- **Move the layout contract into a crate both depend on** — not now; revisit at the next
  reorganisation. D6's test pins the one shared value.

## Consequences

- A default run (from any working directory outside `evidence/`) never adds a file under
  `evidence/`, so it can never shadow an anchored body, appear in the Reports picker or win
  a Lab tie. A container-depth-0 twin left by a pre-fix run, or made by an explicit
  `--reports-dir evidence/<feature>/reports`, still has those effects. The CLI detects only
  a twin of `v0-paper-sma`, the one baseline feature: it then refuses its four baseline
  scenarios (`TwoCorpusHomes`). A twin of any other feature is not detected by the CLI.
  Nothing is ever auto-deleted under `evidence/` (the old post-PASS prune is retired,
  bug-log `#228`).
- The four baseline scenarios run only where `./evidence` holds the baseline: from the
  workspace root, or from a sandbox that plants it. `crates/backtest/tests/determinism.rs`
  seeds the two committed file names as empty stubs (`seed_baseline_stubs`); the stubs
  reach the `baseline_report:` front-matter line only, so every in-test body-SHA constant
  is unchanged. `scripts/wave_b_emit.sh` now `cd`s to the workspace root, so its composed
  group finds the baseline from any working directory.
- The `--release` determinism invocation does not test this: those helpers run the
  hard-coded `target/debug/backtest` and rebuild it only when absent, so the gate for this
  ADR runs the determinism targets in debug, as CI does (bug-log `#229`).
- `CARGO_BIN_EXE_backtest` in a debug `cargo test` is the same uplifted
  `target/debug/backtest` that `determinism.rs` and its R-REAL rebuild use; there is no
  race only because cargo runs test binaries one after another.
- Enforced by:
  - D3/D4 resolution: `crates/backtest/src/cli_paths.rs` unit tests C1-C5 (the depth
    bound, `reports` never a container, two homes refused) and B1-B3 (the exact match, a
    missing baseline refused, no disk read for an undeclared scenario);
    `crates/backtest/tests/cli_report_paths.rs` R1 (every declaration resolves to a
    git-tracked report) and R2 (the declarations equal the committed corpus's
    `baseline_report:` pairs).
  - D1 output root: `cli_report_paths.rs` R3 (`cli_paths::report_dir_for_scenario` is
    outside `evidence/` and git-ignored for every anchored scenario; controls: a corpus
    path and a path under `evidence/<c>/backtest-runs/` are NOT ignored, a crate-dir run
    is). That pins the library function, not the bin's call sites.
  - D1 at the bin's eight writers: an assertion that the printed `Report written:` path
    starts with `backtest-runs` in the default-path runs of `W1` (final SMA writer),
    `determinism.rs`'s `t33_report_sha256_deterministic`, `assert_scenario_deterministic`
    and `run_scenario_once` (final SMA/composed writer; Momentum via `t717_top10_*`;
    TcnOverlay via `tt1_*`) and `multi_pair_determinism.rs`'s `run_pairs_scenario_once`
    (Pairs). Four writers stay unpinned at the bin level because their scenarios need
    `candle` or real data: TcnWeights, PatchTST, GARCH and RegimeDispatcher.
  - D4 ordering and refusal: W1 (the real bin names the committed baseline and adds
    nothing under `evidence/`) and W2 (a missing baseline exits non-zero with the refusal
    sentence and leaves the sandbox byte-identical in names, sizes and mtimes).
  - D6: `crates/ui/src/reports/layout.rs` LY-210.
  - Probe table: bug-log `#210` and the module doc of `tests/cli_report_paths.rs`.
- Anchor-neutral: no `evidence/` byte changes, no report-template change;
  `baseline_report:` is front matter, excluded by `extract_report_body` and
  `hash_report.py`. 119/119.

## Amendment 2026-11-28 (bug-log `#227`: the crate's three research bins write to the same root)

### Context

Until this amendment `run_yahoo_sma` defaulted `--reports-dir` to
`evidence/v1/lab-yahoo-realdata/reports`, `monte_carlo` defaulted `--out-dir` to
`evidence/v1/strategy-robustness-harness/reports/`, and `param_robustness_sweep` defaulted
`--out-dir` to `evidence/v1/momentum-parameter-robustness-sweep/reports/` and, by value
equality with that literal, re-routed it to six more anchored family dirs (horizon, TS,
carry, basis, MN, MR). Each writes a date-stamped name (`backtest-<stamp>-<s>.md`,
`robustness-<ts>-<s>.md`, `robustness-sweep-<ts>-<s>.md`), so a default run planted an
untracked file the gate's newest-wins resolver picks: the `#113` mechanism `#210` fixed for
the `backtest` bin. The same value-equality check also re-routed an EXPLICIT
`--out-dir evidence/v1/momentum-parameter-robustness-sweep/reports[/]` on all 9
non-momentum family invocations (9 of the 10 in PS2's table; 6 distinct dirs), only the
momentum row passing through, contradicting the bin's own "an explicit `--out-dir` always
wins". The sweep's front-matter `slug:` and its router carried two
unpinned copies of the same 7-way family mapping.

**Measured** (a read-only simulation in the `#227` design pass, HEAD `1c5c6f04af6c`, not
re-run here): for every anchored row these bins can produce, one hypothetical default-run
path appended to the resolver's real candidate list, each branch's own selection applied,
under `LC_ALL=C` and `LC_ALL=en_US.UTF-8`; without the hypothetical path the picks equal
`verify_anchors.sh --explain` for all 38 simulated rows (37 producible plus
`eth-2024-h1-sma-cross`, which `run_yahoo_sma` cannot mint). The fresh default path was PICKED
for **37 of 37** rows in both locales: `lab-yahoo-realdata-v0.1.1`/`-v0.1.3` (2 rows; after
the shared prefix `…/lab-yahoo-realdata` the fresh path's next byte is `/`, every committed
one's `-`, so it sorts last whatever its stamp), the `mc-robustness-2026-06` mc row (same
dir, newer stamp) and its 6 sweep rows, `horizon-retest-robustness` (8),
`perp-basis-signal-robustness` (8) and `perp-basis-mn-spread` (12). The 34 sweep rows are
`scripts/relock/surfaces.tsv`'s.

### Decision

- **D7 — The research bins write to the same git-ignored root.** The rule: the new default
  is the pre-amendment default with its leading `evidence/v1/` replaced by
  `backtest-runs/`. So:
  - `run_yahoo_sma`: `backtest-runs/lab-yahoo-realdata/reports`
    (`cli_paths::YAHOO_SMA_FEATURE`);
  - `monte_carlo`: `backtest-runs/strategy-robustness-harness/reports`
    (`cli_paths::MONTE_CARLO_FEATURE`);
  - `param_robustness_sweep`: `backtest-runs/<family>/reports`, `<family>` from
    `backtest::sweep_harness::sweep_family_feature(horizon, selection_mode, score_source,
    direction)`, one of the seven dirs above.

  Feature slugs, file names and report bodies do not change, so promoting a run into the
  corpus is a root swap through the explicit flag. The flags are `Option<PathBuf>`: absent
  means `cli_paths::effective_reports_dir(None, feature)` =
  `cli_paths::default_reports_dir(feature)` (the same `CLI_OUTPUT_ROOT` and shape as D1's
  `report_dir_for_scenario`); present means the value is used verbatim (D2), for every
  sweep family, and the value-equality re-route is retired. One mapping,
  `sweep_family_feature`, names both a sweep report's front-matter `slug:` and its default
  dir; `MONTE_CARLO_FEATURE` does the same for `monte_carlo` (MEASURED: the function equals
  both pre-amendment copies on all 72 axis tuples clap can parse; the slug edits produce
  identical bytes). `run_yahoo_sma` keeps its workspace walk for both forms: from inside the
  workspace a first run lands at `<ws>/backtest-runs/…`, unless
  `<cwd>/backtest-runs/lab-yahoo-realdata/reports` already exists; `monte_carlo` and the
  sweep stay cwd-relative, as the `backtest` bin does. An explicit path into `evidence/` is
  not refused (D2). The existing `.gitignore` rule and carve-out already cover every new
  default (`git check-ignore --no-index`), so no ignore rule is added.

### Consequences

- A default run of any of the three never adds a file under `evidence/` from a working
  directory outside it, so it can no longer shadow an anchored body.
- An explicit `--out-dir` / `--reports-dir` into an existing anchored dir still plants a
  picked body: that is D2, by design. `scripts/relock/run_surfaces.sh` keeps its mandatory
  `--out-dir` and its evidence guard.
- Per family, an explicit `--out-dir evidence/v1/momentum-parameter-robustness-sweep/reports/`
  now lands there verbatim. For MR, carry and TS it is still PICKED, because the
  `mc-robustness-2026-06` branch reads the momentum dir before theirs; the pre-amendment
  re-route planted a picked body in their own dir, so the harm is the same. For horizon,
  basis and MN, whose branches read only their own dir at `-maxdepth 1`, the verbatim write is
  unpicked clutter where the re-route planted a PICKED body: strictly safer.
- Residual for `run_yahoo_sma`: from a cwd under `evidence/` with
  `<cwd>/backtest-runs/lab-yahoo-realdata/reports` already present, the walk's fast path
  writes under the legacy resolver's `find "$root"/evidence`; that file is picked whenever its
  full path sorts after the committed body (e.g. from `evidence/v2/…`, not from `evidence/`
  or `evidence/v1/`). The carve-out keeps it visible in `git status`. `monte_carlo` and the
  sweep have no such residual: their branches read fixed `v1` dirs at `-maxdepth 1`, and no
  `robustness-*` name matches a legacy pattern.
- The pre-amendment yahoo default (container depth 1) was reachable by all three ADR-0055 D8
  readers (the Reports picker, the Lab's report lookup, Compare's scan); a default run no
  longer is. ADR-0055 is not amended: `run_yahoo_sma` is no D8 reader.
- A stale default-run file planted by a pre-amendment binary is neither detected nor
  deleted; nothing is ever auto-deleted under `evidence/` (`#228`).
- `--help` no longer prints `[default: …]` for these flags; each doc comment states the
  default.
- Not decided here: `monte_carlo` mints the anchored scenario name for any `--paths` /
  `--ensemble-seed` (bug-log `#237`).
- Enforced by:
  - `crates/backtest/src/cli_paths.rs` unit tests D1 (`default_reports_dir` and both feature
    constants against literals; `report_dir_for_scenario` has the same shape) and D2 (the
    flag two-sided: absent → default, present → verbatim, into `evidence/` included, and a
    value that looks like another feature's default is not treated as absent);
  - `param_robustness_sweep`'s in-bin PS1 (real clap parsing: no `--out-dir` parses as
    absent and each of 10 family invocations resolves to `backtest-runs/<family>/reports`)
    and PS2 (an explicit value, the pre-amendment re-routed literal included, is verbatim
    for all 10);
  - `crates/backtest/tests/research_bin_report_paths.rs`: G1 (every default dir is outside
    `evidence/` and git-ignored; control: the evidence home is not), G2 (the default slugs
    equal the dirs the committed corpus keeps the mc body and all 34 manifest rows in),
    SRC1/SRC2 (each `main` passes the parsed flag and axes, read from the production part
    of the source), WM1/WS1 (the real `monte_carlo` and sweep, run with no flag in a
    sandbox, write exactly one git-ignored file under `backtest-runs/` and nothing under
    `evidence/`);
  - `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` WY1/WY2/WY3 (the real bin on the
    committed `data/yahoo-sample/`: the default relative to a cwd with no `Cargo.lock` above
    it, an explicit evidence path verbatim, and the default under a workspace marker), run by
    the exact CI command `cargo test -p backtest --features yahoo --test
    run_yahoo_sma_ticker_flag`;
  - probes P227-1..16 in bug-log `#227` and the module doc of
    `tests/research_bin_report_paths.rs`.
- Anchor-neutral: no `evidence/` byte, report body, template or file name changes; the two
  slug edits are front matter and render identical bytes. 119/119, `--explain`
  byte-identical before and after.

## Changelog
- 2026-11-24 (orchestrator, bug-log `#210` fix): initial accept. Supersedes ADR-0037's
  path clause ("the same `spec/<feature>/reports/backtest-<stamp>-<scenario>.md` file at
  the same path"); its body-byte clause stands. Amends ADR-0055 D8's "readers outside D8"
  entry for the backtest CLI.
- 2026-11-25 (orchestrator, bug-log `#229` fix): the Consequences bullet on the `--release`
  determinism invocation ("those helpers run the hard-coded `target/debug/backtest`") is
  historical: the determinism helpers spawn `CARGO_BIN_EXE_backtest`, the invocation's own
  build (ADR-0045 § D7.2a). The next bullet now reads, for `determinism.rs`: its helpers
  spawn that same path, and under a no-feature invocation nothing in that target rebuilds it
  (R-REAL builds only under `--features`), so its "no race" premise holds by construction
  there; under `--features` the builders re-link it while siblings spawn it (pre-existing,
  open as bug-log `#236`). No decision of this ADR changes.
- 2026-11-28 (orchestrator, bug-log `#227` fix): amendment D7 (above). Scope widened to
  `run_yahoo_sma`, `monte_carlo` and `param_robustness_sweep` (D1, D2, D7); their defaults
  move from `evidence/v1/<feature>/reports` to `backtest-runs/<feature>/reports`, their
  flags become `Option` (verbatim when present; the sweep's value-equality re-route is
  retired), and one family mapping (`sweep_harness::sweep_family_feature`) names both the
  sweep's front-matter `slug:` and its default dir. No decision D1-D6 changes.
