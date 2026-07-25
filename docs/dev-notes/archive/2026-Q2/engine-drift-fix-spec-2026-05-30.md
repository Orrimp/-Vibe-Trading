---
date: 2026-05-30
author: architect-agent (claude-opus-4-8)
slug: engine-drift-fix-spec-2026-05-30
status: AMENDED 2026-05-30 — BLOCKER resolved (mixed-provenance namespace); rows 3/4/5/8/9/10 corrected to SYNTHETIC SHAs + D7.1 dual-map adjustment; HANDOFF → developer
answers: docs/dev-notes/engine-drift-diagnosis-2026-05-30.md (diagnosis commit 1cbe3d4)
adr: ADR-0045 § D6 + § D7 (amendment 2026-05-30; § D6.3 + § D7.1b added for mixed-provenance)
---

# Engine-Drift Fix — Architect Decision + Developer Spec (2026-05-30)

> **BLOCKER RESOLUTION (2026-05-30, second architect pass).** The developer
> correctly STOPPED at VR-1: rows 3/4/5/8/9/10 (macd/rsi/bbands × t622+t717)
> did not match their EX-1-mapped `v5-realdata-medium-2026-05` SHAs. Root
> cause confirmed: the `v5-realdata-medium-2026-05` namespace is
> **mixed-provenance** — sma/momentum/tt1 entries are synthetic-run SHAs, but
> macd/rsi/bbands entries are **real-Binance-run SHAs** (the v0.3.0 re-emission
> machine had `btc-2023-1m` parquet present and those 3 scenarios were NOT
> `--force-synthetic-bars`). The `determinism.rs` tests are **pure synthetic
> re-run guards** (`current_dir(tempdir)` → data-path lookup misses → v0
> fallback). So the correct in-test constant for the 6 is the **synthetic**
> SHA, which is not in `anchors.toml`. Decision: re-lock the 6 to the synthetic
> SHAs (§ "BLOCKER resolution" below) and teach D7.1 a synthetic-override map
> (§ EX-4 v2). See ADR-0045 § D6.3 + § D7.1b. **Rows 1/2/6/7/11/12/13/14
> are unchanged and already GREEN.**

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
| 3 | 533 | `t622_macd_trend_anchor_hash_unchanged` | btc-2023-1m-macd-trend | `ef9c5e48` (PREFIX) | ~~`6cb14ac5…`~~ → **`4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a`** (SYNTHETIC — see BLOCKER resolution) |
| 4 | 548 | `t622_rsi_reversion_anchor_hash_unchanged` | btc-2023-1m-rsi-reversion | `bc56d20d` (PREFIX) | ~~`87b4e1cc…`~~ → **`4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7`** (SYNTHETIC) |
| 5 | 563 | `t622_bbands_mean_revert_anchor_hash_unchanged` | btc-2023-1m-bbands-mean-revert | `d8a08a23` (PREFIX) | ~~`5b6237d1…`~~ → **`5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894`** (SYNTHETIC) |
| 6 | 597 | `t717_sma_cross_anchor_hash_unchanged` | btc-2023-1m-sma-cross | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0` |
| 7 | 609 | `t717_sma_baseline_refresh_anchor_hash_unchanged` | btc-2023-1m-sma-baseline-refresh | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0` |
| 8 | 621 | `t717_macd_trend_anchor_hash_unchanged` | btc-2023-1m-macd-trend | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` | ~~`6cb14ac5…`~~ → **`4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a`** (SYNTHETIC — see BLOCKER resolution) |
| 9 | 633 | `t717_rsi_reversion_anchor_hash_unchanged` | btc-2023-1m-rsi-reversion | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` | ~~`87b4e1cc…`~~ → **`4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7`** (SYNTHETIC) |
| 10 | 645 | `t717_bbands_mean_revert_anchor_hash_unchanged` | btc-2023-1m-bbands-mean-revert | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` | ~~`5b6237d1…`~~ → **`5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894`** (SYNTHETIC) |
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

### BLOCKER resolution — the mixed-provenance namespace (rows 3/4/5/8/9/10)

**What the first architect pass got wrong.** EX-1 originally mapped all 14
constants to their `v5-realdata-medium-2026-05` `anchors.toml` SHAs on the
assumption that the no-feature default binary reproduces those SHAs for every
scenario. That holds for 8 rows but NOT for macd/rsi/bbands.

**Evidence (architect re-verified, 2026-05-30).** The saved `v5-realdata-medium`
report files that `anchors.toml` points to declare their own data source in the
front-matter, and they are NOT uniform:

| Scenario | `anchors.toml` v5 SHA | Saved-file `data_source` | Bars replayed |
|----------|-----------------------|--------------------------|---------------|
| btc-2023-1m-sma-cross / -baseline-refresh | `d2fa7616…` | `synthetic (seeded RNG, v0 fallback)` | 525601 |
| **btc-2023-1m-macd-trend** | `6cb14ac5…` | **`real (Binance Vision)`** | **17544** |
| **btc-2023-1m-rsi-reversion** | `87b4e1cc…` | **`real (Binance Vision)`** | **17544** |
| **btc-2023-1m-bbands-mean-revert** | `5b6237d1…` | **`real (Binance Vision)`** | **17544** |
| top10-2023/2024 momentum | `0f6f6eb8…`/`78976062…` | `synthetic (seeded RNG, v1 multi-symbol)` | — |

So the `v5-realdata-medium-2026-05` namespace literally lives up to its name for
macd/rsi/bbands (real Binance data) but is a misnomer for sma/momentum/tt1
(synthetic). Root cause: at the v0.3.0 re-emission (commit `21bda41`,
2026-05-27) the operator's box had `btc-2023-1m` Binance parquet on disk; the
SMA/Composed group was run `--force-synthetic-bars` (Q1=(a) revert, anchors.toml
header line 330) but macd/rsi/bbands were NOT forced and picked up the real
parquet → 17544-bar real-data bodies. The `anchors.toml` header line 330-332's
"Group A → synthetic" is over-broad; it did not name the macd/rsi/bbands
exception. This is now corrected in ADR-0045 § D6.3.

**The determinism.rs tests are pure synthetic guards.** `run_scenario_once`
(determinism.rs:467) spawns the binary with `.current_dir(tmp.path())`; the
binary resolves Binance parquet **relative to CWD**, so in a tempdir the lookup
always misses and the engine takes the v0 synthetic fallback (525600 bars) for
ALL scenarios — regardless of what data the operator's repo happens to hold.
The test's JOB is to guard the deterministic synthetic engine path. Therefore
the correct in-test constant for macd/rsi/bbands is the **synthetic** SHA.

**The 6 corrected synthetic SHAs (architect-verified by independent re-run,
CWD=tempdir, seed 0xC0FFEE — identical to the developer's VR-1):**

| Scenario | SYNTHETIC body-SHA (the new in-test constant) |
|----------|------------------------------------------------|
| btc-2023-1m-macd-trend | `4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a` |
| btc-2023-1m-rsi-reversion | `4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7` |
| btc-2023-1m-bbands-mean-revert | `5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894` |

Each value is used for BOTH the t622 and t717 test of that scenario (rows 3=8,
4=9, 5=10), exactly like the sma pair. These are stable across runs (architect +
developer agree).

**Why NOT add these as `anchors.toml` synthetic rows (option 2b rejected).**
`scripts/verify_anchors.sh` requires **every** `[[anchors]]` row to resolve to a
saved `.md` report on disk whose body hashes to the row's SHA (it keys
file-lookup on the `version` namespace string — see verify_anchors.sh:60-176,
and prints `MISS`+`fail=1` for any row with no matching file). There is NO saved
synthetic report file for macd/rsi/bbands — the only saved v5 files are the
real-data (17544-bar) ones. Adding 3 synthetic rows would therefore force
either (a) committing 3 brand-new synthetic report files (which then become
byte-immutable anchored artifacts forever, plus a new `verify_anchors.sh`
namespace branch, plus 86→89) or (b) leaving the rows unresolvable and breaking
VR-3 (86/86). Both are disproportionate to the goal. The in-test re-run gate
(`determinism.rs`) IS the synthetic regression guard for these scenarios; it
does not need a redundant file-anchor. See ADR-0045 § D7.1b for the rejection
rationale.

**Resolution (option 2a-refined): D7.1 carries an explicit synthetic-override
map.** D7.1 (`check_determinism_anchors.py`) gains a small, documented
`SYNTHETIC_DETERMINISM_SHAS` dict (the 3 entries above). For each in-test site
the script resolves the expected SHA as: (1) if the scenario is in the synthetic
override map, assert equality against that; (2) else assert equality against the
`v5-realdata-medium-2026-05` `anchors.toml` row; (3) if the scenario is in
neither, that is now a HARD ERROR (exit 1), not a silent skip — closing the
blind spot the old "(not in anchors.toml) → skip" branch would otherwise
reopen. This keeps D7.1's invariant uniform and meaningful for ALL 14:
every non-cfg-gated in-test constant has exactly one authoritative source of
truth, and a typo'd or drifted constant always fails the linter. See EX-4 v2.

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

// AFTER (note: SYNTHETIC SHA per BLOCKER resolution — the determinism test
// runs the v0 synthetic-fallback path, NOT the real-data path the
// v5-realdata-medium anchors.toml row was emitted from):
const ANCHOR: &str = "4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a";
let hex = scenario_body_hex("btc-2023-1m-macd-trend");
assert_eq!(
    hex, ANCHOR,
    "T622 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
     Expected: {ANCHOR}\nGot:      {hex}"
);
```

