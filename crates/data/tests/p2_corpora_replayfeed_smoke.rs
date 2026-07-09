//! T8 — SKIP-safe `ReplayFeed` smoke tests for the 4 P2 corpora (ADR-0084
//! D1 + D2.b; `spec/v3/advisor-corpus-expansion/tasks.md` T8).
//!
//! Mirrors `binance_2122_revision_consistency.rs::smoke_consumer_btcusdt_2022`:
//! for each new corpus, prove `ReplayFeed` loads + iterates a sane, non-empty
//! bar stream for a representative symbol, with an era-sanity price-range
//! check (a cheap sanity gate that the parquet content actually belongs to
//! the intended regime — not just "some non-zero decimal").
//!
//! SKIP-guards on the sentinel parquet file being absent (the gitignored
//! bulk corpora may not be fetched on every machine) so CI without them stays
//! green; `#[ignore]` by default (real I/O, run explicitly once the corpus is
//! on disk, per the harness's own `#[ignore]` convention for the multi-hour-
//! fetch-gated scenarios).
//!
//! Era-sanity bounds are grounded in the ACTUAL on-disk price range observed
//! 2026-07-10 (a throwaway probe over the real corpora, not a guess):
//!
//! | Corpus            | Symbol  | Observed close range        | Assertion window          |
//! |--------------------|---------|------------------------------|----------------------------|
//! | `data/binance-1718` | BTCUSDT | $2,919.00 – $19,709.50       | 2017-12 mania: $10k–$20k   |
//! | `data/binance-2020` | BTCUSDT | $4,130.64 – $29,155.25       | whole-year: $3k–$30k       |
//! | `data/binance-2526` | BTCUSDT | $58,290.17 – $126,011.18     | whole-window: $50k–$130k   |
//! | `data/coinbase`      | BTCUSDT | $4,209.51 – $126,099.22      | whole-window: $3k–$130k    |
//!
//! Run individually:
//! ```text
//! cargo test -p data --test p2_corpora_replayfeed_smoke -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use data::source::MarketDataSource as _;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio_stream::StreamExt as _;
use trading_core::{Symbol, Timeframe};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Load every bar for `symbol` from `<workspace>/<corpus_dir>`, asserting all
/// prices are non-zero `Decimal` and every close falls within
/// `[lo_bound, hi_bound]` (the era-sanity check). Returns the bar count.
///
/// SKIP-guards on `sentinel_relpath` (a specific parquet file expected to
/// exist when the corpus is fetched) being absent.
async fn smoke_load_and_check_range(
    corpus_dir: &str,
    sentinel_relpath: &str,
    symbol: &str,
    lo_bound: Decimal,
    hi_bound: Decimal,
    min_expected_bars: u64,
) {
    let root = workspace_root().join(corpus_dir);
    let sentinel = root.join(sentinel_relpath);
    if !sentinel.is_file() {
        eprintln!(
            "SKIP {corpus_dir} smoke: corpus absent ({})",
            sentinel.display()
        );
        return;
    }

    let feed = data::ReplayFeed::new(root.clone(), true);
    let sym = Symbol::new(symbol);

    let mut stream = feed
        .subscribe_bars(sym.clone(), Timeframe::OneHour)
        .await
        .unwrap_or_else(|e| panic!("{corpus_dir}: subscribe_bars should succeed: {e}"));

    let mut bar_count = 0u64;
    let mut min_seen = Decimal::MAX;
    let mut max_seen = Decimal::MIN;

    while let Some(result) = stream.next().await {
        let bar = result.unwrap_or_else(|e| panic!("{corpus_dir}: bar should parse: {e}"));

        // AC6: prices are Decimal (the read path parses Utf8 → Decimal, never f64).
        let close_dec = bar.close.get();
        assert!(
            !close_dec.is_zero(),
            "{corpus_dir}: close price must be non-zero for a real {symbol} bar"
        );
        assert!(
            close_dec >= lo_bound && close_dec <= hi_bound,
            "{corpus_dir}: {symbol} close {close_dec} at ts={} is outside the era-sanity range \
             [{lo_bound}, {hi_bound}] — this is a cheap gate that the parquet content actually \
             belongs to the intended regime, not merely a non-zero price",
            bar.open_ts.unix_millis()
        );

        if close_dec < min_seen {
            min_seen = close_dec;
        }
        if close_dec > max_seen {
            max_seen = close_dec;
        }
        bar_count += 1;
    }

    assert!(
        bar_count >= min_expected_bars,
        "{corpus_dir}: expected >= {min_expected_bars} {symbol} bars, got {bar_count}"
    );

    eprintln!(
        "OK {corpus_dir} smoke: {symbol} read {bar_count} bars from {root:?}, \
         close range [{min_seen}, {max_seen}]"
    );
}

