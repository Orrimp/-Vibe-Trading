---
slug: lab-polish-round-2
kind: verification
date: 2026-06-16
---

# lab-polish-round-2 — retroactive verification (2026-06-16)

This feature (R1 position-curve overlay + R2 SMA fast/slow param editor + R3 KPI-strip
densification on the cockpit Lab screen) was built ad-hoc on 2026-05-25 (issue #62:
commits `c1cddbe`, `ae26281`, `371d870` + the `position_curve` widget) but bypassed the
spec pipeline — it never got a trace row, a tester report, or a presenter deck, and its
status stuck at `proposed`. The 2026-06-15 staleness audit mis-tagged it "not started";
a 2026-06-16 re-verify found all three R's LIVE in the cockpit Lab screen. This note
records that verification (and gives the shipped feature its reports/ evidence).

## Retroactive verification (2026-06-16, orchestrator)

- **R1 — position-curve widget**: `crates/ui/src/widgets/position_curve.rs` present and
  wired into `crates/ui/src/screens/lab.rs` (`position_curve_strip` in the Lab Column
  layout). `cargo test -p ui position_curve` → **5 passed, 0 failed**.
- **R2 — SMA param editor**: shipped (`c1cddbe` backend + `ae26281` UI) — `LabState`
  `sma_fast_input` / `sma_slow_input` text inputs propagate via `SmaComposedRunInput`
  override through `cockpit_live.rs`.
- **R3 — KPI densification**: shipped (`371d870`) — 8-card 2×4 KPI strip
  (Final / Initial / MaxDD / Trades / Buys / Sells / Return% / Fees).
- **Anchor-additive**: in-memory `position_curve_raw` + UI-only KPI render; no Markdown
  report-body change. `verify_anchors.sh` → 119/119 (unchanged by this feature).

## Disposition

Registered as `shipped` 2026-06-16 (commit `553079a`; trace row
`REQ-LAB-POLISH-ROUND-2-001`). No presenter deck — this is retroactive registration of
already-live code, not a fresh ship for operator approval.
