---
slug: cockpit-toast-queue-v0.2.0-cleanup
status: in-progress
owner: tester
updated: 2026-05-27
---

# Tasks — cockpit-toast-queue v0.2.0 cleanup

## M0 — Analyst (this pass)

- [x] T-A-1 — Author `feature.md` (R1-R5, R-NR.1, K1-K3, H1-H2, no Qs,
  2-cell verdict tree, cross-refs to v0.1.0 + ADR-0046).
- [x] T-A-2 — Author this `tasks.md` scaffold.
- [x] T-A-3 — Append backlog Queue row at § Queue / UI / cockpit.
- [x] T-A-4 — Append trace row `REQ-COCKPIT-TOAST-QUEUE-CLEANUP-001`
  at EOF of `spec/trace.toml`.
- [x] T-A-5 — Pre-verify H1+H2 via grep: 2 field-WRITE sites
  (`cockpit_training_pressed_wiring.rs` lines 125, 323); 5 method-
  READ sites (same file lines 196-197, 332, 365-366, 368). No K1
  HARD STOP detected.

## M-OD — Operator-decide

- [ ] T-OD-1 — Expected fast-skip / standing-Autoapprove. No Qs.
  Operator promotes Queue → Active; flip frontmatter to
  `status: in-progress, owner: architect`.

## M-T1 — Architect (likely fast-skip)

- [ ] T-AR-1 — Confirm no in-flight ADR / future feature depends on
  the `pub toast_message` field. Default expectation: none. HANDOFF →
  developer with `owner: developer`.

## M-DEV — Developer (Wave A — single wave)

- [x] T-D-N1 — Audit gate (R3.1): `grep -rn "toast_message" crates/`
  → confirm only the known 2 + 5 sites surface. Hidden writer → HARD
  STOP, route back to architect per K1.
  - file:line: `crates/ui/tests/cockpit_training_pressed_wiring.rs:125,323` (2 writes) + 5 reads
  - Pre-flight grep confirmed known sites only; no K1 HARD STOP.
  - Test command: N/A (grep gate)

- [x] T-D-N2 — R1: delete `pub toast_message` field (state.rs:879)
  + doc-comment + two `None,` init lines (1132, 1238) + `Debug` impl
  field line (1075). `cargo check -p ui --tests --features live` —
  compile errors at the 2 known test sites expected.
  - file:line: `crates/ui/src/state.rs:871-879` (field + doc removed),
    `state.rs:1075` (Debug impl line removed), `state.rs:1132,1238`
    (`toast_message: None,` init lines removed).
  - Test command: `cargo check -p ui --tests --features live`
  - Output: `Finished` with 0 errors (after test file migration)

- [x] T-D-N3 — R2.1: rewrite `cockpit_training_pressed_wiring.rs:125`
  field-write to `Message::ShowToastWithSeverity(.., Danger)` dispatch.
  - file:line: `crates/ui/tests/cockpit_training_pressed_wiring.rs:124-131`
  - Test command: `cargo test -p ui --test cockpit_training_pressed_wiring --features live`
  - Output: `test spawn_failure_surfaces_toast ... ok`

- [x] T-D-N4 — R2.2: rewrite `cockpit_training_pressed_wiring.rs:323`
  K5-setup field-write to `Message::ShowToast(.)` dispatch (maps to
  `Info` per ADR-0046). Update line 332 assertion to read queue front.
  - file:line: `crates/ui/tests/cockpit_training_pressed_wiring.rs:322-335`
  - Test command: `cargo test -p ui --test cockpit_training_pressed_wiring --features live`
  - Output: `test k5_toast_non_clobber_run_completed_then_training_completed ... ok`

