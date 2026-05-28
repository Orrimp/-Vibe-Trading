---
slug: cockpit-toast-queue-v0.2.0-cleanup
status: awaiting-approval
owner: presenter
updated: 2026-05-28
mode: release
verdict_source: spec/cockpit-toast-queue-v0.2.0-cleanup/reports/test-final-2026-05-28-cockpit-toast-queue-v0.2.0-cleanup.md
---

# cockpit-toast-queue v0.2.0 cleanup — sprint review (2026-05-28)

## TL;DR

The legacy `pub toast_message: Option<SmolStr>` field is gone — the queue
is now the one and only source of truth for toasts, exactly as
ADR-0046 § T-AR-5 specified, with zero anchor delta and all K5 / queue
tests green.

## What shipped

- **One field deleted** (`crates/ui/src/state.rs:871-879` —
  `pub toast_message: Option<SmolStr>` + its doc-comment block) plus
  the two `toast_message: None,` constructor inits and the
  `Debug`-impl line that referenced it.
- **One method shim deleted** (`state.rs:1234-1248` —
  `pub fn toast_message(&self) -> Option<&str>`); audit confirmed
  only test code read it.
- **Two field-WRITE sites migrated** to message dispatch in
  `crates/ui/tests/cockpit_training_pressed_wiring.rs`:
  line 125 (spawn-failure Danger toast) → `Message::ShowToastWithSeverity(..,
  Danger)`; line 323 (K5 setup) → `Message::ShowToast(..)` (Info per
  ADR-0046). Mirrors the production pattern in `cockpit_live.rs`.
- **Five field/method READ sites flipped** in the same test file
  (lines 196-197, 332, 365-366, 368) to direct
  `toast_queue.front()` access; plus one method-call migrated in
  `crates/ui/tests/cockpit_toast_queue.rs:129`.
- **Stale-comment polish** — the 2-line dead reference at
  `crates/ui/src/bin/cockpit_live.rs:1181-1182` (which described
  the field shim that no longer exists) was removed by the tester
  during M-FINAL. Post-cleanup `grep -rn "toast_message" crates/` →
  **0 matches**.

## The big picture

This closes the architecture-deviation footnote on the v0.1.0 ship.
ADR-0046 § T-AR-5 specified a back-compat **method** shim for one cycle
to keep the K5 test file readable across the rename — nothing more.
The v0.1.0 developer, faced with two direct field-WRITE sites in
`cockpit_training_pressed_wiring.rs`, kept the underlying **field** as
well and annotated it `// MIGRATION: remove at v0.2.0`. v0.2.0 is that
migration: full removal of the field, the method shim, and the stale
comment. Dual-storage state (queue + legacy field) was the K6 risk row
in the v0.1.0 brief; it is now retired.

## Verification

| Gate | Result | Evidence |
|---|---|---|
| K5 regression (`cockpit_training_pressed_wiring --features live`) | **5/5 PASS** | report § 3 |
| v0.1.0 integration (`cockpit_toast_queue`) | **4/4 PASS** | report § 3 |
| Workspace lib (`ui --lib`) | **397 PASS** (baseline) | report § 3 |
| Workspace sweep | **0 new failures** (pre-existing `lab_run_engine` flake whitelisted) | report § 3 |
| Anchors (`verify_anchors.sh`) | **69/69 byte-identical** (UI-only) | report § 8 |
| `cargo fmt --all -- --check` | PASS (clean) | report § 2 |
| Clippy on changed files | **0 new warnings** (130 pre-existing on parent) | report § 2 |
| `grep -rn "pub toast_message" crates/` | **0 matches** | report § 7 |
| `grep -rn "\.toast_message\s*=" crates/` | **0 matches** | report § 7 |
| `grep -rn "toast_message" crates/ --include="*.rs"` | **0 matches** (was 1 stale comment; tester cleaned) | report § 7 |
| spec-lint | 73/3 — same 3 categories as 72/3 baseline; +1 dead-link is pre-existing carry-forward | report § 9 |

## Live demo — workspace anchor gate

