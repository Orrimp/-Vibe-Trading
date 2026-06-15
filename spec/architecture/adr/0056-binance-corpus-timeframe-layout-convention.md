---
adr: 0056
title: Binance corpus timeframe-layout convention — own-root-per-timeframe (forward lock)
status: accepted
date: 2026-06-15
supersedes: none
superseded-by: none
---

# ADR-0056: Binance corpus timeframe-layout convention — own-root-per-timeframe

## Context

Feature [`binance-corpus-expansion`](../../binance-corpus-expansion/feature.md)
adds **2021–2022 hourly** OHLCV for the existing 10-symbol Binance universe to
firm up the down-market hedge finding from the
[2026-06-13 real-data survey](../../dev-notes/realdata-simple-strategy-survey-2026-06-13.md)
(which rests on only a 2-point down-market sample). The v0.1 corpus lands as a
new sibling root `data/binance-2122/` with its own `REVISION.toml` pin, mirroring
the `data/binance-broaduni/` precedent. v0.1 is **hourly only** and needs no
architectural decision — a flat-layout hourly sibling root works today with zero
reader change.

The forward-looking architectural crux is **Q1**: the Binance on-disk layout
carries **no timeframe path segment**. `ReplayFeed::parquet_files()`
([`crates/data/src/replay_feed.rs:101`](../../../crates/data/src/replay_feed.rs))
resolves `parquet_root.join(symbol)`, then globs `*.parquet` one directory level
down (the year dir). The interval is recorded **only** in `REVISION.toml`
`[revision.metadata].interval` — advisory, **not** part of the hashed aggregate
SHA. Therefore a Binance corpus root is implicitly **single-timeframe**:
`data/binance` *is* the 1h corpus; you cannot drop `1d` bars into
`data/binance/BTCUSDT/2024/` without colliding with the existing hourly
`01.parquet…12.parquet`.

The **Yahoo** corpus diverged here: ADR-0040 § D3 embeds the cadence as a path
segment (`data/yahoo/<TICKER>/<INTERVAL>/<YEAR>/<MONTH>.parquet`, observed on
disk as `data/yahoo/BTC-USD/1d/...`) because Yahoo serves multiple cadences for
one ticker and the Lab dispatch picks a cadence adaptively (ADR-0040 § D6).

A **daily Binance v0.2** is the natural follow-on (the survey § Caveats wants a
daily SMA 20/50 regime study). Before v0.2 bakes a directory convention into a
pinned, gitignored corpus that is painful to reshape, the architect must LOCK
whether new Binance timeframes get **their own root** or a **`/<TF>/` segment**.
This ADR is that lock. It is a **forward** decision; **v0.1 does not depend on
it** (a flat hourly sibling root is correct under either answer).

## Decision

### D1 — Each Binance corpus timeframe is its own root (flat layout retained)

**A new Binance timeframe gets its own corpus root, not a `/<TF>/` path
segment.** The Binance on-disk layout stays flat —
`<root>/<SYMBOL>/<YEAR>/<MM>.parquet` — and the timeframe is encoded by the
**root directory name**, exactly as the venue/universe/range axes already are
(`data/binance`, `data/binance-broaduni`, `data/binance-funding`,
`data/binance-basis`). The interval continues to live in
`REVISION.toml [revision.metadata].interval` as advisory metadata.

**Naming convention for non-1h Binance roots:** append a `-<tf>` suffix to a
descriptive root stem, e.g. a daily 2021–22 corpus is `data/binance-2122-1d/`;
a daily 2023–24 corpus is `data/binance-1d/`. The bare/un-suffixed roots
(`data/binance`, `data/binance-2122`) remain **1h by convention** — this
preserves the existing `data/binance` byte-for-byte. (A future audit nicety —
making the `1h` suffix explicit on new roots — is permitted but NOT required and
must never rename an existing pinned root.)

**v0.1 application:** `data/binance-2122/` is a **1h** root (no suffix, flat
layout). It is correct and final under this convention; no rename is ever owed.

### D2 — Binance and Yahoo layouts intentionally diverge (no convergence retrofit)

The Yahoo `/<INTERVAL>/` segment (ADR-0040 § D3) is **not** retrofitted onto
Binance, and Binance is **not** retrofitted onto Yahoo. The two venues keep
different layouts because their access patterns differ:

- **Yahoo** is consumed by the Lab dispatch path, which derives a cadence
  *adaptively from the requested date range* (ADR-0040 § D6) and may load
  `1m`/`1h`/`1d` for the **same ticker** in one session. A per-ticker cadence
  segment lets one root hold all three. The reader
  (`YahooBarSource::load_cached`) takes an explicit `interval: Interval`
  argument and joins the segment.
- **Binance** is consumed by `ReplayFeed`, which takes **no interval argument**
  — it globs whatever parquet lives under `<root>/<sym>/<year>/`. A root *is* a
  single (universe × range × timeframe) corpus, selected by passing the right
  root path. This is the established, shipped pattern across four Binance roots.

Forcing Binance to adopt the Yahoo segment would require either (i) reshuffling
the existing `data/binance` 1h files into `data/binance/<sym>/1h/<year>/…` —
which **re-emits `data/binance/REVISION.toml`**, flips the `3a8b96c4` aggregate
SHA, and forces re-pinning the four `-realdata` anchors (a REGRESSION-class
event under CLAUDE.md) — or (ii) a new segmented root while the legacy root stays
flat, which produces a **two-shape skew** in `ReplayFeed`'s glob logic (the
reader would need to handle both `<sym>/<year>/` and `<sym>/<tf>/<year>/`). Both
are strictly worse than the status quo for the Binance access pattern. The
divergence is the correct, lower-entropy outcome.

