---
adr: 0055
title: Lab run artifacts live in a git-ignored lab-runs/ root outside the anchor namespace
status: accepted
date: 2026-06-12
supersedes: none
superseded-by: none
updated: 2026-11-28
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
   *(2026-11-24: closed by the bug-log `#209` amendment below: the code moved, § D5 stands.)*
3. **§ D1, third sentence** ("The `<strategy-slug>` matches `equity_loader::strategy_slug`"): the
   engine's `strategy_dir_slug` maps ids that `strategy_slug` does not. Handed off to bug-log
   `#212`; D8 does not depend on it. *(2026-11-24: closed by the bug-log `#212` amendment below:
   one map, which the Lab calls.)*

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
either container depth. *(2026-11-24, bug-log `#209`: for the Lab it is now, as for Compare, an
identical filename (the copy walked first is kept and a later same-named copy is skipped before it
is read) or an equal (tier, range score, `generated:`) tie inside one root; between distinct
filenames a strictly newer `generated:` wins at either container depth.)* For Compare (since bug-log `#206`) it is an identical filename, or an
equal `generated:` stamp for one tuple, inside one root; between distinct filenames a strictly
newer `generated:` still wins at either container depth. On such a tie container depth 0 wins:
- It is where the one-level walk looked, so on such a tie the pick stays the report that walk
  would have found (for example a backtest-CLI twin at `evidence/<feature>/reports/`).
  *(2026-11-24, bug-log `#210`: since ADR-0104 a default backtest-CLI run writes the
  git-ignored `backtest-runs/`, so it no longer creates that twin; one left by a pre-fix run,
  or made by an explicit `--reports-dir evidence/<feature>/reports`, still can.)*
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
  - *Amended 2026-11-24 (bug-log `#210` fix, ADR-0104).* The CLI no longer writes under
    `evidence/`: its default is the git-ignored `backtest-runs/<feature>/reports/`
    (`backtest::cli_paths::report_dir_for_scenario`), and `--reports-dir` is the deliberate way to
    emit into the corpus. Its baseline lookup (`cli_paths::resolve_baseline`) reads the committed
    corpus at container depth ≤ `cli_paths::CORPUS_MAX_CONTAINER_DEPTH`, which a `ui` lib test
    (LY-210) pins equal to `layout::MAX_CONTAINER_DEPTH`, and refuses the run on a miss. A
    default run of the fixed CLI no longer creates such a twin. One left by a pre-fix run, or made
    by an explicit `--reports-dir evidence/<feature>/reports`, still has the "listed by the
    Reports picker / wins a Lab tie" effects above. The fixed CLI detects only a `v0-paper-sma`
    twin (`TwoCorpusHomes` on its four baseline scenarios); a twin of any other feature is not
    detected. No D8 decision changes.
- **The anchor gate:** `scripts/verify_anchors.sh` is not a `ui` reader. It `find`s
  `*/reports/backtest-*-<scenario>.md` at any depth under `evidence/`, and D8 does not bind it.

**The Lab's precedence (what the code does; it diverges from § D5).** *(2026-11-24: items 1–3
describe the code before the bug-log `#209` fix, and item 2's quoted line before `#212` (since then
`fit.score() > s.score()`); its amendment below supersedes them.)* The hot path is unchanged:
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
bug-log `#209` adds that pin with its fix. *(2026-11-24: the bug-log `#209` amendment below
renames the two order-rule pins (E4b, E4) and re-derives the item-1 pins
`route_overlay_two_root_union_lab_runs_wins_collision`, E2 and E2b on an equal `generated:`;
`default_roots_order_lab_runs_first` reads no report and is unchanged; its Pins list is current.)*

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

## Amendment 2026-11-24 (bug-log `#212`: the Lab's report match)

