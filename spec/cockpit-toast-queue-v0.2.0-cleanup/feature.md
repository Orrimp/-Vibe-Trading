---
slug: cockpit-toast-queue-v0.2.0-cleanup
version: 0.2.0
status: shipped
owner: shipped
updated: 2026-05-28
predecessor: cockpit-toast-queue v0.1.0
priority: P3
---

# cockpit-toast-queue v0.2.0 cleanup — retire legacy `toast_message` field

> **Spawned from `cockpit-toast-queue v0.1.0` M-DEV architecture
> deviation (commit `a723d24`, operator-approved 2026-05-27).**
> ADR-0046 § T-AR-5 specified a back-compat METHOD shim. The v0.1.0
> developer kept the `pub toast_message: Option<SmolStr>` FIELD
> alongside the new queue + method because
> `cockpit_training_pressed_wiring.rs` writes the field directly.
> Source annotated `// MIGRATION: remove at v0.2.0`. This brief
> executes the migration.

## Why

Dual-storage state (queue + legacy field) is a foot-gun: a future
contributor sees `pub toast_message`, writes to it, expects a visible
toast — the field is a dead store relative to the queue, no toast
renders. Silent no-op. This retires the v0.1.0 K6 risk row (shim
drift). Cost small while surface small: **2 direct-field-WRITE
sites** in one test file (`cockpit_training_pressed_wiring.rs` lines
125, 323); production `cockpit_live.rs:1177-1189` already uses the
message API. Waiting expands the surface unnecessarily.

## Requirements

### R1 — Remove `pub toast_message: Option<SmolStr>` FIELD

- **R1.1** Delete the field at `crates/ui/src/state.rs:879` and its
  doc-comment block (871-878).
- **R1.2** Delete the two `toast_message: None,` constructor init
  lines (`state.rs:1132` in `Default::default`, `state.rs:1238` in
  `Cockpit::ready`).
- **R1.3** Delete the `Debug` impl field line (`state.rs:1075`).
- **Acceptance:** `grep -rn "pub toast_message" crates/ui/src/state.rs`
  returns ZERO matches.

### R2 — Migrate `cockpit_training_pressed_wiring.rs` field-writes

Two direct-WRITE sites, both in this test file:

- **R2.1** Line 125 (spawn-failure path): rewrite
  `self.cockpit.toast_message = Some(SmolStr::new(format!(...)))` to
  a `ui::state::update(&mut self.cockpit,
  Message::ShowToastWithSeverity(SmolStr::new(format!(...)),
  ToastSeverity::Danger))` call (mirrors `cockpit_live.rs:1183-1189`).
- **R2.2** Line 323 (K5 setup): rewrite
  `cockpit.toast_message = Some(SmolStr::new("Backtest complete"))`
  to a `ui::state::update(&mut cockpit,
  Message::ShowToast(SmolStr::new("Backtest complete")))` call. The
  no-severity `ShowToast` arm maps to `Info` per ADR-0046 — preserves
  the K5 semantic (a Backtest-complete toast exists prior to the
  TrainingExited probe).
- **R2.3** Flip all field-READ assertions in the file (lines 196-197,
  332, 365-366, 368) to either the method shim (R3 sub-route a) or
  direct queue reads (R3 sub-route b).
- **Acceptance:** `grep -rn "\.toast_message\s*=" crates/` returns
  ZERO matches; 5/5 tests in this file still PASS.

### R3 — Audit, then optionally remove the `toast_message()` METHOD shim

The shim at `state.rs:1258` exists solely to keep the K5 test
file readable post-rename. After R2 there may be no readers left.

- **R3.1** Audit gate: `grep -rn "toast_message(" crates/`. The K5
  test (5 sites) is the only known reader.
- **R3.2** **Sub-route (a) — readers remain outside the K5 test:**
  KEEP the method, drop the `// MIGRATION: remove at v0.2.0`
  annotation, update doc-comment to point at `toast_queue.front()`.
- **R3.3** **Sub-route (b) — only K5 test reads remain:** flip the
  5 K5 reads to direct `toast_queue.front()` / `.is_empty()` access,
  then DELETE the method shim.