### D3 — `ReplayFeed` is unchanged; root selection is the only knob

No change to `ReplayFeed::parquet_files()` or any reader. A consumer chooses a
timeframe by choosing a root path. This keeps the **34 / 119 anchor-bearing**
`-realdata` body SHAs structurally unreachable from this feature: the existing
`data/binance` root, its `REVISION.toml`, and the survey harness's hard-coded
`data/binance/...` path are all untouched. Anchor neutrality is **by
construction** — there is no code seam to get wrong (cf. ADR-0055 D2: the
strongest anchor guarantee is the one a reviewer cannot forget to check).

## Consequences

**Enforced by:**

- `scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` — the four `-realdata`
  anchors stay green because `data/binance/REVISION.toml` is never re-emitted
  (AC2 of the feature).
- The sibling-root `.gitignore` stanza (one per new root) keeps bulk parquet out
  of git; only `REVISION.toml` is tracked (AC3).
- A manifest-internal-consistency test per new root (mirrors
  `crates/data/tests/yahoo_revision_verify.rs`) re-derives the aggregate SHA from
  the committed `REVISION.toml`'s own `[files]` map — runs with no parquet
  present, CI-safe (AC1 partial, AC7-friendly).

**What breaks if this is violated:**

- A developer adds a daily timeframe **into** `data/binance/` (or any existing
  1h root) → filename collision with the hourly month parquet, or a re-emit of
  the root's `REVISION.toml` → anchor SHA flip. Caught by `verify_anchors.sh`.
- A developer adds a `/<TF>/` segment to a Binance root expecting `ReplayFeed` to
  find it → `parquet_files()` globs one level too shallow, returns zero bars, the
  consumer SKIPs or errors. Caught by the smoke consumer / coverage assertion.
- A developer renames an existing pinned root to add an explicit `1h` suffix →
  every consumer's hard-coded path breaks and the pin's relpaths shift. The
  convention explicitly forbids renaming existing roots.

**What this enables:**

- **Binance daily v0.2 is a trivial follow-on** — a new `data/binance-2122-1d/`
  (or `data/binance-1d/`) root, fetched with `--interval 1d`, pinned with its own
  `REVISION.toml`, gitignored with its own stanza. **Zero `ReplayFeed` change**,
  zero anchor risk, no reshape of any existing corpus. The convention is the
  reason v0.2 stays cheap.
- **Any future Binance (universe × range × timeframe) corpus** slots in as a
  named sibling root with no cross-cutting code change — the pattern is now
  documented and bounded.

## Cross-references

- [ADR-0032](0032-backtest-realdata-path-and-revision-pin.md) — Binance corpus
  pin contract; `REVISION.toml` schema + aggregate-SHA algorithm reused verbatim
  on every new root.
- [ADR-0040](0040-yahoo-realdata-path.md) § D3 — the Yahoo `/<INTERVAL>/` segment
  this ADR deliberately does **not** mirror onto Binance (D2).
- [ADR-0055](0055-lab-run-persistence-topology-and-anchor-safety.md) D2 —
  anchor-safety-by-construction precedent (the git/path boundary as the
  byte-immutability guarantee, not reviewer vigilance).
- [`crates/data/src/replay_feed.rs`](../../../crates/data/src/replay_feed.rs)
  `parquet_files()` — the no-timeframe-segment glob this convention preserves.
- [`crates/data/src/bin/fetch_binance_klines.rs`](../../../crates/data/src/bin/fetch_binance_klines.rs)
  — `--interval` / `--out` / `--emit-revision-manifest` CLI; writes the flat
  `<out>/<SYMBOL>/<YEAR>/<MM>.parquet` layout; zero code change needed for any
  timeframe under this convention.
- [`spec/binance-corpus-expansion/feature.md`](../../binance-corpus-expansion/feature.md)
  § Open questions Q1 — the question this ADR resolves.

## Changelog

- 2026-06-15 (architect, M-T1 binance-corpus-expansion): initial accept. Locks
  D1 own-root-per-timeframe (flat `<root>/<SYM>/<YEAR>/<MM>.parquet` retained;
  timeframe encoded by root-dir name with a `-<tf>` suffix for non-1h roots;
  bare roots are 1h by convention; existing pinned roots never renamed), D2
  intentional Binance↔Yahoo layout divergence (no convergence retrofit — the
  Yahoo `/<INTERVAL>/` segment is not mirrored onto Binance, justified by the
  differing `ReplayFeed`-glob vs `YahooBarSource`-explicit-interval access
  patterns; a retrofit would re-emit `3a8b96c4` or skew the glob), D3 zero
  `ReplayFeed` change — root-path selection is the only knob, anchors neutral by
  construction. v0.1 `data/binance-2122/` is a flat 1h root, correct and final
  under D1. Daily v0.2 is unblocked as a new suffixed sibling root with no
  reader change. 119/119 anchors stay byte-identical (no anchor SHA in scope; no
  anchor-mutation ADR required). Resolves Q1 of
  `spec/binance-corpus-expansion/feature.md`.