**What it closes.** The 2026-11-15 amendment's handoff item 3. § D1's third sentence ("The
`<strategy-slug>` matches `equity_loader::strategy_slug`") now holds by construction: there is one
map, `backtest::engine::strategy_dir_slug` (now `pub`), and `ui::lab::equity_loader::strategy_slug`
calls it (`backtest` cannot depend on `ui`, so the map lives in `backtest`). The `ui` copy mapped
four families; 5 of the 8 chips `cockpit_live` seeds (`v0.5.rsi`, `v0.5.bbands`, `v1.5a.pairs`,
`v2.5.tcn`, `v2.5.tcn.weights`) and 2 of the 3 fixtures-build ids (`btc_rsi_reversion`,
`btc_bbands_mean_revert`) fell through to a raw-id directory the engine never writes, so their
persisted runs were never read back. The chip list moved to `ui::lab::defaults::LAB_SEED_STRATEGIES`,
which the bin iterates, so the pins below iterate the production list.

**D9 — the Lab's report match.** A report answers a Lab tuple `(strategy, symbol, range, source)`
only if all of these hold. Before this amendment the match took no strategy, accepted every coin a
report did not name, and let `Last30d`, `Last90d` and `Custom` accept every report of the slug.

1. **Strategy.** `backtest::engine::report_strategy_id` maps the report's `strategy:` `id:` and the
   tuple's strategy to the same writer id, both `Some`. The map takes every report-writing dispatch
   alias of an arm, and the id that arm's writer emits, to that id: `sma_crossover`,
   `btc_macd_trend`, `btc_rsi_reversion`, `btc_bbands_mean_revert`, the signal-library `btc_*` ids,
   `top10_momentum_h1`, `pairs_mr_h1`, and the two TCN ids
   `tcn_overlay_momentum/tcn_overlay_momentum` and
   `tcn_overlay_momentum_weights/tcn_overlay_momentum`, which the TCN writers build through
   `scenarios::tcn_overlay{,_weights}::strategy_id_for` (one source). `v0.dvol_regime` writes no
   report today and maps to its own id, so a future writer needs no edit. A report with no `id:`
   answers no tuple. The long/short aliases (`v0.*_ls`) map to `None`: their reports carry the base
   arm's id, so a report cannot tell them apart from it, and no Lab chip offers them. `None` too
   for every id no arm writes a `.md` under: `v0.always_short` and `v0.buyhold` (no-op writers),
   `v0.macro_riskon`, the `v0.8.vote.*` arms, and unknown ids. The leaderboard's Inspect can still put an `_ls` id in the Lab, which runs it long-only
   (`lab_config_to_scenario` sets `short_enabled: false`); that run is written under the base arm's
   dir, id and scenario name, so it is read back under the base arm's tuple, which is what it
   computed, and never under the `_ls` tuple (REASONED).
   The `id:` is read with Compare's tolerant frontmatter reader (`compare::cache::parse_frontmatter`,
   now `pub(crate)`), which takes the engine's unindented `id:` line.
2. **Coin.** The report's `## Summary` `| Symbol |` row, else its `## Universe` list, else a
   `…USDT` ticker in its scenario name; a report naming no coin answers no tuple. Every
   single-symbol engine report and all 23 committed single-coin reports state their coin only in
   the Summary row. A cross-sectional report answers only the coins of its universe: the momentum
   and TCN arms list the Lab's 10 pairs, the pairs arm 4 (BNB, BTC, ETH, SOL). So for the Lab's
   other 6 pairs a `v1.5a.pairs` run is shown from the in-memory mirror but never read back from
   disk: D6's "in-memory equals cached disk" holds for a cross-sectional arm only on its universe's
   coins (the pairs arm ignores `cfg.pair`). That is not a regression (before this amendment the
   pairs chip read no dir at all), and it is pinned two-sided.