Apply the analogous prefix→equality edit to rsi (line 548 → `4a744788…`)
and bbands (line 563 → `5037accb…`) using the SYNTHETIC SHAs from the
BLOCKER-resolution table.

Also update the comment block at lines ~425-436 and ~530-563 (and the
matching t717 block ~583-594) that documents the prefixes / pending state.
Replace the "starts_with" + "PENDING orchestrator review" notes with: SMA /
momentum / tt1 rows are re-locked to the `v5-realdata-medium-2026-05`
namespace (synthetic == real-data SHA for those); macd / rsi / bbands rows
are re-locked to the **synthetic** SHAs because the determinism test exercises
the v0 synthetic-fallback path while the `v5-realdata-medium` row for those 3
was emitted from the real-data path (cite ADR-0045 § D6.3). The `--write`
auto-sync will NOT produce the correct value for these 3 (the map source is
the synthetic-override dict, not `anchors.toml`); hand-edit them or rely on
EX-4 v2's `--write` once the synthetic map is wired.

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

### EX-4 v2 (D7.1, PRIMARY) — `scripts/check_determinism_anchors.py` (dual-map)

The script EXISTS (developer implemented it 2026-05-30). It currently asserts
in-test == `v5-realdata-medium-2026-05` row for every non-cfg-gated site and
**skips** scenarios not in that map. Under the BLOCKER resolution the 6
synthetic sites are NOT in (and must not match) the `v5-realdata-medium` map,
so the script needs the following **adjustment** (the dev makes this edit):

