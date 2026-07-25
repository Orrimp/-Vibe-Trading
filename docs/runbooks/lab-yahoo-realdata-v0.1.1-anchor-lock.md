# Lab Yahoo realdata v0.1.1 — anchor lock runbook

**Version:** v0.1.1
**Owner:** operator (runs steps 1-3) / developer (runs steps 4-5) / tester (closes step 6)
**Related code:**
- `crates/data/src/bin/fetch_yahoo_klines.rs` — CLI populator
- `crates/data/src/yahoo.rs` — bar source + REVISION.toml writer
- `crates/ui/tests/lab_yahoo_anchor.rs` — scaffold integration test (`#[ignore]`-gated until this runbook completes)
- `data/yahoo/REVISION.toml` — per-file SHA manifest
- `evidence/anchors.toml` — repo-level anchor registry
- `docs/archive/pre-bmad-spec/v1/lab-yahoo-realdata/feature.md` — feature brief (shipped v0.1.0 2026-05-24)
- `docs/dev-notes/bug-log.md` #61 — partial-fix row for this work

---

## Overview

`lab-yahoo-realdata v0.1.0` shipped 2026-05-24 with the Yahoo path wired
into the Lab dispatcher + a scaffold integration test
(`crates/ui/tests/lab_yahoo_anchor.rs`) `#[ignore]`-gated against a
populated cache. v0.1.1 closes the loop:

1. Confirm the BTC-USD daily 2024 cache slice is populated.
2. Discover the three deterministic locks the scaffold test will pin:
   the aggregate revision SHA, the SMA(20,50) trade count, and the final
   equity.
3. Wire those values into the test constants + remove `#[ignore]`.
4. (Optional, deferred to v0.1.2 by analyst) lock a body-SHA anchor in
   `evidence/anchors.toml`.

Anchor-additive contract: the 34 existing anchors stay byte-identical
throughout. Yahoo anchors enter under a future namespace pin
(`v0.1.2-yahoo-realdata` or similar). v0.1.1 stops at the lock-the-test
step; v0.1.2 is the body-SHA promotion.

Closes [H1] (Yahoo vs Binance equity divergence < 30%) and [H2]
(fetch success rate > 95%) from
[`docs/archive/pre-bmad-spec/v1/lab-yahoo-realdata/feature.md`](../../docs/archive/pre-bmad-spec/v1/lab-yahoo-realdata/feature.md).

---

## Prerequisites

```bash
# Check existing cache state — should show 12 parquet files under
# data/yahoo/BTC-USD/1d/2024/ + a REVISION.toml at data/yahoo/.
ls data/yahoo/BTC-USD/1d/2024/ | sort
ls data/yahoo/REVISION.toml
```

Expected:
```
01.parquet  02.parquet  03.parquet  04.parquet  05.parquet  06.parquet
07.parquet  08.parquet  09.parquet  10.parquet  11.parquet  12.parquet
```

If files are missing, run Step 1; otherwise skip to Step 2.

**Network**: Step 1 hits `query1.finance.yahoo.com`. Per ADR-0040 § K1,
the unofficial Yahoo API is rate-limited (~10 reqs/sec, no formal SLA).
The CLI's built-in exponential backoff (1 s → 60 s cap, 5 retries) handles
HTTP 429s.

**Disk**: ~10 KB per ticker × interval × month. Daily BTC-USD 2024 = 12
parquet files × ~9 KB ≈ ~110 KB total. Trivial.

---

## Step 1 — Populate the cache (skip if already present)

```bash
cargo run -p data --features yahoo,yahoo-online --bin fetch_yahoo_klines -- \
  --tickers BTC-USD \
  --interval 1d \
  --start 2024-01-01 \
  --end 2024-12-31
```

Expected stderr (trimmed):
```
INFO fetch_yahoo_klines: ticker=BTC-USD bars=366 expected=366 revision_sha=7b33166e... fetched OK
INFO fetch_yahoo_klines: revision manifest written path="data/yahoo/REVISION.toml"
```

**Dry-run first** if you want to confirm the URL + bar count without
hitting the network:

```bash
cargo run -p data --features yahoo,yahoo-online --bin fetch_yahoo_klines -- \
  --tickers BTC-USD \
  --interval 1d \
  --start 2024-01-01 \
  --end 2024-12-31 \
  --dry-run
```

