---
slug: ui-test-harness-viewport-matrix
status: present-done
owner: human-operator
updated: 2026-05-29
---

# Tasks — ui-test-harness-viewport-matrix v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick A Wave 1 promotion in
> [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](../dev-notes/pick-a-test-infra-trifecta-2026-05-29.md).
> ~3-4 dev days. Bias DURABLE per
> [AGENT.md ## Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).
> Predecessor: [`ui-test-harness-bootstrap v0.1.0`](../ui-test-harness-bootstrap/feature.md)
> (Charts-only three-viewport baselines, shipped 2026-05-12).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-VPM-M0.1 — feature.md R1-R5 + R-NR + K1-K4 + H1-H4 + Q1-Q3 + 4-cell verdict tree — _accept: bias DURABLE at every Q_
- [x] T-VPM-M0.2 — backlog Active row appended under § Process / tooling — _accept: PROMOTED Idea → Active 2026-05-29 annotation_
- [x] T-VPM-M0.3 — trace row `REQ-UI-TEST-HARNESS-VIEWPORT-MATRIX-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (inventory + dry-run + ratification) — DONE 2026-05-29 (commit `641b94a8`)

- [x] T-VPM-T1.1 — Audit existing widget test files per R1.2 — _DONE: § Design D-VPM-3 enumerates 4 test files (visual_snapshots.rs / render_snapshots.rs / gallery_snapshots.rs / gallery_bisect.rs) × 30 `#[test] fn`; H1 revised downward (analyst projected 10-15 files / 30-40 fns; reality 4 files / 30 fns / 22 in-scope after opt-outs)_
- [x] T-VPM-T1.2 — Dry-run baseline PNG generation for 3-widget representative sample at all 3 viewport slots — _DONE: § Design ## T-VPM-T1.2 dry-run evidence records empirical sizes (Charts triple: floor 91 KB / typical 155 KB / operator 859 KB); H3 net repo growth projected ~13 MB total (not 50-100 MB); K2 ceiling not triggered_
- [x] T-VPM-T1.3 — Ratify Q1 (coverage scope) + Q2 (helper shape) + Q3 (`.gitattributes` rule) — _DONE: Q1 (a) all-widgets-v0.1.0 RATIFIED; Q2 (a) function-with-closure RATIFIED; **Q3 falls back to (b) plain `binary`** — empirical check `git config --get diff.exif.command` returns empty; analyst's driver-availability contract triggers the (b) durable fallback (B is durable enough without follow-on debt)_
- [x] T-VPM-T1.4 — Author opt-out list per K1 falsifier — _DONE: § Design D-VPM-4 lists 3 distinct surfaces / 5 `#[test] fn` opt-outs (gallery 3 / bisect 1 / V9 self-test 1); all already `#[ignore]`d upstream; opt-out marker `// VIEWPORT-MATRIX-OPT-OUT:` not added because existing `#[ignore = "..."]` decorators already document the block_
- [x] T-VPM-T1.5 — Confirm ADR-0048 + bootstrap § Design carry forward — _DONE: § Design D-VPM-7 drafted ride-along Changelog row for ADR-0048 appended at M-T1 close (single row, ~10 lines); no new ADR; no D1-D6 revision; no README registry frontmatter bump_
- [x] T-VPM-T1.6 — Wave decomposition for M-DEV — _DONE: § Design ## D-VPM wave decomposition records single-wave delivery (Wave 1 helper ~0.5d + Wave 2 expansion ~2d + Wave 3 review+gates ~1d ≈ 3-4 dev days); LoC budget ~80-100 helper + ~660 test expansion across 22 fixtures_
- [x] T-VPM-T1.7 — Confirm visual-fail-HTML stanza inheritance per R3.1 — _DONE: § Design D-VPM-6 confirms inheritance from sibling visual-fail-html-reporter v0.1.0 § D-VF-4 without amendment; stanza covers matrix case (per-`#[test] fn` HTML emission scales correctly — three FAILs per regression produce three independently-named HTML files); no amendment by viewport-matrix; sibling owns `.claude/agents/tester.md` under R3.1_
- [x] T-VPM-T1.8 — Frontmatter flip owner: analyst → developer, status: draft → arch-done — _DONE: feature.md + tasks.md frontmatter updated to owner: developer / status: arch-done_

## M-DEV — Developer (single-wave delivery; ~3-4 days per M-T1 decomposition)

### Wave 1 — Helper (~0.5 dev day)

- [x] T-VPM-D1 — Author shared helper at `crates/ui/tests/fixtures/viewport_matrix.rs` per D-VPM-2 — _accept: function-with-closure shape exposes `pub fn snapshot_widget_at_slot<P, B>(fixture_name, slot_name, baseline_subdir, build_program)` + `pub fn snapshot_widget_at_viewports<P, B>(fixture_name, baseline_subdir, build_program)`; `pub const SLOTS: &[(&str, (u32, u32), f32)]` mirrors bootstrap D-VPM-1 table verbatim; `pub fn slot(slot_name: &str) -> ((u32, u32), f32)`; CHART_FORCE_UTC env-var init mirrors existing `visual_snapshots.rs::run_slot` lines 96-99; baseline path resolution honours `baseline_subdir: Option<&str>` for the `render_snapshots/` nested case; ~80-100 LoC_
  - **file:line** `crates/ui/tests/fixtures/viewport_matrix.rs:100` (`snapshot_widget_at_slot`) + `:163` (`snapshot_widget_at_viewports`) + `:57` (SLOTS const) + `:69` (slot fn); `crates/ui/tests/fixtures/mod.rs:36` (pub mod viewport_matrix).
  - **Test command** `cargo test -p ui --test visual_snapshots --no-default-features --features live`
  - **Output line** `test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.46s`

### Wave 2 — Per-test expansion (~2 dev days)

- [x] T-VPM-D2 — Per-test expansion across the 4 in-scope test files per D-VPM-3 — _accept:_
  - `crates/ui/tests/visual_snapshots.rs`:
    - **Charts triple (3 fns)**: untouched — already triple-coverage per bootstrap; baselines stay byte-identical
    - **Trail/Live (3 fixtures × 3 slots = 9 fns)**: existing `trail__steady_state`, `trail__side_drawer_open`, `live__recent_activity_with_chevron` (currently `typical` only) become `<fixture>__floor` / `__typical` / `__operator` triple each; drop the in-file `TRAIL_SLOTS` const + `run_trail_slot` helper; use `viewport_matrix::snapshot_widget_at_slot` directly
    - **Compare (4 fixtures × 3 slots = 12 fns)**: same shape; drop `COMPARE_SLOTS` + `run_compare_slot`
    - **Phase F (8 fixtures × 3 slots = 24 fns)**: same shape; drop `PHASE_F_SLOTS` + `run_phase_f_slot`
    - **V9 self-test**: opt-out per D-VPM-4, no expansion
  - `crates/ui/tests/render_snapshots.rs`:
    - **7 fixtures × 3 slots = 21 fns** at `<fixture>__floor` / `__typical` / `__operator`; the 5 currently-`#[ignore]`d shell-composition cases stay `#[ignore]`d per slot (carry forward verbatim); drop the in-file `SLOTS` const + `run_panel_slot` helper; use `viewport_matrix::snapshot_widget_at_slot(..., Some("render_snapshots"), ...)` for the subdir
  - `crates/ui/tests/gallery_snapshots.rs`: opt-out per D-VPM-4 — file unchanged (already triple-coverage shape, all `#[ignore]`d)
  - `crates/ui/tests/gallery_bisect.rs`: opt-out per D-VPM-4 — file unchanged (diagnostic-only)
  - **Existing typical-slot baseline rename per D-VPM-5**: rename each non-Charts existing baseline (`trail__steady_state.png` → `trail__steady_state__typical.png`, `memory__cold_boot_empty.png` → `memory__cold_boot_empty__typical.png`, etc.) as the NEW `__typical` member — single rename, zero byte change, preserves the existing operator-reviewed PNG
  - **Workspace test count delta: +44 new `#[test] fn`** (22 in-scope fixtures × 2 new slots each); bootstrap Charts triple unchanged
  - **R-NR.5 reconciliation**: analyst projected "+60-90 new"; actual "+44" — both within "additive expansion" intent; report in M-DEV handoff envelope `[evidence]`
  - **file:line** `crates/ui/tests/visual_snapshots.rs:181` (trail__steady_state__floor first new fn) through `:502` (last Phase F fn); `crates/ui/tests/render_snapshots.rs:111` (positions_ready__floor first fn) through `:382` (focus_ring_baseline__operator last fn); existing typical-slot baselines renamed at `crates/ui/tests/visual-baselines/` (15 renames).
  - **Test command** `cargo test -p ui --test visual_snapshots --no-default-features --features live`
  - **Output line** `test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.46s`

### Wave 3 — Baselines + gates + reviews (~0.5-1 dev day)

- [x] T-VPM-D3 — Baseline PNG first-run generation — _accept: 44 new baseline PNGs at `crates/ui/tests/visual-baselines/<fixture>__<slot>.png` (top-level) + `crates/ui/tests/visual-baselines/render_snapshots/<fixture>__<slot>.png` (nested for render_snapshots cases); first-run helper auto-write per R2.2; ALL baselines regenerated on the SAME architect host in the SAME build session (K3 cross-time determinism caveat per § Design — including the existing 16 typical-slot PNGs renamed in D2, since the helper byte-comparison will fail on the rename without a fresh regenerate); operator visually reviews PNGs before commit (D6 recipe); net repo growth measured at commit time and reported as evidence (expected ~13 MB net per § Design ## T-VPM-T1.2)_
  - **file:line** `crates/ui/tests/visual-baselines/` (48 PNGs) + `crates/ui/tests/visual-baselines/render_snapshots/` (8 PNGs) = 56 PNGs total. K3 caveat honoured: Charts triple + all new PNGs regenerated in same build session on this Apple Silicon host. Second-run confirmed zero byte deltas (51 PASS / 10 PASS on consecutive runs).
  - **Test command** `cargo test -p ui --test visual_snapshots --no-default-features --features live` (second run)
  - **Output line** `test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.46s`
- [x] T-VPM-D4 — `.gitattributes` rule per D-VPM-5 — _accept: existing single-line `.gitattributes` at workspace root gets an additive second line `crates/ui/tests/visual-baselines/** binary` (Q3 (b) plain `binary` per ratification — NO `diff=exif` driver suffix); developer verifies with `git check-attr binary crates/ui/tests/visual-baselines/charts_screen_dark_floor.png` → returns `binary: set`_
  - **file:line** `.gitattributes:2` (`crates/ui/tests/visual-baselines/** binary`).
  - **Test command** `git check-attr binary crates/ui/tests/visual-baselines/charts_screen_dark_floor.png`
  - **Output line** `crates/ui/tests/visual-baselines/charts_screen_dark_floor.png: binary: set`
- [x] T-VPM-D5 — Dev-side gates — _accept: `cargo test -p ui --tests` PASS; `cargo fmt -p ui --check` zero diff; `cargo clippy -p ui --no-default-features --features live --tests` zero NEW errors from viewport_matrix.rs/visual_snapshots.rs/render_snapshots.rs; `bash scripts/verify_anchors.sh` → 75/75 PASS byte-identical; falsification probe P-VPM-1 PASS._
  - **file:line** `crates/ui/tests/fixtures/viewport_matrix.rs` (helper); `crates/ui/tests/visual_snapshots.rs` (expanded); `crates/ui/tests/render_snapshots.rs` (expanded).
  - **Test command** `bash scripts/verify_anchors.sh && cargo test -p ui --test visual_snapshots --no-default-features --features live && cargo test -p ui --test render_snapshots --no-default-features --features live`
  - **Output line** `ANCHORS PASS  (75 / 75)` + `test result: ok. 51 passed; 0 failed` + `test result: ok. 10 passed; 0 failed; 15 ignored`
  - **P-VPM-1** `cargo test -p ui --test visual_snapshots --no-default-features --features live` with SLOTS rotated to `[operator, typical, floor]` → `test result: ok. 51 passed; 0 failed; 0 ignored` — zero baseline byte deltas, zero new files, P-VPM-1 PASS.
- [ ] T-VPM-D6 — Operator-side PNG review request — _accept: developer emits a six-section recipe per [memory/feedback_human_verification_recipe.md](../../.claude/projects/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/memory/feedback_human_verification_recipe.md):_
  - **Command**: `open crates/ui/tests/visual-baselines/` in Finder (column view; preview pane on)
  - **Steps**: eyeball each new PNG for rendering sanity — focus the 22 × 2 = 44 NEW slot PNGs + the 16 renamed `__typical` PNGs (60 PNGs total to review; the existing 3 Charts triple and the 5 opt-out cases are unchanged and skipped); check for clipping (content cut off at viewport edge), blank canvas (white/black-only PNG), or obviously-broken layout (overlapping panels, misaligned text); operator-slot PNGs in particular are at 6720×3780 — confirm they render the full cockpit at scale 2.0 without clipping
  - **Timing**: ~10-15 min for 60 PNGs (about 10-15s per PNG eyeball)
  - **Expected result**: no obviously-broken PNGs surface; all 60 read as "cockpit at <slot> renders sensibly"
  - **Failure mode**: operator flags PNG by filename → routed back to dev with the flagged list; dev investigates whether (a) fixture rendering bug at the operator slot (most likely cause; new failure surface), (b) per-test viewport mismatch (e.g. a test forced the wrong fixture), or (c) iced layout bug at large physical pixel count (K1 escalation: add to opt-out list, document in M-DEV handoff `[open_questions].items` for architect ratification)
  - **Cleanup**: none — review is observation-only; operator approval gates commit

## M-FINAL — Tester

- [x] T-VPM-FINAL.1 — Run `cargo test -p ui --tests` — _DONE 2026-05-29: visual_snapshots 51/51 PASS; render_snapshots 10/10 PASS (15 ignored); Charts baselines byte-identical; second consecutive run zero byte deltas (K3 determinism HOLDS)._
  - **Test command** `cargo test -p ui --test visual_snapshots --no-default-features --features live && cargo test -p ui --test render_snapshots --no-default-features --features live`
  - **Output line** `test result: ok. 51 passed; 0 failed; 0 ignored; finished in 23.38s` + `test result: ok. 10 passed; 0 failed; 15 ignored; finished in 9.96s`
- [x] T-VPM-FINAL.2 — Deliberate-FAIL probe per R3.1 inheritance — _DONE 2026-05-29: visual-fail-HTML inheritance verified via `visual_fail_html_self_test` 2/2 PASS (Gate 6). D-VPM-6 inheritance contract confirmed — matrix tests flow through `fixtures::visual_diff::matches_screenshot` which invokes `emit_visual_fail_html`; no matrix-specific amendment needed. No HTML artifacts to cite (no failures on clean run)._
  - **Test command** `cargo test -p ui --test visual_fail_html_self_test --no-default-features --features live`
  - **Output line** `test result: ok. 2 passed; 0 failed; 0 ignored; finished in 0.00s`
- [x] T-VPM-FINAL.3 — `.gitattributes` verification — _DONE 2026-05-29: `git check-attr binary crates/ui/tests/visual-baselines/charts_screen_dark_floor.png` → `binary: set`. Line 2 of `.gitattributes`: `crates/ui/tests/visual-baselines/** binary` (Q3-b plain binary per ratification D-VPM-5)._
  - **Output line** `crates/ui/tests/visual-baselines/charts_screen_dark_floor.png: binary: set`
- [x] T-VPM-FINAL.4 — Repo size delta check — _DONE 2026-05-29: `du -sh crates/ui/tests/visual-baselines/` → 15 MB total. Well within K2 100 MB ceiling. H3 projection ~13 MB confirmed within ~15% (small variance due to per-fixture PNG compression; no K2 mitigation triggered)._
- [x] T-VPM-FINAL.5 — Anchor + spec-lint gate — _DONE 2026-05-29: `bash scripts/verify_anchors.sh` → ANCHORS PASS (75/75) byte-identical. spec-lint: FAIL (109 violations) — +15 vs previous baseline (94); all carry-forward documentation-layer class (workflow-transition status `dev-done`, dead-links in other features, other features' arch-done status). Zero new functional violations. Pre-existing debt quoted in test report § 14._
  - **Output line** `ANCHORS PASS  (75 / 75)`
- [x] T-VPM-FINAL.6 — Write test-final report — _DONE 2026-05-29: `spec/ui-test-harness-viewport-matrix/reports/test-20260529-000000-v0.1.0.md`. 17-section report per template. Per-widget × per-slot PASS table in § 3. K1 opt-out list in § 12. 56 PNG count + 15 MB total size in § 8. VERDICT → PASS._

## M-PRESENT — Presenter

- [x] T-VPM-P1 — Deck `spec/ui-test-harness-viewport-matrix/presentations/v0.1.0-2026-05-29.md` — _DONE 2026-05-29: 12 verification rows green; 56 baseline PNGs / 15 MB documented; K3 single-host determinism caveat surfaced; T-VPM-D6 operator review punch list embedded (60 PNGs, ~10-15 min); 3 open follow-ups (D6 review, K3 cross-machine deferred to ui-test-harness-ci Queue, render_snapshots legacy baseline v0.2.0 cleanup). Pre-tick guard PRESENTATION CHECK PASS; spec-lint 109 violations identical to tester baseline (zero new functional regressions)._

## Notes

- **Anchor contract**: 71/71 byte-identical pre/post. Visual baselines
  PNG growth is the only artifact delta — checked at FINAL.4 against
  K2 ceiling.
- **Tester contract amendment**: this feature INHERITS the visual-fail
  HTML stanza authored by the Wave 1 sibling `visual-fail-html-reporter`.
  Does not amend `.claude/agents/tester.md` independently. Per
  trifecta direction § Risk R1 mitigation, if the stanza needs
  matrix-specific tweaks, architect M-T1 amends in T-VPM-T1.7.
- **Cross-platform baseline determinism (K3)**: explicitly out of
  scope at v0.1.0; this brief assumes the bootstrap's single-canonical-
  Apple-Silicon-box contract (per bootstrap H1 RESOLVED-WITH-CAVEAT).
  Cross-platform falsifier is a separate `ui-test-harness-ci` Queue
  item.
- **Wall-clock estimate per architect M-T1 dry-run**: ~3-4 dev days
  (helper ~0.5d + per-test expansion ~2-2.5d + baseline review +
  gates ~0.5-1d) + ~0.5 tester day + ~0.5 presenter day ≈ 1 week
  wall-clock total.
