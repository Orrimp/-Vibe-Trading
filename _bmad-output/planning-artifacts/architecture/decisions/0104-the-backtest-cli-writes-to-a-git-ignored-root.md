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

**Scope.** This ADR binds the `backtest` bin (`crates/backtest/src/main.rs`) only. The
crate's other bins that default into `evidence/v1/` (`run_yahoo_sma`'s `--reports-dir`,
`monte_carlo`'s and `param_robustness_sweep`'s output dirs) are bug-log `#227`, and the
`forecast` family is bug-log `#128`; neither is decided here.

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

## Changelog
- 2026-11-24 (orchestrator, bug-log `#210` fix): initial accept. Supersedes ADR-0037's
  path clause ("the same `spec/<feature>/reports/backtest-<stamp>-<scenario>.md` file at
  the same path"); its body-byte clause stands. Amends ADR-0055 D8's "readers outside D8"
  entry for the backtest CLI.
