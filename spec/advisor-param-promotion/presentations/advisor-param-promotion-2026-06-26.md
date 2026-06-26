---
slug: advisor-param-promotion
mode: release
status: draft
audience: human-operator
updated: 2026-06-26
generated: 2026-06-26T07:42:00Z
---

# Promotion wiring — launch a tuned config into the forward €200 paper-trade (release)

## TL;DR
The Tune editor's "Use this config" button now actually works — it launches the
strategy **you tuned** into a forward €200 paper-trade — but ONLY for configs
that cleared the anti-overfit gate; an overfit (FRAGILE) config stays locked and
can never be promoted.

## What changed
- **The button became real.** On the Tune results grid, "Use this config" was
  a decorative label (it carried no action). It is now a live button on
  promotable rows: clicking it carries that tuned strategy into the existing
  forward paper-trade and shows you the plan before your €200 starts.
- **The gate is the gatekeeper.** A FRAGILE row (judged overfit by the frozen
  robustness gate) shows a greyed "Fragile — locked" label with no button. The
  feature READS the gate's verdict; it never recomputes or softens it. Marginal
  and Robust configs promote, exactly as the gate already allowed.
- **What runs is what was scored.** The promoted run executes the byte-identical
  strategy the sweep graded — same params, same rules in the forward plan (e.g.
  the plan describes the tuned "10/20" crossover, not the default "20/50"). No
  silent drift between what you tuned, what you see, and what paper-trades.

## Why
This product exists to AVOID overfitting footguns, and the whole-session finding
(stress-tested across long windows, strategy combinations, shorts, and now
parameter tuning) is blunt: **no active strategy robustly beats just holding.**
The earlier feature (advisor-param-tuning / ADR-0069) lets you sweep a strategy's
parameters and grades each one through a frozen robustness gate that flags overfit
configs as FRAGILE. That feature deliberately left the "carry it forward" step
unwired. This feature (ADR-0070) is that step: it lets you paper-trade a config
**you tuned that at least cleared the gate on its window** — framed honestly as
"survived resampling on THIS window, not a guarantee, not advice", never as a sure
thing. See [feature.md](../feature.md) and
[ADR-0070](../../architecture/adr/0070-promote-tuned-config-into-forward-paper-run.md).

## What you can do now

| Action | Command |
|--------|---------|
| Open the cockpit, sweep a strategy, promote a robust config | `cargo run --release -p ui --bin cockpit_live --features fixtures` (Tune → run sweep → click "Use this config" on a non-fragile row) |
| Re-prove the engine carries the tuned params (day-1 gates) | `cargo test -p agent --test forward_promotion_divergence` |
| Re-prove the click carries the config (the wiring, not just paint) | `cargo test -p ui --features fixtures --test promote_swept_config` |
| Re-generate the screenshots below | `cargo test -p ui --features fixtures --test forward_plan_populated_render --test param_sweep_render` |

## Live demo

This is an engine-seam feature; the load-bearing demo is the day-1 gate suite
that proves the tuned params actually reach the forward loop and that what runs
byte-equals what the sweep scored. Run verbatim:

```
$ cargo test -p agent --test forward_promotion_divergence
running 7 tests
test t6c_plan_none_path_emits_default_lens ... ok
test t6c_plan_reflects_tuned_sma_override ... ok
test t6a_sma_param_override_produces_divergent_signals ... ok
test t6b_rsi_agent_toml_byte_equals_sweep_generator ... ok
test t6b_macd_agent_toml_byte_equals_sweep_generator ... ok
test t6b_bbands_agent_toml_byte_equals_sweep_generator ... ok
test t6a_macd_param_override_produces_divergent_signals ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Read these two rows together: `t6a_sma_param_override_produces_divergent_signals`
proves a tuned override changes the actual signals (it is NOT a no-op button that
just paints accent — the v3-vol-overlay-noop lesson), and
`t6b_macd_agent_toml_byte_equals_sweep_generator` proves the promoted run uses the
exact strategy the gate scored (no fidelity drift). Raw stdout saved at
`spec/advisor-param-promotion/presentations/artifacts/advisor-param-promotion-2026-06-26/forward_promotion_divergence-stdout.txt`.

## Screenshots

The centerpiece — the promoted forward plan
([forward_plan_promoted_render.png](artifacts/advisor-param-promotion-2026-06-26/forward_plan_promoted_render.png)):

- The honesty header reads, verbatim: **"You tuned this SMA crossover config
  (fast 10 / slow 20). It survived resampling on 2024 H1 — that is not a
  guarantee, and not advice. Paper-trading your €200."**
- The Standing rules describe the **TUNED** params: "the 10-bar average crosses
  above the 20-bar average THEN buy" (10/20 — the tuned lens, NOT the default
  20/50).
- "Right now: Flat — no position" / "If it buys next" — the expected just-launched
  state. The forward run is real-time, so it is flat until the first live bar
  lands. This is correct behaviour, not a bug.
- The persistent "Not financial advice. The €200 is a simulated paper budget…"
  footer is still present.

The gate-tied affordance — the mixed sweep grid
([param_sweep_mixed_promote_render.png](artifacts/advisor-param-promotion-2026-06-26/param_sweep_mixed_promote_render.png)):

- Robust/Marginal rows carry an enabled accent **"use this config"** button.
- The FRAGILE row (`fast=15, slow=30`, Max-DD p95 52%, verdict "fragile") shows
  a greyed **"Fragile — locked"** label — no button. This is the anti-overfit
  lock: a fragile config can never promote.

A composed-family (MACD) sweep grid
([param_sweep_macd_populated_render.png](artifacts/advisor-param-promotion-2026-06-26/param_sweep_macd_populated_render.png))
— shows the same promote/locked discrimination works for the non-SMA families
that resolve via the shared in-memory TOML generators.

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| D1/D7a | Tuned params REACH the forward loop (not a silent no-op) | VERIFIED | `t6a_sma_param_override_produces_divergent_signals` + `t6a_macd_…` PASS (forward_promotion_divergence) — re-run by presenter, 7/7 |
| D2/D7b | What runs == what the gate scored (byte-identical TOML, shared generator + `from_str` guard) | VERIFIED | `t6b_macd/rsi/bbands_agent_toml_byte_equals_sweep_generator` PASS — presenter re-run |
| D3/D7c | The forward plan describes the TUNED rules, not defaults | VERIFIED | `t6c_plan_reflects_tuned_sma_override` PASS (emits SmaCross{tuned}); `t6c_plan_none_path_emits_default_lens` negative control PASS — presenter re-run |
| D4 | The click CARRIES the config (preseed + nav), not just paints accent | VERIFIED | `promote_swept_config` 3/3 PASS (`…preseeds_target_and_navigates`, `…maps_every_family_to_its_forward_id`, `…carries_the_swept_window_label`) — presenter re-run with `--features fixtures` |
| D5 | Only non-FRAGILE promotes; FRAGILE stays a locked label; gate untouched | VERIFIED | `sweep_promotable_use_config_is_enabled_accent_button` + `sweep_fragile_promote_disabled_accent_discriminator` PASS (param_sweep_render 9/9); screenshot shows fragile row locked — presenter re-run |
| D6 | Distinct honesty provenance header on a promoted plan | VERIFIED | `forward_plan_promoted_paints_provenance_and_tuned_rules` + `forward_plan_crowned_has_no_provenance_strip` PASS (forward_plan_populated_render 8/8); header copy legible in screenshot — presenter re-run |
| D8a | `None`-path byte-identical → anchors preserved | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` — presenter re-run |
| D8b | Rendered-PIXEL verification (CLAUDE.md non-negotiable for UI) | VERIFIED | param_sweep_render 9/9 + forward_plan_populated_render 8/8 PASS — presenter re-run; PNGs read by presenter |
| ADR-0069 | Identity guards still pass after generators made `pub` | VERIFIED (orchestrator) | `…_toml_shipped_params_round_trip` `--include-ignored` — orchestrator-reported; not separately re-run by presenter |
| Clippy | `--workspace --all-targets --features ui/live` compiles bin + tests | VERIFIED (orchestrator) | EXIT=0 — orchestrator-reported; not separately re-run by presenter (full `ui/live` build) |
| Smoke | cockpit-smoke (0 panics, 7s window) | VERIFIED (orchestrator) | Re-run with instrumentation: process PID 62908 **alive at 3s AND at 7s** (pgrep), then `pkill -9` found-and-killed it (exit 0) → the binary ran the full event-loop window, no early exit; **0 panic signatures**. Empty stderr = clean quiet boot (iced fixtures emits no tracing). Log: `spec/advisor-param-promotion/reports/cockpit-smoke-2026-06-26T07-42Z.log` |

## Numbers that matter
- Day-1 engine gates: **7 / 7** passed (forward_promotion_divergence) — presenter re-run.
- UI wiring test: **3 / 3** passed (promote_swept_config, `--features fixtures`) — presenter re-run.
- Render-pixel proofs: **9 / 9** (param_sweep_render) + **8 / 8** (forward_plan_populated_render) — presenter re-run.
- Anchors: **119 / 119** PASS — presenter re-run; the promotion `None`-path writes no file and stays byte-identical.
- New code (this feature): engine seam `69e2c06` (+844 lines: config/runtime/plan + a 506-line gate test), UI `1854615` (+1104 lines incl. 3 new test files), design `bf7e5c7`.
- Net new engine surface: ONE optional field (`ForwardRunConfig.param_override`), one agent enum, two resolver `Some`-branches, three generators promoted to `pub`. No new dependency; `cargo tree -p ui` unchanged.

## Open decisions

_None._ (The presenter's one open item — the empty cockpit-smoke log — was
RESOLVED by the orchestrator: re-run with instrumentation confirmed the cockpit
process stayed alive the full 7s window with 0 panics; the Smoke row above is now
VERIFIED and cites the non-empty log `cockpit-smoke-2026-06-26T07-42Z.log`.)

Note (not a decision, scope honesty): v0.1 promotion = "launch the tuned config
forward". It carries no replay window (`lookback: None`, same as the crowned-pick
path), so the plan reads "Flat — pending first bar" at launch until a live bar
arrives — expected. A replay-preview window and a promotion-history trail are
explicitly deferred (ADR-0070 Consequences).

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback
<empty until operator fills>

## Changelog
- 2026-06-26 (presenter): initial release deck — TL;DR + anti-overfit framing,
  the promoted-plan + mixed-grid + MACD-grid screenshots, the day-1 gate demo,
  the D1–D9 verification matrix (all presenter-re-run except clippy/ADR-0069
  guards, attributed orchestrator-verified), and one open decision (the empty
  cockpit-smoke log).
