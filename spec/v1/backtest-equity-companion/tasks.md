---
slug: backtest-equity-companion
status: dev-done
owner: ui-designer
updated: 2026-06-18
version: 0.1.0
---

# Tasks — backtest-equity-companion v0.1.0

Ordered checklist for the developer. Each task maps to acceptance criteria
in `feature.md`. The whole change is **additive** — no markdown report body
is edited; `scripts/verify_anchors.sh` must stay **119/119** throughout.

## M-DEV — emit the companion at the CLI seam

- [x] **M-DEV-1 — Promote the companion writer to a shared `pub(crate)` fn.**
  - file: `crates/backtest/src/report/mod.rs:46` (`pub fn write_equity_companion`); `crates/backtest/src/engine.rs:402` (`pub(crate) fn synthetic_timestamps`).
  - Note: implemented as `pub` not `pub(crate)` — the binary is a separate crate from the library so `pub(crate)` is not visible from `main.rs`.
  - test: `cargo test -p backtest --test equity_companion_roundtrip`
  - output: `test result: ok. 3 passed; 0 failed; 0 ignored`
  (AC1, AC5) In `crates/backtest/src/report/mod.rs`, add
  `pub(crate) fn write_equity_companion(report_path: &Path,
  equity_curve: &[Decimal], start_year: i32) -> std::io::Result<()>`.
  - Resolve `artifacts_dir = report_path.parent()/"artifacts"/<report file stem>`
    where the stem is `report_path.file_stem()` (= `backtest-<stamp>-<scenario>`).
  - Resolve `csv_path = artifacts_dir / format!("equity-{stamp}.csv")` — derive
    `<stamp>` from the report file stem (split on the scenario suffix) OR reuse
    the same `OffsetDateTime::now_utc()` stamp the caller already formatted and
    pass it in; either is fine as long as the filename `starts_with("equity-")`
    and ends `.csv` (the loader's match).
  - `create_dir_all(&artifacts_dir)`.
  - Build the CSV with header
    `ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`
    and one row per bar: `ts` = `engine::synthetic_timestamps(start_year,
    equity_curve.len())[i]` formatted RFC3339; `equity_total_usdt` =
    `equity_curve[i].to_string()`; the other three columns = `0`. Reuse the
    exact body of `engine.rs::write_equity_companion_csv` @791 (Decimal→string,
    never `f64`; honest `0` for the untracked columns) so the two paths share
    one implementation — move that fn here and have `engine.rs` call the shared
    one, OR factor the row-formatting into a shared helper. **Do not introduce
    `f64` anywhere in the amount columns.**
  - `synthetic_timestamps` is currently private to `engine.rs`; expose it
    `pub(crate)` (it is pure + deterministic) so `report/mod.rs` can build the
    `ts` column identically to the in-memory `RunReport.equity_series`.

- [x] **M-DEV-2 — Call the helper at all five CLI emit seams.** (AC1) In
  - file: `crates/backtest/src/main.rs` — lines ~1405 (momentum), ~1475 (pairs), ~1580 (TCN overlay), ~1700 (TCN weights), ~1818 (PatchTST), ~1937 (GARCH vol-target), ~2020 (regime), ~2161 (SMA/composed).
  - test: `cargo test -p backtest --test backtest_sharpe_emit_equity_bin` (smoke test exercises the SMA seam)
  - output: `test result: ok. 3 passed; 0 failed`
  `crates/backtest/src/main.rs`, immediately after each successful
  `report::<family>::write(...)` call, add one line:
  `backtest::report::write_equity_companion(&report_path, &result.equity_curve, scenario_start_year)?;`
  Seams (line numbers approximate — match on the `report::*::write` call):
  - SMA/composed `~2142` (`report::sma::write`) — uses `result.equity_curve`.
  - momentum `~1404` (`report::momentum::write`).
  - pairs `~1474` (`report::pairs::write`).
  - tcn / weights `~1572 / 1690 / 1807 / 1923` (`report::tcn_overlay::write`).
  - regime `~2009` (`report::regime_dispatcher::write`).
  Use the same `start_year` each family already passes to its scenario input /
  `synthetic_timestamps`. **Do NOT modify any `report::<family>::write`
  signature or its `build_content` body** — the companion is a sibling call so
  the anchored `.md` bodies stay byte-identical.

- [x] **M-DEV-3 — Confirm zero body drift locally.** (AC4) Run
  - file: N/A (gate verification).
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (119 / 119)` — both before and after demo run (new `.md` from demo deleted because it uses real data vs the synthetic data the anchor was made with; the companion CSV is retained).
  `scripts/verify_anchors.sh` after wiring; it must print **119/119**. If any
  anchor flips, a body writer was touched — revert and re-do M-DEV-2 as a pure
  sibling call. (The companion `.csv` is invisible to the gate by construction;
  a flip means an accidental `.md` edit.)

- [x] **M-DEV-4 — Build + lint gate.** (AC5/AC6) `cargo build -p backtest`
  - file: `crates/backtest/Cargo.toml` (`reports` under `[dev-dependencies]` only, confirmed no `[dependencies]` entry).
  - test: `cargo build -p backtest && cargo clippy -p backtest --lib --tests --bins -- -D warnings && cargo fmt -p backtest --check`
  - output: all clean (`Finished dev profile`, no warnings, no fmt diffs).
  and `cargo clippy -p backtest -- -D warnings` clean. Confirm NO `reports`
  entry was added to `crates/backtest/Cargo.toml` `[dependencies]` (only the
  `[dev-dependencies]` edge from M-TEST-1 is allowed).

## M-TEST — prove round-trip + render + anchor neutrality

- [x] **M-TEST-1 — Round-trip unit test against the canonical reader.**
  - file: `crates/backtest/tests/equity_companion_roundtrip.rs` (3 tests: `equity_companion_roundtrip_basic`, `equity_companion_roundtrip_empty_curve`, `equity_companion_path_layout`).
  - test: `cargo test -p backtest --test equity_companion_roundtrip`
  - output: `test result: ok. 3 passed; 0 failed; 0 ignored`
  (AC2, AC5) Add a test (e.g. `crates/backtest/tests/equity_companion_roundtrip.rs`
  or a `#[cfg(test)]` mod in `report/mod.rs`) that:
  1. builds a small `equity_curve: Vec<Decimal>` (≥3 bars) + a `start_year`;
  2. calls `report::write_equity_companion` into a `tempfile::TempDir`-rooted
     fake `reports/backtest-<stamp>-fixture.md` path;
  3. locates the emitted `artifacts/<stem>/equity-*.csv`;
  4. reads it with **`reports::csv_artifacts::read_equity_csv`** and asserts
     `samples.len() == equity_curve.len()`, each `sample.equity_total` equals
     the input, each `sample.ts` round-trips, and the three P&L columns are
     `Decimal::ZERO`.
  Add `reports = { path = "../reports" }` to `crates/backtest/Cargo.toml`
  **`[dev-dependencies]` only** (test-only edge; justified by AC2). Verify it
  does not introduce a cycle (`reports` has no dev-dep back on `backtest`).

- [x] **M-TEST-2 — Anchor-neutrality assertion in CI-able form.** (AC4) Run
  - file: N/A.
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (119 / 119)`
  `scripts/verify_anchors.sh` and capture **119/119**. (This is the gate the
  tester re-runs before VERDICT → PASS — do not trust a cached result; re-run.)

- [x] **M-TEST-3 — Render the demo report (human-verification recipe).**
  - Status (ui-designer, 2026-06-17): the original `btc-2023-1m-sma-cross`
    companion was NOT committable (anchored scenario + real-data body drift →
    `.md` discarded → companion not retained). Replaced with a **committable
    non-anchored** demo:
    - Report: `spec/v0-paper-sma/reports/backtest-20260617-180015-btc-2024-h1-sma-cross.md`
    - Companion: `spec/v0-paper-sma/reports/artifacts/backtest-20260617-180015-btc-2024-h1-sma-cross/equity-20260617-180015.csv` (17 544 rows + header).
    The loader resolves this pair to `PanelState::Ready` — proven at the data
    layer by `load_equity_companion_real_demo_report_is_ready` (skip-if-absent
    unit test on the exact committed paths). The visual eyeball of the curve in
    the live cockpit Reports screen / `viewer` bin remains an orchestrator/
    human step (the live window is orchestrator-only) per the "verify UI at the
    render layer" rule — recipe below.
  (AC3) Produce ONE report-with-companion and eyeball the curve at the render
  layer (per the "verify UI at the render layer" rule — not an agent
  assertion):
  - **Command (emit, already run — committed artifacts on disk):**
    `cargo run -p backtest --bin backtest -- --scenario btc-2024-h1-sma-cross --seed 0xC0FFEE`
  - **Committed files (on disk now):**
    `spec/v0-paper-sma/reports/backtest-20260617-180015-btc-2024-h1-sma-cross.md`
    (non-anchored body, commits freely) **plus**
    `spec/v0-paper-sma/reports/artifacts/backtest-20260617-180015-btc-2024-h1-sma-cross/equity-20260617-180015.csv`.
  - **Command (render):**
    `cargo run -p ui --bin viewer -- spec/v0-paper-sma/reports/backtest-20260617-180015-btc-2024-h1-sma-cross.md`
    and/or open the cockpit Reports screen and select the
    `backtest-20260617-180015-btc-2024-h1-sma-cross` report.
  - **Steps:** confirm the equity curve + drawdown band area shows a populated
    line (not the "no equity data" empty state) — for this report only; the
    other fixture/committed reports still show Empty (they ship no companion).
  - **Timing:** the backtest run is ~0.02 s (already done); render is interactive.
  - **Failure diagnosis:** empty state still shows ⇒ either the `artifacts/`
    dir is not beside the `.md` (path bug in M-DEV-1), or the CSV header/columns
    don't match `read_equity_csv` (schema bug — re-check M-TEST-1), or the curve
    file isn't named `equity-*.csv`.
  - **Cleanup:** none — the demo report + companion are intentionally
    committable (non-anchored scenario). Keep both. (The earlier
    `btc-2023-1m-sma-cross` plan assumed a byte-identical re-emitted `.md` to
    discard; that scenario was replaced precisely because it could not commit a
    body, see M-TEST-3 status.)

- [x] **M-TEST-5 — Loader stem-match fix + tests (ui-designer).** (AC3) Fix the
  shipped `cockpit-reports-viewer` first-match bug:
  `crates/ui/src/reports/loader.rs::load_equity_companion` now resolves only
  `parent/artifacts/<report-file-stem>/equity-*.csv` (was: first match across
  any `artifacts/<subdir>/`). Empty on missing dir / no csv; never panics.
  - file: `crates/ui/src/reports/loader.rs`.
  - test: `cargo test -p ui --lib reports::loader`
  - output: `test result: ok. 13 passed; 0 failed` — incl. the new
    `load_equity_companion_{matching_stem_dir_is_ready,non_matching_stem_dir_is_empty,matching_stem_dir_no_csv_is_empty,real_demo_report_is_ready}`
    and the four pre-existing four-state tests (still green; reports-panel
    snapshots unchanged).
  - gate: `scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)`;
    `cargo test -p ui` → 860/0/27; clippy + fmt clean. No new crate edge /
    widget / theme token.

- [x] **M-TEST-4 — Spec gate.** (AC6) `python3 scripts/spec_lint.py
  spec/backtest-equity-companion` passes (valid frontmatter, no dead links).
  - file: `spec/backtest-equity-companion/feature.md` and `tasks.md`.
  - test: `python3 scripts/spec_lint.py spec/backtest-equity-companion`
  - output: `spec-lint: PASS (0 violations)`

- [x] **M-TEST-6 — Discoverability UX follow-on (ui-designer, 2026-06-18).**
  (AC3) The render-layer proof (M-TEST-5 / `## Implementation` → *Render-layer
  verification*) diagnosed that the Reports screen "looked empty" because the
  ONE companion-bearing report was buried in a ~112-row picker with a
  no-companion near-duplicate (`…20260527…`) sorting right above it — the
  operator never found the row whose curve populates. This task fixes the
  discoverability so the curve surfaces without hunting. Three changes, all in
  `crates/ui`, no new crate edge / widget / theme token:
  - **(1) `has_companion: bool` on `ReportEntry`** — `crates/ui/src/reports/state.rs:38`;
    computed per entry in `discover_reports()` via the new existence-only
    `loader::report_has_companion()` (`crates/ui/src/reports/loader.rs`), which
    reuses `load_equity_companion`'s stem-match convention but stops at
    existence (dir `is_dir()` + one `read_dir` for `equity-*.csv`; never reads/
    parses the CSV; K2 never-panic).
  - **(2) "● curve" picker marker** — `crates/ui/src/screens/reports.rs::picker_row`
    pushes a trailing `ACCENT` `REPORTS_HAS_CURVE_MARKER` tag
    (`crates/ui/src/strings.rs`) on companion rows. Colour + label (never colour
    alone — accessibility minimum). Existing `ACCENT` token, existing `Text`.
  - **(3) Boot auto-select of the newest companion-bearing report** —
    `crates/ui/src/reports/state.rs::load_into` defaults `selected` to
    `newest_companion_index()` (greatest `file_stem` among `has_companion`
    rows) and `load_selection`s it when the list first becomes `Ready`, guarded
    on `selected.is_none()` (never overrides an operator choice). No companion
    anywhere → unselected (pre-follow-on cold-start prompt).
  - file: `crates/ui/src/reports/state.rs`, `crates/ui/src/reports/loader.rs`,
    `crates/ui/src/screens/reports.rs`, `crates/ui/src/strings.rs`.
  - test: `cargo test -p ui` (incl. 7 new unit tests — `report_has_companion_*`
    ×4, `newest_companion_index_*` ×3 — and the new render-layer guard
    `reports_marker_and_autoselect_render` in
    `crates/ui/tests/reports_populated_curve_render.rs`).
  - render proof: `/tmp/reports_marker_render.png` — shows the "● curve" marker
    on the companion row (only) AND the auto-selected report's populated curve
    in the detail pane (asserts marker ACCENT px isolated to the companion row +
    >1000 curve px). Reuses the >1000-curve-px guard pattern.
  - output: `cargo test -p ui` → **873 passed; 0 failed; 27 ignored**; clippy
    (forced re-lint) + fmt clean; `verify_anchors.sh` → 119/119. The three
    reports textual snapshots (`reports_snapshot__{ready_dark,ready_light,
    detail_error_dark}`) regenerated — the ONLY delta is the new
    `marker=[● curve] color=accent` line on the companion row (confirmed via
    diff; no other field moved).

## Mapping to acceptance criteria

| AC  | Covered by                |
|-----|---------------------------|
| AC1 | M-DEV-1, M-DEV-2          |
| AC2 | M-TEST-1                  |
| AC3 | M-TEST-3, M-TEST-5, M-TEST-6 |
| AC4 | M-DEV-3, M-TEST-2         |
| AC5 | M-DEV-4, M-TEST-1         |
| AC6 | M-TEST-4                  |

## Anchor / determinism reminders

- The companion is `.csv` under `reports/artifacts/<stem>/`; `verify_anchors.sh`
  globs only `*/reports/*.md` — the companion is invisible to the gate by
  construction. A 119/119 → 118/119 flip means a `.md` body was accidentally
  edited; that is the only way this feature can break anchors.
- Amount columns are `Decimal::to_string()` — **never `f64`**. `ts` is RFC3339
  (the format `read_equity_csv` parses).
- The demo scenario is deterministic under a pinned seed; the `.md` body must
  re-emit byte-identically (the report writer is untouched).