**Add the synthetic-override map** near the top of the script, alongside
`CANONICAL_VERSION_SUFFIX`:

```python
# Scenarios whose determinism.rs constant is the SYNTHETIC (v0-fallback) body-SHA,
# NOT the matching v5-realdata-medium-2026-05 anchors.toml SHA. These v5 anchor
# rows were emitted from the REAL-DATA path (17544-bar Binance bodies); the
# determinism tests run the v0 synthetic fallback (525600 bars). See ADR-0045
# § D6.3 / § D7.1b and the engine-drift-fix BLOCKER resolution.
SYNTHETIC_DETERMINISM_SHAS: dict[str, str] = {
    "btc-2023-1m-macd-trend":        "4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a",
    "btc-2023-1m-rsi-reversion":     "4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7",
    "btc-2023-1m-bbands-mean-revert": "5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894",
}
```

**Resolution order** (in `detect_drift` and `apply_write`): for each
non-cfg-gated site, the expected SHA is —
1. `SYNTHETIC_DETERMINISM_SHAS[scenario]` if present (assert full equality);
2. else `canonical[scenario]` (the `v5-realdata-medium-2026-05` row, full
   equality; `ANCHOR_PREFIX` sites use `startswith` until EX-2 converts them);
3. else **HARD ERROR** — append to `mismatches` (not `skipped`) with note
   "no canonical OR synthetic mapping". This replaces the current
   `(not in anchors.toml) → skipped` branch (detect_drift lines ~239-250),
   which would otherwise silently pass an un-anchored constant and reopen the
   exact blind spot D7 exists to close. cfg-gated sites (`m3_*` candle pair)
   remain a legitimate skip (R3, unchanged).

