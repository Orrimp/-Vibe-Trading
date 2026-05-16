---
slug: v25-dl-forecast-overlay
status: draft
owner: pending-analyst
updated: 2026-05-16
---

# Tasks — v2.5 DL forecast overlay (candle-trained)

_Stub. The analyst authors a fresh task list after closing the feature.md
open questions (model family / size / tokenisation / data / loss / horizon
/ success criterion / checkpoint storage / audit integration)._

The architect then decomposes into milestone-shaped tasks.

## Carried forward (Wave A, 2026-05-16 — already shipped)

- [x] **T-WA-1** — `crates/forecast/` scaffolded — `ForecastProvider`
  trait + `overlay::combine()` + 15 tests green.
- [x] **T-WA-2** — `crates/replay-cache/` scaffolded — generic SQLite
  WAL content-addressed cache + 8 tests green.
- [x] **T-WA-3** — `crates/core/src/forecast.rs` — domain value types +
  7 tests green.

## Pending (analyst + architect own)

- [ ] **T-A-1** — Analyst closes the 8+ open questions in feature.md
  ## Requirements.
- [ ] **T-AR-1** — Architect locks model family + size + tokenisation
  strategy + loss + evaluation criterion. Updates feature.md ## Design.
- [ ] **T-AR-2** — Architect decomposes the implementation into ordered
  milestone-shaped tasks. Replaces this stub task list.

## Notes

Wave A carries forward verbatim — only the Kronos-specific files were
removed. The `ForecastProvider` trait, `overlay::combine()`, and the
shared `replay-cache` crate are all model-agnostic by design and ready
for whatever model lands.
