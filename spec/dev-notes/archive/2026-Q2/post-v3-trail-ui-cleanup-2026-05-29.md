# post-v3-trail-ui-cleanup 2026-05-29

**Feature:** `post-v3-retirement-trail-ui-cleanup v0.1.0`
**Predecessor:** `v3-regime-classifier` (operator-retired 2026-05-29 per
`spec/v3-regime-classifier/feature.md` shipped_disposition: T-REG-NO-ALPHA,
-0.294 Sharpe-delta on 2024 held-out validation).
**Author:** ui-designer

## Decision

**Option 2 — remove entirely.** Excised the Wave D-era regime-tag column
scaffolding from the Trail UI surface:

- `crates/ui/src/screens/trail.rs` — removed `regime_tag_cell` (~30 LoC),
  `regime_tag_column_header` (~10 LoC), and the module-doc subsection that
  introduced them. Replaced with a short historical-note paragraph
  pointing here.
- `crates/ui/src/strings.rs` — removed 4 constants
  (`TRAIL_COL_REGIME`, `TRAIL_NO_REGIME_TAG`, `LAB_REGIME_VOLATILE`,
  `LAB_REGIME_CALM`) and their `all()` registrations. Replaced each
  block with a short tombstone comment so future contributors don't
  re-add the constant accidentally.
- `crates/ui/tests/panel_snapshots.rs` — removed the `regime_tag_column`
  mod (5 `assert_snapshot!` tests + 1 constant-gate test, ~110 LoC).
- `crates/ui/tests/snapshots/` — deleted 5 `.snap` files
  (`panel_snapshots__regime_tag_column____regime_tag_column__{bull,bear,
  volatile,calm,none}.snap`).

Total reduction: ~190 LoC + 5 snapshot files + 4 strings.

## Rationale (durable-over-quick)

The decisive fact: **the regime helpers were never wired into `view()`**.
Wave D landed `regime_tag_cell` + `regime_tag_column_header` as module-
public functions with snapshot coverage, but no caller in `screens::trail`
(or anywhere else in `crates/ui`) ever invoked them. The column never
appeared in production — there is no behavioral regression to migrate.

Against this baseline the three options resolved cleanly:

| Option | Cost | Value at v3-retired |
|--------|------|---------------------|
| 1 — Conditional render gate | Adds audit-DB introspection plumbing to the Trail view to count `RegimeTag` rows. **New surface** for code that's currently dormant. | Speculative — assumes some future regime impl will use the same `RegimeTag` event shape and that the column should auto-light on first row. |
| 2 — Remove entirely | Delete 190 LoC + 4 strings + 5 snapshots. Bounded, one-commit. | Restores trail.rs / strings.rs to a state consistent with shipped reality. Git history (commit `ced662d` + the v3 retirement record) is the durable archive. |
| 3 — Rename to "Regime (dormant)" + ship visibly | Same 4 strings + 5 snapshot updates, plus permanently misleading column header in the live Trail view. | Negative — exposes operators to a UI surface for a feature that was retired with negative-alpha evidence. |

Option 1's "future-proofing for a third-party regime classifier" hits the
durable-over-quick contract directly: we do not keep code "in case someone
wants it" — we delete and let `git log -- crates/ui/src/screens/trail.rs`
plus `spec/v3-regime-classifier/feature.md` serve as the institutional
memory. If a v0.2.0 MR or alternative classifier ever ships, that feature
brief will design its own column from current Lumen tokens (which have
evolved since Wave D shipped) rather than inheriting a stale 2026-05
contract.

Option 3 was rejected as actively harmful: a "(dormant)" badge in
production UI is the worst of both worlds — visual noise for operators,
zero functional value, and a permanent invitation for "should we just
remove this?" cleanup tickets downstream.

## Audit-trail anchoring

The Wave D anchored backtest reports under
`spec/v3-regime-classifier/reports/` remain byte-immutable per ADR-0038
§ D6 — this cleanup does NOT touch any anchored bodies. The v3 retirement
disposition is unchanged; this dev-note is purely UI-surface housekeeping.

## Future-regime-impl handoff note

If a successor regime classifier (v0.2.0 MR, third-party, or alternative
taxonomy) ever ships and wants a Trail column:

1. Re-design the column from current Lumen tokens — do not resurrect
   Wave D's `bull`/`bear`/`volatile`/`calm` quartet without re-validating
   against the latest token set.
2. Wire the renderer into `screens::trail::view` (the integration step
   Wave D skipped). Snapshot tests must cover the live wired path, not
   only the helper in isolation.
3. Re-introduce the strings under names chosen by that feature, not
   resurrected from the 2026-05 set.
