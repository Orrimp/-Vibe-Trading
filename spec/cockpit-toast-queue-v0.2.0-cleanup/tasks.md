---
slug: cockpit-toast-queue-v0.2.0-cleanup
status: draft
owner: analyst
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

- [ ] T-D-N1 — Audit gate (R3.1): `grep -rn "toast_message" crates/`
  → confirm only the known 2 + 5 sites surface. Hidden writer → HARD
  STOP, route back to architect per K1.
- [ ] T-D-N2 — R1: delete `pub toast_message` field (state.rs:879)
  + doc-comment + two `None,` init lines (1132, 1238) + `Debug` impl
  field line (1075). `cargo check -p ui --tests --features live` —
  compile errors at the 2 known test sites expected.
- [ ] T-D-N3 — R2.1: rewrite `cockpit_training_pressed_wiring.rs:125`
  field-write to `Message::ShowToastWithSeverity(.., Danger)` dispatch.
- [ ] T-D-N4 — R2.2: rewrite `cockpit_training_pressed_wiring.rs:323`
  K5-setup field-write to `Message::ShowToast(.)` dispatch (maps to
  `Info` per ADR-0046). Update line 332 assertion to read queue front.
- [ ] T-D-N5 — R2.3: flip remaining field-READ assertions in the file
  (196-197, 332, 365-366, 368) per R3 sub-route choice.
- [ ] T-D-N6 — R3 sub-route decision (a keep / b remove method shim);
  document in M-DEV close note with grep evidence.
- [ ] T-D-N7 — Run `cargo test -p ui --test cockpit_training_pressed_wiring
  --features live` → 5/5 PASS; `cargo test -p ui --test cockpit_toast_queue`
  → 4/4 PASS; `cargo test -p ui --lib` matches v0.1.0 baseline.
  HANDOFF → tester.

## M-FINAL — Tester

- [ ] T-T-1 — Anchor gate: `bash scripts/verify_anchors.sh` →
  byte-identical to v0.1.0 baseline.
- [ ] T-T-2 — Workspace sweep: `cargo test --workspace` → no new
  failures vs v0.1.0 baseline.
- [ ] T-T-3 — Grep gates: `grep -rn "pub toast_message" crates/` → 0;
  `grep -rn "\.toast_message\s*=" crates/` → 0.
- [ ] T-T-4 — spec-lint: no new violation categories vs v0.1.0
  baseline.
- [ ] T-T-5 — Author `reports/test-final-<date>-cockpit-toast-queue-v0.2.0-cleanup.md`
  with PASS / REGRESSION verdict per the 2-cell tree.

## M-PRESENTER

- [ ] T-P-1 — Short deck at
  `presentations/cockpit-toast-queue-v0.2.0-cleanup-<date>.md`.
  No operator visual smoke recipe (no visual change). Mechanical
  verdict block (anchor + grep + test counts).