**`--write` mode** must consult the same resolution order so auto-sync writes
the synthetic SHA for the 6 (currently `apply_write` only reads `canonical`,
lines ~304-312 — extend it to check `SYNTHETIC_DETERMINISM_SHAS` first).

Everything else (cfg-gate handling, `--pre-commit`, drift-table format,
`ANCHOR_PREFIX→ANCHOR` rename) is unchanged.

Acceptance: after EX-1/EX-2 land, `python3 scripts/check_determinism_anchors.py`
exits 0 reporting **14 literal(s) match** (8 canonical + 6 synthetic) and the
candle pair skipped. Negative tests: (a) corrupt one of the 6 synthetic
constants in a scratch edit → exit 1, drift table names the synthetic source
→ revert; (b) corrupt one sma/momentum constant → exit 1 against the canonical
source → revert; (c) add a `*_anchor_hash_unchanged` fn for a scenario in
NEITHER map → exit 1 (HARD ERROR), proving the blind spot is closed.

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

- `spec/anchors.toml` header comment: the dual-anchor paragraph already
  landed. ADD a sentence (architect will also patch this — see § EX-6
  anchors.toml note below) clarifying the **mixed-provenance** reality:
  the `v5-realdata-medium-2026-05` rows for `btc-2023-1m-{macd-trend,
  rsi-reversion,bbands-mean-revert}` were emitted from the REAL-DATA path
  (17544-bar Binance bodies), so the matching `determinism.rs` constants
  are the **synthetic** SHAs (held in D7.1's `SYNTHETIC_DETERMINISM_SHAS`,
  NOT here), while the sma / momentum / tt1 v5 rows ARE synthetic and the
  in-test constants mirror them directly (cite ADR-0045 § D6.3).
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
  → exit 0 reporting **14 literal(s) match** (8 canonical + 6 synthetic), candle
  pair skipped. Run the three negative scratch tests in EX-4 v2 acceptance
  (corrupt a synthetic constant, a canonical constant, add an unmapped fn) →
  each exits 1 → revert.
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
- **R4 — synthetic-SHA drift (the 6).** The synthetic SHAs
  `4d8192af…`/`4a744788…`/`5037accb…` were verified twice (developer VR-1 +
  architect independent re-run, both CWD=tempdir seed 0xC0FFEE). They are NOT
  in `anchors.toml`; their sole source of truth is D7.1's
  `SYNTHETIC_DETERMINISM_SHAS`. If a future engine change moves the synthetic
  macd/rsi/bbands output, D7.2 (re-run) catches it but D7.1 will then assert
  against a stale synthetic map entry — the dev who re-locks the constant MUST
  also update the dict (they live together, by design, so this is a
  one-file edit). This is the residual cost of NOT adding file-anchors; it is
  bounded and documented in ADR-0045 § D7.1b.
- **R5 — do NOT let `--write` silently "fix" the 6 with the wrong source.**
  Until the dev wires the synthetic-override map into `apply_write` (EX-4 v2),
  `--write` will rewrite the 6 to the `v5-realdata-medium` (real-data) SHAs,
  re-introducing the BLOCKER. Wire the map into BOTH `detect_drift` and
  `apply_write` in the same edit pass.