3. **Range.** The scenario name's span token: the first `-`/`_`-separated token that is `last30d`,
   `last90d`, `custom<start_ms>to<end_ms>` or a four-digit year (`<year>-h1`/`-h2` a half). The same
   span is an exact match (score 20). For `H1_2024` and `H2_2024` a whole-year `2024` report is a
   narrowed superset (score 10, with `narrowed_from` set from the same fit, so the badge and the
   score cannot drift). A `Custom` tuple matches only a `custom…` token equal to what the run
   mapper derives from its date strings (`lab::runner::custom_range_ms`, the mapper's own parse).
   Nothing else matches.

   **Amended 2026-11-28 (bug-log `#230`, ADR-0105).** Since bug-log `#230` the mapper reads the
   two bounds as two typed fields (`LabRunConfig.range`, which replaced the colon-joined
   `range_label`), and the same per-bound parse (`lab::runner::custom_bound_ms`, which
   `custom_range_ms` calls for each bound) also colours the Custom editor, decides its commits
   and feeds the cadence badge. The tuple a report answers is the committed range, never the
   editor's draft: a keystroke that does not commit moves no tuple, so it walks no root and
   matches no report. The `custom<start_ms>to<end_ms>` token of every pair that ran before is
   unchanged.
4. **Source.** The tiers are unchanged: a report tagged with the tuple's `data_source:` beats an
   untagged legacy one; a differently tagged one never matches.

**What evidence/ still answers (REASONED from the corpus table in bug-log `#212`).** The Lab
reaches only the 9 synthetic-tagged committed reports (the 18 `real (Binance Vision)` ones are
excluded by source, unchanged). Of those, only two answer a Lab range: v1 momentum
`top10-2024-h1-momentum` (its 10 coins, `H1_2024`) and, newly reachable because `v1.5a.pairs` now
reads `v15a-mean-reversion-pairs`, `pairs-2024-h1-zscore-mr` (BNB, BTC, ETH, SOL, `H1_2024`). The
2023 reports answer nothing, because no Lab range is a 2023 year. The committed TCN reports sit in
`evidence/v1/v25-tcn-overlay/`, a dir no chip maps to. Both reachable reports are start/end-only,
so neither draws over price bars (bug-log `#207`); on the bar-less path the pairs `H1_2024` tuple
now draws a start→end line. **No committed report answers `Last30d`, `Last90d` or `Custom`**: a
report records no span beyond its scenario name, and an engine `Last30d` run is not a 2023
full-year run. Only persisted Lab runs answer those ranges.

**D4 note.** The four cross-sectional arms' Lab scenario names gain range and source tokens
(`engine::lab_universe_scenario_name`: `v1.momentum-<range>-<source>`, and `v1.5a.pairs`,
`v2.5.tcn_overlay`, `v2.5.tcn_overlay_weights`), as the single-symbol arms' did on 2026-07-26
(`lab_scenario_name`). Until now they wrote a fixed name with no range, so an `H1_2024` run of
them could not be read back and any `Last30d`, `Last90d` or `Custom` tuple read whichever such run
came first. The `<arm>` prefix is kept, so Compare's `scenario_universe` still rejects these names.
Only the Lab write seam (`write_report = true`, set by `cockpit_live` alone) uses them: CLI bytes
and anchored bodies are unchanged, 119/119.

**What the operator sees change.** The default Lab range (`Last90d`) no longer shows any committed
curve, and a `lab-runs/` run of another coin, strategy or range no longer draws under the tuple's
label. A Lab run of `v0.5.rsi`, `v0.5.bbands`, `v1.5a.pairs` or `v2.5.tcn` is now read back after
a restart. The operator's existing `lab-runs/` reports named `btc-2023-1m-*` (written before
2026-07-26) and the range-less cross-sectional runs never answer again; they age out through the
200-per-dir cap. Over price bars the committed frames do not move.

**Residuals.** A pre-1970 `Custom` bound writes a `-` inside its token, so such a report reads as no
span. The scenario-ticker fallback lists the Lab's 10 pairs only (no ARB, ATOM, NEAR, OP, SUI); it
decides nothing today, because every engine and committed report has a Summary row or a Universe.

**Pins.** `lab::equity_loader` lib tests M1
`a_coin_named_only_in_the_summary_row_matches_that_coin_only`, M2
`a_multi_symbol_report_matches_only_a_coin_in_its_universe`, M3
`a_report_naming_no_coin_matches_no_coin`, M4 `a_report_matches_only_its_own_strategy`, M5
`a_report_without_a_strategy_id_matches_no_strategy`, M6 `range_tokens_match_exactly`, M7 (E4c,
re-derived on `H1_2024`) `a_strictly_better_report_in_the_second_report_dir_wins`, M8
`every_lab_chip_reads_the_dir_the_engine_writes`, and `strategy_slug_mapping`; `backtest::engine`
`strategy_dir_slug_known_ids`, `report_strategy_id_is_what_each_writer_emits`,
`cross_sectional_lab_scenario_names_carry_range_and_source`; end to end
`lab_run_engine.rs::inner::every_lab_chip_run_is_read_back_from_its_write_root` (and H3, which
stays green only because of the cross-sectional names); pixels (AD-10)
`lab_buy_sell_overlay_render.rs` LR-212a `lab_draws_its_tuples_strategy_not_a_sibling_in_its_dir`
and LR-212b `lab_draws_no_report_of_another_coin`.