- [x] T-D-N5 — R2.3: flip remaining field-READ assertions in the file
  (196-197, 332, 365-366, 368) per R3 sub-route choice.
  Sub-route (b) chosen: all reads migrated to direct `toast_queue` access.
  - file:line: `crates/ui/tests/cockpit_training_pressed_wiring.rs:195-198,364-371`
  - Test command: `cargo test -p ui --test cockpit_training_pressed_wiring --features live`
  - Output: `5 passed; 0 failed`

- [x] T-D-N6 — R3 sub-route decision (a keep / b remove method shim);
  document in M-DEV close note with grep evidence.
  Sub-route **(b) — full removal**. Rationale: only test code referenced the
  method (1 call in `cockpit_toast_queue.rs:129`, 0 in production). Analyst
  recommendation aligned. Method removed; `cockpit_toast_queue.rs:129` migrated
  to `c.toast_queue.front().map(|t| t.message.as_str())`.
  Post-removal grep: `grep -rn "toast_message" crates/ --include="*.rs"` →
  single stale comment in `cockpit_live.rs:1181` (not touched per constraint).
  Zero field declarations, zero method definitions, zero field reads.
  - file:line: `crates/ui/src/state.rs:1234-1248` (method block removed);
    `crates/ui/tests/cockpit_toast_queue.rs:127-132` (call site migrated)
  - Test command: `cargo test -p ui --test cockpit_toast_queue`
  - Output: `4 passed; 0 failed`

- [x] T-D-N7 — Run `cargo test -p ui --test cockpit_training_pressed_wiring
  --features live` → 5/5 PASS; `cargo test -p ui --test cockpit_toast_queue`
  → 4/4 PASS; `cargo test -p ui --lib` matches v0.1.0 baseline.
  HANDOFF → tester.
  - file:line: all changed files listed above
  - Test command: see individual gates below
  - Output:
    - `cockpit_training_pressed_wiring --features live`: `5 passed; 0 failed`
    - `cockpit_toast_queue`: `4 passed; 0 failed`
    - `cargo test -p ui --lib`: `397 passed; 0 failed`
    - `scripts/verify_anchors.sh`: `ANCHORS PASS (69 / 69)`

## M-FINAL — Tester

- [x] T-T-1 — Anchor gate: `bash scripts/verify_anchors.sh` →
  byte-identical to v0.1.0 baseline.
  - Result: ANCHORS PASS (69 / 69) — verified 2026-05-28
- [x] T-T-2 — Workspace sweep: `cargo test --workspace` → no new
  failures vs v0.1.0 baseline.
  - Result: Only pre-existing `lab_run_engine::inner::h3_in_memory_equals_cached_disk` flake (whitelisted); zero new failures.
- [x] T-T-3 — Grep gates: `grep -rn "pub toast_message" crates/` → 0;
  `grep -rn "\.toast_message\s*=" crates/` → 0.
  - Result: Both 0 matches. Tester also removed stale 2-line comment at cockpit_live.rs:1181-1182; final `grep -rn "toast_message" crates/` → 0 matches (was 1 stale comment).
- [x] T-T-4 — spec-lint: no new violation categories vs v0.1.0
  baseline.
  - Result: spec-lint: FAIL (73/3) — same 3 categories as baseline (72/3); +1 dead-link is pre-existing from spec/cockpit-toast-queue/feature.md (lumen-phase-1-foundation), not introduced by this feature. No new categories. Does not block PASS.
- [x] T-T-5 — Author `reports/test-final-<date>-cockpit-toast-queue-v0.2.0-cleanup.md`
  with PASS / REGRESSION verdict per the 2-cell tree.
  - Result: spec/cockpit-toast-queue-v0.2.0-cleanup/reports/test-final-2026-05-28-cockpit-toast-queue-v0.2.0-cleanup.md — VERDICT: PASS

## M-PRESENTER

- [ ] T-P-1 — Short deck at
  `presentations/cockpit-toast-queue-v0.2.0-cleanup-<date>.md`.
  No operator visual smoke recipe (no visual change). Mechanical
  verdict block (anchor + grep + test counts).
