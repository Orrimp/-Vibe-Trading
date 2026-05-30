---
date: 2026-05-30
author: architect-agent (claude-opus-4-8)
slug: engine-drift-fix-spec-2026-05-30
status: APPROVED — dev spec ready; HANDOFF → developer
answers: spec/dev-notes/engine-drift-diagnosis-2026-05-30.md (diagnosis commit 1cbe3d4)
adr: ADR-0045 § D6 + § D7 (amendment 2026-05-30)
---

# Engine-Drift Fix — Architect Decision + Developer Spec (2026-05-30)

## Verdict

**PAPERWORK confirmed.** The `Q-D1=(a)` 0→8 bps synthetic-slippage change
(`7e8a7e0`, operator-ratified) is correct engine behaviour. No engine
code changes. The 14 failing `determinism.rs` tests hold **stale
`noop-baseline` SHAs**; the correct (8 bps) SHAs are already committed in
`spec/anchors.toml` under `v5-realdata-medium-2026-05`. We reconcile the
in-test constants to those canonical SHAs and add a drift-gate so this
cannot recur silently.

Architecture decision recorded in **ADR-0045 § D6 + § D7** (amendment).

---

## Decision 1 — APPROVED: re-lock the 14 in-test constants

**Why this is not a new anchor decision.** `spec/anchors.toml` is the
single source of truth (ADR-0045 § D6.1). The `determinism.rs` constants
are a *derived projection* of it. Updating a stale `noop-baseline` SHA to
the already-committed `v5-realdata-medium-2026-05` SHA reconciles a
duplicate to its approved source — it does NOT trigger the ADR-0038
§ D6.b re-emission protocol (no saved file mutates; the canonical
file-anchor was ratified at `7e8a7e0`). Architect sign-off is given here
per the anchors.toml owner policy.

### Mapping derivation (verified)

The non-feature-gated `determinism.rs` tests build the binary with **no
`realdata` / `candle` feature**. In `build_slippage_model_for_scenario`
(`crates/backtest/src/main.rs:195`), every scenario takes the
`Linear { bps: 8 }` synthetic fallback when `realdata` is absent
(`REAL_DATA_SCENARIO_IDS` only holds `-realdata` IDs, gated behind
`#[cfg(feature = "realdata")]`). So each in-test constant for scenario
*S* must equal the `spec/anchors.toml` row
`scenario = S, version = "<base> + v5-realdata-medium-2026-05"`.

There are **16** `const ANCHOR`/`ANCHOR_PREFIX` sites in the file.
**14 must change** (the non-feature-gated synthetic tests). **2 must NOT
change**: the `#[cfg(feature = "candle")]` `m3_*_weights` constants at
lines 809 + 827 — they do not run in the default suite, are not failing,
and have no `v5-realdata-medium` mapping in scope.

> **t622 vs t717 do NOT differ** for any scenario. Both families run the
> same scenario at the same seed with the same (no-feature) binary, so
> the expected SHA is identical across the two families. (t622 historically
> used 8-char prefixes for macd/rsi/bbands; t717 used full 64-char. After
> this fix, replace the t622 prefixes with the full 64-char value to make
> both families full-hash and identical — see EX-1.)

### EX-1 — The 14 constant edits (exact)

Each row: `determinism.rs` line, test fn, scenario, current (stale)
literal → new literal. **New literal = full 64-char SHA from
`spec/anchors.toml` under `v5-realdata-medium-2026-05`.** Replace the
whole `const` literal; for the three t622 `ANCHOR_PREFIX` sites,
**also change the `const` name to `ANCHOR` and the `assert!(hex.starts_with(...))`
to `assert_eq!(hex, ANCHOR, ...)`** so t622 becomes a full-hash equality
gate identical in shape to t717 (EX-2 covers the assertion edit).