```
$ bash scripts/verify_anchors.sh
...
PASS  sharpe-comparison-vol-target-bs1-realbaseline  ff2b934961f8...
PASS  btc-yahoo-2024-1d-sma-cross           8045623b4c9b...
---
ANCHORS PASS  (69 / 69)
```

UI-only refactor; zero report bodies touched; the anchor count and SHAs
remain byte-identical to the v0.1.0 baseline (term gloss:
"body-SHA-256 anchor" = a SHA over the canonicalized body of a
report file, the regression gate that fires if any backtest output
drifts).

## Numbers that matter

- **Field declarations removed:** 1 (`pub toast_message`)
- **Method shims removed:** 1 (`toast_message()`)
- **Constructor init lines removed:** 2 (`Default::default` +
  `Cockpit::ready` / `boot`)
- **Debug-impl lines removed:** 1
- **Stale-comment lines removed:** 2 (`cockpit_live.rs:1181-1182`)
- **Field-WRITE sites migrated:** 2 (Danger + Info dispatch)
- **Field-READ sites flipped:** 5 + 1 = 6 (all to direct
  `toast_queue` access)
- **Tests green:** K5 5/5, integration 4/4, lib 397/397
- **Anchors:** 69/69 byte-identical
- **New clippy warnings on changed files:** 0
- **Operator-decide Qs:** 0 (standing-Autoapprove brief)

## The honest footnote

The brief instructed the developer to NOT touch `cockpit_live.rs` to
keep the dev diff narrowly scoped. That left one stale 2-line comment
in production source ("The back-compat `toast_message` field shim
keeps the `spawn_failure_surfaces_toast` test green via its own
helper.") referring to a shim that no longer existed. During M-FINAL
the tester took the polish — rationale captured in the test report
§ 7: the comment was factually wrong (the shim is gone), the file is
production source rather than an anchored report (so byte-immutability
does not apply), and the 5-line delete carries zero semantic risk. K5
tests re-ran green after the edit. Net result: complete elimination
of every `toast_message` reference in `crates/`, not the dev-claimed
"1 stale comment remaining". This is the right call.

### spec-lint baseline-shift observation (not from this feature)

Presenter ran `spec_lint.py` and saw **74/4** (categories: dead-link
70, missing-frontmatter 1, shipped-no-tests 2, **unreferenced-anchor
1**). The tester's PASS-time baseline was 73/3 (no
`unreferenced-anchor`). The new category fires for
`eth-yahoo-2024-1d-sma-cross` — an anchor introduced by commit
`bd7e04b` (`feat(lab-yahoo-realdata-v0.1.2): M-DEV + M-DEV-UI parallel
lanes complete (69 → 70 anchors)`), which landed AFTER the toast-queue
M-FINAL PASS at `2dcb112`. The toast-queue cleanup itself did not
touch `spec/anchors.toml` or add anchor rows. Surfacing this here for
transparency; the underlying fix (add a `trace.toml` row citing the
new anchor) belongs to the lab-yahoo-realdata-v0.1.2 owner, not this
feature.

## What's next

Nothing from this brief. ADR-0046's one-cycle migration commitment is
honored: the method shim existed for exactly v0.1.0, and v0.2.0
retires it. No v0.3.0 candidate surfaces from this work; the toast
queue is now in its ADR-locked end state.

## Open decisions

_n/a — pure refactor; standing-Autoapprove per the analyst brief; no
operator-visible behaviour change._

## Approval

- [x] Approved — ship  _(2026-05-28, operator)_
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / Rejection reason

_(operator fills if applicable)_

## Feedback log

_(empty — first round)_

## Cross-references

- Tester report: [`reports/test-final-2026-05-28-cockpit-toast-queue-v0.2.0-cleanup.md`](../reports/test-final-2026-05-28-cockpit-toast-queue-v0.2.0-cleanup.md)
- Feature brief: [`feature.md`](../feature.md)
- Tasks: [`tasks.md`](../tasks.md)
- Predecessor: [`cockpit-toast-queue v0.1.0`](../../cockpit-toast-queue/feature.md)
- Parent ADR: [ADR-0046](../../architecture/adr/0046-cockpit-toast-queue.md) § T-AR-5
