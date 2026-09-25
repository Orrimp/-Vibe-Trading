---
adr: 0097
title: One real-data figure reproduces from a fresh clone
status: accepted
date: 2026-09-25
supersedes: none
superseded-by: none
---

# ADR-0097: One real-data figure reproduces from a fresh clone

## Context

Product review 2026-08-04, finding 8: **every pinned corpus is machine-local and
gitignored, so no second party can reproduce a single real-data claim.** The 119 anchored
bodies verify from the repo alone — `verify_anchors.sh` hashes committed text — but the
*runs behind them* were re-executable only by the operator, on the operator's disks.
"Honest" meant honestly-intended rather than verifiable-by-someone-else.

## Decision

**D1 — Commit one slice, and only one.** `data/yahoo-sample/BTC-USD/1d/2024/`: twelve
daily parquets, **96 KB**, plain git, no LFS. Every sibling corpus keeps its
manifest-only `.gitignore` treatment; this is the single exception and the
`.gitignore` comment says why. `the_sample_stays_small` asserts 12 files under 200 KB, so
widening it has to be a decision rather than a drift.

**D2 — Reproduce `btc-yahoo-2024-1d-sma-cross`, because it has an expected value already.**
Body-SHA `076929bb…`, locked 2026-05-28. Choosing a figure with a committed anchor rather
than baselining a new one means the gate compares against something written before this
story existed and with no knowledge of it.

**D3 — The gate runs the BINARY and hashes with the repo's own script.** It would be
easier to call `sma_composed_run::run` and assert on the returned value. That test would
keep passing if `run_yahoo_sma` drifted away from the library, at which point the
runbook's documented command and the gate would be testing different things. Likewise the
body hash comes from `scripts/hash_report.py` — the function `verify_anchors.sh` uses —
rather than a re-implementation that could agree with itself while disagreeing with the
gate.

**D4 — Corpus integrity and figure reproduction are SEPARATE tests.** "The bytes are
intact" and "the engine still computes the same answer from them" are different claims,
and one test mixing them could pass for the wrong reason.
`reproducibility_sample_corpus` (no feature flag, no engine, runs everywhere) owns the
first; `reproducibility_sample_figure` (`yahoo` feature) owns the second.

**D5 — The aggregate SHA is pinned TWICE.** In the sample's `REVISION.toml` and again as a
constant in the test. `read_and_verify_revision_manifest` recomputes the aggregate and
compares it to the manifest's own claim, so a hand-edited manifest agreeing with
hand-edited parquets would verify. The constant sits outside that loop.

**D6 — CI runs it on the ubuntu leg only.** Reproducibility from committed bytes is a
property of the data and the engine, not of the operating system; three legs would triple
the cost of the `yahoo` feature build to re-assert one fact. If cross-OS float determinism
is ever in question that is a separate, measured decision — the same one ADR-0057 D2 made
for the pixel gates.

**D7 — The runbook states what this is NOT, at length.** A committed sample invites the
reader to assume more than it proves. It reproduces ONE claim; the full corpora stay
machine-local; the anchored bodies were already verifiable and it is the *runs* that were
not; and — the sharp one — **several other anchored scenarios do not currently reproduce
at all** (bug-log `#93`, measured: the `btc-2023-1m-*` family now emits hourly bars over
two years where the evidence records minute bars over one). This gate is green precisely
because it covers the scenario that still holds.

## Alternatives considered

- **A Binance slice (one symbol × one month, 26 KB)** — smaller, and rejected. No
  already-asserted single-symbol Binance figure exists, and `main.rs`'s `span()` offers no
  sub-year window, so the realistic floor there is ten symbols × twelve months ≈ 2.6 MB.
  The Yahoo ticker-year is 2.2× the Binance month and buys a figure with a committed
  anchor.
- **Follow `data/binance/`'s `.gitignore` style, as AC1 said** — impossible as written.
  That style commits only `REVISION.toml`; `git ls-files data/` returns 13 files, all
  manifests, and **zero** parquets. The AC's premise was wrong and D1 replaces it.
- **LFS** — not warranted at 96 KB, and `.gitattributes` has no `*.parquet` rule today.
- **Assert a float within a tolerance, per AC2's wording** — rejected. The anchored body
  SHA is exact and already exists; inventing a tolerance would be a weaker claim than the
  one available for free. The human-readable figure is quoted in the runbook instead.

## Consequences

- The repository now contains 96 KB of real market data, tracked, for the first time.
- A prerequisite had to be fixed first and shipped separately: bug-log **#106** —
  `run_yahoo_sma` did not pin the wall-clock it prints into the hashed body, so the anchor
  was reproducible only on hardware fast enough to finish inside 50 ms. Fixed in
  `bb3be261`; without it this gate would have been flaky by construction.
- **The gate's failure message forbids re-pinning the constant.** Making a drifted anchor
  green by updating the expected value is bug-log `#77`'s failure mode and would convert
  the one thing this story delivers into its opposite.
- **Moral**: byte-immutability proves a document has not changed. It says nothing about
  whether anyone can still produce it. Those are different guarantees and this repo had
  only the first.
