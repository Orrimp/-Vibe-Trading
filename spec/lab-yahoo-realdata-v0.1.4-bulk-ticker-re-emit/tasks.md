---
slug: lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit
status: draft
owner: analyst
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

## M-OD — operator decide (LOAD-BEARING Q1)

- [ ] T-OD1 — Q1 = (a) DURABLE 9 Binance H1 regs OR (b) cheap BNB-only + 8 K1.
- [ ] T-OD2 — Q2 = (a) DURABLE single namespace OR (b) cheap per-ticker.
- [ ] T-OD3 — R1 fetch executed; aggregate SHA pasted into T-D1.

## M-T1 — architect (likely fast-skip; ADR-0040 § Changelog amendment)

- [ ] T-T1.1 — Q1+Q2 ratified; if Q1=(a) Binance H1 shape per v0.1.3 D-V0.1.3-5.
- [ ] T-T1.2 — R5.7 frozen-boundary contract: zero diff allowed in
  `report/yahoo.rs`, `report/sma.rs`, `report/mod.rs`, `run_yahoo_sma.rs`.
- [ ] T-T1.3 — ADR-0040 § Changelog amended (no new ADR — mechanical scaling).
- [ ] T-T1.4 — owner flip → developer; trace `state = arch-done`.

## M-DEV — developer (Q1=(a); Wave A ‖ Wave B)

**Wave A — bulk re-emit (R2 + R3):**

- [ ] T-D1 — pre-flight: R1 evidence + helper-bypass guard 3/3 PASS at HEAD.
- [ ] T-D2 — re-emit BTC `--ticker BTC-USD`; SHA must match row 69 `076929bb…`.
- [ ] T-D3 — re-emit ETH-daily `--ticker ETH-USD`; ≥ 2 runs byte-identical;
  record NEW row 70 SHA.
- [ ] T-D4 — re-emit 9 new tickers; ≥ 2 runs each; record all 9 SHAs.
- [ ] T-D5 — `spec/anchors.toml`: row 70 SHA in-place under
  `lab-yahoo-realdata-v0.1.2`; append rows 72-80 under
  `lab-yahoo-realdata-v0.1.4`.
- [ ] T-D6 — `verify_anchors.sh` → ANCHORS PASS (80 / 80).

**Wave B — Binance hourly registrations (R-Q1=(a); ‖ Wave A):**

- [ ] T-D7 — register `{ticker-lc}-2024-h1-sma-cross` × 9 in
  `crates/backtest/src/main.rs` (3 match-arm sites per v0.1.3 D-V0.1.3-5).
- [ ] T-D8 — direct H1 discharge × 9: Yahoo daily vs Binance hourly; record
  deltas; threshold 30%.
- [ ] T-D9 — `dev-notes/yahoo-vs-binance-bulk-h1-2026-05-XX.md` records 9 deltas.

**Wave D — gates:**

- [ ] T-D10 — `cargo fmt --check` + clippy `-D warnings` on touched paths.
- [ ] T-D11 — R5.7 frozen-boundary `git diff` empty for the 4 files.
- [ ] T-D12 — workspace lib tests green; owner flip → tester.

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