- **R3.4** Choice is developer-discretion, NOT operator-decide.
  Analyst recommendation: **route (b) full removal** if audit
  confirms K5-only — the method existed for one cycle, this cleanup
  is its purpose, and `toast_message()` is misleading naming.
- **Acceptance:** chosen sub-route shipped with grep-evidence in the
  M-DEV close note.

### R4 — Non-regression: 5 `cockpit_training_pressed_wiring` tests stay green

- **R4.1** `cargo test -p ui --test cockpit_training_pressed_wiring`
  reports `5 passed` (matches v0.1.0 commit `a723d24`).
- **R4.2** K5 contract semantics preserved by R2.2 (Backtest-complete
  toast is still the front entry after TrainingExited).

### R5 — Non-regression: zero anchor delta (UI-only)

- **R5.1** Zero touched files in `crates/{backtest,strategy,exec,risk,reports,forecast,audit,cost,data}/`.
- **R5.2** `scripts/verify_anchors.sh` exits 0; anchor count + SHAs
  byte-identical to whatever the v0.2.0 cleanup-ship baseline is.
- **R5.3** `cockpit_toast_queue.rs` (4/4) + panel snapshots (86/86)
  byte-stable.

### R-NR.1 — Zero new design tokens; zero `strings.rs` adds

Inherited from v0.1.0 R-NR.4. Verifiable by `git diff` review.

## K1-K3 falsifiers

- **K1 — Hidden field-WRITE outside the K5 test (reflection / serde
  / fixture bin).** `Cockpit` has no serde derive. Detection: post-R1
  a hidden writer surfaces as a Rust compile error — the compiler
  enumerates every field-write site. **HARD STOP ROUTE: surfaced
  writer outside `cockpit_training_pressed_wiring.rs` → route back to
  architect** (decide: keep field with permanent justification /
  migrate writer through message API / re-decide). Analyst pre-check
  2026-05-27: grep returns exactly the 2 known sites. No risk
  detected.
- **K2 — K5 semantic shift via message dispatch vs direct field
  write.** R2.2 rewrites direct write into a `ShowToast` dispatch that
  generates a real `ToastEntry` (id + `created_at: Instant::now()`)
  into the queue. Line-332 assertion `field == Some("Backtest complete")`
  becomes a queue-front read. Mitigation: explicit port of each
  field-read in R2.3; failure surfaces at the test command.
- **K3 — Removing the method shim breaks an unknown reader
  (gallery / panel snapshot / fixture bin).** Pre-check 2026-05-27:
  `grep -rn "toast_message(" crates/` returns exactly the 5 K5 test
  sites. No production reader. R3.1 audit gate; on surprise, take
  sub-route (a).

## H1-H2 hypotheses

- **H1** — *All direct-field-WRITE sites are in test code, not
  production.* **VERIFIED at analyst pass 2026-05-27** (2 sites in
  the K5 test file; production `cockpit_live.rs` already routes via
  message API). Falsifier: unseen writer surfaces during dev → R1.1
  compile error → K1 HARD STOP route.
- **H2** — *No production code reads via the `toast_message()`
  method.* **VERIFIED at analyst pass 2026-05-27** (5 sites, all in
  the K5 test file). Falsifier: R3.1 audit surprise → R3 takes sub-
  route (a).

## Operator-decide questions

**None.** Pure refactor; no operator-visible behaviour change.
Recommendation: **standing-Autoapprove** at M-OD (fast-skip). R3
sub-route is developer-discretion.

## Cost estimate

**~2-4 hours wall-clock.** M0 ~20 min (this pass); M-OD ~0 (no Qs);
M-T1 ~15 min (fast-skip likely); M-DEV Wave A ~1-2 h (audit + grep
gate + R1/R2/R3 refactor + test runs); M-FINAL ~45 min (anchor sweep
+ workspace tests + grep gates); M-PRESENTER ~20 min (short deck;
no visual smoke).

## Pre-drawn verdict tree (2-cell)

- **Cell 1 — PASS (R1+R2+R3 done, R4+R5 green):** ship. Field
  removed; field-writes migrated; test suite + anchors unchanged.
  Update the v0.1.0 brief's architecture-deviation note to reference
  this ship.
