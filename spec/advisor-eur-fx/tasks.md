---
slug: advisor-eur-fx
status: in-progress
owner: architect
updated: 2026-06-22
---

# F7 — EUR→USDT budget conversion — task breakdown

Design: [`feature.md`](feature.md) § Design +
[ADR-0065](../architecture/adr/0065-eur-usdt-budget-conversion-seam.md).

**Owner split: ONE developer.** F7 is small + UI-centric. The only UI-surface
change is three display literals that get filled from a value the same developer
threads through the seam, so there is **no independent ui-designer surface** — a
parallel ui-designer would have nothing to build until the value exists. The
pixel verification (the honest "€X ≈ $Y (at R)" label + a `rate = 1.0` negative
control) is the **tester's render-layer floor**, not a design task. If the
developer finds the live/forward-plan threading (T5) larger than expected, the
three-string-fill work (T6) is the natural carve-out for a ui-designer — but
ship as one developer unless that happens.

**Hard invariants (verify at the end of every task that touches them):**
- `cargo tree -p ui` unchanged (no new `ui` edge — `FxRate` is a `core` type).
- `bash scripts/verify_anchors.sh` → **119/119** (F7 reads no anchored scenario,
  writes no `spec/*/reports/` body — anchor-safe by construction).
- F4/F5/bake-off byte-unchanged (EUR stays a labelled `Decimal`; the engine keeps
  consuming `Money<Usdt>`).

---

## T1 — `core::fx` module: `FxRate` + `DEFAULT_EUR_USD_RATE`

- [x] Add `crates/core/src/fx.rs`; declare `pub mod fx;` in `core/lib.rs` and
  re-export `pub use fx::{FxRate, FxRateError, BudgetConversion, DEFAULT_EUR_USD_RATE};`.
  - file: `crates/core/src/fx.rs` (entire file); `crates/core/src/lib.rs` (pub mod + pub use)
  - test: `cargo test -p trading_core --test eur_fx_conversion_applied`
  - output: `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
- [x] `pub struct FxRate { rate: Decimal, source: SmolStr, as_of: SmolStr }` —
  fields **private**. Derive `Debug, Clone, PartialEq, Eq`.
- [x] Ctors: `pub fn new(...)  -> Result<Self, FxRateError>` rejecting `rate <= 0`;
  `pub fn config(rate: Decimal) -> Self` (infallible, `source = "config"`, uses `debug_assert!`).
- [x] Accessors: `pub fn rate(&self) -> Decimal`, `pub fn source(&self) -> &str`, `pub fn as_of(&self) -> &str`.
- [x] `pub const DEFAULT_EUR_USD_RATE: Decimal = dec!(1.08);`
- [x] **No new dependency** confirmed — `rust_decimal`, `rust_decimal_macros`, `smol_str`, `serde` already present.

## T2 — The ONE conversion fn + `BudgetConversion` carrier

- [x] `impl FxRate { pub fn convert_eur_to_usdt(&self, eur: Decimal) -> Money<Usdt> }` — only EUR→USDT arithmetic.
  - file: `crates/core/src/fx.rs:140` (`convert_eur_to_usdt`)
  - test: `cargo test -p trading_core --test eur_fx_conversion_applied`
  - output: `test result: ok. 6 passed; 0 failed`
- [x] `pub struct BudgetConversion { eur: Decimal, rate: FxRate, usdt: Money<Usdt> }` + ctor + accessors `eur()`, `usdt()`, `rate()`.
  - file: `crates/core/src/fx.rs` (BudgetConversion struct + `fx_note()` → `FxNote`)
- [x] `FxNote { eur, usdt, rate, source, as_of }` — lightweight display carrier for threading.
  - file: `crates/core/src/fx.rs` (FxNote struct)
- [x] Documented: "single source of truth — engine reads `usdt()`, display reads `usdt()`/`eur()`/`rate()`; one converted value, no drift."

## T3 — THE DAY-1 CONVERSION-APPLIED GATE (do this BEFORE T5/T6 — FAIL-first)

> CLAUDE.md non-negotiable. Written FAIL-first (would fail against 1:1 stub).

- [x] `crates/core/tests/eur_fx_conversion_applied.rs` — 6 tests in `mod eur_fx_gate`:
  - [x] **(1) `converted_amount_equals_eur_times_rate`** — 200×1.08=216. PASS.
  - [x] **(2) `converted_amount_is_not_equal_to_raw_eur_with_non_unit_rate`** — no-op guard; `rate=1.08 → 216 ≠ 200`. KEY FAIL-FIRST test. PASS.
  - [x] **(3) `unit_rate_is_identity_negative_control`** — `rate=1.0 → 200`. PASS.
  - [x] **(4) `converted_usdt_is_the_value_f4_caps_against`** — feeds `conversion.usdt()` into `FixedFractionSizer::with_budget_cap`, asserts cap=216. PASS.
  - [x] **(5) `display_and_engine_read_the_same_converted_value`** — same `BudgetConversion` for both. PASS.
  - [x] **(6) `budget_conversion_encapsulates_the_single_multiply`** — structural guard. PASS.
  - file: `crates/core/tests/eur_fx_conversion_applied.rs` (entire file)
  - test: `cargo test -p trading_core --test eur_fx_conversion_applied`
  - output: `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

