---
slug: lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit
status: arch-done
owner: developer
updated: 2026-05-29
---

# Tasks — lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit

> analyst → operator (M-OD + R1 fetch) → architect (likely fast-skip) →
> developer (Wave A bulk-emit ‖ Wave B Binance H1) → tester → presenter.
> R1 operator cache populate is BLOCKER for M-DEV.

## M0 — analyst (this brief)

- [x] T-A1 — feature.md authored (5R + R-NR + 4K + 3H + 2Q + cascade).
- [x] T-A2 — tasks.md authored.
- [x] T-A3 — backlog `## Active` row appended.
- [x] T-A4 — trace row `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` opened `proposed`.
- [x] T-A5 — `verify_anchors.sh` 71/71 PASS pre-ship confirmed.

## M-OD — operator decide (LOAD-BEARING Q1) — RESOLVED 2026-05-29

- [x] T-OD1 — Q1 = **(a) DURABLE 9 Binance H1 regs** (resolved 2026-05-29).
- [x] T-OD2 — Q2 = **(a) DURABLE single namespace `lab-yahoo-realdata-v0.1.4`** (resolved 2026-05-29).
- [ ] T-OD3 — R1 fetch executed; aggregate SHA pasted into T-D1 (operator-side; M-DEV start gate).

## M-T1 — architect (fast-skip CONFIRMED; ADR-0040 § Changelog amendment) — DONE 2026-05-29

- [x] T-T1.1 — Q1+Q2 ratified per D-V0.1.4-1 + D-V0.1.4-4; 9 Binance H1
  regs shape per v0.1.3 D-V0.1.3-5 template.
- [x] T-T1.2 — K1 pre-flight: `bar_count: 262_800` mirrors v0.1.3 BTC+ETH-H1
  verbatim (real-parquet auto-detect overrides; D-V0.1.4-2).
- [x] T-T1.3 — K3 partial-year-listing edge for AVAX/MATIC: 95% threshold
  uniform; operator-side R1 surfaces K1 BEFORE M-DEV; default drop-on-fire
  (D-V0.1.4-3).
- [x] T-T1.4 — Anchor cascade ratified: row 70 in-place under
  `lab-yahoo-realdata-v0.1.2`; rows 72-80 append under
  `lab-yahoo-realdata-v0.1.4`; net 71 → 80 (D-V0.1.4-4).
- [x] T-T1.5 — ADR-0040 § Changelog amended (D-V0.1.4-5 shape); no new ADR.
- [x] T-T1.6 — frontmatter flipped `owner: analyst → developer`; trace
  row `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` `arch` column populated; state
  `proposed → arch-done`.

## M-DEV — wave decomposition (D-V0.1.4-7): Wave A ‖ Wave B → Wave C → Wave D

> **START GATE:** Wave A T-D1 blocks on operator-side R1 fetch evidence
> per D-V0.1.4-9. Wave B can start in parallel after T-D1 paste-in
> (Wave B does NOT consume Yahoo cache; it consumes Binance cache).
> Verbatim R1 command in feature.md § R1.1.

**Wave A — Bulk re-emit (R2 + R3; ~1.5 days) — helper-instantiation only:**

- [ ] T-D1 — pre-flight: paste R1 evidence (post-fetch
  `data/yahoo/REVISION.toml` `[revision].sha256` aggregate; +108 file
  rows + 9 yahoo_response keys) into M-DEV trace row; confirm
  helper-bypass regression guard 3/3 PASS at HEAD.
- [ ] T-D2 — re-emit BTC `--ticker BTC-USD` × 2 runs; SHA must match
  row 69 `076929bb…` (determinism witness; gate before T-D3).
- [ ] T-D3 — re-emit ETH-daily `--ticker ETH-USD` × 2 runs byte-identical;
  record NEW row 70 SHA.
- [ ] T-D4 — re-emit 9 new tickers (BNB/SOL/XRP/ADA/DOGE/AVAX/DOT/LINK/MATIC)
  × 2 runs each; record all 9 SHAs.
- [ ] T-D5 — `spec/anchors.toml`: row 70 SHA in-place under
  `lab-yahoo-realdata-v0.1.2`; append rows 72-80 under
  `lab-yahoo-realdata-v0.1.4`.
- [ ] T-D6 — `scripts/verify_anchors.sh` → ANCHORS PASS (80 / 80).

**Wave B — 9 Binance H1 scenario registrations (R-Q1=(a); ‖ Wave A; ~3-4 days):**

- [ ] T-D7 — register `{ticker-lc}-2024-h1-sma-cross` × 9 in
  `crates/backtest/src/main.rs` at 3 sites each per D-V0.1.4-1. Copy
  `bar_count: 262_800` verbatim per D-V0.1.4-2 (do NOT recompute to 8_760).
- [ ] T-D8 — direct H1 discharge × 9: `cargo run --features realdata
  --bin backtest -- {ticker-lc}-2024-h1-sma-cross` × 2 runs per ticker;
  record Binance hourly equity per ticker; compute delta vs Yahoo daily;
  H1 threshold 30%.

**Wave C — Per-ticker H1 dev-notes (durable honest-reporting contract; ~0.5 day):**

- [ ] T-D9 — emit `spec/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/reports/h1-discharge-{ticker-lc}-2026-05-XX.md`
  × 9 (or single consolidated `yahoo-vs-binance-bulk-h1-2026-05-XX.md`
  with 9 per-ticker sections — developer chooses). Each section records:
  Yahoo-daily equity, Binance-hourly equity, delta %, pass/fail vs 30%,
  K4 falsifier-fire status.

**Wave D — Gates:**

- [ ] T-D10 — `cargo fmt --check` + clippy `-D warnings` on touched paths
  (only `crates/backtest/src/main.rs` for Wave B; pre-existing 9 ui
  clippy carried over per R-NR.3).
- [ ] T-D11 — R5.7 frozen-boundary `git diff HEAD~ HEAD` empty for the
  4 files: `report/yahoo.rs`, `report/sma.rs`, `report/mod.rs`,
  `run_yahoo_sma.rs`.
- [ ] T-D12 — workspace lib tests green (411 ui + non-ui baseline);
  owner flip → tester.

## M-FINAL — tester

- [ ] T-F1 — independent `verify_anchors.sh` 80/80 PASS.
- [ ] T-F2 — re-emit all 11 reports independently; SHA byte-identical.
- [ ] T-F3 — rows 1-68 + row 71 ETH-H1 byte-identical (R5.1, R5.3).
- [ ] T-F4 — helper-bypass guard 3/3 PASS preserved (R5.4); R5.7 diff empty.
- [ ] T-F5 — H2 fetch ≥ 95%; H1 per-ticker delta < 30% × 9.
- [ ] T-F6 — operator-visible H3: Lab tab badge shows "10 tickers".
- [ ] T-F7 — spec-lint baseline-stable; author test-final report; verdict PASS.
- [ ] T-F8 — owner flip → presenter; trace `state = passed`.

## M-PRESENTER

- [ ] T-P1 — sprint-review deck; operator approval → ship.

## Notes

- Backend-only (zero UI files); no M-DEV-UI lane.
- Q1=(b) cheap: skip Wave B for 8 tickers; ship 8 K1 carve-outs; document
  each as v0.1.5+ cleanup brief in M-FINAL.
- R1 fetch is operator-blocker (analyst pass cannot stub data).
