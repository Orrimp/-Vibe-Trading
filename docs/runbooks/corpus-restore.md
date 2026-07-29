# Runbook — Real-Data Corpus Restore (Disaster Recovery)

**Created:** 2026-07-27 (adversarial-review hardening pass, finding: "disaster
recovery for the evidence engine is a shrug").

## What is and isn't in git

| Asset | In git? | Restorable from repo alone? |
|---|---|---|
| `evidence/**/reports` (119 anchored bodies) | ✅ | ✅ — anchors verify against committed files |
| `data/*/REVISION.toml` pin manifests (incl. `data/binance/` since 2026-07-27) | ✅ | ✅ |
| The parquet corpora themselves (`data/binance*`, `data/coinbase`, `data/yahoo`, …) | ❌ (gitignored, ~GBs) | ❌ — machine-local only |
| `crates/forecast/checkpoints/anchors/*.safetensors` | ✅ (LFS) | ✅ via `git lfs pull` |

**Consequence of losing this machine:** every committed proof still verifies
(anchors are self-contained), but every *real-data test* degrades to loud-fail
(post bug-log #66) or skip until the corpora are restored — and a fresh
re-fetch is NOT guaranteed to reproduce the pinned bytes (exchanges revise
history; a diverging fetch trips the revision pin **by design**).

## Restore paths, in order of preference

### 1. Offline backup (the only path that provably re-matches the pin)

Take one now and after any corpus change:

```bash
tar -czf ~/trading-corpora-$(date +%Y%m%d).tar.gz data/binance data/binance-1718 data/binance-2020 data/binance-2122 data/binance-2526 data/binance-basis data/binance-broaduni data/binance-funding data/coinbase data/deribit-dvol data/yahoo data/defillama-stablecoins
```

Store off-machine (external disk / cloud). Restore = untar at the repo root,
then verify every manifest:

```bash
for d in data/*/REVISION.toml; do echo "== $d"; done
cargo run -p data --bin fetch_binance_klines -- --verify-only 2>/dev/null || true
bash scripts/verify_anchors.sh | tail -1
```

(The authoritative per-corpus verifier is the loader itself: any pinned-path
test run — e.g. `cargo test -p backtest --test binance_cache_dispatch -- --nocapture` —
performs the manifest + aggregate-SHA check and fails loudly on divergence.)

### 2. Re-fetch + pin comparison (best-effort, divergence possible)

```bash
cargo run -p data --bin fetch_binance_klines
```

then re-run the pinned-path tests. Two outcomes:

- **Pin matches** — you got lucky (no exchange-side revision); done.
- **Pin trips** — the exchange revised history. Do NOT edit the pin to make it
  pass. This is a formal **re-pin event**: new `REVISION.toml`, a dated
  dev-note recording old vs new aggregate SHA + which months changed, and an
  assessment of whether any anchored conclusion depended on revised bars
  (the anchored reports themselves stay byte-frozen regardless — they are
  historical records of runs against the OLD revision). Treat it like the
  ADR-0045 § D6 re-lock class.

### 3. Partial loss

A single corrupted symbol/month: restore that file from backup, re-run the
manifest check. The manifest is per-file — it names exactly what diverged.

## Standing instruction

- **After adding any new corpus:** add its `REVISION.toml` gitignore exception
  (`!/data/<name>/` + `/data/<name>/*` + `!/data/<name>/REVISION.toml` — see
  `.gitignore`; `data/binance/` itself was missing until 2026-07-27, bug-log
  #66 F1 compounding factor) and refresh the offline backup.
- **Backup cadence:** at minimum after every corpus addition or re-fetch;
  corpora are append-only in practice, so stale backups still cover the old
  eras.
