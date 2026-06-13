---
slug: lab-run-save-compare
status: shipped
owner: architect
updated: 2026-06-12
---

# Tasks — lab-run-save-compare

**Architect-resolved (2026-06-12).** Q1–Q6 are resolved in
[feature.md § Architecture](feature.md#architecture) (A1–A6); the governing
record is **[ADR-0055](../architecture/adr/0055-lab-run-persistence-topology-and-anchor-safety.md)**
(Lab run persistence topology & anchor-safety). Estimate **M**, ≈ 55% exec
(backtest engine) / 45% UI. **Developer ‖ ui-designer run in PARALLEL once
Wave 0 lands** (the contract-pin gate below).

**Settled inputs the design MUST honor** (feature.md § Why + § Architecture):
the report template is `crates/backtest/src/report/*::write` (REUSE verbatim, do
NOT invent a new template); the determinism contract (mandatory non-zero `seed`,
byte-identical body for fixed inputs — `LAB_DEFAULT_SEED`); the `ui` crate has
**no direct sqlx and no `backtest`→`ui` dependency** (`engine::DateRange` is
duplicated for exactly this reason → the engine's `reports_dir` default is
supplied by the Lab caller, NOT by importing `ui`). The run dispatch +
`RunSummary.report_path` capture is ALREADY plumbed (`runner.rs:1132-1156`) — do
not re-plumb it.

**Two code-grounded refinements vs the analyst brief (read these before coding):**
1. **The write seam is in the DISPATCH ARMS, not the `*_to_report` helpers.** The
   `report_path: None` literals at `engine.rs:429/475/511/563` live inside the
   **pure** `*_to_report` helpers, which take only `(result, start_year)` and
   hold no `cfg`, no writer input, no path. The write goes in the arms
   (`engine.rs:648-916`): build the report → `maybe_write_report(&cfg, …)` →
   set `report_path`. (feature.md § A2.)
2. **`report::sma::write` is NOT a drop-in lift.** It has a larger signature than
   momentum/pairs (`state` + `strategy_meta` + `elapsed_secs` + an
   `SmaScenarioInput`); the four single-symbol composed arms feed it from
   `SmaComposedRunResult.state` / `.strategy_meta` (exactly as `main.rs:2109-2110`).
   `elapsed_secs` is frontmatter-only and MAY be `0.0`. (feature.md § A2.1.)

**Project-law reminders (binding):**
- **Anchored reports are byte-immutable AND the anchor verifier must stay
  119/119** (the core constraint). Lab reports live OUTSIDE every
  `spec/**/reports/` glob (Q1=(a) `lab-runs/`, ADR-0055 § D1/D2);
  `scripts/verify_anchors.sh:88` resolves anchors via
  `find spec -path "*/reports/backtest-*-<scenario>.md"` — a sibling `lab-runs/`
  is invisible by construction. **AC7 is a mechanical proof, not a judgement.**
- Money is `Decimal` / `Money<Usdt>`, never `f64` (the writers already comply;
  Sharpe/drawdown are display-only `f64` per ADR-0003).
- Every external I/O behind an injectable seam (R3 — the `reports_dir` override);
  tests point the write at a tempdir.
- Determinism: byte-identical report **body** for a fixed seed (the H3 property;
  `generated:` / `wall_clock_s:` are frontmatter, stripped before hashing).
- NO live/paper trading — real-data **backtesting** only. No strategy / sizing
  math on the shipped path → the **baseline-equity-divergence e2e gate does NOT
  apply** (feature.md § A6; AC6/H3 IS the persisted-vs-in-memory divergence
  guarantee).
- Any Lab/Compare UI change is verified at the **render layer** (the
  `crates/ui/tests/live_equity_render.rs` pattern), not only the model layer.

> **ARCHITECT GATE — DONE.** Q1–Q6 → A1–A6 + ADR-0055 (registered,
> `adr_registry_check.py` exit 0). feature.md § Architecture carries the seam
> map. No code lands before reading § Architecture + the two refinements above.

---

## Wave 0 — Pin the contracts FIRST (blocks the parallel split)

Two small, mergeable tasks define the exec write-seam and the UI loader-root
contract the two tracks code against. Land both, then parallelize.

- [x] **T1 — `reports_dir` seam + `maybe_write_report` in `run_scenario`
  (`crates/backtest`).** Add the anchor-additive `reports_dir: Option<PathBuf>`
  field to `ScenarioConfig` (`engine.rs:192`; default-`None`, every existing
  constructor uses struct-update / names the field). Add the thin seam
  `maybe_write_report(&cfg, &scenario_meta, <writer-closure>) -> Result<Option<PathBuf>, RunError>`:
  when `cfg.write_report`, resolve dir (`cfg.reports_dir` or the workspace-root
  `lab-runs/` default) → `<dir>/<slug>/reports/` → `create_dir_all` → build a
  **millisecond** filename stamp (Q3 — NOT the CLI's second precision) → invoke
  the writer → return `Some(path)`; when `!cfg.write_report` return `None`, touch
  no FS. Wire it into the **`v1.momentum` arm only** for this wave (the H3 arm),
  setting `report_path = maybe_write_report(...)?`. The new field MUST be
  anchor-additive (the `ScenarioDataSource` precedent, `engine.rs:168`) so the 34
  anchor-generating constructors are byte-safe. _acceptance: **AC1** —
  `write_report=true` (momentum) → file exists + `Some(path)`; `write_report=false`
  → no file + `None`. **Gate: `cargo check -p backtest` + `cargo test -p backtest
  --lib`; `scripts/verify_anchors.sh` → 119/119 (the field is additive).**_
- [x] **T2 — Lab-runs root helper + two-root loader contract (`crates/ui`).**
  Add `default_lab_runs_root()` (sibling of `default_spec_root()`,
  `equity_loader.rs:636` — returns `<workspace>/lab-runs`) and generalize
  `route_equity_overlay` / `discover_reports` / `load_equity`
  (`equity_loader.rs:174,668`) + `compare::cache::scan_spec_tree`
  (`compare/cache.rs:300`) to accept a **`&[PathBuf]` root slice** searched in
  order (Q4 default: `[default_lab_runs_root(), default_spec_root()]`, lab-runs
  FIRST). Keep the per-slug `<root>/<slug>/reports/backtest-*.md` shape. This is
  the model-layer half of the UI contract; it does NOT yet write or render.
  _acceptance: the loaders resolve a report placed in a `lab-runs/` tempdir AND a
  `spec/` tempdir (lab-runs wins on tuple collision); existing `spec/`-rooted
  tests still pass. **Gate: `cargo test -p ui --lib`.**_

> **PARALLELIZATION GATE.** After T1 + T2 merge, the **Exec track (T3)** and the
> **UI track (T4–T5)** have no ordering dependency and run concurrently
> (developer ‖ ui-designer, per AGENT.md). They reconverge at the tester wave
> (T6–T8).

---

## Exec track (developer) — parallel after Wave 0; resolves A1/A2

- [x] **T3 — Wire all remaining Lab-reachable arms + retention + the default
  write target.** Complete `maybe_write_report` for every `run_scenario`
  dispatch arm the Lab can reach beyond momentum (T1): the four single-symbol
  composed arms (`v0.sma` / `v0.5.macd` / `v0.5.rsi` / `v0.5.bbands` →
  `report::sma::write`, fed `&result.state` + `&result.strategy_meta` +
  `result.final_equity`, `elapsed_secs = 0.0`, `rev_sha` = `None` synthetic /
  Yahoo SHA when `data_source = YahooCache` — feature.md § A2.1), pairs
  (`report::pairs::write`), and TCN (`report::tcn_overlay::write`, `rev_sha="n/a"`,
  `loaded_info=None`). Default the write target to
  `default_lab_runs_root()/<strategy-slug>/reports/backtest-<ms-stamp>-<scenario>.md`.
  Add the Q5 retention purge (keep last **N = 20** per tuple, purge on completion).
  _acceptance: **AC2** — the Lab-written body is byte-identical to the CLI
  `report::*::write` output for the same fixed inputs (compare frontmatter-stripped
  bodies, or re-hash with `scripts/hash_report.py`); **AC3** — a real-Binance
  single-symbol run (`v0.sma × BTCUSDT × 2023`) produces a non-empty equity series
  + a persisted report; **AC8** — after > N runs of one tuple only the last N
  remain. **Gate: `cargo test -p backtest`; CLI anchor paths unaffected
  (`write_report=false`); `scripts/verify_anchors.sh` → 119/119.**_

---

## UI track (ui-designer) — parallel after Wave 0; resolves A4 (loaders) / R5 (Compare)

- [x] **T4 — Lab history repaints from the Lab-runs home (R4).** Point the Lab
  cold-path loader (`route_equity_overlay` → `EquityCache::get_or_load`,
  `lab.rs:594-597`) at the two-root union (`[default_lab_runs_root(),
  default_spec_root()]`, Q4). After a run persists, the curve repaints from disk
  on the next boot / tuple-select even when the in-memory `last_run_report`
  mirror is absent. No new history *screen* (Q6 minimal). The hot-path in-memory
  mirror (step 1, `equity_loader.rs:675`) is unchanged. _acceptance: **AC4** —
  with the in-memory mirror cleared and one persisted report for the active tuple
  in a `lab-runs/` tempdir, `route_equity_overlay` returns the parsed series via
  the cold `EquityCache` path. **Render-layer verification (see T7): a
  hydrated-from-`lab-runs/` Lab equity curve rasterizes a non-empty polyline.**
  **Gate: `cargo test -p ui --lib`.**_
- [x] **T5 — Compare diffs two persisted Lab runs (R5).** Point
  `compare::cache::scan_spec_tree` at the two-root union (Q4); feed the resulting
  `CachedCell`s into the EXISTING Compare KPI matrix + equity-overlay widget (two
  runs side-by-side: return / Sharpe / max-DD / trade count + both curves on one
  chart). No new compare math. _acceptance: **AC5** — over a `lab-runs/` tempdir
  with two persisted reports, two `CachedCell`s are built with KPIs parsed and
  both equity series loadable for the overlay. **Render-layer verification (see
  T7): a Compare overlay of two runs rasterizes BOTH series.** **Gate:
  `cargo test -p ui --lib` + the render-layer check in T7.**_

---

## Tester wave (reconvergence) — after both tracks

- [x] **T6 — H3 flips skip → real pass (THE headline gate, R6/AC6 — its own
  named exec task).** In `crates/ui/tests/lab_run_engine.rs`: set
  `test_config(tmp_dir)` to `reports_dir: Some(tmp_dir.to_path_buf())` (it
  currently ignores `_tmp_dir`, line 41) so the engine writes
  `<tmp_dir>/<slug>/reports/…`; the existing `report_path.parent().parent().parent()`
  derivation (lines 117-121) yields the same `tmp_dir` the cache reads — pinning
  **write-root == read-root** (ADR-0055 § D6). Remove/neutralize the
  `report_path=None` early-return (lines 110-116); the stale `NotImplemented`
  skip (lines 77-84) is already unreachable — leave or remove. Assert
  element-by-element `in_memory == cached_disk`. _acceptance: **AC6** — H3 reaches
  the assertions and passes. **Gate: `cargo test -p ui --features live --test
  lab_run_engine`.**_
- [x] **T7 — Render-layer verification of the Lab repaint + Compare overlay.**
  Extend / add a render harness in the `crates/ui/tests/live_equity_render.rs`
  style for: (a) the Lab repaint-from-disk curve — a cockpit hydrated from a
  `lab-runs/` tempdir report renders a **non-empty** equity polyline; (b) the
  Compare two-run overlay — two persisted runs render **both** series at the
  pixel layer. Model-Ready is necessary but NOT sufficient (project law —
  MEMORY.md "verify UI at the render layer"). _acceptance: the hydrated-from-disk
  Lab curve and the Compare overlay draw at the pixel layer. **Gate: the render
  test(s).**_
- [x] **T8 — Anchor-safety + fixtures-smoke + full report.** Write a Lab report
  to the Lab-runs home, then run `scripts/verify_anchors.sh` → **119/119 PASS**
  (AC7 — MANDATORY, the core constraint: prove the Lab-runs home is outside every
  `spec/**/reports/` glob, no `anchors.toml` row added, no body-SHA mutated);
  confirm the fixtures-mode cockpit smoke is byte-unchanged (AC9); confirm every
  write is behind the R3 `reports_dir` seam and no new dep landed (AC9). Emit the
  tester report per the rust-test template; for any > 2 min cargo/backtest job
  include a copy-pasteable `watch -n N '<probe>'` block (MEMORY.md). _Gate:
  `verify_anchors.sh` 119/119; `cargo test -p ui` fixtures smoke green;
  `cargo clippy -p backtest -p ui -- -D warnings`._

---

## Gates summary (all must pass before VERDICT → PASS)

- `cargo test -p backtest` (engine write + byte-identical body) — AC1/AC2/AC3/AC8
- `cargo test -p ui --features live --test lab_run_engine` (H3 skip → pass) — AC6
- `cargo test -p ui` (loader two-root plumbing, Compare two-run, fixtures smoke) — AC4/AC5/AC9
- the render-layer Lab/Compare test(s) — AC4/AC5 pixel proof (T7)
- **`scripts/verify_anchors.sh` → 119/119 after a Lab write — AC7 (THE core constraint, MANDATORY)**
- `scripts/check_determinism_anchors.py` (System 2 == System 1, unchanged) — anchor neutrality
- `cargo clippy -p backtest -p ui -- -D warnings`
- `python3 scripts/spec_lint.py` ≤ 70 zero-new + `--self-test` PASS
- `python3 scripts/adr_registry_check.py` exit 0 (ADR-0055 registered)

---

## Appendix — trace.toml row (orchestrator applies)

The `[[req]]` row exists (`state = "proposed"`). On arch-done the architect flips
it to `state = "arch-done"` and appends ADR-0055 to its `arch` list (scoped: that
row only) — applied via `spec-update` by the orchestrator:

```toml
[[req]]
id    = "REQ-LAB-RUN-SAVE-COMPARE-001"
arch  = ["spec/architecture/adr/0055-lab-run-persistence-topology-and-anchor-safety.md"]
state = "arch-done"
```
