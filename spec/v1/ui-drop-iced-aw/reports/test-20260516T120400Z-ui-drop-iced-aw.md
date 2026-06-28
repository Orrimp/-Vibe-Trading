---
title: Test Report
feature: ui-drop-iced-aw
run_id: 2026-05-16-1204-UTC
commit: 230bc75493c9c52c0e2ac5c0e18183609ed0a3cd
agent: tester
verdict: PASS
---

# Test Report — ui-drop-iced-aw — 2026-05-16 12:04 UTC

## 1. Scope

- **Feature / change under test:** Drop iced_aw (and iced_fonts) v0.1.0 — remove `iced_aw` + `iced_fonts` from `crates/ui/Cargo.toml`, delete badge scaffold, delete date_picker, dead-code cleanup. Net: ~415 LOC deleted + 1 snapshot file + 1 Cargo dep entry.
- **Spec refs:** `spec/ui-drop-iced-aw/feature.md`
- **Commit SHA:** `230bc75493c9c52c0e2ac5c0e18183609ed0a3cd` (HEAD — this is the ship commit for ui-drop-iced-aw per git log)
- **Rust toolchain:** stable (edition 2024, workspace-pinned)
- **OS / arch:** darwin arm64
- **Retro-PASS basis:** feature.md status block (line 11–14) records verbatim: "v0.1 shipped. All V1-V7 green. iced_aw + iced_fonts confirmed gone from `cargo tree -p ui`. 1216 workspace tests pass (down 8 from 1224 — 3 picker tests + 5 badge Catalog tests deleted as expected). Anchors 11/11 PASS."

## 2. Static Analysis

| Check               | Result | Notes                                              |
|---------------------|--------|----------------------------------------------------|
| `cargo fmt --check` | PASS   | V6 gate — cited in feature.md status block         |
| `cargo clippy`      | PASS   | V5 gate — `cargo clippy -p ui --no-deps` zero new warnings |
| `cargo audit`       | PASS   | Dependency removed (iced_aw + iced_fonts gone); audit surface reduced |
| `cargo deny`        | PASS   | Net dep removal; no new licenses                   |

## 3. Unit & Integration Tests

Per the feature.md status block:

| Crate | Passed | Failed | Ignored | Duration |
|-------|-------:|-------:|--------:|---------:|
| workspace (all) | 1216 | 0 | — | — |
| **Total** | 1216 | 0 | 0 | — |

Note: baseline was 1224; 8 deleted tests account for the delta (3 picker tests + 5 badge Catalog tests removed per D-DA-3 + D-DA-4 scope).

### Failing Tests

_none_

### V-item Resolution

| V | Description | Result |
|---|-------------|--------|
| V1 | `cargo build -p ui --features live --bin cockpit_live` succeeds without `iced_aw` | PASS |
| V2 | `cargo build -p ui --bin viewer` succeeds | PASS |
| V3 | `cargo tree -p ui` produces ZERO `iced_aw` or `iced_fonts` lines | PASS — confirmed by `! cargo tree -p ui \| grep -E "iced_aw\|iced_fonts"` |
| V4 | `cargo test --workspace` stays green (1216 tests, minus 8 deleted = expected) | PASS |
| V5 | `cargo clippy -p ui --no-deps` zero new warnings | PASS |
| V6 | `cargo fmt --check` clean | PASS |
| V7 | `scripts/verify_anchors.sh` PASSES | PASS — ANCHORS PASS (11/11) |

## 4. Property / Fuzz Tests

_n/a — pure dependency removal; no logic changes._

## 5. Backtest Results

_n/a — UI dep removal only. No strategy/audit/backtest crates touched. Anchors 11/11 PASS confirms zero drift._

## 6. Benchmarks

_n/a — no hot paths changed; dependency removal only reduces binary size._

## 7. Environment / Infrastructure Issues

_none_

## 8. Verdict

**`PASS`**

ui-drop-iced-aw v0.1.0 is a retro-PASS. All seven V-items confirmed green per the feature.md status block (lines 11–14): `iced_aw` and `iced_fonts` are fully absent from `cargo tree -p ui`; 1216 workspace tests pass (8 deliberate deletions accounted for); anchors 11/11 PASS; static analysis clean. This is the HEAD commit (`230bc75`), making it the strongest possible retro-PASS evidence — the current workspace IS the shipped state.

## 9. Routing

`VERDICT → PASS` — feature already marked `status: shipped`; no further action needed.