**Verification (2026-11-24).**

- `cargo test -p ui --lib`: 725 passed (718 + M1–M6 and M8; M7 is E4c re-derived);
  `cargo test -p backtest --lib`: 264 passed, 8 ignored (262 + 2);
  `--test lab_run_engine` 2 (H3 + the end-to-end chip test), `--test lab_buy_sell_overlay_render`
  9 (7 + LR-212a, LR-212b), `--test live_equity_render` 31, `--test lab_binance_persist_compare` 2
  (no edit: its engine report is `btcusdt-2024-h1-sma-cross-binance`, `sma_crossover`, Summary
  BTCUSDT). The whole `cargo test -p ui --no-fail-fast`: 124 test binaries and the doc-tests,
  1508 passed, 0 failed, 32 ignored, among them `visual_snapshots` 55, `render_snapshots` 14 (15 ignored),
  `panel_snapshots` 113 and `gallery_snapshots` 6 (3 ignored), with no baseline file moved;
  `cargo check -p ui --no-default-features --features fixtures` builds;
  `scripts/ci_test_target_counts.py --os macos` PASS (17/17, `lab_run_engine` now 2).
- Pixel margins (macOS): LR-212a F_both vs F_macd 0 px, F_bb vs F_none 0 px, controls F_swap vs
  F_macd 4,338 px and F_macd vs F_none 19,025 px (floor 1,000 each); LR-212b F_eth vs F_none 0 px, control right-edge shift 40 px
  (band [34, 44]). LR-207d, LR-208, LR-208e and LR-211d now render `H1_2024` and keep their
  margins (40 px, 40 px, 40 px, 0 px with control 19,025 px).
- Thirteen probes R1–R13, each applied once and restored byte-identical, the red set compared by
  name (R13, the year inside `Span::Half`, was added after review: M6 holds a 2023 H1 and a 2025 H2
  scenario; on the `#212` tree R13 ran `cargo test -p ui --lib` only, 724 of 725, and its four
  targets were measured on the `#212` + `#209` tree, 2, 10, 31, 2, all green); the table is in the
  module doc of `crates/ui/tests/lab_buy_sell_overlay_render.rs`.
- `cargo clippy -p backtest -p ui --lib --bins --tests -- -D warnings` clean after touching every
  changed file; `bash scripts/verify_anchors.sh` `ANCHORS PASS  (119 / 119)` before and after,
  `--explain` byte-identical; `git status --porcelain -- evidence` empty.

## Amendment 2026-11-24 (bug-log `#209`: the Lab follows § D5)