**Multi-ticker** (if you want ETH-USD too, mirror Binance's 10-pair set):

```bash
cargo run -p data --features yahoo,yahoo-online --bin fetch_yahoo_klines -- \
  --tickers BTC-USD,ETH-USD,BNB-USD,SOL-USD,XRP-USD,ADA-USD,DOGE-USD,AVAX-USD,LINK-USD,DOT-USD \
  --interval 1d \
  --start 2024-01-01 \
  --end 2024-12-31
```

v0.1.1's locked test only needs BTC-USD; the wider set is for future
H1/H2 hypothesis validation.

### Failure modes

| Symptom | Probable cause | Action |
|---|---|---|
| `rate-limited by Yahoo, backing off` repeatedly | API throttle | Wait. Built-in backoff handles it up to 60s × 5 retries. If it fully exhausts, re-run; the cache is incremental. |
| `bars=0 expected=366` | Yahoo returned no rows for the window | Symbol likely de-listed or never traded that range. Pick a different ticker or window. |
| `no such file or directory: data/yahoo` | First run, `--out` parent missing | The CLI auto-creates `--out` (default `data/yahoo`). If you passed a custom `--out` to a path whose parent doesn't exist, create the parent first. |
| `revision_sha` differs from prior run on the same window | Yahoo revised historical data | Expected occasionally for splits / ticker remaps. Investigate the diff; if benign, accept the new SHA and re-lock the test constants (Step 5). |

---

## Step 2 — Discover the three lock values

Run the scaffold test in `--ignored` mode to see the actual values
your cache produces. The current test has placeholder constants
(`EXPECTED_BTC_USD_1D_2024_REVISION_PREFIX = "7b33166e"`,
`EXPECTED_TRADE_COUNT = 10`, `expected_final_equity = $100,000`); these
will probably need updating.

```bash
cargo test --release -p ui --features live,yahoo \
  --test lab_yahoo_anchor -- --ignored --nocapture
```

The test currently runs against the H1_2024 preset (6 months), not the
full year. Inspect the output for:

1. **Revision SHA**: the value printed/asserted near `assertion failed:
   revision_sha starts with "7b33166e"`. If it asserts cleanly, you're
   already locked.
2. **Trade count**: `assertion failed: kpis.trade_count == 10`.
3. **Final equity**: `assertion failed: final equity = $X.XX outside
   $100,000 ± $50,000`. Note the actual value.

> **Important**: `H1_2024` covers `2024-01-01..2024-07-01`. The cache
> must contain at least `01-06.parquet`. If you populated the full year
> (Step 1), this is satisfied.

---

## Step 3 — Capture the discovered values

Edit `crates/ui/tests/lab_yahoo_anchor.rs` and update three locations:

```rust
// Line ~35: revision SHA prefix
const EXPECTED_BTC_USD_1D_2024_REVISION_PREFIX: &str = "<first 8 chars from Step 2>";

// Line ~39: trade count
const EXPECTED_TRADE_COUNT: usize = <observed_count>;

// Line ~46: final equity
fn expected_final_equity() -> Decimal {
    dec!(<observed_equity_rounded_to_2dp>)
}

// Line ~54: optional — tighten the tolerance after the empirical value lands
const FINAL_EQUITY_TOLERANCE: Decimal = dec!(<tighter_window_e.g._1_000>);
```

---

## Step 4 — Remove the `#[ignore]` gate

In `crates/ui/tests/lab_yahoo_anchor.rs`, around line 56-61:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Yahoo cache populated under data/yahoo/BTC-USD/1d/2024 ..."]
async fn yahoo_btc_2024_sma_deterministic() {
```

Remove the entire `#[ignore = "..."]` line. CI now runs this test
every cargo invocation; cache must stay populated or test fails.

---

## Step 5 — Verify

```bash
# 1. Specific test passes
cargo test --release -p ui --features live,yahoo \
  --test lab_yahoo_anchor -- --nocapture

# 2. Full workspace stays green (except pre-existing R8.1 reflection failure)
cargo test --workspace --all-targets

# 3. 34 anchor SHAs unchanged
bash scripts/verify_anchors.sh

# 4. Build/clippy gates
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: all four green (R8.1 still fails pre-existing — separate brief
`reflection-memory-trader-wiring` covers it).

---

## Step 6 — Spec hygiene

1. Update `docs/archive/pre-bmad-spec/v1/lab-yahoo-realdata/feature.md` frontmatter:
   - `version: 0.1.0` → `version: 0.1.1`
   - `status: shipped` → keep `shipped`
   - `updated: <today>`
   - Append a Changelog entry: `- <date> (developer): v0.1.1 anchor lock landed — see runbook docs/runbooks/lab-yahoo-realdata-v0.1.1-anchor-lock.md`.

2. Update `docs/dev-notes/bug-log.md` row for `#61`:
   - `Status: partial-fix` → `Status: fixed`
   - Append the new fix commit hash.

3. Remove the `lab-yahoo-realdata v0.1.1 (live-cache + Yahoo anchor lock)` row from `docs/archive/pre-bmad-spec/backlog.md ## Active`.

4. Commit message template:

   ```
   feat(lab-yahoo): #61 v0.1.1 anchor lock — pin BTC-USD 1d 2024 SMA(20,50) deterministic invariants
   
   Three locks land in lab_yahoo_anchor.rs:
   - revision SHA <prefix> for BTC-USD 1d 2024
   - SMA(20,50) trade count = <N>
   - final equity = $<X.XX> ± $<tolerance>
   
   #[ignore] gate removed; test runs every CI invocation.
   Cache populated 2026-05-XX, REVISION.toml committed.
   ```

---

## Rollback

If the test fails post-merge due to upstream Yahoo data revision:

1. Don't panic — Yahoo data revisions are recoverable.
2. Run Step 2 with `--ignored` removed temporarily to see the new
   SHA + count + equity.
3. If the equity diff is < 30% (H1 hypothesis bound), accept the
   revision: update the three constants + commit with a Changelog
   note. If > 30%, escalate as a data integrity event (could indicate
   Yahoo retroactively corrected a corporate action).
4. If you need to fully revert the v0.1.1 lock: `git revert <commit>`
   re-adds `#[ignore]`. The cache stays populated.

---

## Notes for the operator

- **One-shot vs recurring**: this runbook is one-shot. After the lock
  lands, the test runs every CI invocation against the committed
  REVISION.toml. The cache itself is `.gitignore`-d (per F6 in the
  feature brief); CI must repopulate on cold-start. A future v0.1.2
  task moves the lock from `assert_eq!(trade_count == N)` to a full
  body-SHA anchored Markdown report in `evidence/anchors.toml`.
- **Why not auto-fetch in CI**: ADR-0040 § K4 — the unofficial Yahoo
  API has no SLA. CI flakiness would block every PR. The committed
  REVISION.toml + a fixtures-mode test path lets CI run hermetically;
  the live fetch happens on operator demand only.
- **Background-cadence orchestration is out of scope** — the operator
  triggers fetches explicitly via this runbook; the cockpit reads the
  cache read-only. See F2 in the feature brief for the deferred
  v0.2.0 auto-refresh discussion.
