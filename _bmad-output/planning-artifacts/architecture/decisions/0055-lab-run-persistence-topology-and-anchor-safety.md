---
adr: 0055
title: Lab run artifacts live in a git-ignored lab-runs/ root outside the anchor namespace
status: accepted
date: 2026-06-12
supersedes: none
superseded-by: none
updated: 2026-11-22
---

# ADR-0055: Lab run persistence topology & anchor-safety

## Context

With live execution retired (`docs/dev-notes/live-trading-removed-2026-06-12.md`),
the cockpit **Lab** is the project's strategy-checking surface, but it cannot
**persist** a run: `backtest::engine::run_scenario` builds the full in-memory
`RunReport` yet returns `report_path: None` even when `cfg.write_report = true`
(the deferred "Phase C", `crates/backtest/src/engine.rs:608`). The
`lab-run-save-compare` feature closes that gap — but the closure forces ONE
load-bearing topology decision: **where do Lab-generated reports live on disk?**

The hazard is mechanically precise. `scripts/verify_anchors.sh:88` resolves each
of the 119 anchored scenarios to a file via
`find "$root"/spec -type f -path "*/reports/backtest-*-$scenario.md" | sort | tail -1`
— the **lexicographically-newest** `backtest-*-<scenario>.md` *anywhere under
`spec/**/reports/`* is the file whose frontmatter-stripped body is hashed against
the locked SHA. A Lab run that writes `backtest-<new-stamp>-<scenario>.md` whose
scenario collides with an anchored scenario AND whose stamp sorts newest, into
**any** `spec/*/reports/` directory, would silently shadow the anchor → the gate
hashes the Lab run's body → mismatch → broken regression gate. The two existing
loaders that must *read* Lab reports — `EquityCache::get_or_load` /
`discover_reports` (`crates/ui/src/lab/equity_loader.rs:173`) and
`compare::cache::scan_spec_tree` (`crates/ui/src/compare/cache.rs:300`) — filter
files by exactly the anchor glob's predicate (`/reports/` subdir +
`starts_with("backtest-")` + `.md`). So "readable by the loaders" and "invisible
to the verifier" **cannot both hold for any path under `spec/`**: the loaders'
filter IS the anchor glob's filter. This is a persistence-topology decision with
a durable contract (a git boundary), weighed alternatives, and a precedent for a
future "promote a Lab run into committed `spec/`" flow — hence an ADR.

## Decision

**D1 — Home.** Lab-generated reports are written to a **git-ignored `lab-runs/`
directory at the workspace root**, never under `spec/`. The per-run path is
`lab-runs/<strategy-slug>/reports/backtest-<stamp>-<scenario>.md` — the
`<slug>/reports/backtest-*.md` shape underneath the root is **identical** to the
`spec/` layout, so both loaders read it with only a ROOT change. The
`<strategy-slug>` matches `equity_loader::strategy_slug` (e.g. `v0.sma` →
`v0-paper-sma`, `v1.momentum` → `v1-cross-sectional-momentum`).

**D2 — Anchor-safety is by construction, not by convention.** Add `/lab-runs/`
to `.gitignore` (the `/data/*` precedent). Because `verify_anchors.sh` only ever
`find`s under `spec/`, a sibling `lab-runs/` is **invisible** to it; the verifier
stays 119/119 after any number of Lab writes. No row is added to
`spec/anchors.toml`; no anchored body-SHA is touched. The git-ignore is the
mechanical guarantee that anchored reports remain byte-immutable (CLAUDE.md
non-negotiable / ADR-0038 § D6).

