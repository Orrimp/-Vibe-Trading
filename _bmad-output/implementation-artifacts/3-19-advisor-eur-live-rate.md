# Story 3.19: advisor-eur-live-rate

Status: done

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

- [x] Architect M-T1: **DONE 2026-11-08 — [ADR-0101](../planning-artifacts/architecture/decisions/0101-the-live-eur-usd-rate-seam.md)** + Registry row, same commit (AD-18). Layering, provider, failure policy and the `f64` trap are all settled there; the answers are restated under Dev Notes below so the build does not re-derive them.
- [x] **DONE 2026-11-08** — the trait + live fetcher + static-fallback resolution: [`crates/data/src/fx_rate.rs`](../../crates/data/src/fx_rate.rs) (`RateSource` / `HttpRateSource` / `FakeRateSource`) and [`crates/ui/src/live_fx.rs`](../../crates/ui/src/live_fx.rs) (`resolve_fx_rate` = ANY failure → static; `fetch_live_fx_blocking` = the bounded bridge).
- [x] **DONE 2026-11-08** — fetch-at-plan-render + the explicit refresh affordance; provenance threaded through the `FxNote` labels by one formatter (`widgets::num::fmt_fx_provenance`) used by the Live screen, the forward plan and the plan export.
- [x] **DONE 2026-11-08** — fake-`RateSource` test trio + the `f64` boundary guard + the pixel-layer render proof (see Completion Notes).
- [x] **DONE 2026-11-08** — gates: anchors 119/119 before AND after, spec-lint PASS, clippy `-D warnings` clean, fmt clean.

## Dev Notes

- **Exact integration point (verified in code 2026-07-29):** `crates/core/src/fx.rs` — `FxRate {rate, source, as_of}` value object; its doc already states `DEFAULT_EUR_USD_RATE` (= `dec!(1.08)`) "is also the fallback the future v0.3 live-rate path (ADR-0065 § D6) would use when the live fetch fails". Config surface: `AdvisorConfig { eur_usd_rate, eur_usd_rate_as_of }` (`crates/agent/src/config.rs:828-845`, `[advisor]` table in `config/agent.toml`), resolved at `crates/ui/src/bin/cockpit_live.rs:458-462` and consumed at the two F7 seams (`cockpit_live.rs:1570-1580` and `:1889-1896` — `FxRate::config` / `FxRate::new(rate, "config", as_of)`). Display readers: `screens/leaderboard.rs:167`, `screens/forward_plan.rs`, `export/plan_export.rs`. The live path swaps WHICH `FxRate` reaches those seams; nothing downstream changes.
- **Layering — RESOLVED by ADR-0101 (read it before starting; this is the summary).** `RateSource` + its HTTP impl + a `fixtures`-gated fake go in a new `crates/data/src/fx_rate.rs`, following the ADR-0061 `KlineFetcher` shape (`crates/data/src/binance_klines.rs:344-384` is the model, `MockFetcher` at `:635` the fake). `data` already depends on `trading_core`, so the fetcher returns a validated `FxRate` built through `FxRate::new` — AC3's "only the checked ctor" holds inside `data`. `core` stays pure. The ui side hides behind a NEW `live-fx = ["dep:data"]` feature listed in ui's `default`, because `dep:data` creates no implicit feature to `cfg` on and `cargo check -p ui --no-default-features --features fixtures` (ci.yml:199) must keep compiling. Provider: `https://api.frankfurter.dev/v1/latest?from=EUR&to=USD` (ECB, keyless), provenance `"live:ecb"`, `as_of` = the response's own `date`. **Pin the `.dev` host** — `.app` answers 301. **The rate must never pass through `f64`** (`rust_decimal` serde, never `Value::as_f64`): that is the one place AD-9 breaks while every test still passes. Any failure at all → static config rate with `"config"` provenance; one attempt, 2 s timeout, no retry. "Live" means the latest ECB WORKING-DAY rate — asked on a Sunday it returns Friday's date — so no copy may render it as "now".
- **Superseded layering note (kept for the record):** ADR-0065 rejected a `ui → data` edge *for the static value*; `crates/ui` today has an **optional** `data` dep (`crates/ui/Cargo.toml:82`, Yahoo parquet). Options: home the fetcher in `crates/data` behind that existing optional edge, or in `crates/agent`. Verify `cargo tree -p ui` (default features) is unchanged or the change is ratified in the new ADR.
- **Determinism:** `as_of` remains a label. The live fetch stamps provenance at fetch time; the anchored/headless CLI path is USDT-denominated and never reads the rate — no anchored scenario, no anchors delta, no REVISION change.
- **Do-not-build register check (mandatory): PASS.** Not Group A (no alpha/prediction — the rate is a known input, ranking is FX-invariant); **not B-2 live trading** (read-only public FX quote for a display label; no order path, no exchange keys, no funds); no Group E surface (frozen gate + anchors untouched by construction). No register row implicates a display-rate fetch; the era-qualified thesis is unaffected.
- Small story — keep it tight; the shipped `3-7-advisor-eur-fx` story + its trace row (`REQ-ADVISOR-EUR-FX-001`) carry the full F7 context.

