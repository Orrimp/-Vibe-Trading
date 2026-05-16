---
slug: ui-headless-emulator
status: shipped
owner: shipped
updated: 2026-05-16
---

# Tasks — Headless Emulator adapter v0.1

> ## Ship status (2026-05-16)
>
> - **T01** ✓ — `crates/ui/tests/headless_emulator_smoke.rs` written
> - **T02** ✓ — V1-V6 all green; 1224 workspace tests pass (+1)
> - **T03** ✓ — backlog updated; this commit ships
>
> Net effort: ~1 hour actual (vs ~2.25h estimate). Spike-confirmed
> Emulator API matched the spec almost exactly; one-token correction
> `iced::core::Size` → `iced::Size`.

> Effort budget: **~1 dev-day** per
> [`iced-014-feature-analysis-2026-05-15.md §4`](../dev-notes/iced-014-feature-analysis-2026-05-15.md#headless-mode).
> Standalone scope (decomposed from `ui-test-harness-ci`).

## M1 — Smoke test (1.5h)

- [ ] **T01** — Author
  [`crates/ui/tests/headless_emulator_smoke.rs`](../../crates/ui/tests/headless_emulator_smoke.rs)
  per the [feature.md ## Design ## Test shape](feature.md#test-shape)
  prescription. Bounded event loop (10 ticks max); asserts on
  screenshot dimensions; uses `Mode::Zen`. 1.5 hours (includes
  iced_test API call-site iteration if the spike-confirmed shape
  needs adjustment).

## M2 — Verification (0.5h)

- [ ] **T02** — Run V1-V6 from
  [feature.md ## Acceptance / verification](feature.md#acceptance--verification-v-items):
  - V1: `cargo test -p ui --test headless_emulator_smoke` → exits 0
  - V3: `cargo test --workspace` → 1224+ PASS, 0 FAIL (1 new test)
  - V4: `cargo build -p ui --features live --bin cockpit_live` → succeeds
  - V5: `cargo clippy -p ui --no-deps` → no new warnings
  - V6: `cargo fmt --check` → clean

## M3 — Spec ship (0.25h)

- [ ] **T03** — Backlog entry transitions queued → shipped. Single-
  commit ship per the `ui-session-journal-iced-tester` precedent.

## Effort summary

| Milestone | Hours |
|---|---|
| M1 | 1.5 |
| M2 | 0.5 |
| M3 | 0.25 |
| **Total** | **2.25 hours** |

Way under the 1-dev-day estimate — the heavy lifting (Emulator
itself) is in iced_test 0.14.0 already.

## Status

- 2026-05-16 (orchestrator): tasks authored. Implementation starting.