**D3 — Write seam.** `run_scenario` reaches disk through an injectable override:
a `reports_dir: Option<PathBuf>` field on `ScenarioConfig`, anchor-additive via
`#[serde(default)]` / struct-update (the `ScenarioDataSource` /
`latency_slippage_sim` precedents). `None` ⇒ default to
`ui::lab::equity_loader::default_lab_runs_root()` is NOT possible (backtest must
not depend on ui — `engine::DateRange` is duplicated for exactly this reason);
instead the **Lab caller** (`crates/ui/src/lab/runner.rs`) supplies
`Some(default_lab_runs_root())`, and a `None` `reports_dir` with
`write_report = true` falls back to a backtest-local default that resolves the
same workspace-root `lab-runs/`. The write is gated by `cfg.write_report`; all
CLI / anchor-generating call sites pass `write_report = false` (or never
construct a Lab `reports_dir`), so their bytes are unchanged. Tests point
`reports_dir` at a tempdir — this is the R3 "every external I/O behind a seam"
satisfaction (a path override, the lighter read of the rule; the write itself is
the already-isolated `report::*::write`).

**D4 — The report body reuses the existing writers verbatim.** Each dispatch arm
calls the matching `crates/backtest/src/report/<family>::write`
(`momentum`/`pairs`/`tcn_overlay`/`sma`), feeding it the same inputs the CLI
feeds (`main.rs:1404/1474/1572/2142`). For the four single-symbol composed arms
(`v0.sma`/`v0.5.macd`/`v0.5.rsi`/`v0.5.bbands`) the writer is `report::sma::write`,
fed from `SmaComposedRunResult.state` + `.strategy_meta` (both already on the
result struct, exactly as `main.rs:2109-2110` does it). The persisted **body** is
byte-identical to the CLI output for the same `(strategy, pair, range, seed,
data_source)` — the determinism contract (ADR-0030 / ADR-0002). Run-varying
metadata (`generated:`, `wall_clock_s:`) lives in frontmatter, stripped before
hashing (ADR-0032 § D4). `elapsed_secs` for the SMA writer is frontmatter-only
and MAY be `0.0` on the engine path without affecting the body SHA.

**D5 — Read precedence (two roots, lab-runs first).** The Lab and Compare
loaders read the **union of two roots in a fixed order: `lab-runs/` first, then
`spec/`**. This lets the operator diff a fresh Lab run against a committed
anchored reference, while preserving the pre-existing "load a committed report"
behaviour. Collision rule: when a `lab-runs/` report and a `spec/` report resolve
to the **same `(strategy, symbol, range)` tuple**, the existing most-recent-wins
tiebreaker (`generated:` frontmatter) decides the Lab hot-path pick; on an
*identical filename* across both roots, **`lab-runs/` wins** (it is searched
first). Filenames carry a **sub-second (millisecond) stamp** so two fast
successive Lab runs of one tuple never collide on filename (the CLI's
second-precision stamp is insufficient for the Lab's rapid-iteration cadence —
the filename stamp is computed by the *caller*, independent of the writer's
second-precision `generated:` frontmatter).