| # | Line | Test fn | Scenario | Stale literal | New literal (64-char) |
|---|------|---------|----------|---------------|------------------------|
| 1 | 505 | `t622_sma_cross_anchor_hash_unchanged` | btc-2023-1m-sma-cross | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0` |
| 2 | 518 | `t622_sma_baseline_refresh_anchor_hash_unchanged` | btc-2023-1m-sma-baseline-refresh | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0` |
| 3 | 533 | `t622_macd_trend_anchor_hash_unchanged` | btc-2023-1m-macd-trend | `ef9c5e48` (PREFIX) | `6cb14ac55350325c2785284f6e9a8db29693def83a31b144e1d4607f5baf53f5` |
| 4 | 548 | `t622_rsi_reversion_anchor_hash_unchanged` | btc-2023-1m-rsi-reversion | `bc56d20d` (PREFIX) | `87b4e1cc1b949a5b60420bf4fa2319e40035a57de6590d8b8987eb5357845695` |
| 5 | 563 | `t622_bbands_mean_revert_anchor_hash_unchanged` | btc-2023-1m-bbands-mean-revert | `d8a08a23` (PREFIX) | `5b6237d11f962b98e9ce0f0deb4b7ec7d7638bbcb15f5e418f3909f07a3393cd` |
| 6 | 597 | `t717_sma_cross_anchor_hash_unchanged` | btc-2023-1m-sma-cross | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0` |
| 7 | 609 | `t717_sma_baseline_refresh_anchor_hash_unchanged` | btc-2023-1m-sma-baseline-refresh | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0` |
| 8 | 621 | `t717_macd_trend_anchor_hash_unchanged` | btc-2023-1m-macd-trend | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` | `6cb14ac55350325c2785284f6e9a8db29693def83a31b144e1d4607f5baf53f5` |
| 9 | 633 | `t717_rsi_reversion_anchor_hash_unchanged` | btc-2023-1m-rsi-reversion | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` | `87b4e1cc1b949a5b60420bf4fa2319e40035a57de6590d8b8987eb5357845695` |
| 10 | 645 | `t717_bbands_mean_revert_anchor_hash_unchanged` | btc-2023-1m-bbands-mean-revert | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` | `5b6237d11f962b98e9ce0f0deb4b7ec7d7638bbcb15f5e418f3909f07a3393cd` |
| 11 | 660 | `t717_top10_2023_momentum_anchor_hash_unchanged` | top10-2023-1h-momentum | `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97` | `0f6f6eb8d943fefa866c4883be034f1beb3caff169fe76ec73bf3c29041a8ba3` |
| 12 | 675 | `t717_top10_2024_momentum_anchor_hash_unchanged` | top10-2024-h1-momentum | `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6` | `78976062cf3d62b9bbb2ab579e91822cb49f0d12464dedf912edb427e66c7490` |
| 13 | 704 | `tt1_top10_2023_fy_tcn_overlay_anchor_hash_unchanged` | top10-2023-fy-tcn-overlay | `01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5` | `1460fcc70029746b650ae6f1298a7f2291603e96c54531f26bf6f24c558250fc` |
| 14 | 716 | `tt1_top10_2024_fy_tcn_overlay_anchor_hash_unchanged` | top10-2024-fy-tcn-overlay | `e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163` | `b8e9186bb36abe6539917245f7dec99685792dcc955e11ba52380a7a5293ad1e` |

> **Why rows 13-14 are in scope (they are NOT in the diagnosis table).**
> The diagnosis prose says "14 tests" but its blast-radius table lists
> only 12 (the t622/t717 set). The 2 missing are the `tt1_*` TCN-overlay
> passthrough tests: `top10-2023-fy-tcn-overlay` /
> `top10-2024-fy-tcn-overlay` are **synthetic** (the `-realdata` variants
> are the realdata ones; the bare names are NOT in `REAL_DATA_SCENARIO_IDS`),
> candle is absent in the default binary, so they also take the
> `Linear{bps:8}` fallback and drifted to `1460fcc7…` / `b8e9186b…`.
> 12 + 2 = the 14 the operator brief counts. **Developer: confirm rows
> 13-14 actually fail before editing** (see VR-0); if for any reason they
> pass unchanged, STOP and report — that would mean the TCN passthrough
> path does not route through the slippage fallback and the mapping needs
> review.

### EX-2 — t622 assertion shape change (rows 3, 4, 5 only)

For the three t622 prefix tests, convert prefix→full-hash equality so
the gate is exact and matches t717. Example for macd (apply the
analogous edit to rsi line 548 and bbands line 563):

```rust
// BEFORE (t622_macd_trend_anchor_hash_unchanged):
const ANCHOR_PREFIX: &str = "ef9c5e48";
let hex = scenario_body_hex("btc-2023-1m-macd-trend");
assert!(
    hex.starts_with(ANCHOR_PREFIX),
    "T622 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
     Expected prefix: {ANCHOR_PREFIX}\n\
     Got:             {hex}"
);

