# Story 3.19: advisor-eur-live-rate

Status: ready-for-dev

<!-- Analyst-drafted 2026-07-29 (Mary). Operator-DECIDED build: PRD §13 Q1 answer, 2026-07-27
     (the D4/FR-2 residual fork — "live-fetched rate layered on the static value as fallback").
     This is the ADR-0065 § D6 reserved v0.3 path, promoted from NOTED-not-built to build. -->

## Story

As the operator of the Honest Advisor,
I want the advisor's EUR display and plan annotation to use a live-fetched EUR/USD rate that degrades to the shipped static config rate whenever the fetch is unavailable,
so that the "€200 ≈ $X (at R EUR/USD, ⟨source/as-of⟩)" honesty label reflects the actual rate when online without ever making the advisor network-dependent or touching money-math determinism.

## Acceptance Criteria

1. **Given** external I/O behind a trait (CLAUDE.md rule), **when** the rate is resolved, **then** a `RateSource` seam (the ADR-0065 § D6 reserved shape; test fakes per the ADR-0061 HttpKlineFetcher-vs-mock precedent, never network in tests) yields either a live rate with provenance `source="live:<provider>"` + a real `as_of` label, or — on ANY failure (offline, timeout, non-2xx, unparseable) — the existing static path (`AdvisorConfig.eur_usd_rate` → `DEFAULT_EUR_USD_RATE`) with its `"config"` provenance. The static rate REMAINS a permanent fallback; a dead network never blocks or delays a plan beyond the fetch timeout.
2. Fetch timing: at plan render and on an explicit operator refresh only — **no new always-on network call at boot**, no background polling loop, no new subscription recipe.
3. AD-9 holds: the fetched value is parsed to `Decimal` at the boundary and enters ONLY the checked `FxRate::new(rate, source, as_of)` ctor (reject ≤ 0); `FxRate::convert_eur_to_usdt` stays the single EUR→USDT multiply (existing grep-guard in `crates/core/tests/eur_fx_conversion_applied.rs` stays green); no `f64` anywhere on the rate path. Display/plan-annotation only — the same one `BudgetConversion` feeds engine and display (no second conversion path).
4. Tests via the fake `RateSource`: (a) live-success shows live provenance in the rendered label, (b) forced-failure falls back to static with `"config"` provenance visible, (c) display==engine on the live rate (same `BudgetConversion` value both readers — the ADR-0065 D2 anti-drift discipline). UI-visible changes prove out at the rendered-pixel layer (AD-10).
5. The standing floor holds: `verify_anchors` 119/119 before AND after (the anchored CLI path never reads the rate — anchor-safe by construction, per the shipped F7 trace row); `python3 scripts/spec_lint.py` PASS; `cargo clippy -- -D warnings` clean.

## Tasks / Subtasks

- [ ] Architect M-T1: ratify the `RateSource` trait home + fetcher crate (ADR-0065 § D6 promised "its own ADR" — AD-18 atomic ADR + Registry row) and resolve the layering question (see Dev Notes).
- [ ] Implement the trait + live fetcher (reqwest, short timeout) + static-fallback resolution.
- [ ] Wire fetch-at-plan-render + explicit refresh affordance; thread provenance through the existing `FxNote` source/as-of labels.
- [ ] Fake-`RateSource` test trio (AC 4) + pixel-layer render proof for the label change.
- [ ] Gates: anchors 119/119, spec-lint, clippy, fmt.

## Dev Notes

- **Exact integration point (verified in code 2026-07-29):** `crates/core/src/fx.rs` — `FxRate {rate, source, as_of}` value object; its doc already states `DEFAULT_EUR_USD_RATE` (= `dec!(1.08)`) "is also the fallback the future v0.3 live-rate path (ADR-0065 § D6) would use when the live fetch fails". Config surface: `AdvisorConfig { eur_usd_rate, eur_usd_rate_as_of }` (`crates/agent/src/config.rs:828-845`, `[advisor]` table in `config/agent.toml`), resolved at `crates/ui/src/bin/cockpit_live.rs:458-462` and consumed at the two F7 seams (`cockpit_live.rs:1570-1580` and `:1889-1896` — `FxRate::config` / `FxRate::new(rate, "config", as_of)`). Display readers: `screens/leaderboard.rs:167`, `screens/forward_plan.rs`, `export/plan_export.rs`. The live path swaps WHICH `FxRate` reaches those seams; nothing downstream changes.
- **Layering (architect resolves):** ADR-0065 rejected a `ui → data` edge *for the static value*; `crates/ui` today has an **optional** `data` dep (`crates/ui/Cargo.toml:82`, Yahoo parquet). Options: home the fetcher in `crates/data` behind that existing optional edge, or in `crates/agent`. Verify `cargo tree -p ui` (default features) is unchanged or the change is ratified in the new ADR.
- **Determinism:** `as_of` remains a label. The live fetch stamps provenance at fetch time; the anchored/headless CLI path is USDT-denominated and never reads the rate — no anchored scenario, no anchors delta, no REVISION change.
- **Do-not-build register check (mandatory): PASS.** Not Group A (no alpha/prediction — the rate is a known input, ranking is FX-invariant); **not B-2 live trading** (read-only public FX quote for a display label; no order path, no exchange keys, no funds); no Group E surface (frozen gate + anchors untouched by construction). No register row implicates a display-rate fetch; the era-qualified thesis is unaffected.
- Small story — keep it tight; the shipped `3-7-advisor-eur-fx` story + its trace row (`REQ-ADVISOR-EUR-FX-001`) carry the full F7 context.

### References

- Trace: `REQ-ADVISOR-EUR-LIVE-RATE-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))
- Decision record: PRD §13 Q1 (operator answer 2026-07-27); ADR-0065 § D6 (reserved path); predecessor story `3-7-advisor-eur-fx`.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