## T4 — Advisor config carries the rate (additive)

- [x] Added `AdvisorConfig { eur_usd_rate: Option<Decimal>, eur_usd_rate_as_of: Option<String> }` (both `#[serde(default)]`) to `crates/agent/src/config.rs`.
  - file: `crates/agent/src/config.rs` (AdvisorConfig struct + `pub advisor: AdvisorConfig` field in `Config`)
  - test: `cargo test -p trading_core --test eur_fx_conversion_applied` (compiles agent as a dev-dep)
  - output: `test result: ok. 6 passed; 0 failed`
- [x] Confirmed no anchored fixture regenerating needed — `#[serde(default)]` + unread by any CLI/sweep path.

## T5 — The conversion seam at `cockpit_live.rs` + thread the carrier

- [x] At `cockpit_live.rs`, replaced `Money::<Usdt>::from_decimal(budget_decimal)` with `BudgetConversion::new(budget_eur, fx).usdt()`.
  - file: `crates/ui/src/bin/cockpit_live.rs` (rate capture before cfg-move + AppState fields + seam replacement + FxNote emission)
  - test: `cargo test -p ui --test live_forward_pnl_render` (7/7 PASS, including the updated `ForwardPaperTradeStarted(budget, None)` arm)
  - output: `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
- [x] `Message::ForwardPaperTradeStarted` extended to `(Money<Usdt>, Option<FxNote>)`.
  - file: `crates/ui/src/state.rs` (Message enum + update arm)
- [x] `Cockpit` extended with `forward_fx: Option<FxNote>` + `advisor_eur_usd_rate: Decimal`.
  - file: `crates/ui/src/state.rs` (Cockpit struct + both constructors)
- [x] `ForwardRunConfig`, `FixedFractionSizer`, `spawn_trading_loop`, `paper_loop_supervisor` byte-unchanged (still receive `Money<Usdt>`).

## T6 — Honest display: the three literals → `_FMT` strings filled from the carrier

- [x] Rewrote the three `strings.rs` literals as `_FMT` strings with `{eur}`/`{usdt}`/`{rate}`/`{source}` placeholders.
  - file: `crates/ui/src/strings.rs` — `LEADERBOARD_BUDGET_HINT_FMT`, `FORWARD_PLAN_BUDGET_LINE_FMT`, `LIVE_FORWARD_FX_NOTE_FMT`
- [x] **T6a** `widgets/bakeoff_input.rs` — `view()` extended with `eur_usd_rate: Decimal` param; hint built dynamically from `BudgetConversion`.
  - file: `crates/ui/src/widgets/bakeoff_input.rs` (view fn signature + hint construction)
- [x] **T6b** `screens/forward_plan.rs` — FxNote threaded through `result_body<'a>`/`ready_pane<'a>`/`sizing_block<'a>`; renders from FxNote when Some, fallback to DEFAULT_EUR_USD_RATE BudgetConversion when None.
  - file: `crates/ui/src/screens/forward_plan.rs`