**D6 — H3 invariant: write-root == read-root.** The `report_path` the engine
returns and the root the cache reads from MUST be the same directory tree.
`crates/ui/tests/lab_run_engine.rs::h3_in_memory_equals_cached_disk` enforces
this structurally: `test_config` threads its tempdir into `reports_dir`, the
engine writes `<tempdir>/<slug>/reports/backtest-*.md`, and the test derives the
read-root as `report_path.parent().parent().parent()` (== the tempdir) before
calling `get_or_load(tuple, read_root)`. Equal roots ⇒ the just-written report is
the one parsed ⇒ element-by-element `in_memory == cached_disk`. This is the
divergence-style guarantee for this feature (a "wrote the file but it's
wrong/empty" regression fails loudly). The pre-existing `report_path = None` skip
branch (`lab_run_engine.rs:110-116`) becomes dead once the engine returns
`Some(...)`.

**D7 — Bounded retention.** `lab-runs/` is operator-local and unanchored, so it
is purged in-process: **keep the last N reports per `(strategy, symbol, range)`
tuple (N = 20), purge on run completion** (the cheap equivalent of the ledger's
nightly purge; Compare's most-recent-wins already tolerates multiple reports per
tuple). A future feature wanting *queryable* run metadata (filter/sort runs by
KPI) should revisit the audit SQLite ledger as the home — Markdown files do not
index well — but that is out of scope here.

## Alternatives considered

- **Audit SQLite `lab_runs` table** (the just-shipped `equity_snapshots`
  precedent) — anchor-safe and the strongest crash story, but **discards** the
  Markdown-report rendering plus BOTH existing Markdown loaders
  (`EquityCache`/`scan_spec_tree`), forcing a new equity-from-DB read path on the
  Lab and Compare sides. The right home only IF a future feature needs queryable
  run metadata; a much larger build for the same v0.1.0 outcome. Named as the
  future home in D7.
- **A committed `spec/lab-runs/`** — rejected: the loaders' file filter
  (`/reports/` + `starts_with("backtest-")`) is *identical* to the anchor glob,
  so "readable by the loaders" and "invisible to `verify_anchors.sh`" cannot both
  hold under `spec/`; and committing ad-hoc operator runs pollutes git history.
- **A `ReportSink` trait injected into `run_scenario`** (instead of the
  `reports_dir` path override) — a fuller I/O abstraction (no real FS in tests),
  but more surface than v0.1.0 needs; the write itself is already the isolated
  `report::*::write`, and the path override is the lighter satisfaction of the
  "I/O behind a seam" rule (D3). Defensible for a later refactor.
- **Loaders read ONLY `lab-runs/`** (single root) — simpler, but loses the
  diff-a-fresh-run-against-a-committed-anchored-reference affordance and darkens
  the pre-existing "load a committed report" path. Rejected for D5's two-root
  union.

## Consequences

- **Anchor gate:** `scripts/verify_anchors.sh` stays **119/119** after a Lab run
  writes a report — by construction, since `lab-runs/` is outside every `spec/**`
  glob (the feature's AC7, a mechanical proof). `scripts/check_determinism_anchors.py`
  is unaffected (no anchor mutation). If a future change ever moves the Lab home
  *under* `spec/`, this ADR is violated and the gate breaks — that move requires
  a superseding ADR.
- **Byte-immutability:** anchored reports in `spec/*/reports/` are never written
  by the Lab path (CLAUDE.md non-negotiable / ADR-0038 § D6 upheld by the git
  boundary, not by reviewer vigilance).
- **Determinism:** AC2/AC6 (H3) enforce body byte-identity for a fixed seed via
  the shared `report::*::write` writers and the write-root == read-root invariant
  (D6).
- **Anchor-additive config:** the new `ScenarioConfig.reports_dir` field MUST be
  `#[serde(default)]` / struct-update so the 34 anchor-generating constructors
  stay byte-safe (D3); a non-additive add would regress all 34 SMA/composed
  anchors. Enforced by the AC7 verifier run and the existing
  `scenario_data_source` neutrality test pattern.
- **Money discipline:** the `report::*::write` writers already use `Decimal` /
  `Money<Usdt>` for money; Sharpe/drawdown stay display-only `f64` per ADR-0003.
  No new `f64` money is introduced (ADR-0003 upheld).
- **No live trading:** this persists/compares output of the SHIPPED backtest
  engine on real on-disk data; it touches no live/paper execution path
  (`live-trading-removed-2026-06-12.md` scope retained).
- **Retention is bounded** (D7) so the cache dir does not grow without bound
  (the feature's AC8).

## Amendment 2026-11-15 (bug-log `#205`: the Reports picker and the Lab walk container depth ≤ 1)

**Vocabulary.** **Container depth** counts the directories between a report root and `<slug>/`:
`<root>/<slug>/reports/` is container depth 0, `<root>/<container>/<slug>/reports/` is container
depth 1. Every `spec/` in § Context and § D1–D7 reads `evidence/` since `452ce026` (2026-07-25),
which moved the tree with its container depths unchanged.

**What this amendment corrects, and what it hands off.** It rewrites no sentence of D1–D7.

1. **§ D1, second sentence** ("the `<slug>/reports/backtest-*.md` shape underneath the root is
   **identical** to the `spec/` layout, so both loaders read it with only a ROOT change"): false
   since `1395520629dc`. Corrected under "D1's premise corrected"; D8 replaces it for the readers
   that walk D8, Compare among them since bug-log `#206`.
2. **§ D5, third sentence** ("the existing most-recent-wins tiebreaker (`generated:` frontmatter)
   decides the Lab hot-path pick"): the Lab has never done this. Recorded under "The Lab's
   precedence" and handed off to bug-log `#209` (AD-18), where either the code or § D5 moves.
3. **§ D1, third sentence** ("The `<strategy-slug>` matches `equity_loader::strategy_slug`"): the
   engine's `strategy_dir_slug` maps ids that `strategy_slug` does not. Handed off to bug-log
   `#212`; D8 does not depend on it.

**Context.** `1395520629dc` (2026-06-28, the `spec/v1` reorganisation) moved 68 of the 125
committed `backtest-*.md` reports from container depth 0 to container depth 1, every report of the
four slugs `strategy_slug` maps to among them. The 57 reports of the four
`v5-latency-slippage-sim-v0.*` dirs stayed at container depth 0, and `lab-runs/` kept container
depth 0. Nothing amended D1, and both `ui` readers of the committed corpus kept a one-level walk:

- **The Lab:** `equity_loader::discover_reports` read only
  `let dir = report_root.join(slug).join("reports");`. On `evidence/` it found no report for any
  mapped slug, so § D5's "load a committed report" fallback answered nothing.
- **The Reports picker** (`reports::loader::discover_reports`, a reader this ADR does not name)
  read only `let reports_dir = slug_path.join("reports");`. It listed no container-depth-1 report,
  and every report with an equity companion is one, so boot auto-select selected nothing.

No test failed: the tests that should have caught it skipped. The corpus census, the skip
sites and the probe kills are recorded in bug-log `#205` and in the T-T4 tables in the module docs
of `crates/ui/tests/reports_populated_curve_render.rs` and `crates/ui/tests/live_equity_render.rs`.

**D1's premise corrected.** D1's home is unchanged; what changes is the reason both roots are read.
D1's "both loaders" are the Lab's `EquityCache::get_or_load` / `discover_reports` and Compare's
`compare::cache::scan_spec_tree` (walking in `scan_one_root`). The shapes are not identical:
`lab-runs/` is container depth 0; the committed corpus is container depth 0 (the v5 dirs) plus
container depth 1 (`evidence/v1/`, `evidence/v3/`). The Lab reads both roots because it walks D8,
not because the layouts match, and the Reports picker walks D8 too. Compare walked only
`<root>/<x>/reports/` (`let reports_dir = strategy_dir.join("reports");`) until bug-log `#206`
moved `scan_one_root` onto D8 (`for rd in layout::report_dirs(root) {`), so the corrected
premise now holds for all three `ui` readers.

**D8: the report-root layout contract, for the `ui` readers that walk it.** One module,
`crates/ui/src/reports/layout.rs`, holds the rule:

- The layout is `<root>[/<container>]/<slug>/reports/backtest-*.md`, at container depth ≤
  `layout::MAX_CONTAINER_DEPTH` (= 1). Nothing deeper is read; raising the constant amends D8.
- A directory named `reports` (`layout::REPORTS_DIR`) is never a container and never a slug, so the
  walk never descends into `reports/artifacts/…`.
- The slug is the LEAF directory name directly above `reports/`. The full path, which carries the
  container, stays in the reader's `PathBuf`.
- `backtest-*.md` is a FILE directly inside `reports/` whose name starts with `backtest-` and ends
  in lowercase `.md` (`layout::is_backtest_report`).
- **Order.** `layout::report_dirs(root)` yields the `reports/` dirs in (container depth, path)
  order: every container-depth-0 dir before any container-depth-1 dir, then by path.
  `layout::backtest_files(dir)` yields one dir's files by path. `layout::report_dirs_for_slug(root,
  slug)` equals `report_dirs` filtered to the slug, order included; it is empty for `reports` and
  for any slug that is not exactly one plain path component.
- Symlinks are followed, and a symlink loop stops at the depth bound. An absent or unreadable
  directory contributes nothing and never panics.

**The order rule, and why.** The order matters only to a reader that keeps the FIRST of equal
candidates. For the Lab that is an equal-score tie inside one tier: a report tagged with the
tuple's data source still beats an untagged one, and a strictly better range score still wins, at
either container depth. For Compare (since bug-log `#206`) it is an identical filename, or an
equal `generated:` stamp for one tuple, inside one root; between distinct filenames a strictly
newer `generated:` still wins at either container depth. On such a tie container depth 0 wins:
- It is where the one-level walk looked, so on such a tie the pick stays the report that walk
  would have found (for example a backtest-CLI twin at `evidence/<feature>/reports/`).
- The winner no longer depends on whether a slug's name sorts before its container's name:
  `"v1" < "v1-cross-sectional-momentum"`, but `"v0-paper-sma" < "v1"`, so full-path order would
  pick container depth 1 for one slug and container depth 0 for another.

**The readers D8 binds:**
- **The Reports picker:** `reports::loader::discover_reports_in(root)`, with `discover_reports()`
  its `evidence/` wrapper, and `reports::state::load_into_from(model, root)`, with `load_into` its
  wrapper. It re-sorts its list by (slug, file stem, path), so the D8 order decides nothing there.
  Boot auto-select takes the companion-bearing entry with the greatest file stem
  (`newest_companion_index`).
- **The Lab's report lookup, over both roots:** `lab::equity_loader::discover_reports(root, slug)`,
  behind `load_equity`, `EquityCache::get_or_load` / `get_or_load_roots` and
  `route_equity_overlay`. It concatenates `backtest_files` over `report_dirs_for_slug` in D8 order
  and never re-sorts, so the order is its tie rule.
- **Compare's scan, over both roots** (since bug-log `#206`): `compare::cache::scan_one_root`,
  behind `scan_report_roots` and its one-root wrapper `scan_spec_tree`. It walks `backtest_files`
  over `report_dirs` in D8 order and never re-sorts, so inside one root a container-depth-0 report
  claims an identical filename first and keeps its tuple on an equal `generated:` stamp; across
  roots `lab-runs/` is walked first and still wins an identical filename (§ D5). A cell is keyed by
  the report's `strategy.id` frontmatter; no directory name is read as a strategy or a slug. Pins,
  each two-sided (a container that sorts before the slug, and one that sorts after it):
  `duplicate_filename_inside_a_root_goes_to_container_depth_0` and
  `equal_generated_inside_a_root_goes_to_container_depth_0`; the lowercase-`.md` rule is pinned by
  `scan_skips_an_uppercase_md_report`, the cross-root filename rule by
  `scan_report_roots_identical_filename_lab_runs_wins`.

**The readers outside D8:**
- **The backtest CLI:** `report_dir_for_scenario` writes `evidence/<feature>/reports/` (container
  depth 0), and `find_latest_report` reads only that dir. `backtest` cannot depend on `ui`, so it
  cannot walk this module (bug-log `#210`). Because D8 reads container depth 0 under `evidence/`, a
  CLI run's report there is listed by the Reports picker and wins an equal-score, same-tier Lab tie
  against the committed container-depth-1 report.
- **The anchor gate:** `scripts/verify_anchors.sh` is not a `ui` reader. It `find`s
  `*/reports/backtest-*-<scenario>.md` at any depth under `evidence/`, and D8 does not bind it.

**The Lab's precedence (what the code does; it diverges from § D5).** The hot path is unchanged:
the in-memory mirror of the last run wins while its tuple is on screen. The cold path:

1. **Across roots, the first root with a match wins.** `get_or_load_roots` tries
   `default_report_roots()` = `[lab-runs/, evidence/]` in order (resolved once, into
   `Cockpit::lab_report_roots`, since bug-log `#208`) and stops at the first root whose
   `load_equity` returns `Ok` (`for root in roots {` … `found = Some(Arc::new(loaded));` …
   `break;`), so `lab-runs/` wins every tuple collision. The outcome, a hit or a miss, is then
   cached per tuple until the next `Message::LabRunCompleted` clears the whole cache (bug-log
   `#208`); that is the cache's lifetime, not a precedence rule.
2. **Within a root, the tier, then the first strictly-best score in D8 order.** A report tagged
   with the tuple's data source beats any untagged legacy one (`best_tagged.or(best_untagged)`).
   Inside a tier the FIRST report with the best `range_score` wins
   (`let is_better = slot.as_ref().is_none_or(|(s, _)| range_score > *s);`).
3. **So within one dir the OLDEST stamp wins a tie.** Names are `backtest-<stamp>-<scenario>.md`,
   so path order is stamp order. `generated:` is never read.

§ D5 says the Lab pick is "the existing most-recent-wins tiebreaker (`generated:` frontmatter)";
item 1 ignores `generated:` across roots, and item 3 picks the oldest report within a dir. Compare
does implement § D5 (`cell.generated_at.as_str() <= existing.generated_at.as_str()` skips a report
not newer than the kept one), so the two readers settle a collision differently. This predates
`#205`, which changes neither half; per AD-18 it is handed off to bug-log `#209`, and § D5 is not
rewritten.

**Pins.** Item 1: `route_overlay_two_root_union_lab_runs_wins_collision`,
`default_roots_order_lab_runs_first`, `lab_runs_shadows_a_container_depth_1_evidence_report` /
`container_depth_1_evidence_report_wins_when_searched_first`. The order rule, two-sided (the
container's name sorts before the slug's in one root, after it in the other):
`equal_score_tie_across_container_depths_goes_to_container_depth_0`, with
`equal_score_tie_at_one_container_depth_goes_to_the_first_dir` and
`a_strictly_better_report_in_the_second_report_dir_wins`. Item 3 inside ONE dir is not pinned;
bug-log `#209` adds that pin with its fix.

**What the Lab draws from committed evidence.** The committed reports of the four mapped slugs, all
under `evidence/v1/`, have no `## Equity curve` section and no sibling `-equity.csv` (the Lab's
companion rule); the Reports companions under `artifacts/<stem>/` use a layout the Lab does not
read. So every committed report the Lab resolves is `StartEndOnly`: two samples at the placeholder
timestamps `(0, 1)` (`make_start_end_series`: `Ok(vec![(0, initial), (1, final_eq)])`). It draws
only on the bar-less path, where `draw_equity_polyline_standalone` draws a full-width start→end
line. Over price bars it draws nothing, because both samples fall outside the bar window, and since
bug-log `#207` it reserves no equity gutter either: the chart widens its right gutter only when an
overlay draws (`widgets::chart::equity_axis_shown`, the bar-window gate of `compute_equity_range`
that the equity pass draws behind), so the frame equals the bars-only frame. LR3 first held
`lab_207_characterization_committed_start_end_only_report_draws_no_overlay`, which pinned the
empty gutter (its frame differed from the bars-only frame), so the `#207` fix replaced it under the
same key with `lab_committed_start_end_only_report_over_bars_is_pixel_identical_to_bars_only`,
which asserts that the report resolves through the production roots and then that the frame is
byte-identical to the bars-only frame.

**Unchanged.** D1's home; D2 (`/lab-runs/` is still git-ignored, and anchor safety still rests on
the git boundary); D3; D4; D5's root order, its identical-filename rule and its millisecond stamps;
D6 (`h3_in_memory_equals_cached_disk` still reads the engine's container-depth-0 write root through
`get_or_load`); D7. **Anchor safety:** no byte under `evidence/` changes, because the D8 readers
(Compare's scan among them since bug-log `#206`) only read it; `bash scripts/verify_anchors.sh`
prints `ANCHORS PASS  (119 / 119)`, and
`git status --porcelain -- evidence` prints nothing after the full test runs.

**Verification (2026-11-15, on the tree this amendment ships with).**

- `cargo test -p ui --lib`: 676 passed.
- `cargo test -p ui --test reports_populated_curve_render` passes with no "skipping" line (its
  former skip sites now fail); `cargo test -p ui --test live_equity_render`,
  `cargo test -p ui --test lab_run_engine` and the snapshot targets pass, baselines unmoved.
- Eleven probes, each applied exactly once and restored byte-identical, the FAILED set compared by name with its designed red set: P1 (`MAX_CONTAINER_DEPTH` 1 → 0) turns 22 tests red, D2, D3, R6, R7a, E1 and LR1 among them; P2 (1 → 2) turns 7 red, the container-depth-2 controls among them; P4, P6, P10, P11, P12, O5b, Q3, Q6 and Q8 each turn exactly their designed tests red (the full sets are in the T-T4 tables).
- Pixel margins (macOS): the Reports curve covers 3721–4371 px on the frames that must draw (threshold > 1000) and 0–252 px on the frames that must not (threshold < 500); the Lab's container-depth-1 overlay adds 1284 ACCENT px (threshold ≥ 400), and the `#207` characterisation that then held the LR3 key adds 0 (threshold < 120).
- Boot `load_into` over the committed corpus, with the auto-selected companion load: 13.8–16.5 ms (debug build, two runs).
- The live-corpus oracle is git, never discovery (`evidence_oracle::git_listed_files`:
  `git ls-files -z --cached --others -- evidence`), pinned by
  `git_listed_files_lists_committed_deleted_untracked_and_ignored` and, against a partial listing,
  `git_listed_files_contains_every_indexed_contract_report`.
- The rendered-pixel proof (AD-10), each test with the PNG it writes:
  `reports_boot_autoselects_newest_git_listed_companion_and_paints_it`
  (`/tmp/reports_boot_autoselect.png`); `reports_container_depth_hermetic_pair_depth_1_paints` /
  `reports_container_depth_hermetic_pair_depth_2_is_empty`
  (`/tmp/reports_container_depth_{1,2}_fixture.png`); `reports_switch_to_no_companion_is_empty`
  (`/tmp/reports_no_companion_empty.png`);
  `lab_curve_hydrated_from_container_depth_1_evidence_report_renders` /
  `lab_curve_from_container_depth_2_evidence_report_does_not_resolve`
  (`/tmp/lab_container_depth_{1,2}_evidence_overlay.png`); and LR3
  (`/tmp/lab_207_committed_overlay.png`).
- Each probe's edit and its red and green sets: the T-T4 tables named under § Context.

## Changelog

- 2026-11-22 (orchestrator, bug-log `#206` fix): **Compare's scan walks D8.**
  `compare::cache::scan_one_root` walked only `<root>/<x>/reports/`, so on `evidence/` it
  considered 108 of the 197 in-contract reports git lists; it now walks `layout::report_dirs` and
  `layout::backtest_files`, and the 2026-11-15 amendment's item 1, D1's premise, the order rule
  and the readers list say so. Compare thereby also takes D8's lowercase-`.md` rule and its
  `reports`-is-never-a-container rule (no committed file is affected). On `evidence/` the scan
  discovers 197 reports, not 108 (174 admitted; 2 `MissingScenario`, 21 `UnknownScenario`), with
  the same 66 cell keys; one winner moves, at `(sma_crossover, BTCUSDT)`, a row no build renders,
  and neither its old nor its new report has a companion equity CSV; the fixtures build's Compare
  frame measured byte-identical before and after.
  After the fix the whole `cargo test -p ui` suite passed (1405 passed, 0 failed; lib
  702), with no render baseline moved.
  D8 gains the reader the amendment named; D5's root order and identical-filename rule are
  unchanged, no decision changes, and no ADR is added. Anchor-neutral, 119/119.
- 2026-11-16 (orchestrator, bug-log `#208` fix): **the 2026-11-15 amendment's precedence item 1
  quotes the root loop as it now reads.** The Lab's `EquityCache` caches each tuple's outcome, a
  miss as well as a hit, and `get_or_load_roots` leaves its root loop with `break;`, so the quoted
  `return Ok(arc);` is gone; `lab::view` reads the roots from `Cockpit::lab_report_roots`,
  resolved once from `default_report_roots()`. `state::update` clears the whole cache on every
  `Message::LabRunCompleted`, Ok and Err, so a tuple already looked up sees a report written
  outside the app at the next completed Lab run or a restart (a tuple's first lookup sees it). Cache lifetime is not a decision of this ADR: D5's root order
  and the precedence are unchanged, and no ADR is added. Anchor-neutral, 119/119.
- 2026-11-16 (orchestrator, bug-log `#207` fix): **the 2026-11-15 amendment's "What the Lab draws
  from committed evidence" now states the fixed gutter rule.** The chart reserves its right equity
  gutter only when an overlay draws (`widgets::chart::equity_axis_shown`), so a committed
  start/end-only report over price bars leaves the frame byte-identical to the bars-only frame;
  the empty gutter the amendment recorded is gone. LR3 now names
  `lab_committed_start_end_only_report_over_bars_is_pixel_identical_to_bars_only`, which replaced
  the `#207` characterisation; the Verification list's LR3 figure is that characterisation's. No
  decision changes. Anchor-neutral, 119/119.
- 2026-11-15 (orchestrator, bug-log `#205` amendment): **the Reports picker and the Lab walk
  container depth ≤ 1.** § D1's "identical to the `spec/` layout, so both loaders read it with only
  a ROOT change" has been false since `1395520629dc` moved 68 of the 125 committed reports to
  container depth 1, and the two one-level walks read none of them. **D8** is ONE layout contract,
  `crates/ui/src/reports/layout.rs`: `<root>[/<container>]/<slug>/reports/backtest-*.md`,
  container depth ≤ `MAX_CONTAINER_DEPTH` = 1, `reports` never a container or a slug, the slug is
  the leaf dir name, dirs in (container depth, path) order, which decides only an equal-score,
  same-tier Lab tie. It binds the Reports picker and the Lab's lookup over both roots; Compare
  (`#206`), the backtest CLI (`#210`) and the anchor gate are outside it. The Lab's precedence
  (the first root with a match, then the first strictly-best score in D8 order, so the oldest stamp
  wins a tie) diverges from § D5's "most-recent-wins (`generated:`)" and is handed off to `#209`;
  § D5 is not rewritten. D1's slug-match sentence is handed off to `#212`. A committed start/end-only
  report draws only on the bar-less path. D1's home, D2–D4, D5's root order, D6 and D7 are
  unchanged. Anchor-neutral, 119/119.
- 2026-06-12 (Wave-2 amendment): the `.md` report persists the equity curve as a
  SPARKLINE only (visual, not machine-parseable). The Lab artifact therefore
  ALSO writes a **companion equity CSV** (`backtest-<stamp>-<scenario>-equity.csv`,
  schema = `reports::csv_artifacts` read/write) carrying the FULL per-bar series;
  the loader prefers it for PerBar fidelity. The H3 invariant is redefined as
  "the loader reads the companion CSV → element-by-element equals the in-memory
  series" (verified: 21601 points). Anchor-safe: the `.md` byte-format is
  unchanged and the CSV is lab-runs/-only (verify_anchors 119/119).
- 2026-06-12 (architect): initial accept. Closes the `run_scenario` Phase-C
  deferral for `lab-run-save-compare`; numbered 0055 because 0054 is burned in
  git history (the removed `Mode::Live` ADR, `live-trading-removed-2026-06-12.md`).