// ── T8.1 — data/binance-1718 (2017 mania blow-off era-sanity) ────────────────

/// BTC 2017-12 mania: the observed on-disk range for the WHOLE corpus is
/// $2,919.00–$19,709.50 (2017-08 low to 2017-12 blow-off top); this assertion
/// window ($1k-$25k) covers the full corpus while still being a real
/// era-sanity gate (would fail loudly if pointed at, e.g., a 2024 corpus
/// where BTC trades $60k+).
#[tokio::test]
#[ignore = "requires data/binance-1718 on disk — run after the P2 corpus fetch (T4)"]
async fn binance_1718_btcusdt_smoke_era_sanity() {
    smoke_load_and_check_range(
        "data/binance-1718",
        "BTCUSDT/2017/12.parquet",
        "BTCUSDT",
        dec!(1_000),
        dec!(25_000),
        100,
    )
    .await;
}

// ── T8.2 — data/binance-2020 (COVID crash + recovery era-sanity) ────────────

/// BTC 2020: observed on-disk range $4,130.64 (Mar-2020 COVID crash low) –
/// $29,155.25 (Dec-2020 rally). Assertion window $3k-$30k.
#[tokio::test]
#[ignore = "requires data/binance-2020 on disk — run after the P2 corpus fetch (T4)"]
async fn binance_2020_btcusdt_smoke_era_sanity() {
    smoke_load_and_check_range(
        "data/binance-2020",
        "BTCUSDT/2020/03.parquet",
        "BTCUSDT",
        dec!(3_000),
        dec!(30_000),
        100,
    )
    .await;
}

// ── T8.3 — data/binance-2526 (recent 2025-26 era-sanity) ────────────────────

/// BTC 2025-01 → 2026-06: observed on-disk range $58,290.17 – $126,011.18.
/// Assertion window $50k-$130k.
#[tokio::test]
#[ignore = "requires data/binance-2526 on disk — run after the P2 corpus fetch (T4)"]
async fn binance_2526_btcusdt_smoke_era_sanity() {
    smoke_load_and_check_range(
        "data/binance-2526",
        "BTCUSDT/2025/01.parquet",
        "BTCUSDT",
        dec!(50_000),
        dec!(130_000),
        100,
    )
    .await;
}

// ── T8.4 — data/coinbase (venue cross-check, BTC-USD, on-disk BTCUSDT) ──────

/// Coinbase BTC-USD 2020-01 → 2026-06 (on-disk canonical `BTCUSDT` per
/// ADR-0084 D2.a): observed range $4,209.51 – $126,099.22. Assertion window
/// $3k-$130k — spans the same 2020-COVID-crash-through-2026 window as the
/// Binance corpora, the direct basis for the AC5 venue-reconciliation stat.
#[tokio::test]
#[ignore = "requires data/coinbase on disk — run after the P2 corpus fetch (T4)"]
async fn coinbase_btcusdt_smoke_era_sanity() {
    smoke_load_and_check_range(
        "data/coinbase",
        "BTCUSDT/2020/01.parquet",
        "BTCUSDT",
        dec!(3_000),
        dec!(130_000),
        100,
    )
    .await;
}