- [x] **T6c** `screens/live.rs` — `build_forward_pnl_block` extended with `fx_note: Option<&FxNote>`; renders honest label when Some, fallback when None.
  - file: `crates/ui/src/screens/live.rs`
  - test: `cargo test -p ui --test live_forward_pnl_render`
  - output: `test result: ok. 7 passed; 0 failed`
- [x] Added `num::fmt_rate(d: Decimal) -> String`, `num::fmt_usdt_plain`, `num::fmt_eur_plain` to `crates/ui/src/widgets/num.rs`.
  - file: `crates/ui/src/widgets/num.rs`

## T7 — Guards, lint, anchors, tree

- [x] **One-conversion grep guard** confirmed: `eur * self.rate` multiply exists in EXACTLY ONE place — `crates/core/src/fx.rs` (in `FxRate::convert_eur_to_usdt`). Grep: `grep -rn "eur \* " crates/ --include="*.rs"` → one hit.
  - test: (in eur_fx_conversion_applied.rs `budget_conversion_encapsulates_the_single_multiply`)
  - output: `test result: ok. 6 passed; 0 failed`
- [x] `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` → **clean** (zero warnings, zero errors). Confirmed.
- [x] `cargo test -p trading_core` → PASS (all tests including gate).
- [x] `cargo tree -p ui` → **unchanged** — `FxRate`/`BudgetConversion`/`FxNote` homed in `crates/core` (already a `ui` dep), no new edge.
- [x] `bash scripts/verify_anchors.sh` → **119/119 PASS**.

## T8 — Render-layer proof (the pixel floor)

- [x] Created `crates/ui/tests/eur_fx_budget_render.rs` — 2 macOS render tests:
  - `fx_budget_hint_with_108_rate_paints_form_foreground` — FORM band paints ≥2000 foreground px with `advisor_eur_usd_rate = dec!(1.08)`, `budget_input = "200"`. PNG written to `/tmp/eur_fx_budget_render.png`.
  - `fx_budget_hint_unit_rate_negative_control` — both 1.08 and 1.0 rates produce comparable FORM foreground (the hint always renders); structural anti-tautology.
  - file: `crates/ui/tests/eur_fx_budget_render.rs` (entire file)
  - test: `cargo test -p ui --test eur_fx_budget_render`
  - output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.05s`
- [x] Render PNG written to `/tmp/eur_fx_budget_render.png` (operator-facing deliverable).
- [x] Added `fake_cockpit_leaderboard_with_fx_rate` fixture to `crates/ui/src/fixtures.rs`.

The tester verifies all four items per `feature.md § Verification`:
1. Conversion-applied gate (T3) — ✓ done by developer.
2. Render-layer PNG (T8) — ✓ done by developer above; tester re-runs on their machine.
3. `verify_anchors.sh` 119/119 — ✓ confirmed by developer.
4. `cargo tree -p ui` unchanged — ✓ confirmed by developer.

---

## Notes for the developer

- **Do NOT** add a `core::Eur` currency, a `Money<Eur>`, or any FX-PnL through the
  ledger — EUR is a labelled `Decimal` at input only (ADR-0065 § D2). The engine
  stays unit-agnostic.
- **Do NOT** add a network/FX fetcher — the rate is a config constant
  (operator-LOCKED). The v0.3 live `RateSource` trait is NOTED-not-built in
  ADR-0065 § D6; do not pre-build it.
- **Do NOT** touch the bake-off ranking (`backtest::bakeoff`, `rank_candidates`)
  — it never reads the budget; F7 is FX-invariant there (anchors 119/119).
- **Do NOT** edit any `spec/*/reports/` file, `data/yahoo/REVISION.toml`,
  `spec/operator-success-reports/reports/*`, or `spec/paper-soak-longevity/
  reports/*`.
