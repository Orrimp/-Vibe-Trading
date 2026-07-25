---
adr: 0055
title: Lab run artifacts live in a git-ignored lab-runs/ root outside the anchor namespace
status: accepted
date: 2026-06-12
supersedes: none
superseded-by: none
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

## Changelog

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