// AFTER:
const ANCHOR: &str = "6cb14ac55350325c2785284f6e9a8db29693def83a31b144e1d4607f5baf53f5";
let hex = scenario_body_hex("btc-2023-1m-macd-trend");
assert_eq!(
    hex, ANCHOR,
    "T622 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
     Expected: {ANCHOR}\nGot:      {hex}"
);
```

Also update the comment block at lines ~425-436 and ~530-563 that
documents the prefixes — replace the "starts_with" rationale with a one
-line note that t622 is now full-hash equality re-locked to the
`v5-realdata-medium-2026-05` namespace (cite ADR-0045 § D6).

### EX-3 — DO NOT TOUCH

- `crates/backtest/tests/determinism.rs` lines **809** + **827**
  (`m3_*_weights`, `#[cfg(feature = "candle")]`) — not in the failing
  set, no `v5-realdata-medium` mapping in scope.
- `spec/anchors.toml` — already correct; the file-anchors are canonical.
  The developer must NOT edit it for this fix.
- `crates/backtest/src/{engine,main}.rs`,
  `crates/backtest/src/scenarios/*` — no engine change (PAPERWORK).

---

## Decision 2 — APPROVED: close the blind-spot (ADR-0045 § D7)

Synthesis of options (b) + (a), priority order. Full rationale +
rejected option (c) in ADR-0045 § D7.

### EX-4 (D7.1, PRIMARY) — `scripts/check_determinism_anchors.py`

New script. Sub-second, **no engine execution**. Mirrors the existing
`scripts/adr_registry_check.py` drift-linter pattern.

Behaviour:
1. Parse `spec/anchors.toml`; build a dict
   `{scenario: sha}` for every row whose `version` contains
   `v5-realdata-medium-2026-05`.
2. Parse `crates/backtest/tests/determinism.rs`; for each
   non-feature-gated `*_anchor_hash_unchanged` test fn (the `t622_*`,
   `t717_*`, `tt1_*` families — i.e. every `const ANCHOR`/`ANCHOR_PREFIX`
   site NOT inside a `#[cfg(feature = ...)]` fn), extract the
   `scenario_body_hex("<scenario>")` argument and the `const` literal.
3. For each in-test site, assert the literal **equals** the
   `v5-realdata-medium-2026-05` SHA for that scenario.
4. On any mismatch: exit 1 and print a markdown drift table
   (`scenario | in-test (file:line) | anchors.toml | match?`).
5. `--write` mode: rewrite the in-test literals in place to the
   anchors.toml SHAs (so reconciliation is mechanical, never hand-typed).
6. `--pre-commit` flag: same as default check but only when
   `determinism.rs` or `anchors.toml` is staged (cheap no-op otherwise).

Feature-gate handling: the script SKIPS any `const` site inside a
`#[cfg(feature = ...)]` fn (the `m3_*` candle pair) — those have no
default-binary `v5-realdata-medium` mapping. Detect by walking the fn's
attribute lines.

Acceptance: after EX-1/EX-2 land, `python3 scripts/check_determinism_anchors.py`
exits 0. Add a deliberately-wrong constant in a scratch test → exits 1
with the drift table → revert.