**What it changes.** The Lab's cold-path pick now implements § D5's "most-recent-wins
(`generated:`)" over the union of both roots, as Compare already did. Over every matching report
of `[lab-runs/, evidence/]` the Lab keeps the greatest key **(tier, range score, `generated:`)**:
a report tagged with the tuple's `data_source:` beats an untagged one, an exact range (D9) beats
a superset, and then the newest `generated:` wins, whichever root holds it. § D5's
identical-filename rule holds by a filename set, as in Compare (`seen_filenames`), not by the key:
one set spans the walk of both roots, so the first copy of a filename in walk order (`lab-runs/`
before `evidence/`, container depth 0 before 1) is kept, and a later same-named copy is skipped
before it is read, whatever its `generated:`. An equal key goes to the root searched first
(`lab-runs/`), then to the first report in D8 order. Code: each root offers every matching
report in D8 walk order (`equity_loader::matching_candidates(tuple, root, seen)`, which skips a
name already in `seen`, before reading the file, and adds every name it walks, matching or not, as
Compare does); `equity_loader::resolve_roots` creates one `seen` set for all roots, concatenates
the roots' candidates in root order, sorts them by key, descending and stably (an equal key keeps
root order, then D8 order), and reads them in that order, the first that reads winning. So a
matching report that cannot be read falls through to the next candidate of its source tier, in
its own root as well as in the next one: a newest run that cannot be read (its `.md` cut before
its `Initial capital` or `Final equity` row, or its companion CSV present but cut or empty, which
is what a disk-full write leaves, because the engine writes the `.md` first and the CSV second)
yields to an older readable run of the tuple. The reads stop at the source tier of the
top-ranked candidate: D9 item 4's tiers are an eligibility rule (review D1: an untagged legacy
report is eligible only when no tagged report MATCHES; eligibility by match, not by readability),
so a tagged matching report that cannot be read leaves the tuple with no curve rather than
handing it to an untagged one. Inside the tier a broken exact report does fall through to a
readable superset, which the `narrowed_from` badge discloses. In the Lab a companion CSV that is
present but does not read, or holds no sample, makes its report unreadable
(`equity_loader::read_companion_equity_csv`); an absent one does not (older committed reports
have none). Compare's `load_companion_equity_csv` keeps reading such a CSV as no series.
`EquityCache::get_or_load_roots` is `resolve_roots` behind its cache, `load_equity` is
`resolve_roots` over one root. The per-tuple cache and its clearing (bug-log `#208`) are
unchanged.

**What it supersedes and closes.**
- The 2026-11-15 amendment's "The Lab's precedence" items 1–3 ("across roots, the first root with a
  match wins"; "within a root … the FIRST report with the best `range_score` wins"; "within one dir
  the OLDEST stamp wins a tie", "`generated:` is never read") describe the code before this
  amendment. Its handoff item 2 (§ D5's third sentence) is closed: the code moved, § D5 did not.
- D8's order rule, for the Lab, now decides only an identical filename and an equal (tier, range
  score, `generated:`) tie inside one root, as it does for Compare (`#206`): between distinct
  filenames a strictly newer `generated:` wins at either container depth.
- § D5 says the tiebreaker "decides the Lab hot-path pick". The hot path is the in-memory mirror of
  the last run, which never chooses between reports; read "cold-path pick".

**Consequences and residuals.**
- After a restart (or any chip, range or source change, which clears the mirror) the Lab shows the
  newest retained run of a tuple, not the oldest.
- After a `git pull`, a committed report newer than the operator's own Lab run of the same tuple
  now wins on the cold path, as § D5 says. The in-memory mirror still shows the run just made.
- **Same second.** `generated:` has second precision; the Lab's filename stamp has milliseconds,
  because two runs of one tuple within a second are expected (§ D5). Two such runs tie on the key,
  and the first in D8 order, the OLDER millisecond stamp, wins, as in Compare (`#206`). This is
  deliberate: breaking that tie by descending filename stamp inside one dir would show the newer
  run but make the Lab and Compare pick different reports for one tuple.
- A missing or unparseable `generated:` ranks below any dated report in the Lab (it is parsed as
  RFC 3339). Compare compares the raw strings; the two agree on the `…Z` form every writer emits.
- **The tier boundary (review round 3, ruling (b)).** The fall-through stops at the top-ranked
  candidate's source tier, so D9 item 4 stays an eligibility rule: a broken tagged report never
  yields to an untagged one, in one root or across the two (the `#212` root-by-root union let a
  broken tagged `lab-runs/` report yield to an untagged `evidence/` one). The alternative,
  "untagged wins when no tagged report matches and reads", was rejected: it would draw a
  source-unknown curve under a tuple whose honest tier holds a run. Such a miss returns the last
  root's error, as before; a last root that holds only untagged matches records none, so the
  lookup returns `NoReport`.
- **The range fall-through is kept.** Inside a tier a broken exact report yields to a readable
  superset (a whole-year `2024` report for `H1_2024`/`H2_2024`); the `narrowed_from` badge
  discloses the narrowing, so nothing is drawn under a label it does not carry.