### References

- ADR: [ADR-0101](../planning-artifacts/architecture/decisions/0101-the-live-eur-usd-rate-seam.md) — the M-T1 decision (accepted 2026-11-08)
- Trace: `REQ-ADVISOR-EUR-LIVE-RATE-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))
- Decision record: PRD §13 Q1 (operator answer 2026-07-27); ADR-0065 § D6 (reserved path); predecessor story `3-7-advisor-eur-fx`.

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (`claude-opus-5`), developer seam, 2026-11-08.

### Debug Log References

- `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119) [declared 119 · floor 119]` — run BEFORE any edit and again after the last one.
- `cargo test -p data` → all suites green (incl. the 8 new `fx_rate` unit tests + the 3 boundary tests).
- `cargo test -p ui` → green, 0 failed (the 657-test lib suite plus every integration target, including the four new pixel guards).
- `cargo clippy -p data -p ui -- -D warnings` → clean; same with `--tests`.
- `cargo fmt -p data -p ui -- --check` → clean.
- `uv run --python 3.11 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)`.
- Host note: every Rust command needs `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk` on this machine (the default SDK cannot link).

### Completion Notes List

**AC1 — the seam and the fallback.** `data::fx_rate::RateSource` is the async trait; `HttpRateSource` performs one GET against the pinned `https://api.frankfurter.dev/v1/latest?from=EUR&to=USD` with a 2 s client timeout and no retry; `FakeRateSource` (gated `#[cfg(any(test, feature = "fixtures"))]`) is what every test uses — **no test opens a socket**. Every success path exits through the checked `FxRate::new`, so a non-positive quote is an error, never a rate. `ui::live_fx::resolve_fx_rate` turns *any* `Err` into the configured static rate with its `"config"` provenance, logging the cause at `debug`.

**AC2 — fetch timing.** Exactly two triggers, both in `bin/cockpit_live.rs::update`: a plan render (a `BakeoffRunCompleted` that crowned a row, or a `PromoteSweptConfig`) and `Message::FxRateRefreshRequested`. Nothing fetches at boot, there is no poll and no new subscription recipe. The fetch is resolved **once** per qualifying message and shared by both seams. `the_live_rate_is_fetched_only_when_asked` pins the call count through the fake.

**AC3 — AD-9.** The quote never becomes an `f64`. `serde_json` parses every JSON float into one unless told otherwise, so the rate is deserialized as a `RawValue` — the untouched token — and handed to `Decimal::from_str`. `crates/data/tests/fx_rate_decimal_boundary.rs` pins this **two-sidedly**: a 20-significant-digit token detects the hop, and the *control* proves the measured `1.1165` token could not (both routes agree there), so nobody can replace the fixture with "the realistic one" and quietly make the guard vacuous. A third, structural leg forbids the string `f64` on any executable line of the module. **Gate probed:** reintroducing the hop turned the value guard and the structural guard red while the control stayed green; reverted.

**AC4 — the tests.** `crates/ui/tests/advisor_live_fx_rate.rs` drives the fake through the real bridge: (a) live success renders `live:ecb as of 2026-11-06` with `1.1165` and `$223.30`; (b) the forced-failure fake falls back to `config as of 2026-06-22` with `$216.00` and **no** residual live claim; (c) display == engine — the engine's `conversion.usdt()` and the displayed `fx_note().usdt` are the same number off one `BudgetConversion`.