### EX-5 (D7.2, SECONDARY) — enforce the re-run gate the tester runs

Amend `.claude/skills/verify-anchors/SKILL.md`: after `verify_anchors.sh`
exits 0 (and the existing prune step), add a step that runs the synthetic
re-run determinism tests in **release** mode:

```bash
# D7.2 — engine-drift re-run gate (release; runs after verify_anchors.sh PASS).
cargo test --release -p backtest --test determinism -- t622_ t717_ tt1_
```

- **Release is mandatory** for cost (see § Cost). Debug is multi-minute;
  release is < ~1 min total after the one-time build.
- Scope is the `verify-anchors` gate only (already the documented
  pre-`VERDICT → PASS` gate for strategy/audit/exec/backtest changes) —
  NOT every `cargo test`. The `rust-test` skill's full
  `cargo test --workspace` already includes these tests in debug; this
  D7.2 step just guarantees the tester *runs and surfaces* them on
  exactly the changes that can move engine output.
- Also add a one-line step to the same skill: run
  `python3 scripts/check_determinism_anchors.py` (D7.1) BEFORE the
  re-run (fail-fast, sub-second).

### EX-6 (D7.3) — document the dual-anchor model

- `spec/anchors.toml` header comment: add a short paragraph stating
  there are TWO regression systems (file-anchors here, hashed by
  `verify_anchors.sh`; in-test re-run anchors in `determinism.rs`), that
  THIS file is canonical, and that the in-test constants for the
  no-feature default binary mirror the `v5-realdata-medium-2026-05`
  rows (cite ADR-0045 § D6).
- `spec/architecture.md` § "Regression gate discipline" (or
  "v1.5a regression-gate discipline" if that is the live heading):
  add the dual-system model + the D7.1/D7.2 gates + the D6.1 mapping
  rule. Use `spec-update` (architect-owned file; bump `updated:` +
  Changelog line).

---

## Verification plan (for the tester; developer self-checks VR-0..VR-2 first)

- **VR-0 (pre-edit, developer):** `cargo test -p backtest --test determinism`
  on the **current** tree → confirm exactly **14** failures and that
  rows 13-14 (`tt1_*`) are among them. Capture the failing list. If the
  count ≠ 14 or `tt1_*` are not failing, STOP and report (mapping needs
  review before editing).
- **VR-1 (post-edit):** `cargo test --release -p backtest --test determinism -- t622_ t717_ tt1_`
  → all green (the 14 + the t521 determinism pair + t33 all pass).
- **VR-2 (drift-linter):** `python3 scripts/check_determinism_anchors.py`
  → exit 0; then a scratch wrong-value run → exit 1 with drift table → revert.
- **VR-3 (file-anchor invariant, negative):** `scripts/verify_anchors.sh`
  → still `ANCHORS PASS (86/86)`. This fix touches NO saved report file
  and NO `anchors.toml` row; the file-anchor gate must be byte-identical
  before and after.
- **VR-4 (validate):** `cargo fmt --all -- --check` + `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` clean
  (the prefix→equality edit + new script must not introduce warnings).

---

## Risks

- **R1 — momentum SHA double-check.** Rows 11-12 map to `0f6f6eb8…` /
  `78976062…`. These were the values the diagnosis read live AND they
  are the committed `v5-realdata-medium-2026-05` rows. VR-1 is the
  arbiter — if the live re-run yields a different SHA, it means the
  engine moved AGAIN since the diagnosis; STOP and route back.
- **R2 — tt1 inclusion.** If VR-0 shows `tt1_*` passing unchanged, the
  TCN passthrough path is not routing through the slippage fallback;
  rows 13-14 would then be out of scope and the diagnosis "14" count is
  wrong. Report rather than force the edit.
- **R3 — D7.1 parser scope.** The script must reliably skip
  `#[cfg(feature = ...)]` fns (the candle `m3_*` pair). A naive
  line-by-line parse that ignores attributes would wrongly flag them.
  Test against the real file in VR-2.
