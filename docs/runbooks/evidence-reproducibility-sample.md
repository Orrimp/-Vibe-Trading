# Runbook — Reproduce a real-data figure from a fresh clone

**Created:** 2026-09-25 (story 6-12). **Audience:** anyone who is not the operator — a
future maintainer, a reviewer, a sceptic, or the operator on a new machine.

## The claim, and the command

From a fresh clone, with nothing but the repository and a Rust toolchain:

```bash
cargo test -p backtest --features yahoo --test reproducibility_sample_figure
```

That runs the backtest below against a corpus **committed to this repository** and
asserts the report body hashes to the value `evidence/anchors.toml` has carried since
2026-05-28.

| | |
|---|---|
| Scenario | `btc-yahoo-2024-1d-sma-cross` |
| Data | `data/yahoo-sample/BTC-USD/1d/2024/` — 12 daily parquets, 96 KB, plain git |
| Window | 2024-01-01 → 2024-12-31, daily |
| Seed | `0xC0FFEE` |
| Headline figure | **final equity $104560.08 USDT** (+4.56 %, 7 trades) |
| Anchored body SHA | `076929bb63d9bec03ec83684b85ced818ee32c0b2da41140712ec1d01de6a1e0` |

To see it by hand rather than through the test:

```bash
cargo run --release -p backtest --features yahoo --bin run_yahoo_sma -- --cache-root data/yahoo-sample --ticker BTC-USD --reports-dir /tmp/repro
```

```bash
python3 scripts/hash_report.py /tmp/repro/*btc-yahoo-2024-1d-sma-cross.md
```

The test shells out to that same binary and that same hash script on purpose: a gate
that re-implemented either could agree with itself while disagreeing with the recipe.

## What this is NOT

Read this part. A sample corpus invites the reader to assume more than it proves.

- **It reproduces ONE claim.** One ticker, one year, one strategy. The repository's other
  real-data figures rest on corpora that are gigabytes and remain machine-local; nothing
  here changes that.
- **The 119 anchored bodies were already verifiable from the repo alone** — `verify_anchors.sh`
  hashes committed report bodies, and that has always passed without any corpus. What was
  NOT verifiable by a second party, and now is for this one figure, is the **run behind**
  the body.
- **Some other anchored scenarios do not currently reproduce at all.** Measured 2026-09-24
  and recorded as bug-log `#93`: the `btc-2023-1m-*` family now emits **hourly bars over
  two years** where the committed evidence records **minute bars over one**
  (525601 → 17544 bars; final equity $47290.03 → $107381.95). This gate is green precisely
  because it covers the scenario that still holds. **Do not read it as a statement about
  the corpus at large.** Resolving `#93` belongs to story 1-26.
- **The sample's aggregate SHA differs from `data/yahoo/`'s, by construction** — it pins a
  12-file slice, not the 174-file corpus. That is not a broken pin. The scenario's report
  body carries no revision SHA (`report::yahoo` documents "NO `rev=` substring"), and the
  front matter is stripped before hashing, which is why a different root still produces a
  byte-identical body.

## Why the corpus is committed and its siblings are not

`.gitignore` tracks only the `REVISION.toml` of every pinned corpus — `data/binance/`,
`data/yahoo/`, and the rest — because their parquets are gigabytes. `data/yahoo-sample/`
is the single exception: 96 KB, deliberately small enough to need no LFS, and guarded by
`the_sample_stays_small` so that widening it has to be a decision rather than a drift.

## If the gate goes red

It is telling you something. Work out which before touching anything.

1. **`reproducibility_sample_corpus` also red** → the committed bytes changed. Do **not**
   regenerate the manifest to make it pass; that pins whatever is there now. Find out what
   touched the parquets.
2. **Only the figure test red** → the engine no longer computes the same answer from
   unchanged data. That is real drift, of the kind `#93` documents elsewhere, and the
   anchor has become a historical record rather than a reproducible claim. It is a finding,
   not a chore.
3. **Never re-pin `ANCHORED_BODY_SHA` to the new value.** That converts a caught drift into
   a silent one — bug-log `#77`'s failure mode, and the reason this gate exists.

## Regenerating the sample (operator only)

```bash
cargo test -p data --test reproducibility_sample_corpus regenerate -- --ignored --nocapture
```

Reads the operator's machine-local `data/yahoo/` and rewrites the slice plus its manifest.
The printed aggregate SHA must match `SAMPLE_AGGREGATE_SHA` in that test; if it does not,
the slice changed and the commit message should say why.