- **Cell 2 — REGRESSION (R4 or R5 fails):** route back to developer.
  R4 failure → K5 contract rewrite at R2.2 is wrong (likely a missed
  field→queue read flip); R5 failure → side-channel (e.g. `Debug`
  impl change captured by a fixture-bin snapshot). HARD STOP from
  K1 (hidden writer) surfaces during M-DEV as a compile error and
  routes to architect from the dev wave, not the tester.

## Cross-references

- **Predecessor (closes v0.1.0 architecture-deviation follow-on):**
  [`cockpit-toast-queue v0.1.0`](../cockpit-toast-queue/feature.md)
  § "Architecture deviation" + K6 (shim drift).
- **Parent ADR:**
  [ADR-0046](../architecture/adr/0046-cockpit-toast-queue.md)
  § T-AR-5 specified the method-only shim — field coexistence was
  an implementation deviation. Removing the field restores the
  ADR-locked end state; no ADR amendment required.
- **Grandparent (K5 source):**
  [`cockpit-training-pressed-wiring v0.1.0`](../cockpit-training-pressed-wiring/feature.md)
  owns the test file this brief migrates.

## Backtest scenarios

**None.** UI-only refactor; zero anchor delta is the contract.

## Out of scope

- Any visual / behavioural change to the toast tray (capacity,
  timeout, placement, severity palette).
- Removing or renaming `Message::ShowToast(SmolStr)` — survives;
  Lab Compare cap-hit and K5 depend on it.
- Promoting any v0.1.0 H1/H2/H3 falsifier into a fix.

## Implementation

**Developer wave A completed 2026-05-27.**

### R3 sub-route decision: (b) full removal

Chose **sub-route (b)** — the `toast_message()` method shim was removed
entirely. Rationale: audit confirmed only test code referenced it (1 call
in `cockpit_toast_queue.rs`, 0 production readers). The analyst brief
recommends (b) when K5-only, and the naming `toast_message()` is
misleading now that the queue is the authoritative store.

### Changed files

- `crates/ui/src/state.rs` — deleted `pub toast_message: Option<SmolStr>`
  field (lines 871-879), the `Debug` impl field line (`.field("toast_message",
  &self.toast_message)`), two `toast_message: None,` constructor init lines
  (in `Default::default` and `Cockpit::ready`/`boot`), and the
  `pub fn toast_message()` method shim block.
- `crates/ui/tests/cockpit_training_pressed_wiring.rs` — added
  `Message, ToastSeverity, update` imports; migrated line-125 field-write to
  `update(.., ShowToastWithSeverity(.., Danger))`; migrated line-323
  field-write to `update(.., ShowToast(..))`; flipped all 5 field-READ
  assertions to direct `toast_queue` access.
- `crates/ui/tests/cockpit_toast_queue.rs` — migrated the one
  `c.toast_message()` method call to `c.toast_queue.front().map(|t|
  t.message.as_str())`.

### grep gate output (post-cleanup)

```
grep -rn "toast_message" crates/ --include="*.rs"
crates/ui/src/bin/cockpit_live.rs:1181:  // The back-compat `toast_message` field shim keeps the
```

Single stale comment in `cockpit_live.rs:1181` (not modified per constraint).
Zero field declarations, zero method definitions, zero field reads.

`grep -rn "pub toast_message" crates/` → 0 matches.
`grep -rn "\.toast_message\s*=" crates/` → 0 matches.

### Gate results

| Gate | Result |
|---|---|
| `cargo build -p ui` | PASS |
| `cargo test -p ui --test cockpit_training_pressed_wiring --features live` | 5/5 PASS |
| `cargo test -p ui --test cockpit_toast_queue` | 4/4 PASS |
| `cargo test -p ui --lib` | 397 PASS |
| `scripts/verify_anchors.sh` | 69/69 PASS |
| New clippy warnings from changed files | 0 |

## Changelog

- 2026-05-27 (analyst): v0.1.0 draft. R1-R5 + R-NR.1 + K1-K3 + H1-H2
  closed; no Qs (standing-Autoapprove). H1+H2 pre-verified via grep
  (both VERIFIED, no K1 HARD STOP). Cost ~2-4 h. HANDOFF →
  operator-decide.
- 2026-05-27 (developer): v0.2.0 Wave A complete. R1+R2+R3(b) done.
  All gate tests green. Sub-route (b) shipped. HANDOFF → tester.
