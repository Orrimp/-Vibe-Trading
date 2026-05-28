---
slug: lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1
status: in-progress
owner: developer
updated: 2026-05-28
---

# Tasks — lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1

> Owner flips: analyst → operator → architect → developer → tester →
> presenter. M-T1 likely fast-skips (ADR-0040 § Changelog amendment
> only, per D-V0.1.2-6 precedent). M-OD load-bearing on Q1.

## M0 — analyst (this brief)

- [x] T-A1 — feature.md authored (5R + R-NR + 4K + 3H + 2Q).
- [x] T-A2 — tasks.md authored.
- [x] T-A3 — backlog `## Active` row appended with M0 close annotation.
- [x] T-A4 — trace row `REQ-LAB-YAHOO-REALDATA-V0-1-3-001` opened
  `proposed`.
- [x] T-A5 — spec-lint baseline confirmed (78/5 pre-write; post-write
  re-check confirms no NEW categories from M0).
- [x] T-A6 — `verify_anchors.sh` 70/70 PASS pre-ship confirmed.

## M-OD — operator decide

**RESOLVED 2026-05-28** — operator picked both (a) durable choices.

- [x] T-OD1 (2026-05-28) — Q1 = **(a) helper-extraction [Recommended — DURABLE]**.
- [x] T-OD2 (2026-05-28) — Q2 = **(a) in-place SHA under existing `lab-yahoo-realdata-v0.1.1`** [Recommended — DURABLE].

## M-T1 — architect (NOT a fast-skip; durable boundary locked 2026-05-28)

- [x] T-T1.1 (2026-05-28) — Q1+Q2 ratified; § Design recorded (D-V0.1.3-1
  through D-V0.1.3-9). Operator durable picks locked under AGENT.md
  2026-05-28 contract.
- [x] T-T1.2 (2026-05-28) — K1 grep PASS: `rev=` substring appears in
  exactly one production binary (`crates/backtest/src/bin/run_yahoo_sma.rs:259`).
  Zero leak to other emitters; Q1=(a) scope correctly bounded.
- [x] T-T1.3 (2026-05-28) — K2 grep PASS: `revision_sha:` key not
  present in any existing Yahoo report frontmatter. Insertion
  collision-free.
- [x] T-T1.4 (2026-05-28) — ADR-0040 § Changelog amended with v0.1.3
  entry. No new ADR (helper extraction is mechanical refactor;
  frontmatter field is mechanical re-placement of existing data).
- [x] T-T1.5 (2026-05-28) — owner flip → developer; trace
  `state = arch-done`; trace `arch` column populated with
  M-T1 paths (feature.md § Design, ADR-0040 § Changelog).

**Architect call-outs for developer T-D phase:**

1. **T-D4 cadence ambiguity** — `btc-2024-h1-sma-cross` predecessor
   uses `bar_count: 262_800` which is 1m-equivalent (262_800 ≈ 365d ×
   720m). True H1 would be `8_760` (365d × 24h). Developer pre-flight
   resolves by counting actual bars in `data/binance/ETHUSDT/2024/`
   parquets; mirror predecessor verbatim if its bar_count works at
   runtime, else file an ADR amendment.
2. **T-D2 helper landing order** — extract `report/yahoo.rs` FIRST,
   then migrate `run_yahoo_sma.rs:259` SECOND, then add ETH H1
   scenario THIRD. Out-of-order risks half-migrated state at intermediate
   commit boundaries.
3. **T-D8 anchor in-place protocol** — when updating row 69 in
   `spec/anchors.toml`, preserve the existing namespace label
   `lab-yahoo-realdata-v0.1.1` verbatim; only the SHA value mutates.
   Row 71 append goes under `lab-yahoo-realdata-v0.1.3`. Two distinct
   namespace lines for two distinct contracts.

## M-DEV — developer (Q1=(a) Recommended path)

**Wave A — canonical helper (R1.3):**

- [ ] T-D1 — pre-flight: open one `data/binance/ETHUSDT/2024/*.parquet`,
  confirm schema parity (K3 falsifier).
- [ ] T-D2 — extract helper (recommended `crates/backtest/src/report/yahoo.rs`):
  Data-source body line (no `rev=`) + `revision_sha:` front-matter inject.
- [ ] T-D3 — migrate `run_yahoo_sma.rs:259` to call helper (R1.1, R1.2).

**Wave B — Binance ETH H1 scenario (R2):**

- [ ] T-D4 — add `eth-2024-h1-sma-cross` arm in
  `crates/backtest/src/main.rs` mirroring `btc-2024-h1-sma-cross` (R2.1).
- [ ] T-D5 — extend auxiliary match-arms (L1029 strategy-id, L1762
  namespace dispatch — grep `btc-2024-h1-sma-cross` first).

**Wave C — anchor migration (R3):**

- [ ] T-D6 — re-emit BTC default invocation; grep-confirm no `rev=` (R1.4).
- [ ] T-D7 — emit `eth-2024-h1-sma-cross` ≥ 2 runs; confirm determinism.
- [ ] T-D8 — `spec/anchors.toml` row 69 BTC SHA in-place under namespace
  `lab-yahoo-realdata-v0.1.1` (Q2=(a)); append row 71 under
  `lab-yahoo-realdata-v0.1.3`.
- [ ] T-D9 — `verify_anchors.sh` → 71/71 PASS.

**Wave D — H1 + gates:**

- [ ] T-D10 — `dev-notes/yahoo-vs-binance-eth-h1-2026-05-XX.md`
  (Yahoo ETH daily vs Binance hourly; delta < 30%).
- [ ] T-D11 — `cargo fmt --check` + clippy `-D warnings` on touched paths.
- [ ] T-D12 — workspace lib tests green; owner flip → tester.

## M-FINAL — tester

- [ ] T-F1 — independent `verify_anchors.sh` 71/71 PASS.
- [ ] T-F2 — re-emit BTC + ETH H1 independently; SHA byte-identical.
- [ ] T-F3 — grep `rev=` against v0.1.3 reports (R1.4 post-condition).
- [ ] T-F4 — confirm 68 non-Yahoo anchors + row 70 ETH daily byte-identical.
- [ ] T-F5 — spec-lint baseline-stable (no NEW categories vs 78/5).
- [ ] T-F6 — author `reports/test-final-...md`; verdict → PASS.
- [ ] T-F7 — owner flip → presenter; trace `state = passed`.

## M-PRESENTER — presenter

- [ ] T-P1 — sprint-review deck `presentations/lab-yahoo-realdata-v0.1.3-2026-05-XX.md`;
  operator approval → ship.

## Notes

- Backend-only ship at v0.1.3 (zero UI files); no M-DEV-UI lane.
- Q1=(b) fallback: skip Wave A T-D2; do R1.1+R1.2 inline in
  `run_yahoo_sma.rs` only. Document deferred helper as v0.2.0 prereq
  in the M-FINAL report.