- **The companion CSV, Lab only.** A present but unreadable or empty companion CSV makes the
  Lab's candidate unreadable, so it falls through. Compare admits the same report as its cell,
  with no overlay series, as before.
- **Residuals.** A run whose `.md` reads (complete, or cut after its `Final equity` row) but has
  NO companion CSV (the CSV write never started: the process stopped between the two writes, or
  the CSV's create failed) still reads as a start/end pair and wins on `generated:`; over price
  bars a start/end pair draws nothing (bug-log `#207`), so the tuple's older per-bar runs stay
  hidden. A companion
  CSV cut exactly at a record boundary reads as a shorter per-bar series: nothing in it says that
  records are missing.

**Pins.** `lab::equity_loader` lib tests N1-A `a_newer_generated_in_the_larger_path_wins`, N1-B
`a_newer_generated_in_the_smaller_path_wins` (it tells the newest `generated:` from the last path),
N2 `equal_score_and_generated_inside_one_dir_goes_to_the_first_path` (the 2026-11-15 amendment's
unpinned item 3, now pinned), N3 `an_exact_range_beats_a_newer_superset`, N4
`a_report_without_generated_ranks_oldest`, X1 `a_newer_evidence_report_beats_an_older_lab_runs_one`,
X2 `an_equal_generated_across_roots_goes_to_lab_runs`, X3
`a_tagged_older_evidence_report_beats_an_untagged_newer_lab_runs_one`, X4
`an_exact_evidence_report_beats_a_newer_lab_runs_superset`; every expectation is derived from the
fixture table by the rule above. Re-derived on an equal `generated:` (so they pin the root and D8
order, not recency): `route_overlay_two_root_union_lab_runs_wins_collision`, E2
`lab_runs_shadows_a_container_depth_1_evidence_report`, E2b
`container_depth_1_evidence_report_wins_when_searched_first`, E4 (renamed from
`equal_score_tie_at_one_container_depth_goes_to_the_first_dir`)
`equal_score_and_generated_tie_at_one_container_depth_goes_to_the_first_dir`, E4b (renamed from
`equal_score_tie_across_container_depths_goes_to_container_depth_0`)
`equal_generated_tie_across_container_depths_goes_to_container_depth_0` with its companion
`a_strictly_newer_generated_at_container_depth_1_wins`; and
`clear_replaces_a_stale_committed_hit_with_the_newer_lab_runs_report` gives its newer report a
newer `generated:`. The identical-filename rule, mirroring Compare's
`scan_report_roots_identical_filename_lab_runs_wins` and
`duplicate_filename_inside_a_root_goes_to_container_depth_0`: F1
`an_identical_filename_across_roots_goes_to_lab_runs` (the same name in `lab-runs/`, older, and in
`evidence/`, newer, resolves the `lab-runs/` copy; pair: distinct names, the newer wins) and F2
`an_identical_filename_inside_a_root_goes_to_container_depth_0` (the same at container depth 0
and 1 inside one root, through the union and through `load_equity`, two-sided in the container's
sort position: `a0/` and `zz/` straddle the slug, as Compare's `CONTAINERS_206`), and F4
`a_non_matching_first_copy_still_claims_its_file_name` (a first copy that is not a candidate still
claims its name, so the matching later copy is skipped and the union misses, as Compare drops it;
four variants, one per check that can reject the first copy: non-UTF-8 bytes (the read), no
`scenario:` line (the parse), a `btc-2023-1m-…` range (the match), `data_source: binance` (the
source tier); pair: distinct names, the later report resolves). The fall-through: F3
`a_broken_newer_report_falls_through_to_the_next_candidate` (a newer matching `evidence/` report
that cannot be read yields to an older `lab-runs/` one; pair: made readable, it wins), F5
`a_broken_newest_report_falls_through_to_an_older_one_in_its_root` (the same inside one root,
through the union and through `load_equity`), F7
`a_broken_exact_report_falls_through_to_a_superset_with_its_badge` (a broken exact report yields
to a readable superset, the newer report but ranked lower by its range, drawn with
`narrowed_from`; pair: readable, the exact one wins with no badge) and F8 `a_run_whose_csv_is_cut_short_falls_through_to_an_older_one` (a newer run
whose companion CSV is cut inside its last record, or empty, yields to an older complete run;
pair: with the CSV intact the newer run resolves from it, and with no CSV at all from its `.md`,
a start/end pair). The tier boundary: F6
`a_broken_tagged_report_never_yields_to_an_untagged_one` (a broken tagged report and a readable
untagged one, in one root, across the roots and the reverse: a miss with the documented error;
pair: with no tagged report the untagged one resolves). Pixels (AD-10): LR-209
`lab_draws_the_newest_run_of_its_tuple` in `crates/ui/tests/lab_buy_sell_overlay_render.rs`.

**Verification (2026-11-24, on the `#212` + `#209` tree, after review rounds 1, 2 and 3).**

- `cargo test -p ui --lib`: 743 passed (725 + N1-A, N1-B, N2, N3, N4, X1–X4, E4b's companion and
  F1–F8); `--test lab_buy_sell_overlay_render` 10 (+ LR-209); the whole
  `cargo test -p ui --no-fail-fast`: 124 test binaries and the doc-tests, 1527 passed, 0 failed, 32 ignored, no baseline
  file moved; `cargo test -p backtest --lib` 264. Every changed `.rs` was touched before each run.
- Pixel margins (macOS): LR-209 F_both vs F_new 0 px, controls F_old vs F_new 4,338 px and F_new
  vs an empty root 19,025 px (floor 1,000 each); LR-212a and LR-212b unchanged.
- Twenty-one probes S1–S17 (S3 in two forms, S12 in four), each applied once and restored
  byte-identical, the red set compared by name, every one re-run on the final code after review
  round 3; the table is in the module doc of `crates/ui/tests/lab_buy_sell_overlay_render.rs`. S8
  (the roots reversed) replaces the `#205` tables' P6, whose OLD is gone. S9 (no filename set)
  turns F1, F2 and F4 red, S10 (one set per root) F1 and F4, S11 (no fall-through) F3, F5, F7
  and F8, S12/S12b/S12c/S12d (a name claimed only after `report_matches` / only by a candidate /
  only after the parse / only after the read) F4 at three, four, two and one of its variants,
  S13 (one candidate per root, the round-1 form) F5, F7 and F8, S14 (no tier boundary) F6 alone,
  S15 (the reads stop at the range score too) F7 alone, S16 (the companion-CSV error ignored) F8
  alone, S17 (an absent CSV made an error) F8 and every test whose report has no CSV.
- `cargo clippy -p backtest -p ui --lib --bins --tests -- -D warnings` clean after touching every
  changed file, a planted lint going red in each of the 14 changed Rust files after review round 2
  and in each of the five that review round 3's code changes touched (the orchestrator's later
  edit to a `#212` file, `engine.rs:769`, a doc link made a code span, followed those gates and was
  verified by its split check: ui lib 725, clippy, fmt, anchors, spec-lint);
  `bash scripts/verify_anchors.sh` `ANCHORS PASS  (119 / 119)`, `--explain` byte-identical;
  `git status --porcelain -- evidence` empty.

## Changelog

- 2026-11-28 (orchestrator, bug-log `#230` fix): **D9 item 3 gains a dated note.** The Custom
  range reaches the mapper as two typed bounds (`LabRunConfig.range`, ADR-0105 D1), and one
  per-bound parse, `lab::runner::custom_bound_ms`, serves the mapper, the report lookup, the
  Custom editor's colour and commit rule, and the cadence badge (ADR-0105 D2). The tuple a report
  answers is the committed range, never the editor's draft (ADR-0105 D3). The match itself, the
  token and the tiers are unchanged. No other decision changes. Anchor-neutral, 119/119.
- 2026-11-24 (orchestrator, bug-log `#209` fix): **the Lab follows § D5.** The amendment
  "2026-11-24 (bug-log `#209`)" above: over the union of `lab-runs/` and `evidence/` the Lab keeps
  the greatest (tier, range score, `generated:`); an identical filename keeps its first copy
  (`lab-runs/`, then container depth 0) through a filename set, as in Compare; an equal key goes
  to the root searched first, then to the first report in D8 order; a broken report falls through
  to the next candidate of its source tier (a broken tagged report never to an untagged one), and
  in the Lab a present but unreadable or empty companion CSV makes a run broken. It supersedes the 2026-11-15 amendment's "The Lab's
  precedence" items 1–3 and closes its handoff item 2; D8's order rule for the Lab narrows to an
  identical filename and an equal-key tie, as for Compare; § D5's "hot-path pick" reads "cold-path pick". Two pins are
  renamed (E4, E4b) and five re-derived on an equal `generated:`. No other decision changes; no
  ADR added. Anchor-neutral, 119/119.
- 2026-11-24 (orchestrator, bug-log `#212` fix): **D9, the Lab's report match; one slug map.**
  The amendment "2026-11-24 (bug-log `#212`)" above adds D9: a report answers a Lab tuple only
  through its strategy (`backtest::engine::report_strategy_id` of its `strategy:` `id:`), its coin
  (Summary `| Symbol |` row, else `## Universe`, else a scenario ticker) and its scenario's span
  token (exact; a whole-year `2024` report is a narrowed superset of `H1_2024`/`H2_2024`), then
  the unchanged source tiers. § D1's third sentence now holds by construction: one slug map,
  `backtest::engine::strategy_dir_slug` (`pub`), which the Lab calls; this closes the 2026-11-15
  amendment's handoff item 3. D4 note: the four cross-sectional arms' Lab scenario names carry
  range and source. No committed report answers `Last30d`, `Last90d` or `Custom`; a
  cross-sectional run answers only its universe's coins. No other decision changes; no ADR
  added. Anchor-neutral, 119/119.
- 2026-11-24 (orchestrator, bug-log `#210` fix): **the backtest CLI leaves D8's subject.** The
  "readers outside D8" entry for the backtest CLI and the order rule's backtest-CLI-twin example
  gain dated notes: since ADR-0104 a default CLI run writes the git-ignored `backtest-runs/`, so it
  no longer creates a container-depth-0 twin (a pre-fix leftover or an explicit `--reports-dir`
  into `evidence/` still can, and the CLI detects only a `v0-paper-sma` twin), and the CLI reads
  its baselines at container depth ≤ 1 through `backtest::cli_paths` (the bound pinned equal to
  `layout::MAX_CONTAINER_DEPTH` by LY-210). No decision changes.
  Anchor-neutral, 119/119.
- 2026-11-24 (orchestrator, bug-log `#211` fix): **an overlay draws only its in-window samples,
  and a series with fewer than two of them is no overlay** (`widgets::chart::MIN_DRAWN_SAMPLES`
  = 2, one gate in `drawn_samples` for the range and the stroke). The stroke used to clamp every
  out-of-window sample onto the plot's left or right edge, at y values the range never saw, while
  one in-window sample was enough to scale the axis and reserve the gutter. Now such a series
  neither scales the axis nor reserves the gutter; the volume histogram follows through
  `equity_axis_shown`, and without bars (Compare, Lab F9) a one-sample series draws nothing
  (alone, the chart shows "No data"). So a persisted `lab-runs/` run repaints over the seeded
  bars only where two or more of its samples fall inside the bar window: an `H1_2024` run over
  the `cockpit` bin's seeded hour has one (12:00) and draws nothing. Every Lab run of v1.momentum,
  pairs and tcn, and of the minute-bar SMA arms, changes too: the engine stamps one hour per
  equity entry, one entry per merged or minute bar (`engine.rs` `synthetic_timestamps` at
  :510/:557/:593/:634), so about 90 % (top-10), 50 % (pairs) or 98 % (SMA, minute bars) of their
  samples fall after the bar window. Before the fix those samples were the right-edge smear;
  after it the chart draws only the in-window prefix, the first 1/k of the run, across the full
  window (bug-log `#226`, the engine's stamping, open). No edge point is interpolated: the curve
  starts and ends at a recorded sample. The committed-evidence outcome in the 2026-11-15
  amendment is unchanged (0 in-window samples). No decision changes. Anchor-neutral, 119/119.
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