**AD-10 — rendered-pixel proof.** `crates/ui/tests/advisor_live_fx_render.rs` renders the real screens headless (`iced_test::screenshot`, macOS-gated per ADR-0057) in the **populated** state with negative controls: the live note paints ~1482 px in the budget-line band against ~1232 px for the config note and ~1232 px for the no-note fallback, and a fourth guard asserts the live and config renders differ **only** inside that band (so the measurement is the label, not a layout shift). The PNGs were read by eye: the forward plan renders "€200 ≈ $223.30 (at 1.1165 EUR/USD, live:ecb as of 2026-11-06)" and the control renders the untouched "(at 1.08 EUR/USD, config)"; the pre-launch hint renders the live line with the teal "Refresh rate" affordance beside it. **Gate probed:** dropping the as-of from the formatter turned all three comparison guards red; reverted.

**Decisions taken inside the ADR's envelope (not re-litigations).**
1. *Where the refresh affordance lives.* ADR-0101 D7 names an "explicit operator refresh" but not its surface. It is on the **pre-launch** bake-off input panel, beside the budget hint. A refresh on a launched run's note would restate a rate the engine has already sized against — the one way display and engine could drift, which AC3 forbids. Before launch there is no engine, so re-quoting is free and the operator sees the rate the next run will convert at.
2. *`fmt_rate` widened from 2 dp to up to 4 dp (trailing zeros trimmed).* At 2 dp a live `1.1165` printed as `1.12`, and `200 × 1.12 = 224.00` does not reproduce the `$223.30` next to it — an honesty label whose arithmetic the operator cannot check. Configured 2-dp rates render exactly as before (`1.08` → `"1.08"`, `1.0` → `"1.00"`).
3. *The configured `as_of` is now shown.* `AdvisorConfig.eur_usd_rate_as_of` was threaded into `FxRate` but never rendered. One formatter now appends "as of ⟨label⟩" whenever a rate carries one, so a configured rate reads "config as of 2026-06-22" and an unlabelled one still reads the bare `"config"` — no shipped copy changes unless the operator had set a label.
4. *The bridge is `mpsc::sync_channel` + `recv_timeout`, not `Handle::block_on`.* `block_on` panics if it is ever reached from inside an async context; on the render path that would take the cockpit down, and the whole point of D5 is that a bad fetch costs a label, not the app.

**Anchor safety.** Nothing under `evidence/` was touched and no SHA literal in `crates/` changed; `verify_anchors.sh` printed 119/119 before the first edit and after the last. This is the ADR's "by construction" claim holding: the headless CLI path is USDT-denominated and never reads the rate.

**Out of scope, reported not fixed:** `crates/backtest/tests/report_prose_is_derived_not_asserted.rs` carries **pre-existing** `rustfmt` drift (a closure body rustfmt wants re-wrapped), so a workspace-wide `cargo fmt --all -- --check` is red on this branch independently of this story. It was reverted after `cargo fmt --all` touched it, to keep this commit scoped.

### File List

**New**
- `crates/data/src/fx_rate.rs`
- `crates/data/tests/fx_rate_decimal_boundary.rs`
- `crates/ui/src/live_fx.rs`
- `crates/ui/tests/advisor_live_fx_rate.rs`
- `crates/ui/tests/advisor_live_fx_render.rs`

**Modified**
- `crates/data/Cargo.toml` (serde_json `raw_value`), `crates/data/src/lib.rs` (module + re-exports)
- `crates/ui/Cargo.toml` (`live-fx` feature, added to `default`), `crates/ui/src/lib.rs`
- `crates/ui/src/bin/cockpit_live.rs` (the `RateSource` handle, `refresh_live_fx_rate` / `plan_fx_rate`, both forward seams)
- `crates/ui/src/state.rs` (`advisor_eur_usd_as_of`, `advisor_live_fx`, `Cockpit::resolved_fx_rate`, `Message::FxRateRefreshRequested`)
- `crates/ui/src/strings.rs` (`FX_PROVENANCE_WITH_AS_OF_FMT`, `LEADERBOARD_FX_REFRESH_LABEL`, both registered)
- `crates/ui/src/widgets/num.rs` (`fmt_fx_provenance`, `fmt_rate` precision)
- `crates/ui/src/widgets/bakeoff_input.rs` (hint takes an `FxRate`; the refresh affordance)
- `crates/ui/src/screens/{leaderboard,forward_plan,live}.rs`, `crates/ui/src/export/plan_export.rs` (provenance through the labels)
- `crates/ui/src/fixtures.rs` (`fake_cockpit_leaderboard_with_live_fx_rate`, `fake_cockpit_forward_plan_with_fx`), `crates/ui/src/gallery/routes.rs`
- `_bmad-output/planning-artifacts/trace.toml`, `_bmad-output/implementation-artifacts/sprint-status.yaml`, `CHANGELOG.md`
